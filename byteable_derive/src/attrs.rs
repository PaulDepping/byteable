pub fn parse_field_order(attrs: &[syn::Attribute]) -> Option<u64> {
    let mut result = None;
    for attr in attrs {
        if !attr.path().is_ident("byteable") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("order") {
                let lit: syn::LitInt = meta.value()?.parse()?;
                let value: u64 = lit.base10_parse()?;
                if result.is_some() {
                    return Err(meta.error("duplicate #[byteable(order = ..)]"));
                }
                result = Some(value);
            }
            Ok(())
        });
    }
    result
}

pub fn resolve_field_wire_order(orders: &[Option<u64>]) -> Result<Vec<usize>, String> {
    let n = orders.len();
    let annotated = orders.iter().filter(|o| o.is_some()).count();

    if annotated == 0 {
        return Ok((0..n).collect());
    }
    if annotated != n {
        return Err(format!(
            "#[byteable(order = ..)] must annotate every field or none; {annotated} of {n} \
             fields are annotated"
        ));
    }

    let mut seen = vec![false; n];
    let mut wire_order = vec![0usize; n];
    for (decl_idx, order) in orders.iter().enumerate() {
        let order = order.unwrap() as usize;
        if order >= n {
            return Err(format!(
                "#[byteable(order = {order})] is out of range; with {n} annotated fields, \
                 valid values are 0..{n}"
            ));
        }
        if seen[order] {
            return Err(format!("duplicate #[byteable(order = {order})]"));
        }
        seen[order] = true;
        wire_order[order] = decl_idx;
    }
    Ok(wire_order)
}

/// Returns this variant's explicit `#[byteable(tag = <expr>)]` expression, if present, kept
/// as an opaque `syn::Expr` - the expression is never evaluated or type-checked here. It's
/// emitted verbatim into a generated `const __BYTEABLE_TAG_<ENUM>_<index>: repr_ty = <expr>;`
/// declaration once `repr_ty` is known (see `resolve_enum_discriminants` in `lib.rs`), so
/// anything valid in that const position - a literal, a named const, `1 << 4`, etc. - works
/// with zero special-casing here. Range-fit, overflow, and duplicate-value detection are all
/// deferred to that generated code's own compilation.
pub fn parse_variant_tag(attrs: &[syn::Attribute]) -> Option<syn::Expr> {
    let mut result = None;
    for attr in attrs {
        if !attr.path().is_ident("byteable") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("tag") {
                let expr: syn::Expr = meta.value()?.parse()?;
                if result.is_some() {
                    return Err(meta.error("duplicate #[byteable(tag = ..)]"));
                }
                result = Some(expr);
            }
            Ok(())
        })
        .unwrap_or_else(|e| panic!("{e}"));
    }
    result
}

/// Reads the enum-level `#[byteable(discriminant = <ident>)]` override, which selects the
/// wire type used for the enum's discriminant independently of any real `#[repr(...)]` on the
/// enum. The `<ident>` must be one of the 10 fixed-width integer type names
/// (`u8`/`u16`/`u32`/`u64`/`u128`/`i8`/`i16`/`i32`/`i64`/`i128`); anything else, or a duplicate
/// `discriminant = ..` on the same item, is a hard parse error that's silently swallowed here
/// (unlike `parse_variant_tag`'s errors, which now panic - see its doc comment), so callers see
/// `None` and the derive falls back to `extract_repr_type`/the auto-select ladder.
pub fn parse_discriminant_override(attrs: &[syn::Attribute]) -> Option<syn::Ident> {
    let mut result = None;
    for attr in attrs {
        if !attr.path().is_ident("byteable") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("discriminant") {
                let ident: syn::Ident = meta.value()?.parse()?;
                if !matches!(
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
                    return Err(meta.error(
                        "#[byteable(discriminant = ..)] must be one of u8/u16/u32/u64/u128/i8/i16/i32/i64/i128",
                    ));
                }
                if result.is_some() {
                    return Err(meta.error("duplicate #[byteable(discriminant = ..)]"));
                }
                result = Some(ident);
            }
            Ok(())
        });
    }
    result
}

/// Parses a `#[byteable(fingerprint = "...")]` value string into a `u64`. A `0x` prefix means
/// hex; anything else is read as decimal.
///
/// Both forms are accepted because the mismatch diagnostic prints the actual value in
/// **decimal** and cannot be made to print hex: it comes out of rustc's own const-generic
/// rendering inside `#[diagnostic::on_unimplemented]`, which has no formatting control. The
/// error message tells the user to "replace the asserted value with <N>", and following that
/// instruction literally has to work. When this function was hex-only, it either failed to
/// parse the pasted decimal value (annoying) or - for a decimal value that happens to contain
/// only hex digits, e.g. `7091622255289697877` - silently accepted it as a completely
/// different number, re-arming the assertion against a value the type never had. That is the
/// same "typo silently disables the safety check" failure mode the eager validation in
/// `parse_byteable_attr` exists to prevent.
///
/// A decimal value is not ambiguous with a hex one: hex must carry the `0x` prefix to be read
/// as hex, so `"1234"` is decimal 1234 and `"0x1234"` is 4660.
///
/// Shared between `parse_byteable_attr`'s eager validation (in `lib.rs` - it validates the
/// value at the point where malformed input already becomes a hard panic, so a typo can't
/// silently disable the assertion forever) and `parse_fingerprint_assertion` below, so the two
/// can't disagree about what counts as a valid value.
pub fn parse_fingerprint_value(text: &str) -> Result<u64, std::num::ParseIntError> {
    match text.strip_prefix("0x") {
        Some(hex) => u64::from_str_radix(hex, 16),
        None => text.parse::<u64>(),
    }
}

/// Reads the type-level `#[byteable(fingerprint = "0x...")]` assertion attribute, which the
/// derive macro turns into a compile-time `assert_fingerprint::<EXPECTED, ACTUAL>()` check
/// (see `WireFingerprint`/`FingerprintEq`/`assert_fingerprint` in `src/fingerprint.rs`).
/// Returns `None` if the attribute is absent (no assertion emitted at all), `Some(None)` for
/// the bare `#[byteable(fingerprint)]` no-value form (asserts against `0` - the deliberate
/// "log on first use" default: the very first build after adding the bare attribute fails
/// with the real value in the diagnostic, ready to copy in), and `Some(Some((v, span)))` for
/// an explicit `#[byteable(fingerprint = "0x...")]`/`#[byteable(fingerprint = "4660")]`
/// with `v` the parsed value (hex when `0x`-prefixed, decimal otherwise) and `span`
/// the string literal's own span - the caller applies that span directly to the generated
/// const-generic literal (via `Literal::set_span`) so the diagnostic on a mismatch underlines
/// the user's attribute value, not a macro-internal location. The bare form has no literal to
/// span at all; its caller falls back to `Span::call_site()`.
///
/// A malformed value here returns an error from the `parse_nested_meta` closure, which
/// this function discards (`let _ = ..`) rather than propagating - by itself that would let a
/// typo silently disable the assertion (see `parse_fingerprint_value`'s doc comment); it's safe
/// here specifically because `parse_byteable_attr` (in `lib.rs`) already validates the same
/// value eagerly and panics on it before this function is ever reached in the real derive
/// flow. A duplicate `#[byteable(fingerprint = ..)]` is rejected the same way
/// `parse_variant_tag`/`parse_discriminant_override` reject theirs: the first occurrence
/// sticks, the second is refused (as an `Err` from this closure, so - same caveat - it's
/// `parse_byteable_attr`'s independent duplicate check that actually surfaces this as a panic).
pub fn parse_fingerprint_assertion(
    attrs: &[syn::Attribute],
) -> Option<Option<(u64, proc_macro2::Span)>> {
    let mut result = None;
    for attr in attrs {
        if !attr.path().is_ident("byteable") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("fingerprint") {
                if result.is_some() {
                    return Err(meta.error("duplicate #[byteable(fingerprint = ..)]"));
                }
                if meta.input.peek(syn::Token![=]) {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    let span = lit.span();
                    let parsed = parse_fingerprint_value(&lit.value()).map_err(|_| {
                        meta.error(
                            "expected a fingerprint value: hex with a `0x` prefix \
                             (\"0x1234\") or plain decimal (\"4660\")",
                        )
                    })?;
                    result = Some(Some((parsed, span)));
                } else {
                    result = Some(None);
                }
            }
            Ok(())
        });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn parse_field_order_reads_value() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(order = 3)])];
        assert_eq!(parse_field_order(&attrs), Some(3));
    }

    #[test]
    fn parse_field_order_absent() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(big_endian)])];
        assert_eq!(parse_field_order(&attrs), None);
    }

    #[test]
    fn parse_field_order_combined_with_endian() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(big_endian, order = 1)])];
        assert_eq!(parse_field_order(&attrs), Some(1));
    }

    #[test]
    fn resolve_no_annotations_is_identity() {
        assert_eq!(resolve_field_wire_order(&[None, None, None]), Ok(vec![0, 1, 2]));
    }

    #[test]
    fn resolve_full_reorder() {
        // field 0 -> wire pos 2, field 1 -> wire pos 0, field 2 -> wire pos 1
        let orders = [Some(2), Some(0), Some(1)];
        // wire_order[wire_pos] = decl_idx
        assert_eq!(resolve_field_wire_order(&orders), Ok(vec![1, 2, 0]));
    }

    #[test]
    fn resolve_partial_annotation_errors() {
        let orders = [Some(0), None, Some(1)];
        assert!(resolve_field_wire_order(&orders).is_err());
    }

    #[test]
    fn resolve_gap_errors() {
        // 2 fields, orders {0, 2} - not dense 0..2
        let orders = [Some(0), Some(2)];
        assert!(resolve_field_wire_order(&orders).is_err());
    }

    #[test]
    fn resolve_duplicate_errors() {
        let orders = [Some(0), Some(0)];
        assert!(resolve_field_wire_order(&orders).is_err());
    }

    #[test]
    fn parse_variant_tag_reads_value() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(tag = 5)])];
        let result = parse_variant_tag(&attrs).unwrap();
        assert_eq!(quote::quote!(#result).to_string(), "5");
    }

    #[test]
    fn parse_variant_tag_reads_negative_value() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(tag = -10)])];
        let result = parse_variant_tag(&attrs).unwrap();
        assert_eq!(quote::quote!(#result).to_string(), "- 10");
    }

    #[test]
    fn parse_variant_tag_accepts_named_constant() {
        // A path expression is accepted verbatim - it's emitted into a generated
        // `const X: repr_ty = <expr>;` and type-checked/range-checked by rustc itself,
        // not interpreted here.
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(tag = SOME_CONST)])];
        let result = parse_variant_tag(&attrs).unwrap();
        assert_eq!(quote::quote!(#result).to_string(), "SOME_CONST");
    }

    #[test]
    #[should_panic(expected = "expected")]
    fn parse_variant_tag_rejects_missing_value() {
        // `#[byteable(tag)]` with no `= value` must panic rather than be silently accepted as
        // if `tag` were absent - same "don't swallow a malformed attribute" rule as the
        // duplicate-tag and duplicate-fingerprint checks below.
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(tag)])];
        parse_variant_tag(&attrs);
    }

    #[test]
    #[should_panic(expected = "duplicate")]
    fn parse_variant_tag_rejects_duplicate() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(tag = 1, tag = 2)])];
        parse_variant_tag(&attrs);
    }

    #[test]
    fn parse_discriminant_override_reads_value() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(discriminant = u16)])];
        let ident = parse_discriminant_override(&attrs);
        assert_eq!(ident.map(|i| i.to_string()), Some("u16".to_string()));
    }

    #[test]
    fn parse_discriminant_override_absent() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(big_endian)])];
        assert_eq!(parse_discriminant_override(&attrs), None);
    }

    #[test]
    fn parse_discriminant_override_rejects_invalid_type_name() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(discriminant = u24)])];
        assert_eq!(parse_discriminant_override(&attrs), None);
    }

    #[test]
    fn parse_discriminant_override_rejects_duplicate() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[byteable(discriminant = u8, discriminant = u16)])];
        let ident = parse_discriminant_override(&attrs);
        // First value sticks (mirrors parse_variant_tag_rejects_duplicate's documented
        // left-to-right, stop-at-first-error behavior of parse_nested_meta); what matters is
        // the second value is never accepted.
        assert_eq!(ident.map(|i| i.to_string()), Some("u8".to_string()));
    }

    #[test]
    fn parse_fingerprint_assertion_absent() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(big_endian)])];
        assert!(parse_fingerprint_assertion(&attrs).is_none());
    }

    #[test]
    fn parse_fingerprint_assertion_bare_form_defaults_to_zero() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(fingerprint)])];
        assert!(matches!(parse_fingerprint_assertion(&attrs), Some(None)));
    }

    #[test]
    fn parse_fingerprint_assertion_reads_hex_value() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(fingerprint = "0x1234")])];
        // The span carried alongside the value isn't meaningfully comparable here (its whole
        // purpose is diagnostic placement in the generated code, verified manually - see the
        // derive macro's span-hygiene check) - only the parsed value is asserted.
        let value = parse_fingerprint_assertion(&attrs).map(|inner| inner.map(|(v, _span)| v));
        assert_eq!(value, Some(Some(0x1234)));
    }

    #[test]
    fn parse_fingerprint_assertion_rejects_duplicate() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[byteable(fingerprint = "0x1", fingerprint = "0x2")])];
        let value = parse_fingerprint_assertion(&attrs).map(|inner| inner.map(|(v, _span)| v));
        // First value sticks (mirrors parse_variant_tag_rejects_duplicate's and
        // parse_discriminant_override_rejects_duplicate's documented left-to-right,
        // stop-at-first-error behavior of parse_nested_meta); what matters is the second value
        // is never accepted. In the real derive flow, `parse_byteable_attr`'s own independent
        // duplicate check (in `lib.rs`) is what actually turns this into a hard compile-time
        // panic - this function's own error is discarded by its `let _ = ..`, same as a
        // malformed hex value (see `parse_fingerprint_value`'s doc comment).
        assert_eq!(value, Some(Some(0x1)));
    }

    #[test]
    fn parse_fingerprint_value_reads_0x_prefixed_hex() {
        assert_eq!(parse_fingerprint_value("0x1234").unwrap(), 0x1234);
        assert_eq!(parse_fingerprint_value("0xfbcd309b421eb67c").unwrap(), 0xfbcd309b421eb67c);
    }

    #[test]
    fn parse_fingerprint_value_reads_unprefixed_input_as_decimal() {
        // This is the whole point of accepting two forms: the mismatch diagnostic renders the
        // actual value in decimal (rustc's own const-generic formatting, not ours), and tells
        // the user to paste it back in. `7091622255289697877` happens to contain only hex
        // digits, so the old hex-only parser accepted it *as hex* - silently re-arming the
        // assertion against 8168936092414281847, a value the type never had.
        assert_eq!(parse_fingerprint_value("1234").unwrap(), 1234);
        assert_eq!(
            parse_fingerprint_value("7091622255289697877").unwrap(),
            7091622255289697877
        );
    }

    #[test]
    fn parse_fingerprint_value_rejects_malformed_value() {
        // The bug this guards against: a typo'd hex digit (e.g. "0x7O26af.." with a letter O
        // instead of a zero) must be rejected, not silently treated as if the attribute were
        // absent - see `parse_byteable_attr`'s eager use of this function in `lib.rs`.
        assert!(parse_fingerprint_value("0x7O26af966196fd").is_err());
        // Hex digits without the `0x` prefix are decimal input, so `a`-`f` are malformed too.
        assert!(parse_fingerprint_value("deadbeef").is_err());
        assert!(parse_fingerprint_value("").is_err());
    }
}
