//! Async [`AsyncReadable`]/[`AsyncWritable`] implementations for `heapless`'s fixed-capacity
//! collection types (`tokio` + `heapless` features).
//!
//! Wire formats and capacity-checking behavior are identical to [`crate::ext::heapless::io`];
//! see that module for the rationale. All reads and writes are performed asynchronously using
//! [`tokio::io::AsyncReadExt`] / [`tokio::io::AsyncWriteExt`].

#![allow(clippy::manual_async_fn)]

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::DecodeError;
use crate::async_io::{
    AsyncReadFixed, AsyncReadValue, AsyncReadable, AsyncWritable, AsyncWriteFixed, AsyncWriteValue,
};
use crate::io::ReadableError;
use core::hash::{BuildHasher, Hash};
use heapless::{Deque, IndexMap, IndexSet, LinearMap, String, Vec};
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

impl<T: AsyncReadable, const N: usize> AsyncReadable for Vec<T, N> {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
            let len: u64 = reader.read_fixed().await?;
            let len = checked_len(len, N, "heapless::Vec")?;
            let mut result = Vec::new();
            for _ in 0..len {
                let item = reader.read_value().await?;
                if result.push(item).is_err() {
                    unreachable!("length was checked against capacity above")
                }
            }
            Ok(result)
        }
    }
}

impl<T: AsyncWritable, const N: usize> AsyncWritable for Vec<T, N> {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        self.as_slice().write_to(writer)
    }
}

impl<const N: usize> AsyncReadable for String<N> {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
            let len: u64 = reader.read_fixed().await?;
            let len = checked_len(len, N, "heapless::String")?;
            let mut bytes: Vec<u8, N> = Vec::new();
            bytes
                .resize_default(len)
                .expect("length was checked against capacity above");
            reader.read_exact(&mut bytes).await?;
            Ok(String::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtf8)?)
        }
    }
}

impl<const N: usize> AsyncWritable for String<N> {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        self.as_str().write_to(writer)
    }
}

impl<T: AsyncReadable, const N: usize> AsyncReadable for Deque<T, N> {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
            let len: u64 = reader.read_fixed().await?;
            let len = checked_len(len, N, "heapless::Deque")?;
            let mut result = Deque::new();
            for _ in 0..len {
                let item = reader.read_value().await?;
                if result.push_back(item).is_err() {
                    unreachable!("length was checked against capacity above")
                }
            }
            Ok(result)
        }
    }
}

impl<T: AsyncWritable, const N: usize> AsyncWritable for Deque<T, N> {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        async move {
            let len: u64 = self
                .len()
                .try_into()
                .expect("could not convert usize to u64");
            writer.write_fixed(&len).await?;
            for el in self {
                writer.write_value(el).await?;
            }
            Ok(())
        }
    }
}

impl<K, V, S, const N: usize> AsyncReadable for IndexMap<K, V, S, N>
where
    K: AsyncReadable + Eq + Hash,
    V: AsyncReadable,
    S: BuildHasher + Default,
{
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
            let len: u64 = reader.read_fixed().await?;
            let len = checked_len(len, N, "heapless::IndexMap")?;
            let mut map = IndexMap::default();
            for _ in 0..len {
                let key = reader.read_value().await?;
                let val = reader.read_value().await?;
                if map.insert(key, val).is_err() {
                    unreachable!("length was checked against capacity above")
                }
            }
            Ok(map)
        }
    }
}

impl<K: AsyncWritable, V: AsyncWritable, S, const N: usize> AsyncWritable for IndexMap<K, V, S, N> {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        async move {
            let len: u64 = self
                .len()
                .try_into()
                .expect("could not convert usize to u64");
            writer.write_fixed(&len).await?;
            for (k, v) in self {
                writer.write_value(k).await?;
                writer.write_value(v).await?;
            }
            Ok(())
        }
    }
}

impl<T, S, const N: usize> AsyncReadable for IndexSet<T, S, N>
where
    T: AsyncReadable + Eq + Hash,
    S: BuildHasher + Default,
{
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
            let len: u64 = reader.read_fixed().await?;
            let len = checked_len(len, N, "heapless::IndexSet")?;
            let mut set = IndexSet::default();
            for _ in 0..len {
                let item = reader.read_value().await?;
                if set.insert(item).is_err() {
                    unreachable!("length was checked against capacity above")
                }
            }
            Ok(set)
        }
    }
}

impl<T: AsyncWritable + Eq + Hash, S: BuildHasher, const N: usize> AsyncWritable
    for IndexSet<T, S, N>
{
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        async move {
            let len: u64 = self
                .len()
                .try_into()
                .expect("could not convert usize to u64");
            writer.write_fixed(&len).await?;
            for el in self {
                writer.write_value(el).await?;
            }
            Ok(())
        }
    }
}

impl<K: AsyncReadable + Eq, V: AsyncReadable, const N: usize> AsyncReadable for LinearMap<K, V, N> {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
            let len: u64 = reader.read_fixed().await?;
            let len = checked_len(len, N, "heapless::LinearMap")?;
            let mut map = LinearMap::new();
            for _ in 0..len {
                let key = reader.read_value().await?;
                let val = reader.read_value().await?;
                if map.insert(key, val).is_err() {
                    unreachable!("length was checked against capacity above")
                }
            }
            Ok(map)
        }
    }
}

impl<K: AsyncWritable + Eq, V: AsyncWritable, const N: usize> AsyncWritable for LinearMap<K, V, N> {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        async move {
            let len: u64 = self
                .len()
                .try_into()
                .expect("could not convert usize to u64");
            writer.write_fixed(&len).await?;
            for (k, v) in self {
                writer.write_value(k).await?;
                writer.write_value(v).await?;
            }
            Ok(())
        }
    }
}
