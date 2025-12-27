use byteable::Byteable;

// Simple tuple struct with basic types
#[derive(Clone, Copy, Byteable, Debug, PartialEq)]
struct SimpleTuple(
    u8,
    #[byteable(little_endian)] u16,
    #[byteable(little_endian)] u32,
);

// Tuple struct with endianness markers
#[derive(Clone, Copy, Byteable, Debug, PartialEq)]
struct EndianTuple(
    u8,
    #[byteable(little_endian)] u16,
    #[byteable(big_endian)] u32,
    #[byteable(little_endian)] u64,
);

// Nested tuple struct (using transparent)
#[derive(Clone, Copy, Byteable, Debug, PartialEq)]
struct InnerTuple(u8, #[byteable(little_endian)] u16);

#[derive(Clone, Copy, Byteable, Debug, PartialEq)]
struct OuterTuple(
    #[byteable(transparent)] InnerTuple,
    u8,
    #[byteable(big_endian)] u32,
);

// Tuple struct with array
#[derive(Clone, Copy, Byteable, Debug, PartialEq)]
struct ArrayTuple(u8, [u8; 4], #[byteable(little_endian)] u16);

#[test]
fn test_simple_tuple_struct() {
    println!("=== Testing Simple Tuple Struct ===\n");

    let tuple = SimpleTuple(42, 0x1234, 0x12345678);

    println!(
        "SimpleTuple: ({}, 0x{:04x}, 0x{:08x})",
        tuple.0, tuple.1, tuple.2
    );

    // Test conversion to bytes
    let bytes = tuple.to_byte_array();
    println!("Bytes: {:?}", bytes);
    println!("Size: {} bytes", bytes.len());

    // Expected size: u8(1) + u16(2) + u32(4) = 7 bytes
    assert_eq!(bytes.len(), 7);

    // Test conversion back from bytes
    let restored = SimpleTuple::from_byte_array(bytes);
    println!(
        "Restored: ({}, 0x{:04x}, 0x{:08x})",
        restored.0, restored.1, restored.2
    );

    assert_eq!(tuple, restored);
    println!("✓ Simple tuple struct test passed!\n");
}

#[test]
fn test_endian_tuple_struct() {
    println!("=== Testing Endian Tuple Struct ===\n");

    let tuple = EndianTuple(42, 0x1234, 0x12345678, 0x0102030405060708);

    println!(
        "EndianTuple: ({}, 0x{:04x}, 0x{:08x}, 0x{:016x})",
        tuple.0, tuple.1, tuple.2, tuple.3
    );

    // Test conversion to bytes
    let bytes = tuple.to_byte_array();
    println!("Bytes: {:?}", bytes);
    println!("Size: {} bytes", bytes.len());

    // Expected size: u8(1) + u16(2) + u32(4) + u64(8) = 15 bytes
    assert_eq!(bytes.len(), 15);

    // Verify byte layout
    println!("\nVerifying endianness:");

    // Field 0: u8 at byte 0
    assert_eq!(bytes[0], 42);
    println!("  Field 0 (u8) at byte 0: {} ✓", bytes[0]);

    // Field 1: little-endian u16 at bytes 1-2 (0x1234 -> [0x34, 0x12])
    assert_eq!(bytes[1], 0x34);
    assert_eq!(bytes[2], 0x12);
    println!(
        "  Field 1 (little-endian u16) at bytes 1-2: [0x{:02x}, 0x{:02x}] ✓",
        bytes[1], bytes[2]
    );

    // Field 2: big-endian u32 at bytes 3-6 (0x12345678 -> [0x12, 0x34, 0x56, 0x78])
    assert_eq!(bytes[3], 0x12);
    assert_eq!(bytes[4], 0x34);
    assert_eq!(bytes[5], 0x56);
    assert_eq!(bytes[6], 0x78);
    println!(
        "  Field 2 (big-endian u32) at bytes 3-6: [0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}] ✓",
        bytes[3], bytes[4], bytes[5], bytes[6]
    );

    // Field 3: little-endian u64 at bytes 7-14
    assert_eq!(bytes[7], 0x08);
    assert_eq!(bytes[8], 0x07);
    assert_eq!(bytes[9], 0x06);
    assert_eq!(bytes[10], 0x05);
    assert_eq!(bytes[11], 0x04);
    assert_eq!(bytes[12], 0x03);
    assert_eq!(bytes[13], 0x02);
    assert_eq!(bytes[14], 0x01);
    println!(
        "  Field 3 (little-endian u64) at bytes 7-14: [0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}] ✓",
        bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14]
    );

    // Test conversion back from bytes
    let restored = EndianTuple::from_byte_array(bytes);
    assert_eq!(tuple, restored);
    println!("\n✓ Endian tuple struct test passed!\n");
}

#[test]
fn test_nested_tuple_struct() {
    println!("=== Testing Nested Tuple Struct (Transparent) ===\n");

    let inner = InnerTuple(10, 0x1234);
    println!("InnerTuple: ({}, 0x{:04x})", inner.0, inner.1);

    let inner_bytes = inner.to_byte_array();
    println!("Inner bytes: {:?}", inner_bytes);
    assert_eq!(inner_bytes.len(), 3); // u8(1) + u16(2) = 3

    let outer = OuterTuple(inner, 42, 0x12345678);
    println!(
        "OuterTuple: (InnerTuple({}, 0x{:04x}), {}, 0x{:08x})",
        outer.0.0, outer.0.1, outer.1, outer.2
    );

    // Test conversion to bytes
    let bytes = outer.to_byte_array();
    println!("Bytes: {:?}", bytes);
    println!("Size: {} bytes", bytes.len());

    // Expected size: InnerTuple(3) + u8(1) + u32(4) = 8 bytes
    assert_eq!(bytes.len(), 8);

    // Verify byte layout
    println!("\nVerifying byte layout:");

    // Field 0 (transparent InnerTuple): bytes 0-2
    assert_eq!(bytes[0], 10); // inner.0
    assert_eq!(bytes[1], 0x34); // inner.1 low byte (little-endian)
    assert_eq!(bytes[2], 0x12); // inner.1 high byte (little-endian)
    println!(
        "  Field 0 (transparent InnerTuple) at bytes 0-2: [{}, 0x{:02x}, 0x{:02x}] ✓",
        bytes[0], bytes[1], bytes[2]
    );

    // Field 1 (u8): byte 3
    assert_eq!(bytes[3], 42);
    println!("  Field 1 (u8) at byte 3: {} ✓", bytes[3]);

    // Field 2 (big-endian u32): bytes 4-7
    assert_eq!(bytes[4], 0x12);
    assert_eq!(bytes[5], 0x34);
    assert_eq!(bytes[6], 0x56);
    assert_eq!(bytes[7], 0x78);
    println!(
        "  Field 2 (big-endian u32) at bytes 4-7: [0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}] ✓",
        bytes[4], bytes[5], bytes[6], bytes[7]
    );

    // Test conversion back from bytes
    let restored = OuterTuple::from_byte_array(bytes);
    assert_eq!(outer, restored);
    println!("\n✓ Nested tuple struct test passed!\n");
}

#[test]
fn test_array_tuple_struct() {
    println!("=== Testing Array Tuple Struct ===\n");

    let tuple = ArrayTuple(42, [0xDE, 0xAD, 0xBE, 0xEF], 0x1234);

    println!(
        "ArrayTuple: ({}, {:?}, 0x{:04x})",
        tuple.0, tuple.1, tuple.2
    );

    // Test conversion to bytes
    let bytes = tuple.to_byte_array();
    println!("Bytes: {:?}", bytes);
    println!("Size: {} bytes", bytes.len());

    // Expected size: u8(1) + [u8; 4](4) + u16(2) = 7 bytes
    assert_eq!(bytes.len(), 7);

    // Verify byte layout
    assert_eq!(bytes[0], 42);
    assert_eq!(bytes[1], 0xDE);
    assert_eq!(bytes[2], 0xAD);
    assert_eq!(bytes[3], 0xBE);
    assert_eq!(bytes[4], 0xEF);
    assert_eq!(bytes[5], 0x34); // little-endian low byte
    assert_eq!(bytes[6], 0x12); // little-endian high byte

    // Test conversion back from bytes
    let restored = ArrayTuple::from_byte_array(bytes);
    assert_eq!(tuple, restored);
    println!("✓ Array tuple struct test passed!\n");
}

#[test]
fn test_tuple_struct_roundtrip() {
    println!("=== Testing Multiple Roundtrips ===\n");

    let original = EndianTuple(100, 0xABCD, 0xDEADBEEF, 0x0123456789ABCDEF);

    // Do multiple roundtrips
    for i in 1..=5 {
        let bytes = original.to_byte_array();
        let restored = EndianTuple::from_byte_array(bytes);
        assert_eq!(original, restored);
        println!("  Roundtrip {}: ✓", i);
    }

    println!("\n✓ Multiple roundtrip test passed!\n");
}

#[test]
fn test_tuple_struct_clone() {
    let tuple = SimpleTuple(1, 2, 3);
    let cloned = tuple.clone();
    assert_eq!(tuple, cloned);
    println!("✓ Clone test passed!");
}
