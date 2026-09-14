#![no_std]
#![no_main]

use byteable::Byteable;
use byteable::eio::{EioReadFixed, EioReadValue, EioWriteFixed, EioWriteValue};
use cortex_m_rt::entry;
use panic_halt as _;

#[derive(Byteable, Clone, Copy)]
struct Header {
    magic: u32,
    version: u16,
}

#[derive(Byteable, Debug, PartialEq)]
enum TestEnum {
    Nothing,
    Read(u32, u64),
    Write { a: u32, b: u64 },
}

#[entry]
fn main() -> ! {
    let mut buf = [0u8; 64];

    // Fixed-size derived-struct path over embedded-io.
    let header = Header {
        magic: 0x1234_5678,
        version: 7,
    };
    {
        let mut w: &mut [u8] = &mut buf;
        w.write_fixed(&header).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _restored: Header = r.read_fixed().unwrap();

    // Dynamic (non-alloc) path over embedded-io, via Option<u32>'s hand-written EioReadable/
    // EioWritable impls (Task 3) — no heap, no derive macro involved.
    let flag: Option<u32> = Some(7);
    {
        let mut w: &mut [u8] = &mut buf;
        w.write_value(&flag).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _flag2: Option<u32> = r.read_value().unwrap();

    // Field enum over embedded-io. This crate has no `std` feature at all, so this only
    // compiles because the derive macro's dynamic-pipeline codegen is feature-driven: with
    // only `embedded-io` enabled (not `std`), it emits solely the `EioReadable`/`EioWritable`
    // impls, never a `::std::io`-based `Readable`/`Writable`.
    let cmd = TestEnum::Write { a: 1, b: 2 };
    {
        let mut w: &mut [u8] = &mut buf;
        w.write_value(&cmd).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _cmd2: TestEnum = r.read_value().unwrap();

    loop {}
}
