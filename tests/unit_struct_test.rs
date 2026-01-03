//! Tests for unit struct support in derive macros

#![cfg(feature = "derive")]

use byteable::{Byteable, UnsafeByteableTransmute};

#[test]
fn test_unit_struct_with_byteable_derive() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Byteable)]
    struct Marker;

    // Unit structs should have zero-size byte arrays
    let marker = Marker;
    let bytes = marker.to_byte_array();
    assert_eq!(bytes.len(), 0);
    assert_eq!(bytes, []);

    // Should be able to reconstruct from empty byte array
    let restored = Marker::from_byte_array([]);
    assert_eq!(restored, marker);
}

#[test]
fn test_unit_struct_with_unsafe_byteable_transmute() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, UnsafeByteableTransmute)]
    struct Flag;

    // Unit structs should have zero-size byte arrays
    let flag = Flag;
    let bytes = flag.to_byte_array();
    assert_eq!(bytes.len(), 0);
    assert_eq!(bytes, []);

    // Should be able to reconstruct from empty byte array
    let restored = Flag::from_byte_array([]);
    assert_eq!(restored, flag);
}

#[test]
fn test_multiple_unit_structs() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Byteable)]
    struct TypeA;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Byteable)]
    struct TypeB;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, UnsafeByteableTransmute)]
    struct TypeC;

    // All should have zero size
    assert_eq!(TypeA.to_byte_array(), []);
    assert_eq!(TypeB.to_byte_array(), []);
    assert_eq!(TypeC.to_byte_array(), []);

    // All should be restorable
    assert_eq!(TypeA::from_byte_array([]), TypeA);
    assert_eq!(TypeB::from_byte_array([]), TypeB);
    assert_eq!(TypeC::from_byte_array([]), TypeC);
}

#[test]
fn test_unit_struct_in_generic_context() {
    use byteable::Byteable;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Byteable)]
    struct Token;

    fn serialize<T: Byteable>(value: T) -> T::ByteArray {
        value.to_byte_array()
    }

    fn deserialize<T: Byteable>(bytes: T::ByteArray) -> T {
        T::from_byte_array(bytes)
    }

    let token = Token;
    let bytes = serialize(token);
    let restored = deserialize::<Token>(bytes);

    assert_eq!(bytes, []);
    assert_eq!(restored, token);
}

#[test]
fn test_unit_struct_size() {
    use std::mem::size_of;

    #[derive(Byteable)]
    struct Empty;

    #[derive(UnsafeByteableTransmute)]
    struct AlsoEmpty;

    // Unit structs should have zero size
    assert_eq!(size_of::<Empty>(), 0);
    assert_eq!(size_of::<AlsoEmpty>(), 0);

    // Their byte arrays should also have zero size
    assert_eq!(size_of::<<Empty as byteable::Byteable>::ByteArray>(), 0);
    assert_eq!(size_of::<<AlsoEmpty as byteable::Byteable>::ByteArray>(), 0);
}

#[test]
fn test_unit_struct_byteable_raw() {
    use byteable::ByteableRaw;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Byteable)]
    struct Sentinel;

    // Unit structs should implement ByteableRaw with Raw = Self
    let sentinel = Sentinel;

    // The raw type should be the same as the original for unit structs
    let _raw: <Sentinel as ByteableRaw>::Raw = sentinel;
}
