//! Example demonstrating the use of `Byteable` with `std::io::Cursor`.
//!
//! This example shows how to work with in-memory buffers using Cursor,
//! which is useful for network protocols, packet parsing, and testing.

use byteable::{Byteable, ReadByteable, WriteByteable};
use std::io::Cursor;

/// A simple message header for a network protocol
#[derive(Clone, Copy, PartialEq, Debug, Byteable)]
struct MessageHeader {
    magic: [u8; 4],   // Protocol magic number
    version: u8,      // Protocol version
    message_type: u8, // Message type identifier
    #[byteable(big_endian)]
    payload_length: u16, // Length of payload in bytes
    #[byteable(little_endian)]
    sequence_number: u32, // Message sequence number
}

/// A login request message
#[derive(Clone, Copy, PartialEq, Debug, Byteable)]
struct LoginRequest {
    #[byteable(little_endian)]
    user_id: u32,
    #[byteable(little_endian)]
    session_token: u64,
    flags: u8,
}

/// A status response message
#[derive(Clone, Copy, PartialEq, Debug, Byteable)]
struct StatusResponse {
    #[byteable(big_endian)]
    status_code: u16,
    #[byteable(little_endian)]
    timestamp: u64,
}

fn main() -> std::io::Result<()> {
    println!("=== Cursor-based Byteable Example ===\n");

    // Example 1: Writing to a cursor (in-memory buffer)
    println!("1. Writing messages to an in-memory buffer:");

    let header = MessageHeader {
        magic: *b"DEMO",
        version: 1,
        message_type: 0x01,
        payload_length: 13,
        sequence_number: 1001,
    };

    let login = LoginRequest {
        user_id: 42,
        session_token: 0x1234567890ABCDEF,
        flags: 0b00001111,
    };

    // Write to cursor
    let mut buffer = Cursor::new(Vec::new());
    buffer.write_byteable(header)?;
    buffer.write_byteable(login)?;

    let bytes = buffer.into_inner();
    println!("   Written {} bytes", bytes.len());
    println!("   Buffer contents: {:02X?}\n", bytes);

    // Example 2: Reading from a cursor
    println!("2. Reading messages from the buffer:");
    let mut reader = Cursor::new(bytes.clone());

    let read_header: MessageHeader = reader.read_byteable()?;
    let read_login: LoginRequest = reader.read_byteable()?;

    println!("   Header:");
    println!(
        "      Magic: {}",
        std::str::from_utf8(&read_header.magic).unwrap_or("???")
    );
    println!("      Version: {}", read_header.version);
    println!("      Message Type: 0x{:02X}", read_header.message_type);
    println!("      Payload Length: {} bytes", read_header.payload_length);
    println!("      Sequence Number: {}", read_header.sequence_number);

    println!("\n   Login Request:");
    println!("      User ID: {}", read_login.user_id);
    println!("      Session Token: 0x{:016X}", read_login.session_token);
    println!("      Flags: 0b{:08b}", read_login.flags);

    println!(
        "\n   Data matches: {}\n",
        read_header == header && read_login == login
    );

    // Example 3: Building a packet with multiple messages
    println!("3. Building a multi-message packet:");

    let mut packet = Cursor::new(Vec::new());

    // Write three different messages
    let headers = [
        MessageHeader {
            magic: *b"MSG1",
            version: 1,
            message_type: 0x10,
            payload_length: 0,
            sequence_number: 100,
        },
        MessageHeader {
            magic: *b"MSG2",
            version: 1,
            message_type: 0x20,
            payload_length: 0,
            sequence_number: 101,
        },
        MessageHeader {
            magic: *b"MSG3",
            version: 1,
            message_type: 0x30,
            payload_length: 0,
            sequence_number: 102,
        },
    ];

    for header in &headers {
        packet.write_byteable(*header)?;
    }

    let packet_bytes = packet.into_inner();
    println!("   Packet size: {} bytes", packet_bytes.len());
    println!(
        "   Messages per packet: {}",
        packet_bytes.len() / core::mem::size_of::<MessageHeader>()
    );

    // Read them back
    let mut reader = Cursor::new(packet_bytes);
    println!("\n   Reading messages:");
    for i in 0..3 {
        let msg: MessageHeader = reader.read_byteable()?;
        println!(
            "      Message {}: {} (type: 0x{:02X}, seq: {})",
            i + 1,
            std::str::from_utf8(&msg.magic).unwrap_or("???"),
            msg.message_type,
            msg.sequence_number
        );
    }

    // Example 4: Working with status responses
    println!("\n4. Status response example:");

    let status = StatusResponse {
        status_code: 200,
        timestamp: 1700000000,
    };

    let mut status_buffer = Cursor::new(Vec::new());
    status_buffer.write_byteable(status)?;

    let status_bytes = status_buffer.into_inner();
    println!("   Status response bytes: {:?}", status_bytes);

    let mut status_reader = Cursor::new(status_bytes);
    let read_status: StatusResponse = status_reader.read_byteable()?;

    println!("   Status Code: {}", read_status.status_code);
    println!("   Timestamp: {}", read_status.timestamp);
    println!("   Matches original: {}", read_status == status);

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
