//! [`RawRepr`], [`ToByteArray`], and [`FromByteArray`] implementations for primitive and
//! standard-library types that have a fixed, well-defined byte representation.
//!
//! Covered types: `u8`/`i8` (identity repr), multi-byte integers and floats (little-endian
//! by default), `bool` (1 byte, 0 or 1), `char` (4-byte little-endian Unicode scalar),
//! [`PhantomData<T>`](core::marker::PhantomData) (0 bytes),
//! [`NonZero<T>`](core::num::NonZero), [`Wrapping<T>`](core::num::Wrapping) /
//! [`Saturating<T>`](core::num::Saturating) (same as `T`), [`Ordering`](core::cmp::Ordering)
//! (1 byte, 0/1/2), [`Reverse<T>`](core::cmp::Reverse) (same as `T`), network address types
//! (`Ipv4Addr`, `Ipv6Addr`, `SocketAddrV4`, `SocketAddrV6`), all range variants, and
//! [`Duration`] /
//! [`SystemTime`] (`std` feature only).
//!
//! ## `SystemTime` wire format
//!
//! `SystemTime` is encoded as a **signed** `i64` seconds offset from the Unix epoch
//! followed by a `u32` sub-second nanoseconds field (always in `[0, 999_999_999]`).
//! Negative seconds represent times before 1970-01-01 00:00:00 UTC, following the
//! standard POSIX `timespec` convention.

use crate::{
    Constraint, DecodeError, Endianness, FingerprintBuilder, FingerprintTag, FromByteArray,
    FromRawRepr, LittleEndian, PlainOldData, RawRepr, Signedness, ToByteArray, TryFromByteArray,
    TryFromRawRepr, WireFingerprint, impl_byte_array,
};
use core::{
    cmp::{Ordering, Reverse},
    marker::PhantomData,
    net::Ipv4Addr,
    net::{IpAddr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    num::{NonZero, Saturating, Wrapping},
    ops::{Bound, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
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

            impl ToByteArray for $type {
                type ByteArray = [u8; ::core::mem::size_of::<$type>()];

                fn to_byte_array(&self) -> Self::ByteArray {
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
        )+
    };
}

rawrepr_self!(u8, i8);

// `u8`/`i8` are never wrapped in `BigEndian`/`LittleEndian` (a single byte has no order to
// assert), so their fingerprint has no `.endianness()` call - unlike the multi-byte
// int/float macro below, which folds `.endianness(Endianness::Little)` in on purpose. See
// `impl_fingerprint_int_endian!`'s doc comment for why.
macro_rules! impl_fingerprint_int_no_endian {
    ($signedness:expr, $($type:ty),+ $(,)?) => {
        $(
            impl WireFingerprint for $type {
                const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
                    .tag(FingerprintTag::FixedInt)
                    .width(<$type as ToByteArray>::BYTE_SIZE as u8)
                    .signedness($signedness)
                    .finish();
            }
        )+
    };
}

impl_fingerprint_int_no_endian!(Signedness::Unsigned, u8);
impl_fingerprint_int_no_endian!(Signedness::Signed, i8);

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

impl ToByteArray for bool
where
    bool: RawRepr,
    <bool as RawRepr>::Raw: ToByteArray,
{
    type ByteArray = [u8; ::core::mem::size_of::<<bool as RawRepr>::Raw>()];
    fn to_byte_array(&self) -> Self::ByteArray {
        <Self as RawRepr>::to_raw(self).to_byte_array()
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

impl WireFingerprint for bool {
    const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new().tag(FingerprintTag::Bool).finish();
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

impl ToByteArray for char
where
    char: RawRepr,
    <char as RawRepr>::Raw: ToByteArray,
{
    type ByteArray = [u8; ::core::mem::size_of::<<Self as RawRepr>::Raw>()];
    fn to_byte_array(&self) -> Self::ByteArray {
        <Self as RawRepr>::to_raw(self).to_byte_array()
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

impl WireFingerprint for char {
    const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
        .tag(FingerprintTag::FixedInt)
        .width(4)
        .signedness(Signedness::Unsigned)
        .constraint(Constraint::UnicodeScalar)
        .finish();
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

            impl ToByteArray for $type
            where
                $type: RawRepr,
                <$type as RawRepr>::Raw: ToByteArray,
            {
                type ByteArray = [u8; ::core::mem::size_of::<<Self as RawRepr>::Raw>()];
                fn to_byte_array(&self) -> Self::ByteArray {
                    <Self as RawRepr>::to_raw(self).to_byte_array()
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

// All ten of these types implement `EndianConvert` (`src/byteable_trait.rs`) and therefore
// have `BigEndian<T>`/`LittleEndian<T>` wrappers - and, per `RawRepr` above, an unattributed
// field of one of these types has `RawRepr::Raw = LittleEndian<Self>`, i.e. it is *always*
// wire-identical to an explicit `#[byteable(little_endian)]` field of the same type. So the
// plain type's own fingerprint must fold in `.endianness(Endianness::Little)` - not leave
// endianness unstated - so that `LittleEndian<T>::WIRE_FINGERPRINT` (defined in
// `src/byteable_trait.rs` as a direct delegation to `T::WIRE_FINGERPRINT`) is correct by
// construction, and so `BigEndian<T>`, which states `Endianness::Big`, actually comes out
// different. `u8`/`i8` above are the only fixed-width integers excluded from this - they
// have no endian wrapper to stay consistent with.
macro_rules! impl_fingerprint_int_endian {
    ($signedness:expr, $($type:ty),+ $(,)?) => {
        $(
            impl WireFingerprint for $type {
                const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
                    .tag(FingerprintTag::FixedInt)
                    .width(<$type as ToByteArray>::BYTE_SIZE as u8)
                    .signedness($signedness)
                    .endianness(Endianness::Little)
                    .finish();
            }
        )+
    };
}

impl_fingerprint_int_endian!(Signedness::Unsigned, u16, u32, u64, u128);
impl_fingerprint_int_endian!(Signedness::Signed, i16, i32, i64, i128);

// See `impl_fingerprint_int_endian!` above - floats are wrapped in `BigEndian`/`LittleEndian`
// the same way multi-byte integers are, for the same reason.
macro_rules! impl_fingerprint_float_endian {
    ($($type:ty),+ $(,)?) => {
        $(
            impl WireFingerprint for $type {
                const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
                    .tag(FingerprintTag::Float)
                    .width(<$type as ToByteArray>::BYTE_SIZE as u8)
                    .endianness(Endianness::Little)
                    .finish();
            }
        )+
    };
}

impl_fingerprint_float_endian!(f32, f64);

macro_rules! impl_byte_array_via_raw {
    ($($ty:ty),+) => {
        $(
            impl ToByteArray for $ty
                where $ty : RawRepr,
                      <$ty as RawRepr>::Raw : ToByteArray
            {
                type ByteArray = [u8; ::core::mem::size_of::<<$ty as RawRepr>::Raw>()];
                fn to_byte_array(&self) -> Self::ByteArray {
                    <Self as RawRepr>::to_raw(self).to_byte_array()
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
            impl ToByteArray for $ty
                where $ty : RawRepr,
                      <$ty as RawRepr>::Raw : ToByteArray
            {
                type ByteArray = [u8; ::core::mem::size_of::<<$ty as RawRepr>::Raw>()];
                fn to_byte_array(&self) -> Self::ByteArray {
                    <Self as RawRepr>::to_raw(self).to_byte_array()
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

impl<T> ToByteArray for PhantomData<T> {
    type ByteArray = [u8; 0];

    fn to_byte_array(&self) -> Self::ByteArray {
        []
    }
}

impl<T> FromByteArray for PhantomData<T> {
    fn from_byte_array(_: Self::ByteArray) -> Self {
        Self
    }
}

impl<T> WireFingerprint for PhantomData<T> {
    const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
        .tag(FingerprintTag::Struct)
        .finish();
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

macro_rules! impl_fingerprint_nonzero {
    ($($type:ty),+ $(,)?) => {
        $(
            impl WireFingerprint for core::num::NonZero<$type> {
                const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
                    .nested::<$type>()
                    .constraint(Constraint::NonZero)
                    .finish();
            }
        )+
    };
}

impl_fingerprint_nonzero!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

// Wire format: 1 byte (0 = Less, 1 = Equal, 2 = Greater).
impl RawRepr for Ordering {
    type Raw = u8;

    fn to_raw(&self) -> Self::Raw {
        match self {
            Ordering::Less => 0,
            Ordering::Equal => 1,
            Ordering::Greater => 2,
        }
    }
}

impl TryFromRawRepr for Ordering {
    fn try_from_raw(raw: Self::Raw) -> Result<Self, DecodeError> {
        match raw {
            0 => Ok(Ordering::Less),
            1 => Ok(Ordering::Equal),
            2 => Ok(Ordering::Greater),
            _ => Err(DecodeError::InvalidDiscriminant {
                raw: raw as u64,
                type_name: "Ordering",
            }),
        }
    }
}

impl_try_byte_array_via_raw!(Ordering);

impl WireFingerprint for Ordering {
    const WIRE_FINGERPRINT: u64 = {
        // 3 unit variants, tags 0/1/2 - each variant's own hash computed independently, then
        // combined order-independently via fold_unordered, matching what the derive macro's
        // codegen produces for a real 3-unit-variant enum.
        let less = FingerprintBuilder::new()
            .tag(FingerprintTag::Struct)
            .discriminant_tag(0)
            .finish();
        let equal = FingerprintBuilder::new()
            .tag(FingerprintTag::Struct)
            .discriminant_tag(1)
            .finish();
        let greater = FingerprintBuilder::new()
            .tag(FingerprintTag::Struct)
            .discriminant_tag(2)
            .finish();
        FingerprintBuilder::new()
            .tag(FingerprintTag::Enum)
            .width(1)
            .signedness(Signedness::Unsigned) // matches a `u8`-discriminant derived enum
            .endianness(Endianness::Little) // matches an unattributed derived enum's default
            .variants(&[less, equal, greater])
            .finish()
    };
}

// `Wrapping<T>` / `Saturating<T>` serialize identically to `T` - same raw repr, no invalid
// states, so decoding is infallible (mirrors `TryFromRawRepr`'s trivial wrapping for `u8..i128`
// themselves, not `NonZero<T>`'s validating one).
macro_rules! impl_int_wrapper {
    ($wrapper:ident; $($type:ty),+) => {
        $(
            impl RawRepr for $wrapper<$type> {
                type Raw = <$type as RawRepr>::Raw;

                fn to_raw(&self) -> Self::Raw {
                    self.0.to_raw()
                }
            }

            impl FromRawRepr for $wrapper<$type> {
                fn from_raw(raw: Self::Raw) -> Self {
                    $wrapper(<$type>::from_raw(raw))
                }
            }

            impl_try_from_rawrepr!($wrapper<$type>);

            impl_byte_array_via_raw!($wrapper<$type>);
        )+
    };
}

impl_int_wrapper!(Wrapping; u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
impl_int_wrapper!(Saturating; u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

// Transparent passthroughs, exactly like `Reverse<T>` below: `Wrapping<T>`/`Saturating<T>`
// put the identical bytes on the wire as a bare `T`, so they must fingerprint identically
// too. Stated generically rather than per-int-type (unlike the `RawRepr` impls above, which
// the macro has to spell out concretely) - a `WireFingerprint` for a `T` that has no wire
// impls at all is inert, so there is nothing to be gained from narrowing it.
impl<T: WireFingerprint> WireFingerprint for Wrapping<T> {
    const WIRE_FINGERPRINT: u64 = <T as WireFingerprint>::WIRE_FINGERPRINT;
}

impl<T: WireFingerprint> WireFingerprint for Saturating<T> {
    const WIRE_FINGERPRINT: u64 = <T as WireFingerprint>::WIRE_FINGERPRINT;
}

// `Reverse<T>` serializes identically to `T` (transparent passthrough, same as the
// `Arc`/`Rc`/`Box` treatment in `std_types.rs`). Generic over any `T`, so - unlike
// `Wrapping`/`Saturating` above - it cannot be given the `ToByteArray`/`FromByteArray` fixed
// byte-array API (that requires a concrete `$ty:ty` per `impl_byte_array_via_raw!` invocation);
// it still gets `Readable`/`Writable` (and the `tokio`/`eio`/`eio_async` counterparts) for free
// via the blanket `RawRepr`/`TryFromRawRepr` → `Fixed*` → `*able` chains in each pipeline module.
impl<T: RawRepr> RawRepr for Reverse<T> {
    type Raw = T::Raw;

    fn to_raw(&self) -> Self::Raw {
        self.0.to_raw()
    }
}

impl<T: FromRawRepr> FromRawRepr for Reverse<T> {
    fn from_raw(raw: Self::Raw) -> Self {
        Reverse(T::from_raw(raw))
    }
}

impl<T: TryFromRawRepr> TryFromRawRepr for Reverse<T> {
    fn try_from_raw(raw: Self::Raw) -> Result<Self, DecodeError> {
        Ok(Reverse(T::try_from_raw(raw)?))
    }
}

impl<T: WireFingerprint> WireFingerprint for Reverse<T> {
    const WIRE_FINGERPRINT: u64 = <T as WireFingerprint>::WIRE_FINGERPRINT;
}

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

impl WireFingerprint for Ipv4Addr {
    const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
        .tag(FingerprintTag::FixedInt)
        .width(<Ipv4Addr as ToByteArray>::BYTE_SIZE as u8) // = 4
        .signedness(Signedness::Unsigned)
        .finish();
}

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

impl WireFingerprint for Ipv6Addr {
    const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
        .tag(FingerprintTag::FixedInt)
        .width(<Ipv6Addr as ToByteArray>::BYTE_SIZE as u8) // = 16
        .signedness(Signedness::Unsigned)
        .finish();
}

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

impl WireFingerprint for SocketAddrV4 {
    const WIRE_FINGERPRINT: u64 =
        <(Ipv4Addr, LittleEndian<u16>) as WireFingerprint>::WIRE_FINGERPRINT;
}

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

impl WireFingerprint for SocketAddrV6 {
    const WIRE_FINGERPRINT: u64 = <(
        Ipv6Addr,
        LittleEndian<u16>,
        LittleEndian<u32>,
        LittleEndian<u32>,
    ) as WireFingerprint>::WIRE_FINGERPRINT;
}

macro_rules! impl_range_byteable {
    // Single-byte index types (u8, i8) - no endianness annotation needed.
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
    // Single-byte index types (u8, i8) - no endianness annotation needed.
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

// RangeFrom<T>, RangeTo<T>, RangeToInclusive<T> - single public field.
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

impl WireFingerprint for RangeFull {
    const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
        .tag(FingerprintTag::Struct)
        .finish();
}

impl<T: WireFingerprint> WireFingerprint for Range<T> {
    const WIRE_FINGERPRINT: u64 = <(T, T) as WireFingerprint>::WIRE_FINGERPRINT;
}

impl<T: WireFingerprint> WireFingerprint for RangeInclusive<T> {
    const WIRE_FINGERPRINT: u64 = <(T, T) as WireFingerprint>::WIRE_FINGERPRINT;
}

impl<T: WireFingerprint> WireFingerprint for RangeFrom<T> {
    const WIRE_FINGERPRINT: u64 = <T as WireFingerprint>::WIRE_FINGERPRINT;
}

impl<T: WireFingerprint> WireFingerprint for RangeTo<T> {
    const WIRE_FINGERPRINT: u64 = <T as WireFingerprint>::WIRE_FINGERPRINT;
}

impl<T: WireFingerprint> WireFingerprint for RangeToInclusive<T> {
    const WIRE_FINGERPRINT: u64 = <T as WireFingerprint>::WIRE_FINGERPRINT;
}

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

impl WireFingerprint for Duration {
    const WIRE_FINGERPRINT: u64 = <(u64, u32) as WireFingerprint>::WIRE_FINGERPRINT;
}

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

#[cfg(feature = "std")]
impl WireFingerprint for SystemTime {
    const WIRE_FINGERPRINT: u64 = <(i64, u32) as WireFingerprint>::WIRE_FINGERPRINT;
}

// ---------------------------------------------------------------------------------------
// Fingerprints for dynamic-path core types
// ---------------------------------------------------------------------------------------
//
// The `Readable`/`Writable` impls for the types below are *not* in this module - they live
// once per I/O pipeline (`std_types.rs` for `std`, `eio.rs` for `embedded-io`,
// `std_types_async.rs` for `tokio`, `eio_async.rs` for `embedded-io-async`), because they
// stream rather than transmute. Their *fingerprints* are pipeline-independent pure consts,
// and none of these types needs `std` or even `alloc` to exist, so they are stated here,
// unconditionally, exactly once.
//
// This module must stay unconditional (not gated behind `std`): `SocketAddrV4`,
// `SocketAddrV6`, `Range`, `RangeInclusive` and `Duration` above all define their fingerprints
// in terms of a *tuple* fingerprint, and this module (`core_types.rs`) is always compiled.
// Gating a tuple fingerprint behind `std` would break every `no_std` build that uses one of
// those types. Anything genuinely needing the heap (`Vec`, `String`, `BTreeMap`, ...) is
// fingerprinted in `alloc_types.rs` instead, and the `std`-only leftovers (`HashMap`,
// `PathBuf`, `CString`, `Arc`, ...) stay in `std_types.rs`.

// Wire format: no tag or length prefix - arity is fixed at compile time, so each element is
// just serialized in order, making a tuple wire-identical to a struct of the same fields.
// Arity 1 through 12, matching the `Readable`/`Writable` tuple impls in every pipeline.
macro_rules! impl_fingerprint_tuple {
    ($($T:ident),+) => {
        impl<$($T: WireFingerprint),+> WireFingerprint for ($($T,)+) {
            const WIRE_FINGERPRINT: u64 = {
                let b = FingerprintBuilder::new().tag(FingerprintTag::Struct);
                $( let b = b.nested::<$T>(); )+
                b.finish()
            };
        }
    };
}

impl_fingerprint_tuple!(A);
impl_fingerprint_tuple!(A, B);
impl_fingerprint_tuple!(A, B, C);
impl_fingerprint_tuple!(A, B, C, D);
impl_fingerprint_tuple!(A, B, C, D, E);
impl_fingerprint_tuple!(A, B, C, D, E, F);
impl_fingerprint_tuple!(A, B, C, D, E, F, G);
impl_fingerprint_tuple!(A, B, C, D, E, F, G, H);
impl_fingerprint_tuple!(A, B, C, D, E, F, G, H, I);
impl_fingerprint_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_fingerprint_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_fingerprint_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);

// `u64` byte length prefix + UTF-8 bytes. `String` (in `alloc_types.rs`) delegates here.
impl WireFingerprint for str {
    const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
        .tag(FingerprintTag::Sequence)
        .width(8) // u64 length prefix
        .constraint(Constraint::Utf8)
        .nested::<u8>()
        .finish();
}

impl<T: WireFingerprint> WireFingerprint for Option<T> {
    const WIRE_FINGERPRINT: u64 = {
        // Matches exactly what the derive macro produces for a real 2-variant enum: each
        // variant's own hash (tag value + its own fields) computed independently, then
        // combined order-independently via fold_unordered - not a sequential FNV chain. Uses
        // the same primitive a real derived enum uses, rather than hand-simulating what that
        // codegen would produce.
        let none = FingerprintBuilder::new()
            .tag(FingerprintTag::Struct)
            .discriminant_tag(0) // None's tag value
            .finish();
        let some = FingerprintBuilder::new()
            .tag(FingerprintTag::Struct)
            .discriminant_tag(1) // Some's tag value
            .nested::<T>()
            .finish();
        FingerprintBuilder::new()
            .tag(FingerprintTag::Enum)
            .width(1) // 1-byte tag per the wire format table
            .signedness(Signedness::Unsigned)
            .endianness(Endianness::Little) // matches an unattributed derived enum's default
            .variants(&[none, some])
            .finish()
    };
}

impl<V: WireFingerprint, E: WireFingerprint> WireFingerprint for Result<V, E> {
    const WIRE_FINGERPRINT: u64 = {
        let ok = FingerprintBuilder::new()
            .tag(FingerprintTag::Struct)
            .discriminant_tag(0) // Ok's tag value
            .nested::<V>()
            .finish();
        let err = FingerprintBuilder::new()
            .tag(FingerprintTag::Struct)
            .discriminant_tag(1) // Err's tag value
            .nested::<E>()
            .finish();
        FingerprintBuilder::new()
            .tag(FingerprintTag::Enum)
            .width(1)
            .signedness(Signedness::Unsigned)
            .endianness(Endianness::Little)
            .variants(&[ok, err])
            .finish()
    };
}

// Wire format: 1-byte tag, 0 = Included(T), 1 = Excluded(T), 2 = Unbounded - taken from this
// crate's own `Readable`/`Writable` impls for `Bound`, which are what define the wire format
// (and from the wire-format table in `src/std_types.rs`'s module docs), not from anything
// about `core::ops::Bound`'s own declaration.
impl<T: WireFingerprint> WireFingerprint for Bound<T> {
    const WIRE_FINGERPRINT: u64 = {
        let included = FingerprintBuilder::new()
            .tag(FingerprintTag::Struct)
            .discriminant_tag(0)
            .nested::<T>()
            .finish();
        let excluded = FingerprintBuilder::new()
            .tag(FingerprintTag::Struct)
            .discriminant_tag(1)
            .nested::<T>()
            .finish();
        let unbounded = FingerprintBuilder::new()
            .tag(FingerprintTag::Struct)
            .discriminant_tag(2)
            .finish();
        FingerprintBuilder::new()
            .tag(FingerprintTag::Enum)
            .width(1)
            .signedness(Signedness::Unsigned)
            .endianness(Endianness::Little)
            .variants(&[included, excluded, unbounded])
            .finish()
    };
}

impl WireFingerprint for IpAddr {
    const WIRE_FINGERPRINT: u64 = {
        let v4 = FingerprintBuilder::new()
            .tag(FingerprintTag::Struct)
            .discriminant_tag(0)
            .nested::<Ipv4Addr>()
            .finish();
        let v6 = FingerprintBuilder::new()
            .tag(FingerprintTag::Struct)
            .discriminant_tag(1)
            .nested::<Ipv6Addr>()
            .finish();
        FingerprintBuilder::new()
            .tag(FingerprintTag::Enum)
            .width(1)
            .signedness(Signedness::Unsigned)
            .endianness(Endianness::Little)
            .variants(&[v4, v6])
            .finish()
    };
}

impl WireFingerprint for SocketAddr {
    const WIRE_FINGERPRINT: u64 = {
        let v4 = FingerprintBuilder::new()
            .tag(FingerprintTag::Struct)
            .discriminant_tag(0)
            .nested::<SocketAddrV4>()
            .finish();
        let v6 = FingerprintBuilder::new()
            .tag(FingerprintTag::Struct)
            .discriminant_tag(1)
            .nested::<SocketAddrV6>()
            .finish();
        FingerprintBuilder::new()
            .tag(FingerprintTag::Enum)
            .width(1)
            .signedness(Signedness::Unsigned)
            .endianness(Endianness::Little)
            .variants(&[v4, v6])
            .finish()
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn distinct_primitive_types_have_distinct_fingerprints() {
        use crate::WireFingerprint;
        assert_ne!(u8::WIRE_FINGERPRINT, u16::WIRE_FINGERPRINT);
        assert_ne!(u32::WIRE_FINGERPRINT, i32::WIRE_FINGERPRINT); // same width, different Signedness
        assert_ne!(u32::WIRE_FINGERPRINT, f32::WIRE_FINGERPRINT); // same width, different tag
        assert_ne!(bool::WIRE_FINGERPRINT, u8::WIRE_FINGERPRINT);
    }

    #[test]
    fn same_primitive_type_is_deterministic() {
        use crate::WireFingerprint;
        assert_eq!(u64::WIRE_FINGERPRINT, u64::WIRE_FINGERPRINT);
    }

    #[test]
    fn char_differs_from_u32() {
        use crate::WireFingerprint;
        assert_ne!(char::WIRE_FINGERPRINT, u32::WIRE_FINGERPRINT);
    }

    #[test]
    fn nonzero_differs_from_its_inner_type() {
        use crate::WireFingerprint;
        use core::num::NonZero;
        assert_ne!(
            <NonZero<u32> as WireFingerprint>::WIRE_FINGERPRINT,
            u32::WIRE_FINGERPRINT
        );
    }

    #[test]
    fn nonzero_of_different_widths_differ() {
        use crate::WireFingerprint;
        use core::num::NonZero;
        assert_ne!(
            <NonZero<u32> as WireFingerprint>::WIRE_FINGERPRINT,
            <NonZero<u64> as WireFingerprint>::WIRE_FINGERPRINT
        );
    }

    #[test]
    fn ordering_is_enum_shaped_and_distinct_from_u8() {
        use crate::WireFingerprint;
        assert_ne!(core::cmp::Ordering::WIRE_FINGERPRINT, u8::WIRE_FINGERPRINT);
    }

    #[test]
    fn transparent_wrappers_match_their_inner_type() {
        use crate::WireFingerprint;
        use core::cmp::Reverse;
        assert_eq!(
            <Reverse<u32> as WireFingerprint>::WIRE_FINGERPRINT,
            u32::WIRE_FINGERPRINT
        );
    }

    #[test]
    fn ipv4_and_ipv6_differ() {
        use crate::WireFingerprint;
        use std::net::{Ipv4Addr, Ipv6Addr};
        assert_ne!(Ipv4Addr::WIRE_FINGERPRINT, Ipv6Addr::WIRE_FINGERPRINT);
    }

    #[test]
    fn duration_is_struct_shaped() {
        use crate::WireFingerprint;
        use std::time::Duration;
        assert_eq!(
            Duration::WIRE_FINGERPRINT,
            <(u64, u32) as WireFingerprint>::WIRE_FINGERPRINT
        );
    }
}
