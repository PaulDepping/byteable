//! Synchronous I/O extensions for reading and writing byteable types.
//!
//! This module provides extension traits for `std::io::Read` and `std::io::Write` that
//! enable convenient reading and writing of types implementing the byte conversion traits
//! ([`IntoByteArray`] and [`FromByteArray`]).

use crate::byte_array::ByteArray;
use crate::{FromByteArray, IntoByteArray, LittleEndian, TryFromByteArray, TryIntoByteArray};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt;
use std::hash::{BuildHasher, Hash};
use std::io::{Read, Write};

/// Low-level trait for reading a value from a [`std::io::Read`] source.
///
/// This trait is implemented for:
/// - Types implementing [`FromByteArray`] (primitives, fixed-size structs)
/// - Collection types: [`Vec`], [`VecDeque`], [`HashMap`], [`HashSet`], [`BTreeMap`], [`BTreeSet`]
/// - [`Option<T>`] where `T: Readable`
/// - [`String`]
///
/// Collections are serialized as a little-endian `u64` length prefix followed by each element.
///
/// You typically don't need to implement or call this trait directly — use
/// [`ReadByteable::read_byteable`] instead.
pub trait Readable: Sized {
    fn read_from(reader: &mut (impl Read + ?Sized)) -> std::io::Result<Self>;
}

impl<T: FromByteArray> Readable for T {
    fn read_from(reader: &mut (impl Read + ?Sized)) -> std::io::Result<Self> {
        let mut b = T::ByteArray::zeroed();
        reader.read_exact(b.as_byte_slice_mut())?;
        Ok(T::from_byte_array(b))
    }
}

impl<T: Readable> Readable for Vec<T> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> std::io::Result<Self> {
        let len: LittleEndian<u64> = reader.read_byteable()?;
        let len = len.get() as usize;
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(reader.read_byteable()?);
        }
        Ok(result)
    }
}

impl<T: Readable> Readable for VecDeque<T> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> std::io::Result<Self> {
        let len: LittleEndian<u64> = reader.read_byteable()?;
        let len = len.get() as usize;
        let mut result = VecDeque::with_capacity(len);
        for _ in 0..len {
            result.push_back(reader.read_byteable()?);
        }
        Ok(result)
    }
}

impl<K, V, S> Readable for HashMap<K, V, S>
where
    K: Readable + Eq + Hash,
    V: Readable,
    S: BuildHasher + Default,
{
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> std::io::Result<Self> {
        let len: LittleEndian<u64> = reader.read_byteable()?;
        let len = len.get() as usize;
        let mut map = HashMap::with_capacity_and_hasher(len, S::default());
        for _ in 0..len {
            let key = reader.read_byteable()?;
            let val = reader.read_byteable()?;
            map.insert(key, val);
        }
        Ok(map)
    }
}

impl<T, S> Readable for HashSet<T, S>
where
    T: Readable + Eq + Hash,
    S: BuildHasher + Default,
{
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> std::io::Result<Self> {
        let len: LittleEndian<u64> = reader.read_byteable()?;
        let len = len.get() as usize;
        let mut set = HashSet::with_capacity_and_hasher(len, S::default());
        for _ in 0..len {
            set.insert(reader.read_byteable()?);
        }
        Ok(set)
    }
}

impl<K: Readable + Ord, V: Readable> Readable for BTreeMap<K, V> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> std::io::Result<Self> {
        let len: LittleEndian<u64> = reader.read_byteable()?;
        let len = len.get() as usize;
        let mut map = BTreeMap::new();
        for _ in 0..len {
            let key = reader.read_byteable()?;
            let val = reader.read_byteable()?;
            map.insert(key, val);
        }
        Ok(map)
    }
}

impl<T: Readable + Ord> Readable for BTreeSet<T> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> std::io::Result<Self> {
        let len: LittleEndian<u64> = reader.read_byteable()?;
        let len = len.get() as usize;
        let mut set = BTreeSet::new();
        for _ in 0..len {
            set.insert(reader.read_byteable()?);
        }
        Ok(set)
    }
}

impl<T: Readable> Readable for Option<T> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> std::io::Result<Self> {
        let tag: u8 = reader.read_byteable()?;
        match tag {
            0 => Ok(None),
            1 => Ok(Some(reader.read_byteable()?)),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid Option tag byte",
            )),
        }
    }
}

impl Readable for String {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> std::io::Result<Self> {
        let len: LittleEndian<u64> = reader.read_byteable()?;
        let len = len.get() as usize;
        let mut bytes = vec![0u8; len];
        reader.read_exact(&mut bytes)?;
        String::from_utf8(bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Low-level trait for writing a value to a [`std::io::Write`] sink.
///
/// This trait is implemented for:
/// - Types implementing [`IntoByteArray`] (primitives, fixed-size structs)
/// - Collection types: [`Vec`], [`VecDeque`], [`HashMap`], [`HashSet`], [`BTreeMap`], [`BTreeSet`]
/// - [`Option<T>`] where `T: Writable`
/// - [`str`] and [`String`]
///
/// Collections are serialized as a little-endian `u64` length prefix followed by each element.
///
/// You typically don't need to implement or call this trait directly — use
/// [`WriteByteable::write_byteable`] instead.
pub trait Writable {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> std::io::Result<()>;
}

impl<T: IntoByteArray> Writable for T {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
        let byte_array = self.into_byte_array();
        writer.write_all(byte_array.as_byte_slice())
    }
}

impl<T: Writable> Writable for Vec<T> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
        let len = LittleEndian::new(self.len() as u64);
        writer.write_byteable(&len)?;
        for el in self {
            writer.write_byteable(el)?;
        }
        Ok(())
    }
}

impl<T: Writable> Writable for VecDeque<T> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
        let len = LittleEndian::new(self.len() as u64);
        writer.write_byteable(&len)?;
        for el in self {
            writer.write_byteable(el)?;
        }
        Ok(())
    }
}

impl<K, V, S> Writable for HashMap<K, V, S>
where
    K: Writable,
    V: Writable,
    S: BuildHasher,
{
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
        let len = LittleEndian::new(self.len() as u64);
        writer.write_byteable(&len)?;
        for (k, v) in self {
            writer.write_byteable(k)?;
            writer.write_byteable(v)?;
        }
        Ok(())
    }
}

impl<T, S> Writable for HashSet<T, S>
where
    T: Writable,
    S: BuildHasher,
{
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
        let len = LittleEndian::new(self.len() as u64);
        writer.write_byteable(&len)?;
        for el in self {
            writer.write_byteable(el)?;
        }
        Ok(())
    }
}

impl<K: Writable, V: Writable> Writable for BTreeMap<K, V> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
        let len = LittleEndian::new(self.len() as u64);
        writer.write_byteable(&len)?;
        for (k, v) in self {
            writer.write_byteable(k)?;
            writer.write_byteable(v)?;
        }
        Ok(())
    }
}

impl<T: Writable> Writable for BTreeSet<T> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
        let len = LittleEndian::new(self.len() as u64);
        writer.write_byteable(&len)?;
        for el in self {
            writer.write_byteable(el)?;
        }
        Ok(())
    }
}

impl<T: Writable> Writable for Option<T> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
        match self {
            None => writer.write_byteable(&0u8),
            Some(val) => {
                writer.write_byteable(&1u8)?;
                writer.write_byteable(val)
            }
        }
    }
}

impl Writable for str {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
        let len = LittleEndian::new(self.len() as u64);
        writer.write_byteable(&len)?;
        writer.write_all(self.as_bytes())
    }
}

impl Writable for String {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
        self.as_str().write_to(writer)
    }
}

/// Extension trait for `Read` that adds methods for reading [`Readable`] types.
///
/// This trait is automatically implemented for all types that implement `std::io::Read`,
/// providing convenient methods for reading binary data directly into Rust types.
///
/// The `T` in `read_byteable::<T>()` must implement [`Readable`], which covers:
/// - Primitive types and fixed-size structs (via [`FromByteArray`])
/// - Collections ([`Vec`], [`VecDeque`], [`HashMap`], [`HashSet`], [`BTreeMap`], [`BTreeSet`])
///   serialized as a little-endian `u64` length prefix followed by each element
/// - [`Option<T>`], [`String`]
///
/// # Examples
///
/// ## Reading from a file
///
/// ```no_run
/// # #![cfg(feature = "derive")]
/// use byteable::{Byteable, ReadByteable};
/// use std::fs::File;
///
/// #[derive(Byteable, Debug, Clone, Copy)]
/// struct Header {
///     #[byteable(big_endian)]
///     magic: u32,
///     #[byteable(little_endian)]
///     version: u16,
///     #[byteable(little_endian)]
///     flags: u16,
/// }
///
/// # fn main() -> std::io::Result<()> {
/// let mut file = File::open("data.bin")?;
/// let header: Header = file.read_byteable()?;
/// println!("Header: {:?}", header);
/// # Ok(())
/// # }
/// ```
///
/// ## Reading from a TCP stream
///
/// ```no_run
/// use byteable::ReadByteable;
/// use std::net::TcpStream;
///
/// # fn main() -> std::io::Result<()> {
/// let mut stream = TcpStream::connect("127.0.0.1:8080")?;
///
/// // Read a u32 length prefix
/// let length: u32 = stream.read_byteable()?;
/// println!("Message length: {}", length);
/// # Ok(())
/// # }
/// ```
///
/// ## Reading multiple values
///
/// ```no_run
/// use byteable::ReadByteable;
/// use std::io::Cursor;
///
/// # fn main() -> std::io::Result<()> {
/// let data = vec![1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0];
/// let mut cursor = Cursor::new(data);
///
/// let a: u32 = cursor.read_byteable()?;
/// let b: u32 = cursor.read_byteable()?;
/// let c: u32 = cursor.read_byteable()?;
///
/// #[cfg(target_endian = "little")]
/// assert_eq!((a, b, c), (1, 2, 3));
/// # Ok(())
/// # }
/// ```
pub trait ReadByteable: Read {
    /// Reads a [`Readable`] type from this reader.
    ///
    /// Delegates to `T`'s [`Readable`] implementation. For fixed-size types this reads a
    /// fixed number of bytes; for collection types this reads a length-prefixed sequence.
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - The reader reaches EOF before all required bytes have been read
    /// - Any underlying I/O error occurs
    ///
    /// # Examples
    ///
    /// ```
    /// use byteable::{Byteable, ReadByteable};
    /// use std::io::Cursor;
    ///
    /// let data = vec![0x12, 0x34, 0x56, 0x78];
    /// let mut cursor = Cursor::new(data);
    ///
    /// let value: u32 = cursor.read_byteable().unwrap();
    /// #[cfg(target_endian = "little")]
    /// assert_eq!(value, 0x78563412);
    /// ```
    #[inline]
    fn read_byteable<T: Readable>(&mut self) -> std::io::Result<T> {
        T::read_from(self)
    }
}

// Blanket implementation: any type that implements Read automatically gets ReadByteable
impl<T: Read> ReadByteable for T {}

/// Extension trait for `Write` that adds methods for writing [`Writable`] types.
///
/// This trait is automatically implemented for all types that implement `std::io::Write`,
/// providing convenient methods for writing Rust types directly as binary data.
///
/// The `T` in `write_byteable(&value)` must implement [`Writable`], which covers:
/// - Primitive types and fixed-size structs (via [`IntoByteArray`])
/// - Collections ([`Vec`], [`VecDeque`], [`HashMap`], [`HashSet`], [`BTreeMap`], [`BTreeSet`])
///   serialized as a little-endian `u64` length prefix followed by each element
/// - [`Option<T>`], [`str`], [`String`]
///
/// # Examples
///
/// ## Writing to a file
///
/// ```no_run
/// # #[cfg(feature = "derive")] {
/// use byteable::{Byteable, WriteByteable};
/// use std::fs::File;
///
/// #[derive(Byteable, Clone, Copy)]
/// struct Header {
///     #[byteable(big_endian)]
///     magic: u32,
///     #[byteable(little_endian)]
///     version: u16,
///     #[byteable(little_endian)]
///     flags: u16,
/// }
///
/// # fn main() -> std::io::Result<()> {
/// let header = Header {
///     magic: 0x12345678,
///     version: 1,
///     flags: 0,
/// };
///
/// let mut file = File::create("output.bin")?;
/// file.write_byteable(&header)?;
/// # Ok(())
/// # }
/// # }
/// ```
///
/// ## Writing to a TCP stream
///
/// ```no_run
/// use byteable::WriteByteable;
/// use std::net::TcpStream;
///
/// # fn main() -> std::io::Result<()> {
/// let mut stream = TcpStream::connect("127.0.0.1:8080")?;
///
/// // Write a u32 length prefix
/// stream.write_byteable(&42u32)?;
/// # Ok(())
/// # }
/// ```
///
/// ## Writing multiple values
///
/// ```
/// use byteable::WriteByteable;
/// use std::io::Cursor;
///
/// let mut buffer = Cursor::new(Vec::new());
///
/// buffer.write_byteable(&1u32).unwrap();
/// buffer.write_byteable(&2u32).unwrap();
/// buffer.write_byteable(&3u32).unwrap();
///
/// #[cfg(target_endian = "little")]
/// assert_eq!(
///     buffer.into_inner(),
///     vec![1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]
/// );
/// ```
pub trait WriteByteable: Write {
    /// Writes a [`Writable`] type to this writer.
    ///
    /// Delegates to `T`'s [`Writable`] implementation. For fixed-size types this writes a
    /// fixed number of bytes; for collection types this writes a length-prefixed sequence.
    ///
    /// # Errors
    ///
    /// This method returns an error if any underlying I/O error occurs while writing.
    ///
    /// # Examples
    ///
    /// ```
    /// use byteable::{Byteable, WriteByteable};
    /// use std::io::Cursor;
    ///
    /// let mut buffer = Cursor::new(Vec::new());
    /// buffer.write_byteable(&0x12345678u32).unwrap();
    ///
    /// #[cfg(target_endian = "little")]
    /// assert_eq!(buffer.into_inner(), vec![0x78, 0x56, 0x34, 0x12]);
    /// ```
    #[inline]
    fn write_byteable<T: Writable>(&mut self, data: &T) -> std::io::Result<()> {
        data.write_to(self)
    }
}

// Blanket implementation: any type that implements Write automatically gets WriteByteable
impl<T: Write> WriteByteable for T {}

/// Error type for fallible byteable I/O operations.
///
/// This error type distinguishes between I/O errors that occur when reading/writing bytes,
/// and conversion errors that occur when converting between byte arrays and values.
///
/// # Type Parameter
///
/// - `E`: The error type from the conversion operation (from `TryFromByteArray::Error` or
///   `TryIntoByteArray::Error`)
///
/// # Examples
///
/// ```
/// use byteable::ByteableIoError;
/// use std::io;
///
/// fn handle_error<E: std::fmt::Display>(err: ByteableIoError<E>) {
///     match err {
///         ByteableIoError::Io(io_err) => {
///             eprintln!("I/O error: {}", io_err);
///         }
///         ByteableIoError::Conversion(conv_err) => {
///             eprintln!("Conversion error: {}", conv_err);
///         }
///     }
/// }
/// ```
#[derive(Debug)]
pub enum ByteableIoError<E> {
    /// An I/O error occurred while reading or writing bytes.
    Io(std::io::Error),
    /// A conversion error occurred while converting between bytes and values.
    Conversion(E),
}

impl<E: fmt::Display> fmt::Display for ByteableIoError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ByteableIoError::Io(err) => write!(f, "I/O error: {}", err),
            ByteableIoError::Conversion(err) => write!(f, "Conversion error: {}", err),
        }
    }
}

impl<E: Error + 'static> Error for ByteableIoError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ByteableIoError::Io(err) => Some(err),
            ByteableIoError::Conversion(err) => Some(err),
        }
    }
}

impl<E> From<std::io::Error> for ByteableIoError<E> {
    #[inline]
    fn from(err: std::io::Error) -> Self {
        ByteableIoError::Io(err)
    }
}

/// Low-level trait for writing a value to a [`std::io::Write`] sink with fallible conversion.
///
/// Implemented for all types that implement [`TryIntoByteArray`]. Unlike [`Writable`], the
/// conversion to bytes can fail — useful for validated types or enums where not every bit pattern
/// represents a valid value.
///
/// You typically don't need to implement or call this trait directly — use
/// [`TryWriteByteable::write_try_byteable`] instead.
pub trait TryWritable {
    type Error;

    fn try_write_to(
        &self,
        writer: &mut (impl Write + ?Sized),
    ) -> Result<(), ByteableIoError<Self::Error>>;
}

/// Low-level trait for reading a value from a [`std::io::Read`] source with fallible conversion.
///
/// Implemented for all types that implement [`TryFromByteArray`]. Unlike [`Readable`], the
/// conversion from bytes can fail — useful for types like `bool`, `char`, or enums where not every
/// bit pattern represents a valid value.
///
/// You typically don't need to implement or call this trait directly — use
/// [`TryReadByteable::read_try_byteable`] instead.
pub trait TryReadable: Sized {
    type Error;

    fn try_read_from(
        reader: &mut (impl Read + ?Sized),
    ) -> Result<Self, ByteableIoError<Self::Error>>;
}

impl<T: TryIntoByteArray> TryWritable for T {
    type Error = <T as TryIntoByteArray>::Error;

    fn try_write_to(
        &self,
        writer: &mut (impl Write + ?Sized),
    ) -> Result<(), ByteableIoError<Self::Error>> {
        let byte_array = self
            .try_into_byte_array()
            .map_err(ByteableIoError::Conversion)?;
        writer.write_all(byte_array.as_byte_slice())?;
        Ok(())
    }
}

impl<T: TryFromByteArray> TryReadable for T {
    type Error = <T as TryFromByteArray>::Error;

    fn try_read_from(
        reader: &mut (impl Read + ?Sized),
    ) -> Result<Self, ByteableIoError<Self::Error>> {
        let mut b = T::ByteArray::zeroed();
        reader.read_exact(b.as_byte_slice_mut())?;

        T::try_from_byte_array(b).map_err(ByteableIoError::Conversion)
    }
}

/// Extension trait for `Read` that adds methods for reading types with fallible conversion.
///
/// This trait is automatically implemented for all types that implement `std::io::Read`,
/// providing methods for reading types that implement [`TryFromByteArray`]. Unlike
/// [`ReadByteable`], this trait handles conversion errors explicitly.
///
/// # Error Handling
///
/// This trait returns [`ByteableIoError<E>`] which distinguishes between:
/// - I/O errors (failed to read bytes from the source)
/// - Conversion errors (bytes were read successfully but conversion failed)
///
/// # Examples
///
/// ## Reading with validation
///
/// ```
/// use byteable::{AssociatedByteArray, TryFromByteArray, TryReadByteable};
/// use std::io::Cursor;
///
/// // A type that only accepts even values
/// #[derive(Debug, PartialEq, Clone, Copy)]
/// struct EvenU32(u32);
///
/// #[derive(Debug)]
/// struct NotEvenError;
///
/// impl std::fmt::Display for NotEvenError {
///     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
///         write!(f, "value is not even")
///     }
/// }
///
/// impl std::error::Error for NotEvenError {}
///
/// impl AssociatedByteArray for EvenU32 {
///     type ByteArray = [u8; 4];
/// }
///
/// impl TryFromByteArray for EvenU32 {
///     type Error = NotEvenError;
///
///     fn try_from_byte_array(bytes: [u8; 4]) -> Result<Self, Self::Error> {
///         let value = u32::from_ne_bytes(bytes);
///         if value % 2 == 0 {
///             Ok(EvenU32(value))
///         } else {
///             Err(NotEvenError)
///         }
///     }
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let data = vec![2, 0, 0, 0]; // Even value
/// let mut cursor = Cursor::new(data);
/// let value: EvenU32 = cursor.read_try_byteable()?;
/// #[cfg(target_endian = "little")]
/// assert_eq!(value, EvenU32(2));
///
/// let odd_data = vec![3, 0, 0, 0]; // Odd value
/// let mut cursor = Cursor::new(odd_data);
/// let result: Result<EvenU32, _> = cursor.read_try_byteable();
/// assert!(result.is_err());
/// # Ok(())
/// # }
/// ```
pub trait TryReadByteable: Read {
    /// Reads a [`TryReadable`] type with fallible conversion from this reader.
    ///
    /// This method reads the number of bytes required by `T`'s [`TryFromByteArray`] implementation
    /// and attempts to convert them into a value of type `T`.
    ///
    /// # Errors
    ///
    /// This method returns [`ByteableIoError::Io`] if:
    /// - The reader reaches EOF before all required bytes have been read
    /// - Any underlying I/O error occurs
    ///
    /// This method returns [`ByteableIoError::Conversion`] if:
    /// - The bytes were read successfully but `try_from_byte_array` failed
    ///
    /// # Examples
    ///
    /// ```
    /// use byteable::TryReadByteable;
    /// use std::io::Cursor;
    ///
    /// let data = vec![42, 0, 0, 0];
    /// let mut cursor = Cursor::new(data);
    ///
    /// // u32 implements TryFromByteArray (never fails)
    /// let value: u32 = cursor.read_try_byteable().unwrap();
    /// #[cfg(target_endian = "little")]
    /// assert_eq!(value, 42);
    /// ```
    #[inline]
    fn read_try_byteable<T: TryReadable>(&mut self) -> Result<T, ByteableIoError<T::Error>> {
        T::try_read_from(self)
    }
}

// Blanket implementation: any type that implements Read automatically gets TryReadByteable
impl<T: Read> TryReadByteable for T {}

/// Extension trait for `Write` that adds methods for writing types with fallible conversion.
///
/// This trait is automatically implemented for all types that implement `std::io::Write`,
/// providing methods for writing types that implement [`TryIntoByteArray`]. Unlike
/// [`WriteByteable`], this trait handles conversion errors explicitly.
///
/// # Error Handling
///
/// This trait returns [`ByteableIoError<E>`] which distinguishes between:
/// - Conversion errors (failed to convert value to bytes)
/// - I/O errors (conversion succeeded but writing bytes failed)
///
/// # Examples
///
/// ## Writing with validation
///
/// ```
/// use byteable::{AssociatedByteArray, TryIntoByteArray, TryWriteByteable};
/// use std::io::Cursor;
///
/// // A type that only accepts even values
/// #[derive(Debug, PartialEq, Clone, Copy)]
/// struct EvenU32(u32);
///
/// #[derive(Debug, Clone, Copy)]
/// struct NotEvenError;
///
/// impl std::fmt::Display for NotEvenError {
///     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
///         write!(f, "value is not even")
///     }
/// }
///
/// impl std::error::Error for NotEvenError {}
///
/// impl AssociatedByteArray for EvenU32 {
///     type ByteArray = [u8; 4];
/// }
///
/// impl TryIntoByteArray for EvenU32 {
///     type Error = NotEvenError;
///
///     fn try_into_byte_array(self) -> Result<[u8; 4], Self::Error> {
///         if self.0 % 2 == 0 {
///             Ok(self.0.to_ne_bytes())
///         } else {
///             Err(NotEvenError)
///         }
///     }
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut buffer = Cursor::new(Vec::new());
/// buffer.write_try_byteable(&EvenU32(42))?;
///
/// #[cfg(target_endian = "little")]
/// assert_eq!(buffer.into_inner(), vec![42, 0, 0, 0]);
/// # Ok(())
/// # }
/// ```
pub trait TryWriteByteable: Write {
    /// Writes a [`TryWritable`] type with fallible conversion to this writer.
    ///
    /// This method attempts to convert the value to bytes via [`TryIntoByteArray`] and then
    /// writes all bytes to the writer.
    ///
    /// # Errors
    ///
    /// This method returns [`ByteableIoError::Conversion`] if:
    /// - The value could not be converted to bytes (`try_into_byte_array` failed)
    ///
    /// This method returns [`ByteableIoError::Io`] if:
    /// - Any underlying I/O error occurs while writing
    ///
    /// # Examples
    ///
    /// ```
    /// use byteable::TryWriteByteable;
    /// use std::io::Cursor;
    ///
    /// let mut buffer = Cursor::new(Vec::new());
    ///
    /// // u32 implements TryIntoByteArray (never fails)
    /// buffer.write_try_byteable(&42u32).unwrap();
    ///
    /// #[cfg(target_endian = "little")]
    /// assert_eq!(buffer.into_inner(), vec![42, 0, 0, 0]);
    /// ```
    #[inline]
    fn write_try_byteable<T: TryWritable>(
        &mut self,
        data: &T,
    ) -> Result<(), ByteableIoError<T::Error>> {
        data.try_write_to(self)
    }
}

// Blanket implementation: any type that implements Write automatically gets TryWriteByteable
impl<T: Write> TryWriteByteable for T {}

#[cfg(test)]
mod tests {
    use byteable_derive::UnsafeByteableTransmute;
    use thiserror::Error;

    use super::{ByteableIoError, ReadByteable, TryReadByteable, TryWriteByteable, WriteByteable};
    use crate::{
        AssociatedByteArray, BigEndian, LittleEndian, TryFromByteArray, TryIntoByteArray,
        impl_byteable_via,
    };
    use std::io::Cursor;

    #[derive(Clone, Copy, Debug, UnsafeByteableTransmute)]
    #[repr(C, packed)]
    struct TestPacketRaw {
        id: BigEndian<u16>,
        value: LittleEndian<u32>,
    }

    #[derive(Clone, Copy, PartialEq, Debug)]
    struct TestPacket {
        id: u16,
        value: u32,
    }

    impl From<TestPacket> for TestPacketRaw {
        fn from(value: TestPacket) -> Self {
            Self {
                id: value.id.into(),
                value: value.value.into(),
            }
        }
    }

    impl From<TestPacketRaw> for TestPacket {
        fn from(value: TestPacketRaw) -> Self {
            Self {
                id: value.id.get(),
                value: value.value.get(),
            }
        }
    }

    impl_byteable_via!(TestPacket => TestPacketRaw);

    #[test]
    fn test_write_one() {
        let packet = TestPacket {
            id: 123,
            value: 0x01020304,
        };

        let mut buffer = Cursor::new(vec![]);
        buffer.write_byteable(&packet).unwrap();
        assert_eq!(buffer.into_inner(), vec![0, 123, 4, 3, 2, 1]);
    }

    #[test]
    fn test_read_one() {
        let data = vec![0, 123, 4, 3, 2, 1];
        let mut reader = Cursor::new(data);
        let packet: TestPacket = reader.read_byteable().unwrap();

        let id = packet.id;
        let value = packet.value;
        assert_eq!(id, 123);
        assert_eq!(value, 0x01020304);
    }

    #[test]
    fn test_write_read_roundtrip() {
        let original = TestPacket {
            id: 42,
            value: 0xAABBCCDD,
        };

        let mut buffer = Cursor::new(vec![]);
        buffer.write_byteable(&original).unwrap();

        let mut reader = Cursor::new(buffer.into_inner());
        let read_packet: TestPacket = reader.read_byteable().unwrap();

        assert_eq!(read_packet, original);
    }

    #[test]
    fn test_write_multiple() {
        let mut buffer = Cursor::new(vec![]);

        buffer.write_byteable(&BigEndian::new(0x0102u16)).unwrap();
        buffer
            .write_byteable(&LittleEndian::new(0x0304u16))
            .unwrap();

        assert_eq!(buffer.into_inner(), vec![1, 2, 4, 3]);
    }

    #[test]
    fn test_write_many() {
        let mut buffer = Cursor::new(vec![]);

        buffer
            .write_byteable(&[
                TestPacket { id: 0, value: 1 },
                TestPacket { id: 1, value: 2 },
            ])
            .unwrap();

        assert_eq!(
            buffer.into_inner(),
            vec![0, 0, 1, 0, 0, 0, 0, 1, 2, 0, 0, 0]
        );
    }

    #[derive(Debug, Error)]
    #[error(transparent)]
    enum MyBiggerError {
        Io(#[from] std::io::Error),
        Conversion(#[from] ConversionError),
        Other(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
    }

    impl From<ByteableIoError<ConversionError>> for MyBiggerError {
        fn from(value: ByteableIoError<ConversionError>) -> Self {
            match value {
                ByteableIoError::Io(error) => error.into(),
                ByteableIoError::Conversion(error) => error.into(),
            }
        }
    }

    // Test types for fallible conversion
    #[derive(Debug, PartialEq, Clone, Copy)]
    struct EvenU32(u32);

    #[derive(Debug, PartialEq)]
    struct ConversionError;

    impl std::fmt::Display for ConversionError {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "value must be even")
        }
    }

    impl std::error::Error for ConversionError {}

    impl AssociatedByteArray for EvenU32 {
        type ByteArray = [u8; 4];
    }

    impl TryFromByteArray for EvenU32 {
        type Error = ConversionError;

        fn try_from_byte_array(bytes: [u8; 4]) -> Result<Self, Self::Error> {
            let value = u32::from_le_bytes(bytes);
            if value % 2 == 0 {
                Ok(EvenU32(value))
            } else {
                Err(ConversionError)
            }
        }
    }

    impl TryIntoByteArray for EvenU32 {
        type Error = ConversionError;

        fn try_into_byte_array(self) -> Result<[u8; 4], Self::Error> {
            if self.0 % 2 == 0 {
                Ok(self.0.to_le_bytes())
            } else {
                Err(ConversionError)
            }
        }
    }

    #[test]
    fn test_read_try_byteable_success() {
        let data = vec![42, 0, 0, 0]; // Even value
        let mut cursor = Cursor::new(data);

        let result: Result<EvenU32, _> = cursor.read_try_byteable();
        assert!(result.is_ok());

        assert_eq!(result.unwrap(), EvenU32(42));
    }

    #[test]
    fn test_read_try_byteable_conversion_error() {
        let data = vec![43, 0, 0, 0]; // Odd value
        let mut cursor = Cursor::new(data);

        let result: Result<EvenU32, ByteableIoError<ConversionError>> = cursor.read_try_byteable();
        assert!(matches!(result, Err(ByteableIoError::Conversion(_))));
    }

    #[test]
    fn test_read_try_byteable_error_conversion() {
        fn subfunc_success() -> Result<(), MyBiggerError> {
            let data = vec![42, 0, 0, 0]; // Odd value
            let mut cursor = Cursor::new(data);

            let _: EvenU32 = cursor.read_try_byteable()?;
            Ok(())
        }

        let r = subfunc_success();
        assert!(matches!(r, Ok(_)));

        fn subfunc_fail() -> Result<(), MyBiggerError> {
            let data = vec![43, 0, 0, 0]; // Odd value
            let mut cursor = Cursor::new(data);

            let _: EvenU32 = cursor.read_try_byteable()?;
            Ok(())
        }

        let r = subfunc_fail();
        assert!(matches!(r, Err(MyBiggerError::Conversion(_))));
    }

    #[test]
    fn test_read_try_byteable_io_error() {
        let data = vec![1, 2]; // Not enough bytes
        let mut cursor = Cursor::new(data);

        let result: Result<EvenU32, ByteableIoError<ConversionError>> = cursor.read_try_byteable();
        assert!(result.is_err());

        match result {
            Err(ByteableIoError::Io(_)) => {
                // Expected
            }
            _ => panic!("Expected I/O error"),
        }
    }

    #[test]
    fn test_write_try_byteable_success() {
        let mut buffer = Cursor::new(Vec::new());

        let result = buffer.write_try_byteable(&EvenU32(100));
        assert!(result.is_ok());

        #[cfg(target_endian = "little")]
        assert_eq!(buffer.into_inner(), vec![100, 0, 0, 0]);
    }

    #[test]
    fn test_write_try_byteable_conversion_error() {
        let mut buffer = Cursor::new(Vec::new());

        let result = buffer.write_try_byteable(&EvenU32(101)); // Odd value
        assert!(result.is_err());

        match result {
            Err(ByteableIoError::Conversion(_)) => {
                // Expected
            }
            _ => panic!("Expected conversion error"),
        }
    }

    #[test]
    fn test_try_byteable_roundtrip() {
        let original = EvenU32(1024);

        let mut buffer = Cursor::new(Vec::new());
        buffer.write_try_byteable(&original).unwrap();

        let mut reader = Cursor::new(buffer.into_inner());
        let read_value: EvenU32 = reader.read_try_byteable().unwrap();

        assert_eq!(read_value, original);
    }

    #[test]
    fn test_try_byteable_with_infallible() {
        // Test that regular types work with Try traits (should never fail)
        let mut buffer = Cursor::new(Vec::new());
        buffer.write_try_byteable(&42u32).unwrap();

        let mut reader = Cursor::new(buffer.into_inner());
        let value: u32 = reader.read_try_byteable().unwrap();
        assert_eq!(value, 42);
    }
}
