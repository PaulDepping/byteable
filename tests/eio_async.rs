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
