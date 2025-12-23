//! Core Byteable trait and related functionality.
//!
//! This module defines the core `Byteable` trait along with supporting traits
//! and macros for converting types to and from byte arrays.

use crate::byte_array::ByteableByteArray;

/// Trait for types that can be converted to and from a byte array.
///
/// This trait is central to the `byteable` crate, enabling structured data
/// to be easily serialized into and deserialized from byte arrays.
pub trait Byteable {
    const BINARY_SIZE: usize = Self::ByteArray::BINARY_SIZE;
    /// The associated byte array type that can represent `Self`.
    type ByteArray: ByteableByteArray;
    /// Converts `self` into its `ByteableByteArray` representation.
    fn as_bytearray(self) -> Self::ByteArray;
    /// Creates an instance of `Self` from a `ByteableByteArray`.
    fn from_bytearray(ba: Self::ByteArray) -> Self;
}

impl<T: Byteable, const SIZE: usize> Byteable for [T; SIZE] {
    type ByteArray = [T::ByteArray; SIZE];

    fn as_bytearray(self) -> Self::ByteArray {
        self.map(T::as_bytearray)
    }

    fn from_bytearray(ba: Self::ByteArray) -> Self {
        ba.map(T::from_bytearray)
    }
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
macro_rules! unsafe_impl_directly_byteable {
    ($($type:ty),+) => {
        $(
            impl $crate::Byteable for $type {
                type ByteArray = [u8; ::std::mem::size_of::<Self>()];
                fn as_bytearray(self) -> Self::ByteArray {
                    // Safety: This is safe because #[repr(C, packed)] ensures consistent memory layout
                    // and the size of Self matches the size of Self::ByteArray.
                    // The Byteable trait requires that the struct is `Copy`.
                    unsafe { ::std::mem::transmute(self) }
                }
                fn from_bytearray(ba: Self::ByteArray) -> Self {
                    // Safety: This is safe because #[repr(C, packed)] ensures consistent memory layout
                    // and the size of Self matches the size of Self::ByteArray.
                    // The Byteable trait requires that the struct is `Copy`.
                    unsafe { ::std::mem::transmute(ba) }
                }
            }
        )+
    };
}

macro_rules! impl_byteable_primitive {
    ($($type:ty),+) => {
        $(
            impl $crate::Byteable for $type {
                type ByteArray = [u8; ::std::mem::size_of::<Self>()];
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

#[cfg(test)]
mod tests {
    use byteable_derive::UnsafeByteable;

    use crate::{BigEndian, Byteable, LittleEndian};

    #[derive(Clone, Copy, PartialEq, Debug, UnsafeByteable)]
    #[repr(C, packed)]
    struct ABC {
        a: LittleEndian<u16>,
        b: LittleEndian<u16>,
        c: BigEndian<u16>,
    }

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

    // Raw representation (suitable for byte serialization)
    #[derive(Clone, Copy, PartialEq, Debug, UnsafeByteable)]
    #[repr(C, packed)]
    struct MyRawStruct {
        a: u8,
        b: LittleEndian<u32>,
        c: LittleEndian<u16>,
        d: u8,
        e: u8,
    }

    // Regular representation (more convenient for application logic)
    #[derive(Clone, Copy, PartialEq, Debug)]
    struct MyRegularStruct {
        a: u8,
        b: u32,
        c: u16,
        d: u8,
        e: u8,
    }

    impl Byteable for MyRegularStruct {
        type ByteArray = <MyRawStruct as Byteable>::ByteArray;

        fn as_bytearray(self) -> Self::ByteArray {
            MyRawStruct {
                a: self.a,
                b: LittleEndian::new(self.b),
                c: LittleEndian::new(self.c),
                d: self.d,
                e: self.e,
            }
            .as_bytearray()
        }

        fn from_bytearray(ba: Self::ByteArray) -> Self {
            let raw = MyRawStruct::from_bytearray(ba);
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
}
