# Byteable

[![Crates.io](https://img.shields.io/crates/v/byteable.svg)](https://crates.io/crates/byteable)
[![Documentation](https://docs.rs/byteable/badge.svg)](https://docs.rs/byteable)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

_A Rust crate for zero-overhead, zero-copy serialization and deserialization of byte-oriented data._

`byteable` provides traits and utilities for seamless conversion between data structures and byte arrays, with full support for both synchronous and asynchronous I/O operations, and comprehensive endianness handling.

## Features

- **`Byteable` Trait**: Core trait for types that can be converted to/from byte arrays
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
byteable = "0.16"  # Or latest version
```

### Optional Features

```toml
[dependencies]
byteable = { version = "0.16", features = ["derive", "tokio"] }
```

- **`derive`** (default): Enables the `#[derive(UnsafeByteableTransmute)]` procedural macro
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
let bytes = header.as_byte_array();
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

The `#[derive(Byteable)]` macro uses `unsafe` code (`std::mem::transmute`) internally. When using it, you **must** ensure:

### Safe to Use With:

- Primitive numeric types (`u8`, `i32`, `f64`, etc.)
- `BigEndian<T>` and `LittleEndian<T>` wrappers
- Arrays of safe types
- Structs with `#[repr(C, packed)]` or `#[repr(transparent)]`

### **Never** Use With:

- `bool`, `char`, or enums (have invalid bit patterns)
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
