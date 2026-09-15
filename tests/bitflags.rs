//! Integration tests for bitflags support.
#![cfg(feature = "bitflags")]

use bitflags::bitflags;
use byteable::{Byteable, FromByteArray, IntoByteArray};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Perms: u8 {
        const READ = 0b0000_0001;
        const WRITE = 0b0000_0010;
        const EXEC = 0b0000_0100;
    }
}
byteable::impl_bitflags!(Perms);

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct WideFlags: u32 {
        const A = 0x0000_0001;
        const B = 0x0000_0002;
    }
}
byteable::impl_bitflags!(WideFlags);
byteable::impl_bitflags_endian!(WideFlags);

#[test]
fn u8_backed_flags_roundtrip() {
    let val = Perms::READ | Perms::EXEC;
    let bytes = val.into_byte_array();
    let restored = Perms::from_byte_array(bytes);
    assert_eq!(val, restored);
}

#[test]
fn u32_backed_flags_roundtrip_is_little_endian() {
    let val = WideFlags::B;
    let bytes = val.into_byte_array();
    assert_eq!(bytes, [0x02, 0x00, 0x00, 0x00]);
    let restored = WideFlags::from_byte_array(bytes);
    assert_eq!(val, restored);
}

#[test]
fn decoding_unknown_bits_retains_them_instead_of_erroring() {
    // Bit 0b1000_0000 has no matching flag in `Perms`.
    let bytes = [0b1000_0001u8];
    let restored = Perms::from_byte_array(bytes);
    assert_eq!(restored.bits(), 0b1000_0001);
    assert!(restored.contains(Perms::READ));
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
struct FilePerms {
    #[byteable(big_endian)]
    owner: WideFlags,
    group: Perms,
}

#[test]
fn derived_struct_with_bitflags_field_roundtrip() {
    let val = FilePerms {
        owner: WideFlags::A,
        group: Perms::WRITE,
    };
    let bytes = val.into_byte_array();
    let restored = FilePerms::from_byte_array(bytes);
    assert_eq!(val, restored);
}

// Proves that a bitflags type gets every I/O pipeline for free through this crate's existing
// blanket impls over `RawRepr`/`TryFromRawRepr` — no pipeline-specific code needed for it.

#[cfg(feature = "std")]
#[test]
fn std_io_pipeline_roundtrip() {
    use byteable::io::{ReadFixed, WriteFixed};

    let val = Perms::READ | Perms::WRITE;
    let mut buf = Vec::new();
    buf.write_fixed(&val).unwrap();
    let restored: Perms = std::io::Cursor::new(&buf).read_fixed().unwrap();
    assert_eq!(val, restored);
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn tokio_pipeline_roundtrip() {
    use byteable::async_io::{AsyncReadFixed, AsyncWriteFixed};

    let val = Perms::READ | Perms::EXEC;
    let mut buf = Vec::new();
    buf.write_fixed(&val).await.unwrap();
    let restored: Perms = std::io::Cursor::new(&buf).read_fixed().await.unwrap();
    assert_eq!(val, restored);
}

#[cfg(feature = "embedded-io")]
#[test]
fn embedded_io_pipeline_roundtrip() {
    use byteable::eio::{EioReadFixed, EioWriteFixed};

    let val = WideFlags::A | WideFlags::B;
    let mut buf = [0u8; 4];
    {
        let mut w: &mut [u8] = &mut buf;
        w.write_fixed(&val).unwrap();
    }
    let mut r: &[u8] = &buf;
    let restored: WideFlags = r.read_fixed().unwrap();
    assert_eq!(val, restored);
}

#[cfg(feature = "embedded-io-async")]
#[tokio::test]
async fn embedded_io_async_pipeline_roundtrip() {
    use byteable::eio_async::{EioAsyncReadFixed, EioAsyncWriteFixed};

    let val = WideFlags::B;
    let mut buf = [0u8; 4];
    {
        let mut w: &mut [u8] = &mut buf;
        w.write_fixed(&val).await.unwrap();
    }
    let mut r: &[u8] = &buf;
    let restored: WideFlags = r.read_fixed().await.unwrap();
    assert_eq!(val, restored);
}
