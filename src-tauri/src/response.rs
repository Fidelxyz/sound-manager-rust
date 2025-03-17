use super::core::{Entry, Folder, Tag};

use serde::ser::{Serialize, SerializeStruct, Serializer};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Database(#[from] crate::core::database::Error),
    #[error("player error: {0}")]
    Player(#[from] crate::core::player::Error),
    #[error("waveform error: {0}")]
    Waveform(#[from] crate::core::waveform::Error),
}

#[derive(serde::Serialize)]
#[serde(tag = "kind", content = "message")]
#[serde(rename_all = "camelCase")]
enum ErrorKind {
    DatabaseNotFound(String),
    DatabaseAlreadyExists(String),
    EntryNotFound(String),
    TagNotFound(String),
    TagAlreadyExists(String),
    TagNotFoundForEntry(String),
    TagAlreadyExistsForEntry(String),
    Database(String),
    Player(String),
    Waveform(String),
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let error_message = self.to_string();
        let error_kind = match self {
            Self::Database(e) => match e {
                crate::core::database::Error::DatabaseNotFound(_) => {
                    ErrorKind::DatabaseNotFound(error_message)
                }
                crate::core::database::Error::DatabaseAlreadyExists(_) => {
                    ErrorKind::DatabaseAlreadyExists(error_message)
                }
                crate::core::database::Error::EntryNotFound(_) => {
                    ErrorKind::EntryNotFound(error_message)
                }
                crate::core::database::Error::TagNotFound(_) => {
                    ErrorKind::TagNotFound(error_message)
                }
                crate::core::database::Error::TagAlreadyExists(_) => {
                    ErrorKind::TagAlreadyExists(error_message)
                }
                crate::core::database::Error::TagNotFoundForEntry(_, _) => {
                    ErrorKind::TagNotFoundForEntry(error_message)
                }
                crate::core::database::Error::TagAlreadyExistsForEntry(_, _) => {
                    ErrorKind::TagAlreadyExistsForEntry(error_message)
                }
                _ => ErrorKind::Database(error_message),
            },
            Self::Player(_) => ErrorKind::Player(error_message),
            Self::Waveform(_) => ErrorKind::Waveform(error_message),
        };
        error_kind.serialize(serializer)
    }
}

impl Serialize for Entry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Entry", 8)?;
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
        let mut state = serializer.serialize_struct("Tag", 3)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("color", &self.color)?;
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
