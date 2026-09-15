//! `EioReadable`/`EioWritable` implementations for `heapless`'s fixed-capacity collection
//! types (`embedded-io` + `heapless` features).
//!
//! Wire formats and capacity-checking behavior are identical to [`crate::ext::heapless::io`];
//! see that module for the rationale. Unlike the `alloc`-backed collections in
//! `alloc_types_eio.rs`, none of these types need the `alloc` feature — every collection here
//! is stack-allocated with a compile-time capacity.

use crate::{
    DecodeError,
    eio::{
        EioReadFixed, EioReadValue, EioReadable, EioReadableError, EioReader, EioWritable,
        EioWriteFixed, EioWriteValue, EioWriter,
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

impl<T: EioReadable, const N: usize> EioReadable for Vec<T, N> {
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        let len: u64 = reader.read_fixed()?;
        let len = checked_len(len, N, "heapless::Vec")?;
        let mut result = Vec::new();
        for _ in 0..len {
            let item = reader.read_value()?;
            if result.push(item).is_err() {
                unreachable!("length was checked against capacity above")
            }
        }
        Ok(result)
    }
}

impl<T: EioWritable, const N: usize> EioWritable for Vec<T, N> {
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        self.as_slice().write_to(writer)
    }
}

impl<const N: usize> EioReadable for String<N> {
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        let len: u64 = reader.read_fixed()?;
        let len = checked_len(len, N, "heapless::String")?;
        let mut bytes: Vec<u8, N> = Vec::new();
        bytes
            .resize_default(len)
            .expect("length was checked against capacity above");
        reader.read_exact(&mut bytes)?;
        Ok(String::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtf8)?)
    }
}

impl<const N: usize> EioWritable for String<N> {
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        self.as_str().write_to(writer)
    }
}

impl<T: EioReadable, const N: usize> EioReadable for Deque<T, N> {
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        let len: u64 = reader.read_fixed()?;
        let len = checked_len(len, N, "heapless::Deque")?;
        let mut result = Deque::new();
        for _ in 0..len {
            let item = reader.read_value()?;
            if result.push_back(item).is_err() {
                unreachable!("length was checked against capacity above")
            }
        }
        Ok(result)
    }
}

impl<T: EioWritable, const N: usize> EioWritable for Deque<T, N> {
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        let len: u64 = self
            .len()
            .try_into()
            .expect("could not convert usize to u64");
        writer.write_fixed(&len)?;
        for el in self {
            writer.write_value(el)?;
        }
        Ok(())
    }
}

impl<K, V, S, const N: usize> EioReadable for IndexMap<K, V, S, N>
where
    K: EioReadable + Eq + Hash,
    V: EioReadable,
    S: BuildHasher + Default,
{
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        let len: u64 = reader.read_fixed()?;
        let len = checked_len(len, N, "heapless::IndexMap")?;
        let mut map = IndexMap::default();
        for _ in 0..len {
            let key = reader.read_value()?;
            let val = reader.read_value()?;
            if map.insert(key, val).is_err() {
                unreachable!("length was checked against capacity above")
            }
        }
        Ok(map)
    }
}

impl<K: EioWritable, V: EioWritable, S, const N: usize> EioWritable for IndexMap<K, V, S, N> {
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        let len: u64 = self
            .len()
            .try_into()
            .expect("could not convert usize to u64");
        writer.write_fixed(&len)?;
        for (k, v) in self {
            writer.write_value(k)?;
            writer.write_value(v)?;
        }
        Ok(())
    }
}

impl<T, S, const N: usize> EioReadable for IndexSet<T, S, N>
where
    T: EioReadable + Eq + Hash,
    S: BuildHasher + Default,
{
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        let len: u64 = reader.read_fixed()?;
        let len = checked_len(len, N, "heapless::IndexSet")?;
        let mut set = IndexSet::default();
        for _ in 0..len {
            let item = reader.read_value()?;
            if set.insert(item).is_err() {
                unreachable!("length was checked against capacity above")
            }
        }
        Ok(set)
    }
}

impl<T: EioWritable + Eq + Hash, S: BuildHasher, const N: usize> EioWritable for IndexSet<T, S, N> {
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        let len: u64 = self
            .len()
            .try_into()
            .expect("could not convert usize to u64");
        writer.write_fixed(&len)?;
        for el in self {
            writer.write_value(el)?;
        }
        Ok(())
    }
}

impl<K: EioReadable + Eq, V: EioReadable, const N: usize> EioReadable for LinearMap<K, V, N> {
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        let len: u64 = reader.read_fixed()?;
        let len = checked_len(len, N, "heapless::LinearMap")?;
        let mut map = LinearMap::new();
        for _ in 0..len {
            let key = reader.read_value()?;
            let val = reader.read_value()?;
            if map.insert(key, val).is_err() {
                unreachable!("length was checked against capacity above")
            }
        }
        Ok(map)
    }
}

impl<K: EioWritable + Eq, V: EioWritable, const N: usize> EioWritable for LinearMap<K, V, N> {
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        let len: u64 = self
            .len()
            .try_into()
            .expect("could not convert usize to u64");
        writer.write_fixed(&len)?;
        for (k, v) in self {
            writer.write_value(k)?;
            writer.write_value(v)?;
        }
        Ok(())
    }
}
