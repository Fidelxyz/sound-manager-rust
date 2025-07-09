use super::entry::EntryId;
use crate::response::{serialize_hashmap_with_os_string_keys, serialize_os_string};

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

use serde::Serialize;

pub type FolderId = i32;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: FolderId,

    #[serde(serialize_with = "serialize_os_string")]
    pub name: OsString,

    /// Relative path to the folder
    #[serde(skip)]
    pub path: PathBuf,

    pub parent_id: FolderId,

    #[serde(serialize_with = "serialize_hashmap_with_os_string_keys")]
    pub sub_folders: HashMap<OsString, FolderId>,

    #[serde(skip)]
    pub entries: HashMap<OsString, EntryId>,
}

impl Folder {
    pub fn new(id: FolderId, parent_id: FolderId, name: OsString, path: PathBuf) -> Self {
        debug_assert!(path.is_relative(), "Path must be relative");

        Self {
            id,
            parent_id,
            name,
            path,
            sub_folders: HashMap::new(),
            entries: HashMap::new(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderNode<'a> {
    pub folder: &'a Folder,
    pub sub_folders: Vec<FolderNode<'a>>,
}
