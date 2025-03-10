use super::database::Error;
use super::player::get_format_reader;

use futures::{join, TryFutureExt};
use log::warn;
use std::path::Path;

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use symphonia::core::meta::StandardTagKey;

pub struct Entry {
    pub id: i32,
    pub path: Box<Path>,
    pub file_name: String,
    pub metadata: Option<Metadata>,
}

#[derive(Clone)]
pub struct Metadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<f32>,
}

impl Entry {
    pub fn new(path: Box<Path>) -> Self {
        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();

        Self {
            id: -1,
            path,
            file_name,
            metadata: None,
        }
    }

    pub async fn read(
        &mut self,
        db_conn_pool: &Pool<SqliteConnectionManager>,
    ) -> Result<(), Error> {
        //debug!("Reading entries: {:?}", self.path);

        let future_read_metadata = self.read_metadata().err_into::<Error>();
        let future_query_database = self.query_database(db_conn_pool.get()?).err_into::<Error>();

        let (metadata, id) = join!(future_read_metadata, future_query_database);

        if let Ok(metadata) = metadata {
            self.metadata = Some(metadata);
        } else {
            warn!(
                "Failed to read metadata of file {:?}: {:?}",
                self.path,
                metadata.err().unwrap()
            );
        }

        if let Ok(id) = id {
            self.id = id;
        } else {
            warn!(
                "Failed to query database for file {:?}: {:?}",
                self.path,
                id.err().unwrap()
            );
        }

        Ok(())
    }

    async fn read_metadata(&self) -> Result<Metadata, symphonia::core::errors::Error> {
        let mut ret = Metadata {
            title: None,
            artist: None,
            album: None,
            duration: None,
        };

        // Read metadata
        let mut format = get_format_reader(&self.path)?;

        let mut metadata = format.metadata();
        if let Some(metadata) = metadata.skip_to_latest() {
            for tag in metadata.tags() {
                match tag.std_key {
                    Some(StandardTagKey::TrackTitle) => ret.title = Some(tag.value.to_string()),
                    Some(StandardTagKey::Artist) => ret.artist = Some(tag.value.to_string()),
                    Some(StandardTagKey::Album) => ret.album = Some(tag.value.to_string()),
                    _ => {}
                }
            }
        }

        if let Some(track) = format.default_track() {
            let params = &track.codec_params;
            if let (Some(n_frames), Some(time_base)) = (params.n_frames, params.time_base) {
                let time = time_base.calc_time(n_frames);
                ret.duration = Some(time.seconds as f32 + time.frac as f32);
            }
        }

        Ok(ret)
    }

    async fn query_database(
        &self,
        db: PooledConnection<SqliteConnectionManager>,
    ) -> Result<i32, rusqlite::Error> {
        let ret_id;

        // Query from database
        match db.query_row(
            "SELECT id FROM entries WHERE file_path = ?",
            &[&self.path.to_str().unwrap()],
            |row| row.get(0),
        ) {
            Ok(id) => {
                // Entry already exists
                ret_id = id;
            }
            Err(_) => {
                // Create new entry in database
                db.execute(
                    "INSERT INTO entries (file_path) VALUES (?)",
                    &[&self.path.to_str().unwrap()],
                )?;

                ret_id = db.last_insert_rowid() as i32;
            }
        };

        Ok(ret_id)
    }

    pub fn add_tag(
        &self,
        tag_id: i32,
        db: PooledConnection<SqliteConnectionManager>,
    ) -> Result<(), Error> {
        let mut stmt = db.prepare("SELECT 1 FROM tags WHERE id = ?")?;
        if !stmt.exists([&tag_id])? {
            return Err(Error::TagNotFound(tag_id));
        }

        let mut stmt = db.prepare("SELECT 1 FROM entry_tag WHERE entry_id = ? AND tag_id = ?")?;
        if stmt.exists([&self.id, &tag_id])? {
            return Err(Error::TagAlreadyExistsForEntry(tag_id, self.id));
        }

        db.execute(
            "INSERT INTO entry_tag (entry_id, tag_id) VALUES (?, ?)",
            &[&self.id, &tag_id],
        )?;

        Ok(())
    }

    pub fn remove_tag(
        &self,
        tag_id: i32,
        db: PooledConnection<SqliteConnectionManager>,
    ) -> Result<(), Error> {
        let mut stmt = db.prepare("SELECT 1 FROM entry_tag WHERE entry_id = ? AND tag_id = ?")?;
        if !stmt.exists([&self.id, &tag_id])? {
            return Err(Error::TagNotFoundForEntry(tag_id, self.id));
        }

        db.execute(
            "DELETE FROM entry_tag WHERE entry_id = ? AND tag_id = ?",
            &[&self.id, &tag_id],
        )?;

        Ok(())
    }
}
