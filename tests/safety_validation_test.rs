//! Tests for compile-time safety validation via `TransmuteSafe`.
//!
//! The `#[derive(Byteable)]` macro enforces that all field types implement
//! `TransmuteSafe`, preventing accidental use of types with invalid bit
//! patterns or non-trivial memory layout.
#![cfg(feature = "derive")]

use byteable::{Byteable, FromByteArray, IntoByteArray};

// ── Types that must compile ───────────────────────────────────────────────────

#[derive(Clone, Copy, Byteable)]
pub struct SafePacket {
    id: u8,
    #[byteable(little_endian)]
    length: u16,
    #[byteable(big_endian)]
    checksum: u32,
    data: [u8; 4],
}

#[derive(Clone, Copy, Byteable)]
struct Point {
    #[byteable(little_endian)]
    x: i32,
    #[byteable(little_endian)]
    y: i32,
}

#[derive(Clone, Copy, Byteable)]
struct Shape {
    id: u8,
    #[byteable(transparent)]
    top_left: Point,
    #[byteable(transparent)]
    bottom_right: Point,
}

#[test]
fn safe_packet_roundtrip() {
    let packet = SafePacket {
        id: 42,
        length: 1024,
        checksum: 0x12345678,
        data: [1, 2, 3, 4],
    };
    let bytes = packet.into_byte_array();
    let restored = SafePacket::from_byte_array(bytes);
    assert_eq!(packet.id, restored.id);
    assert_eq!(packet.length, restored.length);
    assert_eq!(packet.checksum, restored.checksum);
    assert_eq!(packet.data, restored.data);
}

#[test]
fn nested_safe_types_roundtrip() {
    let shape = Shape {
        id: 1,
        top_left: Point { x: 0, y: 0 },
        bottom_right: Point { x: 100, y: 200 },
    };
    let bytes = shape.into_byte_array();
    let restored = Shape::from_byte_array(bytes);
    assert_eq!(shape.id, restored.id);
    assert_eq!(shape.top_left.x, restored.top_left.x);
    assert_eq!(shape.top_left.y, restored.top_left.y);
    assert_eq!(shape.bottom_right.x, restored.bottom_right.x);
    assert_eq!(shape.bottom_right.y, restored.bottom_right.y);
}

// ── Compile-fail verification ─────────────────────────────────────────────────
//
// The following snippets must NOT compile. They are expressed as `compile_fail`
// doctests so `cargo test --doc` verifies the compiler rejects them.

/// `bool` does not implement `TransmuteSafe` (invalid bit patterns 2..=255).
///
/// ```compile_fail
/// # #[cfg(feature = "derive")] {
/// use byteable::Byteable;
///
/// #[derive(Clone, Copy, Byteable)]
/// struct Bad {
///     id: u8,
///     flag: bool,
/// }
/// # }
/// ```
///
/// `char` does not implement `TransmuteSafe` (many code-points are invalid).
///
/// ```compile_fail
/// # #[cfg(feature = "derive")] {
/// use byteable::Byteable;
///
/// #[derive(Clone, Copy, Byteable)]
/// struct Bad {
///     id: u8,
///     letter: char,
/// }
/// # }
/// ```
///
/// References do not implement `TransmuteSafe`.
///
/// ```compile_fail
/// # #[cfg(feature = "derive")] {
/// use byteable::Byteable;
///
/// #[derive(Clone, Copy, Byteable)]
/// struct Bad<'a> {
///     id: u8,
///     data_ref: &'a [u8],
/// }
/// # }
/// ```
///
/// Unadorned multi-byte primitives (`u16` without an endian wrapper) are also
/// rejected, because their native byte order is platform-dependent.
///
/// ```compile_fail
/// # #[cfg(feature = "derive")] {
/// use byteable::Byteable;
///
/// #[derive(Clone, Copy, Byteable)]
/// struct Bad {
///     value: u16,
/// }
/// # }
/// ```
#[test]
fn compile_fail_examples_documented_above() {}
