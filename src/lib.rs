//! # Byteable
//!
//! A Rust crate for converting Rust types to and from byte arrays, facilitating
//! easy serialization and deserialization, especially for network protocols or
//! embedded systems. It provides traits for working with byte arrays,
//! byteable types, and handling endianness.
//!
//! ## Features
//! - `derive`: Enables the `Byteable` derive macro for automatic implementation of the `Byteable` trait.
//! - `tokio`: Provides asynchronous read and write capabilities using `tokio`'s I/O traits.
//!
//! ## Usage
//!
//! ### Basic Byteable Conversion
//!
//! Implement the `Byteable` trait manually or use the `#[derive(Byteable)]` macro (with the `derive` feature enabled):
//!
//! ```rust
//! use byteable::{Byteable, ReadByteable, WriteByteable, LittleEndian};
//! use std::io::Cursor;
//!
//! #[derive(Byteable, Clone, Copy, PartialEq, Debug)]
//! #[repr(C, packed)]
//! struct MyPacket {
//!     id: u16,
//!     value: LittleEndian<u32>,
//! }
//!
//! let packet = MyPacket {
//!     id: 123,
//!     value: LittleEndian::new(0x01020304),
//! };
//!
//! // Convert to byte array
//! let byte_array = packet.as_bytearray();
//!
//! // Write to a writer. Cursor implements `std::io::Write`,
//! // thus it gains `write_one` from `WriteByteable`.
//! let mut buffer = Cursor::new(vec![]);
//! buffer.write_one(packet).unwrap();
//! assert_eq!(buffer.into_inner(), vec![123, 0, 4, 3, 2, 1]);
//!
//! // Read from a reader. Cursor implements `std::io::Read`,
//! // thus it gains `read_one` from `ReadByteable`.
//! let mut reader = Cursor::new(vec![123, 0, 4, 3, 2, 1]);
//! let read_packet: MyPacket = reader.read_one().unwrap();
//! assert_eq!(read_packet, packet);
//! ```
//!
//! ### Endianness Handling
//!
//! Use `BigEndian<T>` or `LittleEndian<T>` wrappers to control the byte order of primitive types.
//!
//! ```rust
//! use byteable::{BigEndian, LittleEndian, Endianable};
//!
//! let value_be = BigEndian::new(0x01020304u32);
//! assert_eq!(value_be.get_raw().to_ne_bytes(), [1, 2, 3, 4]);
//!
//! let value_le = LittleEndian::new(0x01020304u32);
//! assert_eq!(value_le.get_raw().to_ne_bytes(), [4, 3, 2, 1]);
//! ```
//!
//! ### Asynchronous I/O (with `tokio` feature)
//!
//! ```rust
//! #[cfg(feature = "tokio")]
//! async fn async_example() -> std::io::Result<()> {
//!     use byteable::{Byteable, AsyncReadByteable, AsyncWriteByteable, LittleEndian};
//!     use std::io::Cursor;
//!
//!     #[derive(Byteable, Clone, Copy, PartialEq, Debug)]
//!     #[repr(C, packed)]
//!     struct AsyncPacket {
//!         sequence: u8,
//!         data: LittleEndian<u16>,
//!     }
//!
//!     let packet = AsyncPacket {
//!         sequence: 5,
//!         data: LittleEndian::new(0xAABB),
//!     };
//!
//!     let mut buffer = Cursor::new(vec![]);
//!     buffer.write_one(packet).await?;
//!     assert_eq!(buffer.into_inner(), vec![5, 0xBB, 0xAA]);
//!
//!     let mut reader = Cursor::new(vec![5, 0xBB, 0xAA]);
//!     let read_packet: AsyncPacket = reader.read_one().await?;
//!     assert_eq!(read_packet, packet);
//!     Ok(())
//! }
//! ```

// Submodules
mod byte_array;
mod byteable;
mod endian;
mod io;

#[cfg(feature = "tokio")]
mod async_io;

// Re-export derive macro
#[cfg(feature = "derive")]
pub use byteable_derive::Byteable;

// Re-export from byte_array module
pub use byte_array::ByteableByteArray;

// Re-export from byteable module
pub use byteable::{Byteable, ByteableRaw, ByteableRegular};

// Re-export from io module
pub use io::{ReadByteable, WriteByteable};

// Re-export from async_io module (when tokio feature is enabled)
#[cfg(feature = "tokio")]
pub use async_io::{AsyncReadByteable, AsyncWriteByteable};

// Re-export from endian module
pub use endian::{BigEndian, Endianable, LittleEndian};
