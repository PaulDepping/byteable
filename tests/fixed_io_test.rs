#![cfg(feature = "std")]

use byteable::{
    BigEndian, Byteable, FixedReadable, FixedWritable, LittleEndian, ReadFixed, ReadValue,
    WriteFixed, WriteValue,
};
use std::io::Cursor;

// A derived fixed-size struct for testing.
#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
struct Header {
    #[byteable(big_endian)]
    magic: u32,
    #[byteable(little_endian)]
    version: u16,
}

#[test]
fn read_fixed_primitive() {
    // [0x01, 0x00, 0x00, 0x00] is 1 in little-endian, 0x01000000 in big-endian.
    let data = 42u32.to_ne_bytes().to_vec();
    let mut cursor = Cursor::new(data);
    let val: u32 = cursor.read_fixed().unwrap();
    assert_eq!(val, 42u32);
}

#[test]
fn read_fixed_primitive_roundtrip() {
    let original: LittleEndian<u32> = LittleEndian::new(0xDEADBEEF);
    let mut buf = Cursor::new(Vec::new());
    buf.write_fixed(&original).unwrap();

    let mut reader = Cursor::new(buf.into_inner());
    let restored: LittleEndian<u32> = reader.read_fixed().unwrap();
    assert_eq!(restored.get(), original.get());
}

#[test]
fn read_fixed_struct() {
    let header = Header {
        magic: 0x12345678,
        version: 42,
    };

    let mut buf = Cursor::new(Vec::new());
    buf.write_fixed(&header).unwrap();

    let mut reader = Cursor::new(buf.into_inner());
    let restored: Header = reader.read_fixed().unwrap();
    assert_eq!(restored, header);
}

#[test]
fn write_fixed_and_read_value_compat() {
    // Write via write_fixed, read back via read_value — must produce identical results.
    let header = Header {
        magic: 0xCAFEBABE,
        version: 7,
    };

    let mut buf = Cursor::new(Vec::new());
    buf.write_fixed(&header).unwrap();
    let fixed_bytes = buf.into_inner();

    let mut buf2 = Cursor::new(Vec::new());
    buf2.write_value(&header).unwrap();
    let byteable_bytes = buf2.into_inner();

    assert_eq!(fixed_bytes, byteable_bytes);

    // read_value can read what write_fixed wrote
    let mut reader = Cursor::new(fixed_bytes);
    let restored: Header = reader.read_value().unwrap();
    assert_eq!(restored, header);
}

#[test]
fn write_value_and_read_fixed_compat() {
    // Write via write_value, read back via read_fixed.
    let val = BigEndian::new(0x0102u16);
    let mut buf = Cursor::new(Vec::new());
    buf.write_value(&val).unwrap();

    let mut reader = Cursor::new(buf.into_inner());
    let restored: BigEndian<u16> = reader.read_fixed().unwrap();
    assert_eq!(restored.get(), val.get());
}

#[test]
fn fixed_readable_is_also_readable() {
    // Fixed-size types implement both FixedReadable and Readable (via blanket).
    fn assert_readable<T: FixedReadable + byteable::Readable>() {}
    assert_readable::<u8>();
    assert_readable::<u32>();
    assert_readable::<LittleEndian<u64>>();
    assert_readable::<Header>();
}

#[test]
fn fixed_writable_is_also_writable() {
    fn assert_writable<T: FixedWritable + byteable::Writable>() {}
    assert_writable::<u8>();
    assert_writable::<u32>();
    assert_writable::<BigEndian<u16>>();
    assert_writable::<Header>();
}

/// Verify that Vec<u32> does NOT implement FixedReadable or FixedWritable.
/// (Compile-fail test — uncomment to confirm the compiler rejects it.)
///
/// ```compile_fail
/// use byteable::{ReadFixed};
/// use std::io::Cursor;
/// let data = vec![0u8; 16];
/// let mut cursor = Cursor::new(data);
/// let _: Vec<u32> = cursor.read_fixed().unwrap();
/// ```
///
/// ```compile_fail
/// use byteable::{WriteFixed};
/// use std::io::Cursor;
/// let mut cursor = Cursor::new(Vec::new());
/// cursor.write_fixed(&vec![1u32, 2, 3]).unwrap();
/// ```
#[test]
fn compile_fail_docs_exist() {
    // This test is a placeholder so the module compiles.
    // The compile_fail doctests above verify the enforcement at the type level.
}
