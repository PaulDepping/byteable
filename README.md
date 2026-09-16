# byteable

[![Crates.io](https://img.shields.io/crates/v/byteable)](https://crates.io/crates/byteable)
[![docs.rs](https://img.shields.io/docsrs/byteable)](https://docs.rs/byteable)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

Byte-level serialization and deserialization for Rust types.

## What is byteable?

`byteable` gives you two paths for working with binary data:

- **Fixed-size path** - For types whose byte representation is known at compile time. Derive
  `Byteable` and get zero-copy `to_byte_array()` / `from_byte_array()` with a compile-time
  `BYTE_SIZE` constant. Under the hood this uses `#[repr(C, packed)]` raw structs and
  `transmute`, so no allocation or iteration is involved.

- **Dynamic path** - For types that contain variable-length data (strings, vecs, maps). Add
  `#[byteable(io_only)]` to derive `Readable` / `Writable` instead, which stream data through
  any `std::io::Read` / `Write` (or their async `tokio` equivalents).

Both paths share the same derive macro and attribute syntax, and both produce deterministic,
self-describing wire formats.

## Installation

```toml
[dependencies]
# default: derive macro + std I/O support
byteable = "0.36"

# with async (tokio) support
byteable = { version = "0.36", features = ["tokio"] }

# with ordered-float support
byteable = { version = "0.36", features = ["ordered-float"] }

# with bitflags support
byteable = { version = "0.36", features = ["bitflags"] }

# with heapless support (fixed-capacity collections, no `alloc` needed)
byteable = { version = "0.36", features = ["heapless"] }

# with arrayvec support (fixed-capacity collections, no `alloc` needed)
byteable = { version = "0.36", features = ["arrayvec"] }

# with tinyvec support (fixed-capacity, no unsafe, no `alloc` needed)
byteable = { version = "0.36", features = ["tinyvec"] }

# with defmt support (Format impls for this crate's own error/wrapper types)
byteable = { version = "0.36", features = ["defmt"] }

# with embedded-io support (no_std friendly, sync - combine with `alloc` for Vec/String support)
byteable = { version = "0.36", features = ["embedded-io"] }

# with embedded-io-async support (no_std friendly, async - combine with `alloc` for Vec/String support)
byteable = { version = "0.36", features = ["embedded-io-async"] }

# everything
byteable = { version = "0.36", features = ["all"] }
```

## Quick Start

### Fixed-size struct (zero-copy)

```rust
use byteable::{Byteable, ToByteArray, TryFromByteArray};

#[derive(Byteable)]
struct Point3D {
    x: f32,
    y: f32,
    z: f32,
}

let p = Point3D { x: 1.0, y: 2.0, z: 3.0 };

// Serialize to a fixed-size byte array - no allocation
let bytes: [u8; 12] = p.to_byte_array();

// Deserialize back
let p2 = Point3D::try_from_byte_array(bytes).unwrap();
assert_eq!(p.x, p2.x);
```

### Dynamic struct (I/O streaming)

```rust
use byteable::Byteable;
use byteable::io::{Writable, Readable, WriteValue, ReadValue};

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

### Embedded-IO I/O streaming

`#[byteable(io_only)]` structs and field/variant enums generate `Readable`/`Writable`
(`std::io`-based) and/or `EioReadable`/`EioWritable` (`embedded-io`-based) purely from which of
`byteable`'s own `std`/`embedded-io` features are enabled - both if both are on, and it's a
compile error at the derive site if neither is. In a genuinely `no_std` build (`embedded-io`
enabled, `std` not), only the `EioReadable`/`EioWritable` impls are generated, so no `std`
reference ever appears - no separate opt-in attribute needed.

```rust
use byteable::Byteable;
use byteable::eio::{EioReadValue, EioWriteValue};

#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct Reading {
    sensor_id: u16,
    value: Option<i32>,
}

// Field enums always use this pipeline - no `#[byteable(io_only)]` needed.
#[derive(Byteable, Debug, PartialEq)]
enum Command {
    Ping,
    SetThreshold(i32),
}

let r = Reading { sensor_id: 3, value: Some(-12) };
let mut buf = [0u8; 16];
{
    let mut w: &mut [u8] = &mut buf;
    w.write_value(&r).unwrap();
}
let mut cursor: &[u8] = &buf;
let r2: Reading = cursor.read_value().unwrap();
assert_eq!(r, r2);
```

### Embedded-IO-Async I/O streaming

The async, `no_std`-friendly counterpart, generated automatically alongside the sync flavors
above whenever the `embedded-io-async` feature is on - no separate opt-in attribute.

```rust
use byteable::Byteable;
use byteable::eio_async::{EioAsyncReadValue, EioAsyncWriteValue};

#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct Reading {
    sensor_id: u16,
    value: Option<i32>,
}

async fn example() {
    let r = Reading { sensor_id: 3, value: Some(-12) };
    let mut buf = [0u8; 16];
    {
        let mut w: &mut [u8] = &mut buf;
        w.write_value(&r).await.unwrap();
    }
    let mut cursor: &[u8] = &buf;
    let r2: Reading = cursor.read_value().await.unwrap();
    assert_eq!(r, r2);
}
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

### Controlling wire layout: `order`, `tag`, `discriminant`

Three attributes let you pin the exact wire layout independently of how a type is declared in
Rust source - useful when the wire format is a fixed protocol spec and declaration order/Rust's
own discriminants aren't allowed to drift with refactors.

**`#[byteable(order = N)]`** (struct fields) pins a field's wire position to `N`, independently
of its declaration order in the struct. It must annotate either every field or none, and the
`N` values must form a dense `0..field_count` permutation - it only changes which byte range
each field occupies, adding zero bytes to the wire format.

```rust
use byteable::Byteable;

#[derive(Byteable)]
struct Header {
    #[byteable(order = 1)]
    flags: u8,
    #[byteable(order = 0)]   // encoded first on the wire, despite being declared second
    version: u8,
}
```

**`#[byteable(tag = N)]`** (enum variants) pins a variant's wire discriminant to the integer
literal `N` (which may be negative), independently of declaration order and of Rust's own
`= N` discriminant syntax. Variants without an explicit `tag` count up from the previous
resolved value (or `0` for the first variant).

**Important: Rust's real `enum Foo { A = 1 }` discriminant syntax is never read for
wire-encoding purposes**, for any enum, unit-only or field-carrying. It still compiles and
still affects `as` casts and `std::mem::discriminant` as normal Rust - it's simply invisible to
`byteable`'s wire format. If you need a specific wire value, use `#[byteable(tag = N)]`.

**`#[byteable(discriminant = uN/iN)]`** (enums) overrides the *wire width* of the enum's
discriminant - `u8`/`u16`/`u32`/`u64`/`u128`/`i8`/`i16`/`i32`/`i64`/`i128` - independently of
any `#[repr(...)]` on the enum. `#[repr(uN/iN)]` still works as a legacy width source (via the
same mechanism as before this attribute existed), but `#[byteable(discriminant = ..)]` is the
modern, `#[repr]`-independent way to say it and takes priority when both are present. Without
either, the width is auto-selected as the smallest of `u8`/`u16`/`u32`/`u64` that fits the
variant count.

```rust
use byteable::Byteable;

#[derive(Byteable, Debug, PartialEq)]
#[byteable(discriminant = u8)]
enum Message {
    #[byteable(tag = 0x01)]
    Ping,
    #[byteable(tag = 0x02)]
    Pong,
    #[byteable(tag = 0xFF)]
    Error,
}
```

A `u128`-specific quirk: `tag` is parsed as `i128`, so no positive literal can express a value
in the upper half of `u128`'s range (`2^127` to `u128::MAX`). Under
`#[byteable(discriminant = u128)]`, a *negative* `tag` is the documented way to reach that
range via two's-complement wraparound - e.g. `tag = -1` means `u128::MAX`, `tag = -10` means
`u128::MAX - 9`. This is the one wire width where a negative `tag` is accepted; every other
unsigned width (`u8`/`u16`/`u32`/`u64`) rejects a negative `tag` as a compile error since it
can never round-trip there.

### bitflags support

Unlike `ordered-float`, `bitflags` types don't get support automatically - each type generated
by the `bitflags!` macro needs a one-line opt-in, since a blanket impl over the foreign `Flags`
trait would conflict with this crate's other impls. Decoding preserves unknown bits
(`from_bits_retain`) rather than rejecting them.

```rust
use byteable::{Byteable, ToByteArray, FromByteArray};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Perms: u32 {
        const READ = 0b001;
        const WRITE = 0b010;
        const EXEC = 0b100;
    }
}
byteable::impl_bitflags!(Perms);
// Only needed for multi-byte-backed flags that use `#[byteable(little_endian)]` /
// `#[byteable(big_endian)]` field attributes:
byteable::impl_bitflags_endian!(Perms);

#[derive(Byteable)]
struct FileEntry {
    #[byteable(big_endian)]
    perms: Perms,
}
```

### heapless support

`heapless`'s fixed-capacity collections (`Vec<T, N>`, `String<N>`, `Deque<T, N>`, `IndexMap`,
`IndexSet`, `LinearMap`) work automatically as `io_only` fields, on every I/O pipeline you have
enabled (`std`, `tokio`, `embedded-io`, `embedded-io-async`) - no opt-in macro needed, unlike
`bitflags`. Unlike `alloc`'s `Vec`/`String`/`HashMap`, none of these need the `alloc` feature at
all, so they work on targets with no heap.

Decoding checks the wire-format length against the container's compile-time capacity `N` and
returns [`DecodeError::CapacityExceeded`](https://docs.rs/byteable/latest/byteable/enum.DecodeError.html)
instead of panicking or truncating if the sender's data doesn't fit.

```rust
use byteable::Byteable;
use heapless::{String, Vec};

#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct Sensor {
    id: u32,
    label: String<16>,
    readings: Vec<u32, 8>,
}
```

### arrayvec support

`arrayvec::ArrayVec<T, N>` and `ArrayString<N>` work the same way as `heapless`'s collections
above - automatically, as `io_only` fields, on every enabled I/O pipeline, no `alloc` needed,
capacity-checked on decode.

```rust
use arrayvec::{ArrayString, ArrayVec};
use byteable::Byteable;

#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct Sensor {
    id: u32,
    label: ArrayString<16>,
    readings: ArrayVec<u32, 8>,
}
```

### tinyvec support

`tinyvec::ArrayVec<[T; N]>` works the same way - automatically, as an `io_only` field, on every
enabled I/O pipeline, no `alloc` needed, capacity-checked on decode. Unlike `heapless` and
`arrayvec`, tinyvec has no `unsafe` code anywhere in its implementation (its `Array::Item: Default`
requirement is how it avoids needing `MaybeUninit`), which is why some safety-critical embedded
projects prefer it over the other two. There's no `ArrayString` equivalent in tinyvec.

```rust
use byteable::Byteable;
use tinyvec::ArrayVec;

#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct Sensor {
    id: u32,
    readings: ArrayVec<[u32; 8]>,
}
```

### defmt support

Unlike the other embedded features above, `defmt` isn't a data type to serialize - it's the
de facto logging framework for embedded Rust. Enabling it adds `defmt::Format` impls for this
crate's own error and wrapper types: `DecodeError`, `LittleEndian<T>`, `BigEndian<T>`,
`eio::EioReadableError<E>`, and `eio::EioReadExactError<E>` (each requires its own type
parameters, if any, to implement `Format` too). `LittleEndian`/`BigEndian` format the
*semantic* (native-endian) value via their own `get()`, not the raw byte-swapped bits stored
internally.

```rust
use byteable::{BigEndian, DecodeError};

fn log_error(e: DecodeError) {
    defmt::error!("decode failed: {}", e);
}

fn log_value(v: BigEndian<u32>) {
    defmt::info!("value: {}", v); // prints the native-endian value, not the raw stored bytes
}
```

This feature is scoped to byteable's own types only - it does **not** forward to the `defmt`
features of `heapless`/`arrayvec`/`bitflags`/`tinyvec`. Those crates' own `defmt` support (where
they have it) targets whatever `defmt` major version *they* pin, which doesn't necessarily match
this crate's; enable their `defmt` feature directly alongside this one if you need it.

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `derive` | yes | `#[derive(Byteable)]` proc-macro |
| `std` | yes | `Readable` / `Writable` I/O traits and `std` type impls |
| `tokio` | no | Async `AsyncReadable` / `AsyncWritable` via tokio |
| `ordered-float` | no | Impls for `OrderedFloat<T>` and `NotNan<T>` |
| `bitflags` | no | `impl_bitflags!`/`impl_bitflags_endian!` macros for types implementing `bitflags::Flags` |
| `heapless` | no | `io_only` support for `heapless`'s fixed-capacity collections, on every enabled I/O pipeline |
| `arrayvec` | no | `io_only` support for `arrayvec::ArrayVec`/`ArrayString`, on every enabled I/O pipeline |
| `tinyvec` | no | `io_only` support for `tinyvec::ArrayVec`, on every enabled I/O pipeline |
| `defmt` | no | `defmt::Format` impls for this crate's own error/wrapper types |
| `alloc` | no (implied by `std`) | `alloc`-backed collection types over `eio`/`eio_async` |
| `embedded-io` | no | `eio` module: `embedded-io`-based (sync) I/O traits for `no_std` targets |
| `embedded-io-async` | no | `eio_async` module: `embedded-io-async`-based (async) I/O traits for `no_std` targets |
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
| `Wrapping<T>` / `Saturating<T>` | same as `T` |
| `Ordering` | 1 byte: `0` = Less, `1` = Equal, `2` = Greater |
| `Reverse<T>` | same as `T` |
| `Option<T>` | 1-byte tag (`0` = None, `1` = Some) + optional value |
| `Result<V, E>` | 1-byte tag (`0` = Ok, `1` = Err) + payload |
| `Bound<T>` | 1-byte tag (`0` = Included, `1` = Excluded, `2` = Unbounded) + value for Included/Excluded |
| `(A, B, ...)` (tuples, arity 1-12) | each element in order, no tag or length prefix |
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
| `IpAddr` | 1-byte tag (`0` = V4, `1` = V6) + `Ipv4Addr` or `Ipv6Addr` |
| `SocketAddr` | 1-byte tag (`0` = V4, `1` = V6) + `SocketAddrV4` or `SocketAddrV6` |
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
| [`ToByteArray`] | Serialize to a `[u8; N]`; provides the compile-time `BYTE_SIZE` constant |
| [`FromByteArray`] | Infallible deserialization from a `[u8; N]` |
| [`TryFromByteArray`] | Fallible deserialization from a `[u8; N]` (returns [`DecodeError`]) |

[`ToByteArray`]: https://docs.rs/byteable/latest/byteable/trait.ToByteArray.html
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

[`Readable`]: https://docs.rs/byteable/latest/byteable/io/trait.Readable.html
[`Writable`]: https://docs.rs/byteable/latest/byteable/io/trait.Writable.html
[`FixedReadable`]: https://docs.rs/byteable/latest/byteable/io/trait.FixedReadable.html
[`FixedWritable`]: https://docs.rs/byteable/latest/byteable/io/trait.FixedWritable.html
[`ReadValue`]: https://docs.rs/byteable/latest/byteable/io/trait.ReadValue.html
[`WriteValue`]: https://docs.rs/byteable/latest/byteable/io/trait.WriteValue.html
[`ReadFixed`]: https://docs.rs/byteable/latest/byteable/io/trait.ReadFixed.html
[`WriteFixed`]: https://docs.rs/byteable/latest/byteable/io/trait.WriteFixed.html

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

[`AsyncReadable`]: https://docs.rs/byteable/latest/byteable/async_io/trait.AsyncReadable.html
[`AsyncWritable`]: https://docs.rs/byteable/latest/byteable/async_io/trait.AsyncWritable.html
[`AsyncFixedReadable`]: https://docs.rs/byteable/latest/byteable/async_io/trait.AsyncFixedReadable.html
[`AsyncFixedWritable`]: https://docs.rs/byteable/latest/byteable/async_io/trait.AsyncFixedWritable.html
[`AsyncReadValue`]: https://docs.rs/byteable/latest/byteable/async_io/trait.AsyncReadValue.html
[`AsyncWriteValue`]: https://docs.rs/byteable/latest/byteable/async_io/trait.AsyncWriteValue.html
[`AsyncReadFixed`]: https://docs.rs/byteable/latest/byteable/async_io/trait.AsyncReadFixed.html
[`AsyncWriteFixed`]: https://docs.rs/byteable/latest/byteable/async_io/trait.AsyncWriteFixed.html

### Embedded-IO traits (`embedded-io` feature)

The `no_std`-friendly, sync counterparts of the I/O streaming traits above, backed by the
`embedded-io` crate via the [`eio::EioReader`]/[`eio::EioWriter`] marker traits.

| Trait | Counterpart of |
|-------|---------------|
| [`eio::EioReadable`] | [`Readable`] |
| [`eio::EioWritable`] | [`Writable`] |
| [`eio::EioFixedReadable`] | [`FixedReadable`] |
| [`eio::EioFixedWritable`] | [`FixedWritable`] |
| [`eio::EioReadValue`] | [`ReadValue`] |
| [`eio::EioWriteValue`] | [`WriteValue`] |
| [`eio::EioReadFixed`] | [`ReadFixed`] |
| [`eio::EioWriteFixed`] | [`WriteFixed`] |

[`eio::EioReader`]: https://docs.rs/byteable/latest/byteable/eio/trait.EioReader.html
[`eio::EioWriter`]: https://docs.rs/byteable/latest/byteable/eio/trait.EioWriter.html
[`eio::EioReadable`]: https://docs.rs/byteable/latest/byteable/eio/trait.EioReadable.html
[`eio::EioWritable`]: https://docs.rs/byteable/latest/byteable/eio/trait.EioWritable.html
[`eio::EioFixedReadable`]: https://docs.rs/byteable/latest/byteable/eio/trait.EioFixedReadable.html
[`eio::EioFixedWritable`]: https://docs.rs/byteable/latest/byteable/eio/trait.EioFixedWritable.html
[`eio::EioReadValue`]: https://docs.rs/byteable/latest/byteable/eio/trait.EioReadValue.html
[`eio::EioWriteValue`]: https://docs.rs/byteable/latest/byteable/eio/trait.EioWriteValue.html
[`eio::EioReadFixed`]: https://docs.rs/byteable/latest/byteable/eio/trait.EioReadFixed.html
[`eio::EioWriteFixed`]: https://docs.rs/byteable/latest/byteable/eio/trait.EioWriteFixed.html

### Embedded-IO-Async traits (`embedded-io-async` feature)

The async counterpart of the `embedded-io` traits above, backed by the `embedded-io-async`
crate via the [`eio_async::EioAsyncReader`]/[`eio_async::EioAsyncWriter`] marker traits.

| Trait | Counterpart of |
|-------|---------------|
| [`eio_async::EioAsyncReadable`] | [`eio::EioReadable`] |
| [`eio_async::EioAsyncWritable`] | [`eio::EioWritable`] |
| [`eio_async::EioAsyncFixedReadable`] | [`eio::EioFixedReadable`] |
| [`eio_async::EioAsyncFixedWritable`] | [`eio::EioFixedWritable`] |
| [`eio_async::EioAsyncReadValue`] | [`eio::EioReadValue`] |
| [`eio_async::EioAsyncWriteValue`] | [`eio::EioWriteValue`] |
| [`eio_async::EioAsyncReadFixed`] | [`eio::EioReadFixed`] |
| [`eio_async::EioAsyncWriteFixed`] | [`eio::EioWriteFixed`] |

[`eio_async::EioAsyncReader`]: https://docs.rs/byteable/latest/byteable/eio_async/trait.EioAsyncReader.html
[`eio_async::EioAsyncWriter`]: https://docs.rs/byteable/latest/byteable/eio_async/trait.EioAsyncWriter.html
[`eio_async::EioAsyncReadable`]: https://docs.rs/byteable/latest/byteable/eio_async/trait.EioAsyncReadable.html
[`eio_async::EioAsyncWritable`]: https://docs.rs/byteable/latest/byteable/eio_async/trait.EioAsyncWritable.html
[`eio_async::EioAsyncFixedReadable`]: https://docs.rs/byteable/latest/byteable/eio_async/trait.EioAsyncFixedReadable.html
[`eio_async::EioAsyncFixedWritable`]: https://docs.rs/byteable/latest/byteable/eio_async/trait.EioAsyncFixedWritable.html
[`eio_async::EioAsyncReadValue`]: https://docs.rs/byteable/latest/byteable/eio_async/trait.EioAsyncReadValue.html
[`eio_async::EioAsyncWriteValue`]: https://docs.rs/byteable/latest/byteable/eio_async/trait.EioAsyncWriteValue.html
[`eio_async::EioAsyncReadFixed`]: https://docs.rs/byteable/latest/byteable/eio_async/trait.EioAsyncReadFixed.html
[`eio_async::EioAsyncWriteFixed`]: https://docs.rs/byteable/latest/byteable/eio_async/trait.EioAsyncWriteFixed.html

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
| [`PlainOldData`] | Unsafe marker: no padding, all bit patterns valid - enables `transmute`-based I/O |
| [`ByteArray`] | Unsafe marker for `[u8; N]` used as the `ToByteArray::ByteArray` associated type |

[`PlainOldData`]: https://docs.rs/byteable/latest/byteable/trait.PlainOldData.html
[`ByteArray`]: https://docs.rs/byteable/latest/byteable/trait.ByteArray.html

### Error types

| Type | When it occurs |
|------|---------------|
| [`DecodeError`] | Bytes decoded successfully but the value is invalid (bad discriminant, NaN, interior null, etc.) |
| [`ReadableError`] | An I/O error or [`DecodeError`] while reading from a `Read` / async reader |

[`DecodeError`]: https://docs.rs/byteable/latest/byteable/enum.DecodeError.html
[`ReadableError`]: https://docs.rs/byteable/latest/byteable/io/enum.ReadableError.html

## License

MIT - see [LICENSE](LICENSE).
