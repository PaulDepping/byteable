//! [`Readable`] and [`Writable`] implementations for standard-library collection and
//! pointer types.
//!
//! ## Wire formats
//!
//! | Type | Encoding |
//! |------|----------|
//! | `Arc<T>` / `Rc<T>` / `Box<T>` | transparent passthrough to inner `T` |
//! | `Vec<T>` / `VecDeque<T>` / `LinkedList<T>` | `u64` element count + elements in order |
//! | `HashMap<K,V>` / `HashSet<T>` | `u64` count + alternating key/value (or element) pairs |
//! | `BTreeMap<K,V>` / `BTreeSet<T>` | same as `HashMap` / `HashSet`, iteration order is sorted |
//! | `Option<T>` | 1-byte tag: `0` = `None`, `1` = `Some` followed by the value |
//! | `Result<V,E>` | 1-byte tag: `0` = `Ok` followed by value, `1` = `Err` followed by error |
//! | `String` | `u64` byte length + UTF-8 bytes (no null terminator) |
//! | `str` (write only) | same as `String` |
//! | `PathBuf` | serialized as `String`; returns [`io::Error`] for non-UTF-8 paths |
//! | `Path` (write only) | same as `PathBuf` |
//! | `CString` | `Vec<u8>` bytes **without** the terminating null |
//! | `CStr` (write only) | same as `CString` |
//!
//! All multi-byte length prefixes are little-endian `u64`.

use crate::{
    DecodeError, FromRawRepr, RawRepr, TryFromRawRepr,
    io::{ReadFixed, ReadValue, Readable, ReadableError, Writable, WriteFixed, WriteValue},
};
use core::{
    ffi::CStr,
    hash::{BuildHasher, Hash},
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, LinkedList, VecDeque},
    ffi::CString,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

macro_rules! smart_pointer_passthrough {
    ($($wrapper:ident),+) => {
        $(
            impl<T: RawRepr> RawRepr for $wrapper<T> {
                type Raw = T::Raw;

                fn to_raw(&self) -> Self::Raw {
                    self.as_ref().to_raw()
                }
            }

            impl<T: FromRawRepr> FromRawRepr for $wrapper<T> {
                fn from_raw(raw: Self::Raw) -> Self {
                    Self::new(T::from_raw(raw))
                }
            }

            impl<T: TryFromRawRepr> TryFromRawRepr for $wrapper<T> {
                fn try_from_raw(raw: Self::Raw) -> Result<Self, DecodeError> {
                    Ok(Self::new(T::try_from_raw(raw)?))
                }
            }
        )+
    };
}

smart_pointer_passthrough!(Arc, Rc, Box);

// Wire format: `u64` element count (LE), then each element serialized in order.
impl<T: Readable> Readable for Vec<T> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let len: u64 = reader.read_fixed()?;
        let len: usize = len.try_into().expect("could not convert u64 to usize");
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(reader.read_value()?);
        }
        Ok(result)
    }
}

// Wire format: `u64` element count (LE), then each element serialized in order.
impl<T: Readable> Readable for VecDeque<T> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let len: u64 = reader.read_fixed()?;
        let len: usize = len.try_into().expect("could not convert u64 to usize");
        let mut result = VecDeque::with_capacity(len);
        for _ in 0..len {
            result.push_back(reader.read_value()?);
        }
        Ok(result)
    }
}

// Wire format: `u64` element count (LE), then each element serialized in order.
impl<T: Readable> Readable for LinkedList<T> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let len: u64 = reader.read_fixed()?;
        let len: usize = len.try_into().expect("could not convert u64 to usize");
        let mut result = LinkedList::new();
        for _ in 0..len {
            result.push_back(reader.read_value()?);
        }
        Ok(result)
    }
}

// Wire format: `u64` entry count (LE), then alternating key/value pairs.
impl<K, V, S> Readable for HashMap<K, V, S>
where
    K: Readable + Eq + std::hash::Hash,
    V: Readable,
    S: BuildHasher + Default,
{
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let len: u64 = reader.read_fixed()?;
        let len: usize = len.try_into().expect("could not convert u64 to usize");
        let mut map = HashMap::with_capacity_and_hasher(len, S::default());
        for _ in 0..len {
            let key = reader.read_value()?;
            let val = reader.read_value()?;
            map.insert(key, val);
        }
        Ok(map)
    }
}

// Wire format: `u64` element count (LE), then each element serialized in order.
impl<T, S> Readable for HashSet<T, S>
where
    T: Readable + Eq + Hash,
    S: BuildHasher + Default,
{
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let len: u64 = reader.read_fixed()?;
        let len: usize = len.try_into().expect("could not convert u64 to usize");
        let mut set = HashSet::with_capacity_and_hasher(len, S::default());
        for _ in 0..len {
            set.insert(reader.read_value()?);
        }
        Ok(set)
    }
}

// Wire format: `u64` entry count (LE), then alternating key/value pairs in sorted order.
impl<K: Readable + Ord, V: Readable> Readable for BTreeMap<K, V> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
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

// Wire format: `u64` element count (LE), then each element in sorted order.
impl<T: Readable + Ord> Readable for BTreeSet<T> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let len: u64 = reader.read_fixed()?;
        let len: usize = len.try_into().expect("could not convert u64 to usize");
        let mut set = BTreeSet::new();
        for _ in 0..len {
            set.insert(reader.read_value()?);
        }
        Ok(set)
    }
}

// Wire format: 1-byte tag (0 = None, 1 = Some), followed by the value when Some.
impl<T: Readable> Readable for Option<T> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let tag: u8 = reader.read_fixed()?;
        match tag {
            0 => Ok(None),
            1 => Ok(Some(reader.read_value()?)),
            _ => Err(ReadableError::DecodeError(DecodeError::InvalidTag {
                raw: tag,
                type_name: "Option",
            })),
        }
    }
}

// Wire format: 1-byte tag (0 = Ok, 1 = Err), followed by the Ok value or Err value.
impl<V: Readable, E: Readable> Readable for Result<V, E> {
    fn read_from(mut reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let discriminator: u8 = reader.read_fixed()?;
        match discriminator {
            0 => Ok(Ok(reader.read_value()?)),
            1 => Ok(Err(reader.read_value()?)),
            _ => Err(ReadableError::DecodeError(DecodeError::InvalidTag {
                raw: discriminator,
                type_name: "Result",
            })),
        }
    }
}

// Wire format: `u64` byte length (LE) + UTF-8 bytes. Rejects invalid UTF-8 with DecodeError.
impl Readable for String {
    fn read_from(reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let len: u64 = reader.read_fixed()?;
        let len: usize = len.try_into().expect("could not convert u64 to usize");
        let mut bytes = vec![0u8; len];
        reader.read_exact(&mut bytes)?;
        String::from_utf8(bytes).map_err(|_| ReadableError::DecodeError(DecodeError::InvalidUtf8))
    }
}

// Wire format: same as String (UTF-8). Non-UTF-8 paths cannot be deserialized.
impl Readable for PathBuf {
    fn read_from(reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let s = String::read_from(reader)?;
        Ok(PathBuf::from(s))
    }
}

// Wire format: Vec<u8> bytes without the null terminator. Rejects interior nulls.
impl Readable for CString {
    fn read_from(reader: &mut (impl Read + ?Sized)) -> Result<Self, ReadableError> {
        let v = Vec::read_from(reader)?;
        CString::new(v).map_err(|_| ReadableError::DecodeError(DecodeError::InvalidCString))
    }
}

// Wire format: `u64` element count (LE), then each element serialized in order.
impl<T: Writable> Writable for [T] {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
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

impl<T: Writable> Writable for Vec<T> {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        self.as_slice().write_to(writer)
    }
}

impl<T: Writable> Writable for VecDeque<T> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
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

impl<T: Writable> Writable for LinkedList<T> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
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

impl<K, V, S> Writable for HashMap<K, V, S>
where
    K: Writable,
    V: Writable,
    S: BuildHasher,
{
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
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

impl<T, S> Writable for HashSet<T, S>
where
    T: Writable,
    S: BuildHasher,
{
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
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

impl<K: Writable, V: Writable> Writable for BTreeMap<K, V> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
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

impl<T: Writable> Writable for BTreeSet<T> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
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

impl<T: Writable> Writable for Option<T> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        match self {
            None => writer.write_fixed(&0u8),
            Some(val) => {
                writer.write_fixed(&1u8)?;
                writer.write_value(val)
            }
        }
    }
}

impl<V: Writable, E: Writable> Writable for Result<V, E> {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        match self {
            Ok(val) => {
                writer.write_fixed(&0u8)?;
                writer.write_value(val)
            }
            Err(err) => {
                writer.write_fixed(&1u8)?;
                writer.write_value(err)
            }
        }
    }
}

impl Writable for str {
    fn write_to(&self, mut writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        let len: u64 = self
            .len()
            .try_into()
            .expect("could not convert usize to u64");
        writer.write_fixed(&len)?;
        writer.write_all(self.as_bytes())
    }
}

impl Writable for Path {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        match self.to_str() {
            Some(s) => s.write_to(writer),
            None => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "path contains invalid UTF-8 characters",
            )),
        }
    }
}

impl Writable for CStr {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        self.to_bytes().write_to(writer)
    }
}

impl Writable for String {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        self.as_str().write_to(writer)
    }
}

impl Writable for CString {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        self.as_bytes().write_to(writer)
    }
}

impl Writable for PathBuf {
    fn write_to(&self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        self.as_path().write_to(writer)
    }
}
