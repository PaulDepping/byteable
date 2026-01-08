# Byteable

[![Crates.io](https://img.shields.io/crates/v/byteable.svg)](https://crates.io/crates/byteable)
[![Documentation](https://docs.rs/byteable/badge.svg)](https://docs.rs/byteable)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

_A Rust crate for zero-overhead, zero-copy serialization and deserialization of byte-oriented data._

`byteable` provides traits and utilities for seamless conversion between data structures and byte arrays, with full support for both synchronous and asynchronous I/O operations, and comprehensive endianness handling.

## Features

- **Byte Conversion Traits**: Modular trait system for byte array conversion:
  - `AssociatedByteArray`: Associates a type with its byte array representation
  - `IntoByteArray`: Converts values into byte arrays
  - `FromByteArray`: Constructs values from byte arrays
  - `TryIntoByteArray` & `TryFromByteArray`: Fallible conversion variants for types that can fail (e.g., `bool`, `char`, enums)
- **`ReadByteable` & `WriteByteable`**: Extension traits for `std::io::Read` and `std::io::Write`
- **`AsyncReadByteable` & `AsyncWriteByteable`**: Async I/O support with tokio (optional)
- **Endianness Support**: `BigEndian<T>` and `LittleEndian<T>` wrappers for explicit byte order
- **`#[derive(Byteable)]`**: Procedural macro for automatic trait implementation with endianness support (optional)
- **Extensive Documentation**: Every function, trait, and type is thoroughly documented with examples
- **Inline Comments**: All implementations include detailed explanatory comments
- **Zero Overhead**: Compiles down to simple memory operations with no runtime cost

## Why byteable?

- **Binary Protocols**: Perfect for implementing network protocols (TCP, UDP, custom formats)
- **File I/O**: Read/write binary file formats with ease
- **Cross-Platform**: Consistent behavior across different architectures with endianness control
- **Type-Safe**: Rust's type system ensures correctness at compile time
- **No Dependencies**: Core functionality has zero dependencies (tokio is optional)

## Installation

Add `byteable` to your `Cargo.toml`:

```toml
[dependencies]
byteable = "0.19"  # Or latest version
```

### Optional Features

```toml
[dependencies]
byteable = { version = "0.19", features = ["derive", "tokio"] }
```

- **`derive`** (default): Enables the `#[derive(Byteable)]` procedural macro
- **`tokio`**: Enables async I/O traits for use with tokio

## Quick Start

### Basic File I/O Example

```rust
use byteable::{Byteable, LittleEndian, ReadByteable, WriteByteable};
use std::fs::File;

#[derive(Byteable, Debug, PartialEq)]
struct Packet {
    id: u8,
    #[byteable(little_endian)]
    length: u16,
    data: [u8; 4],
}

fn main() -> std::io::Result<()> {
    // Create a packet
    let packet = Packet {
        id: 42,
        length: 1024.into(),
        data: [0xDE, 0xAD, 0xBE, 0xEF],
    };

    // Write packet to a file
    let mut file = File::create("packet.bin")?;
    file.write_byteable(packet)?;
    println!("Packet written to file");

    // Read packet back from file
    let mut file = File::open("packet.bin")?;
    let restored: Packet = file.read_byteable()?;

    assert_eq!(packet, restored);
    println!("Packet successfully read back: {:?}", restored);

    Ok(())
}
```

### Network Protocol Example

```rust
use byteable::Byteable;

#[derive(Byteable, Debug, Clone, Copy)]
struct TcpHeader {
    #[byteable(big_endian)]
    source_port: u16,      // Network byte order (big-endian)
    #[byteable(big_endian)]
    dest_port: u16,
    #[byteable(big_endian)]
    sequence_num: u32,
    #[byteable(big_endian)]
    ack_num: u32,
}

let header = TcpHeader {
    source_port: 80,
    dest_port: 8080,
    sequence_num: 12345,
    ack_num: 67890,
};

// Convert to bytes for transmission
let bytes = header.into_byte_array();
```

### Async I/O with Tokio

```rust
use byteable::{AsyncReadByteable, AsyncWriteByteable, Byteable};
use tokio::net::TcpStream;

#[derive(Byteable, Debug, Clone, Copy)]
struct Message {
    msg_type: u8,
    payload: [u8; 64],
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;

    let msg = Message {
        msg_type: 1,
        payload: [0; 64],
    };

    // Async write
    stream.write_byteable(msg).await?;

    // Async read
    let response: Message = stream.read_byteable().await?;

    Ok(())
}
```

## Primitive Type Support

### `bool` and `char`

The crate provides safe support for `bool` and `char` types with proper validation via `TryFromByteArray`. These types have restricted valid byte patterns and will return errors for invalid values.

#### Boolean Support

```rust
use byteable::{IntoByteArray, TryFromByteArray};

// Valid boolean values
let value = true;
let bytes = value.into_byte_array();
assert_eq!(bytes, [1]);

let value = false;
let bytes = value.into_byte_array();
assert_eq!(bytes, [0]);

// Roundtrip conversion
let restored = bool::try_from_byte_array([1]).unwrap();
assert_eq!(restored, true);

// Invalid byte values return errors
let result = bool::try_from_byte_array([2]);
assert!(result.is_err()); // Only 0 and 1 are valid
```

#### Character Support

Rust's `char` type represents a Unicode scalar value (code points U+0000 to U+10FFFF, excluding surrogates). Characters are stored as little-endian 32-bit integers.

```rust
use byteable::{IntoByteArray, TryFromByteArray};

// ASCII character
let ch = 'A';
let bytes = ch.into_byte_array();
assert_eq!(bytes, [0x41, 0x00, 0x00, 0x00]); // Little-endian U+0041

// Unicode emoji
let ch = '🦀';
let bytes = ch.into_byte_array();
assert_eq!(bytes, [0x80, 0xF9, 0x01, 0x00]); // Little-endian U+1F980

// Roundtrip conversion
let restored = char::try_from_byte_array([0x41, 0x00, 0x00, 0x00]).unwrap();
assert_eq!(restored, 'A');

// Invalid code points return errors
let result = char::try_from_byte_array([0xFF, 0xFF, 0xFF, 0xFF]);
assert!(result.is_err()); // Not a valid Unicode scalar value
```

#### Using `bool` and `char` in Structs

```rust
use byteable::{Byteable, TryFromByteArray};

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
struct Config {
    enabled: bool,
    mode: char,
    #[byteable(little_endian)]
    port: u16,
}

fn main() -> Result<(), byteable::EnumFromBytesError> {
    let config = Config {
        enabled: true,
        mode: 'A',
        port: 8080,
    };

    let bytes = config.into_byte_array();

    // Must use try_from_byte_array due to validation
    let restored = Config::try_from_byte_array(bytes)?;
    assert_eq!(restored, config);

    Ok(())
}
```

**Important Notes:**

- Use `TryFromByteArray` instead of `FromByteArray` for types containing `bool` or `char`
- `bool` only accepts `0` (false) or `1` (true)
- `char` validates against Unicode scalar values (excludes surrogates and values > U+10FFFF)
- Characters are always stored as little-endian 32-bit values

## Enum Support

The `#[derive(Byteable)]` macro now supports C-like enums with explicit discriminants! This is perfect for encoding protocol status codes, command types, and other enumerated values in binary formats.

### Basic Enum Usage

```rust
use byteable::{Byteable, IntoByteArray, TryFromByteArray};

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]  // Required: explicit repr type
enum Status {
    Idle = 0,
    Running = 1,
    Completed = 2,
    Failed = 3,
}

fn main() -> Result<(), byteable::EnumFromBytesError> {
    let status = Status::Running;
    let bytes = status.into_byte_array();
    assert_eq!(bytes, [1]);

    // Convert back (fallible because not all bytes are valid)
    let restored = Status::try_from_byte_array(bytes)?;
    assert_eq!(restored, Status::Running);

    // Invalid discriminants return an error
    let invalid = Status::try_from_byte_array([255]);
    assert!(invalid.is_err());

    Ok(())
}
```

### Enum with Endianness

Enums support the same endianness attributes as structs:

```rust
use byteable::Byteable;

// Little-endian enum (common for file formats)
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u16)]
#[byteable(little_endian)]
enum FileType {
    Text = 0x1000,
    Binary = 0x2000,
    Archive = 0x3000,
}

// Big-endian enum (common for network protocols)
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
#[byteable(big_endian)]
enum HttpStatus {
    Ok = 200,
    NotFound = 404,
    InternalError = 500,
}

fn main() {
    let file_type = FileType::Binary;
    let bytes = file_type.into_byte_array();
    // Always [0x00, 0x20] regardless of platform
    assert_eq!(bytes, [0x00, 0x20]);

    let status = HttpStatus::Ok;
    let bytes = status.into_byte_array();
    // Always [0x00, 0x00, 0x00, 0xC8] regardless of platform
    assert_eq!(bytes, [0x00, 0x00, 0x00, 0xC8]);
}
```

### Enum Requirements

When deriving `Byteable` for enums, you **must** ensure:

1. **Explicit repr type**: Use `#[repr(u8)]`, `#[repr(u16)]`, `#[repr(u32)]`, `#[repr(u64)]`,
   `#[repr(i8)]`, `#[repr(i16)]`, `#[repr(i32)]`, or `#[repr(i64)]`
2. **Unit variants only**: All variants must be unit variants (no fields)
3. **Explicit discriminants**: All variants must have explicit discriminant values
4. **Error handling**: Use `TryFromByteArray` instead of `FromByteArray` since invalid byte patterns return errors

### Sparse Enums

Enums with non-sequential discriminants are fully supported:

```rust
use byteable::Byteable;

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum Priority {
    Low = 1,
    Medium = 5,
    High = 10,
    Critical = 100,
}

// Only the defined discriminants are valid
assert_eq!(Priority::Low.into_byte_array(), [1]);
assert_eq!(Priority::Critical.into_byte_array(), [100]);

// Values 2, 3, 4, 6, 7, etc. will return errors
assert!(Priority::try_from_byte_array([2]).is_err());
```

## Usage Patterns

### Working with Different Endianness

```rust
use byteable::Byteable;

#[derive(Byteable, Clone, Copy)]
struct MixedEndianData {
    // Network protocols often use big-endian
    #[byteable(big_endian)]
    network_value: u32,

    // File formats often use little-endian
    #[byteable(little_endian)]
    file_value: u32,

    // Native endianness (matches system)
    native_value: u32,
}
```

### Reading Multiple Values

```rust
use byteable::ReadByteable;
use std::io::Cursor;

let data = vec![/* bytes */];
let mut reader = Cursor::new(data);

let header: u32 = reader.read_byteable()?;
let length: u16 = reader.read_byteable()?;
let checksum: u32 = reader.read_byteable()?;
```

## Safety Considerations

The `#[derive(Byteable)]` macro uses `unsafe` code (`core::mem::transmute`) internally. When using it, you **must** ensure:

### Safe to Use With:

- Primitive numeric types (`u8`, `i32`, `f64`, etc.)
- `bool` and `char` (with validation via `TryFromByteArray`)
- `BigEndian<T>` and `LittleEndian<T>` wrappers
- Arrays of safe types
- Structs with `#[repr(C, packed)]` or `#[repr(transparent)]`
- C-like enums with explicit discriminants (with validation via `TryFromByteArray`)

### **Never** Use With:

- Complex enums with fields (have invalid bit patterns)
- `String`, `Vec`, or any heap-allocated types
- References or pointers (`&T`, `Box<T>`, `*const T`)
- Types with `Drop` implementations
- `NonZero*` types or types with invariants

### Requirements:

1. **Explicit memory layout**: Always use `#[repr(C, packed)]` or similar
2. **All byte patterns valid**: Every possible byte combination must be valid for your type
3. **No padding with undefined values**: Use `packed` to avoid alignment padding
4. **No drop glue**: Types must be `Copy` and have no cleanup logic

## Documentation

The crate includes extensive documentation:

- **API Documentation**: Every trait, type, and function is documented with examples
- **Inline Comments**: All implementations include explanatory comments
- **Safety Guidelines**: Clear warnings about unsafe usage
- **Examples**: Multiple real-world usage examples in the [`examples/`](examples/) directory

Generate and view the documentation locally:

```bash
cargo doc --open --no-deps
```

## See Also

- [API Documentation](https://docs.rs/byteable)
- [Examples Directory](examples/)
- [Changelog](https://github.com/PaulDepping/byteable/releases)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Acknowledgments

Built with ❤️ for the Rust community.
