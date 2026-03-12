//! Endianness handling for byte-oriented serialization.
//!
//! This module provides utilities for handling endianness (byte order) when working with
//! binary data. It includes the `EndianConvert` trait for types that support endianness
//! conversion, and the `BigEndian<T>` and `LittleEndian<T>` wrapper types that ensure
//! values are stored in a specific byte order regardless of the system's native endianness.

use crate::{ByteRepr, FromByteArray, IntoByteArray};
use core::{fmt, hash::Hash};

/// A trait for types that can be converted between different byte orders (endianness).
///
/// This trait provides methods for converting between little-endian, big-endian, and
/// native-endian byte representations. It is implemented for all primitive numeric types.
///
/// # Implementations
///
/// This trait is automatically implemented for:
/// - Unsigned integers: `u8`, `u16`, `u32`, `u64`, `u128`
/// - Signed integers: `i8`, `i16`, `i32`, `i64`, `i128`
/// - Floating point: `f32`, `f64`
///
/// # Examples
///
/// ```
/// use byteable::EndianConvert;
///
/// let value = 0x12345678u32;
///
/// let le_bytes = value.to_le_bytes();
/// assert_eq!(le_bytes, [0x78, 0x56, 0x34, 0x12]);
///
/// let be_bytes = value.to_be_bytes();
/// assert_eq!(be_bytes, [0x12, 0x34, 0x56, 0x78]);
///
/// // Convert back
/// assert_eq!(u32::from_le_bytes(le_bytes), value);
/// assert_eq!(u32::from_be_bytes(be_bytes), value);
/// ```
pub trait EndianConvert: Copy + ByteRepr + IntoByteArray + FromByteArray {
    /// Creates a value from its little-endian representation.
    fn from_le(value: Self) -> Self;

    /// Creates a value from its big-endian representation.
    fn from_be(value: Self) -> Self;

    /// Converts to little-endian representation.
    fn to_le(self) -> Self;

    /// Converts to big-endian representation.
    fn to_be(self) -> Self;
}

macro_rules! impl_endian_convert {
    ($($type:ty),+) => {
        $(
            impl $crate::EndianConvert for $type {
                #[inline]
                fn from_le(value: Self) -> Self {
                    <$type>::from_le(value)
                }

                #[inline]
                fn from_be(value: Self) -> Self {
                    <$type>::from_be(value)
                }

                #[inline]
                fn to_le(self) -> Self {
                    <$type>::to_le(self)
                }

                #[inline]
                fn to_be(self) -> Self {
                    <$type>::to_be(self)
                }
            }
        )+
    };
}

impl_endian_convert!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

// Float implementations use to_bits/from_bits for endianness conversion
impl EndianConvert for f32 {
    #[inline]
    fn from_le(value: Self) -> Self {
        Self::from_bits(u32::from_le(value.to_bits()))
    }

    #[inline]
    fn from_be(value: Self) -> Self {
        Self::from_bits(u32::from_be(value.to_bits()))
    }

    #[inline]
    fn to_le(self) -> Self {
        Self::from_bits(self.to_bits().to_le())
    }

    #[inline]
    fn to_be(self) -> Self {
        Self::from_bits(self.to_bits().to_be())
    }
}

impl EndianConvert for f64 {
    #[inline]
    fn from_le(value: Self) -> Self {
        Self::from_bits(u64::from_le(value.to_bits()))
    }

    #[inline]
    fn from_be(value: Self) -> Self {
        Self::from_bits(u64::from_be(value.to_bits()))
    }

    #[inline]
    fn to_le(self) -> Self {
        Self::from_bits(self.to_bits().to_le())
    }

    #[inline]
    fn to_be(self) -> Self {
        Self::from_bits(self.to_bits().to_be())
    }
}

/// A wrapper type that stores a value in big-endian (network) byte order.
///
/// This type ensures that the wrapped value is always stored in big-endian format,
/// regardless of the system's native endianness. This is particularly useful for:
/// - Network protocols (which typically use big-endian/"network byte order")
/// - File formats that specify big-endian storage
/// - Cross-platform binary data interchange
///
/// The wrapper is `#[repr(transparent)]`, meaning it has the same memory layout as
/// its inner byte array, making it safe to use in packed structs.
///
/// # Type Parameters
///
/// * `T` - The underlying numeric type that implements `EndianConvert`
///
/// # Examples
///
/// ## Basic usage
///
/// ```
/// use byteable::{BigEndian, IntoByteArray};
///
/// let value = BigEndian::new(0x12345678u32);
///
/// // The bytes are always stored in big-endian order
/// assert_eq!(value.into_byte_array(), [0x12, 0x34, 0x56, 0x78]);
///
/// // Get back the native value
/// assert_eq!(value.get(), 0x12345678u32);
/// ```
///
/// ## In a struct for network protocols
///
/// ```
/// # #[cfg(feature = "derive")]
/// use byteable::{BigEndian, IntoByteArray};
///
/// # #[cfg(feature = "derive")]
/// #[derive(byteable::UnsafeByteableTransmute, Debug, Clone, Copy)]
/// #[repr(C, packed)]
/// struct TcpHeader {
///     source_port: BigEndian<u16>,      // Network byte order
///     dest_port: BigEndian<u16>,        // Network byte order
///     sequence: BigEndian<u32>,         // Network byte order
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn example() {
/// let header = TcpHeader {
///     source_port: 80.into(),
///     dest_port: 8080.into(),
///     sequence: 12345.into(),
/// };
/// # }
/// ```
///
/// ## Comparison and hashing
///
/// ```
/// use byteable::BigEndian;
/// use std::collections::HashMap;
///
/// let a = BigEndian::new(100u32);
/// let b = BigEndian::new(100u32);
/// let c = BigEndian::new(200u32);
///
/// assert_eq!(a, b);
/// assert!(a < c);
///
/// // Can be used as HashMap keys
/// let mut map = HashMap::new();
/// map.insert(a, "one hundred");
/// assert_eq!(map.get(&b), Some(&"one hundred"));
/// ```
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct BigEndian<T: EndianConvert>(pub(crate) T);

/// A wrapper type that stores a value in little-endian byte order.
///
/// This type ensures that the wrapped value is always stored in little-endian format,
/// regardless of the system's native endianness. This is particularly useful for:
/// - File formats that specify little-endian storage (e.g., BMP, WAV, PE executables)
/// - USB and Bluetooth protocols
/// - x86/x64 architecture data structures
/// - Cross-platform binary data interchange
///
/// The wrapper is `#[repr(transparent)]`, meaning it has the same memory layout as
/// its inner byte array, making it safe to use in packed structs.
///
/// # Type Parameters
///
/// * `T` - The underlying numeric type that implements `EndianConvert`
///
/// # Examples
///
/// ## Basic usage
///
/// ```
/// use byteable::{LittleEndian, IntoByteArray};
///
/// let value = LittleEndian::new(0x12345678u32);
///
/// // The bytes are always stored in little-endian order
/// assert_eq!(value.into_byte_array(), [0x78, 0x56, 0x34, 0x12]);
///
/// // Get back the native value
/// assert_eq!(value.get(), 0x12345678u32);
/// ```
///
/// ## In a struct for file formats
///
/// ```
/// # #[cfg(feature = "derive")] {
/// use byteable::{LittleEndian};
///
/// #[derive(byteable::UnsafeByteableTransmute, Debug, Clone, Copy)]
/// #[repr(C, packed)]
/// struct BmpHeader {
///     signature: [u8; 2],               // "BM"
///     file_size: LittleEndian<u32>,     // Little-endian
///     reserved: [u8; 4],                // Reserved bytes
///     data_offset: LittleEndian<u32>,   // Little-endian
/// }
///
/// let header = BmpHeader {
///     signature: *b"BM",
///     file_size: 1024.into(),
///     reserved: [0; 4],
///     data_offset: 54.into(),
/// };
/// # }
/// ```
///
/// ## Convenient conversions
///
/// ```
/// use byteable::LittleEndian;
///
/// // Using From trait
/// let le: LittleEndian<u16> = 1000.into();
/// assert_eq!(le.get(), 1000);
///
/// // Direct construction
/// let le = LittleEndian::new(2000u16);
/// assert_eq!(le.get(), 2000);
/// ```
///
/// ## Comparison and hashing
///
/// ```
/// use byteable::LittleEndian;
/// use std::collections::HashSet;
///
/// let a = LittleEndian::new(100u32);
/// let b = LittleEndian::new(100u32);
/// let c = LittleEndian::new(200u32);
///
/// assert_eq!(a, b);
/// assert!(a < c);
///
/// // Can be used in HashSet
/// let mut set = HashSet::new();
/// set.insert(a);
/// assert!(set.contains(&b));
/// ```
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct LittleEndian<T: EndianConvert>(T);

// Shared impls for both endian wrappers — only the conversion direction differs.
macro_rules! impl_endian_wrapper {
    ($name:ident, $to_fn:ident, $from_fn:ident) => {
        impl<T: EndianConvert> $name<T> {
            /// Creates a new wrapper from a native-endian value, converting to the target byte order.
            ///
            /// # Examples
            ///
            /// ```
            /// use byteable::{BigEndian, LittleEndian, IntoByteArray};
            ///
            /// let be = BigEndian::new(0x1234u16);
            /// assert_eq!(be.into_byte_array(), [0x12, 0x34]);
            ///
            /// let le = LittleEndian::new(0x1234u16);
            /// assert_eq!(le.into_byte_array(), [0x34, 0x12]);
            /// ```
            #[inline]
            pub fn new(value: T) -> Self {
                Self(value.$to_fn())
            }

            /// Extracts the native-endian value from this wrapper.
            ///
            /// # Examples
            ///
            /// ```
            /// use byteable::{BigEndian, LittleEndian};
            ///
            /// assert_eq!(BigEndian::new(42u32).get(), 42);
            /// assert_eq!(LittleEndian::new(42u32).get(), 42);
            /// ```
            #[inline]
            pub fn get(self) -> T {
                T::$from_fn(self.0)
            }
        }

        impl<T: fmt::Debug + EndianConvert> fmt::Debug for $name<T> {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.get()).finish()
            }
        }

        impl<T: PartialEq + EndianConvert> PartialEq for $name<T> {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.get() == other.get()
            }
        }

        impl<T: Eq + EndianConvert> Eq for $name<T> {}

        impl<T: PartialOrd + EndianConvert> PartialOrd for $name<T> {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                self.get().partial_cmp(&other.get())
            }
        }

        impl<T: Ord + EndianConvert> Ord for $name<T> {
            #[inline]
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                self.get().cmp(&other.get())
            }
        }

        impl<T: Hash + EndianConvert> Hash for $name<T> {
            #[inline]
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                self.get().hash(state);
            }
        }

        impl<T: EndianConvert + Default> Default for $name<T> {
            #[inline]
            fn default() -> Self {
                Self::new(T::default())
            }
        }

        impl<T: EndianConvert> From<T> for $name<T> {
            #[inline]
            fn from(value: T) -> Self {
                $name::new(value)
            }
        }

        impl<T: EndianConvert> ByteRepr for $name<T> {
            type ByteArray = <T as ByteRepr>::ByteArray;
        }

        impl<T: EndianConvert> IntoByteArray for $name<T> {
            #[inline]
            fn into_byte_array(self) -> Self::ByteArray {
                self.0.into_byte_array()
            }
        }

        impl<T: EndianConvert> FromByteArray for $name<T> {
            #[inline]
            fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                Self(T::from_byte_array(byte_array))
            }
        }
    };
}

impl_endian_wrapper!(BigEndian, to_be, from_be);
impl_endian_wrapper!(LittleEndian, to_le, from_le);

#[cfg(test)]
mod tests {
    use crate::IntoByteArray;

    use super::{BigEndian, LittleEndian};

    #[test]
    fn big_endian_test() {
        let val = 0x01020304u32;
        let be_val = BigEndian::new(val);

        assert_eq!(be_val.get(), val);
        assert_eq!(be_val.into_byte_array(), [1, 2, 3, 4]);
        assert_eq!(u32::from_be_bytes(be_val.into_byte_array()), val);
    }

    #[test]
    fn little_endian_test() {
        let val = 0x01020304u32;
        let le_val = LittleEndian::new(val);

        assert_eq!(le_val.get(), val);
        assert_eq!(le_val.into_byte_array(), [4, 3, 2, 1]);
        assert_eq!(u32::from_le_bytes(le_val.into_byte_array()), val);
    }
}
