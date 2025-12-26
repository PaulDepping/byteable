pub unsafe trait ByteArray: Copy {
    const BYTE_SIZE: usize;
    fn zeroed() -> Self;
    fn as_byte_slice_mut(&mut self) -> &mut [u8];
    fn as_byte_slice(&self) -> &[u8];
}

unsafe impl<const SIZE: usize> ByteArray for [u8; SIZE] {
    const BYTE_SIZE: usize = SIZE;

    fn zeroed() -> Self {
        [0; SIZE]
    }

    fn as_byte_slice_mut(&mut self) -> &mut [u8] {
        self
    }

    fn as_byte_slice(&self) -> &[u8] {
        self
    }
}

unsafe impl<T: ByteArray, const SIZE_OUTER: usize> ByteArray for [T; SIZE_OUTER] {
    const BYTE_SIZE: usize = T::BYTE_SIZE * SIZE_OUTER;

    fn zeroed() -> Self {
        [T::zeroed(); SIZE_OUTER]
    }

    fn as_byte_slice_mut(&mut self) -> &mut [u8] {
        debug_assert_eq!(std::mem::size_of::<Self>(), Self::BYTE_SIZE);
        unsafe {
            std::slice::from_raw_parts_mut(self.as_mut_ptr() as _, std::mem::size_of::<Self>())
        }
    }

    fn as_byte_slice(&self) -> &[u8] {
        debug_assert_eq!(std::mem::size_of::<Self>(), Self::BYTE_SIZE);
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
