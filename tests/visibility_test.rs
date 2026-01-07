use byteable::{Byteable, FromByteArray, IntoByteArray};

// Test 1: Private struct (default visibility)
#[derive(Clone, Copy, Byteable)]
struct PrivateStruct {
    a: u8,
    #[byteable(little_endian)]
    b: u16,
}

// Test 2: Public struct
#[derive(Clone, Copy, Byteable)]
pub struct PublicStruct {
    a: u8,
    #[byteable(big_endian)]
    b: u32,
}

// Test 3: Crate-visible struct
#[derive(Clone, Copy, Byteable)]
pub(crate) struct CrateStruct {
    a: u8,
    #[byteable(little_endian)]
    b: u64,
}

// Test 4: Super-visible struct
mod inner {
    use byteable::Byteable;

    #[derive(Clone, Copy, Byteable)]
    pub(super) struct SuperStruct {
        pub(super) a: u8,
        #[byteable(big_endian)]
        pub(super) b: u16,
    }
}

// Test 5: Tuple struct with public visibility
#[derive(Clone, Copy, Byteable)]
pub struct PublicTupleStruct(
    u8,
    #[byteable(little_endian)] u16,
    #[byteable(big_endian)] u32,
);

// Test 6: Tuple struct with private visibility
#[derive(Clone, Copy, Byteable)]
struct PrivateTupleStruct(u8, #[byteable(little_endian)] u16);

#[test]
fn test_private_struct_visibility() {
    let s = PrivateStruct { a: 42, b: 0x1234 };
    let bytes = s.into_byte_array();
    let restored = PrivateStruct::from_byte_array(bytes);
    assert_eq!(s.a, restored.a);
    assert_eq!(s.b, restored.b);
}

#[test]
fn test_public_struct_visibility() {
    let s = PublicStruct {
        a: 100,
        b: 0x12345678,
    };
    let bytes = s.into_byte_array();
    let restored = PublicStruct::from_byte_array(bytes);
    assert_eq!(s.a, restored.a);
    assert_eq!(s.b, restored.b);
}

#[test]
fn test_crate_struct_visibility() {
    let s = CrateStruct {
        a: 200,
        b: 0x0102030405060708,
    };
    let bytes = s.into_byte_array();
    let restored = CrateStruct::from_byte_array(bytes);
    assert_eq!(s.a, restored.a);
    assert_eq!(s.b, restored.b);
}

#[test]
fn test_super_struct_visibility() {
    let s = inner::SuperStruct { a: 50, b: 0xABCD };
    let bytes = s.into_byte_array();
    let restored = inner::SuperStruct::from_byte_array(bytes);
    assert_eq!(s.a, restored.a);
    assert_eq!(s.b, restored.b);
}

#[test]
fn test_public_tuple_struct_visibility() {
    let s = PublicTupleStruct(10, 0x5678, 0xDEADBEEF);
    let bytes = s.into_byte_array();
    let restored = PublicTupleStruct::from_byte_array(bytes);
    assert_eq!(s.0, restored.0);
    assert_eq!(s.1, restored.1);
    assert_eq!(s.2, restored.2);
}

#[test]
fn test_private_tuple_struct_visibility() {
    let s = PrivateTupleStruct(255, 0xFFFF);
    let bytes = s.into_byte_array();
    let restored = PrivateTupleStruct::from_byte_array(bytes);
    assert_eq!(s.0, restored.0);
    assert_eq!(s.1, restored.1);
}

#[test]
fn test_endianness_with_visibility() {
    // Test that endianness is preserved correctly with visibility changes
    let s = PublicStruct {
        a: 42,
        b: 0x01020304,
    };
    let bytes = s.into_byte_array();

    // a is u8 at position 0
    assert_eq!(bytes[0], 42);

    // b is big-endian u32 starting at position 1
    assert_eq!(bytes[1], 0x01);
    assert_eq!(bytes[2], 0x02);
    assert_eq!(bytes[3], 0x03);
    assert_eq!(bytes[4], 0x04);
}
