#![cfg_attr(not(feature = "std"), no_std)]

//! # Byteable
//!
//! A Rust crate for convenient serialization and deserialization of byte-oriented data.
//!
//! `byteable` provides traits and utilities for seamless conversion between data structures
//! and byte arrays, handling both synchronous and asynchronous I/O operations, and managing
//! endianness.
//!
//! ## Features
//!
//! - **Byte Conversion Traits**: A modular trait system for byte array conversion:
//!   - `ByteRepr`: Associates a type with its byte array representation
//!   - `IntoByteArray`: Converts values into byte arrays
//!   - `FromByteArray`: Constructs values from byte arrays
//!   - `TryFromByteArray`: Fallible deserialization variant
//! - **`ReadValue` & `WriteValue` Traits**: Extension traits for `std::io::Read` and
//!   `std::io::Write`, enabling convenient reading and writing of byteable types.
//! - **`AsyncReadValue` & `AsyncWriteValue` Traits** (with `tokio` feature): Asynchronous
//!   counterparts to `ReadValue` and `WriteValue`, designed for use with `tokio`'s async I/O.
//! - **`EndianConvert` Trait & Wrappers**: Provides methods for converting primitive types between
//!   different endianness (little-endian and big-endian), along with `BigEndian<T>` and
//!   `LittleEndian<T>` wrapper types.
//! - **`#[derive(Byteable)]`** (with `derive` feature): A procedural macro that automatically
//!   implements the byte conversion traits for structs, significantly simplifying boilerplate. For
//!   advanced use cases, `#[derive(UnsafeByteableTransmute)]` is also available for manual
//!   transmute-based implementations.
//!
//! ## Quick Start
//!
//! Add `byteable` to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! byteable = { version = "*", features = ["derive"] }
//! ```
//!
//! ## Basic Usage
//!
//! The core workflow involves:
//! 1. Defining a struct with explicit memory layout (`#[repr(C, packed)]`)
//! 2. Deriving or implementing the `Byteable` trait
//! 3. Using extension traits for reading/writing
//!
//! ### Example: File I/O
//!
//! ```no_run,ignore
//! # #![cfg(feature = "std")]
//! use byteable::{Byteable, ReadValue, WriteValue};
//! use std::fs::File;
//!
//! #[derive(Byteable, Debug, PartialEq, Clone, Copy)]
//! struct Packet {
//!     id: u8,
//!     #[byteable(little_endian)]
//!     length: u16,
//!     data: [u8; 4],
//! }
//!
//! # fn main() -> std::io::Result<()> {
//! // Create a packet
//! let packet = Packet {
//!     id: 42,
//!     length: 1024,
//!     data: [0xDE, 0xAD, 0xBE, 0xEF],
//! };
//!
//! // Write to file
//! let mut file = File::create("packet.bin")?;
//! file.write_value(&packet)?;
//!
//! // Read from file
//! let mut file = File::open("packet.bin")?;
//! let restored: Packet = file.read_value()?;
//!
//! assert_eq!(packet, restored);
//! # Ok(())
//! # }
//! ```
//!
//! ### Example: Network Protocol
//!
//! ```no_run
//! #[cfg(feature = "derive")] {
//! use byteable::Byteable;
//!
//! #[derive(Byteable, Debug, Clone, Copy)]
//! struct NetworkHeader {
//!     #[byteable(big_endian)]
//!     magic: u32,       // Network byte order (big-endian)
//!     version: u8,
//!     flags: u8,
//!     #[byteable(little_endian)]
//!     payload_len: u16, // Little-endian for payload
//! }
//! # }
//! ```
//!
//! ### Example: Working with TCP Streams
//!
//! ```no_run
//! # #[cfg(all(feature = "std", feature = "derive"))] {
//! use byteable::{Byteable, ReadValue, WriteValue};
//! use std::net::TcpStream;
//!
//! #[derive(Byteable, Debug, Clone, Copy)]
//! struct Message {
//!     msg_type: u8,
//!     data: [u8; 16],
//! }
//!
//! # fn main() -> std::io::Result<()> {
//! let mut stream = TcpStream::connect("127.0.0.1:8080")?;
//!
//! // Write message
//! let msg = Message {
//!     msg_type: 1,
//!     data: [0; 16],
//! };
//! stream.write_value(&msg)?;
//!
//! // Read response
//! let response: Message = stream.read_value()?;
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Async I/O (with `tokio` feature)
//!
//! Enable async support in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! byteable = { version = "*", features = ["derive", "tokio"] }
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! Example usage:
//!
//! ```no_run
//! # #[cfg(all(feature = "tokio", feature = "derive"))]
//! # {
//! use byteable::{AsyncReadValue, AsyncWriteValue, Byteable};
//! use tokio::net::TcpStream;
//!
//! #[derive(Byteable, Debug, Clone, Copy)]
//! struct AsyncPacket {
//!     #[byteable(little_endian)]
//!     id: u32,
//!     data: [u8; 8],
//! }
//!
//! # async fn example() -> std::io::Result<()> {
//! let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
//!
//! let packet = AsyncPacket {
//!     id: 123,
//!     data: [1, 2, 3, 4, 5, 6, 7, 8],
//! };
//!
//! stream.write_value(&packet).await?;
//! let response: AsyncPacket = stream.read_value().await?;
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Endianness Handling
//!
//! The crate provides `BigEndian<T>` and `LittleEndian<T>` wrappers for handling byte order:
//!
//! ```
//! use byteable::{BigEndian, LittleEndian, IntoByteArray};
//!
//! // Create endian-aware values
//! let big = BigEndian::new(0x12345678u32);
//! let little = LittleEndian::new(0x12345678u32);
//!
//! // Get raw bytes (in specified endianness)
//! assert_eq!(big.into_byte_array(), [0x12, 0x34, 0x56, 0x78]);
//! assert_eq!(little.into_byte_array(), [0x78, 0x56, 0x34, 0x12]);
//!
//! // Convert back to native value
//! assert_eq!(big.get(), 0x12345678u32);
//! assert_eq!(little.get(), 0x12345678u32);
//! ```
//!
//! ## Enum Support
//!
//! The `#[derive(Byteable)]` macro supports C-like enums with explicit discriminants:
//!
//! ```
//! # #[cfg(feature = "derive")] {
//! use byteable::{Byteable, IntoByteArray, TryFromByteArray};
//!
//! #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
//! #[repr(u8)]
//! enum Status {
//!     Idle = 0,
//!     Running = 1,
//!     Completed = 2,
//!     Failed = 3,
//! }
//!
//! let status = Status::Running;
//! let bytes = status.into_byte_array();
//! assert_eq!(bytes, [1]);
//!
//!  // Use TryFromByteArray for fallible conversion
//! let restored = Status::try_from_byte_array(bytes).unwrap();
//! assert_eq!(restored, Status::Running);
//!
//!  // Invalid discriminants return an error
//! let invalid = Status::try_from_byte_array([255]);
//! assert!(invalid.is_err());
//! # }
//! ```
//!
//! ### Enum with Endianness
//!
//! Enums support type-level endianness attributes for multi-byte discriminants:
//!
//! ```
//! # #[cfg(feature = "derive")] {
//! use byteable::Byteable;
//!
//! // Little-endian (common for file formats)
//! #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
//! #[repr(u16)]
//! #[byteable(little_endian)]
//! enum Command {
//!     Start = 0x1000,
//!     Stop = 0x2000,
//!     Pause = 0x3000,
//! }
//!
//! // Big-endian (common for network protocols)
//! #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
//! #[repr(u32)]
//! #[byteable(big_endian)]
//! enum HttpStatus {
//!     Ok = 200,
//!     NotFound = 404,
//!     InternalError = 500,
//! }
//! # }
//! ```
//!
//! ### Enum Types
//!
//! The `#[derive(Byteable)]` macro supports two kinds of enums:
//!
//! **C-like enums** (unit variants only):
//! - Implement `IntoByteArray` / `TryFromByteArray` (transmute-based, fixed-size)
//! - `#[repr]` is optional; auto-inferred from variant count
//! - Discriminants are optional; auto-assigned from 0 like normal Rust enums
//! - Invalid discriminants on conversion return `InvalidDiscriminantError`
//!
//! **Enums with variant fields**:
//! - Implement `Readable` / `Writable` (stream-based I/O, variable-size)
//! - Discriminant written first, then variant fields in order
//! - `#[repr]` and discriminants both optional (auto-inferred)
//!
//! ### Dynamic Types: `Vec`, `String`, and Collections
//!
//! The `Readable` and `Writable` traits are implemented for many standard library types:
//! - Collections: `Vec<T>`, `VecDeque<T>`, `HashMap<K, V>`, `HashSet<T>`, `BTreeMap<K, V>`, `BTreeSet<T>`
//! - Optional/Result: `Option<T>`, `Result<T, E>`
//! - Text/Path: `String`, `str`, `Path`, `PathBuf`, `CStr`, `CString`
//!
//! For structs containing these types, use `#[byteable(io_only)]`:
//!
//! ```
//! # #[cfg(feature = "derive")] {
//! use byteable::Byteable;
//!
//! #[derive(Byteable, Debug, PartialEq)]
//! #[byteable(io_only)]
//! struct Message {
//!     id: u8,
//!     payload: Vec<u8>,
//!     label: String,
//! }
//! # }
//! ```
//!
//! ## Safety Considerations
//!
//! The `#[derive(Byteable)]` macro uses two code paths with different safety profiles:
//!
//! **Transmute path** (default for fixed-size structs and C-like enums):
//! - Uses `core::mem::transmute`, so all fields must be fixed-size
//! - Supports: primitives, bool, char, enums, arrays, `BigEndian<T>`, `LittleEndian<T>`
//! - **Do not use with**: `Vec`, `String`, references, pointers, types with `Drop`
//!
//! **Stream I/O path** (`#[byteable(io_only)]` structs and enums with fields):
//! - No `transmute` involved; reads/writes fields sequentially
//! - Supports all `Readable`/`Writable` types including `Vec`, `String`, `Option`, `HashMap`, etc.
//!
//! For structs containing dynamic types like `String`, `Vec`, references, etc., use
//! `#[byteable(io_only)]` instead of relying on the default transmute path.
//!
//! ## Advanced Usage
//!
//! ### Custom `Byteable` Implementation
//!
//! The `#[derive(Byteable)]` macro handles most use cases automatically, including
//! endianness conversion via attributes:
//!
//! ```
//! # #[cfg(feature = "derive")] {
//! use byteable::{Byteable, IntoByteArray, FromByteArray};
//!
//! #[derive(Byteable, Debug, PartialEq, Clone, Copy)]
//! struct Point {
//!     #[byteable(little_endian)]
//!     x: i32,
//!     #[byteable(little_endian)]
//!     y: i32,
//! }
//!
//! let point = Point { x: 10, y: 20 };
//! let bytes = point.into_byte_array();
//! let restored = Point::from_byte_array(bytes);
//! assert_eq!(point, restored);
//! # }
//! ```
//!
//! For advanced cases, you can still use the `impl_byteable_via!` macro with manual
//! implementations. See the trait documentation for details.
//!
//! ## Feature Flags
//!
//! - `derive`: Enables the `#[derive(Byteable)]` procedural macro (default: enabled)
//! - `tokio`: Enables async I/O traits for use with tokio (default: disabled)
//!
//! ## Performance
//!
//! This crate is designed for zero-copy, zero-overhead serialization. Operations like
//! `into_byte_array` and `from_byte_array` typically compile down to simple memory operations
//! or even no-ops when possible.

mod byte_array;
mod byteable_trait;
mod derive_safety_helpers;
mod endian;
mod std_types;

#[cfg(feature = "ordered-float")]
mod ordered_float_types;

#[cfg(feature = "std")]
mod io;

extern crate self as byteable; // used to resolve derive macros in examples etc.

#[cfg(feature = "tokio")]
mod async_io;

#[cfg(feature = "derive")]
pub use byteable_derive::{Byteable, UnsafeByteableTransmute};

pub use byte_array::FixedBytes;

pub use byteable_trait::{
    ByteRepr, DiscriminantValue, FromByteArray, IntoByteArray, InvalidDiscriminantError, RawRepr,
    TryFromByteArray, TryRawRepr,
};

#[cfg(feature = "std")]
pub use io::{
    FixedReadable, FixedWritable, ReadFixed, ReadValue, Readable, Writable, WriteFixed, WriteValue,
};

#[cfg(feature = "tokio")]
pub use async_io::{
    AsyncFixedReadable, AsyncFixedWritable, AsyncReadFixed, AsyncReadValue, AsyncReadable,
    AsyncWritable, AsyncWriteFixed, AsyncWriteValue,
};

pub use endian::{BigEndian, EndianConvert, LittleEndian};

pub use derive_safety_helpers::TransmuteSafe;
