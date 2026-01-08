use byteable::{Byteable, IntoByteArray, TryFromByteArray};

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum Status {
    Idle = 0,
    Running = 1,
    Completed = 2,
    Failed = 3,
}

#[test]
fn test_enum_to_bytes() {
    let status = Status::Running;
    let bytes = status.into_byte_array();
    assert_eq!(bytes, [1]);
}

#[test]
fn test_enum_from_bytes() {
    let bytes = [2];
    let status = Status::try_from_byte_array(bytes).unwrap();
    assert_eq!(status, Status::Completed);
}

#[test]
fn test_enum_invalid_discriminant() {
    let bytes = [255]; // Invalid discriminant
    let result = Status::try_from_byte_array(bytes);
    assert!(result.is_err());

    if let Err(err) = result {
        assert_eq!(err.invalid_discriminant, byteable::Discriminant::U8(255));
    }
}

#[test]
fn test_all_status_variants() {
    let variants = [
        Status::Idle,
        Status::Running,
        Status::Completed,
        Status::Failed,
    ];
    let expected_bytes = [[0], [1], [2], [3]];

    for (variant, expected) in variants.iter().zip(expected_bytes.iter()) {
        let bytes = variant.into_byte_array();
        assert_eq!(bytes, *expected);

        let restored = Status::try_from_byte_array(bytes).unwrap();
        assert_eq!(restored, *variant);
    }
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u16)]
#[byteable(little_endian)]
enum Command {
    Start = 0x1000,
    Stop = 0x2000,
    Pause = 0x3000,
}

#[test]
fn test_u16_enum() {
    let cmd = Command::Pause;
    let bytes = cmd.into_byte_array();

    // Verify the bytes match the discriminant (little-endian)
    assert_eq!(bytes, [0x00, 0x30]);

    let restored = Command::try_from_byte_array(bytes).unwrap();
    assert_eq!(restored, cmd);
}

#[test]
fn test_u16_enum_all_variants() {
    assert_eq!(Command::Start.into_byte_array(), 0x1000u16.to_le_bytes());
    assert_eq!(Command::Stop.into_byte_array(), 0x2000u16.to_le_bytes());
    assert_eq!(Command::Pause.into_byte_array(), 0x3000u16.to_le_bytes());
}

#[test]
fn test_u16_enum_invalid_discriminant() {
    let bytes = 0x9999u16.to_le_bytes();
    let result = Command::try_from_byte_array(bytes);
    assert!(result.is_err());
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
#[byteable(little_endian)]
enum NetworkProtocol {
    Tcp = 6,
    Udp = 17,
    Icmp = 1,
}

#[test]
fn test_u32_enum() {
    let protocol = NetworkProtocol::Tcp;
    let bytes = protocol.into_byte_array();
    assert_eq!(bytes, 6u32.to_le_bytes());

    let restored = NetworkProtocol::try_from_byte_array(bytes).unwrap();
    assert_eq!(restored, NetworkProtocol::Tcp);
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(i8)]
enum Temperature {
    Cold = -10,
    Cool = 0,
    Warm = 10,
    Hot = 30,
}

#[test]
fn test_signed_enum() {
    let temp = Temperature::Cold;
    let bytes = temp.into_byte_array();
    assert_eq!(bytes, [(-10i8) as u8]);

    let restored = Temperature::try_from_byte_array(bytes).unwrap();
    assert_eq!(restored, Temperature::Cold);
}

#[test]
fn test_signed_enum_all_variants() {
    assert_eq!(Temperature::Cold.into_byte_array(), (-10i8).to_ne_bytes());
    assert_eq!(Temperature::Cool.into_byte_array(), 0i8.to_ne_bytes());
    assert_eq!(Temperature::Warm.into_byte_array(), 10i8.to_ne_bytes());
    assert_eq!(Temperature::Hot.into_byte_array(), 30i8.to_ne_bytes());
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u64)]
#[byteable(little_endian)]
enum LargeValue {
    Small = 1,
    Medium = 1000,
    Large = 1_000_000,
    Huge = 1_000_000_000_000,
}

#[test]
fn test_u64_enum() {
    let val = LargeValue::Huge;
    let bytes = val.into_byte_array();
    assert_eq!(bytes, 1_000_000_000_000u64.to_le_bytes());

    let restored = LargeValue::try_from_byte_array(bytes).unwrap();
    assert_eq!(restored, LargeValue::Huge);
}

#[test]
fn test_enum_roundtrip() {
    // Test Status enum
    for i in 0u8..=3 {
        let bytes = [i];
        let status = Status::try_from_byte_array(bytes).unwrap();
        let bytes_back = status.into_byte_array();
        assert_eq!(bytes, bytes_back);
    }
}

#[test]
fn test_enum_byte_size() {
    use byteable::AssociatedByteArray;

    assert_eq!(Status::BYTE_SIZE, 1);
    assert_eq!(Command::BYTE_SIZE, 2);
    assert_eq!(NetworkProtocol::BYTE_SIZE, 4);
    assert_eq!(Temperature::BYTE_SIZE, 1);
    assert_eq!(LargeValue::BYTE_SIZE, 8);
}

#[test]
fn test_enum_error_display() {
    let bytes = [255];
    let result = Status::try_from_byte_array(bytes);

    if let Err(err) = result {
        let error_string = format!("{}", err);
        assert!(error_string.contains("Invalid discriminant"));
        assert!(error_string.contains("255"));
    } else {
        panic!("Expected an error");
    }
}

#[cfg(feature = "std")]
#[test]
fn test_enum_error_trait() {
    use std::error::Error;

    let bytes = [100];
    let result = Status::try_from_byte_array(bytes);

    if let Err(err) = result {
        // Test that it implements std::error::Error
        let _: &dyn Error = &err;
        assert!(format!("{:?}", err).contains("EnumFromBytesError"));
    } else {
        panic!("Expected an error");
    }
}

// Test enum with non-sequential discriminants
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum SparseEnum {
    First = 1,
    Second = 5,
    Third = 10,
    Fourth = 100,
}

#[test]
fn test_sparse_enum() {
    assert_eq!(SparseEnum::First.into_byte_array(), [1]);
    assert_eq!(SparseEnum::Second.into_byte_array(), [5]);
    assert_eq!(SparseEnum::Third.into_byte_array(), [10]);
    assert_eq!(SparseEnum::Fourth.into_byte_array(), [100]);

    // Valid values
    assert_eq!(
        SparseEnum::try_from_byte_array([1]).unwrap(),
        SparseEnum::First
    );
    assert_eq!(
        SparseEnum::try_from_byte_array([5]).unwrap(),
        SparseEnum::Second
    );
    assert_eq!(
        SparseEnum::try_from_byte_array([10]).unwrap(),
        SparseEnum::Third
    );
    assert_eq!(
        SparseEnum::try_from_byte_array([100]).unwrap(),
        SparseEnum::Fourth
    );

    // Invalid values (gaps in the discriminants)
    assert!(SparseEnum::try_from_byte_array([0]).is_err());
    assert!(SparseEnum::try_from_byte_array([2]).is_err());
    assert!(SparseEnum::try_from_byte_array([6]).is_err());
    assert!(SparseEnum::try_from_byte_array([99]).is_err());
    assert!(SparseEnum::try_from_byte_array([101]).is_err());
}

// Test enums with explicit little-endian byte order
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u16)]
#[byteable(little_endian)]
enum LittleEndianCommand {
    Start = 0x1000,
    Stop = 0x2000,
    Pause = 0x3000,
}

#[test]
fn test_little_endian_enum() {
    // Little-endian: LSB first
    let cmd = LittleEndianCommand::Start;
    let bytes = cmd.into_byte_array();
    assert_eq!(bytes, [0x00, 0x10]); // 0x1000 in little-endian

    let cmd = LittleEndianCommand::Stop;
    let bytes = cmd.into_byte_array();
    assert_eq!(bytes, [0x00, 0x20]); // 0x2000 in little-endian

    let cmd = LittleEndianCommand::Pause;
    let bytes = cmd.into_byte_array();
    assert_eq!(bytes, [0x00, 0x30]); // 0x3000 in little-endian

    // Test round-trip
    let restored = LittleEndianCommand::try_from_byte_array([0x00, 0x10]).unwrap();
    assert_eq!(restored, LittleEndianCommand::Start);

    let restored = LittleEndianCommand::try_from_byte_array([0x00, 0x20]).unwrap();
    assert_eq!(restored, LittleEndianCommand::Stop);

    let restored = LittleEndianCommand::try_from_byte_array([0x00, 0x30]).unwrap();
    assert_eq!(restored, LittleEndianCommand::Pause);
}

// Test enums with explicit big-endian byte order
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u16)]
#[byteable(big_endian)]
enum BigEndianCommand {
    Start = 0x1000,
    Stop = 0x2000,
    Pause = 0x3000,
}

#[test]
fn test_big_endian_enum() {
    // Big-endian: MSB first
    let cmd = BigEndianCommand::Start;
    let bytes = cmd.into_byte_array();
    assert_eq!(bytes, [0x10, 0x00]); // 0x1000 in big-endian

    let cmd = BigEndianCommand::Stop;
    let bytes = cmd.into_byte_array();
    assert_eq!(bytes, [0x20, 0x00]); // 0x2000 in big-endian

    let cmd = BigEndianCommand::Pause;
    let bytes = cmd.into_byte_array();
    assert_eq!(bytes, [0x30, 0x00]); // 0x3000 in big-endian

    // Test round-trip
    let restored = BigEndianCommand::try_from_byte_array([0x10, 0x00]).unwrap();
    assert_eq!(restored, BigEndianCommand::Start);

    let restored = BigEndianCommand::try_from_byte_array([0x20, 0x00]).unwrap();
    assert_eq!(restored, BigEndianCommand::Stop);

    let restored = BigEndianCommand::try_from_byte_array([0x30, 0x00]).unwrap();
    assert_eq!(restored, BigEndianCommand::Pause);
}

// Test little-endian with u32
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
#[byteable(little_endian)]
enum LittleEndianProtocol {
    Tcp = 0x12345678,
    Udp = 0xABCDEF00,
}

#[test]
fn test_little_endian_u32_enum() {
    let proto = LittleEndianProtocol::Tcp;
    let bytes = proto.into_byte_array();
    assert_eq!(bytes, [0x78, 0x56, 0x34, 0x12]); // Little-endian

    let proto = LittleEndianProtocol::Udp;
    let bytes = proto.into_byte_array();
    assert_eq!(bytes, [0x00, 0xEF, 0xCD, 0xAB]); // Little-endian

    // Round-trip
    let restored = LittleEndianProtocol::try_from_byte_array([0x78, 0x56, 0x34, 0x12]).unwrap();
    assert_eq!(restored, LittleEndianProtocol::Tcp);
}

// Test big-endian with u32
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
#[byteable(big_endian)]
enum BigEndianProtocol {
    Tcp = 0x12345678,
    Udp = 0xABCDEF00,
}

#[test]
fn test_big_endian_u32_enum() {
    let proto = BigEndianProtocol::Tcp;
    let bytes = proto.into_byte_array();
    assert_eq!(bytes, [0x12, 0x34, 0x56, 0x78]); // Big-endian

    let proto = BigEndianProtocol::Udp;
    let bytes = proto.into_byte_array();
    assert_eq!(bytes, [0xAB, 0xCD, 0xEF, 0x00]); // Big-endian

    // Round-trip
    let restored = BigEndianProtocol::try_from_byte_array([0x12, 0x34, 0x56, 0x78]).unwrap();
    assert_eq!(restored, BigEndianProtocol::Tcp);
}

// Test little-endian with u64
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u64)]
#[byteable(little_endian)]
enum LittleEndianLarge {
    Small = 0x1122334455667788,
    Large = 0xAABBCCDDEEFF0011,
}

#[test]
fn test_little_endian_u64_enum() {
    let val = LittleEndianLarge::Small;
    let bytes = val.into_byte_array();
    assert_eq!(bytes, [0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]);

    let val = LittleEndianLarge::Large;
    let bytes = val.into_byte_array();
    assert_eq!(bytes, [0x11, 0x00, 0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA]);

    // Round-trip
    let restored =
        LittleEndianLarge::try_from_byte_array([0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11])
            .unwrap();
    assert_eq!(restored, LittleEndianLarge::Small);
}

// Test big-endian with u64
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u64)]
#[byteable(big_endian)]
enum BigEndianLarge {
    Small = 0x1122334455667788,
    Large = 0xAABBCCDDEEFF0011,
}

#[test]
fn test_big_endian_u64_enum() {
    let val = BigEndianLarge::Small;
    let bytes = val.into_byte_array();
    assert_eq!(bytes, [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]);

    let val = BigEndianLarge::Large;
    let bytes = val.into_byte_array();
    assert_eq!(bytes, [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11]);

    // Round-trip
    let restored =
        BigEndianLarge::try_from_byte_array([0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88])
            .unwrap();
    assert_eq!(restored, BigEndianLarge::Small);
}

// Test that endianness doesn't affect single-byte enums
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum LittleEndianByte {
    A = 1,
    B = 2,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum BigEndianByte {
    A = 1,
    B = 2,
}

#[test]
fn test_endianness_irrelevant_for_u8() {
    // For u8, endianness doesn't matter
    assert_eq!(LittleEndianByte::A.into_byte_array(), [1]);
    assert_eq!(LittleEndianByte::B.into_byte_array(), [2]);
    assert_eq!(BigEndianByte::A.into_byte_array(), [1]);
    assert_eq!(BigEndianByte::B.into_byte_array(), [2]);
}

// Test with signed types
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(i16)]
#[byteable(little_endian)]
enum LittleEndianSigned {
    Negative = -1000,
    Zero = 0,
    Positive = 1000,
}

#[test]
fn test_little_endian_signed_enum() {
    let val = LittleEndianSigned::Negative;
    let bytes = val.into_byte_array();
    assert_eq!(bytes, (-1000i16).to_le_bytes());

    let val = LittleEndianSigned::Positive;
    let bytes = val.into_byte_array();
    assert_eq!(bytes, 1000i16.to_le_bytes());

    // Round-trip
    let restored = LittleEndianSigned::try_from_byte_array((-1000i16).to_le_bytes()).unwrap();
    assert_eq!(restored, LittleEndianSigned::Negative);
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(i16)]
#[byteable(big_endian)]
enum BigEndianSigned {
    Negative = -1000,
    Zero = 0,
    Positive = 1000,
}

#[test]
fn test_big_endian_signed_enum() {
    let val = BigEndianSigned::Negative;
    let bytes = val.into_byte_array();
    assert_eq!(bytes, (-1000i16).to_be_bytes());

    let val = BigEndianSigned::Positive;
    let bytes = val.into_byte_array();
    assert_eq!(bytes, 1000i16.to_be_bytes());

    // Round-trip
    let restored = BigEndianSigned::try_from_byte_array((-1000i16).to_be_bytes()).unwrap();
    assert_eq!(restored, BigEndianSigned::Negative);
}
