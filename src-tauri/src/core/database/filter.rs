use super::{DatabaseData, EntryId, FolderId, TagId};

use std::collections::HashSet;
use std::iter::Iterator;

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    pub search: String,
    pub tag_ids: Vec<TagId>,
    pub folder_id: Option<FolderId>,
}

impl DatabaseData {
    pub fn filter(&self, filter: &Filter) -> Option<Vec<EntryId>> {
        if filter.search.is_empty() && filter.tag_ids.is_empty() && filter.folder_id.is_none() {
            return None;
        }

        let folder_ids = filter
            .folder_id
            .map(|folder_id| self.get_folder_descendants(folder_id));

        self.get_entries()
            .values()
            .filter(|entry| {
                let mut keep = true;

                if let Some(folder_ids) = &folder_ids {
                    keep &= folder_ids.contains(&entry.folder_id);
                }

                if !filter.tag_ids.is_empty() {
                    keep &= entry
                        .tag_ids
                        .is_superset(&filter.tag_ids.iter().copied().collect::<HashSet<_>>());
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
    fn get_folder_descendants(&self, folder_id: FolderId) -> HashSet<FolderId> {
        let mut descendants = HashSet::from([folder_id]);

        for sub_folder in self.folders.get(&folder_id).unwrap().sub_folders.values() {
            descendants.extend(self.get_folder_descendants(*sub_folder));
        }

        descendants
    }
}
