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

    loop {}
}
