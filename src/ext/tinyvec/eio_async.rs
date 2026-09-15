//! `EioAsyncReadable`/`EioAsyncWritable` implementation for `tinyvec::ArrayVec`
//! (`embedded-io-async` + `tinyvec` features).
//!
//! Wire format and capacity-checking behavior are identical to [`crate::ext::tinyvec::io`]
//! and [`crate::ext::tinyvec::eio`]; see the former for the rationale.

#![allow(clippy::manual_async_fn)]

use crate::eio::EioReadableError;
use crate::{
    DecodeError,
    eio_async::{
        EioAsyncReadFixed, EioAsyncReadValue, EioAsyncReadable, EioAsyncReader, EioAsyncWritable,
        EioAsyncWriter,
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

impl<A: Array> EioAsyncReadable for ArrayVec<A>
where
    A::Item: EioAsyncReadable,
{
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
            let len: u64 = reader.read_fixed().await?;
            let len = checked_len(len, A::CAPACITY, "tinyvec::ArrayVec")?;
            let mut result = ArrayVec::new();
            for _ in 0..len {
                let item = reader.read_value().await?;
                if result.try_push(item).is_some() {
                    unreachable!("length was checked against capacity above")
                }
            }
            Ok(result)
        }
    }
}

impl<A: Array> EioAsyncWritable for ArrayVec<A>
where
    A::Item: EioAsyncWritable,
{
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
        self.as_slice().write_to(writer)
    }
}
