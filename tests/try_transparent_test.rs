//! Tests for the try_transparent attribute with enums.
//!
//! This test demonstrates how enums can be used with the TryHasRawType trait
//! for fallible conversion from raw representation.

use byteable::{Byteable, IntoByteArray, TryFromByteArray, TryHasRawType};

/// A simple enum representing status codes
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum Status {
    Idle = 0,
    Running = 1,
    Completed = 2,
    Failed = 3,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
struct Message {
    #[byteable(try_transparent)]
    status: Status,
    #[byteable(little_endian)]
    message: u64,
}

#[test]
fn test_enum_try_has_raw_type() {
    // Test that enums implement TryHasRawType
    let status = Status::Running;

    // Convert to raw (always succeeds)
    let raw: <Status as TryHasRawType>::Raw = status.into();

    // Convert back to enum using TryFrom (may fail for invalid discriminants)
    let restored: Status = Status::try_from(raw).unwrap();
    assert_eq!(restored, Status::Running);
}

#[test]
fn test_enum_raw_roundtrip() {
    // Test all variants can be converted through raw and back
    let variants = [
        Status::Idle,
        Status::Running,
        Status::Completed,
        Status::Failed,
    ];

    for &status in &variants {
        let raw: <Status as TryHasRawType>::Raw = status.into();
        let restored: Status = Status::try_from(raw).unwrap();
        assert_eq!(restored, status);
    }
}

#[test]
fn test_enum_invalid_discriminant_via_raw() {
    // Create a raw representation with an invalid discriminant
    let bytes = [255u8]; // Invalid status

    // TryFromByteArray should fail for invalid discriminant
    let result = Status::try_from_byte_array(bytes);
    assert!(result.is_err());

    if let Err(e) = result {
        assert_eq!(e.invalid_discriminant, byteable::Discriminant::U8(255));
    }
}

#[test]
fn test_enum_byte_conversion() {
    // Test that enum converts to expected byte representation
    assert_eq!(Status::Idle.into_byte_array(), [0]);
    assert_eq!(Status::Running.into_byte_array(), [1]);
    assert_eq!(Status::Completed.into_byte_array(), [2]);
    assert_eq!(Status::Failed.into_byte_array(), [3]);

    // Test that valid bytes convert back correctly
    assert_eq!(Status::try_from_byte_array([0]).unwrap(), Status::Idle);
    assert_eq!(Status::try_from_byte_array([1]).unwrap(), Status::Running);
    assert_eq!(Status::try_from_byte_array([2]).unwrap(), Status::Completed);
    assert_eq!(Status::try_from_byte_array([3]).unwrap(), Status::Failed);
}

// =============================================================================

// Tests for try_transparent attribute on Message struct
// =============================================================================

#[test]
fn test_message_with_valid_status() {
    // Create a message with a valid status
    let msg = Message {
        status: Status::Running,
        message: 0x123456789ABCDEF0,
    };

    // Convert to bytes
    let bytes = msg.into_byte_array();

    // Expected: 1 byte for status (Running = 1) + 8 bytes for u64 message (little-endian)
    assert_eq!(bytes.len(), 9);
    assert_eq!(bytes[0], 1); // Status::Running

    // Convert back from bytes (should succeed)
    let restored = Message::try_from_byte_array(bytes).unwrap();
    assert_eq!(restored, msg);
}

#[test]
fn test_message_with_all_status_variants() {
    // Test all status variants in Message
    let test_cases = [
        (Status::Idle, 0u64),
        (Status::Running, 42u64),
        (Status::Completed, 0xDEADBEEF),
        (Status::Failed, u64::MAX),
    ];

    for (status, message_val) in test_cases {
        let msg = Message {
            status,
            message: message_val,
        };

        let bytes = msg.into_byte_array();
        let restored = Message::try_from_byte_array(bytes).unwrap();
        assert_eq!(restored, msg);
    }
}

#[test]
fn test_message_with_invalid_status_discriminant() {
    // Create a byte array with an invalid status discriminant
    let mut bytes = [0u8; 9];
    bytes[0] = 255; // Invalid status value
    bytes[1..9].copy_from_slice(&0x123456789ABCDEF0u64.to_le_bytes());

    // TryFromByteArray should fail because of the invalid status discriminant
    let result = Message::try_from_byte_array(bytes);
    assert!(result.is_err());

    // Verify the error contains the invalid discriminant value
    if let Err(e) = result {
        assert_eq!(e.invalid_discriminant, byteable::Discriminant::U8(255));
    }
}

#[test]
fn test_message_byte_layout() {
    // Verify the exact byte layout: status (1 byte) + message (8 bytes, little-endian)
    let msg = Message {
        status: Status::Completed,
        message: 0x0102030405060708,
    };

    let bytes = msg.into_byte_array();

    // First byte should be the status discriminant
    assert_eq!(bytes[0], 2); // Status::Completed = 2

    // Next 8 bytes should be the u64 message in little-endian
    assert_eq!(bytes[1], 0x08);
    assert_eq!(bytes[2], 0x07);
    assert_eq!(bytes[3], 0x06);
    assert_eq!(bytes[4], 0x05);
    assert_eq!(bytes[5], 0x04);
    assert_eq!(bytes[6], 0x03);
    assert_eq!(bytes[7], 0x02);
    assert_eq!(bytes[8], 0x01);
}

#[test]
fn test_message_roundtrip_all_variants() {
    // Comprehensive roundtrip test with different combinations
    let messages = [
        Message {
            status: Status::Idle,
            message: 0,
        },
        Message {
            status: Status::Running,
            message: 1,
        },
        Message {
            status: Status::Completed,
            message: 0xFFFFFFFFFFFFFFFF,
        },
        Message {
            status: Status::Failed,
            message: 0x8000000000000000,
        },
    ];

    for msg in &messages {
        let bytes = msg.into_byte_array();
        let restored = Message::try_from_byte_array(bytes).unwrap();
        assert_eq!(&restored, msg);
    }
}

#[test]
fn test_message_invalid_discriminants() {
    // Test various invalid discriminant values
    let invalid_discriminants = [4u8, 5, 10, 100, 200, 254, 255];

    for invalid in invalid_discriminants {
        let mut bytes = [0u8; 9];
        bytes[0] = invalid;

        let result = Message::try_from_byte_array(bytes);
        assert!(
            result.is_err(),
            "Expected error for discriminant {}",
            invalid
        );

        if let Err(e) = result {
            assert_eq!(e.invalid_discriminant, byteable::Discriminant::U8(invalid));
        }
    }
}

// =============================================================================
// Tests for try_transparent with u16 enum in a separate module
// =============================================================================

mod command_packet_tests {
    use byteable::{Byteable, IntoByteArray, TryFromByteArray};

    /// Test enum with u16 representation
    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    #[repr(u16)]
    #[byteable(little_endian)]
    enum Command {
        Start = 0x1000,
        Stop = 0x2000,
        Pause = 0x3000,
    }

    /// Test struct with try_transparent on u16 enum
    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    struct CommandPacket {
        #[byteable(try_transparent)]
        command: Command,
        #[byteable(little_endian)]
        payload: u32,
    }

    #[test]
    fn test_command_packet_with_try_transparent() {
        let packet = CommandPacket {
            command: Command::Start,
            payload: 0xABCD1234,
        };

        let bytes = packet.into_byte_array();

        // Expected: 2 bytes for Command (u16, little-endian) + 4 bytes for u32 payload
        assert_eq!(bytes.len(), 6);
        assert_eq!(bytes[0], 0x00); // Command::Start low byte
        assert_eq!(bytes[1], 0x10); // Command::Start high byte

        let restored = CommandPacket::try_from_byte_array(bytes).unwrap();
        assert_eq!(restored, packet);
    }

    #[test]
    fn test_command_packet_with_invalid_command() {
        let mut bytes = [0u8; 6];
        // Invalid command discriminant
        bytes[0] = 0xFF;
        bytes[1] = 0xFF;
        // Payload
        bytes[2..6].copy_from_slice(&0xABCD1234u32.to_le_bytes());

        let result = CommandPacket::try_from_byte_array(bytes);
        assert!(result.is_err());

        if let Err(e) = result {
            assert_eq!(e.invalid_discriminant, byteable::Discriminant::U16(0xFFFF));
        }
    }

    #[test]
    fn test_command_packet_all_variants() {
        let packets = [
            CommandPacket {
                command: Command::Start,
                payload: 0,
            },
            CommandPacket {
                command: Command::Stop,
                payload: 0xFFFFFFFF,
            },
            CommandPacket {
                command: Command::Pause,
                payload: 0x12345678,
            },
        ];

        for packet in &packets {
            let bytes = packet.into_byte_array();
            let restored = CommandPacket::try_from_byte_array(bytes).unwrap();
            assert_eq!(&restored, packet);
        }
    }
}
