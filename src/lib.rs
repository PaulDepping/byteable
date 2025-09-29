use std::io::{Read, Write};

#[cfg(feature = "derive")]
pub use byteable_derive::Byteable;

#[cfg(feature = "tokio")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub trait ByteableByteArray {
    fn create_zeroed() -> Self;
    fn as_byteslice_mut(&mut self) -> &mut [u8];
    fn as_byteslice(&self) -> &[u8];
}

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

pub trait Byteable: Copy {
    type ByteArray: ByteableByteArray;
    fn as_bytearray(self) -> Self::ByteArray;
    fn from_bytearray(ba: Self::ByteArray) -> Self;
}

pub trait ReadByteable: Read {
    fn read_one<T: Byteable>(&mut self) -> std::io::Result<T> {
        let mut e = T::ByteArray::create_zeroed();
        self.read_exact(e.as_byteslice_mut())?;
        Ok(T::from_bytearray(e))
    }
}

impl<T: Read> ReadByteable for T {}

pub trait WriteByteable: Write {
    fn write_one<T: Byteable>(&mut self, data: T) -> std::io::Result<()> {
        let e = data.as_bytearray();
        self.write_all(e.as_byteslice())
    }
}

impl<T: Write> WriteByteable for T {}

#[cfg(feature = "tokio")]
pub trait AsyncReadByteable: tokio::io::AsyncReadExt {
    fn read_one<T: Byteable>(&mut self) -> impl Future<Output = std::io::Result<T>>
    where
        Self: Unpin + Send,
    {
        async move {
            let mut e = T::ByteArray::create_zeroed();
            self.read_exact(e.as_byteslice_mut()).await?;
            Ok(T::from_bytearray(e))
        }
    }
}

#[cfg(feature = "tokio")]
impl<T: AsyncReadExt> AsyncReadByteable for T {}

#[cfg(feature = "tokio")]
pub trait AsyncWriteByteable: tokio::io::AsyncWriteExt {
    fn write_one<T: Byteable>(&mut self, data: T) -> impl Future<Output = std::io::Result<()>>
    where
        Self: Unpin,
    {
        async move {
            let e = data.as_bytearray();
            self.write_all(e.as_byteslice()).await
        }
    }
}

#[cfg(feature = "tokio")]
impl<T: AsyncWriteExt> AsyncWriteByteable for T {}

pub trait Endianable: Copy {
    fn from_le(self) -> Self;
    fn from_be(self) -> Self;
    fn to_le(self) -> Self;
    fn to_be(self) -> Self;
}

impl Endianable for u8 {
    fn from_le(self) -> Self {
        Self::from_le(self)
    }

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn to_le(self) -> Self {
        Self::to_le(self)
    }

    fn to_be(self) -> Self {
        Self::to_be(self)
    }
}
impl Endianable for u16 {
    fn from_le(self) -> Self {
        Self::from_le(self)
    }

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn to_le(self) -> Self {
        Self::to_le(self)
    }

    fn to_be(self) -> Self {
        Self::to_be(self)
    }
}
impl Endianable for u32 {
    fn from_le(self) -> Self {
        Self::from_le(self)
    }

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn to_le(self) -> Self {
        Self::to_le(self)
    }

    fn to_be(self) -> Self {
        Self::to_be(self)
    }
}
impl Endianable for u64 {
    fn from_le(self) -> Self {
        Self::from_le(self)
    }

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn to_le(self) -> Self {
        Self::to_le(self)
    }

    fn to_be(self) -> Self {
        Self::to_be(self)
    }
}
impl Endianable for u128 {
    fn from_le(self) -> Self {
        Self::from_le(self)
    }

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn to_le(self) -> Self {
        Self::to_le(self)
    }

    fn to_be(self) -> Self {
        Self::to_be(self)
    }
}
impl Endianable for usize {
    fn from_le(self) -> Self {
        Self::from_le(self)
    }

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn to_le(self) -> Self {
        Self::to_le(self)
    }

    fn to_be(self) -> Self {
        Self::to_be(self)
    }
}

impl Endianable for i8 {
    fn from_le(self) -> Self {
        Self::from_le(self)
    }

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn to_le(self) -> Self {
        Self::to_le(self)
    }

    fn to_be(self) -> Self {
        Self::to_be(self)
    }
}
impl Endianable for i16 {
    fn from_le(self) -> Self {
        Self::from_le(self)
    }

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn to_le(self) -> Self {
        Self::to_le(self)
    }

    fn to_be(self) -> Self {
        Self::to_be(self)
    }
}
impl Endianable for i32 {
    fn from_le(self) -> Self {
        Self::from_le(self)
    }

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn to_le(self) -> Self {
        Self::to_le(self)
    }

    fn to_be(self) -> Self {
        Self::to_be(self)
    }
}
impl Endianable for i64 {
    fn from_le(self) -> Self {
        Self::from_le(self)
    }

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn to_le(self) -> Self {
        Self::to_le(self)
    }

    fn to_be(self) -> Self {
        Self::to_be(self)
    }
}

impl Endianable for i128 {
    fn from_le(self) -> Self {
        Self::from_le(self)
    }

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn to_le(self) -> Self {
        Self::to_le(self)
    }

    fn to_be(self) -> Self {
        Self::to_be(self)
    }
}
impl Endianable for isize {
    fn from_le(self) -> Self {
        Self::from_le(self)
    }

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn to_le(self) -> Self {
        Self::to_le(self)
    }

    fn to_be(self) -> Self {
        Self::to_be(self)
    }
}

// impl Endianable for f16 {}
impl Endianable for f32 {
    fn from_le(self) -> Self {
        Self::from_bits(u32::from_le(self.to_bits()))
    }

    fn from_be(self) -> Self {
        Self::from_bits(u32::from_be(self.to_bits()))
    }

    fn to_le(self) -> Self {
        Self::from_bits(u32::to_le(self.to_bits()))
    }

    fn to_be(self) -> Self {
        Self::from_bits(u32::to_be(self.to_bits()))
    }
}
impl Endianable for f64 {
    fn from_le(self) -> Self {
        Self::from_bits(u64::from_le(self.to_bits()))
    }

    fn from_be(self) -> Self {
        Self::from_bits(u64::from_be(self.to_bits()))
    }
    fn to_le(self) -> Self {
        Self::from_bits(u64::to_le(self.to_bits()))
    }

    fn to_be(self) -> Self {
        Self::from_bits(u64::to_be(self.to_bits()))
    }
}
// impl Endianable for f128 {}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct BigEndian<T: Endianable>(pub(crate) T);

impl<T: Endianable> BigEndian<T> {
    pub fn new(val: T) -> Self {
        Self(val.to_be())
    }

    pub fn into_inner(self) -> T {
        self.0.from_be()
    }
}

impl<T: Endianable + Default> Default for BigEndian<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct LittleEndian<T: Endianable>(T);

impl<T: Endianable> LittleEndian<T> {
    pub fn new(val: T) -> Self {
        Self(val.to_le())
    }

    pub fn into_inner(self) -> T {
        self.0.from_le()
    }
}

impl<T: Endianable + Default> Default for LittleEndian<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_endian_test() {
        assert_eq!([1, 2, 3, 4], BigEndian::new(0x01020304u32).0.to_ne_bytes());
    }

    #[test]
    fn little_endian_test() {
        assert_eq!(
            [4, 3, 2, 1],
            LittleEndian::new(0x01020304u32).0.to_ne_bytes()
        );
    }
}
