//! Integration tests for tinyvec support.
#![cfg(feature = "tinyvec")]

// --- std (io) pipeline: round-trip + capacity-exceeded ---

#[cfg(feature = "std")]
mod std_pipeline {
    use byteable::DecodeError;
    use byteable::io::{ReadValue, WriteValue};
    use tinyvec::ArrayVec;

    #[test]
    fn array_vec_roundtrip() {
        let mut v: ArrayVec<[u32; 4]> = ArrayVec::new();
        v.push(1);
        v.push(2);
        let mut buf = std::vec::Vec::new();
        buf.write_value(&v).unwrap();
        let restored: ArrayVec<[u32; 4]> = std::io::Cursor::new(&buf).read_value().unwrap();
        assert_eq!(v, restored);
    }

    #[test]
    fn array_vec_capacity_exceeded_is_decode_error() {
        let mut over: std::vec::Vec<u32> = std::vec::Vec::new();
        for i in 0..5u32 {
            over.push(i);
        }
        let mut buf = std::vec::Vec::new();
        buf.write_value(&over).unwrap();
        let err = std::io::Cursor::new(&buf)
            .read_value::<ArrayVec<[u32; 4]>>()
            .unwrap_err();
        assert!(matches!(
            err,
            byteable::io::ReadableError::DecodeError(DecodeError::CapacityExceeded {
                len: 5,
                capacity: 4,
                ..
            })
        ));
    }
}

// --- pipeline parity: same round-trip over tokio / embedded-io / embedded-io-async ---

#[cfg(feature = "tokio")]
#[tokio::test]
async fn tokio_pipeline_roundtrip() {
    use byteable::async_io::{AsyncReadValue, AsyncWriteValue};
    use tinyvec::ArrayVec;

    let mut v: ArrayVec<[u32; 4]> = ArrayVec::new();
    v.push(1);
    v.push(2);
    let mut buf = std::vec::Vec::new();
    buf.write_value(&v).await.unwrap();
    let restored: ArrayVec<[u32; 4]> = std::io::Cursor::new(&buf).read_value().await.unwrap();
    assert_eq!(v, restored);
}

#[cfg(feature = "embedded-io")]
#[test]
fn embedded_io_pipeline_roundtrip() {
    use byteable::eio::{EioReadValue, EioWriteValue};
    use tinyvec::ArrayVec;

    let mut v: ArrayVec<[u32; 4]> = ArrayVec::new();
    v.push(1);
    v.push(2);
    let mut buf = [0u8; 64];
    let written;
    {
        let mut w: &mut [u8] = &mut buf;
        let before = w.len();
        w.write_value(&v).unwrap();
        written = before - w.len();
    }
    let mut r: &[u8] = &buf[..written];
    let restored: ArrayVec<[u32; 4]> = r.read_value().unwrap();
    assert_eq!(v, restored);
}

#[cfg(feature = "embedded-io-async")]
#[tokio::test]
async fn embedded_io_async_pipeline_roundtrip() {
    use byteable::eio_async::{EioAsyncReadValue, EioAsyncWriteValue};
    use tinyvec::ArrayVec;

    let mut v: ArrayVec<[u32; 4]> = ArrayVec::new();
    v.push(1);
    v.push(2);
    let mut buf = [0u8; 64];
    let written;
    {
        let mut w: &mut [u8] = &mut buf;
        let before = w.len();
        w.write_value(&v).await.unwrap();
        written = before - w.len();
    }
    let mut r: &[u8] = &buf[..written];
    let restored: ArrayVec<[u32; 4]> = r.read_value().await.unwrap();
    assert_eq!(v, restored);
}

// --- derive-macro `io_only` struct field usage ---

#[cfg(all(feature = "std", feature = "derive"))]
mod derive_field {
    use byteable::Byteable;
    use byteable::io::{ReadValue, WriteValue};
    use tinyvec::ArrayVec;

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct Sensor {
        id: u32,
        readings: ArrayVec<[u32; 8]>,
    }

    #[test]
    fn derived_io_only_struct_with_tinyvec_field_roundtrip() {
        let mut readings: ArrayVec<[u32; 8]> = ArrayVec::new();
        readings.push(1);
        readings.push(2);
        let s = Sensor { id: 7, readings };
        let mut buf = std::vec::Vec::new();
        buf.write_value(&s).unwrap();
        let restored: Sensor = std::io::Cursor::new(&buf).read_value().unwrap();
        assert_eq!(s, restored);
    }
}
