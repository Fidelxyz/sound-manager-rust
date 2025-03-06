use super::core;
use super::core::{Entry, Folder, Tag};

use serde::ser::{Serialize, SerializeStruct, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Entry(#[from] core::entry::Error),
    #[error(transparent)]
    Database(#[from] core::database::Error),
    #[error(transparent)]
    Player(#[from] core::player::Error),
    #[error(transparent)]
    Waveform(#[from] core::waveform::Error),
    #[error("entry not found for id: {0}")]
    EntryNotFound(i32),
}

#[derive(serde::Serialize)]
#[serde(tag = "kind", content = "message")]
#[serde(rename_all = "camelCase")]
enum ErrorKind {
    Entry(String),
    Database(String),
    Player(String),
    Waveform(String),
    EntryNotFound(String),
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let error_message = self.to_string();
        let error_kind = match self {
            Self::Entry(_) => ErrorKind::Entry(error_message),
            Self::Database(_) => ErrorKind::Database(error_message),
            Self::Player(_) => ErrorKind::Player(error_message),
            Self::Waveform(_) => ErrorKind::Waveform(error_message),
            Self::EntryNotFound(_) => ErrorKind::EntryNotFound(error_message),
        };
        error_kind.serialize(serializer)
    }
}

impl Serialize for Entry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Entry", 7)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("fileName", &self.file_name)?;
        if let Some(metadata) = &self.metadata {
            if let Some(title) = &metadata.title {
                state.serialize_field("title", title)?;
            }
            if let Some(artist) = &metadata.artist {
                state.serialize_field("artist", artist)?;
            }
            if let Some(album) = &metadata.album {
                state.serialize_field("album", album)?;
            }
            if let Some(duration) = &metadata.duration {
                state.serialize_field("duration", duration)?;
            }
        }
        state.end()
    }
}

impl Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Tag", 2)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("name", &self.name)?;
        state.end()
    }
}

impl Serialize for Folder {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Folder", 3)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("subFolders", &self.sub_folders)?;
        state.end()
    }
}
