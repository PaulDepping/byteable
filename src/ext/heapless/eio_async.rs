//! `EioAsyncReadable`/`EioAsyncWritable` implementations for `heapless`'s fixed-capacity
//! collection types (`embedded-io-async` + `heapless` features).
//!
//! Wire formats and capacity-checking behavior are identical to [`crate::ext::heapless::io`]
//! and [`crate::ext::heapless::eio`]; see the former for the rationale. Like
//! `ext/heapless/eio.rs`, none of these types need the `alloc` feature.

#![allow(clippy::manual_async_fn)]

use crate::eio::EioReadableError;
use crate::{
    DecodeError,
    eio_async::{
        EioAsyncReadFixed, EioAsyncReadValue, EioAsyncReadable, EioAsyncReader, EioAsyncWritable,
        EioAsyncWriteFixed, EioAsyncWriteValue, EioAsyncWriter,
    },
};
use core::hash::{BuildHasher, Hash};
use heapless::{Deque, IndexMap, IndexSet, LinearMap, String, Vec};

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

impl<T: EioAsyncReadable, const N: usize> EioAsyncReadable for Vec<T, N> {
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
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

impl<T: EioAsyncWritable, const N: usize> EioAsyncWritable for Vec<T, N> {
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
        self.as_slice().write_to(writer)
    }
}

impl<const N: usize> EioAsyncReadable for String<N> {
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
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

impl<const N: usize> EioAsyncWritable for String<N> {
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
        self.as_str().write_to(writer)
    }
}

impl<T: EioAsyncReadable, const N: usize> EioAsyncReadable for Deque<T, N> {
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
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

impl<T: EioAsyncWritable, const N: usize> EioAsyncWritable for Deque<T, N> {
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
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

impl<K, V, S, const N: usize> EioAsyncReadable for IndexMap<K, V, S, N>
where
    K: EioAsyncReadable + Eq + Hash,
    V: EioAsyncReadable,
    S: BuildHasher + Default,
{
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
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

impl<K: EioAsyncWritable, V: EioAsyncWritable, S, const N: usize> EioAsyncWritable
    for IndexMap<K, V, S, N>
{
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
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

impl<T, S, const N: usize> EioAsyncReadable for IndexSet<T, S, N>
where
    T: EioAsyncReadable + Eq + Hash,
    S: BuildHasher + Default,
{
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
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

impl<T: EioAsyncWritable + Eq + Hash, S: BuildHasher, const N: usize> EioAsyncWritable
    for IndexSet<T, S, N>
{
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
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

impl<K: EioAsyncReadable + Eq, V: EioAsyncReadable, const N: usize> EioAsyncReadable
    for LinearMap<K, V, N>
{
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
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

impl<K: EioAsyncWritable + Eq, V: EioAsyncWritable, const N: usize> EioAsyncWritable
    for LinearMap<K, V, N>
{
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
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
