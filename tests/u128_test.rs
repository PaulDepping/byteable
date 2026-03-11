/// Tests for u128/i128 primitive types and enum derives.
///
/// u128/i128 use native-endian byte order for the raw primitive trait implementations,
/// just like all other multi-byte primitive types in the crate.

use byteable::{AssociatedByteArray, FromByteArray, IntoByteArray};

// ============================================================================
// u128 primitive tests
// ============================================================================

#[test]
fn test_u128_roundtrip() {
    let values: [u128; 5] = [
        0,
        1,
        u128::MAX,
        0x0102030405060708090A0B0C0D0E0F10,
        u128::MAX / 2,
    ];
    for val in values {
        let bytes = val.into_byte_array();
        let restored = u128::from_byte_array(bytes);
        assert_eq!(restored, val, "Roundtrip failed for {}", val);
    }
}

#[test]
fn test_u128_byte_size() {
    assert_eq!(u128::BYTE_SIZE, 16);
}

#[test]
fn test_u128_byte_layout() {
    // u128 uses native-endian encoding
    let val: u128 = 1;
    let bytes = val.into_byte_array();
    assert_eq!(bytes, 1u128.to_ne_bytes());
}

// ============================================================================
// i128 primitive tests
// ============================================================================

#[test]
fn test_i128_roundtrip() {
    let values: [i128; 5] = [
        0,
        1,
        -1,
        i128::MAX,
        i128::MIN,
    ];
    for val in values {
        let bytes = val.into_byte_array();
        let restored = i128::from_byte_array(bytes);
        assert_eq!(restored, val, "Roundtrip failed for {}", val);
    }
}

#[test]
fn test_i128_byte_size() {
    assert_eq!(i128::BYTE_SIZE, 16);
}

#[test]
fn test_i128_negative_values() {
    let val: i128 = -1_000_000_000_000_000_000_000_000_000;
    let bytes = val.into_byte_array();
    let restored = i128::from_byte_array(bytes);
    assert_eq!(restored, val);
}

// ============================================================================
// u128 enum derive tests
// ============================================================================

#[cfg(feature = "derive")]
mod derive_tests {
    use byteable::{AssociatedByteArray, Byteable, DiscriminantValue, IntoByteArray, TryFromByteArray};

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    #[repr(u128)]
    #[byteable(little_endian)]
    enum LargeU128 {
        Zero = 0,
        Mid = 0x8000_0000_0000_0000_0000_0000_0000_0000,
        Max = u128::MAX,
    }

    #[test]
    fn test_u128_enum_roundtrip() {
        let variants = [LargeU128::Zero, LargeU128::Mid, LargeU128::Max];
        for v in variants {
            let bytes = v.into_byte_array();
            let restored = LargeU128::try_from_byte_array(bytes).unwrap();
            assert_eq!(restored, v);
        }
    }

    #[test]
    fn test_u128_enum_byte_size() {
        assert_eq!(LargeU128::BYTE_SIZE, 16);
    }

    #[test]
    fn test_u128_enum_byte_layout() {
        assert_eq!(LargeU128::Zero.into_byte_array(), 0u128.to_le_bytes());
        assert_eq!(LargeU128::Max.into_byte_array(), u128::MAX.to_le_bytes());
    }

    #[test]
    fn test_u128_enum_invalid_discriminant() {
        let bytes = 1u128.to_le_bytes(); // Not a valid variant
        let result = LargeU128::try_from_byte_array(bytes);
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(err.invalid_discriminant, DiscriminantValue::U128(1));
        }
    }

    // ============================================================================
    // i128 enum derive tests
    // ============================================================================

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    #[repr(i128)]
    #[byteable(little_endian)]
    enum SignedI128 {
        MinVal = i128::MIN,
        Zero = 0,
        MaxVal = i128::MAX,
    }

    #[test]
    fn test_i128_enum_roundtrip() {
        let variants = [SignedI128::MinVal, SignedI128::Zero, SignedI128::MaxVal];
        for v in variants {
            let bytes = v.into_byte_array();
            let restored = SignedI128::try_from_byte_array(bytes).unwrap();
            assert_eq!(restored, v);
        }
    }

    #[test]
    fn test_i128_enum_byte_size() {
        assert_eq!(SignedI128::BYTE_SIZE, 16);
    }

    #[test]
    fn test_i128_enum_byte_layout() {
        assert_eq!(SignedI128::MinVal.into_byte_array(), i128::MIN.to_le_bytes());
        assert_eq!(SignedI128::Zero.into_byte_array(), 0i128.to_le_bytes());
        assert_eq!(SignedI128::MaxVal.into_byte_array(), i128::MAX.to_le_bytes());
    }

    #[test]
    fn test_i128_enum_invalid_discriminant() {
        let bytes = 1i128.to_le_bytes(); // Not a valid variant
        let result = SignedI128::try_from_byte_array(bytes);
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(err.invalid_discriminant, DiscriminantValue::I128(1));
        }
    }

    // big-endian u128 enum
    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    #[repr(u128)]
    #[byteable(big_endian)]
    enum BigEndianU128 {
        Low = 0x0001,
        High = 0xFFFF,
    }

    #[test]
    fn test_u128_enum_big_endian() {
        assert_eq!(BigEndianU128::Low.into_byte_array(), 0x0001u128.to_be_bytes());
        assert_eq!(BigEndianU128::High.into_byte_array(), 0xFFFFu128.to_be_bytes());

        let restored = BigEndianU128::try_from_byte_array(0x0001u128.to_be_bytes()).unwrap();
        assert_eq!(restored, BigEndianU128::Low);
    }

    #[test]
    fn test_discriminant_value_u128_display() {
        let bytes = 2u128.to_le_bytes();
        let result = LargeU128::try_from_byte_array(bytes);
        if let Err(err) = result {
            let s = format!("{}", err);
            assert!(s.contains("2"));
            assert!(s.contains("u128"));
        }
    }

    #[test]
    fn test_discriminant_value_i128_display() {
        let bytes = 2i128.to_le_bytes();
        let result = SignedI128::try_from_byte_array(bytes);
        if let Err(err) = result {
            let s = format!("{}", err);
            assert!(s.contains("2"));
            assert!(s.contains("i128"));
        }
    }
}
