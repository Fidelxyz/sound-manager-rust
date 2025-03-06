use core::time::Duration;
use futures::try_join;
use log::{debug, trace};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::thread::{sleep, spawn};
use symphonia::core::audio::SampleBuffer;

use rodio::{Decoder, OutputStream, Sink};
use serde::Serialize;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("decoder error: {0}")]
    Decoder(#[from] rodio::decoder::DecoderError),
    #[error("seek error: {0}")]
    Seek(#[from] rodio::source::SeekError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("symphonia error: {0}")]
    Symphonia(#[from] symphonia::core::errors::Error),
    #[error("player not started")]
    PlayerNotStarted,
    #[error("tracks not found for source: {0}")]
    TracksNotFound(String),
}

pub struct Player {
    sink: Arc<RwLock<Option<Sink>>>,
    first_transit_pos: Duration,
    on_state_updated: Option<Box<dyn Fn(PlayerState) + Send + Sync + 'static>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub playing: bool,
    pub pos: f32,
}

impl Player {
    pub fn new() -> Self {
        Self {
            sink: Arc::new(None.into()),
            first_transit_pos: Duration::default(),
            on_state_updated: None,
        }
    }

    pub fn run<F>(&mut self, on_state_updated: F)
    where
        F: Fn(PlayerState) + Send + Sync + Clone + 'static,
    {
        let sink_ref = self.sink.clone();
        self.on_state_updated = Some(Box::new(on_state_updated.clone()));

        spawn(move || {
            debug!("start player thread");

            let (_stream, handle) = OutputStream::try_default().unwrap();

            {
                let mut sink = sink_ref.write().unwrap();
                sink.replace(Sink::try_new(&handle).unwrap());
            }

            let mut had_source = false;
            loop {
                //debug!("player thread loop");

                // sleep should before break, to make sure it always sleep each loop
                sleep(Duration::from_millis(100));

                let empty = match sink_ref.read().unwrap().as_ref() {
                    Some(sink) => sink.empty(),
                    None => break, // <== break HERE
                };

                if had_source && empty {
                    // had source -> empty
                    debug!("source ended");
                    on_state_updated(PlayerState {
                        playing: false,
                        pos: 0.,
                    });
                    had_source = false;
                } else if !empty {
                    had_source = true;
                }
            }

            debug!("stop player thread");
        });
    }

    pub fn stop(&self) {
        let mut sink = self.sink.write().unwrap();
        sink.take();
    }

    pub async fn set_source(&mut self, path: Box<Path>) -> Result<(), Error> {
        let sink_ref = self.sink.clone();

        let set_sink_source = async {
            trace!("set_sink_source");

            let sink = sink_ref.read().unwrap();
            let sink = sink.as_ref().ok_or(Error::PlayerNotStarted)?;

            sink.clear();

            let file = BufReader::new(File::open(&path)?);
            let source = Decoder::new(file)?;
            sink.append(source);

            self.on_state_updated.as_ref().unwrap()(PlayerState {
                playing: !sink.is_paused(),
                pos: 0.,
            });

            trace!("set_sink_source done");
            Ok::<_, Error>(())
        };

        let set_first_transit_pos = async {
            trace!("set_first_transit_pos");

            let mut reader = get_format_reader(&path)?;
            let track = reader
                .default_track()
                .ok_or(Error::TracksNotFound(path.to_str().unwrap().to_string()))?;
            let track_id = track.id;

            let params = &track.codec_params;
            let n_channels = params.channels.unwrap().count();
            let sample_rate = params.sample_rate.unwrap();

            let mut decoder =
                symphonia::default::get_codecs().make(&track.codec_params, &Default::default())?;
            let mut sample_buf = None;

            let mut current_pos: usize = 0;

            let pos_frame = loop {
                let pos_frame = match reader.next_packet() {
                    Err(e) => break Err(e),

                    Ok(packet) => {
                        if packet.track_id() != track_id {
                            continue;
                        }

                        let (pos, packet_length) = match decoder.decode(&packet) {
                            Ok(audio_buf) => {
                                let sample_buf = sample_buf.get_or_insert_with(|| {
                                    let spec = *audio_buf.spec();
                                    let capacity = audio_buf.capacity() as u64;
                                    SampleBuffer::<i16>::new(capacity, spec)
                                });

                                sample_buf.copy_interleaved_ref(audio_buf);

                                let pos = sample_buf.samples().iter().position(|&x| x != 0);
                                let packet_length = sample_buf.len();

                                (pos, packet_length)
                            }
                            Err(e) => break Err(e),
                        };

                        match pos {
                            Some(pos) => {
                                let pos_frame = (current_pos + pos) / n_channels;
                                Some(pos_frame)
                            }
                            None => {
                                current_pos += packet_length; // continue
                                None
                            }
                        }
                    }
                };

                if let Some(pos_frame) = pos_frame {
                    break Ok(pos_frame);
                }
            }; // loop

            self.first_transit_pos =
                Duration::from_secs_f32(pos_frame.unwrap_or(0) as f32 / sample_rate as f32);
            debug!("first transit pos: {:?}", self.first_transit_pos);

            trace!("set_first_transit_pos done");

            Ok::<_, Error>(())
        };

        try_join!(set_sink_source, set_first_transit_pos)?;

        Ok(())
    }

    pub fn play(&self, seek: Option<Duration>, skip_silence: bool) -> Result<(), Error> {
        {
            let sink = self.sink.read().unwrap();
            let sink = sink.as_ref().ok_or(Error::PlayerNotStarted)?;

            debug!("continue playing");

            let mut seek_pos = seek.unwrap_or_default();
            if skip_silence {
                seek_pos = seek_pos.max(self.first_transit_pos); // seek to the furthest allowed position
            }

            sink.try_seek(seek_pos)?;
            sink.play();
        }

        self.on_state_updated.as_ref().unwrap()(PlayerState {
            playing: true,
            pos: self.get_pos(),
        });

        Ok(())
    }

    pub fn pause(&self) {
        {
            let sink = self.sink.read().unwrap();
            if let Some(sink) = sink.as_ref() {
                debug!("pause");
                sink.pause();
            }
        }

        self.on_state_updated.as_ref().unwrap()(PlayerState {
            playing: false,
            pos: self.get_pos(),
        });
    }

    pub fn set_volume(&self, volume: f32) -> Result<(), Error> {
        let sink = self.sink.read().unwrap();
        let sink = sink.as_ref().ok_or(Error::PlayerNotStarted)?;

        debug!("set volume to {}", volume);
        sink.set_volume(volume);
        Ok(())
    }

    pub fn get_pos(&self) -> f32 {
        let sink = self.sink.read().unwrap();
        match sink.as_ref() {
            Some(sink) => sink.get_pos().as_secs_f32(),
            None => 0.,
        }
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn get_format_reader(
    path: &Path,
) -> Result<Box<dyn FormatReader>, symphonia::core::errors::Error> {
    let src = std::fs::File::open(path).unwrap();
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = Hint::new();
    hint.with_extension(path.extension().unwrap().to_str().unwrap());
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();
    let probed = symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;
    Ok(probed.format)
}
