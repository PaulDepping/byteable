//! Procedural macros for deriving byte conversion traits.
//!
//! This crate provides procedural macros for automatically implementing the byte conversion traits
//! (`AssociatedByteArray`, `IntoByteArray`, `FromByteArray`) on structs:
//! - `#[derive(Byteable)]` - High-level macro with endianness support
//! - `#[derive(UnsafeByteableTransmute)]` - Low-level transmute-based implementation

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, Meta, Visibility, parse_macro_input};

/// Represents the type of byteable attribute applied to a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttributeType {
    /// Field should use little-endian byte order
    LittleEndian,
    /// Field should use big-endian byte order
    BigEndian,
    /// Field should be stored as its ByteArray representation (infallible conversion)
    Transparent,
    /// Field should be stored as its ByteArray representation (fallible conversion)
    TryTransparent,
    /// No special attribute applied
    None,
}

/// Derives byte conversion traits for a struct using `transmute`.
///
/// This procedural macro automatically implements the byte conversion traits
/// (`AssociatedByteArray`, `IntoByteArray`, `FromByteArray`) for structs by using
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
/// let bytes = color.into_byte_array();
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
/// let bytes = packet.into_byte_array();
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
/// let bytes = line.into_byte_array();
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
    let byteable_crate = match found_crate {
        // If we're inside the byteable crate itself
        FoundCrate::Itself => quote!(::byteable),
        // Otherwise, use the actual crate name (handles renamed imports)
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident )
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
        impl #impl_generics #byteable_crate::AssociatedByteArray for #ident #type_generics #extended_where_clause {
            // The byte array type is a fixed-size array matching the struct size
            type ByteArray = [u8; ::core::mem::size_of::<Self>()];
        }

        impl #impl_generics #byteable_crate::IntoByteArray for #ident #type_generics #extended_where_clause {
            // Convert the struct to bytes using transmute (unsafe but zero-cost)
            fn into_byte_array(self) -> Self::ByteArray {
                unsafe { ::core::mem::transmute(self) }
            }
        }


        impl #impl_generics #byteable_crate::FromByteArray for #ident #type_generics #extended_where_clause {
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
/// - `#[byteable(transparent)]` - Uses the field's raw representation type directly (for nested `Byteable` types implementing `HasRawType`)
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
/// let bytes = packet.into_byte_array();
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

    // Check if it's an enum and handle it
    if let Data::Enum(enum_data) = input.data {
        return handle_enum_derive(
            input.ident,
            input.generics,
            input.vis,
            input.attrs,
            &enum_data,
            byteable_crate,
        )
        .into();
    }

    let original_name = &input.ident;
    let vis = &input.vis; // Capture the visibility of the original struct

    // Create the raw struct name by appending "Raw"
    let raw_name = Ident::new(
        &format!("__byteable_raw_{}", original_name),
        original_name.span(),
    );

    // Extract fields from the struct and determine if it's a tuple struct, named struct, or unit struct
    let field_info = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => Some((&fields.named, false)),
            Fields::Unnamed(fields) => Some((&fields.unnamed, true)),
            Fields::Unit => None, // Unit structs have no fields
        },
        _ => panic!("Byteable only supports structs"),
    };

    // Split generics for the impl block (handles generic types correctly)
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    // Handle unit structs separately - they have zero size and need a direct implementation
    if field_info.is_none() {
        let output = quote! {
            // Direct Byteable implementation for unit struct (zero-sized type)
            impl #impl_generics #byteable_crate::AssociatedByteArray for #original_name #type_generics #where_clause {
                type ByteArray = [u8; 0];
            }

            impl #impl_generics #byteable_crate::IntoByteArray for #original_name #type_generics #where_clause {
                fn into_byte_array(self) -> Self::ByteArray {
                    []
                }
            }

            impl #impl_generics #byteable_crate::FromByteArray for #original_name #type_generics #where_clause {
                fn from_byte_array(_byte_array: Self::ByteArray) -> Self {
                    #original_name
                }
            }

            // Implement HasRawType for unit struct (raw type is itself)
            impl #byteable_crate::HasRawType for #original_name {
                type Raw = Self;
            }

            // Automatic ValidBytecastMarker impl for unit struct
            // Unit structs are always safe as they have no data
            unsafe impl #byteable_crate::ValidBytecastMarker for #original_name {}
        };
        return output.into();
    }

    let (fields, is_tuple_struct) = field_info.unwrap();

    // Process each field to determine its type in the raw struct and conversion logic
    let mut raw_fields = Vec::new();
    let mut raw_field_types = Vec::new(); // Track raw field types for ValidBytecastMarker
    let mut from_original_conversions = Vec::new();
    let mut from_raw_conversions = Vec::new();
    let mut has_try_transparent = false; // Track if any field uses try_transparent

    for (index, field) in fields.iter().enumerate() {
        let field_type = &field.ty;

        // Check for byteable attributes
        let mut attribute_type = AttributeType::None;
        for attr in &field.attrs {
            if attr.path().is_ident("byteable") {
                if let Meta::List(meta_list) = &attr.meta {
                    let tokens = &meta_list.tokens;
                    let tokens_str = tokens.to_string();
                    if tokens_str == "little_endian" {
                        attribute_type = AttributeType::LittleEndian;
                    } else if tokens_str == "big_endian" {
                        attribute_type = AttributeType::BigEndian;
                    } else if tokens_str == "transparent" {
                        attribute_type = AttributeType::Transparent;
                    } else if tokens_str == "try_transparent" {
                        attribute_type = AttributeType::TryTransparent;
                    } else {
                        panic!(
                            "Unknown byteable attribute: {}. Valid attributes are: little_endian, big_endian, transparent, try_transparent",
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
                AttributeType::LittleEndian => {
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
                AttributeType::BigEndian => {
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
                AttributeType::Transparent => {
                    // Use the HasRawType::Raw type directly for better type safety
                    let raw_ty = quote! { <#field_type as #byteable_crate::HasRawType>::Raw };
                    raw_fields.push(raw_ty.clone());
                    raw_field_types.push(raw_ty);
                    from_original_conversions.push(quote! {
                        value.#idx.into()
                    });
                    from_raw_conversions.push(quote! {
                        value.#idx.into()
                    });
                }
                AttributeType::TryTransparent => {
                    has_try_transparent = true;
                    // Use the TryHasRawType::Raw type for fallible conversion
                    let raw_ty = quote! { <#field_type as #byteable_crate::TryHasRawType>::Raw };
                    raw_fields.push(raw_ty.clone());
                    raw_field_types.push(raw_ty);
                    from_original_conversions.push(quote! {
                        value.#idx.into()
                    });
                    // Note: try_transparent fields require TryFrom conversion, handled separately
                    from_raw_conversions.push(quote! {
                        value.#idx.try_into()?
                    });
                }
                AttributeType::None => {
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
                AttributeType::LittleEndian => {
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
                AttributeType::BigEndian => {
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
                AttributeType::Transparent => {
                    // Use the HasRawType::Raw type directly for better type safety
                    let raw_ty = quote! { <#field_type as #byteable_crate::HasRawType>::Raw };
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
                AttributeType::TryTransparent => {
                    has_try_transparent = true;
                    // Use the TryHasRawType::Raw type for fallible conversion
                    let raw_ty = quote! { <#field_type as #byteable_crate::TryHasRawType>::Raw };
                    raw_fields.push(quote! {
                        #field_name: #raw_ty
                    });
                    raw_field_types.push(raw_ty);
                    from_original_conversions.push(quote! {
                        #field_name: value.#field_name.into()
                    });
                    // Note: try_transparent fields require TryFrom conversion, handled separately
                    from_raw_conversions.push(quote! {
                        #field_name: value.#field_name.try_into()?
                    });
                }
                AttributeType::None => {
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

    // Generate the output code based on whether we have try_transparent fields
    let output = if is_tuple_struct {
        if has_try_transparent {
            // Generate TryFrom and TryFromByteArray for tuple structs with try_transparent fields
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

                impl #impl_generics #byteable_crate::AssociatedByteArray for #raw_name #type_generics #where_clause {
                    type ByteArray = [u8; ::core::mem::size_of::<Self>()];
                }

                impl #impl_generics #byteable_crate::IntoByteArray for #raw_name #type_generics #where_clause {
                    fn into_byte_array(self) -> Self::ByteArray {
                        unsafe { ::core::mem::transmute(self) }
                    }
                }

                impl #impl_generics #byteable_crate::FromByteArray for #raw_name #type_generics #where_clause {
                    fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                        unsafe { ::core::mem::transmute(byte_array) }
                    }
                }

                // From original to raw (always infallible)
                impl From<#original_name> for #raw_name {
                    fn from(value: #original_name) -> Self {
                        Self(#(#from_original_conversions),*)
                    }
                }

                // TryFrom raw to original (fallible due to try_transparent fields)
                impl TryFrom<#raw_name> for #original_name {
                    type Error = #byteable_crate::EnumFromBytesError;

                    fn try_from(value: #raw_name) -> Result<Self, Self::Error> {
                        Ok(Self(#(#from_raw_conversions),*))
                    }
                }

                impl #impl_generics #byteable_crate::AssociatedByteArray for #original_name #type_generics #where_clause {
                    type ByteArray = <#raw_name as #byteable_crate::AssociatedByteArray>::ByteArray;
                }

                impl #impl_generics #byteable_crate::IntoByteArray for #original_name #type_generics #where_clause {
                    fn into_byte_array(self) -> Self::ByteArray {
                        let raw: #raw_name = self.into();
                        raw.into_byte_array()
                    }
                }

                // Implement TryFromByteArray instead of FromByteArray
                impl #impl_generics #byteable_crate::TryFromByteArray for #original_name #type_generics #where_clause {
                    type Error = #byteable_crate::EnumFromBytesError;

                    fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, Self::Error> {
                        let raw = <#raw_name as #byteable_crate::FromByteArray>::from_byte_array(byte_array);
                        raw.try_into()
                    }
                }

                // Implement TryHasRawType to expose the raw type with fallible conversion
                impl #byteable_crate::TryHasRawType for #original_name {
                    type Raw = #raw_name;
                }
            }
        } else {
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

                impl #impl_generics #byteable_crate::AssociatedByteArray for #raw_name #type_generics #where_clause {
                    type ByteArray = [u8; ::core::mem::size_of::<Self>()];
                }

                impl #impl_generics #byteable_crate::IntoByteArray for #raw_name #type_generics #where_clause {
                    fn into_byte_array(self) -> Self::ByteArray {
                        unsafe { ::core::mem::transmute(self) }
                    }
                }

                impl #impl_generics #byteable_crate::FromByteArray for #raw_name #type_generics #where_clause {
                    fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                        unsafe { ::core::mem::transmute(byte_array) }
                    }
                }

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

                impl #impl_generics #byteable_crate::AssociatedByteArray for #original_name #type_generics #where_clause {
                    type ByteArray = <#raw_name as #byteable_crate::AssociatedByteArray>::ByteArray;
                }

                impl #impl_generics #byteable_crate::IntoByteArray for #original_name #type_generics #where_clause {
                    fn into_byte_array(self) -> Self::ByteArray {
                        let raw: #raw_name = self.into();
                        raw.into_byte_array()
                    }
                }

                impl #impl_generics #byteable_crate::FromByteArray for #original_name #type_generics #where_clause {
                    fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                        let raw = <#raw_name>::from_byte_array(byte_array);
                        raw.into()
                    }
                }

                // Implement HasRawType to expose the raw type
                impl #byteable_crate::HasRawType for #original_name {
                    type Raw = #raw_name;
                }
            }
        }
    } else {
        if has_try_transparent {
            // Generate TryFrom and TryFromByteArray for named structs with try_transparent fields
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

                impl #impl_generics #byteable_crate::AssociatedByteArray for #raw_name #type_generics #where_clause {
                    type ByteArray = [u8; ::core::mem::size_of::<Self>()];
                }

                impl #impl_generics #byteable_crate::IntoByteArray for #raw_name #type_generics #where_clause {
                    fn into_byte_array(self) -> Self::ByteArray {
                        unsafe { ::core::mem::transmute(self) }
                    }
                }

                impl #impl_generics #byteable_crate::FromByteArray for #raw_name #type_generics #where_clause {
                    fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                        unsafe { ::core::mem::transmute(byte_array) }
                    }
                }

                // From original to raw (always infallible)
                impl From<#original_name> for #raw_name {
                    fn from(value: #original_name) -> Self {
                        Self {
                            #(#from_original_conversions),*
                        }
                    }
                }

                // TryFrom raw to original (fallible due to try_transparent fields)
                impl TryFrom<#raw_name> for #original_name {
                    type Error = #byteable_crate::EnumFromBytesError;

                    fn try_from(value: #raw_name) -> Result<Self, Self::Error> {
                        Ok(Self {
                            #(#from_raw_conversions),*
                        })
                    }
                }

                impl #impl_generics #byteable_crate::AssociatedByteArray for #original_name #type_generics #where_clause {
                    type ByteArray = <#raw_name as #byteable_crate::AssociatedByteArray>::ByteArray;
                }

                impl #impl_generics #byteable_crate::IntoByteArray for #original_name #type_generics #where_clause {
                    fn into_byte_array(self) -> Self::ByteArray {
                        let raw: #raw_name = self.into();
                        raw.into_byte_array()
                    }
                }

                // Implement TryFromByteArray instead of FromByteArray
                impl #impl_generics #byteable_crate::TryFromByteArray for #original_name #type_generics #where_clause {
                    type Error = #byteable_crate::EnumFromBytesError;

                    fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, Self::Error> {
                        let raw = <#raw_name as #byteable_crate::FromByteArray>::from_byte_array(byte_array);
                        raw.try_into()
                    }
                }

                // Implement TryHasRawType to expose the raw type with fallible conversion
                impl #byteable_crate::TryHasRawType for #original_name {
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

            impl #impl_generics #byteable_crate::AssociatedByteArray for #raw_name #type_generics #where_clause {
                type ByteArray = [u8; ::core::mem::size_of::<Self>()];
            }

            impl #impl_generics #byteable_crate::IntoByteArray for #raw_name #type_generics #where_clause {
                fn into_byte_array(self) -> Self::ByteArray {
                    unsafe { ::core::mem::transmute(self) }
                }
            }

            impl #impl_generics #byteable_crate::FromByteArray for #raw_name #type_generics #where_clause {
                fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                    unsafe { ::core::mem::transmute(byte_array) }
                }
            }

            // From original to raw
            impl From<#original_name> for #raw_name {
                fn from(value: #original_name) -> Self {
                    Self {
                        #(#from_original_conversions),*
                    }
                }
            }

            // From raw to original
            impl From<#raw_name> for #original_name {
                fn from(value: #raw_name) -> Self {
                    Self {
                        #(#from_raw_conversions),*
                    }
                }
            }

            impl #impl_generics #byteable_crate::AssociatedByteArray for #original_name #type_generics #where_clause {
                type ByteArray = <#raw_name as #byteable_crate::AssociatedByteArray>::ByteArray;
            }

            impl #impl_generics #byteable_crate::IntoByteArray for #original_name #type_generics #where_clause {
                fn into_byte_array(self) -> Self::ByteArray {
                    let raw: #raw_name = self.into();
                    raw.into_byte_array()
                }
            }

            impl #impl_generics #byteable_crate::FromByteArray for #original_name #type_generics #where_clause {
                fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                    let raw = <#raw_name>::from_byte_array(byte_array);
                    raw.into()
                }
            }

                // Implement HasRawType to expose the raw type
                impl #byteable_crate::HasRawType for #original_name {
                    type Raw = #raw_name;
                }
            }
        }
    };
    output.into()
}

/// Extracts the repr type from enum attributes (e.g., `#[repr(u8)]`).
///
/// Returns `Some(ident)` if a valid integer repr is found, `None` otherwise.
fn extract_repr_type(attrs: &[syn::Attribute]) -> Option<syn::Ident> {
    for attr in attrs {
        if attr.path().is_ident("repr") {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = &meta_list.tokens;
                // Parse simple repr types like u8, u16, etc.
                if let Ok(ident) = syn::parse2::<syn::Ident>(tokens.clone()) {
                    let ident_str: String = ident.to_string();
                    if matches!(
                        ident_str.as_str(),
                        "u8" | "i8"
                            | "u16"
                            | "i16"
                            | "u32"
                            | "i32"
                            | "u64"
                            | "i64"
                            | "u128"
                            | "i128"
                    ) {
                        return Some(ident);
                    }
                }
            }
        }
    }
    None
}

/// Handles deriving Byteable for C-like enums.
///
/// This function generates implementations of `AssociatedByteArray`, `IntoByteArray`,
/// and `TryFromByteArray` for enums with explicit repr types and discriminants.
/// It also generates a raw type with the matching endianness wrapper.
fn handle_enum_derive(
    enum_name: Ident,
    generics: syn::Generics,
    vis: Visibility,
    attrs: Vec<syn::Attribute>,
    enum_data: &syn::DataEnum,
    byteable_crate: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    // Step 1: Validate that all variants are unit variants (no fields)
    for variant in &enum_data.variants {
        match &variant.fields {
            Fields::Unit => {} // Good
            _ => panic!(
                "Byteable can only be derived for C-like enums (all variants must be unit variants). \
                 Variant '{}' has fields.",
                variant.ident
            ),
        }
    }

    // Step 2: Extract the repr type from attributes
    let repr_ty = extract_repr_type(&attrs)
        .expect("Enum must have a #[repr(u8)], #[repr(u16)], #[repr(u32)], or similar attribute");

    // Step 2.5: Check for byteable endianness attributes
    let mut endianness = AttributeType::None;
    for attr in &attrs {
        if attr.path().is_ident("byteable") {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = &meta_list.tokens;
                let tokens_str = tokens.to_string();
                if tokens_str == "little_endian" {
                    endianness = AttributeType::LittleEndian;
                } else if tokens_str == "big_endian" {
                    endianness = AttributeType::BigEndian;
                } else {
                    panic!(
                        "Unknown byteable attribute for enum: {}. Valid attributes are: little_endian, big_endian",
                        tokens_str
                    );
                }
            }
        }
    }

    let from_discriminant_arms = enum_data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        if variant.discriminant.is_none() {
            panic!(
                "All enum variants must have explicit discriminant values for Byteable. \
                 Variant '{}' is missing a discriminant.",
                variant_name
            );
        }
        let (_, expr) = variant.discriminant.as_ref().unwrap();
        quote! {
            #expr => Ok(#enum_name::#variant_name),
        }
    });

    // Step 5: Determine the byte conversion methods and raw type wrapper based on endianness
    let (raw_type_wrapper, raw_type_get) = match endianness {
        AttributeType::LittleEndian => (
            quote! { #byteable_crate::LittleEndian<#repr_ty> },
            quote! {value.0.get()},
        ),
        AttributeType::BigEndian => (
            quote! { #byteable_crate::BigEndian<#repr_ty> },
            quote! {value.0.get()},
        ),
        AttributeType::None => (quote! { #repr_ty }, quote! {value.0}), // No wrapper for native endianness
        AttributeType::Transparent | AttributeType::TryTransparent => {
            panic!("transparent and try_transparent attributes are not supported for enums");
        }
    };

    // Step 6: Create the raw type name
    let raw_name = Ident::new(&format!("__byteable_raw_{}", enum_name), enum_name.span());

    // Step 7: Generate the raw type and implementations
    quote! {
        // Generate the raw type with endianness wrapper
        #[derive(Clone, Copy, Debug)]
        #[repr(transparent)]
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        #vis struct #raw_name(#raw_type_wrapper);

        // Automatic ValidBytecastMarker impl for the raw type
        unsafe impl #byteable_crate::ValidBytecastMarker for #raw_name
        where #raw_type_wrapper: #byteable_crate::ValidBytecastMarker,
        {}

        impl #impl_generics #byteable_crate::AssociatedByteArray for #raw_name #type_generics #where_clause {
            type ByteArray = [u8; ::core::mem::size_of::<Self>()];
        }

        impl #impl_generics #byteable_crate::IntoByteArray for #raw_name #type_generics #where_clause {
            fn into_byte_array(self) -> Self::ByteArray {
                unsafe { ::core::mem::transmute(self) }
            }
        }

        impl #impl_generics #byteable_crate::FromByteArray for #raw_name #type_generics #where_clause {
            fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                unsafe { ::core::mem::transmute(byte_array) }
            }
        }

        // From original enum to raw
        impl From<#enum_name> for #raw_name {
            fn from(value: #enum_name) -> Self {
                // Convert enum to its discriminant value
                let discriminant: #repr_ty = value as _;
                // Wrap in the appropriate endianness type
                Self(discriminant.into())
            }
        }

        // TryFrom raw to enum (fallible because not all byte patterns are valid)
        impl TryFrom<#raw_name> for #enum_name {
            type Error = #byteable_crate::EnumFromBytesError;

            fn try_from(value: #raw_name) -> Result<Self, Self::Error> {
                let value = #raw_type_get;
                match value {
                    #(#from_discriminant_arms)*
                    invalid => Err(#byteable_crate::EnumFromBytesError::new(invalid, ::core::any::type_name::<Self>())),
                }
            }
        }

        // Implement TryHasRawType to expose the raw type with fallible conversion
        // Note: We don't implement HasRawType because enums don't have infallible From<Raw>
        impl #byteable_crate::TryHasRawType for #enum_name {
            type Raw = #raw_name;
        }

        impl #impl_generics #byteable_crate::AssociatedByteArray for #enum_name #type_generics #where_clause {
            type ByteArray = <#raw_name as #byteable_crate::AssociatedByteArray>::ByteArray;
        }

        impl #impl_generics #byteable_crate::IntoByteArray for #enum_name #type_generics #where_clause {
            fn into_byte_array(self) -> Self::ByteArray {
                let raw: #raw_name = self.into();
                <#raw_name as #byteable_crate::IntoByteArray>::into_byte_array(raw)
            }
        }

        // Implement TryFromByteArray instead of FromByteArray
        // because not all byte patterns may be valid enum variants
        impl #impl_generics #byteable_crate::TryFromByteArray for #enum_name #type_generics #where_clause {
            type Error = <Self as TryFrom<#raw_name>>::Error;

            fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, Self::Error> {
                let raw = <#raw_name as #byteable_crate::FromByteArray>::from_byte_array(byte_array);
                raw.try_into()
            }
        }
    }
}
