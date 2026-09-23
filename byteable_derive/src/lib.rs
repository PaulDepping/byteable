use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
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

/// The attribute syntax a user would actually type for `kind`, for use in error messages -
/// `{kind:?}` would print the internal Rust variant name (`IoOnly`) instead.
fn attribute_syntax(kind: AttributeType) -> &'static str {
    match kind {
        AttributeType::LittleEndian => "little_endian",
        AttributeType::BigEndian => "big_endian",
        AttributeType::TryTransparent => "try_transparent",
        AttributeType::IoOnly => "io_only",
        AttributeType::None => "transparent",
    }
}

/// Builds the `attrs::resolve_field_wire_order` input for a field list: each field's
/// `#[byteable(order = ..)]` value and the span a misconfiguration error about *that* field
/// should point at - the order literal's own span when present, else the field's own span (so
/// e.g. "must annotate every field or none" can point at the field that's missing it).
fn field_order_entries<'a>(
    fields: impl IntoIterator<Item = &'a syn::Field>,
) -> Vec<attrs::FieldOrderEntry> {
    fields
        .into_iter()
        .map(|f| match attrs::parse_field_order(&f.attrs) {
            Some((order, span)) => attrs::FieldOrderEntry {
                order: Some(order),
                span,
            },
            None => attrs::FieldOrderEntry {
                order: None,
                span: f.span(),
            },
        })
        .collect()
}

/// Scans `attrs` for a `#[byteable(..)]` attribute and returns which of the mutually-exclusive
/// endian/transparent/io_only kinds is present (`AttributeType::None` if none is), together with
/// the span of the specific token that decided it - `meta.path`'s span for a matched kind, or
/// `Span::call_site()` when nothing matched at all (callers only use the span when a specific
/// `AttributeType` is misused, and `None` is never itself a misuse).
///
/// Every hard error here - unknown attribute, conflicting kinds, a malformed `fingerprint = ..`
/// value or a missing `=` on any of `order`/`discriminant`/`fingerprint` - panics via
/// [`std::panic::panic_any`] with the `syn::Error` payload rather than `panic!`-ing a bare
/// string: the top-level `byteable_derive_macro` entry point is the only place in this crate
/// that catches these panics, and it downcasts the payload back to a `syn::Error` so the
/// resulting `compile_error!` is anchored at the actual offending token instead of the derive
/// site (see `byteable_derive_macro`'s doc comment for the whole scheme).
fn parse_byteable_attr(attrs: &[syn::Attribute]) -> (AttributeType, Span) {
    let mut found: Option<(AttributeType, Span)> = None;
    let mut fingerprint_seen = false;
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
            } else if meta.path.is_ident("fingerprint") {
                // Consumed by `attrs::parse_fingerprint_assertion`, but validated eagerly
                // right here too (both the value and duplicate-detection): this whole
                // `attr.parse_nested_meta` call already aborts on any `Err` (see below), while
                // `parse_fingerprint_assertion`'s own error handling silently discards a
                // malformed/duplicate value (matching `parse_field_order`'s and
                // `parse_discriminant_override`'s pre-fix swallow-on-error style). Without
                // validating here, a typo like `fingerprint = "0x7O26af.."` would silently
                // disable the assertion forever instead of failing to compile - the worst
                // possible failure mode for what's meant to be a safety check.
                if fingerprint_seen {
                    return Err(meta.error("duplicate #[byteable(fingerprint = ..)]"));
                }
                fingerprint_seen = true;
                if meta.input.peek(syn::Token![=]) {
                    let lit: syn::LitStr = meta.value()?.parse()?;
                    attrs::parse_fingerprint_value(&lit.value()).map_err(|_| {
                        meta.error(
                            "expected a fingerprint value: hex with a `0x` prefix \
                             (\"0x1234\") or plain decimal (\"4660\")",
                        )
                    })?;
                }
                None
            } else {
                let name = meta
                    .path
                    .get_ident()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "<path>".to_string());
                return Err(meta.error(format!(
                    "unknown byteable attribute `{name}`; valid attributes are: little_endian, \
                     big_endian, transparent, try_transparent, io_only, order, discriminant, \
                     fingerprint"
                )));
            };
            if let Some(kind) = kind {
                if found.is_some() {
                    return Err(meta.error(
                        "at most one of little_endian/big_endian/transparent/try_transparent/\
                         io_only may be specified",
                    ));
                }
                found = Some((kind, meta.path.span()));
            }
            Ok(())
        })
        .unwrap_or_else(|e| std::panic::panic_any(e));
    }
    found.unwrap_or((AttributeType::None, Span::call_site()))
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
        std::panic::panic_any(syn::Error::new(
            type_name.span(),
            format!(
                "deriving Byteable on `{type_name}` needs the io_only/field-enum dynamic \
                 pipeline, which requires the `std` and/or `embedded-io` feature of `byteable` \
                 to be enabled - neither is active for this build"
            ),
        ));
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
/// | `#[byteable(little_endian)]`/`#[byteable(big_endian)]` (enums only) | Sets the enum's discriminant endianness. Applies only to the discriminant, never to variant-carried fields (those use their own field-level attribute, same as struct fields). Using either on a struct is a compile error - see "Field-level attributes" below. |
/// | `#[byteable(io_only)]` | Generate `Readable`/`Writable`/`EioReadable`/`EioWritable`/`AsyncReadable`/`AsyncWritable`/`EioAsyncReadable`/`EioAsyncWritable` (per enabled feature) instead of fixed-size traits |
/// | `#[byteable(discriminant = uN/iN)]` (enums only) | Overrides the wire width of the enum's discriminant with `uN`/`iN` (`u8`/`u16`/`u32`/`u64`/`u128`/`i8`/`i16`/`i32`/`i64`/`i128`), independently of any `#[repr(...)]` on the enum. |
/// | `#[byteable(fingerprint)]` / `#[byteable(fingerprint = "0x...")]` | Asserts at compile time that this type's wire shape still hashes to the given value. See "Pinning the wire format" below. Type-level only - on a field or variant it is a compile error, since there would be nothing for it to assert. |
///
/// ## Pinning the wire format: `#[byteable(fingerprint = "...")]`
///
/// Every derived type also gets a [`WireFingerprint`] impl: a `u64` computed at compile time
/// from the type's exact wire shape - field types and their order, endianness, enum
/// discriminant width/signedness/endianness, and every variant's tag and payload. Two types
/// that encode and decode identical bytes hash identically; any change that moves a byte
/// changes the hash.
///
/// `#[byteable(fingerprint = "...")]` turns that into an assertion. If the type's wire shape
/// ever drifts - a reordered field, a widened integer, a renumbered variant - the build fails
/// instead of silently shipping an incompatible format:
///
/// ```rust
/// use byteable::Byteable;
///
/// #[derive(Byteable)]
/// #[byteable(fingerprint = "0xa67a58f8363edba2")]
/// struct Checked {
///     a: u8,
/// }
/// ```
///
/// **The value may be written as `0x`-prefixed hex or as plain decimal.** Both are accepted
/// on purpose: the mismatch diagnostic prints the actual fingerprint in *decimal* (that comes
/// out of rustc's own const-generic rendering and can't be reformatted) and tells you to
/// replace the asserted value with it, so pasting that number straight back in has to work.
/// With a `0x` prefix the value is read as hex; without one, as decimal.
///
/// To get the value in the first place, use the bare form. `#[byteable(fingerprint)]` asserts
/// against `0`, which no real type hashes to, so the first build fails and the error names the
/// real value - ready to copy into the attribute:
///
/// ```text
/// error[E0277]: byteable: wire-format fingerprint mismatch (asserted 0, actual 11995998380539960226)
///   = note: if this change was intentional, replace the asserted value with 11995998380539960226
/// ```
///
/// ### Generic types must bound `WireFingerprint` themselves
///
/// The generated `impl WireFingerprint for YourType<T>` reuses your type's own `where` clause
/// verbatim; the derive macro cannot add bounds to it. So **a generic type with a field or
/// variant whose type needs `WireFingerprint` must add that bound itself**:
///
/// ```rust
/// use byteable::{Byteable, TryFromRawRepr, WireFingerprint};
///
/// #[derive(Byteable)]
/// #[byteable(io_only)]
/// struct Envelope<T>
/// where
///     T: TryFromRawRepr + WireFingerprint, // `WireFingerprint` is the part you must add
/// {
///     id: u32,
///     payload: T,
/// }
/// ```
///
/// Without the `T: WireFingerprint` bound, the derive produces a `WireFingerprint` impl whose
/// body names `T` unbounded, and the type fails to compile. This applies to every generic
/// derived type, whether or not it uses `#[byteable(fingerprint = ..)]`.
///
/// [`WireFingerprint`]: byteable::WireFingerprint
///
/// # Field-level attributes
///
/// Place these on individual fields or enum variants:
///
/// | Attribute | Effect |
/// |-----------|--------|
/// | `#[byteable(little_endian)]` | This field uses little-endian. Unannotated multi-byte fields are little-endian by default already (via the field type's own `RawRepr`), so this is only needed to opt into big-endian, or to be explicit. |
/// | `#[byteable(big_endian)]` | This field uses big-endian. |
/// | `#[byteable(try_transparent)]` | Field decode may fail; the struct impl becomes `TryFromRawRepr` |
/// | `#[byteable(order = N)]` (struct fields only) | Pins this field's *wire position* to `N`, independently of its declaration order in the struct. Adds zero bytes to the wire format - it only changes which byte range a given field occupies. Must annotate either every field of the struct or none; the `N` values across all fields must form a dense `0..field_count` permutation. |
///
/// # Variant-level attributes
///
/// Place these on individual enum variants:
///
/// | Attribute | Effect |
/// |-----------|--------|
/// | `#[byteable(tag = <expr>)]` | Pins this variant's *wire discriminant* to `<expr>`, independently of its declaration position and of any real Rust `= N` discriminant (which is never read - see above). `<expr>` can be anything valid in a `const X: repr_ty = <expr>;` position - an integer literal, a negative literal, a named const, a simple arithmetic expression - and is range-checked against `repr_ty` by rustc itself (a value that doesn't fit is a normal "literal out of range"/overflow compile error, not a `byteable`-specific one). Variants without an explicit `tag` continue from the previous variant's resolved value + 1 (or from `0` for the first variant); two variants resolving to the same value is a compile error. |
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
/// Endianness is per-field. There is no struct-level default to inherit from:
/// `#[byteable(big_endian)]`/`#[byteable(little_endian)]` on a *struct* is a compile error,
/// because it would have to reach into nested `Byteable` field types, which have no single
/// top-level endianness of their own. Annotate each multi-byte field you want to change.
///
/// ```rust
/// use byteable::Byteable;
///
/// #[derive(Byteable)]
/// struct NetworkHeader {
///     #[byteable(big_endian)]
///     magic: u32,
///     flags: u16,   // little-endian - the default for any unannotated multi-byte field
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
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        reject_non_type_level_fingerprint(&input);
        match input.data {
            Data::Struct(_) => struct_derive(input),
            Data::Enum(_) => enum_derive(input),
            Data::Union(data) => std::panic::panic_any(syn::Error::new(
                data.union_token.span(),
                "union types are unsupported",
            )),
        }
    }));
    result.unwrap_or_else(abort_payload_to_compile_error)
}

/// Recovers the `syn::Error` every hard error in this crate panics with (see
/// `parse_byteable_attr`'s doc comment for why) and turns it into `compile_error!` tokens
/// anchored at that error's own span, instead of letting the panic surface as rustc's generic,
/// span-less "proc-macro derive panicked" diagnostic. This is the only `catch_unwind` in the
/// crate, so it's the one place a panic from anywhere in the derive - `parse_byteable_attr`,
/// `attrs::resolve_field_wire_order`, `dynamic_pipeline_impls`, etc. - is turned into a normal
/// compile error.
///
/// A panic that didn't opt into this scheme (a bare `panic!`/`.unwrap()`/`.expect()`, e.g. from
/// a future `syn`/`quote` version) falls back to a `Span::call_site()` compile error carrying
/// whatever message text could be recovered from the payload, rather than re-panicking and
/// losing the diagnostic - and losing rustc's own "1 previous error" bookkeeping with it -
/// entirely.
fn abort_payload_to_compile_error(
    payload: Box<dyn std::any::Any + Send>,
) -> proc_macro::TokenStream {
    let err = match payload.downcast::<syn::Error>() {
        Ok(err) => *err,
        Err(payload) => {
            let message = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "the byteable derive macro panicked".to_string());
            syn::Error::new(Span::call_site(), message)
        }
    };
    err.to_compile_error().into()
}

/// Rejects `#[byteable(fingerprint = ..)]` anywhere other than on the derived type itself.
///
/// The assertion is only ever read from the type's own attributes (by
/// `fingerprint_assertion_impl`, via `attrs::parse_fingerprint_assertion(&input.attrs)`), so
/// the same attribute on a field or variant produced *nothing*: no assertion, no error, no
/// warning. A misplaced attribute silently disabling a safety check is exactly the failure
/// mode `parse_byteable_attr`'s eager hex validation was added to prevent, and it deserves the
/// same treatment.
///
/// This sweeps the whole `DeriveInput` from the one entry point rather than being folded into
/// `parse_byteable_attr`, because `parse_byteable_attr` is never called on *variant*
/// attributes at all (variants only go through `attrs::parse_variant_tag`, which looks for
/// `tag` and nothing else) - a context flag threaded through it would still have left
/// `#[byteable(fingerprint = ..)]` on a variant silently ignored.
fn reject_non_type_level_fingerprint(input: &DeriveInput) {
    fn check(attrs: &[syn::Attribute], location: &str) {
        for attr in attrs {
            if !attr.path().is_ident("byteable") {
                continue;
            }
            // A direct, unconditional panic (rather than `return Err(..)`) is deliberate: the
            // whole call below is `let _ = ..`, so an `Err` from this closure for *this* key
            // would be silently swallowed exactly like a malformed value for any other key is
            // (see the comment below) - that's the right behavior for "somebody else's
            // business to validate", but wrong for a misplaced `fingerprint` attribute, which
            // this function exists specifically to catch.
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("fingerprint") {
                    std::panic::panic_any(syn::Error::new(
                        meta.path.span(),
                        format!(
                            "#[byteable(fingerprint = ..)] is only supported on the type \
                             itself, not on {location}. The assertion describes the whole \
                             type's wire shape; there is nothing it could assert here, and \
                             leaving it in place would silently assert nothing at all. Move it \
                             up to the `struct`/`enum` item."
                        ),
                    ));
                }
                // Every other key is somebody else's business to validate - this pass only
                // consumes its `= <value>` so `parse_nested_meta` can keep scanning the rest
                // of the list. `syn::Expr` covers every value form the crate accepts: a
                // string literal (`fingerprint`), an integer (`order`), a bare type name
                // (`discriminant = u16`, a path expression), and an arbitrary const
                // expression (`tag = 1 + (1u128 << 64)`).
                if meta.input.peek(syn::Token![=]) {
                    meta.value()?.parse::<syn::Expr>()?;
                }
                Ok(())
            });
        }
    }

    fn check_fields(fields: &syn::Fields, location: &str) {
        for field in fields {
            check(&field.attrs, location);
        }
    }

    match &input.data {
        Data::Struct(data) => check_fields(&data.fields, "a struct field"),
        Data::Enum(data) => {
            for variant in &data.variants {
                check(&variant.attrs, "an enum variant");
                check_fields(&variant.fields, "an enum variant's field");
            }
        }
        Data::Union(_) => {}
    }
}

fn struct_derive(input: DeriveInput) -> proc_macro::TokenStream {
    let (attr_kind, attr_span) = parse_byteable_attr(&input.attrs);
    match attr_kind {
        AttributeType::IoOnly => io_struct_derive(input),
        AttributeType::LittleEndian | AttributeType::BigEndian => {
            std::panic::panic_any(syn::Error::new(
                attr_span,
                "#[byteable(little_endian)]/#[byteable(big_endian)] is not supported at the \
             struct level - it would need to reach into nested Byteable types, which don't \
             have a single top-level endianness. Annotate each multi-byte field individually, \
             e.g. `#[byteable(big_endian)] field_name: u32`.",
            ))
        }
        _ => fixed_struct_derived(input),
    }
}

/// Builds the field-type list to fingerprint for a struct's wire-ordered fields: the
/// plain declared type for every field, except a field carrying an explicit
/// `#[byteable(big_endian)]`, which gets `BigEndian<T>` substituted in its place. (There is no
/// struct-level endianness default to account for: `#[byteable(big_endian)]` on a struct is a
/// compile error - see `struct_derive`.) Unattributed and
/// explicit-little-endian fields are deliberately NOT distinguished here - `T::WIRE_FINGERPRINT`
/// already equals `LittleEndian<T>::WIRE_FINGERPRINT` by construction (see
/// `impl_fingerprint_endian_int!`/`impl_fingerprint_endian_float!` in `src/byteable_trait.rs`),
/// so no resolution is needed beyond checking for the big-endian case.
/// `#[byteable(try_transparent)]` also needs no
/// special-casing: it only changes whether the *outer* struct's decode is fallible, not
/// what gets fingerprinted - the field's own type already carries its own Constraint
/// correctly via its own WireFingerprint impl.
fn wire_fingerprint_field_types<'a>(
    bc: &proc_macro2::TokenStream,
    wire_ordered_fields: impl Iterator<Item = &'a syn::Field>,
) -> Vec<proc_macro2::TokenStream> {
    wire_ordered_fields
        .map(|field| {
            let ty = &field.ty;
            match parse_byteable_attr(&field.attrs).0 {
                AttributeType::BigEndian => quote! { <#ty as #bc::HasEndianRepr>::BE },
                _ => quote! { #ty },
            }
        })
        .collect()
}

// NOTE for both `wire_fingerprint_impl_for_struct` below and `wire_fingerprint_impl_for_enum`
// further down: the generated `impl ... WireFingerprint for Name<...> #where_clause` always
// reuses the input type's own `where_clause` verbatim (via `input.generics.split_for_impl()`)
// - it is never widened with an inferred `T: WireFingerprint` bound. So a generic struct/enum
// with a field or variant whose type needs `WireFingerprint` (i.e. anything reachable via
// `.nested::<T>()`) must add that bound itself in its own `where`-clause; the derive macro has
// no way to inject it. Two in-repo test fixtures needed exactly this fix after the fact:
// `GenericIo<T>` in `tests/derive_structs.rs` and `GenericMessage<T>` in
// `tests/derive_field_enums.rs`.
fn wire_fingerprint_impl_for_struct(
    bc: &proc_macro2::TokenStream,
    name: &syn::Ident,
    generics: &syn::Generics,
    field_types: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics #bc::WireFingerprint for #name #type_generics #where_clause {
            const WIRE_FINGERPRINT: u64 = {
                let b = #bc::FingerprintBuilder::new().tag(#bc::FingerprintTag::Struct);
                #( let b = b.nested::<#field_types>(); )*
                b.finish()
            };
        }
    }
}

/// The unsigned integer type of the same width as `repr_ty`, plus whether `repr_ty` itself is
/// signed. `repr_ty` is always one of `u8`..`u128`/`i8`..`i128` by construction (both its
/// sources - `attrs::parse_discriminant_override` and `extract_repr_type` - reject anything
/// else, and the auto-selected fallback is always a `u*`), so the `_` arm is unreachable in
/// practice; it falls back to treating the repr as unsigned rather than panicking, since a
/// wrong fingerprint is strictly better than a macro panic on input that can't occur anyway.
fn discriminant_repr_facts(repr_ty: &syn::Ident) -> (syn::Ident, bool) {
    let name = repr_ty.to_string();
    let signed = name.starts_with('i');
    let unsigned_name = match name.as_str() {
        "i8" => "u8",
        "i16" => "u16",
        "i32" => "u32",
        "i64" => "u64",
        "i128" => "u128",
        other => other,
    };
    (Ident::new(unsigned_name, repr_ty.span()), signed)
}

/// Builds one variant's fingerprint contribution: the `Struct` tag, the variant's wire
/// discriminant, and then each of its fields' own fingerprints.
///
/// The discriminant is folded via `discriminant_tag`, at full `u128` width, not `#tag_ref as
/// u64`. That earlier one-`u64` fold silently truncated `u128`/`i128` discriminants: under
/// `#[byteable(discriminant = u128)]`, tags `{0, 1}` and tags `{0, 1 + 2^64}` produced
/// *identical* fingerprints despite genuinely different bytes on the wire.
///
/// The value is first normalized through the same-width *unsigned* type
/// (`#tag_ref as #unsigned_repr`), so what gets hashed is the discriminant's actual wire bit
/// pattern rather than a sign-extended reinterpretation of it - `-1i8` folds as `0xFF`, which
/// is what the encoder really writes, not as `0xFFFF_FFFF_FFFF_FFFF`. Signedness is not lost
/// by this: it is folded once, explicitly, into the enum shell by
/// `wire_fingerprint_impl_for_enum`, exactly the way every fixed-width integer's own
/// `WireFingerprint` folds its `Signedness`.
fn variant_fingerprint_expr(
    bc: &proc_macro2::TokenStream,
    tag_ref: &proc_macro2::TokenStream,
    unsigned_repr: &syn::Ident,
    field_types: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    quote! {
        {
            let __byteable_tag: u128 = (#tag_ref as #unsigned_repr) as u128;
            #bc::FingerprintBuilder::new()
                .tag(#bc::FingerprintTag::Struct)
                .discriminant_tag(__byteable_tag)
                #( .nested::<#field_types>() )*
                .finish()
        }
    }
}

/// Shared shell for a derived enum's `WireFingerprint` impl, used by both `unit_enum_derive`
/// and `enum_derive` so the two copies can't drift out of sync. Must fold `.endianness(...)` as
/// well as `.width(...)`: two enums differing only in discriminant endianness (e.g.
/// `#[byteable(little_endian)]` vs `#[byteable(big_endian)]` on the same multi-byte
/// discriminant) encode different wire bytes and must fingerprint differently - see
/// `ProtocolLE`/`ProtocolBE` in `tests/derive_enums.rs`. `endian_attr` must be resolved the
/// same way the discriminant's real encode/decode codegen resolves it in the caller (default
/// little-endian when unattributed) - this function does not re-derive it.
///
/// The shell folds, in order: the `Enum` tag, the discriminant's width, its signedness, its
/// endianness, and finally `.variants(...)`, which folds the order-independent combination of
/// the per-variant hashes together with the variant *count* - see `FingerprintBuilder::variants`
/// on why both are folded: `fold_unordered` is a plain `wrapping_add`, so the count is cheap
/// extra collision hardening against two different variant sets summing to the same value.
fn wire_fingerprint_impl_for_enum(
    bc: &proc_macro2::TokenStream,
    enum_name: &syn::Ident,
    generics: &syn::Generics,
    repr_ty: &syn::Ident,
    endian_attr: AttributeType,
    variant_hashes: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let n = variant_hashes.len();
    let endianness = match endian_attr {
        AttributeType::BigEndian => quote! { #bc::Endianness::Big },
        _ => quote! { #bc::Endianness::Little },
    };
    let (_, signed) = discriminant_repr_facts(repr_ty);
    let signedness = if signed {
        quote! { #bc::Signedness::Signed }
    } else {
        quote! { #bc::Signedness::Unsigned }
    };
    quote! {
        impl #impl_generics #bc::WireFingerprint for #enum_name #type_generics #where_clause {
            const WIRE_FINGERPRINT: u64 = {
                let variant_hashes: [u64; #n] = [ #(#variant_hashes),* ];
                #bc::FingerprintBuilder::new()
                    .tag(#bc::FingerprintTag::Enum)
                    .width(<#repr_ty as #bc::ToByteArray>::BYTE_SIZE as u8)
                    .signedness(#signedness)
                    .endianness(#endianness)
                    .variants(&variant_hashes)
                    .finish()
            };
        }
    }
}

/// Builds the `#[byteable(fingerprint = "0x...")]` compile-time assertion for a derived type,
/// shared across `fixed_struct_derived`, `io_struct_derive`, `enum_derive`, and
/// `unit_enum_derive` (six call sites total, counting each function's unit/non-unit or
/// field/fieldless branch separately) so they can't drift out of sync the way independently
/// duplicated fingerprint logic can (see `wire_fingerprint_impl_for_enum`'s own doc comment).
/// Returns `None` when the attribute
/// is absent - every call site splices `#fingerprint_assertion` into its output unconditionally,
/// and interpolating `None` via `quote!` produces no tokens.
///
/// The asserted value's const-generic literal is given the real span of the user's
/// `#[byteable(fingerprint = ..)]` string literal via `Literal::set_span` (not
/// `quote_spanned!`, which only assigns its span to tokens written literally inside that
/// macro invocation - an interpolated `#expected_lit` keeps whatever span it already had, so
/// wrapping the interpolation in `quote_spanned!` alone would be a no-op). With `set_span`
/// applied directly to the `Literal` before it's interpolated, the `E0277` diagnostic on a
/// mismatch underlines the actual `"0x.."` string in the user's attribute, not the
/// `#[derive(Byteable)]` line.
fn fingerprint_assertion_impl(
    bc: &proc_macro2::TokenStream,
    attrs: &[syn::Attribute],
    type_name: &syn::Ident,
) -> Option<proc_macro2::TokenStream> {
    attrs::parse_fingerprint_assertion(attrs).map(|maybe_expected| {
        let (expected, span) = maybe_expected.unwrap_or((0, Span::call_site()));
        let mut expected_lit = proc_macro2::Literal::u64_unsuffixed(expected);
        expected_lit.set_span(span);
        quote! {
            const _: () = #bc::assert_fingerprint::<
                #expected_lit,
                { <#type_name as #bc::WireFingerprint>::WIRE_FINGERPRINT },
            >();
        }
    })
}

fn gen_struct_field_write(
    field_access: &proc_macro2::TokenStream,
    field_type: &Type,
    attrs: &[syn::Attribute],
    bc: &proc_macro2::TokenStream,
    awaited: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let (attr_kind, attr_span) = parse_byteable_attr(attrs);
    match attr_kind {
        AttributeType::LittleEndian => quote! {
            writer.write_value(&<#field_type as #bc::HasEndianRepr>::to_little_endian(#field_access))#awaited?;
        },
        AttributeType::BigEndian => quote! {
            writer.write_value(&<#field_type as #bc::HasEndianRepr>::to_big_endian(#field_access))#awaited?;
        },
        AttributeType::None => quote! { writer.write_value(&#field_access)#awaited?; },
        AttributeType::IoOnly => std::panic::panic_any(syn::Error::new(
            attr_span,
            "#[byteable(io_only)] is a struct-level attribute and cannot be used on a field",
        )),
        AttributeType::TryTransparent => std::panic::panic_any(syn::Error::new(
            attr_span,
            "#[byteable(try_transparent)] is not applicable in io_only mode; remove the \
             annotation or use a plain field",
        )),
    }
}

fn gen_field_read(
    field_ident: &Ident,
    field_ty: &syn::Type,
    attrs: &[syn::Attribute],
    bc: &proc_macro2::TokenStream,
    awaited: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let (attr_kind, attr_span) = parse_byteable_attr(attrs);
    match attr_kind {
        AttributeType::LittleEndian => {
            quote! { let #field_ident: #field_ty = reader.read_value::<<#field_ty as #bc::HasEndianRepr>::LE>()#awaited?.get(); }
        }
        AttributeType::BigEndian => {
            quote! { let #field_ident: #field_ty = reader.read_value::<<#field_ty as #bc::HasEndianRepr>::BE>()#awaited?.get(); }
        }
        AttributeType::None => {
            quote! { let #field_ident: #field_ty = reader.read_value()#awaited?; }
        }
        other => std::panic::panic_any(syn::Error::new(
            attr_span,
            format!(
                "unsupported #[byteable({})] attribute on field `{field_ident}`; only \
                 little_endian and big_endian are supported here",
                attribute_syntax(other)
            ),
        )),
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
        let wire_fingerprint_impl =
            wire_fingerprint_impl_for_struct(&bc, name, &input.generics, &[]);
        let fingerprint_assertion = fingerprint_assertion_impl(&bc, &input.attrs, name);
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

            #wire_fingerprint_impl
            #fingerprint_assertion
        }
        .into();
    }

    let (fields, is_tuple) = match fields_data {
        syn::Fields::Named(f) => (&f.named, false),
        syn::Fields::Unnamed(f) => (&f.unnamed, true),
        syn::Fields::Unit => unreachable!(),
    };

    let orders = field_order_entries(fields.iter());
    let wire_order =
        attrs::resolve_field_wire_order(&orders).unwrap_or_else(|e| std::panic::panic_any(e));

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

    let wire_fields: Vec<&syn::Field> = field_accesses.iter().map(|(_, field)| *field).collect();
    let field_types = wire_fingerprint_field_types(&bc, wire_fields.into_iter());
    let wire_fingerprint_impl =
        wire_fingerprint_impl_for_struct(&bc, name, &input.generics, &field_types);
    let fingerprint_assertion = fingerprint_assertion_impl(&bc, &input.attrs, name);

    let mut output = dynamic_pipeline_impls(name, std_impl, eio_impl, async_impl, eio_async_impl);
    output.extend(wire_fingerprint_impl);
    output.extend(fingerprint_assertion);
    output.into()
}

fn fixed_struct_derived(input: DeriveInput) -> proc_macro::TokenStream {
    let bc = byteable_crate_path();
    let original_name = &input.ident;

    let fields_data = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => unreachable!(),
    };

    let vis = &input.vis;
    let raw_name = format_ident!("__byteable_raw_{}", original_name);

    if let Fields::Unit = fields_data {
        let wire_fingerprint_impl =
            wire_fingerprint_impl_for_struct(&bc, original_name, &input.generics, &[]);
        let fingerprint_assertion = fingerprint_assertion_impl(&bc, &input.attrs, original_name);
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

            #wire_fingerprint_impl
            #fingerprint_assertion
        }
        .into();
    }

    let (fields, is_tuple) = match fields_data {
        Fields::Named(f) => (&f.named, false),
        Fields::Unnamed(f) => (&f.unnamed, true),
        Fields::Unit => unreachable!(),
    };

    let orders = field_order_entries(fields.iter());
    let wire_order =
        attrs::resolve_field_wire_order(&orders).unwrap_or_else(|e| std::panic::panic_any(e));
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
        let (attr, attr_span) = parse_byteable_attr(&field.attrs);
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
                AttributeType::IoOnly => std::panic::panic_any(syn::Error::new(
                    attr_span,
                    "#[byteable(io_only)] is a struct-level attribute and cannot be used on individual fields",
                )),
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
                AttributeType::IoOnly => std::panic::panic_any(syn::Error::new(
                    attr_span,
                    "#[byteable(io_only)] is a struct-level attribute and cannot be used on individual fields",
                )),
                AttributeType::None => FieldInfo {
                    raw_field_def: quote! { #vis #name: <#field_type as #bc::RawRepr>::Raw },
                    to_raw_expr: quote! { #name: <#field_type as #bc::RawRepr>::to_raw(&self.#name) },
                    from_raw_expr: quote! { #name: <#field_type as #bc::FromRawRepr>::from_raw(value.#name) },
                },
            }
        };
        field_infos.push(field_info);
    }

    let wire_field_infos: Vec<&FieldInfo> = wire_order
        .iter()
        .map(|&decl_idx| &field_infos[decl_idx])
        .collect();

    let wire_fields: Vec<&syn::Field> = wire_order
        .iter()
        .map(|&decl_idx| &fields[decl_idx])
        .collect();
    let field_types = wire_fingerprint_field_types(&bc, wire_fields.into_iter());
    let wire_fingerprint_impl =
        wire_fingerprint_impl_for_struct(&bc, original_name, &input.generics, &field_types);
    let fingerprint_assertion = fingerprint_assertion_impl(&bc, &input.attrs, original_name);

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
        #wire_fingerprint_impl
        #fingerprint_assertion
    }
    .into()
}

fn extract_repr_type(attrs: &[syn::Attribute]) -> Option<syn::Ident> {
    for attr in attrs {
        if attr.path().is_ident("repr")
            && let Meta::List(meta_list) = &attr.meta
            && let Ok(ident) = syn::parse2::<syn::Ident>(meta_list.tokens.clone())
            && matches!(
                ident.to_string().as_str(),
                "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64" | "u128" | "i128"
            )
        {
            return Some(ident);
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
    let (attr_kind, attr_span) = parse_byteable_attr(attrs);
    match attr_kind {
        AttributeType::LittleEndian => quote! {
            writer.write_value(&<#field_type as #bc::HasEndianRepr>::to_little_endian(*#field_ident))#awaited?;
        },
        AttributeType::BigEndian => quote! {
            writer.write_value(&<#field_type as #bc::HasEndianRepr>::to_big_endian(*#field_ident))#awaited?;
        },
        AttributeType::None => quote! {
            writer.write_value(#field_ident)#awaited?;
        },
        other => std::panic::panic_any(syn::Error::new(
            attr_span,
            format!(
                "unsupported #[byteable({})] attribute on field `{field_ident}`; only \
                 little_endian and big_endian are supported here",
                attribute_syntax(other)
            ),
        )),
    }
}

/// Resolves a field-carrying enum variant's wire field order from any `#[byteable(order = N)]`
/// attributes on its fields, via the same `attrs::resolve_field_wire_order` machinery used for
/// struct fields (`io_struct_derive`/`fixed_struct_derived`) - each variant gets its own
/// independent `0..field_count` namespace, just like a struct's own field list. Panics with the
/// variant name for context (unlike the struct call sites, which don't need it - a struct has
/// only one field list, an enum has one per variant, so naming which one is misconfigured
/// matters here).
fn variant_field_wire_order(variant_name: &Ident, fields: &Fields) -> Vec<usize> {
    let orders = field_order_entries(fields.iter());
    attrs::resolve_field_wire_order(&orders).unwrap_or_else(|e| {
        std::panic::panic_any(syn::Error::new(
            e.span(),
            format!("variant `{variant_name}`: {e}"),
        ))
    })
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

    let endian_attr = parse_byteable_attr(&input.attrs).0;
    let EnumDiscriminants {
        defs: discriminant_defs,
        refs: discriminants,
    } = resolve_enum_discriminants(&name, &enum_data.variants, &repr_ty);

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
                        let fields: Vec<_> = named.named.iter().collect();
                        let field_names: Vec<_> =
                            fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
                        let wire_order = variant_field_wire_order(variant_name, &variant.fields);
                        let field_writes: Vec<_> = wire_order
                            .iter()
                            .map(|&i| {
                                let f = fields[i];
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
                        let fields: Vec<_> = unnamed.unnamed.iter().collect();
                        let field_idents: Vec<_> = (0..fields.len())
                            .map(|i| Ident::new(&format!("__field_{i}"), name.span()))
                            .collect();
                        let wire_order = variant_field_wire_order(variant_name, &variant.fields);
                        let field_writes: Vec<_> = wire_order
                            .iter()
                            .map(|&i| {
                                gen_enum_field_write(&field_idents[i], &fields[i].ty, &fields[i].attrs, &bc, awaited)
                            })
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
                        let fields: Vec<_> = named.named.iter().collect();
                        let field_idents: Vec<_> =
                            fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
                        let wire_order = variant_field_wire_order(variant_name, &variant.fields);
                        let field_reads: Vec<_> = wire_order
                            .iter()
                            .map(|&i| {
                                let f = fields[i];
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
                        let fields: Vec<_> = unnamed.unnamed.iter().collect();
                        let field_idents: Vec<_> = (0..fields.len())
                            .map(|i| Ident::new(&format!("__field_{i}"), name.span()))
                            .collect();
                        let wire_order = variant_field_wire_order(variant_name, &variant.fields);
                        let field_reads: Vec<_> = wire_order
                            .iter()
                            .map(|&i| {
                                gen_field_read(
                                    &field_idents[i],
                                    &fields[i].ty,
                                    &fields[i].attrs,
                                    &bc,
                                    awaited,
                                )
                            })
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

    let (unsigned_repr, _) = discriminant_repr_facts(&repr_ty);
    let variant_hashes: Vec<proc_macro2::TokenStream> = enum_data
        .variants
        .iter()
        .zip(&discriminants)
        .map(|(variant, tag_ref)| {
            let fields: Vec<_> = variant.fields.iter().collect();
            let wire_order = variant_field_wire_order(&variant.ident, &variant.fields);
            let wire_fields = wire_order.into_iter().map(|i| fields[i]);
            let field_types = wire_fingerprint_field_types(&bc, wire_fields);
            variant_fingerprint_expr(&bc, tag_ref, &unsigned_repr, &field_types)
        })
        .collect();

    let wire_fingerprint_impl = wire_fingerprint_impl_for_enum(
        &bc,
        &name,
        &input.generics,
        &repr_ty,
        endian_attr,
        &variant_hashes,
    );
    let fingerprint_assertion = fingerprint_assertion_impl(&bc, &input.attrs, &name);

    let dynamic_impls =
        dynamic_pipeline_impls(&name, std_impl, eio_impl, async_impl, eio_async_impl);
    quote! {
        #discriminant_defs
        #dynamic_impls
        #wire_fingerprint_impl
        #fingerprint_assertion
    }
    .into()
}

/// The result of resolving an enum's per-variant wire discriminants.
///
/// - `defs` is a group of hidden, module-level `const __BYTEABLE_TAG_<ENUMNAME>_<index>:
///   repr_ty = ...;` items (plus a duplicate-value compile-time check) - splice this once into
///   the derive's output, anywhere at the top level.
/// - `refs` is, for each variant in declaration order, the bare identifier expression
///   referencing that variant's const (`__BYTEABLE_TAG_<ENUMNAME>_<index>`) - valid as both
///   a plain expression (the encode/write side) and a match pattern (the decode/read side),
///   since Rust allows consts in both positions. The identifier embeds the variant's
///   *position*, not its name: naming by variant text (even case-normalized) can collide
///   (`Ab`/`AB` in the same enum both uppercase to the same identifier) or panic outright on a
///   raw identifier (`r#type`, since `format_ident!` only strips the `r#` prefix automatically
///   when given a `syn::Ident`, not a `String` - and uppercasing requires going through
///   `String`). The enum's own name is still embedded (via its `Ident`, `r#`-stripped
///   manually) purely to keep two different enums' consts from colliding in the same module;
///   it never needs to disambiguate two variants of the *same* enum from each other, since the
///   index already does that uniquely and unambiguously. The identifier is fully upper-cased so
///   it reads as a real SCREAMING_SNAKE_CASE const to rustc's own `non_upper_case_globals`
///   lint wherever it's used as a match pattern.
struct EnumDiscriminants {
    defs: proc_macro2::TokenStream,
    refs: Vec<proc_macro2::TokenStream>,
}

/// Builds `EnumDiscriminants` for every variant, purely from `#[byteable(tag = <expr>)]`
/// attributes (via `attrs::parse_variant_tag`) - Rust's own real `= N` discriminant syntax is
/// never read. An unannotated variant's const is defined as `<previous variant's const> + 1`;
/// the first variant defaults to `0` if unannotated. No value is ever evaluated or
/// range-checked here - every `const` is typed as the real `repr_ty`, so range-fit, increment
/// overflow, and (via the generated duplicate check) collisions are all rustc's problem.
///
/// Two variants resolving to the same value is caught by emitting a hidden, unused "shadow"
/// enum whose real Rust discriminants (`= <expr>`) are the same free consts referenced by
/// `refs` - if two are equal, this is `E0081: discriminant value assigned more than once`, a
/// hard error from Rust's own enum well-formedness checking. This was chosen over two other
/// mechanisms that were tried and rejected: an `O(n^2)` const-fn/array scan (correct, but hits
/// rustc's deny-by-default `long_running_const_eval` lint above roughly 1100-1400 variants -
/// well within the crate's documented 65,536-variant auto-select-ladder maximum, so a real
/// regression), and `#[deny(unreachable_patterns)]` on the generated decode functions (doesn't
/// work at all: rustc unconditionally suppresses reachability lints for spans originating from
/// a macro in a different crate than the one being compiled, which every line this derive macro
/// generates always is - confirmed with a from-scratch minimal proc-macro with no connection to
/// this codebase). The shadow-enum approach sidesteps both problems: `E0081` is a hard error,
/// not a lint, so it isn't subject to the macro-span exemption, and discriminant-uniqueness
/// checking is native compiler logic for enum lowering, not general const-eval, so it has no
/// meaningful size ceiling (verified: 65,536 variants, the crate's actual documented maximum,
/// compiles in a few seconds either way, duplicate or not).
///
/// These are free module-level consts, not associated consts on `impl #enum_name`: the enum
/// may be generic (the field-carrying/dynamic pipeline supports generic enums via
/// `input.generics.split_for_impl()`), and these values never depend on its type parameters -
/// tying them to the enum's own generics would be both unnecessary and, for a generic enum, a
/// hard compile error (an inherent assoc const on a generic type can't be referenced from a
/// context with no concrete type argument in scope).
fn resolve_enum_discriminants(
    enum_name: &Ident,
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::Token![,]>,
    repr_ty: &syn::Ident,
) -> EnumDiscriminants {
    let enum_name_upper = enum_name
        .to_string()
        .trim_start_matches("r#")
        .to_uppercase();
    let const_idents: Vec<Ident> = (0..variants.len())
        .map(|i| format_ident!("__BYTEABLE_TAG_{}_{}", enum_name_upper, i))
        .collect();

    let const_defs: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .enumerate()
        .map(|(i, variant)| {
            let const_ident = &const_idents[i];
            let value_expr = match attrs::parse_variant_tag(&variant.attrs) {
                Some(expr) => quote! { #expr },
                None if i == 0 => quote! { 0 },
                None => {
                    let prev_ident = &const_idents[i - 1];
                    quote! { #prev_ident + 1 }
                }
            };
            quote! {
                #[doc(hidden)]
                #[allow(dead_code)]
                const #const_ident: #repr_ty = #value_expr;
            }
        })
        .collect();

    let refs: Vec<proc_macro2::TokenStream> =
        const_idents.iter().map(|ident| quote! { #ident }).collect();

    let dup_check = if variants.len() < 2 {
        quote! {}
    } else {
        let shadow_name = format_ident!("__BYTEABLE_TAG_CHECK_{}", enum_name_upper);
        let shadow_variant_idents: Vec<Ident> = (0..variants.len())
            .map(|i| format_ident!("__V{}", i))
            .collect();
        let shadow_variants: Vec<proc_macro2::TokenStream> = shadow_variant_idents
            .iter()
            .zip(&refs)
            .map(|(v, r)| quote! { #v = #r, })
            .collect();
        quote! {
            #[doc(hidden)]
            #[allow(dead_code, non_camel_case_types)]
            #[repr(#repr_ty)]
            enum #shadow_name {
                #(#shadow_variants)*
            }
        }
    };

    let defs = quote! {
        #(#const_defs)*
        #dup_check
    };

    EnumDiscriminants { defs, refs }
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

    let endian_attr = parse_byteable_attr(&input.attrs).0;
    let EnumDiscriminants {
        defs: discriminant_defs,
        refs: discriminants,
    } = resolve_enum_discriminants(enum_name, &enum_data.variants, &repr_ty);

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

    let (unsigned_repr, _) = discriminant_repr_facts(&repr_ty);
    let variant_hashes: Vec<proc_macro2::TokenStream> = discriminants
        .iter()
        // Unit variants: an empty field list beyond the tag.
        .map(|tag_ref| variant_fingerprint_expr(&bc, tag_ref, &unsigned_repr, &[]))
        .collect();

    // Fieldless enums can't meaningfully be generic (a type parameter with no field to
    // appear in would be unused, which rustc rejects on its own), so this derive path never
    // threads real generics through - but `wire_fingerprint_impl_for_enum` is shared with
    // `enum_derive`, which does support generics, so pass an empty `Generics` here purely to
    // get the matching `ImplGenerics`/`TypeGenerics` shape (both render as empty tokens).
    let wire_fingerprint_impl = wire_fingerprint_impl_for_enum(
        &bc,
        enum_name,
        &syn::Generics::default(),
        &repr_ty,
        endian_attr,
        &variant_hashes,
    );
    let fingerprint_assertion = fingerprint_assertion_impl(&bc, &input.attrs, enum_name);

    quote! {
        #discriminant_defs

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

        #wire_fingerprint_impl
        #fingerprint_assertion
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    /// Runs `f`, which is expected to panic with a `syn::Error` payload (the convention every
    /// hard-error path in this crate now follows - see `parse_byteable_attr`'s doc comment),
    /// and returns that error so its message can be asserted on. `#[should_panic]` can't be
    /// used for this: it matches the panic payload against a `&str`/`String`, and a `syn::Error`
    /// payload is neither, so it would report "panic did not contain expected string" even when
    /// the panic and its message are both correct.
    fn expect_abort(f: impl FnOnce() + std::panic::UnwindSafe) -> syn::Error {
        let payload = std::panic::catch_unwind(f).expect_err("expected a panic");
        *payload
            .downcast::<syn::Error>()
            .expect("panic payload should be a syn::Error - see each function's doc comment")
    }

    #[test]
    fn struct_derive_rejects_struct_level_little_endian() {
        let input: DeriveInput = parse_quote! {
            #[byteable(little_endian)]
            struct Foo { a: u32 }
        };
        let err = expect_abort(|| {
            struct_derive(input);
        });
        assert!(
            err.to_string()
                .contains("not supported at the struct level")
        );
    }

    #[test]
    fn struct_derive_rejects_struct_level_big_endian_on_io_only() {
        let input: DeriveInput = parse_quote! {
            #[byteable(io_only)]
            #[byteable(big_endian)]
            struct Foo { a: u32 }
        };
        let err = expect_abort(|| {
            struct_derive(input);
        });
        assert!(err.to_string().contains("at most one of"));
    }

    #[test]
    fn parse_byteable_attr_ignores_order_token() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(order = 0, big_endian)])];
        assert_eq!(parse_byteable_attr(&attrs).0, AttributeType::BigEndian);
    }

    #[test]
    fn parse_byteable_attr_scans_every_attribute_instance() {
        let attrs: Vec<syn::Attribute> = vec![
            parse_quote!(#[byteable(order = 2)]),
            parse_quote!(#[byteable(big_endian)]),
        ];
        assert_eq!(parse_byteable_attr(&attrs).0, AttributeType::BigEndian);
    }

    #[test]
    fn parse_byteable_attr_still_rejects_unknown_tokens() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(not_a_real_attribute)])];
        let err = expect_abort(|| {
            parse_byteable_attr(&attrs);
        });
        assert!(err.to_string().contains("unknown byteable attribute"));
    }

    #[test]
    fn parse_byteable_attr_accepts_fingerprint_token() {
        // `fingerprint` must not fall into the "unknown byteable attribute" branch.
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(fingerprint = "0x1234")])];
        assert_eq!(parse_byteable_attr(&attrs).0, AttributeType::None);
    }

    #[test]
    fn parse_byteable_attr_rejects_malformed_fingerprint_value() {
        // Regression test for the "typo silently disables the assertion forever" bug: a
        // malformed hex value (letter `O` instead of digit `0`) must be a hard compile-time
        // panic here, not something `parse_fingerprint_assertion` is left to quietly ignore.
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[byteable(fingerprint = "0x7O26af966196fd")])];
        let err = expect_abort(|| {
            parse_byteable_attr(&attrs);
        });
        assert!(err.to_string().contains("expected a fingerprint value"));
    }

    #[test]
    fn parse_byteable_attr_rejects_duplicate_fingerprint() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[byteable(fingerprint = "0x1", fingerprint = "0x2")])];
        let err = expect_abort(|| {
            parse_byteable_attr(&attrs);
        });
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn type_level_fingerprint_is_accepted() {
        let input: DeriveInput = parse_quote! {
            #[byteable(fingerprint = "0x1234")]
            struct S { a: u8 }
        };
        reject_non_type_level_fingerprint(&input);
    }

    #[test]
    fn struct_field_fingerprint_is_rejected() {
        // Before this check, a field-level `fingerprint` was hex-validated by
        // `parse_byteable_attr` and then produced *no assertion at all* - the worst possible
        // failure mode for a safety check, and the exact thing the eager hex validation next
        // door exists to prevent.
        let input: DeriveInput = parse_quote! {
            struct S {
                #[byteable(fingerprint = "0x1234")]
                a: u8,
            }
        };
        let err = expect_abort(|| {
            reject_non_type_level_fingerprint(&input);
        });
        assert!(
            err.to_string()
                .contains("only supported on the type itself")
        );
    }

    #[test]
    fn enum_variant_fingerprint_is_rejected() {
        let input: DeriveInput = parse_quote! {
            enum E {
                #[byteable(fingerprint = "0x1234")]
                A,
                B,
            }
        };
        let err = expect_abort(|| {
            reject_non_type_level_fingerprint(&input);
        });
        assert!(
            err.to_string()
                .contains("only supported on the type itself")
        );
    }

    #[test]
    fn enum_variant_field_fingerprint_is_rejected() {
        let input: DeriveInput = parse_quote! {
            enum E {
                A(#[byteable(fingerprint = "0x1234")] u32),
            }
        };
        let err = expect_abort(|| {
            reject_non_type_level_fingerprint(&input);
        });
        assert!(
            err.to_string()
                .contains("only supported on the type itself")
        );
    }

    #[test]
    fn other_field_and_variant_attributes_are_left_alone() {
        // The sweep has to walk *past* every other key's `= <value>` without tripping on it,
        // including a multi-token const expression, or a `fingerprint` later in the same list
        // would be missed.
        let input: DeriveInput = parse_quote! {
            enum E {
                #[byteable(tag = 1 + (1u128 << 64))]
                A(#[byteable(big_endian)] u32),
                #[byteable(tag = 2)]
                B,
            }
        };
        reject_non_type_level_fingerprint(&input);

        let input: DeriveInput = parse_quote! {
            struct S {
                #[byteable(order = 1, big_endian)]
                a: u32,
                #[byteable(order = 0, try_transparent)]
                b: u8,
            }
        };
        reject_non_type_level_fingerprint(&input);
    }

    #[test]
    fn fingerprint_after_another_key_in_the_same_list_is_still_rejected() {
        let input: DeriveInput = parse_quote! {
            enum E {
                #[byteable(tag = 1 + (1u128 << 64), fingerprint = "0x1234")]
                A,
            }
        };
        let err = expect_abort(|| {
            reject_non_type_level_fingerprint(&input);
        });
        assert!(
            err.to_string()
                .contains("only supported on the type itself")
        );
    }
}
