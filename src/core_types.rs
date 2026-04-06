//! [`RawRepr`], [`IntoByteArray`], and [`FromByteArray`] implementations for primitive and
//! standard-library types that have a fixed, well-defined byte representation.
//!
//! Covered types: `u8`/`i8` (identity repr), multi-byte integers and floats (little-endian
//! by default), `bool` (1 byte, 0 or 1), `char` (4-byte little-endian Unicode scalar),
//! [`PhantomData<T>`](core::marker::PhantomData) (0 bytes),
//! [`NonZero<T>`](core::num::NonZero), network address types
//! (`Ipv4Addr`, `Ipv6Addr`, `SocketAddrV4`, `SocketAddrV6`), all range variants, and
//! [`Duration`](core::time::Duration) /
//! [`SystemTime`](std::time::SystemTime) (`std` feature only).
//!
//! ## `SystemTime` wire format
//!
//! `SystemTime` is encoded as a **signed** `i64` seconds offset from the Unix epoch
//! followed by a `u32` sub-second nanoseconds field (always in `[0, 999_999_999]`).
//! Negative seconds represent times before 1970-01-01 00:00:00 UTC, following the
//! standard POSIX `timespec` convention.

use crate::{
    DecodeError, FromByteArray, FromRawRepr, IntoByteArray, LittleEndian, PlainOldData, RawRepr,
    TryFromByteArray, TryFromRawRepr, impl_byte_array,
};
use core::{
    marker::PhantomData,
    net::Ipv4Addr,
    net::{Ipv6Addr, SocketAddrV4, SocketAddrV6},
    num::NonZero,
    ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
    time::Duration,
};
#[cfg(feature = "std")]
use std::time::SystemTime;

macro_rules! rawrepr_self {
    ($($type:ty),+) => {
        $(
            impl RawRepr for $type {
                type Raw = Self;

                fn to_raw(&self) -> Self::Raw {
                    *self
                }
            }

            impl FromRawRepr for $type {
                fn from_raw(raw: Self::Raw) -> Self {
                    raw
                }
            }

            impl TryFromRawRepr for $type {
                fn try_from_raw(raw: Self::Raw) -> Result<Self, DecodeError> {
                    Ok(raw)
                }
            }

            impl IntoByteArray for $type {
                type ByteArray = [u8; ::core::mem::size_of::<$type>()];

                fn into_byte_array(&self) -> Self::ByteArray {
                    #[allow(unnecessary_transmutes)]
                    unsafe { ::core::mem::transmute(*self) }
                }
            }

            impl FromByteArray for $type {
                fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                    #[allow(unnecessary_transmutes)]
                    unsafe { ::core::mem::transmute(byte_array) }
                }
            }


            // impl TryFromRawRepr for $type {
            //     fn try_from_raw(raw: Self::Raw) -> Result<Self, DecodeError> {
            //         Ok(raw)
            //     }
            // }
        )+
    };
}

rawrepr_self!(u8, i8);

impl RawRepr for bool {
    type Raw = u8;

    fn to_raw(&self) -> Self::Raw {
        *self as _
    }
}

impl TryFromRawRepr for bool {
    fn try_from_raw(raw: Self::Raw) -> Result<Self, DecodeError> {
        match raw {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DecodeError::InvalidBool(raw)),
        }
    }
}

impl IntoByteArray for bool
where
    bool: RawRepr,
    <bool as RawRepr>::Raw: IntoByteArray,
{
    type ByteArray = [u8; ::core::mem::size_of::<<bool as RawRepr>::Raw>()];
    fn into_byte_array(&self) -> Self::ByteArray {
        <Self as RawRepr>::to_raw(self).into_byte_array()
    }
}

impl TryFromByteArray for bool
where
    bool: TryFromRawRepr,
    <bool as RawRepr>::Raw: FromByteArray,
{
    fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, DecodeError> {
        let raw = <<Self as RawRepr>::Raw as FromByteArray>::from_byte_array(byte_array);
        Self::try_from_raw(raw)
    }
}

impl RawRepr for char {
    type Raw = LittleEndian<u32>;

    fn to_raw(&self) -> Self::Raw {
        (*self as u32).into()
    }
}

impl TryFromRawRepr for char {
    fn try_from_raw(raw: Self::Raw) -> Result<Self, DecodeError> {
        let c = raw.get();
        char::from_u32(c).ok_or(DecodeError::InvalidChar(c))
    }
}

impl IntoByteArray for char
where
    char: RawRepr,
    <char as RawRepr>::Raw: IntoByteArray,
{
    type ByteArray = [u8; ::core::mem::size_of::<<Self as RawRepr>::Raw>()];
    fn into_byte_array(&self) -> Self::ByteArray {
        <Self as RawRepr>::to_raw(self).into_byte_array()
    }
}

impl TryFromByteArray for char
where
    char: TryFromRawRepr,
    <char as RawRepr>::Raw: FromByteArray,
{
    fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, DecodeError> {
        let raw = <<Self as RawRepr>::Raw as FromByteArray>::from_byte_array(byte_array);
        Self::try_from_raw(raw)
    }
}

macro_rules! impl_try_from_rawrepr {
    ($($type:ty),+) => {
        $(
            impl TryFromRawRepr for $type {
                fn try_from_raw(raw: Self::Raw) -> Result<Self, DecodeError> {
                    Ok(Self::from_raw(raw))
                }
            }
        )+
    };
}

macro_rules! raw_repr_multibyte {
    ($($type:ty),+) => {
        $(
            impl RawRepr for $type {
                type Raw = LittleEndian<Self>;

                fn to_raw(&self) -> Self::Raw {
                    Self::Raw::from(*self)
                }
            }

            impl FromRawRepr for $type {
                fn from_raw(raw: Self::Raw) -> Self {
                    raw.get()
                }
            }

            impl_try_from_rawrepr!($type);

            impl IntoByteArray for $type
            where
                $type: RawRepr,
                <$type as RawRepr>::Raw: IntoByteArray,
            {
                type ByteArray = [u8; ::core::mem::size_of::<<Self as RawRepr>::Raw>()];
                fn into_byte_array(&self) -> Self::ByteArray {
                    <Self as RawRepr>::to_raw(self).into_byte_array()
                }
            }

            impl FromByteArray for $type
            where
                $type: TryFromRawRepr,
                <$type as RawRepr>::Raw: FromByteArray,
            {
                fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                    let raw = <<Self as RawRepr>::Raw as FromByteArray>::from_byte_array(byte_array);
                    Self::from_raw(raw)
                }
            }
        )+
    };
}

raw_repr_multibyte!(u16, u32, u64, u128, i16, i32, i64, i128, f32, f64);

macro_rules! impl_byte_array_via_raw {
    ($($ty:ty),+) => {
        $(
            impl IntoByteArray for $ty
                where $ty : RawRepr,
                      <$ty as RawRepr>::Raw : IntoByteArray
            {
                type ByteArray = [u8; ::core::mem::size_of::<<$ty as RawRepr>::Raw>()];
                fn into_byte_array(&self) -> Self::ByteArray {
                    <Self as RawRepr>::to_raw(self).into_byte_array()
                }
            }

            impl FromByteArray for $ty
                where $ty : FromRawRepr,
                      <$ty as RawRepr>::Raw : FromByteArray
            {
                fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                    let raw = <<Self as RawRepr>::Raw as FromByteArray>::from_byte_array(byte_array);
                    Self::from_raw(raw)
                }
            }
        )+
    };
}

macro_rules! impl_try_byte_array_via_raw {
    ($($ty:ty),+) => {
        $(
            impl IntoByteArray for $ty
                where $ty : RawRepr,
                      <$ty as RawRepr>::Raw : IntoByteArray
            {
                type ByteArray = [u8; ::core::mem::size_of::<<$ty as RawRepr>::Raw>()];
                fn into_byte_array(&self) -> Self::ByteArray {
                    <Self as RawRepr>::to_raw(self).into_byte_array()
                }
            }

            impl TryFromByteArray for $ty
                where $ty : TryFromRawRepr,
                      <$ty as RawRepr>::Raw : FromByteArray
            {
                fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, DecodeError> {
                    let raw = <<Self as RawRepr>::Raw as FromByteArray>::from_byte_array(byte_array);
                    Self::try_from_raw(raw)
                }
            }
        )+
    };
}

#[allow(unused_imports)]
pub(crate) use impl_byte_array_via_raw;
#[allow(unused_imports)]
pub(crate) use impl_try_byte_array_via_raw;

#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct UnitStructRaw;
unsafe impl PlainOldData for UnitStructRaw {}
impl_byte_array!(UnitStructRaw);

impl<T> RawRepr for PhantomData<T> {
    type Raw = UnitStructRaw;

    fn to_raw(&self) -> Self::Raw {
        UnitStructRaw
    }
}

impl<T> FromRawRepr for PhantomData<T> {
    fn from_raw(_: Self::Raw) -> Self {
        Self
    }
}

impl<T> TryFromRawRepr for PhantomData<T> {
    fn try_from_raw(_: Self::Raw) -> Result<Self, DecodeError> {
        Ok(Self)
    }
}

impl<T> IntoByteArray for PhantomData<T> {
    type ByteArray = [u8; 0];

    fn into_byte_array(&self) -> Self::ByteArray {
        []
    }
}

impl<T> FromByteArray for PhantomData<T> {
    fn from_byte_array(_: Self::ByteArray) -> Self {
        Self
    }
}

macro_rules! impl_nonzero {
    ($($type:ty),+) => {
        $(
            impl RawRepr for NonZero<$type> {
                type Raw = <$type as RawRepr>::Raw;

                fn to_raw(&self) -> Self::Raw {
                    self.get().to_raw()
                }
            }

            impl TryFromRawRepr for NonZero<$type> {
                fn try_from_raw(raw: Self::Raw) -> Result<Self, DecodeError> {
                    Self::new(<$type>::from_raw(raw)).ok_or(DecodeError::InvalidZero)
                }
            }

            impl_try_byte_array_via_raw!(NonZero<$type>);
        )+
    };
}

impl_nonzero!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

#[repr(C, packed)]
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct Ipv4AddrRaw {
    octets: [u8; 4],
}
unsafe impl PlainOldData for Ipv4AddrRaw {}
impl_byte_array!(Ipv4AddrRaw);

impl RawRepr for Ipv4Addr {
    type Raw = Ipv4AddrRaw;

    fn to_raw(&self) -> Self::Raw {
        Ipv4AddrRaw {
            octets: self.octets(),
        }
    }
}

impl FromRawRepr for Ipv4Addr {
    fn from_raw(raw: Self::Raw) -> Self {
        Self::from_octets(raw.octets)
    }
}

impl_try_from_rawrepr!(Ipv4Addr);

impl_byte_array_via_raw!(Ipv4Addr);

#[repr(C, packed)]
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct Ipv6AddrRaw {
    octets: [u8; 16],
}
unsafe impl PlainOldData for Ipv6AddrRaw {}
impl_byte_array!(Ipv6AddrRaw);

impl RawRepr for Ipv6Addr {
    type Raw = Ipv6AddrRaw;

    fn to_raw(&self) -> Self::Raw {
        Ipv6AddrRaw {
            octets: self.octets(),
        }
    }
}

impl FromRawRepr for Ipv6Addr {
    fn from_raw(raw: Self::Raw) -> Self {
        Self::from_octets(raw.octets)
    }
}

impl_try_from_rawrepr!(Ipv6Addr);

impl_byte_array_via_raw!(Ipv6Addr);

#[repr(C, packed)]
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct SocketAddrV4Raw {
    ip: <Ipv4Addr as RawRepr>::Raw,
    port: <u16 as RawRepr>::Raw,
}

unsafe impl PlainOldData for SocketAddrV4Raw {}
impl_byte_array!(SocketAddrV4Raw);

impl RawRepr for SocketAddrV4 {
    type Raw = SocketAddrV4Raw;

    fn to_raw(&self) -> Self::Raw {
        SocketAddrV4Raw {
            ip: self.ip().to_raw(),
            port: self.port().to_raw(),
        }
    }
}

impl FromRawRepr for SocketAddrV4 {
    fn from_raw(raw: Self::Raw) -> Self {
        Self::new(Ipv4Addr::from_raw(raw.ip), u16::from_raw(raw.port))
    }
}

impl_try_from_rawrepr!(SocketAddrV4);

impl_byte_array_via_raw!(SocketAddrV4);

#[repr(C, packed)]
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct SocketAddrV6Raw {
    ip: <Ipv6Addr as RawRepr>::Raw,
    port: <u16 as RawRepr>::Raw,
    flowinfo: <u32 as RawRepr>::Raw,
    scope_id: <u32 as RawRepr>::Raw,
}

unsafe impl PlainOldData for SocketAddrV6Raw {}
impl_byte_array!(SocketAddrV6Raw);

impl RawRepr for SocketAddrV6 {
    type Raw = SocketAddrV6Raw;

    fn to_raw(&self) -> Self::Raw {
        SocketAddrV6Raw {
            ip: self.ip().to_raw(),
            port: self.port().to_raw(),
            flowinfo: self.flowinfo().to_raw(),
            scope_id: self.scope_id().to_raw(),
        }
    }
}

impl FromRawRepr for SocketAddrV6 {
    fn from_raw(raw: Self::Raw) -> Self {
        Self::new(
            Ipv6Addr::from_raw(raw.ip),
            u16::from_raw(raw.port),
            u32::from_raw(raw.flowinfo),
            u32::from_raw(raw.scope_id),
        )
    }
}

impl_try_from_rawrepr!(SocketAddrV6);

impl_byte_array_via_raw!(SocketAddrV6);

macro_rules! impl_range_byteable {
    // Single-byte index types (u8, i8) — no endianness annotation needed.
    ($index_type:ty, $raw_name:ident) => {
        #[repr(C, packed)]
        #[derive(Clone, Copy)]
        #[doc(hidden)]
        pub struct $raw_name {
            start: <$index_type as RawRepr>::Raw,
            end: <$index_type as RawRepr>::Raw,
        }

        unsafe impl PlainOldData for $raw_name {}

        impl_byte_array!($raw_name);

        impl RawRepr for Range<$index_type> {
            type Raw = $raw_name;

            fn to_raw(&self) -> Self::Raw {
                $raw_name {
                    start: self.start.to_raw(),
                    end: self.end.to_raw(),
                }
            }
        }

        impl FromRawRepr for Range<$index_type> {
            fn from_raw(raw: Self::Raw) -> Self {
                Self {
                    start: <$index_type>::from_raw(raw.start),
                    end: <$index_type>::from_raw(raw.end),
                }
            }
        }

        impl_try_from_rawrepr!(Range<$index_type>);

        impl_byte_array_via_raw!(Range<$index_type>);
    };
}

impl_range_byteable!(u8, RangeU8);
impl_range_byteable!(u16, RangeU16);
impl_range_byteable!(u32, RangeU32);
impl_range_byteable!(u64, RangeU64);
impl_range_byteable!(u128, RangeU128);
impl_range_byteable!(i8, RangeI8);
impl_range_byteable!(i16, RangeI16);
impl_range_byteable!(i32, RangeI32);
impl_range_byteable!(i64, RangeI64);
impl_range_byteable!(i128, RangeI128);

macro_rules! impl_range_inclusive_byteable {
    // Single-byte index types (u8, i8) — no endianness annotation needed.
    ($index_type:ty, $raw_name:ident) => {
        impl RawRepr for RangeInclusive<$index_type> {
            type Raw = $raw_name;

            fn to_raw(&self) -> Self::Raw {
                $raw_name {
                    start: self.start().to_raw(),
                    end: self.end().to_raw(),
                }
            }
        }

        impl FromRawRepr for RangeInclusive<$index_type> {
            fn from_raw(raw: Self::Raw) -> Self {
                Self::new(
                    <$index_type>::from_raw(raw.start),
                    <$index_type>::from_raw(raw.end),
                )
            }
        }

        impl_try_from_rawrepr!(RangeInclusive<$index_type>);

        impl_byte_array_via_raw!(RangeInclusive<$index_type>);
    };
}

impl_range_inclusive_byteable!(u8, RangeU8);
impl_range_inclusive_byteable!(u16, RangeU16);
impl_range_inclusive_byteable!(u32, RangeU32);
impl_range_inclusive_byteable!(u64, RangeU64);
impl_range_inclusive_byteable!(u128, RangeU128);
impl_range_inclusive_byteable!(i8, RangeI8);
impl_range_inclusive_byteable!(i16, RangeI16);
impl_range_inclusive_byteable!(i32, RangeI32);
impl_range_inclusive_byteable!(i64, RangeI64);
impl_range_inclusive_byteable!(i128, RangeI128);

// RangeFrom<T>, RangeTo<T>, RangeToInclusive<T> — single public field.
macro_rules! impl_range_single_byteable {
    ($std_type:ty, $field:ident, $index_type:ty, $raw_name:ident) => {
        #[repr(transparent)]
        #[derive(Clone, Copy)]
        #[doc(hidden)]
        pub struct $raw_name {
            $field: <$index_type as RawRepr>::Raw,
        }

        unsafe impl PlainOldData for $raw_name {}

        impl_byte_array!($raw_name);

        impl RawRepr for $std_type {
            type Raw = $raw_name;

            fn to_raw(&self) -> Self::Raw {
                $raw_name {
                    $field: self.$field.to_raw(),
                }
            }
        }

        impl FromRawRepr for $std_type {
            fn from_raw(raw: Self::Raw) -> Self {
                Self {
                    $field: <$index_type>::from_raw(raw.$field),
                }
            }
        }

        impl_try_from_rawrepr!($std_type);

        impl_byte_array_via_raw!($std_type);
    };
}

impl_range_single_byteable!(RangeFrom<u8>, start, u8, RangeFromU8);
impl_range_single_byteable!(RangeFrom<i8>, start, i8, RangeFromI8);
impl_range_single_byteable!(RangeFrom<u16>, start, u16, RangeFromU16);
impl_range_single_byteable!(RangeFrom<u32>, start, u32, RangeFromU32);
impl_range_single_byteable!(RangeFrom<u64>, start, u64, RangeFromU64);
impl_range_single_byteable!(RangeFrom<u128>, start, u128, RangeFromU128);
impl_range_single_byteable!(RangeFrom<i16>, start, i16, RangeFromI16);
impl_range_single_byteable!(RangeFrom<i32>, start, i32, RangeFromI32);
impl_range_single_byteable!(RangeFrom<i64>, start, i64, RangeFromI64);
impl_range_single_byteable!(RangeFrom<i128>, start, i128, RangeFromI128);
impl_range_single_byteable!(RangeFrom<f32>, start, f32, RangeFromF32);
impl_range_single_byteable!(RangeFrom<f64>, start, f64, RangeFromF64);

impl_range_single_byteable!(RangeTo<u8>, end, u8, RangeToU8);
impl_range_single_byteable!(RangeTo<i8>, end, i8, RangeToI8);
impl_range_single_byteable!(RangeTo<u16>, end, u16, RangeToU16);
impl_range_single_byteable!(RangeTo<u32>, end, u32, RangeToU32);
impl_range_single_byteable!(RangeTo<u64>, end, u64, RangeToU64);
impl_range_single_byteable!(RangeTo<u128>, end, u128, RangeToU128);
impl_range_single_byteable!(RangeTo<i16>, end, i16, RangeToI16);
impl_range_single_byteable!(RangeTo<i32>, end, i32, RangeToI32);
impl_range_single_byteable!(RangeTo<i64>, end, i64, RangeToI64);
impl_range_single_byteable!(RangeTo<i128>, end, i128, RangeToI128);
impl_range_single_byteable!(RangeTo<f32>, end, f32, RangeToF32);
impl_range_single_byteable!(RangeTo<f64>, end, f64, RangeToF64);

impl_range_single_byteable!(RangeToInclusive<u8>, end, u8, RangeToInclusiveU8);
impl_range_single_byteable!(RangeToInclusive<i8>, end, i8, RangeToInclusiveI8);
impl_range_single_byteable!(RangeToInclusive<u16>, end, u16, RangeToInclusiveU16);
impl_range_single_byteable!(RangeToInclusive<u32>, end, u32, RangeToInclusiveU32);
impl_range_single_byteable!(RangeToInclusive<u64>, end, u64, RangeToInclusiveU64);
impl_range_single_byteable!(RangeToInclusive<u128>, end, u128, RangeToInclusiveU128);
impl_range_single_byteable!(RangeToInclusive<i16>, end, i16, RangeToInclusiveI16);
impl_range_single_byteable!(RangeToInclusive<i32>, end, i32, RangeToInclusiveI32);
impl_range_single_byteable!(RangeToInclusive<i64>, end, i64, RangeToInclusiveI64);
impl_range_single_byteable!(RangeToInclusive<i128>, end, i128, RangeToInclusiveI128);
impl_range_single_byteable!(RangeToInclusive<f32>, end, f32, RangeToInclusiveF32);
impl_range_single_byteable!(RangeToInclusive<f64>, end, f64, RangeToInclusiveF64);

impl RawRepr for RangeFull {
    type Raw = UnitStructRaw;

    fn to_raw(&self) -> Self::Raw {
        UnitStructRaw
    }
}

impl FromRawRepr for RangeFull {
    fn from_raw(_: Self::Raw) -> Self {
        Self
    }
}

impl TryFromRawRepr for RangeFull {
    fn try_from_raw(_: Self::Raw) -> Result<Self, DecodeError> {
        Ok(Self)
    }
}

impl_byte_array_via_raw!(RangeFull);

#[repr(C, packed)]
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct DurationRaw {
    secs: <u64 as RawRepr>::Raw,
    nanos: <u32 as RawRepr>::Raw,
}

unsafe impl PlainOldData for DurationRaw {}
impl_byte_array!(DurationRaw);

impl RawRepr for Duration {
    type Raw = DurationRaw;

    fn to_raw(&self) -> Self::Raw {
        DurationRaw {
            secs: self.as_secs().to_raw(),
            nanos: self.subsec_nanos().to_raw(),
        }
    }
}

impl FromRawRepr for Duration {
    fn from_raw(raw: Self::Raw) -> Self {
        Self::new(u64::from_raw(raw.secs), u32::from_raw(raw.nanos))
    }
}

impl_try_from_rawrepr!(Duration);
impl_byte_array_via_raw!(Duration);

#[cfg(feature = "std")]
#[repr(C, packed)]
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct SystemTimeRaw {
    secs: <i64 as RawRepr>::Raw,
    nanos: <u32 as RawRepr>::Raw,
}

#[cfg(feature = "std")]
unsafe impl PlainOldData for SystemTimeRaw {}
#[cfg(feature = "std")]
impl_byte_array!(SystemTimeRaw);

#[cfg(feature = "std")]
impl RawRepr for SystemTime {
    type Raw = SystemTimeRaw;

    fn to_raw(&self) -> Self::Raw {
        match self.duration_since(std::time::SystemTime::UNIX_EPOCH) {
            Ok(d) => SystemTimeRaw {
                secs: (d.as_secs() as i64).to_raw(),
                nanos: d.subsec_nanos().to_raw(),
            },
            Err(e) => {
                let d = e.duration();
                let secs = d.as_secs();
                let nanos = d.subsec_nanos();
                if nanos == 0 {
                    SystemTimeRaw {
                        secs: (-(secs as i64)).to_raw(),
                        nanos: 0u32.to_raw(),
                    }
                } else {
                    // e.g. 0.5s before epoch: secs=0, nanos=500_000_000
                    // stored as secs=-1, nanos=500_000_000 (floor + forward fraction)
                    SystemTimeRaw {
                        secs: (-(secs as i64) - 1).to_raw(),
                        nanos: (1_000_000_000 - nanos).to_raw(),
                    }
                }
            }
        }
    }
}

#[cfg(feature = "std")]
impl FromRawRepr for SystemTime {
    fn from_raw(raw: Self::Raw) -> Self {
        let secs = i64::from_raw(raw.secs);
        let nanos = u32::from_raw(raw.nanos);
        if secs >= 0 {
            std::time::SystemTime::UNIX_EPOCH + std::time::Duration::new(secs as u64, nanos)
        } else {
            let neg_secs = (-secs) as u64;
            if nanos == 0 {
                std::time::SystemTime::UNIX_EPOCH - std::time::Duration::from_secs(neg_secs)
            } else {
                std::time::SystemTime::UNIX_EPOCH - std::time::Duration::from_secs(neg_secs)
                    + std::time::Duration::from_nanos(nanos as u64)
            }
        }
    }
}

#[cfg(feature = "std")]
impl_try_from_rawrepr!(SystemTime);
#[cfg(feature = "std")]
impl_byte_array_via_raw!(SystemTime);
