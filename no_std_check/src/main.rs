#![no_std]
#![no_main]

use byteable::Byteable;
use byteable::eio::{EioReadFixed, EioReadValue, EioWriteFixed, EioWriteValue};
use byteable::eio_async::{
    EioAsyncReadFixed, EioAsyncReadValue, EioAsyncWriteFixed, EioAsyncWriteValue,
};
use core::future::Future;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use cortex_m_rt::entry;
use panic_halt as _;

// Smoke-test convenience, not a general executor — sound only because every future polled here
// (embedded_io_async's &[u8]/&mut [u8] impls) resolves on its first poll and never returns
// Poll::Pending, so this waker is never actually asked to wake anything.
fn block_on<F: Future>(fut: F) -> F::Output {
    fn noop(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(core::ptr::null(), &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);

    let waker = unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) };
    let mut cx = Context::from_waker(&waker);
    let mut fut = core::pin::pin!(fut);
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("block_on: future did not resolve on first poll"),
    }
}

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

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Perms: u32 {
        const READ = 0b001;
        const WRITE = 0b010;
        const EXEC = 0b100;
    }
}
byteable::impl_bitflags!(Perms);
byteable::impl_bitflags_endian!(Perms);

#[entry]
fn main() -> ! {
    let mut buf = [0u8; 64];

    // Fixed-size derived-struct path over embedded-io.
    let header = Header {
        magic: 0x1234_5678,
        version: 7,
    };
    // Note: call sites below use fully-qualified (UFCS) syntax rather than method syntax.
    // With both the sync (`eio`) and async (`eio_async`) traits imported in the same scope,
    // `&[u8]`/`&mut [u8]` implement both `EioRead*`/`EioWrite*` and `EioAsyncRead*`/
    // `EioAsyncWrite*` with identically-named methods, so plain method-call syntax
    // (`r.read_fixed()`) is ambiguous (E0034) — UFCS picks the trait explicitly.
    {
        let mut w: &mut [u8] = &mut buf;
        EioWriteFixed::write_fixed(&mut w, &header).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _restored: Header = EioReadFixed::read_fixed(&mut r).unwrap();

    // Dynamic (non-alloc) path over embedded-io, via Option<u32>'s hand-written EioReadable/
    // EioWritable impls (Task 3) — no heap, no derive macro involved.
    let flag: Option<u32> = Some(7);
    {
        let mut w: &mut [u8] = &mut buf;
        EioWriteValue::write_value(&mut w, &flag).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _flag2: Option<u32> = EioReadValue::read_value(&mut r).unwrap();

    // Field enum over embedded-io. This crate has no `std` feature at all, so this only
    // compiles because the derive macro's dynamic-pipeline codegen is feature-driven: with
    // only `embedded-io` enabled (not `std`), it emits solely the `EioReadable`/`EioWritable`
    // impls, never a `::std::io`-based `Readable`/`Writable`.
    let cmd = TestEnum::Write { a: 1, b: 2 };
    {
        let mut w: &mut [u8] = &mut buf;
        EioWriteValue::write_value(&mut w, &cmd).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _cmd2: TestEnum = EioReadValue::read_value(&mut r).unwrap();

    // Fixed-size derived-struct path over embedded-io-async.
    {
        let mut w: &mut [u8] = &mut buf;
        block_on(EioAsyncWriteFixed::write_fixed(&mut w, &header)).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _restored_async: Header = block_on(EioAsyncReadFixed::read_fixed(&mut r)).unwrap();

    // Dynamic (non-alloc) path over embedded-io-async, via Option<u32>'s hand-written
    // EioAsyncReadable/EioAsyncWritable impls — no heap, no derive macro involved.
    {
        let mut w: &mut [u8] = &mut buf;
        block_on(EioAsyncWriteValue::write_value(&mut w, &flag)).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _flag2_async: Option<u32> = block_on(EioAsyncReadValue::read_value(&mut r)).unwrap();

    // Field enum over embedded-io-async — proves Task 7's derive retrofit compiles and links
    // on a real no_std target with no heap and no std crate available at all.
    {
        let mut w: &mut [u8] = &mut buf;
        block_on(EioAsyncWriteValue::write_value(&mut w, &cmd)).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _cmd2_async: TestEnum = block_on(EioAsyncReadValue::read_value(&mut r)).unwrap();

    // `impl_bitflags!`/`impl_bitflags_endian!`-generated impls, over both embedded-io and
    // embedded-io-async, with no heap and no `std` — proves the bitflags integration compiles
    // and links on a real no_std target.
    let perms = Perms::READ | Perms::EXEC;
    {
        let mut w: &mut [u8] = &mut buf;
        EioWriteFixed::write_fixed(&mut w, &perms).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _perms2: Perms = EioReadFixed::read_fixed(&mut r).unwrap();

    {
        let mut w: &mut [u8] = &mut buf;
        block_on(EioAsyncWriteFixed::write_fixed(&mut w, &perms)).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _perms2_async: Perms = block_on(EioAsyncReadFixed::read_fixed(&mut r)).unwrap();

    // `heapless::Vec<T, N>`'s hand-written EioReadable/EioWritable and EioAsyncReadable/
    // EioAsyncWritable impls, over both pipelines — no heap, no `alloc` feature at all, proving
    // heapless support works on a genuinely allocation-free no_std target.
    let mut hv: heapless::Vec<u32, 4> = heapless::Vec::new();
    hv.push(1).unwrap();
    hv.push(2).unwrap();
    {
        let mut w: &mut [u8] = &mut buf;
        EioWriteValue::write_value(&mut w, &hv).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _hv2: heapless::Vec<u32, 4> = EioReadValue::read_value(&mut r).unwrap();

    {
        let mut w: &mut [u8] = &mut buf;
        block_on(EioAsyncWriteValue::write_value(&mut w, &hv)).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _hv2_async: heapless::Vec<u32, 4> =
        block_on(EioAsyncReadValue::read_value(&mut r)).unwrap();

    // `arrayvec::ArrayVec<T, N>`'s hand-written EioReadable/EioWritable and EioAsyncReadable/
    // EioAsyncWritable impls, over both pipelines — no heap, no `alloc` feature at all, proving
    // arrayvec support works on a genuinely allocation-free no_std target.
    let mut av: arrayvec::ArrayVec<u32, 4> = arrayvec::ArrayVec::new();
    av.push(1);
    av.push(2);
    {
        let mut w: &mut [u8] = &mut buf;
        EioWriteValue::write_value(&mut w, &av).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _av2: arrayvec::ArrayVec<u32, 4> = EioReadValue::read_value(&mut r).unwrap();

    {
        let mut w: &mut [u8] = &mut buf;
        block_on(EioAsyncWriteValue::write_value(&mut w, &av)).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _av2_async: arrayvec::ArrayVec<u32, 4> =
        block_on(EioAsyncReadValue::read_value(&mut r)).unwrap();

    // `tinyvec::ArrayVec<[T; N]>`'s hand-written EioReadable/EioWritable and EioAsyncReadable/
    // EioAsyncWritable impls, over both pipelines — no heap, no `alloc` feature at all, proving
    // tinyvec support works on a genuinely allocation-free no_std target.
    let mut tv: tinyvec::ArrayVec<[u32; 4]> = tinyvec::ArrayVec::new();
    tv.push(1);
    tv.push(2);
    {
        let mut w: &mut [u8] = &mut buf;
        EioWriteValue::write_value(&mut w, &tv).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _tv2: tinyvec::ArrayVec<[u32; 4]> = EioReadValue::read_value(&mut r).unwrap();

    {
        let mut w: &mut [u8] = &mut buf;
        block_on(EioAsyncWriteValue::write_value(&mut w, &tv)).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _tv2_async: tinyvec::ArrayVec<[u32; 4]> =
        block_on(EioAsyncReadValue::read_value(&mut r)).unwrap();

    // defmt::Format impls for byteable's own error/wrapper types, proven on a real no_std
    // target. Not actually invoking `defmt::info!`/`.format()` here — that needs a
    // `#[defmt::global_logger]` (e.g. `defmt-rtt`), which is a concern for the final firmware
    // binary, not this compile-only smoke test — so this only proves the trait bound holds.
    fn assert_defmt_format<T: defmt::Format>() {}
    assert_defmt_format::<byteable::DecodeError>();
    assert_defmt_format::<byteable::LittleEndian<u32>>();
    assert_defmt_format::<byteable::BigEndian<u32>>();
    assert_defmt_format::<byteable::eio::EioReadableError<()>>();
    assert_defmt_format::<byteable::eio::EioReadExactError<()>>();

    // `OrderedFloat<f32>`'s blanket EioFixedReadable/EioFixedWritable and EioAsyncFixedReadable/
    // EioAsyncFixedWritable impls (via RawRepr/TryFromRawRepr), over both pipelines — proving
    // ordered-float actually works on a true no_std target now that its own `std` feature is no
    // longer pulled in unconditionally (it used to be: ordered-float's default features include
    // `std`, which does `extern crate std;` and fails to link on a target with no std at all).
    let of = ordered_float::OrderedFloat(1.5f32);
    {
        let mut w: &mut [u8] = &mut buf;
        EioWriteFixed::write_fixed(&mut w, &of).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _of2: ordered_float::OrderedFloat<f32> = EioReadFixed::read_fixed(&mut r).unwrap();

    {
        let mut w: &mut [u8] = &mut buf;
        block_on(EioAsyncWriteFixed::write_fixed(&mut w, &of)).unwrap();
    }
    let mut r: &[u8] = &buf;
    let _of2_async: ordered_float::OrderedFloat<f32> =
        block_on(EioAsyncReadFixed::read_fixed(&mut r)).unwrap();

    loop {}
}
