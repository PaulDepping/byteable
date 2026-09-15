//! `arrayvec`'s fixed-capacity collections — `io_only` support on every enabled I/O pipeline.

#[cfg(feature = "std")]
mod io;

#[cfg(feature = "tokio")]
mod async_io;

#[cfg(feature = "embedded-io")]
mod eio;

#[cfg(feature = "embedded-io-async")]
mod eio_async;
