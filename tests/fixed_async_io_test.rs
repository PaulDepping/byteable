#![cfg(all(feature = "std", feature = "tokio"))]

use byteable::{
    AsyncFixedReadable, AsyncFixedWritable, AsyncReadFixed, AsyncReadValue, AsyncWriteFixed,
    AsyncWriteValue, BigEndian, Byteable, LittleEndian,
};
use std::io::Cursor;

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
struct AsyncHeader {
    #[byteable(big_endian)]
    magic: u32,
    #[byteable(little_endian)]
    version: u16,
}

#[tokio::test]
async fn async_read_fixed_primitive_roundtrip() {
    let original: LittleEndian<u32> = LittleEndian::new(0xDEADBEEF);
    let mut buf = Cursor::new(Vec::new());
    buf.write_fixed(&original).await.unwrap();

    let mut reader = Cursor::new(buf.into_inner());
    let restored: LittleEndian<u32> = reader.read_fixed().await.unwrap();
    assert_eq!(restored.get(), original.get());
}

#[tokio::test]
async fn async_read_fixed_struct() {
    let header = AsyncHeader {
        magic: 0x12345678,
        version: 42,
    };

    let mut buf = Cursor::new(Vec::new());
    buf.write_fixed(&header).await.unwrap();

    let mut reader = Cursor::new(buf.into_inner());
    let restored: AsyncHeader = reader.read_fixed().await.unwrap();
    assert_eq!(restored, header);
}

#[tokio::test]
async fn async_write_fixed_and_read_value_compat() {
    // write_fixed and write_value must produce identical bytes.
    let header = AsyncHeader {
        magic: 0xCAFEBABE,
        version: 7,
    };

    let mut buf1 = Cursor::new(Vec::new());
    buf1.write_fixed(&header).await.unwrap();

    let mut buf2 = Cursor::new(Vec::new());
    buf2.write_value(&header).await.unwrap();

    assert_eq!(buf1.into_inner(), buf2.clone().into_inner());

    // read_value can read what write_fixed wrote
    let mut reader = Cursor::new(buf2.into_inner());
    let restored: AsyncHeader = reader.read_value().await.unwrap();
    assert_eq!(restored, header);
}

#[tokio::test]
async fn async_write_value_and_read_fixed_compat() {
    let val = BigEndian::new(0x0102u16);
    let mut buf = Cursor::new(Vec::new());
    buf.write_value(&val).await.unwrap();

    let mut reader = Cursor::new(buf.into_inner());
    let restored: BigEndian<u16> = reader.read_fixed().await.unwrap();
    assert_eq!(restored.get(), val.get());
}

#[tokio::test]
async fn async_fixed_readable_is_also_async_readable() {
    fn assert_both<T: AsyncFixedReadable + byteable::AsyncReadable>() {}
    assert_both::<u8>();
    assert_both::<u32>();
    assert_both::<LittleEndian<u64>>();
    assert_both::<AsyncHeader>();
}

#[tokio::test]
async fn async_fixed_writable_is_also_async_writable() {
    fn assert_both<T: AsyncFixedWritable + byteable::AsyncWritable>() {}
    assert_both::<u8>();
    assert_both::<u32>();
    assert_both::<BigEndian<u16>>();
    assert_both::<AsyncHeader>();
}

/// Vec<u32> is NOT accessible via AsyncReadFixed or AsyncWriteFixed.
///
/// ```compile_fail
/// use byteable::AsyncReadFixed;
/// use std::io::Cursor;
/// # #[tokio::main] async fn main() {
/// let data = vec![0u8; 16];
/// let mut cursor = Cursor::new(data);
/// let _: Vec<u32> = cursor.read_fixed().await.unwrap();
/// # }
/// ```
///
/// ```compile_fail
/// use byteable::AsyncWriteFixed;
/// use std::io::Cursor;
/// # #[tokio::main] async fn main() {
/// let mut cursor = Cursor::new(Vec::new());
/// cursor.write_fixed(&vec![1u32, 2, 3]).await.unwrap();
/// # }
/// ```
#[tokio::test]
async fn compile_fail_docs_exist() {}
