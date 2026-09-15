//! Support for third-party crates, grouped by crate.
//!
//! [`ordered_float`] and [`bitflags`] are `pub` because they hold user-facing docs and (for
//! `bitflags`) exported macros. `heapless`, `arrayvec`, and `tinyvec` are private - they only
//! contain trait impls for those crates' collection types, reached through this crate's own
//! `io`/`async_io`/`eio`/`eio_async` traits rather than through any path of their own.

#[cfg(feature = "ordered-float")]
pub mod ordered_float;

#[cfg(feature = "bitflags")]
pub mod bitflags;

#[cfg(feature = "heapless")]
mod heapless;

#[cfg(feature = "arrayvec")]
mod arrayvec;

#[cfg(feature = "tinyvec")]
mod tinyvec;
