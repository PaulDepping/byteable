//! Async [`AsyncReadable`]/[`AsyncWritable`] implementation for `tinyvec::ArrayVec`
//! (`tokio` + `tinyvec` features).
//!
//! Wire format and capacity-checking behavior are identical to [`crate::ext::tinyvec::io`];
//! see that module for the rationale.

#![allow(clippy::manual_async_fn)]

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::DecodeError;
use crate::async_io::{AsyncReadFixed, AsyncReadValue, AsyncReadable, AsyncWritable};
use crate::io::ReadableError;
use std::io;
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

impl<A: Array> AsyncReadable for ArrayVec<A>
where
    A::Item: AsyncReadable,
{
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
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

impl<A: Array> AsyncWritable for ArrayVec<A>
where
    A::Item: AsyncWritable,
{
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        self.as_slice().write_to(writer)
    }
}
