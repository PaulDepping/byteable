//! [`WireFingerprint`](crate::WireFingerprint) implementations for `alloc`-backed collection
//! types.
//!
//! These types' actual wire impls live once per I/O pipeline - `Readable`/`Writable` in
//! `std_types.rs`, `AsyncReadable`/`AsyncWritable` in `std_types_async.rs`,
//! `EioReadable`/`EioWritable` in `alloc_types_eio.rs`, `EioAsync*` in
//! `alloc_types_eio_async.rs` - and the last two of those are gated on `alloc`, not `std`. A
//! fingerprint gated on `std` would therefore be missing exactly where it is needed most: a
//! `#[derive(Byteable)] #[byteable(io_only)]` struct with a `Vec<u32>` field, built for a
//! `no_std` target with `--features embedded-io,alloc`, generates `.nested::<Vec<u32>>()` and
//! needs `Vec<u32>: WireFingerprint` to exist there. Hence this module, gated on `alloc`
//! alone (see `lib.rs`).
//!
//! `HashMap`/`HashSet` are *not* here: they need `std`'s `RandomState`, so they are
//! fingerprinted in `std_types.rs` alongside their own wire impls. `str`, `Option`, `Result`,
//! `Bound`, tuples and the network enums need no heap at all and are fingerprinted
//! unconditionally in `core_types.rs`.
//!
//! No `extern crate alloc;` needed here - `src/lib.rs` already declares it crate-wide (gated
//! on the same `alloc` feature this module requires).

use crate::{FingerprintBuilder, FingerprintTag, WireFingerprint};
use alloc::{
    collections::{BTreeMap, BTreeSet, LinkedList, VecDeque},
    string::String,
    vec::Vec,
};

// Wire format: `u64` element count (LE), then each element serialized in order.
impl<T: WireFingerprint> WireFingerprint for Vec<T> {
    const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
        .tag(FingerprintTag::Sequence)
        .width(8) // u64 length prefix
        .nested::<T>()
        .finish();
}

// `str` (in `core_types.rs`) is the canonical definition; `String` is wire-identical to it.
impl WireFingerprint for String {
    const WIRE_FINGERPRINT: u64 = <str as WireFingerprint>::WIRE_FINGERPRINT;
}

impl<T: WireFingerprint> WireFingerprint for VecDeque<T> {
    const WIRE_FINGERPRINT: u64 = <Vec<T> as WireFingerprint>::WIRE_FINGERPRINT;
}

impl<T: WireFingerprint> WireFingerprint for LinkedList<T> {
    const WIRE_FINGERPRINT: u64 = <Vec<T> as WireFingerprint>::WIRE_FINGERPRINT;
}

impl<T: WireFingerprint> WireFingerprint for BTreeSet<T> {
    const WIRE_FINGERPRINT: u64 = <Vec<T> as WireFingerprint>::WIRE_FINGERPRINT;
}

impl<K: WireFingerprint, V: WireFingerprint> WireFingerprint for BTreeMap<K, V> {
    const WIRE_FINGERPRINT: u64 = <Vec<(K, V)> as WireFingerprint>::WIRE_FINGERPRINT;
}
