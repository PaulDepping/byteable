//! `heapless`'s fixed-capacity collections - `io_only` support on every enabled I/O pipeline.

#[cfg(feature = "std")]
mod io;

#[cfg(feature = "tokio")]
mod async_io;

#[cfg(feature = "embedded-io")]
mod eio;

#[cfg(feature = "embedded-io-async")]
mod eio_async;

use crate::{Constraint, FingerprintBuilder, FingerprintTag, WireFingerprint};

impl<T: WireFingerprint, const N: usize> WireFingerprint for heapless::Vec<T, N> {
    const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
        .tag(FingerprintTag::Sequence)
        .width(8)
        .len_bound(N as u64)
        .nested::<T>()
        .finish();
}

impl<const N: usize> WireFingerprint for heapless::String<N> {
    const WIRE_FINGERPRINT: u64 = FingerprintBuilder::new()
        .tag(FingerprintTag::Sequence)
        .width(8)
        .constraint(Constraint::Utf8)
        .len_bound(N as u64)
        .nested::<u8>()
        .finish();
}

#[cfg(all(test, feature = "std"))]
mod tests {
    #[test]
    fn bounded_vec_differs_from_unbounded_and_from_other_capacities() {
        use byteable::WireFingerprint;
        assert_ne!(
            <heapless::Vec<u32, 8> as WireFingerprint>::WIRE_FINGERPRINT,
            Vec::<u32>::WIRE_FINGERPRINT
        );
        assert_ne!(
            <heapless::Vec<u32, 8> as WireFingerprint>::WIRE_FINGERPRINT,
            <heapless::Vec<u32, 16> as WireFingerprint>::WIRE_FINGERPRINT
        );
    }

    #[test]
    fn bounded_string_has_both_utf8_and_capacity_constraints() {
        use byteable::WireFingerprint;
        let bounded = <heapless::String<16> as WireFingerprint>::WIRE_FINGERPRINT;
        assert_ne!(bounded, String::WIRE_FINGERPRINT); // capacity differs
        assert_ne!(
            bounded,
            <heapless::Vec<u8, 16> as WireFingerprint>::WIRE_FINGERPRINT
        ); // Utf8 constraint differs
    }
}
