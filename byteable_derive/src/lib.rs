//! Procedural macro for deriving the `Byteable` trait.
//!
//! This crate provides the `#[derive(UnsafeByteable)]` procedural macro for automatically
//! implementing the `Byteable` trait on structs.

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{DeriveInput, Ident, parse_macro_input};

/// Derives the `Byteable` trait for a struct using `transmute`.
///
/// This procedural macro automatically implements the `Byteable` trait for structs by using
/// `std::mem::transmute` to convert between the struct and a byte array. This provides
/// zero-overhead serialization but requires careful attention to memory layout and safety.
///
/// # Safety
///
/// This macro generates `unsafe` code using `std::mem::transmute`. You **must** ensure:
///
/// 1. **The struct has an explicit memory layout**: Use `#[repr(C)]`, `#[repr(C, packed)]`,
///    or `#[repr(transparent)]` to ensure a well-defined layout.
///
/// 2. **All byte patterns are valid**: Every possible combination of bytes must represent
///    a valid value for your struct. This generally means:
///    - Primitive numeric types are fine (`u8`, `i32`, `f64`, etc.)
///    - Endianness wrappers are fine (`BigEndian<T>`, `LittleEndian<T>`)
///    - Arrays of the above are fine
///    - Types with invalid bit patterns are **NOT** safe (`bool`, `char`, enums with
///      discriminants, references, `NonZero*` types, etc.)
///
/// 3. **No padding with uninitialized memory**: When using `#[repr(C)]` without `packed`,
///    padding bytes might contain uninitialized memory. Use `#[repr(C, packed)]` to avoid
///    padding, or ensure all fields are properly aligned.
///
/// 4. **No complex types**: Do **not** use this with:
///    - Types containing pointers or references (`&T`, `Box<T>`, `Vec<T>`, `String`, etc.)
///    - Types with invariants (`NonZero*`, `bool`, `char`, enums with fields, etc.)
///    - Types implementing `Drop`
///
/// # Requirements
///
/// The struct must:
/// - Have a known size at compile time (no `dyn` traits or unsized fields)
/// - Not contain any generic type parameters (or they must implement `Byteable`)
///
/// # Examples
///
/// ## Basic usage
///
/// ```
/// # #[cfg(feature = "derive")]
/// use byteable::{Byteable, UnsafeByteable};
///
/// # #[cfg(feature = "derive")]
/// #[derive(UnsafeByteable, Debug, PartialEq)]
/// #[repr(C, packed)]
/// struct Color {
///     r: u8,
///     g: u8,
///     b: u8,
///     a: u8,
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn example() {
/// let color = Color { r: 255, g: 128, b: 64, a: 255 };
/// let bytes = color.as_byte_array();
/// assert_eq!(bytes, [255, 128, 64, 255]);
///
/// let restored = Color::from_byte_array(bytes);
/// assert_eq!(restored, color);
/// # }
/// ```
///
/// ## With endianness markers
///
/// ```
/// # #[cfg(feature = "derive")]
/// use byteable::{Byteable, BigEndian, LittleEndian, UnsafeByteable};
///
/// # #[cfg(feature = "derive")]
/// #[derive(UnsafeByteable, Debug)]
/// #[repr(C, packed)]
/// struct NetworkPacket {
///     magic: BigEndian<u32>,           // Network byte order
///     version: u8,
///     flags: u8,
///     payload_len: LittleEndian<u16>,  // Different endianness for payload
///     data: [u8; 16],
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn example() {
/// let packet = NetworkPacket {
///     magic: 0x12345678.into(),
///     version: 1,
///     flags: 0,
///     payload_len: 100.into(),
///     data: [0; 16],
/// };
///
/// let bytes = packet.as_byte_array();
/// // magic is big-endian: [0x12, 0x34, 0x56, 0x78]
/// // payload_len is little-endian: [100, 0]
/// # }
/// ```
///
/// ## With nested structs
///
/// ```
/// # #[cfg(feature = "derive")]
/// use byteable::{Byteable, UnsafeByteable};
///
/// # #[cfg(feature = "derive")]
/// #[derive(UnsafeByteable, Debug, Clone, Copy)]
/// #[repr(C, packed)]
/// struct Point {
///     x: i32,
///     y: i32,
/// }
///
/// # #[cfg(feature = "derive")]
/// #[derive(UnsafeByteable, Debug)]
/// #[repr(C, packed)]
/// struct Line {
///     start: Point,
///     end: Point,
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn example() {
/// let line = Line {
///     start: Point { x: 0, y: 0 },
///     end: Point { x: 10, y: 20 },
/// };
///
/// let bytes = line.as_byte_array();
/// assert_eq!(bytes.len(), 16); // 4 i32s × 4 bytes each
/// # }
/// ```
///
/// ## With generics (requires bounds)
///
/// ```
/// # #[cfg(feature = "derive")]
/// use byteable::{Byteable, UnsafeByteable};
///
/// # #[cfg(feature = "derive")]
/// #[derive(UnsafeByteable, Debug)]
/// #[repr(C, packed)]
/// struct Pair<T: Byteable> {
///     first: T,
///     second: T,
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn example() {
/// let pair = Pair {
///     first: 100u32,
///     second: 200u32,
/// };
///
/// let bytes = pair.as_byte_array();
/// assert_eq!(bytes.len(), 8);
/// # }
/// ```
///
/// # Common Mistakes
///
/// ## ❌ Missing repr attribute
///
/// ```compile_fail
/// # #[cfg(feature = "derive")]
/// use byteable::UnsafeByteable;
///
/// # #[cfg(feature = "derive")]
/// #[derive(UnsafeByteable)]  // ❌ No #[repr(...)] - undefined layout!
/// struct Bad {
///     x: u32,
///     y: u16,
/// }
/// ```
///
/// ## ❌ Using invalid types
///
/// ```compile_fail
/// # #[cfg(feature = "derive")]
/// use byteable::UnsafeByteable;
///
/// # #[cfg(feature = "derive")]
/// #[derive(UnsafeByteable)]
/// #[repr(C, packed)]
/// struct Bad {
///     valid: bool,  // ❌ bool has invalid bit patterns (only 0 and 1 are valid)
/// }
/// ```
///
/// ## ❌ Using types with pointers
///
/// ```compile_fail
/// # #[cfg(feature = "derive")]
/// use byteable::UnsafeByteable;
///
/// # #[cfg(feature = "derive")]
/// #[derive(UnsafeByteable)]
/// #[repr(C)]
/// struct Bad {
///     data: Vec<u8>,  // ❌ Contains a pointer - not safe to transmute!
/// }
/// ```
///
/// # See Also
///
/// - [`Byteable`](../byteable/trait.Byteable.html) - The trait being implemented
/// - [`impl_byteable_via!`](../byteable/macro.impl_byteable_via.html) - For complex types
/// - [`unsafe_byteable_transmute!`](../byteable/macro.unsafe_byteable_transmute.html) - Manual implementation macro
#[proc_macro_derive(UnsafeByteable)]
pub fn byteable_derive_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Find the byteable crate name (handles renamed imports and when used within the crate itself)
    let found_crate = crate_name("byteable").expect("my-crate is present in `Cargo.toml`");

    // Determine the correct path to the Byteable trait
    let byteable = match found_crate {
        // If we're inside the byteable crate itself, use crate::Byteable
        FoundCrate::Itself => quote!(crate::Byteable),
        // Otherwise, use the actual crate name (handles renamed imports)
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident::Byteable )
        }
    };

    // Parse the input token stream into a DeriveInput AST
    let input: DeriveInput = parse_macro_input!(input);

    // Extract the struct/enum identifier
    let ident = &input.ident;

    // Split generics for the impl block (handles generic types correctly)
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    // Generate the Byteable trait implementation
    quote! {
        impl #impl_generics #byteable for #ident #type_generics #where_clause {
            // The byte array type is a fixed-size array matching the struct size
            type ByteArray = [u8; ::std::mem::size_of::<Self>()];

            // Convert the struct to bytes using transmute (unsafe but zero-cost)
            fn as_byte_array(self) -> Self::ByteArray {
                unsafe { ::std::mem::transmute(self) }
            }

            // Convert bytes back to the struct using transmute (unsafe but zero-cost)
            fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                unsafe { ::std::mem::transmute(byte_array) }
            }
        }
    }
    .into()
}
