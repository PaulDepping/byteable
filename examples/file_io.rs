//! Example demonstrating the `Byteable` derive macro with file I/O operations.
//!
//! This example shows how to:
//! - Define a struct with the Byteable derive macro
//! - Write byteable structs to a file
//! - Read byteable structs from a file
//! - Handle endianness with BigEndian and LittleEndian wrappers

use byteable::{BigEndian, Byteable, LittleEndian, ReadByteable, WriteByteable};
use std::fs::File;
use std::io::{self, Seek, SeekFrom};

/// A simple network packet structure demonstrating the Byteable derive macro.
///
/// Requirements for deriving Byteable:
/// - Must be `#[repr(C, packed)]` or `#[repr(C)]` for predictable memory layout
/// - Must implement `Copy`
/// - All fields must be Byteable (primitives, or types wrapped in BigEndian/LittleEndian)
#[derive(Byteable, Clone, Copy, PartialEq, Debug)]
#[repr(C, packed)]
struct NetworkPacket {
    /// Packet sequence number (native endianness)
    sequence: u8,
    /// Packet type identifier (little-endian)
    packet_type: LittleEndian<u16>,
    /// Payload length (big-endian)
    payload_length: BigEndian<u32>,
    /// Timestamp (little-endian)
    timestamp: LittleEndian<u64>,
}

/// A configuration structure demonstrating mixed endianness.
#[derive(Byteable, Clone, Copy, PartialEq, Debug)]
#[repr(C, packed)]
struct DeviceConfig {
    /// Device ID (little-endian, common for x86 devices)
    device_id: LittleEndian<u32>,
    /// Protocol version (native endianness for single-byte values)
    version: u8,
    /// Flags bitfield
    flags: u8,
    /// Network port (big-endian, standard for network byte order)
    port: BigEndian<u16>,
    /// Calibration factor (little-endian float)
    calibration: LittleEndian<f32>,
}

fn main() -> io::Result<()> {
    println!("=== Byteable Derive Macro Example ===\n");

    // Example 1: Creating and inspecting a byteable struct
    println!("1. Creating a NetworkPacket:");
    let packet = NetworkPacket {
        sequence: 42,
        packet_type: LittleEndian::new(0x1234),
        payload_length: BigEndian::new(1024),
        timestamp: LittleEndian::new(1638360000),
    };

    println!("   Packet: {:?}", packet);
    println!("   Sequence: {}", packet.sequence);
    println!("   Packet Type: 0x{:04X}", packet.packet_type.get());
    println!("   Payload Length: {} bytes", packet.payload_length.get());
    println!("   Timestamp: {}", packet.timestamp.get());

    // Convert to byte array
    let bytes = packet.as_bytearray();
    println!("   As bytes: {:?}", bytes);
    println!("   Size: {} bytes\n", bytes.len());

    // Example 2: Writing to a file
    println!("2. Writing structs to a file:");
    let mut file = File::create("example_data.bin")?;

    // Write multiple packets
    file.write_one(packet)?;

    let packet2 = NetworkPacket {
        sequence: 43,
        packet_type: LittleEndian::new(0x5678),
        payload_length: BigEndian::new(2048),
        timestamp: LittleEndian::new(1638360001),
    };
    file.write_one(packet2)?;

    // Write a device config
    let config = DeviceConfig {
        device_id: LittleEndian::new(0xABCDEF01),
        version: 1,
        flags: 0b10101010,
        port: BigEndian::new(8080),
        calibration: LittleEndian::new(3.14159),
    };
    file.write_one(config)?;

    println!("   Written 2 NetworkPackets and 1 DeviceConfig to 'example_data.bin'");
    println!(
        "   File size: {} bytes\n",
        std::mem::size_of::<NetworkPacket>() * 2 + std::mem::size_of::<DeviceConfig>()
    );

    // Example 3: Reading from a file
    println!("3. Reading structs from the file:");
    let mut file = File::open("example_data.bin")?;

    // Read the packets back
    let read_packet1: NetworkPacket = file.read_one()?;
    let read_packet2: NetworkPacket = file.read_one()?;
    let read_config: DeviceConfig = file.read_one()?;

    println!("   First packet: {:?}", read_packet1);
    println!("   Matches original: {}", read_packet1 == packet);
    println!();

    println!("   Second packet: {:?}", read_packet2);
    println!("   Sequence: {}", read_packet2.sequence);
    println!();

    println!("   Device config: {:?}", read_config);
    println!("   Device ID: 0x{:08X}", read_config.device_id.get());
    println!("   Version: {}", read_config.version);
    println!("   Flags: 0b{:08b}", read_config.flags);
    println!("   Port: {}", read_config.port.get());
    println!("   Calibration: {:.5}", read_config.calibration.get());
    println!();

    // Example 4: Random access with seek
    println!("4. Random access with seek:");
    file.seek(SeekFrom::Start(0))?;
    let first: NetworkPacket = file.read_one()?;
    println!("   Read first packet again: sequence = {}", first.sequence);

    // Seek to the second packet
    file.seek(SeekFrom::Start(std::mem::size_of::<NetworkPacket>() as u64))?;
    let second: NetworkPacket = file.read_one()?;
    println!("   Seeked to second packet: sequence = {}", second.sequence);
    println!();

    // Example 5: Demonstrating byte array conversion
    println!("5. Manual byte array conversion:");
    let test_packet = NetworkPacket {
        sequence: 100,
        packet_type: LittleEndian::new(0xFF00),
        payload_length: BigEndian::new(512),
        timestamp: LittleEndian::new(999999),
    };

    // Convert to bytes
    let byte_array = test_packet.as_bytearray();
    println!("   Original packet: {:?}", test_packet);
    println!("   Byte array: {:?}", byte_array);

    // Convert back from bytes
    let reconstructed = NetworkPacket::from_bytearray(byte_array);
    println!("   Reconstructed: {:?}", reconstructed);
    println!("   Round-trip successful: {}", test_packet == reconstructed);

    // Cleanup
    println!("\n=== Example completed successfully! ===");
    println!("Note: The file 'example_data.bin' has been created in the current directory.");

    Ok(())
}
