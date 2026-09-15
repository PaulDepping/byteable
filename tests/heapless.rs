//! Integration tests for heapless support.
#![cfg(feature = "heapless")]

// --- std (io) pipeline: round-trip + capacity-exceeded for every type ---

#[cfg(feature = "std")]
mod std_pipeline {
    use byteable::DecodeError;
    use byteable::io::{ReadValue, WriteValue};
    use heapless::index_map::FnvIndexMap;
    use heapless::index_set::FnvIndexSet;
    use heapless::{Deque, LinearMap, String, Vec};

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
    fn vec_roundtrip() {
        let mut v: Vec<u32, 4> = Vec::new();
        v.push(1).unwrap();
        v.push(2).unwrap();
        roundtrip(v);
    }

    #[test]
    fn vec_capacity_exceeded_is_decode_error() {
        let mut over: std::vec::Vec<u32> = std::vec::Vec::new();
        for i in 0..5u32 {
            over.push(i);
        }
        let mut buf = std::vec::Vec::new();
        buf.write_value(&over).unwrap();
        let err = std::io::Cursor::new(&buf)
            .read_value::<Vec<u32, 4>>()
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
    fn string_roundtrip() {
        let s: String<8> = String::try_from("hi").unwrap();
        roundtrip(s);
    }

    #[test]
    fn string_capacity_exceeded_is_decode_error() {
        let over = "this is definitely too long".to_string();
        let mut buf = std::vec::Vec::new();
        buf.write_value(&over).unwrap();
        let err = std::io::Cursor::new(&buf)
            .read_value::<String<4>>()
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
    fn deque_roundtrip() {
        let mut d: Deque<u32, 4> = Deque::new();
        d.push_back(1).unwrap();
        d.push_back(2).unwrap();
        let mut buf = std::vec::Vec::new();
        buf.write_value(&d).unwrap();
        let restored: Deque<u32, 4> = std::io::Cursor::new(&buf).read_value().unwrap();
        assert_eq!(
            d.iter().collect::<std::vec::Vec<_>>(),
            restored.iter().collect::<std::vec::Vec<_>>()
        );
    }

    #[test]
    fn deque_capacity_exceeded_is_decode_error() {
        let mut over: std::vec::Vec<u32> = std::vec::Vec::new();
        for i in 0..5u32 {
            over.push(i);
        }
        let mut buf = std::vec::Vec::new();
        buf.write_value(&over).unwrap();
        let err = std::io::Cursor::new(&buf)
            .read_value::<Deque<u32, 4>>()
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
    fn index_map_roundtrip() {
        let mut m: FnvIndexMap<u32, u32, 4> = FnvIndexMap::new();
        m.insert(1, 10).unwrap();
        m.insert(2, 20).unwrap();
        let mut buf = std::vec::Vec::new();
        buf.write_value(&m).unwrap();
        let restored: FnvIndexMap<u32, u32, 4> = std::io::Cursor::new(&buf).read_value().unwrap();
        assert_eq!(m, restored);
    }

    #[test]
    fn index_map_capacity_exceeded_is_decode_error() {
        let mut over: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        for i in 0..5u32 {
            over.insert(i, i);
        }
        let mut buf = std::vec::Vec::new();
        buf.write_value(&over).unwrap();
        let err = std::io::Cursor::new(&buf)
            .read_value::<FnvIndexMap<u32, u32, 4>>()
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
    fn index_set_roundtrip() {
        let mut s: FnvIndexSet<u32, 4> = FnvIndexSet::new();
        s.insert(1).unwrap();
        s.insert(2).unwrap();
        let mut buf = std::vec::Vec::new();
        buf.write_value(&s).unwrap();
        let restored: FnvIndexSet<u32, 4> = std::io::Cursor::new(&buf).read_value().unwrap();
        assert_eq!(s, restored);
    }

    #[test]
    fn linear_map_roundtrip() {
        let mut m: LinearMap<u32, u32, 4> = LinearMap::new();
        m.insert(1, 10).unwrap();
        m.insert(2, 20).unwrap();
        let mut buf = std::vec::Vec::new();
        buf.write_value(&m).unwrap();
        let restored: LinearMap<u32, u32, 4> = std::io::Cursor::new(&buf).read_value().unwrap();
        assert_eq!(m, restored);
    }

    #[test]
    fn linear_map_capacity_exceeded_is_decode_error() {
        let mut over: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        for i in 0..5u32 {
            over.insert(i, i);
        }
        let mut buf = std::vec::Vec::new();
        buf.write_value(&over).unwrap();
        let err = std::io::Cursor::new(&buf)
            .read_value::<LinearMap<u32, u32, 4>>()
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

// --- pipeline parity: same `Vec<T, N>` round-trip over tokio / embedded-io / embedded-io-async ---

#[cfg(feature = "tokio")]
#[tokio::test]
async fn tokio_pipeline_roundtrip() {
    use byteable::async_io::{AsyncReadValue, AsyncWriteValue};
    use heapless::Vec;

    let mut v: Vec<u32, 4> = Vec::new();
    v.push(1).unwrap();
    v.push(2).unwrap();
    let mut buf = std::vec::Vec::new();
    buf.write_value(&v).await.unwrap();
    let restored: Vec<u32, 4> = std::io::Cursor::new(&buf).read_value().await.unwrap();
    assert_eq!(v, restored);
}

#[cfg(feature = "embedded-io")]
#[test]
fn embedded_io_pipeline_roundtrip() {
    use byteable::eio::{EioReadValue, EioWriteValue};
    use heapless::Vec;

    let mut v: Vec<u32, 4> = Vec::new();
    v.push(1).unwrap();
    v.push(2).unwrap();
    let mut buf = [0u8; 64];
    let written;
    {
        let mut w: &mut [u8] = &mut buf;
        let before = w.len();
        w.write_value(&v).unwrap();
        written = before - w.len();
    }
    let mut r: &[u8] = &buf[..written];
    let restored: Vec<u32, 4> = r.read_value().unwrap();
    assert_eq!(v, restored);
}

#[cfg(feature = "embedded-io-async")]
#[tokio::test]
async fn embedded_io_async_pipeline_roundtrip() {
    use byteable::eio_async::{EioAsyncReadValue, EioAsyncWriteValue};
    use heapless::Vec;

    let mut v: Vec<u32, 4> = Vec::new();
    v.push(1).unwrap();
    v.push(2).unwrap();
    let mut buf = [0u8; 64];
    let written;
    {
        let mut w: &mut [u8] = &mut buf;
        let before = w.len();
        w.write_value(&v).await.unwrap();
        written = before - w.len();
    }
    let mut r: &[u8] = &buf[..written];
    let restored: Vec<u32, 4> = r.read_value().await.unwrap();
    assert_eq!(v, restored);
}

// --- derive-macro `io_only` struct field usage ---

#[cfg(all(feature = "std", feature = "derive"))]
mod derive_field {
    use byteable::Byteable;
    use byteable::io::{ReadValue, WriteValue};
    use heapless::{String, Vec};

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct Sensor {
        id: u32,
        label: String<16>,
        readings: Vec<u32, 8>,
    }

    #[test]
    fn derived_io_only_struct_with_heapless_fields_roundtrip() {
        let mut readings: Vec<u32, 8> = Vec::new();
        readings.push(1).unwrap();
        readings.push(2).unwrap();
        let s = Sensor {
            id: 7,
            label: String::try_from("temp").unwrap(),
            readings,
        };
        let mut buf = std::vec::Vec::new();
        buf.write_value(&s).unwrap();
        let restored: Sensor = std::io::Cursor::new(&buf).read_value().unwrap();
        assert_eq!(s, restored);
    }
}
