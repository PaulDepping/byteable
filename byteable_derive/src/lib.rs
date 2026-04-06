use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Ident, Meta, Type, parse_macro_input};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttributeType {
    LittleEndian,
    BigEndian,
    TryTransparent,
    IoOnly,
    None,
}

fn parse_byteable_attr(attrs: &[syn::Attribute]) -> AttributeType {
    for attr in attrs {
        if attr.path().is_ident("byteable") {
            if let Meta::List(meta_list) = &attr.meta {
                return match meta_list.tokens.to_string().as_str() {
                    "little_endian" => AttributeType::LittleEndian,
                    "big_endian" => AttributeType::BigEndian,
                    "transparent" => AttributeType::None,
                    "try_transparent" => AttributeType::TryTransparent,
                    "io_only" => AttributeType::IoOnly,
                    other => panic!(
                        "Unknown byteable attribute: {other}. \
                         Valid attributes are: little_endian, big_endian, try_transparent, io_only"
                    ),
                };
            }
            panic!(
                "Unknown byteable attribute. \
                 Valid attributes are: little_endian, big_endian, try_transparent, io_only"
            );
        }
    }
    AttributeType::None
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
///   [`TryFromRawRepr`], [`IntoByteArray`], and [`FromByteArray`] or [`TryFromByteArray`].
///   A hidden `#[repr(C, packed)]` raw struct is created to hold the on-wire layout.
///
/// - **I/O streaming** (`#[byteable(io_only)]` on structs, always for field enums):
///   generates [`Readable`] and [`Writable`], reading/writing fields sequentially.
///
/// - **Unit enums** (all variants are unit): generates [`TryFromRawRepr`],
///   [`IntoByteArray`], and [`TryFromByteArray`] using an automatically-chosen
///   discriminant integer type (`u8` → `u16` → `u32` → `u64` based on variant count).
///
/// [`RawRepr`]: byteable::RawRepr
/// [`FromRawRepr`]: byteable::FromRawRepr
/// [`TryFromRawRepr`]: byteable::TryFromRawRepr
/// [`IntoByteArray`]: byteable::IntoByteArray
/// [`FromByteArray`]: byteable::FromByteArray
/// [`TryFromByteArray`]: byteable::TryFromByteArray
/// [`Readable`]: byteable::Readable
/// [`Writable`]: byteable::Writable
///
/// # Struct-level attributes
///
/// Place these on the struct or enum itself:
///
/// | Attribute | Effect |
/// |-----------|--------|
/// | `#[byteable(little_endian)]` | All multi-byte fields use little-endian representation |
/// | `#[byteable(big_endian)]` | All multi-byte fields use big-endian representation |
/// | `#[byteable(io_only)]` | Generate `Readable`/`Writable` instead of fixed-size traits |
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
///
/// # Examples
///
/// ## Basic fixed-size struct
///
/// ```rust
/// use byteable::{Byteable, IntoByteArray, TryFromByteArray};
///
/// #[derive(Byteable)]
/// struct Point {
///     x: f32,
///     y: f32,
/// }
///
/// let p = Point { x: 1.0, y: 2.0 };
/// let bytes = p.into_byte_array();
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
/// use byteable::{Byteable, Writable, Readable};
/// use byteable::io::{WriteValue, ReadValue};
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
/// use byteable::{Byteable, IntoByteArray, TryFromByteArray};
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
/// let bytes = Color::Green.into_byte_array();
/// assert_eq!(Color::try_from_byte_array(bytes).unwrap(), Color::Green);
/// ```
///
/// ## Field enum
///
/// ```rust
/// use byteable::{Byteable, Readable, Writable};
/// use byteable::io::{WriteValue, ReadValue};
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
) -> proc_macro2::TokenStream {
    match parse_byteable_attr(attrs) {
        AttributeType::LittleEndian => quote! {
            writer.write_value(&<#field_type as #bc::HasEndianRepr>::to_little_endian(#field_access))?;
        },
        AttributeType::BigEndian => quote! {
            writer.write_value(&<#field_type as #bc::HasEndianRepr>::to_big_endian(#field_access))?;
        },
        AttributeType::None => quote! { writer.write_value(&#field_access)?; },
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
) -> proc_macro2::TokenStream {
    match parse_byteable_attr(attrs) {
        AttributeType::LittleEndian => {
            quote! { let #field_ident: #field_ty = reader.read_value::<<#field_ty as #bc::HasEndianRepr>::LE>()?.get(); }
        }
        AttributeType::BigEndian => {
            quote! { let #field_ident: #field_ty = reader.read_value::<<#field_ty as #bc::HasEndianRepr>::BE>()?.get(); }
        }
        AttributeType::None => quote! { let #field_ident: #field_ty = reader.read_value()?; },
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

            impl #bc::IntoByteArray for #raw_name
                where #raw_name : #bc::PlainOldData
            {
                type ByteArray = [u8; ::core::mem::size_of::<Self>()];
                fn into_byte_array(&self) -> Self::ByteArray {
                    #[allow(unnecessary_transmutes)]
                    unsafe { ::core::mem::transmute(*self) }
                }
            }

            impl #bc::FromByteArray for #raw_name
                where #raw_name : #bc::PlainOldData
            {
                fn from_byte_array(byte_array: <Self as #bc::IntoByteArray>::ByteArray) -> Self {
                    #[allow(unnecessary_transmutes)]
                    unsafe { ::core::mem::transmute(byte_array) }

                }
            }

            impl #bc::IntoByteArray for #name
            where
                #name: #bc::RawRepr,
                <#name as #bc::RawRepr>::Raw: #bc::IntoByteArray,
            {
                type ByteArray = <<Self as #bc::RawRepr>::Raw as #bc::IntoByteArray>::ByteArray;

                fn into_byte_array(&self) -> Self::ByteArray {
                    <Self as #bc::RawRepr>::to_raw(self).into_byte_array()
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

    let write_stmts: Vec<_> = fields
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
            gen_struct_field_write(&field_access, &field.ty, &field.attrs, &bc)
        })
        .collect();

    let (read_bindings, construct_expr): (Vec<_>, proc_macro2::TokenStream) = if is_tuple {
        let idents: Vec<_> = (0..fields.len())
            .map(|i| syn::Ident::new(&format!("__field_{i}"), name.span()))
            .collect();
        let bindings = fields
            .iter()
            .zip(&idents)
            .map(|(f, id)| gen_field_read(id, &f.ty, &f.attrs, &bc))
            .collect();
        (bindings, quote! { Ok(Self(#(#idents),*)) })
    } else {
        let field_idents: Vec<_> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
        let bindings = fields
            .iter()
            .map(|f| gen_field_read(f.ident.as_ref().unwrap(), &f.ty, &f.attrs, &bc))
            .collect();
        (bindings, quote! { Ok(Self { #(#field_idents),* }) })
    };

    quote! {
        impl #impl_generics #bc::Readable for #name #type_generics #where_clause {
            fn read_from(mut reader: &mut (impl ::std::io::Read + ?Sized)) -> Result<Self, #bc::ReadableError> {
                use #bc::ReadValue;
                #( #read_bindings )*
                #construct_expr
            }
        }
        impl #impl_generics #bc::Writable for #name #type_generics #where_clause {
            fn write_to(&self, mut writer: &mut (impl ::std::io::Write + ?Sized)) -> ::std::io::Result<()> {
                use #bc::WriteValue;
                #( #write_stmts )*
                Ok(())
            }
        }
    }.into()
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

            impl #bc::IntoByteArray for #raw_name
                where #raw_name : #bc::PlainOldData
            {
                type ByteArray = [u8; ::core::mem::size_of::<Self>()];
                fn into_byte_array(&self) -> Self::ByteArray {
                    #[allow(unnecessary_transmutes)]
                    unsafe { ::core::mem::transmute(*self) }
                }
            }

            impl #bc::FromByteArray for #raw_name
                where #raw_name : #bc::PlainOldData
            {
                fn from_byte_array(byte_array: <Self as #bc::IntoByteArray>::ByteArray) -> Self {
                    #[allow(unnecessary_transmutes)]
                    unsafe { ::core::mem::transmute(byte_array) }

                }
            }

            impl #bc::IntoByteArray for #original_name
            where
                #original_name: #bc::RawRepr,
                <#original_name as #bc::RawRepr>::Raw: #bc::IntoByteArray,
            {
                type ByteArray = [u8; ::core::mem::size_of::<<Self as #bc::RawRepr>::Raw>()];
                fn into_byte_array(&self) -> Self::ByteArray {
                    <Self as #bc::RawRepr>::to_raw(self).into_byte_array()
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
            let idx = syn::Index::from(i);
            match attr {
                AttributeType::LittleEndian => FieldInfo {
                    raw_field_def: quote! { #vis <#field_type as #bc::HasEndianRepr>::LE },
                    to_raw_expr: quote! { <#field_type as #bc::HasEndianRepr>::to_little_endian(self.#idx) },
                    from_raw_expr: quote! { <#field_type as #bc::FromEndianRepr>::from_little_endian(value.#idx) },
                },
                AttributeType::BigEndian => FieldInfo {
                    raw_field_def: quote! { #vis <#field_type as #bc::HasEndianRepr>::BE },
                    to_raw_expr: quote! { <#field_type as #bc::HasEndianRepr>::to_big_endian(self.#idx) },
                    from_raw_expr: quote! { <#field_type as #bc::FromEndianRepr>::from_big_endian(value.#idx) },
                },
                AttributeType::TryTransparent => FieldInfo {
                    raw_field_def: quote! { #vis <#field_type as #bc::RawRepr>::Raw },
                    to_raw_expr: quote! { <#field_type as #bc::RawRepr>::to_raw(&self.#idx) },
                    from_raw_expr: quote! { <#field_type as #bc::TryFromRawRepr>::try_from_raw(value.#idx)? },
                },
                AttributeType::IoOnly => panic!(
                    "#[byteable(io_only)] is a struct-level attribute and cannot be used on individual fields"
                ),
                AttributeType::None => FieldInfo {
                    raw_field_def: quote! { #vis <#field_type as #bc::RawRepr>::Raw },
                    to_raw_expr: quote! { <#field_type as #bc::RawRepr>::to_raw(&self.#idx) },
                    from_raw_expr: quote! { <#field_type as #bc::FromRawRepr>::from_raw(value.#idx) },
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

    let raw_struct_def = {
        let field_defs = field_infos.iter().map(|v| &v.raw_field_def);
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

            impl #bc::IntoByteArray for #raw_name
                where #raw_name : #bc::PlainOldData
            {
                type ByteArray = [u8; ::core::mem::size_of::<Self>()];
                fn into_byte_array(&self) -> Self::ByteArray {
                    #[allow(unnecessary_transmutes)]
                    unsafe { ::core::mem::transmute(*self) }
                }
            }

            impl #bc::FromByteArray for #raw_name
                where #raw_name : #bc::PlainOldData
            {
                fn from_byte_array(byte_array: <Self as #bc::IntoByteArray>::ByteArray) -> Self {
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
        let to_raw_exprs = field_infos.iter().map(|v| &v.to_raw_expr);
        if is_tuple {
            quote! {
                impl #bc::RawRepr for #original_name {
                    type Raw = #raw_name;

                    #[inline]
                    fn to_raw(&self) -> Self::Raw {
                        #raw_name (#(#to_raw_exprs),*)
                    }
                }

                impl #bc::IntoByteArray for #original_name
                where
                    #original_name: #bc::RawRepr,
                    <#original_name as #bc::RawRepr>::Raw: #bc::IntoByteArray,
                {
                    type ByteArray = <<Self as #bc::RawRepr>::Raw as #bc::IntoByteArray>::ByteArray;
                    fn into_byte_array(&self) -> Self::ByteArray {
                        <Self as #bc::RawRepr>::to_raw(self).into_byte_array()
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

                impl #bc::IntoByteArray for #original_name
                where
                    #original_name: #bc::RawRepr,
                    <#original_name as #bc::RawRepr>::Raw: #bc::IntoByteArray,
                {
                    type ByteArray = <<Self as #bc::RawRepr>::Raw as #bc::IntoByteArray>::ByteArray;
                    fn into_byte_array(&self) -> Self::ByteArray {
                        <Self as #bc::RawRepr>::to_raw(self).into_byte_array()
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
) -> proc_macro2::TokenStream {
    match parse_byteable_attr(attrs) {
        AttributeType::LittleEndian => quote! {
            writer.write_value(&<#field_type as #bc::HasEndianRepr>::to_little_endian(*#field_ident))?;
        },
        AttributeType::BigEndian => quote! {
            writer.write_value(&<#field_type as #bc::HasEndianRepr>::to_big_endian(*#field_ident))?;
        },
        AttributeType::None => quote! {
            writer.write_value(#field_ident)?;
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

    // Determine repr type — use explicit #[repr(...)] if present, otherwise auto-select.
    let repr_ty = extract_repr_type(&input.attrs).unwrap_or_else(|| {
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
    let discriminants = compute_discriminants(&enum_data.variants);

    let read_disc = match endian_attr {
        AttributeType::LittleEndian => quote! {
            let disc: #repr_ty = <#repr_ty as #bc::FromEndianRepr>::from_little_endian(
                reader.read_value::<<#repr_ty as #bc::HasEndianRepr>::LE>()?
            );
        },
        AttributeType::BigEndian => quote! {
            let disc: #repr_ty = <#repr_ty as #bc::FromEndianRepr>::from_big_endian(
                reader.read_value::<<#repr_ty as #bc::HasEndianRepr>::BE>()?
            );
        },
        _ => quote! {
            let disc: #repr_ty = reader.read_value()?;
        },
    };

    let write_arms = enum_data
        .variants
        .iter()
        .zip(&discriminants)
        .map(|(variant, disc_tokens)| {
            let variant_name = &variant.ident;
            let write_disc = match endian_attr {
                AttributeType::LittleEndian => quote! {
                    let disc_val: #repr_ty = #disc_tokens;
                    writer.write_value(&<#repr_ty as #bc::HasEndianRepr>::to_little_endian(disc_val))?;
                },
                AttributeType::BigEndian => quote! {
                    let disc_val: #repr_ty = #disc_tokens;
                    writer.write_value(&<#repr_ty as #bc::HasEndianRepr>::to_big_endian(disc_val))?;
                },
                _ => quote! {
                    let disc_val: #repr_ty = #disc_tokens;
                    writer.write_value(&disc_val)?;
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
                            gen_enum_field_write(f.ident.as_ref().unwrap(), &f.ty, &f.attrs, &bc)
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
                        .map(|(f, ident)| gen_enum_field_write(ident, &f.ty, &f.attrs, &bc))
                        .collect();
                    quote! {
                        #name::#variant_name(#(#field_idents),*) => {
                            #write_disc
                            #( #field_writes )*
                        }
                    }
                }
            }
        });
    let read_arms = enum_data
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
                        .map(|f| gen_field_read(f.ident.as_ref().unwrap(), &f.ty, &f.attrs, &bc))
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
                        .map(|(f, ident)| gen_field_read(ident, &f.ty, &f.attrs, &bc))
                        .collect();
                    quote! {
                        #disc_tokens => {
                            #( #field_reads )*
                            Ok(#name::#variant_name(#(#field_idents),*))
                        }
                    }
                }
            }
        });
    quote! {
        impl #impl_generics #bc::Writable for #name #type_generics #where_clause {
            fn write_to(&self, mut writer: &mut (impl ::std::io::Write + ?Sized)) -> ::std::io::Result<()> {
                use #bc::WriteValue;
                match self {
                    #(#write_arms)*
                }
                Ok(())
            }
        }


        impl #impl_generics #bc::Readable for #name #type_generics #where_clause {
            fn read_from(mut reader: &mut (impl ::std::io::Read + ?Sized)) -> Result<Self, #bc::ReadableError> {
                use #bc::ReadValue;
                #read_disc
                match disc {
                    #(#read_arms)*
                    _ => Err(#bc::ReadableError::DecodeError(#bc::DecodeError::InvalidDiscriminant { raw: disc as u64, type_name: ::core::stringify!(#name) })),
                }
            }
        }
    }.into()
}

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
                let (prefix, rest) =
                    if let Some(r) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
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

fn unit_enum_derive(input: DeriveInput) -> proc_macro::TokenStream {
    let bc = byteable_crate_path();
    let Data::Enum(enum_data) = &input.data else {
        unreachable!();
    };
    let enum_name = &input.ident;

    let repr_ty = extract_repr_type(&input.attrs).unwrap_or_else(|| {
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
    let discriminants = compute_discriminants(&enum_data.variants);

    let from_discriminant_arms =
        enum_data
            .variants
            .iter()
            .zip(&discriminants)
            .map(|(variant, disc)| {
                let variant_name = &variant.ident;
                quote! { #disc => Ok(#enum_name::#variant_name), }
            });

    let into_byte_array_body = match endian_attr {
        AttributeType::LittleEndian => quote! {
            let v: #repr_ty = *self as _;
            <#repr_ty as #bc::HasEndianRepr>::to_little_endian(v).into_byte_array()
        },
        AttributeType::BigEndian => quote! {
            let v: #repr_ty = *self as _;
            <#repr_ty as #bc::HasEndianRepr>::to_big_endian(v).into_byte_array()
        },
        _ => quote! {
            let v: #repr_ty = *self as _;
            <#repr_ty as #bc::IntoByteArray>::into_byte_array(&v)
        },
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
                *self as _
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

        impl #bc::IntoByteArray for #enum_name {
            type ByteArray = [u8; ::core::mem::size_of::<#repr_ty>()];
            fn into_byte_array(&self) -> Self::ByteArray {
                #into_byte_array_body
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
