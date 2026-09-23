# byteable_derive

[![Crates.io](https://img.shields.io/crates/v/byteable_derive)](https://crates.io/crates/byteable_derive)
[![docs.rs](https://img.shields.io/docsrs/byteable_derive)](https://docs.rs/byteable_derive)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](../LICENSE)

Procedural derive macro for the [`byteable`](https://crates.io/crates/byteable) crate.

This crate provides the `#[derive(Byteable)]` macro. You almost certainly want to depend on
`byteable` directly (which re-exports this macro via the `derive` feature) rather than adding
`byteable_derive` as a direct dependency.

```toml
[dependencies]
byteable = "0.37"          # includes #[derive(Byteable)] by default
```

## What `#[derive(Byteable)]` generates

Applying the macro to a type generates byte-serialization trait impls automatically.
The exact traits depend on the type and attributes used:

| Type / attribute | Generated traits |
|-----------------|-----------------|
| Struct (default) | `RawRepr`, `FromRawRepr`/`TryFromRawRepr`, `ToByteArray`, `FromByteArray`/`TryFromByteArray` |
| Struct `#[byteable(io_only)]` | `Readable`, `Writable` |
| Unit enum | `TryFromRawRepr`, `ToByteArray`, `TryFromByteArray` |
| Field enum | `Readable`, `Writable` |

## Attributes

### Struct / enum level

| Attribute | Effect |
|-----------|--------|
| `#[byteable(little_endian)]` | All multi-byte fields use little-endian representation |
| `#[byteable(big_endian)]` | All multi-byte fields use big-endian representation (on an enum, applies only to the discriminant) |
| `#[byteable(io_only)]` | Generate `Readable`/`Writable` instead of fixed-size traits |
| `#[byteable(discriminant = uN/iN)]` | Enum only: set the discriminant's wire width, independently of any `#[repr(...)]` |
| `#[byteable(fingerprint = "...")]` / `#[byteable(fingerprint)]` | Assert the type's compile-time `WireFingerprint` hash, to catch accidental wire-format drift |

### Field level

| Attribute | Effect |
|-----------|--------|
| `#[byteable(little_endian)]` | This field uses little-endian (overrides struct-level) |
| `#[byteable(big_endian)]` | This field uses big-endian (overrides struct-level) |
| `#[byteable(try_transparent)]` | Field decode may fail; struct impl becomes `TryFromRawRepr` |
| `#[byteable(order = N)]` | Struct field only: pin this field's wire position to `N`, independently of declaration order |

### Variant level

| Attribute | Effect |
|-----------|--------|
| `#[byteable(tag = <expr>)]` | Pin this variant's wire discriminant to `<expr>`, independently of declaration position and of any real Rust `= N` discriminant |

See the [`byteable` README](https://github.com/PaulDepping/byteable#readme) for the full
reference on `order`/`tag`/`discriminant`/`fingerprint`, including examples.

## Examples

### Fixed-size struct

```rust
use byteable::{Byteable, ToByteArray, TryFromByteArray};

#[derive(Byteable)]
struct Point {
    x: f32,
    y: f32,
}

let p = Point { x: 1.0, y: 2.0 };
let bytes: [u8; 8] = p.to_byte_array();
let p2 = Point::try_from_byte_array(bytes).unwrap();
assert_eq!(p.x, p2.x);
```

### Endianness control

```rust
use byteable::Byteable;

#[derive(Byteable)]
#[byteable(big_endian)]
struct NetworkHeader {
    magic: u32,
    #[byteable(little_endian)]  // field-level override
    payload_len: u16,
    version: u8,
}
```

### Dynamic struct (`io_only`)

```rust
use byteable::{Byteable, Writable, Readable};
use byteable::io::{WriteValue, ReadValue};

#[derive(Byteable)]
#[byteable(io_only)]
struct Message {
    id: u32,
    body: String,
    tags: Vec<String>,
}
```

### Unit enum

```rust
use byteable::{Byteable, ToByteArray, TryFromByteArray};

#[derive(Byteable, Debug, PartialEq)]
enum Color { Red, Green, Blue }

// Auto-selected repr: u8 (3 variants fits in 1 byte)
assert_eq!(Color::BYTE_SIZE, 1);
let bytes = Color::Green.to_byte_array();
assert_eq!(Color::try_from_byte_array(bytes).unwrap(), Color::Green);
```

### Field enum

```rust
use byteable::{Byteable, Readable, Writable};
use byteable::io::{WriteValue, ReadValue};

#[derive(Byteable, Debug, PartialEq)]
enum Shape {
    Circle { radius: f32 },
    Rect   { width: f32, height: f32 },
}
```

## License

MIT - see [LICENSE](../LICENSE).
