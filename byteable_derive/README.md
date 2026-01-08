# `byteable_derive`

This crate provides custom `derive` macros for the [`byteable`](https://crates.io/crates/byteable) crate.

## Available Derives

### `#[derive(Byteable)]`

The main derive macro that automatically implements byte conversion traits for your types. Supports:

- **Structs** (named, tuple, and unit structs)
- **Enums** (C-like enums with explicit discriminants)
- **Endianness control** via field and type-level attributes
- **Nested byteable types** via `transparent` and `try_transparent` attributes

## Struct Support

### Basic Struct

```rust
use byteable::Byteable;

#[derive(Byteable, Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}
```

### Struct with Endianness

```rust
use byteable::Byteable;

#[derive(Byteable, Clone, Copy)]
struct NetworkPacket {
    version: u8,
    #[byteable(big_endian)]
    length: u16,
    #[byteable(little_endian)]
    checksum: u32,
}
```

### Field Attributes

- `#[byteable(big_endian)]` - Store field in big-endian byte order
- `#[byteable(little_endian)]` - Store field in little-endian byte order
- `#[byteable(transparent)]` - Use field's raw representation (for nested `Byteable` types)
- `#[byteable(try_transparent)]` - Use field's raw representation with fallible conversion

## Enum Support

The `#[derive(Byteable)]` macro supports C-like enums with explicit discriminants.

### Basic Enum

```rust
use byteable::Byteable;

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum Status {
    Idle = 0,
    Running = 1,
    Completed = 2,
    Failed = 3,
}
```

### Enum with Endianness

Enums support type-level endianness attributes:

```rust
use byteable::Byteable;

// Little-endian (for file formats)
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u16)]
#[byteable(little_endian)]
enum FileType {
    Text = 0x1000,
    Binary = 0x2000,
    Archive = 0x3000,
}

// Big-endian (for network protocols)
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
#[byteable(big_endian)]
enum HttpStatus {
    Ok = 200,
    NotFound = 404,
    InternalError = 500,
}
```

### Enum Requirements

When deriving `Byteable` for enums, you **must**:

1. Use an explicit repr type: `#[repr(u8)]`, `#[repr(u16)]`, `#[repr(u32)]`, `#[repr(u64)]`,
   `#[repr(i8)]`, `#[repr(i16)]`, `#[repr(i32)]`, or `#[repr(i64)]`
2. Have only unit variants (no fields)
3. Provide explicit discriminant values for all variants
4. Use `TryFromByteArray` for deserialization (returns `EnumFromBytesError` for invalid discriminants)

### Type-Level Attributes for Enums

- `#[byteable(big_endian)]` - Store enum discriminant in big-endian byte order
- `#[byteable(little_endian)]` - Store enum discriminant in little-endian byte order
- No attribute - Use native endianness

### Error Handling

Enums use fallible conversion because not all byte patterns represent valid enum variants:

```rust
use byteable::{Byteable, TryFromByteArray};

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum Command {
    Start = 1,
    Stop = 2,
}

fn example() -> Result<(), byteable::EnumFromBytesError> {
    // Valid conversion
    let bytes = [1];
    let cmd = Command::try_from_byte_array(bytes)?;
    assert_eq!(cmd, Command::Start);

    // Invalid discriminant returns error
    let invalid = [255];
    let result = Command::try_from_byte_array(invalid);
    assert!(result.is_err());

    Ok(())
}
```

## Advanced Usage

### Nested Structs with Enums

```rust
use byteable::Byteable;

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum MessageType {
    Data = 1,
    Control = 2,
    Error = 3,
}

#[derive(Byteable, Clone, Copy)]
struct Message {
    #[byteable(try_transparent)]
    msg_type: MessageType,
    #[byteable(big_endian)]
    sequence: u32,
    payload: [u8; 16],
}
```

### Sparse Discriminants

Enums with non-sequential discriminants work perfectly:

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

// Only defined discriminants (1, 5, 10, 100) are valid
// All other values return errors during conversion
```

## See Also

For comprehensive documentation and examples, see the main [`byteable` crate documentation](https://docs.rs/byteable).
