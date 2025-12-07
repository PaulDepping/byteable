_A Rust crate for convenient serialization and deserialization of byte-oriented data._

`byteable` is a Rust crate providing traits and utilities for seamless conversion between data structures and byte arrays, handling both synchronous and asynchronous I/O operations, and managing endianness.

## Features

- **`Byteable` Trait**: The core trait for types that can be converted to and from a byte array.
- **`ReadByteable` & `WriteByteable` Traits**: Extension traits for `std::io::Read` and `std::io::Write`, enabling convenient reading and writing of `Byteable` types.
- **`AsyncReadByteable` & `AsyncWriteByteable` Traits (with `tokio` feature)**: Asynchronous counterparts to `ReadByteable` and `WriteByteable`, designed for use with `tokio`'s async I/O.
- **`Endianable` Trait & Wrappers**: Provides methods for converting primitive types between different endianness (little-endian and big-endian), along with `BigEndian<T>` and `LittleEndian<T>` wrapper types.
- **`#[derive(Byteable)]` (with `derive` feature)**: A procedural macro that automatically implements the `Byteable` trait for structs, significantly simplifying boilerplate.

## Installation

Add `byteable` to your `Cargo.toml`:

```toml
[dependencies]
byteable = "*" # Or specify the latest version
```

To enable the `derive` macro and `tokio` integration, you can specify features:

```toml
[dependencies]
byteable = { version = "*", features = ["derive", "tokio"] }
```

## Usage

Here's a quick example demonstrating basic usage with file I/O:

```rust
use byteable::{Byteable, LittleEndian, ReadByteable, WriteByteable};
use std::fs::File;

#[derive(Byteable, Debug, PartialEq)]
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
        length: LittleEndian::new(1024),
        data: [0xDE, 0xAD, 0xBE, 0xEF],
    };

    // Write packet to a file
    let mut file = File::create("packet.bin")?;
    file.write_one(packet)?;
    println!("Packet written to file");

    // Read packet back from file
    let mut file = File::open("packet.bin")?;
    let restored: Packet = file.read_one()?;

    assert_eq!(packet, restored);
    println!("Packet successfully read back: {:?}", restored);

    Ok(())
}
```

The same `ReadByteable` and `WriteByteable` traits work with any `Read`/`Write` implementor, including TCP streams, in-memory buffers, and more. For more examples, check out the [`examples/`](examples/) directory.

## Contributing

Feel free to open issues or submit pull requests.

## License

This project is licensed under MIT.
