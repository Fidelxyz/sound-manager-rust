mod entry;
mod file_watcher;
mod filter;
mod folder;
mod spotter;
mod tag;

pub use entry::Entry;
pub use filter::Filter;
pub use folder::Folder;
pub use tag::{Tag, TagNode};

use file_watcher::notify;

use log::{debug, info, trace, warn};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

use rusqlite::{Connection, OptionalExtension};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("database not found: {0}")]
    DatabaseNotFound(String),
    #[error("database already exists: {0}")]
    DatabaseAlreadyExists(String),
    #[error("entry not found: {0}")]
    EntryNotFound(i32),
    #[error("entry not found by path: {0}")]
    EntryNotFoundByPath(String),
    #[error("folder not found: {0}")]
    FolderNotFound(String),
    #[error("tag not found: {0}")]
    TagNotFound(i32),
    #[error("tag already exists: {0}")]
    TagAlreadyExists(String),
    #[error("tag {0} not found for entry {1}")]
    TagNotFoundForEntry(i32, i32),
    #[error("tag {0} already exists for entry {1}")]
    TagAlreadyExistsForEntry(i32, i32),
    #[error("file already exists: {0}")]
    FileAlreadyExists(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("path strip prefix error: {0}")]
    PathStripPrefix(#[from] std::path::StripPrefixError),
    #[error("database error: {0}")]
    Rusqlite(#[from] rusqlite::Error),
    #[error("symphonia error: {0}")]
    Symphonia(#[from] symphonia::core::errors::Error),
    #[error("notify error: {0}")]
    Notify(#[from] notify::Error),
}

type Result<T> = std::result::Result<T, Error>;

pub struct Database {
    pub data: Arc<RwLock<DatabaseData>>,
    pub db: Arc<Mutex<Connection>>,

    emitter: Arc<dyn DatabaseEmitter + Send + Sync>,
}

pub struct DatabaseData {
    base_path: Box<Path>,
    folder: Folder,
    entries: HashMap<i32, Entry>,
    path_to_entry_id: HashMap<Box<Path>, i32>,
    tags: HashMap<i32, Tag>,
    root_tag_ids: HashSet<i32>,
}

pub trait DatabaseEmitter {
    fn on_files_updated(&self);
}

impl Database {
    // ========== Constructor ==========

    pub fn open<E>(base_path: Box<Path>, emitter: E) -> Result<Database>
    where
        E: DatabaseEmitter + Send + Sync + 'static,
    {
        let database_file = base_path.join(".soundmanager.db");
        if !database_file.try_exists()? {
            // if database file does not exist
            return Err(Error::DatabaseNotFound(
                base_path.to_string_lossy().to_string(),
            ));
        }

        let db = Connection::open(database_file)?;

        let (tags, root_tag_ids) = Self::read_tags(&db)?;
        let folder = Folder::new(
            base_path.file_name().unwrap().to_string_lossy().to_string(),
            Path::new("").into(),
        );
        let database = Self {
            data: Arc::new(
                DatabaseData {
                    base_path,
                    folder,
                    entries: HashMap::new(),
                    path_to_entry_id: HashMap::new(),
                    tags,
                    root_tag_ids,
                }
                .into(),
            ),
            db: Arc::new(db.into()),
            emitter: Arc::new(emitter),
        };

        database
            .data
            .write()
            .unwrap()
            .scan(&database.db.lock().unwrap())?;
        database.watch_dir()?;

        Ok(database)
    }

    pub fn create<E>(base_path: Box<Path>, emitter: E) -> Result<Database>
    where
        E: DatabaseEmitter + Send + Sync + 'static,
    {
        let database_file = base_path.join(".soundmanager.db");
        if database_file.try_exists()? {
            // if database file already exists
            return Err(Error::DatabaseAlreadyExists(
                base_path.to_string_lossy().to_string(),
            ));
        }

        let db = Connection::open(database_file)?;
        db.execute(
            "CREATE TABLE IF NOT EXISTS entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL
            )",
            (),
        )?;
        db.execute(
            "CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                color INTEGER DEFAULT 0,
                parent INTEGER DEFAULT -1,
                position INTEGER NOT NULL
            )",
            (),
        )?;
        db.execute(
            "CREATE TABLE IF NOT EXISTS entry_tag (
                entry_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (entry_id, tag_id),
                FOREIGN KEY (entry_id) REFERENCES entries(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            )",
            (),
        )?;

        let folder = Folder::new(
            base_path.file_name().unwrap().to_string_lossy().to_string(),
            Path::new("").into(),
        );
        let database = Self {
            data: Arc::new(
                DatabaseData {
                    base_path,
                    folder,
                    entries: HashMap::new(),
                    path_to_entry_id: HashMap::new(),
                    tags: HashMap::new(),
                    root_tag_ids: HashSet::new(),
                }
                .into(),
            ),
            db: Arc::new(db.into()),
            emitter: Arc::new(emitter),
        };

        database
            .data
            .write()
            .unwrap()
            .scan(&database.db.lock().unwrap())?;
        database.watch_dir()?;

        Ok(database)
    }

    pub fn read_tags(db: &Connection) -> Result<(HashMap<i32, Tag>, HashSet<i32>)> {
        let mut tags = db
            .prepare("SELECT id, name, parent, position, color FROM tags")?
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    Tag {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        parent_id: row.get(2)?,
                        position: row.get(3)?,
                        color: row.get(4)?,
                        children_ids: HashSet::new(),
                    },
                ))
            })?
            .collect::<std::result::Result<HashMap<_, _>, _>>()?;

        // build tree relationship
        let tag_parent = tags
            .iter()
            .map(|(id, tag)| (*id, tag.parent_id))
            .collect::<Vec<_>>();
        let mut root_tag_ids = HashSet::new();
        for (id, parent_id) in tag_parent {
            if parent_id == -1 {
                root_tag_ids.insert(id);
            } else {
                let parent = tags.get_mut(&parent_id).unwrap();
                parent.children_ids.insert(id);
            }
        }

        Ok((tags, root_tag_ids))
    }

    pub fn refresh(&self) -> Result<()> {
        self.data.write().unwrap().scan(&self.db.lock().unwrap())?;
        self.emitter.on_files_updated();
        Ok(())
    }
}

impl DatabaseData {
    pub fn to_relative_path(&self, path: &Path) -> Result<Box<Path>> {
        debug_assert!(path.is_absolute());
        let relative_path = path.strip_prefix(&self.base_path)?;
        Ok(relative_path.into())
    }

    pub fn to_absolute_path(&self, path: &Path) -> Box<Path> {
        debug_assert!(path.is_relative());
        let absolute_path = self.base_path.join(path);
        absolute_path.into()
    }

    pub fn get_entry_id(&self, path: &Path) -> Option<i32> {
        debug_assert!(path.is_relative(), "Path must be relative");
        self.path_to_entry_id.get(path).copied()
    }

    fn read_dir(
        folder: &mut Folder,
        base_path: &Path,
        path_to_entry: &HashMap<Box<Path>, i32>,
    ) -> Result<(Vec<i32>, Vec<Box<Path>>)> {
        info!("Reading directory: {:?}", base_path.join(&folder.path));

        let mut existing_entries = Vec::new();
        let mut new_entries = Vec::new();

        for dir_entry in base_path.join(&folder.path).read_dir()? {
            let dir_entry = match dir_entry {
                Ok(dir_entry) => dir_entry,
                Err(err) => {
                    warn!("Failed to read directory entry: {:?}", err);
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
                    warn!("Failed to read file type: {:?}", err);
                    continue;
                }
            };

            if file_type.is_dir() {
                let folder_name = file_name.to_string_lossy();
                let sub_folder = folder
                    .sub_folders
                    .entry(folder_name.to_string())
                    .or_insert_with(|| {
                        Folder::new(folder_name.to_string(), folder.path.join(&file_name).into())
                    });

                match Self::read_dir(sub_folder, base_path, path_to_entry) {
                    Ok((subdir_existing_entries, subdir_new_entries)) => {
                        existing_entries.extend(subdir_existing_entries);
                        new_entries.extend(subdir_new_entries);
                    }
                    Err(err) => warn!("Failed to read directory: {:?}", err),
                };
            } else if file_type.is_file() && is_audio_file(file_name.as_ref()) {
                let relative_path = folder.path.join(&file_name);
                match path_to_entry.get(relative_path.as_path()) {
                    Some(entry_id) => existing_entries.push(*entry_id),
                    None => new_entries.push(relative_path.into()),
                }
            } else {
                warn!(
                    "Failed to read file for path {:?}: unknown directory entry type.",
                    dir_entry.path()
                );
            }
        }

        Ok((existing_entries, new_entries))
    }

    fn scan(&mut self, db: &Connection) -> Result<()> {
        let (existing_entries, new_entries) =
            Self::read_dir(&mut self.folder, &self.base_path, &self.path_to_entry_id)?;

        let existing_entries: HashSet<i32> = existing_entries.into_iter().collect();

        // remove deleted entries
        let removed_entries = self
            .path_to_entry_id
            .iter()
            .filter(|(_, entry_id)| !existing_entries.contains(entry_id))
            .map(|(path, entry_id)| (path.clone(), *entry_id))
            .collect::<Vec<_>>();
        for (path, entry_id) in &removed_entries {
            self.entries.remove(entry_id);
            self.path_to_entry_id.remove(path);
        }
        info!("Removed entries: {:#?}", removed_entries);

        // update existing entries metadata
        for entry in self.entries.values_mut() {
            entry.read_file(&self.base_path);
        }

        // read new entries
        self.add_entries(new_entries, db)?;

        Ok(())
    }

    fn scan_dir(&mut self, folder: &mut Folder, db: &Connection) -> Result<()> {
        let (existing_entries, new_entries) =
            Self::read_dir(folder, &self.base_path, &self.path_to_entry_id)?;

        let existing_entries: HashSet<i32> = existing_entries.into_iter().collect();

        // remove deleted entries
        let removed_entries = self
            .path_to_entry_id
            .iter()
            .filter(|(path, entry_id)| {
                path.starts_with(&folder.path) && !existing_entries.contains(entry_id)
            })
            .map(|(path, entry_id)| (path.clone(), *entry_id))
            .collect::<Vec<_>>();
        for (path, entry_id) in &removed_entries {
            self.entries.remove(entry_id);
            self.path_to_entry_id.remove(path);
        }
        info!("Removed entries: {:#?}", removed_entries);

        // update existing entries metadata
        for entry in self.entries.values_mut() {
            entry.read_file(&self.base_path);
        }

        // read new entries
        self.add_entries(new_entries, db)?;

        Ok(())
    }

    pub fn scan_folders(&mut self) -> Result<()> {
        self.folder.scan_folder(&self.base_path)
    }

    // ========== Folder ==========

    pub fn get_folders(&self) -> &Folder {
        &self.folder
    }

    fn add_folder(&mut self, path: &Path, db: &Connection) -> Result<()> {
        debug_assert!(path.is_relative(), "Path must be relative");

        let mut folder = Folder::new(
            path.file_name().unwrap().to_string_lossy().to_string(),
            path.into(),
        );
        self.scan_dir(&mut folder, db)?;
        self.folder
            .insert_sub_folder(path.components().peekable(), folder)?;
        info!("Added folder: {:?}", path);

        Ok(())
    }

    fn remove_folder(&mut self, path: &Path) -> Result<()> {
        debug_assert!(path.is_relative(), "Path must be relative");

        self.folder
            .remove_sub_folder(path.components().peekable())
            .ok_or_else(|| Error::FolderNotFound(path.to_string_lossy().to_string()))?;

        info!("Removed folder: {:?}", path);

        // Remove entries in the removed folder
        let removed_entries = self
            .path_to_entry_id
            .iter()
            .filter(|(entry_path, _)| entry_path.starts_with(path))
            .map(|(entry_path, entry_id)| (entry_path.clone(), *entry_id))
            .collect::<Vec<_>>();
        for (entry_path, entry_id) in &removed_entries {
            self.entries.remove(entry_id);
            self.path_to_entry_id.remove(entry_path);
        }
        info!("Removed entries: {:#?}", removed_entries);

        Ok(())
    }

    fn move_folder(&mut self, old_path: &Path, new_path: &Path, db: &Connection) -> Result<()> {
        debug_assert!(old_path.is_relative(), "Path must be relative");
        debug_assert!(new_path.is_relative(), "Path must be relative");

        let folder = self
            .folder
            .remove_sub_folder(old_path.components().peekable());

        let folder = match folder {
            // folder exists
            Some(mut folder) => {
                folder.path = new_path.into();
                folder.name = new_path.file_name().unwrap().to_string_lossy().to_string();

                // move entries in the moved folder
                let moved_entries = self
                    .path_to_entry_id
                    .iter()
                    .filter(|(entry_path, _)| entry_path.starts_with(old_path))
                    .map(|(entry_path, entry_id)| (entry_path.clone(), *entry_id))
                    .collect::<Vec<_>>();
                for (path, entry_id) in moved_entries {
                    let new_entry_path = new_path.join(path.strip_prefix(old_path).unwrap());
                    self.entries.get_mut(&entry_id).unwrap().path = new_entry_path.as_path().into();
                    self.path_to_entry_id.remove(&path);
                    self.path_to_entry_id
                        .insert(new_entry_path.as_path().into(), entry_id);
                    info!("Moved entry {:?} to {:?}", entry_id, new_entry_path);
                }

                db.execute(
                    "UPDATE entries
                    SET file_path = CONCAT(?2, SUBSTRING(file_path, LENGTH(?1) + 1))
                    WHERE file_path LIKE CONCAT(?1, '%')",
                    [&old_path.to_string_lossy(), &new_path.to_string_lossy()],
                )?;

                folder
            }
            // folder does not exist
            None => {
                let mut folder = Folder::new(
                    new_path.file_name().unwrap().to_string_lossy().to_string(),
                    new_path.into(),
                );
                self.scan_dir(&mut folder, db)?;
                folder
            }
        };

        self.folder
            .insert_sub_folder(new_path.components().peekable(), folder)?;

        info!("Moved folder from {:?} to {:?}", old_path, new_path);

        Ok(())
    }

    // ========== Entry ==========

    pub fn get_entries(&self) -> &HashMap<i32, Entry> {
        &self.entries
    }

    pub fn get_entry(&self, entry_id: i32) -> Result<&Entry> {
        self.entries
            .get(&entry_id)
            .ok_or_else(|| Error::EntryNotFound(entry_id))
    }

    fn add_entries(&mut self, paths: Vec<Box<Path>>, db: &Connection) -> Result<()> {
        info!("Adding entries: {:#?}", paths);

        debug_assert!(
            paths.iter().all(|path| path.is_relative()),
            "All paths must be relative"
        );

        debug_assert!(
            paths
                .iter()
                .all(|path| !self.path_to_entry_id.contains_key(path)),
            "All new paths must not exist in the database"
        );

        // read entries file metadata
        let mut new_entries = paths
            .iter()
            .map(|path| {
                let mut entry = Entry::new(path.clone());
                entry.read_file(&self.base_path);
                entry
            })
            .collect::<Vec<_>>();

        // use different strategy for different number of new entries
        if paths.len() * 4 < self.entries.len() {
            // small number of new entries
            trace!("add_entries: small number of new entries: {}", paths.len());

            for entry in new_entries.iter_mut() {
                let path = entry.path.to_string_lossy();

                let query_entry_id = db
                    .query_row(
                        "SELECT id FROM entries WHERE file_path = ?",
                        [path.as_ref()],
                        |row| row.get::<_, i32>(0),
                    )
                    .optional()?;

                let mut stmt_insert = db.prepare("INSERT INTO entries (file_path) VALUES (?)")?;
                if let Some(id) = query_entry_id {
                    // entry exists in database
                    entry.id = id;

                    entry.tag_ids = db
                        .prepare("SELECT tag_id FROM entry_tag WHERE entry_id = ?")?
                        .query_map([&entry.id], |row| row.get::<_, i32>(0))?
                        .filter_map(|result| result.ok())
                        .collect();
                } else {
                    // entry does not exist in database
                    let id = stmt_insert.insert([path.as_ref()])? as i32;
                    entry.id = id;
                }
            }

            self.path_to_entry_id.extend(
                new_entries
                    .iter()
                    .map(|entry| (entry.path.clone(), entry.id)),
            );
            self.entries
                .extend(new_entries.into_iter().map(|entry| (entry.id, entry)));
        } else {
            // large number of new entries
            trace!("add_entries: large number of new entries: {}", paths.len());

            // query all entry ids from database in one batch
            // and store them into a path - entry_id map
            let query_entry_ids = db
                .prepare("SELECT id, file_path FROM entries")?
                .query_map([], |row| {
                    Ok((row.get::<_, String>(1)?, row.get::<_, i32>(0)?))
                })?
                .filter_map(|result| result.ok())
                .collect::<HashMap<String, i32>>();

            // match queried rows with entries and perform corresponding actions
            let mut stmt_insert = db.prepare("INSERT INTO entries (file_path) VALUES (?)")?;
            for entry in new_entries.iter_mut() {
                // find matching entry in queried rows
                let path = entry.path.to_string_lossy();
                if let Some(id) = query_entry_ids.get(path.as_ref()) {
                    // if entry already exists in database, set id
                    entry.id = *id;
                } else {
                    // if entry does not exist in database, insert entry and set id
                    let id = stmt_insert.insert([path.as_ref()])? as i32;
                    entry.id = id;
                }
            }

            self.path_to_entry_id.extend(
                new_entries
                    .iter()
                    .map(|entry| (entry.path.clone(), entry.id)),
            );
            self.entries
                .extend(new_entries.into_iter().map(|entry| (entry.id, entry)));

            // query all entry_tag rows from database
            // and store them into a entry_id - tag_id list
            let query_entry_tags = db
                .prepare("SELECT entry_id, tag_id FROM entry_tag")?
                .query_map([], |row| Ok((row.get::<_, i32>(0)?, row.get::<_, i32>(1)?)))?
                .filter_map(|result| result.ok())
                .collect::<Vec<(i32, i32)>>();

            for (entry_id, tag_id) in query_entry_tags {
                if let Some(entry) = self.entries.get_mut(&entry_id) {
                    entry.tag_ids.insert(tag_id);
                }
            }
        }

        Ok(())
    }

    fn move_entry(&mut self, entry_id: i32, new_path: Box<Path>, db: &Connection) -> Result<()> {
        debug_assert!(new_path.is_relative(), "Path must be relative");

        let entry = self.entries.get_mut(&entry_id).expect("Entry not found");

        db.execute(
            "UPDATE entries SET file_path = ? WHERE id = ?",
            (&new_path.to_string_lossy(), &entry_id),
        )?;

        self.path_to_entry_id.remove(&entry.path);
        self.path_to_entry_id.insert(new_path.clone(), entry.id);
        entry.path = new_path;
        entry.file_name = entry
            .path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        info!("Moved entry {:?} to {:?}", entry_id, entry.path);

        Ok(())
    }

    fn remove_entry(&mut self, entry_id: i32) {
        let entry = self.entries.remove(&entry_id).expect("Entry not found");
        self.path_to_entry_id.remove(&entry.path);
    }

    // ========= Tag ==========

    pub fn get_tags(&self) -> Vec<TagNode> {
        TagNode::build(&self.tags, &self.root_tag_ids)
    }

    pub fn new_tag(&mut self, name: String, db: &Connection) -> Result<i32> {
        if self.tags.values().any(|tag| tag.name == name) {
            return Err(Error::TagAlreadyExists(name));
        }

        let position = self.root_tag_ids.len() as i32;

        let id = db
            .prepare("INSERT INTO tags (name, position) VALUES (?, ?)")?
            .insert((&name, &position))? as i32;

        self.tags.insert(
            id,
            Tag {
                id,
                parent_id: -1,
                position,
                children_ids: HashSet::new(),
                name,
                color: 0,
            },
        );
        self.root_tag_ids.insert(id);

        Ok(id)
    }

    pub fn delete_tag(&mut self, tag_id: i32, db: &Connection) -> Result<()> {
        let tag = self
            .tags
            .get(&tag_id)
            .ok_or_else(|| Error::TagNotFound(tag_id))?;

        db.execute("DELETE FROM tags WHERE id = ?", [&tag_id])?;

        let parent_id = tag.parent_id;
        self.tags.remove(&tag_id);
        if parent_id == -1 {
            self.root_tag_ids.remove(&tag_id);
        } else {
            let parent = self
                .tags
                .get_mut(&parent_id)
                .ok_or_else(|| Error::TagNotFound(parent_id))?;
            parent.children_ids.remove(&tag_id);
        }

        for entries in self.entries.values_mut() {
            entries.tag_ids.remove(&tag_id);
        }

        Ok(())
    }

    pub fn rename_tag(&mut self, tag_id: i32, name: String, db: &Connection) -> Result<()> {
        if self.tags.values().any(|tag| tag.name == name) {
            return Err(Error::TagAlreadyExists(name));
        }

        let tag = self
            .tags
            .get_mut(&tag_id)
            .ok_or_else(|| Error::TagNotFound(tag_id))?;

        db.execute("UPDATE tags SET name = ? WHERE id = ?", (&name, &tag_id))?;

        tag.name = name;

        Ok(())
    }

    pub fn reorder_tag(
        &mut self,
        tag_id: i32,
        to_parent_id: i32,
        to_pos: i32,
        db: &mut Connection,
    ) -> Result<()> {
        let tag = self
            .tags
            .get(&tag_id)
            .ok_or_else(|| Error::TagNotFound(tag_id))?;
        let from_pos = tag.position;
        let from_parent_id = tag.parent_id;

        if to_parent_id != -1 && !self.tags.contains_key(&to_parent_id) {
            return Err(Error::TagNotFound(to_parent_id));
        }

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
                tx.execute("UPDATE tags SET position = position - 1 WHERE parent = ? AND position > ? AND position <= ?", [&from_parent_id, &from_pos, &to_pos])?;
            } else {
                // move upwards
                tx.execute("UPDATE tags SET position = position + 1 WHERE parent = ? AND position < ? AND position >= ?", [&from_parent_id, &from_pos, &to_pos])?;
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
            for (_, t) in self.tags.iter_mut() {
                if t.parent_id == from_parent_id && t.position > from_pos {
                    t.position -= 1;
                } else if t.parent_id == to_parent_id && t.position >= to_pos {
                    t.position += 1;
                }
            }
            let tag = self.tags.get_mut(&tag_id).unwrap();
            tag.parent_id = to_parent_id;
            tag.position = to_pos;

            if from_parent_id == -1 {
                self.root_tag_ids.remove(&tag_id);
            } else {
                self.tags
                    .get_mut(&from_parent_id)
                    .unwrap()
                    .children_ids
                    .remove(&tag_id);
            }

            if to_parent_id == -1 {
                self.root_tag_ids.insert(tag_id);
            } else {
                self.tags
                    .get_mut(&to_parent_id)
                    .unwrap()
                    .children_ids
                    .insert(tag_id);
            }
        }

        debug!("tags: {:?}", self.tags);

        Ok(())
    }

    pub fn set_tag_color(&mut self, tag_id: i32, color: i32, db: &Connection) -> Result<()> {
        let tag = self
            .tags
            .get_mut(&tag_id)
            .ok_or_else(|| Error::TagNotFound(tag_id))?;

        db.execute("UPDATE tags SET color = ? WHERE id = ?", [&color, &tag_id])?;

        tag.color = color;

        Ok(())
    }

    // ========== Entry-Tag ==========

    pub fn get_tags_for_entry(&self, entry_id: i32) -> Result<Vec<&Tag>> {
        let entry = self.get_entry(entry_id)?;

        let mut tags = entry
            .tag_ids
            .iter()
            .map(|tag_id| {
                self.tags
                    .get(tag_id)
                    .ok_or_else(|| Error::TagNotFound(*tag_id))
            })
            .collect::<Result<Vec<_>>>()?;

        tags.sort_by_key(|tag| tag.position);

        Ok(tags)
    }

    pub fn add_tag_for_entry(&mut self, entry_id: i32, tag_id: i32, db: &Connection) -> Result<()> {
        if !self.tags.contains_key(&tag_id) {
            return Err(Error::TagNotFound(tag_id));
        }

        let entry = self
            .entries
            .get_mut(&entry_id)
            .ok_or_else(|| Error::EntryNotFound(entry_id))?;

        if entry.tag_ids.contains(&tag_id) {
            return Err(Error::TagAlreadyExistsForEntry(tag_id, entry_id));
        }

        db.execute(
            "INSERT INTO entry_tag (entry_id, tag_id) VALUES (?, ?)",
            [&entry_id, &tag_id],
        )?;

        entry.tag_ids.insert(tag_id);

        Ok(())
    }

    pub fn remove_tag_for_entry(
        &mut self,
        entry_id: i32,
        tag_id: i32,
        db: &Connection,
    ) -> Result<()> {
        let entry = self
            .entries
            .get_mut(&entry_id)
            .ok_or_else(|| Error::EntryNotFound(entry_id))?;

        if !entry.tag_ids.contains(&tag_id) {
            return Err(Error::TagNotFoundForEntry(tag_id, entry_id));
        }

        db.execute(
            "DELETE FROM entry_tag WHERE entry_id = ? AND tag_id = ?",
            [&entry_id, &tag_id],
        )?;

        entry.tag_ids.remove(&tag_id);

        Ok(())
    }
}

fn is_audio_file(path: &Path) -> bool {
    match path.extension() {
        None => false,
        Some(ext) => {
            let ext = ext.to_string_lossy();
            ext == "wav" || ext == "mp3" || ext == "flac" || ext == "ogg"
        }
    }
}

fn is_hidden_file(path: &Path) -> bool {
    path.file_name().unwrap().to_string_lossy().starts_with('.')
}
