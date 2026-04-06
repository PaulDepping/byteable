//! Getting started with `byteable`: serializing Rust structs to and from bytes.
//!
//! `byteable` provides two serialization paths, both accessed through the same
//! `read_value` / `write_value` extension methods on any reader or writer:
//!
//! - **Fixed-size types** (`#[derive(Byteable)]` on plain structs) use a zero-copy
//!   transmute-based implementation. The byte size is a compile-time constant.
//!
//! - **Dynamic types** (`#[byteable(io_only)]`) write fields sequentially and
//!   support `Vec<T>`, `String`, and other variable-length collections.

use byteable::{Byteable, FromByteArray, IntoByteArray, ReadValue, ReadableError, WriteValue};
use std::io::Cursor;

// ── Fixed-size struct ─────────────────────────────────────────────────────────
//
// All fields are f32, automatically serialized as little-endian.
// BYTE_SIZE is available as a compile-time constant.

/// A 3D position in space.
#[derive(Clone, Copy, Debug, PartialEq, Byteable)]
struct Point3D {
    x: f32,
    y: f32,
    z: f32,
}

// ── Dynamic (io_only) struct ──────────────────────────────────────────────────
//
// String means the size isn't known at compile time.
// #[byteable(io_only)] generates Readable / Writable instead of the byte-array
// traits. String and Vec fields are prefixed with a u64 little-endian length.

/// A named waypoint along a path.
#[derive(Debug, PartialEq, Byteable)]
#[byteable(io_only)]
struct Waypoint {
    label: String,
    position: Point3D,
    altitude_m: f32,
}

fn main() -> Result<(), ReadableError> {
    // ── Part 1: fixed-size struct ─────────────────────────────────────────────

    let origin = Point3D {
        x: 1.0,
        y: 2.5,
        z: -0.5,
    };

    // Byte size is a compile-time constant: 3 × 4 bytes = 12.
    println!("Point3D::BYTE_SIZE = {}", Point3D::BYTE_SIZE);

    // Convert directly to a fixed-size byte array and back — no I/O needed.
    let bytes: [u8; Point3D::BYTE_SIZE] = origin.into_byte_array();
    println!("as bytes: {:02x?}", bytes);
    assert_eq!(origin, Point3D::from_byte_array(bytes));

    // Fixed-size types also work with any reader/writer via write_value / read_value.
    let mut buf = Cursor::new(Vec::<u8>::new());
    buf.write_value(&origin)?;
    buf.set_position(0);
    let restored: Point3D = buf.read_value()?;
    assert_eq!(origin, restored);
    println!("Point3D round-trip via Cursor: ok\n");

    // ── Part 2: dynamic (io_only) struct ──────────────────────────────────────

    let route = vec![
        Waypoint {
            label: "trailhead".into(),
            position: Point3D {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            altitude_m: 1200.0,
        },
        Waypoint {
            label: "ridge".into(),
            position: Point3D {
                x: 2.1,
                y: 3.4,
                z: 0.0,
            },
            altitude_m: 1850.0,
        },
        Waypoint {
            label: "summit".into(),
            position: Point3D {
                x: 4.2,
                y: 4.2,
                z: 0.0,
            },
            altitude_m: 2103.0,
        },
    ];

    // write_value and read_value work identically for io_only types,
    // including Vec<T> when T is Writable / Readable.
    let mut buf = Cursor::new(Vec::<u8>::new());
    buf.write_value(&route)?;
    println!(
        "Serialized {} waypoints into {} bytes",
        route.len(),
        buf.position()
    );

    buf.set_position(0);
    let decoded: Vec<Waypoint> = buf.read_value()?;
    for wp in &decoded {
        println!(
            "  {:10}  pos=({:.1}, {:.1})  alt={:.0} m",
            wp.label, wp.position.x, wp.position.y, wp.altitude_m
        );
    }
    assert_eq!(route, decoded);
    println!("All waypoints round-tripped successfully.");

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn main() {
        super::main().unwrap();
    }
}
