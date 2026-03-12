//! Integration tests for async I/O traits.
//!
//! Tests AsyncReadByteable and AsyncWriteByteable
//! against derived structs using tokio's duplex channel and Cursor.
#![cfg(feature = "tokio")]

use std::io::Cursor;

#[cfg(feature = "derive")]
mod derive_async_io_tests {
    use super::*;
    use byteable::{AsyncReadByteable, AsyncWriteByteable, Byteable, LittleEndian};

    // ============================================================================
    // Simple derived struct (infallible)
    // ============================================================================

    #[derive(Byteable, Clone, Copy, Debug, PartialEq)]
    struct Header {
        version: u8,
        #[byteable(little_endian)]
        length: u16,
        #[byteable(big_endian)]
        magic: u32,
    }

    #[tokio::test]
    async fn test_async_write_then_read_derived_struct() {
        let original = Header {
            version: 2,
            length: 0x1234,
            magic: 0xDEAD_BEEF,
        };

        let mut buf = Cursor::new(Vec::new());
        buf.write_byteable(&original).await.unwrap();

        buf.set_position(0);
        let restored: Header = buf.read_byteable().await.unwrap();
        assert_eq!(restored, original);
    }

    #[tokio::test]
    async fn test_async_write_multiple_structs() {
        let headers = [
            Header { version: 1, length: 10, magic: 0x0000_0001 },
            Header { version: 2, length: 20, magic: 0x0000_0002 },
            Header { version: 3, length: 30, magic: 0x0000_0003 },
        ];

        let mut buf = Cursor::new(Vec::new());
        for h in &headers {
            buf.write_byteable(h).await.unwrap();
        }

        buf.set_position(0);
        for expected in &headers {
            let restored: Header = buf.read_byteable().await.unwrap();
            assert_eq!(&restored, expected);
        }
    }

    // ============================================================================
    // Struct with try_transparent enum field (read_byteable handles fallible conversion)
    // ============================================================================

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
    async fn test_async_write_read_try_struct() {
        let original = Frame {
            status: Status::Running,
            payload: 0xCAFE_BABE,
        };

        let mut buf = Cursor::new(Vec::new());
        // Frame implements IntoByteArray → write_byteable
        buf.write_byteable(&original).await.unwrap();

        buf.set_position(0);
        // Frame implements TryFromByteArray → read_byteable
        let restored: Frame = buf.read_byteable().await.unwrap();
        assert_eq!(restored, original);
    }

    #[tokio::test]
    async fn test_async_read_try_struct_invalid_discriminant() {
        let mut bytes = [0u8; 5]; // Frame is 1 + 4 bytes
        bytes[0] = 99; // Invalid Status
        bytes[1..5].copy_from_slice(&0xCAFE_BABEu32.to_le_bytes());

        let mut buf = Cursor::new(bytes.to_vec());
        let result: std::io::Result<Frame> = buf.read_byteable().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn test_async_write_then_read_try_primitives() {
        let mut buf = Cursor::new(Vec::new());
        buf.write_byteable(&LittleEndian::new(0x1234u16)).await.unwrap();
        buf.write_byteable(&100u8).await.unwrap();

        buf.set_position(0);
        let v1: LittleEndian<u16> = buf.read_byteable().await.unwrap();
        let v2: u8 = buf.read_byteable().await.unwrap();

        assert_eq!(v1.get(), 0x1234);
        assert_eq!(v2, 100);
    }

    // ============================================================================
    // Collection async I/O
    // ============================================================================

    #[tokio::test]
    async fn test_async_vec_roundtrip() {
        let original: Vec<u8> = vec![10, 20, 30, 40, 50];

        let mut buf = Cursor::new(Vec::new());
        buf.write_byteable(&original).await.unwrap();

        buf.set_position(0);
        let restored: Vec<u8> = buf.read_byteable().await.unwrap();
        assert_eq!(restored, original);
    }

    #[tokio::test]
    async fn test_async_vec_u32_roundtrip() {
        let original: Vec<u32> = vec![0xDEAD, 0xBEEF, 0xCAFE];

        let mut buf = Cursor::new(Vec::new());
        buf.write_byteable(&original).await.unwrap();

        buf.set_position(0);
        let restored: Vec<u32> = buf.read_byteable().await.unwrap();
        assert_eq!(restored, original);
    }

    #[tokio::test]
    async fn test_async_option_some_roundtrip() {
        let original: Option<u64> = Some(0xABCD_1234_5678_9ABC);

        let mut buf = Cursor::new(Vec::new());
        buf.write_byteable(&original).await.unwrap();

        buf.set_position(0);
        let restored: Option<u64> = buf.read_byteable().await.unwrap();
        assert_eq!(restored, original);
    }

    #[tokio::test]
    async fn test_async_option_none_roundtrip() {
        let original: Option<u64> = None;

        let mut buf = Cursor::new(Vec::new());
        buf.write_byteable(&original).await.unwrap();

        buf.set_position(0);
        let restored: Option<u64> = buf.read_byteable().await.unwrap();
        assert_eq!(restored, original);
    }

    // ============================================================================
    // Duplex channel test (true async bidirectional I/O)
    // ============================================================================

    #[tokio::test]
    async fn test_async_duplex_roundtrip() {
        let (mut writer, mut reader) = tokio::io::duplex(256);

        let original = Header {
            version: 5,
            length: 0xABCD,
            magic: 0x1234_5678,
        };

        writer.write_byteable(&original).await.unwrap();
        drop(writer); // close the write end

        let restored: Header = reader.read_byteable().await.unwrap();
        assert_eq!(restored, original);
    }
}
