use super::player::get_format_reader;

use log::{debug, warn};
use std::path::Path;
use std::slice::from_raw_parts;
use std::sync::{Arc, RwLock};
use std::thread::spawn;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::formats::{FormatReader, Track};
use thiserror::Error;

static BATCH_SAMPLES: usize = 1024;
static SAMPLING_STEP: usize = 512;

#[derive(Error, Debug)]
pub enum Error {
    #[error("source not set")]
    SourceNotSet,
    #[error("source reset")]
    SourceReset,
    #[error("tracks not found for source: {0}")]
    TracksNotFound(String),
    #[error(transparent)]
    Symphonia(#[from] symphonia::core::errors::Error),
}

pub struct WaveformGenerator {
    source_path: Arc<RwLock<Option<Box<Path>>>>,
}

impl WaveformGenerator {
    pub fn new() -> Self {
        Self {
            source_path: Arc::new(None.into()),
        }
    }

    pub fn set_source(&mut self, path: Box<Path>) {
        self.source_path.write().unwrap().replace(path);
    }

    fn get_default_track<'reader>(
        &self,
        reader: &'reader dyn FormatReader,
    ) -> Result<&'reader Track, Error> {
        let track = reader.default_track().ok_or_else(|| {
            Error::TracksNotFound(
                self.source_path
                    .read()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
            )
        })?;
        Ok(track)
    }

    pub fn prepare_waveform(&self) -> Result<u32, Error> {
        let path = self.source_path.read().unwrap();
        let path = path.as_ref().ok_or(Error::SourceNotSet)?;
        let reader = get_format_reader(path)?;

        let track = self.get_default_track(reader.as_ref())?;
        let params = &track.codec_params;
        let n_frames = params.n_frames.unwrap();
        let waveform_samples_num = (n_frames / SAMPLING_STEP as u64) as u32;
        let data_length = waveform_samples_num;

        Ok(data_length)
    }

    pub fn request_waveform<F>(&self, on_data_available: F) -> Result<(), Error>
    where
        F: Fn(&[u8]) + Send + 'static,
    {
        let mut reader = get_format_reader(
            &self
                .source_path
                .write()
                .unwrap()
                .take() // source_path is set to None
                .ok_or(Error::SourceNotSet)?,
        )?;

        let track = self.get_default_track(reader.as_ref())?;
        let track_id = track.id;

        let params = &track.codec_params;
        let n_frames = params.n_frames.unwrap();
        let n_channels = params.channels.unwrap().count();
        let src_samples_per_batch = n_channels * SAMPLING_STEP * BATCH_SAMPLES;
        let waveform_samples_num = (n_frames / SAMPLING_STEP as u64) as u32;

        debug!("n_frames: {}", n_frames);
        debug!("n_channels: {}", n_channels);
        debug!("waveform_samples_num: {}", waveform_samples_num);

        let mut decoder =
            symphonia::default::get_codecs().make(&track.codec_params, &Default::default())?;

        let source_path_ref = self.source_path.clone();
        spawn(move || {
            debug!("start waveform generation");

            let mut sample_buf = None;
            let mut samples = Vec::with_capacity(n_frames as usize * n_channels);
            let mut available_samples_head = 0;

            // Decode all packets, ignoring all decode errors.
            let result: Error = loop {
                // if source_path is not None, it means source is reset
                if !source_path_ref.read().unwrap().is_none() {
                    break Error::SourceReset;
                }

                // loop for packets
                let result = match reader.next_packet() {
                    Err(err) => Err(err), // about to break

                    Ok(packet) => {
                        // If the packet does not belong to the selected track, skip over it.
                        if packet.track_id() != track_id {
                            continue;
                        }

                        // Decode the packet into audio samples.
                        match decoder.decode(&packet) {
                            Ok(audio_buf) => {
                                // create sample buffer if not exists
                                let sample_buf = sample_buf.get_or_insert_with(|| {
                                    let spec = *audio_buf.spec();
                                    let capacity = audio_buf.capacity() as u64;
                                    SampleBuffer::<i16>::new(capacity, spec)
                                });

                                sample_buf.copy_interleaved_ref(audio_buf);
                                samples.extend_from_slice(sample_buf.samples());
                                Ok(()) // continue
                            }
                            Err(symphonia::core::errors::Error::DecodeError(err)) => {
                                warn!("decode error: {}", err);
                                Ok(()) // continue
                            }
                            Err(err) => Err(err), // about to break
                        }
                    }
                };

                // consume available samples
                loop {
                    let available_samples_num = match result {
                        Ok(()) => {
                            // decoding not ended, whether there are enough data for a batch
                            if samples.len() >= available_samples_head + src_samples_per_batch {
                                src_samples_per_batch
                            } else {
                                0
                            }
                        }
                        Err(_) => {
                            // about to break, consume all remaining samples
                            if samples.len() > available_samples_head {
                                samples.len() - available_samples_head //  all remaining samples
                            } else {
                                0
                            }
                        }
                    };

                    if available_samples_num == 0 {
                        break;
                    }

                    let data: Vec<f32> = samples
                        [available_samples_head..available_samples_head + available_samples_num]
                        // average samples of all channels per frame
                        .chunks_exact(n_channels)
                        .map(|chunk| {
                            let sum: i32 = chunk.iter().map(|x| *x as i32).sum();
                            (sum / n_channels as i32) as i16
                        })
                        .collect::<Vec<_>>()
                        // get min and max for each chunk
                        .chunks_exact(SAMPLING_STEP)
                        .map(|chunk| {
                            let max = chunk.iter().map(|x| (*x as i32).abs()).max().unwrap();
                            max as f32 / i16::MAX as f32
                        })
                        .collect();

                    on_data_available(unsafe {
                        from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<f32>())
                    });

                    available_samples_head += src_samples_per_batch;
                }

                // handle break
                if let Err(err) = result {
                    break err.into();
                }
            }; // loop

            match result {
                Error::Symphonia(symphonia::core::errors::Error::IoError(err))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof
                        && err.to_string() == "end of stream" =>
                {
                    // End of stream is expected
                }
                err => {
                    warn!("waveform generator: {}", err);
                }
            }

            debug!("waveform generation done");
        });

        Ok(())
    }
}
