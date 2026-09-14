//! `EioReadable`/`EioWritable` implementations for `alloc`-backed collection types.
//!
//! The `eio`/embedded-io counterpart of the collection impls in `std_types.rs`, gated on
//! `alloc` instead of `std` (no OS/heap-randomness dependent types like `HashMap`/`HashSet`
//! here — those stay `std`-only). Mirrors `std_types.rs`'s wire formats exactly.
//!
//! No `extern crate alloc;` needed here — `src/lib.rs` already declares it crate-wide (gated
//! on the same `alloc` feature this module requires), which puts `alloc` in the 2018+ extern
//! prelude for every module in this crate.

use crate::{
    DecodeError,
    eio::{
        EioReadFixed, EioReadValue, EioReadable, EioReadableError, EioReader, EioWritable,
        EioWriteFixed, EioWriteValue, EioWriter,
    },
};
use alloc::{
    collections::{BTreeMap, BTreeSet, LinkedList, VecDeque},
    string::String,
    vec::Vec,
};

// Wire format: `u64` element count (LE), then each element serialized in order.
impl<T: EioReadable> EioReadable for Vec<T> {
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        let len: u64 = reader.read_fixed()?;
        let len: usize = len.try_into().expect("could not convert u64 to usize");
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(reader.read_value()?);
        }
        Ok(result)
    }
}

impl<T: EioWritable> EioWritable for Vec<T> {
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        self.as_slice().write_to(writer)
    }
}

// Wire format: `u64` element count (LE), then each element serialized in order.
impl<T: EioReadable> EioReadable for VecDeque<T> {
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        let len: u64 = reader.read_fixed()?;
        let len: usize = len.try_into().expect("could not convert u64 to usize");
        let mut result = VecDeque::with_capacity(len);
        for _ in 0..len {
            result.push_back(reader.read_value()?);
        }
        Ok(result)
    }
}

impl<T: EioWritable> EioWritable for VecDeque<T> {
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

// Wire format: `u64` element count (LE), then each element serialized in order.
impl<T: EioReadable> EioReadable for LinkedList<T> {
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        let len: u64 = reader.read_fixed()?;
        let len: usize = len.try_into().expect("could not convert u64 to usize");
        let mut result = LinkedList::new();
        for _ in 0..len {
            result.push_back(reader.read_value()?);
        }
        Ok(result)
    }
}

impl<T: EioWritable> EioWritable for LinkedList<T> {
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

// Wire format: `u64` entry count (LE), then alternating key/value pairs in sorted order.
impl<K: EioReadable + Ord, V: EioReadable> EioReadable for BTreeMap<K, V> {
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        let len: u64 = reader.read_fixed()?;
        let len: usize = len.try_into().expect("could not convert u64 to usize");
        let mut map = BTreeMap::new();
        for _ in 0..len {
            let key = reader.read_value()?;
            let val = reader.read_value()?;
            map.insert(key, val);
        }
        Ok(map)
    }
}

impl<K: EioWritable, V: EioWritable> EioWritable for BTreeMap<K, V> {
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

// Wire format: `u64` element count (LE), then each element in sorted order.
impl<T: EioReadable + Ord> EioReadable for BTreeSet<T> {
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        let len: u64 = reader.read_fixed()?;
        let len: usize = len.try_into().expect("could not convert u64 to usize");
        let mut set = BTreeSet::new();
        for _ in 0..len {
            set.insert(reader.read_value()?);
        }
        Ok(set)
    }
}

impl<T: EioWritable> EioWritable for BTreeSet<T> {
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

// Wire format: `u64` byte length (LE) + UTF-8 bytes. Rejects invalid UTF-8 with DecodeError.
impl EioReadable for String {
    fn read_from<R: EioReader + ?Sized>(
        reader: &mut R,
    ) -> Result<Self, EioReadableError<R::Error>> {
        let len: u64 = reader.read_fixed()?;
        let len: usize = len.try_into().expect("could not convert u64 to usize");
        let mut bytes = alloc::vec![0u8; len];
        reader.read_exact(&mut bytes)?;
        String::from_utf8(bytes)
            .map_err(|_| EioReadableError::DecodeError(DecodeError::InvalidUtf8))
    }
}

impl EioWritable for String {
    fn write_to<W: EioWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        self.as_str().write_to(writer)
    }
}
