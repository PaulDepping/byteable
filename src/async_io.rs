//! Asynchronous I/O traits for reading and writing byteable types.
//!
//! This module provides extension traits for tokio's `AsyncReadExt` and `AsyncWriteExt`
//! that allow asynchronously reading and writing types implementing the `Byteable` trait.
//!
//! This module is only available when the `tokio` feature is enabled.

use crate::byte_array::ByteableByteArray;
use crate::byteable::Byteable;
use std::future::Future;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Extends `tokio::io::AsyncReadExt` with an asynchronous method to read a `Byteable` type.
///
/// This trait is only available when the `tokio` feature is enabled.
pub trait AsyncReadByteable: tokio::io::AsyncReadExt {
    /// Asynchronously reads one `Byteable` element from the reader.
    ///
    /// This method will create a zero-filled byte array, asynchronously read
    /// enough bytes from the underlying reader to fill it, and then convert
    /// the byte array into the specified `Byteable` type.
    fn read_one<T: Byteable>(&mut self) -> impl Future<Output = std::io::Result<T>>
    where
        Self: Unpin + Send,
    {
        async move {
            let mut e = T::ByteArray::create_zeroed();
            self.read_exact(e.as_byteslice_mut()).await?;
            Ok(T::from_bytearray(e))
        }
    }
}

/// Implements `AsyncReadByteable` for all types that implement `tokio::io::AsyncReadExt`.
impl<T: AsyncReadExt> AsyncReadByteable for T {}

/// Extends `tokio::io::AsyncWriteExt` with an asynchronous method to write a `Byteable` type.
///
/// This trait is only available when the `tokio` feature is enabled.
pub trait AsyncWriteByteable: tokio::io::AsyncWriteExt {
    /// Asynchronously writes one `Byteable` element to the writer.
    ///
    /// This method will convert the `Byteable` data into its byte array
    /// representation and then asynchronously write all those bytes to
    /// the underlying writer.
    fn write_one<T: Byteable>(&mut self, data: T) -> impl Future<Output = std::io::Result<()>>
    where
        Self: Unpin,
    {
        async move {
            let e = data.as_bytearray();
            self.write_all(e.as_byteslice()).await
        }
    }
}

/// Implements `AsyncWriteByteable` for all types that implement `tokio::io::AsyncWriteExt`.
impl<T: AsyncWriteExt> AsyncWriteByteable for T {}

#[cfg(test)]
mod tests {
    use super::{AsyncReadByteable, AsyncWriteByteable};
    use crate::{BigEndian, Byteable, LittleEndian, impl_byteable};
    use std::io::Cursor;

    #[derive(Clone, Copy, PartialEq, Debug)]
    #[repr(C, packed)]
    struct AsyncTestPacket {
        id: u16,
        value: LittleEndian<u32>,
    }
    impl_byteable!(AsyncTestPacket);

    #[tokio::test]
    async fn test_async_write_one() {
        let packet = AsyncTestPacket {
            id: 123,
            value: LittleEndian::new(0x01020304),
        };

        let mut buffer = Cursor::new(vec![]);
        buffer.write_one(packet).await.unwrap();
        assert_eq!(buffer.into_inner(), vec![123, 0, 4, 3, 2, 1]);
    }

    #[tokio::test]
    async fn test_async_read_one() {
        let data = vec![123, 0, 4, 3, 2, 1];
        let mut reader = Cursor::new(data);
        let packet: AsyncTestPacket = reader.read_one().await.unwrap();

        // Copy values to avoid packed field reference issues
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
        buffer.write_one(original).await.unwrap();

        let mut reader = Cursor::new(buffer.into_inner());
        let read_packet: AsyncTestPacket = reader.read_one().await.unwrap();

        assert_eq!(read_packet, original);
    }

    #[tokio::test]
    async fn test_async_write_multiple() {
        let mut buffer = Cursor::new(vec![]);

        buffer.write_one(BigEndian::new(0x0102u16)).await.unwrap();
        buffer
            .write_one(LittleEndian::new(0x0304u16))
            .await
            .unwrap();

        assert_eq!(buffer.into_inner(), vec![1, 2, 4, 3]);
    }
}
