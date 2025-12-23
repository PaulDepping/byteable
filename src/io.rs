//! Synchronous I/O traits for reading and writing byteable types.
//!
//! This module provides extension traits for `std::io::Read` and `std::io::Write`
//! that allow reading and writing types implementing the `Byteable` trait.

use crate::byte_array::ByteableByteArray;
use crate::byteable::Byteable;
use std::io::{Read, Write};

/// Extends `std::io::Read` with a method to read a `Byteable` type.
pub trait ReadByteable: Read {
    /// Reads one `Byteable` element from the reader.
    ///
    /// This method will create a zero-filled byte array, read enough bytes
    /// from the underlying reader to fill it, and then convert the byte
    /// array into the specified `Byteable` type.
    fn read_one<T: Byteable>(&mut self) -> std::io::Result<T> {
        let mut e = T::ByteArray::create_zeroed();
        self.read_exact(e.as_byteslice_mut())?;
        Ok(T::from_bytearray(e))
    }
}

/// Implements `ReadByteable` for all types that implement `std::io::Read`.
impl<T: Read> ReadByteable for T {}

/// Extends `std::io::Write` with a method to write a `Byteable` type.
pub trait WriteByteable: Write {
    /// Writes one `Byteable` element to the writer.
    ///
    /// This method will convert the `Byteable` data into its byte array
    /// representation and then write all those bytes to the underlying writer.
    fn write_one<T: Byteable>(&mut self, data: T) -> std::io::Result<()> {
        let e = data.as_bytearray();
        self.write_all(e.as_byteslice())
    }
}

/// Implements `WriteByteable` for all types that implement `std::io::Write`.
impl<T: Write> WriteByteable for T {}

#[cfg(test)]
mod tests {
    use byteable_derive::UnsafeByteable;

    use super::{ReadByteable, WriteByteable};
    use crate::{BigEndian, Byteable, LittleEndian};
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

    impl Byteable for TestPacket {
        type ByteArray = <TestPacketRaw as Byteable>::ByteArray;

        fn as_bytearray(self) -> Self::ByteArray {
            TestPacketRaw {
                id: self.id.into(),
                value: self.value.into(),
            }
            .as_bytearray()
        }

        fn from_bytearray(ba: Self::ByteArray) -> Self {
            let raw = TestPacketRaw::from_bytearray(ba);
            Self {
                id: raw.id.get(),
                value: raw.value.get(),
            }
        }
    }

    #[test]
    fn test_write_one() {
        let packet = TestPacket {
            id: 123,
            value: 0x01020304,
        };

        let mut buffer = Cursor::new(vec![]);
        buffer.write_one(packet).unwrap();
        assert_eq!(buffer.into_inner(), vec![0, 123, 4, 3, 2, 1]);
    }

    #[test]
    fn test_read_one() {
        let data = vec![0, 123, 4, 3, 2, 1];
        let mut reader = Cursor::new(data);
        let packet: TestPacket = reader.read_one().unwrap();

        // Copy values to avoid packed field reference issues
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
        buffer.write_one(original).unwrap();

        let mut reader = Cursor::new(buffer.into_inner());
        let read_packet: TestPacket = reader.read_one().unwrap();

        assert_eq!(read_packet, original);
    }

    #[test]
    fn test_write_multiple() {
        let mut buffer = Cursor::new(vec![]);

        buffer.write_one(BigEndian::new(0x0102u16)).unwrap();
        buffer.write_one(LittleEndian::new(0x0304u16)).unwrap();

        assert_eq!(buffer.into_inner(), vec![1, 2, 4, 3]);
    }

    #[test]
    fn test_write_many() {
        let mut buffer = Cursor::new(vec![]);

        buffer
            .write_one([
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
