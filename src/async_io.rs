use crate::byte_array::ByteArray;
use crate::byteable::Byteable;
use std::future::Future;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub trait AsyncReadByteable: tokio::io::AsyncReadExt {
    fn read_byteable<T: Byteable>(&mut self) -> impl Future<Output = std::io::Result<T>>
    where
        Self: Unpin + Send,
    {
        async move {
            let mut byte_array = T::ByteArray::zeroed();
            self.read_exact(byte_array.as_byte_slice_mut()).await?;
            Ok(T::from_byte_array(byte_array))
        }
    }
}

impl<T: AsyncReadExt> AsyncReadByteable for T {}

pub trait AsyncWriteByteable: tokio::io::AsyncWriteExt {
    fn write_byteable<T: Byteable>(&mut self, data: T) -> impl Future<Output = std::io::Result<()>>
    where
        Self: Unpin,
    {
        async move {
            let byte_array = data.as_byte_array();
            self.write_all(byte_array.as_byte_slice()).await
        }
    }
}

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
