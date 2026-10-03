// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Exact source grammar for the Inventory-owned direct `RegistryBuilder` registrar.

use std::collections::BTreeSet;
use std::path::Path;

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::visit::Visit;
use syn::{
    Expr, FnArg, Item, ItemFn, Pat, Stmt, Type, TypeParamBound, UseTree,
    Visibility, WherePredicate,
};

use super::ident_is;

const INVENTORY_PACKAGE: &str = "wow-world-inventory";
const INVENTORY_MODULE: &str = "crate::handlers::equipment_sets";
const REGISTRAR_NAME: &str = "register_inventory_handlers_like_cpp";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct RegistrarReport {
    pub(crate) entries: usize,
    pub(crate) registrar_count: usize,
}

#[derive(Default)]
struct ImportBindings {
    packet_handler_entry: usize,
    registry_builder: usize,
    duplicate_error: usize,
    errors: Vec<String>,
}

fn collect_use_tree(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    leaves: &mut Vec<(Vec<String>, String, bool)>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_tree(&path.tree, prefix, leaves);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let mut path = prefix.clone();
            path.push(name.ident.to_string());
            leaves.push((path, name.ident.to_string(), false));
        }
        UseTree::Rename(rename) => {
            let mut path = prefix.clone();
            path.push(rename.ident.to_string());
            leaves.push((path, rename.rename.to_string(), true));
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree(item, prefix, leaves);
            }
        }
        UseTree::Glob(_) => {}
    }
}

fn token_stream_mentions_ident(tokens: &TokenStream, expected: &str) -> bool {
    tokens.clone().into_iter().any(|token| match token {
        TokenTree::Ident(ident) => ident_is(&ident, expected),
        TokenTree::Group(group) => token_stream_mentions_ident(&group.stream(), expected),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

fn inspect_imports(items: &[Item]) -> Result<ImportBindings, String> {
    let mut bindings = ImportBindings::default();
    for item in items {
        let Item::Use(item_use) = item else {
            continue;
        };
        let mut leaves = Vec::new();
        collect_use_tree(&item_use.tree, &mut Vec::new(), &mut leaves);
        for (path, local, renamed) in leaves {
            let Some(provider_type) = path.last() else {
                continue;
            };
            let binding = match provider_type.as_str() {
                "PacketHandlerEntry" => Some(("PacketHandlerEntry", &mut bindings.packet_handler_entry)),
                "RegistryBuilder" => Some(("RegistryBuilder", &mut bindings.registry_builder)),
                "DuplicateHandlerRegistrationLikeCpp" => {
                    Some(("DuplicateHandlerRegistrationLikeCpp", &mut bindings.duplicate_error))
                }
                _ => None,
            };
            let Some((expected, count)) = binding else {
                continue;
            };
            *count += 1;
            let canonical = path.len() == 2
                && path[0] == "wow_handler"
                && path[1] == expected
                && local == expected
                && !renamed
                && matches!(&item_use.vis, Visibility::Inherited)
                && item_use.attrs.is_empty();
            if !canonical {
                bindings.errors.push(format!(
                    "direct handler registration must import {expected} unrenamed from wow_handler"
                ));
            }
        }
    }
    for (name, count) in [
        ("PacketHandlerEntry", bindings.packet_handler_entry),
        ("RegistryBuilder", bindings.registry_builder),
        ("DuplicateHandlerRegistrationLikeCpp", bindings.duplicate_error),
    ] {
        if count != 1 {
            bindings.errors.push(format!(
                "direct handler registrar requires exactly one canonical wow_handler::{name} import; found {count}"
            ));
        }
    }
    if bindings.errors.is_empty() {
        Ok(bindings)
    } else {
        Err(bindings.errors.join("; "))
    }
}

struct EntryAndRegisterVisitor {
    entry_names: BTreeSet<String>,
    entry_literals: usize,
    register_calls: usize,
    all_register_calls: usize,
}

impl EntryAndRegisterVisitor {
    fn new(entry_names: BTreeSet<String>) -> Self {
        Self {
            entry_names,
            entry_literals: 0,
            register_calls: 0,
            all_register_calls: 0,
        }
    }
}

struct EntryLiteralFinder<'a> {
    entry_names: &'a BTreeSet<String>,
    found: bool,
}

impl<'ast> Visit<'ast> for EntryLiteralFinder<'_> {
    fn visit_expr_struct(&mut self, expression: &'ast syn::ExprStruct) {
        if expression.path.segments.last().is_some_and(|segment| {
            self.entry_names.contains(&segment.ident.to_string())
        }) {
            self.found = true;
        }
        syn::visit::visit_expr_struct(self, expression);
    }
}

impl<'ast> Visit<'ast> for EntryAndRegisterVisitor {
    fn visit_expr_struct(&mut self, expression: &'ast syn::ExprStruct) {
        if expression.path.segments.last().is_some_and(|segment| {
            self.entry_names.contains(&segment.ident.to_string())
        }) {
            self.entry_literals += 1;
        }
        syn::visit::visit_expr_struct(self, expression);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if ident_is(&call.method, "register") {
            self.all_register_calls += 1;
        }
        let mut argument_contains_entry = EntryLiteralFinder {
            entry_names: &self.entry_names,
            found: false,
        };
        for argument in &call.args {
            argument_contains_entry.visit_expr(argument);
        }
        if ident_is(&call.method, "register") && argument_contains_entry.found {
            self.register_calls += 1;
        }
        syn::visit::visit_expr_method_call(self, call);
    }
}

fn entry_type_bindings(items: &[Item]) -> BTreeSet<String> {
    let mut names = BTreeSet::from(["PacketHandlerEntry".to_owned()]);
    for item in items {
        let Item::Use(item_use) = item else {
            continue;
        };
        let mut leaves = Vec::new();
        collect_use_tree(&item_use.tree, &mut Vec::new(), &mut leaves);
        for (path, local, _) in leaves {
            if path.last().is_some_and(|name| name == "PacketHandlerEntry") {
                names.insert(local);
            }
        }
    }

    loop {
        let mut added = false;
        for item in items {
            let Item::Type(alias) = item else {
                continue;
            };
            struct ReferencedEntry<'a> {
                names: &'a BTreeSet<String>,
                found: bool,
            }
            impl<'ast> Visit<'ast> for ReferencedEntry<'_> {
                fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
                    if path.path.segments.last().is_some_and(|segment| {
                        self.names.contains(&segment.ident.to_string())
                    }) {
                        self.found = true;
                    }
                    syn::visit::visit_type_path(self, path);
                }
            }
            let mut visitor = ReferencedEntry {
                names: &names,
                found: false,
            };
            visitor.visit_type(&alias.ty);
            if visitor.found {
                added |= names.insert(alias.ident.to_string());
            }
        }
        if !added {
            break;
        }
    }
    names
}

fn path_has_only_segment(path: &syn::Path, expected: &str) -> bool {
    path.leading_colon.is_none()
        && path.segments.len() == 1
        && path
            .segments
            .first()
            .is_some_and(|segment| ident_is(&segment.ident, expected))
}

fn type_arguments(ty: &Type) -> Option<Vec<&syn::GenericArgument>> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    Some(arguments.args.iter().collect())
}

fn is_builder_parameter(argument: &FnArg) -> bool {
    let FnArg::Typed(argument) = argument else {
        return false;
    };
    if !argument.attrs.is_empty() {
        return false;
    }
    let Pat::Ident(pattern) = &*argument.pat else {
        return false;
    };
    if !pattern.attrs.is_empty()
        || !ident_is(&pattern.ident, "builder")
        || pattern.by_ref.is_some()
        || pattern.subpat.is_some()
    {
        return false;
    }
    let Type::Reference(reference) = &*argument.ty else {
        return false;
    };
    if reference.mutability.is_none()
        || reference.lifetime.is_some()
    {
        return false;
    }
    let Type::Path(builder_path) = &*reference.elem else {
        return false;
    };
    if builder_path.qself.is_some()
        || !path_has_only_segment(&builder_path.path, "RegistryBuilder")
    {
        return false;
    }
    let Some(arguments) = type_arguments(&reference.elem) else {
        return false;
    };
    arguments.len() == 2
        && matches!(&arguments[0], syn::GenericArgument::Type(Type::Path(path)) if path.qself.is_none() && path.path.is_ident("S"))
        && matches!(&arguments[1], syn::GenericArgument::Type(Type::Path(path)) if path.qself.is_none() && path.path.is_ident("C"))
}

fn is_registrar_signature(function: &ItemFn) -> bool {
    function.attrs.is_empty()
        && matches!(&function.vis, Visibility::Public(_))
        && ident_is(&function.sig.ident, REGISTRAR_NAME)
        && function.sig.constness.is_none()
        && function.sig.asyncness.is_none()
        && function.sig.unsafety.is_none()
        && function.sig.abi.is_none()
        && function.sig.variadic.is_none()
        && function.sig.generics.params.len() == 2
        && function.sig.generics.params.iter().enumerate().all(|(index, parameter)| {
            matches!(parameter, syn::GenericParam::Type(parameter)
                if parameter.attrs.is_empty()
                    && parameter.bounds.is_empty()
                    && parameter.default.is_none()
                    && ident_is(&parameter.ident, if index == 0 { "S" } else { "C" }))
        })
        && function.sig.generics.where_clause.is_some()
        && is_host_where_clause(function.sig.generics.where_clause.as_ref())
        && function.sig.inputs.len() == 1
        && is_builder_parameter(&function.sig.inputs[0])
        && is_registrar_output(&function.sig.output)
}

fn is_plain_type_path(ty: &Type, expected: &str) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.qself.is_none() && path_has_only_segment(&path.path, expected)
}

fn where_type_name(predicate: &syn::PredicateType, expected: &str) -> bool {
    predicate.lifetimes.is_none()
        && is_plain_type_path(&predicate.bounded_ty, expected)
}

fn trait_bound_name(bound: &TypeParamBound, expected: &str) -> bool {
    matches!(bound, TypeParamBound::Trait(bound)
        if bound.paren_token.is_none()
            && bound.modifier == syn::TraitBoundModifier::None
            && bound.lifetimes.is_none()
            && bound.path.leading_colon.is_none()
            && bound.path.segments.len() == 1
            && bound.path.segments.first().is_some_and(|segment| ident_is(&segment.ident, expected)))
}

fn is_host_trait_bound(bound: &TypeParamBound) -> bool {
    let TypeParamBound::Trait(bound) = bound else {
        return false;
    };
    if bound.paren_token.is_some()
        || bound.modifier != syn::TraitBoundModifier::None
        || bound.lifetimes.is_some()
        || bound.path.leading_colon.is_some()
        || bound.path.segments.len() != 1
    {
        return false;
    }
    let segment = &bound.path.segments[0];
    if !ident_is(&segment.ident, "InventoryHandlerHostLikeCpp") {
        return false;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return false;
    };
    arguments.args.len() == 1
        && matches!(arguments.args.first(), Some(syn::GenericArgument::Type(Type::Path(path)))
            if path.qself.is_none() && path.path.is_ident("C"))
}

fn is_host_where_clause(clause: Option<&syn::WhereClause>) -> bool {
    let Some(clause) = clause else {
        return false;
    };
    let predicates: Vec<_> = clause.predicates.iter().collect();
    if predicates.len() != 2 {
        return false;
    }
    let WherePredicate::Type(session) = predicates[0] else {
        return false;
    };
    let WherePredicate::Type(catalogs) = predicates[1] else {
        return false;
    };
    where_type_name(session, "S")
        && session.bounds.len() == 2
        && session.bounds.first().is_some_and(is_host_trait_bound)
        && session
            .bounds
            .iter()
            .nth(1)
            .is_some_and(|bound| trait_bound_name(bound, "Send"))
        && where_type_name(catalogs, "C")
        && catalogs.bounds.len() == 1
        && catalogs.bounds.iter().all(|bound| trait_bound_name(bound, "Sync"))
}

fn is_registrar_output(output: &syn::ReturnType) -> bool {
    let syn::ReturnType::Type(_, ty) = output else {
        return false;
    };
    let Type::Path(result) = &**ty else {
        return false;
    };
    if result.qself.is_some()
        || !path_has_only_segment(&result.path, "Result")
    {
        return false;
    }
    let Some(arguments) = type_arguments(ty) else {
        return false;
    };
    arguments.len() == 2
        && matches!(&arguments[0], syn::GenericArgument::Type(Type::Tuple(tuple)) if tuple.elems.is_empty())
        && matches!(&arguments[1], syn::GenericArgument::Type(Type::Path(path))
            if path.qself.is_none()
                && path.path.leading_colon.is_none()
                && path.path.is_ident("DuplicateHandlerRegistrationLikeCpp"))
}

fn is_builder_receiver(expression: &Expr) -> bool {
    matches!(expression, Expr::Path(path)
        if path.attrs.is_empty()
            && path.qself.is_none()
            && path.path.leading_colon.is_none()
            && path.path.segments.len() == 1
            && path.path.segments.first().is_some_and(|segment| ident_is(&segment.ident, "builder")))
}

fn is_entry_literal(expression: &Expr) -> bool {
    let Expr::Struct(entry) = expression else {
        return false;
    };
    let fields = ["opcode", "status", "processing", "handler_name", "handler"];
    entry.attrs.is_empty()
        && entry.qself.is_none()
        && entry.path.leading_colon.is_none()
        && entry.path.segments.len() == 1
        && entry
            .path
            .segments
            .first()
            .is_some_and(|segment| {
                ident_is(&segment.ident, "PacketHandlerEntry")
                    && matches!(&segment.arguments, syn::PathArguments::None)
            })
        && entry.rest.is_none()
        && entry.fields.len() == fields.len()
        && entry.fields.iter().zip(fields).all(|(field, expected)| {
            matches!(&field.member, syn::Member::Named(name) if ident_is(name, expected))
                && field.attrs.is_empty()
        })
}

fn registration_entry(statement: &Stmt) -> Option<&syn::ExprStruct> {
    let Stmt::Expr(expression, Some(_)) = statement else {
        return None;
    };
    let Expr::Try(try_expression) = expression else {
        return None;
    };
    if !try_expression.attrs.is_empty() {
        return None;
    }
    let Expr::MethodCall(call) = &*try_expression.expr else {
        return None;
    };
    if !ident_is(&call.method, "register")
        || call.turbofish.is_some()
        || !call.attrs.is_empty()
        || !is_builder_receiver(&call.receiver)
        || call.args.len() != 1
        || !is_entry_literal(&call.args[0])
    {
        return None;
    }
    let Expr::Struct(entry) = &call.args[0] else {
        return None;
    };
    Some(entry)
}

fn is_ok_unit_tail(statement: &Stmt) -> bool {
    let Stmt::Expr(expression, None) = statement else {
        return false;
    };
    matches!(expression, Expr::Call(call)
        if call.attrs.is_empty()
            && matches!(&*call.func, Expr::Path(path)
            if path.qself.is_none()
                && path.path.leading_colon.is_none()
                && path.path.is_ident("Ok")
                && path.attrs.is_empty())
            && call.args.len() == 1
            && matches!(call.args.first(), Some(Expr::Tuple(tuple)) if tuple.elems.is_empty() && tuple.attrs.is_empty()))
}

fn opcode_key(entry: &syn::ExprStruct) -> Option<String> {
    entry.fields.iter().find_map(|field| {
        if !matches!(&field.member, syn::Member::Named(name) if ident_is(name, "opcode")) {
            return None;
        }
        let Expr::Path(path) = &field.expr else {
            return None;
        };
        (path.attrs.is_empty()
            && path.qself.is_none()
            && path.path.leading_colon.is_none()
            && path.path.segments.len() == 2
            && path.path.segments.first().is_some_and(|segment| ident_is(&segment.ident, "ClientOpcodes")))
        .then(|| path.to_token_stream().to_string())
    })
}

fn analyze_registrar(function: &ItemFn) -> Result<usize, String> {
    if !is_registrar_signature(function) {
        return Err("Inventory handler registrar has an unexpected signature or attributes".to_owned());
    }
    let Some((tail, registrations)) = function.block.stmts.split_last() else {
        return Err("Inventory handler registrar is empty".to_owned());
    };
    if !is_ok_unit_tail(tail) {
        return Err("Inventory handler registrar must end with Ok(())".to_owned());
    }
    let mut opcodes = BTreeSet::new();
    for statement in registrations {
        let Some(entry) = registration_entry(statement) else {
            return Err(
                "Inventory handler registrar permits only direct builder.register(PacketHandlerEntry { ... })? statements before Ok(())"
                    .to_owned(),
            );
        };
        let opcode = opcode_key(entry).ok_or_else(|| {
            "Inventory PacketHandlerEntry opcode must be a ClientOpcodes path".to_owned()
        })?;
        if !opcodes.insert(opcode.clone()) {
            return Err(format!("duplicate Inventory handler opcode entry {opcode}"));
        }
    }
    if opcodes.is_empty() {
        return Err("Inventory handler registrar contains no direct entries".to_owned());
    }
    Ok(opcodes.len())
}

/// Classify the one Inventory registrar only at its authorized logical module.
pub(crate) fn analyze_owner_source(
    package: &str,
    logical_module: &str,
    source_path: &Path,
    source: &str,
) -> Result<RegistrarReport, String> {
    let syntax = syn::parse_file(source)
        .map_err(|error| format!("cannot parse {}: {error}", source_path.display()))?;
    let registrar_items: Vec<_> = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Fn(function) if ident_is(&function.sig.ident, REGISTRAR_NAME) => Some(function),
            _ => None,
        })
        .collect();
    let entry_names = entry_type_bindings(&syntax.items);
    let mut occurrences = EntryAndRegisterVisitor::new(entry_names);
    occurrences.visit_file(&syntax);
    if registrar_items.is_empty() {
        if occurrences.entry_literals != 0 || occurrences.register_calls != 0 {
            return Err(format!(
                "direct PacketHandlerEntry or builder.register source is outside the exact Inventory registrar in {} ({logical_module})",
                source_path.display()
            ));
        }
        return Ok(RegistrarReport::default());
    }
    if package != INVENTORY_PACKAGE || logical_module != INVENTORY_MODULE {
        return Err(format!(
            "direct Inventory handler registrar is outside {INVENTORY_PACKAGE}::{INVENTORY_MODULE}: {} ({logical_module})",
            source_path.display()
        ));
    }
    if registrar_items.len() != 1 {
        return Err("Inventory handler registrar must be defined exactly once".to_owned());
    }
    inspect_imports(&syntax.items)?;
    for item in &syntax.items {
        if matches!(item, Item::Use(_))
            || matches!(item, Item::Fn(function) if ident_is(&function.sig.ident, REGISTRAR_NAME))
        {
            continue;
        }
        let tokens = item.to_token_stream();
        if token_stream_mentions_ident(&tokens, "PacketHandlerEntry")
            || token_stream_mentions_ident(&tokens, "RegistryBuilder")
        {
            return Err(
                "Inventory handler entry types may appear only in the exact top-level registrar"
                    .to_owned(),
            );
        }
    }
    let entries = analyze_registrar(registrar_items[0])?;
    if occurrences.entry_literals != entries
        || occurrences.register_calls != entries
        || occurrences.all_register_calls != entries
    {
        return Err(format!(
            "Inventory source contains {} PacketHandlerEntry literals, {} entry-bearing register calls, and {} total register calls, but the exact registrar accounts for {entries}",
            occurrences.entry_literals, occurrences.register_calls, occurrences.all_register_calls
        ));
    }
    Ok(RegistrarReport {
        entries,
        registrar_count: 1,
    })
}

/// Reject a direct generic handler entry constructed outside an authorized registrar.
pub(crate) fn unowned_entry_literal_violation(source: &str) -> Result<Option<String>, String> {
    let syntax = syn::parse_file(source).map_err(|error| format!("cannot parse source: {error}"))?;
    let entry_names = entry_type_bindings(&syntax.items);
    let mut occurrences = EntryAndRegisterVisitor::new(entry_names);
    occurrences.visit_file(&syntax);
    if occurrences.entry_literals == 0 && occurrences.register_calls == 0 {
        return Ok(None);
    }
    Ok(Some(format!(
        "{} PacketHandlerEntry struct literal(s) and {} generic register call(s) appear outside an authorized direct registrar",
        occurrences.entry_literals, occurrences.register_calls
    )))
}
