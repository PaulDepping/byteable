//! `arrayvec`'s fixed-capacity collections - `io_only` support on every enabled I/O pipeline.

#[cfg(feature = "std")]
mod io;

#[cfg(feature = "tokio")]
mod async_io;

#[cfg(feature = "embedded-io")]
mod eio;

#[cfg(feature = "embedded-io-async")]
mod eio_async;

use crate::{Constraint, FingerprintBuilder, FingerprintTag, WireFingerprint};

impl<T: WireFingerprint, const N: usize> WireFingerprint for arrayvec::ArrayVec<T, N> {
    const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
        .tag(FingerprintTag::Sequence)
        .width(8)
        .len_bound(N as u64)
        .nested::<T>()
        .finish();
}

impl<const N: usize> WireFingerprint for arrayvec::ArrayString<N> {
    const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
        .tag(FingerprintTag::Sequence)
        .width(8)
        .constraint(Constraint::Utf8)
        .len_bound(N as u64)
        .nested::<u8>()
        .finish();
}
