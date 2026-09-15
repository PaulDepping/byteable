//! Integration tests for defmt support.
//!
//! Actually invoking `defmt::Format::format` requires a `#[defmt::global_logger]` (normally
//! provided by something like `defmt-rtt` on a real embedded target), which this host-side
//! test binary has none of. So these tests assert the trait bound compiles instead of
//! capturing formatted output — the standard way third-party crates test defmt integration
//! outside of an actual embedded runtime.
#![cfg(feature = "defmt")]

fn assert_format<T: defmt::Format>() {}

#[test]
fn decode_error_implements_format() {
    assert_format::<byteable::DecodeError>();
}

#[test]
fn little_endian_implements_format() {
    assert_format::<byteable::LittleEndian<u32>>();
}

#[test]
fn big_endian_implements_format() {
    assert_format::<byteable::BigEndian<u32>>();
}

#[cfg(feature = "embedded-io")]
#[test]
fn eio_readable_error_implements_format() {
    assert_format::<byteable::eio::EioReadableError<u8>>();
}

#[cfg(feature = "embedded-io")]
#[test]
fn eio_read_exact_error_implements_format() {
    assert_format::<byteable::eio::EioReadExactError<u8>>();
}
