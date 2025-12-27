//! Procedural macro for deriving the `Byteable` trait.
//!
//! This crate provides the `#[derive(UnsafeByteableTransmute)]` procedural macro for automatically
//! implementing the `Byteable` trait on structs.

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, Meta, parse_macro_input};

/// Derives the `Byteable` trait for a struct using `transmute`.
///
/// This procedural macro automatically implements the `Byteable` trait for structs by using
/// `core::mem::transmute` to convert between the struct and a byte array. This provides
/// zero-overhead serialization but requires careful attention to memory layout and safety.
///
/// # Safety
///
/// This macro generates `unsafe` code using `core::mem::transmute`. You **must** ensure:
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
/// use byteable::{Byteable, UnsafeByteableTransmute};
///
/// # #[cfg(feature = "derive")]
/// #[derive(UnsafeByteableTransmute, Debug, PartialEq)]
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
/// let bytes = color.to_byte_array();
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
/// use byteable::{Byteable, BigEndian, LittleEndian, UnsafeByteableTransmute};
///
/// # #[cfg(feature = "derive")]
/// #[derive(UnsafeByteableTransmute, Debug)]
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
/// let bytes = packet.to_byte_array();
/// // magic is big-endian: [0x12, 0x34, 0x56, 0x78]
/// // payload_len is little-endian: [100, 0]
/// # }
/// ```
///
/// ## With nested structs
///
/// ```
/// # #[cfg(feature = "derive")]
/// use byteable::{Byteable, UnsafeByteableTransmute};
///
/// # #[cfg(feature = "derive")]
/// #[derive(UnsafeByteableTransmute, Debug, Clone, Copy)]
/// #[repr(C, packed)]
/// struct Point {
///     x: i32,
///     y: i32,
/// }
///
/// # #[cfg(feature = "derive")]
/// #[derive(UnsafeByteableTransmute, Debug)]
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
/// let bytes = line.to_byte_array();
/// assert_eq!(bytes.len(), 16); // 4 i32s × 4 bytes each
/// # }
/// ```
///
/// # Common Mistakes
///
/// ## Missing repr attribute
///
/// ```compile_fail
/// # #[cfg(feature = "derive")]
/// use byteable::UnsafeByteableTransmute;
///
/// # #[cfg(feature = "derive")]
/// #[derive(UnsafeByteableTransmute)]  // No #[repr(...)] - undefined layout!
/// struct Bad {
///     x: u32,
///     y: u16,
/// }
/// ```
///
/// ## Using invalid types
///
/// ```compile_fail
/// # #[cfg(feature = "derive")]
/// use byteable::UnsafeByteableTransmute;
///
/// # #[cfg(feature = "derive")]
/// #[derive(UnsafeByteableTransmute)]
/// #[repr(C, packed)]
/// struct Bad {
///     valid: bool,  // bool has invalid bit patterns (only 0 and 1 are valid)
/// }
/// ```
///
/// ## Using types with pointers
///
/// ```compile_fail
/// # #[cfg(feature = "derive")]
/// use byteable::UnsafeByteableTransmute;
///
/// # #[cfg(feature = "derive")]
/// #[derive(UnsafeByteableTransmute)]
/// #[repr(C)]
/// struct Bad {
///     data: Vec<u8>,  // Contains a pointer - not safe to transmute!
/// }
/// ```
///
/// # See Also
///
/// - [`Byteable`](../byteable/trait.Byteable.html) - The trait being implemented
/// - [`impl_byteable_via!`](../byteable/macro.impl_byteable_via.html) - For complex types
/// - [`unsafe_byteable_transmute!`](../byteable/macro.unsafe_byteable_transmute.html) - Manual implementation macro
#[proc_macro_derive(UnsafeByteableTransmute)]
pub fn byteable_transmute_derive_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Find the byteable crate name (handles renamed imports and when used within the crate itself)
    let found_crate = crate_name("byteable").expect("my-crate is present in `Cargo.toml`");

    // Determine the correct path to the Byteable trait and crate
    let (byteable, byteable_crate) = match found_crate {
        // If we're inside the byteable crate itself
        FoundCrate::Itself => (quote!(::byteable::Byteable), quote!(::byteable)),
        // Otherwise, use the actual crate name (handles renamed imports)
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            (quote!( #ident::Byteable ), quote!( #ident ))
        }
    };

    // Parse the input token stream into a DeriveInput AST
    let input: DeriveInput = parse_macro_input!(input);

    // Extract the struct/enum identifier
    let ident = &input.ident;

    // Split generics for the impl block (handles generic types correctly)
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    // Extract all field types to add ValidBytecastMarker bounds
    let field_types: Vec<_> = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields.named.iter().map(|f| &f.ty).collect(),
            Fields::Unnamed(fields) => fields.unnamed.iter().map(|f| &f.ty).collect(),
            Fields::Unit => Vec::new(),
        },
        _ => Vec::new(),
    };

    // Build where clause that includes ValidBytecastMarker bounds for all fields
    let extended_where_clause = if field_types.is_empty() {
        where_clause.cloned()
    } else {
        let mut clauses = where_clause
            .cloned()
            .unwrap_or_else(|| syn::parse_quote! { where });

        for field_ty in &field_types {
            clauses.predicates.push(syn::parse_quote! {
                #field_ty: #byteable_crate::ValidBytecastMarker
            });
        }
        Some(clauses)
    };

    // Generate the Byteable trait implementation with safety checks
    quote! {
        impl #impl_generics #byteable for #ident #type_generics #extended_where_clause {
            // The byte array type is a fixed-size array matching the struct size
            type ByteArray = [u8; ::core::mem::size_of::<Self>()];

            // Convert the struct to bytes using transmute (unsafe but zero-cost)
            fn to_byte_array(self) -> Self::ByteArray {
                unsafe { ::core::mem::transmute(self) }
            }

            // Convert bytes back to the struct using transmute (unsafe but zero-cost)
            fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                unsafe { ::core::mem::transmute(byte_array) }
            }
        }
    }
    .into()
}

/// Derives a delegate pattern for `Byteable` by generating a raw struct with endianness markers.
///
/// This macro creates a companion `*Raw` struct with `#[repr(C, packed)]` that handles the actual
/// byte conversion, while keeping your original struct clean and easy to work with. Fields can be
/// annotated with `#[byteable(little_endian)]` or `#[byteable(big_endian)]` to specify endianness.
///
/// # Generated Code
///
/// For each struct, this macro generates:
/// 1. A `*Raw` struct with `#[repr(C, packed)]` and endianness wrappers
/// 2. `From<OriginalStruct>` for `OriginalStructRaw` implementation
/// 3. `From<OriginalStructRaw>` for `OriginalStruct` implementation  
/// 4. A `Byteable` implementation via `impl_byteable_via!` macro
///
/// # Attributes
///
/// - `#[byteable(little_endian)]` - Wraps the field in `LittleEndian<T>`
/// - `#[byteable(big_endian)]` - Wraps the field in `BigEndian<T>`
/// - `#[byteable(transparent)]` - Stores the field as its `ByteArray` representation (for nested `Byteable` types)
/// - No attribute - Keeps the field type as-is
///
/// # Requirements
///
/// - The struct must have named fields (not a tuple struct)
/// - Fields with endianness attributes must be numeric types that implement `EndianConvert`
/// - Fields with `transparent` attribute must implement `Byteable`
/// - The struct should derive `Clone` and `Copy` for convenience
///
/// # Examples
///
/// ## Basic usage
///
/// ```
/// # #[cfg(feature = "derive")]
/// use byteable::Byteable;
///
/// # #[cfg(feature = "derive")]
/// #[derive(Clone, Copy, Byteable)]
/// struct Packet {
///     id: u8,
///     #[byteable(little_endian)]
///     length: u16,
///     #[byteable(big_endian)]
///     checksum: u32,
///     data: [u8; 4],
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn example() {
/// let packet = Packet {
///     id: 42,
///     length: 1024,
///     checksum: 0x12345678,
///     data: [1, 2, 3, 4],
/// };
///
/// // Byteable is automatically implemented
/// let bytes = packet.to_byte_array();
/// let restored = Packet::from_byte_array(bytes);
/// # }
/// ```
///
/// ## Generated code
///
/// The above example generates approximately:
///
/// ```ignore
/// #[derive(Clone, Copy, Debug, UnsafeByteableTransmute)]
/// #[repr(C, packed)]
/// struct PacketRaw {
///     id: u8,
///     length: LittleEndian<u16>,
///     checksum: BigEndian<u32>,
///     data: [u8; 4],
/// }
///
/// impl From<Packet> for PacketRaw {
///     fn from(value: Packet) -> Self {
///         Self {
///             id: value.id,
///             length: value.length.into(),
///             checksum: value.checksum.into(),
///             data: value.data,
///         }
///     }
/// }
///
/// impl From<PacketRaw> for Packet {
///     fn from(value: PacketRaw) -> Self {
///         Self {
///             id: value.id,
///             length: value.length.get(),
///             checksum: value.checksum.get(),
///             data: value.data,
///         }
///     }
/// }
///
/// impl_byteable_via!(Packet => PacketRaw);
/// ```
#[proc_macro_derive(Byteable, attributes(byteable))]
pub fn byteable_delegate_derive_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Find the byteable crate name
    let found_crate = crate_name("byteable").expect("byteable is present in `Cargo.toml`");

    // Determine the correct path to the byteable crate
    let byteable_crate = match found_crate {
        FoundCrate::Itself => quote!(::byteable),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident )
        }
    };

    // Parse the input
    let input: DeriveInput = parse_macro_input!(input);
    let original_name = &input.ident;
    let vis = &input.vis; // Capture the visibility of the original struct

    // Create the raw struct name by appending "Raw"
    let raw_name = Ident::new(
        &format!("__byteable_raw_{}", original_name),
        original_name.span(),
    );

    // Extract fields from the struct and determine if it's a tuple struct or named struct
    let (fields, is_tuple_struct) = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => (&fields.named, false),
            Fields::Unnamed(fields) => (&fields.unnamed, true),
            Fields::Unit => panic!("Byteable does not support unit structs"),
        },
        _ => panic!("Byteable only supports structs"),
    };

    // Process each field to determine its type in the raw struct and conversion logic
    let mut raw_fields = Vec::new();
    let mut raw_field_types = Vec::new(); // Track raw field types for ValidBytecastMarker
    let mut from_original_conversions = Vec::new();
    let mut from_raw_conversions = Vec::new();

    for (index, field) in fields.iter().enumerate() {
        let field_type = &field.ty;

        // Check for byteable attributes
        let mut attribute_type = None;
        for attr in &field.attrs {
            if attr.path().is_ident("byteable") {
                if let Meta::List(meta_list) = &attr.meta {
                    let tokens = &meta_list.tokens;
                    let tokens_str = tokens.to_string();
                    if tokens_str == "little_endian" {
                        attribute_type = Some("little");
                    } else if tokens_str == "big_endian" {
                        attribute_type = Some("big");
                    } else if tokens_str == "transparent" {
                        attribute_type = Some("transparent");
                    } else {
                        panic!(
                            "Unknown byteable attribute: {}. Valid attributes are: little_endian, big_endian, transparent",
                            tokens_str
                        );
                    }
                }
            }
        }

        // For tuple structs, use index-based access; for named structs, use field names
        if is_tuple_struct {
            let idx = syn::Index::from(index);

            match attribute_type {
                Some("little") => {
                    let raw_ty = quote! { #byteable_crate::LittleEndian<#field_type> };
                    raw_fields.push(raw_ty.clone());
                    raw_field_types.push(raw_ty);
                    from_original_conversions.push(quote! {
                        value.#idx.into()
                    });
                    from_raw_conversions.push(quote! {
                        value.#idx.get()
                    });
                }
                Some("big") => {
                    let raw_ty = quote! { #byteable_crate::BigEndian<#field_type> };
                    raw_fields.push(raw_ty.clone());
                    raw_field_types.push(raw_ty);
                    from_original_conversions.push(quote! {
                        value.#idx.into()
                    });
                    from_raw_conversions.push(quote! {
                        value.#idx.get()
                    });
                }
                Some("transparent") => {
                    // Use the ByteableRaw::Raw type directly for better type safety
                    let raw_ty = quote! { <#field_type as #byteable_crate::ByteableRaw>::Raw };
                    raw_fields.push(raw_ty.clone());
                    raw_field_types.push(raw_ty);
                    from_original_conversions.push(quote! {
                        value.#idx.into()
                    });
                    from_raw_conversions.push(quote! {
                        value.#idx.into()
                    });
                }
                _ => {
                    let raw_ty = quote! { #field_type };
                    raw_fields.push(raw_ty.clone());
                    raw_field_types.push(raw_ty);
                    from_original_conversions.push(quote! {
                        value.#idx
                    });
                    from_raw_conversions.push(quote! {
                        value.#idx
                    });
                }
            }
        } else {
            // Named struct
            let field_name = field.ident.as_ref().unwrap();

            match attribute_type {
                Some("little") => {
                    let raw_ty = quote! { #byteable_crate::LittleEndian<#field_type> };
                    raw_fields.push(quote! {
                        #field_name: #raw_ty
                    });
                    raw_field_types.push(raw_ty);
                    from_original_conversions.push(quote! {
                        #field_name: value.#field_name.into()
                    });
                    from_raw_conversions.push(quote! {
                        #field_name: value.#field_name.get()
                    });
                }
                Some("big") => {
                    let raw_ty = quote! { #byteable_crate::BigEndian<#field_type> };
                    raw_fields.push(quote! {
                        #field_name: #raw_ty
                    });
                    raw_field_types.push(raw_ty);
                    from_original_conversions.push(quote! {
                        #field_name: value.#field_name.into()
                    });
                    from_raw_conversions.push(quote! {
                        #field_name: value.#field_name.get()
                    });
                }
                Some("transparent") => {
                    // Use the ByteableRaw::Raw type directly for better type safety
                    let raw_ty = quote! { <#field_type as #byteable_crate::ByteableRaw>::Raw };
                    raw_fields.push(quote! {
                        #field_name: #raw_ty
                    });
                    raw_field_types.push(raw_ty);
                    from_original_conversions.push(quote! {
                        #field_name: value.#field_name.into()
                    });
                    from_raw_conversions.push(quote! {
                        #field_name: value.#field_name.into()
                    });
                }
                _ => {
                    let raw_ty = quote! { #field_type };
                    raw_fields.push(quote! {
                        #field_name: #raw_ty
                    });
                    raw_field_types.push(raw_ty);
                    from_original_conversions.push(quote! {
                        #field_name: value.#field_name
                    });
                    from_raw_conversions.push(quote! {
                        #field_name: value.#field_name
                    });
                }
            }
        }
    }

    // Generate the output code
    let output = if is_tuple_struct {
        quote! {
            // Generate the raw struct (tuple struct)
            #[derive(Clone, Copy, Debug)]
            #[repr(C, packed)]
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #vis struct #raw_name(#(#raw_fields),*);

            // Automatic ValidBytecastMarker impl for the raw struct
            // This is safe because all fields implement ValidBytecastMarker
            unsafe impl #byteable_crate::ValidBytecastMarker for #raw_name
            where
                #(#raw_field_types: #byteable_crate::ValidBytecastMarker),*
            {}

            #byteable_crate::unsafe_byteable_transmute!(#raw_name);

            // From original to raw
            impl From<#original_name> for #raw_name {
                fn from(value: #original_name) -> Self {
                    Self(#(#from_original_conversions),*)
                }
            }

            // From raw to original
            impl From<#raw_name> for #original_name {
                fn from(value: #raw_name) -> Self {
                    Self(#(#from_raw_conversions),*)
                }
            }

            // Implement Byteable for the original struct via the raw struct
            #byteable_crate::impl_byteable_via!(#original_name => #raw_name);

            // Implement ByteableRaw to expose the raw type
            impl #byteable_crate::ByteableRaw for #original_name {
                type Raw = #raw_name;
            }
        }
    } else {
        quote! {
            #[derive(Clone, Copy, Debug)]
            #[repr(C, packed)]
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #vis struct #raw_name {
                #(#raw_fields),*
            }

            // Automatic ValidBytecastMarker impl for the raw struct
            // This is safe because all fields implement ValidBytecastMarker
            unsafe impl #byteable_crate::ValidBytecastMarker for #raw_name
            where
                #(#raw_field_types: #byteable_crate::ValidBytecastMarker),*
            {}

            #byteable_crate::unsafe_byteable_transmute!(#raw_name);

            // From original to raw
            impl From<#original_name> for #raw_name {
                fn from(value: #original_name) -> Self {
                    Self {
                        #(#from_original_conversions),*
                    }
                }
            }

            impl From<#raw_name> for #original_name {
                fn from(value: #raw_name) -> Self {
                    Self {
                        #(#from_raw_conversions),*
                    }
                }
            }

            #byteable_crate::impl_byteable_via!(#original_name => #raw_name);

            // Implement ByteableRaw to expose the raw type
            impl #byteable_crate::ByteableRaw for #original_name {
                type Raw = #raw_name;
            }
        }
    };
    output.into()
}
