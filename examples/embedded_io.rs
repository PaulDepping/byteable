//! The `embedded-io` pipeline: the same derive output as `std::io`, but generic over
//! `embedded_io::Read`/`Write` instead, for targets without `std`.
//!
//! This example itself still runs under `std` (for `println!`), but the read/write calls
//! below go through the `eio` module exclusively - the same code compiles unchanged in a
//! `no_std` binary built with `--no-default-features --features embedded-io` (optionally
//! adding `alloc` for `Vec`/`String` support without a full `std`).

use byteable::Byteable;
use byteable::eio::{EioReadValue, EioWriteValue};

/// `#[byteable(io_only)]` generates `EioReadable`/`EioWritable` whenever the `embedded-io`
/// feature is on - no separate opt-in attribute needed, and no reference to `std` appears
/// anywhere in the generated code.
#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct Reading {
    sensor_id: u16,
    #[byteable(big_endian)]
    value: i32,
}

fn main() {
    let r = Reading {
        sensor_id: 3,
        value: -1200,
    };

    // A plain `&mut [u8]` implements `embedded_io::Write` - no heap, no `std::io::Write`.
    let mut buf = [0u8; 16];
    {
        let mut w: &mut [u8] = &mut buf;
        w.write_value(&r).unwrap();
    }

    let mut cursor: &[u8] = &buf;
    let r2: Reading = cursor.read_value().unwrap();
    assert_eq!(r, r2);
    println!("Reading round-tripped over embedded_io::{{Read,Write}}: {r2:?}");
}

#[cfg(test)]
mod tests {
    #[test]
    fn main() {
        super::main();
    }
}
