//! Byte array trait and implementations.
//!
//! This module defines the `ByteableByteArray` trait which provides methods
//! for creating zero-filled byte arrays and accessing them as byte slices.

/// Trait for working with byte arrays.
///
/// This trait provides methods for creating zero-filled byte arrays and
/// accessing them as mutable or immutable byte slices. It is primarily
/// used as an associated type for the `Byteable` trait.
pub trait ByteableByteArray {
    /// Creates a new byte array filled with zeros.
    fn create_zeroed() -> Self;
    /// Returns a mutable slice reference to the underlying byte array.
    fn as_byteslice_mut(&mut self) -> &mut [u8];
    /// Returns an immutable slice reference to the underlying byte array.
    fn as_byteslice(&self) -> &[u8];
}

/// Implements `ByteableByteArray` for fixed-size arrays `[u8; SIZE]`.
impl<const SIZE: usize> ByteableByteArray for [u8; SIZE] {
    fn create_zeroed() -> Self {
        [0; SIZE]
    }

    fn as_byteslice_mut(&mut self) -> &mut [u8] {
        self
    }

    fn as_byteslice(&self) -> &[u8] {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::ByteableByteArray;

    #[test]
    fn test_create_zeroed() {
        let arr: [u8; 5] = ByteableByteArray::create_zeroed();
        assert_eq!(arr, [0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_as_byteslice() {
        let arr: [u8; 4] = [1, 2, 3, 4];
        let slice = arr.as_byteslice();
        assert_eq!(slice, &[1, 2, 3, 4]);
    }

    #[test]
    fn test_as_byteslice_mut() {
        let mut arr: [u8; 3] = [1, 2, 3];
        let slice = arr.as_byteslice_mut();
        slice[1] = 99;
        assert_eq!(arr, [1, 99, 3]);
    }
}
