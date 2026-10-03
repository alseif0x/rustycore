// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Exact production and fixture composition sites for World packet handlers.

use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::{Expr, Item, ItemFn, ItemUse, Meta, Stmt, Token, Type, UseTree, Visibility};

use crate::ownership::{SourceMountContext, WorkspaceSourceMount};
use super::direct_builder::{
    DIRECT_REGISTRAR_CONTRACTS, DirectRegistrarContract, analyze_contract_source,
};

fn crate_ident(package: &str) -> String {
    package.replace('-', "_")
}

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

fn is_domain_registration(
    statement: &Stmt,
    allow_inferred_types: bool,
    contract: DirectRegistrarContract,
) -> bool {
    let Some(call) = try_call(statement) else {
        return false;
    };
    let Expr::Path(function) = &*call.func else {
        return false;
    };
    let Some(registrar) = function.path.segments.last() else {
        return false;
    };
    let crate_name = crate_ident(contract.package);
    let expected_path = [crate_name.as_str(), contract.registrar];
    if !call.attrs.is_empty()
        || !function.attrs.is_empty()
        || registrar.ident != contract.registrar
        || !path_is(&function.path, &expected_path)
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
    arguments.args.len() == contract.production_type_args.len()
        && arguments
            .args
            .iter()
            .zip(contract.production_type_args)
            .all(|(argument, expected)| {
                matches!(argument, syn::GenericArgument::Type(Type::Path(path))
                    if path.qself.is_none()
                        && path.path.leading_colon.is_none()
                        && path.path.is_ident(expected))
            })
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
        if registrar.ident.to_string() != expected_path.last().copied().unwrap_or_default()
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

fn exact_body(
    function: &ItemFn,
    fixture: bool,
    contracts: &[DirectRegistrarContract],
) -> bool {
    let statements = &function.block.stmts;
    if statements.len() != contracts.len() + 3
        || !is_builder_initializer(&statements[0])
    {
        return false;
    }
    let legacy_registration = &statements[contracts.len() + 1];
    let legacy_matches = if fixture {
        fixture_expect_call(
            legacy_registration,
            &["register_remaining_handlers_like_cpp"],
            false,
        )
    } else {
        is_legacy_registration(legacy_registration)
    };
    if !legacy_matches {
        return false;
    }
    let registrations_match = contracts.iter().enumerate().all(|(index, contract)| {
        if fixture {
            let crate_name = crate_ident(contract.package);
            fixture_expect_call(
                &statements[index + 1],
                &[crate_name.as_str(), contract.registrar],
                true,
            )
        } else {
            is_domain_registration(&statements[index + 1], false, *contract)
        }
    });
    registrations_match
        && if fixture {
            is_fixture_builder_result(&statements[contracts.len() + 2])
        } else {
            is_builder_result(&statements[contracts.len() + 2])
        }
}

fn exact_function(
    function: &ItemFn,
    fixture: bool,
    contracts: &[DirectRegistrarContract],
) -> bool {
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
        && exact_body(function, fixture, contracts)
}

#[derive(Default)]
struct RegistrarCallCount {
    calls: usize,
    references: usize,
    wrong_path: bool,
}

struct RegistrarCalls<'a> {
    contracts: &'a [DirectRegistrarContract],
    counts: std::collections::BTreeMap<&'static str, RegistrarCallCount>,
    ambiguous_name: bool,
}

impl<'a> RegistrarCalls<'a> {
    fn new(contracts: &'a [DirectRegistrarContract]) -> Self {
        Self {
            contracts,
            counts: std::collections::BTreeMap::new(),
            ambiguous_name: false,
        }
    }

    fn contract_for_path(&mut self, path: &syn::Path) -> Option<DirectRegistrarContract> {
        let name = path.segments.last()?.ident.to_string();
        let mut matches = self
            .contracts
            .iter()
            .filter(|contract| contract.registrar == name.as_str());
        let contract = *matches.next()?;
        if matches.next().is_some() {
            self.ambiguous_name = true;
        }
        Some(contract)
    }

    fn saw_call(&self) -> bool {
        self.counts.values().any(|count| count.calls != 0 || count.references != 0)
    }
}

impl<'ast> Visit<'ast> for RegistrarCalls<'_> {
    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        if let Some(contract) = self.contract_for_path(&path.path) {
            let count = self.counts.entry(contract.registrar).or_default();
            count.references += 1;
            let crate_name = crate_ident(contract.package);
            if !path_is(&path.path, &[crate_name.as_str(), contract.registrar]) {
                count.wrong_path = true;
            }
        }
        syn::visit::visit_expr_path(self, path);
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let Expr::Path(path) = &*call.func {
            if let Some(contract) = self.contract_for_path(&path.path) {
                self.counts.entry(contract.registrar).or_default().calls += 1;
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
    if path.ident != child {
        return false;
    }
    let actual_names: Vec<String> = match &path.tree {
        UseTree::Name(name) if expected_exports.len() == 1 => vec![name.ident.to_string()],
        UseTree::Group(group) => {
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
            names
        }
        _ => return false,
    };
    if actual_names.len() != expected_exports.len() {
        return false;
    }
    let actual: std::collections::BTreeSet<_> = actual_names.iter().cloned().collect();
    actual_names.len() == expected_exports.len()
        && actual.len() == expected_exports.len()
        && expected_exports.iter().all(|expected| actual.contains(*expected))
}

fn exact_facade(
    item: &ItemUse,
    mount: &WorkspaceSourceMount,
    contract: DirectRegistrarContract,
) -> Option<&'static str> {
    if !item.attrs.is_empty()
        || item.leading_colon.is_some()
        || !matches!(&item.vis, Visibility::Public(_))
    {
        return None;
    }
    let context = (mount.contexts.len() == 1).then(|| mount.contexts.iter().next()).flatten()?;
    if mount.package != contract.package
        || !context.production_possible
        || !context.cfg.is_empty()
        || !context.test_possible
    {
        return None;
    }
    let facade = contract
        .facades
        .iter()
        .find(|facade| facade.module == context.logical_module_path)?;
    if !exact_facade_tree(&item.tree, facade.child, facade.exports) {
        return None;
    }
    Some(facade.module)
}

fn context_is(context: &SourceMountContext, module: &str, fixture: bool) -> bool {
    context.logical_module_path == module
        && context.production_possible
        && context.cfg.is_empty()
        && (!fixture || context.test_possible)
}

fn analyzed_direct_registrars(
    mounts: &[WorkspaceSourceMount],
    configured: &[DirectRegistrarContract],
) -> Result<Vec<DirectRegistrarContract>, String> {
    let mut result = Vec::new();
    let mut names = std::collections::BTreeSet::new();
    for contract in configured {
        if !names.insert(contract.registrar) {
            return Err(format!(
                "direct registrar function name {} is ambiguous across owner contracts",
                contract.registrar
            ));
        }
        if contract.facades.is_empty() {
            return Err(format!(
                "direct registrar {} must declare its exact public facade mounts",
                contract.registrar
            ));
        }
        let mut facade_modules = std::collections::BTreeSet::new();
        if contract
            .facades
            .iter()
            .any(|facade| !facade_modules.insert(facade.module))
        {
            return Err(format!(
                "direct registrar {} repeats a public facade module",
                contract.registrar
            ));
        }
        let mut declarations = 0usize;
        for mount in mounts.iter().filter(|mount| mount.package == contract.package) {
            let syntax = syn::parse_file(&mount.source)
                .map_err(|error| format!("cannot parse {}: {error}", mount.source_path.display()))?;
            let count = syntax
                .items
                .iter()
                .filter(|item| declared_registrar(item, *contract))
                .count();
            if count == 0 {
                continue;
            }
            if mount.contexts.len() != 1 {
                return Err(format!(
                    "direct registrar {} has an ambiguous source mount at {}",
                    contract.registrar,
                    mount.source_path.display()
                ));
            }
            let context = mount.contexts.iter().next().expect("one source context");
            if !context.production_possible || !context.test_possible || !context.cfg.is_empty() {
                return Err(format!(
                    "direct registrar {} must be mounted unconditionally for production and tests at {}",
                    contract.registrar,
                    mount.source_path.display()
                ));
            }
            let report = analyze_contract_source(
                *contract,
                &mount.package,
                &context.logical_module_path,
                &mount.source_path,
                &mount.source,
            )?;
            declarations += report.registrar_count;
            if report.registrar_count == 1 {
                result.push(report.contract.expect("analyzed registrar carries its contract"));
            }
        }
        if declarations != 1 {
            return Err(format!(
                "expected exactly one analyzed {} registrar, found {declarations}",
                contract.owner
            ));
        }
    }
    if result.is_empty() {
        return Err("no finite direct registrar contracts were analyzed".to_owned());
    }
    Ok(result)
}

fn declared_registrar(item: &Item, contract: DirectRegistrarContract) -> bool {
    matches!(item, Item::Fn(function) if function.sig.ident == contract.registrar)
}

#[derive(Default)]
struct OwnerModuleShadows {
    package_names: std::collections::BTreeSet<String>,
    shadowed: std::collections::BTreeSet<String>,
}

impl<'ast> Visit<'ast> for OwnerModuleShadows {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let name = item.ident.to_string();
        if self.package_names.contains(&name) {
            self.shadowed.insert(name);
        }
        syn::visit::visit_item_mod(self, item);
    }
}

/// Verify both composition sites against the finite direct registrars that were analyzed.
pub(crate) fn validate_composition_mounts(mounts: &[WorkspaceSourceMount]) -> Result<(), String> {
    validate_composition_mounts_with_contracts(mounts, DIRECT_REGISTRAR_CONTRACTS)
}

pub(crate) fn validate_composition_mounts_with_contracts(
    mounts: &[WorkspaceSourceMount],
    configured_contracts: &[DirectRegistrarContract],
) -> Result<(), String> {
    let contracts = analyzed_direct_registrars(mounts, configured_contracts)?;
    let mut production_composers = 0usize;
    let mut fixture_composers = 0usize;
    let mut facade_counts = std::collections::BTreeMap::<(String, String), usize>::new();

    for mount in mounts {
        let relevant = contracts
            .iter()
            .any(|contract| mount.source.contains(contract.registrar));
        if !relevant {
            continue;
        }
        let syntax = syn::parse_file(&mount.source)
            .map_err(|error| format!("cannot parse {}: {error}", mount.source_path.display()))?;
        let mut owner_modules = OwnerModuleShadows {
            package_names: contracts
                .iter()
                .map(|contract| crate_ident(contract.package))
                .collect(),
            ..OwnerModuleShadows::default()
        };
        owner_modules.visit_file(&syntax);
        if !owner_modules.shadowed.is_empty() {
            return Err(format!(
                "direct registrar package path is shadowed by a local module in {}: {:?}",
                mount.source_path.display(),
                owner_modules.shadowed
            ));
        }
        let mut invalid_use = false;
        for item in &syntax.items {
            let Item::Use(item_use) = item else {
                continue;
            };
            for contract in &contracts {
                if !tree_mentions(&item_use.tree, contract.registrar)
                    && !tree_mentions(&item_use.tree, &crate_ident(contract.package))
                {
                    continue;
                }
                if let Some(module) = exact_facade(item_use, mount, *contract) {
                    *facade_counts
                        .entry((contract.package.to_owned(), module.to_owned()))
                        .or_default() += 1;
                } else {
                    invalid_use = true;
                }
            }
        }
        let mut calls = RegistrarCalls::new(&contracts);
        calls.visit_file(&syntax);
        let bad_calls = calls.ambiguous_name
            || calls.counts.values().any(|count| {
                count.wrong_path || count.references != count.calls
            });
        if invalid_use || bad_calls {
            return Err(format!(
                "direct registrar references in {} must use their exact qualified providers and no aliases",
                mount.source_path.display()
            ));
        }
        if !calls.saw_call() {
            continue;
        }

        let expected_call_counts = contracts.len();
        let counts_are_exact = calls.counts.len() == expected_call_counts
            && contracts.iter().all(|contract| {
                calls.counts.get(contract.registrar).is_some_and(|count| {
                    count.calls == 1 && count.references == 1 && !count.wrong_path
                })
            });
        if !counts_are_exact {
            return Err(format!(
                "{} must call each analyzed direct registrar exactly once",
                mount.source_path.display()
            ));
        }

        let server_context = mount.package == "world-server"
            && mount.contexts.len() == 1
            && mount
                .contexts
                .iter()
                .all(|context| context_is(context, "crate::handler_registry", false));
        let world_fixture_context = mount.package == "wow-world"
            && mount.contexts.len() == 1
            && mount
                .contexts
                .iter()
                .all(|context| context_is(context, "crate::session::registry", true));
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
                || !has_exact_legacy_import(&syntax.items)
                || !exact_function(composers[0], false, &contracts)
            {
                return Err(format!(
                    "{} must contain one unconditional composer with each direct registrar then legacy registration",
                    mount.source_path.display()
                ));
            }
            production_composers += 1;
        } else if world_fixture_context {
            let composers: Vec<_> = functions
                .iter()
                .filter(|function| function.sig.ident == "build_dispatch_table")
                .collect();
            if composers.len() != 1 || !exact_function(composers[0], true, &contracts) {
                return Err(format!(
                    "{} fixture dispatch builder must preserve its exact registrar set, legacy call, and gate",
                    mount.source_path.display()
                ));
            }
            fixture_composers += 1;
        } else {
            return Err(format!(
                "a direct registrar is called outside the production composer or fixture dispatch builder: {} ({:?})",
                mount.source_path.display(),
                mount
                    .contexts
                    .iter()
                    .map(|context| &context.logical_module_path)
                    .collect::<Vec<_>>()
            ));
        }
    }

    let facade_mismatch = contracts.iter().any(|contract| {
        contract.facades.iter().any(|facade| {
            facade_counts
                .get(&(contract.package.to_owned(), facade.module.to_owned()))
                .copied()
                != Some(1)
        })
    });
    if production_composers != 1 || fixture_composers != 1 || facade_mismatch {
        return Err(format!(
            "expected one production and one fixture composer plus every exact registrar facade; found {production_composers} and {fixture_composers} composers"
        ));
    }
    Ok(())
}
