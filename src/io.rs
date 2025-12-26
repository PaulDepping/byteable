use crate::byte_array::ByteArray;
use crate::byteable::Byteable;
use std::io::{Read, Write};

pub trait ReadByteable: Read {
    fn read_byteable<T: Byteable>(&mut self) -> std::io::Result<T> {
        let mut byte_array = T::ByteArray::zeroed();
        self.read_exact(byte_array.as_byte_slice_mut())?;
        Ok(T::from_byte_array(byte_array))
    }
}

impl<T: Read> ReadByteable for T {}

pub trait WriteByteable: Write {
    fn write_byteable<T: Byteable>(&mut self, data: T) -> std::io::Result<()> {
        let byte_array = data.as_byte_array();
        self.write_all(byte_array.as_byte_slice())
    }
}

impl<T: Write> WriteByteable for T {}

#[cfg(test)]
mod tests {
    use byteable_derive::UnsafeByteable;

    use super::{ReadByteable, WriteByteable};
    use crate::{BigEndian, Byteable, LittleEndian, impl_byteable_via};
    use std::io::Cursor;

    #[derive(Clone, Copy, Debug, UnsafeByteable)]
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
        buffer.write_byteable(packet).unwrap();
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
        buffer.write_byteable(original).unwrap();

        let mut reader = Cursor::new(buffer.into_inner());
        let read_packet: TestPacket = reader.read_byteable().unwrap();

        assert_eq!(read_packet, original);
    }

    #[test]
    fn test_write_multiple() {
        let mut buffer = Cursor::new(vec![]);

        buffer.write_byteable(BigEndian::new(0x0102u16)).unwrap();
        buffer.write_byteable(LittleEndian::new(0x0304u16)).unwrap();

        assert_eq!(buffer.into_inner(), vec![1, 2, 4, 3]);
    }

    #[test]
    fn test_write_many() {
        let mut buffer = Cursor::new(vec![]);

        buffer
            .write_byteable([
                TestPacket { id: 0, value: 1 },
                TestPacket { id: 1, value: 2 },
            ])
            .unwrap();

        assert_eq!(
            buffer.into_inner(),
            vec![0, 0, 1, 0, 0, 0, 0, 1, 2, 0, 0, 0]
        );
    }
}
