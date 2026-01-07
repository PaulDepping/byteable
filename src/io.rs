//! Synchronous I/O extensions for reading and writing byteable types.
//!
//! This module provides extension traits for `std::io::Read` and `std::io::Write` that
//! enable convenient reading and writing of types implementing the byte conversion traits
//! ([`IntoByteArray`] and [`FromByteArray`]).

use crate::byte_array::ByteArray;
use crate::{FromByteArray, IntoByteArray, TryFromByteArray, TryIntoByteArray};
use std::error::Error;
use std::fmt;
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
/// #[derive(byteable::Byteable, Debug)]
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
    fn read_byteable<T: FromByteArray>(&mut self) -> std::io::Result<T> {
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
/// #[derive(byteable::Byteable)]
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
    fn write_byteable<T: IntoByteArray>(&mut self, data: T) -> std::io::Result<()> {
        // Convert the data into its byte array representation
        let byte_array = data.into_byte_array();

        // Write all bytes to the writer
        self.write_all(byte_array.as_byte_slice())
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
/// use byteable::TryByteableError;
/// use std::io;
///
/// fn handle_error<E: std::fmt::Display>(err: TryByteableError<E>) {
///     match err {
///         TryByteableError::Io(io_err) => {
///             eprintln!("I/O error: {}", io_err);
///         }
///         TryByteableError::Conversion(conv_err) => {
///             eprintln!("Conversion error: {}", conv_err);
///         }
///     }
/// }
/// ```
#[derive(Debug)]
pub enum TryByteableError<E> {
    /// An I/O error occurred while reading or writing bytes.
    Io(std::io::Error),
    /// A conversion error occurred while converting between bytes and values.
    Conversion(E),
}

impl<E: fmt::Display> fmt::Display for TryByteableError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TryByteableError::Io(err) => write!(f, "I/O error: {}", err),
            TryByteableError::Conversion(err) => write!(f, "Conversion error: {}", err),
        }
    }
}

impl<E: Error + 'static> Error for TryByteableError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            TryByteableError::Io(err) => Some(err),
            TryByteableError::Conversion(err) => Some(err),
        }
    }
}

impl<E> From<std::io::Error> for TryByteableError<E> {
    fn from(err: std::io::Error) -> Self {
        TryByteableError::Io(err)
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
/// This trait returns [`TryByteableError<E>`] which distinguishes between:
/// - I/O errors (failed to read bytes from the source)
/// - Conversion errors (bytes were read successfully but conversion failed)
///
/// # Examples
///
/// ## Reading with validation
///
/// ```
/// use byteable::{AssociatedByteArray, TryFromByteArray, ReadTryByteable};
/// use std::io::Cursor;
///
/// // A type that only accepts even values
/// #[derive(Debug, PartialEq)]
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
pub trait ReadTryByteable: Read {
    /// Reads a type with fallible conversion from this reader.
    ///
    /// This method reads exactly `T::BYTE_SIZE` bytes from the reader and attempts
    /// to convert them into a value of type `T`.
    ///
    /// # Errors
    ///
    /// This method returns [`TryByteableError::Io`] if:
    /// - The reader reaches EOF before reading `T::BYTE_SIZE` bytes
    /// - Any underlying I/O error occurs
    ///
    /// This method returns [`TryByteableError::Conversion`] if:
    /// - The bytes were read successfully but `try_from_byte_array` failed
    ///
    /// # Examples
    ///
    /// ```
    /// use byteable::ReadTryByteable;
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
    fn read_try_byteable<T: TryFromByteArray>(&mut self) -> Result<T, TryByteableError<T::Error>> {
        // Create a zeroed byte array to hold the data
        let mut byte_array = T::ByteArray::zeroed();

        // Read exactly BYTE_SIZE bytes from the reader into the array
        self.read_exact(byte_array.as_byte_slice_mut())?;

        // Attempt to convert the bytes into the target type
        T::try_from_byte_array(byte_array).map_err(TryByteableError::Conversion)
    }
}

// Blanket implementation: any type that implements Read automatically gets ReadTryByteable
impl<T: Read> ReadTryByteable for T {}

/// Extension trait for `Write` that adds methods for writing types with fallible conversion.
///
/// This trait is automatically implemented for all types that implement `std::io::Write`,
/// providing methods for writing types that implement [`TryIntoByteArray`]. Unlike
/// [`WriteByteable`], this trait handles conversion errors explicitly.
///
/// # Error Handling
///
/// This trait returns [`TryByteableError<E>`] which distinguishes between:
/// - Conversion errors (failed to convert value to bytes)
/// - I/O errors (conversion succeeded but writing bytes failed)
///
/// # Examples
///
/// ## Writing with validation
///
/// ```
/// use byteable::{AssociatedByteArray, TryIntoByteArray, WriteTryByteable};
/// use std::io::Cursor;
///
/// // A type that only accepts even values
/// #[derive(Debug, PartialEq)]
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
/// impl TryIntoByteArray for EvenU32 {
///     type Error = NotEvenError;
///     
///     fn try_to_byte_array(self) -> Result<[u8; 4], Self::Error> {
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
/// buffer.write_try_byteable(EvenU32(42))?;
///
/// #[cfg(target_endian = "little")]
/// assert_eq!(buffer.into_inner(), vec![42, 0, 0, 0]);
/// # Ok(())
/// # }
/// ```
pub trait WriteTryByteable: Write {
    /// Writes a type with fallible conversion to this writer.
    ///
    /// This method attempts to convert the value into its byte array representation
    /// and writes all bytes to the writer.
    ///
    /// # Errors
    ///
    /// This method returns [`TryByteableError::Conversion`] if:
    /// - The value could not be converted to bytes (`try_to_byte_array` failed)
    ///
    /// This method returns [`TryByteableError::Io`] if:
    /// - Any underlying I/O error occurs while writing
    ///
    /// # Examples
    ///
    /// ```
    /// use byteable::WriteTryByteable;
    /// use std::io::Cursor;
    ///
    /// let mut buffer = Cursor::new(Vec::new());
    ///
    /// // u32 implements TryIntoByteArray (never fails)
    /// buffer.write_try_byteable(42u32).unwrap();
    ///
    /// #[cfg(target_endian = "little")]
    /// assert_eq!(buffer.into_inner(), vec![42, 0, 0, 0]);
    /// ```
    fn write_try_byteable<T: TryIntoByteArray>(
        &mut self,
        data: T,
    ) -> Result<(), TryByteableError<T::Error>> {
        // Attempt to convert the data into its byte array representation
        let byte_array = data
            .try_to_byte_array()
            .map_err(TryByteableError::Conversion)?;

        // Write all bytes to the writer
        self.write_all(byte_array.as_byte_slice())?;

        Ok(())
    }
}

// Blanket implementation: any type that implements Write automatically gets WriteTryByteable
impl<T: Write> WriteTryByteable for T {}

#[cfg(test)]
mod tests {
    use byteable_derive::UnsafeByteableTransmute;

    use super::{ReadByteable, ReadTryByteable, TryByteableError, WriteByteable, WriteTryByteable};
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
            let value = u32::from_ne_bytes(bytes);
            if value % 2 == 0 {
                Ok(EvenU32(value))
            } else {
                Err(ConversionError)
            }
        }
    }

    impl TryIntoByteArray for EvenU32 {
        type Error = ConversionError;

        fn try_to_byte_array(self) -> Result<[u8; 4], Self::Error> {
            if self.0 % 2 == 0 {
                Ok(self.0.to_ne_bytes())
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

        #[cfg(target_endian = "little")]
        assert_eq!(result.unwrap(), EvenU32(42));
    }

    #[test]
    fn test_read_try_byteable_conversion_error() {
        let data = vec![43, 0, 0, 0]; // Odd value
        let mut cursor = Cursor::new(data);

        let result: Result<EvenU32, TryByteableError<ConversionError>> = cursor.read_try_byteable();
        assert!(result.is_err());

        match result {
            Err(TryByteableError::Conversion(_)) => {
                // Expected
            }
            _ => panic!("Expected conversion error"),
        }
    }

    #[test]
    fn test_read_try_byteable_io_error() {
        let data = vec![1, 2]; // Not enough bytes
        let mut cursor = Cursor::new(data);

        let result: Result<EvenU32, TryByteableError<ConversionError>> = cursor.read_try_byteable();
        assert!(result.is_err());

        match result {
            Err(TryByteableError::Io(_)) => {
                // Expected
            }
            _ => panic!("Expected I/O error"),
        }
    }

    #[test]
    fn test_write_try_byteable_success() {
        let mut buffer = Cursor::new(Vec::new());

        let result = buffer.write_try_byteable(EvenU32(100));
        assert!(result.is_ok());

        #[cfg(target_endian = "little")]
        assert_eq!(buffer.into_inner(), vec![100, 0, 0, 0]);
    }

    #[test]
    fn test_write_try_byteable_conversion_error() {
        let mut buffer = Cursor::new(Vec::new());

        let result = buffer.write_try_byteable(EvenU32(101)); // Odd value
        assert!(result.is_err());

        match result {
            Err(TryByteableError::Conversion(_)) => {
                // Expected
            }
            _ => panic!("Expected conversion error"),
        }
    }

    #[test]
    fn test_try_byteable_roundtrip() {
        let original = EvenU32(1024);

        let mut buffer = Cursor::new(Vec::new());
        buffer.write_try_byteable(original).unwrap();

        let mut reader = Cursor::new(buffer.into_inner());
        let read_value: EvenU32 = reader.read_try_byteable().unwrap();

        assert_eq!(read_value, original);
    }

    #[test]
    fn test_try_byteable_with_infallible() {
        // Test that regular types work with Try traits (should never fail)
        let mut buffer = Cursor::new(Vec::new());
        buffer.write_try_byteable(42u32).unwrap();

        let mut reader = Cursor::new(buffer.into_inner());
        let value: u32 = reader.read_try_byteable().unwrap();
        assert_eq!(value, 42);
    }
}
