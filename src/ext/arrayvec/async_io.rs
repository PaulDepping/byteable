//! Async [`AsyncReadable`]/[`AsyncWritable`] implementations for `arrayvec`'s fixed-capacity
//! collection types (`tokio` + `arrayvec` features).
//!
//! Wire formats and capacity-checking behavior are identical to
//! [`crate::ext::arrayvec::io`]; see that module for the rationale. All reads and writes are
//! performed asynchronously using [`tokio::io::AsyncReadExt`] / [`tokio::io::AsyncWriteExt`].

#![allow(clippy::manual_async_fn)]

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::DecodeError;
use crate::async_io::{AsyncReadFixed, AsyncReadValue, AsyncReadable, AsyncWritable};
use crate::io::ReadableError;
use arrayvec::{ArrayString, ArrayVec};
use std::io;

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

impl<T: AsyncReadable, const N: usize> AsyncReadable for ArrayVec<T, N> {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
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

impl<T: AsyncWritable, const N: usize> AsyncWritable for ArrayVec<T, N> {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        self.as_slice().write_to(writer)
    }
}

impl<const N: usize> AsyncReadable for ArrayString<N> {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
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

impl<const N: usize> AsyncWritable for ArrayString<N> {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        self.as_str().write_to(writer)
    }
}
