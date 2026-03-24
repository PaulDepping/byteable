//! Integration tests for `#[byteable(io_only)]` struct derive.
//!
//! Structs annotated with `#[byteable(io_only)]` implement `Readable` + `Writable` directly
//! (sequential field I/O) instead of the transmute-based `IntoByteArray`/`FromByteArray` path.
//! This enables structs containing `Vec<T>`, `String`, `Option<T>`, etc.

#![cfg(all(feature = "derive", feature = "std"))]

use byteable::{Byteable, ReadValue, WriteValue};
use std::io::Cursor;

// ── Named struct with Vec<u8> ─────────────────────────────────────────────────

#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct VecStruct {
    tag: u8,
    data: Vec<u8>,
}

#[test]
fn vec_field_roundtrip() {
    let original = VecStruct {
        tag: 7,
        data: vec![1, 2, 3, 4, 5],
    };
    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();
    let decoded: VecStruct = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn vec_field_byte_layout() {
    // tag: 1 byte; data: 8-byte LE u64 length prefix + payload
    let v = VecStruct {
        tag: 42,
        data: vec![0xAA, 0xBB],
    };
    let mut buf = Vec::new();
    buf.write_value(&v).unwrap();

    assert_eq!(buf[0], 42);
    assert_eq!(&buf[1..9], &2u64.to_le_bytes());
    assert_eq!(buf[9], 0xAA);
    assert_eq!(buf[10], 0xBB);
}

// ── Named struct with String ──────────────────────────────────────────────────

#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct StringStruct {
    id: u8,
    name: String,
}

#[test]
fn string_field_roundtrip() {
    let original = StringStruct {
        id: 1,
        name: "hello".to_string(),
    };
    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();
    let decoded: StringStruct = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}

// ── Named struct with Option<u8> ─────────────────────────────────────────────

#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct OptionStruct {
    value: Option<u8>,
}

#[test]
fn option_some_roundtrip() {
    let original = OptionStruct { value: Some(42) };
    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();
    let decoded: OptionStruct = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn option_none_roundtrip() {
    let original = OptionStruct { value: None };
    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();
    let decoded: OptionStruct = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}

// ── Named struct with Option<u64> (default LE) ───────────────────────────────

#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct OptionU64Struct {
    value: Option<u64>,
}

#[test]
fn option_u64_some_roundtrip() {
    let original = OptionU64Struct {
        value: Some(0x0102030405060708),
    };
    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();
    let decoded: OptionU64Struct = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn option_u64_none_roundtrip() {
    let original = OptionU64Struct { value: None };
    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();
    let decoded: OptionU64Struct = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn option_u64_some_byte_layout() {
    // Option<u64>: 1-byte discriminant (1 = Some), then u64 as little-endian
    let original = OptionU64Struct { value: Some(0xFF) };
    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();

    assert_eq!(buf[0], 1); // Some discriminant
    assert_eq!(&buf[1..9], &255u64.to_le_bytes());
}

// ── Mixed: endian-annotated numeric + Vec ─────────────────────────────────────

#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct MixedStruct {
    #[byteable(big_endian)]
    port: u16,
    payload: Vec<u8>,
}

#[test]
fn mixed_struct_roundtrip() {
    let original = MixedStruct {
        port: 8080,
        payload: vec![0xDE, 0xAD],
    };
    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();

    // port 8080 == 0x1F90, big-endian → [0x1F, 0x90]
    assert_eq!(buf[0], 0x1F);
    assert_eq!(buf[1], 0x90);

    let decoded: MixedStruct = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}

// ── Tuple struct ─────────────────────────────────────────────────────────────

#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct TupleIo(u8, Vec<u8>);

#[test]
fn tuple_io_roundtrip() {
    let original = TupleIo(99, vec![1, 2, 3]);
    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();
    let decoded: TupleIo = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}

// ── Unit struct ───────────────────────────────────────────────────────────────

#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct UnitIo;

#[test]
fn unit_io_roundtrip() {
    let original = UnitIo;
    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();
    assert!(buf.is_empty());
    let decoded: UnitIo = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}
