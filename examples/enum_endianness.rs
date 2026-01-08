use byteable::{Byteable, IntoByteArray, TryFromByteArray};

// Enum with explicit little-endian byte order
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u16)]
#[byteable(little_endian)]
enum LittleEndianCommand {
    Start = 0x1000,
    Stop = 0x2000,
    Pause = 0x3000,
}

// Enum with explicit big-endian byte order (common for network protocols)
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u16)]
#[byteable(big_endian)]
enum BigEndianCommand {
    Start = 0x1000,
    Stop = 0x2000,
    Pause = 0x3000,
}

// Network protocol status codes with big-endian (network byte order)
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
#[byteable(big_endian)]
enum HttpStatus {
    Ok = 200,
    NotFound = 404,
    InternalError = 500,
}

fn main() {
    println!("=== Enum Endianness Support Demo ===\n");

    // Little-endian (explicit)
    let le_cmd = LittleEndianCommand::Start;
    let le_bytes = le_cmd.into_byte_array();
    println!("Little-Endian Command::Start bytes: {:02X?}", le_bytes);
    println!("  (Always [0x00, 0x10] regardless of platform)");

    // Big-endian (explicit)
    let be_cmd = BigEndianCommand::Start;
    let be_bytes = be_cmd.into_byte_array();
    println!("\nBig-Endian Command::Start bytes: {:02X?}", be_bytes);
    println!("  (Always [0x10, 0x00] regardless of platform)");

    // Round-trip conversion
    println!("\n=== Round-trip Conversion ===\n");
    let restored_le = LittleEndianCommand::try_from_byte_array([0x00, 0x20]).unwrap();
    println!("Little-Endian [0x00, 0x20] -> {:?}", restored_le);

    let restored_be = BigEndianCommand::try_from_byte_array([0x20, 0x00]).unwrap();
    println!("Big-Endian [0x20, 0x00] -> {:?}", restored_be);

    // HTTP status codes with big-endian (network byte order)
    println!("\n=== Network Protocol Example ===\n");
    let status = HttpStatus::Ok;
    let status_bytes = status.into_byte_array();
    println!("HTTP 200 OK in big-endian: {:02X?}", status_bytes);
    println!("  (Always [0x00, 0x00, 0x00, 0xC8] for network transmission)");

    // Invalid discriminant handling
    println!("\n=== Error Handling ===\n");
    let invalid_bytes = [0xFF, 0xFF];
    match LittleEndianCommand::try_from_byte_array(invalid_bytes) {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Invalid discriminant error: {}", e),
    }

    println!("\n=== Use Case: Network Packet ===\n");
    // Simulating a network packet with big-endian command
    let packet_bytes = [0x20, 0x00]; // Big-endian representation of 0x2000 (Stop)
    match BigEndianCommand::try_from_byte_array(packet_bytes) {
        Ok(cmd) => println!("Received network command: {:?}", cmd),
        Err(e) => println!("Error parsing command: {}", e),
    }

    println!("\n✓ All conversions successful!");
}
