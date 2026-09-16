use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Ident, Meta, Type, parse_macro_input};

mod attrs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttributeType {
    LittleEndian,
    BigEndian,
    TryTransparent,
    IoOnly,
    None,
}

fn parse_byteable_attr(attrs: &[syn::Attribute]) -> AttributeType {
    let mut found: Option<AttributeType> = None;
    for attr in attrs {
        if !attr.path().is_ident("byteable") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            let kind = if meta.path.is_ident("little_endian") {
                Some(AttributeType::LittleEndian)
            } else if meta.path.is_ident("big_endian") {
                Some(AttributeType::BigEndian)
            } else if meta.path.is_ident("transparent") {
                Some(AttributeType::None)
            } else if meta.path.is_ident("try_transparent") {
                Some(AttributeType::TryTransparent)
            } else if meta.path.is_ident("io_only") {
                Some(AttributeType::IoOnly)
            } else if meta.path.is_ident("order") {
                // Consumed by `attrs::parse_field_order`; just skip the `= <int>` here.
                let _ = meta.value()?.parse::<syn::LitInt>()?;
                None
            } else if meta.path.is_ident("discriminant") {
                // Consumed by `attrs::parse_discriminant_override`; just skip the `= <ident>`
                // here.
                let _ = meta.value()?.parse::<syn::Ident>()?;
                None
            } else {
                return Err(meta.error(
                    "Unknown byteable attribute. Valid attributes are: little_endian, \
                     big_endian, try_transparent, io_only, order, discriminant.",
                ));
            };
            if let Some(kind) = kind {
                if found.is_some() {
                    return Err(meta.error(
                        "at most one of little_endian/big_endian/transparent/try_transparent/\
                         io_only may be specified",
                    ));
                }
                found = Some(kind);
            }
            Ok(())
        })
        .unwrap_or_else(|e| panic!("{e}"));
    }
    found.unwrap_or(AttributeType::None)
}

/// Assembles the output of the dynamic (`io_only`/field-enum) pipeline from the four
/// per-flavor impl blocks it is handed - `std::io`-based, `embedded-io`-based, tokio-based,
/// and `embedded-io-async`-based - keeping only the ones this build of `byteable` actually
/// supports. Each flavor is gated independently on its own feature.
///
/// This can't be done with `#[cfg(feature = "std")]` inside the emitted tokens themselves -
/// that cfg would be evaluated against the *downstream* crate's own Cargo features (e.g. a
/// `#![no_std]` binary that doesn't define a `std` feature at all), not `byteable`'s. Instead
/// `byteable_derive` mirrors `byteable`'s `std`/`embedded-io`/`tokio`/`embedded-io-async`
/// features onto itself (forwarded via `byteable_derive?/std`, `byteable_derive?/embedded-io`,
/// `byteable_derive?/tokio` and `byteable_derive?/embedded-io-async` in `byteable/Cargo.toml`),
/// so `cfg!` here - evaluated once, at the time this proc-macro crate itself was compiled -
/// correctly reflects which wire formats are actually available for whoever is deriving.
fn dynamic_pipeline_impls(
    type_name: &Ident,
    std_impl: proc_macro2::TokenStream,
    eio_impl: proc_macro2::TokenStream,
    async_impl: proc_macro2::TokenStream,
    eio_async_impl: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    if !cfg!(feature = "std") && !cfg!(feature = "embedded-io") {
        panic!(
            "deriving Byteable on `{type_name}` needs the io_only/field-enum dynamic pipeline, \
             which requires the `std` and/or `embedded-io` feature of `byteable` to be enabled - \
             neither is active for this build"
        );
    }
    let std_impl = if cfg!(feature = "std") {
        std_impl
    } else {
        quote! {}
    };
    let eio_impl = if cfg!(feature = "embedded-io") {
        eio_impl
    } else {
        quote! {}
    };
    // tokio/embedded-io-async are "bonus" async impls layered on an already-guaranteed sync
    // base (tokio implies std, embedded-io-async implies embedded-io - see Cargo.toml), so
    // they need no analog of the panic! above: it is structurally impossible to enable either
    // without its sync base already satisfying it.
    let async_impl = if cfg!(feature = "tokio") {
        async_impl
    } else {
        quote! {}
    };
    let eio_async_impl = if cfg!(feature = "embedded-io-async") {
        eio_async_impl
    } else {
        quote! {}
    };
    quote! { #std_impl #eio_impl #async_impl #eio_async_impl }
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

/// Derive macro that generates byte-serialization impls for structs and enums.
///
/// `#[derive(Byteable)]` inspects the annotated type and generates one of two sets of
/// traits depending on whether `#[byteable(io_only)]` is present:
///
/// - **Fixed-size** (default for structs): generates [`RawRepr`], [`FromRawRepr`] or
///   [`TryFromRawRepr`], [`ToByteArray`], and [`FromByteArray`] or [`TryFromByteArray`].
///   A hidden `#[repr(C, packed)]` raw struct is created to hold the on-wire layout.
///
/// - **I/O streaming** (`#[byteable(io_only)]` on structs, always for field enums):
///   generates [`Readable`]/[`Writable`] (`std::io`-based) when the `std` feature of
///   `byteable` is enabled, [`eio::EioReadable`]/[`eio::EioWritable`] (the `no_std`-friendly
///   counterpart) when `embedded-io` is enabled, [`async_io::AsyncReadable`]/
///   [`async_io::AsyncWritable`] (tokio-based) when `tokio` is enabled, and
///   [`eio_async::EioAsyncReadable`]/[`eio_async::EioAsyncWritable`] (the async, `no_std`-friendly
///   counterpart) when `embedded-io-async` is enabled - any combination of the four, and a
///   compile error at the derive site if none of `std`/`embedded-io` is on (the two async
///   flavors each imply one of these). Not opt-in per type: if a field type doesn't support the
///   wire format a given feature implies, that surfaces as a normal compile error, same as
///   any other trait with field requirements.
///
/// - **Unit enums** (all variants are unit): generates [`TryFromRawRepr`],
///   [`ToByteArray`], and [`TryFromByteArray`] using an automatically-chosen
///   discriminant integer type (`u8` → `u16` → `u32` → `u64` based on variant count).
///
/// ## Enum wire discriminants: `tag`/counting only, never Rust's real `= N`
///
/// For every enum (unit-only or field-carrying), the value written on the wire for a
/// variant's discriminant comes *purely* from `#[byteable(tag = N)]` and declaration-order
/// counting (see the "Variant-level attributes" table below) - Rust's own `enum Foo { A = 1
/// }` discriminant syntax is **never read** for wire-encoding purposes, even though it still
/// compiles and affects `as` casts/`std::mem::discriminant` as normal Rust. If you need a
/// specific wire value, use `#[byteable(tag = N)]`, not `= N`.
///
/// The discriminant's *wire width* (how many bytes it takes up) is chosen independently: by
/// `#[byteable(discriminant = uN/iN)]` when present, else by a legacy `#[repr(uN/iN)]` on the
/// enum (still honored purely as a width source via `extract_repr_type`, not as a source of
/// discriminant values), else auto-selected as the smallest of `u8`/`u16`/`u32`/`u64` that
/// fits the variant count.
///
/// [`RawRepr`]: byteable::RawRepr
/// [`FromRawRepr`]: byteable::FromRawRepr
/// [`TryFromRawRepr`]: byteable::TryFromRawRepr
/// [`ToByteArray`]: byteable::ToByteArray
/// [`FromByteArray`]: byteable::FromByteArray
/// [`TryFromByteArray`]: byteable::TryFromByteArray
/// [`Readable`]: byteable::io::Readable
/// [`Writable`]: byteable::io::Writable
/// [`eio::EioReadable`]: byteable::eio::EioReadable
/// [`eio::EioWritable`]: byteable::eio::EioWritable
/// [`async_io::AsyncReadable`]: byteable::async_io::AsyncReadable
/// [`async_io::AsyncWritable`]: byteable::async_io::AsyncWritable
/// [`eio_async::EioAsyncReadable`]: byteable::eio_async::EioAsyncReadable
/// [`eio_async::EioAsyncWritable`]: byteable::eio_async::EioAsyncWritable
///
/// # Struct-level attributes
///
/// Place these on the struct or enum itself:
///
/// | Attribute | Effect |
/// |-----------|--------|
/// | `#[byteable(little_endian)]` | All multi-byte fields use little-endian representation |
/// | `#[byteable(big_endian)]` | All multi-byte fields use big-endian representation |
/// | `#[byteable(io_only)]` | Generate `Readable`/`Writable`/`EioReadable`/`EioWritable`/`AsyncReadable`/`AsyncWritable`/`EioAsyncReadable`/`EioAsyncWritable` (per enabled feature) instead of fixed-size traits |
/// | `#[byteable(discriminant = uN/iN)]` (enums only) | Overrides the wire width of the enum's discriminant with `uN`/`iN` (`u8`/`u16`/`u32`/`u64`/`u128`/`i8`/`i16`/`i32`/`i64`/`i128`), independently of any `#[repr(...)]` on the enum. See "Variant-level attributes" below and the note on `u128` there. |
///
/// # Field-level attributes
///
/// Place these on individual fields or enum variants:
///
/// | Attribute | Effect |
/// |-----------|--------|
/// | `#[byteable(little_endian)]` | This field uses little-endian (overrides struct-level) |
/// | `#[byteable(big_endian)]` | This field uses big-endian (overrides struct-level) |
/// | `#[byteable(try_transparent)]` | Field decode may fail; the struct impl becomes `TryFromRawRepr` |
/// | `#[byteable(order = N)]` (struct fields only) | Pins this field's *wire position* to `N`, independently of its declaration order in the struct. Adds zero bytes to the wire format - it only changes which byte range a given field occupies. Must annotate either every field of the struct or none; the `N` values across all fields must form a dense `0..field_count` permutation. |
///
/// # Variant-level attributes
///
/// Place these on individual enum variants:
///
/// | Attribute | Effect |
/// |-----------|--------|
/// | `#[byteable(tag = N)]` | Pins this variant's *wire discriminant* to the integer literal `N`, independently of its declaration position and of any real Rust `= N` discriminant (which is never read - see above). `N` may be a negative literal (e.g. `tag = -1`); variants without an explicit `tag` count up from the previous resolved value (or from `0` for the first variant). |
///
/// A `u128`-specific quirk worth knowing: `tag` is parsed as `i128`, so no positive literal
/// can express a value in the upper half of `u128`'s range (`2^127` to `u128::MAX`). Under
/// `#[byteable(discriminant = u128)]`, a *negative* `tag` is the documented escape hatch for
/// that range, reached via two's-complement wraparound - e.g. `tag = -1` means `u128::MAX`,
/// `tag = -10` means `u128::MAX - 9`. This is the one wire width where a negative `tag` is
/// accepted rather than rejected as "doesn't fit"; every other unsigned width
/// (`u8`/`u16`/`u32`/`u64`) rejects a negative `tag` as a compile error.
///
/// # Examples
///
/// ## Basic fixed-size struct
///
/// ```rust
/// use byteable::{Byteable, ToByteArray, TryFromByteArray};
///
/// #[derive(Byteable)]
/// struct Point {
///     x: f32,
///     y: f32,
/// }
///
/// let p = Point { x: 1.0, y: 2.0 };
/// let bytes = p.to_byte_array();
/// let p2 = Point::try_from_byte_array(bytes).unwrap();
/// assert_eq!(p.x, p2.x);
/// ```
///
/// ## Mixed-endian struct
///
/// ```rust
/// use byteable::Byteable;
///
/// #[derive(Byteable)]
/// #[byteable(big_endian)]
/// struct NetworkHeader {
///     magic: u32,
///     #[byteable(little_endian)]
///     flags: u16,   // little-endian despite struct-level big_endian
///     version: u8,  // single-byte, endian has no effect
/// }
/// ```
///
/// ## Dynamic struct with `io_only`
///
/// ```rust
/// use byteable::Byteable;
/// use byteable::io::{Writable, Readable, WriteValue, ReadValue};
///
/// #[derive(Byteable)]
/// #[byteable(io_only)]
/// struct Message {
///     id: u32,
///     body: String,
///     tags: Vec<String>,
/// }
///
/// let msg = Message { id: 1, body: "hello".into(), tags: vec![] };
/// let mut buf = Vec::new();
/// buf.write_value(&msg).unwrap();
/// let msg2 = std::io::Cursor::new(&buf).read_value::<Message>().unwrap();
/// assert_eq!(msg.id, msg2.id);
/// ```
///
/// ## Unit enum (auto-inferred repr)
///
/// ```rust
/// use byteable::{Byteable, ToByteArray, TryFromByteArray};
///
/// #[derive(Byteable, Debug, PartialEq)]
/// enum Color {
///     Red,
///     Green,
///     Blue,
/// }
///
/// // Fits in u8 (3 variants), so wire size is 1 byte.
/// assert_eq!(Color::BYTE_SIZE, 1);
/// let bytes = Color::Green.to_byte_array();
/// assert_eq!(Color::try_from_byte_array(bytes).unwrap(), Color::Green);
/// ```
///
/// ## Field enum
///
/// ```rust
/// use byteable::Byteable;
/// use byteable::io::{Writable, Readable, WriteValue, ReadValue};
///
/// #[derive(Byteable, Debug, PartialEq)]
/// enum Shape {
///     Circle { radius: f32 },
///     Rect { width: f32, height: f32 },
/// }
///
/// let s = Shape::Circle { radius: 3.0 };
/// let mut buf = Vec::new();
/// buf.write_value(&s).unwrap();
/// let s2 = std::io::Cursor::new(&buf).read_value::<Shape>().unwrap();
/// assert_eq!(s, s2);
/// ```
#[proc_macro_derive(Byteable, attributes(byteable))]
pub fn byteable_derive_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    match input.data {
        Data::Struct(_) => return struct_derive(input),
        Data::Enum(_) => return enum_derive(input),
        Data::Union(_) => panic!("union structs are unsupported"),
    }
}

fn struct_derive(input: DeriveInput) -> proc_macro::TokenStream {
    if parse_byteable_attr(&input.attrs) == AttributeType::IoOnly {
        return io_struct_derive(input);
    }
    fixed_struct_derived(input)
}

fn gen_struct_field_write(
    field_access: &proc_macro2::TokenStream,
    field_type: &Type,
    attrs: &[syn::Attribute],
    bc: &proc_macro2::TokenStream,
    awaited: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match parse_byteable_attr(attrs) {
        AttributeType::LittleEndian => quote! {
            writer.write_value(&<#field_type as #bc::HasEndianRepr>::to_little_endian(#field_access))#awaited?;
        },
        AttributeType::BigEndian => quote! {
            writer.write_value(&<#field_type as #bc::HasEndianRepr>::to_big_endian(#field_access))#awaited?;
        },
        AttributeType::None => quote! { writer.write_value(&#field_access)#awaited?; },
        AttributeType::IoOnly => {
            panic!("#[byteable(io_only)] is a struct-level attribute and cannot be used on a field")
        }
        AttributeType::TryTransparent => panic!(
            "#[byteable(try_transparent)] is not applicable in \
             io_only mode; remove the annotation or use a plain field"
        ),
    }
}

fn gen_field_read(
    field_ident: &Ident,
    field_ty: &syn::Type,
    attrs: &[syn::Attribute],
    bc: &proc_macro2::TokenStream,
    awaited: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match parse_byteable_attr(attrs) {
        AttributeType::LittleEndian => {
            quote! { let #field_ident: #field_ty = reader.read_value::<<#field_ty as #bc::HasEndianRepr>::LE>()#awaited?.get(); }
        }
        AttributeType::BigEndian => {
            quote! { let #field_ident: #field_ty = reader.read_value::<<#field_ty as #bc::HasEndianRepr>::BE>()#awaited?.get(); }
        }
        AttributeType::None => {
            quote! { let #field_ident: #field_ty = reader.read_value()#awaited?; }
        }
        other => panic!(
            "unsupported #[byteable] attribute `{other:?}` on field `{field_ident}`; \
             only little_endian and big_endian are supported here"
        ),
    }
}

fn io_struct_derive(input: DeriveInput) -> proc_macro::TokenStream {
    let bc = byteable_crate_path();
    let name = &input.ident;

    let fields_data = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => unreachable!(),
    };

    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    if let Fields::Unit = fields_data {
        let vis = &input.vis;
        let raw_name = format_ident!("__byteable_raw_{}", name);
        return quote! {
            #[derive(Clone, Copy)]
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #vis struct #raw_name;

            unsafe impl #bc::PlainOldData for #raw_name {}

            impl #bc::RawRepr for #name {
                type Raw = #raw_name;

                #[inline]
                fn to_raw(&self) -> #raw_name {
                    #raw_name
                }
            }

            impl #bc::FromRawRepr for #name {
                #[inline]
                fn from_raw(value: #raw_name) -> Self {
                    Self
                }
            }

            impl #bc::TryFromRawRepr for #name {
                #[inline]
                fn try_from_raw(value: #raw_name) -> Result<Self, #bc::DecodeError> {
                    Ok(Self)
                }
            }

            impl #bc::ToByteArray for #raw_name
                where #raw_name : #bc::PlainOldData
            {
                type ByteArray = [u8; ::core::mem::size_of::<Self>()];
                fn to_byte_array(&self) -> Self::ByteArray {
                    #[allow(unnecessary_transmutes)]
                    unsafe { ::core::mem::transmute(*self) }
                }
            }

            impl #bc::FromByteArray for #raw_name
                where #raw_name : #bc::PlainOldData
            {
                fn from_byte_array(byte_array: <Self as #bc::ToByteArray>::ByteArray) -> Self {
                    #[allow(unnecessary_transmutes)]
                    unsafe { ::core::mem::transmute(byte_array) }

                }
            }

            impl #bc::ToByteArray for #name
            where
                #name: #bc::RawRepr,
                <#name as #bc::RawRepr>::Raw: #bc::ToByteArray,
            {
                type ByteArray = <<Self as #bc::RawRepr>::Raw as #bc::ToByteArray>::ByteArray;

                fn to_byte_array(&self) -> Self::ByteArray {
                    <Self as #bc::RawRepr>::to_raw(self).to_byte_array()
                }
            }

            impl #bc::FromByteArray for #name
            where
                #name: #bc::FromRawRepr,
                <#name as #bc::RawRepr>::Raw: #bc::FromByteArray,
            {
                fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                    let raw = <<Self as #bc::RawRepr>::Raw as #bc::FromByteArray>::from_byte_array(byte_array);
                    <Self as #bc::FromRawRepr>::from_raw(raw)
                }
            }
        }
        .into();
    }

    let (fields, is_tuple) = match fields_data {
        syn::Fields::Named(f) => (&f.named, false),
        syn::Fields::Unnamed(f) => (&f.unnamed, true),
        syn::Fields::Unit => unreachable!(),
    };

    let orders: Vec<Option<u64>> = fields.iter().map(|f| attrs::parse_field_order(&f.attrs)).collect();
    let wire_order = attrs::resolve_field_wire_order(&orders).unwrap_or_else(|e| panic!("{e}"));

    let awaited_sync = quote! {};
    let awaited_async = quote! { .await };

    let field_accesses: Vec<_> = fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let field_access = if is_tuple {
                let idx = syn::Index::from(i);
                quote! { self.#idx }
            } else {
                let fname = field.ident.as_ref().unwrap();
                quote! { self.#fname }
            };
            (field_access, field)
        })
        .collect();
    let field_accesses: Vec<_> = wire_order
        .iter()
        .map(|&decl_idx| field_accesses[decl_idx].clone())
        .collect();
    let write_stmts: Vec<_> = field_accesses
        .iter()
        .map(|(access, field)| {
            gen_struct_field_write(access, &field.ty, &field.attrs, &bc, &awaited_sync)
        })
        .collect();
    let write_stmts_async: Vec<_> = field_accesses
        .iter()
        .map(|(access, field)| {
            gen_struct_field_write(access, &field.ty, &field.attrs, &bc, &awaited_async)
        })
        .collect();

    let (read_bindings, read_bindings_async, construct_expr): (
        Vec<_>,
        Vec<_>,
        proc_macro2::TokenStream,
    ) = if is_tuple {
        let idents: Vec<_> = (0..fields.len())
            .map(|i| syn::Ident::new(&format!("__field_{i}"), name.span()))
            .collect();
        let wire_fields: Vec<_> = wire_order.iter().map(|&i| &fields[i]).collect();
        let wire_idents: Vec<_> = wire_order.iter().map(|&i| &idents[i]).collect();
        let bindings = wire_fields
            .iter()
            .zip(&wire_idents)
            .map(|(f, id)| gen_field_read(id, &f.ty, &f.attrs, &bc, &awaited_sync))
            .collect();
        let bindings_async = wire_fields
            .iter()
            .zip(&wire_idents)
            .map(|(f, id)| gen_field_read(id, &f.ty, &f.attrs, &bc, &awaited_async))
            .collect();
        (bindings, bindings_async, quote! { Ok(Self(#(#idents),*)) })
    } else {
        let field_idents: Vec<_> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
        let wire_fields: Vec<_> = wire_order.iter().map(|&i| &fields[i]).collect();
        let bindings = wire_fields
            .iter()
            .map(|f| {
                gen_field_read(
                    f.ident.as_ref().unwrap(),
                    &f.ty,
                    &f.attrs,
                    &bc,
                    &awaited_sync,
                )
            })
            .collect();
        let bindings_async = wire_fields
            .iter()
            .map(|f| {
                gen_field_read(
                    f.ident.as_ref().unwrap(),
                    &f.ty,
                    &f.attrs,
                    &bc,
                    &awaited_async,
                )
            })
            .collect();
        (
            bindings,
            bindings_async,
            quote! { Ok(Self { #(#field_idents),* }) },
        )
    };

    let std_impl = quote! {
        impl #impl_generics #bc::io::Readable for #name #type_generics #where_clause {
            fn read_from(mut reader: &mut (impl ::std::io::Read + ?Sized)) -> Result<Self, #bc::io::ReadableError> {
                use #bc::io::ReadValue;
                #( #read_bindings )*
                #construct_expr
            }
        }
        impl #impl_generics #bc::io::Writable for #name #type_generics #where_clause {
            fn write_to(&self, mut writer: &mut (impl ::std::io::Write + ?Sized)) -> ::std::io::Result<()> {
                use #bc::io::WriteValue;
                #( #write_stmts )*
                Ok(())
            }
        }
    };
    let eio_impl = quote! {
        impl #impl_generics #bc::eio::EioReadable for #name #type_generics #where_clause {
            fn read_from<__ByteableEioR: #bc::eio::EioReader + ?Sized>(mut reader: &mut __ByteableEioR) -> Result<Self, #bc::eio::EioReadableError<__ByteableEioR::Error>> {
                use #bc::eio::EioReadValue;
                #( #read_bindings )*
                #construct_expr
            }
        }
        impl #impl_generics #bc::eio::EioWritable for #name #type_generics #where_clause {
            fn write_to<__ByteableEioW: #bc::eio::EioWriter + ?Sized>(&self, mut writer: &mut __ByteableEioW) -> Result<(), __ByteableEioW::Error> {
                use #bc::eio::EioWriteValue;
                #( #write_stmts )*
                Ok(())
            }
        }
    };
    let async_impl = quote! {
        impl #impl_generics #bc::async_io::AsyncReadable for #name #type_generics #where_clause {
            fn read_from(mut reader: &mut (impl #bc::__tokio::io::AsyncReadExt + ?Sized + Unpin)) -> impl ::core::future::Future<Output = Result<Self, #bc::io::ReadableError>> {
                async move {
                    use #bc::async_io::AsyncReadValue;
                    #( #read_bindings_async )*
                    #construct_expr
                }
            }
        }
        impl #impl_generics #bc::async_io::AsyncWritable for #name #type_generics #where_clause {
            fn write_to(&self, mut writer: &mut (impl #bc::__tokio::io::AsyncWriteExt + ?Sized + Unpin)) -> impl ::core::future::Future<Output = ::std::io::Result<()>> {
                async move {
                    use #bc::async_io::AsyncWriteValue;
                    #( #write_stmts_async )*
                    Ok(())
                }
            }
        }
    };
    let eio_async_impl = quote! {
        impl #impl_generics #bc::eio_async::EioAsyncReadable for #name #type_generics #where_clause {
            fn read_from<__ByteableEioAsyncR: #bc::eio_async::EioAsyncReader + ?Sized>(mut reader: &mut __ByteableEioAsyncR) -> impl ::core::future::Future<Output = Result<Self, #bc::eio::EioReadableError<__ByteableEioAsyncR::Error>>> {
                async move {
                    use #bc::eio_async::EioAsyncReadValue;
                    #( #read_bindings_async )*
                    #construct_expr
                }
            }
        }
        impl #impl_generics #bc::eio_async::EioAsyncWritable for #name #type_generics #where_clause {
            fn write_to<__ByteableEioAsyncW: #bc::eio_async::EioAsyncWriter + ?Sized>(&self, mut writer: &mut __ByteableEioAsyncW) -> impl ::core::future::Future<Output = Result<(), __ByteableEioAsyncW::Error>> {
                async move {
                    use #bc::eio_async::EioAsyncWriteValue;
                    #( #write_stmts_async )*
                    Ok(())
                }
            }
        }
    };

    dynamic_pipeline_impls(name, std_impl, eio_impl, async_impl, eio_async_impl).into()
}

fn fixed_struct_derived(input: DeriveInput) -> proc_macro::TokenStream {
    let bc = byteable_crate_path();
    let original_name = &input.ident;

    // let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let fields_data = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => unreachable!(),
    };

    let vis = &input.vis;
    let raw_name = format_ident!("__byteable_raw_{}", original_name);

    if let Fields::Unit = fields_data {
        return quote! {
            #[derive(Clone, Copy)]
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #vis struct #raw_name;

            unsafe impl #bc::PlainOldData for #raw_name {}

            impl #bc::RawRepr for #original_name {
                type Raw = #raw_name;

                #[inline]
                fn to_raw(&self) -> #raw_name {
                    #raw_name
                }
            }

            impl #bc::FromRawRepr for #original_name {
                #[inline]
                fn from_raw(value: #raw_name) -> Self {
                    Self
                }
            }

            impl #bc::TryFromRawRepr for #original_name {
                #[inline]
                fn try_from_raw(value: #raw_name) -> Result<Self, #bc::DecodeError> {
                    Ok(Self)
                }
            }

            impl #bc::ToByteArray for #raw_name
                where #raw_name : #bc::PlainOldData
            {
                type ByteArray = [u8; ::core::mem::size_of::<Self>()];
                fn to_byte_array(&self) -> Self::ByteArray {
                    #[allow(unnecessary_transmutes)]
                    unsafe { ::core::mem::transmute(*self) }
                }
            }

            impl #bc::FromByteArray for #raw_name
                where #raw_name : #bc::PlainOldData
            {
                fn from_byte_array(byte_array: <Self as #bc::ToByteArray>::ByteArray) -> Self {
                    #[allow(unnecessary_transmutes)]
                    unsafe { ::core::mem::transmute(byte_array) }

                }
            }

            impl #bc::ToByteArray for #original_name
            where
                #original_name: #bc::RawRepr,
                <#original_name as #bc::RawRepr>::Raw: #bc::ToByteArray,
            {
                type ByteArray = [u8; ::core::mem::size_of::<<Self as #bc::RawRepr>::Raw>()];
                fn to_byte_array(&self) -> Self::ByteArray {
                    <Self as #bc::RawRepr>::to_raw(self).to_byte_array()
                }
            }

            impl #bc::FromByteArray for #original_name
            where
                #original_name: #bc::FromRawRepr,
                <#original_name as #bc::RawRepr>::Raw: #bc::FromByteArray,
            {
                fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                    let raw = <<Self as #bc::RawRepr>::Raw as #bc::FromByteArray>::from_byte_array(byte_array);
                    <Self as #bc::FromRawRepr>::from_raw(raw)
                }
            }
        }
        .into();
    }

    let (fields, is_tuple) = match fields_data {
        Fields::Named(f) => (&f.named, false),
        Fields::Unnamed(f) => (&f.unnamed, true),
        Fields::Unit => unreachable!(),
    };

    let orders: Vec<Option<u64>> = fields.iter().map(|f| attrs::parse_field_order(&f.attrs)).collect();
    let wire_order = attrs::resolve_field_wire_order(&orders).unwrap_or_else(|e| panic!("{e}"));
    let mut wire_position_of = vec![0usize; wire_order.len()];
    for (wire_pos, &decl_idx) in wire_order.iter().enumerate() {
        wire_position_of[decl_idx] = wire_pos;
    }

    struct FieldInfo {
        raw_field_def: proc_macro2::TokenStream,
        to_raw_expr: proc_macro2::TokenStream,
        from_raw_expr: proc_macro2::TokenStream,
    }

    // Process each field: determine raw type and to/from conversion expressions
    let mut field_infos = Vec::new();
    let mut has_try = false;

    for (i, field) in fields.iter().enumerate() {
        let field_type = &field.ty;
        let attr = parse_byteable_attr(&field.attrs);
        if attr == AttributeType::TryTransparent {
            has_try = true;
        }

        let field_info = if is_tuple {
            // `self` is the *original* struct (declaration-order indexed); `value` is
            // the hidden raw struct (wire-order indexed). These differ once a field
            // has been reordered via `#[byteable(order = N)]`, so they need separate
            // indices: `self_idx` for reading the source field out of `self`, `raw_idx`
            // for reading/writing that field's slot in the raw struct.
            let self_idx = syn::Index::from(i);
            let raw_idx = syn::Index::from(wire_position_of[i]);
            match attr {
                AttributeType::LittleEndian => FieldInfo {
                    raw_field_def: quote! { #vis <#field_type as #bc::HasEndianRepr>::LE },
                    to_raw_expr: quote! { <#field_type as #bc::HasEndianRepr>::to_little_endian(self.#self_idx) },
                    from_raw_expr: quote! { <#field_type as #bc::FromEndianRepr>::from_little_endian(value.#raw_idx) },
                },
                AttributeType::BigEndian => FieldInfo {
                    raw_field_def: quote! { #vis <#field_type as #bc::HasEndianRepr>::BE },
                    to_raw_expr: quote! { <#field_type as #bc::HasEndianRepr>::to_big_endian(self.#self_idx) },
                    from_raw_expr: quote! { <#field_type as #bc::FromEndianRepr>::from_big_endian(value.#raw_idx) },
                },
                AttributeType::TryTransparent => FieldInfo {
                    raw_field_def: quote! { #vis <#field_type as #bc::RawRepr>::Raw },
                    to_raw_expr: quote! { <#field_type as #bc::RawRepr>::to_raw(&self.#self_idx) },
                    from_raw_expr: quote! { <#field_type as #bc::TryFromRawRepr>::try_from_raw(value.#raw_idx)? },
                },
                AttributeType::IoOnly => panic!(
                    "#[byteable(io_only)] is a struct-level attribute and cannot be used on individual fields"
                ),
                AttributeType::None => FieldInfo {
                    raw_field_def: quote! { #vis <#field_type as #bc::RawRepr>::Raw },
                    to_raw_expr: quote! { <#field_type as #bc::RawRepr>::to_raw(&self.#self_idx) },
                    from_raw_expr: quote! { <#field_type as #bc::FromRawRepr>::from_raw(value.#raw_idx) },
                },
            }
        } else {
            let name = field.ident.as_ref().unwrap();
            match attr {
                AttributeType::LittleEndian => FieldInfo {
                    raw_field_def: quote! { #vis #name: <#field_type as #bc::HasEndianRepr>::LE },
                    to_raw_expr: quote! { #name: <#field_type as #bc::HasEndianRepr>::to_little_endian(self.#name) },
                    from_raw_expr: quote! { #name: <#field_type as #bc::FromEndianRepr>::from_little_endian(value.#name) },
                },
                AttributeType::BigEndian => FieldInfo {
                    raw_field_def: quote! { #vis #name: <#field_type as #bc::HasEndianRepr>::BE },
                    to_raw_expr: quote! { #name: <#field_type as #bc::HasEndianRepr>::to_big_endian(self.#name) },
                    from_raw_expr: quote! { #name: <#field_type as #bc::FromEndianRepr>::from_big_endian(value.#name) },
                },
                AttributeType::TryTransparent => FieldInfo {
                    raw_field_def: quote! { #vis #name: <#field_type as #bc::RawRepr>::Raw },
                    to_raw_expr: quote! { #name: <#field_type as #bc::RawRepr>::to_raw(&self.#name) },
                    from_raw_expr: quote! { #name: <#field_type as #bc::TryFromRawRepr>::try_from_raw(value.#name)? },
                },
                AttributeType::IoOnly => panic!(
                    "#[byteable(io_only)] is a struct-level attribute and cannot be used on individual fields"
                ),
                AttributeType::None => FieldInfo {
                    raw_field_def: quote! { #vis #name: <#field_type as #bc::RawRepr>::Raw },
                    to_raw_expr: quote! { #name: <#field_type as #bc::RawRepr>::to_raw(&self.#name) },
                    from_raw_expr: quote! { #name: <#field_type as #bc::FromRawRepr>::from_raw(value.#name) },
                },
            }
        };
        field_infos.push(field_info);
    }

    let wire_field_infos: Vec<&FieldInfo> = wire_order.iter().map(|&decl_idx| &field_infos[decl_idx]).collect();

    let raw_struct_def = {
        let field_defs = wire_field_infos.iter().map(|v| &v.raw_field_def);
        if is_tuple {
            quote! {
                #[derive(Clone, Copy)]
                #[repr(C, packed)]
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                #vis struct #raw_name( #(#field_defs),* );
            }
        } else {
            quote! {
                #[derive(Clone, Copy)]
                #[repr(C, packed)]
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                #vis struct #raw_name { #(#field_defs),* }
            }
        }
    };

    let raw_impls = {
        quote! {
            unsafe impl #bc::PlainOldData for #raw_name {}

            impl #bc::ToByteArray for #raw_name
                where #raw_name : #bc::PlainOldData
            {
                type ByteArray = [u8; ::core::mem::size_of::<Self>()];
                fn to_byte_array(&self) -> Self::ByteArray {
                    #[allow(unnecessary_transmutes)]
                    unsafe { ::core::mem::transmute(*self) }
                }
            }

            impl #bc::FromByteArray for #raw_name
                where #raw_name : #bc::PlainOldData
            {
                fn from_byte_array(byte_array: <Self as #bc::ToByteArray>::ByteArray) -> Self {
                    #[allow(unnecessary_transmutes)]
                    unsafe { ::core::mem::transmute(byte_array) }

                }
            }
        }
    };

    let from_raw_body = {
        let from_raw_exprs = field_infos.iter().map(|v| &v.from_raw_expr);
        if is_tuple {
            quote! { Self(#(#from_raw_exprs),*) }
        } else {
            quote! { Self { #(#from_raw_exprs),* } }
        }
    };

    let raw_repr = {
        let to_raw_exprs = wire_field_infos.iter().map(|v| &v.to_raw_expr);
        if is_tuple {
            quote! {
                impl #bc::RawRepr for #original_name {
                    type Raw = #raw_name;

                    #[inline]
                    fn to_raw(&self) -> Self::Raw {
                        #raw_name (#(#to_raw_exprs),*)
                    }
                }

                impl #bc::ToByteArray for #original_name
                where
                    #original_name: #bc::RawRepr,
                    <#original_name as #bc::RawRepr>::Raw: #bc::ToByteArray,
                {
                    type ByteArray = <<Self as #bc::RawRepr>::Raw as #bc::ToByteArray>::ByteArray;
                    fn to_byte_array(&self) -> Self::ByteArray {
                        <Self as #bc::RawRepr>::to_raw(self).to_byte_array()
                    }
                }

            }
        } else {
            quote! {
                impl #bc::RawRepr for #original_name {
                    type Raw = #raw_name;

                    #[inline]
                    fn to_raw(&self) -> Self::Raw {
                        #raw_name { #(#to_raw_exprs),* }
                    }
                }

                impl #bc::ToByteArray for #original_name
                where
                    #original_name: #bc::RawRepr,
                    <#original_name as #bc::RawRepr>::Raw: #bc::ToByteArray,
                {
                    type ByteArray = <<Self as #bc::RawRepr>::Raw as #bc::ToByteArray>::ByteArray;
                    fn to_byte_array(&self) -> Self::ByteArray {
                        <Self as #bc::RawRepr>::to_raw(self).to_byte_array()
                    }
                }
            }
        }
    };

    let original_impls = if has_try {
        quote! {
            impl #bc::TryFromRawRepr for #original_name {
                #[inline]
                fn try_from_raw(value: #raw_name) -> Result<Self, #bc::DecodeError> { Ok(#from_raw_body) }
            }

            impl #bc::TryFromByteArray for #original_name
            where
                #original_name: #bc::TryFromRawRepr,
                <#original_name as #bc::RawRepr>::Raw: #bc::FromByteArray,
            {
                fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, #bc::DecodeError> {
                    let raw = <<Self as #bc::RawRepr>::Raw as #bc::FromByteArray>::from_byte_array(byte_array);
                    <Self as #bc::TryFromRawRepr>::try_from_raw(raw)
                }
            }

        }
    } else {
        quote! {
            impl #bc::FromRawRepr for #original_name {
                #[inline]
                fn from_raw(value: #raw_name) -> Self { #from_raw_body }
            }

            impl #bc::TryFromRawRepr for #original_name {
                #[inline]
                fn try_from_raw(value: #raw_name) -> Result<Self, #bc::DecodeError> { Ok(<Self as #bc::FromRawRepr>::from_raw(value)) }
            }


            impl #bc::FromByteArray for #original_name
            where
                #original_name: #bc::FromRawRepr,
                <#original_name as #bc::RawRepr>::Raw: #bc::FromByteArray,
            {
                fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                    let raw = <<Self as #bc::RawRepr>::Raw as #bc::FromByteArray>::from_byte_array(byte_array);
                    <Self as #bc::FromRawRepr>::from_raw(raw)
                }
            }
        }
    };

    quote! {
        #raw_struct_def
        #raw_impls
        #raw_repr
        #original_impls
    }
    .into()
}

fn extract_repr_type(attrs: &[syn::Attribute]) -> Option<syn::Ident> {
    for attr in attrs {
        if attr.path().is_ident("repr") {
            if let Meta::List(meta_list) = &attr.meta {
                if let Ok(ident) = syn::parse2::<syn::Ident>(meta_list.tokens.clone()) {
                    if matches!(
                        ident.to_string().as_str(),
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

fn gen_enum_field_write(
    field_ident: &Ident,
    field_type: &Type,
    attrs: &[syn::Attribute],
    bc: &proc_macro2::TokenStream,
    awaited: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match parse_byteable_attr(attrs) {
        AttributeType::LittleEndian => quote! {
            writer.write_value(&<#field_type as #bc::HasEndianRepr>::to_little_endian(*#field_ident))#awaited?;
        },
        AttributeType::BigEndian => quote! {
            writer.write_value(&<#field_type as #bc::HasEndianRepr>::to_big_endian(*#field_ident))#awaited?;
        },
        AttributeType::None => quote! {
            writer.write_value(#field_ident)#awaited?;
        },
        other => panic!(
            "unsupported #[byteable] attribute `{other:?}` on field `{field_ident}`; \
             only little_endian and big_endian are supported here"
        ),
    }
}

fn enum_derive(input: DeriveInput) -> proc_macro::TokenStream {
    let Data::Enum(enum_data) = &input.data else {
        unreachable!();
    };
    let has_field_variants = enum_data
        .variants
        .iter()
        .any(|v| !matches!(v.fields, Fields::Unit));
    if !has_field_variants {
        return unit_enum_derive(input);
    }
    let name = input.ident;
    let bc = byteable_crate_path();

    // generate io_only variant

    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    // Determine repr type - use an explicit #[byteable(discriminant = ..)] override if
    // present, else an explicit #[repr(...)], otherwise auto-select.
    let repr_ty = attrs::parse_discriminant_override(&input.attrs)
        .or_else(|| extract_repr_type(&input.attrs))
        .unwrap_or_else(|| {
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
            Ident::new(ty_str, name.span())
        });

    let endian_attr = parse_byteable_attr(&input.attrs);
    let discriminants = resolve_enum_discriminants(&enum_data.variants, &repr_ty);

    let awaited_sync = quote! {};
    let awaited_async = quote! { .await };

    let gen_read_disc = |awaited: &proc_macro2::TokenStream| match endian_attr {
        AttributeType::LittleEndian => quote! {
            let disc: #repr_ty = <#repr_ty as #bc::FromEndianRepr>::from_little_endian(
                reader.read_value::<<#repr_ty as #bc::HasEndianRepr>::LE>()#awaited?
            );
        },
        AttributeType::BigEndian => quote! {
            let disc: #repr_ty = <#repr_ty as #bc::FromEndianRepr>::from_big_endian(
                reader.read_value::<<#repr_ty as #bc::HasEndianRepr>::BE>()#awaited?
            );
        },
        _ => quote! {
            let disc: #repr_ty = reader.read_value()#awaited?;
        },
    };
    let read_disc = gen_read_disc(&awaited_sync);
    let read_disc_async = gen_read_disc(&awaited_async);

    let gen_write_arms = |awaited: &proc_macro2::TokenStream| -> Vec<proc_macro2::TokenStream> {
        enum_data
            .variants
            .iter()
            .zip(&discriminants)
            .map(|(variant, disc_tokens)| {
                let variant_name = &variant.ident;
                let write_disc = match endian_attr {
                    AttributeType::LittleEndian => quote! {
                        let disc_val: #repr_ty = #disc_tokens;
                        writer.write_value(&<#repr_ty as #bc::HasEndianRepr>::to_little_endian(disc_val))#awaited?;
                    },
                    AttributeType::BigEndian => quote! {
                        let disc_val: #repr_ty = #disc_tokens;
                        writer.write_value(&<#repr_ty as #bc::HasEndianRepr>::to_big_endian(disc_val))#awaited?;
                    },
                    _ => quote! {
                        let disc_val: #repr_ty = #disc_tokens;
                        writer.write_value(&disc_val)#awaited?;
                    },
                };
                match &variant.fields {
                    Fields::Unit => quote! {
                        #name::#variant_name => { #write_disc }
                    },
                    Fields::Named(named) => {
                        let field_names: Vec<_> = named
                            .named
                            .iter()
                            .map(|f| f.ident.as_ref().unwrap())
                            .collect();
                        let field_writes: Vec<_> = named
                            .named
                            .iter()
                            .map(|f| {
                                gen_enum_field_write(f.ident.as_ref().unwrap(), &f.ty, &f.attrs, &bc, awaited)
                            })
                            .collect();
                        quote! {
                            #name::#variant_name { #(#field_names),* } => {
                                #write_disc
                                #( #field_writes )*
                            }
                        }
                    }
                    Fields::Unnamed(unnamed) => {
                        let field_idents: Vec<_> = (0..unnamed.unnamed.len())
                            .map(|i| Ident::new(&format!("__field_{i}"), name.span()))
                            .collect();
                        let field_writes: Vec<_> = unnamed
                            .unnamed
                            .iter()
                            .zip(&field_idents)
                            .map(|(f, ident)| gen_enum_field_write(ident, &f.ty, &f.attrs, &bc, awaited))
                            .collect();
                        quote! {
                            #name::#variant_name(#(#field_idents),*) => {
                                #write_disc
                                #( #field_writes )*
                            }
                        }
                    }
                }
            })
            .collect()
    };
    let write_arms = gen_write_arms(&awaited_sync);
    let write_arms_async = gen_write_arms(&awaited_async);

    let gen_read_arms = |awaited: &proc_macro2::TokenStream| -> Vec<proc_macro2::TokenStream> {
        enum_data
            .variants
            .iter()
            .zip(&discriminants)
            .map(|(variant, disc_tokens)| {
                let variant_name = &variant.ident;

                match &variant.fields {
                    Fields::Unit => quote! {
                        #disc_tokens => Ok(#name::#variant_name),
                    },
                    Fields::Named(named) => {
                        let field_idents: Vec<_> = named
                            .named
                            .iter()
                            .map(|f| f.ident.as_ref().unwrap())
                            .collect();
                        let field_reads: Vec<_> = named
                            .named
                            .iter()
                            .map(|f| {
                                gen_field_read(
                                    f.ident.as_ref().unwrap(),
                                    &f.ty,
                                    &f.attrs,
                                    &bc,
                                    awaited,
                                )
                            })
                            .collect();
                        quote! {
                            #disc_tokens => {
                                #( #field_reads )*
                                Ok(#name::#variant_name { #(#field_idents),* })
                            }
                        }
                    }
                    Fields::Unnamed(unnamed) => {
                        let field_idents: Vec<_> = (0..unnamed.unnamed.len())
                            .map(|i| Ident::new(&format!("__field_{i}"), name.span()))
                            .collect();
                        let field_reads: Vec<_> = unnamed
                            .unnamed
                            .iter()
                            .zip(&field_idents)
                            .map(|(f, ident)| gen_field_read(ident, &f.ty, &f.attrs, &bc, awaited))
                            .collect();
                        quote! {
                            #disc_tokens => {
                                #( #field_reads )*
                                Ok(#name::#variant_name(#(#field_idents),*))
                            }
                        }
                    }
                }
            })
            .collect()
    };
    let read_arms = gen_read_arms(&awaited_sync);
    let read_arms_async = gen_read_arms(&awaited_async);

    let std_impl = quote! {
        impl #impl_generics #bc::io::Writable for #name #type_generics #where_clause {
            fn write_to(&self, mut writer: &mut (impl ::std::io::Write + ?Sized)) -> ::std::io::Result<()> {
                use #bc::io::WriteValue;
                match self {
                    #(#write_arms)*
                }
                Ok(())
            }
        }

        impl #impl_generics #bc::io::Readable for #name #type_generics #where_clause {
            fn read_from(mut reader: &mut (impl ::std::io::Read + ?Sized)) -> Result<Self, #bc::io::ReadableError> {
                use #bc::io::ReadValue;
                #read_disc
                match disc {
                    #(#read_arms)*
                    _ => Err(#bc::io::ReadableError::DecodeError(#bc::DecodeError::InvalidDiscriminant { raw: disc as u64, type_name: ::core::stringify!(#name) })),
                }
            }
        }
    };
    let eio_impl = quote! {
        impl #impl_generics #bc::eio::EioWritable for #name #type_generics #where_clause {
            fn write_to<__ByteableEioW: #bc::eio::EioWriter + ?Sized>(&self, mut writer: &mut __ByteableEioW) -> Result<(), __ByteableEioW::Error> {
                use #bc::eio::EioWriteValue;
                match self {
                    #(#write_arms)*
                }
                Ok(())
            }
        }

        impl #impl_generics #bc::eio::EioReadable for #name #type_generics #where_clause {
            fn read_from<__ByteableEioR: #bc::eio::EioReader + ?Sized>(mut reader: &mut __ByteableEioR) -> Result<Self, #bc::eio::EioReadableError<__ByteableEioR::Error>> {
                use #bc::eio::EioReadValue;
                #read_disc
                match disc {
                    #(#read_arms)*
                    _ => Err(#bc::eio::EioReadableError::DecodeError(#bc::DecodeError::InvalidDiscriminant { raw: disc as u64, type_name: ::core::stringify!(#name) })),
                }
            }
        }
    };
    let async_impl = quote! {
        impl #impl_generics #bc::async_io::AsyncWritable for #name #type_generics #where_clause {
            fn write_to(&self, mut writer: &mut (impl #bc::__tokio::io::AsyncWriteExt + ?Sized + Unpin)) -> impl ::core::future::Future<Output = ::std::io::Result<()>> {
                async move {
                    use #bc::async_io::AsyncWriteValue;
                    match self {
                        #(#write_arms_async)*
                    }
                    Ok(())
                }
            }
        }

        impl #impl_generics #bc::async_io::AsyncReadable for #name #type_generics #where_clause {
            fn read_from(mut reader: &mut (impl #bc::__tokio::io::AsyncReadExt + ?Sized + Unpin)) -> impl ::core::future::Future<Output = Result<Self, #bc::io::ReadableError>> {
                async move {
                    use #bc::async_io::AsyncReadValue;
                    #read_disc_async
                    match disc {
                        #(#read_arms_async)*
                        _ => Err(#bc::io::ReadableError::DecodeError(#bc::DecodeError::InvalidDiscriminant { raw: disc as u64, type_name: ::core::stringify!(#name) })),
                    }
                }
            }
        }
    };
    let eio_async_impl = quote! {
        impl #impl_generics #bc::eio_async::EioAsyncWritable for #name #type_generics #where_clause {
            fn write_to<__ByteableEioAsyncW: #bc::eio_async::EioAsyncWriter + ?Sized>(&self, mut writer: &mut __ByteableEioAsyncW) -> impl ::core::future::Future<Output = Result<(), __ByteableEioAsyncW::Error>> {
                async move {
                    use #bc::eio_async::EioAsyncWriteValue;
                    match self {
                        #(#write_arms_async)*
                    }
                    Ok(())
                }
            }
        }

        impl #impl_generics #bc::eio_async::EioAsyncReadable for #name #type_generics #where_clause {
            fn read_from<__ByteableEioAsyncR: #bc::eio_async::EioAsyncReader + ?Sized>(mut reader: &mut __ByteableEioAsyncR) -> impl ::core::future::Future<Output = Result<Self, #bc::eio::EioReadableError<__ByteableEioAsyncR::Error>>> {
                async move {
                    use #bc::eio_async::EioAsyncReadValue;
                    #read_disc_async
                    match disc {
                        #(#read_arms_async)*
                        _ => Err(#bc::eio::EioReadableError::DecodeError(#bc::DecodeError::InvalidDiscriminant { raw: disc as u64, type_name: ::core::stringify!(#name) })),
                    }
                }
            }
        }
    };

    dynamic_pipeline_impls(&name, std_impl, eio_impl, async_impl, eio_async_impl).into()
}

/// Reinterprets a resolved `i128` tag value as the bit pattern it would have if cast to
/// `repr_ty` (e.g. `-10i128` reinterpreted as `u8` is `246`), and returns that value as a bare
/// integer literal token stream, of the correct sign for `repr_ty`.
///
/// This has to work both as a plain expression (the write path: `let disc_val: #repr_ty =
/// #disc_tokens;`) and as a match pattern (the read path: `#disc_tokens => Ok(...)`). The
/// obvious approach - emit the raw tag literal plus a `#lit as #repr_ty` cast - only works as an
/// expression: ordinary match patterns don't accept arbitrary expressions, only literals/consts,
/// and while an inline `const { .. }` block is stable as an *expression* since Rust 1.79, using
/// one as a match *pattern* is still gated behind the unstable `inline_const_pat` feature
/// (confirmed here: rustc 1.97 stable rejects it with "const blocks cannot be used as
/// patterns"). So instead the cast is performed here, at macro-expansion time, using real
/// same-width integer types to get the exact two's-complement truncation `as` would produce,
/// and the *result* is emitted as a plain literal - which is valid as both an expression and a
/// pattern, and needs no cast at the use site.
fn reinterpret_tag_for_repr(value: i128, repr_ty: &syn::Ident) -> proc_macro2::TokenStream {
    let repr_str = repr_ty.to_string();
    let reinterpreted = attrs::reinterpret_i128_for_repr(value, &repr_str);
    let lit = if repr_str.starts_with('u') {
        proc_macro2::Literal::u128_unsuffixed(reinterpreted as u128)
    } else {
        proc_macro2::Literal::i128_unsuffixed(reinterpreted)
    };
    quote! { #lit }
}

/// Computes discriminant token streams for every variant, purely from `#[byteable(tag = N)]`
/// attributes (via `attrs::resolve_variant_tags`) - Rust's own real `= N` discriminant syntax
/// is never read. An unannotated variant continues from the previous *resolved* tag value + 1,
/// starting at 0, mirroring Rust's own auto-increment rule but over signed `i128`. Collisions
/// between resolved values are a hard compile-time error (`panic!`), as is any resolved value
/// that doesn't fit losslessly in `repr_ty` (`attrs::validate_tags_fit_width`) - e.g.
/// `#[byteable(tag = 999)]` on a `u8`-width enum. Each resolved value is
/// converted to a bare literal of the wire (`repr_ty`) type via `reinterpret_tag_for_repr`, so
/// the same token stream can be used as either an expression or a match pattern.
fn resolve_enum_discriminants(
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::Token![,]>,
    repr_ty: &syn::Ident,
) -> Vec<proc_macro2::TokenStream> {
    let variant_tags: Vec<Option<i128>> = variants
        .iter()
        .map(|v| attrs::parse_variant_tag(&v.attrs))
        .collect();
    let resolved = match attrs::resolve_variant_tags(&variant_tags) {
        Ok(resolved) => resolved,
        Err(e) => panic!("{e}"),
    };

    if let Err(e) = attrs::validate_tags_fit_width(&resolved, &repr_ty.to_string()) {
        panic!("{e}");
    }

    resolved
        .into_iter()
        .map(|value| reinterpret_tag_for_repr(value, repr_ty))
        .collect()
}

fn unit_enum_derive(input: DeriveInput) -> proc_macro::TokenStream {
    let bc = byteable_crate_path();
    let Data::Enum(enum_data) = &input.data else {
        unreachable!();
    };
    let enum_name = &input.ident;

    let repr_ty = attrs::parse_discriminant_override(&input.attrs)
        .or_else(|| extract_repr_type(&input.attrs))
        .unwrap_or_else(|| {
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

    let endian_attr = parse_byteable_attr(&input.attrs);
    let discriminants = resolve_enum_discriminants(&enum_data.variants, &repr_ty);

    let from_discriminant_arms =
        enum_data
            .variants
            .iter()
            .zip(&discriminants)
            .map(|(variant, disc)| {
                let variant_name = &variant.ident;
                quote! { #disc => Ok(#enum_name::#variant_name), }
            });

    // `*self as #repr_ty` would require moving `Self` out of the `&self` reference (it's not
    // necessarily `Copy`), which rustc rejects with E0507 - and does so unconditionally, not
    // just for empty enums; there's no special-cased discriminant-read lowering for fieldless
    // enum casts. Matching on `*self` instead never binds or moves anything (every variant is
    // a unit variant), and works uniformly down to zero variants (`match *self {}` is accepted
    // as exhaustive for an uninhabited enum, unlike `match self {}`, since a reference type is
    // always considered inhabited by the exhaustiveness checker).
    let to_raw_arms = enum_data
        .variants
        .iter()
        .zip(&discriminants)
        .map(|(variant, disc)| {
            let variant_name = &variant.ident;
            quote! { #enum_name::#variant_name => #disc, }
        });
    let to_raw_expr = quote! {
        match *self {
            #(#to_raw_arms)*
        }
    };

    // For an empty enum, `to_raw_expr` (`match *self {}`) has type `!` - it already coerces to
    // `Self::ByteArray` on its own. Binding it to `v` first and then converting `v` would leave
    // the conversion as dead code after a diverging `let`, which rustc flags as an
    // `unreachable_code` warning. So skip the intermediate binding entirely in that case; the
    // endian attribute is moot too, since there's no value to convert either way.
    let to_byte_array_body = if enum_data.variants.is_empty() {
        to_raw_expr.clone()
    } else {
        match endian_attr {
            AttributeType::LittleEndian => quote! {
                let v: #repr_ty = #to_raw_expr;
                <#repr_ty as #bc::HasEndianRepr>::to_little_endian(v).to_byte_array()
            },
            AttributeType::BigEndian => quote! {
                let v: #repr_ty = #to_raw_expr;
                <#repr_ty as #bc::HasEndianRepr>::to_big_endian(v).to_byte_array()
            },
            _ => quote! {
                let v: #repr_ty = #to_raw_expr;
                <#repr_ty as #bc::ToByteArray>::to_byte_array(&v)
            },
        }
    };

    let try_from_byte_array_body = match endian_attr {
        AttributeType::LittleEndian => quote! {
            let le = <<#repr_ty as #bc::HasEndianRepr>::LE as #bc::FromByteArray>::from_byte_array(byte_array);
            let raw = <#repr_ty as #bc::FromEndianRepr>::from_little_endian(le);
            <Self as #bc::TryFromRawRepr>::try_from_raw(raw)
        },
        AttributeType::BigEndian => quote! {
            let be = <<#repr_ty as #bc::HasEndianRepr>::BE as #bc::FromByteArray>::from_byte_array(byte_array);
            let raw = <#repr_ty as #bc::FromEndianRepr>::from_big_endian(be);
            <Self as #bc::TryFromRawRepr>::try_from_raw(raw)
        },
        _ => quote! {
            let raw = <#repr_ty as #bc::FromByteArray>::from_byte_array(byte_array);
            <Self as #bc::TryFromRawRepr>::try_from_raw(raw)
        },
    };

    quote! {
        impl #bc::RawRepr for #enum_name {
            type Raw = #repr_ty;
            fn to_raw(&self) -> #repr_ty {
                #to_raw_expr
            }
        }

        impl #bc::TryFromRawRepr for #enum_name {
            fn try_from_raw(raw: Self::Raw) -> Result<Self, #bc::DecodeError> {
                match raw {
                    #(#from_discriminant_arms)*
                    _ => Err(#bc::DecodeError::InvalidDiscriminant { raw: raw as u64, type_name: ::core::stringify!(#enum_name) })
                }
            }
        }

        impl #bc::ToByteArray for #enum_name {
            type ByteArray = [u8; ::core::mem::size_of::<#repr_ty>()];
            fn to_byte_array(&self) -> Self::ByteArray {
                #to_byte_array_body
            }
        }

        impl #bc::TryFromByteArray for #enum_name {
            fn try_from_byte_array(byte_array: Self::ByteArray) -> Result<Self, #bc::DecodeError> {
                #try_from_byte_array_body
            }
        }

    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn parse_byteable_attr_ignores_order_token() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(order = 0, big_endian)])];
        assert_eq!(parse_byteable_attr(&attrs), AttributeType::BigEndian);
    }

    #[test]
    fn parse_byteable_attr_scans_every_attribute_instance() {
        let attrs: Vec<syn::Attribute> = vec![
            parse_quote!(#[byteable(order = 2)]),
            parse_quote!(#[byteable(big_endian)]),
        ];
        assert_eq!(parse_byteable_attr(&attrs), AttributeType::BigEndian);
    }

    #[test]
    fn parse_byteable_attr_still_rejects_unknown_tokens() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(not_a_real_attribute)])];
        let result = std::panic::catch_unwind(|| parse_byteable_attr(&attrs));
        assert!(result.is_err());
    }
}
