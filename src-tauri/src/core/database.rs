mod entry;
mod file_watcher;
mod files;
mod filter;
mod folder;
mod tag;

#[cfg(test)]
mod tests;

pub use entry::{Entry, EntryId};
pub use filter::Filter;
pub use folder::{Folder, FolderId, FolderNode};
pub use tag::{Tag, TagId, TagNode};

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use crossbeam_channel;
use log::{info, trace, warn};
use rusqlite::{Connection, OptionalExtension};
use thiserror::Error;

pub struct Database<E> {
    // Two seperate locks are required
    // due to the need of the interior mutability of data and db seperately
    pub data: RwLock<DatabaseData>,
    pub db: Mutex<Connection>,

    emitter: E,
    stop_tx: crossbeam_channel::Sender<()>,
}

pub struct DatabaseData {
    base_path: PathBuf,
    folders: HashMap<FolderId, Folder>,
    entries: HashMap<EntryId, Entry>,
    tags: HashMap<TagId, Tag>,
}

struct FileDiff {
    pub new_folders: Vec<PathBuf>,
    pub deleted_folders: Vec<FolderId>,
    pub new_entries: Vec<PathBuf>,
    pub deleted_entries: Vec<EntryId>,
}

struct FolderDiff {
    pub existing_folders: Vec<FolderId>,
    pub new_folders: Vec<PathBuf>,
}

const ROOT_FOLDER_ID: FolderId = -1;
const ROOT_TAG_ID: TagId = -1;

pub const SQLITE_DB_PATH: &str = ".soundmanager.db";
const DATABASE_VERSION: i32 = 1;

#[derive(Error, Debug)]
pub enum Error {
    #[error("database not found: {0}")]
    DatabaseNotFound(String),
    #[error("database already exists: {0}")]
    DatabaseAlreadyExists(String),
    #[error("tag already exists: {0}")]
    TagAlreadyExists(String),
    #[error("tag {0} already exists for entry {1}")]
    TagAlreadyExistsForEntry(TagId, EntryId),
    #[error("file already exists: {0}")]
    FileAlreadyExists(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Rusqlite(#[from] rusqlite::Error),
    #[error("symphonia error: {0}")]
    Symphonia(#[from] symphonia::core::errors::Error),
    #[error("notify error: {0}")]
    Notify(#[from] notify_debouncer_full::notify::Error),
    #[error("trash error: {0}")]
    Trash(#[from] trash::Error),
}

type Result<T> = std::result::Result<T, Error>;

pub trait DatabaseEmitter {
    fn on_files_updated(&self, immediate: bool);
}

impl<E> Database<E>
where
    E: DatabaseEmitter + Send + Sync + 'static,
{
    // ========== Constructor ==========

    pub fn open(base_path: PathBuf, emitter: E) -> Result<Arc<Self>> {
        info!("Opening database {base_path:?}");

        let database_file = base_path.join(SQLITE_DB_PATH);
        if !database_file.try_exists()? {
            // if database file does not exist
            return Err(Error::DatabaseNotFound(
                base_path.to_string_lossy().to_string(),
            ));
        }

        let db = Connection::open(database_file)?;

        let tags = Self::read_tags(&db)?;
        let folders = HashMap::from([(
            ROOT_FOLDER_ID,
            Folder::new(
                ROOT_FOLDER_ID,
                ROOT_FOLDER_ID,
                base_path.file_name().unwrap().into(),
                PathBuf::new(),
            ),
        )]);

        let (stop_tx, stop_rx) = crossbeam_channel::bounded(1);

        let database = Self {
            data: RwLock::new(DatabaseData {
                base_path,
                folders,
                entries: HashMap::new(),
                tags,
            }),
            db: Mutex::new(db),
            emitter,
            stop_tx,
        };

        database
            .data
            .write()
            .unwrap()
            .scan(&mut database.db.lock().unwrap())?;

        let database = Arc::new(database);
        database.prune()?;
        database.clone().watch_dir(stop_rx)?;

        Ok(database)
    }

    pub fn create(base_path: PathBuf, emitter: E) -> Result<Arc<Self>> {
        info!("Creating database {base_path:?}");

        let database_file = base_path.join(SQLITE_DB_PATH);
        if database_file.try_exists()? {
            // if database file already exists
            return Err(Error::DatabaseAlreadyExists(
                base_path.to_string_lossy().to_string(),
            ));
        }

        let mut db = Connection::open(database_file)?;
        let tx = db.transaction()?;
        tx.execute(
            "CREATE TABLE metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                version INTEGER NOT NULL
            )",
            (),
        )?;
        tx.execute(
            "INSERT INTO metadata (version) VALUES (?)",
            [DATABASE_VERSION],
        )?;
        tx.execute_batch(
            "CREATE TABLE entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_name TEXT NOT NULL,
                folder_id INTEGER NOT NULL,
                deleted DATETIME DEFAULT NULL,
                FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE CASCADE,
                UNIQUE (folder_id, file_name)
            );
            CREATE TABLE folders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                parent INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                deleted DATETIME DEFAULT NULL,
                UNIQUE (parent, name)
            );
            CREATE TABLE tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                parent INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                position INTEGER NOT NULL,
                color INTEGER DEFAULT 0,
                deleted DATETIME DEFAULT NULL
            );
            CREATE TABLE entry_tag (
                entry_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (entry_id, tag_id),
                FOREIGN KEY (entry_id) REFERENCES entries(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );",
        )?;
        tx.execute(
            "INSERT INTO folders (id, name, parent) VALUES (?, ?, ?)",
            (ROOT_FOLDER_ID, "", ROOT_FOLDER_ID),
        )?;
        tx.execute(
            "INSERT INTO tags (id, name, parent, position) VALUES (?, ?, ?, ?)",
            (ROOT_TAG_ID, "", ROOT_TAG_ID, 0),
        )?;
        tx.commit()?;

        let folders = HashMap::from([(
            ROOT_FOLDER_ID,
            Folder::new(
                ROOT_FOLDER_ID,
                ROOT_FOLDER_ID,
                base_path.file_name().unwrap().into(),
                PathBuf::new(),
            ),
        )]);
        let tags = HashMap::from([(
            ROOT_TAG_ID,
            Tag {
                id: ROOT_TAG_ID,
                name: String::new(),
                parent_id: ROOT_TAG_ID,
                position: 0,
                color: 0,
                children: HashSet::new(),
            },
        )]);

        let (stop_tx, stop_rx) = crossbeam_channel::bounded(1);

        let database = Self {
            data: RwLock::new(DatabaseData {
                base_path,
                folders,
                entries: HashMap::new(),
                tags,
            }),
            db: Mutex::new(db),
            emitter,
            stop_tx,
        };

        database
            .data
            .write()
            .unwrap()
            .scan(&mut database.db.lock().unwrap())?;

        let database = Arc::new(database);
        database.prune()?;
        database.clone().watch_dir(stop_rx)?;

        Ok(database)
    }

    pub fn refresh(&self) -> Result<()> {
        self.data
            .write()
            .unwrap()
            .scan(&mut self.db.lock().unwrap())?;
        self.emitter.on_files_updated(true);
        Ok(())
    }

    pub fn prune(&self) -> Result<()> {
        self.db.lock().unwrap().execute_batch(
            "DELETE FROM entries
            WHERE deleted IS NOT NULL
            AND deleted < datetime('now', '-30 days');
            DELETE FROM folders
            WHERE deleted IS NOT NULL
            AND deleted < datetime('now', '-30 days');
            DELETE FROM tags
            WHERE deleted IS NOT NULL
            AND deleted < datetime('now', '-30 days');",
        )?;
        Ok(())
    }
}

impl<E> Database<E> {
    pub fn close(&self) {
        let _ = self.stop_tx.send(());
    }

    fn read_tags(db: &Connection) -> Result<HashMap<TagId, Tag>> {
        let mut tags = db
            .prepare("SELECT id, name, parent, position, color FROM tags WHERE deleted IS NULL")?
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    Tag {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        parent_id: row.get(2)?,
                        position: row.get(3)?,
                        color: row.get(4)?,
                        children: HashSet::new(),
                    },
                ))
            })?
            .collect::<std::result::Result<HashMap<_, _>, _>>()?;

        // build tree relationship
        let tag_parent = tags
            .iter()
            .map(|(id, tag)| (*id, tag.parent_id))
            .collect::<Vec<_>>();
        for (id, parent_id) in tag_parent {
            // skip root tag
            if id == parent_id {
                continue;
            }

            tags.get_mut(&parent_id).unwrap().children.insert(id);
        }

        Ok(tags)
    }
}

impl<E> Drop for Database<E> {
    fn drop(&mut self) {
        info!("database closed");
    }
}

impl DatabaseData {
    fn to_relative_path(&self, path: &Path) -> PathBuf {
        debug_assert!(path.is_absolute());
        let relative_path = path.strip_prefix(&self.base_path).unwrap();
        relative_path.into()
    }

    fn to_absolute_path(&self, path: &Path) -> PathBuf {
        debug_assert!(path.is_relative());
        self.base_path.join(path)
    }

    /// Read the directory to get the changes in file system compared to the data in memory.
    ///
    /// # Arguments
    ///
    /// * `path` - The absolute path to the directory to read.
    /// * `folder_id` - The current folder ID to read from.
    fn read_dir(&self, path: &Path, folder_id: Option<FolderId>) -> Result<FileDiff> {
        debug_assert!(path.is_absolute(), "Path must be absolute");

        info!("Reading directory: {path:?}");

        let mut new_folders = Vec::new();
        let mut deleted_folders = Vec::new();
        let mut new_entries = Vec::new();
        let mut deleted_entries = Vec::new();

        let mut existing_folders = HashSet::new();
        let mut existing_entries = HashSet::new();

        let folder = folder_id.map(|folder_id| self.folders.get(&folder_id).unwrap());

        for dir_entry in path.read_dir()? {
            let dir_entry = match dir_entry {
                Ok(dir_entry) => dir_entry,
                Err(err) => {
                    warn!("Failed to read directory entry: {err}");
                    continue;
                }
            };

            let file_name = dir_entry.file_name();

            // Skip hidden files and folders
            if is_hidden_file(file_name.as_ref()) {
                continue;
            }

            let file_type = match dir_entry.file_type() {
                Ok(file_type) => file_type,
                Err(err) => {
                    warn!("Failed to read file type: {err}");
                    continue;
                }
            };

            if file_type.is_dir() {
                let sub_folder_path = path.join(&file_name);
                let sub_folder_id =
                    folder.and_then(|folder| folder.sub_folders.get(&file_name).copied());

                // Add folder to the result
                if let Some(folder_id) = sub_folder_id {
                    existing_folders.insert(folder_id);
                } else {
                    new_folders.push(self.to_relative_path(&sub_folder_path));
                }

                // Read the sub-folder
                match self.read_dir(&sub_folder_path, sub_folder_id) {
                    Ok(diff) => {
                        // Merge results from sub-folder
                        new_folders.extend(diff.new_folders);
                        deleted_folders.extend(diff.deleted_folders);
                        new_entries.extend(diff.new_entries);
                        deleted_entries.extend(diff.deleted_entries);
                    }
                    Err(err) => warn!("Failed to read directory: {err}"),
                }
            } else if file_type.is_file() && is_audio_file(file_name.as_ref()) {
                let entry_id = folder.and_then(|folder| folder.entries.get(&file_name).copied());

                // Add file to the result
                if let Some(entry_id) = entry_id {
                    existing_entries.insert(entry_id);
                } else {
                    new_entries.push(self.to_relative_path(&path.join(file_name)));
                }
            } else {
                warn!(
                    "Failed to read file for path {:?}: unknown directory entry type.",
                    dir_entry.path()
                );
            }
        }

        if let Some(folder) = folder {
            deleted_folders.extend(
                folder
                    .sub_folders
                    .values()
                    .filter(|sub_folder_id| !existing_folders.contains(sub_folder_id))
                    .copied(),
            );
            deleted_entries.extend(
                folder
                    .entries
                    .values()
                    .filter(|entry_id| !existing_entries.contains(entry_id))
                    .copied(),
            );
        }

        Ok(FileDiff {
            new_folders,
            deleted_folders,
            new_entries,
            deleted_entries,
        })
    }

    /// Scan the entire directory of the database.
    fn scan(&mut self, db: &mut Connection) -> Result<()> {
        let diff = self.read_dir(&self.base_path, ROOT_FOLDER_ID.into())?;

        self.sync_changes(diff, db)?;
        self.sync_deleted(db)?;

        Ok(())
    }

    /// Scan a specific directory in the database.
    fn scan_dir(&mut self, path: &Path, db: &mut Connection) -> Result<()> {
        let folder_id = self.get_folder_by_path(path).map(|folder| folder.id);
        let diff = self.read_dir(path, folder_id)?;

        self.sync_changes(diff, db)?;

        Ok(())
    }

    /// Read the directory to get the changes in folders only compared to the data in memory.
    ///
    /// # Arguments
    ///
    /// * `path` - The absolute path to the directory to read.
    /// * `folder_id` - The current folder ID to read from.
    fn read_dir_folders(&self, path: &Path, folder_id: Option<FolderId>) -> Result<FolderDiff> {
        debug_assert!(path.is_absolute(), "Path must be absolute");

        info!("Reading directory: {:?}", self.to_absolute_path(path));

        let mut existing_folders = Vec::new();
        let mut new_folders = Vec::new();

        let folder = folder_id.map(|folder_id| self.folders.get(&folder_id).unwrap());

        for dir_entry in self.to_absolute_path(path).read_dir()? {
            let dir_entry = match dir_entry {
                Ok(dir_entry) => dir_entry,
                Err(err) => {
                    warn!("Failed to read directory entry: {err}");
                    continue;
                }
            };

            let file_name = dir_entry.file_name();

            // Skip hidden files and folders
            if is_hidden_file(file_name.as_ref()) {
                continue;
            }

            let file_type = match dir_entry.file_type() {
                Ok(file_type) => file_type,
                Err(err) => {
                    warn!("Failed to read file type: {err}");
                    continue;
                }
            };

            if file_type.is_dir() {
                let sub_folder_path = path.join(&file_name);
                let sub_folder_id =
                    folder.and_then(|folder| folder.sub_folders.get(&file_name).copied());
                // Add sub-folder to the result
                match sub_folder_id {
                    Some(sub_folder_id) => existing_folders.push(sub_folder_id),
                    None => new_folders.push(sub_folder_path.clone()),
                }
                // Read the sub-folder
                match self.read_dir_folders(&sub_folder_path, sub_folder_id) {
                    Ok(diff) => {
                        // Merge results from sub-folder
                        existing_folders.extend(diff.existing_folders);
                        new_folders.extend(diff.new_folders);
                    }
                    Err(err) => warn!("Failed to read directory: {err}"),
                }
            }
        }

        Ok(FolderDiff {
            existing_folders,
            new_folders,
        })
    }

    /// Scan only the folders (excluding files) in the database.
    fn scan_folders(&mut self, db: &mut Connection) -> Result<()> {
        let diff = self.read_dir_folders(&self.base_path, ROOT_FOLDER_ID.into())?;

        // remove deleted folders
        let existing_folders = diff.existing_folders.into_iter().collect::<HashSet<_>>();
        let removed_folders = self
            .folders
            .keys()
            .filter(|folder_id| !existing_folders.contains(folder_id))
            .copied()
            .collect::<Vec<_>>();
        self.remove_folders(removed_folders, db);

        // read new entries
        self.add_folders(&diff.new_folders, db)?;

        Ok(())
    }

    fn sync_changes(&mut self, diff: FileDiff, db: &mut Connection) -> Result<()> {
        // remove deleted entries and folders
        self.remove_entries(diff.deleted_entries, db)?;
        self.remove_folders(diff.deleted_folders, db);

        // update existing entries metadata
        for entry in self.entries.values_mut() {
            entry.read_file(&self.base_path);
        }

        // read new entries and folders
        self.add_folders(&diff.new_folders, db)?;
        self.add_entries(&diff.new_entries, db)?;

        Ok(())
    }

    /// Sync deleted entries, folders and tags in the database (e.g. since last luanch).
    fn sync_deleted(&self, db: &Connection) -> Result<()> {
        // mark deleted entries
        let query_entries = db
            .prepare("SELECT id FROM entries WHERE deleted IS NULL")?
            .query_map([], |row| row.get::<_, EntryId>(0))?
            .collect::<std::result::Result<HashSet<EntryId>, _>>()?;
        let existing_entries = self.entries.keys().copied().collect();
        let deleted_entries = query_entries
            .difference(&existing_entries)
            .collect::<Vec<_>>();

        info!("Deleting entries: {deleted_entries:?}");

        let mut stmt_delete =
            db.prepare("UPDATE entries SET deleted = datetime('now') WHERE id = ?")?;
        for entry_id in deleted_entries {
            stmt_delete.execute([entry_id])?;
        }

        // mark deleted folders
        let query_folders = db
            .prepare("SELECT id FROM folders WHERE deleted IS NULL")?
            .query_map([], |row| row.get::<_, FolderId>(0))?
            .collect::<std::result::Result<HashSet<FolderId>, _>>()?;
        let existing_folders = self.folders.keys().copied().collect();
        let deleted_folders = query_folders
            .difference(&existing_folders)
            .collect::<Vec<_>>();

        info!("Deleting folders: {deleted_folders:?}");

        let mut stmt_delete =
            db.prepare("UPDATE folders SET deleted = datetime('now') WHERE id = ?")?;
        for folder_id in deleted_folders {
            stmt_delete.execute([folder_id])?;
        }

        Ok(())
    }

    // ========== Folder ==========

    pub fn get_folders(&self) -> FolderNode {
        FolderNode::build(&self.folders)
    }

    fn get_folder_by_path(&self, path: &Path) -> Option<&Folder> {
        let mut folder = self.folders.get(&ROOT_FOLDER_ID);
        for folder_name in path.components() {
            let prev_folder = folder?;

            let sub_folder = prev_folder
                .sub_folders
                .get(folder_name.as_os_str())
                .and_then(|sub_folder_id| self.folders.get(sub_folder_id));

            folder = sub_folder;
        }
        folder
    }

    fn add_folders(&mut self, paths: &[PathBuf], db: &Connection) -> Result<()> {
        info!("Adding folders: {paths:#?}");

        debug_assert!(
            paths.iter().all(|path| path.is_relative()),
            "All paths must be relative"
        );

        // use different strategy for different number of new folders
        if paths.len() * 4 < self.folders.len() {
            // small number of new folders
            trace!("add_folders: small number of new folders: {}", paths.len());
            self.add_folders_serial(paths, db)?;
        } else {
            // large number of new folders
            trace!("add_folders: large number of new folders: {}", paths.len());
            self.add_folders_batch(paths, db)?;
        }

        Ok(())
    }

    fn add_folders_serial(&mut self, paths: &[PathBuf], db: &Connection) -> Result<()> {
        for path in paths {
            let mut stmt_select =
                db.prepare("SELECT id, deleted FROM folders WHERE parent = ? AND name = ?")?;
            let mut stmt_insert = db.prepare("INSERT INTO folders (parent, name) VALUES (?, ?)")?;
            let mut stmt_restore = db.prepare("UPDATE folders SET deleted = NULL WHERE id = ?")?;

            let path_components = path.components();
            let mut folder_id = ROOT_FOLDER_ID;
            for component in path_components {
                let folder = self.folders.get(&folder_id).unwrap();

                let sub_folder_name = component.as_os_str();
                let sub_folder_id = match folder.sub_folders.get(sub_folder_name).copied() {
                    Some(sub_folder_id) => sub_folder_id,

                    // folder does not exist, create new folder
                    None => {
                        let query_row = stmt_select
                            .query_row((folder_id, sub_folder_name.to_string_lossy()), |row| {
                                Ok((
                                    row.get::<_, FolderId>(0)?,                 // id
                                    row.get::<_, Option<String>>(1)?.is_some(), // deleted
                                ))
                            })
                            .optional()?;

                        // query folder id from or insert new folder into database
                        let new_folder_id = if let Some((id, deleted)) = query_row {
                            // folder exists in database
                            if deleted {
                                // folder is deleted, restore it
                                stmt_restore.execute([id])?;
                            }
                            id
                        } else {
                            // folder does not exist in database, insert a new one
                            stmt_insert
                                .insert((folder_id, sub_folder_name.to_string_lossy()))?
                                .try_into()
                                .unwrap()
                        };

                        let new_folder = Folder::new(
                            new_folder_id,
                            folder_id,
                            sub_folder_name.to_owned(),
                            folder.path.join(sub_folder_name),
                        );

                        // Add folder to the data in memory
                        self.folders
                            .get_mut(&folder_id)
                            .unwrap()
                            .sub_folders
                            .insert(sub_folder_name.into(), new_folder_id);
                        self.folders.insert(new_folder_id, new_folder);

                        new_folder_id
                    }
                };

                folder_id = sub_folder_id;
            }
        }

        Ok(())
    }

    fn add_folders_batch(&mut self, paths: &[PathBuf], db: &Connection) -> Result<()> {
        // query all folder ids from database in one batch
        // and store them into a path - folder_id map
        let query_rows = db
            .prepare(
                format!(
                    "SELECT id, parent, name, deleted FROM folders WHERE id <> {ROOT_FOLDER_ID}"
                )
                .as_str(),
            )?
            .query_map([], |row| {
                let folder_id = row.get::<_, FolderId>(0)?;
                let parent_id = row.get::<_, FolderId>(1)?;
                let name = row.get::<_, String>(2)?;
                let deleted = row.get::<_, Option<String>>(3)?.is_some();
                Ok(((parent_id, name), (folder_id, deleted)))
            })?
            .filter_map(std::result::Result::ok)
            .collect::<HashMap<(FolderId, String), (FolderId, bool)>>();

        // match queried rows with folders and perform corresponding actions

        let mut stmt_insert = db.prepare("INSERT INTO folders (parent, name) VALUES (?, ?)")?;
        let mut stmt_restore = db.prepare("UPDATE folders SET deleted = NULL WHERE id = ?")?;

        for path in paths {
            let path_components = path.components();
            let mut folder_id = ROOT_FOLDER_ID;
            for component in path_components {
                let folder = self.folders.get(&folder_id).unwrap();

                let sub_folder_name = component.as_os_str();
                let sub_folder_id = match folder.sub_folders.get(sub_folder_name).copied() {
                    Some(sub_folder_id) => sub_folder_id,

                    // folder does not exist, create new folder
                    None => {
                        // find matching folder in queried rows
                        let new_folder_id = if let Some((id, deleted)) = query_rows
                            .get(&(folder_id, sub_folder_name.to_string_lossy().into()))
                            .copied()
                        {
                            // if folder already exists in database

                            if deleted {
                                // folder is deleted, restore it
                                stmt_restore.execute([id])?;
                            }

                            id
                        } else {
                            // if folder does not exist in database, insert folder and set id
                            stmt_insert
                                .insert((folder_id, sub_folder_name.to_string_lossy()))?
                                .try_into()
                                .unwrap()
                        };

                        let new_folder = Folder::new(
                            new_folder_id,
                            folder_id,
                            sub_folder_name.to_owned(),
                            folder.path.join(sub_folder_name),
                        );

                        // Add folder to the data in memory
                        self.folders
                            .get_mut(&folder_id)
                            .unwrap()
                            .sub_folders
                            .insert(sub_folder_name.into(), new_folder_id);
                        self.folders.insert(new_folder_id, new_folder);

                        new_folder_id
                    }
                };

                folder_id = sub_folder_id;
            }
        }

        Ok(())
    }

    /// Recursively remove a folder from the database.
    fn remove_folder(&mut self, folder_id: FolderId, db: &mut Connection) -> Result<()> {
        info!("Removing folder: {folder_id}");

        let folder = self.folders.get(&folder_id).unwrap();
        let sub_folders = folder.sub_folders.values().copied().collect::<Vec<_>>();
        let entries = folder.entries.values().copied().collect::<Vec<_>>();

        // remove sub-folders recursively
        for sub_folder_id in sub_folders {
            self.remove_folder(sub_folder_id, db)?;
        }

        // remove entries from the folder
        self.remove_entries(entries, db)?;

        // remove entries from the database
        db.execute(
            "UPDATE entries SET deleted = datetime('now') WHERE folder_id = ?",
            [folder_id],
        )?;

        // remove folder
        let folder = self.folders.remove(&folder_id).unwrap();

        // remove folder from its parent
        let parent = self.folders.get_mut(&folder.parent_id).unwrap();
        let removed = parent.sub_folders.remove(&folder.name);
        debug_assert!(removed.is_some());

        Ok(())
    }

    fn remove_folders(&mut self, folder_ids: Vec<FolderId>, db: &mut Connection) {
        info!("Removing folders: {folder_ids:?}");

        for folder_id in folder_ids {
            self.remove_folder(folder_id, db).unwrap_or_else(|err| {
                warn!("Failed to remove folder with ID {folder_id}: {err}");
            });
        }
    }

    fn move_folder(&mut self, old_path: &Path, new_path: &Path, db: &mut Connection) -> Result<()> {
        debug_assert!(old_path.is_relative(), "Path must be relative");
        debug_assert!(new_path.is_relative(), "Path must be relative");

        let folder = self.get_folder_by_path(old_path);

        if let Some(folder) = folder {
            // folder exists
            let folder_id = folder.id;

            // check existance of parents
            let old_parent_id = self.folders.get(&folder.parent_id).unwrap().id;
            let new_parent_id = self
                .get_folder_by_path(new_path.parent().unwrap())
                .unwrap()
                .id;

            // update folder in database
            let new_folder_name = new_path.file_name().unwrap();
            let new_folder_name_str = new_folder_name.to_string_lossy();
            let tx = db.transaction()?;
            tx.execute(
                "DELETE FROM folders WHERE parent = ? AND name = ?",
                (folder.id, &new_folder_name_str),
            )?;
            tx.execute(
                "UPDATE folders SET parent = ?, name = ? WHERE id = ?",
                (new_parent_id, &new_folder_name_str, folder.id),
            )?;
            tx.commit()?;

            // remove from old parent
            let removed = self
                .folders
                .get_mut(&old_parent_id)
                .unwrap()
                .sub_folders
                .remove(old_path.file_name().unwrap());
            debug_assert!(removed.is_some());

            // add to new parent
            self.folders
                .get_mut(&new_parent_id)
                .unwrap()
                .sub_folders
                .insert(new_path.file_name().unwrap().to_owned(), folder_id);

            // update folder
            let folder = self.folders.get_mut(&folder_id).unwrap();
            folder.name = new_path.file_name().unwrap().into();

            // update sub-folders and entries paths
            self.update_folder_path(folder_id, new_path);
        } else {
            // folder does not exist, create new folder
            self.scan_dir(new_path, db)?;
        }

        info!("Moved folder from {old_path:?} to {new_path:?}");

        Ok(())
    }

    fn update_folder_path(&mut self, folder_id: FolderId, path: &Path) {
        let folder = self.folders.get_mut(&folder_id).unwrap();
        folder.path = path.into();

        let sub_entry_ids = folder.entries.values().copied().collect::<Vec<_>>();
        let sub_folder_ids = folder.sub_folders.values().copied().collect::<Vec<_>>();

        // update sub-entries paths
        for entry_id in sub_entry_ids {
            let entry = self.entries.get_mut(&entry_id).unwrap();
            entry.path = path.join(&entry.file_name);
        }

        // recursively update sub-folders
        for sub_folder_id in sub_folder_ids {
            let sub_folder = self.folders.get(&sub_folder_id).unwrap();
            self.update_folder_path(sub_folder.id, &path.join(&sub_folder.name));
        }
    }

    // ========== Entry ==========

    pub fn get_entries(&self) -> &HashMap<EntryId, Entry> {
        &self.entries
    }

    pub fn get_entry(&self, entry_id: EntryId) -> Option<&Entry> {
        self.entries.get(&entry_id)
    }

    pub fn get_entry_id(&self, path: &Path) -> Option<EntryId> {
        self.get_folder_by_path(path.parent().unwrap())
            .and_then(|folder| folder.entries.get(path.file_name().unwrap()).copied())
    }

    pub fn get_entry_path(&self, entry_id: EntryId) -> Option<PathBuf> {
        self.entries
            .get(&entry_id)
            .map(|entry| self.to_absolute_path(&entry.path))
    }

    fn add_entries(&mut self, paths: &[PathBuf], db: &Connection) -> Result<()> {
        info!("Adding entries: {paths:#?}");

        debug_assert!(
            paths.iter().all(|path| path.is_relative()),
            "All paths must be relative"
        );

        // use different strategy for different number of new entries
        if paths.len() * 4 < self.entries.len() {
            // small number of new entries
            trace!("add_entries: small number of new entries: {}", paths.len());
            self.add_entries_serial(paths, db)?;
        } else {
            // large number of new entries
            trace!("add_entries: large number of new entries: {}", paths.len());
            self.add_entries_batch(paths, db)?;
        }

        Ok(())
    }

    fn add_entries_serial(&mut self, paths: &[PathBuf], db: &Connection) -> Result<()> {
        // read entries file metadata
        let mut new_entries = paths
            .iter()
            .filter_map(|path| {
                let folder_path = path.parent().unwrap();
                let Some(folder) = self.get_folder_by_path(folder_path) else {
                    warn!("Failed to add entry for {path:?}: folder {folder_path:?} not found");
                    return None;
                };
                let mut entry = Entry::new(path.into(), folder.id);
                entry.read_file(&self.base_path);
                Some(entry)
            })
            .collect::<Vec<_>>();

        let mut stmt_insert =
            db.prepare("INSERT INTO entries (file_name, folder_id) VALUES (?, ?)")?;
        let mut stmt_restore = db.prepare("UPDATE entries SET deleted = NULL WHERE id = ?")?;

        for entry in &mut new_entries {
            let query_row = db
                .query_row(
                    "SELECT id, deleted FROM entries WHERE folder_id = ? AND file_name = ?",
                    (entry.folder_id, entry.file_name.to_string_lossy()),
                    |row| {
                        Ok((
                            row.get::<_, EntryId>(0)?,                  // id
                            row.get::<_, Option<String>>(1)?.is_some(), // deleted
                        ))
                    },
                )
                .optional()?;

            if let Some((id, deleted)) = query_row {
                // entry exists in database
                entry.id = id;

                if deleted {
                    // entry is deleted, restore it
                    stmt_restore.execute([entry.id])?;
                }

                entry.tag_ids = db
                    .prepare(
                        "SELECT entry_tag.tag_id
                            FROM entry_tag
                            JOIN entries ON entry_tag.entry_id = entries.id
                            JOIN tags ON entry_tag.tag_id = tags.id
                            WHERE entry_tag.entry_id = ?
                            AND entries.deleted IS NULL
                            AND tags.deleted IS NULL",
                    )?
                    .query_map([&entry.id], |row| row.get::<_, TagId>(0))?
                    .filter_map(std::result::Result::ok)
                    .collect();
            } else {
                // entry does not exist in database
                let id: EntryId = stmt_insert
                    .insert((entry.file_name.to_string_lossy(), entry.folder_id))?
                    .try_into()
                    .unwrap();
                entry.id = id;
            }
        }

        // Add entries to the data in memory
        for entry in &new_entries {
            self.folders
                .get_mut(&entry.folder_id)
                .unwrap()
                .entries
                .insert(entry.file_name.clone(), entry.id);
        }
        self.entries
            .extend(new_entries.into_iter().map(|entry| (entry.id, entry)));

        Ok(())
    }

    fn add_entries_batch(&mut self, paths: &[PathBuf], db: &Connection) -> Result<()> {
        // read entries file metadata
        let mut new_entries = paths
            .iter()
            .filter_map(|path| {
                let folder_path = path.parent().unwrap();
                let Some(folder) = self.get_folder_by_path(folder_path) else {
                    warn!("Failed to add entry for {path:?}: folder {folder_path:?} not found");
                    return None;
                };
                let mut entry = Entry::new(path.into(), folder.id);
                entry.read_file(&self.base_path);
                Some(entry)
            })
            .collect::<Vec<_>>();

        // query all entry ids from database in one batch
        // and store them into a path - entry_id map
        let query_rows = db
            .prepare("SELECT id, file_name, folder_id, deleted FROM entries")?
            .query_map([], |row| {
                let entry_id = row.get::<_, EntryId>(0)?;
                let file_name = row.get::<_, String>(1)?;
                let folder_id = row.get::<_, FolderId>(2)?;
                let deleted = row.get::<_, Option<String>>(3)?.is_some();
                Ok(((folder_id, file_name), (entry_id, deleted)))
            })?
            .filter_map(std::result::Result::ok)
            .collect::<HashMap<(FolderId, String), (EntryId, bool)>>();

        // match queried rows with entries and perform corresponding actions

        let mut stmt_insert =
            db.prepare("INSERT INTO entries (file_name, folder_id) VALUES (?, ?)")?;
        let mut stmt_restore = db.prepare("UPDATE entries SET deleted = NULL WHERE id = ?")?;

        for entry in &mut new_entries {
            // find matching entry in queried rows
            let file_name = entry.file_name.to_string_lossy();
            if let Some((id, deleted)) = query_rows
                .get(&(entry.folder_id, file_name.into()))
                .copied()
            {
                // if entry already exists in database, set id
                entry.id = id;

                if deleted {
                    // entry is deleted, restore it
                    stmt_restore.execute([entry.id])?;
                }
            } else {
                // if entry does not exist in database, insert entry and set id
                let id: EntryId = stmt_insert
                    .insert((entry.file_name.to_string_lossy(), entry.folder_id))?
                    .try_into()
                    .unwrap();
                entry.id = id;
            }
        }

        // Add entries to the data in memory
        for entry in &new_entries {
            self.folders
                .get_mut(&entry.folder_id)
                .unwrap()
                .entries
                .insert(entry.file_name.clone(), entry.id);
        }
        self.entries
            .extend(new_entries.into_iter().map(|entry| (entry.id, entry)));

        // query all entry_tag rows from database
        // and store them into a entry_id - tag_id list
        let query_entry_tags = db
            .prepare(
                "SELECT entry_tag.entry_id, entry_tag.tag_id
                    FROM entry_tag
                    JOIN entries ON entry_tag.entry_id = entries.id
                    JOIN tags ON entry_tag.tag_id = tags.id
                    WHERE entries.deleted IS NULL
                    AND tags.deleted IS NULL",
            )?
            .query_map([], |row| {
                Ok((row.get::<_, EntryId>(0)?, row.get::<_, TagId>(1)?))
            })?
            .filter_map(std::result::Result::ok)
            .collect::<Vec<(EntryId, TagId)>>();

        for (entry_id, tag_id) in query_entry_tags {
            if let Some(entry) = self.entries.get_mut(&entry_id) {
                entry.tag_ids.insert(tag_id);
            }
        }

        Ok(())
    }

    fn remove_entry(&mut self, entry_id: EntryId, db: &Connection) -> Result<()> {
        info!("Removing entry: {entry_id}");

        let entry = self.entries.remove(&entry_id).unwrap();

        db.execute(
            "UPDATE entries SET deleted = datetime('now') WHERE id = ?",
            [entry_id],
        )?;

        let removed = self
            .folders
            .get_mut(&entry.folder_id)
            .unwrap()
            .entries
            .remove(&entry.file_name);
        debug_assert!(removed.is_some());

        Ok(())
    }

    fn remove_entries(&mut self, entry_ids: Vec<EntryId>, db: &Connection) -> Result<()> {
        info!("Removing entries: {entry_ids:?}");

        let mut stmt = db.prepare("UPDATE entries SET deleted = datetime('now') WHERE id = ?")?;

        for entry_id in entry_ids {
            let entry = self.entries.remove(&entry_id).unwrap();

            // remove entry from its folder
            let folder = self.folders.get_mut(&entry.folder_id).unwrap();
            let removed = folder.entries.remove(&entry.file_name);
            debug_assert!(removed.is_some());

            let result = stmt.execute([entry_id]);
            if let Err(err) = result {
                warn!("Failed to remove entry with ID {entry_id}: {err}");
            }
        }

        Ok(())
    }

    fn move_entry(
        &mut self,
        entry_id: EntryId,
        new_path: PathBuf,
        db: &mut Connection,
    ) -> Result<()> {
        debug_assert!(new_path.is_relative(), "Path must be relative");

        info!("Moving entry {entry_id} to {new_path:?}");

        // check existance of entry and folders
        let entry = self.get_entry(entry_id).unwrap();
        let old_folder_id = entry.folder_id;

        let new_folder_path = new_path.parent().unwrap();
        let new_folder_id = self.get_folder_by_path(new_folder_path).unwrap().id;

        // update entry in database
        let old_file_name = entry.file_name.clone();
        let new_file_name = new_path.file_name().unwrap().to_owned();
        let new_file_name_str = new_file_name.to_string_lossy();
        let tx = db.transaction()?;
        // override the old entry in the new position
        tx.execute(
            "DELETE FROM entries WHERE folder_id = ? AND file_name = ?",
            (new_folder_id, &new_file_name_str),
        )?;
        tx.execute(
            "UPDATE entries SET folder_id = ?, file_name = ? WHERE id = ?",
            (new_folder_id, &new_file_name_str, entry_id),
        )?;
        tx.commit()?;

        // remove entry from the old folder
        let old_folder = self.folders.get_mut(&old_folder_id).unwrap();
        let removed = old_folder.entries.remove(&old_file_name);
        debug_assert!(removed.is_some());

        // add entry to the new folder
        let new_folder = self.folders.get_mut(&new_folder_id).unwrap();
        new_folder.entries.insert(new_file_name.clone(), entry_id);

        // update entry path and file name
        let entry = self.entries.get_mut(&entry_id).unwrap();
        entry.folder_id = new_folder_id;
        entry.path = new_path;
        entry.file_name = new_file_name;

        info!("Moved entry {:?} to {:?}", entry_id, entry.path);

        Ok(())
    }

    fn move_entry_to_folder(
        &mut self,
        entry_id: EntryId,
        folder_id: FolderId,
        db: &mut Connection,
    ) -> Result<()> {
        info!("Moving entry {entry_id} to folder {folder_id}");

        // check existance of entry and folders
        let entry = self.get_entry(entry_id).unwrap();
        let file_name = entry.file_name.clone();
        let old_folder_id = entry.folder_id;

        let new_folder_path = &self.folders.get(&folder_id).unwrap().path;
        let new_path = new_folder_path.join(&file_name);

        // update entry in database
        let tx = db.transaction()?;
        // override the old entry in the new position
        tx.execute(
            "DELETE FROM entries WHERE folder_id = ? AND file_name = ?",
            (folder_id, file_name.to_string_lossy()),
        )?;
        tx.execute(
            "UPDATE entries SET folder_id = ? WHERE id = ?",
            (folder_id, entry_id),
        )?;
        tx.commit()?;

        // remove entry from the old folder
        let old_folder = self.folders.get_mut(&old_folder_id).unwrap();
        let removed = old_folder.entries.remove(&file_name);
        debug_assert!(removed.is_some());

        // add entry to the new folder
        let new_folder = self.folders.get_mut(&folder_id).unwrap();
        new_folder.entries.insert(file_name, entry_id);

        // update entry path and file name
        let entry = self.entries.get_mut(&entry_id).unwrap();
        entry.folder_id = folder_id;
        entry.path = new_path;

        info!("Moved entry {:?} to {:?}", entry_id, entry.path);

        Ok(())
    }

    // ========= Tag ==========

    pub fn get_tags(&self) -> Vec<TagNode> {
        TagNode::build(&self.tags)
    }

    pub fn get_tag_order(&self, tag_id: TagId) -> Vec<i32> {
        let mut order = Vec::new();
        let mut tag_id = tag_id;
        while tag_id != ROOT_TAG_ID {
            let tag = self.tags.get(&tag_id).unwrap();
            order.push(tag.position);
            tag_id = tag.parent_id;
        }
        order.reverse();
        order
    }

    pub fn new_tag(&mut self, name: String, db: &Connection) -> Result<TagId> {
        if self.tags.values().any(|tag| tag.name == name) {
            return Err(Error::TagAlreadyExists(name));
        }

        let root = self.tags.get(&ROOT_TAG_ID).unwrap();
        let position: i32 = root.children.len().try_into().unwrap();

        let id: TagId = db
            .prepare("INSERT INTO tags (name, parent, position) VALUES (?, ?, ?)")?
            .insert((&name, ROOT_TAG_ID, position))?
            .try_into()
            .unwrap();

        self.tags.insert(
            id,
            Tag {
                id,
                name,
                parent_id: ROOT_TAG_ID,
                position,
                children: HashSet::new(),
                color: 0,
            },
        );

        let root = self.tags.get_mut(&ROOT_TAG_ID).unwrap();
        root.children.insert(id);

        info!("Created new tag {id}");

        Ok(id)
    }

    pub fn delete_tag(&mut self, tag_id: TagId, db: &Connection) -> Result<()> {
        let tag = self.tags.get(&tag_id).unwrap();
        let parent_id = tag.parent_id;

        // remove its children recursively
        for child in tag.children.clone() {
            self.delete_tag(child, db)?;
        }

        db.execute(
            "UPDATE tags SET deleted = datetime('now') WHERE id = ?",
            [tag_id],
        )?;

        // remove tag
        let removed = self.tags.remove(&tag_id);
        debug_assert!(removed.is_some());

        // remove tag from its parent
        let parent = self.tags.get_mut(&parent_id).unwrap();
        let removed = parent.children.remove(&tag_id);
        debug_assert!(removed);

        // remove tag from entries
        for entries in self.entries.values_mut() {
            entries.tag_ids.remove(&tag_id);
        }

        info!("Deleted tag {tag_id}");

        Ok(())
    }

    pub fn rename_tag(&mut self, tag_id: TagId, name: String, db: &Connection) -> Result<()> {
        if self.tags.values().any(|tag| tag.name == name) {
            return Err(Error::TagAlreadyExists(name));
        }

        let tag = self.tags.get_mut(&tag_id).unwrap();

        db.execute("UPDATE tags SET name = ? WHERE id = ?", (&name, &tag_id))?;

        tag.name = name;

        info!("Renamed tag {tag_id} to {}", tag.name);

        Ok(())
    }

    /// Move a tag to a new parent and position.
    ///
    /// # Arguments
    ///
    /// * `tag_id` - The ID of the tag to move.
    /// * `to_parent_id` - The ID of the new parent tag.
    /// * `to_pos` - The new position of the tag, or -1 to append to the end.
    pub fn reorder_tag(
        &mut self,
        tag_id: TagId,
        to_parent_id: TagId,
        to_pos: i32,
        db: &mut Connection,
    ) -> Result<()> {
        let tag = self.tags.get(&tag_id).unwrap();
        let from_pos = tag.position;
        let from_parent_id = tag.parent_id;

        // move tag to the end if to_pos == -1
        let to_pos: i32 = if to_pos == -1 {
            self.tags
                .get(&to_parent_id)
                .unwrap()
                .children
                .len()
                .try_into()
                .unwrap()
        } else {
            to_pos
        };

        // if position is not changed
        if from_parent_id == to_parent_id && from_pos == to_pos {
            return Ok(());
        }

        if from_parent_id == to_parent_id {
            // move tag within the same parent

            // update database
            let tx = db.transaction()?;
            if from_pos < to_pos {
                // move downwards
                tx.execute(
                    "UPDATE tags SET position = position - 1 WHERE parent = ? AND position > ? AND position <= ?",
                    [&from_parent_id, &from_pos, &to_pos],
                )?;
            } else {
                // move upwards
                tx.execute(
                    "UPDATE tags SET position = position + 1 WHERE parent = ? AND position < ? AND position >= ?",
                    [&from_parent_id, &from_pos, &to_pos]
                )?;
            }
            tx.execute(
                "UPDATE tags SET position = ? WHERE id = ?",
                [&to_pos, &tag_id],
            )?;
            tx.commit()?;

            // update in-memory data
            if tag.position < to_pos {
                self.tags
                    .iter_mut()
                    .filter(|(_, t)| {
                        t.parent_id == from_parent_id
                            && t.position > from_pos
                            && t.position <= to_pos
                    })
                    .for_each(|(_, t)| {
                        t.position -= 1;
                    });
            } else {
                self.tags
                    .iter_mut()
                    .filter(|(_, t)| {
                        t.parent_id == from_parent_id
                            && t.position < from_pos
                            && t.position >= to_pos
                    })
                    .for_each(|(_, t)| {
                        t.position += 1;
                    });
            }
            self.tags.get_mut(&tag_id).unwrap().position = to_pos;
        } else {
            // move tag across different parents

            // update database
            let tx = db.transaction()?;
            tx.execute(
                "UPDATE tags SET position = position - 1 WHERE parent = ? AND position > ?",
                [&from_parent_id, &from_pos],
            )?;
            tx.execute(
                "UPDATE tags SET position = position + 1 WHERE parent = ? AND position >= ?",
                [&to_parent_id, &to_pos],
            )?;
            tx.execute(
                "UPDATE tags SET parent = ?, position = ? WHERE id = ?",
                [&to_parent_id, &to_pos, &tag_id],
            )?;
            tx.commit()?;

            // update in-memory data
            for t in self.tags.values_mut() {
                if t.parent_id == from_parent_id && t.position > from_pos {
                    t.position -= 1;
                } else if t.parent_id == to_parent_id && t.position >= to_pos {
                    t.position += 1;
                }
            }
            let tag = self.tags.get_mut(&tag_id).unwrap();
            tag.parent_id = to_parent_id;
            tag.position = to_pos;

            // remove tag from its old parent
            let removed = self
                .tags
                .get_mut(&from_parent_id)
                .unwrap()
                .children
                .remove(&tag_id);
            debug_assert!(removed);

            // add tag to its new parent
            self.tags
                .get_mut(&to_parent_id)
                .unwrap()
                .children
                .insert(tag_id);
        }

        info!(
            "Reordered tag {tag_id} from parent {from_parent_id} position {from_pos} to parent {to_parent_id} position {to_pos}"
        );

        Ok(())
    }

    pub fn set_tag_color(&mut self, tag_id: TagId, color: i32, db: &Connection) -> Result<()> {
        let tag = self.tags.get_mut(&tag_id).unwrap();

        db.execute("UPDATE tags SET color = ? WHERE id = ?", [&color, &tag_id])?;

        tag.color = color;

        Ok(())
    }

    // ========== Entry-Tag ==========

    pub fn get_tags_for_entry(&self, entry_id: EntryId) -> Vec<&Tag> {
        let entry = self.get_entry(entry_id).unwrap();

        let mut tags = entry
            .tag_ids
            .iter()
            .filter_map(|tag_id| {
                let tag = self.tags.get(tag_id);
                if tag.is_none() {
                    warn!("Tag of ID {tag_id} not found");
                }
                tag
            })
            .collect::<Vec<_>>();

        tags.sort_by_cached_key(|tag| self.get_tag_order(tag.id));

        tags
    }

    pub fn add_tag_for_entry(
        &mut self,
        entry_id: EntryId,
        tag_id: TagId,
        db: &Connection,
    ) -> Result<()> {
        let entry = self.entries.get_mut(&entry_id).unwrap();

        if !entry.tag_ids.insert(tag_id) {
            // tag already exists
            return Err(Error::TagAlreadyExistsForEntry(tag_id, entry_id));
        }

        db.execute(
            "INSERT INTO entry_tag (entry_id, tag_id) VALUES (?, ?)",
            [&entry_id, &tag_id],
        )?;

        info!(
            "Added tag {tag_id} to entry {:?} ({entry_id})",
            entry.file_name
        );

        Ok(())
    }

    pub fn remove_tag_for_entry(
        &mut self,
        entry_id: EntryId,
        tag_id: TagId,
        db: &Connection,
    ) -> Result<()> {
        let entry = self.entries.get_mut(&entry_id).unwrap();

        db.execute(
            "DELETE FROM entry_tag WHERE entry_id = ? AND tag_id = ?",
            [&entry_id, &tag_id],
        )?;

        let removed = entry.tag_ids.remove(&tag_id);
        debug_assert!(removed);

        info!(
            "Removed tag {tag_id} from entry {:?} ({entry_id})",
            entry.file_name
        );

        Ok(())
    }
}

fn is_audio_file(path: &Path) -> bool {
    match path.extension() {
        None => false,
        Some(ext) => {
            let ext = ext.to_string_lossy().to_ascii_lowercase();
            ext == "wav" || ext == "mp3" || ext == "flac" || ext == "ogg"
        }
    }
}

fn is_hidden_file(path: &Path) -> bool {
    path.file_name().unwrap().to_string_lossy().starts_with('.')
}
