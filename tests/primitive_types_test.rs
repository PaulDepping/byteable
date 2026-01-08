use byteable::{AssociatedByteArray, IntoByteArray, TryFromByteArray};

// ============================================================================
// BOOL TESTS
// ============================================================================

#[test]
fn test_bool_true_to_bytes() {
    let value = true;
    let bytes = value.into_byte_array();
    assert_eq!(bytes, [1]);
}

#[test]
fn test_bool_false_to_bytes() {
    let value = false;
    let bytes = value.into_byte_array();
    assert_eq!(bytes, [0]);
}

#[test]
fn test_bool_from_bytes_true() {
    let bytes = [1];
    let value = bool::try_from_byte_array(bytes).unwrap();
    assert_eq!(value, true);
}

#[test]
fn test_bool_from_bytes_false() {
    let bytes = [0];
    let value = bool::try_from_byte_array(bytes).unwrap();
    assert_eq!(value, false);
}

#[test]
fn test_bool_invalid_byte() {
    // Any value other than 0 or 1 should be invalid
    let invalid_values = [2, 3, 10, 42, 100, 255];

    for invalid in invalid_values {
        let bytes = [invalid];
        let result = bool::try_from_byte_array(bytes);
        assert!(result.is_err(), "Expected error for byte value {}", invalid);

        if let Err(err) = result {
            assert_eq!(
                err.invalid_discriminant,
                byteable::Discriminant::U8(invalid)
            );
            let error_string = format!("{}", err);
            assert!(error_string.contains("Invalid discriminant"));
            assert!(error_string.contains("bool"));
        }
    }
}

#[test]
fn test_bool_roundtrip() {
    let values = [true, false];

    for value in values {
        let bytes = value.into_byte_array();
        let restored = bool::try_from_byte_array(bytes).unwrap();
        assert_eq!(restored, value);
    }
}

#[test]
fn test_bool_byte_size() {
    assert_eq!(bool::BYTE_SIZE, 1);
}

#[cfg(feature = "std")]
#[test]
fn test_bool_error_trait() {
    use std::error::Error;

    let bytes = [2];
    let result = bool::try_from_byte_array(bytes);

    if let Err(err) = result {
        // Test that it implements std::error::Error
        let _: &dyn Error = &err;
        assert!(format!("{:?}", err).contains("EnumFromBytesError"));
    } else {
        panic!("Expected an error");
    }
}

// ============================================================================
// CHAR TESTS
// ============================================================================

#[test]
fn test_char_ascii_to_bytes() {
    let value = 'A';
    let bytes = value.into_byte_array();
    // 'A' is U+0041, stored as little-endian u32
    assert_eq!(bytes, [0x41, 0x00, 0x00, 0x00]);
}

#[test]
fn test_char_unicode_to_bytes() {
    let value = '🦀'; // Rust crab emoji U+1F980
    let bytes = value.into_byte_array();
    // 0x1F980 in little-endian u32
    assert_eq!(bytes, [0x80, 0xF9, 0x01, 0x00]);
}

#[test]
fn test_char_from_bytes_ascii() {
    let bytes = [0x41, 0x00, 0x00, 0x00]; // 'A'
    let value = char::try_from_byte_array(bytes).unwrap();
    assert_eq!(value, 'A');
}

#[test]
fn test_char_from_bytes_unicode() {
    let bytes = [0x80, 0xF9, 0x01, 0x00]; // U+1F980 '🦀'
    let value = char::try_from_byte_array(bytes).unwrap();
    assert_eq!(value, '🦀');
}

#[test]
fn test_char_various_valid_values() {
    let test_cases = [
        ('a', [0x61, 0x00, 0x00, 0x00]),
        ('Z', [0x5A, 0x00, 0x00, 0x00]),
        ('0', [0x30, 0x00, 0x00, 0x00]),
        ('9', [0x39, 0x00, 0x00, 0x00]),
        (' ', [0x20, 0x00, 0x00, 0x00]),
        ('€', [0xAC, 0x20, 0x00, 0x00]),  // U+20AC
        ('中', [0x2D, 0x4E, 0x00, 0x00]), // U+4E2D
        ('😀', [0x00, 0xF6, 0x01, 0x00]), // U+1F600
    ];

    for (ch, expected_bytes) in test_cases {
        let bytes = ch.into_byte_array();
        assert_eq!(bytes, expected_bytes, "Failed for char '{}'", ch);

        let restored = char::try_from_byte_array(bytes).unwrap();
        assert_eq!(restored, ch);
    }
}

#[test]
fn test_char_invalid_code_point() {
    // Unicode code points range from 0 to 0x10FFFF
    // Values 0xD800 to 0xDFFF are invalid (surrogate pairs)
    // Values above 0x10FFFF are invalid

    let invalid_values = [
        [0x00, 0xD8, 0x00, 0x00], // U+D800 (surrogate)
        [0xFF, 0xDF, 0x00, 0x00], // U+DFFF (surrogate)
        [0x00, 0x00, 0x11, 0x00], // U+110000 (too large)
        [0xFF, 0xFF, 0xFF, 0xFF], // 0xFFFFFFFF (invalid)
    ];

    for bytes in invalid_values {
        let result = char::try_from_byte_array(bytes);
        assert!(result.is_err(), "Expected error for bytes {:?}", bytes);

        if let Err(err) = result {
            let error_string = format!("{}", err);
            assert!(error_string.contains("Invalid discriminant"));
            assert!(error_string.contains("char"));
        }
    }
}

#[test]
fn test_char_roundtrip() {
    let test_chars = [
        'a', 'Z', '0', '9', ' ', '!', '@', '#', '€', '£', '¥', '©', '®', '™', 'α', 'β', 'γ', 'π',
        'Σ', '中', '日', '本', '語', '🦀', '🚀', '🌟', '❤', '✓',
    ];

    for ch in test_chars {
        let bytes = ch.into_byte_array();
        let restored = char::try_from_byte_array(bytes).unwrap();
        assert_eq!(restored, ch, "Roundtrip failed for '{}'", ch);
    }
}

#[test]
fn test_char_byte_size() {
    assert_eq!(char::BYTE_SIZE, 4);
}

#[test]
fn test_char_null_character() {
    let value = '\0'; // Null character U+0000
    let bytes = value.into_byte_array();
    assert_eq!(bytes, [0x00, 0x00, 0x00, 0x00]);

    let restored = char::try_from_byte_array(bytes).unwrap();
    assert_eq!(restored, '\0');
}

#[test]
fn test_char_max_valid_code_point() {
    // Maximum valid Unicode code point is U+10FFFF
    let value = '\u{10FFFF}';
    let bytes = value.into_byte_array();
    assert_eq!(bytes, [0xFF, 0xFF, 0x10, 0x00]); // 0x10FFFF in little-endian

    let restored = char::try_from_byte_array(bytes).unwrap();
    assert_eq!(restored, value);
}

#[cfg(feature = "std")]
#[test]
fn test_char_error_trait() {
    use std::error::Error;

    let bytes = [0xFF, 0xFF, 0xFF, 0xFF]; // Invalid code point
    let result = char::try_from_byte_array(bytes);

    if let Err(err) = result {
        // Test that it implements std::error::Error
        let _: &dyn Error = &err;
        assert!(format!("{:?}", err).contains("EnumFromBytesError"));
    } else {
        panic!("Expected an error");
    }
}

// ============================================================================
// INTEGRATION TESTS WITH STRUCTS
// ============================================================================

#[cfg(feature = "derive")]
mod derive_tests {
    use super::*;
    use byteable::Byteable;

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    struct BoolStruct {
        #[byteable(try_transparent)]
        flag1: bool,
        value: u8,
        #[byteable(try_transparent)]
        flag2: bool,
    }

    #[test]
    fn test_bool_in_struct() {
        let s = BoolStruct {
            flag1: true,
            value: 42,
            flag2: false,
        };

        let bytes = s.into_byte_array();
        assert_eq!(bytes, [1, 42, 0]);

        let restored = BoolStruct::try_from_byte_array(bytes).unwrap();
        assert_eq!(restored, s);
    }

    #[test]
    fn test_bool_in_struct_invalid() {
        let bytes = [2, 42, 0]; // Invalid bool value
        let result = BoolStruct::try_from_byte_array(bytes);
        assert!(result.is_err());
    }

    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    struct CharStruct {
        id: u8,
        #[byteable(try_transparent)]
        character: char,
        #[byteable(little_endian)]
        count: u16,
    }

    #[test]
    fn test_char_in_struct() {
        let s = CharStruct {
            id: 10,
            character: '🦀',
            count: 1000,
        };

        let bytes = s.into_byte_array();
        // id(1) + char(4) + count(2) = 7 bytes
        assert_eq!(bytes.len(), 7);
        assert_eq!(bytes[0], 10); // id
        assert_eq!(&bytes[1..5], &[0x80, 0xF9, 0x01, 0x00]); // '🦀'
        assert_eq!(&bytes[5..7], &[0xE8, 0x03]); // 1000 in little-endian

        let restored = CharStruct::try_from_byte_array(bytes).unwrap();
        assert_eq!(restored, s);
    }

    #[test]
    fn test_char_in_struct_invalid() {
        let mut bytes = [0u8; 7];
        bytes[0] = 10; // id
        bytes[1..5].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // Invalid char
        bytes[5..7].copy_from_slice(&[0xE8, 0x03]); // count

        let result = CharStruct::try_from_byte_array(bytes);
        assert!(result.is_err());
    }

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
    fn test_mixed_bool_char_struct() {
        let s = MixedPrimitives {
            is_active: true,
            symbol: '✓',
            count: 5,
            enabled: false,
        };

        let bytes = s.into_byte_array();
        assert_eq!(bytes.len(), 7); // bool(1) + char(4) + u8(1) + bool(1)

        let restored = MixedPrimitives::try_from_byte_array(bytes).unwrap();
        assert_eq!(restored, s);
    }
}
