//! Integration tests for ordered-float support.
#![cfg(feature = "ordered-float")]

use byteable::{EndianConvert, FromByteArray, IntoByteArray, LittleEndian, TransmuteSafe, TryFromByteArray};
use ordered_float::{NotNan, OrderedFloat};

// --- OrderedFloat<f32> ---

#[test]
fn ordered_float_f32_roundtrip() {
    let val = OrderedFloat(1.5f32);
    let bytes = val.into_byte_array();
    let restored = OrderedFloat::<f32>::from_byte_array(bytes);
    assert_eq!(val, restored);
}

#[test]
fn ordered_float_f32_nan_roundtrip() {
    // OrderedFloat supports NaN — it should survive a roundtrip.
    let val = OrderedFloat(f32::NAN);
    let bytes = val.into_byte_array();
    let restored = OrderedFloat::<f32>::from_byte_array(bytes);
    assert!(restored.is_nan());
}

#[test]
fn ordered_float_f32_byte_size() {
    use byteable::ByteRepr;
    assert_eq!(OrderedFloat::<f32>::BYTE_SIZE, 4);
}

#[test]
fn ordered_float_f32_endian_convert() {
    let val = OrderedFloat(1.0f32);
    let le = val.to_le();
    let be = val.to_be();
    assert_eq!(OrderedFloat::<f32>::from_le(le), val);
    assert_eq!(OrderedFloat::<f32>::from_be(be), val);
}

#[test]
fn ordered_float_f32_in_little_endian_wrapper() {
    let val: LittleEndian<OrderedFloat<f32>> = OrderedFloat(42.5f32).into();
    assert_eq!(val.get(), OrderedFloat(42.5f32));
    let bytes = val.into_byte_array();
    let restored = LittleEndian::<OrderedFloat<f32>>::from_byte_array(bytes);
    assert_eq!(restored.get(), OrderedFloat(42.5f32));
}

// --- OrderedFloat<f64> ---

#[test]
fn ordered_float_f64_roundtrip() {
    let val = OrderedFloat(2.718281828f64);
    let bytes = val.into_byte_array();
    let restored = OrderedFloat::<f64>::from_byte_array(bytes);
    assert_eq!(val, restored);
}

#[test]
fn ordered_float_f64_nan_roundtrip() {
    let val = OrderedFloat(f64::NAN);
    let bytes = val.into_byte_array();
    let restored = OrderedFloat::<f64>::from_byte_array(bytes);
    assert!(restored.is_nan());
}

#[test]
fn ordered_float_f64_byte_size() {
    use byteable::ByteRepr;
    assert_eq!(OrderedFloat::<f64>::BYTE_SIZE, 8);
}

// --- NotNan<f32> ---

#[test]
fn not_nan_f32_roundtrip() {
    let val = NotNan::new(3.14f32).unwrap();
    let bytes = val.into_byte_array();
    let restored = NotNan::<f32>::try_from_byte_array(bytes).unwrap();
    assert_eq!(val, restored);
}

#[test]
fn not_nan_f32_nan_rejected() {
    let nan_bytes = f32::NAN.to_ne_bytes();
    let result = NotNan::<f32>::try_from_byte_array(nan_bytes);
    assert!(result.is_err());
}

#[test]
fn not_nan_f32_byte_size() {
    use byteable::ByteRepr;
    assert_eq!(NotNan::<f32>::BYTE_SIZE, 4);
}

// --- NotNan<f64> ---

#[test]
fn not_nan_f64_roundtrip() {
    let val = NotNan::new(2.718281828f64).unwrap();
    let bytes = val.into_byte_array();
    let restored = NotNan::<f64>::try_from_byte_array(bytes).unwrap();
    assert_eq!(val, restored);
}

#[test]
fn not_nan_f64_nan_rejected() {
    let nan_bytes = f64::NAN.to_ne_bytes();
    let result = NotNan::<f64>::try_from_byte_array(nan_bytes);
    assert!(result.is_err());
}

#[test]
fn not_nan_f64_byte_size() {
    use byteable::ByteRepr;
    assert_eq!(NotNan::<f64>::BYTE_SIZE, 8);
}

// --- TransmuteSafe ---

#[test]
fn ordered_float_transmute_safe() {
    // Compile-time check: these types implement TransmuteSafe
    fn assert_transmute_safe<T: TransmuteSafe>() {}
    assert_transmute_safe::<LittleEndian<OrderedFloat<f32>>>();
    assert_transmute_safe::<LittleEndian<OrderedFloat<f64>>>();
    assert_transmute_safe::<byteable::BigEndian<OrderedFloat<f32>>>();
    assert_transmute_safe::<byteable::BigEndian<OrderedFloat<f64>>>();
}

// --- #[derive(Byteable)] integration ---

#[cfg(feature = "derive")]
mod derive_tests {
    use byteable::{Byteable, FromByteArray, IntoByteArray};
    use ordered_float::OrderedFloat;

    // OrderedFloat fields require explicit endianness annotation (the derive macro's
    // auto-wrap only recognises bare primitive idents like `f32`, not `OrderedFloat<f32>`).
    #[derive(Byteable, Debug, Clone, Copy, PartialEq)]
    struct SensorReading {
        id: u8,
        #[byteable(little_endian)]
        temperature: OrderedFloat<f32>,
        #[byteable(big_endian)]
        pressure: OrderedFloat<f64>,
    }

    #[test]
    fn derive_byteable_roundtrip() {
        let reading = SensorReading {
            id: 7,
            temperature: OrderedFloat(36.6),
            pressure: OrderedFloat(101325.0),
        };
        let bytes = reading.into_byte_array();
        let restored = SensorReading::from_byte_array(bytes);
        assert_eq!(reading, restored);
    }

    #[test]
    fn derive_byteable_nan_roundtrip() {
        let reading = SensorReading {
            id: 0,
            temperature: OrderedFloat(f32::NAN),
            pressure: OrderedFloat(f64::NAN),
        };
        let bytes = reading.into_byte_array();
        let restored = SensorReading::from_byte_array(bytes);
        assert!(restored.temperature.is_nan());
        assert!(restored.pressure.is_nan());
    }
}
