//! Integration tests for arrayvec support.
#![cfg(feature = "arrayvec")]

// --- std (io) pipeline: round-trip + capacity-exceeded for every type ---

#[cfg(feature = "std")]
mod std_pipeline {
    use arrayvec::{ArrayString, ArrayVec};
    use byteable::DecodeError;
    use byteable::io::{ReadValue, WriteValue};

    fn roundtrip<
        T: byteable::io::Writable + byteable::io::Readable + PartialEq + core::fmt::Debug,
    >(
        val: T,
    ) {
        let mut buf = std::vec::Vec::new();
        buf.write_value(&val).unwrap();
        let restored: T = std::io::Cursor::new(&buf).read_value().unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn array_vec_roundtrip() {
        let mut v: ArrayVec<u32, 4> = ArrayVec::new();
        v.push(1);
        v.push(2);
        roundtrip(v);
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
            .read_value::<ArrayVec<u32, 4>>()
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

    #[test]
    fn array_string_roundtrip() {
        let s: ArrayString<8> = ArrayString::from("hi").unwrap();
        roundtrip(s);
    }

    #[test]
    fn array_string_capacity_exceeded_is_decode_error() {
        let over = "this is definitely too long".to_string();
        let mut buf = std::vec::Vec::new();
        buf.write_value(&over).unwrap();
        let err = std::io::Cursor::new(&buf)
            .read_value::<ArrayString<4>>()
            .unwrap_err();
        assert!(matches!(
            err,
            byteable::io::ReadableError::DecodeError(DecodeError::CapacityExceeded {
                capacity: 4,
                ..
            })
        ));
    }

    #[test]
    fn array_string_invalid_utf8_is_decode_error() {
        // Valid on the wire (fits capacity) but not valid UTF-8.
        let bytes: std::vec::Vec<u8> = std::vec![0xFF, 0xFE];
        let mut buf = std::vec::Vec::new();
        buf.write_value(&bytes).unwrap();
        let err = std::io::Cursor::new(&buf)
            .read_value::<ArrayString<4>>()
            .unwrap_err();
        assert!(matches!(
            err,
            byteable::io::ReadableError::DecodeError(DecodeError::InvalidUtf8)
        ));
    }
}

// --- pipeline parity: same `ArrayVec<T, N>` round-trip over tokio / embedded-io / embedded-io-async ---

#[cfg(feature = "tokio")]
#[tokio::test]
async fn tokio_pipeline_roundtrip() {
    use arrayvec::ArrayVec;
    use byteable::async_io::{AsyncReadValue, AsyncWriteValue};

    let mut v: ArrayVec<u32, 4> = ArrayVec::new();
    v.push(1);
    v.push(2);
    let mut buf = std::vec::Vec::new();
    buf.write_value(&v).await.unwrap();
    let restored: ArrayVec<u32, 4> = std::io::Cursor::new(&buf).read_value().await.unwrap();
    assert_eq!(v, restored);
}

#[cfg(feature = "embedded-io")]
#[test]
fn embedded_io_pipeline_roundtrip() {
    use arrayvec::ArrayVec;
    use byteable::eio::{EioReadValue, EioWriteValue};

    let mut v: ArrayVec<u32, 4> = ArrayVec::new();
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
    let restored: ArrayVec<u32, 4> = r.read_value().unwrap();
    assert_eq!(v, restored);
}

#[cfg(feature = "embedded-io-async")]
#[tokio::test]
async fn embedded_io_async_pipeline_roundtrip() {
    use arrayvec::ArrayVec;
    use byteable::eio_async::{EioAsyncReadValue, EioAsyncWriteValue};

    let mut v: ArrayVec<u32, 4> = ArrayVec::new();
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
    let restored: ArrayVec<u32, 4> = r.read_value().await.unwrap();
    assert_eq!(v, restored);
}

// --- derive-macro `io_only` struct field usage ---

#[cfg(all(feature = "std", feature = "derive"))]
mod derive_field {
    use arrayvec::{ArrayString, ArrayVec};
    use byteable::Byteable;
    use byteable::io::{ReadValue, WriteValue};

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct Sensor {
        id: u32,
        label: ArrayString<16>,
        readings: ArrayVec<u32, 8>,
    }

    #[test]
    fn derived_io_only_struct_with_arrayvec_fields_roundtrip() {
        let mut readings: ArrayVec<u32, 8> = ArrayVec::new();
        readings.push(1);
        readings.push(2);
        let s = Sensor {
            id: 7,
            label: ArrayString::from("temp").unwrap(),
            readings,
        };
        let mut buf = std::vec::Vec::new();
        buf.write_value(&s).unwrap();
        let restored: Sensor = std::io::Cursor::new(&buf).read_value().unwrap();
        assert_eq!(s, restored);
    }
}
