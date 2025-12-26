use crate::{ByteArray, Byteable};
use std::{fmt, hash::Hash};
pub trait EndianConvert: Copy {
    type ByteArray: ByteArray;
    fn from_le_bytes(byte_array: Self::ByteArray) -> Self;
    fn from_be_bytes(byte_array: Self::ByteArray) -> Self;
    fn from_ne_bytes(byte_array: Self::ByteArray) -> Self;

    fn to_le_bytes(self) -> Self::ByteArray;
    fn to_be_bytes(self) -> Self::ByteArray;
    fn to_ne_bytes(self) -> Self::ByteArray;
}

macro_rules! impl_endianable {
    ($($type:ty),+) => {
        $(
            impl $crate::EndianConvert for $type {
                type ByteArray = [u8; ::std::mem::size_of::<$type>()];

                fn from_le_bytes(byte_array: Self::ByteArray) -> Self {
                    <$type>::from_le_bytes(byte_array)
                }

                fn from_be_bytes(byte_array: Self::ByteArray) -> Self {
                    <$type>::from_be_bytes(byte_array)
                }

                fn from_ne_bytes(byte_array: Self::ByteArray) -> Self {
                    <$type>::from_ne_bytes(byte_array)
                }

                fn to_ne_bytes(self) -> Self::ByteArray {
                    <$type>::to_ne_bytes(self)
                }

                fn to_le_bytes(self) -> Self::ByteArray {
                    <$type>::to_le_bytes(self)
                }

                fn to_be_bytes(self) -> Self::ByteArray {
                    <$type>::to_be_bytes(self)
                }
            }
        )+
    };
}

impl_endianable!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct BigEndian<T: EndianConvert>(pub(crate) T::ByteArray);

impl<T: fmt::Debug + EndianConvert> fmt::Debug for BigEndian<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BigEndian").field(&self.get()).finish()
    }
}

impl<T: PartialEq + EndianConvert> PartialEq for BigEndian<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl<T: Eq + EndianConvert> Eq for BigEndian<T> {}

impl<T: PartialOrd + EndianConvert> PartialOrd for BigEndian<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

impl<T: Ord + EndianConvert> Ord for BigEndian<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T: Hash + EndianConvert> Hash for BigEndian<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state);
    }
}

impl<T: EndianConvert> BigEndian<T> {
    pub fn new(value: T) -> Self {
        Self(value.to_be_bytes())
    }

    pub fn get(self) -> T {
        T::from_be_bytes(self.0)
    }

    pub fn raw_bytes(self) -> T::ByteArray {
        self.0
    }
}

impl<T: EndianConvert + Default> Default for BigEndian<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: EndianConvert> Byteable for BigEndian<T> {
    type ByteArray = <T as EndianConvert>::ByteArray;

    fn as_byte_array(self) -> Self::ByteArray {
        self.0
    }

    fn from_byte_array(byte_array: Self::ByteArray) -> Self {
        Self(byte_array)
    }
}

impl<T: EndianConvert> From<T> for BigEndian<T> {
    fn from(value: T) -> Self {
        BigEndian::new(value)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct LittleEndian<T: EndianConvert>(T::ByteArray);

impl<T: fmt::Debug + EndianConvert> fmt::Debug for LittleEndian<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("LittleEndian").field(&self.get()).finish()
    }
}

impl<T: PartialEq + EndianConvert> PartialEq for LittleEndian<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl<T: Eq + EndianConvert> Eq for LittleEndian<T> {}

impl<T: PartialOrd + EndianConvert> PartialOrd for LittleEndian<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

impl<T: Ord + EndianConvert> Ord for LittleEndian<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T: Hash + EndianConvert> Hash for LittleEndian<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state);
    }
}

impl<T: EndianConvert> LittleEndian<T> {
    pub fn new(value: T) -> Self {
        Self(value.to_le_bytes())
    }

    pub fn get(self) -> T {
        T::from_le_bytes(self.0)
    }

    pub fn raw_bytes(self) -> T::ByteArray {
        self.0
    }
}

impl<T: EndianConvert + Default> Default for LittleEndian<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: EndianConvert> Byteable for LittleEndian<T> {
    type ByteArray = <T as EndianConvert>::ByteArray;

    fn as_byte_array(self) -> Self::ByteArray {
        self.0
    }

    fn from_byte_array(byte_array: Self::ByteArray) -> Self {
        Self(byte_array)
    }
}

impl<T: EndianConvert> From<T> for LittleEndian<T> {
    fn from(value: T) -> Self {
        LittleEndian::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{BigEndian, LittleEndian};

    #[test]
    fn big_endian_test() {
        let val = 0x01020304u32;
        let be_val = BigEndian::new(val);

        assert_eq!(be_val.get(), val);
        assert_eq!(be_val.raw_bytes(), [1, 2, 3, 4]);
        assert_eq!(u32::from_be_bytes(be_val.raw_bytes()), val);
    }

    #[test]
    fn little_endian_test() {
        let val = 0x01020304u32;
        let le_val = LittleEndian::new(val);

        assert_eq!(le_val.get(), val);
        assert_eq!(le_val.raw_bytes(), [4, 3, 2, 1]);
        assert_eq!(u32::from_le_bytes(le_val.raw_bytes()), val);
    }
}
