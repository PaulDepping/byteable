//! Integration tests for asynchronous I/O traits.
//!
//! Covers async fixed-size I/O (`AsyncReadFixed`/`AsyncWriteFixed`), async
//! value/stream I/O (`AsyncReadValue`/`AsyncWriteValue`), and async collection
//! types.
#![cfg(all(feature = "std", feature = "tokio"))]

// ── Async fixed-size I/O ──────────────────────────────────────────────────────

mod fixed_io {
    use byteable::async_io::{
        AsyncFixedReadable, AsyncFixedWritable, AsyncReadFixed, AsyncReadValue, AsyncWriteFixed,
        AsyncWriteValue,
    };
    use byteable::{BigEndian, Byteable, LittleEndian};
    use std::io::Cursor;

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    struct Header {
        #[byteable(big_endian)]
        magic: u32,
        #[byteable(little_endian)]
        version: u16,
    }

    #[tokio::test]
    async fn primitive_roundtrip() {
        let original: LittleEndian<u32> = LittleEndian::new(0xDEADBEEF);
        let mut buf = Cursor::new(Vec::new());
        buf.write_fixed(&original).await.unwrap();
        let restored: LittleEndian<u32> = Cursor::new(buf.into_inner()).read_fixed().await.unwrap();
        assert_eq!(restored.get(), original.get());
    }

    #[tokio::test]
    async fn reverse_roundtrip() {
        use std::cmp::Reverse;

        let original = Reverse(0xDEADBEEFu32);
        let mut buf = Cursor::new(Vec::new());
        buf.write_fixed(&original).await.unwrap();
        buf.set_position(0);
        let restored: Reverse<u32> = buf.read_fixed().await.unwrap();
        assert_eq!(restored, original);
    }

    #[tokio::test]
    async fn derived_struct_roundtrip() {
        let header = Header {
            magic: 0x12345678,
            version: 42,
        };
        let mut buf = Cursor::new(Vec::new());
        buf.write_fixed(&header).await.unwrap();
        let restored: Header = Cursor::new(buf.into_inner()).read_fixed().await.unwrap();
        assert_eq!(restored, header);
    }

    #[tokio::test]
    async fn write_fixed_and_read_value_produce_same_bytes() {
        let header = Header {
            magic: 0xCAFEBABE,
            version: 7,
        };
        let mut buf1 = Cursor::new(Vec::new());
        buf1.write_fixed(&header).await.unwrap();
        let mut buf2 = Cursor::new(Vec::new());
        buf2.write_value(&header).await.unwrap();
        assert_eq!(buf1.into_inner(), buf2.clone().into_inner());
        // read_value can read what write_fixed wrote
        let restored: Header = Cursor::new(buf2.into_inner()).read_value().await.unwrap();
        assert_eq!(restored, header);
    }

    #[tokio::test]
    async fn write_value_readable_by_read_fixed() {
        let val = BigEndian::new(0x0102u16);
        let mut buf = Cursor::new(Vec::new());
        buf.write_value(&val).await.unwrap();
        let restored: BigEndian<u16> = Cursor::new(buf.into_inner()).read_fixed().await.unwrap();
        assert_eq!(restored.get(), val.get());
    }

    #[tokio::test]
    async fn async_fixed_readable_also_implements_async_readable() {
        fn assert_both<T: AsyncFixedReadable + byteable::async_io::AsyncReadable>() {}
        assert_both::<u8>();
        assert_both::<u32>();
        assert_both::<LittleEndian<u64>>();
        assert_both::<Header>();
    }

    #[tokio::test]
    async fn async_fixed_writable_also_implements_async_writable() {
        fn assert_both<T: AsyncFixedWritable + byteable::async_io::AsyncWritable>() {}
        assert_both::<u8>();
        assert_both::<u32>();
        assert_both::<BigEndian<u16>>();
        assert_both::<Header>();
    }

    /// Vec<u32> is NOT accessible via `AsyncReadFixed` or `AsyncWriteFixed`.
    ///
    /// ```compile_fail
    /// use byteable::async_io::AsyncReadFixed;
    /// use std::io::Cursor;
    /// # #[tokio::main] async fn main() {
    /// let mut cursor = Cursor::new(vec![0u8; 16]);
    /// let _: Vec<u32> = cursor.read_fixed().await.unwrap();
    /// # }
    /// ```
    ///
    /// ```compile_fail
    /// use byteable::async_io::AsyncWriteFixed;
    /// use std::io::Cursor;
    /// # #[tokio::main] async fn main() {
    /// let mut cursor = Cursor::new(Vec::new());
    /// cursor.write_fixed(&vec![1u32, 2, 3]).await.unwrap();
    /// # }
    /// ```
    #[tokio::test]
    async fn compile_fail_docs_exist() {}
}

// ── Async value / stream I/O ──────────────────────────────────────────────────

mod value_io {
    use byteable::async_io::{AsyncReadFixed, AsyncReadValue, AsyncWriteFixed, AsyncWriteValue};
    use byteable::io::ReadableError;
    use byteable::{Byteable, LittleEndian};
    use std::io::Cursor;

    #[derive(Byteable, Clone, Copy, Debug, PartialEq)]
    struct Header {
        version: u8,
        #[byteable(little_endian)]
        length: u16,
        #[byteable(big_endian)]
        magic: u32,
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

    #[tokio::test]
    async fn derived_struct_write_then_read() {
        let original = Header {
            version: 2,
            length: 0x1234,
            magic: 0xDEAD_BEEF,
        };
        let mut buf = Cursor::new(Vec::new());
        buf.write_fixed(&original).await.unwrap();
        buf.set_position(0);
        assert_eq!(buf.read_fixed::<Header>().await.unwrap(), original);
    }

    #[tokio::test]
    async fn multiple_sequential_structs() {
        let headers = [
            Header {
                version: 1,
                length: 10,
                magic: 0x0000_0001,
            },
            Header {
                version: 2,
                length: 20,
                magic: 0x0000_0002,
            },
            Header {
                version: 3,
                length: 30,
                magic: 0x0000_0003,
            },
        ];
        let mut buf = Cursor::new(Vec::new());
        for h in &headers {
            buf.write_fixed(h).await.unwrap();
        }
        buf.set_position(0);
        for expected in &headers {
            assert_eq!(&buf.read_fixed::<Header>().await.unwrap(), expected);
        }
    }

    #[tokio::test]
    async fn try_struct_roundtrip() {
        let original = Frame {
            status: Status::Running,
            payload: 0xCAFE_BABE,
        };
        let mut buf = Cursor::new(Vec::new());
        buf.write_fixed(&original).await.unwrap();
        buf.set_position(0);
        assert_eq!(buf.read_fixed::<Frame>().await.unwrap(), original);
    }

    #[tokio::test]
    async fn try_struct_invalid_discriminant() {
        let mut bytes = [0u8; 5];
        bytes[0] = 99; // invalid Status
        bytes[1..5].copy_from_slice(&0xCAFE_BABEu32.to_le_bytes());
        let result: Result<Frame, _> = Cursor::new(bytes.to_vec()).read_fixed().await;
        assert!(matches!(result.unwrap_err(), ReadableError::DecodeError(_)));
    }

    #[tokio::test]
    async fn primitives_write_read() {
        let mut buf = Cursor::new(Vec::new());
        buf.write_fixed(&LittleEndian::new(0x1234u16))
            .await
            .unwrap();
        buf.write_fixed(&100u8).await.unwrap();
        buf.set_position(0);
        assert_eq!(
            buf.read_fixed::<LittleEndian<u16>>().await.unwrap().get(),
            0x1234
        );
        assert_eq!(buf.read_fixed::<u8>().await.unwrap(), 100);
    }

    #[tokio::test]
    async fn vec_roundtrip() {
        let original: Vec<u8> = vec![10, 20, 30, 40, 50];
        let mut buf = Cursor::new(Vec::new());
        buf.write_value(&original).await.unwrap();
        buf.set_position(0);
        assert_eq!(buf.read_value::<Vec<u8>>().await.unwrap(), original);
    }

    #[tokio::test]
    async fn vec_u32_roundtrip() {
        let original: Vec<u32> = vec![0xDEAD, 0xBEEF, 0xCAFE];
        let mut buf = Cursor::new(Vec::new());
        buf.write_value(&original).await.unwrap();
        buf.set_position(0);
        assert_eq!(buf.read_value::<Vec<u32>>().await.unwrap(), original);
    }

    #[tokio::test]
    async fn option_roundtrip() {
        for original in [Some(0xABCD_1234_5678_9ABCu64), None] {
            let mut buf = Cursor::new(Vec::new());
            buf.write_value(&original).await.unwrap();
            buf.set_position(0);
            assert_eq!(buf.read_value::<Option<u64>>().await.unwrap(), original);
        }
    }

    #[tokio::test]
    async fn duplex_channel_roundtrip() {
        let (mut writer, mut reader) = tokio::io::duplex(256);
        let original = Header {
            version: 5,
            length: 0xABCD,
            magic: 0x1234_5678,
        };
        writer.write_fixed(&original).await.unwrap();
        drop(writer);
        assert_eq!(reader.read_fixed::<Header>().await.unwrap(), original);
    }
}

// ── Async collection types ────────────────────────────────────────────────────

mod collections {
    use byteable::async_io::{AsyncReadValue, AsyncWritable, AsyncWriteValue};
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
    use std::io::Cursor;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
    use std::ops::Bound;

    async fn roundtrip<T>(original: &T) -> T
    where
        T: AsyncWritable + byteable::async_io::AsyncReadable,
    {
        let mut buf = Cursor::new(Vec::new());
        buf.write_value(original).await.unwrap();
        buf.set_position(0);
        buf.read_value().await.unwrap()
    }

    #[tokio::test]
    async fn vecdeque_roundtrip() {
        let original: VecDeque<u32> = VecDeque::from([0xDEAD, 0xBEEF, 0xCAFE]);
        assert_eq!(roundtrip(&original).await, original);
    }

    #[tokio::test]
    async fn hashmap_roundtrip() {
        let original: HashMap<u8, u32> = HashMap::from([(1, 100), (2, 200)]);
        assert_eq!(roundtrip(&original).await, original);
    }

    #[tokio::test]
    async fn hashset_roundtrip() {
        let original: HashSet<u32> = HashSet::from([10, 20, 30]);
        assert_eq!(roundtrip(&original).await, original);
    }

    #[tokio::test]
    async fn btreemap_roundtrip() {
        let original: BTreeMap<u8, u32> = BTreeMap::from([(1, 10), (2, 20)]);
        assert_eq!(roundtrip(&original).await, original);
    }

    #[tokio::test]
    async fn btreeset_roundtrip() {
        let original: BTreeSet<u32> = BTreeSet::from([5, 10, 15]);
        assert_eq!(roundtrip(&original).await, original);
    }

    #[tokio::test]
    async fn string_roundtrip() {
        let original = String::from("async hello!");
        assert_eq!(roundtrip(&original).await, original);
    }

    #[tokio::test]
    async fn ip_addr_roundtrip() {
        let v4 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let v6 = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
        assert_eq!(roundtrip(&v4).await, v4);
        assert_eq!(roundtrip(&v6).await, v6);
    }

    #[tokio::test]
    async fn socket_addr_roundtrip() {
        let v4 = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 1), 8080));
        let v6 = SocketAddr::V6(SocketAddrV6::new(
            Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1),
            443,
            0,
            0,
        ));
        assert_eq!(roundtrip(&v4).await, v4);
        assert_eq!(roundtrip(&v6).await, v6);
    }

    #[tokio::test]
    async fn bound_roundtrip() {
        let included: Bound<u32> = Bound::Included(7);
        let excluded: Bound<u32> = Bound::Excluded(9);
        let unbounded: Bound<u32> = Bound::Unbounded;
        assert_eq!(roundtrip(&included).await, included);
        assert_eq!(roundtrip(&excluded).await, excluded);
        assert_eq!(roundtrip(&unbounded).await, unbounded);
    }
}

mod io_only_derive {
    use byteable::Byteable;
    use byteable::async_io::{AsyncReadValue, AsyncWriteValue};
    use std::io::Cursor;

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct Message {
        id: u32,
        flag: Option<u8>,
        #[byteable(big_endian)]
        tag: u16,
    }

    #[tokio::test]
    async fn io_only_struct_roundtrip_over_tokio() {
        let msg = Message {
            id: 42,
            flag: Some(1),
            tag: 0x0102,
        };
        let mut buf = Cursor::new(Vec::new());
        buf.write_value(&msg).await.unwrap();
        // id (4) + flag discriminant (1) + flag payload (1) = 6, so `tag` starts at offset 6
        // and must be laid out most-significant byte first.
        assert_eq!(buf.get_ref().len(), 8);
        assert_eq!(&buf.get_ref()[6..8], &[0x01, 0x02]);
        buf.set_position(0);
        let restored: Message = buf.read_value().await.unwrap();
        assert_eq!(restored, msg);
    }
}

mod field_enum {
    use byteable::Byteable;
    use byteable::async_io::{AsyncReadValue, AsyncWriteValue};
    use std::io::Cursor;

    #[derive(Byteable, Debug, PartialEq)]
    enum Command {
        Nothing,
        Read(u32, u64),
        Write { a: u32, b: u64 },
    }

    #[tokio::test]
    async fn tuple_and_named_variants_roundtrip_over_tokio() {
        for cmd in [
            Command::Nothing,
            Command::Read(1, 2),
            Command::Write { a: 3, b: 4 },
        ] {
            let mut buf = Cursor::new(Vec::new());
            buf.write_value(&cmd).await.unwrap();
            buf.set_position(0);
            let restored: Command = buf.read_value().await.unwrap();
            assert_eq!(restored, cmd);
        }
    }
}
