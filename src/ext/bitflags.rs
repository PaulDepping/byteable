//! [`crate::impl_bitflags!`] — opt-in [`RawRepr`](crate::RawRepr),
//! [`IntoByteArray`](crate::IntoByteArray), and endian-conversion impls for
//! a type implementing [`bitflags::Flags`] (requires the `bitflags` feature).
//!
//! The wire format is identical to the underlying `Flags::Bits` integer (little-endian by
//! default, same as a bare integer field). Decoding uses `Flags::from_bits_retain`, so bits
//! with no matching flag are preserved rather than rejected — matching `bitflags`'s own
//! philosophy, and staying forward-compatible with wire data written by a version of the
//! flags type that defines more bits than the reader knows about. Decoding is therefore
//! infallible.
//!
//! ## Why a macro instead of a blanket impl
//!
//! `impl<T: bitflags::Flags> RawRepr for T` looks natural but does not compile: Rust's
//! coherence checker treats an unconstrained blanket impl over a foreign trait as
//! potentially overlapping with every other impl in the crate (e.g. `Writable for String`),
//! since it cannot prove `bitflags::Flags` will never be implemented for `String` or `Option`
//! upstream. [`crate::impl_bitflags!`] sidesteps this by expanding to impls for the one concrete
//! named type passed to it, exactly like this crate's own internal `impl_byte_array_via_raw!`
//! does for named types such as `Ipv4Addr`.
//!
//! Once invoked for a type, that type gets fixed-size-path support
//! (`IntoByteArray`/`FromByteArray`) and, transitively through the crate's existing blanket
//! impls over `RawRepr`/`TryFromRawRepr`, `io_only` support on every enabled I/O pipeline
//! (`std`, `tokio`, `embedded-io`, `embedded-io-async`) — no per-pipeline code needed.
//!
//! ```
//! # #[cfg(feature = "bitflags")] {
//! bitflags::bitflags! {
//!     #[derive(Debug, Clone, Copy, PartialEq, Eq)]
//!     struct Perms: u8 {
//!         const READ = 0b001;
//!         const WRITE = 0b010;
//!     }
//! }
//! byteable::impl_bitflags!(Perms);
//!
//! use byteable::{IntoByteArray, FromByteArray};
//! let bytes = Perms::READ.into_byte_array();
//! assert_eq!(Perms::from_byte_array(bytes), Perms::READ);
//! # }
//! ```
//!
//! ## `u8`/`i8`-backed flags and [`crate::impl_bitflags_endian!`]
//!
//! [`crate::impl_bitflags!`] alone is enough for every field usage except one: explicit
//! `#[byteable(little_endian)]` / `#[byteable(big_endian)]` field attributes, which need
//! [`HasEndianRepr`](crate::HasEndianRepr). That trait is only implemented for multi-byte
//! integers in this crate — a bare `u8` field can't use those attributes either, since a
//! single byte has no byte order — so a flags type backed by `u8`/`i8` inherits that same
//! restriction. For a flags type backed by a multi-byte integer (`u16`/`u32`/`u64`/`u128`)
//! where you do want those attributes, additionally invoke [`crate::impl_bitflags_endian!`].

/// Implements [`RawRepr`](crate::RawRepr), [`TryFromRawRepr`](crate::TryFromRawRepr),
/// [`IntoByteArray`](crate::IntoByteArray), and [`FromByteArray`](crate::FromByteArray) for one
/// or more types implementing [`bitflags::Flags`], so they can be used as fixed-size fields
/// and (via this crate's blanket impls) on every enabled I/O pipeline. Works for any `Bits`
/// width, including `u8`/`i8`. See the [module docs](self) for why this is a macro rather
/// than a blanket impl, and [`crate::impl_bitflags_endian!`] for multi-byte-backed flags that also
/// need explicit little/big-endian field attributes.
#[macro_export]
macro_rules! impl_bitflags {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl $crate::RawRepr for $ty {
                type Raw = <<$ty as $crate::__bitflags::Flags>::Bits as $crate::RawRepr>::Raw;

                fn to_raw(&self) -> Self::Raw {
                    $crate::RawRepr::to_raw(&$crate::__bitflags::Flags::bits(self))
                }
            }

            impl $crate::FromRawRepr for $ty {
                fn from_raw(raw: Self::Raw) -> Self {
                    let bits = <<$ty as $crate::__bitflags::Flags>::Bits as $crate::FromRawRepr>::from_raw(raw);
                    <$ty as $crate::__bitflags::Flags>::from_bits_retain(bits)
                }
            }

            impl $crate::TryFromRawRepr for $ty {
                fn try_from_raw(raw: Self::Raw) -> Result<Self, $crate::DecodeError> {
                    Ok(<$ty as $crate::FromRawRepr>::from_raw(raw))
                }
            }

            impl $crate::IntoByteArray for $ty {
                type ByteArray = <<$ty as $crate::RawRepr>::Raw as $crate::IntoByteArray>::ByteArray;

                fn into_byte_array(&self) -> Self::ByteArray {
                    $crate::IntoByteArray::into_byte_array(&$crate::RawRepr::to_raw(self))
                }
            }

            impl $crate::FromByteArray for $ty {
                fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                    let raw = <<$ty as $crate::RawRepr>::Raw as $crate::FromByteArray>::from_byte_array(byte_array);
                    <$ty as $crate::FromRawRepr>::from_raw(raw)
                }
            }
        )+
    };
}

/// Additionally implements [`HasEndianRepr`](crate::HasEndianRepr) and
/// [`FromEndianRepr`](crate::FromEndianRepr) for one or more types implementing
/// [`bitflags::Flags`], enabling `#[byteable(little_endian)]` / `#[byteable(big_endian)]` on
/// fields of that type. Only compiles for flags backed by a multi-byte integer (`u16` and
/// wider) — see the [module docs](self). Requires [`crate::impl_bitflags!`] to also be invoked for
/// the same type.
#[macro_export]
macro_rules! impl_bitflags_endian {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl $crate::HasEndianRepr for $ty {
                type LE = <<$ty as $crate::__bitflags::Flags>::Bits as $crate::HasEndianRepr>::LE;
                type BE = <<$ty as $crate::__bitflags::Flags>::Bits as $crate::HasEndianRepr>::BE;

                fn to_little_endian(self) -> Self::LE {
                    $crate::HasEndianRepr::to_little_endian($crate::__bitflags::Flags::bits(&self))
                }

                fn to_big_endian(self) -> Self::BE {
                    $crate::HasEndianRepr::to_big_endian($crate::__bitflags::Flags::bits(&self))
                }
            }

            impl $crate::FromEndianRepr for $ty {
                fn from_little_endian(le: Self::LE) -> Self {
                    let bits = $crate::FromEndianRepr::from_little_endian(le);
                    <$ty as $crate::__bitflags::Flags>::from_bits_retain(bits)
                }

                fn from_big_endian(be: Self::BE) -> Self {
                    let bits = $crate::FromEndianRepr::from_big_endian(be);
                    <$ty as $crate::__bitflags::Flags>::from_bits_retain(bits)
                }
            }
        )+
    };
}
