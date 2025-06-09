use super::{DatabaseData, EntryId, FolderId, TagId, ROOT_FOLDER_ID};

use std::collections::HashSet;
use std::iter::Iterator;

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    pub search: String,
    pub tag_ids: Vec<TagId>,
    pub include_child_tags: bool,
    pub folder_id: Option<FolderId>,
    pub include_subfolders: bool,
}

impl DatabaseData {
    pub fn filter(&self, filter: &Filter) -> Option<Vec<EntryId>> {
        let tag_ids = filter
            .tag_ids
            .iter()
            .flat_map(|tag_id| {
                // include child tags
                if filter.include_child_tags {
                    self.get_tag_descendants(*tag_id)
                } else {
                    vec![*tag_id]
                }
            })
            .collect::<HashSet<_>>();

        let folder_ids = filter
            .folder_id
            .and_then(|folder_id| {
                // bypass the folder filter if possible
                if folder_id == ROOT_FOLDER_ID && filter.include_subfolders {
                    None
                } else {
                    Some(folder_id)
                }
            })
            .map(|folder_id| {
                // include subfolders
                if filter.include_subfolders {
                    self.get_folder_descendants(folder_id)
                } else {
                    vec![folder_id]
                }
            })
            .map(|vec| vec.into_iter().collect::<HashSet<_>>());

        if filter.search.is_empty() && tag_ids.is_empty() && folder_ids.is_none() {
            return None;
        }

        self.get_entries()
            .values()
            .filter(|entry| {
                let mut keep = true;

                if let Some(folder_ids) = &folder_ids {
                    keep &= folder_ids.contains(&entry.folder_id);
                }

                if !filter.tag_ids.is_empty() {
                    keep &= !entry.tag_ids.is_disjoint(&tag_ids);
                }

                if !filter.search.is_empty() {
                    let search = filter.search.to_lowercase();

                    let mut match_search = false;

                    match_search |= entry
                        .file_name
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&search);

                    if let Some(metadata) = &entry.metadata {
                        if let Some(title) = &metadata.title {
                            match_search |= title.to_lowercase().contains(&search);
                        }
                        if let Some(artist) = &metadata.artist {
                            match_search |= artist.to_lowercase().contains(&search);
                        }
                        if let Some(album) = &metadata.album {
                            match_search |= album.to_lowercase().contains(&search);
                        }
                    }

                    keep &= match_search;
                }

                keep
            })
            .map(|entry| entry.id)
            .collect::<Vec<_>>()
            .into()
    }

    /// Return all descendants of a folder, including itself.
    fn get_folder_descendants(&self, folder_id: FolderId) -> Vec<FolderId> {
        let mut descendants = vec![folder_id];

        for sub_folder in self.folders.get(&folder_id).unwrap().sub_folders.values() {
            descendants.extend(self.get_folder_descendants(*sub_folder));
        }

        descendants
    }

    /// Return all descendants of a tag, including itself.
    fn get_tag_descendants(&self, tag_id: TagId) -> Vec<TagId> {
        let mut descendants = vec![tag_id];

        for child_tag in &self.tags.get(&tag_id).unwrap().children {
            descendants.extend(self.get_tag_descendants(*child_tag));
        }

        descendants
    }
}
