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
//!   `#[byteable(io_only)]` to derive [`io::Readable`] / [`io::Writable`] instead, which stream data
//!   through any [`std::io::Read`] / [`std::io::Write`], or their
//!   `embedded-io`/`embedded-io-async`/tokio counterparts, depending on which features are
//!   enabled.
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
//! use byteable::Byteable;
//! use byteable::io::{Writable, Readable, WriteValue, ReadValue};
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
//! | `std` | yes | [`io::Readable`] / [`io::Writable`] I/O traits and `std` type impls |
//! | `tokio` | no | Async I/O traits via tokio |
//! | `ordered-float` | no | Impls for `OrderedFloat<T>` and `NotNan<T>` |
//! | `bitflags` | no | `impl_bitflags!`/`impl_bitflags_endian!` opt-in macros for `bitflags::Flags` types |
//! | `heapless` | no | `io_only` support for `heapless`'s fixed-capacity collections, on every enabled I/O pipeline |
//! | `arrayvec` | no | `io_only` support for `arrayvec::ArrayVec`/`ArrayString`, on every enabled I/O pipeline |
//! | `tinyvec` | no | `io_only` support for `tinyvec::ArrayVec`, on every enabled I/O pipeline |
//! | `defmt` | no | `defmt::Format` impls for this crate's own error/wrapper types (`DecodeError`, `LittleEndian<T>`, `BigEndian<T>`, `eio::EioReadableError<E>`, `eio::EioReadExactError<E>`) |
//! | `alloc` | no (implied by `std`) | `alloc`-backed collection types over the `eio` and `eio_async` modules |
//! | `embedded-io` | no | `eio` module: `embedded-io`-based (sync) I/O traits for `no_std` targets |
//! | `embedded-io-async` | no | `eio_async` module: `embedded-io-async`-based (async) I/O traits for `no_std` targets |
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

/// Hidden re-export of `tokio` so derive-generated code can name its traits without the
/// downstream crate needing `tokio` as a direct dependency of its own.
#[cfg(feature = "tokio")]
#[doc(hidden)]
pub use ::tokio as __tokio;

#[cfg(feature = "std")]
pub mod io;

#[cfg(feature = "embedded-io")]
pub mod eio;

#[cfg(feature = "embedded-io-async")]
pub mod eio_async;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(all(feature = "embedded-io", feature = "alloc"))]
mod alloc_types_eio;

#[cfg(all(feature = "embedded-io-async", feature = "alloc"))]
mod alloc_types_eio_async;

mod core_types;

#[cfg(feature = "std")]
mod std_types;

pub mod ext;

/// Hidden re-export of `bitflags` so [`impl_bitflags!`] can name its traits without the
/// downstream crate needing `bitflags` as a direct dependency of its own.
#[cfg(feature = "bitflags")]
#[doc(hidden)]
pub use ::bitflags as __bitflags;
