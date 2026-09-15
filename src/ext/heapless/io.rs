//! `Readable`/`Writable` implementations for `heapless`'s fixed-capacity collection types
//! (`std` + `heapless` features).
//!
//! Wire formats mirror the `alloc`-backed collections in `std_types.rs` exactly (`u64`
//! element/byte count, then elements/bytes in order), except that decoding checks the wire
//! count against the container's compile-time capacity `N` and returns
//! [`DecodeError::CapacityExceeded`] instead of silently truncating or panicking.

use crate::{
    DecodeError,
    io::{ReadFixed, ReadValue, Readable, ReadableError, Writable, WriteFixed, WriteValue},
};
use core::hash::{BuildHasher, Hash};
use heapless::{Deque, IndexMap, IndexSet, LinearMap, String, Vec};
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
impl<T: Readable, const N: usize> Readable for Vec<T, N> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
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

impl<T: Writable, const N: usize> Writable for Vec<T, N> {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
        self.as_slice().write_to(writer)
    }
}

// Wire format: `u64` byte length (LE) + UTF-8 bytes. Rejects invalid UTF-8 with DecodeError.
impl<const N: usize> Readable for String<N> {
    fn read_from(reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let len: u64 = reader.read_fixed()?;
        let len = checked_len(len, N, "heapless::String")?;
        let mut bytes: Vec<u8, N> = Vec::new();
        bytes
            .resize_default(len)
            .expect("length was checked against capacity above");
        reader.read_exact(&mut bytes)?;
        String::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtf8.into())
    }
}

impl<const N: usize> Writable for String<N> {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
        self.as_str().write_to(writer)
    }
}

// Wire format: `u64` element count (LE), then each element serialized in order.
impl<T: Readable, const N: usize> Readable for Deque<T, N> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
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

impl<T: Writable, const N: usize> Writable for Deque<T, N> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
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

// Wire format: `u64` entry count (LE), then alternating key/value pairs, in insertion order.
impl<K, V, S, const N: usize> Readable for IndexMap<K, V, S, N>
where
    K: Readable + Eq + Hash,
    V: Readable,
    S: BuildHasher + Default,
{
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
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

impl<K: Writable, V: Writable, S, const N: usize> Writable for IndexMap<K, V, S, N> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
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

// Wire format: `u64` element count (LE), then each element in insertion order.
impl<T, S, const N: usize> Readable for IndexSet<T, S, N>
where
    T: Readable + Eq + Hash,
    S: BuildHasher + Default,
{
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
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

impl<T: Writable + Eq + Hash, S: BuildHasher, const N: usize> Writable for IndexSet<T, S, N> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
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

// Wire format: `u64` entry count (LE), then alternating key/value pairs, in insertion order.
impl<K: Readable + Eq, V: Readable, const N: usize> Readable for LinearMap<K, V, N> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
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

impl<K: Writable + Eq, V: Writable, const N: usize> Writable for LinearMap<K, V, N> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> std::io::Result<()> {
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
