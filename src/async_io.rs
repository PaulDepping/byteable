//! Async I/O traits for reading and writing byteable values via tokio.
//!
//! This module is only available when the `tokio` feature is enabled. It mirrors the
//! synchronous API in [`crate::io`] but uses [`tokio::io::AsyncReadExt`] /
//! [`tokio::io::AsyncWriteExt`] and returns `impl Future` from each method.

use std::io;

use crate::{PlainOldData, RawRepr, ReadableError, TryFromRawRepr};

/// Async counterpart of [`crate::io::FixedReadable`].
///
/// Deserializes a fixed-size value from an async reader by filling a zeroed raw buffer
/// with `read_exact`, then validating and converting via [`TryFromRawRepr`].
///
/// A blanket impl covers all types that implement [`TryFromRawRepr`].
///
/// Prefer the extension method [`AsyncReadFixed::read_fixed`] over calling this trait directly.
///
/// # Errors
///
/// Returns [`ReadableError`] if the read fails or the bytes are not a valid encoding.
pub trait AsyncFixedReadable: Sized {
    /// Read exactly `size_of::<Self::Raw>()` bytes from `reader` and decode them into `Self`.
    ///
    /// # Errors
    ///
    /// Returns [`ReadableError::Io`] on I/O failure or [`ReadableError::DecodeError`]
    /// if the bytes do not encode a valid `Self`.
    fn read_fixed_from(
        reader: &mut (impl tokio::io::AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>>;
}

impl<T: TryFromRawRepr> AsyncFixedReadable for T {
    #[inline]
    fn read_fixed_from(
        reader: &mut (impl tokio::io::AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
            let mut b = T::Raw::zeroed();
            reader.read_exact(b.as_bytes_mut()).await?;
            let r = T::try_from_raw(b)?;
            Ok(r)
        }
    }
}

/// Async counterpart of [`crate::io::Readable`].
///
/// Deserializes a value (possibly variable-length) from an async reader. Variable-length
/// types implement this directly; fixed-size types get a blanket impl via
/// [`AsyncFixedReadable`].
///
/// Prefer the extension method [`AsyncReadValue::read_value`] over calling this directly.
///
/// # Errors
///
/// Returns [`ReadableError`] if the read fails or the bytes are not a valid encoding.
pub trait AsyncReadable: Sized {
    /// Read a value from `reader`.
    ///
    /// # Errors
    ///
    /// Returns [`ReadableError`] on I/O failure or decode error.
    fn read_from(
        reader: &mut (impl tokio::io::AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>>;
}

impl<T: AsyncFixedReadable> AsyncReadable for T {
    #[inline]
    fn read_from(
        reader: &mut (impl tokio::io::AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        T::read_fixed_from(reader)
    }
}

/// Async counterpart of [`crate::io::FixedWritable`].
///
/// Serializes a fixed-size value to an async writer by converting to a raw representation
/// and calling `write_all`. A blanket impl covers all types that implement [`RawRepr`].
///
/// Prefer the extension method [`AsyncWriteFixed::write_fixed`] over calling this directly.
pub trait AsyncFixedWritable {
    /// Write the fixed-size byte representation of `self` to `writer`.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if writing fails.
    fn write_fixed_to(
        &self,
        writer: &mut (impl tokio::io::AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>>;
}

impl<T: RawRepr> AsyncFixedWritable for T {
    #[inline]
    fn write_fixed_to(
        &self,
        writer: &mut (impl tokio::io::AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        async {
            let raw = self.to_raw();
            writer.write_all(raw.as_bytes()).await
        }
    }
}

/// Async counterpart of [`crate::io::Writable`].
///
/// Serializes a value (possibly variable-length) to an async writer. Variable-length types
/// implement this directly; fixed-size types get a blanket impl via [`AsyncFixedWritable`].
///
/// Prefer the extension method [`AsyncWriteValue::write_value`] over calling this directly.
pub trait AsyncWritable {
    /// Write `self` to `writer`.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if writing fails.
    fn write_to(
        &self,
        writer: &mut (impl tokio::io::AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>>;
}

impl<T: AsyncFixedWritable> AsyncWritable for T {
    #[inline]
    fn write_to(
        &self,
        writer: &mut (impl tokio::io::AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        self.write_fixed_to(writer)
    }
}

/// Extension trait that adds [`read_fixed`](AsyncReadFixed::read_fixed) to any async reader.
///
/// Automatically implemented for all `T: AsyncReadExt + Unpin`. Async counterpart of
/// [`crate::io::ReadFixed`].
pub trait AsyncReadFixed: tokio::io::AsyncReadExt + Unpin {
    /// Read an [`AsyncFixedReadable`] value from this async reader.
    ///
    /// # Errors
    ///
    /// Returns [`ReadableError`] on I/O failure or decode error.
    #[inline]
    fn read_fixed<T: AsyncFixedReadable>(
        &mut self,
    ) -> impl Future<Output = Result<T, ReadableError>> {
        T::read_fixed_from(self)
    }
}

impl<T: tokio::io::AsyncReadExt + ?Sized + Unpin> AsyncReadFixed for T {}

/// Extension trait that adds [`read_value`](AsyncReadValue::read_value) to any async reader.
///
/// Automatically implemented for all `T: AsyncReadExt + Unpin`. Async counterpart of
/// [`crate::io::ReadValue`].
pub trait AsyncReadValue: tokio::io::AsyncReadExt + Unpin {
    /// Read an [`AsyncReadable`] value from this async reader.
    ///
    /// # Errors
    ///
    /// Returns [`ReadableError`] on I/O failure or decode error.
    #[inline]
    fn read_value<T: AsyncReadable>(&mut self) -> impl Future<Output = Result<T, ReadableError>> {
        T::read_from(self)
    }
}

impl<T: tokio::io::AsyncReadExt + ?Sized + Unpin> AsyncReadValue for T {}

/// Extension trait that adds [`write_fixed`](AsyncWriteFixed::write_fixed) to any async writer.
///
/// Automatically implemented for all `T: AsyncWriteExt + Unpin`. Async counterpart of
/// [`crate::io::WriteFixed`].
pub trait AsyncWriteFixed: tokio::io::AsyncWriteExt + Unpin {
    /// Write an [`AsyncFixedWritable`] value to this async writer.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if writing fails.
    #[inline]
    fn write_fixed(
        &mut self,
        val: &impl AsyncFixedWritable,
    ) -> impl Future<Output = io::Result<()>> {
        val.write_fixed_to(self)
    }
}

impl<T: tokio::io::AsyncWriteExt + ?Sized + Unpin> AsyncWriteFixed for T {}

/// Extension trait that adds [`write_value`](AsyncWriteValue::write_value) to any async writer.
///
/// Automatically implemented for all `T: AsyncWriteExt + Unpin`. Async counterpart of
/// [`crate::io::WriteValue`].
pub trait AsyncWriteValue: tokio::io::AsyncWriteExt + Unpin {
    /// Write an [`AsyncWritable`] value to this async writer.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if writing fails.
    #[inline]
    fn write_value<T: AsyncWritable + ?Sized>(
        &mut self,
        data: &T,
    ) -> impl Future<Output = io::Result<()>> {
        data.write_to(self)
    }
}

impl<T: tokio::io::AsyncWriteExt + ?Sized + Unpin> AsyncWriteValue for T {}
