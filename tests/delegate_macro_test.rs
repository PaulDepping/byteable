use byteable::Byteable;

#[derive(Clone, Copy, Byteable)]
struct TestStruct {
    a: u8,
    #[byteable(little_endian)]
    b: u16,
    #[byteable(big_endian)]
    c: u64,
    #[byteable(little_endian)]
    d: f64,
}

#[test]
fn test_delegate_macro() {
    let test = TestStruct {
        a: 42,
        b: 0x1234,
        c: 0x0102030405060708,
        d: 3.14159,
    };

    // Test conversion to bytes
    let bytes = test.as_byte_array();
    println!("TestStruct as bytes: {:?}", bytes);
    println!("Byte array length: {}", bytes.len());

    // Test conversion back from bytes
    let restored = TestStruct::from_byte_array(bytes);
    println!("\nRestored struct:");
    println!("  a: {}", restored.a);
    println!("  b: 0x{:04x}", restored.b);
    println!("  c: 0x{:016x}", restored.c);
    println!("  d: {}", restored.d);

    // Verify the values match
    assert_eq!(test.a, restored.a);
    assert_eq!(test.b, restored.b);
    assert_eq!(test.c, restored.c);
    assert_eq!(test.d, restored.d);

    println!("\n✓ All tests passed!");

    // Verify endianness is correct
    println!("\nVerifying endianness:");
    // 'a' is just u8, so it's the first byte
    assert_eq!(bytes[0], 42);
    println!("  a (u8) at byte 0: {} ✓", bytes[0]);

    // 'b' should be little-endian u16 (0x1234 -> [0x34, 0x12])
    assert_eq!(bytes[1], 0x34);
    assert_eq!(bytes[2], 0x12);
    println!(
        "  b (little-endian u16) at bytes 1-2: [0x{:02x}, 0x{:02x}] ✓",
        bytes[1], bytes[2]
    );

    // 'c' should be big-endian u64 (0x0102030405060708 -> [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08])
    assert_eq!(bytes[3], 0x01);
    assert_eq!(bytes[4], 0x02);
    assert_eq!(bytes[5], 0x03);
    assert_eq!(bytes[6], 0x04);
    assert_eq!(bytes[7], 0x05);
    assert_eq!(bytes[8], 0x06);
    assert_eq!(bytes[9], 0x07);
    assert_eq!(bytes[10], 0x08);
    println!(
        "  c (big-endian u64) at bytes 3-10: [0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}] ✓",
        bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10]
    );

    // 'd' is f64 (8 bytes), starts at byte 11
    let d_bytes = &bytes[11..19];
    let d_restored = f64::from_ne_bytes(d_bytes.try_into().unwrap());
    assert_eq!(d_restored, 3.14159);
    println!("  d (f64) at bytes 11-18: {} ✓", d_restored);

    println!("\n✓✓ All endianness checks passed!");
}
