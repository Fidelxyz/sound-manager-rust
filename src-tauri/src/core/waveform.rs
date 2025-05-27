use super::player::get_format_reader;

use log::{debug, info, warn};
use std::path::PathBuf;
use std::slice::from_raw_parts;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::codecs::CodecParameters;

use symphonia::core::formats::{FormatReader, Track, TrackType};
use thiserror::Error;

const BATCH_SAMPLES: usize = 1024;
const SAMPLING_STEP: usize = 512;

#[derive(Error, Debug)]
pub enum Error {
    #[error("source not set")]
    SourceNotSet,
    #[error("tracks not found for source: {0}")]
    TracksNotFound(String),
    #[error(transparent)]
    Symphonia(#[from] symphonia::core::errors::Error),
    #[error("codec parameters missing")]
    CodecParamsMissing,
}

pub struct WaveformGenerator {
    source_path: Arc<Mutex<Option<PathBuf>>>,
    reset: Arc<AtomicBool>,
}

impl WaveformGenerator {
    pub fn new() -> Self {
        Self {
            source_path: Arc::new(None.into()),
            reset: Arc::new(false.into()),
        }
    }

    pub fn set_source(&mut self, path: Option<PathBuf>) {
        let mut source_path = self.source_path.lock().unwrap();
        *source_path = path;
        self.reset.store(true, std::sync::atomic::Ordering::Release);
    }

    fn get_default_track<'reader>(
        &self,
        reader: &'reader dyn FormatReader,
    ) -> Result<&'reader Track, Error> {
        let track = reader.default_track(TrackType::Audio).ok_or_else(|| {
            Error::TracksNotFound(
                self.source_path
                    .lock()
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
        let path = self.source_path.lock().unwrap();
        let path = path.as_ref().ok_or(Error::SourceNotSet)?;
        let reader = get_format_reader(path)?;

        let track = self.get_default_track(reader.as_ref())?;
        let n_frames = track.num_frames.unwrap();
        let waveform_samples_num: u32 = (n_frames / SAMPLING_STEP as u64).try_into().unwrap();
        let data_length = waveform_samples_num;

        Ok(data_length)
    }

    pub fn request_waveform<F>(&self, on_data_available: F) -> Result<(), Error>
    where
        F: Fn(&[u8]) + Send + 'static,
    {
        let mut reader = get_format_reader(
            self.source_path
                .lock()
                .unwrap()
                .as_ref()
                .ok_or(Error::SourceNotSet)?,
        )?;
        self.reset
            .store(false, std::sync::atomic::Ordering::Release);

        let track = self.get_default_track(reader.as_ref())?;
        let track_id = track.id;

        let params = track
            .codec_params
            .as_ref()
            .and_then(CodecParameters::audio)
            .ok_or(Error::CodecParamsMissing)?;
        let n_frames = track.num_frames.unwrap();
        let n_channels = params.channels.as_ref().unwrap().count();
        let src_samples_per_batch = n_channels * SAMPLING_STEP * BATCH_SAMPLES;
        let waveform_samples_num: u32 = (n_frames / SAMPLING_STEP as u64).try_into().unwrap();

        debug!("n_frames: {n_frames}");
        debug!("n_channels: {n_channels}");
        debug!("waveform_samples_num: {waveform_samples_num}");

        let mut decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &AudioDecoderOptions::default())?;

        let reset = self.reset.clone();
        spawn(move || {
            debug!("start waveform generation");

            let n_channels_i32 = i32::try_from(n_channels).unwrap();
            let n_samples = usize::try_from(n_frames).unwrap() * n_channels;
            let mut samples = vec![0i16; n_samples];
            let mut available_samples_head = 0; // The index of the next sample to be processed.
            let mut available_samples_tail = 0; // The index of the last sample to be processed.

            // Decode all packets, ignoring all decode errors.
            loop {
                if reset.load(std::sync::atomic::Ordering::Acquire) {
                    info!("Waveform generator: source reset");
                    break;
                }

                // loop for packets
                let end = match reader.next_packet() {
                    Err(err) => {
                        // about to break
                        warn!("Waveform generator: stopped generating waveform, because of unrecoverable error: {err}");
                        true
                    }

                    Ok(Some(packet)) => {
                        // If the packet does not belong to the selected track, skip over it.
                        if packet.track_id() != track_id {
                            continue;
                        }

                        // Decode the packet into audio samples.
                        match decoder.decode(&packet) {
                            Ok(audio_buf) => {
                                let n_new_samples = audio_buf.samples_interleaved();

                                audio_buf.copy_to_slice_interleaved(
                                    &mut samples[available_samples_tail
                                        ..available_samples_tail + n_new_samples],
                                );

                                available_samples_tail += n_new_samples;

                                false // continue
                            }

                            Err(symphonia::core::errors::Error::IoError(err)) => {
                                // The packet failed to decode due to an IO error, skip the packet.
                                warn!("Waveform generator: skipped packet, because of IO error: {err}");
                                false // continue
                            }
                            Err(symphonia::core::errors::Error::DecodeError(err)) => {
                                // The packet failed to decode due to invalid data, skip the packet.
                                warn!("Waveform generator: skipped packet, because of decode error: {err}");
                                false // continue
                            }

                            Err(err) => {
                                // about to break
                                warn!("Waveform generator: stopped generating wavefrom, because of unrecoverable error: {err}");
                                true
                            }
                        }
                    }

                    Ok(None) => true, // end of stream
                };

                if reset.load(std::sync::atomic::Ordering::Acquire) {
                    info!("Waveform generator: source reset");
                    break;
                }

                // consume available samples
                loop {
                    let available_samples_num = if end {
                        // about to break, consume all remaining samples
                        available_samples_tail.saturating_sub(available_samples_head)
                    } else {
                        // decoding not ended, whether there are enough data for a batch
                        if available_samples_tail >= available_samples_head + src_samples_per_batch
                        {
                            src_samples_per_batch
                        } else {
                            0
                        }
                    };

                    if available_samples_num == 0 {
                        break;
                    }

                    #[allow(clippy::cast_possible_truncation)]
                    #[allow(clippy::cast_precision_loss)]
                    let data: Vec<f32> = samples
                        [available_samples_head..available_samples_head + available_samples_num]
                        // average samples of all channels per frame
                        .chunks_exact(n_channels)
                        .map(|chunk| {
                            let sum: i32 = chunk.iter().map(|x| i32::from(*x)).sum();
                            (sum / n_channels_i32) as i16
                        })
                        .collect::<Vec<_>>()
                        // get min and max for each chunk
                        .chunks_exact(SAMPLING_STEP)
                        .map(|chunk| {
                            let max = chunk.iter().map(|x| (i32::from(*x)).abs()).max().unwrap();
                            max as f32 / f32::from(i16::MAX)
                        })
                        .collect();

                    on_data_available(unsafe {
                        from_raw_parts(data.as_ptr().cast::<u8>(), data.len() * size_of::<f32>())
                    });

                    available_samples_head += src_samples_per_batch;
                }
            } // loop

            debug!("waveform generation done");
        });

        Ok(())
    }
}
