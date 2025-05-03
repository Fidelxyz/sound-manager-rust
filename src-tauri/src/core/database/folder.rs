use super::entry::EntryId;
use super::ROOT_FOLDER_ID;
use crate::response::serialize_os_string;

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

    #[serde(skip)]
    pub path: PathBuf,
    #[serde(skip)]
    pub parent_id: FolderId,
    /// Relative path to the folder
    #[serde(skip)]
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

impl FolderNode<'_> {
    /// Recursively build a node for the folder of the given `id` in the `folders` map.
    fn new(id: FolderId, folders: &HashMap<FolderId, Folder>) -> FolderNode {
        let folder = folders.get(&id).unwrap();

        let mut sub_folders = folder
            .sub_folders
            .values()
            .map(|sub_folder_id| FolderNode::new(*sub_folder_id, folders))
            .collect::<Vec<_>>();

        // sort sub_folders by name
        sub_folders.sort_by(|a, b| a.folder.name.cmp(&b.folder.name));

        FolderNode {
            folder,
            sub_folders,
        }
    }

    pub fn build(folders: &HashMap<FolderId, Folder>) -> FolderNode {
        FolderNode::new(ROOT_FOLDER_ID, folders)
    }
}
