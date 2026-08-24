// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Parser for `WorldSession::dispatch_packet`.
//!
//! Until #359 the dispatcher was the second declaration of every opcode: an
//! arm here and a `PacketHandlerEntry` there, compared as exact sets so a
//! one-sided opcode failed instead of silently dropping packets. The
//! registration now carries the call, so there is one declaration per opcode
//! and nothing left to compare.
//!
//! What this parses is therefore the inverse property: the dispatcher must
//! name no opcode and no handler method, and must perform the registered call.
//! Reintroducing either side fails here rather than being tolerated as drift.

use std::collections::BTreeSet;

use syn::visit::Visit;
use syn::{Expr, ImplItem, ImplItemFn, Item, Pat, Type};

use crate::module_policy::CapabilityOwner;
use crate::ownership::{
    WorkspaceSourceMount, cfg_context_allows_production, cfg_context_controls_presence,
    extend_cfg_context,
};

#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct DispatcherContract {
    /// Opcodes the dispatcher still decides by hand. Must be empty (#359).
    pub(crate) opcode_names: BTreeSet<String>,
    /// Handler methods the dispatcher calls on `self`. Must be empty (#359).
    pub(crate) handler_calls: BTreeSet<String>,
    /// Whether the dispatcher performs the call the registration carries.
    pub(crate) dispatches_through_registration: bool,
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

/// Walk one method body for every shape #359 retired, plus the one it kept.
struct DispatchBodyScan {
    opcode_names: BTreeSet<String>,
    handler_calls: BTreeSet<String>,
    dispatches_through_registration: bool,
    errors: Vec<String>,
}

impl<'ast> Visit<'ast> for DispatchBodyScan {
    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        if is_path_named(&node.expr, "opcode") {
            for arm in &node.arms {
                if let Err(error) = collect_dispatch_pattern(&arm.pat, &mut self.opcode_names) {
                    // A pattern this parser cannot read is still an opcode
                    // decision taken here; record it rather than skipping it.
                    self.errors.push(error);
                }
            }
        }
        syn::visit::visit_expr_match(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if is_path_named(&node.receiver, "self") && node.method.to_string().starts_with("handle_") {
            self.handler_calls.insert(node.method.to_string());
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Expr::Paren(paren) = node.func.as_ref()
            && let Expr::Field(field) = paren.expr.as_ref()
            && is_path_named(&field.base, "entry")
            && matches!(&field.member, syn::Member::Named(name) if name == "handler")
        {
            self.dispatches_through_registration = true;
        }
        syn::visit::visit_expr_call(self, node);
    }
}

fn dispatcher_contract_from_method(method: &ImplItemFn) -> Result<DispatcherContract, String> {
    let mut scan = DispatchBodyScan {
        opcode_names: BTreeSet::new(),
        handler_calls: BTreeSet::new(),
        dispatches_through_registration: false,
        errors: Vec::new(),
    };
    scan.visit_block(&method.block);
    if !scan.errors.is_empty() {
        return Err(scan.errors.join("; "));
    }
    Ok(DispatcherContract {
        opcode_names: scan.opcode_names,
        handler_calls: scan.handler_calls,
        dispatches_through_registration: scan.dispatches_through_registration,
    })
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

/// Reject any return to two declarations per opcode.
///
/// #359's property is negative, so it is stated as one: the dispatcher decides
/// nothing per opcode and names no handler, and the call it does make is the
/// one the registration carries. A reintroduced arm or a direct `self.handle_*`
/// call fails here instead of quietly becoming a second source of truth again.
pub(crate) fn assert_single_dispatch_mechanism(
    contract: &DispatcherContract,
) -> Result<(), String> {
    let mut errors = Vec::new();
    if !contract.opcode_names.is_empty() {
        errors.push(format!(
            "dispatch_packet decides {} opcode(s) by hand ({}); an opcode is declared once, \
             in its PacketHandlerEntry",
            contract.opcode_names.len(),
            contract
                .opcode_names
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !contract.handler_calls.is_empty() {
        errors.push(format!(
            "dispatch_packet calls {} handler method(s) on self ({}); the registration carries \
             the call",
            contract.handler_calls.len(),
            contract
                .handler_calls
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !contract.dispatches_through_registration {
        errors.push(
            "dispatch_packet never calls the registered handler; the single mechanism is gone"
                .to_owned(),
        );
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}
