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

/// Represents the type of byteable attribute applied to a field or type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttributeType {
    LittleEndian,
    BigEndian,
    /// Use field's raw representation via `RawRepr` (infallible conversion)
    Transparent,
    /// Use field's raw representation via `TryRawRepr` (fallible conversion)
    TryTransparent,
    /// Struct-level: skip transmute path, generate `Readable`/`Writable` via sequential field I/O
    IoOnly,
    None,
}

/// Resolves the path to the `byteable` crate (handles renamed imports and in-crate use).
fn byteable_crate_path() -> proc_macro2::TokenStream {
    match crate_name("byteable").expect("byteable is present in `Cargo.toml`") {
        FoundCrate::Itself => quote!(::byteable),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(#ident)
        }
    }
}

/// Parses the `#[byteable(...)]` attribute from a list of attributes.
fn parse_byteable_attr(attrs: &[syn::Attribute]) -> AttributeType {
    for attr in attrs {
        if attr.path().is_ident("byteable") {
            if let Meta::List(meta_list) = &attr.meta {
                return match meta_list.tokens.to_string().as_str() {
                    "little_endian" => AttributeType::LittleEndian,
                    "big_endian" => AttributeType::BigEndian,
                    "transparent" => AttributeType::Transparent,
                    "try_transparent" => AttributeType::TryTransparent,
                    "io_only" => AttributeType::IoOnly,
                    other => panic!(
                        "Unknown byteable attribute: {other}. \
                         Valid attributes are: little_endian, big_endian, transparent, try_transparent, io_only"
                    ),
                };
            }
        }
    }
    AttributeType::None
}

/// Generates `BytecastSafe`, `AssociatedByteArray`, `IntoByteArray`, and `FromByteArray`
/// impls for a raw struct, using `transmute` for zero-cost conversion.
fn gen_raw_struct_impls(
    raw_name: &Ident,
    raw_field_types: &[proc_macro2::TokenStream],
    bc: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        unsafe impl #bc::BytecastSafe for #raw_name
        where #(#raw_field_types: #bc::BytecastSafe),*
        {}

        impl #bc::AssociatedByteArray for #raw_name {
            type ByteArray = [u8; ::core::mem::size_of::<Self>()];
        }

        impl #bc::IntoByteArray for #raw_name {
            #[inline]
            fn into_byte_array(self) -> Self::ByteArray {
                unsafe { ::core::mem::transmute(self) }
            }
        }

        impl #bc::FromByteArray for #raw_name {
            #[inline]
            fn from_byte_array(bytes: Self::ByteArray) -> Self {
                unsafe { ::core::mem::transmute(bytes) }
            }
        }
    }
}

/// Generates the token stream for writing a single struct field accessed via `self.field` /
/// `self.0`. Unlike [`gen_field_write`], which dereferences a move-bound variable, this helper
/// takes a pre-formed access expression (`self.field_name` or `self.0`) suitable for use inside
/// `&self` methods.
fn gen_struct_field_write(
    field_access: &proc_macro2::TokenStream,
    field_ty: &syn::Type,
    attrs: &[syn::Attribute],
    bc: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match parse_byteable_attr(attrs) {
        AttributeType::LittleEndian => quote! {
            writer.write_byteable(&#bc::LittleEndian::new(#field_access))?;
        },
        AttributeType::BigEndian => quote! {
            writer.write_byteable(&#bc::BigEndian::new(#field_access))?;
        },
        AttributeType::None => {
            if is_multibyte_primitive(field_ty) {
                panic!(
                    "io_only struct field of type `{}` is a multi-byte primitive and requires \
                     an endianness annotation: add #[byteable(little_endian)] or \
                     #[byteable(big_endian)]",
                    quote!(#field_ty)
                );
            }
            quote! { writer.write_byteable(&#field_access)?; }
        }
        AttributeType::IoOnly => panic!(
            "#[byteable(io_only)] is a struct-level attribute and cannot be used on a field"
        ),
        AttributeType::Transparent | AttributeType::TryTransparent => panic!(
            "#[byteable(transparent)] and #[byteable(try_transparent)] are not applicable in \
             io_only mode; remove the annotation or use a plain field"
        ),
    }
}

/// Generates `Readable` and `Writable` impls for a struct annotated with `#[byteable(io_only)]`.
///
/// Instead of the transmute-based `IntoByteArray`/`FromByteArray` path, this reads and writes
/// each field in declaration order using their own `Readable`/`Writable` implementations. This
/// allows structs to contain types like `Vec<T>`, `String`, and `Option<T>` that are not
/// `BytecastSafe`.
fn handle_io_struct_derive(
    name: &syn::Ident,
    generics: &syn::Generics,
    fields_data: &syn::Fields,
    bc: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    if let syn::Fields::Unit = fields_data {
        return quote! {
            impl #impl_generics #bc::Readable for #name #type_generics #where_clause {
                fn read_from(_reader: &mut (impl ::std::io::Read + ?Sized)) -> ::std::io::Result<Self> {
                    Ok(#name)
                }
            }
            impl #impl_generics #bc::Writable for #name #type_generics #where_clause {
                fn write_to(&self, _writer: &mut (impl ::std::io::Write + ?Sized)) -> ::std::io::Result<()> {
                    Ok(())
                }
            }
        };
    }

    let (fields, is_tuple) = match fields_data {
        syn::Fields::Named(f) => (&f.named, false),
        syn::Fields::Unnamed(f) => (&f.unnamed, true),
        syn::Fields::Unit => unreachable!(),
    };

    let write_stmts: Vec<_> = fields.iter().enumerate().map(|(i, field)| {
        let field_access = if is_tuple {
            let idx = syn::Index::from(i);
            quote! { self.#idx }
        } else {
            let fname = field.ident.as_ref().unwrap();
            quote! { self.#fname }
        };
        gen_struct_field_write(&field_access, &field.ty, &field.attrs, bc)
    }).collect();

    let (read_bindings, construct_expr): (Vec<_>, proc_macro2::TokenStream) = if is_tuple {
        let idents: Vec<_> = (0..fields.len())
            .map(|i| syn::Ident::new(&format!("__field_{i}"), name.span()))
            .collect();
        let bindings = fields.iter().zip(&idents)
            .map(|(f, id)| gen_field_read(id, &f.ty, &f.attrs, bc))
            .collect();
        (bindings, quote! { Ok(Self(#(#idents),*)) })
    } else {
        let field_idents: Vec<_> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
        let bindings = fields.iter()
            .map(|f| gen_field_read(f.ident.as_ref().unwrap(), &f.ty, &f.attrs, bc))
            .collect();
        (bindings, quote! { Ok(Self { #(#field_idents),* }) })
    };

    quote! {
        impl #impl_generics #bc::Readable for #name #type_generics #where_clause {
            fn read_from(mut reader: &mut (impl ::std::io::Read + ?Sized)) -> ::std::io::Result<Self> {
                use #bc::ReadByteable;
                #( #read_bindings )*
                #construct_expr
            }
        }
        impl #impl_generics #bc::Writable for #name #type_generics #where_clause {
            fn write_to(&self, mut writer: &mut (impl ::std::io::Write + ?Sized)) -> ::std::io::Result<()> {
                use #bc::WriteByteable;
                #( #write_stmts )*
                Ok(())
            }
        }
    }
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
    let bc = byteable_crate_path();
    let input: DeriveInput = parse_macro_input!(input);
    let ident = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let field_types: Vec<_> = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields.named.iter().map(|f| &f.ty).collect(),
            Fields::Unnamed(fields) => fields.unnamed.iter().map(|f| &f.ty).collect(),
            Fields::Unit => Vec::new(),
        },
        _ => Vec::new(),
    };

    let extended_where = if field_types.is_empty() {
        where_clause.cloned()
    } else {
        let mut clauses = where_clause
            .cloned()
            .unwrap_or_else(|| syn::parse_quote! { where });
        for ty in &field_types {
            clauses.predicates.push(syn::parse_quote! { #ty: #bc::BytecastSafe });
        }
        Some(clauses)
    };

    quote! {
        impl #impl_generics #bc::AssociatedByteArray for #ident #type_generics #extended_where {
            type ByteArray = [u8; ::core::mem::size_of::<Self>()];
        }

        impl #impl_generics #bc::IntoByteArray for #ident #type_generics #extended_where {
            #[inline]
            fn into_byte_array(self) -> Self::ByteArray {
                unsafe { ::core::mem::transmute(self) }
            }
        }

        impl #impl_generics #bc::FromByteArray for #ident #type_generics #extended_where {
            #[inline]
            fn from_byte_array(bytes: Self::ByteArray) -> Self {
                unsafe { ::core::mem::transmute(bytes) }
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
/// - `#[byteable(transparent)]` - Uses the field's raw representation type directly (for nested `Byteable` types implementing `RawRepr`)
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
    let bc = byteable_crate_path();
    let input: DeriveInput = parse_macro_input!(input);

    if let Data::Enum(enum_data) = input.data {
        let has_field_variants = enum_data.variants.iter().any(|v| !matches!(v.fields, Fields::Unit));
        if has_field_variants {
            return handle_field_enum_derive(input.ident, input.generics, input.attrs, &enum_data, bc).into();
        }
        return handle_enum_derive(input.ident, input.generics, input.vis, input.attrs, &enum_data, bc).into();
    }

    // io_only: generate Readable + Writable by sequential field I/O instead of transmute
    if parse_byteable_attr(&input.attrs) == AttributeType::IoOnly {
        let fields_data = match &input.data {
            Data::Struct(data) => &data.fields,
            _ => panic!("#[byteable(io_only)] is only supported on structs"),
        };
        return handle_io_struct_derive(&input.ident, &input.generics, fields_data, &bc).into();
    }


    let original_name = &input.ident;
    let vis = &input.vis;
    let raw_name = Ident::new(&format!("__byteable_raw_{}", original_name), original_name.span());
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let fields_data = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("Byteable only supports structs"),
    };

    // Unit structs: zero-sized, implement directly without a raw struct
    if let Fields::Unit = fields_data {
        return quote! {
            impl #impl_generics #bc::AssociatedByteArray for #original_name #type_generics #where_clause {
                type ByteArray = [u8; 0];
            }
            impl #impl_generics #bc::IntoByteArray for #original_name #type_generics #where_clause {
                #[inline]
                fn into_byte_array(self) -> Self::ByteArray { [] }
            }
            impl #impl_generics #bc::FromByteArray for #original_name #type_generics #where_clause {
                #[inline]
                fn from_byte_array(_: Self::ByteArray) -> Self { #original_name }
            }
            impl #bc::RawRepr for #original_name { type Raw = Self; }
            unsafe impl #bc::BytecastSafe for #original_name {}
        }
        .into();
    }

    let (fields, is_tuple) = match fields_data {
        Fields::Named(f) => (&f.named, false),
        Fields::Unnamed(f) => (&f.unnamed, true),
        Fields::Unit => unreachable!(),
    };

    // Process each field: determine raw type and to/from conversion expressions
    let mut raw_field_defs = Vec::new();
    let mut raw_field_types = Vec::new();
    let mut to_raw_exprs = Vec::new();
    let mut from_raw_exprs = Vec::new();
    let mut has_try = false;

    for (i, field) in fields.iter().enumerate() {
        let field_type = &field.ty;
        let attr = parse_byteable_attr(&field.attrs);
        if attr == AttributeType::TryTransparent {
            has_try = true;
        }

        let (raw_ty, to_raw_suffix, from_raw_suffix) = match attr {
            AttributeType::LittleEndian => (
                quote! { #bc::LittleEndian<#field_type> },
                quote! { .into() },
                quote! { .get() },
            ),
            AttributeType::BigEndian => (
                quote! { #bc::BigEndian<#field_type> },
                quote! { .into() },
                quote! { .get() },
            ),
            AttributeType::Transparent => (
                quote! { <#field_type as #bc::RawRepr>::Raw },
                quote! { .into() },
                quote! { .into() },
            ),
            AttributeType::TryTransparent => (
                quote! { <#field_type as #bc::TryRawRepr>::Raw },
                quote! { .into() },
                quote! { .try_into()? },
            ),
            AttributeType::None => (quote! { #field_type }, quote! {}, quote! {}),
            AttributeType::IoOnly => panic!(
                "#[byteable(io_only)] is a struct-level attribute and cannot be used on individual fields"
            ),
        };

        raw_field_types.push(raw_ty.clone());

        if is_tuple {
            let idx = syn::Index::from(i);
            raw_field_defs.push(quote! { #raw_ty });
            to_raw_exprs.push(quote! { value.#idx #to_raw_suffix });
            from_raw_exprs.push(quote! { value.#idx #from_raw_suffix });
        } else {
            let name = field.ident.as_ref().unwrap();
            raw_field_defs.push(quote! { #name: #raw_ty });
            to_raw_exprs.push(quote! { #name: value.#name #to_raw_suffix });
            from_raw_exprs.push(quote! { #name: value.#name #from_raw_suffix });
        }
    }

    let raw_struct_def = if is_tuple {
        quote! {
            #[derive(Clone, Copy, Debug)]
            #[repr(C, packed)]
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #vis struct #raw_name(#(#raw_field_defs),*);
        }
    } else {
        quote! {
            #[derive(Clone, Copy, Debug)]
            #[repr(C, packed)]
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #vis struct #raw_name { #(#raw_field_defs),* }
        }
    };

    let raw_impls = gen_raw_struct_impls(&raw_name, &raw_field_types, &bc);

    let from_original = if is_tuple {
        quote! {
            impl From<#original_name> for #raw_name {
                #[inline]
                fn from(value: #original_name) -> Self { Self(#(#to_raw_exprs),*) }
            }
        }
    } else {
        quote! {
            impl From<#original_name> for #raw_name {
                #[inline]
                fn from(value: #original_name) -> Self { Self { #(#to_raw_exprs),* } }
            }
        }
    };

    let from_raw_body = if is_tuple {
        quote! { Self(#(#from_raw_exprs),*) }
    } else {
        quote! { Self { #(#from_raw_exprs),* } }
    };

    // If any field uses try_transparent, the raw→original conversion is fallible
    let original_impls = if has_try {
        quote! {
            impl TryFrom<#raw_name> for #original_name {
                type Error = #bc::InvalidDiscriminantError;
                #[inline]
                fn try_from(value: #raw_name) -> Result<Self, Self::Error> {
                    Ok(#from_raw_body)
                }
            }

            impl #impl_generics #bc::AssociatedByteArray for #original_name #type_generics #where_clause {
                type ByteArray = <#raw_name as #bc::AssociatedByteArray>::ByteArray;
            }

            impl #impl_generics #bc::IntoByteArray for #original_name #type_generics #where_clause {
                #[inline]
                fn into_byte_array(self) -> Self::ByteArray {
                    let raw: #raw_name = self.into();
                    raw.into_byte_array()
                }
            }

            impl #impl_generics #bc::TryFromByteArray for #original_name #type_generics #where_clause {
                type Error = #bc::InvalidDiscriminantError;
                #[inline]
                fn try_from_byte_array(bytes: Self::ByteArray) -> Result<Self, Self::Error> {
                    let raw = <#raw_name as #bc::FromByteArray>::from_byte_array(bytes);
                    raw.try_into()
                }
            }

            impl #bc::TryRawRepr for #original_name { type Raw = #raw_name; }
        }
    } else {
        quote! {
            impl From<#raw_name> for #original_name {
                #[inline]
                fn from(value: #raw_name) -> Self { #from_raw_body }
            }

            impl #impl_generics #bc::AssociatedByteArray for #original_name #type_generics #where_clause {
                type ByteArray = <#raw_name as #bc::AssociatedByteArray>::ByteArray;
            }

            impl #impl_generics #bc::IntoByteArray for #original_name #type_generics #where_clause {
                #[inline]
                fn into_byte_array(self) -> Self::ByteArray {
                    let raw: #raw_name = self.into();
                    raw.into_byte_array()
                }
            }

            impl #impl_generics #bc::FromByteArray for #original_name #type_generics #where_clause {
                #[inline]
                fn from_byte_array(bytes: Self::ByteArray) -> Self {
                    <#raw_name as #bc::FromByteArray>::from_byte_array(bytes).into()
                }
            }

            impl #bc::RawRepr for #original_name { type Raw = #raw_name; }
        }
    };

    quote! {
        #raw_struct_def
        #raw_impls
        #from_original
        #original_impls
    }
    .into()
}

/// Extracts the integer repr type from enum attributes (e.g., `#[repr(u8)]` → `u8`).
fn extract_repr_type(attrs: &[syn::Attribute]) -> Option<syn::Ident> {
    for attr in attrs {
        if attr.path().is_ident("repr") {
            if let Meta::List(meta_list) = &attr.meta {
                if let Ok(ident) = syn::parse2::<syn::Ident>(meta_list.tokens.clone()) {
                    if matches!(
                        ident.to_string().as_str(),
                        "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64" | "u128" | "i128"
                    ) {
                        return Some(ident);
                    }
                }
            }
        }
    }
    None
}

/// Handles deriving `Byteable` for C-like enums with explicit discriminants.
///
/// Generates `AssociatedByteArray`, `IntoByteArray`, and `TryFromByteArray` impls,
/// along with a raw wrapper type for the matching endianness.
fn handle_enum_derive(
    enum_name: Ident,
    generics: syn::Generics,
    vis: Visibility,
    attrs: Vec<syn::Attribute>,
    enum_data: &syn::DataEnum,
    bc: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    // Validate: all variants must be unit variants with explicit discriminants
    for variant in &enum_data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            panic!(
                "Byteable can only be derived for C-like enums (all variants must be unit variants). \
                 Variant '{}' has fields.",
                variant.ident
            );
        }
    }

    let repr_ty = extract_repr_type(&attrs)
        .expect("Enum must have a #[repr(u8)], #[repr(u16)], #[repr(u32)], or similar attribute");

    let endianness = parse_byteable_attr(&attrs);
    if matches!(endianness, AttributeType::Transparent | AttributeType::TryTransparent) {
        panic!("transparent and try_transparent attributes are not supported for enums");
    }

    let from_discriminant_arms = enum_data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let (_, expr) = variant.discriminant.as_ref().unwrap_or_else(|| {
            panic!(
                "All enum variants must have explicit discriminant values for Byteable. \
                 Variant '{}' is missing a discriminant.",
                variant_name
            )
        });
        quote! { #expr => Ok(#enum_name::#variant_name), }
    });

    let (raw_type_wrapper, raw_type_get) = match endianness {
        AttributeType::LittleEndian => (
            quote! { #bc::LittleEndian<#repr_ty> },
            quote! { value.0.get() },
        ),
        AttributeType::BigEndian => (
            quote! { #bc::BigEndian<#repr_ty> },
            quote! { value.0.get() },
        ),
        _ => (quote! { #repr_ty }, quote! { value.0 }),
    };

    let raw_name = Ident::new(&format!("__byteable_raw_{}", enum_name), enum_name.span());
    let raw_impls = gen_raw_struct_impls(&raw_name, &[raw_type_wrapper.clone()], &bc);

    quote! {
        #[derive(Clone, Copy, Debug)]
        #[repr(transparent)]
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        #vis struct #raw_name(#raw_type_wrapper);

        #raw_impls

        impl From<#enum_name> for #raw_name {
            #[inline]
            fn from(value: #enum_name) -> Self {
                let discriminant: #repr_ty = value as _;
                Self(discriminant.into())
            }
        }

        impl TryFrom<#raw_name> for #enum_name {
            type Error = #bc::InvalidDiscriminantError;
            #[inline]
            fn try_from(value: #raw_name) -> Result<Self, Self::Error> {
                let value = #raw_type_get;
                match value {
                    #(#from_discriminant_arms)*
                    invalid => Err(#bc::InvalidDiscriminantError::new(invalid, ::core::any::type_name::<Self>())),
                }
            }
        }

        impl #bc::TryRawRepr for #enum_name { type Raw = #raw_name; }

        impl #impl_generics #bc::AssociatedByteArray for #enum_name #type_generics #where_clause {
            type ByteArray = <#raw_name as #bc::AssociatedByteArray>::ByteArray;
        }

        impl #impl_generics #bc::IntoByteArray for #enum_name #type_generics #where_clause {
            #[inline]
            fn into_byte_array(self) -> Self::ByteArray {
                let raw: #raw_name = self.into();
                <#raw_name as #bc::IntoByteArray>::into_byte_array(raw)
            }
        }

        impl #impl_generics #bc::TryFromByteArray for #enum_name #type_generics #where_clause {
            type Error = <Self as TryFrom<#raw_name>>::Error;
            #[inline]
            fn try_from_byte_array(bytes: Self::ByteArray) -> Result<Self, Self::Error> {
                let raw = <#raw_name as #bc::FromByteArray>::from_byte_array(bytes);
                raw.try_into()
            }
        }
    }
}

/// Returns `true` if `ty` is a bare multi-byte primitive (`u16`, `u32`, `f32`, etc.) that cannot
/// be serialized without an explicit endianness annotation.
fn is_multibyte_primitive(ty: &syn::Type) -> bool {
    if let syn::Type::Path(tp) = ty {
        if tp.qself.is_none() {
            if let Some(ident) = tp.path.get_ident() {
                return matches!(
                    ident.to_string().as_str(),
                    "u16" | "i16" | "u32" | "i32" | "u64" | "i64"
                        | "u128" | "i128" | "f32" | "f64" | "usize" | "isize"
                );
            }
        }
    }
    false
}

/// Generates the token stream for writing a single variant field, respecting its `#[byteable]`
/// attribute. Panics at compile time if the field is a multi-byte primitive with no annotation.
fn gen_field_write(
    field_ident: &Ident,
    field_ty: &syn::Type,
    attrs: &[syn::Attribute],
    bc: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match parse_byteable_attr(attrs) {
        AttributeType::LittleEndian => quote! {
            writer.write_byteable(&#bc::LittleEndian::new(*#field_ident))?;
        },
        AttributeType::BigEndian => quote! {
            writer.write_byteable(&#bc::BigEndian::new(*#field_ident))?;
        },
        AttributeType::None => {
            if is_multibyte_primitive(field_ty) {
                panic!(
                    "field `{}` is a multi-byte type and requires an endianness annotation: \
                     add #[byteable(little_endian)] or #[byteable(big_endian)]",
                    field_ident
                );
            }
            quote! { writer.write_byteable(#field_ident)?; }
        }
        other => panic!(
            "unsupported #[byteable] attribute `{other:?}` on field `{field_ident}`; \
             only little_endian and big_endian are supported here"
        ),
    }
}

/// Generates the token stream for reading a single variant field, respecting its `#[byteable]`
/// attribute. Panics at compile time if the field is a multi-byte primitive with no annotation.
fn gen_field_read(
    field_ident: &Ident,
    field_ty: &syn::Type,
    attrs: &[syn::Attribute],
    bc: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match parse_byteable_attr(attrs) {
        AttributeType::LittleEndian => quote! {
            let #field_ident: #field_ty =
                reader.read_byteable::<#bc::LittleEndian<#field_ty>>()?.get();
        },
        AttributeType::BigEndian => quote! {
            let #field_ident: #field_ty =
                reader.read_byteable::<#bc::BigEndian<#field_ty>>()?.get();
        },
        AttributeType::None => {
            if is_multibyte_primitive(field_ty) {
                panic!(
                    "field `{}` is a multi-byte type and requires an endianness annotation: \
                     add #[byteable(little_endian)] or #[byteable(big_endian)]",
                    field_ident
                );
            }
            quote! { let #field_ident: #field_ty = reader.read_byteable()?; }
        }
        other => panic!(
            "unsupported #[byteable] attribute `{other:?}` on field `{field_ident}`; \
             only little_endian and big_endian are supported here"
        ),
    }
}

/// Attempts to evaluate an integer literal expression to a `u128` value.
///
/// Handles decimal, hex (`0x`), octal (`0o`), and binary (`0b`) literals, plus type suffixes.
/// Returns `None` for non-literal or non-integer expressions.
fn try_eval_int_expr(expr: &syn::Expr) -> Option<u128> {
    match expr {
        syn::Expr::Lit(el) => {
            if let syn::Lit::Int(li) = &el.lit {
                // base10_parse handles decimal literals and strips type suffixes
                if let Ok(v) = li.base10_parse::<u128>() {
                    return Some(v);
                }
                // For non-decimal (hex/bin/oct), parse from the token string
                let s = li.to_string();
                let (prefix, rest) = if let Some(r) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
                    (16u32, r)
                } else if let Some(r) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
                    (2, r)
                } else if let Some(r) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
                    (8, r)
                } else {
                    return None;
                };
                // Strip type suffix and digit separators
                let digits: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .filter(|c| *c != '_' && !c.is_alphabetic())
                    .collect();
                u128::from_str_radix(&digits, prefix).ok()
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Computes discriminant token streams for every variant, auto-assigning values where absent.
///
/// Follows Rust's own rule: starts at `0`, increments by one after each variant. If a variant
/// has an explicit discriminant, that value is used and the counter resets to `explicit + 1`.
/// When the explicit value cannot be statically evaluated (e.g. a named constant), the counter
/// falls back to incrementing from the previous known position.
fn compute_discriminants(
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::Token![,]>,
) -> Vec<proc_macro2::TokenStream> {
    let mut next: u128 = 0;
    variants
        .iter()
        .map(|v| {
            if let Some((_, expr)) = &v.discriminant {
                // Try to evaluate to keep the counter accurate
                if let Some(val) = try_eval_int_expr(expr) {
                    next = val + 1;
                } else {
                    next += 1;
                }
                quote! { #expr }
            } else {
                let val = next;
                next += 1;
                let lit = proc_macro2::Literal::u128_unsuffixed(val);
                quote! { #lit }
            }
        })
        .collect()
}

/// Handles deriving `Readable` and `Writable` for enums that have at least one variant with fields.
///
/// The discriminant is written/read first (using the `#[repr(...)]` type and optional endianness),
/// followed by the fields of the matched variant. All variant fields must implement [`Readable`] /
/// [`Writable`].
///
/// ## Optional attributes
///
/// - `#[repr(...)]` — if omitted, the repr is auto-selected from the variant count:
///   ≤256 → `u8`, ≤65536 → `u16`, ≤2³² → `u32`, otherwise `u64`.
/// - Explicit discriminants — if omitted, values are auto-assigned starting at `0` and
///   incrementing by one, following Rust's own default discriminant rules.
/// - Endianness — required for user-supplied multi-byte reprs; auto-selected reprs default
///   to little-endian.
fn handle_field_enum_derive(
    enum_name: Ident,
    generics: syn::Generics,
    attrs: Vec<syn::Attribute>,
    enum_data: &syn::DataEnum,
    bc: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let endianness = parse_byteable_attr(&attrs);
    if matches!(endianness, AttributeType::Transparent | AttributeType::TryTransparent) {
        panic!("transparent and try_transparent attributes are not supported for enums");
    }

    // Determine repr type — use explicit #[repr(...)] if present, otherwise auto-select.
    let repr_auto_selected = extract_repr_type(&attrs).is_none();
    let repr_ty = extract_repr_type(&attrs).unwrap_or_else(|| {
        let n = enum_data.variants.len();
        let ty_str = if n <= 256 {
            "u8"
        } else if n <= 65_536 {
            "u16"
        } else if n as u64 <= u32::MAX as u64 + 1 {
            "u32"
        } else {
            "u64"
        };
        Ident::new(ty_str, enum_name.span())
    });

    let repr_ty_str = repr_ty.to_string();
    let is_single_byte = matches!(repr_ty_str.as_str(), "u8" | "i8");

    // For multi-byte repr: auto-selected defaults to little-endian; user-specified requires explicit.
    let effective_endianness = if !is_single_byte
        && !matches!(endianness, AttributeType::LittleEndian | AttributeType::BigEndian)
    {
        if repr_auto_selected {
            AttributeType::LittleEndian
        } else {
            panic!(
                "Field enums with an explicit multi-byte repr type require an endianness attribute: \
                 add #[byteable(little_endian)] or #[byteable(big_endian)]"
            )
        }
    } else {
        endianness
    };

    // Pre-compute discriminant tokens for every variant (auto-assign where absent).
    let discriminants = compute_discriminants(&enum_data.variants);

    // Tokens that read the discriminant and bind it as `disc: #repr_ty`.
    let read_disc = if is_single_byte {
        quote! { let disc: #repr_ty = reader.read_byteable()?; }
    } else {
        match effective_endianness {
            AttributeType::LittleEndian => quote! {
                let disc: #repr_ty = reader.read_byteable::<#bc::LittleEndian<#repr_ty>>()?.get();
            },
            AttributeType::BigEndian => quote! {
                let disc: #repr_ty = reader.read_byteable::<#bc::BigEndian<#repr_ty>>()?.get();
            },
            _ => unreachable!(),
        }
    };

    let write_arms = enum_data.variants.iter().zip(&discriminants).map(|(variant, disc_tokens)| {
        let variant_name = &variant.ident;

        let write_disc = if is_single_byte {
            quote! {
                let disc_val: #repr_ty = #disc_tokens as #repr_ty;
                writer.write_byteable(&disc_val)?;
            }
        } else {
            match effective_endianness {
                AttributeType::LittleEndian => quote! {
                    let disc_val: #repr_ty = #disc_tokens as #repr_ty;
                    writer.write_byteable(&#bc::LittleEndian::new(disc_val))?;
                },
                AttributeType::BigEndian => quote! {
                    let disc_val: #repr_ty = #disc_tokens as #repr_ty;
                    writer.write_byteable(&#bc::BigEndian::new(disc_val))?;
                },
                _ => unreachable!(),
            }
        };

        match &variant.fields {
            Fields::Unit => quote! {
                #enum_name::#variant_name => { #write_disc }
            },
            Fields::Named(named) => {
                let field_names: Vec<_> = named.named.iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();
                let field_writes: Vec<_> = named.named.iter()
                    .map(|f| gen_field_write(f.ident.as_ref().unwrap(), &f.ty, &f.attrs, &bc))
                    .collect();
                quote! {
                    #enum_name::#variant_name { #(#field_names),* } => {
                        #write_disc
                        #( #field_writes )*
                    }
                }
            }
            Fields::Unnamed(unnamed) => {
                let field_idents: Vec<_> = (0..unnamed.unnamed.len())
                    .map(|i| Ident::new(&format!("__field_{i}"), enum_name.span()))
                    .collect();
                let field_writes: Vec<_> = unnamed.unnamed.iter().zip(&field_idents)
                    .map(|(f, ident)| gen_field_write(ident, &f.ty, &f.attrs, &bc))
                    .collect();
                quote! {
                    #enum_name::#variant_name(#(#field_idents),*) => {
                        #write_disc
                        #( #field_writes )*
                    }
                }
            }
        }
    });

    let read_arms = enum_data.variants.iter().zip(&discriminants).map(|(variant, disc_tokens)| {
        let variant_name = &variant.ident;

        match &variant.fields {
            Fields::Unit => quote! {
                #disc_tokens => Ok(#enum_name::#variant_name),
            },
            Fields::Named(named) => {
                let field_idents: Vec<_> = named.named.iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();
                let field_reads: Vec<_> = named.named.iter()
                    .map(|f| gen_field_read(f.ident.as_ref().unwrap(), &f.ty, &f.attrs, &bc))
                    .collect();
                quote! {
                    #disc_tokens => {
                        #( #field_reads )*
                        Ok(#enum_name::#variant_name { #(#field_idents),* })
                    }
                }
            }
            Fields::Unnamed(unnamed) => {
                let field_idents: Vec<_> = (0..unnamed.unnamed.len())
                    .map(|i| Ident::new(&format!("__field_{i}"), enum_name.span()))
                    .collect();
                let field_reads: Vec<_> = unnamed.unnamed.iter().zip(&field_idents)
                    .map(|(f, ident)| gen_field_read(ident, &f.ty, &f.attrs, &bc))
                    .collect();
                quote! {
                    #disc_tokens => {
                        #( #field_reads )*
                        Ok(#enum_name::#variant_name(#(#field_idents),*))
                    }
                }
            }
        }
    });

    let enum_name_str = enum_name.to_string();

    quote! {
        impl #impl_generics #bc::Writable for #enum_name #type_generics #where_clause {
            fn write_to(&self, mut writer: &mut (impl ::std::io::Write + ?Sized)) -> ::std::io::Result<()> {
                use #bc::WriteByteable;
                match self {
                    #(#write_arms)*
                }
                Ok(())
            }
        }

        impl #impl_generics #bc::Readable for #enum_name #type_generics #where_clause {
            fn read_from(mut reader: &mut (impl ::std::io::Read + ?Sized)) -> ::std::io::Result<Self> {
                use #bc::ReadByteable;
                #read_disc
                match disc {
                    #(#read_arms)*
                    _ => Err(::std::io::Error::new(
                        ::std::io::ErrorKind::InvalidData,
                        format!("invalid discriminant for enum {}", #enum_name_str),
                    )),
                }
            }
        }
    }
}
