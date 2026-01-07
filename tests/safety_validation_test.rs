/// Tests for compile-time safety validation using ValidBytecastMarker
///
/// This test demonstrates that the Byteable macros now enforce compile-time
/// safety by requiring all field types to implement ValidBytecastMarker.
use byteable::{Byteable, FromByteArray, IntoByteArray};

// This should compile - all fields are safe types
#[derive(Clone, Copy, Byteable)]
pub struct SafePacket {
    id: u8,
    #[byteable(little_endian)]
    length: u16,
    #[byteable(big_endian)]
    checksum: u32,
    data: [u8; 4],
}

// This should also compile - nested safe structs
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
fn test_safe_types_compile() {
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
fn test_nested_safe_types_compile() {
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

// The following tests verify that unsafe types are rejected at compile time.
// These are compile-fail tests that should be uncommented to verify the
// compile-time safety checks are working.

/*
// This should NOT compile - bool has invalid bit patterns
#[derive(Clone, Copy, Byteable)]
struct UnsafePacket1 {
    id: u8,
    is_valid: bool, // ERROR: bool doesn't implement ValidBytecastMarker
}

// This should NOT compile - char has invalid bit patterns
#[derive(Clone, Copy, Byteable)]
struct UnsafePacket2 {
    id: u8,
    letter: char, // ERROR: char doesn't implement ValidBytecastMarker
}

// This should NOT compile - contains pointer
#[derive(Clone, Copy, Byteable)]
struct UnsafePacket3<'a> {
    id: u8,
    data_ref: &'a [u8], // ERROR: &T doesn't implement ValidBytecastMarker
}
*/
