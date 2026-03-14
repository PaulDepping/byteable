//! Byteable implementations for standard-library types: `bool`, `char`, ranges,
//! `Duration`, `SystemTime`, and network address types.
use crate::{
    ByteRepr, FromByteArray, IntoByteArray, InvalidDiscriminantError, LittleEndian, TransmuteSafe,
    TryFromByteArray, TryRawRepr,
};
use crate::{impl_byteable_via, unsafe_byteable_transmute};
use core::{
    marker::PhantomData,
    net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6},
    num::{
        NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroU8, NonZeroU16,
        NonZeroU32, NonZeroU64, NonZeroU128,
    },
    ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
    time::Duration,
};

// PhantomData<T> is a zero-sized type with no bytes.
impl<T> ByteRepr for PhantomData<T> {
    type ByteArray = [u8; 0];
}

impl<T> IntoByteArray for PhantomData<T> {
    #[inline]
    fn into_byte_array(self) -> Self::ByteArray {
        []
    }
}

impl<T> FromByteArray for PhantomData<T> {
    #[inline]
    fn from_byte_array(_byte_array: Self::ByteArray) -> Self {
        PhantomData
    }
}

// NonZero* types serialize identically to their inner type but deserialize fallibly,
// since the byte pattern 0 is invalid.
macro_rules! impl_nonzero_byteable {
    ($($nonzero:ty: $inner:ty),+ $(,)?) => {
        $(
            impl ByteRepr for $nonzero {
                type ByteArray = <$inner as ByteRepr>::ByteArray;
            }

            impl IntoByteArray for $nonzero {
                #[inline]
                fn into_byte_array(self) -> Self::ByteArray {
                    self.get().into_byte_array()
                }
            }

            impl TryFromByteArray for $nonzero {
                type Error = InvalidDiscriminantError;

                #[inline]
                fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, Self::Error> {
                    let val = <$inner>::from_byte_array(byte_array);
                    Self::new(val).ok_or_else(|| {
                        InvalidDiscriminantError::new(val, ::core::any::type_name::<Self>())
                    })
                }
            }
        )+
    };
}

impl_nonzero_byteable!(
    NonZeroU8: u8,
    NonZeroU16: u16,
    NonZeroU32: u32,
    NonZeroU64: u64,
    NonZeroU128: u128,
    NonZeroI8: i8,
    NonZeroI16: i16,
    NonZeroI32: i32,
    NonZeroI64: i64,
    NonZeroI128: i128,
);

// Generates ByteRepr, IntoByteArray, TryFromByteArray, and TryRawRepr for a type
// that delegates to a raw wrapper type via TryRawRepr.
macro_rules! impl_try_raw_byteable {
    ($type:ty, $raw:ty, $error:ty) => {
        impl ByteRepr for $type {
            type ByteArray = <<$type as TryRawRepr>::Raw as ByteRepr>::ByteArray;
        }

        impl IntoByteArray for $type {
            #[inline]
            fn into_byte_array(self) -> Self::ByteArray {
                let raw: <Self as TryRawRepr>::Raw = self.into();
                raw.into_byte_array()
            }
        }

        impl TryFromByteArray for $type {
            type Error = $error;

            #[inline]
            fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, Self::Error> {
                let raw = <Self as TryRawRepr>::Raw::from_byte_array(byte_array);
                raw.try_into()
            }
        }

        impl TryRawRepr for $type {
            type Raw = $raw;
        }
    };
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
#[doc(hidden)]
pub struct RawBool(u8);

unsafe impl TransmuteSafe for RawBool {}

unsafe_byteable_transmute!(RawBool);

impl From<bool> for RawBool {
    #[inline]
    fn from(value: bool) -> Self {
        Self(value as u8)
    }
}

impl TryFrom<RawBool> for bool {
    type Error = InvalidDiscriminantError;

    #[inline]
    fn try_from(value: RawBool) -> Result<Self, Self::Error> {
        match value.0 {
            0 => Ok(false),
            1 => Ok(true),
            invalid => Err(InvalidDiscriminantError::new(
                invalid,
                ::core::any::type_name::<Self>(),
            )),
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
#[doc(hidden)]
pub struct RawChar(LittleEndian<u32>);

unsafe impl TransmuteSafe for RawChar {}

unsafe_byteable_transmute!(RawChar);

impl From<char> for RawChar {
    #[inline]
    fn from(value: char) -> Self {
        Self((value as u32).into())
    }
}

impl TryFrom<RawChar> for char {
    type Error = InvalidDiscriminantError;

    #[inline]
    fn try_from(value: RawChar) -> Result<Self, Self::Error> {
        let num = value.0.get();
        match char::from_u32(num) {
            Some(c) => Ok(c),
            None => Err(InvalidDiscriminantError::new(
                num,
                ::core::any::type_name::<Self>(),
            )),
        }
    }
}

impl_try_raw_byteable!(bool, RawBool, InvalidDiscriminantError);
impl_try_raw_byteable!(char, RawChar, InvalidDiscriminantError);

macro_rules! impl_range_byteable {
    // Single-byte index types (u8, i8) — no endianness annotation needed.
    ($index_type:ty, $raw_name:ident) => {
        #[repr(C, packed)]
        #[derive(Clone, Copy)]
        #[doc(hidden)]
        pub struct $raw_name {
            start: $index_type,
            end: $index_type,
        }

        unsafe_byteable_transmute!($raw_name);

        impl From<Range<$index_type>> for $raw_name {
            fn from(value: Range<$index_type>) -> Self {
                Self { start: value.start, end: value.end }
            }
        }

        impl From<$raw_name> for Range<$index_type> {
            fn from(value: $raw_name) -> Self {
                Self { start: value.start, end: value.end }
            }
        }

        impl_byteable_via!(Range<$index_type> => $raw_name);
    };

    // Little-endian multi-byte index types.
    (little_endian; $index_type:ty, $raw_name:ident) => {
        #[repr(C, packed)]
        #[derive(Clone, Copy)]
        #[doc(hidden)]
        pub struct $raw_name {
            start: $crate::LittleEndian<$index_type>,
            end: $crate::LittleEndian<$index_type>,
        }

        unsafe_byteable_transmute!($raw_name);

        impl From<Range<$index_type>> for $raw_name {
            fn from(value: Range<$index_type>) -> Self {
                Self { start: value.start.into(), end: value.end.into() }
            }
        }

        impl From<$raw_name> for Range<$index_type> {
            fn from(value: $raw_name) -> Self {
                Self { start: value.start.get(), end: value.end.get() }
            }
        }

        impl_byteable_via!(Range<$index_type> => $raw_name);
    };

    // Big-endian multi-byte index types.
    (big_endian; $index_type:ty, $raw_name:ident) => {
        #[repr(C, packed)]
        #[derive(Clone, Copy)]
        #[doc(hidden)]
        pub struct $raw_name {
            start: $crate::BigEndian<$index_type>,
            end: $crate::BigEndian<$index_type>,
        }

        unsafe_byteable_transmute!($raw_name);

        impl From<Range<$index_type>> for $raw_name {
            fn from(value: Range<$index_type>) -> Self {
                Self { start: value.start.into(), end: value.end.into() }
            }
        }

        impl From<$raw_name> for Range<$index_type> {
            fn from(value: $raw_name) -> Self {
                Self { start: value.start.get(), end: value.end.get() }
            }
        }

        impl_byteable_via!(Range<$index_type> => $raw_name);
    };
}

impl_range_byteable!(u8, RangeU8);
impl_range_byteable!(i8, RangeI8);
impl_range_byteable!(little_endian; u16, RangeU16);
impl_range_byteable!(little_endian; u32, RangeU32);
impl_range_byteable!(little_endian; u64, RangeU64);
impl_range_byteable!(little_endian; u128, RangeU128);
impl_range_byteable!(little_endian; i16, RangeI16);
impl_range_byteable!(little_endian; i32, RangeI32);
impl_range_byteable!(little_endian; i64, RangeI64);
impl_range_byteable!(little_endian; i128, RangeI128);
impl_range_byteable!(little_endian; f32, RangeF32);
impl_range_byteable!(little_endian; f64, RangeF64);

// RangeInclusive<T> — same two-field layout as Range<T>, reconstructed via ::new().
macro_rules! impl_range_inclusive_byteable {
    ($index_type:ty, $raw_name:ident) => {
        #[repr(C, packed)]
        #[derive(Clone, Copy)]
        #[doc(hidden)]
        pub struct $raw_name {
            start: $index_type,
            end: $index_type,
        }

        unsafe_byteable_transmute!($raw_name);

        impl From<RangeInclusive<$index_type>> for $raw_name {
            fn from(value: RangeInclusive<$index_type>) -> Self {
                Self { start: *value.start(), end: *value.end() }
            }
        }

        impl From<$raw_name> for RangeInclusive<$index_type> {
            fn from(value: $raw_name) -> Self {
                RangeInclusive::new(value.start, value.end)
            }
        }

        impl_byteable_via!(RangeInclusive<$index_type> => $raw_name);
    };

    (little_endian; $index_type:ty, $raw_name:ident) => {
        #[repr(C, packed)]
        #[derive(Clone, Copy)]
        #[doc(hidden)]
        pub struct $raw_name {
            start: $crate::LittleEndian<$index_type>,
            end: $crate::LittleEndian<$index_type>,
        }

        unsafe_byteable_transmute!($raw_name);

        impl From<RangeInclusive<$index_type>> for $raw_name {
            fn from(value: RangeInclusive<$index_type>) -> Self {
                Self { start: (*value.start()).into(), end: (*value.end()).into() }
            }
        }

        impl From<$raw_name> for RangeInclusive<$index_type> {
            fn from(value: $raw_name) -> Self {
                RangeInclusive::new(value.start.get(), value.end.get())
            }
        }

        impl_byteable_via!(RangeInclusive<$index_type> => $raw_name);
    };

    (big_endian; $index_type:ty, $raw_name:ident) => {
        #[repr(C, packed)]
        #[derive(Clone, Copy)]
        #[doc(hidden)]
        pub struct $raw_name {
            start: $crate::BigEndian<$index_type>,
            end: $crate::BigEndian<$index_type>,
        }

        unsafe_byteable_transmute!($raw_name);

        impl From<RangeInclusive<$index_type>> for $raw_name {
            fn from(value: RangeInclusive<$index_type>) -> Self {
                Self { start: (*value.start()).into(), end: (*value.end()).into() }
            }
        }

        impl From<$raw_name> for RangeInclusive<$index_type> {
            fn from(value: $raw_name) -> Self {
                RangeInclusive::new(value.start.get(), value.end.get())
            }
        }

        impl_byteable_via!(RangeInclusive<$index_type> => $raw_name);
    };
}

impl_range_inclusive_byteable!(u8, RangeInclusiveU8);
impl_range_inclusive_byteable!(i8, RangeInclusiveI8);
impl_range_inclusive_byteable!(little_endian; u16, RangeInclusiveU16);
impl_range_inclusive_byteable!(little_endian; u32, RangeInclusiveU32);
impl_range_inclusive_byteable!(little_endian; u64, RangeInclusiveU64);
impl_range_inclusive_byteable!(little_endian; u128, RangeInclusiveU128);
impl_range_inclusive_byteable!(little_endian; i16, RangeInclusiveI16);
impl_range_inclusive_byteable!(little_endian; i32, RangeInclusiveI32);
impl_range_inclusive_byteable!(little_endian; i64, RangeInclusiveI64);
impl_range_inclusive_byteable!(little_endian; i128, RangeInclusiveI128);
impl_range_inclusive_byteable!(little_endian; f32, RangeInclusiveF32);
impl_range_inclusive_byteable!(little_endian; f64, RangeInclusiveF64);

// RangeFrom<T>, RangeTo<T>, RangeToInclusive<T> — single public field.
macro_rules! impl_range_single_byteable {
    ($std_type:ty, $field:ident, $index_type:ty, $raw_name:ident) => {
        #[repr(C, packed)]
        #[derive(Clone, Copy)]
        #[doc(hidden)]
        pub struct $raw_name {
            $field: $index_type,
        }

        unsafe_byteable_transmute!($raw_name);

        impl From<$std_type> for $raw_name {
            fn from(value: $std_type) -> Self {
                Self { $field: value.$field }
            }
        }

        impl From<$raw_name> for $std_type {
            fn from(value: $raw_name) -> Self {
                Self { $field: value.$field }
            }
        }

        impl_byteable_via!($std_type => $raw_name);
    };

    (little_endian; $std_type:ty, $field:ident, $index_type:ty, $raw_name:ident) => {
        #[repr(C, packed)]
        #[derive(Clone, Copy)]
        #[doc(hidden)]
        pub struct $raw_name {
            $field: $crate::LittleEndian<$index_type>,
        }

        unsafe_byteable_transmute!($raw_name);

        impl From<$std_type> for $raw_name {
            fn from(value: $std_type) -> Self {
                Self { $field: value.$field.into() }
            }
        }

        impl From<$raw_name> for $std_type {
            fn from(value: $raw_name) -> Self {
                Self { $field: value.$field.get() }
            }
        }

        impl_byteable_via!($std_type => $raw_name);
    };

    (big_endian; $std_type:ty, $field:ident, $index_type:ty, $raw_name:ident) => {
        #[repr(C, packed)]
        #[derive(Clone, Copy)]
        #[doc(hidden)]
        pub struct $raw_name {
            $field: $crate::BigEndian<$index_type>,
        }

        unsafe_byteable_transmute!($raw_name);

        impl From<$std_type> for $raw_name {
            fn from(value: $std_type) -> Self {
                Self { $field: value.$field.into() }
            }
        }

        impl From<$raw_name> for $std_type {
            fn from(value: $raw_name) -> Self {
                Self { $field: value.$field.get() }
            }
        }

        impl_byteable_via!($std_type => $raw_name);
    };
}

impl_range_single_byteable!(RangeFrom<u8>, start, u8, RangeFromU8);
impl_range_single_byteable!(RangeFrom<i8>, start, i8, RangeFromI8);
impl_range_single_byteable!(little_endian; RangeFrom<u16>, start, u16, RangeFromU16);
impl_range_single_byteable!(little_endian; RangeFrom<u32>, start, u32, RangeFromU32);
impl_range_single_byteable!(little_endian; RangeFrom<u64>, start, u64, RangeFromU64);
impl_range_single_byteable!(little_endian; RangeFrom<u128>, start, u128, RangeFromU128);
impl_range_single_byteable!(little_endian; RangeFrom<i16>, start, i16, RangeFromI16);
impl_range_single_byteable!(little_endian; RangeFrom<i32>, start, i32, RangeFromI32);
impl_range_single_byteable!(little_endian; RangeFrom<i64>, start, i64, RangeFromI64);
impl_range_single_byteable!(little_endian; RangeFrom<i128>, start, i128, RangeFromI128);
impl_range_single_byteable!(little_endian; RangeFrom<f32>, start, f32, RangeFromF32);
impl_range_single_byteable!(little_endian; RangeFrom<f64>, start, f64, RangeFromF64);

impl_range_single_byteable!(RangeTo<u8>, end, u8, RangeToU8);
impl_range_single_byteable!(RangeTo<i8>, end, i8, RangeToI8);
impl_range_single_byteable!(little_endian; RangeTo<u16>, end, u16, RangeToU16);
impl_range_single_byteable!(little_endian; RangeTo<u32>, end, u32, RangeToU32);
impl_range_single_byteable!(little_endian; RangeTo<u64>, end, u64, RangeToU64);
impl_range_single_byteable!(little_endian; RangeTo<u128>, end, u128, RangeToU128);
impl_range_single_byteable!(little_endian; RangeTo<i16>, end, i16, RangeToI16);
impl_range_single_byteable!(little_endian; RangeTo<i32>, end, i32, RangeToI32);
impl_range_single_byteable!(little_endian; RangeTo<i64>, end, i64, RangeToI64);
impl_range_single_byteable!(little_endian; RangeTo<i128>, end, i128, RangeToI128);
impl_range_single_byteable!(little_endian; RangeTo<f32>, end, f32, RangeToF32);
impl_range_single_byteable!(little_endian; RangeTo<f64>, end, f64, RangeToF64);

impl_range_single_byteable!(RangeToInclusive<u8>, end, u8, RangeToInclusiveU8);
impl_range_single_byteable!(RangeToInclusive<i8>, end, i8, RangeToInclusiveI8);
impl_range_single_byteable!(little_endian; RangeToInclusive<u16>, end, u16, RangeToInclusiveU16);
impl_range_single_byteable!(little_endian; RangeToInclusive<u32>, end, u32, RangeToInclusiveU32);
impl_range_single_byteable!(little_endian; RangeToInclusive<u64>, end, u64, RangeToInclusiveU64);
impl_range_single_byteable!(little_endian; RangeToInclusive<u128>, end, u128, RangeToInclusiveU128);
impl_range_single_byteable!(little_endian; RangeToInclusive<i16>, end, i16, RangeToInclusiveI16);
impl_range_single_byteable!(little_endian; RangeToInclusive<i32>, end, i32, RangeToInclusiveI32);
impl_range_single_byteable!(little_endian; RangeToInclusive<i64>, end, i64, RangeToInclusiveI64);
impl_range_single_byteable!(little_endian; RangeToInclusive<i128>, end, i128, RangeToInclusiveI128);
impl_range_single_byteable!(little_endian; RangeToInclusive<f32>, end, f32, RangeToInclusiveF32);
impl_range_single_byteable!(little_endian; RangeToInclusive<f64>, end, f64, RangeToInclusiveF64);

// RangeFull — zero-sized, serializes to [u8; 0].
#[repr(C, packed)]
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct RangeFullRaw;

unsafe_byteable_transmute!(RangeFullRaw);

impl From<RangeFull> for RangeFullRaw {
    fn from(_: RangeFull) -> Self {
        Self
    }
}

impl From<RangeFullRaw> for RangeFull {
    fn from(_: RangeFullRaw) -> Self {
        RangeFull
    }
}

impl_byteable_via!(RangeFull => RangeFullRaw);

#[repr(C, packed)]
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct DurationRaw {
    secs: LittleEndian<u64>,
    nanos: LittleEndian<u32>,
}

unsafe_byteable_transmute!(DurationRaw);

impl From<Duration> for DurationRaw {
    fn from(value: Duration) -> Self {
        Self {
            secs: value.as_secs().into(),
            nanos: value.subsec_nanos().into(),
        }
    }
}

impl From<DurationRaw> for Duration {
    fn from(value: DurationRaw) -> Self {
        Self::new(value.secs.get(), value.nanos.get())
    }
}

impl_byteable_via!(Duration =>  DurationRaw);

// SystemTime serializes as i64 (seconds since UNIX_EPOCH, negative = before epoch)
// plus u32 nanoseconds (always 0..=999_999_999, the forward fractional part within the second).
#[cfg(feature = "std")]
#[repr(C, packed)]
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct SystemTimeRaw {
    secs: LittleEndian<i64>,
    nanos: LittleEndian<u32>,
}

#[cfg(feature = "std")]
unsafe_byteable_transmute!(SystemTimeRaw);

#[cfg(feature = "std")]
impl From<std::time::SystemTime> for SystemTimeRaw {
    fn from(value: std::time::SystemTime) -> Self {
        match value.duration_since(std::time::SystemTime::UNIX_EPOCH) {
            Ok(d) => Self {
                secs: (d.as_secs() as i64).into(),
                nanos: d.subsec_nanos().into(),
            },
            Err(e) => {
                let d = e.duration();
                let secs = d.as_secs();
                let nanos = d.subsec_nanos();
                if nanos == 0 {
                    Self {
                        secs: (-(secs as i64)).into(),
                        nanos: 0u32.into(),
                    }
                } else {
                    // e.g. 0.5s before epoch: secs=0, nanos=500_000_000
                    // stored as secs=-1, nanos=500_000_000 (floor + forward fraction)
                    Self {
                        secs: (-(secs as i64) - 1).into(),
                        nanos: (1_000_000_000 - nanos).into(),
                    }
                }
            }
        }
    }
}

#[cfg(feature = "std")]
impl From<SystemTimeRaw> for std::time::SystemTime {
    fn from(value: SystemTimeRaw) -> Self {
        let secs = value.secs.get();
        let nanos = value.nanos.get();
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
impl_byteable_via!(std::time::SystemTime => SystemTimeRaw);

#[repr(C, packed)]
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct Ipv4AddrRaw {
    octets: [u8; 4],
}

unsafe_byteable_transmute!(Ipv4AddrRaw);

impl From<Ipv4Addr> for Ipv4AddrRaw {
    fn from(value: Ipv4Addr) -> Self {
        Self {
            octets: value.octets(),
        }
    }
}

impl From<Ipv4AddrRaw> for Ipv4Addr {
    fn from(value: Ipv4AddrRaw) -> Self {
        Self::from_octets(value.octets)
    }
}

impl_byteable_via!(Ipv4Addr => Ipv4AddrRaw);

#[repr(C, packed)]
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct Ipv6AddrRaw {
    octets: [u8; 16],
}

unsafe_byteable_transmute!(Ipv6AddrRaw);

impl From<Ipv6Addr> for Ipv6AddrRaw {
    fn from(value: Ipv6Addr) -> Self {
        Self {
            octets: value.octets(),
        }
    }
}

impl From<Ipv6AddrRaw> for Ipv6Addr {
    fn from(value: Ipv6AddrRaw) -> Self {
        Self::from_octets(value.octets)
    }
}

impl_byteable_via!(Ipv6Addr => Ipv6AddrRaw);

#[repr(C, packed)]
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct SocketAddrV4Raw {
    ip: [u8; 4],
    port: LittleEndian<u16>,
}

unsafe_byteable_transmute!(SocketAddrV4Raw);

impl From<SocketAddrV4> for SocketAddrV4Raw {
    fn from(value: SocketAddrV4) -> Self {
        Self {
            ip: value.ip().octets(),
            port: value.port().into(),
        }
    }
}

impl From<SocketAddrV4Raw> for SocketAddrV4 {
    fn from(value: SocketAddrV4Raw) -> Self {
        Self::new(Ipv4Addr::from_octets(value.ip), value.port.get())
    }
}

impl_byteable_via!(SocketAddrV4 => SocketAddrV4Raw);

#[repr(C, packed)]
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct SocketAddrV6Raw {
    ip: [u8; 16],
    port: LittleEndian<u16>,
    flowinfo: LittleEndian<u32>,
    scope_id: LittleEndian<u32>,
}

unsafe_byteable_transmute!(SocketAddrV6Raw);

impl From<SocketAddrV6> for SocketAddrV6Raw {
    fn from(value: SocketAddrV6) -> Self {
        Self {
            ip: value.ip().octets(),
            port: value.port().into(),
            flowinfo: value.flowinfo().into(),
            scope_id: value.scope_id().into(),
        }
    }
}

impl From<SocketAddrV6Raw> for SocketAddrV6 {
    fn from(value: SocketAddrV6Raw) -> Self {
        Self::new(
            Ipv6Addr::from_octets(value.ip),
            value.port.get(),
            value.flowinfo.get(),
            value.scope_id.get(),
        )
    }
}

impl_byteable_via!(SocketAddrV6 => SocketAddrV6Raw);
