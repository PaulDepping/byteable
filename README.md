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
- **`#[derive(UnsafeByteable)]`**: Procedural macro for automatic trait implementation (optional)
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
byteable = "0.14"  # Check crates.io for the latest version
```

### Optional Features

```toml
[dependencies]
byteable = { version = "0.14", features = ["derive", "tokio"] }
```

- **`derive`** (default): Enables the `#[derive(UnsafeByteable)]` procedural macro
- **`tokio`**: Enables async I/O traits for use with tokio

## Quick Start

### Basic File I/O Example

```rust
use byteable::{Byteable, LittleEndian, ReadByteable, WriteByteable};
use std::fs::File;

#[derive(byteable::UnsafeByteable, Debug, PartialEq)]
#[repr(C, packed)]
struct Packet {
    id: u8,
    length: LittleEndian<u16>,
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
use byteable::{Byteable, BigEndian, UnsafeByteable};

#[derive(UnsafeByteable, Debug)]
#[repr(C, packed)]
struct TcpHeader {
    source_port: BigEndian<u16>,      // Network byte order (big-endian)
    dest_port: BigEndian<u16>,
    sequence_num: BigEndian<u32>,
    ack_num: BigEndian<u32>,
}

let header = TcpHeader {
    source_port: 80.into(),
    dest_port: 8080.into(),
    sequence_num: 12345.into(),
    ack_num: 67890.into(),
};

// Convert to bytes for transmission
let bytes = header.as_byte_array();
```

### Async I/O with Tokio

```rust
use byteable::{AsyncReadByteable, AsyncWriteByteable, UnsafeByteable};
use tokio::net::TcpStream;

#[derive(UnsafeByteable, Debug)]
#[repr(C, packed)]
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
use byteable::{BigEndian, LittleEndian, UnsafeByteable};

#[derive(UnsafeByteable)]
#[repr(C, packed)]
struct MixedEndianData {
    // Network protocols often use big-endian
    network_value: BigEndian<u32>,

    // File formats often use little-endian
    file_value: LittleEndian<u32>,

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

### Custom Byteable Implementation

For types that need special handling, you can use the `impl_byteable_via!` macro:

```rust
use byteable::{Byteable, LittleEndian, UnsafeByteable, impl_byteable_via};

// Raw representation (for byte conversion)
#[derive(UnsafeByteable)]
#[repr(C, packed)]
struct PointRaw {
    x: LittleEndian<i32>,
    y: LittleEndian<i32>,
}

// User-friendly representation
#[derive(Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

// Implement conversions
impl From<Point> for PointRaw {
    fn from(p: Point) -> Self {
        Self { x: p.x.into(), y: p.y.into() }
    }
}

impl From<PointRaw> for Point {
    fn from(raw: PointRaw) -> Self {
        Self { x: raw.x.get(), y: raw.y.get() }
    }
}

// Now Point implements Byteable via PointRaw
impl_byteable_via!(Point => PointRaw);
```

## Safety Considerations

The `#[derive(UnsafeByteable)]` macro uses `unsafe` code (`std::mem::transmute`) internally. When using it, you **must** ensure:

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
