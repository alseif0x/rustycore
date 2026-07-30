// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Source and ownership audit for linked handler registrations.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::visit::Visit;
use syn::{Attribute, Expr, Item, ItemMacro, Lit, Meta, UseTree};

pub(crate) const EXPECTED_REGISTRATION_MACROS: &[&str] = &[
    "register_chat_channel_command_handler",
    "register_chat_channel_player_command_handler",
    "register_move",
    "register_movement_ack_message",
    "register_movement_speed_ack",
    "register_unhandled_threadsafe_null_handler",
];

#[derive(Clone, Debug)]
struct TokenMacroCall {
    path: Vec<String>,
    body: TokenStream,
}

#[derive(Clone, Debug)]
struct TokenMacroDefinition {
    name: String,
    body: TokenStream,
}

#[derive(Clone, Debug)]
struct MacroDefinitionSite {
    name: String,
    calls: Vec<TokenMacroCall>,
    handler_capable: bool,
    contains_conditional_tokens: bool,
    contains_repetition: bool,
    rule_arm_count: usize,
    conditional_context: Option<String>,
    location: String,
}

#[derive(Clone, Debug)]
struct MacroInvocationSite {
    path: Vec<String>,
    body: TokenStream,
    contains_conditional_tokens: bool,
    conditional_context: Option<String>,
    item_level: bool,
    location: String,
}

#[derive(Default)]
struct SourceCollection {
    definitions: Vec<MacroDefinitionSite>,
    invocations: Vec<MacroInvocationSite>,
    unsupported_registration_generators: Vec<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct RegistrationSourceReport {
    pub(crate) direct_submissions: usize,
    pub(crate) registration_macro_invocations: usize,
    pub(crate) registration_macro_names: BTreeSet<String>,
}

impl RegistrationSourceReport {
    pub(crate) fn represented_entries(&self) -> usize {
        self.direct_submissions + self.registration_macro_invocations
    }
}

fn conditional_attribute_name(attributes: &[Attribute]) -> Option<&'static str> {
    attributes.iter().find_map(|attribute| {
        if attribute.path().is_ident("cfg") {
            Some("cfg")
        } else if attribute.path().is_ident("cfg_attr") {
            Some("cfg_attr")
        } else {
            None
        }
    })
}

fn with_conditional_context(
    inherited: Option<String>,
    attributes: &[Attribute],
    owner: &str,
) -> Option<String> {
    inherited.or_else(|| {
        conditional_attribute_name(attributes)
            .map(|attribute| format!("{owner} is guarded by #[{attribute}(...)]"))
    })
}

fn normalized_ident(ident: &proc_macro2::Ident) -> String {
    let raw = ident.to_string();
    raw.strip_prefix("r#").unwrap_or(&raw).to_owned()
}

fn ident_is(ident: &proc_macro2::Ident, expected: &str) -> bool {
    normalized_ident(ident) == expected
}

fn token_stream_mentions_ident(tokens: &TokenStream, expected: &str) -> bool {
    tokens.clone().into_iter().any(|token| match token {
        TokenTree::Ident(ident) => ident_is(&ident, expected),
        TokenTree::Group(group) => token_stream_mentions_ident(&group.stream(), expected),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

fn token_stream_mentions_path(tokens: &TokenStream, expected: &[&str]) -> bool {
    let trees: Vec<_> = tokens.clone().into_iter().collect();
    for index in 0..trees.len() {
        let Some(TokenTree::Ident(first)) = trees.get(index) else {
            continue;
        };
        if expected
            .first()
            .is_none_or(|segment| !ident_is(first, segment))
        {
            continue;
        }
        let mut cursor = index + 1;
        let mut matched = true;
        for segment in &expected[1..] {
            if !is_punctuation(trees.get(cursor), ':')
                || !is_punctuation(trees.get(cursor + 1), ':')
            {
                matched = false;
                break;
            }
            let Some(TokenTree::Ident(actual)) = trees.get(cursor + 2) else {
                matched = false;
                break;
            };
            if !ident_is(actual, segment) {
                matched = false;
                break;
            }
            cursor += 3;
        }
        if matched && !expected.is_empty() {
            return true;
        }
    }
    trees.into_iter().any(|tree| match tree {
        TokenTree::Group(group) => token_stream_mentions_path(&group.stream(), expected),
        _ => false,
    })
}

fn token_stream_path_count(tokens: &TokenStream, expected: &[&str]) -> usize {
    let trees: Vec<_> = tokens.clone().into_iter().collect();
    let mut count = 0usize;
    for index in 0..trees.len() {
        let Some(TokenTree::Ident(first)) = trees.get(index) else {
            continue;
        };
        if expected
            .first()
            .is_none_or(|segment| !ident_is(first, segment))
        {
            continue;
        }
        let mut cursor = index + 1;
        let mut matched = !expected.is_empty();
        for segment in &expected[1..] {
            if !is_punctuation(trees.get(cursor), ':')
                || !is_punctuation(trees.get(cursor + 1), ':')
            {
                matched = false;
                break;
            }
            let Some(TokenTree::Ident(actual)) = trees.get(cursor + 2) else {
                matched = false;
                break;
            };
            if !ident_is(actual, segment) {
                matched = false;
                break;
            }
            cursor += 3;
        }
        count += usize::from(matched);
    }
    count
        + trees
            .into_iter()
            .map(|tree| match tree {
                TokenTree::Group(group) => token_stream_path_count(&group.stream(), expected),
                _ => 0,
            })
            .sum::<usize>()
}

fn is_inventory_registration_macro_name(name: &str) -> bool {
    matches!(name, "collect" | "submit" | "__do_submit")
}

fn token_stream_mentions_inventory_registration_path(tokens: &TokenStream) -> bool {
    ["collect", "submit", "__do_submit"]
        .iter()
        .any(|name| token_stream_mentions_path(tokens, &["inventory", name]))
}

fn token_tree_mentions_inventory_registration_name(tree: &TokenTree) -> bool {
    match tree {
        TokenTree::Ident(ident) => is_inventory_registration_macro_name(&normalized_ident(ident)),
        TokenTree::Group(group) => group
            .stream()
            .into_iter()
            .any(|nested| token_tree_mentions_inventory_registration_name(&nested)),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    }
}

fn token_stream_contains_registration_macro_import(tokens: &TokenStream) -> bool {
    let trees: Vec<_> = tokens.clone().into_iter().collect();
    for (index, tree) in trees.iter().enumerate() {
        let TokenTree::Ident(ident) = tree else {
            continue;
        };
        if !ident_is(ident, "use") {
            continue;
        }
        if trees[index + 1..]
            .iter()
            .take_while(|candidate| !is_punctuation(Some(candidate), ';'))
            .any(token_tree_mentions_inventory_registration_name)
        {
            return true;
        }
    }
    trees.into_iter().any(|tree| match tree {
        TokenTree::Group(group) => token_stream_contains_registration_macro_import(&group.stream()),
        _ => false,
    })
}

fn is_punctuation(token: Option<&TokenTree>, expected: char) -> bool {
    matches!(token, Some(TokenTree::Punct(punctuation)) if punctuation.as_char() == expected)
}

fn collect_token_macro_calls(tokens: &TokenStream, calls: &mut Vec<TokenMacroCall>) {
    let trees: Vec<_> = tokens.clone().into_iter().collect();

    for token in &trees {
        if let TokenTree::Group(group) = token {
            collect_token_macro_calls(&group.stream(), calls);
        }
    }

    for index in 0..trees.len() {
        let TokenTree::Ident(first_segment) = &trees[index] else {
            continue;
        };
        if index >= 3
            && is_punctuation(trees.get(index - 2), ':')
            && is_punctuation(trees.get(index - 1), ':')
            && matches!(trees.get(index - 3), Some(TokenTree::Ident(_)))
        {
            continue;
        }
        let mut path = vec![normalized_ident(first_segment)];
        let mut cursor = index + 1;
        while is_punctuation(trees.get(cursor), ':') && is_punctuation(trees.get(cursor + 1), ':') {
            let Some(TokenTree::Ident(segment)) = trees.get(cursor + 2) else {
                break;
            };
            path.push(normalized_ident(segment));
            cursor += 3;
        }
        if !is_punctuation(trees.get(cursor), '!') {
            continue;
        }
        let Some(TokenTree::Group(body)) = trees.get(cursor + 1) else {
            continue;
        };
        calls.push(TokenMacroCall {
            path,
            body: body.stream(),
        });
    }
}

fn token_macro_calls(tokens: &TokenStream) -> Vec<TokenMacroCall> {
    let mut calls = Vec::new();
    collect_token_macro_calls(tokens, &mut calls);
    calls
}

fn collect_token_macro_definitions(
    tokens: &TokenStream,
    definitions: &mut Vec<TokenMacroDefinition>,
) {
    let trees: Vec<_> = tokens.clone().into_iter().collect();
    for token in &trees {
        if let TokenTree::Group(group) = token {
            collect_token_macro_definitions(&group.stream(), definitions);
        }
    }

    for window in trees.windows(4) {
        let [
            TokenTree::Ident(macro_rules),
            TokenTree::Punct(bang),
            TokenTree::Ident(name),
            TokenTree::Group(body),
        ] = window
        else {
            continue;
        };
        if ident_is(macro_rules, "macro_rules") && bang.as_char() == '!' {
            definitions.push(TokenMacroDefinition {
                name: normalized_ident(name),
                body: body.stream(),
            });
        }
    }
}

fn token_macro_definitions(tokens: &TokenStream) -> Vec<TokenMacroDefinition> {
    let mut definitions = Vec::new();
    collect_token_macro_definitions(tokens, &mut definitions);
    definitions
}

fn token_stream_may_generate_handler(tokens: &TokenStream) -> bool {
    EXPECTED_REGISTRATION_MACROS
        .iter()
        .any(|expected| token_stream_mentions_ident(tokens, expected))
        || token_stream_mentions_ident(tokens, "PacketHandlerEntry")
        || token_stream_mentions_inventory_registration_path(tokens)
        || token_stream_contains_registration_macro_import(tokens)
        || (token_stream_mentions_ident(tokens, "inventory")
            && ["collect", "submit", "__do_submit"]
                .iter()
                .any(|name| token_stream_mentions_ident(tokens, name)))
        || token_stream_mentions_ident(tokens, "mod")
        || token_stream_mentions_ident(tokens, "macro_rules")
        || token_stream_contains_metavariable_macro_invocation(tokens)
        || token_macro_calls(tokens).iter().any(|call| {
            call.path
                .last()
                .is_some_and(|name| is_inventory_registration_macro_name(name))
                || call.path.last().is_some_and(|name| name == "include")
                || is_registration_macro_invocation(&call.path)
        })
}

fn macro_definition_may_generate_handler(definition: &TokenMacroDefinition) -> bool {
    EXPECTED_REGISTRATION_MACROS
        .iter()
        .any(|expected| definition.name == *expected)
        || token_stream_may_generate_handler(&definition.body)
}

fn inventory_tree_can_alias_registration_macro(tree: &UseTree) -> bool {
    match tree {
        UseTree::Path(path) => {
            is_inventory_registration_macro_name(&normalized_ident(&path.ident))
                || inventory_tree_can_alias_registration_macro(&path.tree)
        }
        UseTree::Name(name) => {
            ident_is(&name.ident, "inventory")
                || is_inventory_registration_macro_name(&normalized_ident(&name.ident))
        }
        UseTree::Rename(_) | UseTree::Glob(_) => true,
        UseTree::Group(group) => group
            .items
            .iter()
            .any(inventory_tree_can_alias_registration_macro),
    }
}

fn use_tree_can_alias_inventory_submit(tree: &UseTree) -> bool {
    match tree {
        UseTree::Path(path) if ident_is(&path.ident, "inventory") => {
            inventory_tree_can_alias_registration_macro(&path.tree)
        }
        UseTree::Path(path) => {
            is_inventory_registration_macro_name(&normalized_ident(&path.ident))
                || use_tree_can_alias_inventory_submit(&path.tree)
        }
        UseTree::Name(name) => {
            ident_is(&name.ident, "inventory")
                || is_inventory_registration_macro_name(&normalized_ident(&name.ident))
        }
        UseTree::Rename(rename) => {
            ident_is(&rename.ident, "inventory")
                || ident_is(&rename.rename, "inventory")
                || is_inventory_registration_macro_name(&normalized_ident(&rename.ident))
                || is_inventory_registration_macro_name(&normalized_ident(&rename.rename))
        }
        UseTree::Group(group) => group.items.iter().any(use_tree_can_alias_inventory_submit),
        UseTree::Glob(_) => false,
    }
}

fn use_tree_can_alias_expected_registration_macro(tree: &UseTree) -> bool {
    match tree {
        UseTree::Path(path) => use_tree_can_alias_expected_registration_macro(&path.tree),
        UseTree::Name(name) => EXPECTED_REGISTRATION_MACROS
            .iter()
            .any(|expected| ident_is(&name.ident, expected)),
        UseTree::Rename(rename) => EXPECTED_REGISTRATION_MACROS.iter().any(|expected| {
            ident_is(&rename.ident, expected) || ident_is(&rename.rename, expected)
        }),
        UseTree::Group(group) => group
            .items
            .iter()
            .any(use_tree_can_alias_expected_registration_macro),
        UseTree::Glob(_) => false,
    }
}

#[derive(Default)]
struct InventoryAliasCollector {
    violations: Vec<String>,
}

impl<'ast> Visit<'ast> for InventoryAliasCollector {
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if use_tree_can_alias_inventory_submit(&item.tree) {
            self.violations.push(format!(
                "import {} can alias an inventory registration macro; use only canonical \
                 inventory::collect!/inventory::submit! paths",
                item.to_token_stream()
            ));
        }
        if use_tree_can_alias_expected_registration_macro(&item.tree) {
            self.violations.push(format!(
                "import {} aliases or reexports an audited handler registration macro; \
                 registration macros must remain private to crate::handlers and use their \
                 unqualified audited names",
                item.to_token_stream()
            ));
        }
        syn::visit::visit_item_use(self, item);
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        if ident_is(&item.ident, "inventory")
            || item
                .rename
                .as_ref()
                .is_some_and(|(_, rename)| ident_is(rename, "inventory"))
        {
            self.violations.push(format!(
                "{} is not allowed because #[macro_use] or a crate alias can hide inventory \
                 registration macros; use only canonical qualified paths",
                item.to_token_stream()
            ));
        }
        syn::visit::visit_item_extern_crate(self, item);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if ident_is(&item.ident, "inventory") {
            self.violations.push(format!(
                "module {} shadows the canonical inventory crate namespace",
                item.ident
            ));
        }
        syn::visit::visit_item_mod(self, item);
    }
}

pub(crate) fn registration_alias_violations(source: &str) -> Result<Vec<String>, String> {
    let syntax =
        syn::parse_file(source).map_err(|error| format!("cannot parse Rust source: {error}"))?;
    let mut collector = InventoryAliasCollector::default();
    collector.visit_file(&syntax);
    Ok(collector.violations)
}

fn is_exact_packet_handler_collector(path: &[String], body: &TokenStream) -> bool {
    if path != ["inventory", "collect"] {
        return false;
    }
    let Ok(handler_type) = syn::parse2::<syn::Path>(body.clone()) else {
        return false;
    };
    handler_type.leading_colon.is_none()
        && handler_type.segments.len() == 1
        && handler_type.segments.first().is_some_and(|segment| {
            ident_is(&segment.ident, "PacketHandlerEntry")
                && matches!(segment.arguments, syn::PathArguments::None)
        })
}

#[derive(Debug, Default)]
pub(crate) struct OutsideRegistrationReport {
    pub(crate) exact_packet_handler_collectors: usize,
}

pub(crate) fn handler_capable_macro_definitions(
    source_path: &Path,
    source: &str,
) -> Result<Vec<String>, String> {
    let tokens: TokenStream = source
        .parse()
        .map_err(|error| format!("cannot tokenize {}: {error}", source_path.display()))?;
    let mut definitions: Vec<_> = token_macro_definitions(&tokens)
        .into_iter()
        .filter(macro_definition_may_generate_handler)
        .map(|definition| definition.name)
        .collect();
    definitions.sort();
    definitions.dedup();
    Ok(definitions)
}

pub(crate) fn handler_capable_macro_invocations(
    source_path: &Path,
    source: &str,
) -> Result<Vec<String>, String> {
    let tokens: TokenStream = source
        .parse()
        .map_err(|error| format!("cannot tokenize {}: {error}", source_path.display()))?;
    let mut invocations: Vec<_> = token_macro_calls(&tokens)
        .into_iter()
        .filter(|call| {
            is_registration_macro_invocation(&call.path)
                || token_stream_may_generate_handler(&call.body)
        })
        .map(|call| call.path.join("::"))
        .collect();
    invocations.sort();
    invocations.dedup();
    Ok(invocations)
}

pub(crate) fn inventory_registration_macro_fingerprints(
    source_path: &Path,
    source: &str,
) -> Result<Vec<String>, String> {
    let tokens: TokenStream = source
        .parse()
        .map_err(|error| format!("cannot tokenize {}: {error}", source_path.display()))?;
    let mut fingerprints: Vec<_> = token_macro_calls(&tokens)
        .into_iter()
        .filter(|call| {
            call.path
                .last()
                .is_some_and(|name| is_inventory_registration_macro_name(name))
        })
        .map(|call| format!("{}!{{{}}}", call.path.join("::"), call.body.to_string()))
        .collect();
    fingerprints.sort();
    Ok(fingerprints)
}

#[derive(Default)]
struct ExportedMacroCollector {
    names: Vec<String>,
    invalid: Vec<String>,
}

impl<'ast> Visit<'ast> for ExportedMacroCollector {
    fn visit_item_macro(&mut self, item: &'ast ItemMacro) {
        if item
            .attrs
            .iter()
            .any(|attribute| attribute.path().is_ident("macro_export"))
        {
            if item.mac.path.is_ident("macro_rules") {
                if let Some(name) = &item.ident {
                    self.names.push(normalized_ident(name));
                } else {
                    self.invalid
                        .push("#[macro_export] macro_rules! without a name".to_owned());
                }
            } else {
                self.invalid.push(format!(
                    "#[macro_export] is attached to unsupported item macro {}",
                    item.mac.path.to_token_stream()
                ));
            }
        }
        syn::visit::visit_item_macro(self, item);
    }
}

pub(crate) fn exported_macro_names(
    source_path: &Path,
    source: &str,
) -> Result<Vec<String>, String> {
    let syntax = syn::parse_file(source)
        .map_err(|error| format!("cannot parse {}: {error}", source_path.display()))?;
    let mut collector = ExportedMacroCollector::default();
    collector.visit_file(&syntax);
    if !collector.invalid.is_empty() {
        return Err(format!(
            "{} has invalid macro export grammar: {}",
            source_path.display(),
            collector.invalid.join("; ")
        ));
    }
    collector.names.sort();
    if collector.names.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(format!(
            "{} exports a duplicate macro name",
            source_path.display()
        ));
    }
    Ok(collector.names)
}

pub(crate) fn include_macro_bodies(
    source_path: &Path,
    source: &str,
) -> Result<Vec<String>, String> {
    let tokens: TokenStream = source
        .parse()
        .map_err(|error| format!("cannot tokenize {}: {error}", source_path.display()))?;
    let mut bodies: Vec<_> = token_macro_calls(&tokens)
        .into_iter()
        .filter(|call| call.path.last().is_some_and(|name| name == "include"))
        .map(|call| call.body.to_string())
        .collect();
    bodies.sort();
    Ok(bodies)
}

pub(crate) fn analyze_registration_syntax_outside_handlers(
    source_path: &Path,
    source: &str,
    allow_exact_packet_handler_collector: bool,
) -> Result<OutsideRegistrationReport, String> {
    // Tokenize the complete source without evaluating cfg predicates. This is
    // intentionally independent of rustc's active target/profile so an
    // otherwise invisible #[cfg(...)] registration is still audited.
    let tokens: TokenStream = source
        .parse()
        .map_err(|error| format!("cannot tokenize {}: {error}", source_path.display()))?;
    let mut violations = registration_alias_violations(source)?
        .into_iter()
        .map(|violation| format!("{} {violation}", source_path.display()))
        .collect::<Vec<_>>();
    let syntax = syn::parse_file(source)
        .map_err(|error| format!("cannot parse {}: {error}", source_path.display()))?;
    let top_level_exact_collectors: Vec<_> = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Macro(item)
                if is_exact_packet_handler_collector(
                    &path_segments(&item.mac.path),
                    &item.mac.tokens,
                ) =>
            {
                Some(item)
            }
            _ => None,
        })
        .collect();
    let exact_collector_calls = token_macro_calls(&tokens)
        .iter()
        .filter(|call| is_exact_packet_handler_collector(&call.path, &call.body))
        .count();
    if allow_exact_packet_handler_collector {
        if exact_collector_calls != top_level_exact_collectors.len() {
            violations.push(format!(
                "{} contains an exact inventory::collect!(PacketHandlerEntry) outside module item \
                 level; the collector is allowed only as a top-level item",
                source_path.display()
            ));
        }
        if conditional_attribute_name(&syntax.attrs).is_some() {
            violations.push(format!(
                "{} guards the PacketHandlerEntry collector source with cfg/cfg_attr",
                source_path.display()
            ));
        }
        for collector in &top_level_exact_collectors {
            if conditional_attribute_name(&collector.attrs).is_some() {
                violations.push(format!(
                    "{} conditionally compiles inventory::collect!(PacketHandlerEntry)",
                    source_path.display()
                ));
            }
        }
    }
    for definition in token_macro_definitions(&tokens)
        .into_iter()
        .filter(macro_definition_may_generate_handler)
    {
        violations.push(format!(
            "{} defines handler-capable macro_rules! {} outside crate::handlers",
            source_path.display(),
            definition.name
        ));
    }
    for call in token_macro_calls(&tokens) {
        if call.path.last().is_some_and(|name| name == "include") {
            violations.push(format!(
                "{} uses include! outside crate::handlers; included source could hide a handler \
                 registration",
                source_path.display()
            ));
        } else if allow_exact_packet_handler_collector
            && is_exact_packet_handler_collector(&call.path, &call.body)
        {
            continue;
        } else if call
            .path
            .last()
            .is_some_and(|name| is_inventory_registration_macro_name(name))
        {
            violations.push(format!(
                "{} invokes inventory registration macro {}! outside crate::handlers; all \
                 production submissions belong to the audited handler owner",
                source_path.display(),
                call.path.join("::")
            ));
        } else if token_stream_mentions_inventory_registration_path(&call.body) {
            violations.push(format!(
                "{} passes an inventory registration path through macro {}! outside \
                 crate::handlers; registration forwarders are outside the audited grammar",
                source_path.display(),
                call.path.join("::")
            ));
        } else if macro_call_mentions_handler_entry(&call.body) {
            violations.push(format!(
                "{} contains a macro call mentioning PacketHandlerEntry outside crate::handlers",
                source_path.display()
            ));
        } else if token_stream_may_generate_handler(&call.body) {
            violations.push(format!(
                "{} passes handler-capable source tokens through macro {}! outside \
                 crate::handlers; source-generating macro calls are outside the audited grammar",
                source_path.display(),
                call.path.join("::")
            ));
        } else if is_registration_macro_invocation(&call.path) {
            violations.push(format!(
                "{} invokes audited handler registration macro {}! outside crate::handlers",
                source_path.display(),
                call.path.join("::")
            ));
        }
    }
    violations.sort();
    violations.dedup();

    if violations.is_empty() {
        Ok(OutsideRegistrationReport {
            exact_packet_handler_collectors: top_level_exact_collectors.len(),
        })
    } else {
        Err(violations.join("\n"))
    }
}

#[cfg(test)]
pub(crate) fn reject_registration_syntax_outside_handlers(
    source_path: &Path,
    source: &str,
) -> Result<(), String> {
    analyze_registration_syntax_outside_handlers(source_path, source, false).map(|_| ())
}

fn path_segments(path: &syn::Path) -> Vec<String> {
    path.segments
        .iter()
        .map(|segment| normalized_ident(&segment.ident))
        .collect()
}

fn is_direct_handler_submission(path: &[String], _body: &TokenStream) -> bool {
    is_inventory_submit(path)
}

fn is_inventory_submit(path: &[String]) -> bool {
    path == ["inventory", "submit"]
}

fn macro_call_mentions_handler_entry(body: &TokenStream) -> bool {
    token_stream_mentions_ident(body, "PacketHandlerEntry")
}

fn is_registration_macro_invocation(path: &[String]) -> bool {
    path.last().is_some_and(|name| {
        EXPECTED_REGISTRATION_MACROS
            .iter()
            .any(|expected| name == expected)
    })
}

fn token_stream_contains_macro_repetition(tokens: &TokenStream) -> bool {
    let trees: Vec<_> = tokens.clone().into_iter().collect();
    if trees.windows(3).any(|window| {
        is_punctuation(window.first(), '$')
            && matches!(window.get(1), Some(TokenTree::Group(_)))
            && matches!(
                window.get(2),
                Some(TokenTree::Punct(punctuation))
                    if matches!(punctuation.as_char(), '*' | '+' | '?')
            )
    }) {
        return true;
    }
    trees.into_iter().any(|tree| match tree {
        TokenTree::Group(group) => token_stream_contains_macro_repetition(&group.stream()),
        _ => false,
    })
}

fn token_stream_contains_metavariable_macro_invocation(tokens: &TokenStream) -> bool {
    let trees: Vec<_> = tokens.clone().into_iter().collect();
    if trees.windows(3).any(|window| {
        is_punctuation(window.first(), '$')
            && matches!(window.get(1), Some(TokenTree::Ident(_)))
            && is_punctuation(window.get(2), '!')
    }) {
        return true;
    }
    trees.into_iter().any(|tree| match tree {
        TokenTree::Group(group) => {
            token_stream_contains_metavariable_macro_invocation(&group.stream())
        }
        _ => false,
    })
}

fn top_level_fat_arrow_count(tokens: &TokenStream) -> usize {
    let trees: Vec<_> = tokens.clone().into_iter().collect();
    trees
        .windows(2)
        .filter(|window| is_punctuation(window.first(), '=') && is_punctuation(window.get(1), '>'))
        .count()
}

fn definition_direct_submission_count(definition: &MacroDefinitionSite) -> usize {
    definition
        .calls
        .iter()
        .filter(|call| is_direct_handler_submission(&call.path, &call.body))
        .count()
}

fn module_location(source_path: &Path, module_path: &str) -> String {
    format!("{} ({module_path})", source_path.display())
}

fn collect_macro_item(
    item_macro: &ItemMacro,
    inherited_condition: Option<String>,
    source_path: &Path,
    module_path: &str,
    collection: &mut SourceCollection,
) {
    let location = module_location(source_path, module_path);
    let conditional_context =
        with_conditional_context(inherited_condition, &item_macro.attrs, &location);
    let contains_conditional_tokens = token_stream_mentions_ident(&item_macro.mac.tokens, "cfg")
        || token_stream_mentions_ident(&item_macro.mac.tokens, "cfg_attr");

    if item_macro.mac.path.is_ident("macro_rules") {
        if let Some(name) = &item_macro.ident {
            let normalized_name = normalized_ident(name);
            let handler_capable = macro_definition_may_generate_handler(&TokenMacroDefinition {
                name: normalized_name.clone(),
                body: item_macro.mac.tokens.clone(),
            });
            if handler_capable
                && item_macro
                    .attrs
                    .iter()
                    .any(|attribute| attribute.path().is_ident("macro_export"))
            {
                collection.unsupported_registration_generators.push(format!(
                    "handler-capable macro {normalized_name} uses #[macro_export] in {location}; \
                     registration macros must remain private to crate::handlers"
                ));
            }
            let calls = token_macro_calls(&item_macro.mac.tokens);
            let canonical_submit_calls = calls
                .iter()
                .filter(|call| is_inventory_submit(&call.path))
                .count();
            let inventory_registration_path_mentions = ["collect", "submit", "__do_submit"]
                .iter()
                .map(|registration_macro| {
                    token_stream_path_count(
                        &item_macro.mac.tokens,
                        &["inventory", registration_macro],
                    )
                })
                .sum::<usize>();
            if inventory_registration_path_mentions != canonical_submit_calls
                || calls.iter().any(|call| {
                    call.path
                        .last()
                        .is_some_and(|segment| is_inventory_registration_macro_name(segment))
                        && !is_inventory_submit(&call.path)
                })
            {
                collection.unsupported_registration_generators.push(format!(
                    "registration macro {} aliases/imports an inventory registration macro in {}; \
                     use only one direct inventory::submit! call in the audited module-level \
                     grammar",
                    name, location
                ));
            }
            collection.definitions.push(MacroDefinitionSite {
                name: normalized_name,
                handler_capable,
                calls,
                contains_conditional_tokens,
                contains_repetition: token_stream_contains_macro_repetition(&item_macro.mac.tokens),
                rule_arm_count: top_level_fat_arrow_count(&item_macro.mac.tokens),
                conditional_context,
                location,
            });
        }
        return;
    }

    collection.invocations.push(MacroInvocationSite {
        path: path_segments(&item_macro.mac.path),
        body: item_macro.mac.tokens.clone(),
        contains_conditional_tokens,
        conditional_context,
        item_level: true,
        location,
    });
}

fn collect_nested_item_macros(
    item: &Item,
    inherited_condition: Option<String>,
    source_path: &Path,
    module_path: &str,
    collection: &mut SourceCollection,
) {
    let tokens = item.to_token_stream();
    let contains_conditional_tokens = token_stream_mentions_ident(&tokens, "cfg")
        || token_stream_mentions_ident(&tokens, "cfg_attr");
    let location = module_location(source_path, module_path);
    for definition in token_macro_definitions(&tokens)
        .into_iter()
        .filter(macro_definition_may_generate_handler)
    {
        collection.unsupported_registration_generators.push(format!(
            "nested macro_rules! {} may generate a handler registration in {}; \
                 move audited registration macros to module scope",
            definition.name, location
        ));
    }
    for call in token_macro_calls(&tokens) {
        collection.invocations.push(MacroInvocationSite {
            path: call.path,
            body: call.body,
            contains_conditional_tokens,
            conditional_context: inherited_condition.clone(),
            item_level: false,
            location: location.clone(),
        });
    }
}

pub(crate) fn path_override(attributes: &[Attribute]) -> Result<Option<PathBuf>, String> {
    let mut resolved_path = None;
    for attribute in attributes {
        if attribute.path().is_ident("cfg_attr")
            && token_stream_mentions_ident(&attribute.meta.to_token_stream(), "path")
        {
            return Err(
                "module #[cfg_attr(..., path = ...)] is not allowed; use one unconditional \
                 #[path = \"...\"] attribute"
                    .to_owned(),
            );
        }
        if !attribute.path().is_ident("path") {
            continue;
        }
        if resolved_path.is_some() {
            return Err("module must not declare more than one #[path] attribute".to_owned());
        }
        let Meta::NameValue(name_value) = &attribute.meta else {
            return Err("module #[path] attribute must be a name-value string".to_owned());
        };
        let Expr::Lit(expression) = &name_value.value else {
            return Err("module #[path] value must be a string literal".to_owned());
        };
        let Lit::Str(path) = &expression.lit else {
            return Err("module #[path] value must be a string literal".to_owned());
        };
        resolved_path = Some(PathBuf::from(path.value()));
    }
    Ok(resolved_path)
}

fn resolve_external_module(
    source_path: &Path,
    module_dir: &Path,
    item_mod: &syn::ItemMod,
) -> Result<(PathBuf, PathBuf), String> {
    if let Some(path) = path_override(&item_mod.attrs)? {
        let resolved = source_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path);
        let parent = resolved.parent().unwrap_or_else(|| Path::new("."));
        let child_dir =
            if resolved.file_name().is_some_and(|name| name == "mod.rs") {
                parent.to_owned()
            } else {
                parent.join(resolved.file_stem().ok_or_else(|| {
                    format!("module path {} has no file stem", resolved.display())
                })?)
            };
        return Ok((resolved, child_dir));
    }

    let module_name = normalized_ident(&item_mod.ident);
    let flat = module_dir.join(format!("{module_name}.rs"));
    let nested = module_dir.join(&module_name).join("mod.rs");
    let source_file = if flat.is_file() {
        flat
    } else if nested.is_file() {
        nested
    } else {
        return Err(format!(
            "cannot resolve module {module_name} declared in {}",
            source_path.display()
        ));
    };
    Ok((source_file, module_dir.join(module_name)))
}

fn collect_items(
    items: &[Item],
    source_path: &Path,
    module_dir: &Path,
    module_path: &str,
    inside_inline_module: bool,
    inherited_condition: Option<String>,
    collection: &mut SourceCollection,
    file_stack: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for item in items {
        match item {
            Item::Macro(item_macro) => collect_macro_item(
                item_macro,
                inherited_condition.clone(),
                source_path,
                module_path,
                collection,
            ),
            Item::Mod(item_mod) => {
                let child_module_path = format!("{module_path}::{}", item_mod.ident);
                let module_owner = module_location(source_path, &child_module_path);
                let child_condition = with_conditional_context(
                    inherited_condition.clone(),
                    &item_mod.attrs,
                    &module_owner,
                );
                let child_module_dir = module_dir.join(normalized_ident(&item_mod.ident));
                if let Some((_, inline_items)) = &item_mod.content {
                    if path_override(&item_mod.attrs)?.is_some() {
                        return Err(format!(
                            "inline module {} in {} must not use #[path]; move the module to a \
                             separate .rs file before using an explicit path",
                            item_mod.ident,
                            source_path.display()
                        ));
                    }
                    collect_items(
                        inline_items,
                        source_path,
                        &child_module_dir,
                        &child_module_path,
                        true,
                        child_condition,
                        collection,
                        file_stack,
                    )?;
                } else {
                    if inside_inline_module && path_override(&item_mod.attrs)?.is_some() {
                        return Err(format!(
                            "#[path] module {} in {} is declared inside an inline module; this \
                             closed audit grammar permits #[path] only in file modules",
                            item_mod.ident,
                            source_path.display()
                        ));
                    }
                    let (child_source, resolved_module_dir) =
                        resolve_external_module(source_path, module_dir, item_mod)?;
                    collect_source_file(
                        &child_source,
                        &resolved_module_dir,
                        &child_module_path,
                        child_condition,
                        collection,
                        file_stack,
                    )?;
                }
            }
            _ => collect_nested_item_macros(
                item,
                inherited_condition.clone(),
                source_path,
                module_path,
                collection,
            ),
        }
    }
    Ok(())
}

fn collect_source_file(
    source_path: &Path,
    module_dir: &Path,
    module_path: &str,
    inherited_condition: Option<String>,
    collection: &mut SourceCollection,
    file_stack: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let source_path = source_path
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", source_path.display()))?;
    if file_stack.contains(&source_path) {
        return Err(format!(
            "recursive Rust module path while reading {}",
            source_path.display()
        ));
    }
    let source = fs::read_to_string(&source_path)
        .map_err(|error| format!("cannot read {}: {error}", source_path.display()))?;
    let alias_violations = registration_alias_violations(&source)?;
    if !alias_violations.is_empty() {
        return Err(format!(
            "{}: {}",
            source_path.display(),
            alias_violations.join("; ")
        ));
    }
    let syntax = syn::parse_file(&source)
        .map_err(|error| format!("cannot parse {}: {error}", source_path.display()))?;
    let file_owner = module_location(&source_path, module_path);
    let file_condition = with_conditional_context(inherited_condition, &syntax.attrs, &file_owner);

    file_stack.push(source_path.clone());
    let result = collect_items(
        &syntax.items,
        &source_path,
        module_dir,
        module_path,
        false,
        file_condition,
        collection,
        file_stack,
    );
    file_stack.pop();
    result
}

fn discover_registration_macros(
    definitions: &[MacroDefinitionSite],
) -> Result<BTreeSet<String>, String> {
    let mut definitions_by_name = BTreeMap::new();
    for definition in definitions {
        if definitions_by_name
            .insert(definition.name.clone(), definition)
            .is_some()
        {
            return Err(format!(
                "duplicate macro_rules! definition named {}; registration macros must have \
                 crate-unique names for deterministic auditing",
                definition.name
            ));
        }
    }

    let mut registration_names: BTreeSet<_> = definitions
        .iter()
        .filter(|definition| definition_direct_submission_count(definition) > 0)
        .map(|definition| definition.name.clone())
        .collect();
    loop {
        let before = registration_names.len();
        for definition in definitions {
            if definition.calls.iter().any(|call| {
                call.path
                    .last()
                    .is_some_and(|name| registration_names.contains(name))
            }) {
                registration_names.insert(definition.name.clone());
            }
        }
        if registration_names.len() == before {
            break;
        }
    }
    Ok(registration_names)
}

fn classify_registration_sources(
    collection: SourceCollection,
) -> Result<RegistrationSourceReport, String> {
    let registration_names = discover_registration_macros(&collection.definitions)?;
    let mut errors = collection.unsupported_registration_generators.clone();

    for definition in collection.definitions.iter().filter(|definition| {
        definition.handler_capable && !registration_names.contains(&definition.name)
    }) {
        errors.push(format!(
            "handler-capable macro {} in {} is outside the exact audited registration-macro \
             grammar",
            definition.name, definition.location
        ));
    }

    for definition in collection
        .definitions
        .iter()
        .filter(|definition| registration_names.contains(&definition.name))
    {
        if let Some(context) = &definition.conditional_context {
            errors.push(format!(
                "registration macro {} is conditionally compiled: {context}",
                definition.name
            ));
        }
        if definition.contains_conditional_tokens {
            errors.push(format!(
                "registration macro {} contains cfg/cfg_attr tokens in {}",
                definition.name, definition.location
            ));
        }
        if definition.contains_repetition {
            errors.push(format!(
                "registration macro {} contains a macro repetition in {}; the guard cannot prove \
                 one PacketHandlerEntry per invocation",
                definition.name, definition.location
            ));
        }
        if definition.rule_arm_count != 1 {
            errors.push(format!(
                "registration macro {} has {} rule arms in {}; the guard requires exactly one \
                 unambiguous rule",
                definition.name, definition.rule_arm_count, definition.location
            ));
        }

        let registration_calls = definition_direct_submission_count(definition)
            + definition
                .calls
                .iter()
                .filter(|call| {
                    call.path
                        .last()
                        .is_some_and(|name| registration_names.contains(name))
                })
                .count();
        if registration_calls != 1 {
            errors.push(format!(
                "registration macro {} contains {registration_calls} registration expansions in \
                 {}; the guard requires exactly one PacketHandlerEntry per invocation",
                definition.name, definition.location
            ));
        }
    }

    let mut direct_submissions = 0usize;
    let mut registration_macro_invocations = 0usize;
    for invocation in &collection.invocations {
        let direct = is_direct_handler_submission(&invocation.path, &invocation.body);
        let via_registration_macro = invocation
            .path
            .last()
            .is_some_and(|name| registration_names.contains(name));
        if !invocation.item_level
            && (invocation
                .path
                .last()
                .is_some_and(|name| is_inventory_registration_macro_name(name))
                || invocation.path.last().is_some_and(|name| name == "include")
                || token_stream_mentions_inventory_registration_path(&invocation.body)
                || macro_call_mentions_handler_entry(&invocation.body)
                || via_registration_macro)
        {
            errors.push(format!(
                "handler-capable macro {}! appears inside a block/item body in {}; registration \
                 grammar is allowed only at module item level",
                invocation.path.join("::"),
                invocation.location
            ));
            continue;
        }
        if invocation.item_level && !direct && !via_registration_macro {
            errors.push(format!(
                "unsupported item-level macro {}! in {}; it could generate unaudited handler \
                 source",
                invocation.path.join("::"),
                invocation.location
            ));
        }
        if !direct && !via_registration_macro {
            continue;
        }

        if direct {
            direct_submissions += 1;
        } else {
            registration_macro_invocations += 1;
            if invocation.path.len() != 1 {
                errors.push(format!(
                    "registration macro invocation {} must use its unqualified audited name in {}",
                    invocation.path.join("::"),
                    invocation.location
                ));
            }
        }
        let display_name = invocation
            .path
            .last()
            .map(String::as_str)
            .unwrap_or("<unknown>");
        if let Some(context) = &invocation.conditional_context {
            errors.push(format!(
                "handler registration {display_name}! is conditionally compiled: {context}"
            ));
        }
        if invocation.contains_conditional_tokens {
            errors.push(format!(
                "handler registration {display_name}! contains cfg/cfg_attr tokens in {}",
                invocation.location
            ));
        }
    }

    if errors.is_empty() {
        Ok(RegistrationSourceReport {
            direct_submissions,
            registration_macro_invocations,
            registration_macro_names: registration_names,
        })
    } else {
        Err(errors.join("\n"))
    }
}

pub(crate) fn analyze_handler_source(
    crate_root: &Path,
) -> Result<RegistrationSourceReport, String> {
    let crate_root = crate_root.canonicalize().map_err(|error| {
        format!(
            "cannot resolve crate root {}: {error}",
            crate_root.display()
        )
    })?;
    let source_dir = crate_root
        .parent()
        .ok_or_else(|| format!("crate root {} has no parent", crate_root.display()))?;
    let source = fs::read_to_string(&crate_root)
        .map_err(|error| format!("cannot read {}: {error}", crate_root.display()))?;
    let alias_violations = registration_alias_violations(&source)?;
    if !alias_violations.is_empty() {
        return Err(format!(
            "{}: {}",
            crate_root.display(),
            alias_violations.join("; ")
        ));
    }
    let syntax = syn::parse_file(&source)
        .map_err(|error| format!("cannot parse {}: {error}", crate_root.display()))?;
    let root_condition =
        with_conditional_context(None, &syntax.attrs, &module_location(&crate_root, "crate"));
    let handlers_modules: Vec<_> = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Mod(item_mod) if ident_is(&item_mod.ident, "handlers") => Some(item_mod),
            _ => None,
        })
        .collect();
    if handlers_modules.len() != 1 {
        return Err(format!(
            "expected exactly one crate::handlers module declaration, found {}",
            handlers_modules.len()
        ));
    }
    let handlers_module = handlers_modules[0];
    let handlers_condition = with_conditional_context(
        root_condition,
        &handlers_module.attrs,
        &module_location(&crate_root, "crate::handlers"),
    );

    let mut collection = SourceCollection::default();
    if let Some((_, inline_items)) = &handlers_module.content {
        collect_items(
            inline_items,
            &crate_root,
            &source_dir.join("handlers"),
            "crate::handlers",
            true,
            handlers_condition,
            &mut collection,
            &mut vec![crate_root.clone()],
        )?;
    } else {
        let (handlers_source, handlers_module_dir) =
            resolve_external_module(&crate_root, source_dir, handlers_module)?;
        collect_source_file(
            &handlers_source,
            &handlers_module_dir,
            "crate::handlers",
            handlers_condition,
            &mut collection,
            &mut vec![crate_root.clone()],
        )?;
    }
    classify_registration_sources(collection)
}

#[cfg(test)]
pub(crate) fn analyze_inline_source(source: &str) -> Result<RegistrationSourceReport, String> {
    let alias_violations = registration_alias_violations(source)?;
    if !alias_violations.is_empty() {
        return Err(alias_violations.join("; "));
    }
    let syntax = syn::parse_file(source)
        .map_err(|error| format!("cannot parse synthetic source: {error}"))?;
    let mut collection = SourceCollection::default();
    let source_path = Path::new("<synthetic>");
    let file_condition = with_conditional_context(None, &syntax.attrs, "synthetic crate source");
    collect_items(
        &syntax.items,
        source_path,
        Path::new("."),
        "crate",
        false,
        file_condition,
        &mut collection,
        &mut Vec::new(),
    )?;
    classify_registration_sources(collection)
}
