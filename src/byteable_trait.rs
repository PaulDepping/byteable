//! Core traits for byte-oriented serialization and deserialization.
//!
//! This module contains the fundamental traits for converting types to and from byte arrays:
//! - [`AssociatedByteArray`]: Associates a type with its byte array representation
//! - [`IntoByteArray`]: Converts a value into a byte array
//! - [`FromByteArray`]: Constructs a value from a byte array
//! - [`TryIntoByteArray`]: Fallible conversion to a byte array
//! - [`TryFromByteArray`]: Fallible construction from a byte array
//! - [`HasRawType`]: For types with a distinct raw representation
//!
//! These traits provide the foundation for zero-overhead, zero-copy serialization throughout
//! the crate, along with helper macros for implementing them.

use crate::{LittleEndian, ValidBytecastMarker, byte_array::ByteArray};

/// Associates a type with its byte array representation.
///
/// This trait defines the relationship between a Rust type and its corresponding byte array type.
/// It serves as the foundation for byte-oriented serialization, providing the necessary type
/// information for conversions.
///
/// # Associated Types
///
/// - `ByteArray`: The type of the byte array representation. Usually `[u8; N]` where `N`
///   is the size of the type in bytes. This must implement the [`ByteArray`] trait.
///
/// # Associated Constants
///
/// - `BYTE_SIZE`: The size of the type in bytes. This is automatically derived from
///   `ByteArray::BYTE_SIZE`.
///
/// # Usage
///
/// This trait is typically not implemented directly. Instead, implement the higher-level traits
/// [`IntoByteArray`] and [`FromByteArray`], which require `AssociatedByteArray` as a supertrait,
/// or use the `#[derive(Byteable)]` macro which implements all necessary traits automatically.
///
/// # Examples
///
/// ```
/// use byteable::{AssociatedByteArray, IntoByteArray, FromByteArray};
///
/// // Primitive types implement AssociatedByteArray
/// assert_eq!(u32::BYTE_SIZE, 4);
/// assert_eq!(u64::BYTE_SIZE, 8);
///
/// // Arrays also implement it
/// assert_eq!(<[u16; 3]>::BYTE_SIZE, 6);
/// ```
///
/// ## With custom types using derive
///
/// ```
/// # #![cfg(feature = "derive")]
/// use byteable::{Byteable, AssociatedByteArray};
///
/// #[derive(Byteable, Clone, Copy)]
/// struct Point {
///     x: u8,
///     y: u8,
/// }
///
/// # fn main() {
/// // AssociatedByteArray is automatically implemented
/// assert_eq!(Point::BYTE_SIZE, 2);
/// # }
/// ```
pub trait AssociatedByteArray {
    type ByteArray: ByteArray;
    const BYTE_SIZE: usize = Self::ByteArray::BYTE_SIZE;
}

/// Converts a value into its byte array representation.
///
/// This trait provides the ability to transform a Rust value into a fixed-size byte array,
/// enabling zero-overhead serialization. This is particularly useful for:
/// - Binary file I/O
/// - Network protocols
/// - Low-level system programming
/// - Memory-mapped files
/// - Interfacing with C libraries
///
/// # Implementations
///
/// This trait is implemented for:
/// - All primitive numeric types (`u8`, `i32`, `f64`, etc.)
/// - `bool` and `char` (with validation on deserialization)
/// - Fixed-size arrays of types implementing `IntoByteArray`
/// - `BigEndian<T>` and `LittleEndian<T>` wrappers
/// - Custom types via `#[derive(Byteable)]`
///
/// # Examples
///
/// ## With primitive types
///
/// ```
/// use byteable::IntoByteArray;
///
/// let value: u32 = 0x12345678;
/// let bytes = value.into_byte_array();
///
/// // On little-endian systems
/// #[cfg(target_endian = "little")]
/// assert_eq!(bytes, [0x78, 0x56, 0x34, 0x12]);
/// ```
///
/// ## With custom types
///
/// ```
/// # #![cfg(feature = "derive")]
/// use byteable::{Byteable, IntoByteArray};
///
/// #[derive(Byteable, Clone, Copy)]
/// struct Color {
///     r: u8,
///     g: u8,
///     b: u8,
///     a: u8,
/// }
///
/// # fn main() {
/// let color = Color { r: 255, g: 128, b: 64, a: 255 };
/// let bytes = color.into_byte_array();
/// assert_eq!(bytes, [255, 128, 64, 255]);
/// # }
/// ```
pub trait IntoByteArray: AssociatedByteArray {
    /// Converts `self` into its byte array representation.
    ///
    /// This method consumes the value and returns its byte representation.
    fn into_byte_array(self) -> Self::ByteArray;
}

/// Constructs a value from its byte array representation.
///
/// This trait provides the ability to reconstruct a Rust value from a fixed-size byte array,
/// enabling zero-overhead deserialization. This is the inverse operation of [`IntoByteArray`].
///
/// # Implementations
///
/// This trait is implemented for:
/// - All primitive numeric types (`u8`, `i32`, `f64`, etc.)
/// - `bool` and `char` (via `TryFromByteArray` with validation)
/// - Fixed-size arrays of types implementing `FromByteArray`
/// - `BigEndian<T>` and `LittleEndian<T>` wrappers
/// - Custom types via `#[derive(Byteable)]`
///
/// # Examples
///
/// ## With primitive types
///
/// ```
/// use byteable::FromByteArray;
///
/// let bytes = [0x78, 0x56, 0x34, 0x12];
///
/// // On little-endian systems
/// #[cfg(target_endian = "little")]
/// {
///     let value = u32::from_byte_array(bytes);
///     assert_eq!(value, 0x12345678);
/// }
/// ```
///
/// ## With custom types
///
/// ```
/// # #![cfg(feature = "derive")]
/// use byteable::{Byteable, FromByteArray};
///
/// #[derive(Byteable, Debug, PartialEq, Clone, Copy)]
/// struct Color {
///     r: u8,
///     g: u8,
///     b: u8,
///     a: u8,
/// }
///
/// # fn main() {
/// let bytes = [255, 128, 64, 255];
/// let color = Color::from_byte_array(bytes);
/// assert_eq!(color, Color { r: 255, g: 128, b: 64, a: 255 });
/// # }
/// ```
pub trait FromByteArray: AssociatedByteArray {
    /// Constructs a value from its byte array representation.
    ///
    /// This method consumes the byte array and returns the reconstructed value.
    fn from_byte_array(byte_array: Self::ByteArray) -> Self;
}

/// Attempts to convert a value into its byte array representation, potentially failing.
///
/// This trait provides fallible conversion to byte arrays, useful for types that may need
/// validation or have constraints that could prevent conversion. Types that implement
/// [`IntoByteArray`] automatically implement this trait with `Error = Infallible`.
///
/// # Examples
///
/// ```
/// use byteable::{IntoByteArray, TryIntoByteArray};
///
/// // Types that implement IntoByteArray automatically get TryIntoByteArray
/// let value: u32 = 42;
/// let bytes = value.try_into_byte_array().unwrap();
/// assert_eq!(bytes, value.into_byte_array());
/// ```
pub trait TryIntoByteArray: AssociatedByteArray {
    /// The type returned in the event of a conversion error.
    type Error;

    /// Attempts to convert `self` into its byte array representation.
    fn try_into_byte_array(self) -> Result<Self::ByteArray, Self::Error>;
}

/// Attempts to construct a value from its byte array representation, potentially failing.
///
/// This trait provides fallible construction from byte arrays, useful for types that may need
/// validation or have constraints on valid byte patterns. Types that implement [`FromByteArray`]
/// automatically implement this trait with `Error = Infallible`.
///
/// # Examples
///
/// ```
/// use byteable::{FromByteArray, TryFromByteArray};
///
/// // Types that implement FromByteArray automatically get TryFromByteArray
/// let bytes = [42, 0, 0, 0];
/// let value = u32::try_from_byte_array(bytes).unwrap();
/// assert_eq!(value, u32::from_byte_array(bytes));
/// ```
pub trait TryFromByteArray: AssociatedByteArray + Sized {
    /// The type returned in the event of a conversion error.
    type Error;

    /// Attempts to construct a value from its byte array representation.
    fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, Self::Error>;
}

impl<T: IntoByteArray> TryIntoByteArray for T {
    type Error = core::convert::Infallible;

    fn try_into_byte_array(self) -> Result<Self::ByteArray, Self::Error> {
        Ok(self.into_byte_array())
    }
}

impl<T: FromByteArray> TryFromByteArray for T {
    type Error = core::convert::Infallible;

    fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, Self::Error> {
        Ok(Self::from_byte_array(byte_array))
    }
}

/// A trait for types that have a corresponding raw representation type.
///
/// This trait is automatically implemented by the `#[derive(Byteable)]` macro to expose
/// the generated raw struct type. The raw type is typically a `#[repr(C, packed)]` struct
/// with endianness wrappers and is used internally for byte conversion.
///
/// This trait enables better type safety when using nested `Byteable` structs with the
/// `#[byteable(transparent)]` attribute. Instead of storing nested structs as raw byte arrays
/// (`[u8; N]`), the parent struct's raw type can directly reference the child struct's raw type,
/// maintaining type information throughout the conversion process.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "derive")]
/// use byteable::{Byteable, HasRawType};
///
/// # #[cfg(feature = "derive")]
/// #[derive(Clone, Copy, Byteable)]
/// struct Inner {
///     value: u8,
/// }
///
/// # #[cfg(feature = "derive")]
/// #[derive(Clone, Copy, Byteable)]
/// struct Outer {
///     #[byteable(transparent)]
///     inner: Inner,  // Uses Inner::Raw instead of [u8; 1]
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn example() {
/// // Both Inner and Outer automatically implement HasRawType via derive(Byteable)
/// // The generated raw types are properly nested and type-safe
/// # }
/// ```
pub trait HasRawType:
    AssociatedByteArray + From<Self::Raw> + IntoByteArray + FromByteArray
{
    /// The raw type used for byte conversion.
    ///
    /// This is typically a `#[repr(C, packed)]` struct with endianness wrappers
    /// that handles the actual memory layout and byte-level operations.
    type Raw: AssociatedByteArray + From<Self> + IntoByteArray + FromByteArray;
}

/// A trait for types that have a corresponding raw representation type with fallible conversion.
///
/// This trait is similar to [`HasRawType`], but is designed for types where conversion from
/// the raw representation might fail. This is particularly useful for enums, where not all
/// byte patterns represent valid discriminants.
///
/// This trait is automatically implemented by the `#[derive(Byteable)]` macro for enums.
/// The raw type can be converted to the original type using `TryFrom`.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "derive")]
/// use byteable::{Byteable, TryHasRawType, TryFromByteArray, IntoByteArray};
///
/// # #[cfg(feature = "derive")]
/// #[derive(Clone, Copy, Byteable, Debug, PartialEq)]
/// #[repr(u8)]
/// enum Status {
///     Idle = 0,
///     Running = 1,
///     Completed = 2,
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn example() {
/// // Enums automatically implement TryHasRawType
/// // Converting enum to raw (always succeeds)
/// let status = Status::Running;
/// let raw: <Status as TryHasRawType>::Raw = status.into();
///
/// // Converting raw back to enum (might fail for invalid discriminants)
/// let restored: Status = Status::try_from(raw).unwrap();
/// assert_eq!(restored, Status::Running);
///
/// // TryFromByteArray is also implemented for enums
/// let bytes = status.into_byte_array();
/// let from_bytes = Status::try_from_byte_array(bytes).unwrap();
/// assert_eq!(from_bytes, Status::Running);
/// # }
/// ```
pub trait TryHasRawType:
    AssociatedByteArray + TryFrom<Self::Raw> + IntoByteArray + TryFromByteArray
{
    /// The raw type used for byte conversion.
    ///
    /// This is typically a `#[repr(C, packed)]` struct with endianness wrappers
    /// that handles the actual memory layout and byte-level operations.
    type Raw: AssociatedByteArray + From<Self> + IntoByteArray + FromByteArray;
}

/// Blanket implementation of `TryHasRawType` for types that implement `HasRawType`.
///
/// Types with infallible raw conversion automatically get fallible conversion
/// with `Infallible` as the error type (via the blanket `TryFrom` impl for types implementing `From`).
impl<T: HasRawType> TryHasRawType for T {
    type Raw = <T as HasRawType>::Raw;
}

// Implementation of Byteable for fixed-size arrays of Byteable types
// This allows [T; N] to be Byteable if T is Byteable

impl<T: AssociatedByteArray, const SIZE: usize> AssociatedByteArray for [T; SIZE] {
    // The byte array is an array of the element's byte arrays
    type ByteArray = [T::ByteArray; SIZE];
}

impl<T: IntoByteArray, const SIZE: usize> IntoByteArray for [T; SIZE] {
    fn into_byte_array(self) -> Self::ByteArray {
        // Convert each element to its byte array representation
        self.map(T::into_byte_array)
    }
}
impl<T: FromByteArray, const SIZE: usize> FromByteArray for [T; SIZE] {
    fn from_byte_array(byte_array: Self::ByteArray) -> Self {
        // Convert each byte array back to its element type
        byte_array.map(T::from_byte_array)
    }
}

/// Implements `Byteable` for one or more types using `transmute`.
///
/// This macro provides a quick way to implement `Byteable` for types that can be
/// safely transmuted to/from byte arrays. This is useful for `#[repr(C)]` or
/// `#[repr(transparent)]` types.
///
/// # Safety
///
/// This macro uses `unsafe` code (`core::mem::transmute`). You must ensure:
/// - The type has a well-defined memory layout
/// - All byte patterns are valid for the type
/// - The type has no padding bytes with uninitialized memory
///
/// # Examples
///
/// ```
/// use byteable::{Byteable, unsafe_byteable_transmute, IntoByteArray};
///
/// #[derive(Clone, Copy)]
/// #[repr(transparent)]
/// struct MyU32(u32);
///
/// unsafe_byteable_transmute!(MyU32);
///
/// let value = MyU32(0x12345678);
/// let bytes = value.into_byte_array();
/// ```
///
/// Multiple types can be implemented at once:
///
/// ```
/// use byteable::unsafe_byteable_transmute;
///
/// #[derive(Clone, Copy)]
/// #[repr(transparent)]
/// struct TypeA(u16);
///
/// #[derive(Clone, Copy)]
/// #[repr(transparent)]
/// struct TypeB(u32);
///
/// unsafe_byteable_transmute!(TypeA, TypeB);
/// ```
#[macro_export]
macro_rules! unsafe_byteable_transmute {
    ($($type:ty),+) => {
        $(
            impl $crate::AssociatedByteArray for $type {
                type ByteArray = [u8; ::core::mem::size_of::<Self>()];
            }

            impl $crate::IntoByteArray for $type {
                fn into_byte_array(self) -> Self::ByteArray {
                    unsafe { ::core::mem::transmute(self) }
                }
            }
            impl $crate::FromByteArray for $type {
                fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                    unsafe { ::core::mem::transmute(byte_array) }
                }
            }
        )+
    };
}

/// Implements `Byteable` for a type by delegating to another type.
///
/// This macro is useful when you have a "user-friendly" type and a "raw" type that
/// can be converted between each other. The raw type must already implement `Byteable`,
/// and both types must implement `From` for converting between them.
///
/// This pattern is common when you want to separate concerns:
/// - The raw type handles byte layout (with endianness markers, packed representation)
/// - The user-facing type provides a convenient API (with native types, methods)
///
/// # Requirements
///
/// - `$raw_type` must implement `Byteable`
/// - `$regular_type` must implement `From<$raw_type>`
/// - `$raw_type` must implement `From<$regular_type>`
///
/// # Examples
///
/// ```
/// use byteable::{Byteable, LittleEndian, impl_byteable_via, IntoByteArray, FromByteArray};
///
/// # #[cfg(feature = "derive")]
/// use byteable::UnsafeByteableTransmute;
///
/// // Raw type with explicit byte layout
/// # #[cfg(feature = "derive")]
/// #[derive(byteable::UnsafeByteableTransmute, Clone, Copy)]
/// #[repr(C, packed)]
/// struct PointRaw {
///     x: LittleEndian<i32>,
///     y: LittleEndian<i32>,
/// }
///
/// // User-friendly type
/// #[derive(Debug, PartialEq, Clone, Copy)]
/// struct Point {
///     x: i32,
///     y: i32,
/// }
///
/// # #[cfg(feature = "derive")]
/// // Implement conversions
/// impl From<Point> for PointRaw {
///     fn from(p: Point) -> Self {
///         Self {
///             x: p.x.into(),
///             y: p.y.into(),
///         }
///     }
/// }
///
/// # #[cfg(feature = "derive")]
/// impl From<PointRaw> for Point {
///     fn from(raw: PointRaw) -> Self {
///         Self {
///             x: raw.x.get(),
///             y: raw.y.get(),
///         }
///     }
/// }
///
/// # #[cfg(feature = "derive")]
/// // Now Point implements Byteable via PointRaw
/// impl_byteable_via!(Point => PointRaw);
///
/// # #[cfg(feature = "derive")]
/// # fn example() {
/// let point = Point { x: 100, y: 200 };
/// let bytes = point.into_byte_array();
/// let restored = Point::from_byte_array(bytes);
/// assert_eq!(restored, point);
/// # }
/// ```
#[macro_export]
macro_rules! impl_byteable_via {
    ($regular_type:ty => $raw_type:ty) => {
        impl $crate::AssociatedByteArray for $regular_type {
            type ByteArray = <$raw_type as $crate::AssociatedByteArray>::ByteArray;
        }

        impl $crate::IntoByteArray for $regular_type {
            fn into_byte_array(self) -> Self::ByteArray {
                let raw: $raw_type = self.into();
                raw.into_byte_array()
            }
        }

        impl $crate::FromByteArray for $regular_type {
            fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                let raw = <$raw_type>::from_byte_array(byte_array);
                raw.into()
            }
        }
    };
}

macro_rules! impl_byteable_primitive {
    ($($type:ty),+) => {
        $(
            impl $crate::AssociatedByteArray for $type {
                type ByteArray = [u8; ::core::mem::size_of::<Self>()];
            }

            impl $crate::IntoByteArray for $type {
                fn into_byte_array(self) -> Self::ByteArray {
                    <$type>::to_ne_bytes(self)
                }
            }

            impl $crate::FromByteArray for $type {
                fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                    <$type>::from_ne_bytes(byte_array)
                }
            }
        )+
    };
}

impl_byteable_primitive!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct BoolRaw(u8);

unsafe impl ValidBytecastMarker for BoolRaw {}

unsafe_byteable_transmute!(BoolRaw);

impl From<bool> for BoolRaw {
    fn from(value: bool) -> Self {
        Self(value as u8)
    }
}

impl TryFrom<BoolRaw> for bool {
    type Error = EnumFromBytesError;

    fn try_from(value: BoolRaw) -> Result<Self, Self::Error> {
        match value.0 {
            0 => Ok(false),
            1 => Ok(true),
            invalid => Err(EnumFromBytesError::new(
                invalid,
                ::core::any::type_name::<Self>(),
            )),
        }
    }
}

impl AssociatedByteArray for bool {
    type ByteArray = <<bool as TryHasRawType>::Raw as AssociatedByteArray>::ByteArray;
}

impl IntoByteArray for bool {
    fn into_byte_array(self) -> Self::ByteArray {
        let raw: <Self as TryHasRawType>::Raw = self.into();
        raw.into_byte_array()
    }
}

impl TryFromByteArray for bool {
    type Error = EnumFromBytesError;

    fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, Self::Error> {
        let raw = <Self as TryHasRawType>::Raw::from_byte_array(byte_array);
        raw.try_into()
    }
}

impl TryHasRawType for bool {
    type Raw = BoolRaw;
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct CharRaw(LittleEndian<u32>);

// because of code point representation this should always be little endian, i think

unsafe impl ValidBytecastMarker for CharRaw {}

unsafe_byteable_transmute!(CharRaw);

impl From<char> for CharRaw {
    fn from(value: char) -> Self {
        Self((value as u32).into())
    }
}

impl TryFrom<CharRaw> for char {
    type Error = EnumFromBytesError;

    fn try_from(value: CharRaw) -> Result<Self, Self::Error> {
        let num = value.0.get();
        match char::from_u32(num) {
            Some(c) => Ok(c),
            None => Err(EnumFromBytesError::new(
                num,
                ::core::any::type_name::<Self>(),
            )),
        }
    }
}

impl AssociatedByteArray for char {
    type ByteArray = <<char as TryHasRawType>::Raw as AssociatedByteArray>::ByteArray;
}

impl IntoByteArray for char {
    fn into_byte_array(self) -> Self::ByteArray {
        let raw: <Self as TryHasRawType>::Raw = self.into();
        raw.into_byte_array()
    }
}

impl TryFromByteArray for char {
    type Error = EnumFromBytesError;

    fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, Self::Error> {
        let raw = <Self as TryHasRawType>::Raw::from_byte_array(byte_array);
        raw.try_into()
    }
}

impl TryHasRawType for char {
    type Raw = CharRaw;
}

/// Represents a discriminant value that can be of various integer types.
///
/// This enum stores the invalid discriminant value with its original type information,
/// allowing for proper formatting and type-safe error handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Discriminant {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    U128(u128),
    I128(i128),
}

impl core::fmt::Display for Discriminant {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Discriminant::U8(v) => write!(f, "{} (u8)", v),
            Discriminant::I8(v) => write!(f, "{} (i8)", v),
            Discriminant::U16(v) => write!(f, "{} (u16)", v),
            Discriminant::I16(v) => write!(f, "{} (i16)", v),
            Discriminant::U32(v) => write!(f, "{} (u32)", v),
            Discriminant::I32(v) => write!(f, "{} (i32)", v),
            Discriminant::U64(v) => write!(f, "{} (u64)", v),
            Discriminant::I64(v) => write!(f, "{} (i64)", v),
            Discriminant::U128(v) => write!(f, "{} (u128)", v),
            Discriminant::I128(v) => write!(f, "{} (i128)", v),
        }
    }
}

// Implement From for all supported integer types
impl From<u8> for Discriminant {
    fn from(v: u8) -> Self {
        Discriminant::U8(v)
    }
}

impl From<i8> for Discriminant {
    fn from(v: i8) -> Self {
        Discriminant::I8(v)
    }
}

impl From<u16> for Discriminant {
    fn from(v: u16) -> Self {
        Discriminant::U16(v)
    }
}

impl From<i16> for Discriminant {
    fn from(v: i16) -> Self {
        Discriminant::I16(v)
    }
}

impl From<u32> for Discriminant {
    fn from(v: u32) -> Self {
        Discriminant::U32(v)
    }
}

impl From<i32> for Discriminant {
    fn from(v: i32) -> Self {
        Discriminant::I32(v)
    }
}

impl From<u64> for Discriminant {
    fn from(v: u64) -> Self {
        Discriminant::U64(v)
    }
}

impl From<i64> for Discriminant {
    fn from(v: i64) -> Self {
        Discriminant::I64(v)
    }
}

impl From<u128> for Discriminant {
    fn from(v: u128) -> Self {
        Discriminant::U128(v)
    }
}

impl From<i128> for Discriminant {
    fn from(v: i128) -> Self {
        Discriminant::I128(v)
    }
}

/// Error type for converting bytes to an enum with an invalid discriminant.
///
/// This error is returned when attempting to convert a byte array to an enum
/// value, but the discriminant value doesn't match any valid enum variant.
///
/// The error includes both the invalid discriminant value (with its original type)
/// and the enum type name for better diagnostics.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "derive")]
/// use byteable::{Byteable, TryFromByteArray, IntoByteArray};
///
/// # #[cfg(feature = "derive")]
/// #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
/// #[repr(u8)]
/// enum Status {
///     Idle = 0,
///     Running = 1,
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn example() {
/// // Valid discriminant
/// let bytes = [1];
/// let status = Status::try_from_byte_array(bytes).unwrap();
/// assert_eq!(status, Status::Running);
///
/// // Invalid discriminant
/// let bytes = [255];
/// let result = Status::try_from_byte_array(bytes);
/// assert!(result.is_err());
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumFromBytesError {
    /// The invalid discriminant value that was encountered
    pub invalid_discriminant: Discriminant,
    /// The name of the enum type that failed to match
    pub enum_type_name: &'static str,
}

impl EnumFromBytesError {
    /// Creates a new `EnumFromBytesError` with the given invalid discriminant and type name.
    pub fn new<T: Into<Discriminant>>(
        invalid_discriminant: T,
        enum_type_name: &'static str,
    ) -> Self {
        Self {
            invalid_discriminant: invalid_discriminant.into(),
            enum_type_name,
        }
    }
}

impl core::fmt::Display for EnumFromBytesError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Invalid discriminant value {} for enum '{}'",
            self.invalid_discriminant, self.enum_type_name
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EnumFromBytesError {}

#[cfg(test)]
mod tests {
    use crate::{AssociatedByteArray, BigEndian, FromByteArray, IntoByteArray, LittleEndian};
    use byteable_derive::UnsafeByteableTransmute;

    #[derive(Clone, Copy, PartialEq, Debug, UnsafeByteableTransmute)]
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
        assert_eq!(a.into_byte_array(), expected_bytes);

        let read_a = ABC::from_byte_array(expected_bytes);
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
        assert_eq!(a.into_byte_array(), expected_bytes);

        let read = ABC::from_byte_array(expected_bytes);
        assert_eq!(read.a.get(), 1);
        assert_eq!(read.b.get(), 2);
        assert_eq!(read.c.get(), 3);
        assert_eq!(read, a);
    }

    #[derive(Clone, Copy, PartialEq, Debug, UnsafeByteableTransmute)]
    #[repr(C, packed)]
    struct MyRawStruct {
        a: u8,
        b: LittleEndian<u32>,
        c: LittleEndian<u16>,
        d: u8,
        e: u8,
    }

    #[derive(Clone, Copy, PartialEq, Debug)]
    struct MyRegularStruct {
        a: u8,
        b: u32,
        c: u16,
        d: u8,
        e: u8,
    }

    impl From<MyRawStruct> for MyRegularStruct {
        fn from(value: MyRawStruct) -> Self {
            Self {
                a: value.a,
                b: value.b.get(),
                c: value.c.get(),
                d: value.d,
                e: value.e,
            }
        }
    }

    impl From<MyRegularStruct> for MyRawStruct {
        fn from(value: MyRegularStruct) -> Self {
            Self {
                a: value.a,
                b: value.b.into(),
                c: value.c.into(),
                d: value.d,
                e: value.e,
            }
        }
    }

    impl_byteable_via!(MyRegularStruct => MyRawStruct);

    #[test]
    fn test_byteable_regular() {
        let my_struct = MyRegularStruct {
            a: 192,
            b: 168,
            c: 1,
            d: 1,
            e: 2,
        };

        let bytes = my_struct.into_byte_array();
        assert_eq!(bytes, [192, 168, 0, 0, 0, 1, 0, 1, 2]);

        let struct_from_bytes = MyRegularStruct::from_byte_array([192, 168, 0, 0, 0, 1, 0, 1, 2]);
        assert_eq!(struct_from_bytes, my_struct);

        assert_eq!(MyRegularStruct::BYTE_SIZE, 9);
    }
}
