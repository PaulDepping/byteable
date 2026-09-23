//! Compile-time wire-shape fingerprinting: a `u64` hash computed purely from a type's exact
//! wire encoding (field types and order, endianness, discriminant width and signedness,
//! per-variant tags and payloads), used to detect accidental wire-format drift. See
//! [`WireFingerprint`] and [`FingerprintBuilder`].

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

const fn fnv1a(bytes: &[u8], mut hash: u64) -> u64 {
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
}

/// Combines a set of already-computed hashes without regard to their order. Used to combine
/// an enum's per-variant hashes: declaration order must not affect the result when tags are
/// pinned, but tag values are opaque expressions the derive macro can't sort by. `wrapping_add`
/// rather than XOR: both are commutative/associative/const-fn-safe, but addition has better
/// cross-bit diffusion (carries propagate) - the same reason `java.util.Set`'s `hashCode()`
/// contract sums element hashes rather than XORing them. Safe against the one degenerate case
/// a commutative combine usually has to worry about (two equal entries cancelling/merging):
/// two variants sharing a tag is already a hard compile error via the generated shadow-enum
/// duplicate check, so no two entries folded together can ever be identical.
#[doc(hidden)]
pub const fn fold_unordered(hashes: &[u64]) -> u64 {
    let mut acc = 0u64;
    let mut i = 0;
    while i < hashes.len() {
        acc = acc.wrapping_add(hashes[i]);
        i += 1;
    }
    acc
}

/// A type's wire shape, as a compile-time-computed fingerprint. See the module docs.
pub trait WireFingerprint {
    const WIRE_FINGERPRINT: u64;
}

/// What *kind* of thing a fingerprint describes - the first value folded into every hash.
///
/// This separates shapes that would otherwise be indistinguishable from their remaining
/// parameters alone: a 4-byte integer and a 4-byte float both fold a width of 4, and a
/// length-prefixed sequence and a two-field struct both fold a nested element hash.
///
/// The numbers in [`FingerprintTag::wire_value`] are the contract, not declaration order -
/// see this type's variants for the rules on adding and retiring kinds.
#[derive(Clone, Copy)]
pub enum FingerprintTag {
    FixedInt,
    Float,
    Bool,
    Sequence,
    Struct,
    Enum,
    // Append new kinds anywhere in this list - declaration order has no effect on wire
    // values; only wire_value()'s match arms are load-bearing. A retired kind's number is
    // never reassigned once it has actually shipped; the next new case, if any, simply takes 6.
}

impl FingerprintTag {
    const fn wire_value(self) -> u8 {
        match self {
            Self::FixedInt => 0,
            Self::Float => 1,
            Self::Bool => 2,
            Self::Sequence => 3,
            Self::Struct => 4,
            Self::Enum => 5,
        }
    }
}

/// Whether a fixed-width integer (or an enum discriminant) is signed.
///
/// Folded for every [`FingerprintTag::FixedInt`], and for an enum's discriminant, so that two
/// types differing only in signedness - `u32` vs `i32`, or `#[byteable(discriminant = u8)]` vs
/// `#[byteable(discriminant = i8)]` - do not collide, even though they put identical bytes on
/// the wire and accept identical bytes back.
#[derive(Clone, Copy)]
pub enum Signedness {
    Unsigned,
    Signed,
}

impl Signedness {
    const fn wire_value(self) -> u8 {
        match self {
            Self::Unsigned => 0,
            Self::Signed => 1,
        }
    }
}

/// The byte order of a multi-byte scalar on the wire.
///
/// Folded by every multi-byte integer and float, and by an enum's discriminant. Note that a
/// plain `u32` folds [`Endianness::Little`] rather than leaving endianness unstated: an
/// unattributed field is *always* wire-identical to an explicit `#[byteable(little_endian)]`
/// one, so `LittleEndian<u32>` and `u32` must - and do - fingerprint identically, while
/// `BigEndian<u32>` differs. See `impl_fingerprint_int_endian!` in `src/core_types.rs`.
#[derive(Clone, Copy)]
pub enum Endianness {
    Little,
    Big,
}

impl Endianness {
    const fn wire_value(self) -> u8 {
        match self {
            Self::Little => 0,
            Self::Big => 1,
        }
    }
}

/// UnicodeScalar and Utf8 are not redundant: UnicodeScalar is a value-range check on one
/// fixed-width integer (paired with FixedInt, for `char`); Utf8 is a grammar check over a
/// variable-length byte sequence (paired with Sequence, for String/str).
#[derive(Clone, Copy)]
pub enum Constraint {
    None,
    NonZero,
    NotNan,
    UnicodeScalar,
    Utf8,
}

impl Constraint {
    const fn wire_value(self) -> u8 {
        match self {
            Self::None => 0,
            Self::NonZero => 1,
            Self::NotNan => 2,
            Self::UnicodeScalar => 3,
            Self::Utf8 => 4,
        }
    }
}

/// Which [`FingerprintBuilder`] method produced the value(s) that follow it - folded as a
/// single byte before every method's own payload, so that two different methods can never
/// produce an identical byte stream even when passed numerically identical arguments (e.g.
/// `.width(1)` and `.constraint(Constraint::NonZero)` would otherwise both fold the same
/// underlying byte, since [`Width`](Self::Width)'s and [`Constraint`](Self::Constraint)'s
/// payloads share the same value space).
///
/// The numbers in [`FingerprintSlot::wire_value`] are the contract, not declaration order -
/// same rules as [`FingerprintTag`]: append new kinds anywhere in this list, never reassign a
/// retired kind's number.
#[derive(Clone, Copy)]
enum FingerprintSlot {
    Tag,
    Width,
    Signedness,
    Endianness,
    Constraint,
    LenBound,
    Nested,
    DiscriminantTag,
    Variants,
    // Append new kinds anywhere in this list - declaration order has no effect on wire
    // values; only wire_value()'s match arms are load-bearing. A retired kind's number is
    // never reassigned once it has actually shipped; the next new case, if any, simply takes 9.
}

impl FingerprintSlot {
    const fn wire_value(self) -> u8 {
        match self {
            Self::Tag => 0,
            Self::Width => 1,
            Self::Signedness => 2,
            Self::Endianness => 3,
            Self::Constraint => 4,
            Self::LenBound => 5,
            Self::Nested => 6,
            Self::DiscriminantTag => 7,
            Self::Variants => 8,
        }
    }
}

/// Builds a [`WireFingerprint`] value by folding a type's wire shape into an FNV-1a hash.
///
/// Every method is `const fn` and takes `self` by value, so a whole fingerprint is one
/// chained expression usable directly in a `const WIRE_FINGERPRINT: u64 = ...;`:
///
/// ```rust
/// use byteable::{Endianness, FingerprintBuilder, FingerprintTag, Signedness};
///
/// const MY_U32: u64 = FingerprintBuilder::new()
///     .tag(FingerprintTag::FixedInt)
///     .width(4)
///     .signedness(Signedness::Unsigned)
///     .endianness(Endianness::Little)
///     .finish();
/// assert_eq!(MY_U32, <u32 as byteable::WireFingerprint>::WIRE_FINGERPRINT);
/// ```
///
/// **The fold is order-sensitive**, so two impls describing the same shape must fold the same
/// values in the same sequence to agree. That matters for the hand-written impls in
/// `core_types.rs` (`Option`, `Result`, `Bound`, `IpAddr`, `SocketAddr`, `Ordering`), which
/// have to match what the derive macro emits for an equivalent hand-written enum - see
/// `option_matches_an_equivalent_hand_derived_enum` in `tests/derive_enums.rs`, which is the
/// cross-check that they still do.
///
/// Fold only what the wire actually depends on. A distinction with no byte-level consequence
/// (a field's Rust name, a type's own name, a `#[byteable(order = ..)]` attribute as opposed
/// to the resulting order) must not be folded, or wire-compatible types stop comparing equal.
pub struct FingerprintBuilder(u64);

impl FingerprintBuilder {
    /// Starts a new fingerprint at the FNV-1a offset basis.
    pub const fn new() -> Self {
        Self(FNV_OFFSET_BASIS)
    }

    /// Folds arbitrary bytes into the hash. The one place `fnv1a` is actually called - every
    /// other method on this builder goes through here.
    const fn fold_bytes(self, bytes: &[u8]) -> Self {
        Self(fnv1a(bytes, self.0))
    }

    /// Folds a `u64`'s little-endian bytes. A thin, no-op-cost convenience over
    /// [`fold_bytes`](Self::fold_bytes) for the several methods below whose payload is
    /// genuinely `u64`-wide (a nested fingerprint, a length bound, a variant count).
    const fn fold_u64(self, v: u64) -> Self {
        self.fold_bytes(&v.to_le_bytes())
    }

    /// Folds in which method is about to fold a value, as a single byte, before that value
    /// itself - see [`FingerprintSlot`] on why this is what keeps two different methods from
    /// ever producing an identical byte stream.
    const fn fold_slot(self, s: FingerprintSlot) -> Self {
        self.fold_bytes(&[s.wire_value()])
    }

    /// Folds in what kind of shape this is. Conventionally the first call on a builder.
    pub const fn tag(self, t: FingerprintTag) -> Self {
        self.fold_slot(FingerprintSlot::Tag)
            .fold_bytes(&[t.wire_value()])
    }

    /// Folds in another type's whole fingerprint - a struct field, a sequence element, an
    /// enum variant's payload.
    pub const fn nested<T: WireFingerprint>(self) -> Self {
        self.fold_slot(FingerprintSlot::Nested)
            .fold_u64(T::WIRE_FINGERPRINT)
    }

    /// Folds in whether a fixed-width integer (or enum discriminant) is signed.
    pub const fn signedness(self, s: Signedness) -> Self {
        self.fold_slot(FingerprintSlot::Signedness)
            .fold_bytes(&[s.wire_value()])
    }

    /// Folds in a multi-byte scalar's byte order. See [`Endianness`] on why a plain `u32`
    /// states `Little` rather than leaving this out.
    pub const fn endianness(self, e: Endianness) -> Self {
        self.fold_slot(FingerprintSlot::Endianness)
            .fold_bytes(&[e.wire_value()])
    }

    /// Folds in a validity constraint decoding enforces beyond the raw bytes - the
    /// "acceptance" half of "bytes + acceptance define sameness". `NonZero<u32>` and `u32`
    /// write identical bytes but do not accept identical bytes, so they must not collide.
    pub const fn constraint(self, c: Constraint) -> Self {
        self.fold_slot(FingerprintSlot::Constraint)
            .fold_bytes(&[c.wire_value()])
    }

    /// Folds in a width in *bytes*: a scalar's own size, or the size of a sequence's length
    /// prefix or an enum's discriminant.
    pub const fn width(self, bytes: u8) -> Self {
        self.fold_slot(FingerprintSlot::Width).fold_bytes(&[bytes])
    }

    /// Only call this when a real bound exists (e.g. a capacity-bounded collection).
    /// Unbounded types (`Vec`, `String`, ...) must never call it, not even with a sentinel
    /// like `u64::MAX`: a capacity-bounded collection could legally declare a bound of
    /// `u64::MAX`, which would then collide with the sentinel and fingerprint identically to
    /// an actually-unbounded type despite the two having different acceptance semantics.
    pub const fn len_bound(self, bound: u64) -> Self {
        self.fold_slot(FingerprintSlot::LenBound).fold_u64(bound)
    }

    /// Folds in an enum variant's (or enum's own) wire discriminant, as its raw little-endian
    /// bytes at full `u128` width - wide enough for any discriminant this crate supports,
    /// including `#[byteable(discriminant = u128)]`. Pass the discriminant's actual wire bit
    /// pattern, normalized through its own unsigned representation rather than a
    /// sign-extended reinterpretation of it - `-1i8` folds as `0xFF`, matching what the
    /// encoder really writes, not as `0xFFFF_FFFF_FFFF_FFFF`.
    pub const fn discriminant_tag(self, v: u128) -> Self {
        self.fold_slot(FingerprintSlot::DiscriminantTag)
            .fold_bytes(&v.to_le_bytes())
    }

    /// Folds in an enum's per-variant hashes: their order-independent combination via
    /// [`fold_unordered`], then the variant count. The two are folded together because every
    /// caller needs both, right after computing each variant's own hash in the same order -
    /// see `fold_unordered`'s doc comment for why both the combination and the count matter.
    pub const fn variants(self, hashes: &[u64]) -> Self {
        self.fold_slot(FingerprintSlot::Variants)
            .fold_u64(fold_unordered(hashes))
            .fold_u64(hashes.len() as u64)
    }

    /// Returns the finished fingerprint.
    pub const fn finish(self) -> u64 {
        self.0
    }
}

impl Default for FingerprintBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[diagnostic::on_unimplemented(
    message = "byteable: wire-format fingerprint mismatch (asserted {EXPECTED}, actual {ACTUAL})",
    label = "asserted fingerprint {EXPECTED} does not match this type's actual fingerprint {ACTUAL}",
    note = "if this change was intentional, replace the asserted value with {ACTUAL}"
)]
#[doc(hidden)]
pub trait FingerprintEq<const EXPECTED: u64, const ACTUAL: u64> {}

#[doc(hidden)]
impl<const N: u64> FingerprintEq<N, N> for () {}

#[doc(hidden)]
pub const fn assert_fingerprint<const EXPECTED: u64, const ACTUAL: u64>()
where
    (): FingerprintEq<EXPECTED, ACTUAL>,
{
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_inputs_produce_same_hash() {
        let a = FingerprintBuilder::new()
            .tag(FingerprintTag::FixedInt)
            .width(4)
            .finish();
        let b = FingerprintBuilder::new()
            .tag(FingerprintTag::FixedInt)
            .width(4)
            .finish();
        assert_eq!(a, b);
    }

    #[test]
    fn different_tags_produce_different_hashes() {
        let a = FingerprintBuilder::new()
            .tag(FingerprintTag::FixedInt)
            .finish();
        let b = FingerprintBuilder::new()
            .tag(FingerprintTag::Float)
            .finish();
        assert_ne!(a, b);
    }

    #[test]
    fn different_methods_with_equal_values_do_not_collide() {
        // Width(1) and Constraint(NonZero) fold the same underlying byte (1), but are
        // different methods and must not fingerprint identically - this is the whole reason
        // FingerprintSlot exists.
        let a = FingerprintBuilder::new().width(1).finish();
        let b = FingerprintBuilder::new()
            .constraint(Constraint::NonZero)
            .finish();
        assert_ne!(a, b);
    }

    #[test]
    fn discriminant_tag_distinguishes_high_and_low_bits() {
        // Pins the bug discriminant_tag exists to prevent (see variant_fingerprint_expr's doc
        // comment in byteable_derive): folding only the low 64 bits would make discriminant 1
        // and discriminant 1 + 2^64 fingerprint identically, despite genuinely different
        // wire bytes under a u128 discriminant.
        let a = FingerprintBuilder::new().discriminant_tag(1).finish();
        let b = FingerprintBuilder::new()
            .discriminant_tag(1u128 + (1u128 << 64))
            .finish();
        assert_ne!(a, b);
    }

    #[test]
    fn fingerprint_slot_wire_values_are_stable_by_construction() {
        // Canary, matching fingerprint_tag_wire_values_are_stable_by_construction below: pins
        // the slot contract as literals so a "cleanup" to `self as u8` shows up as a diff.
        assert_eq!(FingerprintSlot::Tag.wire_value(), 0);
        assert_eq!(FingerprintSlot::Width.wire_value(), 1);
        assert_eq!(FingerprintSlot::Signedness.wire_value(), 2);
        assert_eq!(FingerprintSlot::Endianness.wire_value(), 3);
        assert_eq!(FingerprintSlot::Constraint.wire_value(), 4);
        assert_eq!(FingerprintSlot::LenBound.wire_value(), 5);
        assert_eq!(FingerprintSlot::Nested.wire_value(), 6);
        assert_eq!(FingerprintSlot::DiscriminantTag.wire_value(), 7);
        assert_eq!(FingerprintSlot::Variants.wire_value(), 8);
        // 9 isn't reserved for anything yet - it's simply the next available value if a new
        // slot is ever added.
    }

    #[test]
    fn different_widths_produce_different_hashes() {
        let a = FingerprintBuilder::new()
            .tag(FingerprintTag::FixedInt)
            .width(4)
            .finish();
        let b = FingerprintBuilder::new()
            .tag(FingerprintTag::FixedInt)
            .width(8)
            .finish();
        assert_ne!(a, b);
    }

    #[test]
    fn fingerprint_tag_wire_values_are_stable_by_construction() {
        // Not a behavior test so much as a canary: if someone "cleans up" FingerprintTag
        // by converting wire_value() to `self as u8`, this test's literals stop being the
        // actual contract and silently drift with enum declaration order. Pinning them
        // here as literals makes that kind of accidental change show up as a diff.
        assert_eq!(FingerprintTag::FixedInt.wire_value(), 0);
        assert_eq!(FingerprintTag::Float.wire_value(), 1);
        assert_eq!(FingerprintTag::Bool.wire_value(), 2);
        assert_eq!(FingerprintTag::Sequence.wire_value(), 3);
        assert_eq!(FingerprintTag::Struct.wire_value(), 4);
        assert_eq!(FingerprintTag::Enum.wire_value(), 5);
        // 6 isn't reserved for anything yet - it's simply the next available value if a new
        // case is ever added.
    }

    #[test]
    fn fold_unordered_is_order_independent() {
        assert_eq!(fold_unordered(&[1, 2, 3]), fold_unordered(&[3, 1, 2]));
    }

    #[test]
    fn fold_unordered_distinguishes_different_sets() {
        assert_ne!(fold_unordered(&[1, 2, 3]), fold_unordered(&[1, 2, 4]));
    }

    #[test]
    fn fold_unordered_empty_is_zero() {
        assert_eq!(fold_unordered(&[]), 0);
    }
}

/// Compile-time coverage check: every type in the crate's documented wire-format table (see
/// the README's "Wire Format Reference") must implement [`WireFingerprint`].
///
/// This is a `const` item, not a `#[cfg(test)]` test, on purpose: it is therefore compiled in
/// **every** feature configuration, including `--no-default-features`. A `#[test]` would only
/// ever have run under `std`, which is exactly how the gaps it guards against got in - `Bound<T>`,
/// `Wrapping<T>` and `Saturating<T>` were missing outright, and the impls for tuples, `str`,
/// `Option`, `Result`, `IpAddr` and `SocketAddr` were written into a `std`-gated module even
/// though their wire impls are reachable with neither `std` nor `alloc`.
///
/// Each entry is gated to match where that type's own wire impls live, so this file is also
/// the executable statement of which types are expected to be fingerprintable where. Consts
/// produce no code, so this costs nothing at runtime.
mod wire_format_table_coverage {
    use super::WireFingerprint;
    use crate::{BigEndian, LittleEndian};

    const fn covered<T: WireFingerprint + ?Sized>() {}

    // Available in every configuration - no `std`, no `alloc`, no I/O feature.
    const _: () = {
        covered::<u8>();
        covered::<i8>();
        covered::<u16>();
        covered::<u32>();
        covered::<u64>();
        covered::<u128>();
        covered::<i16>();
        covered::<i32>();
        covered::<i64>();
        covered::<i128>();
        covered::<f32>();
        covered::<f64>();
        covered::<bool>();
        covered::<char>();
        covered::<BigEndian<u32>>();
        covered::<LittleEndian<u32>>();
        covered::<core::num::NonZero<u32>>();
        covered::<core::num::Wrapping<u32>>();
        covered::<core::num::Saturating<u32>>();
        covered::<core::cmp::Ordering>();
        covered::<core::cmp::Reverse<u32>>();
        covered::<Option<u32>>();
        covered::<Result<u32, u8>>();
        covered::<core::ops::Bound<u32>>();
        covered::<str>();
        covered::<core::time::Duration>();
        covered::<core::net::Ipv4Addr>();
        covered::<core::net::Ipv6Addr>();
        covered::<core::net::SocketAddrV4>();
        covered::<core::net::SocketAddrV6>();
        covered::<core::net::IpAddr>();
        covered::<core::net::SocketAddr>();
        covered::<[u32; 4]>();
        covered::<core::ops::Range<u32>>();
        covered::<core::ops::RangeInclusive<u32>>();
        covered::<core::ops::RangeFrom<u32>>();
        covered::<core::ops::RangeTo<u32>>();
        covered::<core::ops::RangeToInclusive<u32>>();
        covered::<core::ops::RangeFull>();
        covered::<core::marker::PhantomData<u32>>();
        // Tuples, arity 1 through 12 - the full range this crate implements.
        covered::<(u8,)>();
        covered::<(u8, u8)>();
        covered::<(u8, u8, u8)>();
        covered::<(u8, u8, u8, u8)>();
        covered::<(u8, u8, u8, u8, u8)>();
        covered::<(u8, u8, u8, u8, u8, u8)>();
        covered::<(u8, u8, u8, u8, u8, u8, u8)>();
        covered::<(u8, u8, u8, u8, u8, u8, u8, u8)>();
        covered::<(u8, u8, u8, u8, u8, u8, u8, u8, u8)>();
        covered::<(u8, u8, u8, u8, u8, u8, u8, u8, u8, u8)>();
        covered::<(u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8)>();
        covered::<(u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8)>();
    };

    // Heap-backed collections: fingerprinted in `alloc_types.rs`, matching where their
    // `eio`/`eio_async` wire impls are gated (`alloc_types_eio.rs`).
    #[cfg(feature = "alloc")]
    const _: () = {
        covered::<alloc::string::String>();
        covered::<alloc::vec::Vec<u32>>();
        covered::<alloc::collections::VecDeque<u32>>();
        covered::<alloc::collections::LinkedList<u32>>();
        covered::<alloc::collections::BTreeSet<u32>>();
        covered::<alloc::collections::BTreeMap<u32, u8>>();
    };

    // Genuinely `std`-only types, fingerprinted in `std_types.rs` next to their wire impls.
    #[cfg(feature = "std")]
    const _: () = {
        covered::<std::collections::HashMap<u32, u8>>();
        covered::<std::collections::HashSet<u32>>();
        covered::<std::path::PathBuf>();
        covered::<std::path::Path>();
        covered::<std::ffi::CString>();
        covered::<core::ffi::CStr>();
        covered::<std::sync::Arc<u32>>();
        covered::<std::rc::Rc<u32>>();
        covered::<std::boxed::Box<u32>>();
        covered::<std::time::SystemTime>();
    };
}
