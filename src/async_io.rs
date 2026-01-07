//! Asynchronous I/O extensions for reading and writing byteable types.
//!
//! This module provides extension traits for `tokio::io::AsyncRead` and `tokio::io::AsyncWrite`
//! that enable convenient async reading and writing of types implementing the byte conversion traits
//! ([`IntoByteArray`] and [`FromByteArray`]).
//!
//! This module is only available when the `tokio` feature is enabled.

use crate::byte_array::ByteArray;
use crate::{FromByteArray, IntoByteArray, TryByteableError, TryFromByteArray, TryIntoByteArray};
use core::future::Future;
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
/// # #![cfg(all(feature = "tokio", feature = "derive"))]
/// use byteable::{Byteable, AsyncReadByteable};
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
/// let header: Header = file.read_byteable().await?;
/// println!("Header: {:?}", header);
/// # Ok(())
/// # }
/// ```
///
/// ## Reading from an async TCP stream
///
/// ```no_run
/// # #![cfg(feature = "tokio")]
/// use byteable::{AsyncReadByteable, LittleEndian};
/// use tokio::net::TcpStream;
///
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
///
/// // Read a u32 length prefix
/// let length: LittleEndian<u32> = stream.read_byteable().await?;
/// println!("Message length: {}", length.get());
/// # Ok(())
/// # }
/// ```
///
/// ## Reading multiple values sequentially
///
/// ```no_run
/// # #![cfg(feature = "tokio")]
/// use byteable::{AsyncReadByteable, LittleEndian};
/// use std::io::Cursor;
///
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let data = vec![1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0];
/// let mut cursor = Cursor::new(data);
///
/// let a: LittleEndian<u32> = cursor.read_byteable().await?;
/// let b: LittleEndian<u32> = cursor.read_byteable().await?;
/// let c: LittleEndian<u32> = cursor.read_byteable().await?;
///
/// assert_eq!((a.get(), b.get(), c.get()), (1, 2, 3));
/// # Ok(())
/// # }
/// ```
pub trait AsyncReadByteable: tokio::io::AsyncReadExt + Unpin {
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
    /// # #[tokio::main(flavor = "current_thread")]
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
    fn read_byteable<T: FromByteArray>(&mut self) -> impl Future<Output = std::io::Result<T>> {
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
impl<T: AsyncReadExt + Unpin> AsyncReadByteable for T {}

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
/// # #![cfg(all(feature = "tokio", feature = "derive"))]
/// use byteable::{Byteable, AsyncWriteByteable};
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
/// file.write_byteable(header).await?;
/// # Ok(())
/// # }
/// ```
///
/// ## Writing to an async TCP stream
///
/// ```no_run
/// # #![cfg(feature = "tokio")]
/// use byteable::{AsyncWriteByteable, LittleEndian};
/// use tokio::net::TcpStream;
///
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
///
/// // Write a u32 length prefix
/// stream.write_byteable(LittleEndian::new(42u32)).await?;
/// # Ok(())
/// # }
/// ```
///
/// ## Writing multiple values
///
/// ```no_run
/// # #![cfg(feature = "tokio")]
/// use byteable::{AsyncWriteByteable, LittleEndian};
/// use std::io::Cursor;
///
/// # #[tokio::main]
/// # async fn main() -> std::io::Result<()> {
/// let mut buffer = Cursor::new(Vec::new());
///
/// buffer.write_byteable(LittleEndian::new(1u32)).await?;
/// buffer.write_byteable(LittleEndian::new(2u32)).await?;
/// buffer.write_byteable(LittleEndian::new(3u32)).await?;
///
/// assert_eq!(
///     buffer.into_inner(),
///     vec![1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]
/// );
/// # Ok(())
/// # }
/// ```
pub trait AsyncWriteByteable: tokio::io::AsyncWriteExt + Unpin {
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
    /// # #![cfg(feature = "tokio")]
    /// use byteable::{Byteable, AsyncWriteByteable, LittleEndian};
    /// use std::io::Cursor;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let mut buffer = Cursor::new(Vec::new());
    /// buffer.write_byteable(LittleEndian::new(0x12345678u32)).await?;
    ///
    /// assert_eq!(buffer.into_inner(), vec![0x78, 0x56, 0x34, 0x12]);
    /// # Ok(())
    /// # }
    /// ```
    fn write_byteable<T: IntoByteArray>(
        &mut self,
        data: T,
    ) -> impl Future<Output = std::io::Result<()>> {
        async move {
            // Convert the data into its byte array representation
            let byte_array = data.into_byte_array();

            // Asynchronously write all bytes to the writer
            self.write_all(byte_array.as_byte_slice()).await
        }
    }
}

// Blanket implementation: any type that implements AsyncWriteExt automatically gets AsyncWriteByteable
impl<T: AsyncWriteExt + Unpin> AsyncWriteByteable for T {}

/// Extension trait for `AsyncRead` that adds methods for reading types with fallible conversion asynchronously.
///
/// This trait is automatically implemented for all types that implement `tokio::io::AsyncReadExt`,
/// providing methods for reading types that implement [`TryFromByteArray`] in async contexts. Unlike
/// [`AsyncReadByteable`], this trait handles conversion errors explicitly.
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
/// ```no_run
/// # #![cfg(feature = "tokio")]
/// use byteable::{AssociatedByteArray, TryFromByteArray, AsyncReadTryByteable};
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
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let data = vec![2, 0, 0, 0]; // Even value
/// let mut cursor = Cursor::new(data);
/// let value: EvenU32 = cursor.read_try_byteable().await?;
/// #[cfg(target_endian = "little")]
/// assert_eq!(value, EvenU32(2));
///
/// let odd_data = vec![3, 0, 0, 0]; // Odd value
/// let mut cursor = Cursor::new(odd_data);
/// let result: Result<EvenU32, _> = cursor.read_try_byteable().await;
/// assert!(result.is_err());
/// # Ok(())
/// # }
/// ```
pub trait AsyncReadTryByteable: tokio::io::AsyncReadExt + Unpin {
    /// Asynchronously reads a type with fallible conversion from this reader.
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
    /// ```no_run
    /// # #[cfg(feature = "tokio")]
    /// use byteable::AsyncReadTryByteable;
    /// # #[cfg(feature = "tokio")]
    /// use std::io::Cursor;
    ///
    /// # #[cfg(feature = "tokio")]
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> std::io::Result<()> {
    /// let data = vec![42, 0, 0, 0];
    /// let mut cursor = Cursor::new(data);
    ///
    /// // u32 implements TryFromByteArray (never fails)
    /// let value: u32 = cursor.read_try_byteable().await.unwrap();
    /// #[cfg(target_endian = "little")]
    /// assert_eq!(value, 42);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "tokio"))]
    /// # fn main() {}
    /// ```
    fn read_try_byteable<T: TryFromByteArray>(
        &mut self,
    ) -> impl Future<Output = Result<T, TryByteableError<T::Error>>> {
        async move {
            // Create a zeroed byte array to hold the data
            let mut byte_array = T::ByteArray::zeroed();

            // Asynchronously read exactly BYTE_SIZE bytes from the reader
            self.read_exact(byte_array.as_byte_slice_mut()).await?;

            // Attempt to convert the bytes into the target type
            T::try_from_byte_array(byte_array).map_err(TryByteableError::Conversion)
        }
    }
}

// Blanket implementation: any type that implements AsyncReadExt automatically gets AsyncReadTryByteable
impl<T: AsyncReadExt + Unpin> AsyncReadTryByteable for T {}

/// Extension trait for `AsyncWrite` that adds methods for writing types with fallible conversion asynchronously.
///
/// This trait is automatically implemented for all types that implement `tokio::io::AsyncWriteExt`,
/// providing methods for writing types that implement [`TryIntoByteArray`] in async contexts. Unlike
/// [`AsyncWriteByteable`], this trait handles conversion errors explicitly.
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
/// ```no_run
/// # #![cfg(feature = "tokio")]
/// use byteable::{AssociatedByteArray, TryIntoByteArray, AsyncWriteTryByteable};
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
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut buffer = Cursor::new(Vec::new());
/// buffer.write_try_byteable(EvenU32(42)).await?;
///
/// #[cfg(target_endian = "little")]
/// assert_eq!(buffer.into_inner(), vec![42, 0, 0, 0]);
/// # Ok(())
/// # }
/// ```
pub trait AsyncWriteTryByteable: tokio::io::AsyncWriteExt + Unpin {
    /// Asynchronously writes a type with fallible conversion to this writer.
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
    /// ```no_run
    /// # #[cfg(feature = "tokio")]
    /// use byteable::AsyncWriteTryByteable;
    /// # #[cfg(feature = "tokio")]
    /// use std::io::Cursor;
    ///
    /// # #[cfg(feature = "tokio")]
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> std::io::Result<()> {
    /// let mut buffer = Cursor::new(Vec::new());
    ///
    /// // u32 implements TryIntoByteArray (never fails)
    /// buffer.write_try_byteable(42u32).await.unwrap();
    ///
    /// #[cfg(target_endian = "little")]
    /// assert_eq!(buffer.into_inner(), vec![42, 0, 0, 0]);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "tokio"))]
    /// # fn main() {}
    /// ```
    fn write_try_byteable<T: TryIntoByteArray>(
        &mut self,
        data: T,
    ) -> impl Future<Output = Result<(), TryByteableError<T::Error>>> {
        async move {
            // Attempt to convert the data into its byte array representation
            let byte_array = data
                .try_to_byte_array()
                .map_err(TryByteableError::Conversion)?;

            // Asynchronously write all bytes to the writer
            self.write_all(byte_array.as_byte_slice()).await?;

            Ok(())
        }
    }
}

// Blanket implementation: any type that implements AsyncWriteExt automatically gets AsyncWriteTryByteable
impl<T: AsyncWriteExt + Unpin> AsyncWriteTryByteable for T {}

#[cfg(test)]
mod tests {
    use super::{
        AsyncReadByteable, AsyncReadTryByteable, AsyncWriteByteable, AsyncWriteTryByteable,
        TryByteableError,
    };
    use crate::{
        AssociatedByteArray, BigEndian, Byteable, LittleEndian, TryFromByteArray, TryIntoByteArray,
    };
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
        buffer.write_byteable(packet).await.unwrap();
        assert_eq!(buffer.into_inner(), vec![123, 0, 4, 3, 2, 1]);
    }

    #[tokio::test]
    async fn test_async_read_one() {
        let data = vec![123, 0, 4, 3, 2, 1];
        let mut reader = Cursor::new(data);
        let packet: AsyncTestPacket = reader.read_byteable().await.unwrap();

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

    #[tokio::test]
    async fn test_async_read_try_byteable_success() {
        let data = vec![42, 0, 0, 0]; // Even value
        let mut cursor = Cursor::new(data);

        let result: Result<EvenU32, _> = cursor.read_try_byteable().await;
        assert!(result.is_ok());

        #[cfg(target_endian = "little")]
        assert_eq!(result.unwrap(), EvenU32(42));
    }

    #[tokio::test]
    async fn test_async_read_try_byteable_conversion_error() {
        let data = vec![43, 0, 0, 0]; // Odd value
        let mut cursor = Cursor::new(data);

        let result: Result<EvenU32, TryByteableError<ConversionError>> =
            cursor.read_try_byteable().await;
        assert!(result.is_err());

        match result {
            Err(TryByteableError::Conversion(_)) => {
                // Expected
            }
            _ => panic!("Expected conversion error"),
        }
    }

    #[tokio::test]
    async fn test_async_read_try_byteable_io_error() {
        let data = vec![1, 2]; // Not enough bytes
        let mut cursor = Cursor::new(data);

        let result: Result<EvenU32, TryByteableError<ConversionError>> =
            cursor.read_try_byteable().await;
        assert!(result.is_err());

        match result {
            Err(TryByteableError::Io(_)) => {
                // Expected
            }
            _ => panic!("Expected I/O error"),
        }
    }

    #[tokio::test]
    async fn test_async_write_try_byteable_success() {
        let mut buffer = Cursor::new(Vec::new());

        let result = buffer.write_try_byteable(EvenU32(100)).await;
        assert!(result.is_ok());

        #[cfg(target_endian = "little")]
        assert_eq!(buffer.into_inner(), vec![100, 0, 0, 0]);
    }

    #[tokio::test]
    async fn test_async_write_try_byteable_conversion_error() {
        let mut buffer = Cursor::new(Vec::new());

        let result = buffer.write_try_byteable(EvenU32(101)).await; // Odd value
        assert!(result.is_err());

        match result {
            Err(TryByteableError::Conversion(_)) => {
                // Expected
            }
            _ => panic!("Expected conversion error"),
        }
    }

    #[tokio::test]
    async fn test_async_try_byteable_roundtrip() {
        let original = EvenU32(1024);

        let mut buffer = Cursor::new(Vec::new());
        buffer.write_try_byteable(original).await.unwrap();

        let mut reader = Cursor::new(buffer.into_inner());
        let read_value: EvenU32 = reader.read_try_byteable().await.unwrap();

        assert_eq!(read_value, original);
    }

    #[tokio::test]
    async fn test_async_try_byteable_with_infallible() {
        // Test that regular types work with Try traits (should never fail)
        let mut buffer = Cursor::new(Vec::new());
        buffer.write_try_byteable(42u32).await.unwrap();

        let mut reader = Cursor::new(buffer.into_inner());
        let value: u32 = reader.read_try_byteable().await.unwrap();
        assert_eq!(value, 42);
    }
}
