# byteable

[![Crates.io](https://img.shields.io/crates/v/byteable)](https://crates.io/crates/byteable)
[![docs.rs](https://img.shields.io/docsrs/byteable)](https://docs.rs/byteable)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

Byte-level serialization and deserialization for Rust types.

## What is byteable?

`byteable` gives you two paths for working with binary data:

- **Fixed-size path** — For types whose byte representation is known at compile time. Derive
  `Byteable` and get zero-copy `into_byte_array()` / `from_byte_array()` with a compile-time
  `BYTE_SIZE` constant. Under the hood this uses `#[repr(C, packed)]` raw structs and
  `transmute`, so no allocation or iteration is involved.

- **Dynamic path** — For types that contain variable-length data (strings, vecs, maps). Add
  `#[byteable(io_only)]` to derive `Readable` / `Writable` instead, which stream data through
  any `std::io::Read` / `Write` (or their async `tokio` equivalents).

Both paths share the same derive macro and attribute syntax, and both produce deterministic,
self-describing wire formats.

## Installation

```toml
[dependencies]
# default: derive macro + std I/O support
byteable = "0.31"

# with async (tokio) support
byteable = { version = "0.31", features = ["tokio"] }

# with ordered-float support
byteable = { version = "0.31", features = ["ordered-float"] }

# everything
byteable = { version = "0.31", features = ["all"] }
```

## Quick Start

### Fixed-size struct (zero-copy)

```rust
use byteable::{Byteable, IntoByteArray, TryFromByteArray};

#[derive(Byteable)]
struct Point3D {
    x: f32,
    y: f32,
    z: f32,
}

let p = Point3D { x: 1.0, y: 2.0, z: 3.0 };

// Serialize to a fixed-size byte array — no allocation
let bytes: [u8; 12] = p.into_byte_array();

// Deserialize back
let p2 = Point3D::try_from_byte_array(bytes).unwrap();
assert_eq!(p.x, p2.x);
```

### Dynamic struct (I/O streaming)

```rust
use byteable::{Byteable, Writable, Readable};
use byteable::io::{WriteValue, ReadValue};

#[derive(Byteable)]
#[byteable(io_only)]
struct Waypoint {
    id: u32,
    label: String,
    tags: Vec<String>,
}

let wp = Waypoint {
    id: 42,
    label: "home".into(),
    tags: vec!["start".into()],
};

let mut buf = Vec::new();
buf.write_value(&wp).unwrap();

let wp2 = std::io::Cursor::new(&buf).read_value::<Waypoint>().unwrap();
assert_eq!(wp.id, wp2.id);
assert_eq!(wp.label, wp2.label);
```

### Controlling endianness

```rust
use byteable::Byteable;

#[derive(Byteable)]
#[byteable(big_endian)]          // default for all fields
struct NetworkHeader {
    #[byteable(big_endian)]
    magic: u32,
    #[byteable(little_endian)]   // field-level override
    payload_len: u16,
    version: u8,
}
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `derive` | yes | `#[derive(Byteable)]` proc-macro |
| `std` | yes | `Readable` / `Writable` I/O traits and `std` type impls |
| `tokio` | no | Async `AsyncReadable` / `AsyncWritable` via tokio |
| `ordered-float` | no | Impls for `OrderedFloat<T>` and `NotNan<T>` |
| `all` | no | Enable all of the above |

## Wire Format Reference

| Type | Encoding |
|------|----------|
| `u8`, `i8` | 1 byte, identity |
| `u16`…`u128`, `i16`…`i128` | little-endian by default (overridable) |
| `f32`, `f64` | little-endian IEEE 754 by default |
| `bool` | 1 byte: `0` = false, `1` = true |
| `char` | 4 bytes little-endian `u32` (Unicode scalar value) |
| `NonZero<T>` | same as `T`; decoding rejects zero |
| `Option<T>` | 1-byte tag (`0` = None, `1` = Some) + optional value |
| `Result<V, E>` | 1-byte tag (`0` = Ok, `1` = Err) + payload |
| `String` / `str` | `u64` byte length + UTF-8 bytes |
| `Vec<T>` and other sequences | `u64` element count + elements |
| `HashMap<K,V>` / `BTreeMap<K,V>` | `u64` entry count + alternating key/value pairs |
| `PathBuf` / `Path` | same as `String`; non-UTF-8 paths produce an error |
| `CString` / `CStr` | same as `Vec<u8>` (bytes without null terminator) |
| `Duration` | `u64` secs + `u32` nanos |
| `SystemTime` | `i64` secs relative to Unix epoch + `u32` nanos |
| `Ipv4Addr` | 4 bytes (network octet order) |
| `Ipv6Addr` | 16 bytes (network octet order) |
| `SocketAddrV4` | `Ipv4Addr` + `u16` port (LE) |
| `SocketAddrV6` | `Ipv6Addr` + `u16` port (LE) + `u32` flowinfo (LE) + `u32` scope_id (LE) |
| `Arc<T>` / `Rc<T>` / `Box<T>` | transparent passthrough to inner type |
| `[T; N]` | N consecutive encodings of `T` |
| `Range<T>` / `RangeInclusive<T>` | start + end |
| `RangeFrom<T>` / `RangeTo<T>` / `RangeToInclusive<T>` | single bound |
| `RangeFull` | 0 bytes |
| `PhantomData<T>` | 0 bytes |

## Trait Reference

### Fixed-size byte-array traits

These traits form the fixed-size serialization pipeline. `#[derive(Byteable)]` generates
impls for all of them automatically.

| Trait | Role |
|-------|------|
| [`IntoByteArray`] | Serialize to a `[u8; N]`; provides the compile-time `BYTE_SIZE` constant |
| [`FromByteArray`] | Infallible deserialization from a `[u8; N]` |
| [`TryFromByteArray`] | Fallible deserialization from a `[u8; N]` (returns [`DecodeError`]) |

[`IntoByteArray`]: https://docs.rs/byteable/latest/byteable/trait.IntoByteArray.html
[`FromByteArray`]: https://docs.rs/byteable/latest/byteable/trait.FromByteArray.html
[`TryFromByteArray`]: https://docs.rs/byteable/latest/byteable/trait.TryFromByteArray.html

### Raw representation traits

The "raw repr" layer sits between a typed value and its final bytes. It lets the derive
macro insert endian wrappers transparently before transmuting.

| Trait | Role |
|-------|------|
| [`RawRepr`] | Convert `Self` to a [`PlainOldData`] wire type (e.g. wrap a `u32` in `LittleEndian<u32>`) |
| [`FromRawRepr`] | Infallible conversion from the raw type back to `Self` |
| [`TryFromRawRepr`] | Fallible conversion from the raw type back to `Self` |

[`RawRepr`]: https://docs.rs/byteable/latest/byteable/trait.RawRepr.html
[`FromRawRepr`]: https://docs.rs/byteable/latest/byteable/trait.FromRawRepr.html
[`TryFromRawRepr`]: https://docs.rs/byteable/latest/byteable/trait.TryFromRawRepr.html

### I/O streaming traits (`std` feature)

These traits power the `#[byteable(io_only)]` derive path and the `std` collection impls.

| Trait | Role |
|-------|------|
| [`Readable`] | Read a (possibly variable-length) value from any `std::io::Read` |
| [`Writable`] | Write a (possibly variable-length) value to any `std::io::Write` |
| [`FixedReadable`] | Read a fixed-size value; blanket impl for all `TryFromRawRepr` types |
| [`FixedWritable`] | Write a fixed-size value; blanket impl for all `RawRepr` types |

Extension traits that add ergonomic methods to any reader/writer:

| Trait | Added method | Works on |
|-------|-------------|---------|
| [`ReadValue`] | `.read_value::<T>()` | any `Read` |
| [`WriteValue`] | `.write_value(&val)` | any `Write` |
| [`ReadFixed`] | `.read_fixed::<T>()` | any `Read` |
| [`WriteFixed`] | `.write_fixed(&val)` | any `Write` |

[`Readable`]: https://docs.rs/byteable/latest/byteable/trait.Readable.html
[`Writable`]: https://docs.rs/byteable/latest/byteable/trait.Writable.html
[`FixedReadable`]: https://docs.rs/byteable/latest/byteable/trait.FixedReadable.html
[`FixedWritable`]: https://docs.rs/byteable/latest/byteable/trait.FixedWritable.html
[`ReadValue`]: https://docs.rs/byteable/latest/byteable/trait.ReadValue.html
[`WriteValue`]: https://docs.rs/byteable/latest/byteable/trait.WriteValue.html
[`ReadFixed`]: https://docs.rs/byteable/latest/byteable/trait.ReadFixed.html
[`WriteFixed`]: https://docs.rs/byteable/latest/byteable/trait.WriteFixed.html

### Async I/O traits (`tokio` feature)

Async counterparts of the sync traits above, backed by `tokio::io`.

| Trait | Async counterpart of |
|-------|---------------------|
| [`AsyncReadable`] | [`Readable`] |
| [`AsyncWritable`] | [`Writable`] |
| [`AsyncFixedReadable`] | [`FixedReadable`] |
| [`AsyncFixedWritable`] | [`FixedWritable`] |
| [`AsyncReadValue`] | [`ReadValue`] |
| [`AsyncWriteValue`] | [`WriteValue`] |
| [`AsyncReadFixed`] | [`ReadFixed`] |
| [`AsyncWriteFixed`] | [`WriteFixed`] |

[`AsyncReadable`]: https://docs.rs/byteable/latest/byteable/trait.AsyncReadable.html
[`AsyncWritable`]: https://docs.rs/byteable/latest/byteable/trait.AsyncWritable.html
[`AsyncFixedReadable`]: https://docs.rs/byteable/latest/byteable/trait.AsyncFixedReadable.html
[`AsyncFixedWritable`]: https://docs.rs/byteable/latest/byteable/trait.AsyncFixedWritable.html
[`AsyncReadValue`]: https://docs.rs/byteable/latest/byteable/trait.AsyncReadValue.html
[`AsyncWriteValue`]: https://docs.rs/byteable/latest/byteable/trait.AsyncWriteValue.html
[`AsyncReadFixed`]: https://docs.rs/byteable/latest/byteable/trait.AsyncReadFixed.html
[`AsyncWriteFixed`]: https://docs.rs/byteable/latest/byteable/trait.AsyncWriteFixed.html

### Endianness traits

These traits underpin per-field endian control in the derive macro and the
[`BigEndian<T>`] / [`LittleEndian<T>`] wrapper types.

| Trait / Type | Role |
|--------------|------|
| [`EndianConvert`] | Marker for multi-byte primitives that support byte-swapping (`u16`–`u128`, `i16`–`i128`, `f32`, `f64`) |
| [`BigEndian<T>`] | Transparent wrapper storing `T` in big-endian byte order |
| [`LittleEndian<T>`] | Transparent wrapper storing `T` in little-endian byte order |
| [`HasEndianRepr`] | Provides `LE` / `BE` associated types and `to_little_endian()` / `to_big_endian()` |
| [`FromEndianRepr`] | Infallible conversion back from an endian-specific repr |
| [`TryFromEndianRepr`] | Fallible conversion back from an endian-specific repr |

[`EndianConvert`]: https://docs.rs/byteable/latest/byteable/trait.EndianConvert.html
[`BigEndian<T>`]: https://docs.rs/byteable/latest/byteable/struct.BigEndian.html
[`LittleEndian<T>`]: https://docs.rs/byteable/latest/byteable/struct.LittleEndian.html
[`HasEndianRepr`]: https://docs.rs/byteable/latest/byteable/trait.HasEndianRepr.html
[`FromEndianRepr`]: https://docs.rs/byteable/latest/byteable/trait.FromEndianRepr.html
[`TryFromEndianRepr`]: https://docs.rs/byteable/latest/byteable/trait.TryFromEndianRepr.html

### Low-level traits

| Trait | Role |
|-------|------|
| [`PlainOldData`] | Unsafe marker: no padding, all bit patterns valid — enables `transmute`-based I/O |
| [`ByteArray`] | Unsafe marker for `[u8; N]` used as the `IntoByteArray::ByteArray` associated type |

[`PlainOldData`]: https://docs.rs/byteable/latest/byteable/trait.PlainOldData.html
[`ByteArray`]: https://docs.rs/byteable/latest/byteable/trait.ByteArray.html

### Error types

| Type | When it occurs |
|------|---------------|
| [`DecodeError`] | Bytes decoded successfully but the value is invalid (bad discriminant, NaN, interior null, etc.) |
| [`ReadableError`] | An I/O error or [`DecodeError`] while reading from a `Read` / async reader |

[`DecodeError`]: https://docs.rs/byteable/latest/byteable/enum.DecodeError.html
[`ReadableError`]: https://docs.rs/byteable/latest/byteable/enum.ReadableError.html

## License

MIT — see [LICENSE](LICENSE).
