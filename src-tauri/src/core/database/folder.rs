use super::Result;

use log::{info, warn};
use std::collections::HashMap;
use std::iter::Peekable;
use std::path::{Components, Path};

pub struct Folder {
    pub path: Box<Path>,
    pub name: String,
    pub sub_folders: HashMap<String, Folder>,
}

impl Folder {
    pub fn new(name: String, path: Box<Path>) -> Self {
        debug_assert!(path.is_relative(), "Path must be relative");

        Self {
            path,
            name,
            sub_folders: HashMap::new(),
        }
    }

    pub fn scan_folder(&mut self, base_path: &Path) -> Result<()> {
        let mut existing_sub_folders = Vec::new();
        let mut new_sub_folders = Vec::new();

        for dir_entry in base_path.join(&self.path).read_dir()? {
            let dir_entry = match dir_entry {
                Ok(entry) => entry,
                Err(err) => {
                    warn!("Failed to read directory entry: {:?}", err);
                    continue;
                }
            };

            // Skip non-directory entries
            let file_type = match dir_entry.file_type() {
                Ok(file_type) => file_type,
                Err(err) => {
                    warn!("Failed to read file type: {:?}", err);
                    continue;
                }
            };
            if !file_type.is_dir() {
                continue;
            }

            let file_name = dir_entry.file_name();
            let file_name = file_name.to_string_lossy();

            // Skip hidden files
            if file_name.starts_with('.') {
                continue;
            }

            if self.sub_folders.contains_key(file_name.as_ref()) {
                existing_sub_folders.push(file_name.to_string());
            } else {
                new_sub_folders.push(file_name.to_string());
            }
        }

        // Remove sub folders that no longer exist
        self.sub_folders
            .retain(|name, _| existing_sub_folders.contains(name));

        // Add new sub folders
        self.sub_folders
            .extend(new_sub_folders.into_iter().map(|name| {
                let path = self.path.join(&name);
                let folder = Folder::new(name.to_string(), path.into());
                (name.to_string(), folder)
            }));

        // Recursively scan sub folders
        for sub_folder in self.sub_folders.values_mut() {
            sub_folder.scan_folder(base_path).unwrap_or_else(|err| {
                warn!(
                    "Failed to read directory {:?}: {:?}",
                    self.path.join(&sub_folder.path),
                    err
                );
            })
        }

        Ok(())
    }

    pub fn insert_sub_folder(
        &mut self,
        mut path_components: Peekable<Components>,
        folder: Folder,
    ) -> Result<()> {
        let sub_folder_name = path_components
            .next()
            .unwrap()
            .as_os_str()
            .to_string_lossy();

        // If this is the last component, insert the folder
        if path_components.peek().is_none() {
            // If the folder already exists
            if self.sub_folders.contains_key(sub_folder_name.as_ref()) {
                warn!(
                    "Folder {:?} already exists when inserting a new one",
                    self.path.join(sub_folder_name.as_ref())
                );
            }

            self.sub_folders.insert(sub_folder_name.to_string(), folder);
            return Ok(());
        }

        // If this is not the last component, recursively call insert_sub_folder
        let sub_folder = match self.sub_folders.get_mut(sub_folder_name.as_ref()) {
            Some(sub_folder) => sub_folder,
            None => {
                self.sub_folders.insert(
                    sub_folder_name.to_string(),
                    Folder::new(
                        sub_folder_name.to_string(),
                        self.path.join(sub_folder_name.as_ref()).into(),
                    ),
                );
                info!(
                    "Added folder {:?}",
                    self.path.join(sub_folder_name.as_ref())
                );
                self.sub_folders.get_mut(sub_folder_name.as_ref()).unwrap()
            }
        };
        sub_folder.insert_sub_folder(path_components, folder)
    }

    pub fn remove_sub_folder(
        &mut self,
        mut path_components: Peekable<Components>,
    ) -> Option<Folder> {
        let sub_folder_name = path_components
            .next()
            .unwrap()
            .as_os_str()
            .to_string_lossy();

        // If this is the last component, remove the folder
        if path_components.peek().is_none() {
            let removed = self.sub_folders.remove(sub_folder_name.as_ref());
            if removed.is_none() {
                warn!(
                    "Folder {:?} to remove does not exist",
                    self.path.join(sub_folder_name.as_ref())
                );
            }
            return removed;
        }

        // If this is not the last component, recursively call remove_sub_folder
        if let Some(sub_folder) = self.sub_folders.get_mut(sub_folder_name.as_ref()) {
            sub_folder.remove_sub_folder(path_components)
        } else {
            warn!(
                "Folder {:?} does not exist when removing its child",
                self.path.join(sub_folder_name.as_ref()),
            );
            None
        }
    }
}
