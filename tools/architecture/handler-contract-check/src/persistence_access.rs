// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Exact AST inventory for concrete persistence access.
//!
//! This module is the source-level ratchet required before #186 can move SQL
//! behind typed ports. It inventories already-classified production source
//! mounts and deliberately fails closed when aliases, globs, or opaque macros
//! could hide SQLx or a concrete pool. It is a strict, explicit Rust grammar;
//! it is not a regex search and it does not pretend to perform type checking.
//!
//! The grammar follows explicit `use` renames, type aliases, typed fields and
//! bindings, local value flow, query constructors, transactions, executor
//! methods, and pool escapes through calls, stores, and returns. A new wrapper
//! or macro must therefore be taught here with an adversarial test before it
//! can enter a checked source surface.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprCall, ExprClosure, ExprField, ExprMacro, ExprMethodCall, ExprReturn,
    ExprStruct, FnArg, ImplItem, Item, ItemEnum, ItemFn, ItemImpl, ItemMod, ItemStruct, ItemType,
    ItemUse, Local, Member, Pat, ReturnType, Stmt, Type, UseTree, Visibility,
};

use crate::ownership::{cfg_context_allows_production, extend_cfg_context};

const PERSISTENCE_SCHEMA_VERSION: u32 = 1;

const QUERY_CONSTRUCTORS: &[&str] = &[
    "query",
    "query_as",
    "query_as_with",
    "query_file",
    "query_file_as",
    "query_scalar",
    "query_scalar_with",
    "query_with",
];

const FLOW_PASSTHROUGH_METHODS: &[&str] = &[
    "as_deref",
    "as_deref_mut",
    "as_mut",
    "as_ref",
    "clone",
    "expect",
    "inspect",
    "map",
    "ok_or",
    "ok_or_else",
    "or",
    "or_else",
    "unwrap",
    "unwrap_or",
    "unwrap_or_else",
];

const OPAQUE_PERSISTENCE_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "debug",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "error",
    "info",
    "matches",
    "select",
    "trace",
    "warn",
];

/// Concrete persistence surface represented by an inventory row.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum PersistenceTarget {
    #[serde(rename = "sqlx")]
    Sqlx,
    MySqlPool,
    PgPool,
    DatabaseConnection,
}

impl PersistenceTarget {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "sqlx" => Some(Self::Sqlx),
            "MySqlPool" => Some(Self::MySqlPool),
            "PgPool" => Some(Self::PgPool),
            "DatabaseConnection" => Some(Self::DatabaseConnection),
            _ => None,
        }
    }

    fn source_name(self) -> &'static str {
        match self {
            Self::Sqlx => "sqlx",
            Self::MySqlPool => "MySqlPool",
            Self::PgPool => "PgPool",
            Self::DatabaseConnection => "DatabaseConnection",
        }
    }

    fn is_pool(self) -> bool {
        !matches!(self, Self::Sqlx)
    }
}

/// Exact kind of direct persistence syntax or value escape.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PersistenceOperation {
    Import,
    TypeReference,
    TypeAlias,
    PathReference,
    MacroReference,
    ValueAlias,
    Query,
    Execute,
    Fetch,
    FetchAll,
    FetchMany,
    FetchOne,
    FetchOptional,
    Begin,
    Commit,
    Rollback,
    ArgumentEscape,
    ReturnEscape,
    StoreEscape,
}

impl PersistenceOperation {
    fn from_executor_method(name: &str) -> Option<Self> {
        match name {
            "execute" => Some(Self::Execute),
            "fetch" => Some(Self::Fetch),
            "fetch_all" => Some(Self::FetchAll),
            "fetch_many" => Some(Self::FetchMany),
            "fetch_one" => Some(Self::FetchOne),
            "fetch_optional" => Some(Self::FetchOptional),
            "begin" => Some(Self::Begin),
            "commit" => Some(Self::Commit),
            "rollback" => Some(Self::Rollback),
            _ => None,
        }
    }
}

/// One canonical, counted persistence access row.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PersistenceAccessRecord {
    pub(crate) classification: String,
    pub(crate) package: String,
    pub(crate) module: String,
    pub(crate) source: String,
    pub(crate) enclosing: String,
    pub(crate) target: PersistenceTarget,
    pub(crate) operation: PersistenceOperation,
    pub(crate) symbol: String,
    pub(crate) visibility: String,
    pub(crate) cfg: Vec<String>,
    pub(crate) fingerprint: String,
    pub(crate) count: usize,
}

/// Serializable exact snapshot. Rows are strictly ordered by full identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PersistenceAccessBaseline {
    pub(crate) schema_version: u32,
    pub(crate) accesses: Vec<PersistenceAccessRecord>,
}

/// One production source mount assigned to a runtime-ledger classification.
/// The repository walker owns file discovery and classification; this parser
/// rejects test-only mounts and inventories production-capable nested items.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClassifiedPersistenceSource<'a> {
    pub(crate) classification: &'a str,
    pub(crate) package: &'a str,
    pub(crate) module: &'a str,
    pub(crate) source_path: &'a str,
    pub(crate) inherited_cfg: &'a [String],
    pub(crate) source: &'a str,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct AccessIdentity {
    classification: String,
    package: String,
    module: String,
    source: String,
    enclosing: String,
    target: PersistenceTarget,
    operation: PersistenceOperation,
    symbol: String,
    visibility: String,
    cfg: Vec<String>,
    fingerprint: String,
}

impl PersistenceAccessRecord {
    fn identity(&self) -> AccessIdentity {
        AccessIdentity {
            classification: self.classification.clone(),
            package: self.package.clone(),
            module: self.module.clone(),
            source: self.source.clone(),
            enclosing: self.enclosing.clone(),
            target: self.target,
            operation: self.operation,
            symbol: self.symbol.clone(),
            visibility: self.visibility.clone(),
            cfg: self.cfg.clone(),
            fingerprint: self.fingerprint.clone(),
        }
    }
}

struct RecordContext<'a> {
    classification: &'a str,
    package: &'a str,
    module: &'a str,
    source: &'a str,
}

struct NewAccess<'a> {
    enclosing: &'a str,
    target: PersistenceTarget,
    operation: PersistenceOperation,
    symbol: &'a str,
    visibility: &'a str,
    cfg: &'a [String],
    fingerprint: String,
}

#[derive(Default)]
struct AccessAccumulator {
    rows: BTreeMap<AccessIdentity, usize>,
}

impl AccessAccumulator {
    fn add(&mut self, context: &RecordContext<'_>, access: NewAccess<'_>) {
        let identity = AccessIdentity {
            classification: context.classification.to_owned(),
            package: context.package.to_owned(),
            module: context.module.to_owned(),
            source: context.source.to_owned(),
            enclosing: access.enclosing.to_owned(),
            target: access.target,
            operation: access.operation,
            symbol: access.symbol.to_owned(),
            visibility: access.visibility.to_owned(),
            cfg: access.cfg.to_vec(),
            fingerprint: access.fingerprint,
        };
        *self.rows.entry(identity).or_insert(0) += 1;
    }

    fn finish(self) -> PersistenceAccessBaseline {
        PersistenceAccessBaseline {
            schema_version: PERSISTENCE_SCHEMA_VERSION,
            accesses: self
                .rows
                .into_iter()
                .map(|(identity, count)| PersistenceAccessRecord {
                    classification: identity.classification,
                    package: identity.package,
                    module: identity.module,
                    source: identity.source,
                    enclosing: identity.enclosing,
                    target: identity.target,
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

fn canonical_path(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| normalized_ident(&segment.ident))
        .collect::<Vec<_>>()
        .join("::")
}

fn path_names(path: &syn::Path) -> Vec<String> {
    path.segments
        .iter()
        .map(|segment| normalized_ident(&segment.ident))
        .collect()
}

fn last_path_name(path: &syn::Path) -> Option<String> {
    path.segments
        .last()
        .map(|segment| normalized_ident(&segment.ident))
}

fn canonical_call(call: &ExprCall) -> String {
    let arguments = call
        .args
        .iter()
        .map(normalized_tokens)
        .collect::<Vec<_>>()
        .join(" | ");
    format!("{}({arguments})", normalized_tokens(&call.func))
}

fn canonical_method(method: &ExprMethodCall) -> String {
    let arguments = method
        .args
        .iter()
        .map(normalized_tokens)
        .collect::<Vec<_>>()
        .join(" | ");
    format!(
        "{}.{}({arguments})",
        normalized_tokens(&method.receiver),
        normalized_ident(&method.method)
    )
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

type TargetSet = BTreeSet<PersistenceTarget>;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum FlowStage {
    Pool,
    Query,
    Transaction,
    DerivedPool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Flow(BTreeSet<(PersistenceTarget, FlowStage)>);

impl Flow {
    fn pools(targets: &TargetSet) -> Self {
        Self(
            targets
                .iter()
                .copied()
                .filter(|target| target.is_pool())
                .map(|target| (target, FlowStage::Pool))
                .collect(),
        )
    }

    fn query() -> Self {
        Self(BTreeSet::from([(
            PersistenceTarget::Sqlx,
            FlowStage::Query,
        )]))
    }

    fn union(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    fn targets(&self) -> TargetSet {
        self.0.iter().map(|(target, _)| *target).collect()
    }

    fn pool_targets(&self) -> TargetSet {
        self.0
            .iter()
            .filter_map(|(target, stage)| {
                matches!(stage, FlowStage::Pool | FlowStage::DerivedPool).then_some(*target)
            })
            .collect()
    }

    fn has_stage(&self, stage: FlowStage) -> bool {
        self.0.iter().any(|(_, current)| *current == stage)
    }

    fn map_pool_stage(&self, stage: FlowStage) -> Self {
        Self(
            self.0
                .iter()
                .filter_map(|(target, current)| {
                    matches!(current, FlowStage::Pool | FlowStage::DerivedPool)
                        .then_some((*target, stage))
                })
                .collect(),
        )
    }
}

#[derive(Clone, Debug, Default)]
struct VariableInfo {
    flow: Flow,
}

#[derive(Clone, Debug)]
struct ModuleSymbols {
    type_aliases: BTreeMap<String, TargetSet>,
    field_targets: BTreeMap<String, TargetSet>,
    function_returns: BTreeMap<String, Flow>,
    sqlx_namespaces: BTreeSet<String>,
    database_namespaces: BTreeSet<String>,
    query_callables: BTreeSet<String>,
}

impl Default for ModuleSymbols {
    fn default() -> Self {
        let mut type_aliases = BTreeMap::new();
        for target in [
            PersistenceTarget::MySqlPool,
            PersistenceTarget::PgPool,
            PersistenceTarget::DatabaseConnection,
        ] {
            type_aliases.insert(target.source_name().to_owned(), BTreeSet::from([target]));
        }
        Self {
            type_aliases,
            field_targets: BTreeMap::new(),
            function_returns: BTreeMap::new(),
            sqlx_namespaces: BTreeSet::from(["sqlx".to_owned()]),
            database_namespaces: BTreeSet::from(["wow_database".to_owned()]),
            query_callables: BTreeSet::new(),
        }
    }
}

fn is_query_name(name: &str) -> bool {
    QUERY_CONSTRUCTORS.contains(&name) || name.starts_with("query_")
}

fn is_flow_passthrough_call(names: &[String]) -> bool {
    let suffix = names.iter().rev().take(2).collect::<Vec<_>>();
    matches!(
        suffix.as_slice(),
        [method, owner]
            if matches!(
                (owner.as_str(), method.as_str()),
                ("Arc", "clone")
                    | ("Arc", "new")
                    | ("Rc", "clone")
                    | ("Rc", "new")
                    | ("Box", "new")
                    | ("Option", "Some")
                    | ("Result", "Ok")
            )
    ) || matches!(names, [name] if matches!(name.as_str(), "Some" | "Ok"))
}

fn targets_for_names(names: &[String], symbols: &ModuleSymbols) -> TargetSet {
    let mut targets = TargetSet::new();
    let Some(first) = names.first() else {
        return targets;
    };
    let last = names.last().expect("non-empty path");
    if symbols.sqlx_namespaces.contains(first) {
        targets.insert(PersistenceTarget::from_name(last).unwrap_or(PersistenceTarget::Sqlx));
    }
    if symbols.database_namespaces.contains(first) && last == "DatabaseConnection" {
        targets.insert(PersistenceTarget::DatabaseConnection);
    }
    for name in names {
        if let Some(alias_targets) = symbols.type_aliases.get(name) {
            targets.extend(alias_targets);
        } else if let Some(target) = PersistenceTarget::from_name(name) {
            if target.is_pool() {
                targets.insert(target);
            }
        }
    }
    targets
}

fn targets_for_path(path: &syn::Path, symbols: &ModuleSymbols) -> TargetSet {
    targets_for_names(&path_names(path), symbols)
}

struct TypeTargetCollector<'a> {
    symbols: &'a ModuleSymbols,
    targets: TargetSet,
}

impl<'ast> Visit<'ast> for TypeTargetCollector<'_> {
    fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
        self.targets
            .extend(targets_for_path(&ty.path, self.symbols));
        visit::visit_type_path(self, ty);
    }
}

fn targets_in_type(ty: &Type, symbols: &ModuleSymbols) -> TargetSet {
    let mut collector = TypeTargetCollector {
        symbols,
        targets: TargetSet::new(),
    };
    collector.visit_type(ty);
    collector.targets
}

fn targets_in_generics(generics: &syn::Generics, symbols: &ModuleSymbols) -> TargetSet {
    let mut collector = TypeTargetCollector {
        symbols,
        targets: TargetSet::new(),
    };
    collector.visit_generics(generics);
    collector.targets
}

#[derive(Clone, Debug)]
struct UseLeaf {
    source: Vec<String>,
    local: String,
    fingerprint: String,
}

fn flatten_use_tree(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    leaves: &mut Vec<UseLeaf>,
    globs: &mut Vec<Vec<String>>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(normalized_ident(&path.ident));
            flatten_use_tree(&path.tree, prefix, leaves, globs);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let name = normalized_ident(&name.ident);
            let mut source = prefix.clone();
            let local = if name == "self" {
                prefix.last().cloned().unwrap_or_else(|| name.clone())
            } else {
                source.push(name.clone());
                name
            };
            let fingerprint = source.join("::");
            leaves.push(UseLeaf {
                source,
                local,
                fingerprint,
            });
        }
        UseTree::Rename(rename) => {
            let source_name = normalized_ident(&rename.ident);
            let local = normalized_ident(&rename.rename);
            let mut source = prefix.clone();
            source.push(source_name);
            leaves.push(UseLeaf {
                fingerprint: format!("{} as {local}", source.join("::")),
                source,
                local,
            });
        }
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use_tree(item, prefix, leaves, globs);
            }
        }
        UseTree::Glob(_) => globs.push(prefix.clone()),
    }
}

fn use_leaves(item_use: &ItemUse) -> (Vec<UseLeaf>, Vec<Vec<String>>) {
    let mut leaves = Vec::new();
    let mut globs = Vec::new();
    flatten_use_tree(&item_use.tree, &mut Vec::new(), &mut leaves, &mut globs);
    (leaves, globs)
}

fn source_is_sqlx(source: &[String], symbols: &ModuleSymbols) -> bool {
    source
        .first()
        .is_some_and(|first| symbols.sqlx_namespaces.contains(first))
}

fn source_is_database(source: &[String], symbols: &ModuleSymbols) -> bool {
    source
        .first()
        .is_some_and(|first| symbols.database_namespaces.contains(first))
}

fn targets_for_use_leaf(leaf: &UseLeaf, symbols: &ModuleSymbols) -> TargetSet {
    targets_for_names(&leaf.source, symbols)
}

fn apply_import_symbols(item_use: &ItemUse, symbols: &mut ModuleSymbols) -> bool {
    let (leaves, _) = use_leaves(item_use);
    let mut changed = false;
    for leaf in leaves {
        let source_is_sqlx = source_is_sqlx(&leaf.source, symbols);
        let source_is_database = source_is_database(&leaf.source, symbols);
        if leaf.source.len() == 1 && source_is_sqlx {
            changed |= symbols.sqlx_namespaces.insert(leaf.local.clone());
        }
        if leaf.source.len() == 1 && source_is_database {
            changed |= symbols.database_namespaces.insert(leaf.local.clone());
        }
        let pool_targets = targets_for_use_leaf(&leaf, symbols)
            .into_iter()
            .filter(|target| target.is_pool())
            .collect::<TargetSet>();
        if !pool_targets.is_empty() {
            let entry = symbols.type_aliases.entry(leaf.local.clone()).or_default();
            let before = entry.len();
            entry.extend(pool_targets);
            changed |= entry.len() != before;
        }
        if source_is_sqlx && leaf.source.last().is_some_and(|name| is_query_name(name)) {
            changed |= symbols.query_callables.insert(leaf.local);
        }
    }
    changed
}

fn collect_module_symbols(
    items: &[Item],
    parent: Option<&ModuleSymbols>,
    cfg: &[String],
    errors: &mut Vec<String>,
) -> ModuleSymbols {
    let mut symbols = parent.cloned().unwrap_or_default();
    for _ in 0..=items.len() {
        let mut changed = false;
        for item in items {
            match item {
                Item::Use(item_use)
                    if production(cfg, &item_use.attrs, errors, "use declaration") =>
                {
                    changed |= apply_import_symbols(item_use, &mut symbols);
                }
                Item::ExternCrate(extern_crate)
                    if production(cfg, &extern_crate.attrs, errors, "extern crate") =>
                {
                    let source = normalized_ident(&extern_crate.ident);
                    let local = extern_crate
                        .rename
                        .as_ref()
                        .map(|(_, rename)| normalized_ident(rename))
                        .unwrap_or_else(|| source.clone());
                    if symbols.sqlx_namespaces.contains(&source) {
                        changed |= symbols.sqlx_namespaces.insert(local);
                    }
                }
                Item::Type(alias) if production(cfg, &alias.attrs, errors, "type alias") => {
                    let targets = targets_in_type(&alias.ty, &symbols);
                    if !targets.is_empty() {
                        let entry = symbols
                            .type_aliases
                            .entry(normalized_ident(&alias.ident))
                            .or_default();
                        let before = entry.len();
                        entry.extend(targets);
                        changed |= entry.len() != before;
                    }
                }
                _ => {}
            }
        }
        if !changed {
            break;
        }
    }

    for item in items {
        match item {
            Item::Struct(item_struct) if production(cfg, &item_struct.attrs, errors, "struct") => {
                for field in &item_struct.fields {
                    if !production(cfg, &field.attrs, errors, "struct field") {
                        continue;
                    }
                    let targets = targets_in_type(&field.ty, &symbols);
                    if targets.is_empty() {
                        continue;
                    }
                    let name = field
                        .ident
                        .as_ref()
                        .map(normalized_ident)
                        .unwrap_or_else(|| "tuple_field".to_owned());
                    symbols
                        .field_targets
                        .entry(name)
                        .or_default()
                        .extend(targets);
                }
            }
            Item::Enum(item_enum) if production(cfg, &item_enum.attrs, errors, "enum") => {
                for variant in &item_enum.variants {
                    if !production(cfg, &variant.attrs, errors, "enum variant") {
                        continue;
                    }
                    for field in &variant.fields {
                        if !production(cfg, &field.attrs, errors, "enum field") {
                            continue;
                        }
                        let targets = targets_in_type(&field.ty, &symbols);
                        if targets.is_empty() {
                            continue;
                        }
                        let name = field
                            .ident
                            .as_ref()
                            .map(normalized_ident)
                            .unwrap_or_else(|| "tuple_field".to_owned());
                        symbols
                            .field_targets
                            .entry(name)
                            .or_default()
                            .extend(targets);
                    }
                }
            }
            Item::Fn(function) if production(cfg, &function.attrs, errors, "function") => {
                if let ReturnType::Type(_, ty) = &function.sig.output {
                    let targets = targets_in_type(ty, &symbols);
                    if !targets.is_empty() {
                        symbols
                            .function_returns
                            .insert(normalized_ident(&function.sig.ident), Flow::pools(&targets));
                    }
                }
            }
            Item::Impl(item_impl) if production(cfg, &item_impl.attrs, errors, "impl") => {
                for item in &item_impl.items {
                    let ImplItem::Fn(method) = item else {
                        continue;
                    };
                    if !production(cfg, &method.attrs, errors, "impl method") {
                        continue;
                    }
                    if let ReturnType::Type(_, ty) = &method.sig.output {
                        let targets = targets_in_type(ty, &symbols);
                        if !targets.is_empty() {
                            symbols
                                .function_returns
                                .insert(normalized_ident(&method.sig.ident), Flow::pools(&targets));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    symbols
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
    operation: PersistenceOperation,
) {
    for target in targets_in_type(ty, symbols) {
        accumulator.add(
            context,
            NewAccess {
                enclosing,
                target,
                operation,
                symbol,
                visibility,
                cfg,
                fingerprint: normalized_tokens(ty),
            },
        );
    }
}

fn add_generics_records(
    accumulator: &mut AccessAccumulator,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    generics: &syn::Generics,
    enclosing: &str,
    visibility: &str,
    cfg: &[String],
) {
    for target in targets_in_generics(generics, symbols) {
        accumulator.add(
            context,
            NewAccess {
                enclosing,
                target,
                operation: PersistenceOperation::TypeReference,
                symbol: "generics",
                visibility,
                cfg,
                fingerprint: normalized_tokens(generics),
            },
        );
    }
}

fn pattern_identifiers(pattern: &Pat, output: &mut Vec<String>) {
    match pattern {
        Pat::Ident(ident) => {
            output.push(normalized_ident(&ident.ident));
            if let Some((_, subpat)) = &ident.subpat {
                pattern_identifiers(subpat, output);
            }
        }
        Pat::Reference(reference) => pattern_identifiers(&reference.pat, output),
        Pat::Type(typed) => pattern_identifiers(&typed.pat, output),
        Pat::Tuple(tuple) => {
            for element in &tuple.elems {
                pattern_identifiers(element, output);
            }
        }
        Pat::TupleStruct(tuple) => {
            for element in &tuple.elems {
                pattern_identifiers(element, output);
            }
        }
        Pat::Struct(structure) => {
            for field in &structure.fields {
                pattern_identifiers(&field.pat, output);
            }
        }
        Pat::Slice(slice) => {
            for element in &slice.elems {
                pattern_identifiers(element, output);
            }
        }
        Pat::Paren(paren) => pattern_identifiers(&paren.pat, output),
        Pat::Or(or_pattern) => {
            for case in &or_pattern.cases {
                pattern_identifiers(case, output);
            }
        }
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

fn module_persistence_names(symbols: &ModuleSymbols) -> BTreeSet<String> {
    let mut names = BTreeSet::from([
        "sqlx".to_owned(),
        "MySqlPool".to_owned(),
        "PgPool".to_owned(),
        "DatabaseConnection".to_owned(),
    ]);
    names.extend(symbols.sqlx_namespaces.iter().cloned());
    names.extend(symbols.database_namespaces.iter().cloned());
    names.extend(symbols.type_aliases.keys().cloned());
    names.extend(symbols.query_callables.iter().cloned());
    names
}

fn syntax_mentions_persistence(value: &impl ToTokens, symbols: &ModuleSymbols) -> bool {
    tokens_contain_identifier(value.to_token_stream(), &module_persistence_names(symbols))
}

fn targets_in_tokens(tokens: TokenStream, symbols: &ModuleSymbols) -> TargetSet {
    let mut targets = TargetSet::new();
    if symbols
        .sqlx_namespaces
        .iter()
        .any(|name| tokens_contain_identifier(tokens.clone(), &BTreeSet::from([name.clone()])))
    {
        targets.insert(PersistenceTarget::Sqlx);
    }
    for (name, alias_targets) in &symbols.type_aliases {
        if tokens_contain_identifier(tokens.clone(), &BTreeSet::from([name.clone()])) {
            targets.extend(alias_targets);
        }
    }
    targets
}

fn targets_in_attributes(attribute: &Attribute, symbols: &ModuleSymbols) -> TargetSet {
    if attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr") {
        return TargetSet::new();
    }
    let mut targets = targets_for_path(attribute.path(), symbols);
    targets.extend(targets_in_tokens(attribute.meta.to_token_stream(), symbols));
    targets
}

struct AttributeRecordContext<'a> {
    enclosing: &'a str,
    visibility: &'a str,
    cfg: &'a [String],
}

fn add_attribute_records(
    accumulator: &mut AccessAccumulator,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    attributes: &[Attribute],
    record: AttributeRecordContext<'_>,
) {
    for attribute in attributes {
        let symbol = last_path_name(attribute.path()).unwrap_or_else(|| "attribute".to_owned());
        for target in targets_in_attributes(attribute, symbols) {
            accumulator.add(
                context,
                NewAccess {
                    enclosing: record.enclosing,
                    target,
                    operation: PersistenceOperation::MacroReference,
                    symbol: &symbol,
                    visibility: record.visibility,
                    cfg: record.cfg,
                    fingerprint: normalized_tokens(attribute),
                },
            );
        }
    }
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
        target: PersistenceTarget,
        operation: PersistenceOperation,
        symbol: &str,
        cfg: &[String],
        fingerprint: String,
    ) {
        self.accumulator.add(
            &self.context,
            NewAccess {
                enclosing: &self.enclosing,
                target,
                operation,
                symbol,
                visibility: &self.visibility,
                cfg,
                fingerprint,
            },
        );
    }

    fn allows_production(&mut self, attributes: &[Attribute], owner: &str) -> bool {
        let allowed = production(&self.cfg, attributes, self.errors, owner);
        if allowed {
            let cfg = item_cfg(&self.cfg, attributes);
            add_attribute_records(
                self.accumulator,
                &self.context,
                self.symbols,
                attributes,
                AttributeRecordContext {
                    enclosing: &self.enclosing,
                    visibility: &self.visibility,
                    cfg: &cfg,
                },
            );
        }
        allowed
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
        if let Some(scope) = self
            .scopes
            .iter_mut()
            .rev()
            .find(|scope| scope.contains_key(name))
        {
            scope.insert(name.to_owned(), info);
        } else {
            self.bind(name.to_owned(), info);
        }
    }

    fn info_from_type(&self, ty: &Type) -> VariableInfo {
        VariableInfo {
            flow: Flow::pools(&targets_in_type(ty, self.symbols)),
        }
    }

    fn info_from_expr(&self, expression: &Expr) -> VariableInfo {
        VariableInfo {
            flow: self.flow_of_expr(expression),
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
                let typed_info = self.info_from_type(&typed.ty);
                if typed_info.flow.0.is_empty() {
                    self.bind_pattern(&typed.pat, info);
                } else {
                    self.bind_pattern(&typed.pat, &typed_info);
                }
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
            Pat::Paren(paren) => self.bind_pattern(&paren.pat, info),
            Pat::Or(or_pattern) => {
                for case in &or_pattern.cases {
                    self.bind_pattern(case, info);
                }
            }
            _ => {}
        }
    }

    fn bind_pattern_from_expr(&mut self, pattern: &Pat, expression: &Expr) {
        match (pattern, expression) {
            (Pat::Tuple(pattern), Expr::Tuple(tuple))
                if pattern.elems.len() == tuple.elems.len() =>
            {
                for (pattern, expression) in pattern.elems.iter().zip(&tuple.elems) {
                    let info = self.info_from_expr(expression);
                    self.bind_pattern(pattern, &info);
                }
            }
            _ => {
                let info = self.info_from_expr(expression);
                self.bind_pattern(pattern, &info);
            }
        }
    }

    fn field_flow(&self, field: &ExprField) -> Flow {
        let name = match &field.member {
            Member::Named(ident) => normalized_ident(ident),
            Member::Unnamed(index) => index.index.to_string(),
        };
        if let Some(targets) = self.symbols.field_targets.get(&name) {
            return Flow::pools(targets);
        }
        self.flow_of_expr(&field.base)
            .map_pool_stage(FlowStage::DerivedPool)
    }

    fn flow_of_path(&self, path: &syn::ExprPath) -> Flow {
        if path.qself.is_none() && path.path.segments.len() == 1 {
            if let Some(name) = last_path_name(&path.path) {
                if let Some(info) = self.lookup(&name) {
                    return info.flow.clone();
                }
            }
        }
        Flow::pools(&targets_for_path(&path.path, self.symbols))
    }

    fn flow_of_call(&self, call: &ExprCall) -> Flow {
        let Expr::Path(path) = call.func.as_ref() else {
            return Flow::default();
        };
        let names = path_names(&path.path);
        let last = names.last().map(String::as_str).unwrap_or_default();
        let rooted_sqlx = names
            .first()
            .is_some_and(|first| self.symbols.sqlx_namespaces.contains(first));
        if (rooted_sqlx && is_query_name(last))
            || (names.len() == 1 && self.symbols.query_callables.contains(last))
        {
            return Flow::query();
        }
        if is_flow_passthrough_call(&names) {
            let mut flow = Flow::default();
            for argument in &call.args {
                flow.union(self.flow_of_expr(argument));
            }
            return flow;
        }
        self.symbols
            .function_returns
            .get(last)
            .cloned()
            .unwrap_or_default()
    }

    fn flow_of_method(&self, method: &ExprMethodCall) -> Flow {
        let receiver = self.flow_of_expr(&method.receiver);
        let name = normalized_ident(&method.method);
        match name.as_str() {
            "begin" => receiver.map_pool_stage(FlowStage::Transaction),
            "acquire" => receiver.map_pool_stage(FlowStage::DerivedPool),
            name if FLOW_PASSTHROUGH_METHODS.contains(&name) => receiver,
            _ => self
                .symbols
                .function_returns
                .get(&name)
                .cloned()
                .unwrap_or_default(),
        }
    }

    fn flow_of_expr(&self, expression: &Expr) -> Flow {
        match expression {
            Expr::Path(path) => self.flow_of_path(path),
            Expr::Field(field) => self.field_flow(field),
            Expr::Reference(reference) => self.flow_of_expr(&reference.expr),
            Expr::Paren(paren) => self.flow_of_expr(&paren.expr),
            Expr::Group(group) => self.flow_of_expr(&group.expr),
            Expr::Try(try_expression) => self.flow_of_expr(&try_expression.expr),
            Expr::Await(await_expression) => self.flow_of_expr(&await_expression.base),
            Expr::Cast(cast) => self.flow_of_expr(&cast.expr),
            Expr::Unary(unary) => self.flow_of_expr(&unary.expr),
            Expr::Call(call) => self.flow_of_call(call),
            Expr::MethodCall(method) => self.flow_of_method(method),
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
            Expr::Tuple(tuple) => {
                let mut flow = Flow::default();
                for element in &tuple.elems {
                    flow.union(self.flow_of_expr(element));
                }
                flow
            }
            Expr::Macro(expression) => self.flow_of_macro(&expression.mac),
            _ => Flow::default(),
        }
    }

    fn flow_of_macro(&self, mac: &syn::Macro) -> Flow {
        let names = path_names(&mac.path);
        let last = names.last().map(String::as_str).unwrap_or_default();
        let rooted_sqlx = names
            .first()
            .is_some_and(|first| self.symbols.sqlx_namespaces.contains(first));
        if (rooted_sqlx && is_query_name(last))
            || (names.len() == 1 && self.symbols.query_callables.contains(last))
        {
            Flow::query()
        } else {
            Flow::default()
        }
    }

    fn record_flow(
        &mut self,
        flow: &Flow,
        operation: PersistenceOperation,
        symbol: &str,
        cfg: &[String],
        fingerprint: String,
    ) {
        for target in flow.targets() {
            self.add(target, operation, symbol, cfg, fingerprint.clone());
        }
    }

    fn record_pool_escape(
        &mut self,
        flow: &Flow,
        operation: PersistenceOperation,
        symbol: &str,
        cfg: &[String],
        fingerprint: String,
    ) {
        for target in flow.pool_targets() {
            self.add(target, operation, symbol, cfg, fingerprint.clone());
        }
    }

    fn known_persistence_names(&self) -> BTreeSet<String> {
        let mut names = module_persistence_names(self.symbols);
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

    fn audit_macro(&mut self, mac: &syn::Macro, attributes: &[Attribute], owner: &str) {
        if !self.allows_production(attributes, owner) {
            return;
        }
        let names = path_names(&mac.path);
        let name = names.last().cloned().unwrap_or_default();
        let rooted_sqlx = names
            .first()
            .is_some_and(|first| self.symbols.sqlx_namespaces.contains(first));
        let imported_query = names.len() == 1 && self.symbols.query_callables.contains(&name);
        let cfg = item_cfg(&self.cfg, attributes);
        if (rooted_sqlx && is_query_name(&name)) || imported_query {
            self.add(
                PersistenceTarget::Sqlx,
                PersistenceOperation::Query,
                &name,
                &cfg,
                normalized_tokens(mac),
            );
            return;
        }
        let direct_targets = targets_for_path(&mac.path, self.symbols);
        if rooted_sqlx || !direct_targets.is_empty() {
            let targets = if direct_targets.is_empty() {
                BTreeSet::from([PersistenceTarget::Sqlx])
            } else {
                direct_targets
            };
            for target in targets {
                self.add(
                    target,
                    PersistenceOperation::MacroReference,
                    &name,
                    &cfg,
                    normalized_tokens(mac),
                );
            }
            return;
        }
        let known = self.known_persistence_names();
        if !tokens_contain_identifier(mac.tokens.clone(), &known) {
            return;
        }
        if OPAQUE_PERSISTENCE_MACROS.contains(&name.as_str()) {
            let mut targets = targets_in_tokens(mac.tokens.clone(), self.symbols);
            let mut escaped = Flow::default();
            for scope in &self.scopes {
                for (local, info) in scope {
                    if tokens_contain_identifier(
                        mac.tokens.clone(),
                        &BTreeSet::from([local.clone()]),
                    ) {
                        targets.extend(info.flow.targets());
                        escaped.union(info.flow.clone());
                    }
                }
            }
            for target in targets {
                self.add(
                    target,
                    PersistenceOperation::MacroReference,
                    &name,
                    &cfg,
                    normalized_tokens(mac),
                );
            }
            self.record_pool_escape(
                &escaped,
                PersistenceOperation::ArgumentEscape,
                &format!("macro:{name}"),
                &cfg,
                normalized_tokens(mac),
            );
        } else {
            self.errors.push(format!(
                "{} passes concrete persistence syntax/value through unknown macro {name}!; expose an explicit SQLx query or ordinary Rust call before baselining it",
                self.enclosing
            ));
        }
    }

    fn register_parameters(
        &mut self,
        inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
    ) {
        for input in inputs {
            let FnArg::Typed(typed) = input else {
                continue;
            };
            if !self.allows_production(&typed.attrs, "function parameter") {
                continue;
            }
            let info = self.info_from_type(&typed.ty);
            self.bind_pattern(&typed.pat, &info);
            add_type_records(
                self.accumulator,
                &self.context,
                self.symbols,
                &typed.ty,
                &self.enclosing,
                &normalized_tokens(&typed.pat),
                &self.visibility,
                &item_cfg(&self.cfg, &typed.attrs),
                PersistenceOperation::TypeReference,
            );
        }
    }
}

fn implicit_tail_flow(block: &syn::Block, analyzer: &BodyAnalyzer<'_, '_>) -> Flow {
    match block.stmts.last() {
        Some(Stmt::Expr(expression, None)) => analyzer.flow_of_expr(expression),
        _ => Flow::default(),
    }
}

fn simple_assignment_name(expression: &Expr) -> Option<String> {
    let Expr::Path(path) = expression else {
        return None;
    };
    (path.qself.is_none() && path.path.segments.len() == 1)
        .then(|| last_path_name(&path.path))
        .flatten()
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
            let info = self.info_from_expr(&init.expr);
            let mut names = Vec::new();
            pattern_identifiers(&local.pat, &mut names);
            let cfg = self.cfg.clone();
            for name in names {
                self.record_flow(
                    &info.flow,
                    PersistenceOperation::ValueAlias,
                    &name,
                    &cfg,
                    format!("{}={}", name, normalized_tokens(&init.expr)),
                );
            }
            self.bind_pattern_from_expr(&local.pat, &init.expr);
        } else {
            let info = match &local.pat {
                Pat::Type(typed) => self.info_from_type(&typed.ty),
                _ => VariableInfo::default(),
            };
            self.bind_pattern(&local.pat, &info);
        }
        if let Pat::Type(typed) = &local.pat {
            let cfg = self.cfg.clone();
            add_type_records(
                self.accumulator,
                &self.context,
                self.symbols,
                &typed.ty,
                &self.enclosing,
                &normalized_tokens(&typed.pat),
                &self.visibility,
                &cfg,
                PersistenceOperation::TypeReference,
            );
        }
        self.cfg = previous_cfg;
    }

    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        if !self.allows_production(&path.attrs, "expression path") {
            return;
        }
        if path.qself.is_none() && path.path.segments.len() == 1 {
            if let Some(name) = last_path_name(&path.path) {
                if self.lookup(&name).is_some() {
                    return;
                }
            }
        }
        let cfg = item_cfg(&self.cfg, &path.attrs);
        for target in targets_for_path(&path.path, self.symbols) {
            self.add(
                target,
                PersistenceOperation::PathReference,
                &last_path_name(&path.path).unwrap_or_default(),
                &cfg,
                canonical_path(&path.path),
            );
        }
    }

    fn visit_expr_field(&mut self, field: &'ast ExprField) {
        if !self.allows_production(&field.attrs, "field expression") {
            return;
        }
        self.visit_expr(&field.base);
        let cfg = item_cfg(&self.cfg, &field.attrs);
        let name = match &field.member {
            Member::Named(ident) => normalized_ident(ident),
            Member::Unnamed(index) => index.index.to_string(),
        };
        let flow = self.field_flow(field);
        for target in flow.pool_targets() {
            self.add(
                target,
                PersistenceOperation::PathReference,
                &name,
                &cfg,
                normalized_tokens(field),
            );
        }
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if !self.allows_production(&call.attrs, "function call") {
            return;
        }
        let cfg = item_cfg(&self.cfg, &call.attrs);
        let (name, rooted_sqlx, imported_query, path_targets, flow_passthrough) =
            match call.func.as_ref() {
                Expr::Path(path) => {
                    let names = path_names(&path.path);
                    let name = names.last().cloned().unwrap_or_default();
                    let rooted_sqlx = names
                        .first()
                        .is_some_and(|first| self.symbols.sqlx_namespaces.contains(first));
                    let imported_query =
                        names.len() == 1 && self.symbols.query_callables.contains(&name);
                    (
                        name,
                        rooted_sqlx,
                        imported_query,
                        targets_for_path(&path.path, self.symbols),
                        is_flow_passthrough_call(&names),
                    )
                }
                _ => (String::new(), false, false, TargetSet::new(), false),
            };
        let query = (rooted_sqlx && is_query_name(&name)) || imported_query;
        if query {
            self.add(
                PersistenceTarget::Sqlx,
                PersistenceOperation::Query,
                &name,
                &cfg,
                canonical_call(call),
            );
        } else if let Some(operation) =
            PersistenceOperation::from_executor_method(&name).filter(|_| rooted_sqlx)
        {
            let mut targets = path_targets;
            for argument in &call.args {
                targets.extend(self.flow_of_expr(argument).targets());
            }
            if targets.is_empty() {
                targets.insert(PersistenceTarget::Sqlx);
            }
            for target in targets {
                self.add(target, operation, &name, &cfg, canonical_call(call));
            }
        }

        let known_persistence_call =
            query || (rooted_sqlx && PersistenceOperation::from_executor_method(&name).is_some());
        if !flow_passthrough && !known_persistence_call {
            for argument in &call.args {
                let flow = self.flow_of_expr(argument);
                self.record_pool_escape(
                    &flow,
                    PersistenceOperation::ArgumentEscape,
                    &name,
                    &cfg,
                    normalized_tokens(argument),
                );
            }
        }
        self.visit_expr(&call.func);
        for argument in &call.args {
            self.visit_expr(argument);
        }
    }

    fn visit_expr_method_call(&mut self, method: &'ast ExprMethodCall) {
        if !self.allows_production(&method.attrs, "method call") {
            return;
        }
        let cfg = item_cfg(&self.cfg, &method.attrs);
        let name = normalized_ident(&method.method);
        let receiver = self.flow_of_expr(&method.receiver);
        let operation = if is_query_name(&name) && !receiver.0.is_empty() {
            Some(PersistenceOperation::Query)
        } else {
            PersistenceOperation::from_executor_method(&name)
        };
        if let Some(operation) = operation {
            let valid = match operation {
                PersistenceOperation::Commit | PersistenceOperation::Rollback => {
                    receiver.has_stage(FlowStage::Transaction)
                }
                PersistenceOperation::Begin => !receiver.pool_targets().is_empty(),
                PersistenceOperation::Query => !receiver.0.is_empty(),
                _ => {
                    receiver.has_stage(FlowStage::Query)
                        || receiver.has_stage(FlowStage::Transaction)
                        || !receiver.pool_targets().is_empty()
                }
            };
            if valid {
                let mut targets = receiver.targets();
                for argument in &method.args {
                    targets.extend(self.flow_of_expr(argument).targets());
                }
                for target in targets {
                    self.add(target, operation, &name, &cfg, canonical_method(method));
                }
            }
        } else if !FLOW_PASSTHROUGH_METHODS.contains(&name.as_str()) {
            self.record_pool_escape(
                &receiver,
                PersistenceOperation::ArgumentEscape,
                &format!("receiver:{name}"),
                &cfg,
                normalized_tokens(&method.receiver),
            );
        }

        let known_persistence_method = operation.is_some();
        if !known_persistence_method && !FLOW_PASSTHROUGH_METHODS.contains(&name.as_str()) {
            for argument in &method.args {
                let flow = self.flow_of_expr(argument);
                self.record_pool_escape(
                    &flow,
                    PersistenceOperation::ArgumentEscape,
                    &name,
                    &cfg,
                    normalized_tokens(argument),
                );
            }
        }
        self.visit_expr(&method.receiver);
        for argument in &method.args {
            self.visit_expr(argument);
        }
    }

    fn visit_expr_assign(&mut self, assignment: &'ast syn::ExprAssign) {
        if !self.allows_production(&assignment.attrs, "assignment") {
            return;
        }
        self.visit_expr(&assignment.right);
        self.visit_expr(&assignment.left);
        let cfg = item_cfg(&self.cfg, &assignment.attrs);
        let info = self.info_from_expr(&assignment.right);
        if let Some(name) = simple_assignment_name(&assignment.left) {
            self.assign(&name, info.clone());
            self.record_flow(
                &info.flow,
                PersistenceOperation::ValueAlias,
                &name,
                &cfg,
                normalized_tokens(assignment),
            );
        } else {
            self.record_pool_escape(
                &info.flow,
                PersistenceOperation::StoreEscape,
                "assignment",
                &cfg,
                normalized_tokens(assignment),
            );
        }
    }

    fn visit_expr_struct(&mut self, structure: &'ast ExprStruct) {
        if !self.allows_production(&structure.attrs, "struct expression") {
            return;
        }
        let cfg = item_cfg(&self.cfg, &structure.attrs);
        for target in targets_for_path(&structure.path, self.symbols) {
            self.add(
                target,
                PersistenceOperation::PathReference,
                &last_path_name(&structure.path).unwrap_or_default(),
                &cfg,
                canonical_path(&structure.path),
            );
        }
        for field in &structure.fields {
            self.visit_expr(&field.expr);
            let flow = self.flow_of_expr(&field.expr);
            let symbol = match &field.member {
                Member::Named(ident) => normalized_ident(ident),
                Member::Unnamed(index) => index.index.to_string(),
            };
            self.record_pool_escape(
                &flow,
                PersistenceOperation::StoreEscape,
                &symbol,
                &cfg,
                normalized_tokens(&field.expr),
            );
        }
        if let Some(rest) = &structure.rest {
            self.visit_expr(rest);
            let flow = self.flow_of_expr(rest);
            self.record_pool_escape(
                &flow,
                PersistenceOperation::StoreEscape,
                "rest",
                &cfg,
                normalized_tokens(rest),
            );
        }
    }

    fn visit_expr_return(&mut self, returned: &'ast ExprReturn) {
        if !self.allows_production(&returned.attrs, "return expression") {
            return;
        }
        if let Some(expression) = &returned.expr {
            self.visit_expr(expression);
            let flow = self.flow_of_expr(expression);
            let cfg = item_cfg(&self.cfg, &returned.attrs);
            self.record_pool_escape(
                &flow,
                PersistenceOperation::ReturnEscape,
                "pool",
                &cfg,
                normalized_tokens(expression),
            );
        }
    }

    fn visit_expr_macro(&mut self, expression: &'ast ExprMacro) {
        self.audit_macro(&expression.mac, &expression.attrs, "macro expression");
    }

    fn visit_stmt_macro(&mut self, statement: &'ast syn::StmtMacro) {
        self.audit_macro(&statement.mac, &statement.attrs, "statement macro");
    }

    fn visit_expr_closure(&mut self, closure: &'ast ExprClosure) {
        if !self.allows_production(&closure.attrs, "closure") {
            return;
        }
        let previous_cfg = self.cfg.clone();
        self.cfg = item_cfg(&self.cfg, &closure.attrs);
        self.scopes.push(BTreeMap::new());
        for input in &closure.inputs {
            self.bind_pattern(input, &VariableInfo::default());
        }
        self.visit_expr(&closure.body);
        self.scopes.pop();
        self.cfg = previous_cfg;
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
    add_attribute_records(
        accumulator,
        context,
        symbols,
        &item_use.attrs,
        AttributeRecordContext {
            enclosing: "module",
            visibility: &normalized_visibility(&item_use.vis),
            cfg,
        },
    );
    let (leaves, globs) = use_leaves(item_use);
    for glob in globs {
        if source_is_sqlx(&glob, symbols) || source_is_database(&glob, symbols) {
            errors.push(format!(
                "glob import {}::* can hide concrete persistence access; import every SQLx/database symbol explicitly",
                glob.join("::")
            ));
        }
    }
    for leaf in leaves {
        for target in targets_for_use_leaf(&leaf, symbols) {
            accumulator.add(
                context,
                NewAccess {
                    enclosing: "module",
                    target,
                    operation: PersistenceOperation::Import,
                    symbol: &leaf.local,
                    visibility: &normalized_visibility(&item_use.vis),
                    cfg,
                    fingerprint: leaf.fingerprint.clone(),
                },
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
    let visibility = normalized_visibility(&alias.vis);
    add_attribute_records(
        accumulator,
        context,
        symbols,
        &alias.attrs,
        AttributeRecordContext {
            enclosing: "module",
            visibility: &visibility,
            cfg,
        },
    );
    add_generics_records(
        accumulator,
        context,
        symbols,
        &alias.generics,
        "module",
        &visibility,
        cfg,
    );
    add_type_records(
        accumulator,
        context,
        symbols,
        &alias.ty,
        "module",
        &normalized_ident(&alias.ident),
        &visibility,
        cfg,
        PersistenceOperation::TypeAlias,
    );
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
    let visibility = normalized_visibility(&item_struct.vis);
    add_attribute_records(
        accumulator,
        context,
        symbols,
        &item_struct.attrs,
        AttributeRecordContext {
            enclosing: &enclosing,
            visibility: &visibility,
            cfg,
        },
    );
    add_generics_records(
        accumulator,
        context,
        symbols,
        &item_struct.generics,
        &enclosing,
        &visibility,
        cfg,
    );
    for field in &item_struct.fields {
        if !production(cfg, &field.attrs, errors, "struct field") {
            continue;
        }
        let symbol = field
            .ident
            .as_ref()
            .map(normalized_ident)
            .unwrap_or_else(|| "tuple_field".to_owned());
        let field_cfg = item_cfg(cfg, &field.attrs);
        let field_visibility = normalized_visibility(&field.vis);
        add_attribute_records(
            accumulator,
            context,
            symbols,
            &field.attrs,
            AttributeRecordContext {
                enclosing: &enclosing,
                visibility: &field_visibility,
                cfg: &field_cfg,
            },
        );
        add_type_records(
            accumulator,
            context,
            symbols,
            &field.ty,
            &enclosing,
            &symbol,
            &field_visibility,
            &field_cfg,
            PersistenceOperation::TypeReference,
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
    let enum_enclosing = format!("enum {}", normalized_ident(&item_enum.ident));
    let enum_visibility = normalized_visibility(&item_enum.vis);
    add_attribute_records(
        accumulator,
        context,
        symbols,
        &item_enum.attrs,
        AttributeRecordContext {
            enclosing: &enum_enclosing,
            visibility: &enum_visibility,
            cfg,
        },
    );
    add_generics_records(
        accumulator,
        context,
        symbols,
        &item_enum.generics,
        &enum_enclosing,
        &enum_visibility,
        cfg,
    );
    for variant in &item_enum.variants {
        if !production(cfg, &variant.attrs, errors, "enum variant") {
            continue;
        }
        let enclosing = format!(
            "enum {}::{}",
            normalized_ident(&item_enum.ident),
            normalized_ident(&variant.ident)
        );
        let variant_cfg = item_cfg(cfg, &variant.attrs);
        add_attribute_records(
            accumulator,
            context,
            symbols,
            &variant.attrs,
            AttributeRecordContext {
                enclosing: &enclosing,
                visibility: &enum_visibility,
                cfg: &variant_cfg,
            },
        );
        for field in &variant.fields {
            if !production(&variant_cfg, &field.attrs, errors, "enum field") {
                continue;
            }
            let symbol = field
                .ident
                .as_ref()
                .map(normalized_ident)
                .unwrap_or_else(|| "tuple_field".to_owned());
            let field_cfg = item_cfg(&variant_cfg, &field.attrs);
            let field_visibility = normalized_visibility(&field.vis);
            add_attribute_records(
                accumulator,
                context,
                symbols,
                &field.attrs,
                AttributeRecordContext {
                    enclosing: &enclosing,
                    visibility: &field_visibility,
                    cfg: &field_cfg,
                },
            );
            add_type_records(
                accumulator,
                context,
                symbols,
                &field.ty,
                &enclosing,
                &symbol,
                &field_visibility,
                &field_cfg,
                PersistenceOperation::TypeReference,
            );
        }
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
    add_attribute_records(
        accumulator,
        &context,
        symbols,
        &function.attrs,
        AttributeRecordContext {
            enclosing: &enclosing,
            visibility: &visibility,
            cfg: &cfg,
        },
    );
    add_generics_records(
        accumulator,
        &context,
        symbols,
        &function.sig.generics,
        &enclosing,
        &visibility,
        &cfg,
    );
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
            PersistenceOperation::TypeReference,
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
    analyzer.register_parameters(&function.sig.inputs);
    for statement in &function.block.stmts {
        analyzer.visit_stmt(statement);
    }
    let tail = implicit_tail_flow(&function.block, &analyzer);
    if !tail.pool_targets().is_empty() {
        let fingerprint = function
            .block
            .stmts
            .last()
            .map(normalized_tokens)
            .unwrap_or_default();
        analyzer.record_pool_escape(
            &tail,
            PersistenceOperation::ReturnEscape,
            "pool",
            &cfg,
            fingerprint,
        );
    }
}

fn impl_self_name(item_impl: &ItemImpl) -> String {
    normalized_tokens(&item_impl.self_ty)
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
    let impl_enclosing = format!("impl {self_name}");
    add_attribute_records(
        accumulator,
        &context,
        symbols,
        &item_impl.attrs,
        AttributeRecordContext {
            enclosing: &impl_enclosing,
            visibility: "",
            cfg: &cfg,
        },
    );
    add_generics_records(
        accumulator,
        &context,
        symbols,
        &item_impl.generics,
        &impl_enclosing,
        "",
        &cfg,
    );
    add_type_records(
        accumulator,
        &context,
        symbols,
        &item_impl.self_ty,
        &impl_enclosing,
        "self",
        "",
        &cfg,
        PersistenceOperation::TypeReference,
    );
    if let Some((_, trait_path, _)) = &item_impl.trait_ {
        for target in targets_for_path(trait_path, symbols) {
            accumulator.add(
                &context,
                NewAccess {
                    enclosing: &impl_enclosing,
                    target,
                    operation: PersistenceOperation::TypeReference,
                    symbol: "trait",
                    visibility: "",
                    cfg: &cfg,
                    fingerprint: normalized_tokens(trait_path),
                },
            );
        }
    }
    for item in &item_impl.items {
        let ImplItem::Fn(method) = item else {
            if syntax_mentions_persistence(item, symbols) {
                errors.push(format!(
                    "{impl_enclosing} contains unsupported associated persistence syntax; use an ordinary method/type surface before baselining: {}",
                    normalized_tokens(item)
                ));
            }
            continue;
        };
        if !production(&cfg, &method.attrs, errors, "impl method") {
            continue;
        }
        let method_cfg = item_cfg(&cfg, &method.attrs);
        let enclosing = format!("impl {self_name}::{}", normalized_ident(&method.sig.ident));
        let visibility = normalized_visibility(&method.vis);
        add_attribute_records(
            accumulator,
            &context,
            symbols,
            &method.attrs,
            AttributeRecordContext {
                enclosing: &enclosing,
                visibility: &visibility,
                cfg: &method_cfg,
            },
        );
        add_generics_records(
            accumulator,
            &context,
            symbols,
            &method.sig.generics,
            &enclosing,
            &visibility,
            &method_cfg,
        );
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
                PersistenceOperation::TypeReference,
            );
        }
        let mut analyzer = BodyAnalyzer::new(
            RecordContext {
                classification: context.classification,
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
        analyzer.register_parameters(&method.sig.inputs);
        for statement in &method.block.stmts {
            analyzer.visit_stmt(statement);
        }
        let tail = implicit_tail_flow(&method.block, &analyzer);
        if !tail.pool_targets().is_empty() {
            let fingerprint = method
                .block
                .stmts
                .last()
                .map(normalized_tokens)
                .unwrap_or_default();
            analyzer.record_pool_escape(
                &tail,
                PersistenceOperation::ReturnEscape,
                "pool",
                &method_cfg,
                fingerprint,
            );
        }
    }
}

fn analyze_item_macro(
    item_macro: &syn::ItemMacro,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let path_name = last_path_name(&item_macro.mac.path).unwrap_or_default();
    let symbol = item_macro
        .ident
        .as_ref()
        .map(normalized_ident)
        .unwrap_or_else(|| path_name.clone());
    let mut targets = targets_for_path(&item_macro.mac.path, symbols);
    targets.extend(targets_in_tokens(item_macro.mac.tokens.clone(), symbols));
    if path_name == "include" {
        if !targets.is_empty() {
            errors.push(format!(
                "{} contains include! with concrete persistence tokens; mount and parse the included Rust source explicitly",
                context.module
            ));
        }
        return;
    }
    for target in targets {
        accumulator.add(
            context,
            NewAccess {
                enclosing: "module",
                target,
                operation: PersistenceOperation::MacroReference,
                symbol: &symbol,
                visibility: "",
                cfg,
                fingerprint: normalized_tokens(&item_macro.mac),
            },
        );
    }
}

fn analyze_module_items(
    items: &[Item],
    context: RecordContext<'_>,
    parent_symbols: Option<&ModuleSymbols>,
    cfg: Vec<String>,
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let symbols = collect_module_symbols(items, parent_symbols, &cfg, errors);
    for item in items {
        match item {
            Item::Use(item_use) => {
                if !production(&cfg, &item_use.attrs, errors, "use declaration") {
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
            Item::ExternCrate(extern_crate) => {
                if !production(&cfg, &extern_crate.attrs, errors, "extern crate") {
                    continue;
                }
                let source = normalized_ident(&extern_crate.ident);
                let extern_cfg = item_cfg(&cfg, &extern_crate.attrs);
                let extern_visibility = normalized_visibility(&extern_crate.vis);
                add_attribute_records(
                    accumulator,
                    &context,
                    &symbols,
                    &extern_crate.attrs,
                    AttributeRecordContext {
                        enclosing: "module",
                        visibility: &extern_visibility,
                        cfg: &extern_cfg,
                    },
                );
                if source == "sqlx" {
                    let local = extern_crate
                        .rename
                        .as_ref()
                        .map(|(_, rename)| normalized_ident(rename))
                        .unwrap_or_else(|| source.clone());
                    accumulator.add(
                        &context,
                        NewAccess {
                            enclosing: "module",
                            target: PersistenceTarget::Sqlx,
                            operation: PersistenceOperation::Import,
                            symbol: &local,
                            visibility: &extern_visibility,
                            cfg: &extern_cfg,
                            fingerprint: format!("extern crate {source} as {local}"),
                        },
                    );
                }
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
                        classification: context.classification,
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
                        classification: context.classification,
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
                let item_cfg = item_cfg(&cfg, &item_const.attrs);
                let enclosing = format!("const {}", normalized_ident(&item_const.ident));
                add_attribute_records(
                    accumulator,
                    &context,
                    &symbols,
                    &item_const.attrs,
                    AttributeRecordContext {
                        enclosing: &enclosing,
                        visibility: "",
                        cfg: &item_cfg,
                    },
                );
                add_type_records(
                    accumulator,
                    &context,
                    &symbols,
                    &item_const.ty,
                    &enclosing,
                    &normalized_ident(&item_const.ident),
                    "",
                    &item_cfg,
                    PersistenceOperation::TypeReference,
                );
                let mut analyzer = BodyAnalyzer::new(
                    RecordContext {
                        classification: context.classification,
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
                let item_cfg = item_cfg(&cfg, &item_static.attrs);
                let enclosing = format!("static {}", normalized_ident(&item_static.ident));
                let visibility = normalized_visibility(&item_static.vis);
                add_attribute_records(
                    accumulator,
                    &context,
                    &symbols,
                    &item_static.attrs,
                    AttributeRecordContext {
                        enclosing: &enclosing,
                        visibility: &visibility,
                        cfg: &item_cfg,
                    },
                );
                add_type_records(
                    accumulator,
                    &context,
                    &symbols,
                    &item_static.ty,
                    &enclosing,
                    &normalized_ident(&item_static.ident),
                    &visibility,
                    &item_cfg,
                    PersistenceOperation::TypeReference,
                );
                let mut analyzer = BodyAnalyzer::new(
                    RecordContext {
                        classification: context.classification,
                        package: context.package,
                        module: context.module,
                        source: context.source,
                    },
                    accumulator,
                    errors,
                    &symbols,
                    enclosing,
                    visibility,
                    item_cfg,
                );
                analyzer.visit_expr(&item_static.expr);
            }
            Item::Macro(item_macro) => {
                if !production(&cfg, &item_macro.attrs, errors, "item macro") {
                    continue;
                }
                let macro_cfg = item_cfg(&cfg, &item_macro.attrs);
                add_attribute_records(
                    accumulator,
                    &context,
                    &symbols,
                    &item_macro.attrs,
                    AttributeRecordContext {
                        enclosing: "module",
                        visibility: "",
                        cfg: &macro_cfg,
                    },
                );
                analyze_item_macro(
                    item_macro,
                    &context,
                    &symbols,
                    &macro_cfg,
                    accumulator,
                    errors,
                );
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
                let module_cfg = item_cfg(&cfg, attrs);
                add_attribute_records(
                    accumulator,
                    &context,
                    &symbols,
                    attrs,
                    AttributeRecordContext {
                        enclosing: &child_module,
                        visibility: "",
                        cfg: &module_cfg,
                    },
                );
                analyze_module_items(
                    child_items,
                    RecordContext {
                        classification: context.classification,
                        package: context.package,
                        module: &child_module,
                        source: context.source,
                    },
                    Some(&symbols),
                    module_cfg,
                    accumulator,
                    errors,
                );
            }
            unsupported => {
                if syntax_mentions_persistence(unsupported, &symbols) {
                    errors.push(format!(
                        "{} contains persistence syntax in an unsupported item grammar; expose an explicit import/type/function/impl before baselining: {}",
                        context.module,
                        normalized_tokens(unsupported)
                    ));
                }
            }
        }
    }
}

/// Parse and inventory an already-classified set of production source mounts.
/// Source order is irrelevant and duplicate logical mounts fail closed.
pub(crate) fn inventory_persistence_accesses(
    sources: &[ClassifiedPersistenceSource<'_>],
) -> Result<PersistenceAccessBaseline, String> {
    let mut ordered = sources.iter().copied().collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        (
            left.classification,
            left.package,
            left.module,
            left.source_path,
        )
            .cmp(&(
                right.classification,
                right.package,
                right.module,
                right.source_path,
            ))
    });
    let mut seen_mounts = BTreeSet::new();
    let mut accumulator = AccessAccumulator::default();
    let mut errors = Vec::new();
    for source in ordered {
        if source.classification.is_empty()
            || source.package.is_empty()
            || source.module.is_empty()
            || source.source_path.is_empty()
        {
            errors.push(
                "persistence source classification/package/module/path must be non-empty"
                    .to_owned(),
            );
            continue;
        }
        if !seen_mounts.insert((source.package, source.module, source.source_path)) {
            errors.push(format!(
                "duplicate classified persistence source mount {} {} {}",
                source.package, source.module, source.source_path
            ));
            continue;
        }
        let syntax = match syn::parse_file(source.source) {
            Ok(syntax) => syntax,
            Err(error) => {
                errors.push(format!(
                    "cannot parse persistence source {}: {error}",
                    source.source_path
                ));
                continue;
            }
        };
        match cfg_context_allows_production(source.inherited_cfg, &syntax.attrs) {
            Ok(true) => {}
            Ok(false) => {
                errors.push(format!(
                    "source {} was classified as production persistence but its file attributes are test-only",
                    source.source_path
                ));
                continue;
            }
            Err(error) => {
                errors.push(format!(
                    "invalid file cfg in persistence source {}: {error}",
                    source.source_path
                ));
                continue;
            }
        }
        analyze_module_items(
            &syntax.items,
            RecordContext {
                classification: source.classification,
                package: source.package,
                module: source.module,
                source: source.source_path,
            },
            None,
            extend_cfg_context(source.inherited_cfg, &syntax.attrs),
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
    baseline: &PersistenceAccessBaseline,
) -> Result<BTreeMap<AccessIdentity, usize>, String> {
    if baseline.schema_version != PERSISTENCE_SCHEMA_VERSION {
        return Err(format!(
            "{label} persistence baseline schema version is {}, expected {PERSISTENCE_SCHEMA_VERSION}",
            baseline.schema_version
        ));
    }
    let mut map = BTreeMap::new();
    let mut previous: Option<AccessIdentity> = None;
    for record in &baseline.accesses {
        if record.count == 0 {
            return Err(format!(
                "{label} persistence baseline contains zero-count row for {:?} {}",
                record.target, record.symbol
            ));
        }
        let identity = record.identity();
        if previous
            .as_ref()
            .is_some_and(|previous| previous >= &identity)
        {
            return Err(format!(
                "{label} persistence baseline rows are not in strict canonical order near {:?} {}",
                record.target, record.symbol
            ));
        }
        previous = Some(identity.clone());
        if map.insert(identity, record.count).is_some() {
            return Err(format!(
                "{label} persistence baseline contains a duplicate row for {:?} {}",
                record.target, record.symbol
            ));
        }
    }
    Ok(map)
}

fn describe_identity(identity: &AccessIdentity) -> String {
    format!(
        "{} {} {} {}::{} {} {:?} {:?} {} [{}]",
        identity.classification,
        identity.package,
        identity.source,
        identity.module,
        identity.enclosing,
        identity.symbol,
        identity.target,
        identity.operation,
        identity.fingerprint,
        identity.cfg.join(", ")
    )
}

/// Compare exact persistence identities and multiplicities in both directions.
pub(crate) fn compare_persistence_access_baseline(
    expected: &PersistenceAccessBaseline,
    actual: &PersistenceAccessBaseline,
) -> Result<(), String> {
    let expected = validated_baseline_map("expected", expected)?;
    let actual = validated_baseline_map("actual", actual)?;
    let mut errors = Vec::new();
    for (identity, actual_count) in &actual {
        match expected.get(identity) {
            None => errors.push(format!(
                "untracked direct persistence access: {} (count {actual_count})",
                describe_identity(identity)
            )),
            Some(expected_count) if expected_count != actual_count => errors.push(format!(
                "direct persistence access multiplicity changed: {} expected {expected_count}, actual {actual_count}",
                describe_identity(identity)
            )),
            Some(_) => {}
        }
    }
    for (identity, expected_count) in &expected {
        if !actual.contains_key(identity) {
            errors.push(format!(
                "obsolete direct persistence baseline row: {} (expected count {expected_count})",
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

    fn inventory(source: &str) -> Result<PersistenceAccessBaseline, String> {
        inventory_persistence_accesses(&[ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::fixture",
            source_path: "src/fixture.rs",
            inherited_cfg: &[],
            source,
        }])
    }

    fn operations(
        baseline: &PersistenceAccessBaseline,
    ) -> BTreeSet<(PersistenceTarget, PersistenceOperation, String)> {
        baseline
            .accesses
            .iter()
            .map(|record| (record.target, record.operation, record.symbol.clone()))
            .collect()
    }

    #[test]
    fn persistence_inventory_tracks_aliases_queries_transactions_and_pool_returns() {
        let baseline = inventory(
            r#"
                use sqlx::{query_as as load, MySqlPool as Pool};
                use std::sync::Arc;

                type SharedPool = Arc<Pool>;
                struct Adapter { pool: SharedPool }

                async fn work(adapter: &Adapter) -> SharedPool {
                    let pool = Arc::clone(&adapter.pool);
                    let mut tx = pool.begin().await.unwrap();
                    load::<_, Row>("SELECT 1")
                        .fetch_optional(&mut tx)
                        .await
                        .unwrap();
                    tx.commit().await.unwrap();
                    pool
                }
            "#,
        )
        .expect("strict persistence fixture parses");
        let found = operations(&baseline);
        for expected in [
            (
                PersistenceTarget::Sqlx,
                PersistenceOperation::Import,
                "load",
            ),
            (
                PersistenceTarget::MySqlPool,
                PersistenceOperation::Import,
                "Pool",
            ),
            (
                PersistenceTarget::MySqlPool,
                PersistenceOperation::TypeAlias,
                "SharedPool",
            ),
            (PersistenceTarget::Sqlx, PersistenceOperation::Query, "load"),
            (
                PersistenceTarget::MySqlPool,
                PersistenceOperation::Begin,
                "begin",
            ),
            (
                PersistenceTarget::Sqlx,
                PersistenceOperation::FetchOptional,
                "fetch_optional",
            ),
            (
                PersistenceTarget::MySqlPool,
                PersistenceOperation::Commit,
                "commit",
            ),
            (
                PersistenceTarget::MySqlPool,
                PersistenceOperation::ReturnEscape,
                "pool",
            ),
        ] {
            assert!(
                found.contains(&(expected.0, expected.1, expected.2.to_owned())),
                "missing {expected:?} from {found:#?}"
            );
        }
        assert!(
            !baseline.accesses.iter().any(|record| {
                record.operation == PersistenceOperation::ArgumentEscape && record.symbol == "clone"
            }),
            "the explicit Arc::clone grammar is value flow, not an unknown escape"
        );
    }

    #[test]
    fn persistence_inventory_follows_module_and_type_aliases_independent_of_order() {
        let baseline = inventory(
            r#"
                use db::PgPool as Pool;
                use sqlx as db;
                type Outer = Option<std::sync::Arc<Pool>>;
                fn expose(pool: Outer) -> Outer { pool }
            "#,
        )
        .expect("aliases resolve to a fixed point");
        let found = operations(&baseline);
        assert!(found.contains(&(
            PersistenceTarget::Sqlx,
            PersistenceOperation::Import,
            "db".to_owned()
        )));
        assert!(found.contains(&(
            PersistenceTarget::PgPool,
            PersistenceOperation::Import,
            "Pool".to_owned()
        )));
        assert!(found.contains(&(
            PersistenceTarget::PgPool,
            PersistenceOperation::ReturnEscape,
            "pool".to_owned()
        )));
    }

    #[test]
    fn persistence_inventory_records_query_macros_and_rejects_opaque_macro_escapes() {
        let baseline = inventory(
            r#"
                use sqlx::MySqlPool;
                #[derive(sqlx::FromRow)]
                struct ProjectedRow { value: u64 }

                #[sqlx::test]
                async fn adapter_contract() {}

                fn query(pool: &MySqlPool) {
                    sqlx::query!("SELECT 1").execute(pool);
                }

                fn logged(pool: &MySqlPool) { info!(?pool, "pool"); }
            "#,
        )
        .expect("known SQLx macro is explicit inventory grammar");
        assert!(operations(&baseline).contains(&(
            PersistenceTarget::Sqlx,
            PersistenceOperation::Query,
            "query".to_owned()
        )));
        assert!(operations(&baseline).contains(&(
            PersistenceTarget::Sqlx,
            PersistenceOperation::MacroReference,
            "derive".to_owned()
        )));
        assert!(operations(&baseline).contains(&(
            PersistenceTarget::Sqlx,
            PersistenceOperation::MacroReference,
            "test".to_owned()
        )));
        assert!(operations(&baseline).contains(&(
            PersistenceTarget::MySqlPool,
            PersistenceOperation::MacroReference,
            "info".to_owned()
        )));
        assert!(operations(&baseline).contains(&(
            PersistenceTarget::MySqlPool,
            PersistenceOperation::ArgumentEscape,
            "macro:info".to_owned()
        )));

        let error = inventory(
            r#"
                use sqlx::MySqlPool;
                fn hidden(pool: &MySqlPool) { hide_access!(pool); }
            "#,
        )
        .expect_err("unknown macro cannot hide a concrete pool");
        assert!(error.contains("unknown macro hide_access!"), "{error}");

        let generated = inventory(
            r#"
                macro_rules! hidden_query { () => { sqlx::query("SELECT 1") } }
            "#,
        )
        .expect("macro-generated persistence is an exact opaque baseline row");
        assert!(operations(&generated).contains(&(
            PersistenceTarget::Sqlx,
            PersistenceOperation::MacroReference,
            "hidden_query".to_owned()
        )));

        let error = inventory(
            r#"
                trait HiddenPort { fn pool(&self) -> sqlx::PgPool; }
            "#,
        )
        .expect_err("unsupported item grammars must fail closed");
        assert!(error.contains("unsupported item grammar"), "{error}");
    }

    #[test]
    fn persistence_inventory_rejects_relevant_globs_and_records_pool_escapes() {
        let error = inventory("use sqlx::*;")
            .expect_err("glob import can hide arbitrary concrete SQLx syntax");
        assert!(error.contains("glob import sqlx::*"), "{error}");

        let baseline = inventory(
            r#"
                use sqlx::PgPool;
                struct Holder { value: usize }
                fn consume(_: &str, _: &PgPool) {}
                fn escapes(pool: PgPool, mut holder: Holder) -> PgPool {
                    consume("pool", &pool);
                    evil::clone(&pool);
                    holder.value = 1;
                    Wrapper { pool: pool.clone() };
                    pool
                }
            "#,
        )
        .expect("ordinary escapes are inventoried");
        let found = operations(&baseline);
        assert!(found.contains(&(
            PersistenceTarget::PgPool,
            PersistenceOperation::ArgumentEscape,
            "consume".to_owned()
        )));
        assert!(found.contains(&(
            PersistenceTarget::PgPool,
            PersistenceOperation::ArgumentEscape,
            "clone".to_owned()
        )));
        assert!(found.contains(&(
            PersistenceTarget::PgPool,
            PersistenceOperation::StoreEscape,
            "pool".to_owned()
        )));
        assert!(found.contains(&(
            PersistenceTarget::PgPool,
            PersistenceOperation::ReturnEscape,
            "pool".to_owned()
        )));
    }

    #[test]
    fn persistence_inventory_is_cfg_aware_and_malformed_cfg_fails_closed() {
        let baseline = inventory(
            r#"
                #[cfg(test)]
                fn test_only(pool: sqlx::PgPool) { pool.begin(); }

                #[cfg(any(test, feature = "live-db"))]
                fn production_capable(pool: sqlx::PgPool) { pool.begin(); }
            "#,
        )
        .expect("production satisfiability is classified");
        assert!(
            baseline
                .accesses
                .iter()
                .all(|record| record.enclosing != "fn test_only")
        );
        assert!(
            baseline
                .accesses
                .iter()
                .any(|record| record.enclosing == "fn production_capable")
        );

        let error = inventory(
            r#"
                #[cfg_attr(test)]
                fn malformed(pool: sqlx::PgPool) { pool.begin(); }
            "#,
        )
        .expect_err("malformed cfg_attr must fail closed");
        assert!(error.contains("invalid cfg"), "{error}");
    }

    #[test]
    fn persistence_baseline_detects_same_count_substitution_and_multiplicity() {
        let expected = inventory(
            r#"
                use sqlx::PgPool;
                fn transaction(pool: &PgPool) { pool.begin(); }
            "#,
        )
        .unwrap();
        let actual = inventory(
            r#"
                use sqlx::PgPool;
                fn transaction(pool: &PgPool) { pool.execute("DELETE"); }
            "#,
        )
        .unwrap();
        let error = compare_persistence_access_baseline(&expected, &actual)
            .expect_err("same-count operation swap must fail");
        assert!(
            error.contains("untracked direct persistence access"),
            "{error}"
        );
        assert!(
            error.contains("obsolete direct persistence baseline row"),
            "{error}"
        );

        let mut noncanonical = expected.clone();
        noncanonical.accesses[0].count = 0;
        assert!(
            compare_persistence_access_baseline(&noncanonical, &actual)
                .unwrap_err()
                .contains("zero-count")
        );

        let mut multiplicity = expected.clone();
        multiplicity
            .accesses
            .iter_mut()
            .find(|row| row.operation == PersistenceOperation::Begin)
            .expect("begin row")
            .count += 1;
        assert!(
            compare_persistence_access_baseline(&multiplicity, &expected)
                .unwrap_err()
                .contains("multiplicity changed")
        );
    }

    #[test]
    fn persistence_baseline_is_serializable_and_input_order_independent() {
        let first = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "a",
            module: "crate::a",
            source_path: "src/a.rs",
            inherited_cfg: &[],
            source: "fn a(pool: sqlx::PgPool) { pool.begin(); }",
        };
        let second = ClassifiedPersistenceSource {
            classification: "wow_world_concrete_persistence_leaks",
            package: "b",
            module: "crate::b",
            source_path: "src/b.rs",
            inherited_cfg: &[],
            source: "fn b(pool: sqlx::MySqlPool) { pool.begin(); }",
        };
        let forward = inventory_persistence_accesses(&[first, second]).unwrap();
        let reverse = inventory_persistence_accesses(&[second, first]).unwrap();
        assert_eq!(forward, reverse);
        let json = serde_json::to_string(&forward).expect("baseline serializes");
        assert_eq!(
            serde_json::from_str::<PersistenceAccessBaseline>(&json)
                .expect("baseline deserializes"),
            forward
        );
    }
}
