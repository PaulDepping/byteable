//! Example demonstrating the `UnsafeByteable` derive macro with file I/O operations.
//!
//! This example shows how to:
//! - Define a struct with the UnsafeByteable derive macro
//! - Write byteable structs to a file
//! - Read byteable structs from a file
//! - Handle endianness with BigEndian and LittleEndian wrappers

use byteable::{BigEndian, Byteable, LittleEndian, ReadByteable, UnsafeByteable, WriteByteable};
use std::fs::File;
use std::io::{self, Seek, SeekFrom};

/// A simple network packet structure.
#[derive(Clone, Copy, PartialEq, Debug)]
struct NetworkPacket {
    /// Packet sequence number
    sequence: u8,
    /// Packet type identifier
    packet_type: u16,
    /// Payload length
    payload_length: u32,
    /// Timestamp
    timestamp: u64,
}

/// Requirements for deriving Byteable:
/// - Must be `#[repr(C, packed)]` or `#[repr(C)]` for predictable memory layout
/// - Must implement `Copy`
/// - All fields must be Byteable (primitives, or types wrapped in BigEndian/LittleEndian)
#[derive(Clone, Copy, Debug, UnsafeByteable)]
#[repr(C, packed)]
struct NetworkPacketRaw {
    /// Packet sequence number (native endianness)
    sequence: u8,
    /// Packet type identifier (little-endian)
    packet_type: LittleEndian<u16>,
    /// Payload length (big-endian)
    payload_length: BigEndian<u32>,
    /// Timestamp (little-endian)
    timestamp: LittleEndian<u64>,
}

impl Byteable for NetworkPacket {
    type ByteArray = <NetworkPacketRaw as Byteable>::ByteArray;

    fn as_bytearray(self) -> Self::ByteArray {
        NetworkPacketRaw {
            sequence: self.sequence,
            packet_type: self.packet_type.into(),
            payload_length: self.payload_length.into(),
            timestamp: self.timestamp.into(),
        }
        .as_bytearray()
    }

    fn from_bytearray(ba: Self::ByteArray) -> Self {
        let raw = NetworkPacketRaw::from_bytearray(ba);
        Self {
            sequence: raw.sequence,
            packet_type: raw.packet_type.get(),
            payload_length: raw.payload_length.get(),
            timestamp: raw.timestamp.get(),
        }
    }
}

/// A configuration structure demonstrating mixed endianness.
#[derive(Clone, Copy, PartialEq, Debug)]
struct DeviceConfig {
    /// Device ID
    device_id: u32,
    /// Protocol version
    version: u8,
    /// Flags bitfield
    flags: u8,
    /// Network port
    port: u16,
    /// Calibration factor
    calibration: f32,
}

#[derive(Clone, Copy, Debug, UnsafeByteable)]
#[repr(C, packed)]
struct DeviceConfigRaw {
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

impl Byteable for DeviceConfig {
    type ByteArray = <DeviceConfigRaw as Byteable>::ByteArray;

    fn as_bytearray(self) -> Self::ByteArray {
        DeviceConfigRaw {
            device_id: self.device_id.into(),
            version: self.version,
            flags: self.flags,
            port: self.port.into(),
            calibration: self.calibration.into(),
        }
        .as_bytearray()
    }

    fn from_bytearray(ba: Self::ByteArray) -> Self {
        let raw = DeviceConfigRaw::from_bytearray(ba);
        Self {
            device_id: raw.device_id.get(),
            version: raw.version,
            flags: raw.flags,
            port: raw.port.get(),
            calibration: raw.calibration.get(),
        }
    }
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
        packet_type: 0x5678,
        payload_length: 2048,
        timestamp: 1638360001,
    };
    file.write_one(packet2)?;

    // Write a device config
    let config = DeviceConfig {
        device_id: 0xABCDEF01,
        version: 1,
        flags: 0b10101010,
        port: 8080,
        calibration: 3.14159,
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
    println!("   Device ID: 0x{:08X}", read_config.device_id);
    println!("   Version: {}", read_config.version);
    println!("   Flags: 0b{:08b}", read_config.flags);
    println!("   Port: {}", read_config.port);
    println!("   Calibration: {:.5}", read_config.calibration);
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
        packet_type: 0xFF00,
        payload_length: 512,
        timestamp: 999999,
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
