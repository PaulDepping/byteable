# byteable

`byteable` is a Rust crate providing traits and utilities for seamless conversion between data structures and byte arrays, handling both synchronous and asynchronous I/O operations, and managing endianness.

## Features

- **`Byteable` Trait**: Define how your custom types can be converted into and from byte arrays.
- **`ReadByteable` & `WriteByteable` Traits**: Extend `std::io::Read` and `std::io::Write` to easily read and write `Byteable` types.
- **`AsyncReadByteable` & `AsyncWriteByteable` Traits (with `tokio` feature)**: Asynchronous versions of I/O traits for `Byteable` types, integrated with `tokio`.
- **`Endianable` Trait & Wrappers**: Utility for handling endianness conversion for primitive types, including `BigEndian` and `LittleEndian` wrappers.
- **`#[derive(Byteable)]` (with `derive` feature)**: A procedural macro to automatically implement the `Byteable` trait for your structs.

## Installation

Add `byteable` to your `Cargo.toml`:

```toml
[dependencies]
byteable = "0.1" # Or specify the latest version
```

To enable the `derive` macro and `tokio` integration, you can specify features:

```toml
[dependencies]
byteable = { version = "0.1", features = ["derive", "tokio"] }
```

## Usage

### Basic `Byteable` Implementation

```rust
use byteable::{Byteable, ReadByteable, WriteByteable};

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
