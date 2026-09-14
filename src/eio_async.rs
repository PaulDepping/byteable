//! `embedded-io-async` support — the async counterpart to [`crate::eio`] (sync) and
//! [`crate::async_io`] (tokio). Mirrors `eio.rs`'s shape, but every method returns
//! `impl Future<Output = ...>` instead of a bare `Result`, generic over the
//! [`EioAsyncReader`]/[`EioAsyncWriter`] marker traits defined here rather than bound directly
//! to `embedded_io_async::Read`/`Write`. Gated on the `embedded-io-async` feature (`src/lib.rs`),
//! same as `eio.rs` is gated on `embedded-io` — `embedded-io-async` implies `embedded-io`
//! (`Cargo.toml`), so `crate::eio` is always available here.
//!
//! Reuses [`crate::eio::EioReadExactError`] and [`crate::eio::EioReadableError`] directly rather
//! than duplicating them — both are already generic over the backend's error type with no
//! sync-specific bound, and `embedded-io-async` re-exports `embedded-io`'s own `ReadExactError`
//! rather than defining its own, so there is nothing sync-specific to diverge from.

use crate::{
    DecodeError, PlainOldData, RawRepr, TryFromRawRepr,
    eio::{EioReadExactError, EioReadableError},
};

/// Minimal async read primitive. Bridged to `embedded_io_async::Read` when the
/// `embedded-io-async` feature is on.
///
/// Deliberately mirrors [`crate::eio::EioReader`]'s method surface (not just the raw `read`
/// primitive) so that any specialized override a concrete driver provides for `read_exact`
/// (e.g. a DMA-driven bulk transfer) is what actually runs, not a generic loop of ours.
pub trait EioAsyncReader {
    /// The backend's own error type.
    type Error;

    /// Read some bytes into `buf`, returning how many were read.
    fn read(&mut self, buf: &mut [u8]) -> impl Future<Output = Result<usize, Self::Error>>;

    /// Read exactly `buf.len()` bytes, waiting as needed.
    fn read_exact(
        &mut self,
        buf: &mut [u8],
    ) -> impl Future<Output = Result<(), EioReadExactError<Self::Error>>>;
}

/// Minimal async write primitive. Bridged to `embedded_io_async::Write` when the
/// `embedded-io-async` feature is on.
pub trait EioAsyncWriter {
    /// The backend's own error type.
    type Error;

    /// Write some bytes from `buf`, returning how many were written.
    fn write(&mut self, buf: &[u8]) -> impl Future<Output = Result<usize, Self::Error>>;

    /// Write all of `buf`, waiting as needed.
    fn write_all(&mut self, buf: &[u8]) -> impl Future<Output = Result<(), Self::Error>>;

    /// Flush any buffered output.
    fn flush(&mut self) -> impl Future<Output = Result<(), Self::Error>>;
}

impl<T: ::embedded_io_async::Read + ?Sized> EioAsyncReader for T {
    type Error = T::Error;

    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> impl Future<Output = Result<usize, Self::Error>> {
        ::embedded_io_async::Read::read(self, buf)
    }

    #[inline]
    fn read_exact(
        &mut self,
        buf: &mut [u8],
    ) -> impl Future<Output = Result<(), EioReadExactError<Self::Error>>> {
        async move {
            ::embedded_io_async::Read::read_exact(self, buf)
                .await
                .map_err(|e| match e {
                    ::embedded_io::ReadExactError::UnexpectedEof => {
                        EioReadExactError::UnexpectedEof
                    }
                    ::embedded_io::ReadExactError::Other(err) => EioReadExactError::Other(err),
                })
        }
    }
}

impl<T: ::embedded_io_async::Write + ?Sized> EioAsyncWriter for T {
    type Error = T::Error;

    #[inline]
    fn write(&mut self, buf: &[u8]) -> impl Future<Output = Result<usize, Self::Error>> {
        ::embedded_io_async::Write::write(self, buf)
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> impl Future<Output = Result<(), Self::Error>> {
        ::embedded_io_async::Write::write_all(self, buf)
    }

    #[inline]
    fn flush(&mut self) -> impl Future<Output = Result<(), Self::Error>> {
        ::embedded_io_async::Write::flush(self)
    }
}

/// `eio_async` counterpart of [`crate::eio::EioFixedReadable`].
pub trait EioAsyncFixedReadable: Sized {
    /// Read exactly `size_of::<Self::Raw>()` bytes and decode them into `Self`.
    fn read_fixed_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>>;
}

impl<T: TryFromRawRepr> EioAsyncFixedReadable for T {
    #[inline]
    fn read_fixed_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
            let mut b = T::Raw::zeroed();
            reader.read_exact(b.as_bytes_mut()).await?;
            let r = T::try_from_raw(b)?;
            Ok(r)
        }
    }
}

/// `eio_async` counterpart of [`crate::eio::EioReadable`].
pub trait EioAsyncReadable: Sized {
    /// Read a value from `reader`.
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>>;
}

impl<T: EioAsyncFixedReadable> EioAsyncReadable for T {
    #[inline]
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        T::read_fixed_from(reader)
    }
}

/// `eio_async` counterpart of [`crate::eio::EioFixedWritable`].
pub trait EioAsyncFixedWritable {
    /// Write the fixed-size byte representation of `self` to `writer`.
    fn write_fixed_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>>;
}

impl<T: RawRepr> EioAsyncFixedWritable for T {
    #[inline]
    fn write_fixed_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
        async move {
            let raw = self.to_raw();
            writer.write_all(raw.as_bytes()).await
        }
    }
}

/// `eio_async` counterpart of [`crate::eio::EioWritable`].
pub trait EioAsyncWritable {
    /// Write `self` to `writer`.
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>>;
}

impl<T: EioAsyncFixedWritable> EioAsyncWritable for T {
    #[inline]
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
        self.write_fixed_to(writer)
    }
}

/// Extension trait adding [`read_fixed`](EioAsyncReadFixed::read_fixed) to any [`EioAsyncReader`].
pub trait EioAsyncReadFixed: EioAsyncReader {
    #[inline]
    fn read_fixed<T: EioAsyncFixedReadable>(
        &mut self,
    ) -> impl Future<Output = Result<T, EioReadableError<Self::Error>>> {
        T::read_fixed_from(self)
    }
}
impl<T: EioAsyncReader + ?Sized> EioAsyncReadFixed for T {}

/// Extension trait adding [`read_value`](EioAsyncReadValue::read_value) to any [`EioAsyncReader`].
pub trait EioAsyncReadValue: EioAsyncReader {
    #[inline]
    fn read_value<T: EioAsyncReadable>(
        &mut self,
    ) -> impl Future<Output = Result<T, EioReadableError<Self::Error>>> {
        T::read_from(self)
    }
}
impl<T: EioAsyncReader + ?Sized> EioAsyncReadValue for T {}

/// Extension trait adding [`write_fixed`](EioAsyncWriteFixed::write_fixed) to any [`EioAsyncWriter`].
pub trait EioAsyncWriteFixed: EioAsyncWriter {
    #[inline]
    fn write_fixed(
        &mut self,
        val: &impl EioAsyncFixedWritable,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        val.write_fixed_to(self)
    }
}
impl<T: EioAsyncWriter + ?Sized> EioAsyncWriteFixed for T {}

/// Extension trait adding [`write_value`](EioAsyncWriteValue::write_value) to any [`EioAsyncWriter`].
pub trait EioAsyncWriteValue: EioAsyncWriter {
    #[inline]
    fn write_value<T: EioAsyncWritable + ?Sized>(
        &mut self,
        data: &T,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        data.write_to(self)
    }
}
impl<T: EioAsyncWriter + ?Sized> EioAsyncWriteValue for T {}

// Wire format: 1-byte tag (0 = None, 1 = Some), followed by the value when Some. No alloc needed.
impl<T: EioAsyncReadable> EioAsyncReadable for Option<T> {
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
            let tag: u8 = reader.read_fixed().await?;
            match tag {
                0 => Ok(None),
                1 => Ok(Some(reader.read_value().await?)),
                _ => Err(EioReadableError::DecodeError(DecodeError::InvalidTag {
                    raw: tag,
                    type_name: "Option",
                })),
            }
        }
    }
}

impl<T: EioAsyncWritable> EioAsyncWritable for Option<T> {
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
        async move {
            match self {
                None => writer.write_fixed(&0u8).await,
                Some(val) => {
                    writer.write_fixed(&1u8).await?;
                    writer.write_value(val).await
                }
            }
        }
    }
}

// Wire format: 1-byte tag (0 = Ok, 1 = Err), followed by the Ok value or Err value.
impl<V: EioAsyncReadable, E: EioAsyncReadable> EioAsyncReadable for Result<V, E> {
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
            let discriminator: u8 = reader.read_fixed().await?;
            match discriminator {
                0 => Ok(Ok(reader.read_value().await?)),
                1 => Ok(Err(reader.read_value().await?)),
                _ => Err(EioReadableError::DecodeError(DecodeError::InvalidTag {
                    raw: discriminator,
                    type_name: "Result",
                })),
            }
        }
    }
}

impl<V: EioAsyncWritable, Er: EioAsyncWritable> EioAsyncWritable for Result<V, Er> {
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
        async move {
            match self {
                Ok(val) => {
                    writer.write_fixed(&0u8).await?;
                    writer.write_value(val).await
                }
                Err(err) => {
                    writer.write_fixed(&1u8).await?;
                    writer.write_value(err).await
                }
            }
        }
    }
}

// Wire format: `u64` element count (LE) + elements in order. Write-only (borrowed): never
// allocates, so this is available without the `alloc` feature.
impl<T: EioAsyncWritable> EioAsyncWritable for [T] {
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
        async move {
            let len: u64 = self
                .len()
                .try_into()
                .expect("could not convert usize to u64");
            writer.write_fixed(&len).await?;
            for el in self {
                writer.write_value(el).await?;
            }
            Ok(())
        }
    }
}

// Wire format: `u64` byte length (LE) + UTF-8 bytes. Write-only (borrowed), no `alloc` needed.
impl EioAsyncWritable for str {
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
        async move {
            let len: u64 = self
                .len()
                .try_into()
                .expect("could not convert usize to u64");
            writer.write_fixed(&len).await?;
            writer.write_all(self.as_bytes()).await
        }
    }
}
