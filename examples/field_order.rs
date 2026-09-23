//! Pinning wire field order independently of Rust declaration order.
//!
//! A fixed protocol spec dictates the exact byte layout of a message. Rust field declaration
//! order is convenient for readability, but it shouldn't be load-bearing for the wire format:
//! a refactor that reorders fields for clarity must not silently change what's on the wire.
//!
//! `#[byteable(order = N)]` decouples the two. Every field must be annotated (or none), and
//! the `N` values must form a dense `0..field_count` permutation - it only changes which byte
//! range each field occupies, adding zero bytes to the format.

use byteable::{Byteable, ToByteArray, TryFromByteArray};

/// A sensor reading, laid out to match an external protocol's byte order:
/// `[flags: u8][reading: f32 BE]`. Declared in the order that reads best in Rust
/// (`reading` first, since it's the primary value), pinned to the protocol's actual order
/// with `order`.
#[derive(Debug, PartialEq, Byteable)]
struct SensorReading {
    #[byteable(order = 1)]
    reading: f32,
    #[byteable(order = 0)]
    flags: u8,
}

fn main() {
    let r = SensorReading {
        reading: 21.5,
        flags: 0b0000_0001,
    };

    let bytes = r.to_byte_array();
    println!("SensorReading::BYTE_SIZE = {}", SensorReading::BYTE_SIZE);
    println!("wire bytes: {:02x?}", bytes);

    // Byte 0 is `flags` (order = 0), bytes 1..5 are `reading` in native (little-endian) byte
    // order (order = 1) - matching the protocol layout, not the struct's declaration order.
    assert_eq!(bytes[0], r.flags);

    let r2 = SensorReading::try_from_byte_array(bytes).unwrap();
    assert_eq!(r, r2);
    println!(
        "Round-tripped successfully; wire layout matches the pinned `order`, not declaration order."
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn main() {
        super::main();
    }
}
