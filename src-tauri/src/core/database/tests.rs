use super::{Database, DatabaseEmitter, Error, ROOT_FOLDER_ID, ROOT_TAG_ID};

use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs::{create_dir, exists, remove_dir_all, remove_file, rename, File};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use test_log::test;
use testdir::testdir;

const EMITTER_TIMEOUT: Duration = Duration::from_secs(2);

macro_rules! assert_err {
    ( $expression:expr, $($pattern:tt)+ ) => {
        match $expression {
            $($pattern)+ => (),
            ref e => panic!("expected `{}` but got `{:?}`", stringify!($($pattern)+), e),
        }
    }
}

struct TestEmitter {
    condvar: Condvar,
    files_updated: Mutex<bool>,
}

impl TestEmitter {
    fn new() -> Self {
        Self {
            condvar: Condvar::new(),
            files_updated: Mutex::new(false),
        }
    }

    fn wait_for_files_updated(&self, timeout: Duration) -> bool {
        let mut files_updated_guard = self.files_updated.lock().unwrap();
        files_updated_guard = self
            .condvar
            .wait_timeout_while(files_updated_guard, timeout, |&mut files_updated| {
                !files_updated
            })
            .unwrap()
            .0;

        let files_updated = *files_updated_guard;
        *files_updated_guard = false;
        files_updated
    }
}

impl DatabaseEmitter for TestEmitter {
    fn on_files_updated(&self, _immediate: bool) {
        let mut files_updated = self.files_updated.lock().unwrap();
        *files_updated = true;
        self.condvar.notify_all();
    }
}

fn setup_files(dir: &Path) -> PathBuf {
    let base_path = dir.join("test_database");
    if base_path.exists() {
        remove_dir_all(&base_path).unwrap();
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

fn setup_database(dir: &Path) -> (PathBuf, Arc<Database<TestEmitter>>, Arc<TestEmitter>) {
    let base_path = setup_files(dir);
    let emitter = Arc::new(TestEmitter::new());
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

    let emitter = Arc::new(TestEmitter::new());
    let _database = Database::open(base_path.clone(), emitter).unwrap();
}

#[test]
fn test_refresh() {
    let (_base_path, database, emitter) = setup_database(testdir!().as_path());

    database.refresh().unwrap();

    assert!(emitter.wait_for_files_updated(EMITTER_TIMEOUT));
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

    let folders = data.get_folders();

    let root_folder = &folders[&ROOT_FOLDER_ID];
    assert_eq!(root_folder.name, "test_database");
    assert_eq!(root_folder.id, ROOT_FOLDER_ID);
    assert_eq!(root_folder.parent_id, ROOT_FOLDER_ID);
    assert_eq!(root_folder.path, Path::new(""));
    assert_eq!(root_folder.sub_folders.len(), 2);
    assert_eq!(root_folder.entries.len(), 4);

    let folder1 = &folders[&root_folder.sub_folders[OsStr::new("folder1")]];
    assert_eq!(folder1.name, "folder1");
    assert_eq!(folder1.parent_id, root_folder.id);
    assert_eq!(folder1.path, Path::new("folder1"));
    assert_eq!(folder1.sub_folders.len(), 2);
    assert_eq!(folder1.entries.len(), 1);

    let folder1_1 = &folders[&folder1.sub_folders[OsStr::new("folder1-1")]];
    assert_eq!(folder1_1.name, "folder1-1");
    assert_eq!(folder1_1.parent_id, folder1.id);
    assert_eq!(folder1_1.path, Path::new("folder1/folder1-1"));
    assert_eq!(folder1_1.sub_folders.len(), 0);
    assert_eq!(folder1_1.entries.len(), 1);

    let folder1_2 = &folders[&folder1.sub_folders[OsStr::new("folder1-2")]];
    assert_eq!(folder1_2.name, "folder1-2");
    assert_eq!(folder1_2.parent_id, folder1.id);
    assert_eq!(folder1_2.path, Path::new("folder1/folder1-2"));
    assert_eq!(folder1_2.sub_folders.len(), 0);
    assert_eq!(folder1_2.entries.len(), 1);

    let folder2 = &folders[&root_folder.sub_folders[OsStr::new("folder2")]];
    assert_eq!(folder2.name, "folder2");
    assert_eq!(folder2.parent_id, root_folder.id);
    assert_eq!(folder2.path, Path::new("folder2"));
    assert_eq!(folder2.sub_folders.len(), 0);
    assert_eq!(folder2.entries.len(), 1);
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
    assert_eq!(tags[&ROOT_TAG_ID].children, tag_ids.into());
    for i in 0..TAGS_NUM {
        let tag_i = &tags[&tag_ids[i]];
        assert_eq!(tag_i.name, format!("tag{i}"));
        assert_eq!(tag_i.color, 0);
        assert_eq!(tag_i.parent_id, ROOT_TAG_ID);
        assert_eq!(tag_i.children.len(), 0);
        assert_eq!(tag_i.position, i32::try_from(i).unwrap());
    }

    for i in 0..TAGS_NUM {
        assert_err!(
            data.new_tag(format!("tag{i}"), &db),
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
    let tags = tags[&ROOT_TAG_ID]
        .children
        .iter()
        .map(|tag_id| &tags[tag_id]);
    let mut actual_tag_id_to_color = HashMap::with_capacity(TAGS_NUM);
    for tag in tags {
        actual_tag_id_to_color.insert(tag.id, tag.color);
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

        assert_eq!(tags[&ROOT_TAG_ID].children.len(), TAGS_NUM - i);
        let tag_0 = &tags[&tag_ids[0]];
        assert_eq!(tag_0.id, tag_ids[0]);
        assert_eq!(
            tag_0.children,
            tag_ids[1..=i].iter().copied().collect::<HashSet<_>>()
        );

        let child_tags = &mut tag_0
            .children
            .iter()
            .map(|tag_id| &tags[tag_id])
            .collect::<Vec<_>>();
        child_tags.sort_unstable_by_key(|tag| tag.position);

        let tag_i = &child_tags[i - 1];
        assert_eq!(tag_i.id, tag_ids[i]);
        assert_eq!(tag_i.parent_id, tag_ids[0]);
        assert_eq!(tag_i.position, i32::try_from(i).unwrap() - 1);
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
    let mut child_tags = tags[&tag_ids[0]]
        .children
        .iter()
        .map(|tag_id| &tags[tag_id])
        .collect::<Vec<_>>();
    child_tags.sort_unstable_by_key(|tag| tag.position);
    let child_ids = child_tags.iter().map(|tag| tag.id);
    assert!(child_ids.eq([1, 2, 3, 4, 5, 6, 7, 8, 9].map(|i| tag_ids[i])));
    let child_positions = child_tags.iter().map(|tag| tag.position);
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
    let mut child_tags = tags[&tag_ids[0]]
        .children
        .iter()
        .map(|tag_id| &tags[tag_id])
        .collect::<Vec<_>>();
    child_tags.sort_unstable_by_key(|tag| tag.position);
    let child_ids = child_tags.iter().map(|tag| tag.id).collect::<Vec<_>>();
    assert_eq!(child_ids, [2, 3, 4, 5, 6, 7, 8, 9, 1].map(|i| tag_ids[i]));
    let child_positions = child_tags.iter().map(|tag| tag.position);
    assert!(child_positions.eq([0, 1, 2, 3, 4, 5, 6, 7, 8]));

    // Move to the front
    // Move tag 1 to position 0
    // 0 - [1, 2, 3, 4, 5, 6, 7, 8, 9]
    //      ^  ^  ^  ^  ^  ^  ^  ^  ^
    //      0  1  2  3  4  5  6  7  8
    data.reorder_tag(tag_ids[1], tag_ids[0], 0, &mut db)
        .unwrap();
    let tags = data.get_tags();
    let mut child_tags = tags[&tag_ids[0]]
        .children
        .iter()
        .map(|tag_id| &tags[tag_id])
        .collect::<Vec<_>>();
    child_tags.sort_unstable_by_key(|tag| tag.position);
    let child_ids = child_tags.iter().map(|tag| tag.id).collect::<Vec<_>>();
    assert_eq!(child_ids, [1, 2, 3, 4, 5, 6, 7, 8, 9].map(|i| tag_ids[i]));
    let child_positions = child_tags.iter().map(|tag| tag.position);
    assert!(child_positions.eq([0, 1, 2, 3, 4, 5, 6, 7, 8]));

    // Move to the middle
    // Move tag 7 to position 2
    // 0 - [1, 2, 7, 3, 4, 5, 6, 8, 9]
    //      ^  ^  ^  ^  ^  ^  ^  ^  ^
    //      0  1  2  3  4  5  6  7  8
    data.reorder_tag(tag_ids[7], tag_ids[0], 2, &mut db)
        .unwrap();
    let tags = data.get_tags();
    let mut child_tags = tags[&tag_ids[0]]
        .children
        .iter()
        .map(|tag_id| &tags[tag_id])
        .collect::<Vec<_>>();
    child_tags.sort_unstable_by_key(|tag| tag.position);
    let child_ids = child_tags.iter().map(|tag| tag.id).collect::<Vec<_>>();
    assert_eq!(child_ids, [1, 2, 7, 3, 4, 5, 6, 8, 9].map(|i| tag_ids[i]));
    let child_positions = child_tags.iter().map(|tag| tag.position);
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
    let mut child0_tags = tags[&tag_ids[0]]
        .children
        .iter()
        .map(|tag_id| &tags[tag_id])
        .collect::<Vec<_>>();
    child0_tags.sort_unstable_by_key(|tag| tag.position);
    let child0_ids = child0_tags.iter().map(|tag| tag.id);
    assert!(child0_ids.eq([2, 3, 4, 5].map(|i| tag_ids[i])));
    let child0_positions = child0_tags.iter().map(|tag| tag.position);
    assert!(child0_positions.eq([0, 1, 2, 3]));
    let mut child1_tags = tags[&tag_ids[1]]
        .children
        .iter()
        .map(|tag_id| &tags[tag_id])
        .collect::<Vec<_>>();
    child1_tags.sort_unstable_by_key(|tag| tag.position);
    let child1_ids = child1_tags.iter().map(|tag| tag.id);
    assert!(child1_ids.eq([6, 7, 8, 9].map(|i| tag_ids[i])));
    let child1_positions = child1_tags.iter().map(|tag| tag.position);
    assert!(child1_positions.eq([0, 1, 2, 3]));

    // Move tag 4 to tag 1 at position 1
    // 0 - [2, 3, 5], 1 - [6, 4, 7, 8, 9]
    //      ^  ^  ^        ^  ^  ^  ^  ^
    //      0  1  2        0  1  2  3  4
    data.reorder_tag(tag_ids[4], tag_ids[1], 1, &mut db)
        .unwrap();
    let tags = data.get_tags();
    let mut child0_tags = tags[&tag_ids[0]]
        .children
        .iter()
        .map(|tag_id| &tags[tag_id])
        .collect::<Vec<_>>();
    child0_tags.sort_unstable_by_key(|tag| tag.position);
    let child0_ids = child0_tags.iter().map(|tag| tag.id);
    assert!(child0_ids.eq([2, 3, 5].map(|i| tag_ids[i])));
    let child0_positions = child0_tags.iter().map(|tag| tag.position);
    assert!(child0_positions.eq([0, 1, 2]));
    let mut child1_tags = tags[&tag_ids[1]]
        .children
        .iter()
        .map(|tag_id| &tags[tag_id])
        .collect::<Vec<_>>();
    child1_tags.sort_unstable_by_key(|tag| tag.position);
    let child1_ids = child1_tags.iter().map(|tag| tag.id);
    assert!(child1_ids.eq([6, 4, 7, 8, 9].map(|i| tag_ids[i])));
    let child1_positions = child1_tags.iter().map(|tag| tag.position);
    assert!(child1_positions.eq([0, 1, 2, 3, 4]));

    // Move tag 8 to tag 0 at position 1
    // 0 - [2, 8, 3, 5], 1 - [6, 4, 7, 9]
    //      ^  ^  ^  ^        ^  ^  ^  ^
    //      0  1  2  3        0  1  2  3
    data.reorder_tag(tag_ids[8], tag_ids[0], 1, &mut db)
        .unwrap();
    let tags = data.get_tags();
    let mut child0_tags = tags[&tag_ids[0]]
        .children
        .iter()
        .map(|tag_id| &tags[tag_id])
        .collect::<Vec<_>>();
    child0_tags.sort_unstable_by_key(|tag| tag.position);
    let child0_ids = child0_tags.iter().map(|tag| tag.id);
    assert!(child0_ids.eq([2, 8, 3, 5].map(|i| tag_ids[i])));
    let child0_positions = child0_tags.iter().map(|tag| tag.position);
    assert!(child0_positions.eq([0, 1, 2, 3]));
    let mut child1_tags = tags[&tag_ids[1]]
        .children
        .iter()
        .map(|tag_id| &tags[tag_id])
        .collect::<Vec<_>>();
    child1_tags.sort_unstable_by_key(|tag| tag.position);
    let child1_ids = child1_tags.iter().map(|tag| tag.id);
    assert!(child1_ids.eq([6, 4, 7, 9].map(|i| tag_ids[i])));
    let child1_positions = child1_tags.iter().map(|tag| tag.position);
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
    let mut child1_tags = tags[&tag_ids[1]]
        .children
        .iter()
        .map(|tag_id| &tags[tag_id])
        .collect::<Vec<_>>();
    child1_tags.sort_unstable_by_key(|tag| tag.position);
    let child1_ids = child1_tags.iter().map(|tag| tag.id);
    assert!(child1_ids.eq([6, 4, 0, 7, 9].map(|i| tag_ids[i])));
    let child1_positions = child1_tags.iter().map(|tag| tag.position);
    assert!(child1_positions.eq([0, 1, 2, 3, 4]));
    let mut child0_tags = tags[&tag_ids[0]]
        .children
        .iter()
        .map(|tag_id| &tags[tag_id])
        .collect::<Vec<_>>();
    child0_tags.sort_unstable_by_key(|tag| tag.position);
    let child0_ids = child0_tags.iter().map(|tag| tag.id);
    assert!(child0_ids.eq([2, 8, 3, 5].map(|i| tag_ids[i])));
    let child0_positions = child0_tags.iter().map(|tag| tag.position);
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
        let mut child1_tags = tags[&tag_ids[1]]
            .children
            .iter()
            .map(|tag_id| &tags[tag_id])
            .collect::<Vec<_>>();
        child1_tags.sort_unstable_by_key(|tag| tag.position);
        let child1_ids = child1_tags.iter().map(|tag| tag.id);
        assert!(child1_ids.eq([6, 4, 0, 7, 9].map(|i| tag_ids[i])));
        let child1_positions = child1_tags.iter().map(|tag| tag.position);
        assert!(child1_positions.eq([0, 1, 2, 3, 4]));
        let mut child0_tags = tags[&tag_ids[0]]
            .children
            .iter()
            .map(|tag_id| &tags[tag_id])
            .collect::<Vec<_>>();
        child0_tags.sort_unstable_by_key(|tag| tag.position);
        let child0_ids = child0_tags.iter().map(|tag| tag.id);
        assert!(child0_ids.eq([2, 8, 3, 5].map(|i| tag_ids[i])));
        let child0_positions = child0_tags.iter().map(|tag| tag.position);
        assert!(child0_positions.eq([0, 1, 2, 3]));
    }
    drop(database);

    let database = Database::open(_base_path, Arc::new(TestEmitter::new())).unwrap();
    let data = database.data.read().unwrap();
    let tags = data.get_tags();
    let mut child1_tags = tags[&tag_ids[1]]
        .children
        .iter()
        .map(|tag_id| &tags[tag_id])
        .collect::<Vec<_>>();
    child1_tags.sort_unstable_by_key(|tag| tag.position);
    let child1_ids = child1_tags.iter().map(|tag| tag.id);
    assert!(child1_ids.eq([6, 4, 0, 7, 9].map(|i| tag_ids[i])));
    let child1_positions = child1_tags.iter().map(|tag| tag.position);
    assert!(child1_positions.eq([0, 1, 2, 3, 4]));
    let mut child0_tags = tags[&tag_ids[0]]
        .children
        .iter()
        .map(|tag_id| &tags[tag_id])
        .collect::<Vec<_>>();
    child0_tags.sort_unstable_by_key(|tag| tag.position);
    let child0_ids = child0_tags.iter().map(|tag| tag.id);
    assert!(child0_ids.eq([2, 8, 3, 5].map(|i| tag_ids[i])));
    let child0_positions = child0_tags.iter().map(|tag| tag.position);
    assert!(child0_positions.eq([0, 1, 2, 3]));
}

#[test]
fn test_rename_tag() {
    const TAGS_NUM: usize = 3;
    let mut tag_ids: [i32; TAGS_NUM] = [0; TAGS_NUM];

    let (_base_path, database, _emitter) = setup_database(testdir!().as_path());

    {
        let mut data = database.data.write().unwrap();
        let db = database.db.lock().unwrap();

        for (i, tag_id) in tag_ids.iter_mut().enumerate() {
            *tag_id = data.new_tag(format!("tag{i}"), &db).unwrap();
        }

        // Rename tag 1 to "renamed_tag"
        data.rename_tag(tag_ids[1], "renamed_tag".to_string(), &db)
            .unwrap();

        // Tag 1 should now have the new name
        let tags = data.get_tags();
        assert_eq!(tags[&tag_ids[1]].name, "renamed_tag");

        // Other tags should remain unchanged
        assert_eq!(tags[&tag_ids[0]].name, "tag0");
        assert_eq!(tags[&tag_ids[2]].name, "tag2");

        // Rename to an existing tag name should fail
        assert_err!(
            data.rename_tag(tag_ids[0], "renamed_tag".to_string(), &db),
            Err(Error::TagAlreadyExists(..))
        );
    }
    drop(database);

    // Reopen the database to verify the changes persist
    let database = Database::open(_base_path, Arc::new(TestEmitter::new())).unwrap();
    let mut data = database.data.write().unwrap();
    let db = database.db.lock().unwrap();

    // Tag 1 should now have the new name
    let tags = data.get_tags();
    assert_eq!(tags[&tag_ids[1]].name, "renamed_tag");

    // Other tags should remain unchanged
    assert_eq!(tags[&tag_ids[0]].name, "tag0");
    assert_eq!(tags[&tag_ids[2]].name, "tag2");

    // Rename to an existing tag name should fail
    assert_err!(
        data.rename_tag(tag_ids[0], "renamed_tag".to_string(), &db),
        Err(Error::TagAlreadyExists(..))
    );
}

#[test]
fn test_delete_tag() {
    const TAGS_NUM: usize = 6;
    let mut tag_ids: [i32; TAGS_NUM] = [0; TAGS_NUM];

    let (_base_path, database, _emitter) = setup_database(testdir!().as_path());

    {
        let mut data = database.data.write().unwrap();
        let mut db = database.db.lock().unwrap();

        for (i, tag_id) in tag_ids.iter_mut().enumerate() {
            *tag_id = data.new_tag(format!("tag{i}"), &db).unwrap();
        }

        // Reparent tags
        // 0 - [1 - [2], 3], 4 - [5]
        data.reorder_tag(tag_ids[0], ROOT_TAG_ID, -1, &mut db)
            .unwrap();
        data.reorder_tag(tag_ids[1], tag_ids[0], -1, &mut db)
            .unwrap();
        data.reorder_tag(tag_ids[2], tag_ids[1], -1, &mut db)
            .unwrap();
        data.reorder_tag(tag_ids[3], tag_ids[0], -1, &mut db)
            .unwrap();
        data.reorder_tag(tag_ids[4], ROOT_TAG_ID, -1, &mut db)
            .unwrap();
        data.reorder_tag(tag_ids[5], tag_ids[4], -1, &mut db)
            .unwrap();

        // Delete tag 1
        // 0 - [3], 4 - [5]
        data.delete_tag(tag_ids[1], &db).unwrap();

        // Tag 1 should be removed
        let tags = data.get_tags();
        assert_eq!(tags[&ROOT_TAG_ID].children, [tag_ids[0], tag_ids[4]].into());
        assert_eq!(tags[&tag_ids[0]].children, [tag_ids[3]].into());

        // Other tags should remain unchanged
        assert_eq!(tags[&tag_ids[3]].children.len(), 0);
        assert_eq!(tags[&tag_ids[4]].children, [tag_ids[5]].into());
        assert_eq!(tags[&tag_ids[5]].children.len(), 0);
    }
    drop(database);

    // Reopen the database to verify the changes persist
    let database = Database::open(_base_path, Arc::new(TestEmitter::new())).unwrap();
    let data = database.data.read().unwrap();

    // Tag 1 should be removed
    let tags = data.get_tags();
    assert_eq!(tags[&ROOT_TAG_ID].children, [tag_ids[0], tag_ids[4]].into());
    assert_eq!(tags[&tag_ids[0]].children, [tag_ids[3]].into());

    // Other tags should remain unchanged
    assert_eq!(tags[&tag_ids[3]].children.len(), 0);
    assert_eq!(tags[&tag_ids[4]].children, [tag_ids[5]].into());
    assert_eq!(tags[&tag_ids[5]].children.len(), 0);
}

#[test]
fn test_file_watcher_create_single_file() {
    let (base_path, database, emitter) = setup_database(testdir!().as_path());

    let new_file_path = PathBuf::from("new_mp3_audio.mp3");
    File::create(base_path.join(&new_file_path)).unwrap();

    assert!(emitter.wait_for_files_updated(EMITTER_TIMEOUT));

    let data = database.data.read().unwrap();
    let entries = data.get_entries();

    assert_eq!(entries.len(), 9); // 8 existing + 1 new files
    let new_entry = &entries[&data.get_entry_id(&new_file_path).unwrap()];
    assert_eq!(new_entry.path, new_file_path);
    assert_eq!(new_entry.folder_id, ROOT_FOLDER_ID);
}

#[test]
fn test_file_watcher_create_single_file_in_subfolder() {
    let (base_path, database, emitter) = setup_database(testdir!().as_path());

    let new_file_path = PathBuf::from("folder1/new_flac_audio.flac");
    File::create(base_path.join(&new_file_path)).unwrap();

    assert!(emitter.wait_for_files_updated(EMITTER_TIMEOUT));

    let data = database.data.read().unwrap();
    let entries = data.get_entries();

    assert_eq!(entries.len(), 9); // 8 existing + 1 new files
    let new_entry = &entries[&data.get_entry_id(&new_file_path).unwrap()];
    assert_eq!(new_entry.path, new_file_path);
    assert_eq!(
        new_entry.folder_id,
        data.get_folder_by_path(new_file_path.parent().unwrap())
            .unwrap()
            .id
    );
}

#[test]
fn test_file_watcher_create_multiple_files() {
    let (base_path, database, emitter) = setup_database(testdir!().as_path());

    let new_file_path_1 = PathBuf::from("new_mp3_audio.mp3");
    let new_file_path_2 = PathBuf::from("folder1/new_flac_audio.flac");
    let new_file_path_3 = PathBuf::from("folder1/folder1-1/new_wave_audio.wav");
    File::create(base_path.join(&new_file_path_1)).unwrap();
    File::create(base_path.join(&new_file_path_2)).unwrap();
    File::create(base_path.join(&new_file_path_3)).unwrap();

    assert!(emitter.wait_for_files_updated(EMITTER_TIMEOUT));

    let data = database.data.read().unwrap();
    let entries = data.get_entries();

    assert_eq!(entries.len(), 11); // 8 existing + 3 new files

    let new_entry_1 = &entries[&data.get_entry_id(&new_file_path_1).unwrap()];
    assert_eq!(new_entry_1.path, new_file_path_1);
    assert_eq!(new_entry_1.folder_id, ROOT_FOLDER_ID);

    let new_entry_2 = &entries[&data.get_entry_id(&new_file_path_2).unwrap()];
    assert_eq!(new_entry_2.path, new_file_path_2);
    assert_eq!(
        new_entry_2.folder_id,
        data.get_folder_by_path(new_file_path_2.parent().unwrap())
            .unwrap()
            .id
    );

    let new_entry_3 = &entries[&data.get_entry_id(&new_file_path_3).unwrap()];
    assert_eq!(new_entry_3.path, new_file_path_3);
    assert_eq!(
        new_entry_3.folder_id,
        data.get_folder_by_path(new_file_path_3.parent().unwrap())
            .unwrap()
            .id
    );
}

#[test]
fn test_file_watcher_delete_single_file() {
    let (base_path, database, emitter) = setup_database(testdir!().as_path());

    let file_path = PathBuf::from("wave_audio_1.wav");
    remove_file(base_path.join(&file_path)).unwrap();

    assert!(emitter.wait_for_files_updated(EMITTER_TIMEOUT));

    let data = database.data.read().unwrap();
    let entries = data.get_entries();

    assert_eq!(entries.len(), 7); // 8 existing files - 1 deleted file
    assert!(data.get_entry_id(&file_path).is_none());
}

#[test]
fn test_file_watcher_delete_single_file_in_subfolder() {
    let (base_path, database, emitter) = setup_database(testdir!().as_path());

    let file_path = PathBuf::from("folder1/folder1-1/flac_audio_2.flac");
    remove_file(base_path.join(&file_path)).unwrap();

    assert!(emitter.wait_for_files_updated(EMITTER_TIMEOUT));

    let data = database.data.read().unwrap();
    let entries = data.get_entries();

    assert_eq!(entries.len(), 7); // 8 existing files - 1 deleted file
    assert!(data.get_entry_id(&file_path).is_none());
}

#[test]
fn test_file_watcher_delete_multiple_files() {
    let (base_path, database, emitter) = setup_database(testdir!().as_path());

    let file_path_1 = PathBuf::from("wave_audio_1.wav");
    let file_path_2 = PathBuf::from("folder1/wave_audio_2.wav");
    let file_path_3 = PathBuf::from("folder1/folder1-2/mp3_audio_2.mp3");
    remove_file(base_path.join(&file_path_1)).unwrap();
    remove_file(base_path.join(&file_path_2)).unwrap();
    remove_file(base_path.join(&file_path_3)).unwrap();

    assert!(emitter.wait_for_files_updated(EMITTER_TIMEOUT));

    let data = database.data.read().unwrap();
    let entries = data.get_entries();

    assert_eq!(entries.len(), 5); // 8 existing files - 3 deleted files
    assert!(data.get_entry_id(&file_path_1).is_none());
    assert!(data.get_entry_id(&file_path_2).is_none());
    assert!(data.get_entry_id(&file_path_3).is_none());
}

#[ignore = "notify-rs emits incorrect events for auto-test-triggered file moves for unknown reason."]
#[test]
fn test_file_watcher_move_single_file() {
    let (base_path, database, emitter) = setup_database(testdir!().as_path());

    let old_file_path = PathBuf::from("mp3_audio_1.mp3");
    let new_file_path = PathBuf::from("folder1/folder1-1/mp3_audio_1.mp3");
    rename(
        base_path.join(&old_file_path),
        base_path.join(&new_file_path),
    )
    .unwrap();

    assert!(emitter.wait_for_files_updated(EMITTER_TIMEOUT));

    let data = database.data.read().unwrap();
    let entries = data.get_entries();

    assert_eq!(entries.len(), 8); // 8 existing files
    let moved_entry = &entries[&data.get_entry_id(&new_file_path).unwrap()];
    assert_eq!(moved_entry.path, new_file_path);
    assert_eq!(
        moved_entry.folder_id,
        data.get_folder_by_path(new_file_path.parent().unwrap())
            .unwrap()
            .id
    );
}

#[ignore = "notify-rs emits incorrect events for auto-test-triggered file moves for unknown reason."]
#[test]
fn test_file_watcher_move_multiple_files() {
    let (base_path, database, emitter) = setup_database(testdir!().as_path());

    let old_file_path_1 = PathBuf::from("mp3_audio_1.mp3");
    let new_file_path_1 = PathBuf::from("folder1/folder1-1/mp3_audio_1.mp3");
    let old_file_path_2 = PathBuf::from("flac_audio_1.flac");
    let new_file_path_2 = PathBuf::from("folder1/flac_audio_1.flac");
    let old_file_path_3 = PathBuf::from("folder1/folder1-2/mp3_audio_2.mp3");
    let new_file_path_3 = PathBuf::from("mp3_audio_2.mp3");

    rename(
        base_path.join(&old_file_path_1),
        base_path.join(&new_file_path_1),
    )
    .unwrap();
    rename(
        base_path.join(&old_file_path_2),
        base_path.join(&new_file_path_2),
    )
    .unwrap();
    rename(
        base_path.join(&old_file_path_3),
        base_path.join(&new_file_path_3),
    )
    .unwrap();

    assert!(emitter.wait_for_files_updated(EMITTER_TIMEOUT));

    let data = database.data.read().unwrap();
    let entries = data.get_entries();

    assert_eq!(entries.len(), 8); // 8 existing files

    let moved_entry_1 = &entries[&data.get_entry_id(&new_file_path_1).unwrap()];
    assert_eq!(moved_entry_1.path, new_file_path_1);
    assert_eq!(
        moved_entry_1.folder_id,
        data.get_folder_by_path(new_file_path_1.parent().unwrap())
            .unwrap()
            .id
    );

    let moved_entry_2 = &entries[&data.get_entry_id(&new_file_path_2).unwrap()];
    assert_eq!(moved_entry_2.path, new_file_path_2);
    assert_eq!(
        moved_entry_2.folder_id,
        data.get_folder_by_path(new_file_path_2.parent().unwrap())
            .unwrap()
            .id
    );

    let moved_entry_3 = &entries[&data.get_entry_id(&new_file_path_3).unwrap()];
    assert_eq!(moved_entry_3.path, new_file_path_3);
    assert_eq!(
        moved_entry_3.folder_id,
        data.get_folder_by_path(new_file_path_3.parent().unwrap())
            .unwrap()
            .id
    );
}

#[test]
fn test_file_watcher_move_in_single_file() {
    let (base_path, database, emitter) = setup_database(testdir!().as_path());

    let file_name = PathBuf::from("new_mp3_audio.mp3");
    File::create(testdir!().join(&file_name)).unwrap();

    let new_file_path = file_name.clone();
    rename(testdir!().join(&file_name), base_path.join(&new_file_path)).unwrap();

    assert!(emitter.wait_for_files_updated(EMITTER_TIMEOUT));

    let data = database.data.read().unwrap();
    let entries = data.get_entries();

    assert_eq!(entries.len(), 9); // 8 existing + 1 new files
    let new_entry = &entries[&data.get_entry_id(&new_file_path).unwrap()];
    assert_eq!(new_entry.path, new_file_path);
    assert_eq!(new_entry.folder_id, ROOT_FOLDER_ID);
}

#[test]
fn test_file_watcher_move_in_single_file_in_subfolder() {
    let (base_path, database, emitter) = setup_database(testdir!().as_path());

    let file_name = PathBuf::from("new_flac_audio.flac");
    File::create(testdir!().join(&file_name)).unwrap();

    let new_file_path = PathBuf::from("folder1").join(&file_name);
    rename(testdir!().join(&file_name), base_path.join(&new_file_path)).unwrap();

    assert!(emitter.wait_for_files_updated(EMITTER_TIMEOUT));

    let data = database.data.read().unwrap();
    let entries = data.get_entries();

    assert_eq!(entries.len(), 9); // 8 existing + 1 new files
    let new_entry = &entries[&data.get_entry_id(&new_file_path).unwrap()];
    assert_eq!(new_entry.path, new_file_path);
    assert_eq!(
        new_entry.folder_id,
        data.get_folder_by_path(new_file_path.parent().unwrap())
            .unwrap()
            .id
    );
}

#[test]
fn test_file_watcher_move_in_multiple_files() {
    let (base_path, database, emitter) = setup_database(testdir!().as_path());

    let file_name_1 = PathBuf::from("new_mp3_audio.mp3");
    let file_name_2 = PathBuf::from("new_flac_audio.flac");
    let file_name_3 = PathBuf::from("new_wave_audio.wav");
    File::create(testdir!().join(&file_name_1)).unwrap();
    File::create(testdir!().join(&file_name_2)).unwrap();
    File::create(testdir!().join(&file_name_3)).unwrap();

    let new_file_path_1 = file_name_1.clone();
    let new_file_path_2 = PathBuf::from("folder1").join(&file_name_2);
    let new_file_path_3 = PathBuf::from("folder1/folder1-1").join(&file_name_3);
    rename(
        testdir!().join(&file_name_1),
        base_path.join(&new_file_path_1),
    )
    .unwrap();
    rename(
        testdir!().join(&file_name_2),
        base_path.join(&new_file_path_2),
    )
    .unwrap();
    rename(
        testdir!().join(&file_name_3),
        base_path.join(&new_file_path_3),
    )
    .unwrap();

    assert!(emitter.wait_for_files_updated(EMITTER_TIMEOUT));

    let data = database.data.read().unwrap();
    let entries = data.get_entries();

    assert_eq!(entries.len(), 11); // 8 existing + 3 new files

    let new_entry_1 = &entries[&data.get_entry_id(&new_file_path_1).unwrap()];
    assert_eq!(new_entry_1.path, new_file_path_1);
    assert_eq!(new_entry_1.folder_id, ROOT_FOLDER_ID);

    let new_entry_2 = &entries[&data.get_entry_id(&new_file_path_2).unwrap()];
    assert_eq!(new_entry_2.path, new_file_path_2);
    assert_eq!(
        new_entry_2.folder_id,
        data.get_folder_by_path(new_file_path_2.parent().unwrap())
            .unwrap()
            .id
    );

    let new_entry_3 = &entries[&data.get_entry_id(&new_file_path_3).unwrap()];
    assert_eq!(new_entry_3.path, new_file_path_3);
    assert_eq!(
        new_entry_3.folder_id,
        data.get_folder_by_path(new_file_path_3.parent().unwrap())
            .unwrap()
            .id
    );
}

#[test]
fn test_file_watcher_move_out_single_file() {
    let (base_path, database, emitter) = setup_database(testdir!().as_path());

    let file_path = PathBuf::from("wave_audio_1.wav");
    let file_name = file_path.file_name().unwrap();
    rename(base_path.join(&file_path), testdir!().join(file_name)).unwrap();

    assert!(emitter.wait_for_files_updated(EMITTER_TIMEOUT));

    let data = database.data.read().unwrap();
    let entries = data.get_entries();

    assert_eq!(entries.len(), 7); // 8 existing files - 1 deleted file
    assert!(data.get_entry_id(&file_path).is_none());
}

#[test]
fn test_file_watcher_move_out_single_file_in_subfolder() {
    let (base_path, database, emitter) = setup_database(testdir!().as_path());

    let file_path = PathBuf::from("folder1/folder1-1/flac_audio_2.flac");
    let file_name = file_path.file_name().unwrap();
    rename(base_path.join(&file_path), testdir!().join(file_name)).unwrap();

    assert!(emitter.wait_for_files_updated(EMITTER_TIMEOUT));

    let data = database.data.read().unwrap();
    let entries = data.get_entries();

    assert_eq!(entries.len(), 7); // 8 existing files - 1 deleted file
    assert!(data.get_entry_id(&file_path).is_none());
}

#[test]
fn test_file_watcher_move_out_multiple_files() {
    let (base_path, database, emitter) = setup_database(testdir!().as_path());

    let file_path_1 = PathBuf::from("wave_audio_1.wav");
    let file_path_2 = PathBuf::from("folder1/wave_audio_2.wav");
    let file_path_3 = PathBuf::from("folder1/folder1-2/mp3_audio_2.mp3");

    let file_name_1 = file_path_1.file_name().unwrap();
    let file_name_2 = file_path_2.file_name().unwrap();
    let file_name_3 = file_path_3.file_name().unwrap();

    rename(base_path.join(&file_path_1), testdir!().join(file_name_1)).unwrap();
    rename(base_path.join(&file_path_2), testdir!().join(file_name_2)).unwrap();
    rename(base_path.join(&file_path_3), testdir!().join(file_name_3)).unwrap();

    assert!(emitter.wait_for_files_updated(EMITTER_TIMEOUT));

    let data = database.data.read().unwrap();
    let entries = data.get_entries();

    assert_eq!(entries.len(), 5); // 8 existing files - 3 deleted files
    assert!(data.get_entry_id(&file_path_1).is_none());
    assert!(data.get_entry_id(&file_path_2).is_none());
    assert!(data.get_entry_id(&file_path_3).is_none());
}
