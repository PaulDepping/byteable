//! I/O tests for collection types and `Result<V, E>`.
//!
//! Covers the types that are supported by `Readable`/`Writable` (and their
//! async counterparts) but are missing from the basic `io_test.rs`:
//! `VecDeque`, `HashMap`, `HashSet`, `BTreeMap`, `BTreeSet`, `Result`, and
//! writing via `&str`.
#![cfg(feature = "std")]

// ── Sync ──────────────────────────────────────────────────────────────────────

mod sync {
    use byteable::{ReadValue, WriteValue};
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
    use std::io::Cursor;

    fn roundtrip<T>(original: &T) -> T
    where
        T: byteable::Writable + byteable::Readable,
    {
        let mut buf = Vec::new();
        buf.write_value(original).unwrap();
        Cursor::new(&buf).read_value().unwrap()
    }

    #[test]
    fn vecdeque_u32_roundtrip() {
        let original: VecDeque<u32> = VecDeque::from([0xDEAD, 0xBEEF, 0xCAFE]);
        let restored: VecDeque<u32> = roundtrip(&original);
        assert_eq!(original, restored);
    }

    #[test]
    fn vecdeque_empty_roundtrip() {
        let original: VecDeque<u8> = VecDeque::new();
        let restored: VecDeque<u8> = roundtrip(&original);
        assert_eq!(original, restored);
    }

    #[test]
    fn hashmap_roundtrip() {
        let mut original: HashMap<u8, u32> = HashMap::new();
        original.insert(1, 100);
        original.insert(2, 200);
        original.insert(3, 300);
        let restored: HashMap<u8, u32> = roundtrip(&original);
        assert_eq!(original, restored);
    }

    #[test]
    fn hashset_roundtrip() {
        let original: HashSet<u32> = HashSet::from([10, 20, 30, 40]);
        let restored: HashSet<u32> = roundtrip(&original);
        assert_eq!(original, restored);
    }

    #[test]
    fn btreemap_roundtrip() {
        let original: BTreeMap<u8, u32> = BTreeMap::from([(1, 100), (2, 200), (3, 300)]);
        let restored: BTreeMap<u8, u32> = roundtrip(&original);
        assert_eq!(original, restored);
    }

    #[test]
    fn btreeset_roundtrip() {
        let original: BTreeSet<u32> = BTreeSet::from([5, 10, 15, 20]);
        let restored: BTreeSet<u32> = roundtrip(&original);
        assert_eq!(original, restored);
    }

    #[test]
    fn result_ok_roundtrip() {
        let original: Result<u32, u8> = Ok(0xDEAD_BEEF);
        let restored: Result<u32, u8> = roundtrip(&original);
        assert_eq!(original, restored);
    }

    #[test]
    fn result_err_roundtrip() {
        let original: Result<u32, u8> = Err(42);
        let restored: Result<u32, u8> = roundtrip(&original);
        assert_eq!(original, restored);
    }

    #[test]
    fn str_write_then_string_read() {
        let s = "hello, byteable!";
        let mut buf = Vec::new();
        buf.write_value(s).unwrap();
        let restored: String = Cursor::new(&buf).read_value().unwrap();
        assert_eq!(restored, s);
    }

    #[test]
    fn option_invalid_tag_is_err() {
        // Tag byte 2 is neither None (0) nor Some (1).
        let mut buf = Cursor::new(vec![2u8]);
        let result: std::io::Result<Option<u32>> = buf.read_value();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn result_invalid_tag_is_err() {
        // Tag byte 2 is neither Ok (0) nor Err (1).
        let mut buf = Cursor::new(vec![2u8]);
        let result: std::io::Result<Result<u32, u8>> = buf.read_value();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    }
}

// ── Derive-macro integration (io_only with collections) ───────────────────────

/// Tests that `#[derive(Byteable)] #[byteable(io_only)]` works correctly with collection-type
/// fields: `Vec`, `VecDeque`, `HashMap`, `HashSet`, `BTreeMap`, `BTreeSet`, `Option`, `String`,
/// nested `io_only` structs, and endian-annotated numeric fields alongside collection fields.
#[cfg(feature = "derive")]
mod derive_io_only {
    use byteable::{Byteable, ReadValue, WriteValue};
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
    use std::io::Cursor;

    fn roundtrip<T>(original: &T) -> T
    where
        T: byteable::Writable + byteable::Readable,
    {
        let mut buf = Vec::new();
        buf.write_value(original).unwrap();
        Cursor::new(&buf).read_value().unwrap()
    }

    // ── Vec<u8> field ─────────────────────────────────────────────────────

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct ByteVecPacket {
        tag: u8,
        data: Vec<u8>,
    }

    #[test]
    fn byte_vec_field_roundtrip() {
        let original = ByteVecPacket { tag: 7, data: vec![1, 2, 3, 4, 5] };
        assert_eq!(roundtrip(&original), original);
    }

    #[test]
    fn byte_vec_field_empty_roundtrip() {
        let original = ByteVecPacket { tag: 0, data: vec![] };
        assert_eq!(roundtrip(&original), original);
    }

    // ── Vec<u32> (non-byte element) ───────────────────────────────────────

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct U32VecPacket {
        tag: u8,
        values: Vec<u32>,
    }

    #[test]
    fn u32_vec_field_roundtrip() {
        let original = U32VecPacket {
            tag: 5,
            values: vec![0xDEAD, 0xBEEF, 0xCAFE, 0xBABE],
        };
        assert_eq!(roundtrip(&original), original);
    }

    #[test]
    fn u32_vec_field_empty_roundtrip() {
        let original = U32VecPacket { tag: 0, values: vec![] };
        assert_eq!(roundtrip(&original), original);
    }

    // ── VecDeque field ────────────────────────────────────────────────────

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct DequePacket {
        tag: u8,
        queue: VecDeque<u32>,
    }

    #[test]
    fn vecdeque_field_roundtrip() {
        let original = DequePacket {
            tag: 1,
            queue: VecDeque::from([0xDEAD, 0xBEEF, 0xCAFE]),
        };
        assert_eq!(roundtrip(&original), original);
    }

    #[test]
    fn vecdeque_field_empty_roundtrip() {
        let original = DequePacket { tag: 0, queue: VecDeque::new() };
        assert_eq!(roundtrip(&original), original);
    }

    // ── HashMap field ─────────────────────────────────────────────────────

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct HashMapPacket {
        id: u8,
        table: HashMap<u8, u32>,
    }

    #[test]
    fn hashmap_field_roundtrip() {
        let original = HashMapPacket {
            id: 42,
            table: HashMap::from([(1u8, 100u32), (2, 200), (3, 300)]),
        };
        assert_eq!(roundtrip(&original), original);
    }

    #[test]
    fn hashmap_field_empty_roundtrip() {
        let original = HashMapPacket { id: 0, table: HashMap::new() };
        assert_eq!(roundtrip(&original), original);
    }

    // ── HashSet field ─────────────────────────────────────────────────────

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct HashSetPacket {
        version: u8,
        members: HashSet<u32>,
    }

    #[test]
    fn hashset_field_roundtrip() {
        let original = HashSetPacket {
            version: 1,
            members: HashSet::from([10u32, 20, 30, 40]),
        };
        assert_eq!(roundtrip(&original), original);
    }

    // ── BTreeMap field ────────────────────────────────────────────────────

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct BTreeMapPacket {
        id: u8,
        entries: BTreeMap<u8, u32>,
    }

    #[test]
    fn btreemap_field_roundtrip() {
        let original = BTreeMapPacket {
            id: 7,
            entries: BTreeMap::from([(1u8, 10u32), (2, 20), (3, 30)]),
        };
        assert_eq!(roundtrip(&original), original);
    }

    // ── BTreeSet field ────────────────────────────────────────────────────

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct BTreeSetPacket {
        version: u8,
        ids: BTreeSet<u32>,
    }

    #[test]
    fn btreeset_field_roundtrip() {
        let original = BTreeSetPacket {
            version: 2,
            ids: BTreeSet::from([5u32, 10, 15, 20]),
        };
        assert_eq!(roundtrip(&original), original);
    }

    // ── Option<u32> (multi-byte option) ───────────────────────────────────

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct OptionalU32 {
        flag: u8,
        value: Option<u32>,
    }

    #[test]
    fn option_u32_some_roundtrip() {
        let original = OptionalU32 { flag: 1, value: Some(0xDEAD_BEEF) };
        assert_eq!(roundtrip(&original), original);
    }

    #[test]
    fn option_u32_none_roundtrip() {
        let original = OptionalU32 { flag: 0, value: None };
        assert_eq!(roundtrip(&original), original);
    }

    // ── String field ──────────────────────────────────────────────────────

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct LabeledData {
        id: u8,
        label: String,
        data: Vec<u8>,
    }

    #[test]
    fn string_and_vec_fields_roundtrip() {
        let original = LabeledData {
            id: 9,
            label: "hello, byteable!".to_string(),
            data: vec![0xDE, 0xAD],
        };
        assert_eq!(roundtrip(&original), original);
    }

    // ── Endian-annotated numerics + collection ────────────────────────────

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct FramePacket {
        #[byteable(little_endian)]
        sequence: u32,
        #[byteable(big_endian)]
        checksum: u16,
        payload: Vec<u8>,
    }

    #[test]
    fn frame_packet_roundtrip() {
        let original = FramePacket {
            sequence: 0xDEAD_BEEF,
            checksum: 0x1234,
            payload: vec![0xAA, 0xBB, 0xCC],
        };
        assert_eq!(roundtrip(&original), original);
    }

    #[test]
    fn frame_packet_byte_layout() {
        let frame = FramePacket { sequence: 1, checksum: 0x0102, payload: vec![0xFF] };
        let mut buf = Vec::new();
        buf.write_value(&frame).unwrap();

        assert_eq!(&buf[0..4], &1u32.to_le_bytes());      // sequence LE
        assert_eq!(&buf[4..6], &0x0102u16.to_be_bytes()); // checksum BE
        assert_eq!(&buf[6..14], &1u64.to_le_bytes());     // Vec length prefix (LE u64)
        assert_eq!(buf[14], 0xFF);
    }

    // ── Nested io_only structs ────────────────────────────────────────────

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct InnerRecord {
        kind: u8,
        items: Vec<u32>,
    }

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct OuterRecord {
        #[byteable(big_endian)]
        id: u16,
        inner: InnerRecord,
        tags: BTreeSet<u8>,
    }

    #[test]
    fn nested_io_only_roundtrip() {
        let original = OuterRecord {
            id: 0xCAFE,
            inner: InnerRecord { kind: 3, items: vec![10, 20, 30] },
            tags: BTreeSet::from([1u8, 2, 3]),
        };
        assert_eq!(roundtrip(&original), original);
    }

    // ── Tuple struct with collection fields ───────────────────────────────

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct TupleWithCollections(u8, Vec<u32>, BTreeSet<u8>);

    #[test]
    fn tuple_with_collections_roundtrip() {
        let original = TupleWithCollections(42, vec![1, 2, 3], BTreeSet::from([10u8, 20, 30]));
        assert_eq!(roundtrip(&original), original);
    }

    // ── Multiple collection fields ────────────────────────────────────────

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct MultiCollectionRecord {
        label: String,
        data: Vec<u8>,
        ids: BTreeSet<u32>,
        counts: BTreeMap<u8, u32>,
    }

    #[test]
    fn multi_collection_fields_roundtrip() {
        let original = MultiCollectionRecord {
            label: "test".to_string(),
            data: vec![1, 2, 3],
            ids: BTreeSet::from([100u32, 200, 300]),
            counts: BTreeMap::from([(1u8, 10u32), (2, 20)]),
        };
        assert_eq!(roundtrip(&original), original);
    }
}

// ── Async ─────────────────────────────────────────────────────────────────────
//
// Note: `Result<V, E>` is not implemented for the async I/O traits (only sync),
// so those roundtrip tests live only in the `sync` module above.

#[cfg(feature = "tokio")]
mod async_ {
    use byteable::{AsyncReadValue, AsyncWritable, AsyncWriteValue};
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
    use std::io::Cursor;

    // Vec<u8> doesn't implement tokio::AsyncWrite; use Cursor<Vec<u8>> instead.
    async fn roundtrip<T>(original: &T) -> T
    where
        T: AsyncWritable + byteable::AsyncReadable,
    {
        let mut buf = Cursor::new(Vec::new());
        buf.write_value(original).await.unwrap();
        buf.set_position(0);
        buf.read_value().await.unwrap()
    }

    #[tokio::test]
    async fn async_vecdeque_u32_roundtrip() {
        let original: VecDeque<u32> = VecDeque::from([0xDEAD, 0xBEEF, 0xCAFE]);
        let restored: VecDeque<u32> = roundtrip(&original).await;
        assert_eq!(original, restored);
    }

    #[tokio::test]
    async fn async_hashmap_roundtrip() {
        let mut original: HashMap<u8, u32> = HashMap::new();
        original.insert(1, 100);
        original.insert(2, 200);
        let restored: HashMap<u8, u32> = roundtrip(&original).await;
        assert_eq!(original, restored);
    }

    #[tokio::test]
    async fn async_hashset_roundtrip() {
        let original: HashSet<u32> = HashSet::from([10, 20, 30]);
        let restored: HashSet<u32> = roundtrip(&original).await;
        assert_eq!(original, restored);
    }

    #[tokio::test]
    async fn async_btreemap_roundtrip() {
        let original: BTreeMap<u8, u32> = BTreeMap::from([(1, 10), (2, 20)]);
        let restored: BTreeMap<u8, u32> = roundtrip(&original).await;
        assert_eq!(original, restored);
    }

    #[tokio::test]
    async fn async_btreeset_roundtrip() {
        let original: BTreeSet<u32> = BTreeSet::from([5, 10, 15]);
        let restored: BTreeSet<u32> = roundtrip(&original).await;
        assert_eq!(original, restored);
    }

    #[tokio::test]
    async fn async_string_roundtrip() {
        // AsyncWriteValue::write_value requires Sized, so we test String (which
        // delegates to str internally) rather than &str directly.
        let original = String::from("async hello!");
        let restored: String = roundtrip(&original).await;
        assert_eq!(original, restored);
    }
}
