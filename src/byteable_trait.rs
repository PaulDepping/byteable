/// Marker trait for types that can be safely transmuted to and from raw bytes.
///
/// A type may implement `PlainOldData` only when **all** of the following hold:
///
/// - It has no padding bytes (i.e. every byte in `size_of::<Self>()` is part of a field).
/// - Every possible bit pattern is a valid value (no invalid states, no pointer indirection).
/// - It is `Copy` and `Sized`.
///
/// These invariants are the precondition for the safe use of `transmute` in
/// [`IntoByteArray`] and [`FromByteArray`], and for reinterpreting the type as a
/// `&[u8]` slice via [`as_bytes`](PlainOldData::as_bytes).
///
/// Implemented for: `u8`, `i8`, `u16`, `u32`, `u64`, `u128`, `i16`, `i32`, `i64`, `i128`,
/// `f32`, `f64`, [`BigEndian<T>`], [`LittleEndian<T>`], and fixed-size arrays `[T; N]` where
/// `T: PlainOldData`.
///
/// # Safety
///
/// The implementor must guarantee the invariants above. Violating them causes
/// undefined behaviour in the `transmute`-based paths.
pub unsafe trait PlainOldData: Copy + Sized {
    /// The number of bytes this type occupies in memory, equal to `size_of::<Self>()`.
    const BYTE_SIZE: usize = core::mem::size_of::<Self>();

    /// Returns a zero-initialized value of this type.
    ///
    /// Because every bit pattern is valid (by the `PlainOldData` contract), all-zeros is
    /// guaranteed to be a safe value.
    #[inline]
    fn zeroed() -> Self {
        unsafe { core::mem::zeroed() }
    }

    /// Returns a mutable view of this value as a byte slice.
    ///
    /// The returned slice has length [`BYTE_SIZE`](PlainOldData::BYTE_SIZE).
    ///
    /// # Safety
    ///
    /// Safe to call because `PlainOldData` guarantees no padding and all bit patterns valid,
    /// so writing arbitrary bytes through the returned slice cannot produce an invalid value.
    #[inline]
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: PlainOldData guarantees T has no padding and all bit patterns are valid.
        // The slice length equals the total byte size of the array.
        unsafe {
            core::slice::from_raw_parts_mut(
                core::ptr::from_mut(self) as *mut u8,
                core::mem::size_of::<Self>(),
            )
        }
    }

    /// Returns a read-only view of this value as a byte slice.
    ///
    /// The returned slice has length [`BYTE_SIZE`](PlainOldData::BYTE_SIZE).
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        // SAFETY: Same reasoning as as_bytes_mut, but for shared reference.
        unsafe {
            core::slice::from_raw_parts(
                core::ptr::from_ref(self) as *const u8,
                core::mem::size_of::<Self>(),
            )
        }
    }
}

unsafe impl<T: PlainOldData, const N: usize> PlainOldData for [T; N] {}

/// Marker trait for types that are fixed-size byte arrays.
///
/// Currently only implemented for `[u8; N]`. It is used as the associated `ByteArray` type
/// in [`IntoByteArray`] to represent the serialized form of a value.
///
/// # Safety
///
/// The implementor must be a plain byte array with `BYTE_SIZE` equal to its actual size.
pub unsafe trait ByteArray: Copy {
    /// The number of bytes in this array.
    const BYTE_SIZE: usize;
}
unsafe impl<const N: usize> ByteArray for [u8; N] {
    const BYTE_SIZE: usize = N;
}

/// Conversion from a value into its fixed-size byte representation.
///
/// Implementing this trait (together with [`TryFromByteArray`] or [`FromByteArray`]) enables
/// zero-copy, allocation-free serialization for types whose wire size is known at compile time.
///
/// [`BYTE_SIZE`](IntoByteArray::BYTE_SIZE) is a compile-time constant equal to the number of
/// bytes produced by [`into_byte_array`](IntoByteArray::into_byte_array).
///
/// # Examples
///
/// ```rust
/// use byteable::{Byteable, IntoByteArray};
///
/// #[derive(Byteable)]
/// struct Pair {
///     a: u16,
///     b: u16,
/// }
///
/// let p = Pair { a: 1, b: 2 };
/// assert_eq!(Pair::BYTE_SIZE, 4);
/// let bytes: [u8; 4] = p.into_byte_array();
/// ```
pub trait IntoByteArray: Sized {
    /// The fixed-size byte array type that this value serializes to (always `[u8; N]`).
    type ByteArray: ByteArray;

    /// Compile-time byte size of the serialized form.
    const BYTE_SIZE: usize = Self::ByteArray::BYTE_SIZE;

    /// Serialize this value into a fixed-size byte array.
    fn into_byte_array(&self) -> Self::ByteArray;
}

/// Infallible conversion from a fixed-size byte array back into a value.
///
/// This is the infallible counterpart to [`TryFromByteArray`]. Implement this when
/// decoding cannot fail (i.e. every possible byte array is a valid value).
///
/// A blanket impl automatically provides [`TryFromByteArray`] for every type that
/// implements `FromByteArray`.
pub trait FromByteArray: IntoByteArray {
    /// Deserialize a value from a fixed-size byte array. Infallible.
    fn from_byte_array(byte_array: Self::ByteArray) -> Self;
}

/// Fallible conversion from a fixed-size byte array back into a value.
///
/// Use this when decoding can fail (e.g. `bool`, `char`, `NonZero<T>`). Returns
/// [`DecodeError`] on failure.
///
/// For types that implement [`FromByteArray`], a blanket impl provides `TryFromByteArray`
/// automatically by wrapping the infallible conversion in `Ok(...)`.
///
/// # Errors
///
/// Returns [`DecodeError`] if the byte array does not represent a valid value of `Self`.
pub trait TryFromByteArray: IntoByteArray {
    /// Attempt to deserialize a value from a fixed-size byte array.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError`] if the bytes are not a valid encoding of `Self`.
    fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, DecodeError>;
}

impl<T: FromByteArray> TryFromByteArray for T {
    fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, DecodeError> {
        Ok(Self::from_byte_array(byte_array))
    }
}

macro_rules! unsafe_impl_plain_old_data {
    ($($ty:ty),+) => {
        $(
            unsafe impl PlainOldData for $ty {}
        )+
    };
}

macro_rules! impl_byte_array {
    ($($ty:ty),+) => {
        $(
            impl IntoByteArray for $ty
                where $ty : PlainOldData
            {
                type ByteArray = [u8; ::core::mem::size_of::<Self>()];
                fn into_byte_array(&self) -> Self::ByteArray {
                    #[allow(unnecessary_transmutes)]
                    unsafe { ::core::mem::transmute(*self) }
                }
            }

            impl FromByteArray for $ty
                where $ty : PlainOldData
            {
                fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                    #[allow(unnecessary_transmutes)]
                    unsafe { ::core::mem::transmute(byte_array) }
                }
            }
        )+
    };
}
pub(crate) use impl_byte_array;

unsafe_impl_plain_old_data!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);
// impl_byte_array!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

/// Error returned when decoding bytes into a typed value fails.
///
/// This error is produced by [`TryFromByteArray::try_from_byte_array`],
/// [`TryFromRawRepr::try_from_raw`], and the I/O read traits when the raw bytes do not
/// represent a valid value of the target type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// The raw discriminant value does not correspond to any variant of the enum.
    InvalidDiscriminant { raw: u64, type_name: &'static str },
    /// A `bool` field contained a byte other than `0` (false) or `1` (true).
    InvalidBool(u8),
    /// A `char` field contained a `u32` value that is not a valid Unicode scalar.
    InvalidChar(u32),
    /// A tag byte for a dynamically-tagged type (e.g. `Option`, `Result`, field enum)
    /// was not one of the expected values.
    InvalidTag { raw: u8, type_name: &'static str },
    /// A `String` field contained bytes that are not valid UTF-8.
    InvalidUtf8,
    /// A `CString` field contained an interior null byte.
    InvalidCString,
    /// A `NonZero<T>` field decoded to zero, which is not allowed.
    InvalidZero,
    /// A `NotNan<T>` field decoded to NaN, which is not allowed.
    InvalidNaN,
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::InvalidDiscriminant { raw, type_name } => {
                write!(f, "invalid discriminant {raw} for type {type_name}")
            }
            DecodeError::InvalidBool(v) => write!(f, "invalid value {v} for bool"),
            DecodeError::InvalidChar(v) => write!(f, "invalid value {v} for char"),
            DecodeError::InvalidTag { raw, type_name } => {
                write!(f, "invalid tag {raw} for {type_name}")
            }
            DecodeError::InvalidUtf8 => write!(f, "invalid UTF-8"),
            DecodeError::InvalidCString => write!(f, "invalid CString: interior null byte"),
            DecodeError::InvalidZero => write!(f, "invalid value: zero not allowed"),
            DecodeError::InvalidNaN => write!(f, "invalid value: NaN not allowed"),
        }
    }
}

impl core::error::Error for DecodeError {}

/// Conversion of a value to its raw, [`PlainOldData`] representation.
///
/// The "raw representation" is an intermediate type that:
/// 1. Is [`PlainOldData`] (no padding, all bit patterns valid), so it can be safely transmuted.
/// 2. Encodes any necessary byte-order or layout transformations (e.g. wrapping multi-byte
///    integers in [`LittleEndian<T>`] or [`BigEndian<T>`]).
///
/// For example, `u32` has `Raw = LittleEndian<u32>`, so serializing a `u32` always produces
/// little-endian bytes regardless of the host byte order.
///
/// This is the write half of the fixed-size serialization pipeline. The read half is
/// [`TryFromRawRepr`] (fallible) or [`FromRawRepr`] (infallible).
///
/// A blanket impl provides `RawRepr` for `[T; N]` when `T: RawRepr`.
pub trait RawRepr: Sized {
    /// The [`PlainOldData`] type that `Self` serializes to.
    type Raw: PlainOldData;

    /// Convert this value to its raw representation for serialization.
    fn to_raw(&self) -> Self::Raw;
}

impl<T: RawRepr, const N: usize> RawRepr for [T; N] {
    type Raw = [T::Raw; N];

    fn to_raw(&self) -> Self::Raw {
        self.each_ref().map(|e| e.to_raw())
    }
}

/// Infallible conversion from a raw [`PlainOldData`] representation back into a value.
///
/// Implement this when every possible raw bit pattern is a valid `Self`. When decoding can
/// fail (e.g. `bool`, `char`, `NonZero<T>`), implement [`TryFromRawRepr`] instead.
///
/// A blanket impl provides `FromRawRepr` for `[T; N]` when `T: FromRawRepr`.
pub trait FromRawRepr: RawRepr {
    /// Convert a raw representation into `Self`. Infallible.
    fn from_raw(raw: Self::Raw) -> Self;
}

impl<T: FromRawRepr, const N: usize> FromRawRepr for [T; N] {
    fn from_raw(raw: Self::Raw) -> Self {
        raw.map(|el| T::from_raw(el))
    }
}

/// Fallible conversion from a raw [`PlainOldData`] representation back into a value.
///
/// Use this when decoding can produce an invalid value (e.g. `bool` must be 0 or 1,
/// `char` must be a valid Unicode scalar). Returns [`DecodeError`] on failure.
///
/// A blanket impl provides `TryFromRawRepr` for `[T; N]` when `T: TryFromRawRepr`,
/// properly dropping already-initialized elements on failure.
///
/// # Errors
///
/// Returns [`DecodeError`] if the raw bytes do not encode a valid `Self`.
pub trait TryFromRawRepr: RawRepr {
    /// Attempt to convert a raw representation into `Self`.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError`] if the raw bytes do not encode a valid `Self`.
    fn try_from_raw(raw: Self::Raw) -> Result<Self, DecodeError>;
}

impl<T: TryFromRawRepr, const N: usize> TryFromRawRepr for [T; N] {
    fn try_from_raw(raw: Self::Raw) -> Result<Self, DecodeError> {
        use core::mem::MaybeUninit;
        let mut out: [MaybeUninit<T>; N] = [const { MaybeUninit::uninit() }; N];
        let mut initialized = 0usize;
        for (slot, el) in out.iter_mut().zip(raw) {
            match T::try_from_raw(el) {
                Ok(v) => {
                    slot.write(v);
                    initialized += 1;
                }
                Err(e) => {
                    for s in &mut out[..initialized] {
                        unsafe { s.assume_init_drop() };
                    }
                    return Err(e);
                }
            }
        }
        Ok(out.map(|e| unsafe { e.assume_init() }))
    }
}

/// Marker trait for multi-byte primitive types that support byte-order conversion.
///
/// Implemented for `u16`, `u32`, `u64`, `u128`, `i16`, `i32`, `i64`, `i128`, `f32`, `f64`.
///
/// This trait is used by [`BigEndian<T>`] and [`LittleEndian<T>`] to perform byte-swapping,
/// and is also required for field-level endian control in the derive macro via
/// [`HasEndianRepr`].
///
/// # Safety
///
/// The implementor must be [`PlainOldData`] and the byte-swap operations must be correct
/// (i.e. `from_le(to_le(x)) == x` and similarly for big-endian).
pub unsafe trait EndianConvert: PlainOldData {
    /// Converts a value from little-endian byte order to native byte order.
    fn from_le(value: Self) -> Self;

    /// Converts a value from big-endian byte order to native byte order.
    fn from_be(value: Self) -> Self;

    /// Converts `self` from native byte order to little-endian byte order.
    fn to_le(self) -> Self;

    /// Converts `self` from native byte order to big-endian byte order.
    fn to_be(self) -> Self;
}

macro_rules! impl_endian_convert {
    ($($type:ty),+) => {
        $(
            unsafe impl EndianConvert for $type {
                #[inline]
                fn from_le(value: Self) -> Self {
                    <$type>::from_le(value)
                }

                #[inline]
                fn from_be(value: Self) -> Self {
                    <$type>::from_be(value)
                }

                #[inline]
                fn to_le(self) -> Self {
                    <$type>::to_le(self)
                }

                #[inline]
                fn to_be(self) -> Self {
                    <$type>::to_be(self)
                }
            }
        )+
    };
}

impl_endian_convert!(u16, u32, u64, u128, i16, i32, i64, i128);

macro_rules! impl_endian_convert_float {
    ($($type:ty => $base:ty),+) => {
        $(
            unsafe impl EndianConvert for $type {
                #[inline]
                fn from_le(value: Self) -> Self {
                    Self::from_bits(<$base>::from_le(value.to_bits()))
                }

                #[inline]
                fn from_be(value: Self) -> Self {
                    Self::from_bits(<$base>::from_be(value.to_bits()))
                }

                #[inline]
                fn to_le(self) -> Self {
                    Self::from_bits(self.to_bits().to_le())
                }

                #[inline]
                fn to_be(self) -> Self {
                    Self::from_bits(self.to_bits().to_be())
                }
            }
        )+
    };
}

impl_endian_convert_float!(f32 => u32, f64 => u64);

/// A transparent wrapper that stores an [`EndianConvert`] value in **big-endian** byte order.
///
/// `BigEndian<T>` is `#[repr(transparent)]` and implements [`PlainOldData`], so it can be
/// embedded directly in `#[repr(C, packed)]` raw structs used for zero-copy serialization.
///
/// Use [`new`](BigEndian::new) to construct from a native-endian value, and
/// [`get`](BigEndian::get) to retrieve the native-endian value.
///
/// # Examples
///
/// ```rust
/// use byteable::{BigEndian, IntoByteArray};
///
/// let be = BigEndian::new(0x1234u16);
/// assert_eq!(be.get(), 0x1234u16);
/// // The internal bytes are stored in big-endian order:
/// assert_eq!(be.into_byte_array(), [0x12, 0x34]);
/// ```
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct BigEndian<T: EndianConvert>(T);

/// A transparent wrapper that stores an [`EndianConvert`] value in **little-endian** byte order.
///
/// `LittleEndian<T>` is `#[repr(transparent)]` and implements [`PlainOldData`], so it can be
/// embedded directly in `#[repr(C, packed)]` raw structs used for zero-copy serialization.
///
/// Use [`new`](LittleEndian::new) to construct from a native-endian value, and
/// [`get`](LittleEndian::get) to retrieve the native-endian value.
///
/// # Examples
///
/// ```rust
/// use byteable::{LittleEndian, IntoByteArray};
///
/// let le = LittleEndian::new(0x1234u16);
/// assert_eq!(le.get(), 0x1234u16);
/// // The internal bytes are stored in little-endian order:
/// assert_eq!(le.into_byte_array(), [0x34, 0x12]);
/// ```
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct LittleEndian<T: EndianConvert>(T);

macro_rules! impl_endian_wrapper {
    ($name:ident, $to_fn:ident, $from_fn:ident) => {
        impl<T: EndianConvert> $name<T> {
            /// Wraps `value` by converting it from native byte order to the target byte order.
            #[inline]
            pub fn new(value: T) -> Self {
                Self(value.$to_fn())
            }

            /// Retrieves the wrapped value, converting it from the stored byte order to native.
            #[inline]
            pub fn get(self) -> T {
                T::$from_fn(self.0)
            }
        }

        impl<T: core::fmt::Debug + EndianConvert> core::fmt::Debug for $name<T> {
            #[inline]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.get()).finish()
            }
        }

        impl<T: PartialEq + EndianConvert> PartialEq for $name<T> {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.get() == other.get()
            }
        }

        impl<T: Eq + EndianConvert> Eq for $name<T> {}

        impl<T: PartialOrd + EndianConvert> PartialOrd for $name<T> {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                self.get().partial_cmp(&other.get())
            }
        }

        impl<T: Ord + EndianConvert> Ord for $name<T> {
            #[inline]
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                self.get().cmp(&other.get())
            }
        }

        impl<T: core::hash::Hash + EndianConvert> core::hash::Hash for $name<T> {
            #[inline]
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                self.get().hash(state);
            }
        }

        impl<T: EndianConvert + Default> Default for $name<T> {
            #[inline]
            fn default() -> Self {
                Self::new(T::default())
            }
        }

        impl<T: EndianConvert> From<T> for $name<T> {
            fn from(value: T) -> Self {
                Self::new(value)
            }
        }

        impl<T: EndianConvert> RawRepr for $name<T> {
            type Raw = Self;

            fn to_raw(&self) -> Self::Raw {
                *self
            }
        }

        impl<T: EndianConvert> FromRawRepr for $name<T> {
            fn from_raw(raw: Self::Raw) -> Self {
                raw
            }
        }

        impl<T: EndianConvert> TryFromRawRepr for $name<T> {
            fn try_from_raw(raw: Self::Raw) -> Result<Self, DecodeError> {
                Ok(Self::from_raw(raw))
            }
        }
    };
}

impl_endian_wrapper!(BigEndian, to_be, from_be);
impl_endian_wrapper!(LittleEndian, to_le, from_le);

macro_rules! impl_from_endian_for_primitive {
    ($($ty:ty),+) => {
        $(
            impl From<BigEndian<$ty>> for $ty {
                #[inline]
                fn from(v: BigEndian<$ty>) -> $ty { v.get() }
            }
            impl From<LittleEndian<$ty>> for $ty {
                #[inline]
                fn from(v: LittleEndian<$ty>) -> $ty { v.get() }
            }
        )+
    };
}

unsafe impl<T: EndianConvert + PlainOldData> PlainOldData for BigEndian<T> {}
unsafe impl<T: EndianConvert + PlainOldData> PlainOldData for LittleEndian<T> {}

macro_rules! impl_byte_array_endian {
    ($($type:ty),+) => {
        $(
            impl_byte_array!(LittleEndian<$type>, BigEndian<$type>);
        )+
    };
}

impl_byte_array_endian!(u16, u32, u64, u128, i16, i32, i64, i128, f32, f64);
impl_from_endian_for_primitive!(u16, u32, u64, u128, i16, i32, i64, i128, f32, f64);

/// Provides typed little-endian and big-endian representations for a type.
///
/// This trait allows the derive macro to express per-field endian constraints at the type
/// level. For example, marking a `u32` field as `#[byteable(little_endian)]` causes the
/// generated code to call `u32::to_little_endian()` and store a `<u32 as HasEndianRepr>::LE`
/// (i.e. `LittleEndian<u32>`) in the raw struct.
///
/// Implemented for all [`EndianConvert`] types (primitives and floats), and by the
/// `ordered-float` feature for `OrderedFloat<T>` and `NotNan<T>`.
pub trait HasEndianRepr: Sized {
    /// The little-endian representation type (e.g. `LittleEndian<u32>` for `u32`).
    type LE: PlainOldData;
    /// The big-endian representation type (e.g. `BigEndian<u32>` for `u32`).
    type BE: PlainOldData;

    /// Convert `self` to its little-endian representation.
    fn to_little_endian(self) -> Self::LE;

    /// Convert `self` to its big-endian representation.
    fn to_big_endian(self) -> Self::BE;
}

/// Infallible conversion from a typed endian representation back to the native value.
///
/// Implemented automatically for all [`EndianConvert`] types via a blanket impl.
/// For types where decoding can fail (e.g. `NotNan<T>`), implement [`TryFromEndianRepr`]
/// directly instead.
///
/// A blanket impl provides [`TryFromEndianRepr`] for every type that implements
/// `FromEndianRepr`.
pub trait FromEndianRepr: HasEndianRepr {
    /// Convert from a little-endian representation to the native value. Infallible.
    fn from_little_endian(le: Self::LE) -> Self;

    /// Convert from a big-endian representation to the native value. Infallible.
    fn from_big_endian(be: Self::BE) -> Self;
}

/// Fallible conversion from a typed endian representation back to the native value.
///
/// Use this when the conversion can fail (e.g. `NotNan<T>` must reject NaN). Returns
/// [`DecodeError`] on failure.
///
/// For infallible types, implement [`FromEndianRepr`] instead; a blanket impl then provides
/// `TryFromEndianRepr` automatically.
///
/// # Errors
///
/// Returns [`DecodeError`] if the endian representation does not encode a valid `Self`.
pub trait TryFromEndianRepr: HasEndianRepr {
    /// Attempt to convert from a little-endian representation.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError`] if the value is invalid for `Self`.
    fn try_from_little_endian(le: Self::LE) -> Result<Self, DecodeError>;

    /// Attempt to convert from a big-endian representation.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError`] if the value is invalid for `Self`.
    fn try_from_big_endian(be: Self::BE) -> Result<Self, DecodeError>;
}

impl<T: FromEndianRepr> TryFromEndianRepr for T {
    fn try_from_little_endian(le: Self::LE) -> Result<Self, DecodeError> {
        Ok(Self::from_little_endian(le))
    }

    fn try_from_big_endian(be: Self::BE) -> Result<Self, DecodeError> {
        Ok(Self::from_big_endian(be))
    }
}

impl<T: EndianConvert> HasEndianRepr for T {
    type LE = LittleEndian<T>;
    type BE = BigEndian<T>;

    fn to_little_endian(self) -> Self::LE {
        LittleEndian::new(self)
    }

    fn to_big_endian(self) -> Self::BE {
        BigEndian::new(self)
    }
}

impl<T: EndianConvert> FromEndianRepr for T {
    fn from_little_endian(le: Self::LE) -> Self {
        le.get()
    }

    fn from_big_endian(be: Self::BE) -> Self {
        be.get()
    }
}
