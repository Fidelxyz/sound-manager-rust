use super::{Database, DatabaseData, DatabaseEmitter, EntryId, Error, Result};

use std::fs::copy;
use std::path::Path;
use trash::delete;

use open;

impl<E> Database<E>
where
    E: DatabaseEmitter + Send + Sync + 'static,
{
    pub fn delete_file(&self, entry_id: EntryId) -> Result<()> {
        let mut data = self.data.write().unwrap();

        let path = data.to_absolute_path(&data.get_entry(entry_id).unwrap().path);
        delete(path)?;

        data.remove_entry(entry_id, &self.db.lock().unwrap())?;
        self.emitter.on_files_updated();

        Ok(())
    }
}

impl DatabaseData {
    pub fn spot(
        &self,
        entry_id: EntryId,
        save_path: Option<&Path>,
        open_in_application: Option<&Path>,
        force: bool,
    ) -> Result<()> {
        let entry = self.get_entry(entry_id).unwrap();
        let entry_path = self.to_absolute_path(&entry.path);

        // file to be opened in application
        let src_file_path = if let Some(save_path) = save_path {
            // save file to the new location
            let to_path = save_path.join(&entry.file_name);

            if !force && to_path.exists() {
                return Err(Error::FileAlreadyExists(to_path.to_string_lossy().into()));
            }

            copy(entry_path, &to_path)?;
            to_path
        } else {
            entry_path
        };

        // open in application
        if let Some(open_in_application) = open_in_application {
            if open_in_application.exists() {
                open::with(src_file_path, open_in_application.to_string_lossy())?;
            }
        }

        Ok(())
    }
}
