//! Integration tests for enums with fields derived via `#[derive(Byteable)]`.
//!
//! Field enums implement `Readable` + `Writable` (stream-based I/O) rather than
//! `ToByteArray`/`FromByteArray`, because variant sizes differ.
#![cfg(all(feature = "std", feature = "derive"))]
use byteable::Byteable;
use byteable::io::{ReadValue, ReadableError, WriteValue};
use std::io::Cursor;

// ── Basic field enum with u8 discriminant ────────────────────────────────────

#[derive(Byteable, Debug, PartialEq)]
#[repr(u8)]
enum Message {
    Ping = 0,
    Pong { id: u8 } = 1,
    Data { length: u8, value: [u8; 4] } = 2,
}

#[test]
fn unit_variant_roundtrip() {
    let mut buf = Vec::new();
    buf.write_value(&Message::Ping).unwrap();
    assert_eq!(buf, [0u8]); // discriminant only

    let msg: Message = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(msg, Message::Ping);
}

#[test]
fn named_field_variant_roundtrip() {
    let original = Message::Pong { id: 42 };

    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();
    assert_eq!(buf, [1u8, 42u8]); // discriminant + id

    let decoded: Message = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn multi_field_variant_roundtrip() {
    let original = Message::Data {
        length: 4,
        value: [0xDE, 0xAD, 0xBE, 0xEF],
    };

    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();
    assert_eq!(buf, [2u8, 4u8, 0xDE, 0xAD, 0xBE, 0xEF]);

    let decoded: Message = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn invalid_discriminant_returns_error() {
    let buf = [0xFFu8]; // not a valid discriminant
    let result: Result<Message, ReadableError> = Cursor::new(&buf).read_value();
    assert!(result.is_err());
}

// ── Tuple variant ─────────────────────────────────────────────────────────────

#[derive(Byteable, Debug, PartialEq)]
#[repr(u8)]
enum Packet {
    Empty = 0,
    Single(u8) = 1,
    Pair(u8, u8) = 2,
}

#[test]
fn tuple_variant_roundtrip() {
    for original in [Packet::Empty, Packet::Single(99), Packet::Pair(1, 2)] {
        let mut buf = Vec::new();
        buf.write_value(&original).unwrap();
        let decoded: Packet = Cursor::new(&buf).read_value().unwrap();
        assert_eq!(decoded, original);
    }
}

#[test]
fn tuple_pair_byte_layout() {
    let mut buf = Vec::new();
    buf.write_value(&Packet::Pair(10, 20)).unwrap();
    assert_eq!(buf, [2u8, 10u8, 20u8]);
}

// ── Mixed unit and field variants ────────────────────────────────────────────

#[derive(Byteable, Debug, PartialEq)]
#[repr(u8)]
enum Command {
    Noop = 0,
    SetValue { value: u8 } = 1,
    SetPair(u8, u8) = 2,
    Reset = 3,
}

#[test]
fn mixed_variants_roundtrip() {
    let cases = [
        Command::Noop,
        Command::SetValue { value: 7 },
        Command::SetPair(3, 4),
        Command::Reset,
    ];
    for original in &cases {
        let mut buf = Vec::new();
        buf.write_value(original).unwrap();
        let decoded: Command = Cursor::new(&buf).read_value().unwrap();
        assert_eq!(&decoded, original);
    }
}

// ── Multi-byte discriminant with endianness ───────────────────────────────────

#[derive(Byteable, Debug, PartialEq)]
#[repr(u16)]
enum Request {
    #[byteable(tag = 0x0001)]
    Ping,
    #[byteable(tag = 0x0002)]
    GetValue { key: u8 },
    #[byteable(tag = 0x0003)]
    SetValue { key: u8, val: u8 },
}

#[test]
fn little_endian_discriminant_byte_layout() {
    let mut buf = Vec::new();
    buf.write_value(&Request::Ping).unwrap();
    assert_eq!(buf, [0x01, 0x00]); // 0x0001 in little-endian
}

#[test]
fn little_endian_field_variant_roundtrip() {
    let original = Request::SetValue { key: 5, val: 42 };
    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();
    assert_eq!(buf, [0x03, 0x00, 5, 42]);

    let decoded: Request = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}

#[derive(Byteable, Debug, PartialEq)]
#[repr(u16)]
#[byteable(big_endian)]
enum Response {
    #[byteable(tag = 0x0001)]
    Ok,
    #[byteable(tag = 0x0002)]
    Error { code: u8 },
}

#[test]
fn big_endian_discriminant_byte_layout() {
    let mut buf = Vec::new();
    buf.write_value(&Response::Ok).unwrap();
    assert_eq!(buf, [0x00, 0x01]); // 0x0001 in big-endian
}

#[test]
fn big_endian_field_variant_roundtrip() {
    let original = Response::Error { code: 99 };
    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();
    assert_eq!(buf, [0x00, 0x02, 99]);

    let decoded: Response = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}

// ── Field-level endianness annotations ───────────────────────────────────────

#[derive(Byteable, Debug, PartialEq)]
#[repr(u8)]
enum Typed {
    Small {
        val: u8,
    } = 0,
    Wide {
        val: u32,
    } = 1,
    Network {
        #[byteable(big_endian)]
        port: u16,
        #[byteable(big_endian)]
        addr: u32,
    } = 2,
}

#[test]
fn little_endian_field_annotation_roundtrip() {
    let original = Typed::Wide { val: 0xDEADBEEF };
    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();
    // discriminant(1) + 4 bytes little-endian
    assert_eq!(buf, [1, 0xEF, 0xBE, 0xAD, 0xDE]);

    let decoded: Typed = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn big_endian_field_annotation_roundtrip() {
    let original = Typed::Network {
        port: 8080,
        addr: 0x7F000001,
    };
    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();
    // discriminant(2) + port big-endian + addr big-endian
    assert_eq!(buf, [2, 0x1F, 0x90, 0x7F, 0x00, 0x00, 0x01]);

    let decoded: Typed = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn mixed_field_annotations_roundtrip() {
    for original in [
        Typed::Small { val: 42 },
        Typed::Wide { val: 0x12345678 },
        Typed::Network {
            port: 443,
            addr: 0xC0A80001,
        },
    ] {
        let mut buf = Vec::new();
        buf.write_value(&original).unwrap();
        let decoded: Typed = Cursor::new(&buf).read_value().unwrap();
        assert_eq!(decoded, original);
    }
}

// ── C-like enum as a field (uses newly-added Readable impl) ──────────────────

#[derive(Byteable, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum Status {
    Ok = 0,
    Err = 1,
}

#[derive(Byteable, Debug, PartialEq)]
#[repr(u8)]
enum Envelope {
    Empty = 0,
    WithStatus { status: Status } = 1,
}

#[test]
fn c_like_enum_as_field_roundtrip() {
    let original = Envelope::WithStatus {
        status: Status::Err,
    };
    let mut buf = Vec::new();
    buf.write_value(&original).unwrap();
    assert_eq!(buf, [1u8, 1u8]); // envelope disc + status disc

    let decoded: Envelope = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn c_like_enum_invalid_nested_discriminant_returns_error() {
    let buf = [1u8, 0xFFu8]; // valid envelope disc, invalid status disc
    let result: Result<Envelope, _> = Cursor::new(&buf).read_value();
    assert!(result.is_err());
}

// ── bool field (uses newly-added Readable impl for bool) ─────────────────────

#[derive(Byteable, Debug, PartialEq)]
#[repr(u8)]
enum Flagged {
    Off = 0,
    On { enabled: bool } = 1,
}

#[test]
fn bool_field_roundtrip() {
    for enabled in [false, true] {
        let original = Flagged::On { enabled };
        let mut buf = Vec::new();
        buf.write_value(&original).unwrap();
        let decoded: Flagged = Cursor::new(&buf).read_value().unwrap();
        assert_eq!(decoded, original);
    }
}

// ── Auto-repr (no #[repr]) and auto-discriminants ────────────────────────────

/// No #[repr] or explicit discriminants - fully automatic.
/// 3 variants → auto repr u8, discriminants 0/1/2.
#[derive(Byteable, Debug, PartialEq)]
enum Auto {
    First,
    Second { x: u8 },
    Third(u8, u8),
}

#[test]
fn auto_repr_discriminant_byte_layout() {
    let mut buf = Vec::new();
    buf.write_value(&Auto::First).unwrap();
    assert_eq!(buf, [0u8]); // disc = 0

    buf.clear();
    buf.write_value(&Auto::Second { x: 7 }).unwrap();
    assert_eq!(buf, [1u8, 7u8]); // disc = 1, x

    buf.clear();
    buf.write_value(&Auto::Third(3, 4)).unwrap();
    assert_eq!(buf, [2u8, 3u8, 4u8]); // disc = 2, fields
}

#[test]
fn auto_repr_roundtrip() {
    for original in [Auto::First, Auto::Second { x: 42 }, Auto::Third(10, 20)] {
        let mut buf = Vec::new();
        buf.write_value(&original).unwrap();
        let decoded: Auto = Cursor::new(&buf).read_value().unwrap();
        assert_eq!(decoded, original);
    }
}

/// Auto-discriminants with one explicit override - subsequent variants continue from there.
#[derive(Byteable, Debug, PartialEq)]
#[repr(u8)]
enum PartialDiscriminants {
    First, // implicit: 0
    #[byteable(tag = 10)]
    Explicit,
    AfterExplicit {
        val: u8,
    }, // implicit, continues from Explicit's tag=10 -> 11
}

#[test]
fn partial_discriminants_byte_layout() {
    let mut buf = Vec::new();
    buf.write_value(&PartialDiscriminants::First).unwrap();
    assert_eq!(buf[0], 0);

    buf.clear();
    buf.write_value(&PartialDiscriminants::Explicit).unwrap();
    assert_eq!(buf[0], 10);

    buf.clear();
    buf.write_value(&PartialDiscriminants::AfterExplicit { val: 5 })
        .unwrap();
    assert_eq!(buf, [11u8, 5u8]);
}

#[test]
fn partial_discriminants_roundtrip() {
    for original in [
        PartialDiscriminants::First,
        PartialDiscriminants::Explicit,
        PartialDiscriminants::AfterExplicit { val: 99 },
    ] {
        let mut buf = Vec::new();
        buf.write_value(&original).unwrap();
        let decoded: PartialDiscriminants = Cursor::new(&buf).read_value().unwrap();
        assert_eq!(decoded, original);
    }
}

/// Hex discriminant - counter resumes correctly after a hex literal.
#[derive(Byteable, Debug, PartialEq)]
#[repr(u8)]
enum HexDiscriminants {
    #[byteable(tag = 0x10)]
    Base,
    Next {
        x: u8,
    }, // implicit, continues from Base's tag=0x10 -> 17
}

#[test]
fn hex_discriminant_tracking() {
    let mut buf = Vec::new();
    buf.write_value(&HexDiscriminants::Base).unwrap();
    assert_eq!(buf[0], 0x10);

    buf.clear();
    buf.write_value(&HexDiscriminants::Next { x: 1 }).unwrap();
    assert_eq!(buf, [0x11u8, 1u8]); // disc = 0x11 = 17
}

#[test]
fn hex_discriminant_roundtrip() {
    for original in [HexDiscriminants::Base, HexDiscriminants::Next { x: 55 }] {
        let mut buf = Vec::new();
        buf.write_value(&original).unwrap();
        let decoded: HexDiscriminants = Cursor::new(&buf).read_value().unwrap();
        assert_eq!(decoded, original);
    }
}

// ── Generic field-carrying enum ──────────────────────────────────────────────
//
// Regression coverage for a bug where `resolve_enum_discriminants`'s generated discriminant
// consts lived in `impl #enum_name { const ... }` with no generics on the `impl` block, even
// though `enum_derive` (this dynamic/I-O pipeline) explicitly supports generic enums elsewhere
// via `input.generics.split_for_impl()`. That produced E0107 ("missing generics for enum") on
// any generic field-carrying enum. Fixed by making the discriminant consts free module-level
// items instead of associated consts, since their values never depend on the enum's type
// parameters in the first place.
//
// Gated on all four I/O pipeline features (in addition to this file's std+derive gate) because
// the derive macro generates all four pipelines' impls whenever byteable_derive itself has all
// four features enabled (as it does under `--all-features`), and each pipeline imposes its own
// trait bound on `T` - this test only compiles/runs when all four are simultaneously active,
// which is exactly the scenario that broke.
#[cfg(all(
    feature = "embedded-io",
    feature = "tokio",
    feature = "embedded-io-async"
))]
#[derive(Byteable, Debug, PartialEq)]
enum GenericMessage<T>
where
    T: byteable::io::Readable
        + byteable::io::Writable
        + byteable::eio::EioReadable
        + byteable::eio::EioWritable
        + byteable::async_io::AsyncReadable
        + byteable::async_io::AsyncWritable
        + byteable::eio_async::EioAsyncReadable
        + byteable::eio_async::EioAsyncWritable
        + byteable::WireFingerprint
        + std::fmt::Debug
        + PartialEq,
{
    Value(T),
    Empty,
}

#[cfg(all(
    feature = "embedded-io",
    feature = "tokio",
    feature = "embedded-io-async"
))]
#[test]
fn generic_field_carrying_enum_compiles_and_roundtrips() {
    let msg: GenericMessage<u32> = GenericMessage::Value(42);
    let mut buf = Vec::new();
    buf.write_value(&msg).unwrap();
    let decoded: GenericMessage<u32> = Cursor::new(&buf).read_value().unwrap();
    assert_eq!(msg, decoded);

    let empty: GenericMessage<u32> = GenericMessage::Empty;
    let mut buf2 = Vec::new();
    buf2.write_value(&empty).unwrap();
    let decoded2: GenericMessage<u32> = Cursor::new(&buf2).read_value().unwrap();
    assert_eq!(empty, decoded2);
}
