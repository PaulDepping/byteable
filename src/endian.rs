//! Endianness handling types and traits.
//!
//! This module provides the `Endianable` trait for types that support endianness conversion,
//! along with `BigEndian` and `LittleEndian` wrapper types for explicit endianness control.

use crate::Byteable;
use std::{fmt, hash::Hash};

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
    ($type:ty) => {
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

/// Macro to implement the `Endianable` trait for floating-point types.
///
/// This macro generates an `Endianable` implementation for floating-point types
/// by converting them to their integer bit representation, applying endianness
/// conversion, and then converting back to the float type.
///
/// # Parameters
///
/// * `$ftype` - The floating-point type (e.g., `f32` or `f64`)
/// * `$ntype` - The corresponding integer type for bit representation (e.g., `u32` for `f32`, `u64` for `f64`)
///
/// # Example
///
/// ```ignore
/// impl_endianable_float!(f32, u32);
/// // Expands to: impl Endianable for f32 { ... }
/// ```
macro_rules! impl_endianable_float {
    ($ftype:ty,$ntype:ty) => {
        impl Endianable for $ftype {
            fn from_le(self) -> Self {
                Self::from_bits(<$ntype>::from_le(self.to_bits()))
            }

            fn from_be(self) -> Self {
                Self::from_bits(<$ntype>::from_be(self.to_bits()))
            }

            fn to_le(self) -> Self {
                Self::from_bits(<$ntype>::to_le(self.to_bits()))
            }

            fn to_be(self) -> Self {
                Self::from_bits(<$ntype>::to_be(self.to_bits()))
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
#[derive(Clone, Copy)]
pub struct BigEndian<T: Endianable>(pub(crate) T);

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

crate::impl_byteable!(BigEndian<u8>);
crate::impl_byteable!(BigEndian<u16>);
crate::impl_byteable!(BigEndian<u32>);
crate::impl_byteable!(BigEndian<u64>);
crate::impl_byteable!(BigEndian<u128>);
crate::impl_byteable!(BigEndian<usize>);
crate::impl_byteable!(BigEndian<i8>);
crate::impl_byteable!(BigEndian<i16>);
crate::impl_byteable!(BigEndian<i32>);
crate::impl_byteable!(BigEndian<i64>);
crate::impl_byteable!(BigEndian<i128>);
crate::impl_byteable!(BigEndian<isize>);
crate::impl_byteable!(BigEndian<f32>);
crate::impl_byteable!(BigEndian<f64>);

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
#[derive(Clone, Copy)]
pub struct LittleEndian<T: Endianable>(pub(crate) T);

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

crate::impl_byteable!(LittleEndian<u8>);
crate::impl_byteable!(LittleEndian<u16>);
crate::impl_byteable!(LittleEndian<u32>);
crate::impl_byteable!(LittleEndian<u64>);
crate::impl_byteable!(LittleEndian<u128>);
crate::impl_byteable!(LittleEndian<usize>);
crate::impl_byteable!(LittleEndian<i8>);
crate::impl_byteable!(LittleEndian<i16>);
crate::impl_byteable!(LittleEndian<i32>);
crate::impl_byteable!(LittleEndian<i64>);
crate::impl_byteable!(LittleEndian<i128>);
crate::impl_byteable!(LittleEndian<isize>);
crate::impl_byteable!(LittleEndian<f32>);
crate::impl_byteable!(LittleEndian<f64>);

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
    use super::{BigEndian, LittleEndian};

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
