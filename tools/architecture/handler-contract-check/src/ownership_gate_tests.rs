// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F4a-P2: `feature = "test-fixtures"` is test-only, like `test`.
use super::{cfg_context_allows_production, cfg_context_allows_test, cfg_is_test_only};

fn availability(attribute: syn::Attribute) -> (bool, bool) {
    let attributes = [attribute];
    (
        cfg_context_allows_production(&[], &attributes).expect("production cfg evaluates"),
        cfg_context_allows_test(&[], &attributes).expect("test cfg evaluates"),
    )
}

#[test]
fn plain_cfg_test_is_test_only() {
    assert_eq!(availability(syn::parse_quote!(#[cfg(test)])), (false, true));
    assert!(cfg_is_test_only("cfg (test)"));
}

#[test]
fn test_fixtures_gate_is_test_only() {
    assert_eq!(
        availability(syn::parse_quote!(#[cfg(any(test, feature = "test-fixtures"))])),
        (false, true)
    );
    assert!(cfg_is_test_only(
        "cfg (any (test , feature = \"test-fixtures\"))"
    ));
    assert!(cfg_is_test_only("cfg (feature = \"test-fixtures\")"));
}

#[test]
fn any_with_a_production_arm_stays_production() {
    assert_eq!(
        availability(syn::parse_quote!(#[cfg(any(test, feature = "test-fixtures", unix))])),
        (true, true)
    );
    assert!(!cfg_is_test_only(
        "cfg (any (test , feature = \"test-fixtures\" , unix))"
    ));
    assert_eq!(
        availability(syn::parse_quote!(#[cfg(any(test, feature = "other"))])),
        (true, true)
    );
}

#[test]
fn negated_gate_is_production() {
    assert_eq!(
        availability(syn::parse_quote!(#[cfg(not(any(test, feature = "test-fixtures")))])),
        (true, false)
    );
    assert!(!cfg_is_test_only(
        "cfg (not (any (test , feature = \"test-fixtures\")))"
    ));
}
