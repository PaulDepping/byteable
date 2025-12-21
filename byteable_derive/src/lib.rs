//! # byteable_derive
//!
//! This crate provides the `#[derive(Byteable)]` procedural macro for the `byteable` crate.
//!
//! The derive macro automatically implements the `Byteable` trait for structs, allowing them
//! to be easily converted to and from byte arrays.
//!
//! ## Example
//!
//! ```rust
//! use byteable::Byteable;
//!
//! #[derive(Byteable, Clone, Copy, PartialEq, Debug)]
//! #[repr(C, packed)]
//! struct MyStruct {
//!     field1: LittleEndian<u16>,
//!     field2: u8,
//! }
//!
//! let instance = MyStruct { field1: 0xABCD, field2: 0xEF };
//!
//! // Convert to byte array
//! let byte_array = instance.as_bytearray();
//! assert_eq!(byte_array, [0xCD, 0xAB, 0xEF]);
//!
//! // Convert from byte array
//! let new_instance = MyStruct::from_bytearray([0xCD, 0xAB, 0xEF]);
//! assert_eq!(new_instance, instance);
//! ```
//!
//! ### Requirements for `#[derive(Byteable)]`
//!
//! - The struct should be `#[repr(C)]` or `#[repr(C, packed)]` to ensure a well-defined memory layout.
//! - The struct must implement `Copy`.
//! - All fields in the struct must themselves be `Byteable` or `Endianable` (e.g., primitive integers,
//!   `BigEndian<T>`, `LittleEndian<T>`).
//!
//! The macro uses `std::mem::transmute` for efficiency, leveraging the `#[repr(C, packed)]`
//! attribute to safely reinterpret the struct as a byte array and vice-versa.
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

/// Implements the `Byteable` trait for a struct.
///
/// This procedural macro automatically generates the necessary `Byteable`
/// implementation for a given struct. It relies on `std::mem::size_of`
/// to determine the `ByteArray` size and uses `std::mem::transmute`
/// for efficient conversion, assuming a `#[repr(C)]` or `#[repr(C, packed)]`
/// layout.
///
/// ### Panics
///
/// This macro will trigger a compilation error if the input is not a struct,
/// or if `std::mem::size_of::<Self>()` is not equal to
/// `std::mem::size_of::<Self::ByteArray>()`, which can happen if
/// the `#[repr(C)]` or `#[repr(packed)]` attributes are not correctly applied,
/// or if there are padding bytes. Ensure the struct has a predictable layout.
#[proc_macro_derive(Byteable)]
pub fn byteable_derive_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: DeriveInput = parse_macro_input!(input);

    let ident = &input.ident;

    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics Byteable for #ident #type_generics #where_clause {
            type ByteArray = [u8; ::std::mem::size_of::<Self>()];
            fn as_bytearray(self) -> Self::ByteArray {
                // Safety: This is safe because #[repr(C, packed)] ensures consistent memory layout
                // and the size of Self matches the size of Self::ByteArray.
                // The Byteable trait requires that the struct is `Copy`.
                unsafe { ::std::mem::transmute(self) }
            }

            fn from_bytearray(ba: Self::ByteArray) -> Self {
                // Safety: This is safe because #[repr(C, packed)] ensures consistent memory layout
                // and the size of Self matches the size of Self::ByteArray.
                // The Byteable trait requires that the struct is `Copy`.
                unsafe { ::std::mem::transmute(ba) }
            }
        }
    }
    .into()
}
