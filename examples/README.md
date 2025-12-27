# Byteable Examples

This directory contains comprehensive examples demonstrating the usage of the `byteable` crate and its `UnsafeByteableTransmute` derive macro.

## Running the Examples

To run any example, use:

```bash
cargo run --example <example_name>
```

## Available Examples

### 1. `simple_usage.rs` - Basic Byteable Operations

**What it demonstrates:**

- Creating simple structs with the `UnsafeByteableTransmute` derive macro
- Converting structs to byte arrays
- Reconstructing structs from byte arrays
- Working with arrays of byteable structs
- Basic usage of `BigEndian` and `LittleEndian` wrappers

**Key concepts:**

- Sensor readings with mixed endianness
- RGB color structures
- Basic byte conversions

**Run with:**

```bash
cargo run --example simple_usage
```

---

### 2. `file_io.rs` - File-based I/O Operations

**What it demonstrates:**

- Writing byteable structs to binary files
- Reading byteable structs from binary files
- Random access with `Seek`
- Working with multiple different struct types in one file
- Practical use of `ReadByteable` and `WriteByteable` traits

**Key concepts:**

- Network packet serialization
- Device configuration storage
- File-based persistence
- Sequential and random access patterns

**Run with:**

```bash
cargo run --example file_io
```

**Note:** This example creates a file called `example_data.bin` in the current directory.

---

### 3. `cursor_usage.rs` - In-Memory Buffer Operations

**What it demonstrates:**

- Using `std::io::Cursor` for in-memory operations
- Building network protocol messages
- Parsing binary data buffers
- Working with multi-message packets
- Testing serialization without file I/O

**Key concepts:**

- Network protocol message headers
- Login request/response patterns
- Status codes and timestamps
- Buffer-based message queues

**Run with:**

```bash
cargo run --example cursor_usage
```

---

## Common Patterns

All examples demonstrate these important concepts:

### 1. Struct Requirements

To use `#[derive(UnsafeByteableTransmute)]`, your struct must:

- Be annotated with `#[repr(C, packed)]` or `#[repr(C)]`
- Implement `Copy`
- Have all fields be `Byteable` types (primitives or wrapped in `BigEndian`/`LittleEndian`)

Example:

```rust
#[derive(UnsafeByteable, Clone, Copy, PartialEq, Debug)]
#[repr(C, packed)]
struct MyStruct {
    field1: u16,
    field2: LittleEndian<u32>,
}
```

### 2. Endianness Control

Use `BigEndian<T>` or `LittleEndian<T>` wrappers:

```rust
let be_value = BigEndian::new(0x1234);   // Network byte order
let le_value = LittleEndian::new(0x1234); // Intel byte order

// Get the native value back
let native = le_value.get();
```

### 3. Byte Conversions

```rust
// Convert to bytes
let bytes = my_struct.to_byte_array();

// Convert from bytes
let reconstructed = MyStruct::from_byte_array(bytes);
```

### 4. I/O Operations

```rust
use byteable::{ReadByteable, WriteByteable};

// Write to any Writer (File, Cursor, TcpStream, etc.)
writer.write_byteable(my_struct)?;

// Read from any Reader
let my_struct: MyStruct = reader.read_byteable()?;
```

## Use Cases

The `byteable` crate is particularly useful for:

- **Network Protocols**: Parsing and creating binary network messages
- **Embedded Systems**: Low-level hardware communication with fixed layouts
- **File Formats**: Reading/writing binary file formats
- **Serialization**: Fast, zero-copy serialization for simple data structures
- **IPC**: Inter-process communication with shared memory or pipes
- **Game Development**: Network packet serialization, save file formats

## Additional Features

### Tokio Support

With the `tokio` feature enabled, you can use async I/O:

```rust
use byteable::{AsyncReadByteable, AsyncWriteByteable};

// Async read
let my_struct: MyStruct = async_reader.read_byteable().await?;

// Async write
async_writer.write_byteable(my_struct).await?;
```

## Tips and Best Practices

1. **Always use `#[repr(C, packed)]`** for predictable memory layout
2. **Be mindful of alignment** - packed structs can cause performance issues on some platforms
3. **Choose the right endianness** - use `BigEndian` for network protocols, `LittleEndian` for x86/ARM systems
4. **Add padding explicitly** when needed for alignment requirements
5. **Implement `Debug` and `PartialEq`** for easier testing and debugging
6. **Document field meanings** especially when dealing with packed binary formats

## Learn More

- Check the main [README.md](../README.md) for installation and feature information
- Read the [documentation](https://docs.rs/byteable) for API details
- View the [source code](../src/lib.rs) for implementation details
