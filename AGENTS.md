# AGENTS.md

This file provides guidance to AI agents when working with code in this repository.

## Project Overview

`byteable` is a Rust crate for zero-overhead, zero-copy serialization/deserialization of byte-oriented data. The crate provides a modular trait system for converting between Rust types and byte arrays, with support for endianness control, sync/async I/O, and procedural macros.

## Build and Development Commands

### Testing

```bash
# Run all tests
cargo test

# Run tests for a specific test file
cargo test --test derive_enums
cargo test --test type_impls

# Run tests with tokio feature
cargo test --features tokio

# Run tests without default features (no_std)
cargo test --no-default-features

# Run doctests
cargo test --doc
```

### Building

```bash
# Build the project
cargo build

# Build with all features
cargo build --features all

# Build without std (no_std mode)
cargo build --no-default-features

# Build documentation
cargo doc --open --no-deps
```

### Benchmarks

```bash
# Run benchmarks (uses criterion)
cargo bench
```

### Examples

```bash
# Run examples (requires std feature)
cargo run --example simple_usage
cargo run --example file_io
cargo run --example enum_endianness
cargo run --example cursor_usage
```

### Publishing

The project is a workspace with two crates:

- Main crate: `byteable` (root directory)
- Proc macro crate: `byteable_derive` (subdirectory)

When bumping versions, update both crates and ensure version compatibility in [Cargo.toml](Cargo.toml).

## Architecture

### Crate Structure

This is a **dual-crate architecture**:

1. **`byteable`** (main crate) - Core traits and implementations
2. **`byteable_derive`** (proc-macro crate) - Procedural macros for deriving traits

### Core Trait System

The crate uses a **modular trait hierarchy** (as of v0.20+):

1. **`ByteRepr`** - Associates a type with its byte array representation
2. **`IntoByteArray`** - Infallible conversion to bytes
3. **`FromByteArray`** - Infallible conversion from bytes
4. **`TryFromByteArray`** - Fallible conversion from bytes (used for `bool`, `char`, enums)

These traits replaced the previous monolithic `Byteable` trait to provide more flexibility.

### Key Components

#### Endianness System ([src/endian.rs](src/endian.rs))

- **`EndianConvert` trait** - For types supporting endianness conversion
- **`BigEndian<T>` wrapper** - Ensures big-endian byte order
- **`LittleEndian<T>` wrapper** - Ensures little-endian byte order
- These wrappers are `#[repr(transparent)]` for zero-cost abstraction

#### Raw Type Delegation Pattern

The derive macro generates a hidden "raw" struct (e.g., `__byteable_raw_Foo` for `struct Foo`) with:

- `#[repr(C, packed)]` for predictable layout
- Endianness wrappers around fields: explicit `#[byteable(big_endian)]`/`#[byteable(little_endian)]` attributes, or auto-`LittleEndian<T>` for unannotated multi-byte primitives
- Direct `transmute`-based `IntoByteArray`/`FromByteArray` implementations

The original struct gets `From` conversions to/from the raw struct, creating a safe API layer.

#### Safety Validation ([src/derive_safety_helpers.rs](src/derive_safety_helpers.rs))

- **`PlainOldData` trait** - Marks types safe for `transmute`-based conversions
- Automatically implemented for `u8`, `i8`, endian wrappers, and arrays thereof
- Multi-byte primitives (`u16`, `u32`, etc.) are **not** directly `PlainOldData` — the derive macro auto-wraps unannotated multi-byte fields in `LittleEndian<T>`, which is `PlainOldData`
- Raw structs require all fields to implement `PlainOldData`
- This prevents unsafe usage with types like `String`, `Vec`, references, etc.

#### I/O Extension Traits

- **Sync I/O** ([src/io.rs](src/io.rs)): `ReadValue`, `WriteValue` - Extend `std::io::Read`/`Write`
- **Async I/O** ([src/async_io.rs](src/async_io.rs)): `AsyncReadValue`, `AsyncWriteValue` - Extend `tokio::io::AsyncRead`/`AsyncWrite`

### Derive Macro Implementation ([byteable_derive/src/lib.rs](byteable_derive/src/lib.rs))

The `#[derive(Byteable)]` macro handles two code paths:

**Transmute path** (default for structs and C-like enums):
1. **Named structs** - Generates raw struct with field-level endianness attributes
2. **Tuple structs** - Similar to named structs but with positional fields
3. **Unit structs** - Direct implementation (zero-sized type, empty byte array)
4. **C-like enums** - Implements `TryFromByteArray`/`IntoByteArray` via a transparent wrapper

**Stream I/O path** (for dynamic/variable-size types):
5. **`io_only` structs** - Annotated with `#[byteable(io_only)]`; generates `Readable`/`Writable` impls via sequential field I/O. Supports `Vec<T>`, `String`, `Option<T>`, and any other `Readable`/`Writable` type.
6. **Field enums** - Enums with variant fields (named or tuple); generates `Readable`/`Writable` impls. The discriminant is written first, then the variant's fields in declaration order.

#### Enum Support

**C-like enums (transmute path):**
- Requires `#[repr(u8)]`, `#[repr(u16)]`, etc. (or auto-inferred)
- All variants must be unit variants (no fields)
- Discriminants are auto-assigned from 0 if not explicit
- Implements `TryFromByteArray` (not `FromByteArray`) because invalid discriminants are possible
- Supports type-level endianness: `#[byteable(big_endian)]` or `#[byteable(little_endian)]`

**Field enums (stream I/O path):**
- Triggered automatically when any variant has fields
- Supports unit, named-field, and tuple-field variants in any combination
- Discriminant written first (endianness-aware), then fields written in order
- `#[repr]` is optional; auto-inferred as `u8` for ≤256 variants, `u16` otherwise
- Invalid discriminants on read return `io::ErrorKind::InvalidData`

#### Field Attributes

- `#[byteable(little_endian)]` - Wrap field in `LittleEndian<T>`
- `#[byteable(big_endian)]` - Wrap field in `BigEndian<T>`
- `#[byteable(try_transparent)]` - Use field's raw type with fallible conversion (via `TryRawRepr::Raw`)

**Default endianness:** Multi-byte primitive fields (`u16`, `u32`, `u64`, `i16`, `f32`, etc.) with no endianness annotation are automatically treated as `LittleEndian<T>` in both the transmute path and the stream I/O path. Use `#[byteable(big_endian)]` to override. This is now driven by the blanket `RawRepr` impl: unannotated fields use `<FieldType as RawRepr>::Raw` as their raw type.

When `try_transparent` is used, the struct implements `TryFromByteArray` instead of `FromByteArray`.

### Feature Flags

- `derive` (default) - Enables `#[derive(Byteable)]` and `#[derive(UnsafeByteableTransmute)]`
- `std` (default) - Enables `std` support (I/O traits, error types)
- `tokio` - Enables async I/O traits for tokio
- `all` - Enables all features

### no_std Support

The crate is `no_std` compatible when built with `--no-default-features`. The I/O traits require `std` feature.

## Important Development Guidelines

### Safety Requirements for Byteable Types

The safety requirements differ depending on which derive path is used.

**Transmute path** — safe field types:

- Primitive numeric types (`u8`, `i32`, `f64`, etc.)
- `bool`, `char` (with `TryFromByteArray` validation)
- Arrays of safe types
- `BigEndian<T>`, `LittleEndian<T>` wrappers
- Structs with `#[repr(C, packed)]`
- C-like enums with explicit discriminants (with `TryFromByteArray`)

**Transmute path** — never use as struct fields:

- `String`, `Vec`, or heap-allocated types — use `#[byteable(io_only)]` instead
- References or pointers (`&T`, `Box<T>`, `*const T`)
- Enums with variant fields (use field enum derive instead, which uses the stream I/O path)
- Types with `Drop` implementations
- `NonZero*` types or types with invariants

**Stream I/O path** (`io_only` structs and field enums) — no transmute; any `Readable`/`Writable` type works as a field, including `Vec`, `String`, `Option`, `HashMap`, etc.

### When Adding New Features

1. **Primitive type support** - Add implementations in [src/byteable_trait.rs](src/byteable_trait.rs)
2. **Endianness changes** - Modify [src/endian.rs](src/endian.rs)
3. **Macro changes** - Edit [byteable_derive/src/lib.rs](byteable_derive/src/lib.rs)
4. **New I/O traits** - Add to [src/io.rs](src/io.rs) or [src/async_io.rs](src/async_io.rs)

### Testing Strategy

- **Integration tests** ([tests/](tests/)) - Test derive macros, specific features
- **Doctests** - Embedded in rustdoc comments, run with `cargo test --doc`
- **Examples** ([examples/](examples/)) - Demonstrate real-world usage patterns
- **Benchmarks** ([benches/](benches/)) - Performance validation with criterion

Important test files:

- [tests/derive_structs.rs](tests/derive_structs.rs) - Named/tuple/unit struct derive; `transparent`; visibility; safety (`PlainOldData`)
- [tests/derive_enums.rs](tests/derive_enums.rs) - C-like enum derive (all repr types, endianness, auto-inference)
- [tests/derive_field_enums.rs](tests/derive_field_enums.rs) - Field enum derive (variants with data)
- [tests/try_transparent.rs](tests/try_transparent.rs) - `try_transparent` attribute (fallible nested types)
- [tests/type_impls.rs](tests/type_impls.rs) - `ByteRepr` impls: primitives, arrays, `NonZero*`, network, time, ranges, `bool`, `char`, `u128`/`i128`
- [tests/ordered_float.rs](tests/ordered_float.rs) - `ordered-float` crate integration
- [tests/io_sync.rs](tests/io_sync.rs) - Sync I/O: fixed-size, value/stream, `io_only` derive, collections
- [tests/io_async.rs](tests/io_async.rs) - Async I/O: fixed-size, value/stream, async collections

## Common Patterns

### Adding a New Primitive Type

If adding support for a new primitive type that needs validation (like `bool`, `char`):

1. Implement `TryFromByteArray` instead of `FromByteArray`
2. Add validation logic that returns `DecodeError` for invalid byte patterns
3. Implement `IntoByteArray` for infallible conversion
4. Add test coverage in [tests/type_impls.rs](tests/type_impls.rs)

### Enum Implementation Pattern

Enums generate a raw wrapper type like:

```rust
#[repr(transparent)]
struct __byteable_raw_Status(u8); // or BigEndian<u8>, LittleEndian<u8>
```

The `TryFrom<Raw>` implementation validates discriminants and returns errors for invalid values.

### Version Coordination

When releasing new versions:

1. Update `byteable_derive` version in [byteable_derive/Cargo.toml](byteable_derive/Cargo.toml)
2. Update main crate version and `byteable_derive` dependency in [Cargo.toml](Cargo.toml)
3. Ensure compatibility - macro crate should be backward compatible when possible
