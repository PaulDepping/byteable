//! Asynchronous I/O extensions for reading and writing byteable types.
//!
//! This module provides extension traits for `tokio::io::AsyncRead` and `tokio::io::AsyncWrite`
//! that enable convenient async reading and writing of types implementing the byte conversion traits
//! ([`IntoByteArray`] and [`FromByteArray`]).
//!
//! This module is only available when the `tokio` feature is enabled.

use crate::byte_array::FixedBytes;
use crate::{IntoByteArray, LittleEndian, TryFromByteArray};
use core::future::Future;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::hash::{BuildHasher, Hash};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Low-level trait for asynchronously reading a fixed-size value from a `tokio::io::AsyncRead` source.
///
/// This is the async counterpart of [`crate::io::FixedReadable`]. It is implemented for all types
/// implementing [`TryFromByteArray`] (primitives, fixed-size structs, enums, `bool`, `char`).
/// These types have a statically-known byte size and need no length prefix.
///
/// Use the [`AsyncReadFixed`] extension trait to call `read_fixed` on any async reader.
pub trait AsyncFixedReadable: Sized {
    fn read_fixed_from(
        reader: &mut (impl tokio::io::AsyncReadExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<Self>>;
}

impl<T: TryFromByteArray> AsyncFixedReadable for T
where
    T::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    fn read_fixed_from(
        reader: &mut (impl tokio::io::AsyncReadExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<Self>> {
        async move {
            let mut b = T::ByteArray::zeroed();
            reader.read_exact(b.as_byte_slice_mut()).await?;
            T::try_from_byte_array(b)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }
    }
}

/// Low-level trait for asynchronously reading a value from a `tokio::io::AsyncRead` source.
///
/// This trait is the async counterpart of [`crate::io::Readable`]. It is implemented for:
/// - All types implementing [`AsyncFixedReadable`] (primitives, fixed-size structs, enums, `bool`, `char`)
/// - Collection types: [`Vec`], [`VecDeque`], [`HashMap`], [`HashSet`], [`BTreeMap`], [`BTreeSet`]
/// - [`Option<T>`] where `T: AsyncReadable`
/// - [`String`]
///
/// Collections are serialized as a little-endian `u64` length prefix followed by each element.
///
/// You typically don't need to implement or call this trait directly — use
/// [`AsyncReadValue::read_value`] or [`AsyncReadFixed::read_fixed`] instead.
pub trait AsyncReadable: Sized {
    fn read_from(
        reader: &mut (impl tokio::io::AsyncReadExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<Self>>;
}

impl<T: AsyncFixedReadable> AsyncReadable for T {
    fn read_from(
        reader: &mut (impl tokio::io::AsyncReadExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<Self>> {
        T::read_fixed_from(reader)
    }
}

/// Low-level trait for asynchronously writing a fixed-size value to a `tokio::io::AsyncWrite` sink.
///
/// This is the async counterpart of [`crate::io::FixedWritable`]. It is implemented for all types
/// implementing [`IntoByteArray`] (primitives, fixed-size structs). These types have a
/// statically-known byte size and are written without a length prefix.
///
/// Use the [`AsyncWriteFixed`] extension trait to call `write_fixed` on any async writer.
pub trait AsyncFixedWritable {
    fn write_fixed_to(
        &self,
        writer: &mut (impl tokio::io::AsyncWriteExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<()>>;
}

impl<T: IntoByteArray> AsyncFixedWritable for T {
    fn write_fixed_to(
        &self,
        writer: &mut (impl tokio::io::AsyncWriteExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<()>> {
        async move {
            let b = self.into_byte_array();
            writer.write_all(b.as_byte_slice()).await
        }
    }
}

/// Low-level trait for asynchronously writing a value to a `tokio::io::AsyncWrite` sink.
///
/// This trait is the async counterpart of [`crate::io::Writable`]. It is implemented for:
/// - All types implementing [`AsyncFixedWritable`] (primitives, fixed-size structs)
/// - Collection types: [`Vec`], [`VecDeque`], [`HashMap`], [`HashSet`], [`BTreeMap`], [`BTreeSet`]
/// - [`Option<T>`] where `T: AsyncWritable`
/// - [`str`] and [`String`]
///
/// Collections are serialized as a little-endian `u64` length prefix followed by each element.
///
/// You typically don't need to implement or call this trait directly — use
/// [`AsyncWriteValue::write_value`] or [`AsyncWriteFixed::write_fixed`] instead.
pub trait AsyncWritable {
    fn write_to(
        &self,
        writer: &mut (impl tokio::io::AsyncWriteExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<()>>;
}

impl<T: AsyncFixedWritable> AsyncWritable for T {
    fn write_to(
        &self,
        writer: &mut (impl tokio::io::AsyncWriteExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<()>> {
        self.write_fixed_to(writer)
    }
}

impl<T: AsyncReadable> AsyncReadable for Vec<T> {
    fn read_from(
        mut reader: &mut (impl AsyncReadExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<Self>> {
        async move {
            let len: LittleEndian<u64> = reader.read_value().await?;
            let len = len.get() as usize;
            let mut result = Vec::with_capacity(len);
            for _ in 0..len {
                result.push(reader.read_value().await?);
            }
            Ok(result)
        }
    }
}

impl<T: AsyncWritable> AsyncWritable for Vec<T> {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<()>> {
        async move {
            writer
                .write_value(&LittleEndian::new(self.len() as u64))
                .await?;
            for el in self {
                writer.write_value(el).await?;
            }
            Ok(())
        }
    }
}

impl<T: AsyncReadable> AsyncReadable for VecDeque<T> {
    fn read_from(
        mut reader: &mut (impl AsyncReadExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<Self>> {
        async move {
            let len: LittleEndian<u64> = reader.read_value().await?;
            let len = len.get() as usize;
            let mut result = VecDeque::with_capacity(len);
            for _ in 0..len {
                result.push_back(reader.read_value().await?);
            }
            Ok(result)
        }
    }
}

impl<T: AsyncWritable> AsyncWritable for VecDeque<T> {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<()>> {
        async move {
            writer
                .write_value(&LittleEndian::new(self.len() as u64))
                .await?;
            for el in self {
                writer.write_value(el).await?;
            }
            Ok(())
        }
    }
}

impl<K, V, S> AsyncReadable for HashMap<K, V, S>
where
    K: AsyncReadable + Eq + Hash,
    V: AsyncReadable,
    S: BuildHasher + Default + Send,
{
    fn read_from(
        mut reader: &mut (impl AsyncReadExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<Self>> {
        async move {
            let len: LittleEndian<u64> = reader.read_value().await?;
            let len = len.get() as usize;
            let mut map = HashMap::with_capacity_and_hasher(len, S::default());
            for _ in 0..len {
                let key = reader.read_value().await?;
                let val = reader.read_value().await?;
                map.insert(key, val);
            }
            Ok(map)
        }
    }
}

impl<K, V, S> AsyncWritable for HashMap<K, V, S>
where
    K: AsyncWritable,
    V: AsyncWritable,
    S: BuildHasher + Send + Sync,
{
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<()>> {
        async move {
            writer
                .write_value(&LittleEndian::new(self.len() as u64))
                .await?;
            for (k, v) in self {
                writer.write_value(k).await?;
                writer.write_value(v).await?;
            }
            Ok(())
        }
    }
}

impl<T, S> AsyncReadable for HashSet<T, S>
where
    T: AsyncReadable + Eq + Hash,
    S: BuildHasher + Default + Send,
{
    fn read_from(
        mut reader: &mut (impl AsyncReadExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<Self>> {
        async move {
            let len: LittleEndian<u64> = reader.read_value().await?;
            let len = len.get() as usize;
            let mut set = HashSet::with_capacity_and_hasher(len, S::default());
            for _ in 0..len {
                set.insert(reader.read_value().await?);
            }
            Ok(set)
        }
    }
}

impl<T, S> AsyncWritable for HashSet<T, S>
where
    T: AsyncWritable,
    S: BuildHasher + Send + Sync,
{
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<()>> {
        async move {
            writer
                .write_value(&LittleEndian::new(self.len() as u64))
                .await?;
            for el in self {
                writer.write_value(el).await?;
            }
            Ok(())
        }
    }
}

impl<K: AsyncReadable + Ord, V: AsyncReadable> AsyncReadable for BTreeMap<K, V> {
    fn read_from(
        mut reader: &mut (impl AsyncReadExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<Self>> {
        async move {
            let len: LittleEndian<u64> = reader.read_value().await?;
            let len = len.get() as usize;
            let mut map = BTreeMap::new();
            for _ in 0..len {
                let key = reader.read_value().await?;
                let val = reader.read_value().await?;
                map.insert(key, val);
            }
            Ok(map)
        }
    }
}

impl<K: AsyncWritable, V: AsyncWritable> AsyncWritable for BTreeMap<K, V> {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<()>> {
        async move {
            writer
                .write_value(&LittleEndian::new(self.len() as u64))
                .await?;
            for (k, v) in self {
                writer.write_value(k).await?;
                writer.write_value(v).await?;
            }
            Ok(())
        }
    }
}

impl<T: AsyncReadable + Ord> AsyncReadable for BTreeSet<T> {
    fn read_from(
        mut reader: &mut (impl AsyncReadExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<Self>> {
        async move {
            let len: LittleEndian<u64> = reader.read_value().await?;
            let len = len.get() as usize;
            let mut set = BTreeSet::new();
            for _ in 0..len {
                set.insert(reader.read_value().await?);
            }
            Ok(set)
        }
    }
}

impl<T: AsyncWritable> AsyncWritable for BTreeSet<T> {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<()>> {
        async move {
            writer
                .write_value(&LittleEndian::new(self.len() as u64))
                .await?;
            for el in self {
                writer.write_value(el).await?;
            }
            Ok(())
        }
    }
}

impl<T: AsyncReadable> AsyncReadable for Option<T> {
    fn read_from(
        mut reader: &mut (impl AsyncReadExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<Self>> {
        async move {
            let tag: u8 = reader.read_value().await?;
            match tag {
                0 => Ok(None),
                1 => Ok(Some(reader.read_value().await?)),
                _ => Err(tokio::io::Error::new(
                    tokio::io::ErrorKind::InvalidData,
                    "invalid Option tag byte",
                )),
            }
        }
    }
}

impl<T: AsyncWritable> AsyncWritable for Option<T> {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<()>> {
        async move {
            match self {
                None => writer.write_value(&0u8).await,
                Some(val) => {
                    writer.write_value(&1u8).await?;
                    writer.write_value(val).await
                }
            }
        }
    }
}

impl AsyncReadable for String {
    fn read_from(
        mut reader: &mut (impl AsyncReadExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<Self>> {
        async move {
            let len: LittleEndian<u64> = reader.read_value().await?;
            let len = len.get() as usize;
            let mut bytes = vec![0u8; len];
            reader.read_exact(&mut bytes).await?;
            String::from_utf8(bytes)
                .map_err(|e| tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, e))
        }
    }
}

impl AsyncWritable for str {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<()>> {
        async move {
            writer
                .write_value(&LittleEndian::new(self.len() as u64))
                .await?;
            writer.write_all(self.as_bytes()).await
        }
    }
}

impl AsyncWritable for String {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + Unpin + ?Sized),
    ) -> impl Future<Output = tokio::io::Result<()>> {
        self.as_str().write_to(writer)
    }
}

/// Extension trait for `AsyncRead` that adds methods for reading [`AsyncReadable`] types asynchronously.
///
/// This trait is automatically implemented for all types that implement `tokio::io::AsyncReadExt`,
/// providing convenient methods for reading binary data directly into Rust types in async contexts.
///
/// The `T` in `read_value::<T>()` must implement [`AsyncReadable`], which covers:
/// - Primitive types and fixed-size structs (via [`FromByteArray`])
/// - Collections ([`Vec`], [`VecDeque`], [`HashMap`], [`HashSet`], [`BTreeMap`], [`BTreeSet`])
///   serialized as a little-endian `u64` length prefix followed by each element
/// - [`Option<T>`], [`String`]
///
/// # Examples
///
/// ## Reading from an async file
///
/// ```no_run
/// # #![cfg(all(feature = "tokio", feature = "derive"))]
/// use byteable::{Byteable, AsyncReadValue};
/// use tokio::fs::File;
///
/// #[derive(Byteable, Clone, Copy, Debug)]
/// struct Header {
///     #[byteable(little_endian)]
///     magic: u32,
///     #[byteable(little_endian)]
///     version: u16,
///     #[byteable(little_endian)]
///     flags: u16,
/// }
///
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let mut file = File::open("data.bin").await?;
/// let header: Header = file.read_value().await?;
/// println!("Header: {:?}", header);
/// # Ok(())
/// # }
/// ```
///
/// ## Reading from an async TCP stream
///
/// ```no_run
/// # #![cfg(feature = "tokio")]
/// use byteable::{AsyncReadValue, LittleEndian};
/// use tokio::net::TcpStream;
///
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
///
/// // Read a u32 length prefix
/// let length: LittleEndian<u32> = stream.read_value().await?;
/// println!("Message length: {}", length.get());
/// # Ok(())
/// # }
/// ```
///
/// ## Reading multiple values sequentially
///
/// ```no_run
/// # #![cfg(feature = "tokio")]
/// use byteable::{AsyncReadValue, LittleEndian};
/// use std::io::Cursor;
///
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let data = vec![1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0];
/// let mut cursor = Cursor::new(data);
///
/// let a: LittleEndian<u32> = cursor.read_value().await?;
/// let b: LittleEndian<u32> = cursor.read_value().await?;
/// let c: LittleEndian<u32> = cursor.read_value().await?;
///
/// assert_eq!((a.get(), b.get(), c.get()), (1, 2, 3));
/// # Ok(())
/// # }
/// ```
pub trait AsyncReadValue: tokio::io::AsyncReadExt + Unpin {
    /// Asynchronously reads an [`AsyncReadable`] type from this reader.
    ///
    /// Delegates to `T`'s [`AsyncReadable`] implementation. For fixed-size types this reads a
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
    /// ```no_run
    /// # #![cfg(feature = "tokio")]
    /// use byteable::{Byteable, AsyncReadValue};
    /// use std::io::Cursor;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> std::io::Result<()> {
    /// let data = vec![0x12, 0x34, 0x56, 0x78];
    /// let mut cursor = Cursor::new(data);
    ///
    /// let value: u32 = cursor.read_value().await?;
    /// #[cfg(target_endian = "little")]
    /// assert_eq!(value, 0x78563412);
    /// # Ok(())
    /// # }
    /// ```
    fn read_value<T: AsyncReadable>(&mut self) -> impl Future<Output = std::io::Result<T>> {
        T::read_from(self)
    }
}

// Blanket implementation: any type that implements AsyncReadExt automatically gets AsyncReadValue
impl<T: AsyncReadExt + Unpin> AsyncReadValue for T {}

/// Extension trait for `AsyncWrite` that adds methods for writing [`AsyncWritable`] types asynchronously.
///
/// This trait is automatically implemented for all types that implement `tokio::io::AsyncWriteExt`,
/// providing convenient methods for writing Rust types directly as binary data in async contexts.
///
/// The `T` in `write_value(&value)` must implement [`AsyncWritable`], which covers:
/// - Primitive types and fixed-size structs (via [`IntoByteArray`])
/// - Collections ([`Vec`], [`VecDeque`], [`HashMap`], [`HashSet`], [`BTreeMap`], [`BTreeSet`])
///   serialized as a little-endian `u64` length prefix followed by each element
/// - [`Option<T>`], [`str`], [`String`]
///
/// # Examples
///
/// ## Writing to an async file
///
/// ```no_run
/// # #![cfg(all(feature = "tokio", feature = "derive"))]
/// use byteable::{Byteable, AsyncWriteValue};
/// use tokio::fs::File;
///
/// #[derive(Byteable, Clone, Copy)]
/// struct Header {
///     #[byteable(little_endian)]
///     magic: u32,
///     #[byteable(little_endian)]
///     version: u16,
///     #[byteable(little_endian)]
///     flags: u16,
/// }
///
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let header = Header {
///     magic: 0x12345678,
///     version: 1,
///     flags: 0,
/// };
///
/// let mut file = File::create("output.bin").await?;
/// file.write_value(&header).await?;
/// # Ok(())
/// # }
/// ```
///
/// ## Writing to an async TCP stream
///
/// ```no_run
/// # #![cfg(feature = "tokio")]
/// use byteable::{AsyncWriteValue, LittleEndian};
/// use tokio::net::TcpStream;
///
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
///
/// // Write a u32 length prefix
/// stream.write_value(&LittleEndian::new(42u32)).await?;
/// # Ok(())
/// # }
/// ```
///
/// ## Writing multiple values
///
/// ```no_run
/// # #![cfg(feature = "tokio")]
/// use byteable::{AsyncWriteValue, LittleEndian};
/// use std::io::Cursor;
///
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let mut buffer = Cursor::new(Vec::new());
///
/// buffer.write_value(&LittleEndian::new(1u32)).await?;
/// buffer.write_value(&LittleEndian::new(2u32)).await?;
/// buffer.write_value(&LittleEndian::new(3u32)).await?;
///
/// assert_eq!(
///     buffer.into_inner(),
///     vec![1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]
/// );
/// # Ok(())
/// # }
/// ```
pub trait AsyncWriteValue: tokio::io::AsyncWriteExt + Unpin {
    /// Asynchronously writes an [`AsyncWritable`] type to this writer.
    ///
    /// Delegates to `T`'s [`AsyncWritable`] implementation. For fixed-size types this writes a
    /// fixed number of bytes; for collection types this writes a length-prefixed sequence.
    ///
    /// # Errors
    ///
    /// This method returns an error if any underlying I/O error occurs while writing.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![cfg(feature = "tokio")]
    /// use byteable::{Byteable, AsyncWriteValue, LittleEndian};
    /// use std::io::Cursor;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let mut buffer = Cursor::new(Vec::new());
    /// buffer.write_value(&LittleEndian::new(0x12345678u32)).await?;
    ///
    /// assert_eq!(buffer.into_inner(), vec![0x78, 0x56, 0x34, 0x12]);
    /// # Ok(())
    /// # }
    /// ```
    fn write_value<T: AsyncWritable>(
        &mut self,
        data: &T,
    ) -> impl Future<Output = std::io::Result<()>> {
        data.write_to(self)
    }
}

// Blanket implementation: any type that implements AsyncWriteExt automatically gets AsyncWriteValue
impl<T: AsyncWriteExt + Unpin + ?Sized> AsyncWriteValue for T {}

/// Extension trait for `AsyncRead` that adds a `read_fixed` method for types implementing
/// [`AsyncFixedReadable`].
///
/// Importing this trait instead of (or alongside) [`AsyncReadValue`] signals at the call site
/// that the type being read has a statically-known, fixed byte size with no length prefix.
pub trait AsyncReadFixed: tokio::io::AsyncReadExt + Unpin {
    /// Asynchronously reads an [`AsyncFixedReadable`] type from this reader.
    ///
    /// Unlike [`AsyncReadValue::read_value`], this method is only callable for fixed-size
    /// types — it will not compile for collection types like [`Vec`] or [`String`].
    fn read_fixed<T: AsyncFixedReadable>(&mut self) -> impl Future<Output = tokio::io::Result<T>> {
        T::read_fixed_from(self)
    }
}

impl<T: AsyncReadExt + Unpin> AsyncReadFixed for T {}

/// Extension trait for `AsyncWrite` that adds a `write_fixed` method for types implementing
/// [`AsyncFixedWritable`].
///
/// Importing this trait instead of (or alongside) [`AsyncWriteValue`] signals at the call site
/// that the type being written has a statically-known, fixed byte size with no length prefix.
pub trait AsyncWriteFixed: tokio::io::AsyncWriteExt + Unpin {
    /// Asynchronously writes an [`AsyncFixedWritable`] type to this writer.
    ///
    /// Unlike [`AsyncWriteValue::write_value`], this method is only callable for fixed-size
    /// types — it will not compile for collection types like [`Vec`] or [`String`].
    fn write_fixed(
        &mut self,
        val: &impl AsyncFixedWritable,
    ) -> impl Future<Output = tokio::io::Result<()>> {
        val.write_fixed_to(self)
    }
}

impl<T: AsyncWriteExt + Unpin + ?Sized> AsyncWriteFixed for T {}

#[cfg(test)]
mod tests {
    use super::{AsyncReadValue, AsyncWriteValue};
    use crate::{ByteRepr, BigEndian, Byteable, LittleEndian, TryFromByteArray};
    use std::io::Cursor;

    #[derive(Clone, Copy, PartialEq, Debug, Byteable)]
    struct AsyncTestPacket {
        #[byteable(little_endian)]
        id: u16,
        #[byteable(little_endian)]
        value: u32,
    }

    #[tokio::test]
    async fn test_async_write_one() {
        let packet = AsyncTestPacket {
            id: 123,
            value: 0x01020304,
        };

        let mut buffer = Cursor::new(vec![]);
        buffer.write_value(&packet).await.unwrap();
        assert_eq!(buffer.into_inner(), vec![123, 0, 4, 3, 2, 1]);
    }

    #[tokio::test]
    async fn test_async_read_one() {
        let data = vec![123, 0, 4, 3, 2, 1];
        let mut reader = Cursor::new(data);
        let packet: AsyncTestPacket = reader.read_value().await.unwrap();

        let id = packet.id;
        let value = packet.value;
        assert_eq!(id, 123);
        assert_eq!(value, 0x01020304);
    }

    #[tokio::test]
    async fn test_async_write_read_roundtrip() {
        let original = AsyncTestPacket {
            id: 42,
            value: 0xAABBCCDD,
        };

        let mut buffer = Cursor::new(vec![]);
        buffer.write_value(&original).await.unwrap();

        let mut reader = Cursor::new(buffer.into_inner());
        let read_packet: AsyncTestPacket = reader.read_value().await.unwrap();

        assert_eq!(read_packet, original);
    }

    #[tokio::test]
    async fn test_async_write_multiple() {
        let mut buffer = Cursor::new(vec![]);

        buffer
            .write_value(&BigEndian::new(0x0102u16))
            .await
            .unwrap();
        buffer
            .write_value(&LittleEndian::new(0x0304u16))
            .await
            .unwrap();

        assert_eq!(buffer.into_inner(), vec![1, 2, 4, 3]);
    }

    // Test types for fallible async conversion
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

    impl ByteRepr for EvenU32 {
        type ByteArray = [u8; 4];
    }

    impl TryFromByteArray for EvenU32 {
        type Error = ConversionError;

        fn try_from_byte_array(bytes: [u8; 4]) -> Result<Self, Self::Error> {
            let value = u32::from_ne_bytes(bytes);
            if value % 2 == 0 {
                Ok(EvenU32(value))
            } else {
                Err(ConversionError)
            }
        }
    }

    #[tokio::test]
    async fn test_async_read_value_success() {
        let data = vec![42, 0, 0, 0]; // Even value
        let mut cursor = Cursor::new(data);

        let result: Result<EvenU32, _> = cursor.read_value().await;
        assert!(result.is_ok());

        #[cfg(target_endian = "little")]
        assert_eq!(result.unwrap(), EvenU32(42));
    }

    #[tokio::test]
    async fn test_async_read_value_conversion_error() {
        let data = vec![43, 0, 0, 0]; // Odd value
        let mut cursor = Cursor::new(data);

        let result: std::io::Result<EvenU32> = cursor.read_value().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn test_async_read_value_io_error() {
        let data = vec![1, 2]; // Not enough bytes
        let mut cursor = Cursor::new(data);

        let result: std::io::Result<EvenU32> = cursor.read_value().await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::UnexpectedEof
        );
    }
}
