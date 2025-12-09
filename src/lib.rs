//! # Byteable
//!
//! A Rust crate for converting Rust types to and from byte arrays, facilitating
//! easy serialization and deserialization, especially for network protocols or
//! embedded systems. It provides traits for working with byte arrays,
//! byteable types, and handling endianness.
//!
//! ## Features
//! - `derive`: Enables the `Byteable` derive macro for automatic implementation of the `Byteable` trait.
//! - `tokio`: Provides asynchronous read and write capabilities using `tokio`'s I/O traits.
//!
//! ## Usage
//!
//! ### Basic Byteable Conversion
//!
//! Implement the `Byteable` trait manually or use the `#[derive(Byteable)]` macro (with the `derive` feature enabled):
//!
//! ```rust
//! use byteable::{Byteable, ReadByteable, WriteByteable, LittleEndian};
//! use std::io::Cursor;
//!
//! #[derive(Byteable, Clone, Copy, PartialEq, Debug)]
//! #[repr(C, packed)]
//! struct MyPacket {
//!     id: u16,
//!     value: LittleEndian<u32>,
//! }
//!
//! let packet = MyPacket {
//!     id: 123,
//!     value: LittleEndian::new(0x01020304),
//! };
//!
//! // Convert to byte array
//! let byte_array = packet.as_bytearray();
//!
//! // Write to a writer. Cursor implements `std::io::Write`,
//! // thus it gains `write_one` from `WriteByteable`.
//! let mut buffer = Cursor::new(vec![]);
//! buffer.write_one(packet).unwrap();
//! assert_eq!(buffer.into_inner(), vec![123, 0, 4, 3, 2, 1]);
//!
//! // Read from a reader. Cursor implements `std::io::Read`,
//! // thus it gains `read_one` from `ReadByteable`.
//! let mut reader = Cursor::new(vec![123, 0, 4, 3, 2, 1]);
//! let read_packet: MyPacket = reader.read_one().unwrap();
//! assert_eq!(read_packet, packet);
//! ```
//!
//! ### Endianness Handling
//!
//! Use `BigEndian<T>` or `LittleEndian<T>` wrappers to control the byte order of primitive types.
//!
//! ```rust
//! use byteable::{BigEndian, LittleEndian, Endianable};
//!
//! let value_be = BigEndian::new(0x01020304u32);
//! assert_eq!(value_be.get_raw().to_ne_bytes(), [1, 2, 3, 4]);
//!
//! let value_le = LittleEndian::new(0x01020304u32);
//! assert_eq!(value_le.get_raw().to_ne_bytes(), [4, 3, 2, 1]);
//! ```
//!
//! ### Asynchronous I/O (with `tokio` feature)
//!
//! ```rust
//! #[cfg(feature = "tokio")]
//! async fn async_example() -> std::io::Result<()> {
//!     use byteable::{Byteable, AsyncReadByteable, AsyncWriteByteable, LittleEndian};
//!     use std::io::Cursor;
//!
//!     #[derive(Byteable, Clone, Copy, PartialEq, Debug)]
//!     #[repr(C, packed)]
//!     struct AsyncPacket {
//!         sequence: u8,
//!         data: LittleEndian<u16>,
//!     }
//!
//!     let packet = AsyncPacket {
//!         sequence: 5,
//!         data: LittleEndian::new(0xAABB),
//!     };
//!
//!     let mut buffer = Cursor::new(vec![]);
//!     buffer.write_one(packet).await?;
//!     assert_eq!(buffer.into_inner(), vec![5, 0xBB, 0xAA]);
//!
//!     let mut reader = Cursor::new(vec![5, 0xBB, 0xAA]);
//!     let read_packet: AsyncPacket = reader.read_one().await?;
//!     assert_eq!(read_packet, packet);
//!     Ok(())
//! }
//! ```

#[cfg(feature = "tokio")]
use std::future::Future;
use std::io::{Read, Write};

#[cfg(feature = "derive")]
pub use byteable_derive::Byteable;

#[cfg(feature = "tokio")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Trait for types that can be represented as a byte array.
///
/// This trait provides methods for creating zero-filled byte arrays and
/// accessing them as mutable or immutable byte slices. It is primarily
/// used as an associated type for the `Byteable` trait.
pub trait ByteableByteArray {
    /// Creates a new byte array filled with zeros.
    fn create_zeroed() -> Self;
    /// Returns a mutable slice reference to the underlying byte array.
    #[must_use]
    fn as_byteslice_mut(&mut self) -> &mut [u8];
    /// Returns an immutable slice reference to the underlying byte array.
    #[must_use]
    fn as_byteslice(&self) -> &[u8];
}

/// Implements `ByteableByteArray` for fixed-size arrays `[u8; SIZE]`.
impl<const SIZE: usize> ByteableByteArray for [u8; SIZE] {
    fn create_zeroed() -> Self {
        [0; SIZE]
    }

    fn as_byteslice_mut(&mut self) -> &mut [u8] {
        self
    }

    fn as_byteslice(&self) -> &[u8] {
        self
    }
}

/// Trait for types that can be converted to and from a `ByteableByteArray`.
///
/// This trait is central to the `byteable` crate, enabling structured data
/// to be easily serialized into and deserialized from byte arrays.
/// It requires the type to implement `Copy`.
pub trait Byteable: Copy {
    /// The associated byte array type that can represent `Self`.
    type ByteArray: ByteableByteArray;
    /// Converts `self` into its `ByteableByteArray` representation.
    #[must_use]
    fn as_bytearray(self) -> Self::ByteArray;
    /// Creates an instance of `Self` from a `ByteableByteArray`.
    fn from_bytearray(ba: Self::ByteArray) -> Self;
}

macro_rules! impl_byteable {
    ($type:ident) => {
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
    };
}

macro_rules! impl_byteable_generic {
    ($type:ident, $generic:ident) => {
        impl Byteable for $type<$generic> {
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
    };
}

pub trait ByteableRaw<Regular>: Byteable {
    fn to_regular(self) -> Regular;
    fn from_regular(regular: Regular) -> Self;
}

pub trait ByteableRegular: Sized {
    type Raw: Byteable;
    fn to_raw(self) -> Self::Raw;
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

/// Extends `std::io::Read` with a method to read a `Byteable` type.
pub trait ReadByteable: Read {
    /// Reads one `Byteable` element from the reader.
    ///
    /// This method will create a zero-filled byte array, read enough bytes
    /// from the underlying reader to fill it, and then convert the byte
    /// array into the specified `Byteable` type.
    fn read_one<T: Byteable>(&mut self) -> std::io::Result<T> {
        let mut e = T::ByteArray::create_zeroed();
        self.read_exact(e.as_byteslice_mut())?;
        Ok(T::from_bytearray(e))
    }
}

/// Implements `ReadByteable` for all types that implement `std::io::Read`.
impl<T: Read> ReadByteable for T {}

/// Extends `std::io::Write` with a method to write a `Byteable` type.
pub trait WriteByteable: Write {
    /// Writes one `Byteable` element to the writer.
    ///
    /// This method will convert the `Byteable` data into its byte array
    /// representation and then write all those bytes to the underlying writer.
    fn write_one<T: Byteable>(&mut self, data: T) -> std::io::Result<()> {
        let e = data.as_bytearray();
        self.write_all(e.as_byteslice())
    }
}

/// Implements `WriteByteable` for all types that implement `std::io::Write`.
impl<T: Write> WriteByteable for T {}

/// Extends `tokio::io::AsyncReadExt` with an asynchronous method to read a `Byteable` type.
///
/// This trait is only available when the `tokio` feature is enabled.
#[cfg(feature = "tokio")]
pub trait AsyncReadByteable: tokio::io::AsyncReadExt {
    /// Asynchronously reads one `Byteable` element from the reader.
    ///
    /// This method will create a zero-filled byte array, asynchronously read
    /// enough bytes from the underlying reader to fill it, and then convert
    /// the byte array into the specified `Byteable` type.
    fn read_one<T: Byteable>(&mut self) -> impl Future<Output = std::io::Result<T>>
    where
        Self: Unpin + Send,
    {
        async move {
            let mut e = T::ByteArray::create_zeroed();
            self.read_exact(e.as_byteslice_mut()).await?;
            Ok(T::from_bytearray(e))
        }
    }
}

/// Implements `AsyncReadByteable` for all types that implement `tokio::io::AsyncReadExt`.
#[cfg(feature = "tokio")]
impl<T: AsyncReadExt> AsyncReadByteable for T {}

/// Extends `tokio::io::AsyncWriteExt` with an asynchronous method to write a `Byteable` type.
///
/// This trait is only available when the `tokio` feature is enabled.
#[cfg(feature = "tokio")]
pub trait AsyncWriteByteable: tokio::io::AsyncWriteExt {
    /// Asynchronously writes one `Byteable` element to the writer.
    ///
    /// This method will convert the `Byteable` data into its byte array
    /// representation and then asynchronously write all those bytes to
    /// the underlying writer.
    fn write_one<T: Byteable>(&mut self, data: T) -> impl Future<Output = std::io::Result<()>>
    where
        Self: Unpin,
    {
        async move {
            let e = data.as_bytearray();
            self.write_all(e.as_byteslice()).await
        }
    }
}

/// Implements `AsyncWriteByteable` for all types that implement `tokio::io::AsyncWriteExt`.
#[cfg(feature = "tokio")]
impl<T: AsyncWriteExt> AsyncWriteByteable for T {}

/// Trait for types that support endianness conversion.
///
/// This trait provides methods to convert values to and from little-endian (LE)
/// and big-endian (BE) byte orders. It is implemented for most primitive integer
/// and floating-point types.
pub trait Endianable: Copy {
    /// Converts a value from its little-endian representation to the native endianness.
    fn from_le(self) -> Self;
    /// Converts a value from its big-endian representation to the native endianness.
    fn from_be(self) -> Self;
    /// Converts a value from the native endianness to its little-endian representation.
    fn to_le(self) -> Self;
    /// Converts a value from the native endianness to its big-endian representation.
    fn to_be(self) -> Self;
}

macro_rules! impl_endianable {
    ($type:ident) => {
        impl Endianable for $type {
            fn from_le(self) -> Self {
                Self::from_le(self)
            }

            fn from_be(self) -> Self {
                Self::from_be(self)
            }

            fn to_le(self) -> Self {
                Self::to_le(self)
            }

            fn to_be(self) -> Self {
                Self::to_be(self)
            }
        }
    };
}

macro_rules! impl_endianable_float {
    ($ftype:ident,$ntype:ident) => {
        impl Endianable for $ftype {
            fn from_le(self) -> Self {
                Self::from_bits($ntype::from_le(self.to_bits()))
            }

            fn from_be(self) -> Self {
                Self::from_bits($ntype::from_be(self.to_bits()))
            }

            fn to_le(self) -> Self {
                Self::from_bits($ntype::to_le(self.to_bits()))
            }

            fn to_be(self) -> Self {
                Self::from_bits($ntype::to_be(self.to_bits()))
            }
        }
    };
}

impl_endianable!(u8);
impl_endianable!(u16);
impl_endianable!(u32);
impl_endianable!(u64);
impl_endianable!(u128);
impl_endianable!(usize);
impl_endianable!(i8);
impl_endianable!(i16);
impl_endianable!(i32);
impl_endianable!(i64);
impl_endianable!(i128);
impl_endianable!(isize);

impl_endianable_float!(f32, u32);
impl_endianable_float!(f64, u64);

/// A wrapper type that ensures the inner `Endianable` value is treated as Big-Endian.
///
/// When creating a `BigEndian` instance, the value is converted to big-endian.
/// When retrieving the inner value with `get`, it is converted back
/// to the native endianness.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BigEndian<T: Endianable>(pub(crate) T);

impl_byteable_generic!(BigEndian, u8);
impl_byteable_generic!(BigEndian, u16);
impl_byteable_generic!(BigEndian, u32);
impl_byteable_generic!(BigEndian, u64);
impl_byteable_generic!(BigEndian, u128);
impl_byteable_generic!(BigEndian, usize);
impl_byteable_generic!(BigEndian, i8);
impl_byteable_generic!(BigEndian, i16);
impl_byteable_generic!(BigEndian, i32);
impl_byteable_generic!(BigEndian, i64);
impl_byteable_generic!(BigEndian, i128);
impl_byteable_generic!(BigEndian, isize);
impl_byteable_generic!(BigEndian, f32);
impl_byteable_generic!(BigEndian, f64);

impl<T: Endianable> BigEndian<T> {
    /// Creates a new `BigEndian` instance from a value, converting it to big-endian.
    pub fn new(val: T) -> Self {
        Self(val.to_be())
    }

    /// Returns the inner value, converting it from big-endian to the native endianness.
    pub fn get(self) -> T {
        self.get_raw().from_be()
    }

    /// Returns the underlying native representation without any endian conversion.
    pub fn get_raw(self) -> T {
        self.0
    }
}

impl<T: Endianable + Default> Default for BigEndian<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

/// A wrapper type that ensures the inner `Endianable` value is treated as Little-Endian.
///
/// When creating a `LittleEndian` instance, the value is converted to little-endian.
/// When retrieving the inner value with `get`, it is converted back
/// to the native endianness.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LittleEndian<T: Endianable>(pub(crate) T);

impl_byteable_generic!(LittleEndian, u8);
impl_byteable_generic!(LittleEndian, u16);
impl_byteable_generic!(LittleEndian, u32);
impl_byteable_generic!(LittleEndian, u64);
impl_byteable_generic!(LittleEndian, u128);
impl_byteable_generic!(LittleEndian, usize);
impl_byteable_generic!(LittleEndian, i8);
impl_byteable_generic!(LittleEndian, i16);
impl_byteable_generic!(LittleEndian, i32);
impl_byteable_generic!(LittleEndian, i64);
impl_byteable_generic!(LittleEndian, i128);
impl_byteable_generic!(LittleEndian, isize);
impl_byteable_generic!(LittleEndian, f32);
impl_byteable_generic!(LittleEndian, f64);

impl<T: Endianable> LittleEndian<T> {
    /// Creates a new `LittleEndian` instance from a value, converting it to little-endian.
    pub fn new(val: T) -> Self {
        Self(val.to_le())
    }

    /// Returns the inner value, converting it from little-endian to the native endianness.
    pub fn get(self) -> T {
        self.get_raw().from_le()
    }

    /// Returns the underlying native representation without any endian conversion.
    pub fn get_raw(self) -> T {
        self.0
    }
}

impl<T: Endianable + Default> Default for LittleEndian<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[cfg(test)]
mod tests {

    mod byteable {
        #[cfg(feature = "derive")]
        mod derive {
            use crate::{BigEndian, Byteable, LittleEndian}; // Corrected use for Byteable
            #[derive(Byteable, Clone, Copy, PartialEq, Debug)] // Added PartialEq and Debug for assertions
            #[repr(C, packed)]
            struct ABC {
                a: LittleEndian<u16>,
                b: LittleEndian<u16>,
                c: BigEndian<u16>,
            }

            #[test]
            fn test_derive() {
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
        }
    }

    mod endian {
        use super::super::{BigEndian, LittleEndian};
        #[test]
        fn big_endian_test() {
            // Test with a known big-endian system or convert to bytes and check order
            let val = 0x01020304u32;
            let be_val = BigEndian::new(val);

            // get converts from BE to native, so if we create it from a native value,
            // and then turn it back, it should be the original value.
            assert_eq!(be_val.get(), val);
            assert_eq!(be_val.get_raw().to_ne_bytes(), [1, 2, 3, 4]);
            assert_eq!(u32::from_be_bytes(be_val.get_raw().to_ne_bytes()), val);
        }

        #[test]
        fn little_endian_test() {
            // Test with a known little-endian system or convert to bytes and check order
            let val = 0x01020304u32;
            let le_val = LittleEndian::new(val);

            // get converts from LE to native, so if we create it from a native value,
            // and then turn it back, it should be the original value.
            assert_eq!(le_val.get(), val);
            assert_eq!(le_val.get_raw().to_ne_bytes(), [4, 3, 2, 1]);
            assert_eq!(u32::from_le_bytes(le_val.get_raw().to_ne_bytes()), val);
        }
    }

    mod derive {
        use crate::{Byteable, ByteableRegular};

        #[derive(Clone, Copy)]
        struct Basic {
            a: u16,
            b: u32,
            c: u16,
        }

        #[derive(Clone, Copy)]
        #[repr(C, packed)]
        struct BasicRaw {
            a: u16,
            b: u32,
            c: u16,
        }
        impl_byteable!(BasicRaw);

        impl From<Basic> for BasicRaw {
            fn from(value: Basic) -> Self {
                BasicRaw {
                    a: value.a,
                    b: value.b,
                    c: value.c,
                }
            }
        }
        impl From<BasicRaw> for Basic {
            fn from(value: BasicRaw) -> Self {
                Basic {
                    a: value.a,
                    b: value.b,
                    c: value.c,
                }
            }
        }

        impl ByteableRegular for Basic {
            type Raw = BasicRaw;

            fn to_raw(self) -> Self::Raw {
                BasicRaw {
                    a: self.a,
                    b: self.b,
                    c: self.c,
                }
            }

            fn from_raw(raw: Self::Raw) -> Self {
                Self {
                    a: raw.a,
                    b: raw.b,
                    c: raw.c,
                }
            }
        }
    }
}
