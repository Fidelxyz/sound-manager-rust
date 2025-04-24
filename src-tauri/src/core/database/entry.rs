use crate::core::player::get_format_reader;

use log::warn;
use std::collections::HashSet;
use std::path::Path;

use symphonia::core::meta::StandardTagKey;

pub struct Entry {
    pub id: i32,
    pub path: Box<Path>,
    pub file_name: String,
    pub metadata: Option<Metadata>,
    pub tag_ids: HashSet<i32>,
}

pub struct Metadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<f32>,
}

impl Entry {
    pub fn new(path: Box<Path>) -> Self {
        debug_assert!(path.is_relative(), "Path must be relative");

        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();

        Self {
            id: -1,
            path,
            file_name,
            metadata: None,
            tag_ids: HashSet::new(),
        }
    }

    pub fn read_file(&mut self, base_path: &Path) {
        let metadata = self.read_metadata(base_path);

        match metadata {
            Ok(metadata) => self.metadata = Some(metadata),
            Err(err) => warn!(
                "Failed to read metadata of file {:?}: {:?}",
                base_path.join(&self.path),
                err
            ),
        }
    }

    fn read_metadata(&self, base_path: &Path) -> Result<Metadata, symphonia::core::errors::Error> {
        let mut ret = Metadata {
            title: None,
            artist: None,
            album: None,
            duration: None,
        };

        // Read metadata
        let mut format = get_format_reader(&base_path.join(&self.path))?;

        let mut metadata = format.metadata();
        if let Some(metadata) = metadata.skip_to_latest() {
            for tag in metadata.tags() {
                match tag.std_key {
                    Some(StandardTagKey::TrackTitle) => ret.title = Some(tag.value.to_string()),
                    Some(StandardTagKey::Artist) => ret.artist = Some(tag.value.to_string()),
                    Some(StandardTagKey::Album) => ret.album = Some(tag.value.to_string()),
                    _ => {}
                }
            }
        }

        if let Some(track) = format.default_track() {
            let params = &track.codec_params;
            if let (Some(n_frames), Some(time_base)) = (params.n_frames, params.time_base) {
                let time = time_base.calc_time(n_frames);
                ret.duration = Some(time.seconds as f32 + time.frac as f32);
            }
        }

        Ok(ret)
    }
}
