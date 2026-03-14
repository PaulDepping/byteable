//! Tests for the `#[derive(Byteable)]` macro with mixed field endianness.
#![cfg(feature = "derive")]

use byteable::{ByteRepr, Byteable, FromByteArray, IntoByteArray};

#[derive(Clone, Copy, Byteable)]
struct TestStruct {
    a: u8,
    #[byteable(little_endian)]
    b: u16,
    #[byteable(big_endian)]
    c: u64,
    #[byteable(little_endian)]
    d: f64,
}

fn make_test() -> TestStruct {
    TestStruct {
        a: 42,
        b: 0x1234,
        c: 0x0102030405060708,
        d: 3.14159,
    }
}

#[test]
fn byte_size() {
    // u8(1) + u16(2) + u64(8) + f64(8) = 19
    assert_eq!(TestStruct::BYTE_SIZE, 19);
}

#[test]
fn u8_field_layout() {
    let bytes = make_test().into_byte_array();
    assert_eq!(bytes[0], 42);
}

#[test]
fn le_u16_field_layout() {
    let bytes = make_test().into_byte_array();
    // 0x1234 in little-endian: low byte first
    assert_eq!(bytes[1], 0x34);
    assert_eq!(bytes[2], 0x12);
}

#[test]
fn be_u64_field_layout() {
    let bytes = make_test().into_byte_array();
    // 0x0102030405060708 in big-endian: most-significant byte first
    assert_eq!(
        &bytes[3..11],
        &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
    );
}

#[test]
fn le_f64_field_layout() {
    let bytes = make_test().into_byte_array();
    let d_bytes: [u8; 8] = bytes[11..19].try_into().unwrap();
    assert_eq!(f64::from_le_bytes(d_bytes), 3.14159);
}

#[test]
fn roundtrip() {
    let original = make_test();
    let bytes = original.into_byte_array();
    let restored = TestStruct::from_byte_array(bytes);
    assert_eq!(original.a, restored.a);
    assert_eq!(original.b, restored.b);
    assert_eq!(original.c, restored.c);
    assert_eq!(original.d, restored.d);
}
