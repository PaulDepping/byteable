//! Synchronous I/O extensions for reading and writing `Byteable` types.
//!
//! This module provides extension traits for `std::io::Read` and `std::io::Write` that
//! enable convenient reading and writing of types implementing the `Byteable` trait.

use crate::byte_array::ByteArray;
use crate::byteable::Byteable;
use std::io::{Read, Write};

/// Extension trait for `Read` that adds methods for reading `Byteable` types.
///
/// This trait is automatically implemented for all types that implement `std::io::Read`,
/// providing convenient methods for reading binary data directly into Rust types.
///
/// # Examples
///
/// ## Reading from a file
///
/// ```no_run
/// use byteable::{Byteable, ReadByteable};
/// use std::fs::File;
///
/// # #[cfg(feature = "derive")]
/// #[derive(byteable::UnsafeByteable, Debug)]
/// #[repr(C, packed)]
/// struct Header {
///     magic: u32,
///     version: u16,
///     flags: u16,
/// }
///
/// # fn main() -> std::io::Result<()> {
/// # #[cfg(feature = "derive")] {
/// let mut file = File::open("data.bin")?;
/// let header: Header = file.read_byteable()?;
/// println!("Header: {:?}", header);
/// # }
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
    /// Reads a `Byteable` type from this reader.
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
    fn read_byteable<T: Byteable>(&mut self) -> std::io::Result<T> {
        // Create a zeroed byte array to hold the data
        let mut byte_array = T::ByteArray::zeroed();

        // Read exactly BYTE_SIZE bytes from the reader into the array
        self.read_exact(byte_array.as_byte_slice_mut())?;

        // Convert the bytes into the target type
        Ok(T::from_byte_array(byte_array))
    }
}

// Blanket implementation: any type that implements Read automatically gets ReadByteable
impl<T: Read> ReadByteable for T {}

/// Extension trait for `Write` that adds methods for writing `Byteable` types.
///
/// This trait is automatically implemented for all types that implement `std::io::Write`,
/// providing convenient methods for writing Rust types directly as binary data.
///
/// # Examples
///
/// ## Writing to a file
///
/// ```no_run
/// use byteable::{Byteable, WriteByteable};
/// use std::fs::File;
///
/// # #[cfg(feature = "derive")]
/// #[derive(byteable::UnsafeByteable)]
/// #[repr(C, packed)]
/// struct Header {
///     magic: u32,
///     version: u16,
///     flags: u16,
/// }
///
/// # fn main() -> std::io::Result<()> {
/// # #[cfg(feature = "derive")] {
/// let header = Header {
///     magic: 0x12345678,
///     version: 1,
///     flags: 0,
/// };
///
/// let mut file = File::create("output.bin")?;
/// file.write_byteable(header)?;
/// # }
/// # Ok(())
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
/// stream.write_byteable(42u32)?;
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
/// buffer.write_byteable(1u32).unwrap();
/// buffer.write_byteable(2u32).unwrap();
/// buffer.write_byteable(3u32).unwrap();
///
/// #[cfg(target_endian = "little")]
/// assert_eq!(
///     buffer.into_inner(),
///     vec![1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]
/// );
/// ```
pub trait WriteByteable: Write {
    /// Writes a `Byteable` type to this writer.
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
    /// ```
    /// use byteable::{Byteable, WriteByteable};
    /// use std::io::Cursor;
    ///
    /// let mut buffer = Cursor::new(Vec::new());
    /// buffer.write_byteable(0x12345678u32).unwrap();
    ///
    /// #[cfg(target_endian = "little")]
    /// assert_eq!(buffer.into_inner(), vec![0x78, 0x56, 0x34, 0x12]);
    /// ```
    fn write_byteable<T: Byteable>(&mut self, data: T) -> std::io::Result<()> {
        // Convert the data into its byte array representation
        let byte_array = data.as_byte_array();

        // Write all bytes to the writer
        self.write_all(byte_array.as_byte_slice())
    }
}

// Blanket implementation: any type that implements Write automatically gets WriteByteable
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
