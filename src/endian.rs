//! Endianness handling for byte-oriented serialization.
//!
//! This module provides utilities for handling endianness (byte order) when working with
//! binary data. It includes the `EndianConvert` trait for types that support endianness
//! conversion, and the `BigEndian<T>` and `LittleEndian<T>` wrapper types that ensure
//! values are stored in a specific byte order regardless of the system's native endianness.

use crate::{ByteArray, Byteable};
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
pub trait EndianConvert: Copy {
    /// The byte array type used to represent this type.
    type ByteArray: ByteArray;

    /// Creates a value from its little-endian byte representation.
    fn from_le_bytes(byte_array: Self::ByteArray) -> Self;

    /// Creates a value from its big-endian byte representation.
    fn from_be_bytes(byte_array: Self::ByteArray) -> Self;

    /// Creates a value from its native-endian byte representation.
    fn from_ne_bytes(byte_array: Self::ByteArray) -> Self;

    /// Returns the little-endian byte representation of this value.
    fn to_le_bytes(self) -> Self::ByteArray;

    /// Returns the big-endian byte representation of this value.
    fn to_be_bytes(self) -> Self::ByteArray;

    /// Returns the native-endian byte representation of this value.
    fn to_ne_bytes(self) -> Self::ByteArray;
}

macro_rules! impl_endianable {
    ($($type:ty),+) => {
        $(
            impl $crate::EndianConvert for $type {
                type ByteArray = [u8; ::core::mem::size_of::<$type>()];

                fn from_le_bytes(byte_array: Self::ByteArray) -> Self {
                    <$type>::from_le_bytes(byte_array)
                }

                fn from_be_bytes(byte_array: Self::ByteArray) -> Self {
                    <$type>::from_be_bytes(byte_array)
                }

                fn from_ne_bytes(byte_array: Self::ByteArray) -> Self {
                    <$type>::from_ne_bytes(byte_array)
                }

                fn to_ne_bytes(self) -> Self::ByteArray {
                    <$type>::to_ne_bytes(self)
                }

                fn to_le_bytes(self) -> Self::ByteArray {
                    <$type>::to_le_bytes(self)
                }

                fn to_be_bytes(self) -> Self::ByteArray {
                    <$type>::to_be_bytes(self)
                }
            }
        )+
    };
}

impl_endianable!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

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
/// use byteable::BigEndian;
///
/// let value = BigEndian::new(0x12345678u32);
///
/// // The bytes are always stored in big-endian order
/// assert_eq!(value.raw_bytes(), [0x12, 0x34, 0x56, 0x78]);
///
/// // Get back the native value
/// assert_eq!(value.get(), 0x12345678u32);
/// ```
///
/// ## In a struct for network protocols
///
/// ```
/// # #[cfg(feature = "derive")]
/// use byteable::{BigEndian};
///
/// # #[cfg(feature = "derive")]
/// #[derive(byteable::UnsafeByteableTransmute, Debug)]
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
pub struct BigEndian<T: EndianConvert>(pub(crate) T::ByteArray);

impl<T: fmt::Debug + EndianConvert> fmt::Debug for BigEndian<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BigEndian").field(&self.get()).finish()
    }
}

impl<T: PartialEq + EndianConvert> PartialEq for BigEndian<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl<T: Eq + EndianConvert> Eq for BigEndian<T> {}

impl<T: PartialOrd + EndianConvert> PartialOrd for BigEndian<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

impl<T: Ord + EndianConvert> Ord for BigEndian<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T: Hash + EndianConvert> Hash for BigEndian<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state);
    }
}

impl<T: EndianConvert> BigEndian<T> {
    /// Creates a new `BigEndian` value from a native value.
    ///
    /// The value is converted to big-endian byte order upon construction.
    ///
    /// # Examples
    ///
    /// ```
    /// use byteable::BigEndian;
    ///
    /// let be = BigEndian::new(0x1234u16);
    /// assert_eq!(be.raw_bytes(), [0x12, 0x34]);
    /// ```
    pub fn new(value: T) -> Self {
        // Convert to big-endian bytes and store internally
        Self(value.to_be_bytes())
    }

    /// Extracts the native value from this `BigEndian` wrapper.
    ///
    /// The bytes are converted from big-endian to the system's native byte order.
    ///
    /// # Examples
    ///
    /// ```
    /// use byteable::BigEndian;
    ///
    /// let be = BigEndian::new(42u32);
    /// assert_eq!(be.get(), 42);
    /// ```
    pub fn get(self) -> T {
        // Convert from big-endian bytes back to native value
        T::from_be_bytes(self.0)
    }

    /// Returns the raw bytes in big-endian order.
    ///
    /// This returns the actual byte representation without any conversion.
    ///
    /// # Examples
    ///
    /// ```
    /// use byteable::BigEndian;
    ///
    /// let be = BigEndian::new(0x12345678u32);
    /// assert_eq!(be.raw_bytes(), [0x12, 0x34, 0x56, 0x78]);
    /// ```
    pub fn raw_bytes(self) -> T::ByteArray {
        // Return the stored bytes directly
        self.0
    }
}

impl<T: EndianConvert + Default> Default for BigEndian<T> {
    fn default() -> Self {
        // Create a BigEndian wrapper with the default value of T
        Self::new(T::default())
    }
}

impl<T: EndianConvert> Byteable for BigEndian<T> {
    type ByteArray = <T as EndianConvert>::ByteArray;

    fn to_byte_array(self) -> Self::ByteArray {
        // Return the stored big-endian bytes directly (no conversion needed)
        self.0
    }

    fn from_byte_array(byte_array: Self::ByteArray) -> Self {
        // Wrap the bytes directly (they're already in big-endian format)
        Self(byte_array)
    }
}

impl<T: EndianConvert> From<T> for BigEndian<T> {
    fn from(value: T) -> Self {
        // Convenient conversion from native value to BigEndian
        BigEndian::new(value)
    }
}

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
/// use byteable::LittleEndian;
///
/// let value = LittleEndian::new(0x12345678u32);
///
/// // The bytes are always stored in little-endian order
/// assert_eq!(value.raw_bytes(), [0x78, 0x56, 0x34, 0x12]);
///
/// // Get back the native value
/// assert_eq!(value.get(), 0x12345678u32);
/// ```
///
/// ## In a struct for file formats
///
/// ```
/// # #[cfg(feature = "derive")]
/// use byteable::{LittleEndian};
///
/// # #[cfg(feature = "derive")]
/// #[derive(byteable::UnsafeByteableTransmute, Debug)]
/// #[repr(C, packed)]
/// struct BmpHeader {
///     signature: [u8; 2],               // "BM"
///     file_size: LittleEndian<u32>,     // Little-endian
///     reserved: [u8; 4],                // Reserved bytes
///     data_offset: LittleEndian<u32>,   // Little-endian
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn example() {
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
pub struct LittleEndian<T: EndianConvert>(T::ByteArray);

impl<T: fmt::Debug + EndianConvert> fmt::Debug for LittleEndian<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("LittleEndian").field(&self.get()).finish()
    }
}

impl<T: PartialEq + EndianConvert> PartialEq for LittleEndian<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl<T: Eq + EndianConvert> Eq for LittleEndian<T> {}

impl<T: PartialOrd + EndianConvert> PartialOrd for LittleEndian<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

impl<T: Ord + EndianConvert> Ord for LittleEndian<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T: Hash + EndianConvert> Hash for LittleEndian<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state);
    }
}

impl<T: EndianConvert> LittleEndian<T> {
    /// Creates a new `LittleEndian` value from a native value.
    ///
    /// The value is converted to little-endian byte order upon construction.
    ///
    /// # Examples
    ///
    /// ```
    /// use byteable::LittleEndian;
    ///
    /// let le = LittleEndian::new(0x1234u16);
    /// assert_eq!(le.raw_bytes(), [0x34, 0x12]);
    /// ```
    pub fn new(value: T) -> Self {
        // Convert to little-endian bytes and store internally
        Self(value.to_le_bytes())
    }

    /// Extracts the native value from this `LittleEndian` wrapper.
    ///
    /// The bytes are converted from little-endian to the system's native byte order.
    ///
    /// # Examples
    ///
    /// ```
    /// use byteable::LittleEndian;
    ///
    /// let le = LittleEndian::new(42u32);
    /// assert_eq!(le.get(), 42);
    /// ```
    pub fn get(self) -> T {
        // Convert from little-endian bytes back to native value
        T::from_le_bytes(self.0)
    }

    /// Returns the raw bytes in little-endian order.
    ///
    /// This returns the actual byte representation without any conversion.
    ///
    /// # Examples
    ///
    /// ```
    /// use byteable::LittleEndian;
    ///
    /// let le = LittleEndian::new(0x12345678u32);
    /// assert_eq!(le.raw_bytes(), [0x78, 0x56, 0x34, 0x12]);
    /// ```
    pub fn raw_bytes(self) -> T::ByteArray {
        // Return the stored bytes directly
        self.0
    }
}

impl<T: EndianConvert + Default> Default for LittleEndian<T> {
    fn default() -> Self {
        // Create a LittleEndian wrapper with the default value of T
        Self::new(T::default())
    }
}

impl<T: EndianConvert> Byteable for LittleEndian<T> {
    type ByteArray = <T as EndianConvert>::ByteArray;

    fn to_byte_array(self) -> Self::ByteArray {
        // Return the stored little-endian bytes directly (no conversion needed)
        self.0
    }

    fn from_byte_array(byte_array: Self::ByteArray) -> Self {
        // Wrap the bytes directly (they're already in little-endian format)
        Self(byte_array)
    }
}

impl<T: EndianConvert> From<T> for LittleEndian<T> {
    fn from(value: T) -> Self {
        // Convenient conversion from native value to LittleEndian
        LittleEndian::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{BigEndian, LittleEndian};

    #[test]
    fn big_endian_test() {
        let val = 0x01020304u32;
        let be_val = BigEndian::new(val);

        assert_eq!(be_val.get(), val);
        assert_eq!(be_val.raw_bytes(), [1, 2, 3, 4]);
        assert_eq!(u32::from_be_bytes(be_val.raw_bytes()), val);
    }

    #[test]
    fn little_endian_test() {
        let val = 0x01020304u32;
        let le_val = LittleEndian::new(val);

        assert_eq!(le_val.get(), val);
        assert_eq!(le_val.raw_bytes(), [4, 3, 2, 1]);
        assert_eq!(u32::from_le_bytes(le_val.raw_bytes()), val);
    }
}
