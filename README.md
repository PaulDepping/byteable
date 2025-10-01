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

### Basic `Byteable` Conversion

Implement the `Byteable` trait manually or use the `#[derive(Byteable)]` macro (with the `derive` feature enabled):

```rust
use byteable::{Byteable, ReadByteable, WriteByteable, LittleEndian};
use std::io::Cursor;

#[derive(Byteable, Debug, PartialEq, Copy, Clone)]
#[byteable(transparent)] // For single-field structs
struct MyCustomId(u32);

fn main() -> std::io::Result<()> {
    let id = MyCustomId(12345);
    let mut buffer = Vec::new();

    // Write to buffer
    buffer.write_one(id)?;

    // Read from buffer
    let mut reader = &buffer[..];
    let read_id: MyCustomId = reader.read_one()?;

    println!("Original ID: {:?}, Read ID: {:?}", id, read_id);
    assert_eq!(id, read_id);
    Ok(())
}
```

### Endianness Conversion

```rust
use byteable::{BigEndian, LittleEndian};

fn main() {
    let value: u32 = 0x01020304;

    // Convert to Big Endian
    let be_value = BigEndian::new(value);
    println!("Value in Big Endian: {:?}", be_value); // Will show 0x01020304


    // Convert to Little Endian
    let le_value = LittleEndian::new(value);
    println!("Value in Little Endian: {:?}", le_value); // Will show 0x04030201
}
```

### Asynchronous I/O (with `tokio` feature)

```rust
#[cfg(feature = "tokio")]
#[tokio::main]
async fn main() -> std::io::Result<()> {
    use byteable::{Byteable, AsyncReadByteable, AsyncWriteByteable};
    use tokio::io::Cursor;

    #[derive(Byteable, Debug, PartialEq, Copy, Clone)]
    struct SensorData {
        temperature: f32,
        humidity: u16,
    }

    let data = SensorData {
        temperature: 25.5,
        humidity: 60,
    };
    let mut buffer = Cursor::new(Vec::new());

    // Write asynchronously
    buffer.write_one(data).await?;

    // Reset cursor and read asynchronously
    buffer.set_position(0);
    let read_data: SensorData = buffer.read_one().await?;

    println!("Original Data: {:?}, Read Data: {:?}", data, read_data);
    assert_eq!(data, read_data);
    Ok(())
}
```

## Contributing

Feel free to open issues or submit pull requests.

## License

This project is licensed under MIT.
