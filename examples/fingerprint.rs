//! Catching accidental wire-format drift at compile time with `WireFingerprint`.
//!
//! Every derived type gets a `WireFingerprint` impl: a `u64` computed purely from its exact
//! wire shape (field types and order, endianness, discriminant width, tags, ...). Two types
//! that put identical bytes on the wire and accept identical bytes back hash identically; any
//! change that moves a byte changes the hash.
//!
//! `#[byteable(fingerprint = "...")]` turns that into a compile-time assertion, so a reordered
//! field or a widened integer breaks the build instead of silently shipping an incompatible
//! format to whatever's on the other end of the wire.

use byteable::{Byteable, WireFingerprint};

/// The asserted value is this type's actual `WIRE_FINGERPRINT` at the time it was pinned. If
/// you change this struct's wire shape (reorder fields, widen `count`, ...), the build fails
/// with a diagnostic naming the new value - paste it back in to re-arm the assertion.
#[derive(Byteable)]
#[byteable(fingerprint = "0xf0bf7c86bfff41ca")]
struct Header {
    magic: [u8; 4],
    #[byteable(big_endian)]
    count: u16,
}

fn main() {
    println!("Header::WIRE_FINGERPRINT = {:#x}", Header::WIRE_FINGERPRINT);

    // Two independently-defined types that happen to encode the same bytes fingerprint
    // identically - useful for confirming a hand-written type stays wire-compatible with a
    // derived one it's meant to interoperate with.
    #[derive(Byteable)]
    struct AlsoFourBytesThenU16Be {
        id: [u8; 4],
        #[byteable(big_endian)]
        n: u16,
    }
    assert_eq!(
        Header::WIRE_FINGERPRINT,
        AlsoFourBytesThenU16Be::WIRE_FINGERPRINT
    );
    println!("Header and AlsoFourBytesThenU16Be are wire-compatible (same fingerprint).");

    // Changing which byte order `count` uses changes the wire bytes, so it must change the
    // fingerprint too.
    #[derive(Byteable)]
    struct HeaderLittleEndianCount {
        magic: [u8; 4],
        count: u16, // little-endian by default, unlike Header's explicit big_endian
    }
    assert_ne!(
        Header::WIRE_FINGERPRINT,
        HeaderLittleEndianCount::WIRE_FINGERPRINT
    );
    println!(
        "HeaderLittleEndianCount has a different fingerprint - it puts different bytes on the wire."
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn main() {
        super::main();
    }
}
