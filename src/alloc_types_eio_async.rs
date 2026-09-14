//! `EioAsyncReadable`/`EioAsyncWritable` implementations for `alloc`-backed collection types.
//!
//! The `eio_async` counterpart of the collection impls in `alloc_types_eio.rs`, gated on
//! `alloc` instead of `std` (no OS/heap-randomness dependent types like `HashMap`/`HashSet`
//! here — those stay `std`-only, and get their `EioAsyncReadable`/`EioAsyncWritable` impls
//! alongside their sync `EioReadable`/`EioWritable` ones directly in `std_types.rs` instead).
//! Mirrors `alloc_types_eio.rs`'s wire formats exactly.
//!
//! No `extern crate alloc;` needed here — `src/lib.rs` already declares it crate-wide (gated
//! on the same `alloc` feature this module requires), which puts `alloc` in the 2018+ extern
//! prelude for every module in this crate.

use crate::{
    DecodeError,
    eio_async::{
        EioAsyncReadFixed, EioAsyncReadValue, EioAsyncReadable, EioAsyncReader, EioAsyncWritable,
        EioAsyncWriteFixed, EioAsyncWriteValue, EioAsyncWriter,
    },
};
use crate::eio::EioReadableError;
use alloc::{
    collections::{BTreeMap, BTreeSet, LinkedList, VecDeque},
    string::String,
    vec::Vec,
};

// Wire format: `u64` element count (LE), then each element serialized in order.
impl<T: EioAsyncReadable> EioAsyncReadable for Vec<T> {
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
            let len: u64 = reader.read_fixed().await?;
            let len: usize = len.try_into().expect("could not convert u64 to usize");
            let mut result = Vec::with_capacity(len);
            for _ in 0..len {
                result.push(reader.read_value().await?);
            }
            Ok(result)
        }
    }
}

impl<T: EioAsyncWritable> EioAsyncWritable for Vec<T> {
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
        self.as_slice().write_to(writer)
    }
}

// Wire format: `u64` element count (LE), then each element serialized in order.
impl<T: EioAsyncReadable> EioAsyncReadable for VecDeque<T> {
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
            let len: u64 = reader.read_fixed().await?;
            let len: usize = len.try_into().expect("could not convert u64 to usize");
            let mut result = VecDeque::with_capacity(len);
            for _ in 0..len {
                result.push_back(reader.read_value().await?);
            }
            Ok(result)
        }
    }
}

impl<T: EioAsyncWritable> EioAsyncWritable for VecDeque<T> {
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

// Wire format: `u64` element count (LE), then each element serialized in order.
impl<T: EioAsyncReadable> EioAsyncReadable for LinkedList<T> {
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
            let len: u64 = reader.read_fixed().await?;
            let len: usize = len.try_into().expect("could not convert u64 to usize");
            let mut result = LinkedList::new();
            for _ in 0..len {
                result.push_back(reader.read_value().await?);
            }
            Ok(result)
        }
    }
}

impl<T: EioAsyncWritable> EioAsyncWritable for LinkedList<T> {
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

// Wire format: `u64` entry count (LE), then alternating key/value pairs in sorted order.
impl<K: EioAsyncReadable + Ord, V: EioAsyncReadable> EioAsyncReadable for BTreeMap<K, V> {
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
            let len: u64 = reader.read_fixed().await?;
            let len: usize = len.try_into().expect("could not convert u64 to usize");
            let mut map = BTreeMap::new();
            for _ in 0..len {
                let key = reader.read_value().await?;
                let val = reader.read_value().await?;
                map.insert(key, val);
            }
            Ok(map)
        }
    }
}

impl<K: EioAsyncWritable, V: EioAsyncWritable> EioAsyncWritable for BTreeMap<K, V> {
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

// Wire format: `u64` element count (LE), then each element in sorted order.
impl<T: EioAsyncReadable + Ord> EioAsyncReadable for BTreeSet<T> {
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
            let len: u64 = reader.read_fixed().await?;
            let len: usize = len.try_into().expect("could not convert u64 to usize");
            let mut set = BTreeSet::new();
            for _ in 0..len {
                set.insert(reader.read_value().await?);
            }
            Ok(set)
        }
    }
}

impl<T: EioAsyncWritable> EioAsyncWritable for BTreeSet<T> {
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

// Wire format: `u64` byte length (LE) + UTF-8 bytes. Rejects invalid UTF-8 with DecodeError.
impl EioAsyncReadable for String {
    fn read_from<R: EioAsyncReader + ?Sized>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self, EioReadableError<R::Error>>> {
        async move {
            let len: u64 = reader.read_fixed().await?;
            let len: usize = len.try_into().expect("could not convert u64 to usize");
            let mut bytes = alloc::vec![0u8; len];
            reader.read_exact(&mut bytes).await?;
            String::from_utf8(bytes)
                .map_err(|_| EioReadableError::DecodeError(DecodeError::InvalidUtf8))
        }
    }
}

impl EioAsyncWritable for String {
    fn write_to<W: EioAsyncWriter + ?Sized>(
        &self,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), W::Error>> {
        self.as_str().write_to(writer)
    }
}
