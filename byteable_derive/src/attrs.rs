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

/// Parses the value expression of `#[byteable(tag = ..)]` into an `i128`.
///
/// Accepts exactly two syntactic forms:
/// - a bare positive integer literal (`5`)
/// - a unary-negated integer literal (`-10`)
///
/// Any other expression (a named constant like `i128::MIN`, an arbitrary
/// expression, etc.) is a hard parse error. A negative literal whose
/// magnitude doesn't fit in a positive `i128` (i.e. `i128::MIN`'s magnitude,
/// 170141183460469231731687303715884105728) is also a hard parse error
/// rather than a panic or silently-wrong value - `i128::MIN` itself simply
/// cannot be written as `#[byteable(tag = ..)]` today. This is treated as an
/// acceptable, extreme edge case rather than something worth special-casing.
fn parse_tag_expr(expr: &syn::Expr) -> syn::Result<i128> {
    match expr {
        syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(li), .. }) => li.base10_parse::<i128>(),
        syn::Expr::Unary(syn::ExprUnary { op: syn::UnOp::Neg(_), expr: inner, .. }) => {
            match &**inner {
                syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(li), .. }) => {
                    // `base10_parse::<i128>` only accepts magnitudes that fit in a
                    // *positive* i128, so `i128::MIN`'s magnitude (one past
                    // i128::MAX) fails to parse here rather than overflowing on
                    // negation below.
                    let magnitude = li.base10_parse::<i128>()?;
                    Ok(-magnitude)
                }
                _ => Err(syn::Error::new_spanned(
                    expr,
                    "#[byteable(tag = ..)] must be an integer literal",
                )),
            }
        }
        _ => Err(syn::Error::new_spanned(
            expr,
            "#[byteable(tag = ..)] must be an integer literal",
        )),
    }
}

pub fn parse_variant_tag(attrs: &[syn::Attribute]) -> Option<i128> {
    let mut result = None;
    for attr in attrs {
        if !attr.path().is_ident("byteable") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("tag") {
                let expr: syn::Expr = meta.value()?.parse()?;
                let value: i128 = parse_tag_expr(&expr)?;
                if result.is_some() {
                    return Err(meta.error("duplicate #[byteable(tag = ..)]"));
                }
                result = Some(value);
            }
            Ok(())
        });
    }
    result
}

pub fn resolve_variant_tags(tags: &[Option<i128>]) -> Result<Vec<i128>, String> {
    let mut resolved = Vec::with_capacity(tags.len());
    let mut next: i128 = 0;
    let mut seen = std::collections::BTreeSet::new();

    for tag in tags {
        let value = tag.unwrap_or(next);
        if !seen.insert(value) {
            return Err(format!("duplicate #[byteable(tag = {value})]"));
        }
        resolved.push(value);
        next = value.checked_add(1).unwrap_or(value);
    }
    Ok(resolved)
}

/// Reinterprets `value` as the two's-complement bit pattern it would have if cast to
/// `repr_ty` (one of the 10 fixed-width integer type names: `u8`/`u16`/`u32`/`u64`/`u128`/
/// `i8`/`i16`/`i32`/`i64`/`i128`), widened back to `i128` so the return type is uniform.
///
/// This is the pure numeric half of what `reinterpret_tag_for_repr` in `lib.rs` does; that
/// function additionally wraps the result in a `proc_macro2::Literal`.
///
/// Panics on an unrecognized `repr_ty` string. This is unreachable in practice - the only
/// callers ever pass a string produced by `extract_repr_type` or one of the count-based
/// auto-select ladders, both of which only ever produce these 10 strings - but the panic is
/// kept as a safety net rather than silently returning a wrong value.
///
/// ## `u128` is intentionally special: negative values are accepted, unlike every other
/// ## unsigned width
///
/// For `u8`/`u16`/`u32`/`u64`, a negative `value` never round-trips (the intermediate `as iN`
/// cast changes it, so `validate_tags_fit_width` correctly rejects e.g. `tag = -10` against
/// `u8`). For `u128` the cast `(value as u128) as i128` is instead a lossless, bit-preserving
/// round trip for *every* `i128` value, negative ones included - so a negative `tag` against
/// `#[byteable(discriminant = u128)]` is accepted, not rejected.
///
/// This is deliberate, not an oversight: `tag` is `i128`-typed, so no positive literal can
/// ever express a `u128` value in the upper half of its range (`2^127` to `u128::MAX`).
/// Rejecting negative-against-`u128` the same way as the other widths would make that entire
/// half of the wire format's range completely inexpressible via `tag`. Instead, a negative
/// `tag` reaches that range via two's-complement wraparound - e.g. `tag = -1` reinterprets to
/// `u128::MAX`, `tag = -10` reinterprets to `u128::MAX - 9`. Unintuitive, but it is the only
/// way to reach that range, so it is kept as a documented escape hatch. See
/// `validate_tags_fit_width_accepts_negative_against_u128_as_documented_escape_hatch` in the
/// test module below, and the "Enum-level attributes" reference in `byteable_derive::lib`'s
/// derive-macro doc comment for the user-facing explanation.
pub fn reinterpret_i128_for_repr(value: i128, repr_ty: &str) -> i128 {
    match repr_ty {
        "u8" => ((value as i8) as u8) as i128,
        "u16" => ((value as i16) as u16) as i128,
        "u32" => ((value as i32) as u32) as i128,
        "u64" => ((value as i64) as u64) as i128,
        // See the doc comment above: this is the one width where a negative `value` is
        // accepted rather than rejected, by design.
        "u128" => (value as u128) as i128,
        "i8" => (value as i8) as i128,
        "i16" => (value as i16) as i128,
        "i32" => (value as i32) as i128,
        "i64" => (value as i64) as i128,
        "i128" => value,
        other => panic!("unsupported repr type `{other}` for enum discriminant"),
    }
}

/// Reads the enum-level `#[byteable(discriminant = <ident>)]` override, which selects the
/// wire type used for the enum's discriminant independently of any real `#[repr(...)]` on the
/// enum. The `<ident>` must be one of the 10 fixed-width integer type names accepted by
/// `reinterpret_i128_for_repr`; anything else, or a duplicate `discriminant = ..` on the same
/// item, is a hard parse error (silently ignored, so `parse_variant_tag`-style callers see
/// `None` and the derive falls back to `extract_repr_type`/the auto-select ladder - matching
/// how other malformed `#[byteable(..)]` sub-attributes in this file behave).
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

/// Validates that every resolved discriminant value in `tags` fits losslessly in `repr_ty`,
/// i.e. reinterpreting it through `reinterpret_i128_for_repr` round-trips to the same value.
/// This is the general, correct way to detect "doesn't fit" uniformly for both signed and
/// unsigned `repr_ty` without needing separate bit-width-range-formula logic per type.
///
/// Returns `Err` naming the first offending value and `repr_ty`, and suggesting the user
/// either fix the value or widen the enum's discriminant width via
/// `#[byteable(discriminant = ..)]`.
pub fn validate_tags_fit_width(tags: &[i128], repr_ty: &str) -> Result<(), String> {
    for &value in tags {
        if reinterpret_i128_for_repr(value, repr_ty) != value {
            return Err(format!(
                "#[byteable(tag = {value})] does not fit in the enum's discriminant type \
                 `{repr_ty}`; either change the tag value or widen the discriminant width \
                 with #[byteable(discriminant = ..)]"
            ));
        }
    }
    Ok(())
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
        assert_eq!(parse_variant_tag(&attrs), Some(5));
    }

    #[test]
    fn parse_variant_tag_reads_negative_value() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(tag = -10)])];
        assert_eq!(parse_variant_tag(&attrs), Some(-10));
    }

    #[test]
    fn parse_variant_tag_rejects_named_constant() {
        // `i128::MIN` parses as a path expression, not an integer literal (with
        // or without unary negation) - it must not be silently accepted.
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(tag = i128::MIN)])];
        assert_eq!(parse_variant_tag(&attrs), None);
    }

    #[test]
    fn parse_variant_tag_rejects_missing_value() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(tag)])];
        assert_eq!(parse_variant_tag(&attrs), None);
    }

    #[test]
    fn parse_variant_tag_rejects_duplicate() {
        // `parse_nested_meta` processes nested items left-to-right and stops at
        // the first error; the first `tag = ..` is already recorded by the time
        // the duplicate is rejected, so it sticks (mirroring pre-existing
        // behavior of the u128 version) rather than being reset to `None`. What
        // matters here is that the *second* value is never accepted.
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[byteable(tag = 1, tag = 2)])];
        assert_eq!(parse_variant_tag(&attrs), Some(1));
    }

    #[test]
    fn parse_tag_expr_accepts_positive_literal() {
        let expr: syn::Expr = parse_quote!(5);
        assert_eq!(parse_tag_expr(&expr).unwrap(), 5);
    }

    #[test]
    fn parse_tag_expr_accepts_negative_literal() {
        let expr: syn::Expr = parse_quote!(-10);
        assert_eq!(parse_tag_expr(&expr).unwrap(), -10);
    }

    #[test]
    fn parse_tag_expr_rejects_named_constant() {
        let expr: syn::Expr = parse_quote!(i128::MIN);
        assert!(parse_tag_expr(&expr).is_err());
    }

    #[test]
    fn parse_tag_expr_rejects_negated_named_constant() {
        let expr: syn::Expr = parse_quote!(-i128::MAX);
        assert!(parse_tag_expr(&expr).is_err());
    }

    #[test]
    fn parse_tag_expr_rejects_i128_min_magnitude_as_parse_error() {
        // i128::MIN's magnitude (170141183460469231731687303715884105728) is
        // one past i128::MAX and doesn't fit in a positive i128, so this is a
        // parse error rather than a silently wrong value or an overflow panic.
        let expr: syn::Expr = parse_quote!(-170141183460469231731687303715884105728);
        assert!(parse_tag_expr(&expr).is_err());
    }

    #[test]
    fn resolve_all_implicit_counts_from_zero() {
        assert_eq!(resolve_variant_tags(&[None, None, None]), Ok(vec![0, 1, 2]));
    }

    #[test]
    fn resolve_explicit_resets_counter() {
        // A=5 (explicit), B (implicit, continues from 6), C=2 (explicit), D (implicit, continues from 3)
        let tags = [Some(5), None, Some(2), None];
        assert_eq!(resolve_variant_tags(&tags), Ok(vec![5, 6, 2, 3]));
    }

    #[test]
    fn resolve_gaps_are_allowed() {
        let tags = [Some(0), Some(100)];
        assert_eq!(resolve_variant_tags(&tags), Ok(vec![0, 100]));
    }

    #[test]
    fn resolve_duplicate_tag_errors() {
        let tags = [Some(1), None]; // implicit continues from 1's +1 = 2, no collision...
        assert!(resolve_variant_tags(&tags).is_ok());
        let colliding = [Some(1), Some(1)];
        assert!(resolve_variant_tags(&colliding).is_err());
    }

    #[test]
    fn resolve_negative_explicit_tag_continues_counting_into_positive() {
        // A=-10 (explicit), B (implicit, continues from -10+1 = -9), ...,
        // and an explicit tag crossing back to 0 continues normally from there.
        let tags = [Some(-10), None, None, Some(0), None];
        assert_eq!(resolve_variant_tags(&tags), Ok(vec![-10, -9, -8, 0, 1]));
    }

    #[test]
    fn resolve_negative_explicit_tag_duplicate_with_counted_value_errors() {
        // A=-5 (explicit), B (implicit, continues from -4), ... F (implicit,
        // continues to -1), G=-1 (explicit) collides with F's counted value.
        let tags = [Some(-5), None, None, None, None, Some(-1)];
        assert!(resolve_variant_tags(&tags).is_err());
    }

    #[test]
    fn resolve_all_negative_is_ok() {
        let tags = [Some(-3), None, None];
        assert_eq!(resolve_variant_tags(&tags), Ok(vec![-3, -2, -1]));
    }

    #[test]
    fn resolve_i128_max_single_variant_does_not_panic() {
        // This is the RED case that would previously panic on overflow.
        // Confirms that resolve_variant_tags(&[Some(i128::MAX)]) returns Ok(vec![i128::MAX])
        // without panicking, and without trying to compute i128::MAX + 1.
        assert_eq!(resolve_variant_tags(&[Some(i128::MAX)]), Ok(vec![i128::MAX]));
    }

    #[test]
    fn resolve_i128_max_with_implicit_next_collides() {
        // After resolving i128::MAX for the first variant, the second implicit variant
        // would try to continue from i128::MAX (via checked_add returning None, then
        // unwrap_or(i128::MAX)), which collides with the already-seen i128::MAX.
        // This should produce a clean "duplicate #[byteable(tag = ...)]" error,
        // not a panic.
        let result = resolve_variant_tags(&[Some(i128::MAX), None]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("duplicate"));
    }

    #[test]
    fn reinterpret_i128_for_repr_fits_cleanly_round_trips() {
        assert_eq!(reinterpret_i128_for_repr(5, "u8"), 5);
        assert_eq!(reinterpret_i128_for_repr(-5, "i8"), -5);
    }

    #[test]
    fn reinterpret_i128_for_repr_negative_into_unsigned() {
        // -10 as i8 -> 0xF6 -> 246u8
        assert_eq!(reinterpret_i128_for_repr(-10, "u8"), 246);
    }

    #[test]
    fn reinterpret_i128_for_repr_truncates_too_large_value() {
        // 300 doesn't fit in a u8; as i8 it truncates to 300 - 256 = 44, then as u8 stays 44.
        assert_eq!(reinterpret_i128_for_repr(300, "u8"), 44);
    }

    #[test]
    #[should_panic(expected = "unsupported repr type")]
    fn reinterpret_i128_for_repr_panics_on_unknown_type() {
        reinterpret_i128_for_repr(0, "u24");
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
    fn validate_tags_fit_width_accepts_well_fitting_values() {
        assert!(validate_tags_fit_width(&[0, 1, 255], "u8").is_ok());
    }

    #[test]
    fn validate_tags_fit_width_rejects_overflowing_value() {
        let result = validate_tags_fit_width(&[300], "u8");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("300"));
    }

    #[test]
    fn validate_tags_fit_width_rejects_out_of_range_explicit_tag_even_with_few_variants() {
        // Only 2 variants, so count-based width inference would happily pick u8 - but one of
        // them has an explicit tag = 999, which does not fit in u8. Only checking the actual
        // resolved value (not the variant count) catches this.
        let result = validate_tags_fit_width(&[0, 999], "u8");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("999"));
    }

    #[test]
    fn validate_tags_fit_width_rejects_negative_against_unsigned_narrow_width() {
        assert!(validate_tags_fit_width(&[-10], "u8").is_err());
    }

    #[test]
    fn validate_tags_fit_width_accepts_negative_against_signed_width() {
        assert!(validate_tags_fit_width(&[-10], "i8").is_ok());
    }

    #[test]
    fn validate_tags_fit_width_accepts_negative_against_u128_as_documented_escape_hatch() {
        // See reinterpret_i128_for_repr's doc comment: this is the only way to express
        // the top half of u128's range via `tag`, since `tag` itself is i128-typed.
        assert!(validate_tags_fit_width(&[-10], "u128").is_ok());
    }
}
