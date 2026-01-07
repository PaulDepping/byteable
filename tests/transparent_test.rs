use byteable::{Byteable, FromByteArray, IntoByteArray};

#[derive(Clone, Copy, Byteable)]
struct MemberStruct {
    a: u8,
    #[byteable(little_endian)]
    b: u16,
}

#[derive(Clone, Copy, Byteable)]
struct TestStruct {
    #[byteable(transparent)]
    member: MemberStruct,
    a: u8,
    #[byteable(little_endian)]
    b: u16,
    #[byteable(big_endian)]
    c: u64,
    #[byteable(little_endian)]
    d: f64,
}

#[test]
fn test_transparent_attribute() {
    println!("=== Testing Transparent Attribute Feature ===\n");

    // Create a MemberStruct
    let member = MemberStruct { a: 10, b: 0x1234 };

    println!("MemberStruct:");
    println!("  a: {}", member.a);
    println!("  b: 0x{:04x}", member.b);

    // Test MemberStruct serialization
    let member_bytes = member.into_byte_array();
    println!("  bytes: {:?}", member_bytes);
    println!("  size: {} bytes", member_bytes.len());
    assert_eq!(member_bytes.len(), 3); // u8 + u16 = 3 bytes

    // Verify MemberStruct byte layout
    assert_eq!(member_bytes[0], 10); // a
    assert_eq!(member_bytes[1], 0x34); // b low byte (little-endian)
    assert_eq!(member_bytes[2], 0x12); // b high byte (little-endian)
    println!("  ✓ MemberStruct byte layout verified\n");

    // Create a TestStruct with transparent member
    let test = TestStruct {
        member,
        a: 42,
        b: 0x5678,
        c: 0x0102030405060708,
        d: 3.14159,
    };

    println!("TestStruct:");
    println!("  member.a: {}", test.member.a);
    println!("  member.b: 0x{:04x}", test.member.b);
    println!("  a: {}", test.a);
    println!("  b: 0x{:04x}", test.b);
    println!("  c: 0x{:016x}", test.c);
    println!("  d: {}", test.d);

    // Test TestStruct serialization
    let test_bytes = test.into_byte_array();
    println!("  bytes: {:?}", test_bytes);
    println!("  size: {} bytes", test_bytes.len());

    // Calculate expected size:
    // member (transparent): 3 bytes
    // a: 1 byte
    // b (little-endian u16): 2 bytes
    // c (big-endian u64): 8 bytes
    // d (f64): 8 bytes
    // Total: 22 bytes
    assert_eq!(test_bytes.len(), 22);
    println!("  ✓ Total size is correct (22 bytes)\n");

    // Verify byte layout
    println!("Verifying byte layout:");

    // member (transparent) - should be stored as [u8; 3]
    println!("  member (bytes 0-2): {:?}", &test_bytes[0..3]);
    assert_eq!(test_bytes[0], 10); // member.a
    assert_eq!(test_bytes[1], 0x34); // member.b low byte
    assert_eq!(test_bytes[2], 0x12); // member.b high byte
    println!("    ✓ transparent member field correct");

    // a (byte 3)
    assert_eq!(test_bytes[3], 42);
    println!("    ✓ field 'a' correct (byte 3)");

    // b (little-endian, bytes 4-5)
    assert_eq!(test_bytes[4], 0x78); // low byte
    assert_eq!(test_bytes[5], 0x56); // high byte
    println!("    ✓ field 'b' correct (little-endian, bytes 4-5)");

    // c (big-endian, bytes 6-13)
    assert_eq!(test_bytes[6], 0x01);
    assert_eq!(test_bytes[7], 0x02);
    assert_eq!(test_bytes[8], 0x03);
    assert_eq!(test_bytes[9], 0x04);
    assert_eq!(test_bytes[10], 0x05);
    assert_eq!(test_bytes[11], 0x06);
    assert_eq!(test_bytes[12], 0x07);
    assert_eq!(test_bytes[13], 0x08);
    println!("    ✓ field 'c' correct (big-endian, bytes 6-13)");

    // d (little-endian f64, bytes 14-21)
    let d_bytes = &test_bytes[14..22];
    let d_restored = f64::from_le_bytes(d_bytes.try_into().unwrap());
    assert_eq!(d_restored, 3.14159);
    println!("    ✓ field 'd' correct (little-endian f64, bytes 14-21)");

    // Test deserialization
    println!("\nTesting deserialization:");
    let restored = TestStruct::from_byte_array(test_bytes);

    assert_eq!(restored.member.a, test.member.a);
    assert_eq!(restored.member.b, test.member.b);
    assert_eq!(restored.a, test.a);
    assert_eq!(restored.b, test.b);
    assert_eq!(restored.c, test.c);
    assert_eq!(restored.d, test.d);

    println!("  restored.member.a: {}", restored.member.a);
    println!("  restored.member.b: 0x{:04x}", restored.member.b);
    println!("  restored.a: {}", restored.a);
    println!("  restored.b: 0x{:04x}", restored.b);
    println!("  restored.c: 0x{:016x}", restored.c);
    println!("  restored.d: {}", restored.d);
    println!("  ✓ All values correctly restored");

    println!("\n=== ✓✓ All tests passed! ===");
    println!("\nThe transparent attribute successfully stores the nested");
    println!("Byteable struct as its [u8; N] representation!");
}
