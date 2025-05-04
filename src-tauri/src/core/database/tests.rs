use super::{Database, DatabaseEmitter, Error, ROOT_FOLDER_ID, ROOT_TAG_ID};

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs::{create_dir, exists, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use testdir::testdir;

macro_rules! assert_err {
    ( $expression:expr, $($pattern:tt)+ ) => {
        match $expression {
            $($pattern)+ => (),
            ref e => panic!("expected `{}` but got `{:?}`", stringify!($($pattern)+), e),
        }
    }
}

#[derive(Clone)]
struct TestEmitter {
    files_updated: Arc<AtomicBool>,
}

impl TestEmitter {
    fn new() -> Self {
        TestEmitter {
            files_updated: Arc::new(AtomicBool::new(false)),
        }
    }

    fn files_updated(&self) -> bool {
        self.files_updated
            .swap(false, std::sync::atomic::Ordering::AcqRel)
    }
}

impl DatabaseEmitter for TestEmitter {
    fn on_files_updated(&self) {
        self.files_updated
            .store(true, std::sync::atomic::Ordering::Release);
    }
}

fn setup_files(dir: &Path) -> PathBuf {
    let base_path = dir.join("test_database");
    if base_path.exists() {
        std::fs::remove_dir_all(&base_path).unwrap();
    }

    std::fs::create_dir_all(&base_path).unwrap();

    // create directories
    create_dir(base_path.join("folder1")).unwrap();
    create_dir(base_path.join("folder2")).unwrap();

    create_dir(base_path.join("folder1/folder1-1")).unwrap();
    create_dir(base_path.join("folder1/folder1-2")).unwrap();

    // create files
    File::create(base_path.join("wave_audio_1.wav")).unwrap();
    File::create(base_path.join("flac_audio_1.flac")).unwrap();
    File::create(base_path.join("mp3_audio_1.mp3")).unwrap();
    File::create(base_path.join("ogg_audio_1.ogg")).unwrap();

    File::create(base_path.join("folder1/wave_audio_2.wav")).unwrap();
    File::create(base_path.join("folder1/folder1-1/flac_audio_2.flac")).unwrap();
    File::create(base_path.join("folder1/folder1-2/mp3_audio_2.mp3")).unwrap();
    File::create(base_path.join("folder2/ogg_audio_2.ogg")).unwrap();

    base_path
}

fn setup_database(dir: &Path) -> (PathBuf, Arc<Database<TestEmitter>>, TestEmitter) {
    let base_path = setup_files(dir);
    let emitter = TestEmitter::new();
    let database = Database::create(base_path.clone(), emitter.clone()).unwrap();
    (base_path, database, emitter)
}

#[test]
fn test_create_database() {
    let (base_path, _database, _emitter) = setup_database(testdir!().as_path());

    assert!(exists(base_path.join(".soundmanager.db")).unwrap());
}

#[test]
fn test_open_database() {
    let base_path = {
        let (base_path, _database, _emitter) = setup_database(testdir!().as_path());
        base_path
    };

    let emitter = TestEmitter::new();
    let _database = Database::open(base_path.clone(), emitter).unwrap();
}

#[test]
fn test_refresh() {
    let (_base_path, database, emitter) = setup_database(testdir!().as_path());

    database.refresh().unwrap();

    assert!(emitter.files_updated());
}

#[test]
fn test_get_entries() {
    let (_base_path, database, _emitter) = setup_database(testdir!().as_path());

    let data = database.data.read().unwrap();
    let entries = data.get_entries();

    assert_eq!(entries.len(), 8);

    let actual = entries
        .values()
        .map(|entry| entry.file_name.clone())
        .collect::<HashSet<_>>();

    let expected = [
        "wave_audio_1.wav",
        "flac_audio_1.flac",
        "mp3_audio_1.mp3",
        "ogg_audio_1.ogg",
        "wave_audio_2.wav",
        "flac_audio_2.flac",
        "mp3_audio_2.mp3",
        "ogg_audio_2.ogg",
    ]
    .iter()
    .map(OsString::from)
    .collect::<HashSet<_>>();

    assert_eq!(actual, expected);
}

#[test]
fn test_get_folders() {
    let (_base_path, database, _emitter) = setup_database(testdir!().as_path());

    let data = database.data.read().unwrap();

    let root = data.get_folders();
    assert_eq!(root.folder.name, "test_database");
    assert_eq!(root.folder.id, ROOT_FOLDER_ID);
    assert_eq!(root.folder.parent_id, ROOT_FOLDER_ID);
    assert_eq!(root.folder.path, Path::new(""));
    assert_eq!(root.folder.sub_folders.len(), 2);
    assert_eq!(root.folder.entries.len(), 4);

    assert_eq!(root.sub_folders.len(), 2);

    let folder1 = root
        .sub_folders
        .iter()
        .find(|node| node.folder.name == "folder1")
        .unwrap();
    assert_eq!(folder1.folder.name, "folder1");
    assert_eq!(
        &folder1.folder.id,
        root.folder
            .sub_folders
            .get(&OsString::from("folder1"))
            .unwrap()
    );
    assert_eq!(folder1.folder.parent_id, root.folder.id);
    assert_eq!(folder1.folder.path, Path::new("folder1"));
    assert_eq!(folder1.folder.sub_folders.len(), 2);
    assert_eq!(folder1.folder.entries.len(), 1);

    let folder1_1 = folder1
        .sub_folders
        .iter()
        .find(|node| node.folder.name == "folder1-1")
        .unwrap();
    assert_eq!(folder1_1.folder.name, "folder1-1");
    assert_eq!(
        &folder1_1.folder.id,
        folder1
            .folder
            .sub_folders
            .get(&OsString::from("folder1-1"))
            .unwrap()
    );
    assert_eq!(folder1_1.folder.parent_id, folder1.folder.id);
    assert_eq!(folder1_1.folder.path, Path::new("folder1/folder1-1"));
    assert_eq!(folder1_1.folder.sub_folders.len(), 0);
    assert_eq!(folder1_1.folder.entries.len(), 1);

    let folder1_2 = folder1
        .sub_folders
        .iter()
        .find(|node| node.folder.name == "folder1-2")
        .unwrap();
    assert_eq!(folder1_2.folder.name, "folder1-2");
    assert_eq!(
        &folder1_2.folder.id,
        folder1
            .folder
            .sub_folders
            .get(&OsString::from("folder1-2"))
            .unwrap()
    );
    assert_eq!(folder1_2.folder.parent_id, folder1.folder.id);
    assert_eq!(folder1_2.folder.path, Path::new("folder1/folder1-2"));
    assert_eq!(folder1_2.folder.sub_folders.len(), 0);
    assert_eq!(folder1_2.folder.entries.len(), 1);

    let folder2 = root
        .sub_folders
        .iter()
        .find(|node| node.folder.name == "folder2")
        .unwrap();
    assert_eq!(folder2.folder.name, "folder2");
    assert_eq!(
        &folder2.folder.id,
        root.folder
            .sub_folders
            .get(&OsString::from("folder2"))
            .unwrap()
    );
    assert_eq!(folder2.folder.parent_id, root.folder.id);
    assert_eq!(folder2.folder.path, Path::new("folder2"));
    assert_eq!(folder2.folder.sub_folders.len(), 0);
    assert_eq!(folder2.folder.entries.len(), 1);
}

#[test]
fn test_new_tag() {
    const TAGS_NUM: usize = 10;

    let (_base_path, database, _emitter) = setup_database(testdir!().as_path());

    let mut data = database.data.write().unwrap();
    let db = database.db.lock().unwrap();

    let mut tag_id_to_name = HashMap::with_capacity(TAGS_NUM);
    for i in 0..TAGS_NUM {
        let tag_name = format!("tag{i}");
        let tag_id = data.new_tag(tag_name.clone(), &db).unwrap();
        tag_id_to_name.insert(tag_id, tag_name);
    }

    let tags = data.get_tags();
    let mut positions = HashSet::new();
    assert_eq!(tags.len(), TAGS_NUM);
    for tag in tags {
        assert_eq!(tag.children.len(), 0);
        assert_eq!(&tag.tag.name, tag_id_to_name.get(&tag.tag.id).unwrap());
        assert_eq!(tag.tag.color, 0);
        assert_eq!(tag.tag.parent_id, ROOT_TAG_ID);
        assert_eq!(tag.tag.children.len(), 0);
        assert!(positions.insert(tag.tag.position)); // position must be unique
    }
    assert_eq!(
        positions,
        (0..TAGS_NUM.try_into().unwrap()).collect::<HashSet<i32>>()
    );

    for i in 0..TAGS_NUM {
        let tag_name = format!("tag{i}");
        assert_err!(
            data.new_tag(tag_name.clone(), &db),
            Err(Error::TagAlreadyExists(..))
        );
    }
}
