//! `Readable`/`Writable` implementations for `arrayvec`'s fixed-capacity collection types
//! (`std` + `arrayvec` features).
//!
//! Wire formats and capacity-checking behavior mirror `ext/heapless/io.rs`; see that module
//! for the rationale.

use crate::{
    DecodeError,
    io::{ReadFixed, ReadValue, Readable, ReadableError, Writable},
};
use arrayvec::{ArrayString, ArrayVec};
use std::io::{Read, Write};

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
impl<T: Readable, const N: usize> Readable for ArrayVec<T, N> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let len: u64 = reader.read_fixed()?;
        let len = checked_len(len, N, "arrayvec::ArrayVec")?;
        let mut result = ArrayVec::new();
        for _ in 0..len {
            let item = reader.read_value()?;
            if result.try_push(item).is_err() {
                unreachable!("length was checked against capacity above")
            }
        }
        Ok(result)
    }
}

impl<T: Writable, const N: usize> Writable for ArrayVec<T, N> {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
        self.as_slice().write_to(writer)
    }
}

// Wire format: `u64` byte length (LE) + UTF-8 bytes. Rejects invalid UTF-8 with DecodeError.
impl<const N: usize> Readable for ArrayString<N> {
    fn read_from(reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let len: u64 = reader.read_fixed()?;
        let len = checked_len(len, N, "arrayvec::ArrayString")?;
        let mut buf = [0u8; N];
        reader.read_exact(&mut buf[..len])?;
        let s = core::str::from_utf8(&buf[..len]).map_err(|_| DecodeError::InvalidUtf8)?;
        match ArrayString::from(s) {
            Ok(s) => Ok(s),
            Err(_) => unreachable!("length was checked against capacity above"),
        }
    }
}

impl<const N: usize> Writable for ArrayString<N> {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
        self.as_str().write_to(writer)
    }
}
