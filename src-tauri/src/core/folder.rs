use std::path::Path;

pub struct Folder {
    pub path: Box<Path>,
    pub name: String,
    pub sub_folders: Vec<Folder>,
}
