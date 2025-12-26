//! Asynchronous I/O extensions for reading and writing `Byteable` types.
//!
//! This module provides extension traits for `tokio::io::AsyncRead` and `tokio::io::AsyncWrite`
//! that enable convenient async reading and writing of types implementing the `Byteable` trait.
//!
//! This module is only available when the `tokio` feature is enabled.

use crate::byte_array::ByteArray;
use crate::byteable::Byteable;
use std::future::Future;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Extension trait for `AsyncRead` that adds methods for reading `Byteable` types asynchronously.
///
/// This trait is automatically implemented for all types that implement `tokio::io::AsyncReadExt`,
/// providing convenient methods for reading binary data directly into Rust types in async contexts.
///
/// # Examples
///
/// ## Reading from an async file
///
/// ```no_run
/// # #[cfg(all(feature = "tokio", feature = "derive"))]
/// use byteable::{Byteable, AsyncReadByteable};
/// # #[cfg(all(feature = "tokio", feature = "derive"))]
/// use tokio::fs::File;
///
/// # #[cfg(all(feature = "tokio", feature = "derive"))]
/// #[derive(byteable::UnsafeByteable, Debug)]
/// #[repr(C, packed)]
/// struct Header {
///     magic: u32,
///     version: u16,
///     flags: u16,
/// }
///
/// # #[cfg(all(feature = "tokio", feature = "derive"))]
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let mut file = File::open("data.bin").await?;
/// let header: Header = file.read_byteable().await?;
/// println!("Header: {:?}", header);
/// # Ok(())
/// # }
/// # #[cfg(not(all(feature = "tokio", feature = "derive")))]
/// # fn main() {}
/// ```
///
/// ## Reading from an async TCP stream
///
/// ```no_run
/// # #[cfg(feature = "tokio")]
/// use byteable::AsyncReadByteable;
/// # #[cfg(feature = "tokio")]
/// use tokio::net::TcpStream;
///
/// # #[cfg(feature = "tokio")]
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
///
/// // Read a u32 length prefix
/// let length: u32 = stream.read_byteable().await?;
/// println!("Message length: {}", length);
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "tokio"))]
/// # fn main() {}
/// ```
///
/// ## Reading multiple values sequentially
///
/// ```no_run
/// # #[cfg(feature = "tokio")]
/// use byteable::AsyncReadByteable;
/// # #[cfg(feature = "tokio")]
/// use std::io::Cursor;
///
/// # #[cfg(feature = "tokio")]
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let data = vec![1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0];
/// let mut cursor = Cursor::new(data);
///
/// let a: u32 = cursor.read_byteable().await?;
/// let b: u32 = cursor.read_byteable().await?;
/// let c: u32 = cursor.read_byteable().await?;
///
/// #[cfg(target_endian = "little")]
/// assert_eq!((a, b, c), (1, 2, 3));
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "tokio"))]
/// # fn main() {}
/// ```
pub trait AsyncReadByteable: tokio::io::AsyncReadExt {
    /// Asynchronously reads a `Byteable` type from this reader.
    ///
    /// This method reads exactly `T::BYTE_SIZE` bytes from the reader and converts
    /// them into a value of type `T`.
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - The reader reaches EOF before reading `T::BYTE_SIZE` bytes
    /// - Any underlying I/O error occurs
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "tokio")]
    /// use byteable::{Byteable, AsyncReadByteable};
    /// # #[cfg(feature = "tokio")]
    /// use std::io::Cursor;
    ///
    /// # #[cfg(feature = "tokio")]
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let data = vec![0x12, 0x34, 0x56, 0x78];
    /// let mut cursor = Cursor::new(data);
    ///
    /// let value: u32 = cursor.read_byteable().await?;
    /// #[cfg(target_endian = "little")]
    /// assert_eq!(value, 0x78563412);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "tokio"))]
    /// # fn main() {}
    /// ```
    fn read_byteable<T: Byteable>(&mut self) -> impl Future<Output = std::io::Result<T>>
    where
        Self: Unpin + Send,
    {
        async move {
            // Create a zeroed byte array to hold the data
            let mut byte_array = T::ByteArray::zeroed();

            // Asynchronously read exactly BYTE_SIZE bytes from the reader
            self.read_exact(byte_array.as_byte_slice_mut()).await?;

            // Convert the bytes into the target type
            Ok(T::from_byte_array(byte_array))
        }
    }
}

// Blanket implementation: any type that implements AsyncReadExt automatically gets AsyncReadByteable
impl<T: AsyncReadExt> AsyncReadByteable for T {}

/// Extension trait for `AsyncWrite` that adds methods for writing `Byteable` types asynchronously.
///
/// This trait is automatically implemented for all types that implement `tokio::io::AsyncWriteExt`,
/// providing convenient methods for writing Rust types directly as binary data in async contexts.
///
/// # Examples
///
/// ## Writing to an async file
///
/// ```no_run
/// # #[cfg(all(feature = "tokio", feature = "derive"))]
/// use byteable::{Byteable, AsyncWriteByteable};
/// # #[cfg(all(feature = "tokio", feature = "derive"))]
/// use tokio::fs::File;
///
/// # #[cfg(all(feature = "tokio", feature = "derive"))]
/// #[derive(byteable::UnsafeByteable)]
/// #[repr(C, packed)]
/// struct Header {
///     magic: u32,
///     version: u16,
///     flags: u16,
/// }
///
/// # #[cfg(all(feature = "tokio", feature = "derive"))]
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let header = Header {
///     magic: 0x12345678,
///     version: 1,
///     flags: 0,
/// };
///
/// let mut file = File::create("output.bin").await?;
/// file.write_byteable(header).await?;
/// # Ok(())
/// # }
/// # #[cfg(not(all(feature = "tokio", feature = "derive")))]
/// # fn main() {}
/// ```
///
/// ## Writing to an async TCP stream
///
/// ```no_run
/// # #[cfg(feature = "tokio")]
/// use byteable::AsyncWriteByteable;
/// # #[cfg(feature = "tokio")]
/// use tokio::net::TcpStream;
///
/// # #[cfg(feature = "tokio")]
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
///
/// // Write a u32 length prefix
/// stream.write_byteable(42u32).await?;
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "tokio"))]
/// # fn main() {}
/// ```
///
/// ## Writing multiple values
///
/// ```no_run
/// # #[cfg(feature = "tokio")]
/// use byteable::AsyncWriteByteable;
/// # #[cfg(feature = "tokio")]
/// use std::io::Cursor;
///
/// # #[cfg(feature = "tokio")]
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let mut buffer = Cursor::new(Vec::new());
///
/// buffer.write_byteable(1u32).await?;
/// buffer.write_byteable(2u32).await?;
/// buffer.write_byteable(3u32).await?;
///
/// #[cfg(target_endian = "little")]
/// assert_eq!(
///     buffer.into_inner(),
///     vec![1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]
/// );
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "tokio"))]
/// # fn main() {}
/// ```
pub trait AsyncWriteByteable: tokio::io::AsyncWriteExt {
    /// Asynchronously writes a `Byteable` type to this writer.
    ///
    /// This method converts the value into its byte array representation and writes
    /// all bytes to the writer.
    ///
    /// # Errors
    ///
    /// This method returns an error if any underlying I/O error occurs while writing.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "tokio")]
    /// use byteable::{Byteable, AsyncWriteByteable};
    /// # #[cfg(feature = "tokio")]
    /// use std::io::Cursor;
    ///
    /// # #[cfg(feature = "tokio")]
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let mut buffer = Cursor::new(Vec::new());
    /// buffer.write_byteable(0x12345678u32).await?;
    ///
    /// #[cfg(target_endian = "little")]
    /// assert_eq!(buffer.into_inner(), vec![0x78, 0x56, 0x34, 0x12]);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "tokio"))]
    /// # fn main() {}
    /// ```
    fn write_byteable<T: Byteable>(&mut self, data: T) -> impl Future<Output = std::io::Result<()>>
    where
        Self: Unpin,
    {
        async move {
            // Convert the data into its byte array representation
            let byte_array = data.as_byte_array();

            // Asynchronously write all bytes to the writer
            self.write_all(byte_array.as_byte_slice()).await
        }
    }
}

// Blanket implementation: any type that implements AsyncWriteExt automatically gets AsyncWriteByteable
impl<T: AsyncWriteExt> AsyncWriteByteable for T {}

#[cfg(test)]
mod tests {
    use byteable_derive::UnsafeByteable;

    use super::{AsyncReadByteable, AsyncWriteByteable};
    use crate::{BigEndian, LittleEndian};
    use std::io::Cursor;

    #[derive(Clone, Copy, PartialEq, Debug, UnsafeByteable)]
    #[repr(C, packed)]
    struct AsyncTestPacket {
        id: u16,
        value: LittleEndian<u32>,
    }

    #[tokio::test]
    async fn test_async_write_one() {
        let packet = AsyncTestPacket {
            id: 123,
            value: LittleEndian::new(0x01020304),
        };

        let mut buffer = Cursor::new(vec![]);
        buffer.write_byteable(packet).await.unwrap();
        assert_eq!(buffer.into_inner(), vec![123, 0, 4, 3, 2, 1]);
    }

    #[tokio::test]
    async fn test_async_read_one() {
        let data = vec![123, 0, 4, 3, 2, 1];
        let mut reader = Cursor::new(data);
        let packet: AsyncTestPacket = reader.read_byteable().await.unwrap();

        let id = packet.id;
        let value = packet.value.get();
        assert_eq!(id, 123);
        assert_eq!(value, 0x01020304);
    }

    #[tokio::test]
    async fn test_async_write_read_roundtrip() {
        let original = AsyncTestPacket {
            id: 42,
            value: LittleEndian::new(0xAABBCCDD),
        };

        let mut buffer = Cursor::new(vec![]);
        buffer.write_byteable(original).await.unwrap();

        let mut reader = Cursor::new(buffer.into_inner());
        let read_packet: AsyncTestPacket = reader.read_byteable().await.unwrap();

        assert_eq!(read_packet, original);
    }

    #[tokio::test]
    async fn test_async_write_multiple() {
        let mut buffer = Cursor::new(vec![]);

        buffer
            .write_byteable(BigEndian::new(0x0102u16))
            .await
            .unwrap();
        buffer
            .write_byteable(LittleEndian::new(0x0304u16))
            .await
            .unwrap();

        assert_eq!(buffer.into_inner(), vec![1, 2, 4, 3]);
    }
}
