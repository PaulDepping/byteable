//! Byteable implementations for [`ordered_float`] types: [`OrderedFloat`] and [`NotNan`].
//!
//! [`OrderedFloat<T>`] implements [`FromByteArray`] infallibly — any bit pattern is valid,
//! because `OrderedFloat` deliberately supports NaN values (treating them as equal to each other).
//!
//! [`NotNan<T>`] implements [`TryFromByteArray`] fallibly — NaN bit patterns are rejected.
//!
//! Both types also implement [`EndianConvert`] by delegating to the inner float type.
//!
//! [`OrderedFloat`]: ordered_float::OrderedFloat
//! [`NotNan`]: ordered_float::NotNan
use crate::{
    BigEndian, ByteRepr, DecodeError, EndianConvert, FromByteArray, IntoByteArray,
    LittleEndian, PlainOldData, TryFromByteArray,
};
use ordered_float::{NotNan, OrderedFloat};

// --- OrderedFloat<f32> ---

impl ByteRepr for OrderedFloat<f32> {
    type ByteArray = [u8; 4];
}

impl IntoByteArray for OrderedFloat<f32> {
    #[inline]
    fn into_byte_array(self) -> Self::ByteArray {
        self.0.into_byte_array()
    }
}

impl FromByteArray for OrderedFloat<f32> {
    #[inline]
    fn from_byte_array(byte_array: Self::ByteArray) -> Self {
        OrderedFloat(f32::from_byte_array(byte_array))
    }
}

impl EndianConvert for OrderedFloat<f32> {
    #[inline]
    fn from_le(value: Self) -> Self {
        OrderedFloat(f32::from_le(value.0))
    }

    #[inline]
    fn from_be(value: Self) -> Self {
        OrderedFloat(f32::from_be(value.0))
    }

    #[inline]
    fn to_le(self) -> Self {
        OrderedFloat(self.0.to_le())
    }

    #[inline]
    fn to_be(self) -> Self {
        OrderedFloat(self.0.to_be())
    }
}

// --- OrderedFloat<f64> ---

impl ByteRepr for OrderedFloat<f64> {
    type ByteArray = [u8; 8];
}

impl IntoByteArray for OrderedFloat<f64> {
    #[inline]
    fn into_byte_array(self) -> Self::ByteArray {
        self.0.into_byte_array()
    }
}

impl FromByteArray for OrderedFloat<f64> {
    #[inline]
    fn from_byte_array(byte_array: Self::ByteArray) -> Self {
        OrderedFloat(f64::from_byte_array(byte_array))
    }
}

impl EndianConvert for OrderedFloat<f64> {
    #[inline]
    fn from_le(value: Self) -> Self {
        OrderedFloat(f64::from_le(value.0))
    }

    #[inline]
    fn from_be(value: Self) -> Self {
        OrderedFloat(f64::from_be(value.0))
    }

    #[inline]
    fn to_le(self) -> Self {
        OrderedFloat(self.0.to_le())
    }

    #[inline]
    fn to_be(self) -> Self {
        OrderedFloat(self.0.to_be())
    }
}

// --- NotNan<f32> ---

impl ByteRepr for NotNan<f32> {
    type ByteArray = [u8; 4];
}

impl IntoByteArray for NotNan<f32> {
    #[inline]
    fn into_byte_array(self) -> Self::ByteArray {
        self.into_inner().into_byte_array()
    }
}

impl TryFromByteArray for NotNan<f32> {
    type Error = DecodeError;

    #[inline]
    fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, Self::Error> {
        let val = f32::from_byte_array(byte_array);
        NotNan::new(val).map_err(|_| {
            DecodeError::new(val.to_bits(), ::core::any::type_name::<Self>())
        })
    }
}

// --- NotNan<f64> ---

impl ByteRepr for NotNan<f64> {
    type ByteArray = [u8; 8];
}

impl IntoByteArray for NotNan<f64> {
    #[inline]
    fn into_byte_array(self) -> Self::ByteArray {
        self.into_inner().into_byte_array()
    }
}

impl TryFromByteArray for NotNan<f64> {
    type Error = DecodeError;

    #[inline]
    fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, Self::Error> {
        let val = f64::from_byte_array(byte_array);
        NotNan::new(val).map_err(|_| {
            DecodeError::new(val.to_bits(), ::core::any::type_name::<Self>())
        })
    }
}

// --- PlainOldData ---
//
// `BigEndian<OrderedFloat<T>>` and `LittleEndian<OrderedFloat<T>>` are safe to transmute:
// - `OrderedFloat<T>` is `#[repr(transparent)]` over `T`, so the memory layout is identical
// - Every possible bit pattern is a valid `OrderedFloat` (NaN is explicitly allowed)
// - The endian wrappers enforce explicit byte-order choice, matching the crate's portability policy
//
// `NotNan<T>` does NOT get `PlainOldData` (nor its endian-wrapped forms, which don't exist
// because `NotNan` intentionally doesn't implement `EndianConvert`):
// - NaN bit patterns are invalid for `NotNan`, so transmuting arbitrary bytes would be unsound

unsafe impl PlainOldData for BigEndian<OrderedFloat<f32>> {}
unsafe impl PlainOldData for LittleEndian<OrderedFloat<f32>> {}
unsafe impl PlainOldData for BigEndian<OrderedFloat<f64>> {}
unsafe impl PlainOldData for LittleEndian<OrderedFloat<f64>> {}
