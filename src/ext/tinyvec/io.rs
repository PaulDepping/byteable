//! `Readable`/`Writable` implementations for `tinyvec::ArrayVec` (`std` + `tinyvec` features).
//!
//! Wire formats and capacity-checking behavior mirror `ext/heapless/io.rs`/
//! `ext/arrayvec/io.rs`; see the former for the rationale. Generic over `tinyvec::Array`
//! rather than a bare const generic, so this covers any `Array` implementor (not just
//! `[T; N]`) - `A::Item: Default` comes for free since `Array` itself requires it.

use crate::{
    DecodeError,
    io::{ReadFixed, ReadValue, Readable, ReadableError, Writable},
};
use std::io::{Read, Write};
use tinyvec::{Array, ArrayVec};

fn checked_len(len: u64, capacity: usize, type_name: &'static str) -> Result<usize, DecodeError> {
    let len_usize: usize = len.try_into().expect("could not convert u64 to usize");
    if len_usize > capacity {
        return Err(DecodeError::CapacityExceeded {
            len,
            capacity,
            type_name,
        });
    }
    Ok(len_usize)
}

// Wire format: `u64` element count (LE), then each element serialized in order.
impl<A: Array> Readable for ArrayVec<A>
where
    A::Item: Readable,
{
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let len: u64 = reader.read_fixed()?;
        let len = checked_len(len, A::CAPACITY, "tinyvec::ArrayVec")?;
        let mut result = ArrayVec::new();
        for _ in 0..len {
            let item = reader.read_value()?;
            if result.try_push(item).is_some() {
                unreachable!("length was checked against capacity above")
            }
        }
        Ok(result)
    }
}

impl<A: Array> Writable for ArrayVec<A>
where
    A::Item: Writable,
{
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
        self.as_slice().write_to(writer)
    }
}
