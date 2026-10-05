// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Exact grammar for the World registry's legacy inventory adapter.

use proc_macro2::{Group, TokenStream, TokenTree};
use std::path::Path;

use syn::visit::Visit;
use syn::{Attribute, Fields, Item, ItemMacro, ItemStruct, Type, UseTree, Visibility};

use super::{
    conditional_attribute_name, macro_definition_may_generate_handler, path_segments,
    token_macro_calls, token_macro_definitions,
};

const WRAPPER_TYPE: &str = "LegacyPacketHandlerRegistrationLikeCpp";
const WRAPPER_MACRO: &str = "register_packet_handler_like_cpp";
const WRAPPER_MACRO_PATH: &[&str] = &[
    "crate",
    "session",
    "registry",
    "register_packet_handler_like_cpp",
];
const WRAPPER_MACRO_BODY: &str = r#"($entry:expr) => {
    const _: () = {
        static ENTRY: $crate::session::registry::PacketHandlerEntry = $entry;
        inventory::submit! {
            $crate::session::registry::LegacyPacketHandlerRegistrationLikeCpp {
                entry: &ENTRY,
            }
        }
    };
};"#;
const WRAPPER_SUBMIT_BODY: &str = r#"$crate::session::registry::LegacyPacketHandlerRegistrationLikeCpp {
    entry: &ENTRY,
}"#;

pub(super) struct BridgeAnalysis {
    pub(super) violations: Vec<String>,
    pub(super) exact_collectors: usize,
    pub(super) complete: bool,
}

fn unqualified_type_path(ty: &Type, expected: &str) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.qself.is_none()
        && path.path.leading_colon.is_none()
        && path.path.segments.len() == 1
        && path.path.segments.first().is_some_and(|segment| {
            super::ident_is(&segment.ident, expected)
                && matches!(segment.arguments, syn::PathArguments::None)
        })
}

fn only_documentation_attributes(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .all(|attribute| attribute.path().is_ident("doc"))
}

fn use_tree_contains_glob(tree: &UseTree) -> bool {
    match tree {
        UseTree::Path(path) => use_tree_contains_glob(&path.tree),
        UseTree::Group(group) => group.items.iter().any(use_tree_contains_glob),
        UseTree::Glob(_) => true,
        UseTree::Name(_) | UseTree::Rename(_) => false,
    }
}

#[derive(Default)]
struct WildcardUseCollector(usize);

impl<'ast> Visit<'ast> for WildcardUseCollector {
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if use_tree_contains_glob(&item.tree) {
            self.0 += 1;
        }
        syn::visit::visit_item_use(self, item);
    }
}

fn crate_visibility(visibility: &Visibility) -> bool {
    matches!(visibility, Visibility::Restricted(restricted)
        if restricted.in_token.is_none() && restricted.path.is_ident("crate"))
}

pub(super) fn is_wrapper_macro_name(name: &proc_macro2::Ident) -> bool {
    super::ident_is(name, WRAPPER_MACRO)
}

pub(super) fn is_exact_wrapper_reexport(item: &syn::ItemUse) -> bool {
    item.leading_colon.is_none()
        && item.attrs.is_empty()
        && crate_visibility(&item.vis)
        && matches!(&item.tree, syn::UseTree::Name(name) if super::ident_is(&name.ident, WRAPPER_MACRO))
}

fn is_exact_wrapper_type(item: &ItemStruct) -> bool {
    if !super::ident_is(&item.ident, WRAPPER_TYPE)
        || !crate_visibility(&item.vis)
        || !only_documentation_attributes(&item.attrs)
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
    {
        return false;
    }
    let Fields::Named(fields) = &item.fields else {
        return false;
    };
    if fields.named.len() != 1 {
        return false;
    }
    let field = &fields.named[0];
    let Type::Reference(reference) = &field.ty else {
        return false;
    };
    field
        .ident
        .as_ref()
        .is_some_and(|ident| super::ident_is(ident, "entry"))
        && crate_visibility(&field.vis)
        && only_documentation_attributes(&field.attrs)
        && reference.mutability.is_none()
        && reference
            .lifetime
            .as_ref()
            .is_some_and(|lifetime| lifetime.ident == "static")
        && unqualified_type_path(&reference.elem, "PacketHandlerEntry")
}

fn token_stream_matches(actual: &TokenStream, expected: &str) -> bool {
    expected
        .parse::<TokenStream>()
        .is_ok_and(|expected| actual.to_string() == expected.to_string())
}

fn packet_handler_entry_mentions(tokens: &TokenStream) -> usize {
    tokens
        .clone()
        .into_iter()
        .map(|token| match token {
            TokenTree::Ident(ident) => usize::from(super::ident_is(&ident, "PacketHandlerEntry")),
            TokenTree::Group(group) => packet_handler_entry_mentions(&group.stream()),
            TokenTree::Punct(_) | TokenTree::Literal(_) => 0,
        })
        .sum()
}

fn normalize_macro_parameters(tokens: TokenStream) -> Option<TokenStream> {
    let mut tokens = tokens.into_iter().peekable();
    let mut normalized = TokenStream::new();
    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Punct(punctuation) if punctuation.as_char() == '$' => {
                let Some(TokenTree::Ident(parameter)) = tokens.next() else {
                    // Macro repetitions and dollar-prefixed forms other than a
                    // single metavariable are outside the wrapper grammar.
                    return None;
                };
                if parameter.to_string() == "crate" {
                    return None;
                }
                normalized.extend([TokenTree::Ident(proc_macro2::Ident::new(
                    "__legacy_wrapper_parameter",
                    parameter.span(),
                ))]);
            }
            TokenTree::Group(group) => {
                let mut normalized_group = Group::new(
                    group.delimiter(),
                    normalize_macro_parameters(group.stream())?,
                );
                normalized_group.set_span(group.span());
                normalized.extend([TokenTree::Group(normalized_group)]);
            }
            token => normalized.extend([token]),
        }
    }
    Some(normalized)
}

fn is_entry_struct_expression(expression: syn::Expr, original: &TokenStream) -> bool {
    let syn::Expr::Struct(entry) = expression else {
        return false;
    };
    entry.path.leading_colon.is_none()
        && entry.path.segments.len() == 1
        && entry.path.segments.first().is_some_and(|segment| {
            super::ident_is(&segment.ident, "PacketHandlerEntry")
                && matches!(segment.arguments, syn::PathArguments::None)
        })
        && packet_handler_entry_mentions(original) == 1
}

fn is_exact_wrapper_macro(item: &ItemMacro) -> bool {
    item.mac.path.is_ident("macro_rules")
        && item
            .ident
            .as_ref()
            .is_some_and(|ident| super::ident_is(ident, WRAPPER_MACRO))
        && only_documentation_attributes(&item.attrs)
        && token_stream_matches(&item.mac.tokens, WRAPPER_MACRO_BODY)
}

pub(super) fn is_exact_wrapper_collector(path: &[String], body: &TokenStream) -> bool {
    if path != ["inventory", "collect"] {
        return false;
    }
    let Ok(handler_type) = syn::parse2::<syn::Path>(body.clone()) else {
        return false;
    };
    handler_type.leading_colon.is_none()
        && handler_type.segments.len() == 1
        && handler_type.segments.first().is_some_and(|segment| {
            super::ident_is(&segment.ident, WRAPPER_TYPE)
                && matches!(segment.arguments, syn::PathArguments::None)
        })
}

pub(super) fn is_wrapper_macro_path(path: &[String]) -> bool {
    path.iter()
        .map(String::as_str)
        .eq(WRAPPER_MACRO_PATH.iter().copied())
}

pub(super) fn is_wrapper_macro_name_path(path: &[String]) -> bool {
    path.last().is_some_and(|name| name == WRAPPER_MACRO)
}

pub(super) fn is_wrapper_entry_expression(body: &TokenStream) -> bool {
    let Ok(expression) = syn::parse2::<syn::Expr>(body.clone()) else {
        return false;
    };
    is_entry_struct_expression(expression, body)
}

pub(super) fn is_wrapper_template_entry_expression(body: &TokenStream) -> bool {
    let Some(normalized) = normalize_macro_parameters(body.clone()) else {
        return false;
    };
    let Ok(expression) = syn::parse2::<syn::Expr>(normalized) else {
        return false;
    };
    let syn::Expr::Struct(entry) = &expression else {
        return false;
    };
    let expected_fields = ["opcode", "status", "processing", "handler_name", "handler"];
    entry.rest.is_none()
        && entry.fields.len() == expected_fields.len()
        && expected_fields.iter().all(|expected| {
            entry.fields.iter().any(|field| {
                matches!(&field.member, syn::Member::Named(name) if super::ident_is(name, expected))
            })
        })
        && is_entry_struct_expression(expression, body)
}

fn is_wrapper_submit(path: &[String], body: &TokenStream) -> bool {
    path == ["inventory", "submit"] && token_stream_matches(body, WRAPPER_SUBMIT_BODY)
}

pub(super) fn analyze_bridge(
    source_path: &Path,
    tokens: &TokenStream,
    syntax: &syn::File,
    allow_bridge: bool,
) -> BridgeAnalysis {
    let mut violations = Vec::new();
    let calls = token_macro_calls(tokens);
    let top_level_exact_collectors: Vec<_> = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Macro(item)
                if is_exact_wrapper_collector(&path_segments(&item.mac.path), &item.mac.tokens) =>
            {
                Some(item)
            }
            _ => None,
        })
        .collect();
    let exact_collector_calls = calls
        .iter()
        .filter(|call| is_exact_wrapper_collector(&call.path, &call.body))
        .count();
    let wrapper_structs: Vec<_> = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(item) if super::ident_is(&item.ident, WRAPPER_TYPE) => Some(item),
            _ => None,
        })
        .collect();
    let exact_wrapper_structs = wrapper_structs
        .iter()
        .filter(|item| is_exact_wrapper_type(item))
        .count();
    let wrapper_macros: Vec<_> = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Macro(item)
                if item
                    .ident
                    .as_ref()
                    .is_some_and(|ident| super::ident_is(ident, WRAPPER_MACRO)) =>
            {
                Some(item)
            }
            _ => None,
        })
        .collect();
    let exact_wrapper_macros = wrapper_macros
        .iter()
        .filter(|item| is_exact_wrapper_macro(item))
        .count();
    let exact_wrapper_submit_calls = calls
        .iter()
        .filter(|call| is_wrapper_submit(&call.path, &call.body))
        .count();
    let exact_wrapper_reexports = syntax
        .items
        .iter()
        .filter(|item| matches!(item, Item::Use(item) if is_exact_wrapper_reexport(item)))
        .count();

    let source_is_unconditional = conditional_attribute_name(&syntax.attrs).is_none();
    let collectors_are_unconditional = top_level_exact_collectors
        .iter()
        .all(|collector| conditional_attribute_name(&collector.attrs).is_none());
    let wrapper_struct_is_exact = wrapper_structs.len() == 1 && exact_wrapper_structs == 1;
    let wrapper_macro_is_exact = wrapper_macros.len() == 1 && exact_wrapper_macros == 1;
    let complete = allow_bridge
        && exact_collector_calls == 1
        && top_level_exact_collectors.len() == 1
        && source_is_unconditional
        && collectors_are_unconditional
        && wrapper_struct_is_exact
        && wrapper_macro_is_exact
        && exact_wrapper_reexports == 1
        && exact_wrapper_submit_calls == 1;

    if allow_bridge {
        if exact_collector_calls != top_level_exact_collectors.len() {
            violations.push(format!(
                "{} contains the exact legacy-wrapper inventory collector outside module item \
                 level; the collector is allowed only as a top-level item",
                source_path.display()
            ));
        }
        if !source_is_unconditional {
            violations.push(format!(
                "{} guards the legacy registry bridge source with cfg/cfg_attr",
                source_path.display()
            ));
        }
        for collector in &top_level_exact_collectors {
            if !only_documentation_attributes(&collector.attrs) {
                violations.push(format!(
                    "{} adds non-documentation attributes to the legacy wrapper inventory collector",
                    source_path.display()
                ));
            }
            if conditional_attribute_name(&collector.attrs).is_some() {
                violations.push(format!(
                    "{} conditionally compiles inventory::collect!(LegacyPacketHandlerRegistrationLikeCpp)",
                    source_path.display()
                ));
            }
        }
        if !wrapper_struct_is_exact {
            violations.push(format!(
                "{} must define exactly one crate-private legacy wrapper with only entry: \
                 &'static PacketHandlerEntry",
                source_path.display()
            ));
        }
        if !wrapper_macro_is_exact {
            violations.push(format!(
                "{} must define exactly one exact register_packet_handler_like_cpp! bridge macro",
                source_path.display()
            ));
        }
        if exact_wrapper_reexports != 1 {
            violations.push(format!(
                "{} must contain exactly one top-level pub(crate) use register_packet_handler_like_cpp reexport",
                source_path.display()
            ));
        }
        if exact_wrapper_submit_calls != 1 {
            violations.push(format!(
                "{} exact legacy bridge must contain exactly one inventory::submit! of its wrapper entry; found {exact_wrapper_submit_calls}",
                source_path.display()
            ));
        }
        let mut wildcard_uses = WildcardUseCollector::default();
        wildcard_uses.visit_file(syntax);
        if wildcard_uses.0 != 0 {
            violations.push(format!(
                "{} uses a wildcard import in the legacy registry module; the wrapper bridge permits only its exact crate-private macro reexport",
                source_path.display()
            ));
        }
    }

    for definition in token_macro_definitions(tokens)
        .into_iter()
        .filter(macro_definition_may_generate_handler)
    {
        let exact_bridge_macro_is_unique = complete && definition.name == WRAPPER_MACRO;
        if !exact_bridge_macro_is_unique {
            violations.push(format!(
                "{} defines handler-capable macro_rules! {} outside the declared handler-registration owner",
                source_path.display(),
                definition.name
            ));
        }
    }

    BridgeAnalysis {
        violations,
        exact_collectors: top_level_exact_collectors.len(),
        complete,
    }
}

pub(super) fn is_exact_wrapper_submit(path: &[String], body: &TokenStream) -> bool {
    is_wrapper_submit(path, body)
}
