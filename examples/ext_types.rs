//! Third-party type support: `heapless` capacity-bounded collections, `bitflags`, and
//! `ordered-float`.
//!
//! These are opt-in per Cargo feature (`heapless`, `bitflags`, `ordered-float`). `heapless`
//! and `ordered-float` work automatically once the feature is on; `bitflags` additionally
//! needs a one-line macro invocation per flags type, since a blanket impl over the foreign
//! `Flags` trait would conflict with this crate's other impls.

use byteable::io::{ReadValue, ReadableError, WriteValue};
use byteable::{Byteable, ToByteArray, TryFromByteArray};
use heapless::{String as HString, Vec as HVec};
use ordered_float::NotNan;
use std::io::Cursor;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Perms: u32 {
        const READ = 0b001;
        const WRITE = 0b010;
        const EXEC = 0b100;
    }
}
byteable::impl_bitflags!(Perms);

/// A fixed-size record: `perms` is a `bitflags` type, `confidence` rejects NaN. `NotNan`'s
/// decode can fail, so its field needs `try_transparent` - that promotes the containing
/// struct's own decode from infallible (`FromByteArray`) to fallible (`TryFromByteArray`).
#[derive(Debug, PartialEq, Byteable)]
struct FileMeta {
    perms: Perms,
    #[byteable(try_transparent)]
    confidence: NotNan<f32>,
}

/// A dynamic record using `heapless`'s fixed-capacity collections instead of `alloc`'s -
/// no heap allocation anywhere, so this also compiles under `--no-default-features
/// --features embedded-io,heapless` (no `alloc` needed).
#[derive(Debug, PartialEq, Byteable)]
#[byteable(io_only)]
struct Device {
    id: u32,
    name: HString<16>,
    tags: HVec<u32, 4>,
}

fn main() -> Result<(), ReadableError> {
    let meta = FileMeta {
        perms: Perms::READ | Perms::EXEC,
        confidence: NotNan::new(0.97).unwrap(),
    };
    let bytes = meta.to_byte_array();
    println!("FileMeta: {:?} -> {:02x?}", meta, bytes);
    assert_eq!(FileMeta::try_from_byte_array(bytes).unwrap(), meta);

    let mut tags = HVec::new();
    tags.extend_from_slice(&[1, 2, 3]).unwrap();
    let device = Device {
        id: 7,
        name: HString::try_from("sensor-a").unwrap(),
        tags,
    };

    let mut buf = Cursor::new(Vec::<u8>::new());
    buf.write_value(&device)?;
    buf.set_position(0);
    let decoded: Device = buf.read_value()?;
    assert_eq!(device, decoded);
    println!("Device round-tripped: {decoded:?}");

    // Decoding checks the wire length against the container's compile-time capacity instead
    // of panicking or truncating: a `HVec<u32, 4>` field can hold at most 4 elements, so bytes
    // claiming a 5th element decode to `DecodeError::CapacityExceeded`, not a panic.
    let mut oversized = Cursor::new(Vec::<u8>::new());
    oversized.write_value(&5u64)?; // claim 5 elements
    for i in 0..5u32 {
        oversized.write_value(&i)?;
    }
    oversized.set_position(0);
    let result: Result<HVec<u32, 4>, _> = oversized.read_value();
    assert!(matches!(
        result,
        Err(ReadableError::DecodeError(
            byteable::DecodeError::CapacityExceeded { .. }
        ))
    ));
    println!("Oversized wire data correctly rejected with CapacityExceeded, not a panic.");

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn main() {
        super::main().unwrap();
    }
}
