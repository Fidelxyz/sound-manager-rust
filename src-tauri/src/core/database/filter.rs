use super::{DatabaseData, EntryId, TagId};

use std::collections::HashSet;
use std::iter::Iterator;

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    pub search: String,
    pub tag_ids: Vec<TagId>,
    pub folder_id: Option<i32>,
}

impl DatabaseData {
    pub fn filter(&self, filter: &Filter) -> Option<Vec<EntryId>> {
        if filter.search.is_empty() && filter.tag_ids.is_empty() && filter.folder_id.is_none() {
            return None;
        }

        self.get_entries()
            .values()
            .filter_map(|entry| {
                let mut keep = true;

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

                if let Some(folder_id) = filter.folder_id {
                    keep &= entry.folder_id == folder_id;
                }

                if !filter.tag_ids.is_empty() {
                    keep &= entry
                        .tag_ids
                        .is_superset(&filter.tag_ids.iter().copied().collect::<HashSet<_>>());
                }

                if keep {
                    Some(entry.id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .into()
    }
}
