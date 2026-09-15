//! `embedded-io` (sync) support — the `no_std`-friendly counterpart to [`crate::io`].
//!
//! Mirrors `io.rs`'s shape, but is generic over the [`EioReader`]/[`EioWriter`] marker traits
//! defined here rather than bound directly to `embedded_io::Read`/`Write`. Gated on the
//! `embedded-io` feature (`src/lib.rs`), same as `io.rs` is gated on `std`. The derive macro
//! only ever emits references to this module's traits when `byteable_derive`'s own mirrored
//! `embedded-io` feature is active (see `dynamic_pipeline_impls` in `byteable_derive/src/lib.rs`),
//! which is forwarded 1:1 from this crate's own `embedded-io` feature — so generated code and
//! this module's availability always agree; there's no case where generated code needs to
//! name-resolve these traits while this module is absent.

use crate::{DecodeError, PlainOldData, RawRepr, TryFromRawRepr};
use core::fmt;

/// Minimal read primitive. Bridged to `embedded_io::Read` when the `embedded-io` feature is on.
///
/// Deliberately mirrors `embedded_io::Read`'s method surface (not just the raw `read`
/// primitive) so that any specialized override a concrete driver provides for `read_exact`
/// (e.g. a DMA-driven bulk transfer) is what actually runs, not a generic loop of ours.
pub trait EioReader {
    /// The backend's own error type.
    type Error;

    /// Read some bytes into `buf`, returning how many were read.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    /// Read exactly `buf.len()` bytes, blocking as needed.
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), EioReadExactError<Self::Error>>;
}

/// Minimal write primitive. Bridged to `embedded_io::Write` when the `embedded-io` feature is on.
pub trait EioWriter {
    /// The backend's own error type.
    type Error;

    /// Write some bytes from `buf`, returning how many were written.
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;

    /// Write all of `buf`, blocking as needed.
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error>;

    /// Flush any buffered output.
    fn flush(&mut self) -> Result<(), Self::Error>;
}

/// Error returned by [`EioReader::read_exact`]. Mirrors `embedded_io::ReadExactError<E>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum EioReadExactError<E> {
    /// End-of-input was reached before `buf` was fully filled.
    UnexpectedEof,
    /// The underlying reader returned an error.
    Other(E),
}

impl<E: fmt::Debug> fmt::Display for EioReadExactError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(feature = "std")]
impl<E: fmt::Debug> std::error::Error for EioReadExactError<E> {}

impl<T: ::embedded_io::Read + ?Sized> EioReader for T {
    type Error = T::Error;

    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        ::embedded_io::Read::read(self, buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), EioReadExactError<Self::Error>> {
        ::embedded_io::Read::read_exact(self, buf).map_err(|e| match e {
            ::embedded_io::ReadExactError::UnexpectedEof => EioReadExactError::UnexpectedEof,
            ::embedded_io::ReadExactError::Other(err) => EioReadExactError::Other(err),
        })
    }
}

impl<T: ::embedded_io::Write + ?Sized> EioWriter for T {
    type Error = T::Error;

    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        ::embedded_io::Write::write(self, buf)
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        ::embedded_io::Write::write_all(self, buf)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        ::embedded_io::Write::flush(self)
    }
}

/// Error returned when reading a value from an [`EioReader`] source fails.
///
/// Independent of [`crate::io::ReadableError`], which stays `std::io::Error`-based and
/// untouched by this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum EioReadableError<E> {
    /// The underlying reader returned an error.
    Io(E),
    /// End-of-input was reached before a complete value could be read.
    UnexpectedEof,
    /// The bytes were read successfully but do not decode into a valid value.
    DecodeError(DecodeError),
}

impl<E> From<EioReadExactError<E>> for EioReadableError<E> {
    fn from(e: EioReadExactError<E>) -> Self {
        match e {
            EioReadExactError::UnexpectedEof => Self::UnexpectedEof,
            EioReadExactError::Other(err) => Self::Io(err),
        }
    }
}

impl<E> From<DecodeError> for EioReadableError<E> {
    fn from(e: DecodeError) -> Self {
        Self::DecodeError(e)
    }
}

impl<E: fmt::Debug> fmt::Display for EioReadableError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(feature = "std")]
impl<E: fmt::Debug> std::error::Error for EioReadableError<E> {}

/// `eio` counterpart of [`crate::io::FixedReadable`].
pub trait EioFixedReadable: Sized {
    /// Read exactly `size_of::<Self::Raw>()` bytes and decode them into `Self`.
    fn read_fixed_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>>;
}

impl<T: TryFromRawRepr> EioFixedReadable for T {
    #[inline]
    fn read_fixed_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        let mut b = T::Raw::zeroed();
        reader.read_exact(b.as_bytes_mut())?;
        let r = T::try_from_raw(b)?;
        Ok(r)
    }
}

/// `eio` counterpart of [`crate::io::Readable`].
pub trait EioReadable: Sized {
    /// Read a value from `reader`.
    fn read_from<R: EioReader + ?Sized>(reader: &mut R)
    -> Result<Self, EioReadableError<R::Error>>;
}

impl<T: EioFixedReadable> EioReadable for T {
    #[inline]
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        T::read_fixed_from(reader)
    }
}

/// `eio` counterpart of [`crate::io::FixedWritable`].
pub trait EioFixedWritable {
    /// Write the fixed-size byte representation of `self` to `writer`.
    fn write_fixed_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error>;
}

impl<T: RawRepr> EioFixedWritable for T {
    #[inline]
    fn write_fixed_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        let raw = self.to_raw();
        writer.write_all(raw.as_bytes())
    }
}

/// `eio` counterpart of [`crate::io::Writable`].
pub trait EioWritable {
    /// Write `self` to `writer`.
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error>;
}

impl<T: EioFixedWritable> EioWritable for T {
    #[inline]
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        self.write_fixed_to(writer)
    }
}

/// Extension trait adding [`read_fixed`](EioReadFixed::read_fixed) to any [`EioReader`].
pub trait EioReadFixed: EioReader {
    #[inline]
    fn read_fixed<T: EioFixedReadable>(&mut self) -> Result<T, EioReadableError<Self::Error>> {
        T::read_fixed_from(self)
    }
}
impl<T: EioReader + ?Sized> EioReadFixed for T {}

/// Extension trait adding [`read_value`](EioReadValue::read_value) to any [`EioReader`].
pub trait EioReadValue: EioReader {
    #[inline]
    fn read_value<T: EioReadable>(&mut self) -> Result<T, EioReadableError<Self::Error>> {
        T::read_from(self)
    }
}
impl<T: EioReader + ?Sized> EioReadValue for T {}

/// Extension trait adding [`write_fixed`](EioWriteFixed::write_fixed) to any [`EioWriter`].
pub trait EioWriteFixed: EioWriter {
    #[inline]
    fn write_fixed(&mut self, val: &impl EioFixedWritable) -> Result<(), Self::Error> {
        val.write_fixed_to(self)
    }
}
impl<T: EioWriter + ?Sized> EioWriteFixed for T {}

/// Extension trait adding [`write_value`](EioWriteValue::write_value) to any [`EioWriter`].
pub trait EioWriteValue: EioWriter {
    #[inline]
    fn write_value<T: EioWritable + ?Sized>(&mut self, data: &T) -> Result<(), Self::Error> {
        data.write_to(self)
    }
}
impl<T: EioWriter + ?Sized> EioWriteValue for T {}

// Wire format: 1-byte tag (0 = None, 1 = Some), followed by the value when Some. No alloc needed.
impl<T: EioReadable> EioReadable for Option<T> {
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        let tag: u8 = reader.read_fixed()?;
        match tag {
            0 => Ok(None),
            1 => Ok(Some(reader.read_value()?)),
            _ => Err(EioReadableError::DecodeError(DecodeError::InvalidTag {
                raw: tag,
                type_name: "Option",
            })),
        }
    }
}

impl<T: EioWritable> EioWritable for Option<T> {
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        match self {
            None => writer.write_fixed(&0u8),
            Some(val) => {
                writer.write_fixed(&1u8)?;
                writer.write_value(val)
            }
        }
    }
}

// Wire format: 1-byte tag (0 = Ok, 1 = Err), followed by the Ok value or Err value.
impl<V: EioReadable, E: EioReadable> EioReadable for Result<V, E> {
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        let discriminator: u8 = reader.read_fixed()?;
        match discriminator {
            0 => Ok(Ok(reader.read_value()?)),
            1 => Ok(Err(reader.read_value()?)),
            _ => Err(EioReadableError::DecodeError(DecodeError::InvalidTag {
                raw: discriminator,
                type_name: "Result",
            })),
        }
    }
}

impl<V: EioWritable, Er: EioWritable> EioWritable for Result<V, Er> {
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        match self {
            Ok(val) => {
                writer.write_fixed(&0u8)?;
                writer.write_value(val)
            }
            Err(err) => {
                writer.write_fixed(&1u8)?;
                writer.write_value(err)
            }
        }
    }
}

// Wire format: `u64` element count (LE) + elements in order. Write-only (borrowed): never
// allocates, so this is available without the `alloc` feature.
impl<T: EioWritable> EioWritable for [T] {
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        let len: u64 = self
            .len()
            .try_into()
            .expect("could not convert usize to u64");
        writer.write_fixed(&len)?;
        for el in self {
            writer.write_value(el)?;
        }
        Ok(())
    }
}

// Wire format: `u64` byte length (LE) + UTF-8 bytes. Write-only (borrowed), no `alloc` needed.
impl EioWritable for str {
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        let len: u64 = self
            .len()
            .try_into()
            .expect("could not convert usize to u64");
        writer.write_fixed(&len)?;
        writer.write_all(self.as_bytes())
    }
}
