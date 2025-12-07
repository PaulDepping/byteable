//! Example demonstrating the use of `Byteable` with `std::io::Cursor`.
//!
//! This example shows how to work with in-memory buffers using Cursor,
//! which is useful for network protocols, packet parsing, and testing.

use byteable::{BigEndian, Byteable, LittleEndian, ReadByteable, WriteByteable};
use std::io::Cursor;

/// A simple message header for a network protocol
#[derive(Byteable, Clone, Copy, PartialEq, Debug)]
#[repr(C, packed)]
struct MessageHeader {
    magic: [u8; 4],                     // Protocol magic number
    version: u8,                        // Protocol version
    message_type: u8,                   // Message type identifier
    payload_length: BigEndian<u16>,     // Length of payload in bytes
    sequence_number: LittleEndian<u32>, // Message sequence number
}

/// A login request message
#[derive(Byteable, Clone, Copy, PartialEq, Debug)]
#[repr(C, packed)]
struct LoginRequest {
    user_id: LittleEndian<u32>,
    session_token: LittleEndian<u64>,
    flags: u8,
    padding: [u8; 3], // Padding for alignment
}

/// A status response message
#[derive(Byteable, Clone, Copy, PartialEq, Debug)]
#[repr(C, packed)]
struct StatusResponse {
    status_code: BigEndian<u16>,
    timestamp: LittleEndian<u64>,
    reserved: [u8; 6],
}

fn main() -> std::io::Result<()> {
    println!("=== Cursor-based Byteable Example ===\n");

    // Example 1: Writing to a cursor (in-memory buffer)
    println!("1. Writing messages to an in-memory buffer:");

    let header = MessageHeader {
        magic: *b"DEMO",
        version: 1,
        message_type: 0x01,
        payload_length: BigEndian::new(16),
        sequence_number: LittleEndian::new(1001),
    };

    let login = LoginRequest {
        user_id: LittleEndian::new(42),
        session_token: LittleEndian::new(0x1234567890ABCDEF),
        flags: 0b00001111,
        padding: [0; 3],
    };

    // Write to cursor
    let mut buffer = Cursor::new(Vec::new());
    buffer.write_one(header)?;
    buffer.write_one(login)?;

    let bytes = buffer.into_inner();
    println!("   Written {} bytes", bytes.len());
    println!("   Buffer contents: {:02X?}\n", bytes);

    // Example 2: Reading from a cursor
    println!("2. Reading messages from the buffer:");
    let mut reader = Cursor::new(bytes.clone());

    let read_header: MessageHeader = reader.read_one()?;
    let read_login: LoginRequest = reader.read_one()?;

    println!("   Header:");
    println!(
        "      Magic: {}",
        std::str::from_utf8(&read_header.magic).unwrap_or("???")
    );
    println!("      Version: {}", read_header.version);
    println!("      Message Type: 0x{:02X}", read_header.message_type);
    println!(
        "      Payload Length: {} bytes",
        read_header.payload_length.get()
    );
    println!(
        "      Sequence Number: {}",
        read_header.sequence_number.get()
    );

    println!("\n   Login Request:");
    println!("      User ID: {}", read_login.user_id.get());
    println!(
        "      Session Token: 0x{:016X}",
        read_login.session_token.get()
    );
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
            payload_length: BigEndian::new(16),
            sequence_number: LittleEndian::new(100),
        },
        MessageHeader {
            magic: *b"MSG2",
            version: 1,
            message_type: 0x20,
            payload_length: BigEndian::new(16),
            sequence_number: LittleEndian::new(101),
        },
        MessageHeader {
            magic: *b"MSG3",
            version: 1,
            message_type: 0x30,
            payload_length: BigEndian::new(16),
            sequence_number: LittleEndian::new(102),
        },
    ];

    for header in &headers {
        packet.write_one(*header)?;
    }

    let packet_bytes = packet.into_inner();
    println!("   Packet size: {} bytes", packet_bytes.len());
    println!(
        "   Messages per packet: {}",
        packet_bytes.len() / std::mem::size_of::<MessageHeader>()
    );

    // Read them back
    let mut reader = Cursor::new(packet_bytes);
    println!("\n   Reading messages:");
    for i in 0..3 {
        let msg: MessageHeader = reader.read_one()?;
        println!(
            "      Message {}: {} (type: 0x{:02X}, seq: {})",
            i + 1,
            std::str::from_utf8(&msg.magic).unwrap_or("???"),
            msg.message_type,
            msg.sequence_number.get()
        );
    }

    // Example 4: Working with status responses
    println!("\n4. Status response example:");

    let status = StatusResponse {
        status_code: BigEndian::new(200),
        timestamp: LittleEndian::new(1700000000),
        reserved: [0; 6],
    };

    let mut status_buffer = Cursor::new(Vec::new());
    status_buffer.write_one(status)?;

    let status_bytes = status_buffer.into_inner();
    println!("   Status response bytes: {:?}", status_bytes);

    let mut status_reader = Cursor::new(status_bytes);
    let read_status: StatusResponse = status_reader.read_one()?;

    println!("   Status Code: {}", read_status.status_code.get());
    println!("   Timestamp: {}", read_status.timestamp.get());
    println!("   Matches original: {}", read_status == status);

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
