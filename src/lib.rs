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
//! - **`#[derive(UnsafeByteable)]`** (with `derive` feature): A procedural macro that automatically
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
//! use byteable::{Byteable, LittleEndian, ReadByteable, WriteByteable};
//! use std::fs::File;
//!
//! #[derive(Debug, PartialEq)]
//! # #[cfg(feature = "derive")]
//! #[derive(byteable::UnsafeByteable)]
//! #[repr(C, packed)]
//! struct Packet {
//!     id: u8,
//!     length: LittleEndian<u16>,
//!     data: [u8; 4],
//! }
//!
//! # fn main() -> std::io::Result<()> {
//! # #[cfg(feature = "derive")] {
//! // Create a packet
//! let packet = Packet {
//!     id: 42,
//!     length: 1024.into(),
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
//! # }
//! # Ok(())
//! # }
//! ```
//!
//! ### Example: Network Protocol
//!
//! ```
//! use byteable::{Byteable, BigEndian, LittleEndian};
//! # #[cfg(feature = "derive")]
//! use byteable::UnsafeByteable;
//!
//! # #[cfg(feature = "derive")]
//! #[derive(UnsafeByteable, Debug, Clone, Copy)]
//! #[repr(C, packed)]
//! struct NetworkHeader {
//!     magic: BigEndian<u32>,      // Network byte order (big-endian)
//!     version: u8,
//!     flags: u8,
//!     payload_len: LittleEndian<u16>,  // Little-endian for payload
//! }
//! ```
//!
//! ### Example: Working with TCP Streams
//!
//! ```no_run
//! use byteable::{ReadByteable, WriteByteable, Byteable};
//! use std::net::TcpStream;
//!
//! # #[cfg(feature = "derive")]
//! #[derive(byteable::UnsafeByteable, Debug)]
//! #[repr(C, packed)]
//! struct Message {
//!     msg_type: u8,
//!     data: [u8; 16],
//! }
//!
//! # fn main() -> std::io::Result<()> {
//! # #[cfg(feature = "derive")] {
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
//! # }
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
//! use byteable::{AsyncReadByteable, AsyncWriteByteable, Byteable};
//! # #[cfg(feature = "tokio")]
//! use tokio::net::TcpStream;
//!
//! # #[cfg(all(feature = "derive", feature = "tokio"))]
//! #[derive(byteable::UnsafeByteable, Debug)]
//! #[repr(C, packed)]
//! struct AsyncPacket {
//!     id: u32,
//!     data: [u8; 8],
//! }
//!
//! # #[cfg(all(feature = "derive", feature = "tokio"))]
//! # #[tokio::main]
//! # async fn main() -> std::io::Result<()> {
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
//! # #[cfg(not(all(feature = "derive", feature = "tokio")))]
//! # fn main() {}
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
//! The `#[derive(UnsafeByteable)]` macro uses `std::mem::transmute` internally, which is unsafe.
//! When using this macro, ensure that:
//!
//! 1. Your struct has `#[repr(C, packed)]` or another explicit layout
//! 2. All fields in the struct implement `Byteable`
//! 3. The struct doesn't contain padding bytes with undefined values
//! 4. Reading arbitrary bytes into your struct won't violate invariants
//!
//! For types with complex invariants (like `String`, `Vec`, references, etc.), do **not** use
//! `UnsafeByteable`. Use only with plain old data (POD) types.
//!
//! ## Advanced Usage
//!
//! ### Custom `Byteable` Implementation
//!
//! You can implement `Byteable` manually for types that need special handling:
//!
//! ```
//! use byteable::{Byteable, impl_byteable_via};
//!
//! # #[cfg(feature = "derive")]
//! use byteable::{LittleEndian, UnsafeByteable};
//!
//! // Raw representation suitable for byte conversion
//! # #[cfg(feature = "derive")]
//! #[derive(UnsafeByteable)]
//! #[repr(C, packed)]
//! struct PointRaw {
//!     x: LittleEndian<i32>,
//!     y: LittleEndian<i32>,
//! }
//!
//! // User-friendly representation
//! #[derive(Debug, PartialEq)]
//! struct Point {
//!     x: i32,
//!     y: i32,
//! }
//!
//! # #[cfg(feature = "derive")]
//! impl From<Point> for PointRaw {
//!     fn from(p: Point) -> Self {
//!         Self {
//!             x: p.x.into(),
//!             y: p.y.into(),
//!         }
//!     }
//! }
//!
//! # #[cfg(feature = "derive")]
//! impl From<PointRaw> for Point {
//!     fn from(raw: PointRaw) -> Self {
//!         Self {
//!             x: raw.x.get(),
//!             y: raw.y.get(),
//!         }
//!     }
//! }
//!
//! // Implement Byteable via the raw type
//! # #[cfg(feature = "derive")]
//! impl_byteable_via!(Point => PointRaw);
//! ```
//!
//! ## Feature Flags
//!
//! - `derive`: Enables the `#[derive(UnsafeByteable)]` procedural macro (default: enabled)
//! - `tokio`: Enables async I/O traits for use with tokio (default: disabled)
//!
//! ## Performance
//!
//! This crate is designed for zero-copy, zero-overhead serialization. Operations like
//! `as_byte_array` and `from_byte_array` typically compile down to simple memory operations
//! or even no-ops when possible.

mod byte_array;
mod byteable;
mod endian;
mod io;

#[cfg(feature = "tokio")]
mod async_io;

#[cfg(feature = "derive")]
pub use byteable_derive::UnsafeByteable;

pub use byte_array::ByteArray;

pub use byteable::Byteable;

pub use io::{ReadByteable, WriteByteable};

#[cfg(feature = "tokio")]
pub use async_io::{AsyncReadByteable, AsyncWriteByteable};

pub use endian::{BigEndian, EndianConvert, LittleEndian};
