use redb::{Legacy, ReadableDatabase, ReadableTableMetadata, TableError};

const ELEMENTS: usize = 3;

trait TestData: redb::Value + redb2_6::Value {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS]
    where
        Self: 'a;

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS]
    where
        Self: 'a;
}

impl TestData for u8 {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [0, 1, 2]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [0, 1, 2]
    }
}

impl TestData for u16 {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [0, 1, 2]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [0, 1, 2]
    }
}

impl TestData for u32 {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [0, 1, 2]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [0, 1, 2]
    }
}

impl TestData for u64 {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [0, 1, 2]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [0, 1, 2]
    }
}

impl TestData for u128 {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [0, 1, 2]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [0, 1, 2]
    }
}

impl TestData for i8 {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [-1, 1, 2]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [-1, 1, 2]
    }
}

impl TestData for i16 {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [-1, 1, 2]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [-1, 1, 2]
    }
}

impl TestData for i32 {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [-1, 1, 2]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [-1, 1, 2]
    }
}

impl TestData for i64 {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [-1, 1, 2]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [-1, 1, 2]
    }
}

impl TestData for i128 {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [-1, 1, 2]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [-1, 1, 2]
    }
}

impl TestData for f32 {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [f32::NAN, f32::INFINITY, f32::MIN_POSITIVE]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [f32::NAN, f32::INFINITY, f32::MIN_POSITIVE]
    }
}

impl TestData for f64 {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [f64::MIN, f64::NEG_INFINITY, f64::MAX]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [f64::MIN, f64::NEG_INFINITY, f64::MAX]
    }
}

impl TestData for () {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [(), (), ()]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [(), (), ()]
    }
}

impl TestData for &'static str {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        ["hello", "world1", "hi"]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        ["hello", "world1", "hi"]
    }
}

impl TestData for &'static [u8] {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [b"test", b"bytes", b"now"]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [b"test", b"bytes", b"now"]
    }
}

impl TestData for &'static [u8; 5] {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [b"test1", b"bytes", b"now12"]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [b"test1", b"bytes", b"now12"]
    }
}

impl TestData for [&str; 3] {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS]
    where
        Self: 'a,
    {
        [
            ["test1", "hi", "world"],
            ["test2", "hi", "world"],
            ["test3", "hi", "world"],
        ]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS]
    where
        Self: 'a,
    {
        [
            ["test1", "hi", "world"],
            ["test2", "hi", "world"],
            ["test3", "hi", "world"],
        ]
    }
}

impl TestData for [u128; 3] {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [[1, 2, 3], [3, 2, 1], [300, 200, 100]]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [[1, 2, 3], [3, 2, 1], [300, 200, 100]]
    }
}

impl TestData for Vec<&str> {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS]
    where
        Self: 'a,
    {
        [
            vec!["test1", "hi", "world"],
            vec!["test2", "hi", "world"],
            vec!["test3", "hi", "world"],
        ]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS]
    where
        Self: 'a,
    {
        [
            vec!["test1", "hi", "world"],
            vec!["test2", "hi", "world"],
            vec!["test3", "hi", "world"],
        ]
    }
}

impl TestData for Option<u64> {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [None, Some(0), Some(7)]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [None, Some(0), Some(7)]
    }
}

impl TestData for (u64, &'static str) {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [(0, "hi"), (1, "bye"), (2, "byte")]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [(0, "hi"), (1, "bye"), (2, "byte")]
    }
}

impl TestData for (u64, u32) {
    fn make_data_v2_6<'a>() -> [<Self as redb2_6::Value>::SelfType<'a>; ELEMENTS] {
        [(0, 3), (1, 4), (2, 5)]
    }

    fn make_data<'a>() -> [<Self as redb::Value>::SelfType<'a>; ELEMENTS] {
        [(0, 3), (1, 4), (2, 5)]
    }
}

fn create_tempfile() -> tempfile::NamedTempFile {
    if cfg!(target_os = "wasi") {
        tempfile::NamedTempFile::new_in("/tmp").unwrap()
    } else {
        tempfile::NamedTempFile::new().unwrap()
    }
}

fn test_helper<K: TestData + redb::Key + redb2_6::Key + 'static, V: TestData + 'static>() {
    {
        let tmpfile = create_tempfile();
        let db = redb2_6::Database::builder()
            .create_with_file_format_v3(true)
            .create(tmpfile.path())
            .unwrap();
        let table_def: redb2_6::TableDefinition<K, V> = redb2_6::TableDefinition::new("table");
        let write_txn = db.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(table_def).unwrap();
            for i in 0..ELEMENTS {
                table
                    .insert(&K::make_data_v2_6()[i], &V::make_data_v2_6()[i])
                    .unwrap();
            }
        }
        write_txn.commit().unwrap();
        drop(db);

        let db = redb::Database::open(tmpfile.path()).unwrap();
        let read_txn = db.begin_read().unwrap();
        let table_def: redb::TableDefinition<K, V> = redb::TableDefinition::new("table");
        let table = read_txn.open_table(table_def).unwrap();
        assert_eq!(table.len().unwrap(), ELEMENTS as u64);
        for i in 0..ELEMENTS {
            let result = table.get(&K::make_data()[i]).unwrap().unwrap();
            let value = result.value();
            let bytes = <V as redb::Value>::as_bytes(&value);
            let expected = &V::make_data()[i];
            let expected_bytes = <V as redb::Value>::as_bytes(expected);
            assert_eq!(bytes.as_ref(), expected_bytes.as_ref());
        }
    }
}

#[test]
fn primitive_types() {
    test_helper::<u8, u8>();
    test_helper::<u16, u16>();
    test_helper::<u32, u32>();
    test_helper::<u64, u64>();
    test_helper::<u128, u128>();
    test_helper::<i8, i8>();
    test_helper::<i16, i16>();
    test_helper::<i32, i32>();
    test_helper::<i64, i64>();
    test_helper::<i128, i128>();
    test_helper::<i128, f32>();
    test_helper::<i128, f64>();
    test_helper::<&str, &str>();
    test_helper::<u8, ()>();
}

#[test]
fn container_types() {
    test_helper::<&[u8], &[u8]>();
    test_helper::<&[u8; 5], &[u8; 5]>();
    test_helper::<u64, Option<u64>>();
    test_helper::<(u64, u32), &str>();
    test_helper::<[&str; 3], [u128; 3]>();
    test_helper::<u64, Vec<&str>>();
}

#[test]
fn mixed_width() {
    test_helper::<u8, &[u8]>();
    test_helper::<&[u8; 5], &str>();
}

#[test]
fn tuple_types() {
    let tmpfile = create_tempfile();
    let db = redb2_6::Database::builder()
        .create_with_file_format_v3(true)
        .create(tmpfile.path())
        .unwrap();
    let table_def: redb2_6::TableDefinition<(u64, &str), &str> =
        redb2_6::TableDefinition::new("table");
    let write_txn = db.begin_write().unwrap();
    {
        let mut table = write_txn.open_table(table_def).unwrap();
        for i in 0..ELEMENTS {
            table
                .insert(
                    &<(u64, &str)>::make_data_v2_6()[i],
                    &<&str>::make_data_v2_6()[i],
                )
                .unwrap();
        }
    }
    write_txn.commit().unwrap();
    drop(db);

    let db = redb::Database::open(tmpfile.path()).unwrap();
    let read_txn = db.begin_read().unwrap();
    let bad_table_def: redb::TableDefinition<(u64, &str), &str> =
        redb::TableDefinition::new("table");
    assert!(matches!(
        read_txn.open_table(bad_table_def).unwrap_err(),
        TableError::TableTypeMismatch { .. }
    ));
    let table_def: redb::TableDefinition<Legacy<(u64, &str)>, &str> =
        redb::TableDefinition::new("table");
    let table = read_txn.open_table(table_def).unwrap();
    assert_eq!(table.len().unwrap(), ELEMENTS as u64);
    for i in 0..ELEMENTS {
        let result = table.get(&<(u64, &str)>::make_data()[i]).unwrap().unwrap();
        let value = result.value();
        let bytes = <&str as redb::Value>::as_bytes(&value);
        let expected = &<&str>::make_data()[i];
        let expected_bytes = <&str as redb::Value>::as_bytes(expected);
        assert_eq!(bytes, expected_bytes);
    }
}
