//! Tests for `#[derive(Byteable)]` on structs: named structs with field
//! endianness attributes, tuple structs, unit structs, visibility modifiers,
//! the `transparent` field attribute, and compile-time safety validation.
#![cfg(feature = "derive")]

// ── Named structs with field endianness ───────────────────────────────────────

mod named_structs {
    use byteable::{Byteable, FromByteArray, IntoByteArray};

    #[derive(Clone, Copy, Byteable)]
    struct TestStruct {
        a: u8,
        b: u16,
        #[byteable(big_endian)]
        c: u64,
        d: f64,
    }

    fn make_test() -> TestStruct {
        TestStruct {
            a: 42,
            b: 0x1234,
            c: 0x0102030405060708,
            d: 3.14159,
        }
    }

    #[test]
    fn byte_size() {
        // u8(1) + u16(2) + u64(8) + f64(8) = 19
        assert_eq!(TestStruct::BYTE_SIZE, 19);
    }

    #[test]
    fn u8_field_layout() {
        let bytes = make_test().into_byte_array();
        assert_eq!(bytes[0], 42);
    }

    #[test]
    fn le_u16_field_layout() {
        let bytes = make_test().into_byte_array();
        assert_eq!(bytes[1], 0x34);
        assert_eq!(bytes[2], 0x12);
    }

    #[test]
    fn be_u64_field_layout() {
        let bytes = make_test().into_byte_array();
        assert_eq!(
            &bytes[3..11],
            &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        );
    }

    #[test]
    fn le_f64_field_layout() {
        let bytes = make_test().into_byte_array();
        let d_bytes: [u8; 8] = bytes[11..19].try_into().unwrap();
        assert_eq!(f64::from_le_bytes(d_bytes), 3.14159);
    }

    #[test]
    fn roundtrip() {
        let original = make_test();
        let bytes = original.into_byte_array();
        let restored = TestStruct::from_byte_array(bytes);
        assert_eq!(original.a, restored.a);
        assert_eq!(original.b, restored.b);
        assert_eq!(original.c, restored.c);
        assert_eq!(original.d, restored.d);
    }
}

// ── Tuple structs ─────────────────────────────────────────────────────────────

mod tuple_structs {
    use byteable::{Byteable, FromByteArray, IntoByteArray};

    #[derive(Clone, Copy, Byteable, Debug, PartialEq)]
    struct SimpleTuple(u8, u16, u32);

    #[derive(Clone, Copy, Byteable, Debug, PartialEq)]
    struct EndianTuple(u8, u16, #[byteable(big_endian)] u32, u64);

    #[derive(Clone, Copy, Byteable, Debug, PartialEq)]
    struct InnerTuple(u8, u16);

    #[derive(Clone, Copy, Byteable, Debug, PartialEq)]
    struct OuterTuple(InnerTuple, u8, #[byteable(big_endian)] u32);

    #[derive(Clone, Copy, Byteable, Debug, PartialEq)]
    struct ArrayTuple(u8, [u8; 4], u16);

    #[test]
    fn simple_tuple_roundtrip() {
        let tuple = SimpleTuple(42, 0x1234, 0x12345678);
        // u8(1) + u16(2) + u32(4) = 7 bytes
        let bytes = tuple.into_byte_array();
        assert_eq!(bytes.len(), 7);
        assert_eq!(SimpleTuple::from_byte_array(bytes), tuple);
    }

    #[test]
    fn endian_tuple_byte_layout() {
        let tuple = EndianTuple(42, 0x1234, 0x12345678, 0x0102030405060708);
        let bytes = tuple.into_byte_array();
        // u8(1) + u16(2) + u32(4) + u64(8) = 15 bytes
        assert_eq!(bytes.len(), 15);
        assert_eq!(bytes[0], 42);
        // LE u16
        assert_eq!(&bytes[1..3], &[0x34, 0x12]);
        // BE u32
        assert_eq!(&bytes[3..7], &[0x12, 0x34, 0x56, 0x78]);
        // LE u64
        assert_eq!(
            &bytes[7..15],
            &[0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]
        );
        assert_eq!(EndianTuple::from_byte_array(bytes), tuple);
    }

    #[test]
    fn nested_tuple_transparent_field() {
        let inner = InnerTuple(10, 0x1234);
        let outer = OuterTuple(inner, 42, 0x12345678);
        let bytes = outer.into_byte_array();
        // InnerTuple(3) + u8(1) + u32(4) = 8 bytes
        assert_eq!(bytes.len(), 8);
        // transparent InnerTuple at bytes 0-2
        assert_eq!(bytes[0], 10);
        assert_eq!(&bytes[1..3], &[0x34, 0x12]); // inner.1 LE u16
        // u8 at byte 3
        assert_eq!(bytes[3], 42);
        // BE u32 at bytes 4-7
        assert_eq!(&bytes[4..8], &[0x12, 0x34, 0x56, 0x78]);
        assert_eq!(OuterTuple::from_byte_array(bytes), outer);
    }

    #[test]
    fn array_tuple_byte_layout() {
        let tuple = ArrayTuple(42, [0xDE, 0xAD, 0xBE, 0xEF], 0x1234);
        let bytes = tuple.into_byte_array();
        // u8(1) + [u8;4](4) + u16(2) = 7 bytes
        assert_eq!(bytes.len(), 7);
        assert_eq!(bytes[0], 42);
        assert_eq!(&bytes[1..5], &[0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(&bytes[5..7], &[0x34, 0x12]); // LE u16
        assert_eq!(ArrayTuple::from_byte_array(bytes), tuple);
    }

    #[test]
    fn multiple_roundtrips() {
        let original = EndianTuple(100, 0xABCD, 0xDEADBEEF, 0x0123456789ABCDEF);
        for _ in 0..5 {
            let bytes = original.into_byte_array();
            assert_eq!(EndianTuple::from_byte_array(bytes), original);
        }
    }

    #[test]
    fn clone() {
        let tuple = SimpleTuple(1, 2, 3);
        assert_eq!(tuple.clone(), tuple);
    }
}

// ── Unit structs ─────────────────────────────────────────────────────────────

mod unit_structs {
    use byteable::{Byteable, FromByteArray, IntoByteArray};

    #[test]
    fn byteable_derive() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Byteable)]
        struct Marker;

        let bytes = Marker.into_byte_array();
        assert_eq!(bytes, [0u8; 0]);
        assert_eq!(Marker::from_byte_array([]), Marker);
    }

    #[test]
    fn multiple_unit_structs() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Byteable)]
        struct TypeA;
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Byteable)]
        struct TypeB;

        assert!(TypeA.into_byte_array().is_empty());
        assert!(TypeB.into_byte_array().is_empty());
        assert_eq!(TypeA::from_byte_array([]), TypeA);
        assert_eq!(TypeB::from_byte_array([]), TypeB);
    }

    #[test]
    fn generic_context() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Byteable)]
        struct Token;

        fn serialize<T: IntoByteArray>(value: T) -> T::ByteArray {
            value.into_byte_array()
        }
        fn deserialize<T: FromByteArray>(bytes: T::ByteArray) -> T {
            T::from_byte_array(bytes)
        }

        let bytes = serialize(Token);
        assert!(bytes.is_empty());
        assert_eq!(deserialize::<Token>(bytes), Token);
    }

    #[test]
    fn zero_size() {
        use core::mem::size_of;

        #[derive(Byteable, Clone, Copy)]
        struct Empty;

        assert_eq!(size_of::<Empty>(), 0);
        assert_eq!(
            size_of::<<Empty as byteable::IntoByteArray>::ByteArray>(),
            0
        );
    }
}

// ── Visibility modifiers ──────────────────────────────────────────────────────

mod visibility {
    use byteable::{Byteable, FromByteArray, IntoByteArray};

    #[derive(Clone, Copy, Byteable)]
    struct PrivateStruct {
        a: u8,
        #[byteable(little_endian)]
        b: u16,
    }

    #[derive(Clone, Copy, Byteable)]
    pub struct PublicStruct {
        a: u8,
        #[byteable(big_endian)]
        b: u32,
    }

    #[derive(Clone, Copy, Byteable)]
    pub(crate) struct CrateStruct {
        a: u8,
        #[byteable(little_endian)]
        b: u64,
    }

    mod inner {
        use byteable::Byteable;

        #[derive(Clone, Copy, Byteable)]
        pub(super) struct SuperStruct {
            pub(super) a: u8,
            #[byteable(big_endian)]
            pub(super) b: u16,
        }
    }

    #[derive(Clone, Copy, Byteable)]
    pub struct PublicTupleStruct(
        u8,
        #[byteable(little_endian)] u16,
        #[byteable(big_endian)] u32,
    );

    #[derive(Clone, Copy, Byteable)]
    struct PrivateTupleStruct(u8, #[byteable(little_endian)] u16);

    #[test]
    fn private_struct() {
        let s = PrivateStruct { a: 42, b: 0x1234 };
        let bytes = s.into_byte_array();
        let restored = PrivateStruct::from_byte_array(bytes);
        assert_eq!(s.a, restored.a);
        assert_eq!(s.b, restored.b);
    }

    #[test]
    fn public_struct() {
        let s = PublicStruct {
            a: 100,
            b: 0x12345678,
        };
        let bytes = s.into_byte_array();
        let restored = PublicStruct::from_byte_array(bytes);
        assert_eq!(s.a, restored.a);
        assert_eq!(s.b, restored.b);
    }

    #[test]
    fn crate_struct() {
        let s = CrateStruct {
            a: 200,
            b: 0x0102030405060708,
        };
        let bytes = s.into_byte_array();
        let restored = CrateStruct::from_byte_array(bytes);
        assert_eq!(s.a, restored.a);
        assert_eq!(s.b, restored.b);
    }

    #[test]
    fn super_struct() {
        let s = inner::SuperStruct { a: 50, b: 0xABCD };
        let bytes = s.into_byte_array();
        let restored = inner::SuperStruct::from_byte_array(bytes);
        assert_eq!(s.a, restored.a);
        assert_eq!(s.b, restored.b);
    }

    #[test]
    fn public_tuple_struct() {
        let s = PublicTupleStruct(10, 0x5678, 0xDEADBEEF);
        let bytes = s.into_byte_array();
        let restored = PublicTupleStruct::from_byte_array(bytes);
        assert_eq!(s.0, restored.0);
        assert_eq!(s.1, restored.1);
        assert_eq!(s.2, restored.2);
    }

    #[test]
    fn private_tuple_struct() {
        let s = PrivateTupleStruct(255, 0xFFFF);
        let bytes = s.into_byte_array();
        let restored = PrivateTupleStruct::from_byte_array(bytes);
        assert_eq!(s.0, restored.0);
        assert_eq!(s.1, restored.1);
    }

    #[test]
    fn endianness_preserved_across_visibilities() {
        let s = PublicStruct {
            a: 42,
            b: 0x01020304,
        };
        let bytes = s.into_byte_array();
        assert_eq!(bytes[0], 42);
        // big-endian u32
        assert_eq!(&bytes[1..5], &[0x01, 0x02, 0x03, 0x04]);
    }
}

// ── Transparent field attribute ───────────────────────────────────────────────

mod transparent {
    use byteable::{Byteable, FromByteArray, IntoByteArray};

    #[derive(Clone, Copy, Byteable)]
    struct MemberStruct {
        a: u8,
        #[byteable(little_endian)]
        b: u16,
    }

    #[derive(Clone, Copy, Byteable)]
    struct TestStruct {
        #[byteable(transparent)]
        member: MemberStruct,
        a: u8,
        #[byteable(little_endian)]
        b: u16,
        #[byteable(big_endian)]
        c: u64,
        #[byteable(little_endian)]
        d: f64,
    }

    #[test]
    fn member_struct_byte_size() {
        assert_eq!(MemberStruct::BYTE_SIZE, 3); // u8(1) + u16(2)
    }

    #[test]
    fn member_struct_byte_layout() {
        let m = MemberStruct { a: 10, b: 0x1234 };
        let bytes = m.into_byte_array();
        assert_eq!(bytes[0], 10);
        assert_eq!(bytes[1], 0x34); // LE low byte
        assert_eq!(bytes[2], 0x12);
    }

    #[test]
    fn outer_struct_byte_size() {
        // member(3) + u8(1) + u16(2) + u64(8) + f64(8) = 22
        assert_eq!(TestStruct::BYTE_SIZE, 22);
    }

    #[test]
    fn transparent_field_occupies_start() {
        let member = MemberStruct { a: 10, b: 0x1234 };
        let outer = TestStruct {
            member,
            a: 0,
            b: 0,
            c: 0,
            d: 0.0,
        };
        let bytes = outer.into_byte_array();
        let member_bytes = member.into_byte_array();
        assert_eq!(&bytes[0..3], member_bytes.as_ref());
    }

    #[test]
    fn outer_field_layout() {
        let outer = TestStruct {
            member: MemberStruct { a: 10, b: 0x1234 },
            a: 42,
            b: 0x5678,
            c: 0x0102030405060708,
            d: 3.14159,
        };
        let bytes = outer.into_byte_array();
        assert_eq!(bytes[3], 42); // a at byte 3
        assert_eq!(&bytes[4..6], &[0x78, 0x56]); // LE u16
        assert_eq!(
            &bytes[6..14],
            &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        ); // BE u64
        let d_bytes: [u8; 8] = bytes[14..22].try_into().unwrap();
        assert_eq!(f64::from_le_bytes(d_bytes), 3.14159);
    }

    #[test]
    fn roundtrip() {
        let original = TestStruct {
            member: MemberStruct { a: 10, b: 0x1234 },
            a: 42,
            b: 0x5678,
            c: 0x0102030405060708,
            d: 3.14159,
        };
        let bytes = original.into_byte_array();
        let restored = TestStruct::from_byte_array(bytes);
        assert_eq!(original.member.a, restored.member.a);
        assert_eq!(original.member.b, restored.member.b);
        assert_eq!(original.a, restored.a);
        assert_eq!(original.b, restored.b);
        assert_eq!(original.c, restored.c);
        assert_eq!(original.d, restored.d);
    }
}

// ── Compile-time safety validation (PlainOldData) ────────────────────────────

mod safety {
    use byteable::{Byteable, FromByteArray, IntoByteArray};

    #[derive(Clone, Copy, Byteable)]
    pub struct SafePacket {
        id: u8,
        #[byteable(little_endian)]
        length: u16,
        #[byteable(big_endian)]
        checksum: u32,
        data: [u8; 4],
    }

    #[derive(Clone, Copy, Byteable)]
    struct Point {
        #[byteable(little_endian)]
        x: i32,
        #[byteable(little_endian)]
        y: i32,
    }

    #[derive(Clone, Copy, Byteable)]
    struct Shape {
        id: u8,
        #[byteable(transparent)]
        top_left: Point,
        #[byteable(transparent)]
        bottom_right: Point,
    }

    #[test]
    fn safe_packet_roundtrip() {
        let packet = SafePacket {
            id: 42,
            length: 1024,
            checksum: 0x12345678,
            data: [1, 2, 3, 4],
        };
        let bytes = packet.into_byte_array();
        let restored = SafePacket::from_byte_array(bytes);
        assert_eq!(packet.id, restored.id);
        assert_eq!(packet.length, restored.length);
        assert_eq!(packet.checksum, restored.checksum);
        assert_eq!(packet.data, restored.data);
    }

    #[test]
    fn nested_safe_types_roundtrip() {
        let shape = Shape {
            id: 1,
            top_left: Point { x: 0, y: 0 },
            bottom_right: Point { x: 100, y: 200 },
        };
        let bytes = shape.into_byte_array();
        let restored = Shape::from_byte_array(bytes);
        assert_eq!(shape.id, restored.id);
        assert_eq!(shape.top_left.x, restored.top_left.x);
        assert_eq!(shape.top_left.y, restored.top_left.y);
        assert_eq!(shape.bottom_right.x, restored.bottom_right.x);
        assert_eq!(shape.bottom_right.y, restored.bottom_right.y);
    }

    /// `bool` does not implement `PlainOldData` (invalid bit patterns 2..=255).
    ///
    /// ```compile_fail
    /// # #[cfg(feature = "derive")] {
    /// use byteable::Byteable;
    ///
    /// #[derive(Clone, Copy, Byteable)]
    /// struct Bad {
    ///     id: u8,
    ///     flag: bool,
    /// }
    /// # }
    /// ```
    ///
    /// `char` does not implement `PlainOldData` (many code-points are invalid).
    ///
    /// ```compile_fail
    /// # #[cfg(feature = "derive")] {
    /// use byteable::Byteable;
    ///
    /// #[derive(Clone, Copy, Byteable)]
    /// struct Bad {
    ///     id: u8,
    ///     letter: char,
    /// }
    /// # }
    /// ```
    ///
    /// References do not implement `PlainOldData`.
    ///
    /// ```compile_fail
    /// # #[cfg(feature = "derive")] {
    /// use byteable::Byteable;
    ///
    /// #[derive(Clone, Copy, Byteable)]
    /// struct Bad<'a> {
    ///     id: u8,
    ///     data_ref: &'a [u8],
    /// }
    /// # }
    /// ```
    ///
    /// Unannotated multi-byte primitives (`u16` without an endian wrapper) are rejected
    /// because their native byte order is platform-dependent.
    ///
    /// ```compile_fail
    /// # #[cfg(feature = "derive")] {
    /// use byteable::Byteable;
    ///
    /// #[derive(Clone, Copy, Byteable)]
    /// struct Bad {
    ///     value: u16,
    /// }
    /// # }
    /// ```
    #[test]
    fn compile_fail_examples_documented_above() {}
}
