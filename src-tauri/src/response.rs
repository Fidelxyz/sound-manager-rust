use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
};

use super::core::Entry;

use serde::ser::{Serialize, SerializeMap, SerializeStruct, Serializer};
use serde_json::{Map, Value};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("database not open")]
    DatabaseNotOpen,
    #[error(transparent)]
    Database(#[from] crate::core::database::Error),
    #[error("player error: {0}")]
    Player(#[from] crate::core::player::Error),
    #[error("waveform error: {0}")]
    Waveform(#[from] crate::core::waveform::Error),
    #[error("migrator error: {0}")]
    Migrator(#[from] crate::core::migrator::Error),
    #[error("opener error: {0}")]
    Opener(#[from] tauri_plugin_opener::Error),
}

#[derive(serde::Serialize)]
#[serde(tag = "kind", content = "message")]
#[serde(rename_all = "camelCase")]
enum ErrorKind {
    DatabaseNotFound(String),
    DatabaseAlreadyExists(String),
    TagAlreadyExists(String),
    TagAlreadyExistsForEntry(String),
    FileAlreadyExists(String),
    Other(String),
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let error_message = self.to_string();
        let error_kind = match self {
            Self::Database(err) => match err {
                crate::core::database::Error::DatabaseNotFound(_) => {
                    ErrorKind::DatabaseNotFound(error_message)
                }
                crate::core::database::Error::DatabaseAlreadyExists(_) => {
                    ErrorKind::DatabaseAlreadyExists(error_message)
                }
                crate::core::database::Error::TagAlreadyExists(_) => {
                    ErrorKind::TagAlreadyExists(error_message)
                }
                crate::core::database::Error::TagAlreadyExistsForEntry(..) => {
                    ErrorKind::TagAlreadyExistsForEntry(error_message)
                }
                crate::core::database::Error::FileAlreadyExists(_) => {
                    ErrorKind::FileAlreadyExists(error_message)
                }
                _ => ErrorKind::Other(error_message),
            },
            Self::Migrator(err) => match err {
                crate::core::migrator::Error::DatabaseAlreadyExists(_) => {
                    ErrorKind::DatabaseAlreadyExists(error_message)
                }
            },
            _ => ErrorKind::Other(error_message),
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
        state.serialize_field("fileName", &self.file_name.to_string_lossy())?;
        state.serialize_field("folderId", &self.folder_id)?;
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

pub fn serialize_os_string<S>(str: &OsStr, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    str.to_string_lossy().serialize(serializer)
}

pub fn serialize_hashmap_with_os_string_keys<V, S>(
    map: &HashMap<OsString, V>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    V: Serialize,
    S: Serializer,
{
    let mut state = serializer.serialize_map(Some(map.len()))?;
    for (key, value) in map {
        state.serialize_entry(&key.to_string_lossy(), value)?;
    }
    state.end()
}

pub fn to_serializable_map<K, V, M>(map: M) -> Result<Map<String, Value>, serde_json::Error>
where
    K: ToString,
    V: Serialize,
    M: Iterator<Item = (K, V)>,
{
    map.map(|(k, v)| Ok::<_, _>((k.to_string(), serde_json::to_value(v)?)))
        .collect()
}
