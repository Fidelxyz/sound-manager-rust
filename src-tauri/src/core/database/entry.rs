use super::folder::FolderId;
use super::tag::TagId;
use crate::core::player::get_format_reader;

use log::warn;
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use symphonia::core::meta::StandardTagKey;

pub type EntryId = i32;

pub struct Entry {
    pub id: EntryId,
    pub folder_id: FolderId,
    /// Relative path to the file
    pub path: PathBuf,
    pub file_name: OsString,
    pub metadata: Option<Metadata>,
    pub tag_ids: HashSet<TagId>,
}

pub struct Metadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<f32>,
}

impl Entry {
    pub fn new(path: PathBuf, folder_id: FolderId) -> Self {
        debug_assert!(path.is_relative(), "Path must be relative");

        let file_name = path.file_name().unwrap().to_owned();

        Self {
            id: -1,
            folder_id,
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

    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
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
