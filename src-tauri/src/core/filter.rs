use super::Database;

use std::collections::HashSet;
use std::iter::Iterator;

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    pub search: String,
    pub tag_ids: Vec<i32>,
    pub folder_path: String,
}

impl Database {
    pub fn filter(&self, filter: Filter) -> Option<Vec<i32>> {
        if filter.search.is_empty() && filter.tag_ids.is_empty() && filter.folder_path.is_empty() {
            return None;
        }

        self.get_entries()
            .values()
            .filter_map(|entry| {
                let mut keep = true;

                if !filter.search.is_empty() {
                    let search = filter.search.to_lowercase();

                    let mut match_search = false;

                    match_search |= entry.file_name.to_lowercase().contains(&search);

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
                };

                if !filter.folder_path.is_empty() {
                    keep &= entry
                        .path
                        .to_str()
                        .unwrap()
                        .starts_with(&filter.folder_path);
                };

                if !filter.tag_ids.is_empty() {
                    keep &= entry
                        .tag_ids
                        .is_superset(&HashSet::from_iter(filter.tag_ids.iter().cloned()));
                };

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
