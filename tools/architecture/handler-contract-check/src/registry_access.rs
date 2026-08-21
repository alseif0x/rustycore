// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Exact syntax inventory for direct access to the transitional registries.
//!
//! `PlayerRegistry`, `GroupRegistry`, and `PendingInvites` are currently public
//! aliases for `DashMap`. That makes a simple text search an unsafe ratchet:
//! imports can rename the aliases, values can flow through `Arc`/`Option`, and
//! a same-count replacement can change a read into a write. This module parses
//! production Rust with `syn`, follows the ordinary alias/value shapes used by
//! the workspace, and records exact, deterministic access fingerprints.
//!
//! This is intentionally a strict source guard, not a Rust type checker. It
//! understands explicit imports, type aliases, typed fields/parameters/locals,
//! local assignments, the known `WorldSession` accessors, and common wrapper
//! methods. An unknown macro receiving a known registry value is rejected
//! rather than silently omitted. Procedural-macro expansion and registry
//! values obtained from an untyped external generic remain outside `syn`'s
//! knowledge; callers must keep those surfaces out of the accepted grammar or
//! add an explicit, tested rule before using them.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprCall, ExprClosure, ExprField, ExprIf, ExprMacro, ExprMatch,
    ExprMethodCall, ExprReturn, FnArg, ImplItem, Item, ItemEnum, ItemFn, ItemImpl, ItemMod,
    ItemStruct, ItemType, ItemUse, Local, Member, Pat, ReturnType, Stmt, Type, UseTree, Visibility,
};

use crate::ownership::{cfg_context_allows_production, extend_cfg_context};

const REGISTRY_SCHEMA_VERSION: u32 = 1;
const PASSTHROUGH_METHODS: &[&str] = &[
    "as_ref",
    "as_deref",
    "as_mut",
    "as_deref_mut",
    "unwrap",
    "unwrap_or",
    "unwrap_or_else",
    "expect",
    "map",
    "and_then",
    "filter",
    "inspect",
    "or",
    "or_else",
];
const KNOWN_OPAQUE_VALUE_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "debug",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "error",
    "format",
    "info",
    "matches",
    "trace",
    "tracing",
    "warn",
];

/// One of the three public registry aliases being retired by #150/#151.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum RegistryKind {
    #[serde(rename = "PlayerRegistry")]
    Player,
    #[serde(rename = "GroupRegistry")]
    Group,
    #[serde(rename = "PendingInvites")]
    PendingInvites,
}

impl RegistryKind {
    fn source_name(self) -> &'static str {
        match self {
            Self::Player => "PlayerRegistry",
            Self::Group => "GroupRegistry",
            Self::PendingInvites => "PendingInvites",
        }
    }

    fn from_source_name(name: &str) -> Option<Self> {
        match name {
            "PlayerRegistry" => Some(Self::Player),
            "GroupRegistry" => Some(Self::Group),
            "PendingInvites" => Some(Self::PendingInvites),
            _ => None,
        }
    }

    fn from_member_or_accessor(name: &str) -> Option<Self> {
        match name {
            "player_registry" => Some(Self::Player),
            "group_registry" => Some(Self::Group),
            "pending_invites" => Some(Self::PendingInvites),
            _ => None,
        }
    }
}

/// The exact syntactic capability exposed at an access site.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RegistryOperation {
    TypeReference,
    ImportAlias,
    TypeAlias,
    Member,
    Accessor,
    Construct,
    LocalAlias,
    AssignmentAlias,
    Clone,
    Return,
    ArgumentEscape,
    Index,
    Get,
    GetMut,
    Iter,
    Entry,
    Insert,
    Remove,
    Retain,
    Clear,
    OpaqueMacroBoundary,
}

impl RegistryOperation {
    fn from_method(name: &str) -> Option<Self> {
        match name {
            "get" => Some(Self::Get),
            "get_mut" => Some(Self::GetMut),
            "iter" => Some(Self::Iter),
            "entry" => Some(Self::Entry),
            "insert" => Some(Self::Insert),
            "remove" => Some(Self::Remove),
            "retain" => Some(Self::Retain),
            "clear" => Some(Self::Clear),
            _ => None,
        }
    }
}

/// A canonical baseline row. `count` preserves identical repeated operations;
/// all other fields form the exact identity used by the comparator.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegistryAccessRecord {
    pub(crate) package: String,
    pub(crate) module: String,
    pub(crate) source: String,
    pub(crate) enclosing: String,
    pub(crate) registry: RegistryKind,
    pub(crate) operation: RegistryOperation,
    pub(crate) symbol: String,
    pub(crate) visibility: String,
    pub(crate) cfg: Vec<String>,
    pub(crate) fingerprint: String,
    pub(crate) count: usize,
}

/// Serializable exact snapshot. Rows emitted by the inventory are sorted,
/// unique by identity, and carry a positive multiplicity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegistryAccessBaseline {
    pub(crate) schema_version: u32,
    pub(crate) accesses: Vec<RegistryAccessRecord>,
}

/// One already-classified production source mount. The workspace/module walker
/// remains the caller's responsibility so this module can be tested without
/// executing Cargo metadata. Pass repository-relative `source_path` values.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProductionRegistrySource<'a> {
    pub(crate) package: &'a str,
    pub(crate) module: &'a str,
    pub(crate) source_path: &'a str,
    pub(crate) inherited_cfg: &'a [String],
    pub(crate) source: &'a str,
}

struct ParsedRegistrySource<'a> {
    mount: ProductionRegistrySource<'a>,
    syntax: syn::File,
    cfg: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct AccessIdentity {
    package: String,
    module: String,
    source: String,
    enclosing: String,
    registry: RegistryKind,
    operation: RegistryOperation,
    symbol: String,
    visibility: String,
    cfg: Vec<String>,
    fingerprint: String,
}

impl RegistryAccessRecord {
    fn identity(&self) -> AccessIdentity {
        AccessIdentity {
            package: self.package.clone(),
            module: self.module.clone(),
            source: self.source.clone(),
            enclosing: self.enclosing.clone(),
            registry: self.registry,
            operation: self.operation,
            symbol: self.symbol.clone(),
            visibility: self.visibility.clone(),
            cfg: self.cfg.clone(),
            fingerprint: self.fingerprint.clone(),
        }
    }
}

#[derive(Default)]
struct AccessAccumulator {
    rows: BTreeMap<AccessIdentity, usize>,
}

struct RecordContext<'a> {
    package: &'a str,
    module: &'a str,
    source: &'a str,
}

struct NewAccess<'a> {
    enclosing: &'a str,
    registry: RegistryKind,
    operation: RegistryOperation,
    symbol: &'a str,
    visibility: &'a str,
    cfg: &'a [String],
    fingerprint: String,
}

impl AccessAccumulator {
    fn add(&mut self, context: &RecordContext<'_>, access: NewAccess<'_>) {
        let identity = AccessIdentity {
            package: context.package.to_owned(),
            module: context.module.to_owned(),
            source: context.source.to_owned(),
            enclosing: access.enclosing.to_owned(),
            registry: access.registry,
            operation: access.operation,
            symbol: access.symbol.to_owned(),
            visibility: access.visibility.to_owned(),
            cfg: access.cfg.to_vec(),
            fingerprint: access.fingerprint,
        };
        *self.rows.entry(identity).or_default() += 1;
    }

    fn finish(self) -> RegistryAccessBaseline {
        RegistryAccessBaseline {
            schema_version: REGISTRY_SCHEMA_VERSION,
            accesses: self
                .rows
                .into_iter()
                .map(|(identity, count)| RegistryAccessRecord {
                    package: identity.package,
                    module: identity.module,
                    source: identity.source,
                    enclosing: identity.enclosing,
                    registry: identity.registry,
                    operation: identity.operation,
                    symbol: identity.symbol,
                    visibility: identity.visibility,
                    cfg: identity.cfg,
                    fingerprint: identity.fingerprint,
                    count,
                })
                .collect(),
        }
    }
}

fn normalized_ident(ident: &proc_macro2::Ident) -> String {
    let value = ident.to_string();
    value.strip_prefix("r#").unwrap_or(&value).to_owned()
}

fn normalized_tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

fn normalized_visibility(visibility: &Visibility) -> String {
    normalized_tokens(visibility)
}

fn method_fingerprint(method: &ExprMethodCall) -> String {
    let arguments = method
        .args
        .iter()
        .map(normalized_tokens)
        .collect::<Vec<_>>()
        .join(" | ");
    format!("{}({arguments})", normalized_ident(&method.method))
}

fn path_segments(path: &syn::Path) -> Vec<String> {
    path.segments
        .iter()
        .map(|segment| normalized_ident(&segment.ident))
        .collect()
}

fn last_path_ident(path: &syn::Path) -> Option<String> {
    path.segments
        .last()
        .map(|segment| normalized_ident(&segment.ident))
}

fn canonical_use_tree(tree: &UseTree) -> String {
    match tree {
        UseTree::Path(path) => format!(
            "{}::{}",
            normalized_ident(&path.ident),
            canonical_use_tree(&path.tree)
        ),
        UseTree::Name(name) => normalized_ident(&name.ident),
        UseTree::Rename(rename) => format!(
            "{} as {}",
            normalized_ident(&rename.ident),
            normalized_ident(&rename.rename)
        ),
        UseTree::Glob(_) => "*".to_owned(),
        UseTree::Group(group) => {
            let mut items = group
                .items
                .iter()
                .map(canonical_use_tree)
                .collect::<Vec<_>>();
            items.sort();
            format!("{{{}}}", items.join(","))
        }
    }
}

type KindSet = BTreeSet<RegistryKind>;

#[derive(Clone, Debug)]
struct GlobalAliasDefinition {
    package: String,
    module: String,
    local_name: String,
    target_paths: Vec<Vec<String>>,
}

#[derive(Clone, Debug)]
struct GlobalGlobImport {
    package: String,
    module: String,
    target_path: Vec<String>,
}

#[derive(Default)]
struct GlobalAliasIndex {
    aliases: BTreeMap<(String, String), BTreeMap<String, KindSet>>,
    known_modules: BTreeSet<(String, String)>,
    package_by_crate_name: BTreeMap<String, String>,
}

impl GlobalAliasIndex {
    fn aliases_for(&self, package: &str, module: &str) -> Option<&BTreeMap<String, KindSet>> {
        self.aliases.get(&(package.to_owned(), module.to_owned()))
    }

    fn extend_module_symbols(&self, package: &str, module: &str, symbols: &mut ModuleSymbols) {
        let Some(aliases) = self.aliases_for(package, module) else {
            return;
        };
        for (name, kinds) in aliases {
            symbols
                .type_aliases
                .entry(name.clone())
                .or_default()
                .extend(kinds.iter().copied());
        }
    }

    fn insert_aliases(&mut self, package: &str, module: &str, name: &str, kinds: KindSet) -> bool {
        if kinds.is_empty() {
            return false;
        }
        let entry = self
            .aliases
            .entry((package.to_owned(), module.to_owned()))
            .or_default()
            .entry(name.to_owned())
            .or_default();
        let previous_len = entry.len();
        entry.extend(kinds);
        entry.len() != previous_len
    }

    fn resolve_symbol_path(&self, package: &str, current_module: &str, path: &[String]) -> KindSet {
        let mut kinds = path
            .iter()
            .filter_map(|segment| RegistryKind::from_source_name(segment))
            .collect::<KindSet>();
        let Some((target_package, target_module, name)) =
            self.resolve_symbol_location(package, current_module, path)
        else {
            return kinds;
        };
        if let Some(resolved) = self
            .aliases_for(&target_package, &target_module)
            .and_then(|aliases| aliases.get(&name))
        {
            kinds.extend(resolved.iter().copied());
        }
        kinds
    }

    fn resolve_symbol_location(
        &self,
        package: &str,
        current_module: &str,
        path: &[String],
    ) -> Option<(String, String, String)> {
        let name = path.last()?.clone();
        if path.len() == 1 {
            return Some((package.to_owned(), current_module.to_owned(), name));
        }
        let qualifiers = &path[..path.len() - 1];
        let (target_package, target_module) =
            self.resolve_module_path(package, current_module, qualifiers)?;
        Some((target_package, target_module, name))
    }

    fn resolve_module_path(
        &self,
        package: &str,
        current_module: &str,
        path: &[String],
    ) -> Option<(String, String)> {
        let first = path.first().map(String::as_str)?;
        match first {
            "crate" => Some((package.to_owned(), path.join("::"))),
            "self" => {
                let suffix = &path[1..];
                let module = if suffix.is_empty() {
                    current_module.to_owned()
                } else {
                    format!("{current_module}::{}", suffix.join("::"))
                };
                Some((package.to_owned(), module))
            }
            "super" => {
                let mut module: Vec<_> = current_module.split("::").collect();
                let mut index = 0;
                while path.get(index).is_some_and(|segment| segment == "super") {
                    if module.len() > 1 {
                        module.pop();
                    }
                    index += 1;
                }
                module.extend(path[index..].iter().map(String::as_str));
                Some((package.to_owned(), module.join("::")))
            }
            crate_name if self.package_by_crate_name.contains_key(crate_name) => {
                let target_package = self.package_by_crate_name.get(crate_name)?.clone();
                let suffix = &path[1..];
                let module = if suffix.is_empty() {
                    "crate".to_owned()
                } else {
                    format!("crate::{}", suffix.join("::"))
                };
                Some((target_package, module))
            }
            _ => {
                let root_candidate = (package.to_owned(), format!("crate::{}", path.join("::")));
                if self.known_modules.contains(&root_candidate) {
                    return Some(root_candidate);
                }
                let relative_candidate = (
                    package.to_owned(),
                    format!("{current_module}::{}", path.join("::")),
                );
                self.known_modules
                    .contains(&relative_candidate)
                    .then_some(relative_candidate)
            }
        }
    }
}

#[derive(Default)]
struct AliasTypePathCollector {
    paths: Vec<Vec<String>>,
}

impl<'ast> Visit<'ast> for AliasTypePathCollector {
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        self.paths.push(path_segments(&node.path));
        visit::visit_type_path(self, node);
    }
}

fn flatten_use_alias_definitions(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    package: &str,
    module: &str,
    definitions: &mut Vec<GlobalAliasDefinition>,
    globs: &mut Vec<GlobalGlobImport>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(normalized_ident(&path.ident));
            flatten_use_alias_definitions(&path.tree, prefix, package, module, definitions, globs);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let source_name = normalized_ident(&name.ident);
            if source_name == "self" {
                return;
            }
            let mut target_path = prefix.clone();
            target_path.push(source_name.clone());
            definitions.push(GlobalAliasDefinition {
                package: package.to_owned(),
                module: module.to_owned(),
                local_name: source_name,
                target_paths: vec![target_path],
            });
        }
        UseTree::Rename(rename) => {
            let mut target_path = prefix.clone();
            target_path.push(normalized_ident(&rename.ident));
            definitions.push(GlobalAliasDefinition {
                package: package.to_owned(),
                module: module.to_owned(),
                local_name: normalized_ident(&rename.rename),
                target_paths: vec![target_path],
            });
        }
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use_alias_definitions(item, prefix, package, module, definitions, globs);
            }
        }
        UseTree::Glob(_) => globs.push(GlobalGlobImport {
            package: package.to_owned(),
            module: module.to_owned(),
            target_path: prefix.clone(),
        }),
    }
}

fn collect_global_alias_definitions(
    package: &str,
    module: &str,
    items: &[Item],
    cfg: &[String],
    index: &mut GlobalAliasIndex,
    definitions: &mut Vec<GlobalAliasDefinition>,
    globs: &mut Vec<GlobalGlobImport>,
    errors: &mut Vec<String>,
) {
    index
        .known_modules
        .insert((package.to_owned(), module.to_owned()));
    for item in items {
        match item {
            Item::Use(item_use) if production(cfg, &item_use.attrs, errors, "use item") => {
                flatten_use_alias_definitions(
                    &item_use.tree,
                    &mut Vec::new(),
                    package,
                    module,
                    definitions,
                    globs,
                );
            }
            Item::Type(alias) if production(cfg, &alias.attrs, errors, "type alias") => {
                let mut collector = AliasTypePathCollector::default();
                collector.visit_type(&alias.ty);
                definitions.push(GlobalAliasDefinition {
                    package: package.to_owned(),
                    module: module.to_owned(),
                    local_name: normalized_ident(&alias.ident),
                    target_paths: collector.paths,
                });
            }
            Item::Mod(ItemMod {
                attrs,
                ident,
                content: Some((_, child_items)),
                ..
            }) if production(cfg, attrs, errors, "inline module") => {
                collect_global_alias_definitions(
                    package,
                    &format!("{module}::{}", normalized_ident(ident)),
                    child_items,
                    &item_cfg(cfg, attrs),
                    index,
                    definitions,
                    globs,
                    errors,
                );
            }
            _ => {}
        }
    }
}

fn build_global_alias_index(
    sources: &[ParsedRegistrySource<'_>],
    errors: &mut Vec<String>,
) -> GlobalAliasIndex {
    let mut index = GlobalAliasIndex::default();
    let mut definitions = Vec::new();
    let mut globs = Vec::new();
    for source in sources {
        let crate_name = source.mount.package.replace('-', "_");
        if let Some(previous) = index
            .package_by_crate_name
            .insert(crate_name.clone(), source.mount.package.to_owned())
            && previous != source.mount.package
        {
            errors.push(format!(
                "registry alias pre-scan has ambiguous crate name {crate_name}: {previous} and {}",
                source.mount.package
            ));
        }
        collect_global_alias_definitions(
            source.mount.package,
            source.mount.module,
            &source.syntax.items,
            &source.cfg,
            &mut index,
            &mut definitions,
            &mut globs,
            errors,
        );
    }

    for _ in 0..=definitions.len() {
        let mut changed = false;
        for definition in &definitions {
            let kinds = definition
                .target_paths
                .iter()
                .flat_map(|path| {
                    index.resolve_symbol_path(&definition.package, &definition.module, path)
                })
                .collect();
            changed |= index.insert_aliases(
                &definition.package,
                &definition.module,
                &definition.local_name,
                kinds,
            );
        }
        if !changed {
            break;
        }
    }

    // Private module splits commonly inherit their parent facade with
    // `use super::*`. Resolve those imports through the same exact alias index
    // instead of rejecting a layout-only change. Iterate to a fixed point so
    // a private descendant can inherit an alias through more than one facade.
    // External registry-capable globs remain rejected by `collect_use_bindings`.
    for _ in 0..=globs.len() {
        let mut changed = false;
        for glob in &globs {
            let Some((target_package, target_module)) =
                index.resolve_module_path(&glob.package, &glob.module, &glob.target_path)
            else {
                continue;
            };
            let Some(aliases) = index.aliases_for(&target_package, &target_module).cloned() else {
                continue;
            };
            for (name, kinds) in aliases {
                changed |= index.insert_aliases(&glob.package, &glob.module, &name, kinds);
            }
        }
        if !changed {
            break;
        }
    }
    index
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum FlowStage {
    Registry,
    Guard,
    Iterator,
    Entry,
    Derived,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Flow(BTreeSet<(RegistryKind, FlowStage)>);

impl Flow {
    fn registry(kind: RegistryKind) -> Self {
        Self(BTreeSet::from([(kind, FlowStage::Registry)]))
    }

    fn from_kinds(kinds: &KindSet) -> Self {
        Self(
            kinds
                .iter()
                .map(|kind| (*kind, FlowStage::Registry))
                .collect(),
        )
    }

    fn union(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    fn registry_kinds(&self) -> KindSet {
        self.0
            .iter()
            .filter_map(|(kind, stage)| (*stage == FlowStage::Registry).then_some(*kind))
            .collect()
    }

    fn all_kinds(&self) -> KindSet {
        self.0.iter().map(|(kind, _)| *kind).collect()
    }

    fn has_registry(&self) -> bool {
        self.0
            .iter()
            .any(|(_, stage)| *stage == FlowStage::Registry)
    }

    fn map_registry_stage(&self, stage: FlowStage) -> Self {
        Self(
            self.registry_kinds()
                .into_iter()
                .map(|kind| (kind, stage))
                .collect(),
        )
    }
}

#[derive(Clone, Debug, Default)]
struct VariableInfo {
    flow: Flow,
    struct_types: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct ModuleSymbols {
    type_aliases: BTreeMap<String, KindSet>,
    field_kinds: BTreeMap<String, KindSet>,
    struct_names: BTreeSet<String>,
    function_returns: BTreeMap<String, Flow>,
}

impl Default for ModuleSymbols {
    fn default() -> Self {
        let type_aliases = [
            RegistryKind::Player,
            RegistryKind::Group,
            RegistryKind::PendingInvites,
        ]
        .into_iter()
        .map(|kind| (kind.source_name().to_owned(), BTreeSet::from([kind])))
        .collect();
        Self {
            type_aliases,
            field_kinds: BTreeMap::new(),
            struct_names: BTreeSet::new(),
            function_returns: BTreeMap::new(),
        }
    }
}

struct TypeKindCollector<'a> {
    aliases: &'a BTreeMap<String, KindSet>,
    kinds: KindSet,
}

impl<'ast> Visit<'ast> for TypeKindCollector<'_> {
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        for segment in &node.path.segments {
            let name = normalized_ident(&segment.ident);
            if let Some(kinds) = self.aliases.get(&name) {
                self.kinds.extend(kinds);
            }
        }
        visit::visit_type_path(self, node);
    }
}

fn registry_kinds_in_type(ty: &Type, symbols: &ModuleSymbols) -> KindSet {
    let mut collector = TypeKindCollector {
        aliases: &symbols.type_aliases,
        kinds: BTreeSet::new(),
    };
    collector.visit_type(ty);
    collector.kinds
}

struct StructNameCollector<'a> {
    known: &'a BTreeSet<String>,
    names: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for StructNameCollector<'_> {
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        for segment in &node.path.segments {
            let name = normalized_ident(&segment.ident);
            if self.known.contains(&name) {
                self.names.insert(name);
            }
        }
        visit::visit_type_path(self, node);
    }
}

fn struct_names_in_type(ty: &Type, symbols: &ModuleSymbols) -> BTreeSet<String> {
    let mut collector = StructNameCollector {
        known: &symbols.struct_names,
        names: BTreeSet::new(),
    };
    collector.visit_type(ty);
    collector.names
}

#[derive(Debug)]
struct ImportBinding {
    local_name: String,
    registry: RegistryKind,
}

fn collect_use_bindings(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    symbols: &ModuleSymbols,
    bindings: &mut Vec<ImportBinding>,
    errors: &mut Vec<String>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(normalized_ident(&path.ident));
            collect_use_bindings(&path.tree, prefix, symbols, bindings, errors);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let source_name = normalized_ident(&name.ident);
            let local_name = if source_name == "self" {
                prefix
                    .last()
                    .cloned()
                    .unwrap_or_else(|| source_name.clone())
            } else {
                source_name.clone()
            };
            let kinds = RegistryKind::from_source_name(&source_name)
                .map(|kind| BTreeSet::from([kind]))
                .or_else(|| symbols.type_aliases.get(&source_name).cloned())
                .or_else(|| symbols.type_aliases.get(&local_name).cloned())
                .unwrap_or_default();
            for registry in kinds {
                bindings.push(ImportBinding {
                    local_name: local_name.clone(),
                    registry,
                });
            }
        }
        UseTree::Rename(rename) => {
            let source_name = normalized_ident(&rename.ident);
            let local_name = normalized_ident(&rename.rename);
            let kinds = RegistryKind::from_source_name(&source_name)
                .map(|kind| BTreeSet::from([kind]))
                .or_else(|| symbols.type_aliases.get(&source_name).cloned())
                .or_else(|| symbols.type_aliases.get(&local_name).cloned())
                .unwrap_or_default();
            for registry in kinds {
                bindings.push(ImportBinding {
                    local_name: local_name.clone(),
                    registry,
                });
            }
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_use_bindings(item, prefix, symbols, bindings, errors);
            }
        }
        UseTree::Glob(_) => {
            // `directory` is the relocated player-directory owner module from
            // issue #138 and `wow_social`/`group` the relocated Group owner from
            // issue #137; `player_registry` remains the `wow-network` mailbox.
            let hides_registry = prefix.iter().any(|segment| {
                matches!(
                    segment.as_str(),
                    "wow_network"
                        | "wow_world"
                        | "wow_social"
                        | "player_registry"
                        | "group_registry"
                        | "group"
                        | "directory"
                )
            });
            if hides_registry {
                errors.push(format!(
                    "glob import {}::* can hide a registry alias; import each registry explicitly",
                    prefix.join("::")
                ));
            }
        }
    }
}

fn item_cfg(parent: &[String], attributes: &[Attribute]) -> Vec<String> {
    extend_cfg_context(parent, attributes)
}

fn production(
    parent: &[String],
    attributes: &[Attribute],
    errors: &mut Vec<String>,
    owner: &str,
) -> bool {
    match cfg_context_allows_production(parent, attributes) {
        Ok(production) => production,
        Err(error) => {
            errors.push(format!("invalid cfg on {owner}: {error}"));
            false
        }
    }
}

fn pat_identifiers(pattern: &Pat, output: &mut Vec<String>) {
    match pattern {
        Pat::Ident(ident) => {
            output.push(normalized_ident(&ident.ident));
            if let Some((_, subpat)) = &ident.subpat {
                pat_identifiers(subpat, output);
            }
        }
        Pat::Reference(reference) => pat_identifiers(&reference.pat, output),
        Pat::Type(typed) => pat_identifiers(&typed.pat, output),
        Pat::Tuple(tuple) => {
            for element in &tuple.elems {
                pat_identifiers(element, output);
            }
        }
        Pat::TupleStruct(tuple) => {
            for element in &tuple.elems {
                pat_identifiers(element, output);
            }
        }
        Pat::Struct(structure) => {
            for field in &structure.fields {
                pat_identifiers(&field.pat, output);
            }
        }
        Pat::Slice(slice) => {
            for element in &slice.elems {
                pat_identifiers(element, output);
            }
        }
        Pat::Or(or) => {
            for case in &or.cases {
                pat_identifiers(case, output);
            }
        }
        Pat::Paren(paren) => pat_identifiers(&paren.pat, output),
        _ => {}
    }
}

fn tokens_contain_identifier(tokens: TokenStream, names: &BTreeSet<String>) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Ident(ident) => names.contains(&normalized_ident(&ident)),
        TokenTree::Group(group) => tokens_contain_identifier(group.stream(), names),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

fn macro_name(path: &syn::Path) -> String {
    last_path_ident(path).unwrap_or_else(|| normalized_tokens(path))
}

fn is_known_opaque_value_macro(name: &str) -> bool {
    KNOWN_OPAQUE_VALUE_MACROS.contains(&name)
}

fn add_type_records(
    accumulator: &mut AccessAccumulator,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    ty: &Type,
    enclosing: &str,
    symbol: &str,
    visibility: &str,
    cfg: &[String],
) {
    for registry in registry_kinds_in_type(ty, symbols) {
        accumulator.add(
            context,
            NewAccess {
                enclosing,
                registry,
                operation: RegistryOperation::TypeReference,
                symbol,
                visibility,
                cfg,
                fingerprint: normalized_tokens(ty),
            },
        );
    }
}

fn collect_module_symbols(
    items: &[Item],
    parent: Option<&ModuleSymbols>,
    global_aliases: &GlobalAliasIndex,
    package: &str,
    module: &str,
    cfg: &[String],
    errors: &mut Vec<String>,
) -> ModuleSymbols {
    let mut symbols = parent.cloned().unwrap_or_default();
    global_aliases.extend_module_symbols(package, module, &mut symbols);

    for item in items {
        let Item::Use(item_use) = item else {
            continue;
        };
        if !production(cfg, &item_use.attrs, errors, "use item") {
            continue;
        }
        let mut bindings = Vec::new();
        collect_use_bindings(
            &item_use.tree,
            &mut Vec::new(),
            &symbols,
            &mut bindings,
            errors,
        );
        for binding in bindings {
            symbols
                .type_aliases
                .entry(binding.local_name)
                .or_default()
                .insert(binding.registry);
        }
    }

    // Resolve explicit type aliases to a fixed point. This covers chains such
    // as `type Players = PlayerRegistry; type Shared = Arc<Players>`.
    let mut aliases = Vec::new();
    for item in items {
        if let Item::Type(alias) = item
            && production(cfg, &alias.attrs, errors, "type alias")
        {
            aliases.push(alias);
        }
    }
    for _ in 0..=aliases.len() {
        let mut changed = false;
        for alias in &aliases {
            let name = normalized_ident(&alias.ident);
            let mut kinds = registry_kinds_in_type(&alias.ty, &symbols);
            if let Some(canonical) = RegistryKind::from_source_name(&name) {
                kinds.insert(canonical);
            }
            if !kinds.is_empty() {
                let entry = symbols.type_aliases.entry(name).or_default();
                let before = entry.len();
                entry.extend(kinds);
                changed |= entry.len() != before;
            }
        }
        if !changed {
            break;
        }
    }

    for item in items {
        match item {
            Item::Struct(item_struct) if production(cfg, &item_struct.attrs, errors, "struct") => {
                symbols
                    .struct_names
                    .insert(normalized_ident(&item_struct.ident));
            }
            Item::Enum(item_enum) if production(cfg, &item_enum.attrs, errors, "enum") => {
                symbols
                    .struct_names
                    .insert(normalized_ident(&item_enum.ident));
            }
            _ => {}
        }
    }

    for item in items {
        match item {
            Item::Struct(item_struct) if production(cfg, &item_struct.attrs, errors, "struct") => {
                let struct_cfg = item_cfg(cfg, &item_struct.attrs);
                for field in &item_struct.fields {
                    if !production(&struct_cfg, &field.attrs, errors, "struct field") {
                        continue;
                    }
                    let kinds = registry_kinds_in_type(&field.ty, &symbols);
                    if kinds.is_empty() {
                        continue;
                    }
                    let name = field
                        .ident
                        .as_ref()
                        .map(normalized_ident)
                        .unwrap_or_else(|| "<tuple-field>".to_owned());
                    symbols.field_kinds.entry(name).or_default().extend(kinds);
                }
            }
            Item::Enum(item_enum) if production(cfg, &item_enum.attrs, errors, "enum") => {
                let enum_cfg = item_cfg(cfg, &item_enum.attrs);
                for variant in &item_enum.variants {
                    if !production(&enum_cfg, &variant.attrs, errors, "enum variant") {
                        continue;
                    }
                    let variant_cfg = item_cfg(&enum_cfg, &variant.attrs);
                    for field in &variant.fields {
                        if !production(&variant_cfg, &field.attrs, errors, "enum field") {
                            continue;
                        }
                        let kinds = registry_kinds_in_type(&field.ty, &symbols);
                        if kinds.is_empty() {
                            continue;
                        }
                        let name = field
                            .ident
                            .as_ref()
                            .map(normalized_ident)
                            .unwrap_or_else(|| "<tuple-field>".to_owned());
                        symbols.field_kinds.entry(name).or_default().extend(kinds);
                    }
                }
            }
            _ => {}
        }
    }

    for item in items {
        match item {
            Item::Fn(function) if production(cfg, &function.attrs, errors, "function") => {
                if let ReturnType::Type(_, ty) = &function.sig.output {
                    let kinds = registry_kinds_in_type(ty, &symbols);
                    if !kinds.is_empty() {
                        symbols.function_returns.insert(
                            normalized_ident(&function.sig.ident),
                            Flow::from_kinds(&kinds),
                        );
                    }
                }
            }
            Item::Impl(item_impl) if production(cfg, &item_impl.attrs, errors, "impl") => {
                let impl_cfg = item_cfg(cfg, &item_impl.attrs);
                for impl_item in &item_impl.items {
                    let ImplItem::Fn(method) = impl_item else {
                        continue;
                    };
                    if !production(&impl_cfg, &method.attrs, errors, "impl method") {
                        continue;
                    }
                    if let ReturnType::Type(_, ty) = &method.sig.output {
                        let kinds = registry_kinds_in_type(ty, &symbols);
                        if !kinds.is_empty() {
                            symbols.function_returns.insert(
                                normalized_ident(&method.sig.ident),
                                Flow::from_kinds(&kinds),
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }

    symbols
}

struct BodyAnalyzer<'a, 'b> {
    context: RecordContext<'a>,
    accumulator: &'b mut AccessAccumulator,
    errors: &'b mut Vec<String>,
    symbols: &'b ModuleSymbols,
    enclosing: String,
    visibility: String,
    cfg: Vec<String>,
    scopes: Vec<BTreeMap<String, VariableInfo>>,
}

impl<'a, 'b> BodyAnalyzer<'a, 'b> {
    fn new(
        context: RecordContext<'a>,
        accumulator: &'b mut AccessAccumulator,
        errors: &'b mut Vec<String>,
        symbols: &'b ModuleSymbols,
        enclosing: String,
        visibility: String,
        cfg: Vec<String>,
    ) -> Self {
        Self {
            context,
            accumulator,
            errors,
            symbols,
            enclosing,
            visibility,
            cfg,
            scopes: vec![BTreeMap::new()],
        }
    }

    fn add(
        &mut self,
        registry: RegistryKind,
        operation: RegistryOperation,
        symbol: &str,
        cfg: &[String],
        fingerprint: String,
    ) {
        self.accumulator.add(
            &self.context,
            NewAccess {
                enclosing: &self.enclosing,
                registry,
                operation,
                symbol,
                visibility: &self.visibility,
                cfg,
                fingerprint,
            },
        );
    }

    fn allows_production(&mut self, attributes: &[Attribute], owner: &str) -> bool {
        production(&self.cfg, attributes, self.errors, owner)
    }

    fn audit_macro(&mut self, mac: &syn::Macro, attributes: &[Attribute], owner: &str) {
        if !self.allows_production(attributes, owner) {
            return;
        }
        let names = self.known_registry_token_names();
        if !tokens_contain_identifier(mac.tokens.clone(), &names) {
            return;
        }
        let name = macro_name(&mac.path);
        if !is_known_opaque_value_macro(&name) {
            self.errors.push(format!(
                "{} passes a registry alias/value through unknown macro {name}!; expose ordinary Rust syntax before baselining it",
                self.enclosing
            ));
            return;
        }
        let cfg = item_cfg(&self.cfg, attributes);
        let mut kinds = KindSet::new();
        for token_name in names {
            if tokens_contain_identifier(mac.tokens.clone(), &BTreeSet::from([token_name.clone()]))
            {
                if let Some(info) = self.lookup(&token_name) {
                    kinds.extend(info.flow.all_kinds());
                }
                if let Some(alias_kinds) = self.symbols.type_aliases.get(&token_name) {
                    kinds.extend(alias_kinds);
                }
            }
        }
        for registry in kinds {
            self.add(
                registry,
                RegistryOperation::OpaqueMacroBoundary,
                &name,
                &cfg,
                normalized_tokens(mac),
            );
        }
    }

    fn lookup(&self, name: &str) -> Option<&VariableInfo> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    fn bind(&mut self, name: String, info: VariableInfo) {
        self.scopes
            .last_mut()
            .expect("body analyzer always has a scope")
            .insert(name, info);
    }

    fn assign(&mut self, name: &str, info: VariableInfo) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(existing) = scope.get_mut(name) {
                *existing = info;
                return;
            }
        }
        self.bind(name.to_owned(), info);
    }

    fn info_from_type(&self, ty: &Type) -> VariableInfo {
        VariableInfo {
            flow: Flow::from_kinds(&registry_kinds_in_type(ty, self.symbols)),
            struct_types: struct_names_in_type(ty, self.symbols),
        }
    }

    fn info_from_expr(&self, expression: &Expr) -> VariableInfo {
        VariableInfo {
            flow: self.flow_of_expr(expression),
            struct_types: BTreeSet::new(),
        }
    }

    fn field_flow(&self, field: &ExprField) -> Flow {
        let Member::Named(member) = &field.member else {
            return Flow::default();
        };
        let name = normalized_ident(member);
        if let Some(kind) = RegistryKind::from_member_or_accessor(&name) {
            return Flow::registry(kind);
        }
        if let Some(kinds) = self.symbols.field_kinds.get(&name) {
            return Flow::from_kinds(kinds);
        }
        Flow::default()
    }

    fn flow_of_expr(&self, expression: &Expr) -> Flow {
        match expression {
            Expr::Path(path) => {
                let Some(name) = last_path_ident(&path.path) else {
                    return Flow::default();
                };
                self.lookup(&name)
                    .map(|info| info.flow.clone())
                    .or_else(|| self.symbols.type_aliases.get(&name).map(Flow::from_kinds))
                    .unwrap_or_default()
            }
            Expr::Field(field) => self.field_flow(field),
            Expr::Reference(reference) => self.flow_of_expr(&reference.expr),
            Expr::Paren(paren) => self.flow_of_expr(&paren.expr),
            Expr::Group(group) => self.flow_of_expr(&group.expr),
            Expr::Try(try_expression) => self.flow_of_expr(&try_expression.expr),
            Expr::Await(await_expression) => self.flow_of_expr(&await_expression.base),
            Expr::Cast(cast) => self.flow_of_expr(&cast.expr),
            Expr::Unary(unary) => self.flow_of_expr(&unary.expr),
            Expr::Tuple(tuple) => {
                let mut flow = Flow::default();
                for element in &tuple.elems {
                    flow.union(self.flow_of_expr(element));
                }
                flow
            }
            Expr::Array(array) => {
                let mut flow = Flow::default();
                for element in &array.elems {
                    flow.union(self.flow_of_expr(element));
                }
                flow
            }
            Expr::If(if_expression) => {
                let mut flow = implicit_tail_flow(&if_expression.then_branch, self);
                if let Some((_, else_expression)) = &if_expression.else_branch {
                    flow.union(self.flow_of_expr(else_expression));
                }
                flow
            }
            Expr::Match(match_expression) => {
                let mut flow = Flow::default();
                for arm in &match_expression.arms {
                    flow.union(self.flow_of_expr(&arm.body));
                }
                flow
            }
            Expr::Block(block) => implicit_tail_flow(&block.block, self),
            Expr::Call(call) => self.flow_of_call(call),
            Expr::MethodCall(method) => self.flow_of_method(method),
            Expr::Index(index) => self
                .flow_of_expr(&index.expr)
                .map_registry_stage(FlowStage::Derived),
            _ => Flow::default(),
        }
    }

    fn flow_of_call(&self, call: &ExprCall) -> Flow {
        let Expr::Path(function_path) = call.func.as_ref() else {
            return Flow::default();
        };
        let segments = path_segments(&function_path.path);
        let last = segments.last().map(String::as_str).unwrap_or_default();
        if matches!(last, "clone" | "cloned") {
            return call
                .args
                .first()
                .map(|argument| self.flow_of_expr(argument))
                .unwrap_or_default();
        }
        if matches!(last, "Some" | "Ok" | "Box" | "new") {
            let mut flow = Flow::default();
            for argument in &call.args {
                flow.union(self.flow_of_expr(argument));
            }
            if !flow.0.is_empty() {
                return flow;
            }
        }
        for segment in &segments {
            if let Some(kinds) = self.symbols.type_aliases.get(segment) {
                if matches!(last, "new" | "default" | "with_capacity" | "from_iter") {
                    return Flow::from_kinds(kinds);
                }
            }
        }
        self.symbols
            .function_returns
            .get(last)
            .cloned()
            .unwrap_or_default()
    }

    fn flow_of_method(&self, method: &ExprMethodCall) -> Flow {
        let name = normalized_ident(&method.method);
        if let Some(kind) = RegistryKind::from_member_or_accessor(&name) {
            return Flow::registry(kind);
        }
        let receiver = self.flow_of_expr(&method.receiver);
        match name.as_str() {
            "get" | "get_mut" => receiver.map_registry_stage(FlowStage::Guard),
            "iter" => receiver.map_registry_stage(FlowStage::Iterator),
            "entry" => receiver.map_registry_stage(FlowStage::Entry),
            "insert" | "remove" => receiver.map_registry_stage(FlowStage::Derived),
            "retain" | "clear" => Flow::default(),
            "clone" | "cloned" => Flow::from_kinds(&receiver.registry_kinds()),
            method if PASSTHROUGH_METHODS.contains(&method) => {
                Flow::from_kinds(&receiver.registry_kinds())
            }
            _ => Flow::default(),
        }
    }

    fn bind_pattern(&mut self, pattern: &Pat, info: &VariableInfo) {
        match pattern {
            Pat::Ident(ident) => {
                self.bind(normalized_ident(&ident.ident), info.clone());
                if let Some((_, subpat)) = &ident.subpat {
                    self.bind_pattern(subpat, info);
                }
            }
            Pat::Reference(reference) => self.bind_pattern(&reference.pat, info),
            Pat::Type(typed) => {
                let mut typed_info = self.info_from_type(&typed.ty);
                typed_info.flow.union(info.flow.clone());
                typed_info.struct_types.extend(info.struct_types.clone());
                self.bind_pattern(&typed.pat, &typed_info);
            }
            Pat::Tuple(tuple) => {
                for element in &tuple.elems {
                    self.bind_pattern(element, info);
                }
            }
            Pat::TupleStruct(tuple) => {
                for element in &tuple.elems {
                    self.bind_pattern(element, info);
                }
            }
            Pat::Struct(structure) => {
                for field in &structure.fields {
                    self.bind_pattern(&field.pat, info);
                }
            }
            Pat::Slice(slice) => {
                for element in &slice.elems {
                    self.bind_pattern(element, info);
                }
            }
            Pat::Or(or) => {
                for case in &or.cases {
                    self.bind_pattern(case, info);
                }
            }
            Pat::Paren(paren) => self.bind_pattern(&paren.pat, info),
            _ => {}
        }
    }

    fn bind_pattern_from_expr(&mut self, pattern: &Pat, expression: &Expr) {
        match (pattern, expression) {
            (Pat::Tuple(pattern), Expr::Tuple(expression))
                if pattern.elems.len() == expression.elems.len() =>
            {
                for (pattern, expression) in pattern.elems.iter().zip(&expression.elems) {
                    self.bind_pattern_from_expr(pattern, expression);
                }
            }
            (Pat::TupleStruct(pattern), _) if pattern.elems.len() == 1 => {
                self.bind_pattern_from_expr(
                    pattern.elems.first().expect("one tuple-struct element"),
                    expression,
                );
            }
            (Pat::Reference(pattern), _) => self.bind_pattern_from_expr(&pattern.pat, expression),
            (Pat::Paren(pattern), _) => self.bind_pattern_from_expr(&pattern.pat, expression),
            _ => {
                let info = self.info_from_expr(expression);
                self.bind_pattern(pattern, &info);
            }
        }
    }

    fn record_aliases(&mut self, pattern: &Pat, flow: &Flow, operation: RegistryOperation) {
        if !flow.has_registry() {
            return;
        }
        let mut names = Vec::new();
        pat_identifiers(pattern, &mut names);
        let cfg = self.cfg.clone();
        for name in names {
            for registry in flow.registry_kinds() {
                self.add(
                    registry,
                    operation,
                    &name,
                    &cfg,
                    format!("{name}:{}", registry.source_name()),
                );
            }
        }
    }

    fn record_return_flow(&mut self, flow: &Flow, fingerprint: String, cfg: &[String]) {
        for (registry, stage) in &flow.0 {
            self.add(
                *registry,
                RegistryOperation::Return,
                match stage {
                    FlowStage::Registry => "registry",
                    FlowStage::Guard => "guard",
                    FlowStage::Iterator => "iterator",
                    FlowStage::Entry => "entry",
                    FlowStage::Derived => "derived",
                },
                cfg,
                fingerprint.clone(),
            );
        }
    }

    fn known_registry_token_names(&self) -> BTreeSet<String> {
        let mut names: BTreeSet<_> = self.symbols.type_aliases.keys().cloned().collect();
        for scope in &self.scopes {
            names.extend(
                scope
                    .iter()
                    .filter(|(_, info)| !info.flow.0.is_empty())
                    .map(|(name, _)| name.clone()),
            );
        }
        names
    }

    fn visit_closure_with_input(&mut self, closure: &ExprClosure, input: &Flow) {
        if !self.allows_production(&closure.attrs, "closure") {
            return;
        }
        let next_cfg = item_cfg(&self.cfg, &closure.attrs);
        let previous_cfg = std::mem::replace(&mut self.cfg, next_cfg);
        self.scopes.push(BTreeMap::new());
        let info = VariableInfo {
            flow: Flow::from_kinds(&input.registry_kinds()),
            struct_types: BTreeSet::new(),
        };
        for input_pattern in &closure.inputs {
            self.bind_pattern(input_pattern, &info);
        }
        self.visit_expr(&closure.body);
        self.scopes.pop();
        self.cfg = previous_cfg;
    }
}

fn implicit_tail_flow(block: &syn::Block, analyzer: &BodyAnalyzer<'_, '_>) -> Flow {
    match block.stmts.last() {
        Some(Stmt::Expr(expression, None)) => analyzer.flow_of_expr(expression),
        _ => Flow::default(),
    }
}

impl<'ast> Visit<'ast> for BodyAnalyzer<'_, '_> {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.scopes.push(BTreeMap::new());
        for statement in &block.stmts {
            self.visit_stmt(statement);
        }
        self.scopes.pop();
    }

    fn visit_local(&mut self, local: &'ast Local) {
        if !self.allows_production(&local.attrs, "local binding") {
            return;
        }
        let previous_cfg = self.cfg.clone();
        self.cfg = item_cfg(&self.cfg, &local.attrs);
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            if let Some((_, diverge)) = &init.diverge {
                self.visit_expr(diverge);
            }
            let flow = self.flow_of_expr(&init.expr);
            self.record_aliases(&local.pat, &flow, RegistryOperation::LocalAlias);
            self.bind_pattern_from_expr(&local.pat, &init.expr);
        } else {
            let info = match &local.pat {
                Pat::Type(typed) => self.info_from_type(&typed.ty),
                _ => VariableInfo::default(),
            };
            self.bind_pattern(&local.pat, &info);
        }
        if let Pat::Type(typed) = &local.pat {
            let visibility = self.visibility.clone();
            let cfg = self.cfg.clone();
            add_type_records(
                self.accumulator,
                &self.context,
                self.symbols,
                &typed.ty,
                &self.enclosing,
                &normalized_tokens(&typed.pat),
                &visibility,
                &cfg,
            );
        }
        self.cfg = previous_cfg;
    }

    fn visit_expr_assign(&mut self, assignment: &'ast syn::ExprAssign) {
        if !self.allows_production(&assignment.attrs, "assignment") {
            return;
        }
        self.visit_expr(&assignment.right);
        self.visit_expr(&assignment.left);
        let Expr::Path(path) = assignment.left.as_ref() else {
            return;
        };
        let Some(name) = last_path_ident(&path.path) else {
            return;
        };
        let info = self.info_from_expr(&assignment.right);
        if info.flow.has_registry() {
            let cfg = item_cfg(&self.cfg, &assignment.attrs);
            for registry in info.flow.registry_kinds() {
                self.add(
                    registry,
                    RegistryOperation::AssignmentAlias,
                    &name,
                    &cfg,
                    normalized_tokens(assignment),
                );
            }
        }
        self.assign(&name, info);
    }

    fn visit_expr_field(&mut self, field: &'ast ExprField) {
        if !self.allows_production(&field.attrs, "field expression") {
            return;
        }
        let flow = self.field_flow(field);
        let symbol = match &field.member {
            Member::Named(member) => normalized_ident(member),
            Member::Unnamed(index) => index.index.to_string(),
        };
        let cfg = item_cfg(&self.cfg, &field.attrs);
        for registry in flow.registry_kinds() {
            self.add(
                registry,
                RegistryOperation::Member,
                &symbol,
                &cfg,
                normalized_tokens(field),
            );
        }
        visit::visit_expr_field(self, field);
    }

    fn visit_expr_method_call(&mut self, method: &'ast ExprMethodCall) {
        if !self.allows_production(&method.attrs, "method call") {
            return;
        }
        let name = normalized_ident(&method.method);
        let receiver = self.flow_of_expr(&method.receiver);
        let cfg = item_cfg(&self.cfg, &method.attrs);
        if let Some(kind) = RegistryKind::from_member_or_accessor(&name) {
            self.add(
                kind,
                RegistryOperation::Accessor,
                &name,
                &cfg,
                method_fingerprint(method),
            );
        }
        if let Some(operation) = RegistryOperation::from_method(&name) {
            for registry in receiver.registry_kinds() {
                self.add(registry, operation, &name, &cfg, method_fingerprint(method));
            }
        } else if matches!(name.as_str(), "clone" | "cloned") {
            for registry in receiver.registry_kinds() {
                self.add(
                    registry,
                    RegistryOperation::Clone,
                    &name,
                    &cfg,
                    method_fingerprint(method),
                );
            }
        }

        self.visit_expr(&method.receiver);
        let combinator_input = Flow::from_kinds(&receiver.registry_kinds());
        for argument in &method.args {
            if let Expr::Closure(closure) = argument {
                if matches!(name.as_str(), "map" | "and_then" | "filter" | "inspect")
                    && combinator_input.has_registry()
                {
                    self.visit_closure_with_input(closure, &combinator_input);
                    continue;
                }
            }
            let argument_flow = self.flow_of_expr(argument);
            for registry in argument_flow.registry_kinds() {
                self.add(
                    registry,
                    RegistryOperation::ArgumentEscape,
                    &name,
                    &cfg,
                    normalized_tokens(argument),
                );
            }
            self.visit_expr(argument);
        }
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if !self.allows_production(&call.attrs, "function call") {
            return;
        }
        let function_name = match call.func.as_ref() {
            Expr::Path(path) => last_path_ident(&path.path).unwrap_or_else(|| "<call>".to_owned()),
            _ => "<call>".to_owned(),
        };
        let clone_call = matches!(function_name.as_str(), "clone" | "cloned");
        let constructor_kinds = match call.func.as_ref() {
            Expr::Path(path)
                if matches!(
                    function_name.as_str(),
                    "new" | "default" | "with_capacity" | "from_iter"
                ) =>
            {
                path.path
                    .segments
                    .iter()
                    .filter_map(|segment| {
                        self.symbols
                            .type_aliases
                            .get(&normalized_ident(&segment.ident))
                    })
                    .flatten()
                    .copied()
                    .collect()
            }
            _ => KindSet::new(),
        };
        let cfg = item_cfg(&self.cfg, &call.attrs);
        for registry in constructor_kinds {
            self.add(
                registry,
                RegistryOperation::Construct,
                &function_name,
                &cfg,
                normalized_tokens(call),
            );
        }
        for argument in &call.args {
            let flow = self.flow_of_expr(argument);
            for registry in flow.registry_kinds() {
                self.add(
                    registry,
                    if clone_call {
                        RegistryOperation::Clone
                    } else {
                        RegistryOperation::ArgumentEscape
                    },
                    &function_name,
                    &cfg,
                    normalized_tokens(argument),
                );
            }
            self.visit_expr(argument);
        }
        self.visit_expr(&call.func);
    }

    fn visit_expr_index(&mut self, index: &'ast syn::ExprIndex) {
        if !self.allows_production(&index.attrs, "index expression") {
            return;
        }
        let receiver = self.flow_of_expr(&index.expr);
        let cfg = item_cfg(&self.cfg, &index.attrs);
        for registry in receiver.registry_kinds() {
            self.add(
                registry,
                RegistryOperation::Index,
                "index",
                &cfg,
                normalized_tokens(index),
            );
        }
        visit::visit_expr_index(self, index);
    }

    fn visit_expr_return(&mut self, returned: &'ast ExprReturn) {
        if !self.allows_production(&returned.attrs, "return expression") {
            return;
        }
        if let Some(expression) = &returned.expr {
            self.visit_expr(expression);
            let flow = self.flow_of_expr(expression);
            let cfg = item_cfg(&self.cfg, &returned.attrs);
            self.record_return_flow(&flow, normalized_tokens(expression), &cfg);
        }
    }

    fn visit_expr_macro(&mut self, expression: &'ast ExprMacro) {
        self.audit_macro(&expression.mac, &expression.attrs, "macro expression");
    }

    fn visit_stmt_macro(&mut self, statement: &'ast syn::StmtMacro) {
        self.audit_macro(&statement.mac, &statement.attrs, "statement macro");
    }

    fn visit_expr_if(&mut self, if_expression: &'ast ExprIf) {
        if !self.allows_production(&if_expression.attrs, "if expression") {
            return;
        }
        self.visit_expr(&if_expression.cond);
        self.scopes.push(BTreeMap::new());
        if let Expr::Let(let_expression) = if_expression.cond.as_ref() {
            self.bind_pattern_from_expr(&let_expression.pat, &let_expression.expr);
        }
        self.visit_block(&if_expression.then_branch);
        self.scopes.pop();
        if let Some((_, else_expression)) = &if_expression.else_branch {
            self.visit_expr(else_expression);
        }
    }

    fn visit_expr_match(&mut self, match_expression: &'ast ExprMatch) {
        if !self.allows_production(&match_expression.attrs, "match expression") {
            return;
        }
        self.visit_expr(&match_expression.expr);
        for arm in &match_expression.arms {
            if !self.allows_production(&arm.attrs, "match arm") {
                continue;
            }
            self.scopes.push(BTreeMap::new());
            self.bind_pattern_from_expr(&arm.pat, &match_expression.expr);
            if let Some((_, guard)) = &arm.guard {
                self.visit_expr(guard);
            }
            self.visit_expr(&arm.body);
            self.scopes.pop();
        }
    }

    fn visit_expr_closure(&mut self, closure: &'ast ExprClosure) {
        if !self.allows_production(&closure.attrs, "closure") {
            return;
        }
        self.scopes.push(BTreeMap::new());
        for input in &closure.inputs {
            let info = match input {
                Pat::Type(typed) => self.info_from_type(&typed.ty),
                _ => VariableInfo::default(),
            };
            self.bind_pattern(input, &info);
        }
        self.visit_expr(&closure.body);
        self.scopes.pop();
    }
}

fn impl_self_name(item_impl: &ItemImpl) -> String {
    match item_impl.self_ty.as_ref() {
        Type::Path(path) => last_path_ident(&path.path).unwrap_or_else(|| "<impl>".to_owned()),
        ty => normalized_tokens(ty),
    }
}

fn register_function_parameters(
    analyzer: &mut BodyAnalyzer<'_, '_>,
    inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>,
    cfg: &[String],
) {
    for argument in inputs {
        let FnArg::Typed(typed) = argument else {
            continue;
        };
        if !analyzer.allows_production(&typed.attrs, "function parameter") {
            continue;
        }
        let info = analyzer.info_from_type(&typed.ty);
        analyzer.bind_pattern(&typed.pat, &info);
        add_type_records(
            analyzer.accumulator,
            &analyzer.context,
            analyzer.symbols,
            &typed.ty,
            &analyzer.enclosing,
            &normalized_tokens(&typed.pat),
            &analyzer.visibility,
            &item_cfg(cfg, &typed.attrs),
        );
    }
}

fn analyze_function(
    function: &ItemFn,
    context: RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: Vec<String>,
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let enclosing = format!("fn {}", normalized_ident(&function.sig.ident));
    let visibility = normalized_visibility(&function.vis);
    if let ReturnType::Type(_, ty) = &function.sig.output {
        add_type_records(
            accumulator,
            &context,
            symbols,
            ty,
            &enclosing,
            "return",
            &visibility,
            &cfg,
        );
    }
    let mut analyzer = BodyAnalyzer::new(
        context,
        accumulator,
        errors,
        symbols,
        enclosing,
        visibility,
        cfg.clone(),
    );
    register_function_parameters(&mut analyzer, &function.sig.inputs, &cfg);
    for statement in &function.block.stmts {
        analyzer.visit_stmt(statement);
    }
    let tail = implicit_tail_flow(&function.block, &analyzer);
    if !tail.0.is_empty() {
        let fingerprint = function
            .block
            .stmts
            .last()
            .map(normalized_tokens)
            .unwrap_or_default();
        analyzer.record_return_flow(&tail, fingerprint, &cfg);
    }
}

fn analyze_impl(
    item_impl: &ItemImpl,
    context: RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: Vec<String>,
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let self_name = impl_self_name(item_impl);
    for item in &item_impl.items {
        let ImplItem::Fn(method) = item else {
            continue;
        };
        if !production(&cfg, &method.attrs, errors, "impl method") {
            continue;
        }
        let method_cfg = item_cfg(&cfg, &method.attrs);
        let enclosing = format!("impl {self_name}::{}", normalized_ident(&method.sig.ident));
        let visibility = normalized_visibility(&method.vis);
        if let ReturnType::Type(_, ty) = &method.sig.output {
            add_type_records(
                accumulator,
                &context,
                symbols,
                ty,
                &enclosing,
                "return",
                &visibility,
                &method_cfg,
            );
        }
        let mut analyzer = BodyAnalyzer::new(
            RecordContext {
                package: context.package,
                module: context.module,
                source: context.source,
            },
            accumulator,
            errors,
            symbols,
            enclosing,
            visibility,
            method_cfg.clone(),
        );
        register_function_parameters(&mut analyzer, &method.sig.inputs, &method_cfg);
        for statement in &method.block.stmts {
            analyzer.visit_stmt(statement);
        }
        let tail = implicit_tail_flow(&method.block, &analyzer);
        if !tail.0.is_empty() {
            let fingerprint = method
                .block
                .stmts
                .last()
                .map(normalized_tokens)
                .unwrap_or_default();
            analyzer.record_return_flow(&tail, fingerprint, &method_cfg);
        }
    }
}

fn analyze_struct(
    item_struct: &ItemStruct,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let enclosing = format!("struct {}", normalized_ident(&item_struct.ident));
    for (index, field) in item_struct.fields.iter().enumerate() {
        if !production(cfg, &field.attrs, errors, "struct field") {
            continue;
        }
        let name = field
            .ident
            .as_ref()
            .map(normalized_ident)
            .unwrap_or_else(|| index.to_string());
        add_type_records(
            accumulator,
            context,
            symbols,
            &field.ty,
            &enclosing,
            &name,
            &normalized_visibility(&field.vis),
            &item_cfg(cfg, &field.attrs),
        );
    }
}

fn analyze_enum(
    item_enum: &ItemEnum,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let enum_name = normalized_ident(&item_enum.ident);
    for variant in &item_enum.variants {
        if !production(cfg, &variant.attrs, errors, "enum variant") {
            continue;
        }
        let variant_cfg = item_cfg(cfg, &variant.attrs);
        let variant_name = normalized_ident(&variant.ident);
        for (index, field) in variant.fields.iter().enumerate() {
            if !production(&variant_cfg, &field.attrs, errors, "enum field") {
                continue;
            }
            let field_name = field
                .ident
                .as_ref()
                .map(normalized_ident)
                .unwrap_or_else(|| index.to_string());
            add_type_records(
                accumulator,
                context,
                symbols,
                &field.ty,
                &format!("enum {enum_name}::{variant_name}"),
                &field_name,
                &normalized_visibility(&item_enum.vis),
                &item_cfg(&variant_cfg, &field.attrs),
            );
        }
    }
}

fn analyze_type_alias(
    alias: &ItemType,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
) {
    let name = normalized_ident(&alias.ident);
    let mut kinds = registry_kinds_in_type(&alias.ty, symbols);
    if let Some(resolved) = symbols.type_aliases.get(&name) {
        kinds.extend(resolved.iter().copied());
    }
    if let Some(canonical) = RegistryKind::from_source_name(&name) {
        kinds.insert(canonical);
    }
    for registry in kinds {
        accumulator.add(
            context,
            NewAccess {
                enclosing: "module",
                registry,
                operation: RegistryOperation::TypeAlias,
                symbol: &name,
                visibility: &normalized_visibility(&alias.vis),
                cfg,
                fingerprint: normalized_tokens(&alias.ty),
            },
        );
    }
}

fn analyze_import(
    item_use: &ItemUse,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let mut bindings = Vec::new();
    collect_use_bindings(
        &item_use.tree,
        &mut Vec::new(),
        symbols,
        &mut bindings,
        errors,
    );
    for binding in bindings {
        accumulator.add(
            context,
            NewAccess {
                enclosing: "module",
                registry: binding.registry,
                operation: RegistryOperation::ImportAlias,
                symbol: &binding.local_name,
                visibility: &normalized_visibility(&item_use.vis),
                cfg,
                fingerprint: canonical_use_tree(&item_use.tree),
            },
        );
    }
}

fn macro_definition_or_invocation_error(
    item: &syn::ItemMacro,
    symbols: &ModuleSymbols,
    module: &str,
) -> Option<String> {
    let names: BTreeSet<_> = symbols.type_aliases.keys().cloned().collect();
    if !tokens_contain_identifier(item.mac.tokens.clone(), &names) {
        return None;
    }
    Some(format!(
        "module {module} hides registry provenance inside item macro {}!; expose ordinary Rust syntax before baselining it",
        macro_name(&item.mac.path)
    ))
}

fn analyze_module_items(
    items: &[Item],
    context: RecordContext<'_>,
    parent_symbols: Option<&ModuleSymbols>,
    global_aliases: &GlobalAliasIndex,
    cfg: Vec<String>,
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let symbols = collect_module_symbols(
        items,
        parent_symbols,
        global_aliases,
        context.package,
        context.module,
        &cfg,
        errors,
    );
    for item in items {
        match item {
            Item::Use(item_use) => {
                if !production(&cfg, &item_use.attrs, errors, "use item") {
                    continue;
                }
                analyze_import(
                    item_use,
                    &context,
                    &symbols,
                    &item_cfg(&cfg, &item_use.attrs),
                    accumulator,
                    errors,
                );
            }
            Item::Type(alias) => {
                if !production(&cfg, &alias.attrs, errors, "type alias") {
                    continue;
                }
                analyze_type_alias(
                    alias,
                    &context,
                    &symbols,
                    &item_cfg(&cfg, &alias.attrs),
                    accumulator,
                );
            }
            Item::Struct(item_struct) => {
                if !production(&cfg, &item_struct.attrs, errors, "struct") {
                    continue;
                }
                analyze_struct(
                    item_struct,
                    &context,
                    &symbols,
                    &item_cfg(&cfg, &item_struct.attrs),
                    accumulator,
                    errors,
                );
            }
            Item::Enum(item_enum) => {
                if !production(&cfg, &item_enum.attrs, errors, "enum") {
                    continue;
                }
                analyze_enum(
                    item_enum,
                    &context,
                    &symbols,
                    &item_cfg(&cfg, &item_enum.attrs),
                    accumulator,
                    errors,
                );
            }
            Item::Fn(function) => {
                if !production(&cfg, &function.attrs, errors, "function") {
                    continue;
                }
                analyze_function(
                    function,
                    RecordContext {
                        package: context.package,
                        module: context.module,
                        source: context.source,
                    },
                    &symbols,
                    item_cfg(&cfg, &function.attrs),
                    accumulator,
                    errors,
                );
            }
            Item::Impl(item_impl) => {
                if !production(&cfg, &item_impl.attrs, errors, "impl") {
                    continue;
                }
                analyze_impl(
                    item_impl,
                    RecordContext {
                        package: context.package,
                        module: context.module,
                        source: context.source,
                    },
                    &symbols,
                    item_cfg(&cfg, &item_impl.attrs),
                    accumulator,
                    errors,
                );
            }
            Item::Const(item_const) => {
                if !production(&cfg, &item_const.attrs, errors, "const") {
                    continue;
                }
                let enclosing = format!("const {}", normalized_ident(&item_const.ident));
                let item_cfg = item_cfg(&cfg, &item_const.attrs);
                add_type_records(
                    accumulator,
                    &context,
                    &symbols,
                    &item_const.ty,
                    &enclosing,
                    &normalized_ident(&item_const.ident),
                    "",
                    &item_cfg,
                );
                let mut analyzer = BodyAnalyzer::new(
                    RecordContext {
                        package: context.package,
                        module: context.module,
                        source: context.source,
                    },
                    accumulator,
                    errors,
                    &symbols,
                    enclosing,
                    String::new(),
                    item_cfg,
                );
                analyzer.visit_expr(&item_const.expr);
            }
            Item::Static(item_static) => {
                if !production(&cfg, &item_static.attrs, errors, "static") {
                    continue;
                }
                let enclosing = format!("static {}", normalized_ident(&item_static.ident));
                let item_cfg = item_cfg(&cfg, &item_static.attrs);
                add_type_records(
                    accumulator,
                    &context,
                    &symbols,
                    &item_static.ty,
                    &enclosing,
                    &normalized_ident(&item_static.ident),
                    &normalized_visibility(&item_static.vis),
                    &item_cfg,
                );
                let mut analyzer = BodyAnalyzer::new(
                    RecordContext {
                        package: context.package,
                        module: context.module,
                        source: context.source,
                    },
                    accumulator,
                    errors,
                    &symbols,
                    enclosing,
                    normalized_visibility(&item_static.vis),
                    item_cfg,
                );
                analyzer.visit_expr(&item_static.expr);
            }
            Item::Macro(item_macro) => {
                if !production(&cfg, &item_macro.attrs, errors, "item macro") {
                    continue;
                }
                if let Some(error) =
                    macro_definition_or_invocation_error(item_macro, &symbols, context.module)
                {
                    errors.push(error);
                }
            }
            Item::Mod(ItemMod {
                attrs,
                ident,
                content: Some((_, child_items)),
                ..
            }) => {
                if !production(&cfg, attrs, errors, "inline module") {
                    continue;
                }
                let child_module = format!("{}::{}", context.module, normalized_ident(ident));
                analyze_module_items(
                    child_items,
                    RecordContext {
                        package: context.package,
                        module: &child_module,
                        source: context.source,
                    },
                    Some(&symbols),
                    global_aliases,
                    item_cfg(&cfg, attrs),
                    accumulator,
                    errors,
                );
            }
            _ => {}
        }
    }
}

/// Parse and inventory an already-classified set of production source mounts.
/// Input order is irrelevant. Duplicate mounts intentionally increase record
/// multiplicity, so callers should pass each `(package, logical module, file)`
/// context once.
pub(crate) fn inventory_registry_accesses(
    sources: &[ProductionRegistrySource<'_>],
) -> Result<RegistryAccessBaseline, String> {
    let mut ordered: Vec<_> = sources.iter().copied().collect();
    ordered.sort_by(|left, right| {
        (left.package, left.module, left.source_path).cmp(&(
            right.package,
            right.module,
            right.source_path,
        ))
    });
    let mut seen_mounts = BTreeSet::new();
    let mut errors = Vec::new();
    let mut parsed_sources = Vec::new();
    for source in ordered {
        if source.package.is_empty() || source.module.is_empty() || source.source_path.is_empty() {
            errors.push("registry source package/module/path must be non-empty".to_owned());
            continue;
        }
        if !seen_mounts.insert((source.package, source.module, source.source_path)) {
            errors.push(format!(
                "duplicate production registry source mount {} {} {}",
                source.package, source.module, source.source_path
            ));
            continue;
        }
        let syntax = match syn::parse_file(source.source) {
            Ok(syntax) => syntax,
            Err(error) => {
                errors.push(format!(
                    "cannot parse registry source {}: {error}",
                    source.source_path
                ));
                continue;
            }
        };
        let cfg = extend_cfg_context(source.inherited_cfg, &syntax.attrs);
        match cfg_context_allows_production(source.inherited_cfg, &syntax.attrs) {
            Ok(true) => {}
            Ok(false) => {
                errors.push(format!(
                    "source {} was passed as production but its file attributes are test-only",
                    source.source_path
                ));
                continue;
            }
            Err(error) => {
                errors.push(format!(
                    "invalid file cfg in registry source {}: {error}",
                    source.source_path
                ));
                continue;
            }
        }
        parsed_sources.push(ParsedRegistrySource {
            mount: source,
            syntax,
            cfg,
        });
    }

    let global_aliases = build_global_alias_index(&parsed_sources, &mut errors);
    let mut accumulator = AccessAccumulator::default();
    for source in &parsed_sources {
        analyze_module_items(
            &source.syntax.items,
            RecordContext {
                package: source.mount.package,
                module: source.mount.module,
                source: source.mount.source_path,
            },
            None,
            &global_aliases,
            source.cfg.clone(),
            &mut accumulator,
            &mut errors,
        );
    }
    if errors.is_empty() {
        Ok(accumulator.finish())
    } else {
        errors.sort();
        errors.dedup();
        Err(errors.join("\n"))
    }
}

fn validated_baseline_map(
    label: &str,
    baseline: &RegistryAccessBaseline,
) -> Result<BTreeMap<AccessIdentity, usize>, String> {
    if baseline.schema_version != REGISTRY_SCHEMA_VERSION {
        return Err(format!(
            "{label} registry baseline schema version is {}, expected {REGISTRY_SCHEMA_VERSION}",
            baseline.schema_version
        ));
    }
    let mut map = BTreeMap::new();
    let mut previous: Option<AccessIdentity> = None;
    for record in &baseline.accesses {
        if record.count == 0 {
            return Err(format!(
                "{label} registry baseline contains zero-count row for {:?} {}",
                record.registry, record.symbol
            ));
        }
        let identity = record.identity();
        if previous
            .as_ref()
            .is_some_and(|previous| previous >= &identity)
        {
            return Err(format!(
                "{label} registry baseline rows are not in strict canonical order near {:?} {}",
                record.registry, record.symbol
            ));
        }
        previous = Some(identity.clone());
        if map.insert(identity, record.count).is_some() {
            return Err(format!(
                "{label} registry baseline contains a duplicate row for {:?} {}",
                record.registry, record.symbol
            ));
        }
    }
    Ok(map)
}

fn describe_identity(identity: &AccessIdentity) -> String {
    format!(
        "{} {} {}::{} {} {:?} {:?} {} [{}]",
        identity.package,
        identity.source,
        identity.module,
        identity.enclosing,
        identity.symbol,
        identity.registry,
        identity.operation,
        identity.fingerprint,
        identity.cfg.join(", ")
    )
}

/// Exact comparison: additions, removals, same-count swaps, and multiplicity
/// changes all fail. A debt reduction therefore requires deleting its stale
/// baseline row in the same reviewed change, preventing later reintroduction.
pub(crate) fn compare_registry_access_baseline(
    expected: &RegistryAccessBaseline,
    actual: &RegistryAccessBaseline,
) -> Result<(), String> {
    let expected = validated_baseline_map("expected", expected)?;
    let actual = validated_baseline_map("actual", actual)?;
    let mut errors = Vec::new();
    for (identity, actual_count) in &actual {
        match expected.get(identity) {
            None => errors.push(format!(
                "untracked direct registry access: {} (count {actual_count})",
                describe_identity(identity)
            )),
            Some(expected_count) if expected_count != actual_count => errors.push(format!(
                "direct registry access multiplicity changed: {} expected {expected_count}, actual {actual_count}",
                describe_identity(identity)
            )),
            Some(_) => {}
        }
    }
    for (identity, expected_count) in &expected {
        if !actual.contains_key(identity) {
            errors.push(format!(
                "obsolete direct registry baseline row: {} (expected count {expected_count})",
                describe_identity(identity)
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inventory(source: &str) -> Result<RegistryAccessBaseline, String> {
        inventory_registry_accesses(&[ProductionRegistrySource {
            package: "fixture",
            module: "crate::fixture",
            source_path: "src/fixture.rs",
            inherited_cfg: &[],
            source,
        }])
    }

    fn operations(
        baseline: &RegistryAccessBaseline,
    ) -> BTreeSet<(RegistryKind, RegistryOperation, String)> {
        baseline
            .accesses
            .iter()
            .map(|record| (record.registry, record.operation, record.symbol.clone()))
            .collect()
    }

    #[test]
    fn registry_inventory_tracks_nested_types_aliases_members_methods_clones_and_returns() {
        let baseline = inventory(
            r#"
                use wow_network::{
                    GroupRegistry,
                    PendingInvites,
                    PlayerRegistry as Players,
                };
                use std::sync::Arc;

                type SharedPlayers = Option<Arc<Players>>;

                struct Holder {
                    players: SharedPlayers,
                    pub(crate) groups: Arc<GroupRegistry>,
                }

                fn expose(
                    holder: &Holder,
                    invites: &PendingInvites,
                ) -> Arc<Players> {
                    holder.groups.get(&7);
                    invites.remove(&9);
                    let players = Arc::clone(holder.players.as_ref().expect("players"));
                    players.iter();
                    players
                }
            "#,
        )
        .expect("synthetic registry surface parses");

        let found = operations(&baseline);
        for expected in [
            (
                RegistryKind::Player,
                RegistryOperation::ImportAlias,
                "Players",
            ),
            (
                RegistryKind::Player,
                RegistryOperation::TypeAlias,
                "SharedPlayers",
            ),
            (RegistryKind::Group, RegistryOperation::Member, "groups"),
            (RegistryKind::Group, RegistryOperation::Get, "get"),
            (
                RegistryKind::PendingInvites,
                RegistryOperation::Remove,
                "remove",
            ),
            (RegistryKind::Player, RegistryOperation::Clone, "clone"),
            (
                RegistryKind::Player,
                RegistryOperation::LocalAlias,
                "players",
            ),
            (RegistryKind::Player, RegistryOperation::Iter, "iter"),
            (RegistryKind::Player, RegistryOperation::Return, "registry"),
        ] {
            assert!(
                found.contains(&(expected.0, expected.1, expected.2.to_owned())),
                "missing {expected:?} from {found:#?}"
            );
        }
        assert_eq!(
            serde_json::to_string(&baseline).expect("baseline serializes"),
            serde_json::to_string(
                &inventory(
                    r#"
                    use wow_network::{GroupRegistry, PendingInvites, PlayerRegistry as Players};
                    use std::sync::Arc;
                    type SharedPlayers = Option<Arc<Players>>;
                    struct Holder { players: SharedPlayers, pub(crate) groups: Arc<GroupRegistry> }
                    fn expose(holder: &Holder, invites: &PendingInvites) -> Arc<Players> {
                        holder.groups.get(&7);
                        invites.remove(&9);
                        let players = Arc::clone(holder.players.as_ref().expect("players"));
                        players.iter();
                        players
                    }
                "#,
                )
                .unwrap()
            )
            .expect("baseline serializes"),
            "formatting must not perturb the AST inventory"
        );
    }

    #[test]
    fn registry_inventory_resolves_cross_file_alias_reexports() {
        let aliases = ProductionRegistrySource {
            package: "fixture",
            module: "crate::aliases",
            source_path: "src/aliases.rs",
            inherited_cfg: &[],
            source: "pub use wow_world::session::directory::PlayerRegistry as Players;",
        };
        let consumer = ProductionRegistrySource {
            package: "fixture",
            module: "crate::consumer",
            source_path: "src/consumer.rs",
            inherited_cfg: &[],
            source: r#"
                use crate::aliases::Players;
                use crate::aliases::Players as Directory;

                fn lookup(players: &Players) {
                    players.get(&7);
                }

                fn lookup_renamed(directory: &Directory) {
                    directory.get(&8);
                }
            "#,
        };

        let baseline = inventory_registry_accesses(&[consumer, aliases])
            .expect("cross-file alias provenance resolves");
        let consumer_rows: BTreeSet<_> = baseline
            .accesses
            .iter()
            .filter(|record| record.module == "crate::consumer")
            .map(|record| (record.registry, record.operation, record.symbol.clone()))
            .collect();
        for expected in [
            (
                RegistryKind::Player,
                RegistryOperation::ImportAlias,
                "Players",
            ),
            (
                RegistryKind::Player,
                RegistryOperation::TypeReference,
                "players",
            ),
            (
                RegistryKind::Player,
                RegistryOperation::ImportAlias,
                "Directory",
            ),
            (
                RegistryKind::Player,
                RegistryOperation::TypeReference,
                "directory",
            ),
            (RegistryKind::Player, RegistryOperation::Get, "get"),
        ] {
            assert!(
                consumer_rows.contains(&(expected.0, expected.1, expected.2.to_owned())),
                "missing cross-file provenance {expected:?} from {consumer_rows:#?}"
            );
        }

        let reordered = inventory_registry_accesses(&[aliases, consumer])
            .expect("global alias resolution is input-order independent");
        assert_eq!(baseline, reordered);
    }

    #[test]
    fn registry_inventory_follows_accessors_combinators_and_tuple_bindings() {
        let baseline = inventory(
            r#"
                struct Session;
                impl Session {
                    fn inspect(&self) {
                        let (Some(players), Some(groups)) =
                            (self.player_registry(), self.group_registry()) else { return; };
                        players.get(&1);
                        groups.get_mut(&2);
                        self.pending_invites().and_then(|invites| invites.get(&3));
                    }
                }
            "#,
        )
        .expect("accessor provenance is inspectable");
        let found = operations(&baseline);
        assert!(found.contains(&(
            RegistryKind::Player,
            RegistryOperation::Accessor,
            "player_registry".to_owned()
        )));
        assert!(found.contains(&(
            RegistryKind::Group,
            RegistryOperation::GetMut,
            "get_mut".to_owned()
        )));
        assert!(found.contains(&(
            RegistryKind::PendingInvites,
            RegistryOperation::Get,
            "get".to_owned()
        )));
        assert!(
            !baseline.accesses.iter().any(|record| {
                record.registry == RegistryKind::Group
                    && record.operation == RegistryOperation::Get
                    && record.fingerprint.starts_with("get(")
            }),
            "tuple binding must not give the player get() group provenance"
        );
    }

    #[test]
    fn registry_inventory_distinguishes_production_from_test_only_cfg() {
        let baseline = inventory(
            r#"
                #[cfg(test)]
                fn test_only(registry: &PlayerRegistry) {
                    registry.clear();
                }

                #[cfg(any(test, feature = "fixture"))]
                fn production_capable(registry: &GroupRegistry) {
                    registry.retain(|_, _| true);
                }
            "#,
        )
        .expect("cfg-aware source parses");
        assert!(!baseline.accesses.iter().any(|record| {
            record.registry == RegistryKind::Player || record.operation == RegistryOperation::Clear
        }));
        assert!(baseline.accesses.iter().any(|record| {
            record.registry == RegistryKind::Group
                && record.operation == RegistryOperation::Retain
                && record
                    .cfg
                    .iter()
                    .any(|cfg| cfg.contains("feature = \"fixture\""))
        }));
    }

    #[test]
    fn registry_inventory_rejects_globs_and_unknown_macro_escape() {
        let glob = inventory("use wow_network::*;\n")
            .expect_err("a registry-capable glob must fail closed");
        assert!(glob.contains("can hide a registry alias"), "{glob}");

        let alias_module = ProductionRegistrySource {
            package: "fixture",
            module: "crate::aliases",
            source_path: "src/aliases.rs",
            inherited_cfg: &[],
            source: "pub use wow_world::session::directory::PlayerRegistry as Players;",
        };
        let glob_consumer = ProductionRegistrySource {
            package: "fixture",
            module: "crate::consumer",
            source_path: "src/consumer.rs",
            inherited_cfg: &[],
            source: "use crate::aliases::*; fn read(players: &Players) { players.get(&1); }",
        };
        let cross_module_glob = inventory_registry_accesses(&[alias_module, glob_consumer])
            .expect("a private cross-module glob must inherit indexed aliases exactly");
        assert!(
            cross_module_glob.accesses.iter().any(|record| {
                record.module == "crate::consumer"
                    && record.registry == RegistryKind::Player
                    && record.operation == RegistryOperation::Get
            }),
            "inherited Players alias must retain PlayerRegistry provenance"
        );

        let macro_escape = inventory(
            r#"
                fn hidden(players: &PlayerRegistry) {
                    hide_access!(players);
                }
            "#,
        )
        .expect_err("an unknown macro cannot consume registry provenance");
        assert!(
            macro_escape.contains("unknown macro hide_access!"),
            "{macro_escape}"
        );

        let macro_definition = inventory(
            r#"
                macro_rules! hidden_alias {
                    () => { type Hidden = PlayerRegistry; };
                }
            "#,
        )
        .expect_err("a macro-generated alias cannot bypass the inventory");
        assert!(
            macro_definition.contains("hides registry provenance inside item macro"),
            "{macro_definition}"
        );
    }

    /// Issue #138 moved the player directory to `wow_world::session::directory`.
    /// The glob guard must fail closed on the relocated owner exactly as it
    /// already does on the `wow-network` mailbox module, otherwise a single
    /// `use ...::directory::*;` would silently reintroduce hidden
    /// `PlayerRegistry` access after the move.
    #[test]
    fn registry_inventory_rejects_relocated_directory_glob() {
        for import in [
            "use wow_world::session::directory::*;\n",
            "use crate::session::directory::*;\n",
        ] {
            let error = inventory(import)
                .expect_err("a glob over the relocated directory owner must fail closed");
            assert!(
                error.contains("can hide a registry alias"),
                "{import} -> {error}"
            );
        }

        inventory("use crate::session::admission::*;\n")
            .expect("an unrelated session submodule glob stays allowed");
    }

    /// Issue #137 moved the Group owner to `wow_social::group`. The glob guard
    /// must fail closed on the relocated owner exactly as it already does on
    /// `wow_network`, otherwise one `use ...::group::*;` would silently
    /// reintroduce hidden `GroupRegistry`/`PendingInvites` access.
    #[test]
    fn registry_inventory_rejects_relocated_group_owner_glob() {
        for import in [
            "use wow_social::group::*;\n",
            "use wow_social::*;\n",
            "use crate::group::invites::*;\n",
        ] {
            let error = inventory(import)
                .expect_err("a glob over the relocated Group owner must fail closed");
            assert!(
                error.contains("can hide a registry alias"),
                "{import} -> {error}"
            );
        }

        inventory("use crate::handlers::party_ui::*;\n")
            .expect("an unrelated social-adjacent module glob stays allowed");
    }

    #[test]
    fn registry_inventory_records_argument_escape_and_index_access() {
        let baseline = inventory(
            r#"
                fn generic<T>(_value: T) {}
                fn escape(players: &PlayerRegistry) {
                    generic(players.clone());
                    let _ = &players[&7];
                }
            "#,
        )
        .expect("ordinary generic escape remains visible");
        let found = operations(&baseline);
        assert!(found.contains(&(
            RegistryKind::Player,
            RegistryOperation::Clone,
            "clone".to_owned()
        )));
        assert!(found.contains(&(
            RegistryKind::Player,
            RegistryOperation::ArgumentEscape,
            "generic".to_owned()
        )));
        assert!(found.contains(&(
            RegistryKind::Player,
            RegistryOperation::Index,
            "index".to_owned()
        )));
    }

    #[test]
    fn registry_baseline_is_input_order_independent_and_exact() {
        let source_a = ProductionRegistrySource {
            package: "a",
            module: "crate::a",
            source_path: "src/a.rs",
            inherited_cfg: &[],
            source: "fn a(registry: &PlayerRegistry) { registry.get(&1); }",
        };
        let source_b = ProductionRegistrySource {
            package: "b",
            module: "crate::b",
            source_path: "src/b.rs",
            inherited_cfg: &[],
            source: "fn b(registry: &GroupRegistry) { registry.insert(1, value); }",
        };
        let expected = inventory_registry_accesses(&[source_a, source_b]).unwrap();
        let reordered = inventory_registry_accesses(&[source_b, source_a]).unwrap();
        assert_eq!(expected, reordered);
        compare_registry_access_baseline(&expected, &reordered)
            .expect("identical exact baseline passes");

        let changed = inventory_registry_accesses(&[
            source_a,
            ProductionRegistrySource {
                source: "fn b(registry: &GroupRegistry) { registry.remove(&1); }",
                ..source_b
            },
        ])
        .unwrap();
        let error = compare_registry_access_baseline(&expected, &changed)
            .expect_err("same-count operation swap must fail");
        assert!(
            error.contains("untracked direct registry access"),
            "{error}"
        );
        assert!(
            error.contains("obsolete direct registry baseline row"),
            "{error}"
        );
    }

    #[test]
    fn registry_baseline_rejects_multiplicity_and_noncanonical_rows() {
        let expected =
            inventory("fn f(registry: &PlayerRegistry) { registry.get(&1); registry.get(&1); }")
                .unwrap();
        let actual = inventory("fn f(registry: &PlayerRegistry) { registry.get(&1); }").unwrap();
        let error = compare_registry_access_baseline(&expected, &actual)
            .expect_err("multiplicity reduction needs an explicit baseline cleanup");
        assert!(error.contains("multiplicity changed"), "{error}");

        let mut invalid = actual.clone();
        invalid.accesses.reverse();
        let error = compare_registry_access_baseline(&invalid, &actual)
            .expect_err("checked-in rows must remain deterministic");
        assert!(error.contains("strict canonical order"), "{error}");

        let mut zero = actual.clone();
        zero.accesses[0].count = 0;
        let error = compare_registry_access_baseline(&zero, &actual)
            .expect_err("zero-count rows are meaningless");
        assert!(error.contains("zero-count row"), "{error}");
    }
}
