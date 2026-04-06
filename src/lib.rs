//! Byte-level serialization and deserialization for Rust types.
//!
//! `byteable` provides two complementary paths for working with binary data:
//!
//! - **Fixed-size path** — For types whose wire size is known at compile time. Derive
//!   [`Byteable`] and get zero-copy [`IntoByteArray::into_byte_array`] /
//!   [`TryFromByteArray::try_from_byte_array`] with a compile-time [`IntoByteArray::BYTE_SIZE`]
//!   constant. The derive macro generates a `#[repr(C, packed)]` raw struct and uses
//!   `transmute`, so no heap allocation or per-field iteration is required.
//!
//! - **Dynamic path** — For types that contain variable-length data (strings, vecs, maps). Add
//!   `#[byteable(io_only)]` to derive [`Readable`] / [`Writable`] instead, which stream data
//!   through any [`std::io::Read`] / [`std::io::Write`] (or the async tokio equivalents when
//!   the `tokio` feature is enabled).
//!
//! # Quick Start
//!
//! ## Fixed-size struct
//!
//! ```rust
//! use byteable::{Byteable, IntoByteArray, TryFromByteArray};
//!
//! #[derive(Byteable)]
//! struct Point3D {
//!     x: f32,
//!     y: f32,
//!     z: f32,
//! }
//!
//! let p = Point3D { x: 1.0, y: 2.0, z: 3.0 };
//! let bytes: [u8; 12] = p.into_byte_array();
//! let p2 = Point3D::try_from_byte_array(bytes).unwrap();
//! assert_eq!(p.x, p2.x);
//! ```
//!
//! ## Dynamic struct (I/O streaming)
//!
//! ```rust
//! use byteable::{Byteable, Writable, Readable};
//! use byteable::io::{WriteValue, ReadValue};
//!
//! #[derive(Byteable)]
//! #[byteable(io_only)]
//! struct Waypoint {
//!     id: u32,
//!     label: String,
//! }
//!
//! let wp = Waypoint { id: 1, label: "home".into() };
//! let mut buf = Vec::new();
//! buf.write_value(&wp).unwrap();
//! let wp2 = std::io::Cursor::new(&buf).read_value::<Waypoint>().unwrap();
//! assert_eq!(wp.id, wp2.id);
//! ```
//!
//! # Feature Flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `derive` | yes | `#[derive(Byteable)]` proc-macro |
//! | `std` | yes | [`Readable`] / [`Writable`] I/O traits and `std` type impls |
//! | `tokio` | no | Async I/O traits via tokio |
//! | `ordered-float` | no | Impls for `OrderedFloat<T>` and `NotNan<T>` |
//! | `all` | no | All of the above |

#![cfg_attr(not(feature = "std"), no_std)]

extern crate self as byteable; // used to resolve derive macros in examples etc.

pub mod byteable_trait;

pub use byteable_trait::*;

#[cfg(feature = "derive")]
pub use byteable_derive::Byteable;

#[cfg(feature = "tokio")]
pub mod async_io;
#[cfg(feature = "tokio")]
mod std_types_async;
#[cfg(feature = "tokio")]
pub use async_io::*;

#[cfg(feature = "std")]
pub mod io;

#[cfg(feature = "std")]
pub use io::*;

mod core_types;

#[cfg(feature = "std")]
mod std_types;

#[cfg(feature = "ordered-float")]
pub mod ordered_float_types;
