//! Tests for `#[derive(Byteable)]` on C-like enums.
//!
//! Covers all supported `#[repr]` types (`u8`–`u128`, `i8`–`i64`), explicit
//! and auto-inferred endianness, auto-inferred repr and discriminants, sparse
//! discriminants, and invalid-discriminant error reporting.
#![cfg(feature = "derive")]

use byteable::{Byteable, DecodeError, IntoByteArray, PlainOldData, RawRepr, TryFromByteArray};

// ── u8 repr ───────────────────────────────────────────────────────────────────

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum Status {
    Idle = 0,
    Running = 1,
    Completed = 2,
    Failed = 3,
}

#[test]
fn u8_enum_roundtrip() {
    for (variant, byte) in [
        (Status::Idle, 0u8),
        (Status::Running, 1),
        (Status::Completed, 2),
        (Status::Failed, 3),
    ] {
        assert_eq!(variant.to_raw().as_bytes(), [byte]);
        assert_eq!(Status::try_from_byte_array([byte]).unwrap(), variant);
    }
}

#[test]
fn u8_enum_byte_size() {
    assert_eq!(Status::BYTE_SIZE, 1);
}

#[test]
fn u8_enum_invalid_discriminant() {
    let result = Status::try_from_byte_array([255]);
    assert!(result.is_err());
}

// ── u16 repr with endianness ──────────────────────────────────────────────────

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u16)]
#[byteable(little_endian)]
enum Command {
    Start = 0x1000,
    Stop = 0x2000,
    Pause = 0x3000,
}

#[test]
fn u16_le_enum_byte_layout() {
    assert_eq!(Command::Start.into_byte_array(), [0x00, 0x10]);
    assert_eq!(Command::Stop.into_byte_array(), [0x00, 0x20]);
    assert_eq!(Command::Pause.into_byte_array(), [0x00, 0x30]);
}

#[test]
fn u16_le_enum_roundtrip() {
    assert_eq!(
        Command::try_from_byte_array([0x00, 0x10]).unwrap(),
        Command::Start
    );
    assert_eq!(
        Command::try_from_byte_array([0x00, 0x20]).unwrap(),
        Command::Stop
    );
    assert_eq!(
        Command::try_from_byte_array([0x00, 0x30]).unwrap(),
        Command::Pause
    );
}

#[test]
fn u16_le_enum_invalid_discriminant() {
    assert!(Command::try_from_byte_array(0x9999u16.to_le_bytes()).is_err());
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u16)]
#[byteable(big_endian)]
enum CommandBE {
    Start = 0x1000,
    Stop = 0x2000,
    Pause = 0x3000,
}

#[test]
fn u16_be_enum_byte_layout() {
    assert_eq!(CommandBE::Start.into_byte_array(), [0x10, 0x00]);
    assert_eq!(CommandBE::Stop.into_byte_array(), [0x20, 0x00]);
    assert_eq!(CommandBE::Pause.into_byte_array(), [0x30, 0x00]);
}

#[test]
fn u16_be_enum_roundtrip() {
    assert_eq!(
        CommandBE::try_from_byte_array([0x10, 0x00]).unwrap(),
        CommandBE::Start
    );
    assert_eq!(
        CommandBE::try_from_byte_array([0x20, 0x00]).unwrap(),
        CommandBE::Stop
    );
}

// ── u32 repr ──────────────────────────────────────────────────────────────────

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
#[byteable(little_endian)]
enum NetworkProtocol {
    Tcp = 6,
    Udp = 17,
    Icmp = 1,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
#[byteable(little_endian)]
enum ProtocolLE {
    Tcp = 0x12345678,
    Udp = 0xABCDEF00,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
#[byteable(big_endian)]
enum ProtocolBE {
    Tcp = 0x12345678,
    Udp = 0xABCDEF00,
}

#[test]
fn u32_enum_roundtrip() {
    let bytes = NetworkProtocol::Tcp.into_byte_array();
    assert_eq!(bytes, 6u32.to_le_bytes());
    assert_eq!(
        NetworkProtocol::try_from_byte_array(bytes).unwrap(),
        NetworkProtocol::Tcp
    );
}

#[test]
fn u32_le_enum_byte_layout() {
    assert_eq!(ProtocolLE::Tcp.into_byte_array(), [0x78, 0x56, 0x34, 0x12]);
    assert_eq!(ProtocolLE::Udp.into_byte_array(), [0x00, 0xEF, 0xCD, 0xAB]);
    assert_eq!(
        ProtocolLE::try_from_byte_array([0x78, 0x56, 0x34, 0x12]).unwrap(),
        ProtocolLE::Tcp
    );
}

#[test]
fn u32_be_enum_byte_layout() {
    assert_eq!(ProtocolBE::Tcp.into_byte_array(), [0x12, 0x34, 0x56, 0x78]);
    assert_eq!(ProtocolBE::Udp.into_byte_array(), [0xAB, 0xCD, 0xEF, 0x00]);
    assert_eq!(
        ProtocolBE::try_from_byte_array([0x12, 0x34, 0x56, 0x78]).unwrap(),
        ProtocolBE::Tcp
    );
}

#[test]
fn discriminant_value_u32() {
    let err = NetworkProtocol::try_from_byte_array(0xFFFF_FFFFu32.to_le_bytes()).unwrap_err();
    assert!(matches!(
        err,
        DecodeError::InvalidDiscriminant {
            raw: _,
            type_name: _
        }
    ))
}

// ── u64 repr ──────────────────────────────────────────────────────────────────

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u64)]
#[byteable(little_endian)]
enum LargeValue {
    Small = 1,
    Medium = 1000,
    Large = 1_000_000,
    Huge = 1_000_000_000_000,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u64)]
#[byteable(little_endian)]
enum LargeLE {
    Small = 0x1122334455667788,
    Large = 0xAABBCCDDEEFF0011,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u64)]
#[byteable(big_endian)]
enum LargeBE {
    Small = 0x1122334455667788,
    Large = 0xAABBCCDDEEFF0011,
}

#[test]
fn u64_enum_roundtrip() {
    let bytes = LargeValue::Huge.into_byte_array();
    assert_eq!(bytes, 1_000_000_000_000u64.to_le_bytes());
    assert_eq!(
        LargeValue::try_from_byte_array(bytes).unwrap(),
        LargeValue::Huge
    );
}

#[test]
fn u64_le_enum_byte_layout() {
    assert_eq!(
        LargeLE::Small.into_byte_array(),
        [0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]
    );
    assert_eq!(
        LargeLE::Large.into_byte_array(),
        [0x11, 0x00, 0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA]
    );
    assert_eq!(
        LargeLE::try_from_byte_array([0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]).unwrap(),
        LargeLE::Small
    );
}

#[test]
fn u64_be_enum_byte_layout() {
    assert_eq!(
        LargeBE::Small.into_byte_array(),
        [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]
    );
    assert_eq!(
        LargeBE::Large.into_byte_array(),
        [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11]
    );
    assert_eq!(
        LargeBE::try_from_byte_array([0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]).unwrap(),
        LargeBE::Small
    );
}

#[test]
fn discriminant_value_u64() {
    let err = LargeValue::try_from_byte_array(42u64.to_le_bytes()).unwrap_err();
    assert!(matches!(
        err,
        DecodeError::InvalidDiscriminant {
            raw: _,
            type_name: _
        }
    ))
}

// ── i8 repr ───────────────────────────────────────────────────────────────────

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(i8)]
enum Temperature {
    Cold = -10,
    Cool = 0,
    Warm = 10,
    Hot = 30,
}

#[test]
fn i8_enum_roundtrip() {
    assert_eq!(Temperature::Cold.into_byte_array(), [(-10i8) as u8]);
    assert_eq!(Temperature::Cool.into_byte_array(), [0]);
    assert_eq!(Temperature::Warm.into_byte_array(), [10]);
    assert_eq!(Temperature::Hot.into_byte_array(), [30]);
    assert_eq!(
        Temperature::try_from_byte_array([(-10i8) as u8]).unwrap(),
        Temperature::Cold
    );
}

#[test]
fn discriminant_value_i8() {
    let err = Temperature::try_from_byte_array([5i8 as u8]).unwrap_err();
    assert!(matches!(
        err,
        DecodeError::InvalidDiscriminant {
            raw: _,
            type_name: _
        }
    ))
}

// ── i16 repr ──────────────────────────────────────────────────────────────────

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(i16)]
#[byteable(little_endian)]
enum SignedLE {
    Negative = -1000,
    Zero = 0,
    Positive = 1000,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(i16)]
#[byteable(big_endian)]
enum SignedBE {
    Negative = -1000,
    Zero = 0,
    Positive = 1000,
}

#[test]
fn i16_le_enum_byte_layout() {
    assert_eq!(
        SignedLE::Negative.into_byte_array(),
        (-1000i16).to_le_bytes()
    );
    assert_eq!(SignedLE::Positive.into_byte_array(), 1000i16.to_le_bytes());
    assert_eq!(
        SignedLE::try_from_byte_array((-1000i16).to_le_bytes()).unwrap(),
        SignedLE::Negative
    );
}

#[test]
fn i16_be_enum_byte_layout() {
    assert_eq!(
        SignedBE::Negative.into_byte_array(),
        (-1000i16).to_be_bytes()
    );
    assert_eq!(SignedBE::Positive.into_byte_array(), 1000i16.to_be_bytes());
    assert_eq!(
        SignedBE::try_from_byte_array((-1000i16).to_be_bytes()).unwrap(),
        SignedBE::Negative
    );
}

// ── i32 repr ──────────────────────────────────────────────────────────────────

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(i32)]
#[byteable(little_endian)]
enum SignedI32LE {
    Min = -2_000_000,
    Zero = 0,
    Max = 2_000_000,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(i32)]
#[byteable(big_endian)]
enum SignedI32BE {
    Negative = -2_000_000,
    Zero = 0,
    Positive = 2_000_000,
}

#[test]
fn i32_le_enum_roundtrip() {
    assert_eq!(
        SignedI32LE::Min.into_byte_array(),
        (-2_000_000i32).to_le_bytes()
    );
    assert_eq!(SignedI32LE::Zero.into_byte_array(), 0i32.to_le_bytes());
    assert_eq!(
        SignedI32LE::Max.into_byte_array(),
        2_000_000i32.to_le_bytes()
    );
    assert_eq!(
        SignedI32LE::try_from_byte_array((-2_000_000i32).to_le_bytes()).unwrap(),
        SignedI32LE::Min
    );
}

#[test]
fn i32_le_enum_invalid_discriminant() {
    let err = SignedI32LE::try_from_byte_array(42i32.to_le_bytes()).unwrap_err();
    assert!(matches!(
        err,
        DecodeError::InvalidDiscriminant {
            raw: _,
            type_name: _
        }
    ))
}

#[test]
fn i32_be_enum_roundtrip() {
    assert_eq!(
        SignedI32BE::Negative.into_byte_array(),
        (-2_000_000i32).to_be_bytes()
    );
    assert_eq!(
        SignedI32BE::try_from_byte_array((-2_000_000i32).to_be_bytes()).unwrap(),
        SignedI32BE::Negative
    );
}

#[test]
fn i32_enum_byte_size() {
    assert_eq!(SignedI32LE::BYTE_SIZE, 4);
    assert_eq!(SignedI32BE::BYTE_SIZE, 4);
}

// ── i64 repr ──────────────────────────────────────────────────────────────────

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(i64)]
#[byteable(little_endian)]
enum SignedI64LE {
    LargeNeg = -9_000_000_000_000,
    Zero = 0,
    LargePos = 9_000_000_000_000,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(i64)]
#[byteable(big_endian)]
enum SignedI64BE {
    LargeNeg = -9_000_000_000_000,
    Zero = 0,
    LargePos = 9_000_000_000_000,
}

#[test]
fn i64_le_enum_roundtrip() {
    assert_eq!(
        SignedI64LE::LargeNeg.into_byte_array(),
        (-9_000_000_000_000i64).to_le_bytes()
    );
    assert_eq!(
        SignedI64LE::try_from_byte_array((-9_000_000_000_000i64).to_le_bytes()).unwrap(),
        SignedI64LE::LargeNeg
    );
}

#[test]
fn i64_le_enum_invalid_discriminant() {
    let err = SignedI64LE::try_from_byte_array(1i64.to_le_bytes()).unwrap_err();
    assert!(matches!(
        err,
        DecodeError::InvalidDiscriminant {
            raw: _,
            type_name: _
        }
    ))
}

#[test]
fn i64_be_enum_roundtrip() {
    assert_eq!(
        SignedI64BE::LargeNeg.into_byte_array(),
        (-9_000_000_000_000i64).to_be_bytes()
    );
    assert_eq!(
        SignedI64BE::try_from_byte_array((-9_000_000_000_000i64).to_be_bytes()).unwrap(),
        SignedI64BE::LargeNeg
    );
}

#[test]
fn i64_enum_byte_size() {
    assert_eq!(SignedI64LE::BYTE_SIZE, 8);
    assert_eq!(SignedI64BE::BYTE_SIZE, 8);
}

// ── Sparse and single-byte endianness ────────────────────────────────────────

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum SparseEnum {
    First = 1,
    Second = 5,
    Third = 10,
    Fourth = 100,
}

#[test]
fn sparse_enum_valid_discriminants() {
    assert_eq!(SparseEnum::First.into_byte_array(), [1]);
    assert_eq!(SparseEnum::Second.into_byte_array(), [5]);
    assert_eq!(SparseEnum::Third.into_byte_array(), [10]);
    assert_eq!(SparseEnum::Fourth.into_byte_array(), [100]);
    assert_eq!(
        SparseEnum::try_from_byte_array([1]).unwrap(),
        SparseEnum::First
    );
    assert_eq!(
        SparseEnum::try_from_byte_array([100]).unwrap(),
        SparseEnum::Fourth
    );
}

#[test]
fn sparse_enum_gaps_are_invalid() {
    for invalid in [0u8, 2, 6, 99, 101] {
        assert!(SparseEnum::try_from_byte_array([invalid]).is_err());
    }
}

#[test]
fn endianness_irrelevant_for_u8() {
    // Single-byte enums behave identically regardless of endian annotation.
    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    #[repr(u8)]
    enum ByteEnum {
        A = 1,
        B = 2,
    }
    assert_eq!(ByteEnum::A.into_byte_array(), [1]);
    assert_eq!(ByteEnum::B.into_byte_array(), [2]);
}

// ── Auto-repr and auto-discriminant inference ─────────────────────────────────

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
enum AutoReprEnum {
    A,
    B,
    C,
}

#[test]
fn auto_repr_and_discriminants() {
    // ≤256 variants → u8; discriminants assigned 0, 1, 2
    assert_eq!(AutoReprEnum::A.into_byte_array(), [0u8]);
    assert_eq!(AutoReprEnum::B.into_byte_array(), [1u8]);
    assert_eq!(AutoReprEnum::C.into_byte_array(), [2u8]);
    assert_eq!(
        AutoReprEnum::try_from_byte_array([0u8]),
        Ok(AutoReprEnum::A)
    );
    assert_eq!(
        AutoReprEnum::try_from_byte_array([2u8]),
        Ok(AutoReprEnum::C)
    );
    assert!(AutoReprEnum::try_from_byte_array([3u8]).is_err());
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum AutoDiscEnum {
    X,
    Y,
    Z,
}

#[test]
fn auto_discriminants_explicit_repr() {
    assert_eq!(AutoDiscEnum::X.into_byte_array(), [0u8]);
    assert_eq!(AutoDiscEnum::Y.into_byte_array(), [1u8]);
    assert_eq!(AutoDiscEnum::Z.into_byte_array(), [2u8]);
    assert_eq!(
        AutoDiscEnum::try_from_byte_array([2u8]),
        Ok(AutoDiscEnum::Z)
    );
    assert!(AutoDiscEnum::try_from_byte_array([5u8]).is_err());
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
enum MixedDiscEnum {
    First = 10,
    Second, // auto: 11
    Third = 20,
    Fourth, // auto: 21
}

#[test]
fn mixed_explicit_and_auto_discriminants() {
    assert_eq!(MixedDiscEnum::First.into_byte_array(), [10u8]);
    assert_eq!(MixedDiscEnum::Second.into_byte_array(), [11u8]);
    assert_eq!(MixedDiscEnum::Third.into_byte_array(), [20u8]);
    assert_eq!(MixedDiscEnum::Fourth.into_byte_array(), [21u8]);
    assert_eq!(
        MixedDiscEnum::try_from_byte_array([10u8]),
        Ok(MixedDiscEnum::First)
    );
    assert_eq!(
        MixedDiscEnum::try_from_byte_array([11u8]),
        Ok(MixedDiscEnum::Second)
    );
    assert!(MixedDiscEnum::try_from_byte_array([12u8]).is_err());
}

// ── u128 / i128 repr ──────────────────────────────────────────────────────────

mod u128_enums {
    use byteable::{Byteable, DecodeError, IntoByteArray, TryFromByteArray};

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    #[repr(u128)]
    #[byteable(little_endian)]
    enum LargeU128 {
        Zero = 0,
        Mid = 0x8000_0000_0000_0000_0000_0000_0000_0000,
        Max = u128::MAX,
    }

    #[test]
    fn u128_enum_roundtrip() {
        for v in [LargeU128::Zero, LargeU128::Mid, LargeU128::Max] {
            let bytes = v.into_byte_array();
            assert_eq!(LargeU128::try_from_byte_array(bytes).unwrap(), v);
        }
    }

    #[test]
    fn u128_enum_byte_size() {
        assert_eq!(LargeU128::BYTE_SIZE, 16);
    }

    #[test]
    fn u128_enum_byte_layout() {
        assert_eq!(LargeU128::Zero.into_byte_array(), 0u128.to_le_bytes());
        assert_eq!(LargeU128::Max.into_byte_array(), u128::MAX.to_le_bytes());
    }

    #[test]
    fn u128_enum_invalid_discriminant() {
        let err = LargeU128::try_from_byte_array(1u128.to_le_bytes()).unwrap_err();
        assert!(matches!(
            err,
            DecodeError::InvalidDiscriminant {
                raw: _,
                type_name: _
            }
        ))
    }

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    #[repr(i128)]
    #[byteable(little_endian)]
    enum SignedI128 {
        MinVal = i128::MIN,
        Zero = 0,
        MaxVal = i128::MAX,
    }

    #[test]
    fn i128_enum_roundtrip() {
        for v in [SignedI128::MinVal, SignedI128::Zero, SignedI128::MaxVal] {
            let bytes = v.into_byte_array();
            assert_eq!(SignedI128::try_from_byte_array(bytes).unwrap(), v);
        }
    }

    #[test]
    fn i128_enum_byte_size() {
        assert_eq!(SignedI128::BYTE_SIZE, 16);
    }

    #[test]
    fn i128_enum_byte_layout() {
        assert_eq!(
            SignedI128::MinVal.into_byte_array(),
            i128::MIN.to_le_bytes()
        );
        assert_eq!(SignedI128::Zero.into_byte_array(), 0i128.to_le_bytes());
        assert_eq!(
            SignedI128::MaxVal.into_byte_array(),
            i128::MAX.to_le_bytes()
        );
    }

    #[test]
    fn i128_enum_invalid_discriminant() {
        let err = SignedI128::try_from_byte_array(1i128.to_le_bytes()).unwrap_err();
        assert!(matches!(
            err,
            DecodeError::InvalidDiscriminant {
                raw: _,
                type_name: _
            }
        ))
    }

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    #[repr(u128)]
    #[byteable(big_endian)]
    enum BigEndianU128 {
        Low = 0x0001,
        High = 0xFFFF,
    }

    #[test]
    fn u128_big_endian_enum() {
        assert_eq!(
            BigEndianU128::Low.into_byte_array(),
            0x0001u128.to_be_bytes()
        );
        assert_eq!(
            BigEndianU128::High.into_byte_array(),
            0xFFFFu128.to_be_bytes()
        );
        assert_eq!(
            BigEndianU128::try_from_byte_array(0x0001u128.to_be_bytes()).unwrap(),
            BigEndianU128::Low
        );
    }
}
