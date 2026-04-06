use core::{error, fmt};
use std::io::{self, Read, Write};

use crate::{PlainOldData, RawRepr, TryFromRawRepr, byteable_trait::DecodeError};

/// Error returned when reading a value from a [`Read`] source fails.
///
/// Wraps either an I/O error from the underlying reader or a [`DecodeError`] produced
/// when the bytes are valid I/O but do not represent a valid value of the target type.
#[derive(Debug)]
pub enum ReadableError {
    /// An I/O error occurred while reading bytes from the underlying reader.
    Io(std::io::Error),
    /// The bytes were read successfully but could not be decoded into the target type.
    DecodeError(DecodeError),
}

impl fmt::Display for ReadableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadableError::Io(error) => error.fmt(f),
            ReadableError::DecodeError(decode_error) => decode_error.fmt(f),
        }
    }
}

impl error::Error for ReadableError {}

impl From<std::io::Error> for ReadableError {
    #[inline]
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<DecodeError> for ReadableError {
    #[inline]
    fn from(value: DecodeError) -> Self {
        Self::DecodeError(value)
    }
}

/// Deserialize a fixed-size value from a [`Read`] source.
///
/// A blanket impl covers all types that implement [`TryFromRawRepr`]: it allocates a
/// zeroed raw buffer, fills it with `read_exact`, then calls `try_from_raw` to validate
/// and convert.
///
/// Prefer the extension method [`ReadFixed::read_fixed`] over calling this trait directly.
///
/// # Errors
///
/// Returns [`ReadableError`] if the read fails or the bytes are not a valid encoding.
pub trait FixedReadable: Sized {
    /// Read exactly `size_of::<Self::Raw>()` bytes and decode them into `Self`.
    ///
    /// # Errors
    ///
    /// Returns [`ReadableError::Io`] on I/O failure or [`ReadableError::DecodeError`]
    /// if the bytes do not encode a valid `Self`.
    fn read_fixed_from(reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError>;
}

impl<T: TryFromRawRepr> FixedReadable for T {
    #[inline]
    fn read_fixed_from(reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let mut b = T::Raw::zeroed();
        reader.read_exact(b.as_bytes_mut())?;
        let r = T::try_from_raw(b)?;
        Ok(r)
    }
}

/// Deserialize a value (possibly variable-length) from a [`Read`] source.
///
/// This is the more general counterpart to [`FixedReadable`]. Variable-length types such
/// as `Vec<T>`, `String`, `HashMap`, and `Option<T>` implement `Readable` directly.
/// Fixed-size types get a blanket impl that delegates to [`FixedReadable`].
///
/// Prefer the extension method [`ReadValue::read_value`] over calling this trait directly.
///
/// # Errors
///
/// Returns [`ReadableError`] if the read fails or the bytes are not a valid encoding.
pub trait Readable: Sized {
    /// Read a value from `reader`.
    ///
    /// # Errors
    ///
    /// Returns [`ReadableError`] on I/O failure or decode error.
    fn read_from(reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError>;
}

impl<T: FixedReadable> Readable for T {
    #[inline]
    fn read_from(reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        T::read_fixed_from(reader)
    }
}

/// Serialize a fixed-size value to a [`Write`] sink.
///
/// A blanket impl covers all types that implement [`RawRepr`]: it calls `to_raw()` and
/// writes the raw bytes with `write_all`.
///
/// Prefer the extension method [`WriteFixed::write_fixed`] over calling this trait directly.
pub trait FixedWritable {
    /// Write the fixed-size byte representation of `self` to `writer`.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if writing fails.
    fn write_fixed_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()>;
}

impl<T: RawRepr> FixedWritable for T {
    #[inline]
    fn write_fixed_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        let raw = self.to_raw();
        writer.write_all(raw.as_bytes())
    }
}

/// Serialize a value (possibly variable-length) to a [`Write`] sink.
///
/// This is the more general counterpart to [`FixedWritable`]. Variable-length types such
/// as `Vec<T>`, `String`, `HashMap`, and slices implement `Writable` directly.
/// Fixed-size types get a blanket impl that delegates to [`FixedWritable`].
///
/// Prefer the extension method [`WriteValue::write_value`] over calling this trait directly.
pub trait Writable {
    /// Write `self` to `writer`.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if writing fails.
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()>;
}

impl<T: FixedWritable> Writable for T {
    #[inline]
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        self.write_fixed_to(writer)
    }
}

/// Extension trait that adds [`read_fixed`](ReadFixed::read_fixed) to any [`Read`] impl.
///
/// Automatically implemented for all `T: Read`. Use this to read fixed-size types
/// ergonomically without naming the source trait explicitly:
///
/// ```rust
/// use byteable::io::ReadFixed;
/// use std::io::Cursor;
///
/// let mut cursor = Cursor::new([1u8, 0, 0, 0]);
/// let n: u32 = cursor.read_fixed().unwrap();
/// assert_eq!(n, 1u32);
/// ```
pub trait ReadFixed: Read {
    /// Read a [`FixedReadable`] value from this reader.
    ///
    /// # Errors
    ///
    /// Returns [`ReadableError`] on I/O failure or decode error.
    #[inline]
    fn read_fixed<T: FixedReadable>(&mut self) -> Result<T, ReadableError> {
        T::read_fixed_from(self)
    }
}

impl<T: Read + ?Sized> ReadFixed for T {}

/// Extension trait that adds [`read_value`](ReadValue::read_value) to any [`Read`] impl.
///
/// Automatically implemented for all `T: Read`. Use this to read variable-length or
/// fixed-size [`Readable`] types ergonomically:
///
/// ```rust
/// use byteable::io::ReadValue;
/// use std::io::Cursor;
///
/// // Write a length-prefixed string manually, then read it back.
/// let mut data = Vec::new();
/// data.extend_from_slice(&4u64.to_le_bytes()); // length
/// data.extend_from_slice(b"test");
/// let s: String = Cursor::new(data).read_value().unwrap();
/// assert_eq!(s, "test");
/// ```
pub trait ReadValue: Read {
    /// Read a [`Readable`] value from this reader.
    ///
    /// # Errors
    ///
    /// Returns [`ReadableError`] on I/O failure or decode error.
    #[inline]
    fn read_value<T: Readable>(&mut self) -> Result<T, ReadableError> {
        T::read_from(self)
    }
}

impl<T: Read> ReadValue for T {}

/// Extension trait that adds [`write_fixed`](WriteFixed::write_fixed) to any [`Write`] impl.
///
/// Automatically implemented for all `T: Write`.
pub trait WriteFixed: Write {
    /// Write a [`FixedWritable`] value to this writer.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if writing fails.
    #[inline]
    fn write_fixed(&mut self, val: &impl FixedWritable) -> io::Result<()> {
        val.write_fixed_to(self)
    }
}

impl<T: Write> WriteFixed for T {}

/// Extension trait that adds [`write_value`](WriteValue::write_value) to any [`Write`] impl.
///
/// Automatically implemented for all `T: Write`. Use this to write variable-length or
/// fixed-size [`Writable`] types ergonomically:
///
/// ```rust
/// use byteable::io::WriteValue;
///
/// let mut buf = Vec::new();
/// buf.write_value("hello").unwrap();
/// // Wire format: 8-byte LE length prefix + UTF-8 bytes
/// assert_eq!(&buf[..8], &5u64.to_le_bytes());
/// assert_eq!(&buf[8..], b"hello");
/// ```
pub trait WriteValue: Write {
    /// Write a [`Writable`] value to this writer.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if writing fails.
    #[inline]
    fn write_value<T: Writable + ?Sized>(&mut self, data: &T) -> io::Result<()> {
        data.write_to(self)
    }
}

impl<T: Write> WriteValue for T {}
