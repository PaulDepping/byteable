# AGENTS.md

This file provides guidance to AI-Agents when working with code in this repository.

## Project Overview

`byteable` is a Rust crate for zero-overhead, zero-copy serialization/deserialization of byte-oriented data. The crate provides a modular trait system for converting between Rust types and byte arrays, with support for endianness control, sync/async I/O, and procedural macros.

## Build and Development Commands

### Testing

```bash
# Run all tests
cargo test

# Run tests for a specific test file
cargo test --test enum_test
cargo test --test primitive_types_test

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
cargo run --example try_io
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

1. **`AssociatedByteArray`** - Associates a type with its byte array representation
2. **`IntoByteArray`** - Infallible conversion to bytes
3. **`FromByteArray`** - Infallible conversion from bytes
4. **`TryIntoByteArray`** - Fallible conversion to bytes
5. **`TryFromByteArray`** - Fallible conversion from bytes (used for `bool`, `char`, enums)

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
- Endianness wrappers around fields based on `#[byteable(big_endian)]` or `#[byteable(little_endian)]` attributes
- Direct `transmute`-based `IntoByteArray`/`FromByteArray` implementations

The original struct gets `From` conversions to/from the raw struct, creating a safe API layer.

#### Safety Validation ([src/derive_safety_helpers.rs](src/derive_safety_helpers.rs))

- **`ValidBytecastMarker` trait** - Marks types safe for `transmute`-based conversions
- Automatically implemented for primitive types, endian wrappers, and arrays
- Raw structs require all fields to implement `ValidBytecastMarker`
- This prevents unsafe usage with types like `String`, `Vec`, references, etc.

#### I/O Extension Traits

- **Sync I/O** ([src/io.rs](src/io.rs)): `ReadByteable`, `WriteByteable` - Extend `std::io::Read`/`Write`
- **Async I/O** ([src/async_io.rs](src/async_io.rs)): `AsyncReadByteable`, `AsyncWriteByteable` - Extend `tokio::io::AsyncRead`/`AsyncWrite`

### Derive Macro Implementation ([byteable_derive/src/lib.rs](byteable_derive/src/lib.rs))

The `#[derive(Byteable)]` macro handles three cases:

1. **Named structs** - Generates raw struct with field-level endianness attributes
2. **Tuple structs** - Similar to named structs but with positional fields
3. **Unit structs** - Direct implementation (zero-sized type, empty byte array)
4. **Enums** - Only C-like enums with explicit discriminants

#### Enum Support

- Requires `#[repr(u8)]`, `#[repr(u16)]`, etc.
- All variants must be unit variants (no fields)
- All variants must have explicit discriminant values
- Implements `TryFromByteArray` (not `FromByteArray`) because invalid discriminants are possible
- Supports type-level endianness: `#[byteable(big_endian)]` or `#[byteable(little_endian)]`

#### Field Attributes

- `#[byteable(little_endian)]` - Wrap field in `LittleEndian<T>`
- `#[byteable(big_endian)]` - Wrap field in `BigEndian<T>`
- `#[byteable(transparent)]` - Use field's raw type (via `HasRawType::Raw`)
- `#[byteable(try_transparent)]` - Use field's raw type with fallible conversion (via `TryHasRawType::Raw`)

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

**Safe to use:**

- Primitive numeric types (`u8`, `i32`, `f64`, etc.)
- `bool`, `char` (with `TryFromByteArray` validation)
- Arrays of safe types
- `BigEndian<T>`, `LittleEndian<T>` wrappers
- Structs with `#[repr(C, packed)]`
- C-like enums with explicit discriminants (with `TryFromByteArray`)

**Never use:**

- `String`, `Vec`, or heap-allocated types
- References or pointers (`&T`, `Box<T>`, `*const T`)
- Complex enums with fields
- Types with `Drop` implementations
- `NonZero*` types or types with invariants

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

- [tests/enum_test.rs](tests/enum_test.rs) - C-like enum derive validation
- [tests/primitive_types_test.rs](tests/primitive_types_test.rs) - `bool`, `char` validation
- [tests/try_transparent_test.rs](tests/try_transparent_test.rs) - Nested enum/validated types
- [tests/safety_validation_test.rs](tests/safety_validation_test.rs) - `ValidBytecastMarker` tests

## Common Patterns

### Adding a New Primitive Type

If adding support for a new primitive type that needs validation (like `bool`, `char`):

1. Implement `TryFromByteArray` instead of `FromByteArray`
2. Add validation logic that returns `EnumFromBytesError` for invalid byte patterns
3. Implement `IntoByteArray` for infallible conversion
4. Add test coverage in [tests/primitive_types_test.rs](tests/primitive_types_test.rs)

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
