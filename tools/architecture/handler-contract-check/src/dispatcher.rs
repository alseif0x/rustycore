// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Parser and exact drift comparison for `WorldSession::dispatch_packet`.

use std::collections::{BTreeMap, BTreeSet};

use syn::{Attribute, Expr, ImplItem, Item, Pat, Stmt, Type};

#[derive(Clone, Copy, Debug)]
pub(crate) struct KnownDispatchDrift {
    pub(crate) opcode_name: &'static str,
    pub(crate) tracking_issue: u32,
}

// These are pre-existing gameplay defects, not accepted architecture. Issue #142
// owns their C++-anchored correction. Exact-set comparison makes this a removal
// ratchet: new drift fails, and a fixed mismatch leaves a stale exception that
// also fails.
pub(crate) const REGISTERED_WITHOUT_DISPATCH_ARM: &[KnownDispatchDrift] = &[KnownDispatchDrift {
    opcode_name: "TrainerBuySpell",
    tracking_issue: 142,
}];
pub(crate) const DISPATCH_ARM_WITHOUT_REGISTRATION: &[KnownDispatchDrift] = &[
    KnownDispatchDrift {
        opcode_name: "MoveSetVehicleRecIdAck",
        tracking_issue: 142,
    },
    KnownDispatchDrift {
        opcode_name: "PartyUninvite",
        tracking_issue: 142,
    },
];
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct DispatcherContract {
    pub(crate) opcode_names: BTreeSet<String>,
}

fn is_path_named(expr: &Expr, name: &str) -> bool {
    matches!(
        expr,
        Expr::Path(path) if path.qself.is_none() && path.path.is_ident(name)
    )
}

fn is_type_named(type_expression: &Type, name: &str) -> bool {
    matches!(
        type_expression,
        Type::Path(path)
            if path.qself.is_none()
                && path.path.segments.last().is_some_and(|segment| segment.ident == name)
    )
}

fn has_conditional_compilation(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr"))
}

fn collect_dispatch_pattern(
    pattern: &Pat,
    opcode_names: &mut BTreeSet<String>,
) -> Result<bool, String> {
    match pattern {
        Pat::Or(or_pattern) => {
            let mut contains_wildcard = false;
            for case in &or_pattern.cases {
                contains_wildcard |= collect_dispatch_pattern(case, opcode_names)?;
            }
            Ok(contains_wildcard)
        }
        Pat::Paren(paren) => collect_dispatch_pattern(&paren.pat, opcode_names),
        Pat::Path(path_pattern) => {
            let segments: Vec<_> = path_pattern.path.segments.iter().collect();
            if path_pattern.qself.is_some()
                || segments.len() < 2
                || segments[segments.len() - 2].ident != "ClientOpcodes"
            {
                return Err("dispatcher path arm is not a ClientOpcodes variant".to_owned());
            }
            let opcode_name = segments
                .last()
                .expect("path with at least two segments")
                .ident
                .to_string();
            if !opcode_names.insert(opcode_name.clone()) {
                return Err(format!(
                    "dispatcher contains duplicate opcode arm {opcode_name}"
                ));
            }
            Ok(false)
        }
        Pat::Wild(_) => Ok(true),
        _ => Err("unsupported top-level dispatch pattern".to_owned()),
    }
}

pub(crate) fn dispatcher_contract_from_source(source: &str) -> Result<DispatcherContract, String> {
    let syntax = syn::parse_file(source)
        .map_err(|error| format!("cannot parse world-session source: {error}"))?;
    let mut dispatch_methods = Vec::new();

    for item in &syntax.items {
        let Item::Impl(item_impl) = item else {
            continue;
        };
        if item_impl.trait_.is_some() || !is_type_named(&item_impl.self_ty, "WorldSession") {
            continue;
        }
        for impl_item in &item_impl.items {
            let ImplItem::Fn(method) = impl_item else {
                continue;
            };
            if method.sig.ident == "dispatch_packet" {
                if has_conditional_compilation(&item_impl.attrs)
                    || has_conditional_compilation(&method.attrs)
                {
                    return Err(
                        "WorldSession::dispatch_packet must not be conditionally compiled"
                            .to_owned(),
                    );
                }
                dispatch_methods.push(method);
            }
        }
    }

    if dispatch_methods.len() != 1 {
        return Err(format!(
            "expected exactly one WorldSession::dispatch_packet method, found {}",
            dispatch_methods.len()
        ));
    }

    let dispatch_matches: Vec<_> = dispatch_methods[0]
        .block
        .stmts
        .iter()
        .filter_map(|statement| {
            let Stmt::Expr(Expr::Match(match_expression), _) = statement else {
                return None;
            };
            is_path_named(&match_expression.expr, "opcode").then_some(match_expression)
        })
        .collect();
    if dispatch_matches.len() != 1 {
        return Err(format!(
            "expected exactly one top-level `match opcode` in dispatch_packet, found {}",
            dispatch_matches.len()
        ));
    }

    let dispatch_match = dispatch_matches[0];
    if has_conditional_compilation(&dispatch_match.attrs) {
        return Err("dispatch_packet opcode match must not be conditionally compiled".to_owned());
    }

    let mut opcode_names = BTreeSet::new();
    let mut wildcard_arms = 0usize;
    for (index, arm) in dispatch_match.arms.iter().enumerate() {
        if has_conditional_compilation(&arm.attrs) {
            return Err(
                "dispatch_packet opcode arms must not be conditionally compiled".to_owned(),
            );
        }
        if arm.guard.is_some() {
            return Err("dispatch_packet opcode arms must not use match guards".to_owned());
        }
        if collect_dispatch_pattern(&arm.pat, &mut opcode_names)? {
            if !matches!(arm.pat, Pat::Wild(_)) {
                return Err("dispatch_packet wildcard must be a standalone `_` arm".to_owned());
            }
            if index + 1 != dispatch_match.arms.len() {
                return Err("dispatch_packet wildcard arm must be last".to_owned());
            }
            wildcard_arms += 1;
        }
    }
    if wildcard_arms != 1 {
        return Err(format!(
            "expected exactly one wildcard dispatcher arm, found {wildcard_arms}"
        ));
    }

    Ok(DispatcherContract { opcode_names })
}

fn known_drift_map(
    label: &str,
    exceptions: &[KnownDispatchDrift],
) -> Result<BTreeMap<String, u32>, String> {
    let mut known = BTreeMap::new();
    for exception in exceptions {
        if exception.tracking_issue == 0 {
            return Err(format!(
                "{label} exception {} has no tracking issue",
                exception.opcode_name
            ));
        }
        if known
            .insert(exception.opcode_name.to_owned(), exception.tracking_issue)
            .is_some()
        {
            return Err(format!(
                "duplicate {label} exception {}",
                exception.opcode_name
            ));
        }
    }
    Ok(known)
}

pub(crate) fn compare_dispatch_sides(
    registered: &BTreeSet<String>,
    dispatched: &BTreeSet<String>,
    registered_without_arm: &[KnownDispatchDrift],
    arm_without_registration: &[KnownDispatchDrift],
) -> Result<(), String> {
    let actual_registered_without_arm: BTreeSet<_> =
        registered.difference(dispatched).cloned().collect();
    let actual_arm_without_registration: BTreeSet<_> =
        dispatched.difference(registered).cloned().collect();
    let known_registered_without_arm =
        known_drift_map("registered-without-arm", registered_without_arm)?;
    let known_arm_without_registration =
        known_drift_map("arm-without-registration", arm_without_registration)?;
    let expected_registered_without_arm: BTreeSet<_> =
        known_registered_without_arm.keys().cloned().collect();
    let expected_arm_without_registration: BTreeSet<_> =
        known_arm_without_registration.keys().cloned().collect();

    let mut errors = Vec::new();
    for opcode in actual_registered_without_arm.difference(&expected_registered_without_arm) {
        errors.push(format!(
            "registered opcode {opcode} has no dispatcher arm and no tracked exception"
        ));
    }
    for opcode in expected_registered_without_arm.difference(&actual_registered_without_arm) {
        errors.push(format!(
            "obsolete registered-without-arm exception {opcode} tracked by #{}",
            known_registered_without_arm[opcode]
        ));
    }
    for opcode in actual_arm_without_registration.difference(&expected_arm_without_registration) {
        errors.push(format!(
            "dispatcher arm {opcode} has no registration and no tracked exception"
        ));
    }
    for opcode in expected_arm_without_registration.difference(&actual_arm_without_registration) {
        errors.push(format!(
            "obsolete arm-without-registration exception {opcode} tracked by #{}",
            known_arm_without_registration[opcode]
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}
