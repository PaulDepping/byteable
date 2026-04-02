//! Integration tests for synchronous I/O traits.
//!
//! Covers fixed-size I/O (`ReadFixed`/`WriteFixed`), value/stream I/O
//! (`ReadValue`/`WriteValue`), `#[byteable(io_only)]` struct derive, and
//! collection types (`Vec`, `VecDeque`, `HashMap`, etc.).
#![cfg(all(feature = "std", feature = "derive"))]

// ── Fixed-size I/O ────────────────────────────────────────────────────────────

mod fixed_io {
    use byteable::{
        BigEndian, Byteable, FixedReadable, FixedWritable, LittleEndian, ReadFixed, ReadValue,
        WriteFixed, WriteValue,
    };
    use std::io::Cursor;

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    struct Header {
        #[byteable(big_endian)]
        magic: u32,
        #[byteable(little_endian)]
        version: u16,
    }

    #[test]
    fn primitive_roundtrip() {
        let data = 42u32.to_ne_bytes().to_vec();
        let val: u32 = Cursor::new(data).read_fixed().unwrap();
        assert_eq!(val, 42u32);
    }

    #[test]
    fn endian_wrapper_roundtrip() {
        let original: LittleEndian<u32> = LittleEndian::new(0xDEADBEEF);
        let mut buf = Cursor::new(Vec::new());
        buf.write_fixed(&original).unwrap();
        let restored: LittleEndian<u32> = Cursor::new(buf.into_inner()).read_fixed().unwrap();
        assert_eq!(restored.get(), original.get());
    }

    #[test]
    fn derived_struct_roundtrip() {
        let header = Header { magic: 0x12345678, version: 42 };
        let mut buf = Cursor::new(Vec::new());
        buf.write_fixed(&header).unwrap();
        let restored: Header = Cursor::new(buf.into_inner()).read_fixed().unwrap();
        assert_eq!(restored, header);
    }

    #[test]
    fn write_fixed_and_read_value_produce_same_bytes() {
        let header = Header { magic: 0xCAFEBABE, version: 7 };
        let mut buf1 = Cursor::new(Vec::new());
        buf1.write_fixed(&header).unwrap();
        let mut buf2 = Cursor::new(Vec::new());
        buf2.write_value(&header).unwrap();
        assert_eq!(buf1.into_inner(), buf2.into_inner());
    }

    #[test]
    fn write_fixed_readable_by_read_value() {
        let header = Header { magic: 0xCAFEBABE, version: 7 };
        let mut buf = Cursor::new(Vec::new());
        buf.write_fixed(&header).unwrap();
        let restored: Header = Cursor::new(buf.into_inner()).read_value().unwrap();
        assert_eq!(restored, header);
    }

    #[test]
    fn write_value_readable_by_read_fixed() {
        let val = BigEndian::new(0x0102u16);
        let mut buf = Cursor::new(Vec::new());
        buf.write_value(&val).unwrap();
        let restored: BigEndian<u16> = Cursor::new(buf.into_inner()).read_fixed().unwrap();
        assert_eq!(restored.get(), val.get());
    }

    #[test]
    fn fixed_readable_also_implements_readable() {
        fn assert_both<T: FixedReadable + byteable::Readable>() {}
        assert_both::<u8>();
        assert_both::<u32>();
        assert_both::<LittleEndian<u64>>();
        assert_both::<Header>();
    }

    #[test]
    fn fixed_writable_also_implements_writable() {
        fn assert_both<T: FixedWritable + byteable::Writable>() {}
        assert_both::<u8>();
        assert_both::<u32>();
        assert_both::<BigEndian<u16>>();
        assert_both::<Header>();
    }

    /// Vec<u32> does NOT implement `FixedReadable` or `FixedWritable`.
    ///
    /// ```compile_fail
    /// use byteable::ReadFixed;
    /// use std::io::Cursor;
    /// let mut cursor = Cursor::new(vec![0u8; 16]);
    /// let _: Vec<u32> = cursor.read_fixed().unwrap();
    /// ```
    ///
    /// ```compile_fail
    /// use byteable::WriteFixed;
    /// use std::io::Cursor;
    /// let mut cursor = Cursor::new(Vec::new());
    /// cursor.write_fixed(&vec![1u32, 2, 3]).unwrap();
    /// ```
    #[test]
    fn compile_fail_docs_exist() {}
}

// ── Value / stream I/O ────────────────────────────────────────────────────────

mod value_io {
    use byteable::{Byteable, LittleEndian, ReadFixed, ReadValue, WriteFixed, WriteValue};
    use std::io::Cursor;

    #[derive(Byteable, Clone, Copy, Debug, PartialEq)]
    struct Packet {
        id: u8,
        #[byteable(little_endian)]
        length: u16,
        #[byteable(big_endian)]
        checksum: u32,
    }

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    #[repr(u8)]
    enum Status {
        Idle = 0,
        Running = 1,
        Done = 2,
    }

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    struct Frame {
        #[byteable(try_transparent)]
        status: Status,
        #[byteable(little_endian)]
        payload: u32,
    }

    #[test]
    fn derived_struct_write_then_read() {
        let original = Packet { id: 7, length: 0x1234, checksum: 0xDEADBEEF };
        let mut buf = Cursor::new(Vec::new());
        buf.write_fixed(&original).unwrap();
        buf.set_position(0);
        assert_eq!(buf.read_fixed::<Packet>().unwrap(), original);
    }

    #[test]
    fn multiple_sequential_structs() {
        let packets = [
            Packet { id: 1, length: 10, checksum: 0xAAAA_AAAA },
            Packet { id: 2, length: 20, checksum: 0xBBBB_BBBB },
            Packet { id: 3, length: 30, checksum: 0xCCCC_CCCC },
        ];
        let mut buf = Cursor::new(Vec::new());
        for p in &packets {
            buf.write_fixed(p).unwrap();
        }
        buf.set_position(0);
        for expected in &packets {
            assert_eq!(&buf.read_fixed::<Packet>().unwrap(), expected);
        }
    }

    #[test]
    fn try_struct_roundtrip() {
        let original = Frame { status: Status::Running, payload: 0xCAFE_BABE };
        let mut buf = Cursor::new(Vec::new());
        buf.write_fixed(&original).unwrap();
        buf.set_position(0);
        assert_eq!(buf.read_fixed::<Frame>().unwrap(), original);
    }

    #[test]
    fn try_struct_invalid_discriminant() {
        let mut bytes = [0u8; 5];
        bytes[0] = 99; // invalid Status
        bytes[1..5].copy_from_slice(&0xCAFE_BABEu32.to_le_bytes());
        let result: std::io::Result<Frame> = Cursor::new(bytes.to_vec()).read_fixed();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn primitives_write_read() {
        let mut buf = Cursor::new(Vec::new());
        buf.write_fixed(&42u32).unwrap();
        buf.write_fixed(&LittleEndian::new(0x1234u16)).unwrap();
        buf.set_position(0);
        assert_eq!(buf.read_fixed::<u32>().unwrap(), 42);
        assert_eq!(buf.read_fixed::<LittleEndian<u16>>().unwrap().get(), 0x1234);
    }

    #[test]
    fn vec_roundtrip() {
        let original: Vec<u8> = vec![1, 2, 3, 4, 5];
        let mut buf = Cursor::new(Vec::new());
        buf.write_value(&original).unwrap();
        buf.set_position(0);
        assert_eq!(buf.read_value::<Vec<u8>>().unwrap(), original);
    }

    #[test]
    fn vec_u32_roundtrip() {
        let original: Vec<u32> = vec![0xDEAD, 0xBEEF, 0xCAFE, 0xBABE];
        let mut buf = Cursor::new(Vec::new());
        buf.write_value(&original).unwrap();
        buf.set_position(0);
        assert_eq!(buf.read_value::<Vec<u32>>().unwrap(), original);
    }

    #[test]
    fn option_roundtrip() {
        let some: Option<u32> = Some(0xABCD1234);
        let none: Option<u32> = None;
        for original in [some, none] {
            let mut buf = Cursor::new(Vec::new());
            buf.write_value(&original).unwrap();
            buf.set_position(0);
            assert_eq!(buf.read_value::<Option<u32>>().unwrap(), original);
        }
    }

    #[test]
    fn string_roundtrip() {
        let original = String::from("hello, byteable!");
        let mut buf = Cursor::new(Vec::new());
        buf.write_value(&original).unwrap();
        buf.set_position(0);
        assert_eq!(buf.read_value::<String>().unwrap(), original);
    }

    #[test]
    fn eof_returns_unexpected_eof_error() {
        let result: std::io::Result<u32> = Cursor::new(vec![]).read_value();
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::UnexpectedEof);
    }
}

// ── io_only struct derive ─────────────────────────────────────────────────────

mod io_only_derive {
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

    // ── Byte layout tests ─────────────────────────────────────────────────

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct VecStruct {
        tag: u8,
        data: Vec<u8>,
    }

    #[test]
    fn vec_u8_byte_layout() {
        // tag: 1 byte; data: 8-byte LE u64 length prefix + payload
        let v = VecStruct { tag: 42, data: vec![0xAA, 0xBB] };
        let mut buf = Vec::new();
        buf.write_value(&v).unwrap();
        assert_eq!(buf[0], 42);
        assert_eq!(&buf[1..9], &2u64.to_le_bytes());
        assert_eq!(&buf[9..11], &[0xAA, 0xBB]);
    }

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct OptionU64Struct {
        value: Option<u64>,
    }

    #[test]
    fn option_u64_byte_layout() {
        // Option<u64>: 1-byte discriminant (1 = Some), then u64 LE
        let v = OptionU64Struct { value: Some(0xFF) };
        let mut buf = Vec::new();
        buf.write_value(&v).unwrap();
        assert_eq!(buf[0], 1); // Some
        assert_eq!(&buf[1..9], &255u64.to_le_bytes());
    }

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct MixedStruct {
        #[byteable(big_endian)]
        port: u16,
        payload: Vec<u8>,
    }

    #[test]
    fn big_endian_field_with_vec() {
        let v = MixedStruct { port: 8080, payload: vec![0xDE, 0xAD] };
        let mut buf = Vec::new();
        buf.write_value(&v).unwrap();
        // 8080 = 0x1F90, big-endian
        assert_eq!(&buf[0..2], &[0x1F, 0x90]);
        let restored: MixedStruct = Cursor::new(&buf).read_value().unwrap();
        assert_eq!(restored, v);
    }

    // ── Collection field roundtrips ───────────────────────────────────────

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct ByteVecPacket {
        tag: u8,
        data: Vec<u8>,
    }

    #[test]
    fn vec_u8_field_roundtrip() {
        assert_eq!(
            roundtrip(&ByteVecPacket { tag: 7, data: vec![1, 2, 3, 4, 5] }),
            ByteVecPacket { tag: 7, data: vec![1, 2, 3, 4, 5] }
        );
        assert_eq!(
            roundtrip(&ByteVecPacket { tag: 0, data: vec![] }),
            ByteVecPacket { tag: 0, data: vec![] }
        );
    }

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct U32VecPacket {
        tag: u8,
        values: Vec<u32>,
    }

    #[test]
    fn vec_u32_field_roundtrip() {
        let original = U32VecPacket { tag: 5, values: vec![0xDEAD, 0xBEEF, 0xCAFE, 0xBABE] };
        assert_eq!(roundtrip(&original), original);
    }

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct DequePacket {
        tag: u8,
        queue: VecDeque<u32>,
    }

    #[test]
    fn vecdeque_field_roundtrip() {
        let original = DequePacket { tag: 1, queue: VecDeque::from([0xDEAD, 0xBEEF, 0xCAFE]) };
        assert_eq!(roundtrip(&original), original);
        assert_eq!(roundtrip(&DequePacket { tag: 0, queue: VecDeque::new() }), DequePacket { tag: 0, queue: VecDeque::new() });
    }

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

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct HashSetPacket {
        version: u8,
        members: HashSet<u32>,
    }

    #[test]
    fn hashset_field_roundtrip() {
        let original = HashSetPacket { version: 1, members: HashSet::from([10u32, 20, 30, 40]) };
        assert_eq!(roundtrip(&original), original);
    }

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

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct BTreeSetPacket {
        version: u8,
        ids: BTreeSet<u32>,
    }

    #[test]
    fn btreeset_field_roundtrip() {
        let original = BTreeSetPacket { version: 2, ids: BTreeSet::from([5u32, 10, 15, 20]) };
        assert_eq!(roundtrip(&original), original);
    }

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
    fn frame_packet_byte_layout() {
        let frame = FramePacket { sequence: 1, checksum: 0x0102, payload: vec![0xFF] };
        let mut buf = Vec::new();
        buf.write_value(&frame).unwrap();
        assert_eq!(&buf[0..4], &1u32.to_le_bytes());      // sequence LE
        assert_eq!(&buf[4..6], &0x0102u16.to_be_bytes()); // checksum BE
        assert_eq!(&buf[6..14], &1u64.to_le_bytes());     // Vec length prefix
        assert_eq!(buf[14], 0xFF);
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

    // ── Nested and tuple io_only structs ──────────────────────────────────

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

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct TupleWithCollections(u8, Vec<u32>, BTreeSet<u8>);

    #[test]
    fn tuple_with_collections_roundtrip() {
        let original = TupleWithCollections(42, vec![1, 2, 3], BTreeSet::from([10u8, 20, 30]));
        assert_eq!(roundtrip(&original), original);
    }

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct UnitIo;

    #[test]
    fn unit_io_only_roundtrip() {
        let mut buf = Vec::new();
        buf.write_value(&UnitIo).unwrap();
        assert!(buf.is_empty());
        assert_eq!(Cursor::new(&buf).read_value::<UnitIo>().unwrap(), UnitIo);
    }
}

// ── Collection types ──────────────────────────────────────────────────────────

mod collections {
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
    fn vecdeque_roundtrip() {
        let original: VecDeque<u32> = VecDeque::from([0xDEAD, 0xBEEF, 0xCAFE]);
        assert_eq!(roundtrip(&original), original);
        assert_eq!(roundtrip(&VecDeque::<u8>::new()), VecDeque::<u8>::new());
    }

    #[test]
    fn hashmap_roundtrip() {
        let original: HashMap<u8, u32> = HashMap::from([(1, 100), (2, 200), (3, 300)]);
        assert_eq!(roundtrip(&original), original);
    }

    #[test]
    fn hashset_roundtrip() {
        let original: HashSet<u32> = HashSet::from([10, 20, 30, 40]);
        assert_eq!(roundtrip(&original), original);
    }

    #[test]
    fn btreemap_roundtrip() {
        let original: BTreeMap<u8, u32> = BTreeMap::from([(1, 100), (2, 200), (3, 300)]);
        assert_eq!(roundtrip(&original), original);
    }

    #[test]
    fn btreeset_roundtrip() {
        let original: BTreeSet<u32> = BTreeSet::from([5, 10, 15, 20]);
        assert_eq!(roundtrip(&original), original);
    }

    #[test]
    fn result_roundtrip() {
        let ok: Result<u32, u8> = Ok(0xDEAD_BEEF);
        let err: Result<u32, u8> = Err(42);
        assert_eq!(roundtrip(&ok), ok);
        assert_eq!(roundtrip(&err), err);
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
        let result: std::io::Result<Option<u32>> = Cursor::new(vec![2u8]).read_value();
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn result_invalid_tag_is_err() {
        let result: std::io::Result<Result<u32, u8>> = Cursor::new(vec![2u8]).read_value();
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    }
}
