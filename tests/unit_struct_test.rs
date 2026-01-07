//! Tests for unit struct support in derive macros

#![cfg(feature = "derive")]

use byteable::{Byteable, FromByteArray, IntoByteArray, UnsafeByteableTransmute};

#[test]
fn test_unit_struct_with_byteable_derive() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Byteable)]
    struct Marker;

    // Unit structs should have zero-size byte arrays
    let marker = Marker;
    let bytes = marker.into_byte_array();
    assert_eq!(bytes.len(), 0);
    assert_eq!(bytes, [0u8; 0]);

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
    let bytes = flag.into_byte_array();
    assert_eq!(bytes.len(), 0);
    assert_eq!(bytes, [0u8; 0]);

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
    assert!(TypeA.into_byte_array().is_empty());
    assert!(TypeB.into_byte_array().is_empty());
    assert!(TypeC.into_byte_array().is_empty());

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

    fn serialize<T: IntoByteArray>(value: T) -> T::ByteArray {
        value.into_byte_array()
    }

    fn deserialize<T: FromByteArray>(bytes: T::ByteArray) -> T {
        T::from_byte_array(bytes)
    }

    let token = Token;
    let bytes = serialize(token);
    let restored = deserialize::<Token>(bytes);

    assert!(bytes.is_empty());
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
    assert_eq!(
        size_of::<<Empty as byteable::AssociatedByteArray>::ByteArray>(),
        0
    );
    assert_eq!(
        size_of::<<AlsoEmpty as byteable::AssociatedByteArray>::ByteArray>(),
        0
    );
}

#[test]
fn test_unit_struct_byteable_raw() {
    use byteable::HasRawType;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Byteable)]
    struct Sentinel;

    // Unit structs should implement HasRawType with Raw = Self
    let sentinel = Sentinel;

    // The raw type should be the same as the original for unit structs
    let _raw: <Sentinel as HasRawType>::Raw = sentinel;
}
