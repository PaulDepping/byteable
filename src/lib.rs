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
//!   - `AssociatedByteArray`: Associates a type with its byte array representation
//!   - `IntoByteArray`: Converts values into byte arrays
//!   - `FromByteArray`: Constructs values from byte arrays
//!   - `TryIntoByteArray` & `TryFromByteArray`: Fallible conversion variants
//! - **`ReadByteable` & `WriteByteable` Traits**: Extension traits for `std::io::Read` and
//!   `std::io::Write`, enabling convenient reading and writing of byteable types.
//! - **`AsyncReadByteable` & `AsyncWriteByteable` Traits** (with `tokio` feature): Asynchronous
//!   counterparts to `ReadByteable` and `WriteByteable`, designed for use with `tokio`'s async I/O.
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
//! use byteable::{Byteable, ReadByteable, WriteByteable};
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
//! file.write_byteable(packet)?;
//!
//! // Read from file
//! let mut file = File::open("packet.bin")?;
//! let restored: Packet = file.read_byteable()?;
//!
//! assert_eq!(packet, restored);
//! # Ok(())
//! # }
//! ```
//!
//! ### Example: Network Protocol
//!
//! ```no_run
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
//! # fn main() {}
//! ```
//!
//! ### Example: Working with TCP Streams
//!
//! ```no_run,ignore
//! # #![cfg(feature = "std")]
//! use byteable::{Byteable, ReadByteable, WriteByteable};
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
//! stream.write_byteable(msg)?;
//!
//! // Read response
//! let response: Message = stream.read_byteable()?;
//! # Ok(())
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
//! # #[cfg(feature = "tokio")]
//! {
//! use byteable::{AsyncReadByteable, AsyncWriteByteable, Byteable};
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
//! stream.write_byteable(packet).await?;
//! let response: AsyncPacket = stream.read_byteable().await?;
//! # Ok(())
//! # }
//! # fn main() {}
//! # }
//! ```
//!
//! ## Endianness Handling
//!
//! The crate provides `BigEndian<T>` and `LittleEndian<T>` wrappers for handling byte order:
//!
//! ```
//! use byteable::{BigEndian, LittleEndian};
//!
//! // Create endian-aware values
//! let big = BigEndian::new(0x12345678u32);
//! let little = LittleEndian::new(0x12345678u32);
//!
//! // Get raw bytes (in specified endianness)
//! assert_eq!(big.raw_bytes(), [0x12, 0x34, 0x56, 0x78]);
//! assert_eq!(little.raw_bytes(), [0x78, 0x56, 0x34, 0x12]);
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
//! # #[cfg(feature = "derive")]
//! use byteable::{Byteable, IntoByteArray, TryFromByteArray};
//!
//! # #[cfg(feature = "derive")]
//! #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
//! #[repr(u8)]
//! enum Status {
//!     Idle = 0,
//!     Running = 1,
//!     Completed = 2,
//!     Failed = 3,
//! }
//!
//! # #[cfg(feature = "derive")]
//! # fn main() -> Result<(), byteable::EnumFromBytesError> {
//! let status = Status::Running;
//! let bytes = status.into_byte_array();
//! assert_eq!(bytes, [1]);
//!
//! // Use TryFromByteArray for fallible conversion
//! let restored = Status::try_from_byte_array(bytes)?;
//! assert_eq!(restored, Status::Running);
//!
//! // Invalid discriminants return an error
//! let invalid = Status::try_from_byte_array([255]);
//! assert!(invalid.is_err());
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "derive"))]
//! # fn main() {}
//! ```
//!
//! ### Enum with Endianness
//!
//! Enums support type-level endianness attributes for multi-byte discriminants:
//!
//! ```
//! # #[cfg(feature = "derive")]
//! use byteable::Byteable;
//!
//! # #[cfg(feature = "derive")]
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
//! # #[cfg(feature = "derive")]
//! // Big-endian (common for network protocols)
//! #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
//! #[repr(u32)]
//! #[byteable(big_endian)]
//! enum HttpStatus {
//!     Ok = 200,
//!     NotFound = 404,
//!     InternalError = 500,
//! }
//! # fn main() {}
//! ```
//!
//! ### Enum Requirements
//!
//! When deriving `Byteable` for enums:
//!
//! 1. **Explicit repr type required**: `#[repr(u8)]`, `#[repr(u16)]`, `#[repr(u32)]`, `#[repr(u64)]`,
//!    `#[repr(i8)]`, `#[repr(i16)]`, `#[repr(i32)]`, or `#[repr(i64)]`
//! 2. **Unit variants only**: All variants must be unit variants (no fields)
//! 3. **Explicit discriminants**: All variants must have explicit values
//! 4. **Fallible conversion**: Use `TryFromByteArray` (not `FromByteArray`) because invalid
//!    discriminants return `EnumFromBytesError`
//!
//! ### Nested Enums in Structs
//!
//! Use the `#[byteable(try_transparent)]` attribute for enum fields in structs:
//!
//! ```
//! # #[cfg(feature = "derive")]
//! use byteable::Byteable;
//!
//! # #[cfg(feature = "derive")]
//! #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
//! #[repr(u8)]
//! enum MessageType {
//!     Data = 1,
//!     Control = 2,
//!     ErrorMsg = 3,
//! }
//!
//! # #[cfg(feature = "derive")]
//! #[derive(Byteable, Clone, Copy)]
//! struct Message {
//!     #[byteable(try_transparent)]
//!     msg_type: MessageType,
//!     #[byteable(big_endian)]
//!     sequence: u32,
//!     payload: [u8; 16],
//! }
//! # fn main() {}
//! ```
//!
//! ## Safety Considerations
//!
//! The `#[derive(Byteable)]` macro uses `core::mem::transmute` internally, which is unsafe.
//! When using this macro, ensure that:
//!
//! 1. All fields are primitive types or have endianness attributes (`#[byteable(big_endian)]`, `#[byteable(little_endian)]`)
//! 2. The struct doesn't contain types with invalid bit patterns (e.g., `bool`, `char`)
//! 3. C-like enums with explicit discriminants are safe (supported via derive)
//! 4. Complex enums with fields are **not** safe
//!
//! For types with complex invariants (like `String`, `Vec`, references, etc.), do **not** use
//! the `Byteable` derive macro. Use only with plain old data (POD) types.
//!
//! ## Advanced Usage
//!
//! ### Custom `Byteable` Implementation
//!
//! The `#[derive(Byteable)]` macro handles most use cases automatically, including
//! endianness conversion via attributes:
//!
//! ```
//! #![cfg(feature = "derive")]
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
//! # fn main() {
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

#[cfg(feature = "std")]
mod io;

extern crate self as byteable; // used to resolve derive macros in examples etc.

#[cfg(feature = "tokio")]
mod async_io;

#[cfg(feature = "derive")]
pub use byteable_derive::{Byteable, UnsafeByteableTransmute};

pub use byte_array::ByteArray;

pub use byteable_trait::{
    AssociatedByteArray, BoolRaw, CharRaw, Discriminant, EnumFromBytesError, FromByteArray,
    HasRawType, IntoByteArray, TryFromByteArray, TryHasRawType, TryIntoByteArray,
};

#[cfg(feature = "std")]
pub use io::{ReadByteable, ReadTryByteable, TryByteableError, WriteByteable, WriteTryByteable};

#[cfg(feature = "tokio")]
pub use async_io::{
    AsyncReadByteable, AsyncReadTryByteable, AsyncWriteByteable, AsyncWriteTryByteable,
};

pub use endian::{BigEndian, EndianConvert, LittleEndian};

pub use derive_safety_helpers::ValidBytecastMarker;
