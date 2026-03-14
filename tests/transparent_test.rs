//! Tests for the `#[byteable(transparent)]` field attribute.
//!
//! A transparent field stores a nested `Byteable` struct inline using its raw byte
//! representation rather than wrapping it in an endian marker.
#![cfg(feature = "derive")]

use byteable::{ByteRepr, Byteable, FromByteArray, IntoByteArray};

#[derive(Clone, Copy, Byteable)]
struct MemberStruct {
    a: u8,
    #[byteable(little_endian)]
    b: u16,
}

#[derive(Clone, Copy, Byteable)]
struct TestStruct {
    #[byteable(transparent)]
    member: MemberStruct,
    a: u8,
    #[byteable(little_endian)]
    b: u16,
    #[byteable(big_endian)]
    c: u64,
    #[byteable(little_endian)]
    d: f64,
}

// ── MemberStruct ─────────────────────────────────────────────────────────────

#[test]
fn member_struct_byte_size() {
    // u8(1) + u16(2) = 3
    assert_eq!(MemberStruct::BYTE_SIZE, 3);
}

#[test]
fn member_struct_byte_layout() {
    let m = MemberStruct { a: 10, b: 0x1234 };
    let bytes = m.into_byte_array();
    assert_eq!(bytes[0], 10); // a
    assert_eq!(bytes[1], 0x34); // b low byte (little-endian)
    assert_eq!(bytes[2], 0x12); // b high byte
}

// ── TestStruct (with transparent member) ─────────────────────────────────────

#[test]
fn outer_struct_byte_size() {
    // member(3) + u8(1) + u16(2) + u64(8) + f64(8) = 22
    assert_eq!(TestStruct::BYTE_SIZE, 22);
}

#[test]
fn transparent_field_at_start() {
    let member = MemberStruct { a: 10, b: 0x1234 };
    let outer = TestStruct {
        member,
        a: 0,
        b: 0,
        c: 0,
        d: 0.0,
    };
    let bytes = outer.into_byte_array();
    let member_bytes = member.into_byte_array();
    assert_eq!(&bytes[0..3], member_bytes.as_ref());
}

#[test]
fn outer_field_layout() {
    let outer = TestStruct {
        member: MemberStruct { a: 10, b: 0x1234 },
        a: 42,
        b: 0x5678,
        c: 0x0102030405060708,
        d: 3.14159,
    };
    let bytes = outer.into_byte_array();

    // a at byte 3
    assert_eq!(bytes[3], 42);
    // b (little-endian u16) at bytes 4-5
    assert_eq!(bytes[4], 0x78);
    assert_eq!(bytes[5], 0x56);
    // c (big-endian u64) at bytes 6-13
    assert_eq!(
        &bytes[6..14],
        &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
    );
    // d (little-endian f64) at bytes 14-21
    let d_bytes: [u8; 8] = bytes[14..22].try_into().unwrap();
    assert_eq!(f64::from_le_bytes(d_bytes), 3.14159);
}

#[test]
fn roundtrip() {
    let original = TestStruct {
        member: MemberStruct { a: 10, b: 0x1234 },
        a: 42,
        b: 0x5678,
        c: 0x0102030405060708,
        d: 3.14159,
    };
    let bytes = original.into_byte_array();
    let restored = TestStruct::from_byte_array(bytes);
    assert_eq!(original.member.a, restored.member.a);
    assert_eq!(original.member.b, restored.member.b);
    assert_eq!(original.a, restored.a);
    assert_eq!(original.b, restored.b);
    assert_eq!(original.c, restored.c);
    assert_eq!(original.d, restored.d);
}
