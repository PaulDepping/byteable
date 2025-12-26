//! Low-level byte array abstraction for the `Byteable` trait.
//!
//! This module provides the `ByteArray` trait, which is used internally by the `Byteable`
//! trait to represent the byte array form of types. Most users don't need to interact with
//! this trait directly.

/// A trait for types that can be used as byte array representations.
///
/// This trait is used internally by the `Byteable` trait to abstract over different
/// byte array types. It provides methods for creating zeroed arrays and converting
/// between the array type and byte slices.
///
/// # Safety
///
/// This is an `unsafe` trait because implementations must guarantee:
/// - The type is `Copy` and has a known, fixed size
/// - `zeroed()` creates a valid instance with all bytes set to zero
/// - `as_byte_slice()` and `as_byte_slice_mut()` return slices of exactly `BYTE_SIZE` bytes
/// - The memory layout allows safe reinterpretation as a byte slice
///
/// # Implementations
///
/// This trait is implemented for:
/// - `[u8; N]` for any constant `N` - representing a fixed-size byte array
/// - `[T; N]` where `T: ByteArray` - representing nested arrays
///
/// Most users should not need to implement this trait manually, as the provided
/// implementations cover common use cases.
///
/// # Examples
///
/// ```
/// use byteable::ByteArray;
///
/// // Create a zeroed byte array
/// let arr: [u8; 4] = ByteArray::zeroed();
/// assert_eq!(arr, [0, 0, 0, 0]);
///
/// // Get byte size
/// assert_eq!(<[u8; 4]>::BYTE_SIZE, 4);
///
/// // Convert to byte slice
/// let mut arr = [1u8, 2, 3, 4];
/// let slice = arr.as_byte_slice();
/// assert_eq!(slice, &[1, 2, 3, 4]);
/// ```
///
/// ## Nested arrays
///
/// ```
/// use byteable::ByteArray;
///
/// // Nested arrays also work
/// let nested: [[u8; 2]; 3] = ByteArray::zeroed();
/// assert_eq!(nested, [[0, 0], [0, 0], [0, 0]]);
///
/// // Byte size is computed correctly
/// assert_eq!(<[[u8; 2]; 3]>::BYTE_SIZE, 6);
/// ```
pub unsafe trait ByteArray: Copy {
    /// The size of this byte array in bytes.
    const BYTE_SIZE: usize;

    /// Creates a new instance with all bytes set to zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use byteable::ByteArray;
    ///
    /// let arr: [u8; 5] = ByteArray::zeroed();
    /// assert_eq!(arr, [0, 0, 0, 0, 0]);
    /// ```
    fn zeroed() -> Self;

    /// Returns a mutable byte slice view of this array.
    ///
    /// The returned slice has exactly `BYTE_SIZE` bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use byteable::ByteArray;
    ///
    /// let mut arr: [u8; 3] = [1, 2, 3];
    /// let slice = arr.as_byte_slice_mut();
    /// slice[1] = 99;
    /// assert_eq!(arr, [1, 99, 3]);
    /// ```
    fn as_byte_slice_mut(&mut self) -> &mut [u8];

    /// Returns a byte slice view of this array.
    ///
    /// The returned slice has exactly `BYTE_SIZE` bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use byteable::ByteArray;
    ///
    /// let arr: [u8; 4] = [1, 2, 3, 4];
    /// let slice = arr.as_byte_slice();
    /// assert_eq!(slice, &[1, 2, 3, 4]);
    /// ```
    fn as_byte_slice(&self) -> &[u8];
}

// Implementation for fixed-size byte arrays [u8; N]
// This is the base case - a byte array is already bytes
unsafe impl<const SIZE: usize> ByteArray for [u8; SIZE] {
    const BYTE_SIZE: usize = SIZE;

    fn zeroed() -> Self {
        // Create an array filled with zeros
        [0; SIZE]
    }

    fn as_byte_slice_mut(&mut self) -> &mut [u8] {
        // A byte array can be directly returned as a byte slice
        self
    }

    fn as_byte_slice(&self) -> &[u8] {
        // A byte array can be directly returned as a byte slice
        self
    }
}

// Implementation for nested arrays [T; N] where T implements ByteArray
// This allows arrays of Byteable types to also be Byteable
unsafe impl<T: ByteArray, const SIZE_OUTER: usize> ByteArray for [T; SIZE_OUTER] {
    // Total byte size is the size of one element times the number of elements
    const BYTE_SIZE: usize = T::BYTE_SIZE * SIZE_OUTER;

    fn zeroed() -> Self {
        // Create an array where each element is zeroed
        [T::zeroed(); SIZE_OUTER]
    }

    fn as_byte_slice_mut(&mut self) -> &mut [u8] {
        // Verify that our computed BYTE_SIZE matches the actual memory size
        debug_assert_eq!(std::mem::size_of::<Self>(), Self::BYTE_SIZE);

        // Reinterpret the array of T as a flat byte slice
        // SAFETY: ByteArray guarantees contiguous memory layout
        unsafe {
            std::slice::from_raw_parts_mut(self.as_mut_ptr() as _, std::mem::size_of::<Self>())
        }
    }

    fn as_byte_slice(&self) -> &[u8] {
        // Verify that our computed BYTE_SIZE matches the actual memory size
        debug_assert_eq!(std::mem::size_of::<Self>(), Self::BYTE_SIZE);

        // Reinterpret the array of T as a flat byte slice
        // SAFETY: ByteArray guarantees contiguous memory layout
        unsafe { std::slice::from_raw_parts(self.as_ptr() as _, std::mem::size_of::<Self>()) }
    }
}

#[cfg(test)]
mod tests {
    use super::ByteArray;

    #[test]
    fn test_create_zeroed() {
        let arr: [u8; 5] = ByteArray::zeroed();
        assert_eq!(arr, [0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_as_byteslice() {
        let arr: [u8; 4] = [1, 2, 3, 4];
        let slice = arr.as_byte_slice();
        assert_eq!(slice, &[1, 2, 3, 4]);
    }

    #[test]
    fn test_nested_byteslice() {
        let mut arr: [[u8; 3]; 3] = [[1, 2, 3]; 3];
        assert_eq!(<[[u8; 3]; 3]>::BYTE_SIZE, 9);
        assert_eq!(arr.as_byte_slice().len(), 9);
        assert_eq!(arr.as_byte_slice_mut().len(), 9);
        let slice = arr.as_byte_slice();
        assert_eq!(slice, &[1, 2, 3, 1, 2, 3, 1, 2, 3]);
    }

    #[test]
    fn test_as_byteslice_mut() {
        let mut arr: [u8; 3] = [1, 2, 3];
        let slice = arr.as_byte_slice_mut();
        slice[1] = 99;
        assert_eq!(arr, [1, 99, 3]);
    }
}
