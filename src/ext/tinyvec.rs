//! `tinyvec`'s fixed-capacity `ArrayVec` - `io_only` support on every enabled I/O pipeline.

#[cfg(feature = "std")]
mod io;

#[cfg(feature = "tokio")]
mod async_io;

#[cfg(feature = "embedded-io")]
mod eio;

#[cfg(feature = "embedded-io-async")]
mod eio_async;

use crate::{FingerprintBuilder, FingerprintTag, WireFingerprint};

impl<T: WireFingerprint + Default, const N: usize> WireFingerprint for tinyvec::ArrayVec<[T; N]> {
    const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
        .tag(FingerprintTag::Sequence)
        .width(8)
        .len_bound(N as u64)
        .nested::<T>()
        .finish();
}
