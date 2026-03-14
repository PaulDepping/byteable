# Byteable

[![Crates.io](https://img.shields.io/crates/v/byteable.svg)](https://crates.io/crates/byteable)
[![Documentation](https://docs.rs/byteable/badge.svg)](https://docs.rs/byteable)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

_A Rust crate for zero-overhead, zero-copy serialization and deserialization of byte-oriented data._

`byteable` provides traits and utilities for seamless conversion between data structures and byte arrays, with full support for both synchronous and asynchronous I/O operations, and comprehensive endianness handling.

## Features

- **Byte Conversion Traits**: Modular trait system for byte array conversion:
  - `ByteRepr`: Associates a type with its byte array representation
  - `IntoByteArray`: Converts values into byte arrays
  - `FromByteArray`: Constructs values from byte arrays
  - `TryFromByteArray`: Fallible deserialization for types that can fail (e.g., `bool`, `char`, enums)
- **`ReadValue` & `WriteValue`**: Extension traits for `std::io::Read` and `std::io::Write`
- **`AsyncReadValue` & `AsyncWriteValue`**: Async I/O support with tokio (optional)
- **Endianness Support**: `BigEndian<T>` and `LittleEndian<T>` wrappers for explicit byte order
- **`#[derive(Byteable)]`**: Procedural macro for automatic trait implementation with endianness support (optional):
  - Fixed-size structs via zero-copy transmute
  - `#[byteable(io_only)]` structs for types containing `Vec`, `String`, `Option`, etc.
  - C-like enums and enums with variant fields
- **Standard Collection I/O**: Built-in `Readable`/`Writable` for `Vec`, `String`, `Option`, `HashMap`, `BTreeMap`, and more
- **Zero Overhead**: Fixed-size types compile down to simple memory operations with no runtime cost

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
byteable = "0.25"  # Or latest version
```

### Optional Features

```toml
[dependencies]
byteable = { version = "0.25", features = ["derive", "tokio"] }
```

- **`derive`** (default): Enables the `#[derive(Byteable)]` procedural macro
- **`tokio`**: Enables async I/O traits for use with tokio

## Quick Start

### Basic File I/O Example

```rust
use byteable::{Byteable, LittleEndian, ReadValue, WriteValue};
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
    file.write_value(&packet)?;
    println!("Packet written to file");

    // Read packet back from file
    let mut file = File::open("packet.bin")?;
    let restored: Packet = file.read_value()?;

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
use byteable::{AsyncReadValue, AsyncWriteValue, Byteable};
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
    stream.write_value(&msg).await?;

    // Async read
    let response: Message = stream.read_value().await?;

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

fn main() -> Result<(), byteable::InvalidDiscriminantError> {
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

The `#[derive(Byteable)]` macro supports two kinds of enums: **C-like enums** (unit variants only, with fixed-size byte array conversion) and **field enums** (variants with data, using stream-based I/O).

### C-Like Enums

C-like enums (unit variants with explicit discriminants) implement `IntoByteArray` / `TryFromByteArray` for zero-copy fixed-size conversion.

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

fn main() -> Result<(), byteable::InvalidDiscriminantError> {
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

// Only the defined discriminants are valid; all others return errors
assert_eq!(Priority::Low.into_byte_array(), [1]);
assert!(Priority::try_from_byte_array([2]).is_err());
```

### Enums with Fields

Enums with variant fields (named or tuple) implement `Readable` / `Writable` for stream-based I/O. The discriminant is written first, followed by the variant's fields in order.

```rust
use byteable::{Byteable, ReadValue, WriteValue};
use std::io::Cursor;

#[derive(Byteable, Debug, PartialEq)]
#[repr(u8)]
enum Message {
    Ping = 0,
    Pong { id: u8 } = 1,
    Data { length: u8, value: [u8; 4] } = 2,
}

let original = Message::Data { length: 4, value: [0xDE, 0xAD, 0xBE, 0xEF] };

let mut buf = Vec::new();
buf.write_value(&original).unwrap();
assert_eq!(buf, [2, 4, 0xDE, 0xAD, 0xBE, 0xEF]); // discriminant + fields

let decoded: Message = Cursor::new(&buf).read_value().unwrap();
assert_eq!(decoded, original);
```

Discriminants and fields both support endianness annotations:

```rust
use byteable::Byteable;

// Little-endian u16 discriminant
#[derive(Byteable, Debug, PartialEq)]
#[repr(u16)]
#[byteable(little_endian)]
enum Request {
    Ping = 0x0001,
    GetValue { key: u8 } = 0x0002,
    SetValue { key: u8, val: u8 } = 0x0003,
}

// Individual fields can have per-field endianness
#[derive(Byteable, Debug, PartialEq)]
#[repr(u8)]
enum Typed {
    Small { val: u8 } = 0,
    Wide { #[byteable(little_endian)] val: u32 } = 1,
    Network {
        #[byteable(big_endian)] port: u16,
        #[byteable(big_endian)] addr: u32,
    } = 2,
}
```

If `#[repr]` is omitted, the macro infers the smallest integer type that fits all variants (e.g. `u8` for up to 255 variants), and discriminants auto-increment from 0 like ordinary Rust enums.

### Enum Endianness (C-like)

C-like enums also support type-level endianness for their fixed-size representation:

```rust
use byteable::Byteable;

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u16)]
#[byteable(little_endian)]
enum FileType {
    Text = 0x1000,
    Binary = 0x2000,
}

let bytes = FileType::Binary.into_byte_array();
assert_eq!(bytes, [0x00, 0x20]); // little-endian, platform-independent
```

## `io_only` Structs

The standard `#[derive(Byteable)]` path uses `transmute`-based zero-copy conversion and requires every field to be a fixed-size, `TransmuteSafe` type. For structs that contain `Vec<T>`, `String`, `Option<T>`, or other dynamically-sized types, annotate the struct with `#[byteable(io_only)]` to generate sequential field I/O instead:

```rust
use byteable::{Byteable, ReadValue, WriteValue};
use std::io::Cursor;

#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct Packet {
    tag: u8,
    payload: Vec<u8>,
    label: String,
    optional: Option<u8>,
}

let original = Packet {
    tag: 1,
    payload: vec![0xDE, 0xAD, 0xBE, 0xEF],
    label: "hello".to_string(),
    optional: Some(42),
};

let mut buf = Vec::new();
buf.write_value(&original).unwrap();

let decoded: Packet = Cursor::new(&buf).read_value().unwrap();
assert_eq!(decoded, original);
```

`io_only` structs implement `Readable` / `Writable` (not `IntoByteArray` / `FromByteArray`), so they always require a reader or writer. Fields are written in declaration order. Field-level endianness attributes still apply:

```rust
#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct MixedPacket {
    #[byteable(big_endian)]
    port: u16,          // written as big-endian u16
    payload: Vec<u8>,   // length-prefixed sequence
}
```

Tuple structs and unit structs are also supported with `#[byteable(io_only)]`.

## Standard Collection I/O

The `Readable` and `Writable` traits are implemented for common standard library collection types. All collections are serialized as a **little-endian `u64` length** (number of elements) followed by each element in sequence.

| Type | Notes |
|---|---|
| `Vec<T>` | Sequential elements |
| `VecDeque<T>` | Sequential elements |
| `LinkedList<T>` | Sequential elements |
| `HashMap<K, V>` | Sequential key-value pairs |
| `HashSet<T>` | Sequential elements |
| `BTreeMap<K, V>` | Sequential key-value pairs |
| `BTreeSet<T>` | Sequential elements |
| `Option<T>` | `0u8` for `None`, `1u8` + value for `Some` |
| `Result<V, E>` | `0u8` + value for `Ok`, `1u8` + error for `Err` |
| `String` | UTF-8 bytes prefixed by a little-endian `u64` byte-length |
| `Path` / `PathBuf` | Same encoding as `String` (UTF-8 path) |
| `CStr` / `CString` | Null-terminated bytes |

These implementations are used automatically by `ReadValue::read_value` / `WriteValue::write_value` and are composed transparently within `io_only` structs and field enums.

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
use byteable::ReadValue;
use std::io::Cursor;

let data = vec![/* bytes */];
let mut reader = Cursor::new(data);

let header: u32 = reader.read_value()?;
let length: u16 = reader.read_value()?;
let checksum: u32 = reader.read_value()?;
```

## Safety Considerations

`#[derive(Byteable)]` uses two distinct code-generation paths with different safety profiles:

### Transmute path (default)

Used for ordinary structs and C-like enums. Internally uses `core::mem::transmute`, so every field must be a fixed-size, `TransmuteSafe` type.

**Safe to use:**

- Primitive numeric types (`u8`, `i32`, `f64`, etc.)
- `bool` and `char` (with validation via `TryFromByteArray`)
- `BigEndian<T>` and `LittleEndian<T>` wrappers
- Arrays of the above
- C-like enums with explicit discriminants

**Never use on the transmute path:**

- `String`, `Vec`, or any heap-allocated types — use `#[byteable(io_only)]` instead
- References or pointers (`&T`, `Box<T>`, `*const T`)
- Types with `Drop` implementations
- `NonZero*` types or types with invariants

### `io_only` / field-enum path

Used for `#[byteable(io_only)]` structs and enums with variant fields. No `transmute` is involved — values are read/written field by field via the `Readable`/`Writable` traits. Standard library collection types (`Vec`, `String`, `Option`, `HashMap`, etc.) are fully supported on this path.

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
