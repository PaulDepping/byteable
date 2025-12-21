//! Core Byteable trait and related functionality.
//!
//! This module defines the core `Byteable` trait along with supporting traits
//! and macros for converting types to and from byte arrays.

use crate::byte_array::ByteableByteArray;

/// Trait for types that can be converted to and from a byte array.
///
/// This trait is central to the `byteable` crate, enabling structured data
/// to be easily serialized into and deserialized from byte arrays.
pub trait Byteable: Copy {
    const BINARY_SIZE: usize = Self::ByteArray::BINARY_SIZE;
    /// The associated byte array type that can represent `Self`.
    type ByteArray: ByteableByteArray;
    /// Converts `self` into its `ByteableByteArray` representation.
    fn as_bytearray(self) -> Self::ByteArray;
    /// Creates an instance of `Self` from a `ByteableByteArray`.
    fn from_bytearray(ba: Self::ByteArray) -> Self;
}

/// Macro to implement the `Byteable` trait for types.
///
/// This macro generates a `Byteable` implementation using `std::mem::transmute`
/// to convert between the type and its byte array representation.
///
/// # Safety
///
/// The implementation assumes the type has `#[repr(C, packed)]` or similar
/// to ensure a consistent memory layout for safe transmutation.
#[macro_export]
macro_rules! impl_byteable {
    ($($type:ty),+) => {
        $(
            impl Byteable for $type {
                type ByteArray = [u8; std::mem::size_of::<Self>()];
                fn as_bytearray(self) -> Self::ByteArray {
                    // Safety: This is safe because #[repr(C, packed)] ensures consistent memory layout
                    // and the size of Self matches the size of Self::ByteArray.
                    // The Byteable trait requires that the struct is `Copy`.
                    unsafe { std::mem::transmute(self) }
                }
                fn from_bytearray(ba: Self::ByteArray) -> Self {
                    // Safety: This is safe because #[repr(C, packed)] ensures consistent memory layout
                    // and the size of Self matches the size of Self::ByteArray.
                    // The Byteable trait requires that the struct is `Copy`.
                    unsafe { std::mem::transmute(ba) }
                }
            }
        )+
    };
}

macro_rules! impl_byteable_primitive {
    ($($type:ty),+) => {
        $(
            impl Byteable for $type {
                type ByteArray = [u8; std::mem::size_of::<Self>()];
                fn as_bytearray(self) -> Self::ByteArray {
                    <$type>::to_ne_bytes(self)
                }
                fn from_bytearray(ba: Self::ByteArray) -> Self {
                    <$type>::from_ne_bytes(ba)
                }
            }
        )+
    };
}

impl_byteable_primitive!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

/// Trait for types that have a raw byteable representation and can be converted to/from a regular form.
///
/// This trait is automatically implemented for types that implement `Byteable` when there is
/// a corresponding `ByteableRegular` type that uses them as their raw representation.
///
/// This trait facilitates a pattern where you have a "raw" type (suitable for byte serialization)
/// and a "regular" type (more convenient for application logic), and you need to convert between them.
pub trait ByteableRaw<Regular>: Byteable {
    /// Converts the raw representation to the regular form.
    fn to_regular(self) -> Regular;
    /// Converts the regular form to the raw representation.
    fn from_regular(regular: Regular) -> Self;
}

/// Trait for types that can be represented in a raw byteable form.
///
/// This trait allows types to specify an associated raw type that implements `Byteable`,
/// providing conversion methods between the regular type and its raw representation.
///
/// By implementing this trait, your type automatically gains a `Byteable` implementation
/// that delegates to the raw type's implementation.
///
/// # Example
///
/// This is useful for types that need preprocessing before serialization, such as
/// converting between different representations (e.g., IPv4 addresses as `u32` vs `[u8; 4]`) or setting a concrete endianness for members.
pub trait ByteableRegular: Sized {
    /// The raw byteable type that represents this type in serialized form.
    type Raw: Byteable;
    /// Converts this type to its raw representation.
    fn to_raw(&self) -> Self::Raw;
    /// Constructs this type from its raw representation.
    fn from_raw(raw: Self::Raw) -> Self;
}

impl<Raw, Regular> ByteableRaw<Regular> for Raw
where
    Regular: ByteableRegular<Raw = Raw>,
    Raw: Byteable,
{
    fn to_regular(self) -> Regular {
        Regular::from_raw(self)
    }

    fn from_regular(regular: Regular) -> Self {
        regular.to_raw()
    }
}

impl<Raw, Regular> Byteable for Regular
where
    Regular: ByteableRegular<Raw = Raw> + Copy,
    Raw: Byteable,
{
    type ByteArray = Raw::ByteArray;

    fn as_bytearray(self) -> Self::ByteArray {
        self.to_raw().as_bytearray()
    }

    fn from_bytearray(ba: Self::ByteArray) -> Self {
        Self::from_raw(Raw::from_bytearray(ba))
    }
}

#[cfg(test)]
mod tests {
    use crate::{BigEndian, Byteable, LittleEndian};

    #[derive(Clone, Copy, PartialEq, Debug)]
    #[repr(C, packed)]
    struct ABC {
        a: LittleEndian<u16>,
        b: LittleEndian<u16>,
        c: BigEndian<u16>,
    }
    impl_byteable!(ABC);

    #[test]
    fn test_impl() {
        let a = ABC {
            a: LittleEndian::new(1),
            b: LittleEndian::new(2),
            c: BigEndian::new(3),
        };

        let expected_bytes = [1, 0, 2, 0, 0, 3];
        assert_eq!(a.as_bytearray(), expected_bytes);

        let read_a = ABC::from_bytearray(expected_bytes);
        assert_eq!(read_a.a.get(), 1);
        assert_eq!(read_a.b.get(), 2);
        assert_eq!(read_a.c.get(), 3);
        assert_eq!(read_a, a);
    }

    #[test]
    fn test_cursor() {
        let a = ABC {
            a: LittleEndian::new(1),
            b: LittleEndian::new(2),
            c: BigEndian::new(3),
        };

        let expected_bytes = [1, 0, 2, 0, 0, 3];
        assert_eq!(a.as_bytearray(), expected_bytes);

        let read = ABC::from_bytearray(expected_bytes);
        assert_eq!(read.a.get(), 1);
        assert_eq!(read.b.get(), 2);
        assert_eq!(read.c.get(), 3);
        assert_eq!(read, a);
    }

    // Test ByteableRegular trait
    use super::{ByteableRaw, ByteableRegular};

    // Raw representation (suitable for byte serialization)
    #[derive(Clone, Copy, PartialEq, Debug)]
    #[repr(C, packed)]
    struct MyRawStruct {
        a: u8,
        b: LittleEndian<u32>,
        c: LittleEndian<u16>,
        d: u8,
        e: u8,
    }
    impl_byteable!(MyRawStruct);

    // Regular representation (more convenient for application logic)
    #[derive(Clone, Copy, PartialEq, Debug)]
    struct MyRegularStruct {
        a: u8,
        b: u32,
        c: u16,
        d: u8,
        e: u8,
    }

    impl ByteableRegular for MyRegularStruct {
        type Raw = MyRawStruct;

        fn to_raw(&self) -> Self::Raw {
            MyRawStruct {
                a: self.a,
                b: LittleEndian::new(self.b),
                c: LittleEndian::new(self.c),
                d: self.d,
                e: self.e,
            }
        }

        fn from_raw(raw: Self::Raw) -> Self {
            MyRegularStruct {
                a: raw.a,
                b: raw.b.get(),
                c: raw.c.get(),
                d: raw.d,
                e: raw.e,
            }
        }
    }

    #[test]
    fn test_byteable_regular() {
        // Create a regular IPv4 address
        let my_struct = MyRegularStruct {
            a: 192,
            b: 168,
            c: 1,
            d: 1,
            e: 2,
        };

        // Test that ByteableRegular automatically implements Byteable
        let bytes = my_struct.as_bytearray();
        assert_eq!(bytes, [192, 168, 0, 0, 0, 1, 0, 1, 2]);

        // Test conversion back
        let struct_from_bytes = MyRegularStruct::from_bytearray([192, 168, 0, 0, 0, 1, 0, 1, 2]);
        assert_eq!(struct_from_bytes, my_struct);

        // Test binary_size
        assert_eq!(MyRegularStruct::BINARY_SIZE, 9);
    }

    #[test]
    fn test_byteable_raw_conversion() {
        // Test the ByteableRaw trait for converting between raw and regular forms
        let my_struct = MyRegularStruct {
            a: 10,
            b: 0,
            c: 0,
            d: 1,
            e: 2,
        };

        let raw = MyRawStruct::from_regular(my_struct);
        assert_eq!(raw.as_bytearray(), [10, 0, 0, 0, 0, 0, 0, 1, 2]);

        let regular: MyRegularStruct = raw.to_regular();
        assert_eq!(regular, my_struct);
    }
}
