use super::{Entry, Folder, Tag, TagNode};

use std::collections::{HashMap, HashSet};
use std::path::Path;

use futures::executor::block_on;
use futures::future::{join, join_all};
use log::{debug, warn};
use r2d2::Pool;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("database not found: {0}")]
    DatabaseNotFound(String),
    #[error("database already exists: {0}")]
    DatabaseAlreadyExists(String),
    #[error("entry not found: {0}")]
    EntryNotFound(i32),
    #[error("tag not found: {0}")]
    TagNotFound(i32),
    #[error("tag already exists: {0}")]
    TagAlreadyExists(String),
    #[error("tag {0} not found for entry {1}")]
    TagNotFoundForEntry(i32, i32),
    #[error("tag {0} already exists for entry {1}")]
    TagAlreadyExistsForEntry(i32, i32),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("r2d2 error: {0}")]
    R2d2(#[from] r2d2::Error),
    #[error("symphonia error: {0}")]
    Symphonia(#[from] symphonia::core::errors::Error),
}

type Result<T> = std::result::Result<T, Error>;

pub struct Database {
    pub base_path: Box<Path>,

    folder: Folder,
    entries: HashMap<i32, Entry>,
    tags: HashMap<i32, Tag>,
    root_tag_ids: HashSet<i32>,

    db_conn_pool: Pool<SqliteConnectionManager>,
}

impl Database {
    // ========== Constructor ==========

    pub fn open(base_path: Box<Path>) -> Result<Database> {
        let database_file = base_path.join(".soundmanager.db");
        if !database_file.try_exists()? {
            // if database file does not exist
            return Err(Error::DatabaseNotFound(
                base_path.to_string_lossy().to_string(),
            ));
        }

        let manager = SqliteConnectionManager::file(database_file);
        let db_conn_pool = r2d2::Pool::new(manager)?;

        let (entries, folder) = block_on(Self::read_dir(base_path.clone(), &db_conn_pool))?;
        let (tags, root_tag_ids) = Self::read_tags(&db_conn_pool)?;

        Ok(Self {
            base_path,
            folder,
            entries: Self::read_entries(entries, db_conn_pool.get()?)?,
            tags,
            root_tag_ids,
            db_conn_pool,
        })
    }

    pub fn create(base_path: Box<Path>) -> Result<Database> {
        let database_file = base_path.join(".soundmanager.db");
        if database_file.try_exists()? {
            // if database file already exists
            return Err(Error::DatabaseAlreadyExists(
                base_path.to_string_lossy().to_string(),
            ));
        }

        let manager = SqliteConnectionManager::file(database_file);
        let db_conn_pool = r2d2::Pool::new(manager)?;

        let db = db_conn_pool.get()?;
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

        let (entries, folder) = block_on(Self::read_dir(base_path.clone(), &db_conn_pool))?;
        let (tags, root_tag_ids) = Self::read_tags(&db_conn_pool)?;

        Ok(Self {
            base_path,
            folder,
            entries: Self::read_entries(entries, db_conn_pool.get()?)?,
            tags,
            root_tag_ids,
            db_conn_pool,
        })
    }

    async fn read_dir(
        path: Box<Path>,
        db_conn_pool: &Pool<SqliteConnectionManager>,
    ) -> Result<(Vec<Entry>, Folder)> {
        debug!("Reading directory: {:?}", path);

        let mut futures = Vec::new();
        let mut entries = Vec::new();

        for dir_entry in path.read_dir()? {
            let dir_entry = match dir_entry {
                Ok(dir_entry) => dir_entry,
                Err(err) => {
                    warn!("Failed to read directory entry: {:?}", err);
                    continue;
                }
            };

            // Skip hidden files and folders
            if dir_entry.file_name().to_string_lossy().starts_with(".") {
                continue;
            }

            let file_type = match dir_entry.file_type() {
                Ok(file_type) => file_type,
                Err(err) => {
                    warn!("Failed to read file type: {:?}", err);
                    continue;
                }
            };

            if file_type.is_symlink() {
                todo!("Handle symlink");
            }

            if file_type.is_dir() {
                let path = dir_entry.path().as_path().into();
                futures.push(Self::read_dir(path, db_conn_pool));
            } else if file_type.is_file() {
                let path = dir_entry.path();
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy();
                    if ext == "wav" || ext == "mp3" || ext == "flac" || ext == "ogg" {
                        entries.push(Entry::new(dir_entry.path().as_path().into()));
                    }
                }
            }
        }

        let results = join_all(futures).await;
        let mut sub_folders = Vec::new();

        for result in results {
            if let Ok((sub_entries, sub_folder)) = result {
                entries.extend(sub_entries);
                sub_folders.push(sub_folder);
            }
        }

        let name = path.file_name().unwrap().to_string_lossy().to_string();

        Ok((
            entries,
            Folder {
                path,
                name,
                sub_folders,
            },
        ))
    }

    fn read_entries(
        mut entries: Vec<Entry>,
        db: PooledConnection<SqliteConnectionManager>,
    ) -> Result<HashMap<i32, Entry>> {
        debug!("Reading entries");

        // read entries in parallel
        let futures_read = entries.iter_mut().map(|entry| entry.read());

        // query all entries from database in one batch
        let future_query = async {
            let entries = db
                .prepare("SELECT id, file_path FROM entries")?
                .query_map([], |row| {
                    Ok((row.get::<_, String>(1)?, row.get::<_, i32>(0)?))
                })?
                .filter_map(|result| result.ok())
                .collect::<HashMap<_, _>>();

            let entry_tag = db
                .prepare("SELECT entry_id, tag_id FROM entry_tag")?
                .query_map([], |row| Ok((row.get::<_, i32>(0)?, row.get::<_, i32>(1)?)))?
                .filter_map(|result| result.ok())
                .collect::<Vec<_>>();

            Ok::<_, Error>((entries, entry_tag))
        };

        // parallelly read and query
        let (_, query) = block_on(join(join_all(futures_read), future_query));

        let (query_entries, query_entry_tag) = query?;

        // match queried rows with entries and perform corresponding actions
        let mut stmt = db.prepare("INSERT INTO entries (file_path) VALUES (?)")?;
        for entry in entries.iter_mut() {
            // find matching entry in queried rows
            if let Some(path) = entry.path.to_str() {
                let row = query_entries.get(path);
                if let Some(id) = row {
                    // if entry already exists in database, set id
                    entry.id = *id;
                } else {
                    // if entry does not exist in database, insert entry and set id
                    let id = stmt.insert(&[path])? as i32;
                    entry.id = id;
                }
            }
        }

        let mut entries = entries
            .into_iter()
            .map(|entry| (entry.id, entry))
            .collect::<HashMap<_, _>>();

        for (entry_id, tag_id) in query_entry_tag {
            if let Some(entry) = entries.get_mut(&entry_id) {
                entry.tag_ids.insert(tag_id);
            }
        }

        Ok(entries)
    }

    pub fn read_tags(
        db_conn_pool: &Pool<SqliteConnectionManager>,
    ) -> Result<(HashMap<i32, Tag>, HashSet<i32>)> {
        let db = db_conn_pool.get()?;

        let mut stmt = db.prepare("SELECT id, name, parent, position, color FROM tags")?;
        let mut tags = stmt
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

    // ========== Folder ==========

    pub fn get_folder(&self) -> &Folder {
        &self.folder
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

    // ========== Tag ==========

    pub fn get_tags(&self) -> Vec<TagNode> {
        TagNode::build(&self.tags, &self.root_tag_ids)
    }

    pub fn new_tag(&mut self, name: String) -> Result<i32> {
        let db = self.db_conn_pool.get()?;

        if self.tags.values().any(|tag| tag.name == name) {
            return Err(Error::TagAlreadyExists(name));
        }

        let position = self.root_tag_ids.len() as i32;

        let mut stmt = db.prepare("INSERT INTO tags (name, position) VALUES (?, ?)")?;
        let id = stmt.insert((&name, &position))? as i32;

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

    pub fn delete_tag(&mut self, tag_id: i32) -> Result<()> {
        let tag = self
            .tags
            .get(&tag_id)
            .ok_or_else(|| Error::TagNotFound(tag_id))?;

        let db = self.db_conn_pool.get()?;
        db.execute("DELETE FROM tags WHERE id = ?", &[&tag_id])?;

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

    pub fn rename_tag(&mut self, tag_id: i32, name: String) -> Result<()> {
        let db = self.db_conn_pool.get()?;

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

    pub fn reorder_tag(&mut self, tag_id: i32, to_parent_id: i32, to_pos: i32) -> Result<()> {
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

        let mut db = self.db_conn_pool.get()?;

        if from_parent_id == to_parent_id {
            // move tag within the same parent

            // update database
            let tx = db.transaction()?;
            if from_pos < to_pos {
                // move downwards
                tx.execute("UPDATE tags SET position = position - 1 WHERE parent = ? AND position > ? AND position <= ?", &[&from_parent_id, &from_pos, &to_pos])?;
            } else {
                // move upwards
                tx.execute("UPDATE tags SET position = position + 1 WHERE parent = ? AND position < ? AND position >= ?", &[&from_parent_id, &from_pos, &to_pos])?;
            }
            tx.execute(
                "UPDATE tags SET position = ? WHERE id = ?",
                &[&to_pos, &tag_id],
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
                &[&from_parent_id, &from_pos],
            )?;
            tx.execute(
                "UPDATE tags SET position = position + 1 WHERE parent = ? AND position >= ?",
                &[&to_parent_id, &to_pos],
            )?;
            tx.execute(
                "UPDATE tags SET parent = ?, position = ? WHERE id = ?",
                &[&to_parent_id, &to_pos, &tag_id],
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

    pub fn set_tag_color(&mut self, tag_id: i32, color: i32) -> Result<()> {
        let tag = self
            .tags
            .get_mut(&tag_id)
            .ok_or_else(|| Error::TagNotFound(tag_id))?;

        let db = self.db_conn_pool.get()?;
        db.execute("UPDATE tags SET color = ? WHERE id = ?", &[&color, &tag_id])?;

        tag.color = color;

        Ok(())
    }

    // ========== Entry-Tag ==========

    pub fn add_tag_for_entry(&mut self, entry_id: i32, tag_id: i32) -> Result<()> {
        let entry = self
            .entries
            .get_mut(&entry_id)
            .ok_or_else(|| Error::EntryNotFound(entry_id))?;

        if !self.tags.contains_key(&tag_id) {
            return Err(Error::TagNotFound(tag_id));
        }

        if entry.tag_ids.contains(&tag_id) {
            return Err(Error::TagAlreadyExistsForEntry(tag_id, entry_id));
        }

        let db = self.db_conn_pool.get()?;
        db.execute(
            "INSERT INTO entry_tag (entry_id, tag_id) VALUES (?, ?)",
            &[&entry_id, &tag_id],
        )?;

        entry.tag_ids.insert(tag_id);

        Ok(())
    }

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

    pub fn remove_tag_for_entry(&mut self, entry_id: i32, tag_id: i32) -> Result<()> {
        let entry = self
            .entries
            .get_mut(&entry_id)
            .ok_or_else(|| Error::EntryNotFound(entry_id))?;

        if !entry.tag_ids.contains(&tag_id) {
            return Err(Error::TagNotFoundForEntry(tag_id, entry_id));
        }

        let db = self.db_conn_pool.get()?;
        db.execute(
            "DELETE FROM entry_tag WHERE entry_id = ? AND tag_id = ?",
            &[&entry_id, &tag_id],
        )?;

        entry.tag_ids.remove(&tag_id);

        Ok(())
    }
}
