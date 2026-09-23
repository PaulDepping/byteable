//! Tests for `#[derive(Byteable)]` on C-like enums.
//!
//! Covers all supported `#[repr]` types (`u8`–`u128`, `i8`–`i64`), explicit
//! and auto-inferred endianness, auto-inferred repr and discriminants, sparse
//! discriminants, and invalid-discriminant error reporting.
#![cfg(feature = "derive")]

use byteable::{Byteable, DecodeError, PlainOldData, RawRepr, ToByteArray, TryFromByteArray};

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
    #[byteable(tag = 0x1000)]
    Start,
    #[byteable(tag = 0x2000)]
    Stop,
    #[byteable(tag = 0x3000)]
    Pause,
}

#[test]
fn u16_le_enum_byte_layout() {
    assert_eq!(Command::Start.to_byte_array(), [0x00, 0x10]);
    assert_eq!(Command::Stop.to_byte_array(), [0x00, 0x20]);
    assert_eq!(Command::Pause.to_byte_array(), [0x00, 0x30]);
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
    #[byteable(tag = 0x1000)]
    Start,
    #[byteable(tag = 0x2000)]
    Stop,
    #[byteable(tag = 0x3000)]
    Pause,
}

#[test]
fn u16_be_enum_byte_layout() {
    assert_eq!(CommandBE::Start.to_byte_array(), [0x10, 0x00]);
    assert_eq!(CommandBE::Stop.to_byte_array(), [0x20, 0x00]);
    assert_eq!(CommandBE::Pause.to_byte_array(), [0x30, 0x00]);
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
    #[byteable(tag = 6)]
    Tcp,
    #[byteable(tag = 17)]
    Udp,
    #[byteable(tag = 1)]
    Icmp,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
#[byteable(little_endian)]
enum ProtocolLE {
    #[byteable(tag = 0x12345678)]
    Tcp,
    #[byteable(tag = 0xABCDEF00)]
    Udp,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
#[byteable(big_endian)]
enum ProtocolBE {
    #[byteable(tag = 0x12345678)]
    Tcp,
    #[byteable(tag = 0xABCDEF00)]
    Udp,
}

#[test]
fn u32_enum_roundtrip() {
    let bytes = NetworkProtocol::Tcp.to_byte_array();
    assert_eq!(bytes, 6u32.to_le_bytes());
    assert_eq!(
        NetworkProtocol::try_from_byte_array(bytes).unwrap(),
        NetworkProtocol::Tcp
    );
}

#[test]
fn u32_le_enum_byte_layout() {
    assert_eq!(ProtocolLE::Tcp.to_byte_array(), [0x78, 0x56, 0x34, 0x12]);
    assert_eq!(ProtocolLE::Udp.to_byte_array(), [0x00, 0xEF, 0xCD, 0xAB]);
    assert_eq!(
        ProtocolLE::try_from_byte_array([0x78, 0x56, 0x34, 0x12]).unwrap(),
        ProtocolLE::Tcp
    );
}

#[test]
fn u32_be_enum_byte_layout() {
    assert_eq!(ProtocolBE::Tcp.to_byte_array(), [0x12, 0x34, 0x56, 0x78]);
    assert_eq!(ProtocolBE::Udp.to_byte_array(), [0xAB, 0xCD, 0xEF, 0x00]);
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
    #[byteable(tag = 1)]
    Small,
    #[byteable(tag = 1000)]
    Medium,
    #[byteable(tag = 1_000_000)]
    Large,
    #[byteable(tag = 1_000_000_000_000)]
    Huge,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u64)]
#[byteable(little_endian)]
enum LargeLE {
    #[byteable(tag = 0x1122334455667788)]
    Small,
    #[byteable(tag = 0xAABBCCDDEEFF0011)]
    Large,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u64)]
#[byteable(big_endian)]
enum LargeBE {
    #[byteable(tag = 0x1122334455667788)]
    Small,
    #[byteable(tag = 0xAABBCCDDEEFF0011)]
    Large,
}

#[test]
fn u64_enum_roundtrip() {
    let bytes = LargeValue::Huge.to_byte_array();
    assert_eq!(bytes, 1_000_000_000_000u64.to_le_bytes());
    assert_eq!(
        LargeValue::try_from_byte_array(bytes).unwrap(),
        LargeValue::Huge
    );
}

#[test]
fn u64_le_enum_byte_layout() {
    assert_eq!(
        LargeLE::Small.to_byte_array(),
        [0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]
    );
    assert_eq!(
        LargeLE::Large.to_byte_array(),
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
        LargeBE::Small.to_byte_array(),
        [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]
    );
    assert_eq!(
        LargeBE::Large.to_byte_array(),
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
    #[byteable(tag = -10)]
    Cold,
    #[byteable(tag = 0)]
    Cool,
    #[byteable(tag = 10)]
    Warm,
    #[byteable(tag = 30)]
    Hot,
}

#[test]
fn i8_enum_roundtrip() {
    assert_eq!(Temperature::Cold.to_byte_array(), [(-10i8) as u8]);
    assert_eq!(Temperature::Cool.to_byte_array(), [0]);
    assert_eq!(Temperature::Warm.to_byte_array(), [10]);
    assert_eq!(Temperature::Hot.to_byte_array(), [30]);
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
    #[byteable(tag = -1000)]
    Negative,
    #[byteable(tag = 0)]
    Zero,
    #[byteable(tag = 1000)]
    Positive,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(i16)]
#[byteable(big_endian)]
enum SignedBE {
    #[byteable(tag = -1000)]
    Negative,
    #[byteable(tag = 0)]
    Zero,
    #[byteable(tag = 1000)]
    Positive,
}

#[test]
fn i16_le_enum_byte_layout() {
    assert_eq!(SignedLE::Negative.to_byte_array(), (-1000i16).to_le_bytes());
    assert_eq!(SignedLE::Positive.to_byte_array(), 1000i16.to_le_bytes());
    assert_eq!(
        SignedLE::try_from_byte_array((-1000i16).to_le_bytes()).unwrap(),
        SignedLE::Negative
    );
}

#[test]
fn i16_be_enum_byte_layout() {
    assert_eq!(SignedBE::Negative.to_byte_array(), (-1000i16).to_be_bytes());
    assert_eq!(SignedBE::Positive.to_byte_array(), 1000i16.to_be_bytes());
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
    #[byteable(tag = -2_000_000)]
    Min,
    #[byteable(tag = 0)]
    Zero,
    #[byteable(tag = 2_000_000)]
    Max,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(i32)]
#[byteable(big_endian)]
enum SignedI32BE {
    #[byteable(tag = -2_000_000)]
    Negative,
    #[byteable(tag = 0)]
    Zero,
    #[byteable(tag = 2_000_000)]
    Positive,
}

#[test]
fn i32_le_enum_roundtrip() {
    assert_eq!(
        SignedI32LE::Min.to_byte_array(),
        (-2_000_000i32).to_le_bytes()
    );
    assert_eq!(SignedI32LE::Zero.to_byte_array(), 0i32.to_le_bytes());
    assert_eq!(SignedI32LE::Max.to_byte_array(), 2_000_000i32.to_le_bytes());
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
        SignedI32BE::Negative.to_byte_array(),
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
    #[byteable(tag = -9_000_000_000_000)]
    LargeNeg,
    #[byteable(tag = 0)]
    Zero,
    #[byteable(tag = 9_000_000_000_000)]
    LargePos,
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(i64)]
#[byteable(big_endian)]
enum SignedI64BE {
    #[byteable(tag = -9_000_000_000_000)]
    LargeNeg,
    #[byteable(tag = 0)]
    Zero,
    #[byteable(tag = 9_000_000_000_000)]
    LargePos,
}

#[test]
fn i64_le_enum_roundtrip() {
    assert_eq!(
        SignedI64LE::LargeNeg.to_byte_array(),
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
        SignedI64BE::LargeNeg.to_byte_array(),
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
    #[byteable(tag = 1)]
    First,
    #[byteable(tag = 5)]
    Second,
    #[byteable(tag = 10)]
    Third,
    #[byteable(tag = 100)]
    Fourth,
}

#[test]
fn sparse_enum_valid_discriminants() {
    assert_eq!(SparseEnum::First.to_byte_array(), [1]);
    assert_eq!(SparseEnum::Second.to_byte_array(), [5]);
    assert_eq!(SparseEnum::Third.to_byte_array(), [10]);
    assert_eq!(SparseEnum::Fourth.to_byte_array(), [100]);
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
        #[byteable(tag = 1)]
        A,
        B, // implicit, continues from A's tag=1 -> 2
    }
    assert_eq!(ByteEnum::A.to_byte_array(), [1]);
    assert_eq!(ByteEnum::B.to_byte_array(), [2]);
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
    assert_eq!(AutoReprEnum::A.to_byte_array(), [0u8]);
    assert_eq!(AutoReprEnum::B.to_byte_array(), [1u8]);
    assert_eq!(AutoReprEnum::C.to_byte_array(), [2u8]);
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
    assert_eq!(AutoDiscEnum::X.to_byte_array(), [0u8]);
    assert_eq!(AutoDiscEnum::Y.to_byte_array(), [1u8]);
    assert_eq!(AutoDiscEnum::Z.to_byte_array(), [2u8]);
    assert_eq!(
        AutoDiscEnum::try_from_byte_array([2u8]),
        Ok(AutoDiscEnum::Z)
    );
    assert!(AutoDiscEnum::try_from_byte_array([5u8]).is_err());
}

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
enum MixedDiscEnum {
    #[byteable(tag = 10)]
    First,
    Second, // implicit, continues from First's tag=10 -> 11
    #[byteable(tag = 20)]
    Third,
    Fourth, // implicit, continues from Third's tag=20 -> 21
}

#[test]
fn mixed_explicit_and_auto_discriminants() {
    assert_eq!(MixedDiscEnum::First.to_byte_array(), [10u8]);
    assert_eq!(MixedDiscEnum::Second.to_byte_array(), [11u8]);
    assert_eq!(MixedDiscEnum::Third.to_byte_array(), [20u8]);
    assert_eq!(MixedDiscEnum::Fourth.to_byte_array(), [21u8]);
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
    use byteable::{Byteable, DecodeError, ToByteArray, TryFromByteArray};

    // NOTE: `#[byteable(tag = <expr>)]` is emitted verbatim into a generated
    // `const __BYTEABLE_TAG_<ENUM>_<index>: repr_ty = <expr>;` declaration (see
    // `resolve_enum_discriminants` in `byteable_derive/src/lib.rs`) and type/range-checked by
    // rustc itself, rather than being parsed/evaluated as an `i128` by `byteable_derive`. So
    // `Mid` can express `u128::MAX` (every bit, including bit 127, the actual high bit of a
    // `u128`, set) directly via the named const `u128::MAX`, with no special-casing needed for
    // the upper half of `u128`'s range - unlike the previous `i128`-typed `tag` parser, which
    // could only reach that range via a negative-tag two's-complement escape hatch.
    //
    // `Max` uses `i128::MAX` directly (`2^127 - 1`) as a plain integer literal, still within the
    // *lower* half of `u128`'s range - included alongside `Mid` for coverage of both halves.
    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    #[repr(u128)]
    #[byteable(little_endian)]
    enum LargeU128 {
        #[byteable(tag = 0)]
        Zero,
        // u128::MAX, expressed directly via the named const; see NOTE above.
        #[byteable(tag = u128::MAX)]
        Mid,
        // i128::MAX; see NOTE above.
        #[byteable(tag = 170141183460469231731687303715884105727)]
        Max,
    }

    #[test]
    fn u128_enum_roundtrip() {
        for v in [LargeU128::Zero, LargeU128::Mid, LargeU128::Max] {
            let bytes = v.to_byte_array();
            assert_eq!(LargeU128::try_from_byte_array(bytes).unwrap(), v);
        }
    }

    #[test]
    fn u128_enum_byte_size() {
        assert_eq!(LargeU128::BYTE_SIZE, 16);
    }

    #[test]
    fn u128_enum_byte_layout() {
        assert_eq!(LargeU128::Zero.to_byte_array(), 0u128.to_le_bytes());
        // `Mid` is tagged `u128::MAX` directly; see the NOTE on `LargeU128`.
        assert_eq!(LargeU128::Mid.to_byte_array(), u128::MAX.to_le_bytes());
        // `Max` is tagged `i128::MAX` directly - see the NOTE on `LargeU128`.
        assert_eq!(
            LargeU128::Max.to_byte_array(),
            (i128::MAX as u128).to_le_bytes()
        );
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

    // NOTE: `#[byteable(tag = <expr>)]` is spliced verbatim into a generated
    // `const __BYTEABLE_TAG_<ENUM>_<index>: repr_ty = <expr>;` declaration and
    // evaluated/range-checked by rustc itself, not parsed by `byteable_derive`. `MinVal` is kept
    // as `i128::MIN + 1` rather than `i128::MIN` mainly for symmetry with `MaxVal` below (both
    // near-boundary rather than exactly-boundary values) - `tag = i128::MIN` would also work
    // today, unlike under the old `i128`-magnitude-based parser. `MaxVal` uses `i128::MAX`
    // directly as the last variant here, with no successor variant to implicitly continue from
    // it.
    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    #[repr(i128)]
    #[byteable(little_endian)]
    enum SignedI128 {
        // i128::MIN + 1; substitute for i128::MIN, see NOTE above
        #[byteable(tag = -170141183460469231731687303715884105727)]
        MinVal,
        #[byteable(tag = 0)]
        Zero,
        // i128::MAX; see NOTE above.
        #[byteable(tag = 170141183460469231731687303715884105727)]
        MaxVal,
    }

    #[test]
    fn i128_enum_roundtrip() {
        for v in [SignedI128::MinVal, SignedI128::Zero, SignedI128::MaxVal] {
            let bytes = v.to_byte_array();
            assert_eq!(SignedI128::try_from_byte_array(bytes).unwrap(), v);
        }
    }

    #[test]
    fn i128_enum_byte_size() {
        assert_eq!(SignedI128::BYTE_SIZE, 16);
    }

    #[test]
    fn i128_enum_byte_layout() {
        // `MinVal` is now tagged `i128::MIN + 1` rather than `i128::MIN` - see the NOTE on
        // `SignedI128`.
        assert_eq!(
            SignedI128::MinVal.to_byte_array(),
            (i128::MIN + 1).to_le_bytes()
        );
        assert_eq!(SignedI128::Zero.to_byte_array(), 0i128.to_le_bytes());
        // `MaxVal` is tagged `i128::MAX` directly - see the NOTE on `SignedI128`.
        assert_eq!(SignedI128::MaxVal.to_byte_array(), i128::MAX.to_le_bytes());
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
        #[byteable(tag = 0x0001)]
        Low,
        #[byteable(tag = 0xFFFF)]
        High,
    }

    #[test]
    fn u128_big_endian_enum() {
        assert_eq!(BigEndianU128::Low.to_byte_array(), 0x0001u128.to_be_bytes());
        assert_eq!(
            BigEndianU128::High.to_byte_array(),
            0xFFFFu128.to_be_bytes()
        );
        assert_eq!(
            BigEndianU128::try_from_byte_array(0x0001u128.to_be_bytes()).unwrap(),
            BigEndianU128::Low
        );
    }
}

// ── non-`Copy` and empty enums ──────────────────────────────────────────────────
//
// Every enum above derives `Clone, Copy`, which masked a bug: the unit-enum codegen used
// to build `to_raw`/`to_byte_array` from `*self as _`, which only compiles when `Self`
// is `Copy` (an enum-to-integer cast still needs to move the operand out of the `&self`
// reference otherwise) - and, separately, never compiles for an empty enum regardless of
// `Copy`, since there's no value to move at all.

#[derive(Byteable, Debug, PartialEq)]
enum NonCopyStatus {
    Idle,
    Running,
    Done,
}

#[test]
fn non_copy_unit_enum_roundtrip() {
    for (variant, byte) in [
        (NonCopyStatus::Idle, 0u8),
        (NonCopyStatus::Running, 1),
        (NonCopyStatus::Done, 2),
    ] {
        assert_eq!(variant.to_raw(), byte);
        assert_eq!(NonCopyStatus::try_from_byte_array([byte]).unwrap(), variant);
    }
}

#[derive(Debug, Byteable)]
enum EmptyEnum {}

#[test]
fn empty_enum_byte_size() {
    assert_eq!(EmptyEnum::BYTE_SIZE, 1);
}

#[test]
fn empty_enum_every_discriminant_is_invalid() {
    let err = EmptyEnum::try_from_byte_array([0]).unwrap_err();
    assert!(matches!(err, DecodeError::InvalidDiscriminant { .. }));
}

// ── `#[byteable(tag = N)]` on variants ──────────────────────────────────────────

mod variant_tags {
    use byteable::{Byteable, ToByteArray, TryFromByteArray};

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    enum Command {
        #[byteable(tag = 1)]
        Ping,
        #[byteable(tag = 0)]
        Pong,
    }

    #[test]
    fn tag_controls_discriminant_independent_of_declaration_order() {
        // Ping is declared first but tagged 1; Pong is declared second but tagged 0.
        assert_eq!(Command::Ping.to_byte_array(), [1]);
        assert_eq!(Command::Pong.to_byte_array(), [0]);
        assert_eq!(Command::try_from_byte_array([1]).unwrap(), Command::Ping);
        assert_eq!(Command::try_from_byte_array([0]).unwrap(), Command::Pong);
    }

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    enum Mixed {
        A, // implicit -> 0
        #[byteable(tag = 5)]
        B, // explicit -> 5
        C, // implicit, continues from 5 -> 6
    }

    #[test]
    fn tag_mixing_continues_counter_from_last_explicit() {
        assert_eq!(Mixed::A.to_byte_array(), [0]);
        assert_eq!(Mixed::B.to_byte_array(), [5]);
        assert_eq!(Mixed::C.to_byte_array(), [6]);
    }
}

// ── `#[byteable(discriminant = ..)]` enum-level width override ─────────────────

mod discriminant_override {
    use byteable::{Byteable, ToByteArray, TryFromByteArray};

    // Only 2 variants, so the count-based auto-select ladder would pick `u8` - but `Big`'s
    // explicit tag of 999 doesn't fit in a u8. `#[byteable(discriminant = u16)]` overrides the
    // auto-selected width (independent of any real `#[repr(...)]`, of which there is none
    // here) so the enum's discriminant is encoded as u16 instead.
    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    #[byteable(discriminant = u16)]
    enum Wide {
        Small, // implicit -> 0
        #[byteable(tag = 999)]
        Big,
    }

    #[test]
    fn discriminant_override_widens_wire_size() {
        // u16 discriminant -> 2 bytes on the wire, not the 1 byte a u8 auto-select would use.
        assert_eq!(Wide::BYTE_SIZE, 2);
    }

    #[test]
    fn discriminant_override_roundtrip() {
        assert_eq!(Wide::Small.to_byte_array(), 0u16.to_le_bytes());
        assert_eq!(Wide::Big.to_byte_array(), 999u16.to_le_bytes());
        assert_eq!(
            Wide::try_from_byte_array(0u16.to_le_bytes()).unwrap(),
            Wide::Small
        );
        assert_eq!(
            Wide::try_from_byte_array(999u16.to_le_bytes()).unwrap(),
            Wide::Big
        );
    }

    // `#[byteable(discriminant = ..)]` with a SIGNED wire width, exercised with a negative
    // `tag` - confirms the override accepts signed integer types end-to-end, not just unsigned
    // ones like `Wide` above.
    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    #[byteable(discriminant = i16)]
    enum Signed {
        #[byteable(tag = -1)]
        Negative,
        #[byteable(tag = 0)]
        Zero,
    }

    #[test]
    fn discriminant_override_accepts_signed_width() {
        assert_eq!(Signed::BYTE_SIZE, 2);
        assert_eq!(Signed::Negative.to_byte_array(), (-1i16).to_le_bytes());
        assert_eq!(Signed::Zero.to_byte_array(), 0i16.to_le_bytes());
        assert_eq!(
            Signed::try_from_byte_array((-1i16).to_le_bytes()).unwrap(),
            Signed::Negative
        );
        assert_eq!(
            Signed::try_from_byte_array(0i16.to_le_bytes()).unwrap(),
            Signed::Zero
        );
    }

    const CUSTOM_TAG: u16 = 12345;

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    #[byteable(discriminant = u16)]
    enum NamedConstTag {
        #[byteable(tag = CUSTOM_TAG)]
        Custom,
        NextOne, // implicit, continues from CUSTOM_TAG + 1
    }

    #[test]
    fn tag_accepts_named_constant() {
        assert_eq!(
            NamedConstTag::Custom.to_byte_array(),
            CUSTOM_TAG.to_le_bytes()
        );
        assert_eq!(
            NamedConstTag::NextOne.to_byte_array(),
            (CUSTOM_TAG + 1).to_le_bytes()
        );
        assert_eq!(
            NamedConstTag::try_from_byte_array(CUSTOM_TAG.to_le_bytes()).unwrap(),
            NamedConstTag::Custom
        );
        assert_eq!(
            NamedConstTag::try_from_byte_array((CUSTOM_TAG + 1).to_le_bytes()).unwrap(),
            NamedConstTag::NextOne
        );
    }

    // `#[byteable(discriminant = ..)]` on a field-carrying enum - this goes through
    // `enum_derive`'s dynamic/I-O path (`Readable`/`Writable`), not `unit_enum_derive`'s
    // fixed-size path that `Wide`/`Signed` above exercise, so it needs its own coverage.
    #[cfg(feature = "std")]
    mod field_variant_override {
        use super::*;
        use byteable::io::{ReadValue, WriteValue};
        use std::io::Cursor;

        #[derive(Byteable, Debug, PartialEq)]
        #[byteable(discriminant = u16)]
        enum WideField {
            Small,
            #[byteable(tag = 999)]
            Big {
                value: u8,
            },
        }

        #[test]
        fn discriminant_override_on_field_enum_roundtrip() {
            let mut buf = Vec::new();
            buf.write_value(&WideField::Big { value: 7 }).unwrap();
            // u16 discriminant (not the u8 an auto-selected width would pick for 2 variants),
            // followed by the variant's field.
            let mut expected = 999u16.to_le_bytes().to_vec();
            expected.push(7);
            assert_eq!(buf, expected);

            let decoded: WideField = Cursor::new(&buf).read_value().unwrap();
            assert_eq!(decoded, WideField::Big { value: 7 });

            let mut small_buf = Vec::new();
            small_buf.write_value(&WideField::Small).unwrap();
            assert_eq!(small_buf, 0u16.to_le_bytes());
            let decoded_small: WideField = Cursor::new(&small_buf).read_value().unwrap();
            assert_eq!(decoded_small, WideField::Small);
        }

        const FIELD_CUSTOM_TAG: u16 = 500;

        #[derive(Byteable, Debug, PartialEq)]
        #[byteable(discriminant = u16)]
        enum NamedConstFieldTag {
            #[byteable(tag = FIELD_CUSTOM_TAG)]
            Custom { value: u8 },
        }

        #[test]
        fn tag_accepts_named_constant_on_field_variant() {
            let mut buf = Vec::new();
            buf.write_value(&NamedConstFieldTag::Custom { value: 9 })
                .unwrap();
            let mut expected = FIELD_CUSTOM_TAG.to_le_bytes().to_vec();
            expected.push(9);
            assert_eq!(buf, expected);
            let decoded: NamedConstFieldTag = Cursor::new(&buf).read_value().unwrap();
            assert_eq!(decoded, NamedConstFieldTag::Custom { value: 9 });
        }
    }
}

/// A `tag` value that doesn't fit the enum's resolved discriminant width is a compile error -
/// caught by rustc directly (the generated `const` declaration doesn't type-check), not by any
/// `byteable`-specific validation.
///
/// ```compile_fail
/// # #[cfg(feature = "derive")] {
/// use byteable::Byteable;
///
/// #[derive(Byteable, Clone, Copy)]
/// enum Bad {
///     #[byteable(tag = 999)]
///     A, // 999 doesn't fit in the auto-selected u8 width for a 1-variant enum
/// }
/// # }
/// ```
///
/// Two variants resolving to the same wire discriminant is a compile error, caught by the
/// generated duplicate-tag check.
///
/// ```compile_fail
/// # #[cfg(feature = "derive")] {
/// use byteable::Byteable;
///
/// #[derive(Byteable, Clone, Copy)]
/// enum Bad {
///     #[byteable(tag = 5)]
///     A,
///     #[byteable(tag = 5)]
///     B,
/// }
/// # }
/// ```
#[test]
fn compile_fail_tag_examples_documented_above() {}

mod raw_identifiers {
    use byteable::{Byteable, ToByteArray, TryFromByteArray};

    // Regression test: the derive macro used to panic on a raw-identifier enum name or
    // variant name ("... is not a valid identifier") because discriminant-const naming routed
    // the name through `.to_string().to_uppercase()`, which doesn't preserve `format_ident!`'s
    // usual automatic `r#`-stripping (that only happens when passed a `syn::Ident` directly).

    #[allow(non_camel_case_types)]
    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    enum r#struct {
        A,
        B,
    }

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    enum RawVariant {
        #[allow(non_camel_case_types)]
        r#type,
        Other,
    }

    #[test]
    fn raw_ident_enum_name_roundtrip() {
        assert_eq!(
            r#struct::try_from_byte_array(r#struct::A.to_byte_array()).unwrap(),
            r#struct::A
        );
        assert_eq!(
            r#struct::try_from_byte_array(r#struct::B.to_byte_array()).unwrap(),
            r#struct::B
        );
    }

    #[test]
    fn raw_ident_variant_roundtrip() {
        assert_eq!(
            RawVariant::try_from_byte_array(RawVariant::r#type.to_byte_array()).unwrap(),
            RawVariant::r#type
        );
        assert_eq!(
            RawVariant::try_from_byte_array(RawVariant::Other.to_byte_array()).unwrap(),
            RawVariant::Other
        );
    }
}

mod case_folding {
    use byteable::{Byteable, ToByteArray, TryFromByteArray};

    // Regression test: variants differing only by case used to collide, because discriminant
    // consts were named from the uppercased variant text (e.g. `Ab` and `AB` both produced
    // `__BYTEABLE_TAG_CASEFOLD_AB`).

    #[allow(non_camel_case_types)]
    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    enum CaseFold {
        Ab,
        AB,
        aB,
    }

    #[test]
    fn case_folding_variants_are_distinct() {
        assert_ne!(CaseFold::Ab.to_byte_array(), CaseFold::AB.to_byte_array());
        assert_ne!(CaseFold::AB.to_byte_array(), CaseFold::aB.to_byte_array());
        assert_eq!(
            CaseFold::try_from_byte_array(CaseFold::Ab.to_byte_array()).unwrap(),
            CaseFold::Ab
        );
        assert_eq!(
            CaseFold::try_from_byte_array(CaseFold::AB.to_byte_array()).unwrap(),
            CaseFold::AB
        );
        assert_eq!(
            CaseFold::try_from_byte_array(CaseFold::aB.to_byte_array()).unwrap(),
            CaseFold::aB
        );
    }
}

mod large_enum {
    use byteable::{Byteable, ToByteArray, TryFromByteArray};

    // Regression test: the previous O(n^2) const-fn/array-literal duplicate-tag scan hit
    // rustc's deny-by-default `long_running_const_eval` lint (a hard compile error) above
    // roughly 1100-1400 variants. The hidden "shadow enum" mechanism that replaced it uses
    // Rust's own native discriminant-uniqueness check for enum lowering, not the general
    // const-eval interpreter, so it has no comparable size ceiling. This enum - well within
    // the crate's own documented auto-select discriminant-width ladder, which explicitly
    // provisions for enums up to 65,536 variants - simply compiling at all (in well under a
    // few seconds) is the actual regression test; no need to time it here.
    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    enum LargeEnum {
        V0,
        V1,
        V2,
        V3,
        V4,
        V5,
        V6,
        V7,
        V8,
        V9,
        V10,
        V11,
        V12,
        V13,
        V14,
        V15,
        V16,
        V17,
        V18,
        V19,
        V20,
        V21,
        V22,
        V23,
        V24,
        V25,
        V26,
        V27,
        V28,
        V29,
        V30,
        V31,
        V32,
        V33,
        V34,
        V35,
        V36,
        V37,
        V38,
        V39,
        V40,
        V41,
        V42,
        V43,
        V44,
        V45,
        V46,
        V47,
        V48,
        V49,
        V50,
        V51,
        V52,
        V53,
        V54,
        V55,
        V56,
        V57,
        V58,
        V59,
        V60,
        V61,
        V62,
        V63,
        V64,
        V65,
        V66,
        V67,
        V68,
        V69,
        V70,
        V71,
        V72,
        V73,
        V74,
        V75,
        V76,
        V77,
        V78,
        V79,
        V80,
        V81,
        V82,
        V83,
        V84,
        V85,
        V86,
        V87,
        V88,
        V89,
        V90,
        V91,
        V92,
        V93,
        V94,
        V95,
        V96,
        V97,
        V98,
        V99,
        V100,
        V101,
        V102,
        V103,
        V104,
        V105,
        V106,
        V107,
        V108,
        V109,
        V110,
        V111,
        V112,
        V113,
        V114,
        V115,
        V116,
        V117,
        V118,
        V119,
        V120,
        V121,
        V122,
        V123,
        V124,
        V125,
        V126,
        V127,
        V128,
        V129,
        V130,
        V131,
        V132,
        V133,
        V134,
        V135,
        V136,
        V137,
        V138,
        V139,
        V140,
        V141,
        V142,
        V143,
        V144,
        V145,
        V146,
        V147,
        V148,
        V149,
        V150,
        V151,
        V152,
        V153,
        V154,
        V155,
        V156,
        V157,
        V158,
        V159,
        V160,
        V161,
        V162,
        V163,
        V164,
        V165,
        V166,
        V167,
        V168,
        V169,
        V170,
        V171,
        V172,
        V173,
        V174,
        V175,
        V176,
        V177,
        V178,
        V179,
        V180,
        V181,
        V182,
        V183,
        V184,
        V185,
        V186,
        V187,
        V188,
        V189,
        V190,
        V191,
        V192,
        V193,
        V194,
        V195,
        V196,
        V197,
        V198,
        V199,
        V200,
        V201,
        V202,
        V203,
        V204,
        V205,
        V206,
        V207,
        V208,
        V209,
        V210,
        V211,
        V212,
        V213,
        V214,
        V215,
        V216,
        V217,
        V218,
        V219,
        V220,
        V221,
        V222,
        V223,
        V224,
        V225,
        V226,
        V227,
        V228,
        V229,
        V230,
        V231,
        V232,
        V233,
        V234,
        V235,
        V236,
        V237,
        V238,
        V239,
        V240,
        V241,
        V242,
        V243,
        V244,
        V245,
        V246,
        V247,
        V248,
        V249,
        V250,
        V251,
        V252,
        V253,
        V254,
        V255,
        V256,
        V257,
        V258,
        V259,
        V260,
        V261,
        V262,
        V263,
        V264,
        V265,
        V266,
        V267,
        V268,
        V269,
        V270,
        V271,
        V272,
        V273,
        V274,
        V275,
        V276,
        V277,
        V278,
        V279,
        V280,
        V281,
        V282,
        V283,
        V284,
        V285,
        V286,
        V287,
        V288,
        V289,
        V290,
        V291,
        V292,
        V293,
        V294,
        V295,
        V296,
        V297,
        V298,
        V299,
        V300,
        V301,
        V302,
        V303,
        V304,
        V305,
        V306,
        V307,
        V308,
        V309,
        V310,
        V311,
        V312,
        V313,
        V314,
        V315,
        V316,
        V317,
        V318,
        V319,
        V320,
        V321,
        V322,
        V323,
        V324,
        V325,
        V326,
        V327,
        V328,
        V329,
        V330,
        V331,
        V332,
        V333,
        V334,
        V335,
        V336,
        V337,
        V338,
        V339,
        V340,
        V341,
        V342,
        V343,
        V344,
        V345,
        V346,
        V347,
        V348,
        V349,
        V350,
        V351,
        V352,
        V353,
        V354,
        V355,
        V356,
        V357,
        V358,
        V359,
        V360,
        V361,
        V362,
        V363,
        V364,
        V365,
        V366,
        V367,
        V368,
        V369,
        V370,
        V371,
        V372,
        V373,
        V374,
        V375,
        V376,
        V377,
        V378,
        V379,
        V380,
        V381,
        V382,
        V383,
        V384,
        V385,
        V386,
        V387,
        V388,
        V389,
        V390,
        V391,
        V392,
        V393,
        V394,
        V395,
        V396,
        V397,
        V398,
        V399,
        V400,
        V401,
        V402,
        V403,
        V404,
        V405,
        V406,
        V407,
        V408,
        V409,
        V410,
        V411,
        V412,
        V413,
        V414,
        V415,
        V416,
        V417,
        V418,
        V419,
        V420,
        V421,
        V422,
        V423,
        V424,
        V425,
        V426,
        V427,
        V428,
        V429,
        V430,
        V431,
        V432,
        V433,
        V434,
        V435,
        V436,
        V437,
        V438,
        V439,
        V440,
        V441,
        V442,
        V443,
        V444,
        V445,
        V446,
        V447,
        V448,
        V449,
        V450,
        V451,
        V452,
        V453,
        V454,
        V455,
        V456,
        V457,
        V458,
        V459,
        V460,
        V461,
        V462,
        V463,
        V464,
        V465,
        V466,
        V467,
        V468,
        V469,
        V470,
        V471,
        V472,
        V473,
        V474,
        V475,
        V476,
        V477,
        V478,
        V479,
        V480,
        V481,
        V482,
        V483,
        V484,
        V485,
        V486,
        V487,
        V488,
        V489,
        V490,
        V491,
        V492,
        V493,
        V494,
        V495,
        V496,
        V497,
        V498,
        V499,
        V500,
        V501,
        V502,
        V503,
        V504,
        V505,
        V506,
        V507,
        V508,
        V509,
        V510,
        V511,
        V512,
        V513,
        V514,
        V515,
        V516,
        V517,
        V518,
        V519,
        V520,
        V521,
        V522,
        V523,
        V524,
        V525,
        V526,
        V527,
        V528,
        V529,
        V530,
        V531,
        V532,
        V533,
        V534,
        V535,
        V536,
        V537,
        V538,
        V539,
        V540,
        V541,
        V542,
        V543,
        V544,
        V545,
        V546,
        V547,
        V548,
        V549,
        V550,
        V551,
        V552,
        V553,
        V554,
        V555,
        V556,
        V557,
        V558,
        V559,
        V560,
        V561,
        V562,
        V563,
        V564,
        V565,
        V566,
        V567,
        V568,
        V569,
        V570,
        V571,
        V572,
        V573,
        V574,
        V575,
        V576,
        V577,
        V578,
        V579,
        V580,
        V581,
        V582,
        V583,
        V584,
        V585,
        V586,
        V587,
        V588,
        V589,
        V590,
        V591,
        V592,
        V593,
        V594,
        V595,
        V596,
        V597,
        V598,
        V599,
        V600,
        V601,
        V602,
        V603,
        V604,
        V605,
        V606,
        V607,
        V608,
        V609,
        V610,
        V611,
        V612,
        V613,
        V614,
        V615,
        V616,
        V617,
        V618,
        V619,
        V620,
        V621,
        V622,
        V623,
        V624,
        V625,
        V626,
        V627,
        V628,
        V629,
        V630,
        V631,
        V632,
        V633,
        V634,
        V635,
        V636,
        V637,
        V638,
        V639,
        V640,
        V641,
        V642,
        V643,
        V644,
        V645,
        V646,
        V647,
        V648,
        V649,
        V650,
        V651,
        V652,
        V653,
        V654,
        V655,
        V656,
        V657,
        V658,
        V659,
        V660,
        V661,
        V662,
        V663,
        V664,
        V665,
        V666,
        V667,
        V668,
        V669,
        V670,
        V671,
        V672,
        V673,
        V674,
        V675,
        V676,
        V677,
        V678,
        V679,
        V680,
        V681,
        V682,
        V683,
        V684,
        V685,
        V686,
        V687,
        V688,
        V689,
        V690,
        V691,
        V692,
        V693,
        V694,
        V695,
        V696,
        V697,
        V698,
        V699,
        V700,
        V701,
        V702,
        V703,
        V704,
        V705,
        V706,
        V707,
        V708,
        V709,
        V710,
        V711,
        V712,
        V713,
        V714,
        V715,
        V716,
        V717,
        V718,
        V719,
        V720,
        V721,
        V722,
        V723,
        V724,
        V725,
        V726,
        V727,
        V728,
        V729,
        V730,
        V731,
        V732,
        V733,
        V734,
        V735,
        V736,
        V737,
        V738,
        V739,
        V740,
        V741,
        V742,
        V743,
        V744,
        V745,
        V746,
        V747,
        V748,
        V749,
        V750,
        V751,
        V752,
        V753,
        V754,
        V755,
        V756,
        V757,
        V758,
        V759,
        V760,
        V761,
        V762,
        V763,
        V764,
        V765,
        V766,
        V767,
        V768,
        V769,
        V770,
        V771,
        V772,
        V773,
        V774,
        V775,
        V776,
        V777,
        V778,
        V779,
        V780,
        V781,
        V782,
        V783,
        V784,
        V785,
        V786,
        V787,
        V788,
        V789,
        V790,
        V791,
        V792,
        V793,
        V794,
        V795,
        V796,
        V797,
        V798,
        V799,
        V800,
        V801,
        V802,
        V803,
        V804,
        V805,
        V806,
        V807,
        V808,
        V809,
        V810,
        V811,
        V812,
        V813,
        V814,
        V815,
        V816,
        V817,
        V818,
        V819,
        V820,
        V821,
        V822,
        V823,
        V824,
        V825,
        V826,
        V827,
        V828,
        V829,
        V830,
        V831,
        V832,
        V833,
        V834,
        V835,
        V836,
        V837,
        V838,
        V839,
        V840,
        V841,
        V842,
        V843,
        V844,
        V845,
        V846,
        V847,
        V848,
        V849,
        V850,
        V851,
        V852,
        V853,
        V854,
        V855,
        V856,
        V857,
        V858,
        V859,
        V860,
        V861,
        V862,
        V863,
        V864,
        V865,
        V866,
        V867,
        V868,
        V869,
        V870,
        V871,
        V872,
        V873,
        V874,
        V875,
        V876,
        V877,
        V878,
        V879,
        V880,
        V881,
        V882,
        V883,
        V884,
        V885,
        V886,
        V887,
        V888,
        V889,
        V890,
        V891,
        V892,
        V893,
        V894,
        V895,
        V896,
        V897,
        V898,
        V899,
        V900,
        V901,
        V902,
        V903,
        V904,
        V905,
        V906,
        V907,
        V908,
        V909,
        V910,
        V911,
        V912,
        V913,
        V914,
        V915,
        V916,
        V917,
        V918,
        V919,
        V920,
        V921,
        V922,
        V923,
        V924,
        V925,
        V926,
        V927,
        V928,
        V929,
        V930,
        V931,
        V932,
        V933,
        V934,
        V935,
        V936,
        V937,
        V938,
        V939,
        V940,
        V941,
        V942,
        V943,
        V944,
        V945,
        V946,
        V947,
        V948,
        V949,
        V950,
        V951,
        V952,
        V953,
        V954,
        V955,
        V956,
        V957,
        V958,
        V959,
        V960,
        V961,
        V962,
        V963,
        V964,
        V965,
        V966,
        V967,
        V968,
        V969,
        V970,
        V971,
        V972,
        V973,
        V974,
        V975,
        V976,
        V977,
        V978,
        V979,
        V980,
        V981,
        V982,
        V983,
        V984,
        V985,
        V986,
        V987,
        V988,
        V989,
        V990,
        V991,
        V992,
        V993,
        V994,
        V995,
        V996,
        V997,
        V998,
        V999,
        V1000,
        V1001,
        V1002,
        V1003,
        V1004,
        V1005,
        V1006,
        V1007,
        V1008,
        V1009,
        V1010,
        V1011,
        V1012,
        V1013,
        V1014,
        V1015,
        V1016,
        V1017,
        V1018,
        V1019,
        V1020,
        V1021,
        V1022,
        V1023,
        V1024,
        V1025,
        V1026,
        V1027,
        V1028,
        V1029,
        V1030,
        V1031,
        V1032,
        V1033,
        V1034,
        V1035,
        V1036,
        V1037,
        V1038,
        V1039,
        V1040,
        V1041,
        V1042,
        V1043,
        V1044,
        V1045,
        V1046,
        V1047,
        V1048,
        V1049,
        V1050,
        V1051,
        V1052,
        V1053,
        V1054,
        V1055,
        V1056,
        V1057,
        V1058,
        V1059,
        V1060,
        V1061,
        V1062,
        V1063,
        V1064,
        V1065,
        V1066,
        V1067,
        V1068,
        V1069,
        V1070,
        V1071,
        V1072,
        V1073,
        V1074,
        V1075,
        V1076,
        V1077,
        V1078,
        V1079,
        V1080,
        V1081,
        V1082,
        V1083,
        V1084,
        V1085,
        V1086,
        V1087,
        V1088,
        V1089,
        V1090,
        V1091,
        V1092,
        V1093,
        V1094,
        V1095,
        V1096,
        V1097,
        V1098,
        V1099,
        V1100,
        V1101,
        V1102,
        V1103,
        V1104,
        V1105,
        V1106,
        V1107,
        V1108,
        V1109,
        V1110,
        V1111,
        V1112,
        V1113,
        V1114,
        V1115,
        V1116,
        V1117,
        V1118,
        V1119,
        V1120,
        V1121,
        V1122,
        V1123,
        V1124,
        V1125,
        V1126,
        V1127,
        V1128,
        V1129,
        V1130,
        V1131,
        V1132,
        V1133,
        V1134,
        V1135,
        V1136,
        V1137,
        V1138,
        V1139,
        V1140,
        V1141,
        V1142,
        V1143,
        V1144,
        V1145,
        V1146,
        V1147,
        V1148,
        V1149,
        V1150,
        V1151,
        V1152,
        V1153,
        V1154,
        V1155,
        V1156,
        V1157,
        V1158,
        V1159,
        V1160,
        V1161,
        V1162,
        V1163,
        V1164,
        V1165,
        V1166,
        V1167,
        V1168,
        V1169,
        V1170,
        V1171,
        V1172,
        V1173,
        V1174,
        V1175,
        V1176,
        V1177,
        V1178,
        V1179,
        V1180,
        V1181,
        V1182,
        V1183,
        V1184,
        V1185,
        V1186,
        V1187,
        V1188,
        V1189,
        V1190,
        V1191,
        V1192,
        V1193,
        V1194,
        V1195,
        V1196,
        V1197,
        V1198,
        V1199,
        V1200,
        V1201,
        V1202,
        V1203,
        V1204,
        V1205,
        V1206,
        V1207,
        V1208,
        V1209,
        V1210,
        V1211,
        V1212,
        V1213,
        V1214,
        V1215,
        V1216,
        V1217,
        V1218,
        V1219,
        V1220,
        V1221,
        V1222,
        V1223,
        V1224,
        V1225,
        V1226,
        V1227,
        V1228,
        V1229,
        V1230,
        V1231,
        V1232,
        V1233,
        V1234,
        V1235,
        V1236,
        V1237,
        V1238,
        V1239,
        V1240,
        V1241,
        V1242,
        V1243,
        V1244,
        V1245,
        V1246,
        V1247,
        V1248,
        V1249,
        V1250,
        V1251,
        V1252,
        V1253,
        V1254,
        V1255,
        V1256,
        V1257,
        V1258,
        V1259,
        V1260,
        V1261,
        V1262,
        V1263,
        V1264,
        V1265,
        V1266,
        V1267,
        V1268,
        V1269,
        V1270,
        V1271,
        V1272,
        V1273,
        V1274,
        V1275,
        V1276,
        V1277,
        V1278,
        V1279,
        V1280,
        V1281,
        V1282,
        V1283,
        V1284,
        V1285,
        V1286,
        V1287,
        V1288,
        V1289,
        V1290,
        V1291,
        V1292,
        V1293,
        V1294,
        V1295,
        V1296,
        V1297,
        V1298,
        V1299,
        V1300,
        V1301,
        V1302,
        V1303,
        V1304,
        V1305,
        V1306,
        V1307,
        V1308,
        V1309,
        V1310,
        V1311,
        V1312,
        V1313,
        V1314,
        V1315,
        V1316,
        V1317,
        V1318,
        V1319,
        V1320,
        V1321,
        V1322,
        V1323,
        V1324,
        V1325,
        V1326,
        V1327,
        V1328,
        V1329,
        V1330,
        V1331,
        V1332,
        V1333,
        V1334,
        V1335,
        V1336,
        V1337,
        V1338,
        V1339,
        V1340,
        V1341,
        V1342,
        V1343,
        V1344,
        V1345,
        V1346,
        V1347,
        V1348,
        V1349,
        V1350,
        V1351,
        V1352,
        V1353,
        V1354,
        V1355,
        V1356,
        V1357,
        V1358,
        V1359,
        V1360,
        V1361,
        V1362,
        V1363,
        V1364,
        V1365,
        V1366,
        V1367,
        V1368,
        V1369,
        V1370,
        V1371,
        V1372,
        V1373,
        V1374,
        V1375,
        V1376,
        V1377,
        V1378,
        V1379,
        V1380,
        V1381,
        V1382,
        V1383,
        V1384,
        V1385,
        V1386,
        V1387,
        V1388,
        V1389,
        V1390,
        V1391,
        V1392,
        V1393,
        V1394,
        V1395,
        V1396,
        V1397,
        V1398,
        V1399,
        V1400,
        V1401,
        V1402,
        V1403,
        V1404,
        V1405,
        V1406,
        V1407,
        V1408,
        V1409,
        V1410,
        V1411,
        V1412,
        V1413,
        V1414,
        V1415,
        V1416,
        V1417,
        V1418,
        V1419,
        V1420,
        V1421,
        V1422,
        V1423,
        V1424,
        V1425,
        V1426,
        V1427,
        V1428,
        V1429,
        V1430,
        V1431,
        V1432,
        V1433,
        V1434,
        V1435,
        V1436,
        V1437,
        V1438,
        V1439,
        V1440,
        V1441,
        V1442,
        V1443,
        V1444,
        V1445,
        V1446,
        V1447,
        V1448,
        V1449,
        V1450,
        V1451,
        V1452,
        V1453,
        V1454,
        V1455,
        V1456,
        V1457,
        V1458,
        V1459,
        V1460,
        V1461,
        V1462,
        V1463,
        V1464,
        V1465,
        V1466,
        V1467,
        V1468,
        V1469,
        V1470,
        V1471,
        V1472,
        V1473,
        V1474,
        V1475,
        V1476,
        V1477,
        V1478,
        V1479,
        V1480,
        V1481,
        V1482,
        V1483,
        V1484,
        V1485,
        V1486,
        V1487,
        V1488,
        V1489,
        V1490,
        V1491,
        V1492,
        V1493,
        V1494,
        V1495,
        V1496,
        V1497,
        V1498,
        V1499,
    }

    #[test]
    fn large_enum_compiles_and_roundtrips() {
        assert_eq!(
            LargeEnum::try_from_byte_array(LargeEnum::V0.to_byte_array()).unwrap(),
            LargeEnum::V0
        );
        assert_eq!(
            LargeEnum::try_from_byte_array(LargeEnum::V750.to_byte_array()).unwrap(),
            LargeEnum::V750
        );
        assert_eq!(
            LargeEnum::try_from_byte_array(LargeEnum::V1499.to_byte_array()).unwrap(),
            LargeEnum::V1499
        );
    }
}

mod fingerprint {
    use byteable::{Byteable, WireFingerprint};

    #[derive(Byteable)]
    enum Declared {
        A,
        B(u32),
    }

    #[derive(Byteable)]
    enum Reordered {
        #[byteable(tag = 1)]
        B(u32),
        #[byteable(tag = 0)]
        A,
    }

    #[test]
    fn wire_compatible_enums_share_a_fingerprint() {
        assert_eq!(Declared::WIRE_FINGERPRINT, Reordered::WIRE_FINGERPRINT);
    }

    #[derive(Byteable)]
    enum DifferentPayload {
        A,
        B(u64), // different width from Declared's B(u32)
    }

    #[test]
    fn structurally_different_enums_differ() {
        assert_ne!(
            Declared::WIRE_FINGERPRINT,
            DifferentPayload::WIRE_FINGERPRINT
        );
    }

    #[test]
    fn option_matches_an_equivalent_hand_derived_enum() {
        // Same shape as Option<u32>: unit variant tagged 0, one-field variant tagged 1.
        assert_eq!(Declared::WIRE_FINGERPRINT, Option::<u32>::WIRE_FINGERPRINT);
    }

    #[derive(Byteable)]
    enum TagSortedA {
        #[byteable(tag = 0)]
        First,
        #[byteable(tag = 1)]
        Second(u32),
    }

    #[derive(Byteable)]
    enum TagSortedB {
        // Same tags, declared in the opposite order - must fingerprint identically,
        // since the wire format is fully pinned by `tag` and doesn't depend on
        // declaration order at all.
        #[byteable(tag = 1)]
        Second(u32),
        #[byteable(tag = 0)]
        First,
    }

    #[test]
    fn fingerprint_is_invariant_under_declaration_reorder_when_tags_are_pinned() {
        assert_eq!(TagSortedA::WIRE_FINGERPRINT, TagSortedB::WIRE_FINGERPRINT);
    }

    // Regression test for a real collision found in review: the enum fingerprint shell only
    // folded `.width(...)`, never `.endianness(...)`, so `ProtocolLE`/`ProtocolBE` (this file's
    // own `#[byteable(little_endian)]`/`#[byteable(big_endian)]` multi-byte-discriminant
    // fixtures, defined at the top of this file) fingerprinted identically despite producing
    // genuinely different wire bytes. First confirm the byte-level difference is real (not
    // incidental), then confirm the fingerprints now differ because of it.
    #[test]
    fn discriminant_endianness_changes_the_fingerprint() {
        use byteable::ToByteArray;

        assert_ne!(
            super::ProtocolLE::Tcp.to_byte_array(),
            super::ProtocolBE::Tcp.to_byte_array(),
            "sanity check: these must actually encode differently, or this test would prove nothing"
        );
        assert_ne!(
            super::ProtocolLE::WIRE_FINGERPRINT,
            super::ProtocolBE::WIRE_FINGERPRINT
        );
    }

    // The test above only covers `unit_enum_derive`. The endianness fold lives in the shared
    // `wire_fingerprint_impl_for_enum` shell, but `enum_derive` (any enum with at least one
    // data-carrying variant) is a *separate* call site that resolves and passes `endian_attr`
    // itself - so it needs its own proof that the fold actually reaches it.
    #[derive(Byteable)]
    #[byteable(discriminant = u16, little_endian)]
    enum PayloadEnumLE {
        Ping,
        Data(u32),
    }

    #[derive(Byteable)]
    #[byteable(discriminant = u16, big_endian)]
    enum PayloadEnumBE {
        Ping,
        Data(u32),
    }

    #[test]
    fn discriminant_endianness_changes_the_fingerprint_on_the_field_carrying_derive_path() {
        use byteable::io::WriteValue;

        // Sanity check first, exactly as the unit-enum test above does: prove the two really
        // do put different bytes on the wire, so the fingerprint difference means something.
        let mut le = Vec::new();
        le.write_value(&PayloadEnumLE::Data(7)).unwrap();
        let mut be = Vec::new();
        be.write_value(&PayloadEnumBE::Data(7)).unwrap();
        assert_ne!(
            le, be,
            "sanity check: these must actually encode differently, or this test would prove nothing"
        );

        assert_ne!(
            PayloadEnumLE::WIRE_FINGERPRINT,
            PayloadEnumBE::WIRE_FINGERPRINT
        );
    }

    // Regression test for a real truncation found in review: the per-variant tag fold was
    // `#tag_ref as u64`, which throws away the top 64 bits of a `u128`/`i128` discriminant.
    // These two enums differ *only* in the high half of `Big`'s tag, and so fingerprinted
    // identically despite encoding genuinely different discriminant bytes.
    #[derive(Byteable)]
    #[byteable(discriminant = u128)]
    enum WideLow {
        Zero,
        #[byteable(tag = 1)]
        Big,
    }

    #[derive(Byteable)]
    #[byteable(discriminant = u128)]
    enum WideHigh {
        Zero,
        #[byteable(tag = 1 + (1u128 << 64))]
        Big,
    }

    #[test]
    fn u128_discriminant_high_bits_reach_the_fingerprint() {
        use byteable::ToByteArray;

        assert_ne!(
            WideLow::Big.to_byte_array(),
            WideHigh::Big.to_byte_array(),
            "sanity check: these must actually encode differently, or this test would prove nothing"
        );
        assert_ne!(WideLow::WIRE_FINGERPRINT, WideHigh::WIRE_FINGERPRINT);
    }

    #[derive(Byteable)]
    #[byteable(discriminant = i128)]
    enum SignedWideA {
        Zero,
        #[byteable(tag = -1)]
        Neg,
    }

    #[derive(Byteable)]
    #[byteable(discriminant = i128)]
    enum SignedWideB {
        Zero,
        #[byteable(tag = 1)]
        Pos,
    }

    #[test]
    fn i128_discriminants_are_distinguished_across_all_128_bits() {
        // `-1i128` and `1i128` agree in neither half, but a fold that only kept the low 64
        // bits of the *unsigned* reinterpretation would still separate them; what this pins
        // is that signed 128-bit tags go through the same two-half path at all and don't
        // regress to a lossy single `as u64`.
        assert_ne!(SignedWideA::WIRE_FINGERPRINT, SignedWideB::WIRE_FINGERPRINT);
    }

    #[derive(Byteable)]
    #[byteable(discriminant = i8)]
    enum SignedTag {
        Zero,
        #[byteable(tag = -1)]
        MinusOne,
    }

    #[derive(Byteable)]
    #[byteable(discriminant = u8)]
    enum UnsignedTag {
        Zero,
        #[byteable(tag = 255)]
        TwoFiftyFive,
    }

    #[test]
    fn discriminant_signedness_is_folded_into_the_fingerprint() {
        use byteable::ToByteArray;

        // These two encode byte-identically - `-1i8` and `255u8` are the same 0xFF - so the
        // *only* thing separating their fingerprints is the `Signedness` the enum shell now
        // folds in. That mirrors how every fixed-width integer is already fingerprinted
        // (`u32` and `i32` differ by `Signedness` alone despite identical bytes and identical
        // acceptance), and it is what the tag fold's own unsigned normalization relies on:
        // the tag is hashed as its wire bit pattern, and signedness is recorded separately
        // rather than smuggled in via sign extension.
        assert_eq!(
            SignedTag::MinusOne.to_byte_array(),
            UnsignedTag::TwoFiftyFive.to_byte_array(),
            "sanity check: these must encode identically, or this test would prove nothing"
        );
        assert_ne!(SignedTag::WIRE_FINGERPRINT, UnsignedTag::WIRE_FINGERPRINT);
    }

    #[derive(Byteable)]
    enum TwoVariants {
        A,
        B,
    }

    #[derive(Byteable)]
    enum ThreeVariants {
        A,
        B,
        C,
    }

    #[test]
    fn variant_count_is_folded_into_the_fingerprint() {
        // `fold_unordered` is a plain `wrapping_add`, so the variant set alone is in principle
        // summable to the same value by a different set. The shell folds `hashes.len()`
        // alongside it as cheap extra hardening; this pins that the count is actually there.
        assert_ne!(
            TwoVariants::WIRE_FINGERPRINT,
            ThreeVariants::WIRE_FINGERPRINT
        );
    }
}

mod fingerprint_assertion {
    // Exercises `unit_enum_derive` specifically (a different derive branch from
    // `tests/derive_structs.rs`'s `fingerprint_assertion` module, which only covers
    // `fixed_struct_derived`'s non-unit-struct branch) - `#[byteable(fingerprint = ..)]` is
    // wired into all four derive functions and this is the regression coverage for one of the
    // other three. This enum's asserted value must be the actual WIRE_FINGERPRINT for this
    // exact 2-unit-variant enum at the time this test is written; if it fails to compile,
    // read the real value out of the compile error and paste it in.
    #[derive(byteable::Byteable)]
    #[byteable(fingerprint = "0x65bae08f0da6605c")]
    enum Checked {
        A,
        B,
    }

    #[test]
    fn pins_value() {
        assert_eq!(
            <Checked as byteable::WireFingerprint>::WIRE_FINGERPRINT,
            0x65bae08f0da6605c
        );
    }
}
