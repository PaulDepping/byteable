//! Integration tests for sync I/O traits.
//!
//! Tests ReadValue and WriteValue
//! against derived structs using `std::io::Cursor`.
#![cfg(all(feature = "std", feature = "derive"))]

use byteable::{Byteable, LittleEndian, ReadFixed, ReadValue, WriteFixed, WriteValue};
use std::io::Cursor;

// ============================================================================
// Simple derived struct (infallible)
// ============================================================================

#[derive(Byteable, Clone, Copy, Debug, PartialEq)]
struct Packet {
    id: u8,
    #[byteable(little_endian)]
    length: u16,
    #[byteable(big_endian)]
    checksum: u32,
}

#[test]
fn test_write_then_read_derived_struct() {
    let original = Packet {
        id: 7,
        length: 0x1234,
        checksum: 0xDEADBEEF,
    };

    let mut buf = Cursor::new(Vec::new());
    buf.write_fixed(&original).unwrap();

    buf.set_position(0);
    let restored: Packet = buf.read_fixed().unwrap();
    assert_eq!(restored, original);
}

#[test]
fn test_write_multiple_structs_sequential_read() {
    let packets = [
        Packet {
            id: 1,
            length: 10,
            checksum: 0xAAAA_AAAA,
        },
        Packet {
            id: 2,
            length: 20,
            checksum: 0xBBBB_BBBB,
        },
        Packet {
            id: 3,
            length: 30,
            checksum: 0xCCCC_CCCC,
        },
    ];

    let mut buf = Cursor::new(Vec::new());
    for p in &packets {
        buf.write_fixed(p).unwrap();
    }

    buf.set_position(0);
    for expected in &packets {
        let restored: Packet = buf.read_fixed().unwrap();
        assert_eq!(&restored, expected);
    }
}

// ============================================================================
// Struct with try_transparent enum field (read_value handles fallible conversion)
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

#[test]
fn test_write_read_try_struct() {
    let original = Frame {
        status: Status::Running,
        payload: 0xCAFE_BABE,
    };

    let mut buf = Cursor::new(Vec::new());
    // Frame implements IntoByteArray → use write_fixed
    buf.write_fixed(&original).unwrap();

    buf.set_position(0);
    // Frame implements TryFromByteArray → use read_fixed
    let restored: Frame = buf.read_fixed().unwrap();
    assert_eq!(restored, original);
}

#[test]
fn test_read_try_struct_invalid_discriminant() {
    // Manually craft bytes with invalid Status discriminant
    let mut bytes = [0u8; 5]; // Frame is 1 + 4 bytes
    bytes[0] = 99; // Invalid Status
    bytes[1..5].copy_from_slice(&0xCAFE_BABEu32.to_le_bytes());

    let mut buf = Cursor::new(bytes.to_vec());
    let result: std::io::Result<Frame> = buf.read_fixed();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn test_write_then_read_try_primitives() {
    let mut buf = Cursor::new(Vec::new());

    buf.write_fixed(&42u32).unwrap();
    buf.write_fixed(&LittleEndian::new(0x1234u16)).unwrap();

    buf.set_position(0);
    let v1: u32 = buf.read_fixed().unwrap();
    let v2: LittleEndian<u16> = buf.read_fixed().unwrap();

    assert_eq!(v1, 42);
    assert_eq!(v2.get(), 0x1234);
}

// ============================================================================
// Collection I/O
// ============================================================================

#[test]
fn test_vec_roundtrip() {
    let original: Vec<u8> = vec![1, 2, 3, 4, 5];

    let mut buf = Cursor::new(Vec::new());
    buf.write_value(&original).unwrap();

    buf.set_position(0);
    let restored: Vec<u8> = buf.read_value().unwrap();
    assert_eq!(restored, original);
}

#[test]
fn test_vec_u32_roundtrip() {
    let original: Vec<u32> = vec![0xDEAD, 0xBEEF, 0xCAFE, 0xBABE];

    let mut buf = Cursor::new(Vec::new());
    buf.write_value(&original).unwrap();

    buf.set_position(0);
    let restored: Vec<u32> = buf.read_value().unwrap();
    assert_eq!(restored, original);
}

#[test]
fn test_option_some_roundtrip() {
    let original: Option<u32> = Some(0xABCD1234);

    let mut buf = Cursor::new(Vec::new());
    buf.write_value(&original).unwrap();

    buf.set_position(0);
    let restored: Option<u32> = buf.read_value().unwrap();
    assert_eq!(restored, original);
}

#[test]
fn test_option_none_roundtrip() {
    let original: Option<u32> = None;

    let mut buf = Cursor::new(Vec::new());
    buf.write_value(&original).unwrap();

    buf.set_position(0);
    let restored: Option<u32> = buf.read_value().unwrap();
    assert_eq!(restored, original);
}

#[test]
fn test_string_roundtrip() {
    let original = String::from("hello, byteable!");

    let mut buf = Cursor::new(Vec::new());
    buf.write_value(&original).unwrap();

    buf.set_position(0);
    let restored: String = buf.read_value().unwrap();
    assert_eq!(restored, original);
}

#[test]
fn test_read_value_io_error() {
    // Empty buffer → I/O error (EOF) when reading a u32
    let mut buf = Cursor::new(vec![]);
    let result: std::io::Result<u32> = buf.read_value();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );
}
