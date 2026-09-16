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

    // NOTE: `#[byteable(tag = N)]` is `i128`-typed internally (see `attrs::parse_variant_tag`),
    // so no positive literal can express a `u128` value in the upper half of its range (`2^127`
    // to `u128::MAX`) - this is a real capability gap of the `tag` mechanism, not a bug. The
    // documented escape hatch (see `reinterpret_i128_for_repr`'s doc comment in `attrs.rs`) is
    // a *negative* tag, which reinterprets via two's-complement wraparound: `tag = -1` means
    // `u128::MAX` (every bit, including bit 127, the actual high bit of a `u128`, set). `Mid`
    // uses that escape hatch to genuinely exercise the high-bit-set case that this test cares
    // about, rather than the `2^126` substitute a previous revision used (which set bit 126,
    // not bit 127, and did not actually exercise a high-bit-set wire layout despite claiming to).
    //
    // `Max` uses `i128::MAX` directly (`2^127 - 1`, the largest value expressible as a positive
    // tag literal - still within the *lower* half of `u128`'s range). Earlier this was
    // substituted with `i128::MAX - 1`, because `attrs::resolve_variant_tags`'s counter-advance
    // used to run unconditionally after every variant, including the last, and `i128::MAX + 1`
    // overflowed. That was fixed (see `resolve_variant_tags`'s `checked_add`/`unwrap_or`) so the
    // counter no longer advances past `i128::MAX`; since `Max` is the last variant here, there
    // is no subsequent implicit variant to collide, and `tag = i128::MAX` now works directly.
    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    #[repr(u128)]
    #[byteable(little_endian)]
    enum LargeU128 {
        #[byteable(tag = 0)]
        Zero,
        // u128::MAX, reached via the negative-tag escape hatch (tag = -1); see NOTE above.
        #[byteable(tag = -1)]
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
        // `Mid` is tagged `-1`, reinterpreted as `u128::MAX` via the negative-tag escape hatch -
        // see the NOTE on `LargeU128`.
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

    // NOTE: `#[byteable(tag = N)]` parses a positive magnitude and then negates it (see
    // `attrs::parse_tag_expr`), so it can represent any value in `i128::MIN + 1 ..= i128::MAX`
    // but not `i128::MIN` itself - `i128::MIN`'s magnitude is one past what fits in a positive
    // `i128`, so `tag = -170141183460469231731687303715884105728` is a hard parse error by
    // design, not something we can work around here. `MinVal` is substituted with
    // `i128::MIN + 1`, the closest representable near-boundary value; this is unrelated to the
    // `MaxVal` note below and is not fixable via `tag` alone.
    //
    // `MaxVal` uses `i128::MAX` directly, unlike an earlier revision that substituted
    // `i128::MAX - 1`: `attrs::resolve_variant_tags`'s counter-advance used to run
    // unconditionally after every variant, including the last, so a tag of exactly `i128::MAX`
    // would overflow computing the (unused, since there's no next variant) implicit successor.
    // That's fixed now (`checked_add`/`unwrap_or`), so `tag = i128::MAX` works directly as the
    // last variant here.
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
        assert_eq!(Wide::Small.to_byte_array(), 0u16.to_ne_bytes());
        assert_eq!(Wide::Big.to_byte_array(), 999u16.to_ne_bytes());
        assert_eq!(
            Wide::try_from_byte_array(0u16.to_ne_bytes()).unwrap(),
            Wide::Small
        );
        assert_eq!(
            Wide::try_from_byte_array(999u16.to_ne_bytes()).unwrap(),
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
        assert_eq!(Signed::Negative.to_byte_array(), (-1i16).to_ne_bytes());
        assert_eq!(Signed::Zero.to_byte_array(), 0i16.to_ne_bytes());
        assert_eq!(
            Signed::try_from_byte_array((-1i16).to_ne_bytes()).unwrap(),
            Signed::Negative
        );
        assert_eq!(
            Signed::try_from_byte_array(0i16.to_ne_bytes()).unwrap(),
            Signed::Zero
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
            let mut expected = 999u16.to_ne_bytes().to_vec();
            expected.push(7);
            assert_eq!(buf, expected);

            let decoded: WideField = Cursor::new(&buf).read_value().unwrap();
            assert_eq!(decoded, WideField::Big { value: 7 });

            let mut small_buf = Vec::new();
            small_buf.write_value(&WideField::Small).unwrap();
            assert_eq!(small_buf, 0u16.to_ne_bytes());
            let decoded_small: WideField = Cursor::new(&small_buf).read_value().unwrap();
            assert_eq!(decoded_small, WideField::Small);
        }
    }
}
