use super::{is_audio_file, is_hidden_file, Database, DatabaseData, Result};

use log::{debug, trace, warn};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex, RwLock};
use std::thread::spawn;
use std::time::Duration;

use notify_debouncer_full::new_debouncer;
pub use notify_debouncer_full::notify;
use notify_debouncer_full::notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};
use notify_debouncer_full::notify::EventKind::{Create, Modify, Remove};
use notify_debouncer_full::DebouncedEvent;
use rusqlite::Connection;
use thiserror::Error;

#[derive(Error, Debug)]
enum FileWatcherError {
    #[error("Entry not found for path: {0}")]
    EntryNotFound(PathBuf),
    #[error("Folder not found for path: {0}")]
    FolderNotFound(PathBuf),
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Database(#[from] super::Error),
}

type FileWatcherResult<T> = std::result::Result<T, FileWatcherError>;

impl Database {
    pub fn watch_dir(&self) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = new_debouncer(Duration::from_secs(1), None, tx)?;
        watcher.watch(
            &self.data.read().unwrap().base_path,
            notify::RecursiveMode::Recursive,
        )?;
        debug!("watch directory: {:?}", self.data.read().unwrap().base_path);

        let data = self.data.clone();
        let db = self.db.clone();
        let emitter = self.emitter.clone();

        spawn(move || {
            debug!("start directory watcher thread");

            // keep watcher alive
            let _watcher = watcher;

            for result in rx {
                let events = match result {
                    Ok(events) => events,
                    Err(err) => {
                        warn!("Error watching directory: {err:?}");
                        continue;
                    }
                };

                let mut updated = false;

                for event in events {
                    if event.need_rescan() {
                        debug!("Rescanning directory");

                        data.write()
                            .unwrap()
                            .scan(&mut db.lock().unwrap())
                            .unwrap_or_else(|err| {
                                warn!("Failed to scan directory: {err:?}");
                            });

                        break; // skip the rest of the events
                    }

                    trace!("File event: {event:?}");

                    updated |= handle_file_event(&event, &data, &db).unwrap_or_else(|err| {
                        warn!("Failed to process file change event: {err:?}");
                        false
                    });
                }

                if updated {
                    emitter.on_files_updated();
                }
            }

            debug!("stop directory watcher thread");
        });

        Ok(())
    }
}

fn handle_file_event(
    event: &DebouncedEvent,
    data: &RwLock<DatabaseData>,
    db: &Mutex<Connection>,
) -> FileWatcherResult<bool> {
    match event.kind {
        Create(CreateKind::Folder) => {
            debug!("Folder created: {event:?}");

            // Create(Folder) event is untrushworthy, rescan the folder
            data.write()
                .unwrap()
                .scan_folders(&mut db.lock().unwrap())?;
            Ok(true)
        }

        Create(CreateKind::File) => {
            let path = &event.paths[0];
            if is_hidden_file(path) || !is_audio_file(path) {
                return Ok(false);
            }

            debug!("File created: {event:?}");
            file_created(
                &mut data.write().unwrap(),
                path.as_path(),
                &db.lock().unwrap(),
            )?;
            Ok(true)
        }

        Remove(RemoveKind::Folder) => {
            let path = &event.paths[0];
            if is_hidden_file(path) {
                return Ok(false);
            }

            debug!("Folder removed: {event:?}");
            folder_removed(&mut data.write().unwrap(), path.as_path(), db)?;
            Ok(true)
        }

        Remove(RemoveKind::File) => {
            let path = &event.paths[0];
            if is_hidden_file(path) || !is_audio_file(path) {
                return Ok(false);
            }

            debug!("File removed: {event:?}");
            file_removed(&mut data.write().unwrap(), path.as_path(), db)?;
            Ok(true)
        }

        Modify(ModifyKind::Name(RenameMode::Both)) => {
            let old_path = &event.paths[0];
            let new_path = &event.paths[1];

            if new_path.is_dir() {
                debug!("Folder moved: {event:?}");
                folder_moved(&mut data.write().unwrap(), old_path, new_path, db)?;
                Ok(true)
            } else if new_path.is_file() {
                match (
                    !is_hidden_file(old_path) && is_audio_file(old_path),
                    !is_hidden_file(new_path) && is_audio_file(new_path),
                ) {
                    (true, true) => {
                        // audio -> audio
                        debug!("File moved: {event:?}");
                        file_moved(&mut data.write().unwrap(), old_path, new_path, db)?;
                        Ok(true)
                    }

                    (false, true) => {
                        // non-audio -> audio
                        debug!("File moved: {event:?}");
                        file_created(
                            &mut data.write().unwrap(),
                            new_path.as_path(),
                            &db.lock().unwrap(),
                        )?;
                        Ok(true)
                    }

                    (true, false) => {
                        // audio -> non-audio
                        debug!("File moved: {event:?}");
                        file_removed(&mut data.write().unwrap(), old_path.as_path(), db)?;
                        Ok(true)
                    }

                    _ => Ok(false), // non-audio -> non-audio
                }
            } else {
                panic!("Unexpected file type for path {new_path:?}");
            }
        }

        Modify(ModifyKind::Name(RenameMode::Any)) => {
            // move in or move out of the watching folder will trigger this event
            let path = &event.paths[0];
            debug!("File or folder moved: {event:?}");
            file_or_folder_updated(
                &mut data.write().unwrap(),
                path.as_path(),
                &mut db.lock().unwrap(),
            )?;
            Ok(true)
        }

        Modify(_) => {
            let path = &event.paths[0];

            if !path.is_file() || is_hidden_file(path) || !is_audio_file(path) {
                return Ok(false);
            }

            debug!("File modified: {event:?}");
            file_created(
                &mut data.write().unwrap(),
                path.as_path(),
                &db.lock().unwrap(),
            )?;
            Ok(true)
        }

        _ => Ok(false),
    }
}

fn file_or_folder_updated(
    data: &mut DatabaseData,
    path: &Path,
    db: &mut Connection,
) -> FileWatcherResult<bool> {
    let relative_path = data.to_relative_path(path);

    if is_hidden_file(path) {
        return Ok(false);
    }

    if path.try_exists()? {
        if path.is_dir() {
            // directory exists, try to add folder
            data.add_folders(vec![relative_path], db);
            Ok(true)
        } else if path.is_file() {
            // file exists, try to add entry
            if !is_audio_file(path) {
                return Ok(false);
            }
            file_created(data, path, db)?;
            Ok(true)
        } else {
            panic!("Unexpected file type for path {path:?}");
        }
    } else {
        // file does not exists, try to remove it
        if let Some(entry_id) = data.get_entry_id(&relative_path) {
            // entry exists, remove it
            data.remove_entry(entry_id, db)?;
            Ok(true)
        } else if let Some(folder) = data.get_folder_by_path(&relative_path) {
            // entry does not exist, try to remove folder
            data.remove_folders(vec![folder.id], db);
            Ok(true)
        } else {
            // entry and folder do not exist, do nothing
            Ok(false)
        }
    }
}

fn file_created(data: &mut DatabaseData, path: &Path, db: &Connection) -> FileWatcherResult<()> {
    debug_assert!(path.is_file());
    debug_assert!(path.is_absolute());
    let relative_path = data.to_relative_path(path);

    match data.get_entry_id(&relative_path) {
        // entry does not exist, add it
        None => Ok(data.add_entries(&[relative_path], db)?),

        // entry already exists, reread it
        Some(entry_id) => {
            data.entries
                .get_mut(&entry_id)
                .expect("Entry not found")
                .read_file(&data.base_path);
            Ok(())
        }
    }
}

fn file_removed(
    data: &mut DatabaseData,
    path: &Path,
    db: &Mutex<Connection>,
) -> FileWatcherResult<()> {
    debug_assert!(path.is_file());
    debug_assert!(path.is_absolute());
    let relative_path = data.to_relative_path(path);

    let entry_id = data
        .get_entry_id(&relative_path)
        .ok_or_else(|| FileWatcherError::EntryNotFound(relative_path))?;
    data.remove_entry(entry_id, &db.lock().unwrap())?;
    Ok(())
}

fn folder_removed(
    data: &mut DatabaseData,
    path: &Path,
    db: &Mutex<Connection>,
) -> FileWatcherResult<()> {
    debug_assert!(path.is_dir());
    debug_assert!(path.is_absolute());
    let relative_path = data.to_relative_path(path);

    let folder_id = data
        .get_folder_by_path(&relative_path)
        .ok_or_else(|| FileWatcherError::FolderNotFound(relative_path))?
        .id;
    data.remove_folders(vec![folder_id], &mut db.lock().unwrap());
    Ok(())
}

fn file_moved(
    data: &mut DatabaseData,
    old_path: &Path,
    new_path: &Path,
    db: &Mutex<Connection>,
) -> FileWatcherResult<()> {
    debug_assert!(new_path.is_file());
    let relative_old_path = data.to_relative_path(old_path);
    let relative_new_path = data.to_relative_path(new_path);

    if let Some(entry_id) = data.get_entry_id(&relative_old_path) {
        // if old entry exists, change its path
        Ok(data.move_entry(entry_id, relative_new_path, &db.lock().unwrap())?)
    } else {
        // if old entry does not exist, add new entry
        Ok(data.add_entries(&[relative_new_path], &db.lock().unwrap())?)
    }
}

fn folder_moved(
    data: &mut DatabaseData,
    old_path: &Path,
    new_path: &Path,
    db: &Mutex<Connection>,
) -> FileWatcherResult<()> {
    let relative_old_path = data.to_relative_path(old_path);
    let relative_new_path = data.to_relative_path(new_path);
    Ok(data.move_folder(
        &relative_old_path,
        &relative_new_path,
        &mut db.lock().unwrap(),
    )?)
}
