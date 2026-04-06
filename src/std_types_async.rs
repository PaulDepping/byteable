//! Async [`AsyncReadable`] and [`AsyncWritable`] implementations for standard-library
//! collection and pointer types (tokio feature).
//!
//! Wire formats are identical to those in [`crate::std_types`]; see that module for the
//! encoding reference table. All reads and writes are performed asynchronously using
//! [`tokio::io::AsyncReadExt`] / [`tokio::io::AsyncWriteExt`].

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    AsyncReadFixed, AsyncReadValue, AsyncReadable, AsyncWritable, AsyncWriteFixed, AsyncWriteValue,
    DecodeError, io::ReadableError,
};
use core::{
    ffi::CStr,
    hash::{BuildHasher, Hash},
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, LinkedList, VecDeque},
    ffi::CString,
    io::{self},
    path::{Path, PathBuf},
};

impl<T: AsyncReadable> AsyncReadable for Vec<T> {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
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

impl<T: AsyncReadable> AsyncReadable for VecDeque<T> {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
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

impl<T: AsyncReadable> AsyncReadable for LinkedList<T> {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
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

impl<K, V, S> AsyncReadable for HashMap<K, V, S>
where
    K: AsyncReadable + Eq + std::hash::Hash,
    V: AsyncReadable,
    S: BuildHasher + Default,
{
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
            let len: u64 = reader.read_fixed().await?;
            let len: usize = len.try_into().expect("could not convert u64 to usize");
            let mut map = HashMap::with_capacity_and_hasher(len, S::default());
            for _ in 0..len {
                let key = reader.read_value().await?;
                let val = reader.read_value().await?;
                map.insert(key, val);
            }
            Ok(map)
        }
    }
}

impl<T, S> AsyncReadable for HashSet<T, S>
where
    T: AsyncReadable + Eq + Hash,
    S: BuildHasher + Default,
{
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
            let len: u64 = reader.read_fixed().await?;
            let len: usize = len.try_into().expect("could not convert u64 to usize");
            let mut set = HashSet::with_capacity_and_hasher(len, S::default());
            for _ in 0..len {
                set.insert(reader.read_value().await?);
            }
            Ok(set)
        }
    }
}

impl<K: AsyncReadable + Ord, V: AsyncReadable> AsyncReadable for BTreeMap<K, V> {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
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

impl<T: AsyncReadable + Ord> AsyncReadable for BTreeSet<T> {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
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

impl<T: AsyncReadable> AsyncReadable for Option<T> {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
            let tag: u8 = reader.read_fixed().await?;
            match tag {
                0 => Ok(None),
                1 => Ok(Some(reader.read_value().await?)),
                _ => Err(ReadableError::DecodeError(DecodeError::InvalidTag {
                    raw: tag,
                    type_name: "Option",
                })),
            }
        }
    }
}

impl<V: AsyncReadable, E: AsyncReadable> AsyncReadable for Result<V, E> {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
            let discriminator: u8 = reader.read_fixed().await?;
            match discriminator {
                0 => Ok(Ok(reader.read_value().await?)),
                1 => Ok(Err(reader.read_value().await?)),
                _ => Err(ReadableError::DecodeError(DecodeError::InvalidTag {
                    raw: discriminator,
                    type_name: "Result",
                })),
            }
        }
    }
}

impl AsyncReadable for String {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
            let len: u64 = reader.read_fixed().await?;
            let len: usize = len.try_into().expect("could not convert u64 to usize");
            let mut bytes = vec![0u8; len];
            reader.read_exact(&mut bytes).await?;
            String::from_utf8(bytes)
                .map_err(|_| ReadableError::DecodeError(DecodeError::InvalidUtf8))
        }
    }
}

impl AsyncReadable for PathBuf {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
            let s = String::read_from(reader).await?;
            Ok(PathBuf::from(s))
        }
    }
}

impl AsyncReadable for CString {
    fn read_from(
        reader: &mut (impl AsyncReadExt + ?Sized + Unpin),
    ) -> impl Future<Output = Result<Self, ReadableError>> {
        async {
            let v = Vec::read_from(reader).await?;
            CString::new(v).map_err(|_| ReadableError::DecodeError(DecodeError::InvalidCString))
        }
    }
}

impl<T: AsyncWritable> AsyncWritable for [T] {
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

impl<T: AsyncWritable> AsyncWritable for Vec<T> {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        self.as_slice().write_to(writer)
    }
}

impl<T: AsyncWritable> AsyncWritable for VecDeque<T> {
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

impl<T: AsyncWritable> AsyncWritable for LinkedList<T> {
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

impl<K, V, S> AsyncWritable for HashMap<K, V, S>
where
    K: AsyncWritable,
    V: AsyncWritable,
    S: BuildHasher,
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
            for (k, v) in self {
                writer.write_value(k).await?;
                writer.write_value(v).await?;
            }
            Ok(())
        }
    }
}

impl<T, S> AsyncWritable for HashSet<T, S>
where
    T: AsyncWritable,
    S: BuildHasher,
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

impl<K: AsyncWritable, V: AsyncWritable> AsyncWritable for BTreeMap<K, V> {
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

impl<T: AsyncWritable> AsyncWritable for BTreeSet<T> {
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

impl<T: AsyncWritable> AsyncWritable for Option<T> {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        async move {
            match self {
                None => writer.write_fixed(&0u8).await,
                Some(val) => {
                    writer.write_fixed(&1u8).await?;
                    writer.write_value(val).await
                }
            }
        }
    }
}

impl<V: AsyncWritable, E: AsyncWritable> AsyncWritable for Result<V, E> {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        async move {
            match self {
                Ok(val) => {
                    writer.write_fixed(&0u8).await?;
                    writer.write_value(val).await
                }
                Err(err) => {
                    writer.write_fixed(&1u8).await?;
                    writer.write_value(err).await
                }
            }
        }
    }
}

impl AsyncWritable for str {
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
            writer.write_all(self.as_bytes()).await
        }
    }
}

impl AsyncWritable for Path {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        async move {
            match self.to_str() {
                Some(s) => s.write_to(writer).await,
                None => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "path contains invalid UTF-8 characters",
                )),
            }
        }
    }
}

impl AsyncWritable for CStr {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        self.to_bytes().write_to(writer)
    }
}

impl AsyncWritable for String {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        self.as_str().write_to(writer)
    }
}

impl AsyncWritable for CString {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        self.as_bytes().write_to(writer)
    }
}

impl AsyncWritable for PathBuf {
    fn write_to(
        &self,
        writer: &mut (impl AsyncWriteExt + ?Sized + Unpin),
    ) -> impl Future<Output = io::Result<()>> {
        self.as_path().write_to(writer)
    }
}
