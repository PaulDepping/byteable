//! Endianness handling types and traits.
//!
//! This module provides the `Endianable` trait for types that support endianness conversion,
//! along with `BigEndian` and `LittleEndian` wrapper types for explicit endianness control.

use crate::{Byteable, ByteableByteArray};
use std::{fmt, hash::Hash};

/// Trait for types that support endianness conversion.
///
/// This trait provides methods to convert values to and from little-endian (LE)
/// and big-endian (BE) byte orders. It is implemented for most primitive integer
/// and floating-point types.
pub trait Endianable: Copy {
    // always a bytearray of [u8; std::mem::size_of::<Self>()]
    type ByteArray: ByteableByteArray;
    /// Converts a value from its little-endian representation to the native endianness.
    fn from_le(ba: Self::ByteArray) -> Self;
    /// Converts a value from its big-endian representation to the native endianness.
    fn from_be(ba: Self::ByteArray) -> Self;
    /// Converts a value from its native-endian representation to the native endianness.
    fn from_ne(ba: Self::ByteArray) -> Self;

    /// Converts a value from the native endianness to its little-endian representation.
    fn to_le(self) -> Self::ByteArray;
    /// Converts a value from the native endianness to its big-endian representation.
    fn to_be(self) -> Self::ByteArray;
    /// Converts a value from the native endianness to its binary representation.
    fn to_ne(self) -> Self::ByteArray;
}

/// Macro to implement the `Endianable` trait for integer types.
///
/// This macro generates an `Endianable` implementation for primitive integer types
/// by delegating to their built-in endianness conversion methods.
///
/// # Example
///
/// ```ignore
/// impl_endianable!(u32);
/// // Expands to: impl Endianable for u32 { ... }
/// ```
macro_rules! impl_endianable {
    ($($type:ty),+) => {
        $(
            impl $crate::Endianable for $type {
                type ByteArray = [u8; ::std::mem::size_of::<$type>()];

                fn from_le(ba: Self::ByteArray) -> Self {
                    Self::from_le_bytes(ba)
                }

                fn from_be(ba: Self::ByteArray) -> Self {
                    Self::from_be_bytes(ba)
                }

                fn from_ne(ba: Self::ByteArray) -> Self {
                    Self::from_ne_bytes(ba)
                }

                fn to_ne(self) -> Self::ByteArray {
                    Self::to_ne_bytes(self)
                }

                fn to_le(self) -> Self::ByteArray {
                    Self::to_le_bytes(self)
                }

                fn to_be(self) -> Self::ByteArray {
                    Self::to_be_bytes(self)
                }
            }
        )+
    };
}

impl_endianable!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

/// A wrapper type that ensures the inner `Endianable` value is treated as Big-Endian.
///
/// When creating a `BigEndian` instance, the value is converted to big-endian.
/// When retrieving the inner value with `get`, it is converted back
/// to the native endianness.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct BigEndian<T: Endianable>(pub(crate) T::ByteArray);

impl<T: fmt::Debug + Endianable> fmt::Debug for BigEndian<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BigEndian").field(&self.get()).finish()
    }
}

impl<T: PartialEq + Endianable> PartialEq for BigEndian<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl<T: Eq + Endianable> Eq for BigEndian<T> {}

impl<T: PartialOrd + Endianable> PartialOrd for BigEndian<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

impl<T: Ord + Endianable> Ord for BigEndian<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T: Hash + Endianable> Hash for BigEndian<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state);
    }
}

impl<T: Endianable> BigEndian<T> {
    /// Creates a new `BigEndian` instance from a value, converting it to big-endian.
    pub fn new(val: T) -> Self {
        Self(val.to_be())
    }

    /// Returns the inner value, converting it from big-endian to the native endianness.
    pub fn get(self) -> T {
        T::from_be(self.0)
    }

    pub fn get_raw(self) -> T::ByteArray {
        self.0
    }
}

impl<T: Endianable + Default> Default for BigEndian<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Endianable> Byteable for BigEndian<T> {
    type ByteArray = <T as Endianable>::ByteArray;

    fn as_bytearray(self) -> Self::ByteArray {
        self.0
    }

    fn from_bytearray(ba: Self::ByteArray) -> Self {
        Self(ba)
    }
}

impl<T: Endianable> From<T> for BigEndian<T> {
    fn from(value: T) -> Self {
        BigEndian::new(value)
    }
}

/// A wrapper type that ensures the inner `Endianable` value is treated as Little-Endian.
///
/// When creating a `LittleEndian` instance, the value is converted to little-endian.
/// When retrieving the inner value with `get`, it is converted back
/// to the native endianness.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct LittleEndian<T: Endianable>(T::ByteArray);

impl<T: fmt::Debug + Endianable> fmt::Debug for LittleEndian<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("LittleEndian").field(&self.get()).finish()
    }
}

impl<T: PartialEq + Endianable> PartialEq for LittleEndian<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl<T: Eq + Endianable> Eq for LittleEndian<T> {}

impl<T: PartialOrd + Endianable> PartialOrd for LittleEndian<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

impl<T: Ord + Endianable> Ord for LittleEndian<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T: Hash + Endianable> Hash for LittleEndian<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state);
    }
}

impl<T: Endianable> LittleEndian<T> {
    /// Creates a new `LittleEndian` instance from a value, converting it to little-endian.
    pub fn new(val: T) -> Self {
        Self(val.to_le())
    }

    /// Returns the inner value, converting it from little-endian to the native endianness.
    pub fn get(self) -> T {
        T::from_le(self.0)
    }

    pub fn get_raw(self) -> T::ByteArray {
        self.0
    }
}

impl<T: Endianable + Default> Default for LittleEndian<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Endianable> Byteable for LittleEndian<T> {
    type ByteArray = <T as Endianable>::ByteArray;

    fn as_bytearray(self) -> Self::ByteArray {
        self.0
    }

    fn from_bytearray(ba: Self::ByteArray) -> Self {
        Self(ba)
    }
}

impl<T: Endianable> From<T> for LittleEndian<T> {
    fn from(value: T) -> Self {
        LittleEndian::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{BigEndian, LittleEndian};

    #[test]
    fn big_endian_test() {
        // Test with a known big-endian system or convert to bytes and check order
        let val = 0x01020304u32;
        let be_val = BigEndian::new(val);

        // get converts from BE to native, so if we create it from a native value,
        // and then turn it back, it should be the original value.
        assert_eq!(be_val.get(), val);
        assert_eq!(be_val.get_raw(), [1, 2, 3, 4]);
        assert_eq!(u32::from_be_bytes(be_val.get_raw()), val);
    }

    #[test]
    fn little_endian_test() {
        // Test with a known little-endian system or convert to bytes and check order
        let val = 0x01020304u32;
        let le_val = LittleEndian::new(val);

        // get converts from LE to native, so if we create it from a native value,
        // and then turn it back, it should be the original value.
        assert_eq!(le_val.get(), val);
        assert_eq!(le_val.get_raw(), [4, 3, 2, 1]);
        assert_eq!(u32::from_le_bytes(le_val.get_raw()), val);
    }
}
