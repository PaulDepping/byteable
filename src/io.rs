//! Synchronous I/O extensions for reading and writing byteable types.
//!
//! This module provides extension traits for `std::io::Read` and `std::io::Write` that
//! enable convenient reading and writing of types implementing the byte conversion traits
//! ([`IntoByteArray`] and [`FromByteArray`]).

use crate::byte_array::FixedBytes;
use crate::{IntoByteArray, LittleEndian, TryFromByteArray};
use core::error;
use core::ffi::CStr;
use core::hash::{BuildHasher, Hash};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, LinkedList, VecDeque};
use std::ffi::CString;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// Low-level trait for reading a fixed-size value from a [`std::io::Read`] source.
///
/// Implemented for all types that implement [`TryFromByteArray`] (primitives, fixed-size structs,
/// enums, `bool`, `char`). These types have a statically-known byte size and need no length prefix.
///
/// Use the [`ReadFixed`] extension trait to call `read_fixed` on any reader.
pub trait FixedReadable: Sized {
    fn read_fixed_from(reader: &mut (impl Read + ?Sized)) -> io::Result<Self>;
}

impl<T: TryFromByteArray> FixedReadable for T
where
    T::Error: Into<Box<dyn error::Error + Send + Sync + 'static>>,
{
    #[inline]
    fn read_fixed_from(reader: &mut (impl Read + ?Sized)) -> io::Result<Self> {
        let mut b = T::ByteArray::zeroed();
        reader.read_exact(b.as_byte_slice_mut())?;
        T::try_from_byte_array(b).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

/// Low-level trait for reading a value from a [`std::io::Read`] source.
///
/// This trait is implemented for:
/// - All types implementing [`FixedReadable`] (primitives, fixed-size structs, enums, `bool`, `char`)
/// - Collection types: [`Vec`], [`VecDeque`], [`HashMap`], [`HashSet`], [`BTreeMap`], [`BTreeSet`]
/// - [`Option<T>`] where `T: Readable`
/// - [`String`]
///
/// Collections are serialized as a little-endian `u64` length prefix followed by each element.
///
/// You typically don't need to implement or call this trait directly — use
/// [`ReadValue::read_value`] or [`ReadFixed::read_fixed`] instead.
pub trait Readable: Sized {
    fn read_from(reader: &mut (impl Read + ?Sized)) -> io::Result<Self>;
}

impl<T: FixedReadable> Readable for T {
    #[inline]
    fn read_from(reader: &mut (impl Read + ?Sized)) -> io::Result<Self> {
        T::read_fixed_from(reader)
    }
}

impl<T: Readable> Readable for Vec<T> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> io::Result<Self> {
        let len: LittleEndian<u64> = reader.read_fixed()?;
        let len = len.get() as usize;
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(reader.read_value()?);
        }
        Ok(result)
    }
}

impl<T: Readable> Readable for VecDeque<T> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> io::Result<Self> {
        let len: LittleEndian<u64> = reader.read_fixed()?;
        let len = len.get() as usize;
        let mut result = VecDeque::with_capacity(len);
        for _ in 0..len {
            result.push_back(reader.read_value()?);
        }
        Ok(result)
    }
}

impl<T: Readable> Readable for LinkedList<T> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> io::Result<Self> {
        let len: LittleEndian<u64> = reader.read_fixed()?;
        let len = len.get() as usize;
        let mut result = LinkedList::new();
        for _ in 0..len {
            result.push_back(reader.read_value()?);
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
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> io::Result<Self> {
        let len: LittleEndian<u64> = reader.read_fixed()?;
        let len = len.get() as usize;
        let mut map = HashMap::with_capacity_and_hasher(len, S::default());
        for _ in 0..len {
            let key = reader.read_value()?;
            let val = reader.read_value()?;
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
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> io::Result<Self> {
        let len: LittleEndian<u64> = reader.read_fixed()?;
        let len = len.get() as usize;
        let mut set = HashSet::with_capacity_and_hasher(len, S::default());
        for _ in 0..len {
            set.insert(reader.read_value()?);
        }
        Ok(set)
    }
}

impl<K: Readable + Ord, V: Readable> Readable for BTreeMap<K, V> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> io::Result<Self> {
        let len: LittleEndian<u64> = reader.read_fixed()?;
        let len = len.get() as usize;
        let mut map = BTreeMap::new();
        for _ in 0..len {
            let key = reader.read_value()?;
            let val = reader.read_value()?;
            map.insert(key, val);
        }
        Ok(map)
    }
}

impl<T: Readable + Ord> Readable for BTreeSet<T> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> io::Result<Self> {
        let len: LittleEndian<u64> = reader.read_fixed()?;
        let len = len.get() as usize;
        let mut set = BTreeSet::new();
        for _ in 0..len {
            set.insert(reader.read_value()?);
        }
        Ok(set)
    }
}

impl<T: Readable> Readable for Option<T> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> io::Result<Self> {
        let tag: u8 = reader.read_fixed()?;
        match tag {
            0 => Ok(None),
            1 => Ok(Some(reader.read_value()?)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid Option tag byte",
            )),
        }
    }
}

impl<V: Readable, E: Readable> Readable for Result<V, E> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> io::Result<Self> {
        let discriminator: u8 = reader.read_fixed()?;
        match discriminator {
            0 => Ok(Ok(reader.read_value()?)),
            1 => Ok(Err(reader.read_value()?)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid Result tag byte",
            )),
        }
    }
}

impl Readable for String {
    fn read_from(reader: &mut (impl Read + ?Sized)) -> io::Result<Self> {
        let len: LittleEndian<u64> = reader.read_fixed()?;
        let len = len.get() as usize;
        let mut bytes = vec![0u8; len];
        reader.read_exact(&mut bytes)?;
        String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

impl Readable for PathBuf {
    fn read_from(reader: &mut (impl Read + ?Sized)) -> io::Result<Self> {
        let s = String::read_from(reader)?;
        Ok(PathBuf::from(s))
    }
}

impl Readable for CString {
    fn read_from(reader: &mut (impl Read + ?Sized)) -> io::Result<Self> {
        let v = Vec::read_from(reader)?;
        Ok(CString::new(v).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?)
    }
}

/// Low-level trait for writing a fixed-size value to a [`std::io::Write`] sink.
///
/// Implemented for all types that implement [`IntoByteArray`] (primitives, fixed-size structs).
/// These types have a statically-known byte size and are written without a length prefix.
///
/// Use the [`WriteFixed`] extension trait to call `write_fixed` on any writer.
pub trait FixedWritable {
    fn write_fixed_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()>;
}

impl<T: IntoByteArray + Clone> FixedWritable for T {
    #[inline]
    fn write_fixed_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        let byte_array = self.clone().into_byte_array();
        writer.write_all(byte_array.as_byte_slice())
    }
}

/// Low-level trait for writing a value to a [`std::io::Write`] sink.
///
/// This trait is implemented for:
/// - All types implementing [`FixedWritable`] (primitives, fixed-size structs)
/// - Collection types: [`Vec`], [`VecDeque`], [`HashMap`], [`HashSet`], [`BTreeMap`], [`BTreeSet`]
/// - [`Option<T>`] where `T: Writable`
/// - [`str`] and [`String`]
///
/// Collections are serialized as a little-endian `u64` length prefix followed by each element.
///
/// You typically don't need to implement or call this trait directly — use
/// [`WriteValue::write_value`] or [`WriteFixed::write_fixed`] instead.
pub trait Writable {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()>;
}

impl<T: FixedWritable> Writable for T {
    #[inline]
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        self.write_fixed_to(writer)
    }
}

impl<T: Writable> Writable for [T] {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        let len = LittleEndian::new(self.len() as u64);
        writer.write_fixed(&len)?;
        for el in self {
            writer.write_value(el)?;
        }
        Ok(())
    }
}

impl<T: Writable> Writable for Vec<T> {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        self.as_slice().write_to(writer)
    }
}

impl<T: Writable> Writable for VecDeque<T> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        let len = LittleEndian::new(self.len() as u64);
        writer.write_fixed(&len)?;
        for el in self {
            writer.write_value(el)?;
        }
        Ok(())
    }
}

impl<T: Writable> Writable for LinkedList<T> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        let len = LittleEndian::new(self.len() as u64);
        writer.write_fixed(&len)?;
        for el in self {
            writer.write_value(el)?;
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
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        let len = LittleEndian::new(self.len() as u64);
        writer.write_fixed(&len)?;
        for (k, v) in self {
            writer.write_value(k)?;
            writer.write_value(v)?;
        }
        Ok(())
    }
}

impl<T, S> Writable for HashSet<T, S>
where
    T: Writable,
    S: BuildHasher,
{
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        let len = LittleEndian::new(self.len() as u64);
        writer.write_fixed(&len)?;
        for el in self {
            writer.write_value(el)?;
        }
        Ok(())
    }
}

impl<K: Writable, V: Writable> Writable for BTreeMap<K, V> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        let len = LittleEndian::new(self.len() as u64);
        writer.write_fixed(&len)?;
        for (k, v) in self {
            writer.write_value(k)?;
            writer.write_value(v)?;
        }
        Ok(())
    }
}

impl<T: Writable> Writable for BTreeSet<T> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        let len = LittleEndian::new(self.len() as u64);
        writer.write_fixed(&len)?;
        for el in self {
            writer.write_value(el)?;
        }
        Ok(())
    }
}

impl<T: Writable> Writable for Option<T> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        match self {
            None => writer.write_fixed(&0u8),
            Some(val) => {
                writer.write_fixed(&1u8)?;
                writer.write_value(val)
            }
        }
    }
}

impl<V: Writable, E: Writable> Writable for Result<V, E> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
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

impl Writable for str {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        let len = LittleEndian::new(self.len() as u64);
        writer.write_fixed(&len)?;
        writer.write_all(self.as_bytes())
    }
}

impl Writable for String {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        self.as_str().write_to(writer)
    }
}

impl Writable for Path {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        match self.to_str() {
            Some(s) => s.write_to(writer),
            None => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "path contains invalid UTF-8 characters",
            )),
        }
    }
}

impl Writable for CStr {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        self.to_bytes().write_to(writer)
    }
}

impl Writable for CString {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        self.as_bytes().write_to(writer)
    }
}

impl Writable for PathBuf {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        self.as_path().write_to(writer)
    }
}

/// Extension trait for `Read` that adds a `read_fixed` method for types implementing [`FixedReadable`].
///
/// Importing this trait instead of (or alongside) [`ReadValue`] signals at the call site that
/// the type being read has a statically-known, fixed byte size with no length prefix.
pub trait ReadFixed: Read {
    /// Reads a [`FixedReadable`] type from this reader.
    ///
    /// Unlike [`ReadValue::read_value`], this method is only callable for fixed-size types
    /// — it will not compile for collection types like [`Vec`] or [`String`].
    #[inline]
    fn read_fixed<T: FixedReadable>(&mut self) -> io::Result<T> {
        T::read_fixed_from(self)
    }
}

impl<T: Read + ?Sized> ReadFixed for T {}

/// Extension trait for `Read` that adds methods for reading [`Readable`] types.
///
/// This trait is automatically implemented for all types that implement `std::io::Read`,
/// providing convenient methods for reading binary data directly into Rust types.
///
/// The `T` in `read_value::<T>()` must implement [`Readable`], which covers:
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
/// use byteable::{Byteable, ReadValue};
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
/// let header: Header = file.read_value()?;
/// println!("Header: {:?}", header);
/// # Ok(())
/// # }
/// ```
///
/// ## Reading from a TCP stream
///
/// ```no_run
/// use byteable::ReadValue;
/// use std::net::TcpStream;
///
/// # fn main() -> std::io::Result<()> {
/// let mut stream = TcpStream::connect("127.0.0.1:8080")?;
///
/// // Read a u32 length prefix
/// let length: u32 = stream.read_value()?;
/// println!("Message length: {}", length);
/// # Ok(())
/// # }
/// ```
///
/// ## Reading multiple values
///
/// ```no_run
/// use byteable::ReadValue;
/// use std::io::Cursor;
///
/// # fn main() -> std::io::Result<()> {
/// let data = vec![1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0];
/// let mut cursor = Cursor::new(data);
///
/// let a: u32 = cursor.read_value()?;
/// let b: u32 = cursor.read_value()?;
/// let c: u32 = cursor.read_value()?;
///
/// #[cfg(target_endian = "little")]
/// assert_eq!((a, b, c), (1, 2, 3));
/// # Ok(())
/// # }
/// ```
pub trait ReadValue: Read {
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
    /// use byteable::{Byteable, ReadValue};
    /// use std::io::Cursor;
    ///
    /// let data = vec![0x12, 0x34, 0x56, 0x78];
    /// let mut cursor = Cursor::new(data);
    ///
    /// let value: u32 = cursor.read_value().unwrap();
    /// #[cfg(target_endian = "little")]
    /// assert_eq!(value, 0x78563412);
    /// ```
    #[inline]
    fn read_value<T: Readable>(&mut self) -> io::Result<T> {
        T::read_from(self)
    }
}

// Blanket implementation: any type that implements Read automatically gets ReadValue
impl<T: Read> ReadValue for T {}

/// Extension trait for `Write` that adds a `write_fixed` method for types implementing [`FixedWritable`].
///
/// Importing this trait instead of (or alongside) [`WriteValue`] signals at the call site that
/// the type being written has a statically-known, fixed byte size with no length prefix.
pub trait WriteFixed: Write {
    /// Writes a [`FixedWritable`] type to this writer.
    ///
    /// Unlike [`WriteValue::write_value`], this method is only callable for fixed-size types
    /// — it will not compile for collection types like [`Vec`] or [`String`].
    #[inline]
    fn write_fixed(&mut self, val: &impl FixedWritable) -> io::Result<()> {
        val.write_fixed_to(self)
    }
}

impl<T: Write> WriteFixed for T {}

/// Extension trait for `Write` that adds methods for writing [`Writable`] types.
///
/// This trait is automatically implemented for all types that implement `std::io::Write`,
/// providing convenient methods for writing Rust types directly as binary data.
///
/// The `T` in `write_value(&value)` must implement [`Writable`], which covers:
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
/// use byteable::{Byteable, WriteValue};
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
/// file.write_value(&header)?;
/// # Ok(())
/// # }
/// # }
/// ```
///
/// ## Writing to a TCP stream
///
/// ```no_run
/// use byteable::WriteValue;
/// use std::net::TcpStream;
///
/// # fn main() -> std::io::Result<()> {
/// let mut stream = TcpStream::connect("127.0.0.1:8080")?;
///
/// // Write a u32 length prefix
/// stream.write_value(&42u32)?;
/// # Ok(())
/// # }
/// ```
///
/// ## Writing multiple values
///
/// ```
/// use byteable::WriteValue;
/// use std::io::Cursor;
///
/// let mut buffer = Cursor::new(Vec::new());
///
/// buffer.write_value(&1u32).unwrap();
/// buffer.write_value(&2u32).unwrap();
/// buffer.write_value(&3u32).unwrap();
///
/// #[cfg(target_endian = "little")]
/// assert_eq!(
///     buffer.into_inner(),
///     vec![1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]
/// );
/// ```
pub trait WriteValue: Write {
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
    /// use byteable::{Byteable, WriteValue};
    /// use std::io::Cursor;
    ///
    /// let mut buffer = Cursor::new(Vec::new());
    /// buffer.write_value(&0x12345678u32).unwrap();
    ///
    /// #[cfg(target_endian = "little")]
    /// assert_eq!(buffer.into_inner(), vec![0x78, 0x56, 0x34, 0x12]);
    /// ```
    #[inline]
    fn write_value<T: Writable + ?Sized>(&mut self, data: &T) -> io::Result<()> {
        data.write_to(self)
    }
}

// Blanket implementation: any type that implements Write automatically gets WriteValue
impl<T: Write> WriteValue for T {}

#[cfg(test)]
mod tests {
    use byteable_derive::UnsafeByteableTransmute;

    use super::{ReadFixed, ReadValue, WriteFixed};
    use crate::{BigEndian, ByteRepr, LittleEndian, TryFromByteArray, impl_byteable_via};
    use core::fmt;
    use std::io::{self, Cursor};

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
        buffer.write_fixed(&packet).unwrap();
        assert_eq!(buffer.into_inner(), vec![0, 123, 4, 3, 2, 1]);
    }

    #[test]
    fn test_read_one() {
        let data = vec![0, 123, 4, 3, 2, 1];
        let mut reader = Cursor::new(data);
        let packet: TestPacket = reader.read_fixed().unwrap();

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
        buffer.write_fixed(&original).unwrap();

        let mut reader = Cursor::new(buffer.into_inner());
        let read_packet: TestPacket = reader.read_fixed().unwrap();

        assert_eq!(read_packet, original);
    }

    #[test]
    fn test_write_multiple() {
        let mut buffer = Cursor::new(vec![]);

        buffer.write_fixed(&BigEndian::new(0x0102u16)).unwrap();
        buffer.write_fixed(&LittleEndian::new(0x0304u16)).unwrap();

        assert_eq!(buffer.into_inner(), vec![1, 2, 4, 3]);
    }

    #[test]
    fn test_write_many() {
        let mut buffer = Cursor::new(vec![]);

        buffer
            .write_fixed(&[
                TestPacket { id: 0, value: 1 },
                TestPacket { id: 1, value: 2 },
            ])
            .unwrap();

        assert_eq!(
            buffer.into_inner(),
            vec![0, 0, 1, 0, 0, 0, 0, 1, 2, 0, 0, 0]
        );
    }

    // Test types for fallible conversion
    #[derive(Debug, PartialEq, Clone, Copy)]
    struct EvenU32(u32);

    #[derive(Debug, PartialEq)]
    struct ConversionError;

    impl fmt::Display for ConversionError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "value must be even")
        }
    }

    impl core::error::Error for ConversionError {}

    impl ByteRepr for EvenU32 {
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

    #[test]
    fn test_read_value_success() {
        let data = vec![42, 0, 0, 0]; // Even value
        let mut cursor = Cursor::new(data);

        let result: io::Result<EvenU32> = cursor.read_value();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), EvenU32(42));
    }

    #[test]
    fn test_read_value_conversion_error() {
        let data = vec![43, 0, 0, 0]; // Odd value
        let mut cursor = Cursor::new(data);

        let result: io::Result<EvenU32> = cursor.read_value();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_read_value_question_mark() {
        // Demonstrates that ? now works directly with io::Result
        fn read_even(data: Vec<u8>) -> io::Result<EvenU32> {
            let mut cursor = Cursor::new(data);
            let value: EvenU32 = cursor.read_value()?;
            Ok(value)
        }

        assert!(read_even(vec![42, 0, 0, 0]).is_ok());
        assert!(read_even(vec![43, 0, 0, 0]).is_err());
    }

    #[test]
    fn test_read_value_io_error() {
        let data = vec![1, 2]; // Not enough bytes
        let mut cursor = Cursor::new(data);

        let result: io::Result<EvenU32> = cursor.read_value();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::UnexpectedEof
        );
    }
}
