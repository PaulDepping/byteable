//! Tests for `ByteRepr`/`IntoByteArray`/`TryFromByteArray` implementations on
//! standard-library and primitive types: numeric primitives, arrays, endian
//! wrappers, `PhantomData`, `u128`/`i128`, `NonZero*`, network types,
//! `Duration`, `SystemTime`, range types, `bool`, and `char`.

use byteable::{BigEndian, FromByteArray, IntoByteArray, LittleEndian, TryFromByteArray};
use core::marker::PhantomData;
use core::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
use core::num::{NonZeroI32, NonZeroU8, NonZeroU32, NonZeroU64};
use core::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use core::time::Duration;

// ── Primitive byte sizes ──────────────────────────────────────────────────────

#[test]
fn primitive_byte_sizes() {
    assert_eq!(u8::BYTE_SIZE, 1);
    assert_eq!(u16::BYTE_SIZE, 2);
    assert_eq!(u32::BYTE_SIZE, 4);
    assert_eq!(u64::BYTE_SIZE, 8);
    assert_eq!(u128::BYTE_SIZE, 16);
    assert_eq!(i8::BYTE_SIZE, 1);
    assert_eq!(i16::BYTE_SIZE, 2);
    assert_eq!(i32::BYTE_SIZE, 4);
    assert_eq!(i64::BYTE_SIZE, 8);
    assert_eq!(i128::BYTE_SIZE, 16);
    assert_eq!(f32::BYTE_SIZE, 4);
    assert_eq!(f64::BYTE_SIZE, 8);
}

// ── Endian wrappers ───────────────────────────────────────────────────────────

#[test]
fn endian_wrapper_byte_sizes() {
    assert_eq!(BigEndian::<u16>::BYTE_SIZE, 2);
    assert_eq!(LittleEndian::<u16>::BYTE_SIZE, 2);
    assert_eq!(BigEndian::<u64>::BYTE_SIZE, 8);
    assert_eq!(LittleEndian::<u64>::BYTE_SIZE, 8);
}

// ── PhantomData ───────────────────────────────────────────────────────────────

#[test]
fn phantom_data_byte_size() {
    assert_eq!(PhantomData::<u32>::BYTE_SIZE, 0);
    assert_eq!(PhantomData::<String>::BYTE_SIZE, 0);
}

#[test]
fn phantom_data_roundtrip() {
    let original: PhantomData<u64> = PhantomData;
    let bytes = original.into_byte_array();
    assert_eq!(bytes, [0u8; 0]);
    let _restored = PhantomData::<u64>::from_byte_array(bytes);
}

// ── u128 / i128 primitives ────────────────────────────────────────────────────

#[test]
fn u128_roundtrip() {
    for val in [
        0u128,
        1,
        u128::MAX,
        0x0102030405060708090A0B0C0D0E0F10,
        u128::MAX / 2,
    ] {
        let bytes = val.into_byte_array();
        assert_eq!(u128::from_byte_array(bytes), val);
    }
}

#[test]
fn u128_byte_layout_is_native_endian() {
    assert_eq!(1u128.into_byte_array(), 1u128.to_ne_bytes());
}

#[test]
fn i128_roundtrip() {
    for val in [0i128, 1, -1, i128::MAX, i128::MIN] {
        let bytes = val.into_byte_array();
        assert_eq!(i128::from_byte_array(bytes), val);
    }
}

#[test]
fn i128_large_negative() {
    let val: i128 = -1_000_000_000_000_000_000_000_000_000;
    let bytes = val.into_byte_array();
    assert_eq!(i128::from_byte_array(bytes), val);
}

// ── NonZero types ─────────────────────────────────────────────────────────────

#[test]
fn nonzero_u8_roundtrip() {
    let original = NonZeroU8::new(42).unwrap();
    let bytes = original.into_byte_array();
    assert_eq!(NonZeroU8::try_from_byte_array(bytes).unwrap(), original);
}

#[test]
fn nonzero_u32_roundtrip() {
    let original = NonZeroU32::new(0xDEADBEEF).unwrap();
    let bytes = original.into_byte_array();
    assert_eq!(NonZeroU32::try_from_byte_array(bytes).unwrap(), original);
}

#[test]
fn nonzero_u64_roundtrip() {
    let original = NonZeroU64::new(u64::MAX).unwrap();
    let bytes = original.into_byte_array();
    assert_eq!(NonZeroU64::try_from_byte_array(bytes).unwrap(), original);
}

#[test]
fn nonzero_i32_roundtrip() {
    let original = NonZeroI32::new(-1).unwrap();
    let bytes = original.into_byte_array();
    assert_eq!(NonZeroI32::try_from_byte_array(bytes).unwrap(), original);
}

#[test]
fn nonzero_zero_is_err() {
    assert!(NonZeroU8::try_from_byte_array([0]).is_err());
    assert!(NonZeroU32::try_from_byte_array([0, 0, 0, 0]).is_err());
}

// ── Network types ─────────────────────────────────────────────────────────────

#[test]
fn ipv4_addr_roundtrip() {
    let original = Ipv4Addr::new(192, 168, 1, 100);
    assert_eq!(
        Ipv4Addr::from_byte_array(original.into_byte_array()),
        original
    );
}

#[test]
fn ipv4_addr_byte_layout() {
    assert_eq!(Ipv4Addr::new(10, 0, 0, 1).into_byte_array(), [10, 0, 0, 1]);
}

#[test]
fn ipv6_addr_roundtrip() {
    let original = Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 1);
    assert_eq!(
        Ipv6Addr::from_byte_array(original.into_byte_array()),
        original
    );
}

#[test]
fn socket_addr_v4_roundtrip() {
    let original = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080);
    let restored = SocketAddrV4::from_byte_array(original.into_byte_array());
    assert_eq!(original.ip(), restored.ip());
    assert_eq!(original.port(), restored.port());
}

#[test]
fn socket_addr_v6_roundtrip() {
    let original = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 443, 0, 0);
    let restored = SocketAddrV6::from_byte_array(original.into_byte_array());
    assert_eq!(original.ip(), restored.ip());
    assert_eq!(original.port(), restored.port());
    assert_eq!(original.flowinfo(), restored.flowinfo());
    assert_eq!(original.scope_id(), restored.scope_id());
}

// ── Duration ──────────────────────────────────────────────────────────────────

#[test]
fn duration_byte_size() {
    assert_eq!(Duration::BYTE_SIZE, 12); // u64 secs (8) + u32 nanos (4)
}

#[test]
fn duration_roundtrip() {
    for d in [
        Duration::ZERO,
        Duration::from_secs(3600),
        Duration::new(1, 500_000_000),
    ] {
        assert_eq!(Duration::from_byte_array(d.into_byte_array()), d);
    }
}

// ── SystemTime ────────────────────────────────────────────────────────────────

#[cfg(feature = "std")]
mod system_time_tests {
    use byteable::{FromByteArray, IntoByteArray};
    use std::time::{Duration, SystemTime};

    #[test]
    fn system_time_byte_size() {
        assert_eq!(SystemTime::BYTE_SIZE, 12); // i64 secs (8) + u32 nanos (4)
    }

    #[test]
    fn system_time_roundtrip() {
        for t in [
            SystemTime::UNIX_EPOCH,
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000),
            SystemTime::UNIX_EPOCH - Duration::from_secs(86400),
            SystemTime::UNIX_EPOCH - Duration::from_millis(500), // nanos-carry path
        ] {
            assert_eq!(SystemTime::from_byte_array(t.into_byte_array()), t);
        }
    }
}

// ── Range types ───────────────────────────────────────────────────────────────

#[test]
fn range_roundtrips() {
    let r: Range<u8> = 10..200;
    assert_eq!(Range::<u8>::from_byte_array(r.clone().into_byte_array()), r);

    let r: Range<u32> = 0..0xDEAD_BEEF;
    assert_eq!(
        Range::<u32>::from_byte_array(r.clone().into_byte_array()),
        r
    );

    let r: RangeInclusive<u32> = 1..=0xFFFF_FFFF;
    assert_eq!(
        RangeInclusive::<u32>::from_byte_array(r.clone().into_byte_array()),
        r
    );
}

#[test]
fn range_from_roundtrip() {
    let r: RangeFrom<u32> = 42..;
    let restored = RangeFrom::<u32>::from_byte_array(r.clone().into_byte_array());
    assert_eq!(r.start, restored.start);
}

#[test]
fn range_to_roundtrips() {
    let r: RangeTo<u32> = ..999;
    assert_eq!(
        RangeTo::<u32>::from_byte_array(r.into_byte_array()).end,
        999
    );

    let r: RangeToInclusive<u32> = ..=1000;
    assert_eq!(
        RangeToInclusive::<u32>::from_byte_array(r.into_byte_array()).end,
        1000
    );
}

#[test]
fn range_full_is_zero_sized() {
    assert_eq!(RangeFull::BYTE_SIZE, 0);
    let bytes = RangeFull.into_byte_array();
    assert_eq!(bytes, [0u8; 0]);
    let _restored = RangeFull::from_byte_array(bytes);
}

// ── bool ─────────────────────────────────────────────────────────────────────

#[test]
fn bool_byte_size() {
    assert_eq!(bool::BYTE_SIZE, 1);
}

#[test]
fn bool_roundtrip() {
    assert_eq!(true.into_byte_array(), [1]);
    assert_eq!(false.into_byte_array(), [0]);
    assert_eq!(bool::try_from_byte_array([1]).unwrap(), true);
    assert_eq!(bool::try_from_byte_array([0]).unwrap(), false);
}

#[test]
fn bool_invalid_bytes_are_err() {
    for invalid in [2u8, 3, 10, 42, 100, 255] {
        let _err = bool::try_from_byte_array([invalid]).unwrap_err();
    }
}

#[cfg(feature = "std")]
#[test]
fn bool_error_implements_std_error() {
    use std::error::Error;
    let err = bool::try_from_byte_array([2]).unwrap_err();
    let _: &dyn Error = &err;
}

// ── char ─────────────────────────────────────────────────────────────────────

#[test]
fn char_byte_size() {
    assert_eq!(char::BYTE_SIZE, 4);
}

#[test]
fn char_ascii_byte_layout() {
    // 'A' = U+0041, stored as little-endian u32
    assert_eq!('A'.into_byte_array(), [0x41, 0x00, 0x00, 0x00]);
}

#[test]
fn char_roundtrip() {
    let chars = [
        'A',
        'z',
        '0',
        '9',
        ' ',
        '!',
        '\0',
        '\n',
        '€',
        '£',
        '¥',
        '©',
        'α',
        'β',
        'γ',
        'π',
        'Σ',
        '中',
        '日',
        '本',
        '語',
        '🦀',
        '🚀',
        '🌟',
        '❤',
        '✓',
        '\u{10FFFF}',
    ];
    for c in chars {
        let bytes = c.into_byte_array();
        assert_eq!(char::try_from_byte_array(bytes).unwrap(), c);
    }
}

#[test]
fn char_specific_byte_layouts() {
    // '🦀' = U+1F980 in little-endian u32
    assert_eq!('🦀'.into_byte_array(), [0x80, 0xF9, 0x01, 0x00]);
    // '€' = U+20AC
    assert_eq!('€'.into_byte_array(), [0xAC, 0x20, 0x00, 0x00]);
    // '\u{10FFFF}' = max valid codepoint
    assert_eq!('\u{10FFFF}'.into_byte_array(), [0xFF, 0xFF, 0x10, 0x00]);
}

#[test]
fn char_invalid_codepoints_are_err() {
    let invalid = [
        [0x00, 0xD8, 0x00, 0x00], // U+D800 — surrogate
        [0xFF, 0xDF, 0x00, 0x00], // U+DFFF — surrogate
        [0x00, 0x00, 0x11, 0x00], // U+110000 — out of range
        [0xFF, 0xFF, 0xFF, 0xFF], // 0xFFFFFFFF — out of range
    ];
    for bytes in invalid {
        let _ = char::try_from_byte_array(bytes).expect_err("expected error for {bytes:?}");
    }
}

#[cfg(feature = "std")]
#[test]
fn char_error_implements_std_error() {
    use std::error::Error;
    let err = char::try_from_byte_array([0xFF, 0xFF, 0xFF, 0xFF]).unwrap_err();
    let _: &dyn Error = &err;
}

// ── Derive-macro integration ──────────────────────────────────────────────────

#[cfg(feature = "derive")]
mod derive_std_types {
    use byteable::{Byteable, IntoByteArray, TryFromByteArray};

    // ── bool in a derived struct ──────────────────────────────────────────

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    struct FlagPacket {
        #[byteable(try_transparent)]
        enabled: bool,
        value: u8,
        #[byteable(try_transparent)]
        ready: bool,
    }

    #[test]
    fn flag_packet_byte_size() {
        assert_eq!(FlagPacket::BYTE_SIZE, 3); // bool(1) + u8(1) + bool(1)
    }

    #[test]
    fn flag_packet_roundtrip() {
        for (enabled, value, ready) in [(true, 42u8, false), (false, 0, true), (true, 255, true)] {
            let p = FlagPacket {
                enabled,
                value,
                ready,
            };
            let bytes = p.into_byte_array();
            assert_eq!(bytes[0], enabled as u8);
            assert_eq!(bytes[1], value);
            assert_eq!(bytes[2], ready as u8);
            assert_eq!(FlagPacket::try_from_byte_array(bytes).unwrap(), p);
        }
    }

    #[test]
    fn flag_packet_invalid_bool_first_field() {
        let _err = FlagPacket::try_from_byte_array([2, 0, 0]).unwrap_err();
    }

    #[test]
    fn flag_packet_invalid_bool_second_field() {
        let _err = FlagPacket::try_from_byte_array([1, 42, 200]).unwrap_err();
    }

    // ── char in a derived struct ──────────────────────────────────────────

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    struct CharRecord {
        id: u8,
        #[byteable(try_transparent)]
        symbol: char,
        #[byteable(little_endian)]
        count: u16,
    }

    #[test]
    fn char_record_byte_size() {
        assert_eq!(CharRecord::BYTE_SIZE, 7); // u8(1) + char(4) + u16(2)
    }

    #[test]
    fn char_record_roundtrip() {
        for (id, symbol, count) in [(1u8, 'A', 0u16), (255, '🦀', 1000), (0, '€', 65535)] {
            let r = CharRecord { id, symbol, count };
            assert_eq!(
                CharRecord::try_from_byte_array(r.into_byte_array()).unwrap(),
                r
            );
        }
    }

    #[test]
    fn char_record_byte_layout() {
        let r = CharRecord {
            id: 7,
            symbol: 'A',
            count: 0x0102,
        };
        let bytes = r.into_byte_array();
        assert_eq!(bytes[0], 7);
        assert_eq!(&bytes[1..5], &[0x41, 0x00, 0x00, 0x00]); // 'A' LE u32
        assert_eq!(&bytes[5..7], &[0x02, 0x01]); // 0x0102 LE
    }

    #[test]
    fn char_record_invalid_codepoint() {
        let mut bytes = [0u8; 7];
        bytes[1..5].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
        assert!(CharRecord::try_from_byte_array(bytes).is_err());
    }

    // ── bool + char combined ──────────────────────────────────────────────

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    struct Annotation {
        #[byteable(try_transparent)]
        active: bool,
        #[byteable(try_transparent)]
        label: char,
        #[byteable(big_endian)]
        tag: u32,
    }

    #[test]
    fn annotation_byte_size() {
        assert_eq!(Annotation::BYTE_SIZE, 9); // bool(1) + char(4) + u32(4)
    }

    #[test]
    fn annotation_roundtrip() {
        let a = Annotation {
            active: true,
            label: '✓',
            tag: 0xDEADBEEF,
        };
        assert_eq!(
            Annotation::try_from_byte_array(a.into_byte_array()).unwrap(),
            a
        );
    }

    #[test]
    fn annotation_byte_layout() {
        let a = Annotation {
            active: false,
            label: 'Z',
            tag: 0x01020304,
        };
        let bytes = a.into_byte_array();
        assert_eq!(bytes[0], 0); // false
        assert_eq!(&bytes[1..5], &[0x5A, 0x00, 0x00, 0x00]); // 'Z' = U+005A LE
        assert_eq!(&bytes[5..9], &[0x01, 0x02, 0x03, 0x04]); // BE u32
    }

    #[test]
    fn annotation_invalid_bool() {
        let mut bytes = [0u8; 9];
        bytes[0] = 5;
        assert!(Annotation::try_from_byte_array(bytes).is_err());
    }

    #[test]
    fn annotation_invalid_char() {
        let mut bytes = [0u8; 9];
        bytes[0] = 1;
        bytes[1..5].copy_from_slice(&[0x00, 0x00, 0x11, 0x00]); // U+110000 — invalid
        assert!(Annotation::try_from_byte_array(bytes).is_err());
    }

    // ── bool + char + u8 mixed ────────────────────────────────────────────

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    struct MixedPrimitives {
        #[byteable(try_transparent)]
        is_active: bool,
        #[byteable(try_transparent)]
        symbol: char,
        count: u8,
        #[byteable(try_transparent)]
        enabled: bool,
    }

    #[test]
    fn mixed_primitives_roundtrip() {
        let s = MixedPrimitives {
            is_active: true,
            symbol: '✓',
            count: 5,
            enabled: false,
        };
        let bytes = s.into_byte_array();
        assert_eq!(bytes.len(), 7); // bool(1) + char(4) + u8(1) + bool(1)
        assert_eq!(MixedPrimitives::try_from_byte_array(bytes).unwrap(), s);
    }
}
