use core::time::Duration;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{sleep, spawn};

use log::{debug, warn};
use rodio::{Decoder, OutputStream, Sink};
use serde::Serialize;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("decoder error: {0}")]
    Decoder(#[from] rodio::decoder::DecoderError),
    #[error("seek error: {0}")]
    Seek(#[from] rodio::source::SeekError),
    #[error("stream error: {0}")]
    Stream(#[from] rodio::StreamError),
    #[error("play error: {0}")]
    Play(#[from] rodio::PlayError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("symphonia error: {0}")]
    Symphonia(#[from] symphonia::core::errors::Error),
    #[error("player not started")]
    PlayerNotStarted,
    #[error("source not set")]
    SourceNotSet,
    #[error("tracks not found for source: {0}")]
    TracksNotFound(String),
    #[error("codec parameters missing")]
    CodecParamsMissing,
}

pub struct Player {
    sink: Arc<RwLock<Option<Sink>>>,
    source: Mutex<Option<SourceInfo>>,
    emitter: Arc<dyn PlayerEmitter + Send + Sync>,
}

struct SourceInfo {
    path: PathBuf,
    first_transit_pos: Duration,
}

pub trait PlayerEmitter {
    fn on_player_state_updated(&self, state: PlayerState);
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub playing: bool,
    pub pos: f32,
}

impl Player {
    pub fn new<T>(emitter: T) -> Self
    where
        T: PlayerEmitter + Send + Sync + 'static,
    {
        Self {
            sink: Arc::new(None.into()),
            source: None.into(),
            emitter: Arc::new(emitter),
        }
    }

    pub fn run(&mut self) {
        let sink = self.sink.clone();
        let emitter = self.emitter.clone();

        spawn(move || {
            debug!("start player thread");

            // stream must be created in the player thread
            // because it needs to live as long as the sink
            let (_stream, handle) =
                OutputStream::try_default().expect("failed to create output stream");
            sink.write()
                .unwrap()
                .replace(Sink::try_new(&handle).expect("failed to create sink"));

            let mut had_source = false;
            loop {
                // sleep should before break, to make sure it always sleep each loop
                sleep(Duration::from_millis(100));

                let empty = match sink.read().unwrap().as_ref() {
                    Some(sink) => sink.empty(),
                    None => break, // <== break HERE
                };

                if had_source && empty {
                    // had source -> empty
                    debug!("source ended");
                    emitter.on_player_state_updated(PlayerState {
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

    pub fn terminate(&self) {
        self.sink.write().unwrap().take();
    }

    pub fn set_source(&mut self, path: PathBuf) -> Result<(), Error> {
        let sink = self.sink.read().unwrap();
        let sink = sink.as_ref().ok_or(Error::PlayerNotStarted)?;

        debug!("set source: {path:?}");

        sink.clear();
        self.emitter.on_player_state_updated(PlayerState {
            playing: !sink.is_paused(),
            pos: 0.,
        });

        let first_transit_pos = get_first_transit_pos(&path)?;
        debug!("first transit pos: {first_transit_pos:?}");
        self.source.lock().unwrap().replace(SourceInfo {
            path,
            first_transit_pos,
        });

        Ok(())
    }

    pub fn play(&self, seek: Option<Duration>, skip_silence: bool) -> Result<(), Error> {
        {
            let sink = self.sink.read().unwrap();
            let sink = sink.as_ref().ok_or(Error::PlayerNotStarted)?;

            let source_info = self.source.lock().unwrap();
            let source_info = source_info.as_ref().ok_or(Error::SourceNotSet)?;

            if sink.empty() {
                let source_path = source_info.path.as_path();
                let file = BufReader::new(File::open(source_path)?);
                let source = Decoder::new(file)?;
                sink.append(source);
            }

            debug!("continue playing");

            let mut seek_pos = seek.unwrap_or_default();
            if skip_silence {
                seek_pos = seek_pos.max(source_info.first_transit_pos); // seek to the furthest allowed position
            }

            dbg!(sink.empty());

            sink.try_seek(seek_pos)?;
            sink.play();
        }

        self.emitter.on_player_state_updated(PlayerState {
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

        self.emitter.on_player_state_updated(PlayerState {
            playing: false,
            pos: self.get_pos(),
        });
    }

    pub fn stop(&self) {
        {
            let sink = self.sink.read().unwrap();
            if let Some(sink) = sink.as_ref() {
                debug!("stop");
                sink.clear();
            }
        }

        self.emitter.on_player_state_updated(PlayerState {
            playing: false,
            pos: 0.,
        });
    }

    pub fn set_volume(&self, volume: f32) -> Result<(), Error> {
        let sink = self.sink.read().unwrap();
        let sink = sink.as_ref().ok_or(Error::PlayerNotStarted)?;

        debug!("set volume to {volume}");
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
        self.terminate();
    }
}

pub fn get_format_reader(
    path: &Path,
) -> Result<Box<dyn FormatReader>, symphonia::core::errors::Error> {
    let src = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(src), MediaSourceStreamOptions::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension() {
        hint.with_extension(ext.to_string_lossy().as_ref());
    }
    let format = symphonia::default::get_probe().probe(
        &hint,
        mss,
        FormatOptions::default(),
        MetadataOptions::default(),
    )?;
    Ok(format)
}

#[allow(clippy::cast_precision_loss)]
pub fn get_first_transit_pos(path: &Path) -> Result<Duration, Error> {
    let mut reader = get_format_reader(path)?;
    let track = reader
        .default_track(TrackType::Audio)
        .ok_or(Error::TracksNotFound(path.to_string_lossy().to_string()))?;
    let track_id = track.id;

    let params = &track
        .codec_params
        .as_ref()
        .and_then(CodecParameters::audio)
        .ok_or_else(|| Error::CodecParamsMissing)?;
    let n_channels = params
        .channels
        .as_ref()
        .ok_or_else(|| Error::CodecParamsMissing)?
        .count();
    let sample_rate = params.sample_rate.unwrap();

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())?;
    let mut sample_buf = Vec::<i16>::new();

    let mut current_pos: usize = 0;

    let pos_frame = loop {
        match reader.next_packet() {
            Err(err) => break Err(err),

            Ok(Some(packet)) => {
                if packet.track_id() != track_id {
                    continue;
                }

                match decoder.decode(&packet) {
                    Ok(audio_buf) => {
                        sample_buf.resize(audio_buf.samples_interleaved(), 0);
                        audio_buf.copy_to_slice_interleaved(&mut sample_buf);

                        if let Some(pos) = sample_buf.iter().position(|&x| x != 0) {
                            // first transit found
                            let pos_frame = (current_pos + pos) / n_channels;
                            break Ok(Some(pos_frame));
                        }

                        // first transit not found, continue
                        let packet_length = sample_buf.len();
                        current_pos += packet_length;
                    }

                    // The packet failed to decode due to an IO error or invalid data, skip the packet.
                    Err(symphonia::core::errors::Error::IoError(err)) => {
                        // The packet failed to decode due to an IO error, skip the packet.
                        warn!("IO error: {err}");
                    }
                    Err(symphonia::core::errors::Error::DecodeError(err)) => {
                        // The packet failed to decode due to invalid data, skip the packet.
                        warn!("decode error: {err}");
                    }

                    // An unrecoverable error occurred, halt decoding.
                    Err(err) => break Err(err),
                }
            }

            Ok(None) => break Ok(None), // end of stream
        }
    }?; // loop

    // Default to 0
    Ok(Duration::from_secs_f32(
        pos_frame.unwrap_or(0) as f32 / sample_rate as f32,
    ))
}
