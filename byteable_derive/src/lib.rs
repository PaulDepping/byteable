//! # byteable_derive
//!
//! This crate provides the `#[derive(UnsafeByteable)]` procedural macro for the `byteable` crate.
//!
//! The derive macro automatically implements the `Byteable` trait for structs, allowing them
//! to be easily converted to and from byte arrays.
//!
//! ## Example
//!
//! ```ignore
//! use byteable::Byteable;
//!
//! #[derive(UnsafeByteable, Clone, Copy, PartialEq, Debug)]
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
//! ### Requirements for `#[derive(UnsafeByteable)]`
//!
//! - The struct should be `#[repr(C)]` or `#[repr(C, packed)]` to ensure a well-defined memory layout.
//! - The struct must implement `Copy`.
//! - All fields in the struct must themselves be `Byteable` or `Endianable` (e.g., primitive integers,
//!   `BigEndian<T>`, `LittleEndian<T>`).
//!
//! The macro uses `std::mem::transmute` for efficiency, leveraging the `#[repr(C, packed)]`
//! attribute to safely reinterpret the struct as a byte array and vice-versa.
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{DeriveInput, Ident, parse_macro_input};

/// Implements the `Byteable` trait for a struct.
///
/// This procedural macro automatically generates the necessary `Byteable`
/// implementation for a given struct. It relies on `std::mem::size_of`
/// to determine the `ByteArray` size and uses `std::mem::transmute`
/// for efficient conversion, assuming a `#[repr(C)]` or `#[repr(C, packed)]`
/// layout.
///
/// ### UB
///
/// This macro includes unsafe code which may produce UB if the struct is not valid for all possible bytearray-values due to using transmute.
///
/// ### Panics
///
/// This macro will trigger a compilation error if the input is not a struct.

#[proc_macro_derive(UnsafeByteable)]
pub fn byteable_derive_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let found_crate = crate_name("byteable").expect("my-crate is present in `Cargo.toml`");
    let byteable = match found_crate {
        FoundCrate::Itself => quote!(crate::Byteable),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident::Byteable )
        }
    };

    let input: DeriveInput = parse_macro_input!(input);

    let ident = &input.ident;

    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics #byteable for #ident #type_generics #where_clause {
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
