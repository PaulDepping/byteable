//! `EioReadable`/`EioWritable` implementations for `arrayvec`'s fixed-capacity collection
//! types (`embedded-io` + `arrayvec` features).
//!
//! Wire formats and capacity-checking behavior are identical to
//! [`crate::ext::arrayvec::io`]; see that module for the rationale. Neither `ArrayVec` nor
//! `ArrayString` need the `alloc` feature — both are stack-allocated with a compile-time
//! capacity.

use crate::{
    DecodeError,
    eio::{
        EioReadFixed, EioReadValue, EioReadable, EioReadableError, EioReader, EioWritable,
        EioWriter,
    },
};
use arrayvec::{ArrayString, ArrayVec};

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

impl<T: EioReadable, const N: usize> EioReadable for ArrayVec<T, N> {
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
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

impl<T: EioWritable, const N: usize> EioWritable for ArrayVec<T, N> {
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        self.as_slice().write_to(writer)
    }
}

impl<const N: usize> EioReadable for ArrayString<N> {
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
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

impl<const N: usize> EioWritable for ArrayString<N> {
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        self.as_str().write_to(writer)
    }
}
