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

    let mut tag_ids: [i32; TAGS_NUM] = [0; TAGS_NUM];
    for (i, tag_id) in tag_ids.iter_mut().enumerate() {
        *tag_id = data.new_tag(format!("tag{i}"), &db).unwrap();
    }

    let tags = data.get_tags();
    assert_eq!(tags.len(), TAGS_NUM);
    for (i, tag) in tags.iter().enumerate() {
        assert_eq!(tag.children.len(), 0);
        assert_eq!(&tag.tag.name, &format!("tag{i}"));
        assert_eq!(tag.tag.color, 0);
        assert_eq!(tag.tag.parent_id, ROOT_TAG_ID);
        assert_eq!(tag.tag.children.len(), 0);
        assert_eq!(tag.tag.position, i32::try_from(i).unwrap());
    }

    for i in 0..TAGS_NUM {
        let tag_name = format!("tag{i}");
        assert_err!(
            data.new_tag(tag_name.clone(), &db),
            Err(Error::TagAlreadyExists(..))
        );
    }
}

#[test]
fn test_set_tag_color() {
    const TAGS_NUM: usize = 10;

    let (_base_path, database, _emitter) = setup_database(testdir!().as_path());

    let mut data = database.data.write().unwrap();
    let db = database.db.lock().unwrap();

    let mut expected_tag_id_to_color = HashMap::with_capacity(TAGS_NUM);
    for i in 0..TAGS_NUM {
        let tag_id = data.new_tag(format!("tag{i}"), &db).unwrap();
        let tag_color: i32 = i.try_into().unwrap();
        data.set_tag_color(tag_id, tag_color, &db).unwrap();
        expected_tag_id_to_color.insert(tag_id, tag_color);
    }

    let tags = data.get_tags();
    let mut actual_tag_id_to_color = HashMap::with_capacity(TAGS_NUM);
    for tag in tags {
        actual_tag_id_to_color.insert(tag.tag.id, tag.tag.color);
    }

    assert_eq!(actual_tag_id_to_color, expected_tag_id_to_color);
}

#[test]
fn test_reorder_tag_reparent() {
    const TAGS_NUM: usize = 10;

    let (_base_path, database, _emitter) = setup_database(testdir!().as_path());

    let mut data = database.data.write().unwrap();
    let mut db = database.db.lock().unwrap();

    let mut tag_ids: [i32; TAGS_NUM] = [0; TAGS_NUM];
    for (i, tag_id) in tag_ids.iter_mut().enumerate() {
        *tag_id = data.new_tag(format!("tag{i}"), &db).unwrap();
    }

    // Reparent tag 1..TAGS_NUM to tag 0
    for i in 1..TAGS_NUM {
        data.reorder_tag(tag_ids[i], tag_ids[0], -1, &mut db)
            .unwrap();

        let tags = data.get_tags();

        assert_eq!(tags.len(), TAGS_NUM - i);
        let tag_0 = &tags[0];
        assert_eq!(tag_0.tag.id, tag_ids[0]);
        assert_eq!(tag_0.children.len(), i);
        assert_eq!(
            tag_0.tag.children,
            tag_ids[1..=i].iter().copied().collect::<HashSet<_>>()
        );

        let tag_i = &tag_0.children[i - 1];
        assert_eq!(tag_i.tag.id, tag_ids[i]);
        assert_eq!(tag_i.tag.parent_id, tag_ids[0]);
        assert_eq!(tag_i.tag.position, i32::try_from(i).unwrap() - 1);
    }
}

#[test]
fn test_reorder_tag_same_parent() {
    const TAGS_NUM: usize = 10;

    let (_base_path, database, _emitter) = setup_database(testdir!().as_path());

    let mut data = database.data.write().unwrap();
    let mut db = database.db.lock().unwrap();

    let mut tag_ids: [i32; TAGS_NUM] = [0; TAGS_NUM];
    for (i, tag_id) in tag_ids.iter_mut().enumerate() {
        *tag_id = data.new_tag(format!("tag{i}"), &db).unwrap();
    }

    // Reparent tag 1..TAGS_NUM to tag 0
    // 0 - [1, 2, 3, 4, 5, 6, 7, 8, 9]
    //      ^  ^  ^  ^  ^  ^  ^  ^  ^
    //      0  1  2  3  4  5  6  7  8
    for i in 1..TAGS_NUM {
        data.reorder_tag(tag_ids[i], tag_ids[0], -1, &mut db)
            .unwrap();
    }
    let tags = data.get_tags();
    let child_ids = tags[0].children.iter().map(|tag| tag.tag.id);
    assert!(child_ids.eq([1, 2, 3, 4, 5, 6, 7, 8, 9].map(|i| tag_ids[i])));
    let child_positions = tags[0].children.iter().map(|tag| tag.tag.position);
    assert!(child_positions.eq([0, 1, 2, 3, 4, 5, 6, 7, 8]));

    // Reorder children of tag 0

    // Move to the end
    // Move tag 1 to position 8
    // 0 - [2, 3, 4, 5, 6, 7, 8, 9, 1]
    //      ^  ^  ^  ^  ^  ^  ^  ^  ^
    //      0  1  2  3  4  5  6  7  8
    data.reorder_tag(tag_ids[1], tag_ids[0], 8, &mut db)
        .unwrap();
    let tags = data.get_tags();
    let child_ids = tags[0].children.iter().map(|tag| tag.tag.id);
    assert!(child_ids.eq([2, 3, 4, 5, 6, 7, 8, 9, 1].map(|i| tag_ids[i])));
    let child_positions = tags[0].children.iter().map(|tag| tag.tag.position);
    assert!(child_positions.eq([0, 1, 2, 3, 4, 5, 6, 7, 8]));

    // Move to the front
    // Move tag 1 to position 0
    // 0 - [1, 2, 3, 4, 5, 6, 7, 8, 9]
    //      ^  ^  ^  ^  ^  ^  ^  ^  ^
    //      0  1  2  3  4  5  6  7  8
    data.reorder_tag(tag_ids[1], tag_ids[0], 0, &mut db)
        .unwrap();
    let tags = data.get_tags();
    let child_ids = tags[0].children.iter().map(|tag| tag.tag.id);
    assert!(child_ids.eq([1, 2, 3, 4, 5, 6, 7, 8, 9].map(|i| tag_ids[i])));
    let child_positions = tags[0].children.iter().map(|tag| tag.tag.position);
    assert!(child_positions.eq([0, 1, 2, 3, 4, 5, 6, 7, 8]));

    // Move to the middle
    // Move tag 7 to position 2
    // 0 - [1, 2, 7, 3, 4, 5, 6, 8, 9]
    //      ^  ^  ^  ^  ^  ^  ^  ^  ^
    //      0  1  2  3  4  5  6  7  8
    data.reorder_tag(tag_ids[7], tag_ids[0], 2, &mut db)
        .unwrap();
    let tags = data.get_tags();
    let child_ids = tags[0].children.iter().map(|tag| tag.tag.id);
    assert!(child_ids.eq([1, 2, 7, 3, 4, 5, 6, 8, 9].map(|i| tag_ids[i])));
    let child_positions = tags[0].children.iter().map(|tag| tag.tag.position);
    assert!(child_positions.eq([0, 1, 2, 3, 4, 5, 6, 7, 8]));
}

#[test]
fn test_reorder_tag_across_parents() {
    const TAGS_NUM: usize = 10;

    let (_base_path, database, _emitter) = setup_database(testdir!().as_path());

    let mut data = database.data.write().unwrap();
    let mut db = database.db.lock().unwrap();

    let mut tag_ids: [i32; TAGS_NUM] = [0; TAGS_NUM];
    for (i, tag_id) in tag_ids.iter_mut().enumerate() {
        *tag_id = data.new_tag(format!("tag{i}"), &db).unwrap();
    }

    // Reparent tag 2..=5 to tag 0
    // Reparent tag 6..=9 to tag 1
    // 0 - [2, 3, 4, 5], 1 - [6, 7, 8, 9]
    //      ^  ^  ^  ^        ^  ^  ^  ^
    //      0  1  2  3        0  1  2  3
    for i in 2..=5 {
        data.reorder_tag(tag_ids[i], tag_ids[0], -1, &mut db)
            .unwrap();
    }
    for i in 6..=9 {
        data.reorder_tag(tag_ids[i], tag_ids[1], -1, &mut db)
            .unwrap();
    }
    let tags = data.get_tags();
    let child0_ids = tags[0].children.iter().map(|tag| tag.tag.id);
    assert!(child0_ids.eq([2, 3, 4, 5].map(|i| tag_ids[i])));
    let child0_positions = tags[0].children.iter().map(|tag| tag.tag.position);
    assert!(child0_positions.eq([0, 1, 2, 3]));
    let child1_ids = tags[1].children.iter().map(|tag| tag.tag.id);
    assert!(child1_ids.eq([6, 7, 8, 9].map(|i| tag_ids[i])));
    let child1_positions = tags[1].children.iter().map(|tag| tag.tag.position);
    assert!(child1_positions.eq([0, 1, 2, 3]));

    // Move tag 4 to tag 1 at position 1
    // 0 - [2, 3, 5], 1 - [6, 4, 7, 8, 9]
    //      ^  ^  ^        ^  ^  ^  ^  ^
    //      0  1  2        0  1  2  3  4
    data.reorder_tag(tag_ids[4], tag_ids[1], 1, &mut db)
        .unwrap();
    let tags = data.get_tags();
    let child0_ids = tags[0].children.iter().map(|tag| tag.tag.id);
    assert!(child0_ids.eq([2, 3, 5].map(|i| tag_ids[i])));
    let child0_positions = tags[0].children.iter().map(|tag| tag.tag.position);
    assert!(child0_positions.eq([0, 1, 2]));
    let child1_ids = tags[1].children.iter().map(|tag| tag.tag.id);
    assert!(child1_ids.eq([6, 4, 7, 8, 9].map(|i| tag_ids[i])));
    let child1_positions = tags[1].children.iter().map(|tag| tag.tag.position);
    assert!(child1_positions.eq([0, 1, 2, 3, 4]));

    // Move tag 8 to tag 0 at position 1
    // 0 - [2, 8, 3, 5], 1 - [6, 4, 7, 9]
    //      ^  ^  ^  ^        ^  ^  ^  ^
    //      0  1  2  3        0  1  2  3
    data.reorder_tag(tag_ids[8], tag_ids[0], 1, &mut db)
        .unwrap();
    let tags = data.get_tags();
    let child0_ids = tags[0].children.iter().map(|tag| tag.tag.id);
    assert!(child0_ids.eq([2, 8, 3, 5].map(|i| tag_ids[i])));
    let child0_positions = tags[0].children.iter().map(|tag| tag.tag.position);
    assert!(child0_positions.eq([0, 1, 2, 3]));
    let child1_ids = tags[1].children.iter().map(|tag| tag.tag.id);
    assert!(child1_ids.eq([6, 4, 7, 9].map(|i| tag_ids[i])));
    let child1_positions = tags[1].children.iter().map(|tag| tag.tag.position);
    assert!(child1_positions.eq([0, 1, 2, 3]));

    // Move tag 0 to tag 1 at position 2
    // 1 - [6, 4, 0 - [2, 8, 3, 5], 7, 9]
    //                 ^  ^  ^  ^
    //                 0  1  2  3
    //      ^  ^  ^                 ^  ^
    //      0  1  2                 3  4
    data.reorder_tag(tag_ids[0], tag_ids[1], 2, &mut db)
        .unwrap();
    let tags = data.get_tags();
    let tag_1 = &tags[0];
    let child1_ids = tag_1.children.iter().map(|tag| tag.tag.id);
    assert!(child1_ids.eq([6, 4, 0, 7, 9].map(|i| tag_ids[i])));
    let child1_positions = tag_1.children.iter().map(|tag| tag.tag.position);
    assert!(child1_positions.eq([0, 1, 2, 3, 4]));
    let tag_0 = &tags[0].children[2];
    let child0_ids = tag_0.children.iter().map(|tag| tag.tag.id);
    assert!(child0_ids.eq([2, 8, 3, 5].map(|i| tag_ids[i])));
    let child0_positions = tag_0.children.iter().map(|tag| tag.tag.position);
    assert!(child0_positions.eq([0, 1, 2, 3]));
}

#[test]
fn test_read_tags() {
    const TAGS_NUM: usize = 10;

    let (_base_path, database, _emitter) = setup_database(testdir!().as_path());

    let mut tag_ids: [i32; TAGS_NUM] = [0; TAGS_NUM];
    {
        let mut data = database.data.write().unwrap();
        let mut db = database.db.lock().unwrap();

        for (i, tag_id) in tag_ids.iter_mut().enumerate() {
            *tag_id = data.new_tag(format!("tag{i}"), &db).unwrap();
        }

        // Reparent tag 2..=5 to tag 0
        // Reparent tag 6..=9 to tag 1
        // 0 - [2, 3, 4, 5], 1 - [6, 7, 8, 9]
        //      ^  ^  ^  ^        ^  ^  ^  ^
        //      0  1  2  3        0  1  2  3
        for i in 2..=5 {
            data.reorder_tag(tag_ids[i], tag_ids[0], -1, &mut db)
                .unwrap();
        }
        for i in 6..=9 {
            data.reorder_tag(tag_ids[i], tag_ids[1], -1, &mut db)
                .unwrap();
        }

        // Move tag 4 to tag 1 at position 1
        // 0 - [2, 3, 5], 1 - [6, 4, 7, 8, 9]
        //      ^  ^  ^        ^  ^  ^  ^  ^
        //      0  1  2        0  1  2  3  4
        data.reorder_tag(tag_ids[4], tag_ids[1], 1, &mut db)
            .unwrap();

        // Move tag 8 to tag 0 at position 1
        // 0 - [2, 8, 3, 5], 1 - [6, 4, 7, 9]
        //      ^  ^  ^  ^        ^  ^  ^  ^
        //      0  1  2  3        0  1  2  3
        data.reorder_tag(tag_ids[8], tag_ids[0], 1, &mut db)
            .unwrap();

        // Move tag 0 to tag 1 at position 2
        // 1 - [6, 4, 0 - [2, 8, 3, 5], 7, 9]
        //                 ^  ^  ^  ^
        //                 0  1  2  3
        //      ^  ^  ^                 ^  ^
        //      0  1  2                 3  4
        data.reorder_tag(tag_ids[0], tag_ids[1], 2, &mut db)
            .unwrap();
        let tags = data.get_tags();
        let tag_1 = &tags[0];
        let child1_ids = tag_1.children.iter().map(|tag| tag.tag.id);
        assert!(child1_ids.eq([6, 4, 0, 7, 9].map(|i| tag_ids[i])));
        let child1_positions = tag_1.children.iter().map(|tag| tag.tag.position);
        assert!(child1_positions.eq([0, 1, 2, 3, 4]));
        let tag_0 = &tags[0].children[2];
        let child0_ids = tag_0.children.iter().map(|tag| tag.tag.id);
        assert!(child0_ids.eq([2, 8, 3, 5].map(|i| tag_ids[i])));
        let child0_positions = tag_0.children.iter().map(|tag| tag.tag.position);
        assert!(child0_positions.eq([0, 1, 2, 3]));
    }
    drop(database);

    let database = Database::open(_base_path, TestEmitter::new()).unwrap();
    let data = database.data.read().unwrap();
    let tags = data.get_tags();
    let tag_1 = &tags[0];
    let child1_ids = tag_1.children.iter().map(|tag| tag.tag.id);
    assert!(child1_ids.eq([6, 4, 0, 7, 9].map(|i| tag_ids[i])));
    let child1_positions = tag_1.children.iter().map(|tag| tag.tag.position);
    assert!(child1_positions.eq([0, 1, 2, 3, 4]));
    let tag_0 = &tags[0].children[2];
    let child0_ids = tag_0.children.iter().map(|tag| tag.tag.id);
    assert!(child0_ids.eq([2, 8, 3, 5].map(|i| tag_ids[i])));
    let child0_positions = tag_0.children.iter().map(|tag| tag.tag.position);
    assert!(child0_positions.eq([0, 1, 2, 3]));
}
