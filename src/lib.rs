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
//! - **`Byteable` Trait**: The core trait for types that can be converted to and from a byte array.
//! - **`ReadByteable` & `WriteByteable` Traits**: Extension traits for `std::io::Read` and
//!   `std::io::Write`, enabling convenient reading and writing of `Byteable` types.
//! - **`AsyncReadByteable` & `AsyncWriteByteable` Traits** (with `tokio` feature): Asynchronous
//!   counterparts to `ReadByteable` and `WriteByteable`, designed for use with `tokio`'s async I/O.
//! - **`EndianConvert` Trait & Wrappers**: Provides methods for converting primitive types between
//!   different endianness (little-endian and big-endian), along with `BigEndian<T>` and
//!   `LittleEndian<T>` wrapper types.
//! - **`#[derive(UnsafeByteableTransmute)]`** (with `derive` feature): A procedural macro that automatically
//!   implements the `Byteable` trait for structs, significantly simplifying boilerplate.
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
//! ```no_run
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
//! ```no_run
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
//! ## Safety Considerations
//!
//! The `#[derive(Byteable)]` macro uses `std::mem::transmute` internally, which is unsafe.
//! When using this macro, ensure that:
//!
//! 1. All fields are primitive types or have endianness attributes (`#[byteable(big_endian)]`, `#[byteable(little_endian)]`)
//! 2. The struct doesn't contain types with invalid bit patterns (e.g., `bool`, `char`, enums)
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
//! use byteable::Byteable;
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
//! let bytes = point.as_byte_array();
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
//! `as_byte_array` and `from_byte_array` typically compile down to simple memory operations
//! or even no-ops when possible.

mod byte_array;
mod byteable_trait;
mod derive_safety_helpers;
mod endian;
mod io;

extern crate self as byteable; // used to resolve derive macros in examples etc.

#[cfg(feature = "tokio")]
mod async_io;

#[cfg(feature = "derive")]
pub use byteable_derive::{Byteable, UnsafeByteableTransmute};

// Deprecated aliases for backwards compatibility
#[cfg(feature = "derive")]
#[deprecated(since = "0.17.0", note = "Use `UnsafeByteableTransmute` instead")]
pub use byteable_derive::UnsafeByteableTransmute as UnsafeByteable;

pub use byte_array::ByteArray;

pub use byteable_trait::Byteable;

pub use io::{ReadByteable, WriteByteable};

#[cfg(feature = "tokio")]
pub use async_io::{AsyncReadByteable, AsyncWriteByteable};

pub use endian::{BigEndian, EndianConvert, LittleEndian};

pub use derive_safety_helpers::ValidBytecastMarker;
