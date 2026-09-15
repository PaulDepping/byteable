//! `EioAsyncReadable`/`EioAsyncWritable` implementations for `arrayvec`'s fixed-capacity
//! collection types (`embedded-io-async` + `arrayvec` features).
//!
//! Wire formats and capacity-checking behavior are identical to
//! [`crate::ext::arrayvec::io`] and [`crate::ext::arrayvec::eio`]; see the former for the
//! rationale.

#![allow(clippy::manual_async_fn)]

use crate::eio::EioReadableError;
use crate::{
    DecodeError,
    eio_async::{
        EioAsyncReadFixed, EioAsyncReadValue, EioAsyncReadable, EioAsyncReader, EioAsyncWritable,
        EioAsyncWriter,
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

impl<T: EioAsyncReadable, const N: usize> EioAsyncReadable for ArrayVec<T, N> {
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
            let len: u64 = reader.read_fixed().await?;
            let len = checked_len(len, N, "arrayvec::ArrayVec")?;
            let mut result = ArrayVec::new();
            for _ in 0..len {
                let item = reader.read_value().await?;
                if result.try_push(item).is_err() {
                    unreachable!("length was checked against capacity above")
                }
            }
            Ok(result)
        }
    }
}

impl<T: EioAsyncWritable, const N: usize> EioAsyncWritable for ArrayVec<T, N> {
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
        self.as_slice().write_to(writer)
    }
}

impl<const N: usize> EioAsyncReadable for ArrayString<N> {
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
            let len: u64 = reader.read_fixed().await?;
            let len = checked_len(len, N, "arrayvec::ArrayString")?;
            let mut buf = [0u8; N];
            reader.read_exact(&mut buf[..len]).await?;
            let s = core::str::from_utf8(&buf[..len]).map_err(|_| DecodeError::InvalidUtf8)?;
            match ArrayString::from(s) {
                Ok(s) => Ok(s),
                Err(_) => unreachable!("length was checked against capacity above"),
            }
        }
    }
}

impl<const N: usize> EioAsyncWritable for ArrayString<N> {
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
        self.as_str().write_to(writer)
    }
}
