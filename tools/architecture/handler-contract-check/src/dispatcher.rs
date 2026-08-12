// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Parser and exact drift comparison for `WorldSession::dispatch_packet`.

use std::collections::{BTreeMap, BTreeSet};

use syn::{Expr, ImplItem, ImplItemFn, Item, Pat, Stmt, Type};

use crate::module_policy::CapabilityOwner;
use crate::ownership::{
    WorkspaceSourceMount, cfg_context_allows_production, cfg_context_controls_presence,
    extend_cfg_context,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct KnownDispatchDrift {
    pub(crate) opcode_name: &'static str,
    pub(crate) tracking_issue: u32,
}

// Exact-set comparison makes this a drift ratchet: a new one-sided opcode
// fails until its gameplay defect is repaired rather than silently tolerated.
pub(crate) const REGISTERED_WITHOUT_DISPATCH_ARM: &[KnownDispatchDrift] = &[];
pub(crate) const DISPATCH_ARM_WITHOUT_REGISTRATION: &[KnownDispatchDrift] = &[];
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct DispatcherContract {
    pub(crate) opcode_names: BTreeSet<String>,
}

#[derive(Debug)]
struct LocatedDispatcher {
    contract: DispatcherContract,
    logical_module: String,
    source_path: String,
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

fn resolved_self_type_module(type_expression: &Type, logical_module: &str) -> Option<String> {
    let Type::Path(type_path) = type_expression else {
        return None;
    };
    if type_path.qself.is_some() {
        return None;
    }
    let mut segments: Vec<_> = type_path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    if segments.last().map(String::as_str) != Some("WorldSession") {
        return None;
    }
    segments.pop();
    if segments.is_empty() {
        return Some(logical_module.to_owned());
    }

    let mut resolved: Vec<String> = logical_module.split("::").map(str::to_owned).collect();
    match segments.first().map(String::as_str) {
        Some("crate") => resolved = vec!["crate".to_owned()],
        Some("self") => {}
        Some("super") => {}
        _ => return None,
    }
    for segment in segments {
        match segment.as_str() {
            "crate" | "self" => {}
            "super" => {
                if resolved.len() == 1 {
                    return None;
                }
                resolved.pop();
            }
            _ => resolved.push(segment),
        }
    }
    Some(resolved.join("::"))
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

fn dispatch_methods_in_items<'a>(items: &'a [Item]) -> Vec<(&'a syn::ItemImpl, &'a ImplItemFn)> {
    let mut dispatch_methods = Vec::new();
    for item in items {
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
                dispatch_methods.push((item_impl, method));
            }
        }
    }
    dispatch_methods
}

fn dispatcher_contract_from_method(method: &ImplItemFn) -> Result<DispatcherContract, String> {
    let dispatch_matches: Vec<_> = method
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
    if cfg_context_controls_presence(&[], &dispatch_match.attrs)? {
        return Err("dispatch_packet opcode match must not be conditionally compiled".to_owned());
    }

    let mut opcode_names = BTreeSet::new();
    let mut wildcard_arms = 0usize;
    for (index, arm) in dispatch_match.arms.iter().enumerate() {
        if cfg_context_controls_presence(&[], &arm.attrs)? {
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

#[cfg(test)]
pub(crate) fn dispatcher_contract_from_source(source: &str) -> Result<DispatcherContract, String> {
    let syntax = syn::parse_file(source)
        .map_err(|error| format!("cannot parse world-session source: {error}"))?;
    let dispatch_methods = dispatch_methods_in_items(&syntax.items);

    if dispatch_methods.len() != 1 {
        return Err(format!(
            "expected exactly one WorldSession::dispatch_packet method, found {}",
            dispatch_methods.len()
        ));
    }
    let (item_impl, method) = dispatch_methods[0];
    if cfg_context_controls_presence(&[], &item_impl.attrs)?
        || cfg_context_controls_presence(&[], &method.attrs)?
    {
        return Err("WorldSession::dispatch_packet must not be conditionally compiled".to_owned());
    }
    dispatcher_contract_from_method(method)
}

fn collect_module_dispatchers(
    items: &[Item],
    source_path: &str,
    package: &str,
    logical_module: &str,
    inherited_cfg: &[String],
    owner: &CapabilityOwner,
    dispatchers: &mut Vec<LocatedDispatcher>,
) -> Result<(), String> {
    for (item_impl, method) in dispatch_methods_in_items(items) {
        let impl_cfg = extend_cfg_context(inherited_cfg, &item_impl.attrs);
        let method_cfg = extend_cfg_context(&impl_cfg, &method.attrs);
        if !cfg_context_allows_production(&method_cfg, &[])? {
            continue;
        }
        if cfg_context_controls_presence(&impl_cfg, &method.attrs)? {
            return Err(format!(
                "WorldSession::dispatch_packet in {source_path} ({logical_module}) has conditional module/impl/method ownership: {method_cfg:?}"
            ));
        }
        if !owner.owns_module(package, logical_module) {
            return Err(format!(
                "WorldSession::dispatch_packet in {source_path} is owned by logical module {logical_module}, outside declared capability owner {}::{}",
                owner.package, owner.module
            ));
        }
        if resolved_self_type_module(&item_impl.self_ty, logical_module).as_deref()
            != Some(owner.module.as_str())
        {
            return Err(format!(
                "dispatch_packet in {source_path} ({logical_module}) does not implement the canonical {}::WorldSession; private child modules must use an explicit self/super/crate path",
                owner.module
            ));
        }
        let contract = dispatcher_contract_from_method(method)?;
        dispatchers.push(LocatedDispatcher {
            contract,
            logical_module: logical_module.to_owned(),
            source_path: source_path.to_owned(),
        });
    }

    for item in items {
        let Item::Mod(module) = item else {
            continue;
        };
        let Some((_, inline_items)) = &module.content else {
            continue;
        };
        let child_cfg = extend_cfg_context(inherited_cfg, &module.attrs);
        if !cfg_context_allows_production(&child_cfg, &[])? {
            continue;
        }
        let child_logical_module = format!("{logical_module}::{}", module.ident);
        collect_module_dispatchers(
            inline_items,
            source_path,
            package,
            &child_logical_module,
            &child_cfg,
            owner,
            dispatchers,
        )?;
    }
    Ok(())
}

fn collect_world_session_definitions(
    items: &[Item],
    source_path: &str,
    logical_module: &str,
    inherited_cfg: &[String],
    definitions: &mut Vec<String>,
) -> Result<(), String> {
    for item in items {
        if let Item::Struct(item_struct) = item
            && item_struct.ident == "WorldSession"
            && cfg_context_allows_production(inherited_cfg, &item_struct.attrs)?
        {
            if cfg_context_controls_presence(inherited_cfg, &item_struct.attrs)? {
                return Err(format!(
                    "WorldSession definition in {source_path} ({logical_module}) is conditionally owned"
                ));
            }
            definitions.push(format!("{logical_module} ({source_path})"));
        }
        let Item::Mod(module) = item else {
            continue;
        };
        let Some((_, inline_items)) = &module.content else {
            continue;
        };
        let child_cfg = extend_cfg_context(inherited_cfg, &module.attrs);
        if cfg_context_allows_production(&child_cfg, &[])? {
            collect_world_session_definitions(
                inline_items,
                source_path,
                &format!("{logical_module}::{}", module.ident),
                &child_cfg,
                definitions,
            )?;
        }
    }
    Ok(())
}

/// Find the sole concrete dispatcher through the already validated workspace
/// module graph, independent of its physical source filename.
pub(crate) fn dispatcher_contract_from_mounts(
    mounts: &[WorkspaceSourceMount],
    owner: &CapabilityOwner,
) -> Result<DispatcherContract, String> {
    let mut dispatchers = Vec::new();
    let mut definitions = Vec::new();
    for mount in mounts.iter().filter(|mount| mount.package == owner.package) {
        let syntax = syn::parse_file(&mount.source)
            .map_err(|error| format!("cannot parse {}: {error}", mount.source_path.display()))?;
        for context in mount
            .contexts
            .iter()
            .filter(|context| context.production_possible)
        {
            collect_world_session_definitions(
                &syntax.items,
                &mount.source_path.display().to_string(),
                &context.logical_module_path,
                &context.cfg,
                &mut definitions,
            )?;
            collect_module_dispatchers(
                &syntax.items,
                &mount.source_path.display().to_string(),
                &mount.package,
                &context.logical_module_path,
                &context.cfg,
                owner,
                &mut dispatchers,
            )?;
        }
    }
    let expected_definition_prefix = format!("{} (", owner.module);
    if definitions.len() != 1 || !definitions[0].starts_with(&expected_definition_prefix) {
        return Err(format!(
            "expected exactly one canonical {}::WorldSession definition and no production homonyms, found {}: {:?}",
            owner.module,
            definitions.len(),
            definitions
        ));
    }
    if dispatchers.len() != 1 {
        let locations: Vec<_> = dispatchers
            .iter()
            .map(|dispatcher| format!("{} ({})", dispatcher.source_path, dispatcher.logical_module))
            .collect();
        return Err(format!(
            "expected exactly one production WorldSession::dispatch_packet owner, found {}{}",
            dispatchers.len(),
            if locations.is_empty() {
                String::new()
            } else {
                format!(": {}", locations.join(", "))
            }
        ));
    }
    Ok(dispatchers.pop().expect("one dispatcher").contract)
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
