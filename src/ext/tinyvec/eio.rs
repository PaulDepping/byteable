//! `EioReadable`/`EioWritable` implementation for `tinyvec::ArrayVec`
//! (`embedded-io` + `tinyvec` features).
//!
//! Wire format and capacity-checking behavior are identical to [`crate::ext::tinyvec::io`];
//! see that module for the rationale. `tinyvec::ArrayVec` doesn't need the `alloc` feature —
//! it's array-backed with a compile-time capacity.

use crate::{
    DecodeError,
    eio::{
        EioReadFixed, EioReadValue, EioReadable, EioReadableError, EioReader, EioWritable,
        EioWriter,
    },
};
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

impl<A: Array> EioReadable for ArrayVec<A>
where
    A::Item: EioReadable,
{
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
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

impl<A: Array> EioWritable for ArrayVec<A>
where
    A::Item: EioWritable,
{
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        self.as_slice().write_to(writer)
    }
}
