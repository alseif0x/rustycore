// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Exact production and fixture composition sites for World packet handlers.

use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::{Expr, Item, ItemFn, ItemUse, Meta, Stmt, Token, Type, UseTree, Visibility};

use crate::ownership::{SourceMountContext, WorkspaceSourceMount};

const REGISTRAR: &str = "register_inventory_handlers_like_cpp";
const INVENTORY_PACKAGE: &str = "wow-world-inventory";
const ROOT_FACADE_EXPORTS: &[&str] = &[
    "EquipmentSetsHandlerCxLikeCpp",
    "InventoryHandlerHostLikeCpp",
    "EquipmentSetsSaveCxLikeCpp",
    REGISTRAR,
];
const HANDLER_FACADE_EXPORTS: &[&str] = &[
    "EquipmentSetsHandlerCxLikeCpp",
    "InventoryHandlerHostLikeCpp",
    REGISTRAR,
];

fn path_is(path: &syn::Path, expected: &[&str]) -> bool {
    path.leading_colon.is_none()
        && path.segments.len() == expected.len()
        && path
            .segments
            .iter()
            .zip(expected)
            .all(|(segment, expected)| segment.ident.to_string() == *expected)
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

fn has_exact_legacy_import(items: &[Item]) -> bool {
    let mut matching = 0usize;
    let mut invalid = false;
    for item in items {
        let Item::Use(item_use) = item else {
            continue;
        };
        let mut leaves = Vec::new();
        collect_use_tree(&item_use.tree, &mut Vec::new(), &mut leaves);
        for (path, local, renamed) in leaves {
            if path.last().is_none_or(|name| name != "register_remaining_handlers_like_cpp") {
                continue;
            }
            matching += 1;
            let expected = [
                "wow_world",
                "session",
                "registry",
                "register_remaining_handlers_like_cpp",
            ];
            invalid |= path.len() != expected.len()
                || !path.iter().zip(expected).all(|(actual, expected)| actual == expected)
                || local != "register_remaining_handlers_like_cpp"
                || renamed
                || !item_use.attrs.is_empty()
                || !matches!(&item_use.vis, Visibility::Inherited);
        }
    }
    matching == 1 && !invalid
}

fn is_plain_path(ty: &Type, expected: &[&str]) -> bool {
    matches!(ty, Type::Path(path)
        if path.qself.is_none() && path_is(&path.path, expected))
}

fn is_arc_registry(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    if path.qself.is_some()
        || path.path.leading_colon.is_some()
        || path.path.segments.len() != 1
        || path.path.segments[0].ident != "Arc"
    {
        return false;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &path.path.segments[0].arguments else {
        return false;
    };
    arguments.args.len() == 1
        && matches!(arguments.args.first(), Some(syn::GenericArgument::Type(registry))
            if is_plain_path(registry, &["WorldPacketHandlerRegistry"]))
}

fn is_composer_output(output: &syn::ReturnType, fixture: bool) -> bool {
    let syn::ReturnType::Type(_, ty) = output else {
        return false;
    };
    if fixture {
        return is_arc_registry(ty);
    }
    let Type::Path(result) = &**ty else {
        return false;
    };
    if result.qself.is_some()
        || result.path.leading_colon.is_some()
        || result.path.segments.len() != 1
        || result.path.segments[0].ident != "Result"
    {
        return false;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &result.path.segments[0].arguments else {
        return false;
    };
    arguments.args.len() == 2
        && matches!(arguments.args.first(), Some(syn::GenericArgument::Type(registry))
            if is_arc_registry(registry))
        && matches!(arguments.args.iter().nth(1), Some(syn::GenericArgument::Type(error))
            if is_plain_path(error, &["DuplicateHandlerRegistrationLikeCpp"]))
}

fn exact_cfg_fixture(attribute: &syn::Attribute) -> bool {
    let Meta::List(cfg) = &attribute.meta else {
        return false;
    };
    if !cfg.path.is_ident("cfg") {
        return false;
    }
    let Ok(outer) = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(cfg.tokens.clone()) else {
        return false;
    };
    if outer.len() != 1 {
        return false;
    }
    let Some(Meta::List(any)) = outer.first() else {
        return false;
    };
    if !any.path.is_ident("any") {
        return false;
    }
    let Ok(inner) = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(any.tokens.clone()) else {
        return false;
    };
    inner.len() == 2
        && matches!(inner.first(), Some(Meta::Path(path)) if path.is_ident("test"))
        && matches!(inner.iter().nth(1), Some(Meta::NameValue(value)
            if value.path.is_ident("feature")
                && matches!(&value.value, Expr::Lit(literal)
                    if matches!(&literal.lit, syn::Lit::Str(feature) if feature.value() == "test-fixtures"))))
}

fn builder_mut_ref(expression: &Expr) -> bool {
    matches!(expression, Expr::Reference(reference)
        if reference.mutability.is_some()
            && reference.attrs.is_empty()
            && matches!(&*reference.expr, Expr::Path(path)
                if path.qself.is_none()
                    && path.attrs.is_empty()
                    && path.path.is_ident("builder")))
}

fn is_builder_initializer(statement: &Stmt) -> bool {
    let Stmt::Local(local) = statement else {
        return false;
    };
    if !local.attrs.is_empty()
        || !matches!(&local.pat, syn::Pat::Ident(pattern)
            if pattern.attrs.is_empty() && pattern.ident == "builder" && pattern.mutability.is_some() && pattern.subpat.is_none())
    {
        return false;
    }
    let Some(initializer) = &local.init else {
        return false;
    };
    if initializer.diverge.is_some() {
        return false;
    }
    matches!(&*initializer.expr, Expr::Call(call)
        if call.attrs.is_empty()
            && call.args.is_empty()
            && matches!(&*call.func, Expr::Path(path)
                if path.qself.is_none()
                    && path.attrs.is_empty()
                    && path.path.segments.len() == 2
                    && path.path.segments[0].ident == "WorldPacketHandlerRegistryBuilder"
                    && matches!(&path.path.segments[0].arguments, syn::PathArguments::None)
                    && path.path.segments[1].ident == "new"
                    && matches!(&path.path.segments[1].arguments, syn::PathArguments::None)))
}

fn try_call(statement: &Stmt) -> Option<&syn::ExprCall> {
    let Stmt::Expr(expression, Some(_)) = statement else {
        return None;
    };
    let Expr::Try(try_expression) = expression else {
        return None;
    };
    if !try_expression.attrs.is_empty() {
        return None;
    }
    let Expr::Call(call) = &*try_expression.expr else {
        return None;
    };
    Some(call)
}

fn is_domain_registration(statement: &Stmt, allow_inferred_types: bool) -> bool {
    let Some(call) = try_call(statement) else {
        return false;
    };
    let Expr::Path(function) = &*call.func else {
        return false;
    };
    let Some(registrar) = function.path.segments.last() else {
        return false;
    };
    if !call.attrs.is_empty()
        || !function.attrs.is_empty()
        || registrar.ident != REGISTRAR
        || !path_is(&function.path, &["wow_world_inventory", REGISTRAR])
        || call.args.len() != 1
        || !builder_mut_ref(&call.args[0])
    {
        return false;
    }
    if allow_inferred_types {
        return matches!(&registrar.arguments, syn::PathArguments::None)
            && function
                .path
                .segments
                .iter()
                .take(function.path.segments.len() - 1)
                .all(|segment| matches!(&segment.arguments, syn::PathArguments::None));
    }
    let syn::PathArguments::AngleBracketed(arguments) = &registrar.arguments else {
        return false;
    };
    arguments.args.len() == 2
        && matches!(arguments.args.first(), Some(syn::GenericArgument::Type(Type::Path(path)))
            if path.qself.is_none() && path.path.is_ident("WorldSession"))
        && matches!(arguments.args.iter().nth(1), Some(syn::GenericArgument::Type(Type::Path(path)))
            if path.qself.is_none() && path.path.is_ident("SessionHandlerCatalogsLikeCpp"))
}

fn is_legacy_registration(statement: &Stmt) -> bool {
    let Some(call) = try_call(statement) else {
        return false;
    };
    matches!(&*call.func, Expr::Path(function)
        if function.attrs.is_empty()
            && path_is(&function.path, &["register_remaining_handlers_like_cpp"])
            && matches!(function.path.segments.last().map(|segment| &segment.arguments), Some(syn::PathArguments::None)))
        && call.args.len() == 1
        && builder_mut_ref(&call.args[0])
}

fn fixture_expect_call(statement: &Stmt, expected_path: &[&str], domain: bool) -> bool {
    let Stmt::Expr(expression, Some(_)) = statement else {
        return false;
    };
    let Expr::MethodCall(expect) = expression else {
        return false;
    };
        if expect.method != "expect"
        || expect.turbofish.is_some()
        || !expect.attrs.is_empty()
        || expect.args.len() != 1
        || !matches!(expect.args.first(), Some(Expr::Lit(lit))
            if matches!(&lit.lit, syn::Lit::Str(value)
                if value.value() == "invalid duplicate packet handler composition"))
    {
        return false;
    }
    let Expr::Call(call) = &*expect.receiver else {
        return false;
    };
    if !call.attrs.is_empty() {
        return false;
    }
    let Expr::Path(path) = &*call.func else {
        return false;
    };
    if !path.attrs.is_empty() {
        return false;
    }
    if domain {
        let Some(registrar) = path.path.segments.last() else {
            return false;
        };
        if registrar.ident != REGISTRAR
            || !path_is(&path.path, expected_path)
            || !matches!(&registrar.arguments, syn::PathArguments::None)
        {
            return false;
        }
    } else if !path_is(&path.path, expected_path) {
        return false;
    }
    call.args.len() == 1 && builder_mut_ref(&call.args[0])
}

fn is_builder_result(statement: &Stmt) -> bool {
    let Stmt::Expr(expression, None) = statement else {
        return false;
    };
    let Expr::Call(ok_call) = expression else {
        return false;
    };
    if !ok_call.attrs.is_empty()
        || !path_is(match &*ok_call.func {
        Expr::Path(path) if path.attrs.is_empty() => &path.path,
        _ => return false,
    }, &["Ok"])
        || ok_call.args.len() != 1
    {
        return false;
    }
    let Expr::Call(arc_call) = &ok_call.args[0] else {
        return false;
    };
    if !arc_call.attrs.is_empty()
        || !path_is(match &*arc_call.func {
        Expr::Path(path) if path.attrs.is_empty() => &path.path,
        _ => return false,
    }, &["Arc", "new"])
        || arc_call.args.len() != 1
    {
        return false;
    }
    let Expr::MethodCall(build_call) = &arc_call.args[0] else {
        return false;
    };
    build_call.method == "build"
        && build_call.turbofish.is_none()
        && build_call.args.is_empty()
        && build_call.attrs.is_empty()
        && matches!(&*build_call.receiver, Expr::Path(path)
            if path.attrs.is_empty() && path.path.is_ident("builder"))
}

fn is_fixture_builder_result(statement: &Stmt) -> bool {
    let Stmt::Expr(expression, None) = statement else {
        return false;
    };
    let Expr::Call(arc_call) = expression else {
        return false;
    };
    if !arc_call.attrs.is_empty()
        || !path_is(match &*arc_call.func {
        Expr::Path(path) if path.attrs.is_empty() => &path.path,
        _ => return false,
    }, &["Arc", "new"])
        || arc_call.args.len() != 1
    {
        return false;
    }
    let Expr::MethodCall(build_call) = &arc_call.args[0] else {
        return false;
    };
    build_call.method == "build"
        && build_call.turbofish.is_none()
        && build_call.args.is_empty()
        && build_call.attrs.is_empty()
        && matches!(&*build_call.receiver, Expr::Path(path)
            if path.attrs.is_empty() && path.path.is_ident("builder"))
}

fn exact_body(function: &ItemFn, fixture: bool) -> bool {
    let statements = &function.block.stmts;
    statements.len() == 4
        && is_builder_initializer(&statements[0])
        && if fixture {
            fixture_expect_call(
                &statements[1],
                &["wow_world_inventory", REGISTRAR],
                true,
            ) && fixture_expect_call(
                &statements[2],
                &["register_remaining_handlers_like_cpp"],
                false,
            ) && is_fixture_builder_result(&statements[3])
        } else {
            is_domain_registration(&statements[1], false)
                && is_legacy_registration(&statements[2])
                && is_builder_result(&statements[3])
        }
}

fn exact_function(function: &ItemFn, fixture: bool) -> bool {
    let name = if fixture {
        "build_dispatch_table"
    } else {
        "compose_packet_handlers_like_cpp"
    };
    let mut doc_count = 0usize;
    let mut must_use_count = 0usize;
    let mut cfg_count = 0usize;
    let attributes_valid = function.attrs.iter().all(|attribute| {
        if attribute.path().is_ident("doc") {
            doc_count += 1;
            true
        } else if fixture && attribute.path().is_ident("must_use") {
            must_use_count += 1;
            matches!(&attribute.meta, Meta::Path(_))
        } else if fixture && attribute.path().is_ident("cfg") {
            cfg_count += 1;
            exact_cfg_fixture(attribute)
        } else {
            false
        }
    });
    function.sig.ident == name
        && matches!(&function.vis, Visibility::Public(_))
        && attributes_valid
        && doc_count > 0
        && must_use_count == usize::from(fixture)
        && function.sig.generics.params.is_empty()
        && function.sig.inputs.is_empty()
        && function.sig.constness.is_none()
        && function.sig.asyncness.is_none()
        && function.sig.unsafety.is_none()
        && function.sig.abi.is_none()
        && function.sig.variadic.is_none()
        && cfg_count == usize::from(fixture)
        && is_composer_output(&function.sig.output, fixture)
        && exact_body(function, fixture)
}

#[derive(Default)]
struct RegistrarCalls {
    calls: usize,
    references: usize,
    wrong_path: bool,
}

impl<'ast> Visit<'ast> for RegistrarCalls {
    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        if path.path.segments.last().is_some_and(|segment| segment.ident == REGISTRAR) {
            self.references += 1;
            if !path_is(&path.path, &["wow_world_inventory", REGISTRAR]) {
                self.wrong_path = true;
            }
        }
        syn::visit::visit_expr_path(self, path);
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let Expr::Path(path) = &*call.func {
            if path.path.segments.last().is_some_and(|segment| segment.ident == REGISTRAR) {
                self.calls += 1;
            }
        }
        syn::visit::visit_expr_call(self, call);
    }

}

fn tree_mentions(tree: &UseTree, expected: &str) -> bool {
    match tree {
        UseTree::Path(path) => path.ident == expected || tree_mentions(&path.tree, expected),
        UseTree::Name(name) => name.ident == expected,
        UseTree::Rename(rename) => rename.ident == expected || rename.rename == expected,
        UseTree::Group(group) => group.items.iter().any(|item| tree_mentions(item, expected)),
        UseTree::Glob(_) => false,
    }
}

fn exact_facade_tree(tree: &UseTree, child: &str, expected_exports: &[&str]) -> bool {
    let UseTree::Path(path) = tree else {
        return false;
    };
    if path.ident != child || !matches!(&path.tree, UseTree::Group(_)) {
        return false;
    }
    let UseTree::Group(group) = &path.tree else {
        return false;
    };
    if group.items.len() != expected_exports.len() {
        return false;
    }
    let Some(names): Option<Vec<_>> = group
        .items
        .iter()
        .map(|item| match item {
            UseTree::Name(name) => Some(name.ident.to_string()),
            _ => None,
        })
        .collect()
    else {
        return false;
    };
    let actual: std::collections::BTreeSet<_> = names.iter().cloned().collect();
    names.len() == expected_exports.len()
        && actual.len() == expected_exports.len()
        && expected_exports.iter().all(|expected| actual.contains(*expected))
}

fn exact_facade(item: &ItemUse, mount: &WorkspaceSourceMount) -> Option<&'static str> {
    if !item.attrs.is_empty()
        || item.leading_colon.is_some()
        || !matches!(&item.vis, Visibility::Public(_))
    {
        return None;
    }
    let context = (mount.contexts.len() == 1).then(|| mount.contexts.iter().next()).flatten()?;
    if mount.package != INVENTORY_PACKAGE
        || !context.production_possible
        || !context.cfg.is_empty()
        || !context.test_possible
    {
        return None;
    }
    let (child, expected_exports) = match context.logical_module_path.as_str() {
        "crate" => ("handlers", ROOT_FACADE_EXPORTS),
        "crate::handlers" => ("equipment_sets", HANDLER_FACADE_EXPORTS),
        _ => return None,
    };
    if !exact_facade_tree(&item.tree, child, expected_exports) {
        return None;
    }
    Some(match context.logical_module_path.as_str() {
        "crate" => "crate",
        "crate::handlers" => "crate::handlers",
        _ => return None,
    })
}

fn context_is(context: &SourceMountContext, module: &str, fixture: bool) -> bool {
    context.logical_module_path == module
        && context.production_possible
        && context.cfg.is_empty()
        && (!fixture || context.test_possible)
}

/// Verify both real composition sites and reject alternate registrar call paths.
pub(crate) fn validate_composition_mounts(mounts: &[WorkspaceSourceMount]) -> Result<(), String> {
    let mut production_composers = 0usize;
    let mut fixture_composers = 0usize;
    let mut root_facades = 0usize;
    let mut handler_facades = 0usize;
    for mount in mounts {
        if !mount.source.contains(REGISTRAR) {
            continue;
        }
        let syntax = syn::parse_file(&mount.source)
            .map_err(|error| format!("cannot parse {}: {error}", mount.source_path.display()))?;
        let mut invalid_use = false;
        for item in &syntax.items {
            let Item::Use(item_use) = item else {
                continue;
            };
            if !tree_mentions(&item_use.tree, REGISTRAR)
                && !tree_mentions(&item_use.tree, "wow_world_inventory")
            {
                continue;
            }
            match exact_facade(item_use, mount) {
                Some("crate") => root_facades += 1,
                Some("crate::handlers") => handler_facades += 1,
                _ => invalid_use = true,
            }
        }
        let mut calls = RegistrarCalls::default();
        calls.visit_file(&syntax);
        if invalid_use || calls.wrong_path || calls.references != calls.calls {
            return Err(format!(
                "Inventory registrar reference in {} must use its exact qualified provider and no import alias",
                mount.source_path.display()
            ));
        }
        if calls.calls == 0 {
            continue;
        }
        let server_context = mount.package == "world-server"
            && mount.contexts.len() == 1
            && mount.contexts.iter().all(|context| {
                context_is(context, "crate::handler_registry", false)
            });
        let world_fixture_context = mount.package == "wow-world"
            && mount.contexts.len() == 1
            && mount.contexts.iter().all(|context| {
                context_is(context, "crate::session::registry", true)
            });
        let functions: Vec<_> = syntax
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Fn(function) => Some(function),
                _ => None,
            })
            .collect();
        if server_context {
            let composers: Vec<_> = functions
                .iter()
                .filter(|function| function.sig.ident == "compose_packet_handlers_like_cpp")
                .collect();
            if composers.len() != 1
                || calls.calls != 1
                || !has_exact_legacy_import(&syntax.items)
                || !exact_function(composers[0], false)
            {
                return Err(format!(
                    "{} must contain exactly one unconditional production composer with Domain then legacy registration",
                    mount.source_path.display()
                ));
            }
            production_composers += 1;
        } else if world_fixture_context {
            let composers: Vec<_> = functions
                .iter()
                .filter(|function| function.sig.ident == "build_dispatch_table")
                .collect();
            if composers.len() != 1 || calls.calls != 1 || !exact_function(composers[0], true) {
                return Err(format!(
                    "{} fixture dispatch builder must preserve its exact paired Domain/legacy registration and gate",
                    mount.source_path.display()
                ));
            }
            fixture_composers += 1;
        } else {
            return Err(format!(
                "Inventory registrar is called outside the production composer or fixture dispatch builder: {} ({:?})",
                mount.source_path.display(),
                mount.contexts.iter().map(|context| &context.logical_module_path).collect::<Vec<_>>()
            ));
        }
    }
    if production_composers != 1
        || fixture_composers != 1
        || root_facades != 1
        || handler_facades != 1
    {
        return Err(format!(
            "expected one production composer, one fixture dispatch builder, and two exact Inventory facade re-exports; found {production_composers}, {fixture_composers}, {root_facades}, and {handler_facades}"
        ));
    }
    Ok(())
}
