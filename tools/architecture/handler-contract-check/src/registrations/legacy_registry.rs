// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The retired World legacy inventory registration bridge (#1360).
//!
//! Every world packet handler registers through an explicit area registrar
//! composed by `wow_world::handler_composition`. The `inventory`-backed
//! bridge (`LegacyPacketHandlerRegistrationLikeCpp`, its collector, the
//! `register_packet_handler_like_cpp!` wrapper and the
//! `register_remaining_handlers_like_cpp` drain) is gone, and nothing drains
//! an `inventory` collection into the registry any more. This module keeps
//! the wrapper's names and entry grammar only so that a reintroduction is
//! recognized and rejected: the wrapper type and macro are rejected by name
//! anywhere, an `inventory::collect!`/`submit!` outside a handler owner fails
//! the shared registration-syntax audit, and a direct submission inside one
//! fails the repository check because no drain would ever register it.

use proc_macro2::{Group, TokenStream, TokenTree};
use std::path::Path;

use syn::Item;

use super::{macro_definition_may_generate_handler, token_macro_definitions};

const WRAPPER_TYPE: &str = "LegacyPacketHandlerRegistrationLikeCpp";
const WRAPPER_MACRO: &str = "register_packet_handler_like_cpp";
const WRAPPER_MACRO_PATH: &[&str] = &[
    "crate",
    "session",
    "registry",
    "register_packet_handler_like_cpp",
];

pub(super) fn is_wrapper_macro_name(name: &proc_macro2::Ident) -> bool {
    super::ident_is(name, WRAPPER_MACRO)
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

/// Violations for a source outside every declared handler owner: the retired
/// wrapper type or macro by name, and any handler-capable `macro_rules!`.
pub(super) fn retired_bridge_violations(
    source_path: &Path,
    tokens: &TokenStream,
    syntax: &syn::File,
) -> Vec<String> {
    let mut violations = Vec::new();
    for item in &syntax.items {
        let reintroduced = match item {
            Item::Struct(item) => super::ident_is(&item.ident, WRAPPER_TYPE),
            Item::Macro(item) => item
                .ident
                .as_ref()
                .is_some_and(|ident| super::ident_is(ident, WRAPPER_MACRO)),
            _ => false,
        };
        if reintroduced {
            violations.push(format!(
                "{} reintroduces the retired legacy inventory registration bridge \
                 ({WRAPPER_TYPE} / {WRAPPER_MACRO}!, retired by #1360); register handlers through \
                 an explicit area registrar",
                source_path.display()
            ));
        }
    }
    for definition in token_macro_definitions(tokens)
        .into_iter()
        .filter(macro_definition_may_generate_handler)
    {
        violations.push(format!(
            "{} defines handler-capable macro_rules! {} outside the declared handler-registration owner",
            source_path.display(),
            definition.name
        ));
    }
    violations
}
