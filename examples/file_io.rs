//! Example demonstrating the `Byteable` derive macro with file I/O operations.
//!
//! This example shows how to:
//! - Define a struct with the Byteable derive macro
//! - Write byteable structs to a file
//! - Read byteable structs from a file
//! - Handle endianness with #[byteable(little_endian)] and #[byteable(big_endian)] attributes

use byteable::{Byteable, ReadByteable, WriteByteable};
use std::fs::File;
use std::io::{self, Seek, SeekFrom};

/// A simple network packet structure.
#[derive(Clone, Copy, PartialEq, Debug, Byteable)]
struct NetworkPacket {
    /// Packet sequence number (native endianness)
    sequence: u8,
    /// Packet type identifier (little-endian)
    #[byteable(little_endian)]
    packet_type: u16,
    /// Payload length (big-endian)
    #[byteable(big_endian)]
    payload_length: u32,
    /// Timestamp (little-endian)
    #[byteable(little_endian)]
    timestamp: u64,
}

/// A configuration structure demonstrating mixed endianness.
#[derive(Clone, Copy, PartialEq, Debug, Byteable)]
struct DeviceConfig {
    /// Device ID (little-endian, common for x86 devices)
    #[byteable(little_endian)]
    device_id: u32,
    /// Protocol version (native endianness for single-byte values)
    version: u8,
    /// Flags bitfield
    flags: u8,
    /// Network port (big-endian, standard for network byte order)
    #[byteable(big_endian)]
    port: u16,
    /// Calibration factor (little-endian float)
    #[byteable(little_endian)]
    calibration: f32,
}

fn main() -> io::Result<()> {
    println!("=== Byteable Derive Macro Example ===\n");

    // Example 1: Creating and inspecting a byteable struct
    println!("1. Creating a NetworkPacket:");
    let packet = NetworkPacket {
        sequence: 42,
        packet_type: 0x1234,
        payload_length: 1024,
        timestamp: 1638360000,
    };

    println!("   Packet: {:?}", packet);
    println!("   Sequence: {}", packet.sequence);
    println!("   Packet Type: 0x{:04X}", packet.packet_type);
    println!("   Payload Length: {} bytes", packet.payload_length);
    println!("   Timestamp: {}", packet.timestamp);

    // Convert to byte array
    let bytes = packet.to_byte_array();
    println!("   As bytes: {:?}", bytes);
    println!("   Size: {} bytes\n", bytes.len());

    // Example 2: Writing to a file
    println!("2. Writing structs to a file:");
    let mut file = File::create("example_data.bin")?;

    // Write multiple packets
    file.write_byteable(packet)?;

    let packet2 = NetworkPacket {
        sequence: 43,
        packet_type: 0x5678,
        payload_length: 2048,
        timestamp: 1638360001,
    };
    file.write_byteable(packet2)?;

    // Write a device config
    let config = DeviceConfig {
        device_id: 0xABCDEF01,
        version: 1,
        flags: 0b10101010,
        port: 8080,
        calibration: 3.14159,
    };
    file.write_byteable(config)?;

    println!("   Written 2 NetworkPackets and 1 DeviceConfig to 'example_data.bin'");
    println!(
        "   File size: {} bytes\n",
        core::mem::size_of::<NetworkPacket>() * 2 + core::mem::size_of::<DeviceConfig>()
    );

    // Example 3: Reading from a file
    println!("3. Reading structs from the file:");
    let mut file = File::open("example_data.bin")?;

    // Read the packets back
    let read_packet1: NetworkPacket = file.read_byteable()?;
    let read_packet2: NetworkPacket = file.read_byteable()?;
    let read_config: DeviceConfig = file.read_byteable()?;

    println!("   First packet: {:?}", read_packet1);
    println!("   Matches original: {}", read_packet1 == packet);
    println!();

    println!("   Second packet: {:?}", read_packet2);
    println!("   Sequence: {}", read_packet2.sequence);
    println!();

    println!("   Device config: {:?}", read_config);
    println!("   Device ID: 0x{:08X}", read_config.device_id);
    println!("   Version: {}", read_config.version);
    println!("   Flags: 0b{:08b}", read_config.flags);
    println!("   Port: {}", read_config.port);
    println!("   Calibration: {:.5}", read_config.calibration);
    println!();

    // Example 4: Random access with seek
    println!("4. Random access with seek:");
    file.seek(SeekFrom::Start(0))?;
    let first: NetworkPacket = file.read_byteable()?;
    println!("   Read first packet again: sequence = {}", first.sequence);

    // Seek to the second packet
    file.seek(SeekFrom::Start(core::mem::size_of::<NetworkPacket>() as u64))?;
    let second: NetworkPacket = file.read_byteable()?;
    println!("   Seeked to second packet: sequence = {}", second.sequence);
    println!();

    // Example 5: Demonstrating byte array conversion
    println!("5. Manual byte array conversion:");
    let test_packet = NetworkPacket {
        sequence: 100,
        packet_type: 0xFF00,
        payload_length: 512,
        timestamp: 999999,
    };

    // Convert to bytes
    let byte_array = test_packet.to_byte_array();
    println!("   Original packet: {:?}", test_packet);
    println!("   Byte array: {:?}", byte_array);

    // Convert back from bytes
    let reconstructed = NetworkPacket::from_byte_array(byte_array);
    println!("   Reconstructed: {:?}", reconstructed);
    println!("   Round-trip successful: {}", test_packet == reconstructed);

    // Cleanup
    println!("\n=== Example completed successfully! ===");
    println!("Note: The file 'example_data.bin' has been created in the current directory.");

    Ok(())
}
