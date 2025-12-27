//! Safety helpers for validating types suitable for byte casting.
//!
//! This module provides the `ValidBytecastMarker` trait, which marks types that are safe
//! to transmute to/from byte arrays. This trait acts as a compile-time safety mechanism
//! to prevent UB when using the `Byteable` derive macros.

use crate::{BigEndian, LittleEndian};

/// Marker trait for types that are safe to transmute to and from byte arrays.
///
/// # Safety
///
/// This trait should only be implemented for types where:
/// 1. **All byte patterns are valid** - Any combination of bytes represents a valid value
/// 2. **No interior pointers** - The type doesn't contain references, `Box`, `Vec`, etc.
/// 3. **No invalid bit patterns** - Unlike `bool` (only 0/1 valid) or `char` (invalid Unicode)
/// 4. **No Drop semantics** - The type doesn't implement `Drop` with side effects
/// 5. **Deterministic layout** - The type has `#[repr(C)]`, `#[repr(transparent)]`, or similar
/// 6. **Explicit endianness** - Multi-byte types must specify byte order for portability
///
/// # Endianness Requirement
///
/// **Multi-byte primitive types (u16, u32, i32, f32, etc.) are NOT implemented for this trait.**
///
/// You must explicitly wrap them in `BigEndian<T>` or `LittleEndian<T>`, or use the
/// `#[byteable(little_endian)]` or `#[byteable(big_endian)]` attributes in the `Byteable` derive macro.
///
/// This design choice ensures:
/// - **Cross-platform compatibility**: Data is portable between different architectures
/// - **No silent bugs**: Won't compile if endianness is forgotten
/// - **Explicit intent**: Makes byte order visible in the code
///
/// Raw multi-byte primitives would use the platform's native byte order, causing subtle bugs
/// when data is exchanged between systems with different endianness (e.g., x86 vs ARM big-endian).
///
/// # Types that implement this trait
///
/// - **Single-byte primitives**: `u8`, `i8` (no endianness needed)
/// - **Endianness wrappers**: `BigEndian<T>` and `LittleEndian<T>` for multi-byte types
/// - **Arrays**: `[T; N]` where `T: ValidBytecastMarker`
/// - **Custom structs**: Explicitly marked with `unsafe impl ValidBytecastMarker` (use with caution!)
///
/// # Types that should NOT implement this trait
///
/// - **Raw multi-byte primitives**: `u16`, `u32`, `u64`, `u128`, `i16`, `i32`, `i64`, `i128`, `f32`, `f64`
///   (use `BigEndian<T>` or `LittleEndian<T>` instead)
/// - **Bool/char**: `bool` (only 0x00/0x01 valid), `char` (invalid Unicode ranges)
/// - **NonZero types**: `NonZero*` types (0x00 is invalid)
/// - **Pointers/references**: `&T`, `&mut T`, `*const T`, `*mut T`
/// - **Smart pointers**: `Box<T>`, `Rc<T>`, `Arc<T>`
/// - **Collections**: `Vec<T>`, `String`, `HashMap`, etc.
/// - **Complex enums**: `Option<T>`, `Result<T, E>` with niches
/// - **Function types**: Function pointers or trait objects
///
/// # Examples
///
/// ## Safe types (compile successfully):
///
/// ```
/// use byteable::{ValidBytecastMarker, LittleEndian, BigEndian};
///
/// fn ensure_valid<T: ValidBytecastMarker>() {}
///
/// // Single-byte types - no endianness needed
/// ensure_valid::<u8>();
/// ensure_valid::<i8>();
///
/// // Multi-byte types with explicit endianness - safe and portable
/// ensure_valid::<LittleEndian<u16>>();
/// ensure_valid::<BigEndian<u32>>();
/// ensure_valid::<LittleEndian<f64>>();
/// ensure_valid::<BigEndian<i64>>();
///
/// // Arrays of valid types
/// ensure_valid::<[u8; 16]>();
/// ensure_valid::<[LittleEndian<u32>; 4]>();
/// ```
///
/// ## Unsafe types (won't compile):
///
/// ```compile_fail
/// use byteable::ValidBytecastMarker;
///
/// fn ensure_valid<T: ValidBytecastMarker>() {}
///
/// // Multi-byte primitives without endianness wrapper - REJECTED
/// ensure_valid::<u16>();      // Error: no explicit endianness
/// ensure_valid::<u32>();      // Error: no explicit endianness
/// ensure_valid::<i64>();      // Error: no explicit endianness
/// ensure_valid::<f32>();      // Error: no explicit endianness
///
/// // Types with invalid bit patterns - REJECTED
/// ensure_valid::<bool>();     // Error: only 0/1 valid
/// ensure_valid::<char>();     // Error: invalid Unicode ranges
///
/// // Types with pointers - REJECTED
/// ensure_valid::<&u8>();      // Error: contains pointer
/// ensure_valid::<String>();   // Error: contains pointer + length
/// ```
///
/// ## Correct usage in structs:
///
/// ```
/// # #![cfg(feature = "derive")]
/// use byteable::Byteable;
///
/// // CORRECT: Explicit endianness
/// #[derive(Clone, Copy, Byteable)]
/// struct GoodPacket {
///     id: u8,  // Single byte - OK
///     #[byteable(little_endian)]
///     length: u16,  // Multi-byte with explicit endianness
///     #[byteable(big_endian)]
///     checksum: u32,  // Multi-byte with explicit endianness
/// }
/// # fn main() {}
/// ```
///
/// ```compile_fail
/// use byteable::Byteable;
///
/// // WRONG: Will not compile - no endianness specified
/// #[derive(Clone, Copy, Byteable)]
/// struct BadPacket {
///     id: u8,
///     length: u16,     // Error: needs endianness wrapper or attribute
///     checksum: u32,   // Error: needs endianness wrapper or attribute
/// }
/// # fn main() {}
/// ```
pub unsafe trait ValidBytecastMarker {}

// Implement for the one-byte primitive numeric types (all bit patterns valid)
unsafe impl ValidBytecastMarker for u8 {}
unsafe impl ValidBytecastMarker for i8 {}

// Implement for LittleEndian wrappers
unsafe impl ValidBytecastMarker for LittleEndian<u16> {}
unsafe impl ValidBytecastMarker for LittleEndian<u32> {}
unsafe impl ValidBytecastMarker for LittleEndian<u64> {}
unsafe impl ValidBytecastMarker for LittleEndian<u128> {}
unsafe impl ValidBytecastMarker for LittleEndian<i16> {}
unsafe impl ValidBytecastMarker for LittleEndian<i32> {}
unsafe impl ValidBytecastMarker for LittleEndian<i64> {}
unsafe impl ValidBytecastMarker for LittleEndian<i128> {}
unsafe impl ValidBytecastMarker for LittleEndian<f32> {}
unsafe impl ValidBytecastMarker for LittleEndian<f64> {}

// Implement for BigEndian wrappers
unsafe impl ValidBytecastMarker for BigEndian<u16> {}
unsafe impl ValidBytecastMarker for BigEndian<u32> {}
unsafe impl ValidBytecastMarker for BigEndian<u64> {}
unsafe impl ValidBytecastMarker for BigEndian<u128> {}
unsafe impl ValidBytecastMarker for BigEndian<i16> {}
unsafe impl ValidBytecastMarker for BigEndian<i32> {}
unsafe impl ValidBytecastMarker for BigEndian<i64> {}
unsafe impl ValidBytecastMarker for BigEndian<i128> {}
unsafe impl ValidBytecastMarker for BigEndian<f32> {}
unsafe impl ValidBytecastMarker for BigEndian<f64> {}

// Arrays of valid types are also valid
unsafe impl<T: ValidBytecastMarker, const SIZE: usize> ValidBytecastMarker for [T; SIZE] {}
