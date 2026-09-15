//! [`RawRepr`], [`IntoByteArray`], and endian-conversion impls for
//! [`ordered_float::OrderedFloat<T>`] and [`ordered_float::NotNan<T>`]
//! (requires the `ordered-float` feature).
//!
//! ## `OrderedFloat<T>`
//!
//! `OrderedFloat<T>` is a transparent wrapper around `f32` or `f64` that provides a total
//! ordering. Its serialization delegates entirely to the inner float, so the wire format is
//! identical to the unwrapped primitive. Decoding is infallible (NaN is a valid
//! `OrderedFloat` value).
//!
//! ## `NotNan<T>`
//!
//! `NotNan<T>` is a wrapper around `f32` or `f64` that guarantees the value is not NaN.
//! Its wire format is the same as the inner float, but decoding returns
//! [`DecodeError::InvalidNaN`] if the bytes decode to NaN.

use crate::{
    BigEndian, DecodeError, EndianConvert, FromByteArray, FromEndianRepr, FromRawRepr,
    HasEndianRepr, IntoByteArray, LittleEndian, PlainOldData, RawRepr, TryFromByteArray,
    TryFromEndianRepr, TryFromRawRepr,
    core_types::{impl_byte_array_via_raw, impl_try_byte_array_via_raw},
};
use ordered_float::{FloatCore, NotNan, OrderedFloat};

// --- OrderedFloat<f32> ---

impl<T: RawRepr> RawRepr for OrderedFloat<T> {
    type Raw = T::Raw;

    fn to_raw(&self) -> Self::Raw {
        self.0.to_raw()
    }
}

impl<T: FromRawRepr> FromRawRepr for OrderedFloat<T> {
    fn from_raw(raw: Self::Raw) -> Self {
        Self(T::from_raw(raw))
    }
}

impl<T: TryFromRawRepr> TryFromRawRepr for OrderedFloat<T> {
    fn try_from_raw(raw: Self::Raw) -> Result<Self, DecodeError> {
        Ok(Self(T::try_from_raw(raw)?))
    }
}

unsafe impl<T: PlainOldData> PlainOldData for OrderedFloat<T> {}

impl_byte_array_via_raw!(OrderedFloat<f32>, OrderedFloat<f64>);

impl<T: RawRepr + FloatCore> RawRepr for NotNan<T> {
    type Raw = T::Raw;

    fn to_raw(&self) -> Self::Raw {
        self.into_inner().to_raw()
    }
}

impl<T: TryFromRawRepr + FloatCore> TryFromRawRepr for NotNan<T> {
    fn try_from_raw(raw: Self::Raw) -> Result<Self, DecodeError> {
        let inner = T::try_from_raw(raw)?;
        Self::new(inner).map_err(|_| DecodeError::InvalidNaN)
    }
}

impl<T: EndianConvert> HasEndianRepr for OrderedFloat<T> {
    type LE = LittleEndian<T>;

    type BE = BigEndian<T>;

    fn to_little_endian(self) -> Self::LE {
        LittleEndian::new(self.0)
    }

    fn to_big_endian(self) -> Self::BE {
        BigEndian::new(self.0)
    }
}

impl<T: EndianConvert> FromEndianRepr for OrderedFloat<T> {
    fn from_little_endian(le: Self::LE) -> Self {
        Self(le.get())
    }

    fn from_big_endian(be: Self::BE) -> Self {
        Self(be.get())
    }
}

impl<T: EndianConvert + FloatCore> HasEndianRepr for NotNan<T> {
    type LE = LittleEndian<T>;

    type BE = BigEndian<T>;

    fn to_little_endian(self) -> Self::LE {
        LittleEndian::new(self.into_inner())
    }

    fn to_big_endian(self) -> Self::BE {
        BigEndian::new(self.into_inner())
    }
}

impl<T: EndianConvert + FloatCore> TryFromEndianRepr for NotNan<T> {
    fn try_from_little_endian(le: Self::LE) -> Result<Self, DecodeError> {
        Self::new(le.get()).map_err(|_| DecodeError::InvalidNaN)
    }

    fn try_from_big_endian(be: Self::BE) -> Result<Self, DecodeError> {
        Self::new(be.get()).map_err(|_| DecodeError::InvalidNaN)
    }
}

impl_try_byte_array_via_raw!(NotNan<f32>, NotNan<f64>);
