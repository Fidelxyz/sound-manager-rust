use crate::migrator_warn;

use super::{Migrator, MigratorResult};
use crate::core::{database::DatabaseEmitter, Database};

use std::collections::HashMap;
use std::path::{Path, PathBuf, MAIN_SEPARATOR_STR};

use rusqlite::{Connection, OpenFlags};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Database(#[from] crate::core::database::Error),
    #[error("Rusqlite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),
}

struct NullEmitter {}

impl DatabaseEmitter for NullEmitter {
    fn on_files_updated(&self) {}
}

struct Folder {
    pid: i32,
    name: String,
}

struct File {
    id: i32,
    name: String,
    pid: i32,
}

struct Tag {
    id: i32,
    name: String,
    pid: i32,
    color: i32,
}

pub struct BillfishMigrator {}

impl Migrator<Error> for BillfishMigrator {
    fn migrate(base_path: &Path, result: &mut MigratorResult) -> Result<(), Error> {
        let database = Database::create(base_path.into(), NullEmitter {})?;
        let mut data = database.data.write().unwrap();
        let mut db = database.db.lock().unwrap();

        // read from the billfish database
        let bf_db = Connection::open_with_flags(
            base_path.join(".bf/billfish.db"),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;

        let bf_folders = bf_db
            .prepare(
                "SELECT id, pid, name FROM bf_folder WHERE pid <> -1 AND hide = 0 AND is_recycle = 0",
            )?
            .query_map([], |row| {
                let id  = row.get(0)?;
                let pid  = row.get(1)?;
                let name = row.get(2)?;
               Ok((id, Folder {pid, name}))
            })?
            .filter_map(|folder| {
                folder.map_err(|err| {
                    migrator_warn!(result, "Failed to read folder record: {:?}", err);
                })
                .ok()
            })
            .collect::<HashMap<i32, Folder>>();

        let bf_files = bf_db
            .prepare("SELECT id, name, pid FROM bf_file WHERE pid <> -1 AND is_hide = 0")?
            .query_map([], |row| {
                Ok(File {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    pid: row.get(2)?,
                })
            })?
            .filter_map(|file| {
                file.map_err(|err| {
                    migrator_warn!(result, "Failed to read file record: {:?}", err);
                })
                .ok()
            })
            .collect::<Vec<File>>();

        let bf_tags = bf_db
            .prepare("SELECT id, name, pid, color FROM bf_tag_v2 ORDER BY seq")?
            .query_map([], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    pid: row.get(2)?,
                    color: row.get(3)?,
                })
            })?
            .filter_map(|tag| {
                tag.map_err(|err| {
                    migrator_warn!(result, "Failed to read tag record: {:?}", err);
                })
                .ok()
            })
            .collect::<Vec<Tag>>();

        let bf_file_to_bf_tag = bf_db
            .prepare("SELECT file_id, tag_id FROM bf_tag_join_file")?
            .query_map([], |row| {
                let file_id = row.get(0)?;
                let tag_id = row.get(1)?;
                Ok((file_id, tag_id))
            })?
            .filter_map(|tag| {
                tag.map_err(|err| {
                    migrator_warn!(result, "Failed to read file to tag record: {:?}", err);
                })
                .ok()
            })
            .collect::<Vec<(i32, i32)>>();

        // convert to the native data structure

        // parse the path for each folder
        let mut folder_id_to_path = HashMap::new();
        folder_id_to_path.insert(0, PathBuf::from(""));
        for (bf_folder_id, bf_folder) in &bf_folders {
            // iterate over parent folders to get the full path
            let mut path = vec![bf_folder.name.clone()];
            let mut id = bf_folder.pid;
            while id != 0 {
                let Some(folder) = bf_folders.get(&id) else {
                    break;
                };
                path.push(folder.name.clone());
                id = folder.pid;
            }

            if id != 0 {
                migrator_warn!(
                    result,
                    "Failed to parse path for folder {:?}: parent folder of id {} not found",
                    bf_folder.name,
                    id
                );
                continue;
            }

            path.reverse();
            let path = PathBuf::from(path.join(MAIN_SEPARATOR_STR));
            folder_id_to_path.insert(*bf_folder_id, path);
        }

        let mut new_tag_ids = HashMap::new();
        for bf_tag in &bf_tags {
            // create the tag in the new database
            let tag_id = match data.new_tag(bf_tag.name.clone(), &db) {
                Ok(tag_id) => tag_id,
                Err(err) => {
                    migrator_warn!(result, "Failed to create tag {:?}: {:?}", &bf_tag.name, err);
                    continue;
                }
            };
            new_tag_ids.insert(bf_tag.id, tag_id);

            // set the color of the tag
            if let Err(err) = data.set_tag_color(tag_id, bf_tag.color, &db) {
                migrator_warn!(
                    result,
                    "Failed to set color of tag {:?}: {:?}",
                    &bf_tag.name,
                    err
                );
            }
        }

        // set the parent tag for each tag
        for bf_tag in &bf_tags {
            // if the tag has no parent
            if bf_tag.pid == 0 {
                continue;
            }

            let Some(&tag_id) = new_tag_ids.get(&bf_tag.id) else {
                migrator_warn!(
                    result,
                    "Failed to set parent tag for tag {:?}: tag of id {} not found",
                    bf_tag.name,
                    bf_tag.id
                );
                continue;
            };
            let Some(&parent_tag_id) = new_tag_ids.get(&bf_tag.pid) else {
                migrator_warn!(
                    result,
                    "Failed to set parent tag for tag {:?}: parent tag of id {} not found",
                    bf_tag.name,
                    bf_tag.pid
                );
                continue;
            };

            if let Err(err) = data.reorder_tag(tag_id, parent_tag_id, -1, &mut db) {
                migrator_warn!(
                    result,
                    "Failed to set parent tag for tag {}: {:?}",
                    &bf_tag.name,
                    err
                );
            }
        }

        // get the new entry ids for each file
        let mut new_entry_ids = HashMap::new();
        for file in &bf_files {
            let Some(folder_path) = folder_id_to_path.get(&file.pid) else {
                migrator_warn!(
                    result,
                    "Failed to parse path for file {}: parent folder of id {} not found",
                    file.name,
                    file.pid
                );
                continue;
            };
            let file_path = folder_path.join(&file.name);

            let Some(entry_id) = data.get_entry_id(&file_path) else {
                migrator_warn!(result, "File of path {:?} not found", file_path);
                continue;
            };
            new_entry_ids.insert(file.id, entry_id);
        }

        // set tags for each file
        for (bf_file_id, bf_tag_id) in bf_file_to_bf_tag {
            let Some(&entry_id) = new_entry_ids.get(&bf_file_id) else {
                let file_name = match bf_files.iter().find(|f| f.id == bf_file_id) {
                    Some(file) => file.name.clone(),
                    None => String::from("unknown"),
                };
                migrator_warn!(
                    result,
                    "Failed to set tag for file {:?}: file of id {} not found",
                    file_name,
                    bf_file_id
                );
                continue;
            };
            let Some(&tag_id) = new_tag_ids.get(&bf_tag_id) else {
                let file_name = match bf_files.iter().find(|f| f.id == bf_file_id) {
                    Some(file) => file.name.clone(),
                    None => String::from("unknown"),
                };
                migrator_warn!(
                    result,
                    "Failed to set tag for file {:?}: tag of id {} not found",
                    file_name,
                    bf_tag_id
                );
                continue;
            };

            if let Err(err) = data.add_tag_for_entry(entry_id, tag_id, &db) {
                let file_name = match bf_files.iter().find(|f| f.id == bf_file_id) {
                    Some(file) => file.name.clone(),
                    None => String::from("unknown"),
                };
                migrator_warn!(
                    result,
                    "Failed to set tag for file {:?}: {:?}",
                    file_name,
                    err
                );
            }
        }

        Ok(())
    }
}
