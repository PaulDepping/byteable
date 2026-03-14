//! Tests for `ByteRepr`/`IntoByteArray`/`TryFromByteArray` implementations on
//! standard-library types: primitives, arrays, `PhantomData`, `NonZero*`, network
//! types, `Duration`, `SystemTime`, and range types.

use byteable::{BigEndian, ByteRepr, DiscriminantValue, FromByteArray, IntoByteArray, LittleEndian, TryFromByteArray};
use core::marker::PhantomData;
use core::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
use core::num::{NonZeroI32, NonZeroU8, NonZeroU32, NonZeroU64};
use core::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use core::time::Duration;

// ── BYTE_SIZE constants ───────────────────────────────────────────────────────

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

#[test]
fn array_byte_sizes() {
    assert_eq!(<[u8; 0]>::BYTE_SIZE, 0);
    assert_eq!(<[u8; 4]>::BYTE_SIZE, 4);
    assert_eq!(<[u32; 4]>::BYTE_SIZE, 16);
    assert_eq!(<[u8; 16]>::BYTE_SIZE, 16);
}

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
    let restored = PhantomData::<u64>::from_byte_array(bytes);
    let _ = restored; // zero-sized; equality not meaningful, just verify it compiles
}

// ── Arrays of non-u8 types ────────────────────────────────────────────────────

#[test]
fn u32_array_roundtrip() {
    let original: [u32; 4] = [0xDEAD, 0xBEEF, 0xCAFE, 0xBABE];
    let bytes = original.into_byte_array();
    let restored = <[u32; 4]>::from_byte_array(bytes);
    assert_eq!(original, restored);
}

#[test]
fn u8_array_roundtrip() {
    let original: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let bytes = original.into_byte_array();
    let restored = <[u8; 8]>::from_byte_array(bytes);
    assert_eq!(original, restored);
}

#[test]
fn nested_array_roundtrip() {
    let original: [[u8; 2]; 3] = [[0x11, 0x22], [0x33, 0x44], [0x55, 0x66]];
    let bytes = original.into_byte_array();
    let restored = <[[u8; 2]; 3]>::from_byte_array(bytes);
    assert_eq!(original, restored);
}

// ── NonZero types ─────────────────────────────────────────────────────────────

#[test]
fn nonzero_u8_roundtrip() {
    let original = NonZeroU8::new(42).unwrap();
    let bytes = original.into_byte_array();
    let restored = NonZeroU8::try_from_byte_array(bytes).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn nonzero_u32_roundtrip() {
    let original = NonZeroU32::new(0xDEADBEEF).unwrap();
    let bytes = original.into_byte_array();
    let restored = NonZeroU32::try_from_byte_array(bytes).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn nonzero_u64_roundtrip() {
    let original = NonZeroU64::new(u64::MAX).unwrap();
    let bytes = original.into_byte_array();
    let restored = NonZeroU64::try_from_byte_array(bytes).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn nonzero_i32_roundtrip() {
    let original = NonZeroI32::new(-1).unwrap();
    let bytes = original.into_byte_array();
    let restored = NonZeroI32::try_from_byte_array(bytes).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn nonzero_u8_zero_is_err() {
    let result = NonZeroU8::try_from_byte_array([0]);
    assert!(result.is_err());
}

#[test]
fn nonzero_u32_zero_is_err() {
    let result = NonZeroU32::try_from_byte_array([0, 0, 0, 0]);
    assert!(result.is_err());
}

// ── Network types ─────────────────────────────────────────────────────────────

#[test]
fn ipv4_addr_roundtrip() {
    let original = Ipv4Addr::new(192, 168, 1, 100);
    let bytes = original.into_byte_array();
    let restored = Ipv4Addr::from_byte_array(bytes);
    assert_eq!(original, restored);
}

#[test]
fn ipv4_addr_byte_layout() {
    let addr = Ipv4Addr::new(10, 0, 0, 1);
    let bytes = addr.into_byte_array();
    assert_eq!(bytes, [10, 0, 0, 1]);
}

#[test]
fn ipv6_addr_roundtrip() {
    let original = Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 1);
    let bytes = original.into_byte_array();
    let restored = Ipv6Addr::from_byte_array(bytes);
    assert_eq!(original, restored);
}

#[test]
fn socket_addr_v4_roundtrip() {
    let ip = Ipv4Addr::new(127, 0, 0, 1);
    let original = SocketAddrV4::new(ip, 8080);
    let bytes = original.into_byte_array();
    let restored = SocketAddrV4::from_byte_array(bytes);
    assert_eq!(original.ip(), restored.ip());
    assert_eq!(original.port(), restored.port());
}

#[test]
fn socket_addr_v6_roundtrip() {
    let ip = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);
    let original = SocketAddrV6::new(ip, 443, 0, 0);
    let bytes = original.into_byte_array();
    let restored = SocketAddrV6::from_byte_array(bytes);
    assert_eq!(original.ip(), restored.ip());
    assert_eq!(original.port(), restored.port());
    assert_eq!(original.flowinfo(), restored.flowinfo());
    assert_eq!(original.scope_id(), restored.scope_id());
}

// ── Duration ──────────────────────────────────────────────────────────────────

#[test]
fn duration_byte_size() {
    // u64 secs (8) + u32 nanos (4) = 12
    assert_eq!(Duration::BYTE_SIZE, 12);
}

#[test]
fn duration_zero_roundtrip() {
    let original = Duration::ZERO;
    let bytes = original.into_byte_array();
    let restored = Duration::from_byte_array(bytes);
    assert_eq!(original, restored);
}

#[test]
fn duration_roundtrip() {
    let original = Duration::from_secs(3600);
    let bytes = original.into_byte_array();
    let restored = Duration::from_byte_array(bytes);
    assert_eq!(original, restored);
}

#[test]
fn duration_with_nanos_roundtrip() {
    let original = Duration::new(1, 500_000_000);
    let bytes = original.into_byte_array();
    let restored = Duration::from_byte_array(bytes);
    assert_eq!(original, restored);
}

// ── SystemTime ────────────────────────────────────────────────────────────────

#[cfg(feature = "std")]
mod system_time_tests {
    use byteable::{ByteRepr, FromByteArray, IntoByteArray};
    use std::time::{Duration, SystemTime};

    #[test]
    fn system_time_byte_size() {
        // i64 secs (8) + u32 nanos (4) = 12
        assert_eq!(SystemTime::BYTE_SIZE, 12);
    }

    #[test]
    fn system_time_unix_epoch_roundtrip() {
        let original = SystemTime::UNIX_EPOCH;
        let bytes = original.into_byte_array();
        let restored = SystemTime::from_byte_array(bytes);
        assert_eq!(original, restored);
    }

    #[test]
    fn system_time_after_epoch_roundtrip() {
        let original = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let bytes = original.into_byte_array();
        let restored = SystemTime::from_byte_array(bytes);
        assert_eq!(original, restored);
    }

    #[test]
    fn system_time_before_epoch_roundtrip() {
        let original = SystemTime::UNIX_EPOCH - Duration::from_secs(86400);
        let bytes = original.into_byte_array();
        let restored = SystemTime::from_byte_array(bytes);
        assert_eq!(original, restored);
    }

    #[test]
    fn system_time_sub_second_before_epoch_roundtrip() {
        // 0.5s before epoch — exercises the nanos-carry path
        let original = SystemTime::UNIX_EPOCH - Duration::from_millis(500);
        let bytes = original.into_byte_array();
        let restored = SystemTime::from_byte_array(bytes);
        assert_eq!(original, restored);
    }
}

// ── Range types ───────────────────────────────────────────────────────────────

#[test]
fn range_u8_roundtrip() {
    let original: Range<u8> = 10..200;
    let bytes = original.clone().into_byte_array();
    let restored = Range::<u8>::from_byte_array(bytes);
    assert_eq!(original, restored);
}

#[test]
fn range_u32_roundtrip() {
    let original: Range<u32> = 0..0xDEAD_BEEF;
    let bytes = original.clone().into_byte_array();
    let restored = Range::<u32>::from_byte_array(bytes);
    assert_eq!(original, restored);
}

#[test]
fn range_inclusive_u32_roundtrip() {
    let original: RangeInclusive<u32> = 1..=0xFFFF_FFFF;
    let bytes = original.clone().into_byte_array();
    let restored = RangeInclusive::<u32>::from_byte_array(bytes);
    assert_eq!(original, restored);
}

#[test]
fn range_from_u32_roundtrip() {
    let original: RangeFrom<u32> = 42..;
    let bytes = original.clone().into_byte_array();
    let restored = RangeFrom::<u32>::from_byte_array(bytes);
    assert_eq!(original.start, restored.start);
}

#[test]
fn range_to_u32_roundtrip() {
    let original: RangeTo<u32> = ..999;
    let bytes = original.into_byte_array();
    let restored = RangeTo::<u32>::from_byte_array(bytes);
    assert_eq!(original.end, restored.end);
}

#[test]
fn range_to_inclusive_u32_roundtrip() {
    let original: RangeToInclusive<u32> = ..=1000;
    let bytes = original.into_byte_array();
    let restored = RangeToInclusive::<u32>::from_byte_array(bytes);
    assert_eq!(original.end, restored.end);
}

#[test]
fn range_full_byte_size() {
    assert_eq!(RangeFull::BYTE_SIZE, 0);
}

#[test]
fn range_full_roundtrip() {
    let original = RangeFull;
    let bytes = original.into_byte_array();
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
        let result = bool::try_from_byte_array([invalid]);
        assert!(result.is_err(), "expected error for byte {invalid}");
        assert_eq!(
            result.unwrap_err().invalid_discriminant,
            DiscriminantValue::U8(invalid)
        );
    }
}

// ── char ─────────────────────────────────────────────────────────────────────

#[test]
fn char_byte_size() {
    assert_eq!(char::BYTE_SIZE, 4);
}

#[test]
fn char_roundtrip() {
    for c in ['A', 'z', '0', '\n', '€', '🦀', '\u{10FFFF}'] {
        let bytes = c.into_byte_array();
        let restored = char::try_from_byte_array(bytes).unwrap();
        assert_eq!(c, restored);
    }
}

#[test]
fn char_ascii_byte_layout() {
    // 'A' = U+0041, stored as little-endian u32
    assert_eq!('A'.into_byte_array(), [0x41, 0x00, 0x00, 0x00]);
}

#[test]
fn char_invalid_codepoints_are_err() {
    let invalid = [
        [0x00, 0xD8, 0x00, 0x00], // U+D800 — surrogate
        [0xFF, 0xDF, 0x00, 0x00], // U+DFFF — surrogate
        [0x00, 0x00, 0x11, 0x00], // U+110000 — out of range
        [0xFF, 0xFF, 0xFF, 0xFF], // 0xFFFF_FFFF — out of range
    ];
    for bytes in invalid {
        assert!(char::try_from_byte_array(bytes).is_err(), "expected error for {bytes:?}");
    }
}

// ── Derive-macro integration ──────────────────────────────────────────────────

/// Tests that structs using std-type fields via `try_transparent` compile and
/// round-trip correctly with the `#[derive(Byteable)]` macro.
#[cfg(feature = "derive")]
mod derive_std_types {
    use byteable::{Byteable, ByteRepr, DiscriminantValue, IntoByteArray, TryFromByteArray};

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
        // bool(1) + u8(1) + bool(1) = 3
        assert_eq!(FlagPacket::BYTE_SIZE, 3);
    }

    #[test]
    fn flag_packet_roundtrip() {
        for (enabled, value, ready) in [(true, 42u8, false), (false, 0, true), (true, 255, true)] {
            let p = FlagPacket { enabled, value, ready };
            let bytes = p.into_byte_array();
            assert_eq!(bytes[0], enabled as u8);
            assert_eq!(bytes[1], value);
            assert_eq!(bytes[2], ready as u8);
            assert_eq!(FlagPacket::try_from_byte_array(bytes).unwrap(), p);
        }
    }

    #[test]
    fn flag_packet_invalid_bool_first_field() {
        let result = FlagPacket::try_from_byte_array([2, 0, 0]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().invalid_discriminant, DiscriminantValue::U8(2));
    }

    #[test]
    fn flag_packet_invalid_bool_second_field() {
        let result = FlagPacket::try_from_byte_array([1, 42, 200]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().invalid_discriminant, DiscriminantValue::U8(200));
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
        // u8(1) + char(4) + u16(2) = 7
        assert_eq!(CharRecord::BYTE_SIZE, 7);
    }

    #[test]
    fn char_record_roundtrip() {
        for (id, symbol, count) in [(1u8, 'A', 0u16), (255, '🦀', 1000), (0, '€', 65535)] {
            let r = CharRecord { id, symbol, count };
            let bytes = r.into_byte_array();
            assert_eq!(CharRecord::try_from_byte_array(bytes).unwrap(), r);
        }
    }

    #[test]
    fn char_record_byte_layout() {
        let r = CharRecord { id: 7, symbol: 'A', count: 0x0102 };
        let bytes = r.into_byte_array();
        assert_eq!(bytes[0], 7);                                     // id
        assert_eq!(&bytes[1..5], &[0x41, 0x00, 0x00, 0x00]);        // 'A' LE u32
        assert_eq!(&bytes[5..7], &[0x02, 0x01]);                     // 0x0102 LE
    }

    #[test]
    fn char_record_invalid_codepoint() {
        let mut bytes = [0u8; 7];
        bytes[0] = 1;
        bytes[1..5].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // invalid char
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
        // bool(1) + char(4) + u32(4) = 9
        assert_eq!(Annotation::BYTE_SIZE, 9);
    }

    #[test]
    fn annotation_roundtrip() {
        let a = Annotation { active: true, label: '✓', tag: 0xDEADBEEF };
        let bytes = a.into_byte_array();
        let restored = Annotation::try_from_byte_array(bytes).unwrap();
        assert_eq!(restored, a);
    }

    #[test]
    fn annotation_byte_layout() {
        let a = Annotation { active: false, label: 'Z', tag: 0x01020304 };
        let bytes = a.into_byte_array();
        assert_eq!(bytes[0], 0);                                     // active = false
        assert_eq!(&bytes[1..5], &[0x5A, 0x00, 0x00, 0x00]);        // 'Z' = U+005A LE
        assert_eq!(&bytes[5..9], &[0x01, 0x02, 0x03, 0x04]);        // 0x01020304 BE
    }

    #[test]
    fn annotation_invalid_bool() {
        let mut bytes = [0u8; 9];
        bytes[0] = 5; // invalid bool
        assert!(Annotation::try_from_byte_array(bytes).is_err());
    }

    #[test]
    fn annotation_invalid_char() {
        let mut bytes = [0u8; 9];
        bytes[0] = 1; // valid bool
        bytes[1..5].copy_from_slice(&[0x00, 0x00, 0x11, 0x00]); // U+110000 — invalid
        assert!(Annotation::try_from_byte_array(bytes).is_err());
    }
}
