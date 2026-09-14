//! Integration tests for the `eio` (embedded-io) support module.
#![cfg(all(feature = "embedded-io", feature = "derive"))]

mod fixed {
    use byteable::eio::{EioReadFixed, EioWriteFixed};
    use byteable::{Byteable, LittleEndian};

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    struct Header {
        #[byteable(big_endian)]
        magic: u32,
        #[byteable(little_endian)]
        version: u16,
    }

    #[test]
    fn primitive_roundtrip_over_slice() {
        let mut buf = [0u8; 4];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_fixed(&42u32).unwrap();
        }
        let mut r: &[u8] = &buf;
        let val: u32 = r.read_fixed().unwrap();
        assert_eq!(val, 42u32);
    }

    #[test]
    fn endian_wrapper_roundtrip_over_slice() {
        let original: LittleEndian<u32> = LittleEndian::new(0xDEADBEEF);
        let mut buf = [0u8; 4];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_fixed(&original).unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: LittleEndian<u32> = r.read_fixed().unwrap();
        assert_eq!(restored.get(), original.get());
    }

    #[test]
    fn derived_struct_roundtrip_over_slice() {
        let header = Header {
            magic: 0x1234_5678,
            version: 7,
        };
        let mut buf = [0u8; 6];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_fixed(&header).unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: Header = r.read_fixed().unwrap();
        assert_eq!(restored, header);
    }

    #[test]
    fn read_exact_error_on_truncated_slice() {
        use byteable::eio::EioReadableError;
        let buf = [0u8; 2]; // too short for a u32
        let mut r: &[u8] = &buf;
        let err = r.read_fixed::<u32>().unwrap_err();
        assert!(matches!(err, EioReadableError::UnexpectedEof));
    }
}

mod dynamic_over_fixed {
    use byteable::Byteable;
    use byteable::eio::{EioReadValue, EioWriteValue};

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[test]
    fn read_value_write_value_roundtrip_fixed_type() {
        let p = Point { x: -5, y: 100 };
        let mut buf = [0u8; 8];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_value(&p).unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: Point = r.read_value().unwrap();
        assert_eq!(restored, p);
    }
}

mod option_result {
    use byteable::eio::{EioReadValue, EioWriteValue};

    #[test]
    fn option_some_roundtrip() {
        let val: Option<u32> = Some(7);
        let mut buf = [0u8; 5];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_value(&val).unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: Option<u32> = r.read_value().unwrap();
        assert_eq!(restored, val);
    }

    #[test]
    fn option_none_roundtrip() {
        let val: Option<u32> = None;
        let mut buf = [0u8; 5];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_value(&val).unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: Option<u32> = r.read_value().unwrap();
        assert_eq!(restored, val);
    }

    #[test]
    fn result_roundtrip() {
        let val: Result<u8, u8> = Ok(9);
        let mut buf = [0u8; 2];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_value(&val).unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: Result<u8, u8> = r.read_value().unwrap();
        assert_eq!(restored, val);
    }
}

mod borrowed_write {
    use byteable::eio::EioWriteValue;

    #[test]
    fn write_u8_slice_no_alloc() {
        let data: &[u8] = &[1, 2, 3];
        let mut buf = [0u8; 11];
        let mut w: &mut [u8] = &mut buf;
        w.write_value(data).unwrap();
        assert_eq!(&buf[..8], &3u64.to_le_bytes());
        assert_eq!(&buf[8..11], &[1, 2, 3]);
    }

    #[test]
    fn write_str_no_alloc() {
        let s: &str = "hi";
        let mut buf = [0u8; 10];
        let mut w: &mut [u8] = &mut buf;
        w.write_value(s).unwrap();
        assert_eq!(&buf[..8], &2u64.to_le_bytes());
        assert_eq!(&buf[8..10], b"hi");
    }
}

mod dynamic_struct {
    use byteable::Byteable;
    use byteable::eio::{EioReadValue, EioWriteValue};

    #[derive(Byteable, Debug, PartialEq)]
    #[byteable(io_only)]
    struct Message {
        id: u32,
        flag: Option<u8>,
    }

    #[test]
    fn io_only_struct_roundtrip_over_eio() {
        let msg = Message {
            id: 42,
            flag: Some(1),
        };
        let mut buf = [0u8; 6];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_value(&msg).unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: Message = r.read_value().unwrap();
        assert_eq!(restored, msg);
    }

    #[test]
    fn io_only_struct_roundtrip_none_flag() {
        let msg = Message { id: 1, flag: None };
        let mut buf = [0u8; 5];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_value(&msg).unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: Message = r.read_value().unwrap();
        assert_eq!(restored, msg);
    }
}

mod field_enum {
    use byteable::Byteable;
    use byteable::eio::{EioReadValue, EioWriteValue};

    #[derive(Byteable, Debug, PartialEq)]
    enum Shape {
        Circle { radius: f32 },
        Rect { width: f32, height: f32 },
    }

    #[test]
    fn field_enum_roundtrip_over_eio() {
        let s = Shape::Circle { radius: 3.0 };
        let mut buf = [0u8; 5];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_value(&s).unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: Shape = r.read_value().unwrap();
        assert_eq!(restored, s);
    }

    #[test]
    fn field_enum_second_variant_roundtrip() {
        let s = Shape::Rect {
            width: 2.0,
            height: 4.0,
        };
        let mut buf = [0u8; 9];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_value(&s).unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: Shape = r.read_value().unwrap();
        assert_eq!(restored, s);
    }

    #[derive(Byteable, Debug, PartialEq)]
    enum Command {
        Nothing,
        Read(u32, u64),
        Write { a: u32, b: u64 },
    }

    #[test]
    fn tuple_and_named_variants_roundtrip_over_eio() {
        for cmd in [
            Command::Nothing,
            Command::Read(1, 2),
            Command::Write { a: 3, b: 4 },
        ] {
            let mut buf = [0u8; 32];
            {
                let mut w: &mut [u8] = &mut buf;
                w.write_value(&cmd).unwrap();
            }
            let mut r: &[u8] = &buf;
            let restored: Command = r.read_value().unwrap();
            assert_eq!(restored, cmd);
        }
    }
}

#[cfg(feature = "alloc")]
mod alloc_collections {
    extern crate alloc;

    use alloc::string::String;
    use alloc::vec::Vec;
    use byteable::eio::{EioReadValue, EioWriteValue};

    #[test]
    fn vec_roundtrip() {
        let v: Vec<u32> = alloc::vec![1, 2, 3];
        let mut buf = [0u8; 20];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_value(&v).unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: Vec<u32> = r.read_value().unwrap();
        assert_eq!(restored, v);
    }

    #[test]
    fn string_roundtrip() {
        let s: String = String::from("hello");
        let mut buf = [0u8; 13];
        {
            let mut w: &mut [u8] = &mut buf;
            w.write_value(&s).unwrap();
        }
        let mut r: &[u8] = &buf;
        let restored: String = r.read_value().unwrap();
        assert_eq!(restored, s);
    }

    #[test]
    fn string_invalid_utf8_is_decode_error() {
        use byteable::DecodeError;
        use byteable::eio::EioReadableError;
        let mut buf = [0u8; 9];
        buf[..8].copy_from_slice(&1u64.to_le_bytes());
        buf[8] = 0xFF; // invalid UTF-8 byte
        let mut r: &[u8] = &buf;
        let err = r.read_value::<String>().unwrap_err();
        assert!(matches!(
            err,
            EioReadableError::DecodeError(DecodeError::InvalidUtf8)
        ));
    }
}
