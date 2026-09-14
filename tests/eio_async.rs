//! Integration tests for the `eio_async` (embedded-io-async) support module.
#![cfg(all(feature = "embedded-io-async", feature = "derive"))]

mod fixed {
    use byteable::eio_async::{EioAsyncReadFixed, EioAsyncWriteFixed};
    use byteable::{Byteable, LittleEndian};

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    struct Header {
        #[byteable(big_endian)]
        magic: u32,
        #[byteable(little_endian)]
        version: u16,
    }

    #[tokio::test]
    async fn primitive_roundtrip_over_slice() {
        let mut buf = [0u8; 4];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_fixed(&42u32).await.unwrap();
        }
        let mut r: &[u8] = &buf;
        let val: u32 = r.read_fixed().await.unwrap();
        assert_eq!(val, 42u32);
    }

    #[tokio::test]
    async fn endian_wrapper_roundtrip_over_slice() {
        let original: LittleEndian<u32> = LittleEndian::new(0xDEADBEEF);
        let mut buf = [0u8; 4];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_fixed(&original).await.unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: LittleEndian<u32> = r.read_fixed().await.unwrap();
        assert_eq!(restored.get(), original.get());
    }

    #[tokio::test]
    async fn derived_struct_roundtrip_over_slice() {
        let header = Header {
            magic: 0x1234_5678,
            version: 7,
        };
        let mut buf = [0u8; 6];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_fixed(&header).await.unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: Header = r.read_fixed().await.unwrap();
        assert_eq!(restored, header);
    }

    #[tokio::test]
    async fn read_exact_error_on_truncated_slice() {
        use byteable::eio::EioReadableError;
        let buf = [0u8; 2]; // too short for a u32
        let mut r: &[u8] = &buf;
        let err = r.read_fixed::<u32>().await.unwrap_err();
        assert!(matches!(err, EioReadableError::UnexpectedEof));
    }
}

mod dynamic_over_fixed {
    use byteable::Byteable;
    use byteable::eio_async::{EioAsyncReadValue, EioAsyncWriteValue};

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[tokio::test]
    async fn read_value_write_value_roundtrip_fixed_type() {
        let p = Point { x: -5, y: 100 };
        let mut buf = [0u8; 8];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_value(&p).await.unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: Point = r.read_value().await.unwrap();
        assert_eq!(restored, p);
    }
}

mod option_result {
    use byteable::eio_async::{EioAsyncReadValue, EioAsyncWriteValue};

    #[tokio::test]
    async fn option_some_roundtrip() {
        let val: Option<u32> = Some(7);
        let mut buf = [0u8; 5];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_value(&val).await.unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: Option<u32> = r.read_value().await.unwrap();
        assert_eq!(restored, val);
    }

    #[tokio::test]
    async fn option_none_roundtrip() {
        let val: Option<u32> = None;
        let mut buf = [0u8; 5];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_value(&val).await.unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: Option<u32> = r.read_value().await.unwrap();
        assert_eq!(restored, val);
    }

    #[tokio::test]
    async fn result_roundtrip() {
        let val: Result<u8, u8> = Ok(9);
        let mut buf = [0u8; 2];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_value(&val).await.unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: Result<u8, u8> = r.read_value().await.unwrap();
        assert_eq!(restored, val);
    }
}

mod borrowed_write {
    use byteable::eio_async::EioAsyncWriteValue;

    #[tokio::test]
    async fn write_u8_slice_no_alloc() {
        let data: &[u8] = &[1, 2, 3];
        let mut buf = [0u8; 11];
        let mut w: &mut [u8] = &mut buf;
        w.write_value(data).await.unwrap();
        assert_eq!(&buf[..8], &3u64.to_le_bytes());
        assert_eq!(&buf[8..11], &[1, 2, 3]);
    }

    #[tokio::test]
    async fn write_str_no_alloc() {
        let s: &str = "hi";
        let mut buf = [0u8; 10];
        let mut w: &mut [u8] = &mut buf;
        w.write_value(s).await.unwrap();
        assert_eq!(&buf[..8], &2u64.to_le_bytes());
        assert_eq!(&buf[8..10], b"hi");
    }
}
