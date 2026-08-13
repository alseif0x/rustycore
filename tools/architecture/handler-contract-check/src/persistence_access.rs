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

use crate::ownership::{
    cfg_context_allows_production, cfg_context_allows_test, extend_cfg_context,
};

const PERSISTENCE_SCHEMA_VERSION: u32 = 3;

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
    "map_err",
    "ok_or",
    "ok_or_else",
    "or",
    "or_else",
    "unwrap",
    "unwrap_or",
    "unwrap_or_else",
    "context",
    "with_context",
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
    "ensure",
    "info",
    "matches",
    "panic",
    "select",
    "trace",
    "vec",
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
    SqlxTransaction,
    Database,
    LoginDatabase,
    WorldDatabase,
    CharacterDatabase,
    HotfixDatabase,
    LoginStatements,
    WorldStatements,
    CharStatements,
    HotfixStatements,
    PreparedStatement,
    SqlParam,
    SqlTransaction,
    SqlTransactionCommitError,
    SqlResult,
    SqlFields,
    SqlQueryHolder,
    SqlQueryHolderResult,
    StatementDef,
    DatabaseError,
    ItemGuidAllocatorAdvisoryLockLikeCpp,
}

impl PersistenceTarget {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "sqlx" => Some(Self::Sqlx),
            "MySqlPool" => Some(Self::MySqlPool),
            "PgPool" => Some(Self::PgPool),
            "DatabaseConnection" => Some(Self::DatabaseConnection),
            "Transaction" => Some(Self::SqlxTransaction),
            "Database" => Some(Self::Database),
            "LoginDatabase" => Some(Self::LoginDatabase),
            "WorldDatabase" => Some(Self::WorldDatabase),
            "CharacterDatabase" => Some(Self::CharacterDatabase),
            "HotfixDatabase" => Some(Self::HotfixDatabase),
            "LoginStatements" => Some(Self::LoginStatements),
            "WorldStatements" => Some(Self::WorldStatements),
            "CharStatements" => Some(Self::CharStatements),
            "HotfixStatements" => Some(Self::HotfixStatements),
            "PreparedStatement" => Some(Self::PreparedStatement),
            "SqlParam" => Some(Self::SqlParam),
            "SqlTransaction" => Some(Self::SqlTransaction),
            "SqlTransactionCommitError" => Some(Self::SqlTransactionCommitError),
            "SqlResult" => Some(Self::SqlResult),
            "SqlFields" => Some(Self::SqlFields),
            "SqlQueryHolder" => Some(Self::SqlQueryHolder),
            "SqlQueryHolderResult" => Some(Self::SqlQueryHolderResult),
            "StatementDef" => Some(Self::StatementDef),
            "DatabaseError" => Some(Self::DatabaseError),
            "ItemGuidAllocatorAdvisoryLockLikeCpp" => {
                Some(Self::ItemGuidAllocatorAdvisoryLockLikeCpp)
            }
            _ => None,
        }
    }

    fn source_name(self) -> &'static str {
        match self {
            Self::Sqlx => "sqlx",
            Self::MySqlPool => "MySqlPool",
            Self::PgPool => "PgPool",
            Self::DatabaseConnection => "DatabaseConnection",
            Self::SqlxTransaction => "Transaction",
            Self::Database => "Database",
            Self::LoginDatabase => "LoginDatabase",
            Self::WorldDatabase => "WorldDatabase",
            Self::CharacterDatabase => "CharacterDatabase",
            Self::HotfixDatabase => "HotfixDatabase",
            Self::LoginStatements => "LoginStatements",
            Self::WorldStatements => "WorldStatements",
            Self::CharStatements => "CharStatements",
            Self::HotfixStatements => "HotfixStatements",
            Self::PreparedStatement => "PreparedStatement",
            Self::SqlParam => "SqlParam",
            Self::SqlTransaction => "SqlTransaction",
            Self::SqlTransactionCommitError => "SqlTransactionCommitError",
            Self::SqlResult => "SqlResult",
            Self::SqlFields => "SqlFields",
            Self::SqlQueryHolder => "SqlQueryHolder",
            Self::SqlQueryHolderResult => "SqlQueryHolderResult",
            Self::StatementDef => "StatementDef",
            Self::DatabaseError => "DatabaseError",
            Self::ItemGuidAllocatorAdvisoryLockLikeCpp => "ItemGuidAllocatorAdvisoryLockLikeCpp",
        }
    }

    fn carries_persistence_flow(self) -> bool {
        !matches!(
            self,
            Self::Sqlx
                | Self::LoginStatements
                | Self::WorldStatements
                | Self::CharStatements
                | Self::HotfixStatements
                | Self::SqlParam
                | Self::StatementDef
                | Self::DatabaseError
                | Self::SqlTransactionCommitError
        )
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
    PoolAccess,
    PrepareStatement,
    DirectQuery,
    DirectExecute,
    RawSql,
    NonliteralSql,
    InterpolatedSql,
    TransactionAppend,
    GeneratedIdRead,
    AdvisoryLock,
    DatabaseOpen,
    TransactionConstruct,
    StatementBuilder,
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
            "pool" => Some(Self::PoolAccess),
            "prepare" => Some(Self::PrepareStatement),
            "direct_query" => Some(Self::DirectQuery),
            "direct_execute" => Some(Self::DirectExecute),
            "commit_transaction" | "commit_with_outcome_like_cpp" => Some(Self::Commit),
            "append" | "append_expect_rows_affected" | "execute_or_append" => {
                Some(Self::TransactionAppend)
            }
            "append_raw_sql_like_cpp" | "raw_sql_like_cpp" => Some(Self::RawSql),
            "last_insert_id" => Some(Self::GeneratedIdRead),
            "acquire_like_cpp" | "release_like_cpp" | "wait_until_lost_like_cpp" => {
                Some(Self::AdvisoryLock)
            }
            "open"
            | "open_with_pool_size"
            | "open_with_pool_size_and_auto_create_like_cpp"
            | "from_pool" => Some(Self::DatabaseOpen),
            "new" => Some(Self::TransactionConstruct),
            "with_capacity_like_cpp" => Some(Self::StatementBuilder),
            _ => None,
        }
    }
}

/// One canonical, counted persistence access row.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PersistenceAccessRecord {
    pub(crate) classification: String,
    pub(crate) source_class: String,
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
    pub(crate) generated_input: bool,
    pub(crate) count: usize,
}

/// Serializable exact snapshot. Rows are strictly ordered by full identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PersistenceAccessBaseline {
    pub(crate) schema_version: u32,
    pub(crate) accesses: Vec<PersistenceAccessRecord>,
}

impl Default for PersistenceAccessBaseline {
    fn default() -> Self {
        Self {
            schema_version: PERSISTENCE_SCHEMA_VERSION,
            accesses: Vec::new(),
        }
    }
}

/// One production/test source mount assigned to a runtime-ledger
/// classification. The repository walker owns file discovery and logical cfg
/// ancestry; this parser inventories production-capable and test-only items
/// as distinct exact baseline rows.
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
    source_class: String,
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
    generated_input: bool,
}

impl PersistenceAccessRecord {
    fn identity(&self) -> AccessIdentity {
        AccessIdentity {
            classification: self.classification.clone(),
            source_class: self.source_class.clone(),
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
            generated_input: self.generated_input,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PersistenceSourceClass {
    Production,
    TestFixture,
}

impl PersistenceSourceClass {
    fn as_str(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::TestFixture => "test_fixture",
        }
    }
}

#[derive(Clone, Copy)]
struct RecordContext<'a> {
    classification: &'a str,
    source_class: PersistenceSourceClass,
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
    generated_input: bool,
}

#[derive(Default)]
struct AccessAccumulator {
    rows: BTreeMap<AccessIdentity, usize>,
}

impl AccessAccumulator {
    fn add(&mut self, context: &RecordContext<'_>, access: NewAccess<'_>) {
        // The test view needs production-visible imports and aliases for exact
        // name/value resolution, but those rows already belong to the
        // production view. Only retain syntax that is satisfiable with
        // `cfg(test)` and impossible without it.
        if context.source_class == PersistenceSourceClass::TestFixture
            && cfg_context_allows_production(access.cfg, &[])
                .expect("persistence cfg was validated before recording")
        {
            return;
        }
        let identity = AccessIdentity {
            classification: context.classification.to_owned(),
            source_class: context.source_class.as_str().to_owned(),
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
            generated_input: access.generated_input,
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
                    source_class: identity.source_class,
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
                    generated_input: identity.generated_input,
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

fn sql_is_advisory_lock(fingerprint: &str) -> bool {
    ["GET_LOCK", "RELEASE_LOCK", "IS_USED_LOCK"]
        .iter()
        .any(|needle| fingerprint.contains(needle))
}

fn sqlx_calls_in_tokens(tokens: TokenStream, output: &mut Vec<(String, String)>) {
    let trees = tokens.into_iter().collect::<Vec<_>>();
    let mut index = 0;
    while index < trees.len() {
        if let TokenTree::Group(group) = &trees[index] {
            sqlx_calls_in_tokens(group.stream(), output);
        }
        let is_sqlx =
            matches!(&trees[index], TokenTree::Ident(ident) if normalized_ident(ident) == "sqlx");
        if is_sqlx && index + 3 < trees.len() {
            let separator = matches!(&trees[index + 1], TokenTree::Punct(punct) if punct.as_char() == ':')
                && matches!(&trees[index + 2], TokenTree::Punct(punct) if punct.as_char() == ':');
            if separator && let TokenTree::Ident(callable) = &trees[index + 3] {
                let callable = normalized_ident(callable);
                if is_query_name(&callable) {
                    let fingerprint = trees[index + 4..]
                        .iter()
                        .find_map(|token| match token {
                            TokenTree::Group(group)
                                if group.delimiter() == proc_macro2::Delimiter::Parenthesis =>
                            {
                                Some(format!("sqlx::{callable}({})", group.stream()))
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| format!("sqlx::{callable}(opaque-macro-arguments)"));
                    output.push((callable, fingerprint));
                }
            }
        }
        index += 1;
    }
}

fn persistence_methods_in_tokens(
    tokens: TokenStream,
    names: &BTreeSet<String>,
    output: &mut Vec<String>,
) {
    let trees = tokens.into_iter().collect::<Vec<_>>();
    for (index, token) in trees.iter().enumerate() {
        if let TokenTree::Group(group) = token {
            persistence_methods_in_tokens(group.stream(), names, output);
        }
        if matches!(token, TokenTree::Punct(punct) if punct.as_char() == '.')
            && let Some(TokenTree::Ident(method)) = trees.get(index + 1)
        {
            let method = normalized_ident(method);
            if names.contains(&method) {
                output.push(method);
            }
        }
    }
}

fn item_cfg(parent: &[String], attributes: &[Attribute]) -> Vec<String> {
    extend_cfg_context(parent, attributes)
}

fn source_class_allows(
    source_class: PersistenceSourceClass,
    parent: &[String],
    attributes: &[Attribute],
    errors: &mut Vec<String>,
    owner: &str,
) -> bool {
    let production = cfg_context_allows_production(parent, attributes);
    let test = cfg_context_allows_test(parent, attributes);
    match (production, test) {
        (Ok(production), Ok(test)) => match source_class {
            PersistenceSourceClass::Production => production,
            PersistenceSourceClass::TestFixture => test,
        },
        (production, test) => {
            if let Err(error) = production {
                errors.push(format!("invalid cfg (production) on {owner}: {error}"));
            }
            if let Err(error) = test {
                errors.push(format!("invalid cfg (test) on {owner}: {error}"));
            }
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
                .filter(|target| target.carries_persistence_flow())
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

    fn is_empty(&self) -> bool {
        self.0.is_empty()
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum SqlExpressionKind {
    #[default]
    Static,
    Nonliteral,
    Interpolated,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct VariableInfo {
    flow: Flow,
    sql_expression: SqlExpressionKind,
    nominal_types: BTreeSet<String>,
    payload_variants: BTreeSet<Vec<NominalShape>>,
    tuple_items: Vec<VariableInfo>,
    trait_bounds: BTreeSet<String>,
}

impl VariableInfo {
    fn union(&mut self, other: &Self) {
        self.flow.union(other.flow.clone());
        self.nominal_types
            .extend(other.nominal_types.iter().cloned());
        self.payload_variants
            .extend(other.payload_variants.iter().cloned());
        self.trait_bounds.extend(other.trait_bounds.iter().cloned());
        if self.sql_expression == SqlExpressionKind::Static {
            self.sql_expression = other.sql_expression;
        } else if other.sql_expression == SqlExpressionKind::Interpolated {
            self.sql_expression = SqlExpressionKind::Interpolated;
        }
        if self.tuple_items.len() < other.tuple_items.len() {
            self.tuple_items
                .resize_with(other.tuple_items.len(), VariableInfo::default);
        }
        for (item, other_item) in self.tuple_items.iter_mut().zip(&other.tuple_items) {
            item.union(other_item);
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct NominalShape {
    nominal_types: BTreeSet<String>,
    arguments: Vec<NominalShape>,
}

#[derive(Clone, Debug)]
struct ModuleSymbols {
    type_aliases: BTreeMap<String, TargetSet>,
    nominal_type_aliases: BTreeMap<String, BTreeSet<String>>,
    type_alias_info: BTreeMap<String, VariableInfo>,
    path_aliases: BTreeMap<String, Vec<String>>,
    traits_in_scope: BTreeMap<String, String>,
    anonymous_traits_in_scope: BTreeSet<String>,
    module_path: Vec<String>,
    field_targets: BTreeMap<(String, String), TargetSet>,
    field_owners: BTreeMap<String, BTreeSet<String>>,
    tuple_field_targets: BTreeMap<(String, String), TargetSet>,
    field_nominal_types: BTreeMap<(String, String), BTreeSet<String>>,
    function_returns: BTreeMap<String, VariableInfo>,
    method_returns: BTreeMap<(String, Option<String>, String), VariableInfo>,
    trait_method_returns: std::sync::Arc<BTreeMap<(String, String), VariableInfo>>,
    trait_supertraits: std::sync::Arc<BTreeMap<String, BTreeSet<String>>>,
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
            nominal_type_aliases: BTreeMap::new(),
            type_alias_info: BTreeMap::new(),
            path_aliases: BTreeMap::new(),
            traits_in_scope: BTreeMap::new(),
            anonymous_traits_in_scope: BTreeSet::new(),
            module_path: Vec::new(),
            field_targets: BTreeMap::new(),
            field_owners: BTreeMap::new(),
            tuple_field_targets: BTreeMap::new(),
            field_nominal_types: BTreeMap::new(),
            function_returns: BTreeMap::new(),
            method_returns: BTreeMap::new(),
            trait_method_returns: std::sync::Arc::new(BTreeMap::new()),
            trait_supertraits: std::sync::Arc::new(BTreeMap::new()),
            sqlx_namespaces: BTreeSet::from(["sqlx".to_owned()]),
            database_namespaces: BTreeSet::from(["wow_database".to_owned()]),
            query_callables: BTreeSet::new(),
        }
    }
}

impl ModuleSymbols {
    fn for_package(package: &str) -> Self {
        let mut symbols = Self::default();
        if package == "wow-database" {
            symbols.database_namespaces.insert("crate".to_owned());
            for target in [
                PersistenceTarget::Database,
                PersistenceTarget::LoginDatabase,
                PersistenceTarget::WorldDatabase,
                PersistenceTarget::CharacterDatabase,
                PersistenceTarget::HotfixDatabase,
                PersistenceTarget::LoginStatements,
                PersistenceTarget::WorldStatements,
                PersistenceTarget::CharStatements,
                PersistenceTarget::HotfixStatements,
                PersistenceTarget::PreparedStatement,
                PersistenceTarget::SqlParam,
                PersistenceTarget::SqlTransaction,
                PersistenceTarget::SqlTransactionCommitError,
                PersistenceTarget::SqlResult,
                PersistenceTarget::SqlFields,
                PersistenceTarget::SqlQueryHolder,
                PersistenceTarget::SqlQueryHolderResult,
                PersistenceTarget::StatementDef,
                PersistenceTarget::DatabaseError,
                PersistenceTarget::ItemGuidAllocatorAdvisoryLockLikeCpp,
            ] {
                symbols
                    .type_aliases
                    .insert(target.source_name().to_owned(), BTreeSet::from([target]));
            }
        }
        symbols
    }
}

fn is_query_name(name: &str) -> bool {
    QUERY_CONSTRUCTORS.contains(&name) || name.starts_with("query_")
}

fn database_getter_target(name: &str) -> Option<PersistenceTarget> {
    match name {
        "login_db" | "login_database" => Some(PersistenceTarget::LoginDatabase),
        "world_db" | "world_database" => Some(PersistenceTarget::WorldDatabase),
        "char_db" | "character_db" | "character_database" => {
            Some(PersistenceTarget::CharacterDatabase)
        }
        "hotfix_db" | "hotfix_database" => Some(PersistenceTarget::HotfixDatabase),
        _ => None,
    }
}

fn database_field_target(name: &str) -> Option<PersistenceTarget> {
    match name {
        "login_db" => Some(PersistenceTarget::LoginDatabase),
        "world_db" => Some(PersistenceTarget::WorldDatabase),
        "char_db" | "character_db" => Some(PersistenceTarget::CharacterDatabase),
        "hotfix_db" => Some(PersistenceTarget::HotfixDatabase),
        _ => None,
    }
}

fn is_generated_id_read_statement(name: &str) -> bool {
    name.starts_with("SEL_MAX_")
        || name.starts_with("SEL_BNET_MAX_")
        || name.contains("_MAXID")
        || name.ends_with("_MAX_NODEID")
        || name.ends_with("_MAX_PATHID")
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
        targets.insert(
            match names.iter().find_map(|name| match name.as_str() {
                "Transaction" => Some(PersistenceTarget::SqlxTransaction),
                _ => None,
            }) {
                Some(target) => target,
                None => match last.as_str() {
                    "MySqlPool" => PersistenceTarget::MySqlPool,
                    "PgPool" => PersistenceTarget::PgPool,
                    "DatabaseConnection" => PersistenceTarget::DatabaseConnection,
                    _ => PersistenceTarget::Sqlx,
                },
            },
        );
    }
    if symbols.database_namespaces.contains(first) {
        for name in names.iter().skip(1) {
            if let Some(target) = PersistenceTarget::from_name(name) {
                targets.insert(target);
                break;
            }
        }
    }
    // A locally imported type alias can only own the root of a path. Looking
    // at every segment makes an unrelated enum variant such as
    // `DatabaseError::Transaction` inherit an in-scope `sqlx::Transaction`
    // import merely because the leaf names collide.
    if let Some(alias_targets) = symbols.type_aliases.get(first) {
        targets.extend(alias_targets);
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

fn nominal_types_in_type(ty: &Type) -> BTreeSet<String> {
    match ty {
        Type::Path(path) => last_path_name(&path.path)
            .map(|name| BTreeSet::from([name]))
            .unwrap_or_default(),
        Type::Reference(reference) => nominal_types_in_type(&reference.elem),
        Type::Ptr(pointer) => nominal_types_in_type(&pointer.elem),
        Type::Paren(paren) => nominal_types_in_type(&paren.elem),
        Type::Group(group) => nominal_types_in_type(&group.elem),
        _ => BTreeSet::new(),
    }
}

fn receiver_nominal_types_in_type(ty: &Type) -> BTreeSet<String> {
    match ty {
        Type::Path(path) => {
            let Some(segment) = path.path.segments.last() else {
                return BTreeSet::new();
            };
            let name = normalized_ident(&segment.ident);
            if matches!(name.as_str(), "Box" | "Arc" | "Rc" | "Pin")
                && let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments
                && let Some(syn::GenericArgument::Type(inner)) = arguments.args.first()
            {
                receiver_nominal_types_in_type(inner)
            } else {
                BTreeSet::from([name])
            }
        }
        Type::Reference(reference) => receiver_nominal_types_in_type(&reference.elem),
        Type::Ptr(pointer) => receiver_nominal_types_in_type(&pointer.elem),
        Type::Paren(paren) => receiver_nominal_types_in_type(&paren.elem),
        Type::Group(group) => receiver_nominal_types_in_type(&group.elem),
        _ => BTreeSet::new(),
    }
}

fn nominal_shape_in_type(ty: &Type, symbols: &ModuleSymbols) -> Option<NominalShape> {
    match ty {
        Type::Tuple(tuple) => Some(NominalShape {
            nominal_types: BTreeSet::new(),
            arguments: tuple
                .elems
                .iter()
                .map(|element| {
                    nominal_shape_in_type(element, symbols).unwrap_or_else(|| NominalShape {
                        nominal_types: BTreeSet::new(),
                        arguments: Vec::new(),
                    })
                })
                .collect(),
        }),
        Type::Path(path) => path.path.segments.last().map(|segment| NominalShape {
            nominal_types: resolve_nominal_types(
                BTreeSet::from([normalized_ident(&segment.ident)]),
                symbols,
            ),
            arguments: match &segment.arguments {
                syn::PathArguments::AngleBracketed(arguments) => arguments
                    .args
                    .iter()
                    .filter_map(|argument| match argument {
                        syn::GenericArgument::Type(inner) => nominal_shape_in_type(inner, symbols),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            },
        }),
        Type::Reference(reference) => nominal_shape_in_type(&reference.elem, symbols),
        Type::Ptr(pointer) => nominal_shape_in_type(&pointer.elem, symbols),
        Type::Paren(paren) => nominal_shape_in_type(&paren.elem, symbols),
        Type::Group(group) => nominal_shape_in_type(&group.elem, symbols),
        _ => None,
    }
}

fn payload_variants_in_type(ty: &Type, symbols: &ModuleSymbols) -> BTreeSet<Vec<NominalShape>> {
    nominal_shape_in_type(ty, symbols)
        .and_then(|shape| (!shape.arguments.is_empty()).then_some(shape.arguments))
        .map(|arguments| BTreeSet::from([arguments]))
        .unwrap_or_default()
}

fn tuple_items_in_type(ty: &Type, symbols: &ModuleSymbols) -> Vec<VariableInfo> {
    match ty {
        Type::Tuple(tuple) => tuple
            .elems
            .iter()
            .map(|element| VariableInfo {
                flow: Flow::pools(&targets_in_type(element, symbols)),
                sql_expression: SqlExpressionKind::Static,
                nominal_types: resolve_nominal_types(
                    receiver_nominal_types_in_type(element),
                    symbols,
                ),
                payload_variants: payload_variants_in_type(element, symbols),
                tuple_items: tuple_items_in_type(element, symbols),
                trait_bounds: trait_bounds_in_type(element, symbols),
            })
            .collect(),
        Type::Reference(reference) => tuple_items_in_type(&reference.elem, symbols),
        Type::Ptr(pointer) => tuple_items_in_type(&pointer.elem, symbols),
        Type::Paren(paren) => tuple_items_in_type(&paren.elem, symbols),
        Type::Group(group) => tuple_items_in_type(&group.elem, symbols),
        Type::Path(path) => path
            .path
            .segments
            .last()
            .and_then(|segment| {
                symbols
                    .type_alias_info
                    .get(&normalized_ident(&segment.ident))
            })
            .map(|info| info.tuple_items.clone())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn variable_info_in_type(ty: &Type, symbols: &ModuleSymbols) -> VariableInfo {
    let mut info = VariableInfo {
        flow: Flow::pools(&targets_in_type(ty, symbols)),
        sql_expression: SqlExpressionKind::Static,
        nominal_types: resolve_nominal_types(receiver_nominal_types_in_type(ty), symbols),
        payload_variants: payload_variants_in_type(ty, symbols),
        tuple_items: tuple_items_in_type(ty, symbols),
        trait_bounds: trait_bounds_in_type(ty, symbols),
    };
    if let Type::Path(path) = ty
        && let Some(alias) = path.path.segments.last().and_then(|segment| {
            symbols
                .type_alias_info
                .get(&normalized_ident(&segment.ident))
        })
    {
        info.union(alias);
    }
    info
}

fn payload_variants_in_path(
    path: &syn::Path,
    symbols: &ModuleSymbols,
) -> BTreeSet<Vec<NominalShape>> {
    path.segments
        .last()
        .and_then(|segment| match &segment.arguments {
            syn::PathArguments::AngleBracketed(arguments) => {
                let arguments = arguments
                    .args
                    .iter()
                    .filter_map(|argument| match argument {
                        syn::GenericArgument::Type(inner) => nominal_shape_in_type(inner, symbols),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                (!arguments.is_empty()).then_some(BTreeSet::from([arguments]))
            }
            _ => None,
        })
        .unwrap_or_default()
}

fn resolve_nominal_types(types: BTreeSet<String>, symbols: &ModuleSymbols) -> BTreeSet<String> {
    types
        .into_iter()
        .flat_map(|nominal| {
            symbols
                .nominal_type_aliases
                .get(&nominal)
                .cloned()
                .unwrap_or_else(|| BTreeSet::from([nominal]))
        })
        .collect()
}

fn canonical_path_names(mut names: Vec<String>, symbols: &ModuleSymbols) -> Vec<String> {
    let mut absolute = false;
    let mut base = symbols.module_path.clone();
    match names.first().map(String::as_str) {
        Some("crate") => {
            names.remove(0);
            base.clear();
            absolute = true;
        }
        Some("self") => {
            names.remove(0);
            absolute = true;
        }
        Some("super") => {
            while names.first().is_some_and(|name| name == "super") {
                names.remove(0);
                base.pop();
            }
            absolute = true;
        }
        _ => {}
    }
    let mut seen = BTreeSet::new();
    while let Some(first) = names.first().cloned() {
        if !seen.insert(first.clone()) {
            break;
        }
        let Some(source) = symbols.path_aliases.get(&first) else {
            break;
        };
        if source.len() == 1 && source[0] == first {
            break;
        }
        let mut expanded = source.clone();
        expanded.extend(names.into_iter().skip(1));
        names = expanded;
        base.clear();
        absolute = true;
    }
    if !absolute {
        base.extend(names);
        base
    } else {
        base.extend(names);
        base
    }
}

fn canonical_trait_path(path: &syn::Path, symbols: &ModuleSymbols) -> String {
    canonical_path_names(path_names(path), symbols).join("::")
}

fn canonical_trait_path_in_module(
    path: &syn::Path,
    module_path: &[String],
    symbols: &ModuleSymbols,
) -> String {
    let mut scoped = symbols.clone();
    scoped.module_path = module_path.to_vec();
    canonical_trait_path(path, &scoped)
}

fn record_trait_supertraits(
    trait_path: &str,
    bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::token::Plus>,
    module_path: &[String],
    symbols: &mut ModuleSymbols,
) {
    let supertraits = bounds
        .iter()
        .filter_map(|bound| match bound {
            syn::TypeParamBound::Trait(bound) => Some(canonical_trait_path_in_module(
                &bound.path,
                module_path,
                symbols,
            )),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if !supertraits.is_empty() {
        std::sync::Arc::make_mut(&mut symbols.trait_supertraits)
            .entry(trait_path.to_owned())
            .or_default()
            .extend(supertraits);
    }
}

fn trait_bounds_in_type(ty: &Type, symbols: &ModuleSymbols) -> BTreeSet<String> {
    let bounds = match ty {
        Type::TraitObject(object) => Some(&object.bounds),
        Type::ImplTrait(object) => Some(&object.bounds),
        Type::Reference(reference) => return trait_bounds_in_type(&reference.elem, symbols),
        Type::Ptr(pointer) => return trait_bounds_in_type(&pointer.elem, symbols),
        Type::Paren(paren) => return trait_bounds_in_type(&paren.elem, symbols),
        Type::Group(group) => return trait_bounds_in_type(&group.elem, symbols),
        _ => None,
    };
    bounds
        .into_iter()
        .flatten()
        .filter_map(|bound| match bound {
            syn::TypeParamBound::Trait(bound) => Some(canonical_trait_path(&bound.path, symbols)),
            _ => None,
        })
        .collect()
}

#[derive(Clone, Debug)]
struct UseLeaf {
    source: Vec<String>,
    local: String,
    fingerprint: String,
    namespace_self: bool,
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
            let namespace_self = name == "self";
            let mut source = prefix.clone();
            let local = if namespace_self {
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
                namespace_self,
            });
        }
        UseTree::Rename(rename) => {
            let source_name = normalized_ident(&rename.ident);
            let local = normalized_ident(&rename.rename);
            let mut source = prefix.clone();
            let namespace_self = source_name == "self";
            if !namespace_self {
                source.push(source_name.clone());
            }
            leaves.push(UseLeaf {
                fingerprint: if namespace_self {
                    format!("{}::self as {local}", source.join("::"))
                } else {
                    format!("{} as {local}", source.join("::"))
                },
                source,
                local,
                namespace_self,
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
    let mut targets = targets_for_names(&leaf.source, symbols);
    if leaf.namespace_self {
        if source_is_sqlx(&leaf.source, symbols) {
            targets.insert(PersistenceTarget::Sqlx);
        }
        if source_is_database(&leaf.source, symbols) {
            targets.insert(PersistenceTarget::Database);
        }
    }
    // `use` paths resolve the imported symbol at the leaf, unlike expression
    // paths where only the root can be an in-scope type alias. The adapter's
    // own `crate`/`self`/`super` re-exports therefore need leaf resolution,
    // while consumer crates must still avoid same-named local imports.
    if symbols.database_namespaces.contains("crate") {
        if let Some(last) = leaf.source.last() {
            if let Some(alias_targets) = symbols.type_aliases.get(last) {
                targets.extend(alias_targets);
            }
        }
    }
    targets
}

fn apply_import_symbols(item_use: &ItemUse, symbols: &mut ModuleSymbols) -> bool {
    let (leaves, _) = use_leaves(item_use);
    let mut changed = false;
    for leaf in leaves {
        let canonical_source = match leaf.source.first() {
            // `use foo::foo` binds the leaf, not a recursively expanding
            // alias for its own root. Preserve the external/root path.
            Some(first) if leaf.source.len() > 1 && first == &leaf.local => leaf.source.clone(),
            Some(first) if leaf.source.len() > 1 => symbols
                .path_aliases
                .get(first)
                .map(|mapped| {
                    let mut source = mapped.clone();
                    source.extend(leaf.source.iter().skip(1).cloned());
                    source
                })
                .unwrap_or_else(|| canonical_path_names(leaf.source.clone(), symbols)),
            _ => canonical_path_names(leaf.source.clone(), symbols),
        };
        if leaf.local != "_"
            && !leaf.namespace_self
            && symbols.path_aliases.get(&leaf.local) != Some(&canonical_source)
        {
            symbols
                .path_aliases
                .insert(leaf.local.clone(), canonical_source.clone());
            changed = true;
        }
        let canonical_trait = canonical_source.join("::");
        if leaf.namespace_self {
            // A namespace-self import puts the module in scope; it does not
            // import a trait under that local module name.
        } else if leaf.local == "_" {
            changed |= symbols.anonymous_traits_in_scope.insert(canonical_trait);
        } else if symbols.traits_in_scope.get(&leaf.local) != Some(&canonical_trait) {
            symbols
                .traits_in_scope
                .insert(leaf.local.clone(), canonical_trait);
            changed = true;
        }
        let source_is_sqlx = source_is_sqlx(&leaf.source, symbols);
        let source_is_database = source_is_database(&leaf.source, symbols);
        if (leaf.namespace_self || leaf.source.len() == 1) && source_is_sqlx {
            changed |= symbols.sqlx_namespaces.insert(leaf.local.clone());
        }
        if (leaf.namespace_self || leaf.source.len() == 1) && source_is_database {
            changed |= symbols.database_namespaces.insert(leaf.local.clone());
        }
        let imported_targets = targets_for_use_leaf(&leaf, symbols);
        if !imported_targets.is_empty() {
            let entry = symbols.type_aliases.entry(leaf.local.clone()).or_default();
            let before = entry.len();
            entry.extend(imported_targets);
            changed |= entry.len() != before;
        }
        if source_is_sqlx && leaf.source.last().is_some_and(|name| is_query_name(name)) {
            changed |= symbols.query_callables.insert(leaf.local);
        }
    }
    changed
}

fn collect_nested_trait_returns(
    items: &[Item],
    module_path: &[String],
    cfg: &[String],
    source_class: PersistenceSourceClass,
    symbols: &mut ModuleSymbols,
    errors: &mut Vec<String>,
) {
    for item in items {
        match item {
            Item::Trait(item_trait)
                if source_class_allows(source_class, cfg, &item_trait.attrs, errors, "trait") =>
            {
                let mut trait_path = module_path.to_vec();
                trait_path.push(normalized_ident(&item_trait.ident));
                let trait_path = trait_path.join("::");
                record_trait_supertraits(
                    &trait_path,
                    &item_trait.supertraits,
                    module_path,
                    symbols,
                );
                for item in &item_trait.items {
                    let syn::TraitItem::Fn(method) = item else {
                        continue;
                    };
                    if !source_class_allows(
                        source_class,
                        cfg,
                        &method.attrs,
                        errors,
                        "trait method",
                    ) {
                        continue;
                    }
                    if let ReturnType::Type(_, ty) = &method.sig.output {
                        let mut info = variable_info_in_type(ty, symbols);
                        info.sql_expression = SqlExpressionKind::Nonliteral;
                        if !info.flow.is_empty()
                            || !info.nominal_types.is_empty()
                            || !info.payload_variants.is_empty()
                            || !info.tuple_items.is_empty()
                            || !info.trait_bounds.is_empty()
                        {
                            std::sync::Arc::make_mut(&mut symbols.trait_method_returns).insert(
                                (trait_path.clone(), normalized_ident(&method.sig.ident)),
                                info,
                            );
                        }
                    }
                }
            }
            Item::Mod(item_mod)
                if source_class_allows(
                    source_class,
                    cfg,
                    &item_mod.attrs,
                    errors,
                    "inline module",
                ) && item_mod.content.is_some() =>
            {
                let mut child_path = module_path.to_vec();
                child_path.push(normalized_ident(&item_mod.ident));
                let child_cfg = item_cfg(cfg, &item_mod.attrs);
                collect_nested_trait_returns(
                    &item_mod.content.as_ref().expect("checked content").1,
                    &child_path,
                    &child_cfg,
                    source_class,
                    symbols,
                    errors,
                );
            }
            _ => {}
        }
    }
}

fn collect_module_symbols(
    items: &[Item],
    parent: Option<&ModuleSymbols>,
    package: &str,
    module: &str,
    cfg: &[String],
    source_class: PersistenceSourceClass,
    errors: &mut Vec<String>,
) -> ModuleSymbols {
    let is_root_collection = parent.is_none();
    let mut symbols = parent
        .cloned()
        .unwrap_or_else(|| ModuleSymbols::for_package(package));
    symbols.module_path = module
        .split("::")
        .filter(|segment| *segment != "crate")
        .map(str::to_owned)
        .collect();
    symbols.traits_in_scope.clear();
    symbols.anonymous_traits_in_scope.clear();
    for _ in 0..=items.len() {
        let mut changed = false;
        for item in items {
            match item {
                Item::Trait(item_trait)
                    if source_class_allows(
                        source_class,
                        cfg,
                        &item_trait.attrs,
                        errors,
                        "trait",
                    ) =>
                {
                    let local = normalized_ident(&item_trait.ident);
                    let mut path = symbols.module_path.clone();
                    path.push(local.clone());
                    let canonical = path.join("::");
                    if symbols.traits_in_scope.get(&local) != Some(&canonical) {
                        symbols.traits_in_scope.insert(local, canonical);
                        changed = true;
                    }
                }
                Item::Use(item_use)
                    if source_class_allows(
                        source_class,
                        cfg,
                        &item_use.attrs,
                        errors,
                        "use declaration",
                    ) =>
                {
                    changed |= apply_import_symbols(item_use, &mut symbols);
                }
                Item::ExternCrate(extern_crate)
                    if source_class_allows(
                        source_class,
                        cfg,
                        &extern_crate.attrs,
                        errors,
                        "extern crate",
                    ) =>
                {
                    let source = normalized_ident(&extern_crate.ident);
                    let local = extern_crate
                        .rename
                        .as_ref()
                        .map(|(_, rename)| normalized_ident(rename))
                        .unwrap_or_else(|| source.clone());
                    if symbols.sqlx_namespaces.contains(&source) {
                        changed |= symbols.sqlx_namespaces.insert(local);
                    } else if source == "wow_database" {
                        changed |= symbols.database_namespaces.insert(local);
                    }
                }
                Item::Type(alias)
                    if source_class_allows(
                        source_class,
                        cfg,
                        &alias.attrs,
                        errors,
                        "type alias",
                    ) =>
                {
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
                    let nominal_targets = receiver_nominal_types_in_type(&alias.ty);
                    if !nominal_targets.is_empty() {
                        let resolved_targets = nominal_targets
                            .into_iter()
                            .flat_map(|nominal| {
                                symbols
                                    .nominal_type_aliases
                                    .get(&nominal)
                                    .cloned()
                                    .unwrap_or_else(|| BTreeSet::from([nominal]))
                            })
                            .collect::<BTreeSet<_>>();
                        let entry = symbols
                            .nominal_type_aliases
                            .entry(normalized_ident(&alias.ident))
                            .or_default();
                        let before = entry.len();
                        entry.extend(resolved_targets);
                        changed |= entry.len() != before;
                    }
                    let alias_info = variable_info_in_type(&alias.ty, &symbols);
                    let entry = symbols
                        .type_alias_info
                        .entry(normalized_ident(&alias.ident))
                        .or_default();
                    let before = entry.clone();
                    entry.union(&alias_info);
                    changed |= *entry != before;
                }
                _ => {}
            }
        }
        if !changed {
            break;
        }
    }

    if is_root_collection {
        collect_nested_trait_returns(
            items,
            &symbols.module_path.clone(),
            cfg,
            source_class,
            &mut symbols,
            errors,
        );
    }

    for item in items {
        match item {
            Item::Struct(item_struct)
                if source_class_allows(source_class, cfg, &item_struct.attrs, errors, "struct") =>
            {
                for (index, field) in item_struct.fields.iter().enumerate() {
                    if !source_class_allows(source_class, cfg, &field.attrs, errors, "struct field")
                    {
                        continue;
                    }
                    let targets = targets_in_type(&field.ty, &symbols);
                    let field_name = field
                        .ident
                        .as_ref()
                        .map(normalized_ident)
                        .unwrap_or_else(|| index.to_string());
                    symbols
                        .field_owners
                        .entry(field_name.clone())
                        .or_default()
                        .insert(normalized_ident(&item_struct.ident));
                    let nominal_types = receiver_nominal_types_in_type(&field.ty)
                        .into_iter()
                        .flat_map(|nominal| {
                            symbols
                                .nominal_type_aliases
                                .get(&nominal)
                                .cloned()
                                .unwrap_or_else(|| BTreeSet::from([nominal]))
                        })
                        .collect::<BTreeSet<_>>();
                    if !nominal_types.is_empty() {
                        symbols
                            .field_nominal_types
                            .entry((normalized_ident(&item_struct.ident), field_name))
                            .or_default()
                            .extend(nominal_types);
                    }
                    if targets.is_empty() {
                        continue;
                    }
                    if let Some(ident) = &field.ident {
                        symbols
                            .field_targets
                            .entry((
                                normalized_ident(&item_struct.ident),
                                normalized_ident(ident),
                            ))
                            .or_default()
                            .extend(targets);
                    } else {
                        symbols
                            .tuple_field_targets
                            .entry((normalized_ident(&item_struct.ident), index.to_string()))
                            .or_default()
                            .extend(targets);
                    }
                }
            }
            Item::Enum(item_enum)
                if source_class_allows(source_class, cfg, &item_enum.attrs, errors, "enum") =>
            {
                for variant in &item_enum.variants {
                    if !source_class_allows(
                        source_class,
                        cfg,
                        &variant.attrs,
                        errors,
                        "enum variant",
                    ) {
                        continue;
                    }
                    for (index, field) in variant.fields.iter().enumerate() {
                        if !source_class_allows(
                            source_class,
                            cfg,
                            &field.attrs,
                            errors,
                            "enum field",
                        ) {
                            continue;
                        }
                        let targets = targets_in_type(&field.ty, &symbols);
                        let variant_owner = format!(
                            "{}::{}",
                            normalized_ident(&item_enum.ident),
                            normalized_ident(&variant.ident)
                        );
                        let field_name = field
                            .ident
                            .as_ref()
                            .map(normalized_ident)
                            .unwrap_or_else(|| index.to_string());
                        symbols
                            .field_owners
                            .entry(field_name.clone())
                            .or_default()
                            .insert(variant_owner.clone());
                        let nominal_types = receiver_nominal_types_in_type(&field.ty)
                            .into_iter()
                            .flat_map(|nominal| {
                                symbols
                                    .nominal_type_aliases
                                    .get(&nominal)
                                    .cloned()
                                    .unwrap_or_else(|| BTreeSet::from([nominal]))
                            })
                            .collect::<BTreeSet<_>>();
                        if !nominal_types.is_empty() {
                            symbols
                                .field_nominal_types
                                .entry((variant_owner.clone(), field_name))
                                .or_default()
                                .extend(nominal_types);
                        }
                        if targets.is_empty() {
                            continue;
                        }
                        if let Some(ident) = &field.ident {
                            symbols
                                .field_targets
                                .entry((variant_owner, normalized_ident(ident)))
                                .or_default()
                                .extend(targets);
                        } else {
                            symbols
                                .tuple_field_targets
                                .entry((variant_owner, index.to_string()))
                                .or_default()
                                .extend(targets);
                        }
                    }
                }
            }
            Item::Fn(function)
                if source_class_allows(source_class, cfg, &function.attrs, errors, "function") =>
            {
                if let ReturnType::Type(_, ty) = &function.sig.output {
                    let mut return_info = variable_info_in_type(ty, &symbols);
                    return_info.sql_expression = SqlExpressionKind::Nonliteral;
                    if !return_info.flow.is_empty()
                        || !return_info.nominal_types.is_empty()
                        || !return_info.payload_variants.is_empty()
                        || !return_info.tuple_items.is_empty()
                    {
                        symbols
                            .function_returns
                            .insert(normalized_ident(&function.sig.ident), return_info);
                    }
                }
            }
            Item::Impl(item_impl)
                if source_class_allows(source_class, cfg, &item_impl.attrs, errors, "impl") =>
            {
                let trait_name = item_impl
                    .trait_
                    .as_ref()
                    .map(|(_, path, _)| canonical_trait_path(path, &symbols));
                let receiver_types = nominal_types_in_type(&item_impl.self_ty)
                    .into_iter()
                    .flat_map(|nominal| {
                        symbols
                            .nominal_type_aliases
                            .get(&nominal)
                            .cloned()
                            .unwrap_or_else(|| BTreeSet::from([nominal]))
                    })
                    .collect::<BTreeSet<_>>();
                for item in &item_impl.items {
                    let ImplItem::Fn(method) = item else {
                        continue;
                    };
                    if !source_class_allows(source_class, cfg, &method.attrs, errors, "impl method")
                    {
                        continue;
                    }
                    if let ReturnType::Type(_, ty) = &method.sig.output {
                        let mut return_info = variable_info_in_type(ty, &symbols);
                        return_info.sql_expression = SqlExpressionKind::Nonliteral;
                        if !return_info.flow.is_empty()
                            || !return_info.nominal_types.is_empty()
                            || !return_info.payload_variants.is_empty()
                            || !return_info.tuple_items.is_empty()
                        {
                            let method_name = normalized_ident(&method.sig.ident);
                            for receiver_type in &receiver_types {
                                let info = symbols
                                    .method_returns
                                    .entry((
                                        receiver_type.clone(),
                                        trait_name.clone(),
                                        method_name.clone(),
                                    ))
                                    .or_default();
                                info.union(&return_info);
                            }
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
                generated_input: false,
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
                generated_input: false,
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

fn tokens_contain_path_root(tokens: TokenStream, names: &BTreeSet<String>) -> bool {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    for window in tokens.windows(3) {
        if let [
            TokenTree::Ident(ident),
            TokenTree::Punct(first),
            TokenTree::Punct(second),
        ] = window
            && names.contains(&normalized_ident(ident))
            && first.as_char() == ':'
            && second.as_char() == ':'
        {
            return true;
        }
    }
    tokens.into_iter().any(|token| match token {
        TokenTree::Group(group) => tokens_contain_path_root(group.stream(), names),
        _ => false,
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
    if tokens_contain_path_root(tokens.clone(), &symbols.sqlx_namespaces) {
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
                    generated_input: true,
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
    local_path_alias_scopes: Vec<BTreeMap<String, Vec<String>>>,
    anonymous_trait_scopes: Vec<BTreeSet<String>>,
    generic_trait_bounds: BTreeMap<String, BTreeSet<String>>,
}

struct DirectChildFlowCollector<'analyzer, 'a, 'b> {
    analyzer: &'analyzer BodyAnalyzer<'a, 'b>,
    flow: Flow,
    at_root: bool,
}

impl<'ast> Visit<'ast> for DirectChildFlowCollector<'_, '_, '_> {
    fn visit_expr(&mut self, expression: &'ast Expr) {
        if self.at_root {
            self.at_root = false;
            visit::visit_expr(self, expression);
        } else {
            self.flow.union(self.analyzer.flow_of_expr(expression));
            visit::visit_expr(self, expression);
        }
    }
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
            local_path_alias_scopes: vec![BTreeMap::new()],
            anonymous_trait_scopes: vec![BTreeSet::new()],
            generic_trait_bounds: BTreeMap::new(),
        }
    }

    fn canonical_local_path_names(&self, names: Vec<String>) -> Vec<String> {
        if let Some(first) = names.first().cloned()
            && let Some(source) = self
                .local_path_alias_scopes
                .iter()
                .rev()
                .find_map(|scope| scope.get(&first))
        {
            let mut expanded = source.clone();
            expanded.extend(names.into_iter().skip(1));
            return expanded;
        }
        canonical_path_names(names, self.symbols)
    }

    fn register_local_uses(&mut self, statements: &[Stmt]) {
        let uses = statements.iter().filter_map(|statement| match statement {
            Stmt::Item(Item::Use(item_use)) => Some(item_use),
            _ => None,
        });
        for item_use in uses {
            if !source_class_allows(
                self.context.source_class,
                &self.cfg,
                &item_use.attrs,
                self.errors,
                "local use declaration",
            ) {
                continue;
            }
            let (leaves, _) = use_leaves(item_use);
            for leaf in leaves {
                let source = self.canonical_local_path_names(leaf.source);
                if leaf.local == "_" {
                    self.anonymous_trait_scopes
                        .last_mut()
                        .expect("body analyzer has a lexical scope")
                        .insert(source.join("::"));
                } else {
                    self.local_path_alias_scopes
                        .last_mut()
                        .expect("body analyzer has a lexical scope")
                        .insert(leaf.local, source);
                }
            }
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(BTreeMap::new());
        self.local_path_alias_scopes.push(BTreeMap::new());
        self.anonymous_trait_scopes.push(BTreeSet::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
        self.local_path_alias_scopes.pop();
        self.anonymous_trait_scopes.pop();
    }

    fn trait_is_in_scope(&self, trait_name: &str) -> bool {
        if self.symbols.anonymous_traits_in_scope.contains(trait_name)
            || self
                .anonymous_trait_scopes
                .iter()
                .any(|scope| scope.contains(trait_name))
        {
            return true;
        }
        let mut shadowed = BTreeSet::new();
        for scope in self.local_path_alias_scopes.iter().rev() {
            for (local, path) in scope {
                if shadowed.insert(local) && path.join("::") == trait_name {
                    return true;
                }
            }
        }
        self.symbols
            .traits_in_scope
            .iter()
            .any(|(local, path)| !shadowed.contains(local) && path == trait_name)
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
                generated_input: false,
            },
        );
    }

    fn add_generated(
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
                generated_input: true,
            },
        );
    }

    fn allows_source_class(&mut self, attributes: &[Attribute], owner: &str) -> bool {
        let allowed = source_class_allows(
            self.context.source_class,
            &self.cfg,
            attributes,
            self.errors,
            owner,
        );
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
        let mut info = variable_info_in_type(ty, self.symbols);
        for name in info.nominal_types.clone() {
            if let Some(bounds) = self.generic_trait_bounds.get(&name) {
                info.trait_bounds.extend(bounds.iter().cloned());
            }
        }
        info
    }

    fn method_return_info(&self, method: &ExprMethodCall) -> VariableInfo {
        let method_name = normalized_ident(&method.method);
        let mut result = VariableInfo::default();
        let receiver_types = self.nominal_types_of_expr(&method.receiver);
        let mut trait_bounds = BTreeSet::new();
        for receiver_type in &receiver_types {
            if let Some(bounds) = self.generic_trait_bounds.get(receiver_type) {
                trait_bounds.extend(bounds.iter().cloned());
            }
        }
        if receiver_types.is_empty() {
            trait_bounds.extend(self.shallow_trait_bounds_of_expr(&method.receiver));
        }
        for receiver_type in receiver_types {
            if let Some(info) =
                self.symbols
                    .method_returns
                    .get(&(receiver_type.clone(), None, method_name.clone()))
            {
                result.union(info);
                continue;
            }
            for ((owner, trait_name, candidate), info) in &self.symbols.method_returns {
                if owner == &receiver_type
                    && trait_name
                        .as_ref()
                        .is_some_and(|trait_name| self.trait_is_in_scope(trait_name))
                    && candidate == &method_name
                {
                    result.union(info);
                }
            }
        }
        let mut pending = trait_bounds.into_iter().collect::<Vec<_>>();
        let mut expanded_trait_bounds = BTreeSet::new();
        while let Some(trait_bound) = pending.pop() {
            if !expanded_trait_bounds.insert(trait_bound.clone()) {
                continue;
            }
            if let Some(supertraits) = self.symbols.trait_supertraits.get(&trait_bound) {
                pending.extend(supertraits.iter().cloned());
            }
        }
        for trait_bound in &expanded_trait_bounds {
            if let Some(info) = self
                .symbols
                .trait_method_returns
                .get(&(trait_bound.clone(), method_name.clone()))
            {
                result.union(info);
            }
        }
        if method_name == "recv" {
            result
                .payload_variants
                .extend(self.info_from_expr(&method.receiver).payload_variants);
        }
        result
    }

    fn shallow_trait_bounds_of_expr(&self, expression: &Expr) -> BTreeSet<String> {
        match expression {
            Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
                last_path_name(&path.path)
                    .and_then(|name| self.lookup(&name))
                    .map(|info| info.trait_bounds.clone())
                    .unwrap_or_default()
            }
            Expr::Reference(reference) => self.shallow_trait_bounds_of_expr(&reference.expr),
            Expr::Paren(paren) => self.shallow_trait_bounds_of_expr(&paren.expr),
            Expr::Group(group) => self.shallow_trait_bounds_of_expr(&group.expr),
            Expr::Try(try_expression) => self.shallow_trait_bounds_of_expr(&try_expression.expr),
            Expr::Await(await_expression) => {
                self.shallow_trait_bounds_of_expr(&await_expression.base)
            }
            Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                self.shallow_trait_bounds_of_expr(&unary.expr)
            }
            _ => BTreeSet::new(),
        }
    }

    fn associated_return_info(&self, expression: &syn::ExprPath) -> VariableInfo {
        let path = &expression.path;
        if expression.qself.is_none() && path.segments.len() < 2 {
            return VariableInfo::default();
        }
        let method_name = normalized_ident(&path.segments.last().expect("path has a method").ident);
        let trait_name = expression.qself.as_ref().and_then(|qself| {
            (qself.position > 0).then(|| {
                self.canonical_local_path_names(
                    path.segments
                        .iter()
                        .take(qself.position)
                        .map(|segment| normalized_ident(&segment.ident))
                        .collect(),
                )
                .join("::")
            })
        });
        let receiver_types = if let Some(qself) = &expression.qself {
            receiver_nominal_types_in_type(&qself.ty)
                .into_iter()
                .flat_map(|owner| {
                    self.symbols
                        .nominal_type_aliases
                        .get(&owner)
                        .cloned()
                        .unwrap_or_else(|| BTreeSet::from([owner]))
                })
                .collect()
        } else {
            let owner = normalized_ident(
                &path
                    .segments
                    .iter()
                    .nth_back(1)
                    .expect("path has an owner")
                    .ident,
            );
            if owner == "Self" {
                self.lookup("Self")
                    .map(|info| info.nominal_types.clone())
                    .unwrap_or_default()
            } else {
                self.symbols
                    .nominal_type_aliases
                    .get(&owner)
                    .cloned()
                    .unwrap_or_else(|| BTreeSet::from([owner]))
            }
        };
        let mut result = VariableInfo::default();
        for receiver_type in receiver_types {
            if let Some(info) = self.symbols.method_returns.get(&(
                receiver_type,
                trait_name.clone(),
                method_name.clone(),
            )) {
                result.union(info);
            }
        }
        result
    }

    fn info_from_expr(&self, expression: &Expr) -> VariableInfo {
        match expression {
            Expr::Reference(reference) => return self.info_from_expr(&reference.expr),
            Expr::Paren(paren) => return self.info_from_expr(&paren.expr),
            Expr::Group(group) => return self.info_from_expr(&group.expr),
            Expr::Try(try_expression) => return self.info_from_expr(&try_expression.expr),
            Expr::Await(await_expression) => return self.info_from_expr(&await_expression.base),
            _ => {}
        }
        if let Expr::Path(path) = expression
            && path.qself.is_none()
            && path.path.segments.len() == 1
            && let Some(name) = last_path_name(&path.path)
            && let Some(info) = self.lookup(&name)
        {
            return info.clone();
        }
        if let Expr::Call(call) = expression
            && let Expr::Path(path) = call.func.as_ref()
        {
            if path.path.segments.len() == 1
                && let Some(name) = last_path_name(&path.path)
                && let Some(info) = self.symbols.function_returns.get(&name)
            {
                return info.clone();
            }
            let return_info = self.associated_return_info(path);
            if !return_info.flow.is_empty()
                || !return_info.nominal_types.is_empty()
                || !return_info.payload_variants.is_empty()
                || !return_info.tuple_items.is_empty()
            {
                return return_info;
            }
            if let Some(variant) = last_path_name(&path.path)
                && matches!(variant.as_str(), "Some" | "Ok" | "Err")
            {
                let arguments = call
                    .args
                    .iter()
                    .map(|argument| self.nominal_shape_from_expr(argument))
                    .collect::<Vec<_>>();
                let mut info = VariableInfo {
                    flow: self.flow_of_expr(expression),
                    sql_expression: self.sql_expression_kind(expression),
                    nominal_types: BTreeSet::from([variant]),
                    ..VariableInfo::default()
                };
                if arguments.iter().any(Option::is_some) {
                    info.payload_variants.insert(
                        arguments
                            .into_iter()
                            .map(|shape| {
                                shape.unwrap_or_else(|| NominalShape {
                                    nominal_types: BTreeSet::new(),
                                    arguments: Vec::new(),
                                })
                            })
                            .collect(),
                    );
                }
                return info;
            }
        }
        if let Expr::MethodCall(method) = expression {
            let return_info = self.method_return_info(method);
            return VariableInfo {
                flow: self.flow_of_expr(expression),
                sql_expression: self.sql_expression_kind(expression),
                nominal_types: return_info.nominal_types,
                payload_variants: return_info.payload_variants,
                tuple_items: return_info.tuple_items,
                trait_bounds: return_info.trait_bounds,
            };
        }
        if let Expr::Call(call) = expression
            && let Expr::Path(path) = call.func.as_ref()
            && let Some(owner) = last_path_name(&path.path)
        {
            let nominal_types = if path.qself.is_none()
                && path.path.segments.len() >= 2
                && matches!(owner.as_str(), "default" | "new")
            {
                path.path
                    .segments
                    .iter()
                    .nth_back(1)
                    .map(|segment| BTreeSet::from([normalized_ident(&segment.ident)]))
                    .unwrap_or_default()
            } else {
                BTreeSet::from([owner])
            };
            return VariableInfo {
                flow: self.flow_of_expr(expression),
                sql_expression: self.sql_expression_kind(expression),
                nominal_types,
                payload_variants: payload_variants_in_path(&path.path, self.symbols),
                tuple_items: Vec::new(),
                trait_bounds: BTreeSet::new(),
            };
        }
        if let Expr::Tuple(tuple) = expression {
            return VariableInfo {
                flow: self.flow_of_expr(expression),
                sql_expression: self.sql_expression_kind(expression),
                tuple_items: tuple
                    .elems
                    .iter()
                    .map(|element| self.info_from_expr(element))
                    .collect(),
                ..VariableInfo::default()
            };
        }
        VariableInfo {
            flow: self.flow_of_expr(expression),
            sql_expression: self.sql_expression_kind(expression),
            nominal_types: BTreeSet::new(),
            payload_variants: BTreeSet::new(),
            tuple_items: Vec::new(),
            trait_bounds: BTreeSet::new(),
        }
    }

    fn nominal_shape_from_expr(&self, expression: &Expr) -> Option<NominalShape> {
        if let Expr::Tuple(tuple) = expression {
            return Some(NominalShape {
                nominal_types: BTreeSet::new(),
                arguments: tuple
                    .elems
                    .iter()
                    .map(|element| {
                        self.nominal_shape_from_expr(element)
                            .unwrap_or_else(|| NominalShape {
                                nominal_types: BTreeSet::new(),
                                arguments: Vec::new(),
                            })
                    })
                    .collect(),
            });
        }
        let info = self.info_from_expr(expression);
        (!info.nominal_types.is_empty()).then(|| NominalShape {
            nominal_types: info.nominal_types,
            arguments: info.payload_variants.into_iter().next().unwrap_or_default(),
        })
    }

    fn nominal_types_of_expr(&self, expression: &Expr) -> BTreeSet<String> {
        match expression {
            Expr::Path(path) if path.qself.is_none() => last_path_name(&path.path)
                .map(|name| {
                    self.lookup(&name)
                        .map(|info| info.nominal_types.clone())
                        .or_else(|| self.symbols.nominal_type_aliases.get(&name).cloned())
                        .unwrap_or_else(|| {
                            let known_owner = self
                                .symbols
                                .method_returns
                                .keys()
                                .any(|(owner, _, _)| owner == &name)
                                || self
                                    .symbols
                                    .tuple_field_targets
                                    .keys()
                                    .any(|(owner, _)| owner == &name);
                            known_owner
                                .then(|| BTreeSet::from([name]))
                                .unwrap_or_default()
                        })
                })
                .unwrap_or_default(),
            Expr::Reference(reference) => self.nominal_types_of_expr(&reference.expr),
            Expr::Paren(paren) => self.nominal_types_of_expr(&paren.expr),
            Expr::Group(group) => self.nominal_types_of_expr(&group.expr),
            Expr::Try(try_expression) => self.nominal_types_of_expr(&try_expression.expr),
            Expr::Await(await_expression) => self.nominal_types_of_expr(&await_expression.base),
            Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                self.nominal_types_of_expr(&unary.expr)
            }
            Expr::Field(field) => {
                let field_name = match &field.member {
                    Member::Named(ident) => normalized_ident(ident),
                    Member::Unnamed(index) => index.index.to_string(),
                };
                let mut nominal_types = BTreeSet::new();
                for owner in self.nominal_types_of_expr(&field.base) {
                    if let Some(field_types) = self
                        .symbols
                        .field_nominal_types
                        .get(&(owner, field_name.clone()))
                    {
                        nominal_types.extend(field_types.iter().cloned());
                    }
                }
                nominal_types
            }
            Expr::Call(call) => match call.func.as_ref() {
                Expr::Path(path) if path.path.segments.len() == 1 => last_path_name(&path.path)
                    .map(|name| {
                        self.symbols
                            .function_returns
                            .get(&name)
                            .map(|info| info.nominal_types.clone())
                            .unwrap_or_else(|| BTreeSet::from([name]))
                    })
                    .unwrap_or_default(),
                Expr::Path(path) => self.associated_return_info(path).nominal_types,
                _ => BTreeSet::new(),
            },
            Expr::MethodCall(method) => self.method_return_info(method).nominal_types,
            Expr::Struct(expression) => last_path_name(&expression.path)
                .map(|name| {
                    self.symbols
                        .nominal_type_aliases
                        .get(&name)
                        .cloned()
                        .unwrap_or_else(|| BTreeSet::from([name]))
                })
                .unwrap_or_default(),
            _ => BTreeSet::new(),
        }
    }

    fn flow_in_block(&self, block: &syn::Block) -> Flow {
        let mut collector = DirectChildFlowCollector {
            analyzer: self,
            flow: Flow::default(),
            at_root: false,
        };
        collector.visit_block(block);
        collector.flow
    }

    fn declared_field_info(&self, owner: &str, field: &str, named: bool) -> VariableInfo {
        let owners = self
            .symbols
            .nominal_type_aliases
            .get(owner)
            .cloned()
            .unwrap_or_else(|| BTreeSet::from([owner.to_owned()]));
        let mut info = VariableInfo::default();
        for owner in owners {
            let targets = if named {
                self.symbols
                    .field_targets
                    .get(&(owner.clone(), field.to_owned()))
            } else {
                self.symbols
                    .tuple_field_targets
                    .get(&(owner.clone(), field.to_owned()))
            };
            if let Some(targets) = targets {
                info.flow.union(Flow::pools(targets));
            }
            if let Some(types) = self
                .symbols
                .field_nominal_types
                .get(&(owner, field.to_owned()))
            {
                info.nominal_types.extend(types.iter().cloned());
            }
        }
        info
    }

    fn has_declared_fields(&self, owner: &str) -> bool {
        let owners = self
            .symbols
            .nominal_type_aliases
            .get(owner)
            .cloned()
            .unwrap_or_else(|| BTreeSet::from([owner.to_owned()]));
        owners.iter().any(|owner| {
            self.symbols
                .field_targets
                .keys()
                .any(|(field_owner, _)| field_owner == owner)
                || self
                    .symbols
                    .tuple_field_targets
                    .keys()
                    .any(|(field_owner, _)| field_owner == owner)
                || self
                    .symbols
                    .field_nominal_types
                    .keys()
                    .any(|(field_owner, _)| field_owner == owner)
        })
    }

    fn pattern_owner(&self, path: &syn::Path) -> String {
        let names = path_names(path);
        if names.len() < 2 {
            return names.last().cloned().unwrap_or_default();
        }
        let variant = names.last().cloned().unwrap_or_default();
        let owner = names.get(names.len() - 2).cloned().unwrap_or_default();
        let owner = self
            .symbols
            .nominal_type_aliases
            .get(&owner)
            .and_then(|owners| {
                (owners.len() == 1)
                    .then(|| owners.iter().next().cloned())
                    .flatten()
            })
            .unwrap_or(owner);
        format!("{owner}::{variant}")
    }

    fn wrapper_payload_info(&self, owner: &str, info: &VariableInfo) -> Option<VariableInfo> {
        let argument_index = match owner.rsplit("::").next().unwrap_or(owner) {
            "Some" | "Ok" => 0,
            "Err" => 1,
            _ => return None,
        };
        let shapes = info
            .payload_variants
            .iter()
            .filter_map(|arguments| arguments.get(argument_index))
            .collect::<Vec<_>>();
        if shapes.is_empty() {
            return None;
        }
        let mut payload = VariableInfo {
            flow: info.flow.clone(),
            sql_expression: info.sql_expression,
            ..VariableInfo::default()
        };
        for shape in shapes {
            payload
                .nominal_types
                .extend(shape.nominal_types.iter().cloned());
            if !shape.arguments.is_empty() {
                if shape.nominal_types.is_empty() {
                    payload.tuple_items =
                        shape.arguments.iter().map(Self::info_from_shape).collect();
                } else {
                    payload.payload_variants.insert(shape.arguments.clone());
                }
            }
        }
        for nominal in payload.nominal_types.clone() {
            if let Some(alias) = self.symbols.type_alias_info.get(&nominal) {
                payload.union(alias);
            }
        }
        Some(payload)
    }

    fn info_from_shape(shape: &NominalShape) -> VariableInfo {
        let mut info = VariableInfo {
            nominal_types: shape.nominal_types.clone(),
            ..VariableInfo::default()
        };
        if shape.nominal_types.is_empty() {
            info.tuple_items = shape.arguments.iter().map(Self::info_from_shape).collect();
        } else if !shape.arguments.is_empty() {
            info.payload_variants.insert(shape.arguments.clone());
        }
        info
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
                typed_info
                    .nominal_types
                    .extend(info.nominal_types.iter().cloned());
                typed_info
                    .payload_variants
                    .extend(info.payload_variants.iter().cloned());
                if typed_info.sql_expression == SqlExpressionKind::Static {
                    typed_info.sql_expression = info.sql_expression;
                }
                self.bind_pattern(&typed.pat, &typed_info);
            }
            Pat::Tuple(tuple) => {
                for (index, element) in tuple.elems.iter().enumerate() {
                    self.bind_pattern(element, info.tuple_items.get(index).unwrap_or(info));
                }
            }
            Pat::TupleStruct(tuple) => {
                let owner = self.pattern_owner(&tuple.path);
                for (index, element) in tuple.elems.iter().enumerate() {
                    if self.has_declared_fields(&owner) {
                        let field_info =
                            self.declared_field_info(&owner, &index.to_string(), false);
                        self.bind_pattern(element, &field_info);
                    } else if let Some(payload_info) = self.wrapper_payload_info(&owner, info) {
                        self.bind_pattern(element, &payload_info);
                    } else {
                        self.bind_pattern(element, info);
                    }
                }
            }
            Pat::Struct(structure) => {
                let owner = self.pattern_owner(&structure.path);
                for field in &structure.fields {
                    let field_name = match &field.member {
                        Member::Named(ident) => normalized_ident(ident),
                        Member::Unnamed(index) => index.index.to_string(),
                    };
                    if self.has_declared_fields(&owner) {
                        let field_info = self.declared_field_info(
                            &owner,
                            &field_name,
                            matches!(field.member, Member::Named(_)),
                        );
                        self.bind_pattern(&field.pat, &field_info);
                    } else {
                        self.bind_pattern(&field.pat, info);
                    }
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
                    let mut info = self.info_from_expr(expression);
                    info.sql_expression = self.sql_expression_kind(expression);
                    self.bind_pattern(pattern, &info);
                }
            }
            _ => {
                let mut info = self.info_from_expr(expression);
                info.sql_expression = self.sql_expression_kind(expression);
                self.bind_pattern(pattern, &info);
            }
        }
    }

    fn sql_expression_kind(&self, expression: &Expr) -> SqlExpressionKind {
        match expression {
            Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Str(_)) => {
                SqlExpressionKind::Static
            }
            Expr::Reference(reference) => self.sql_expression_kind(&reference.expr),
            Expr::Paren(paren) => self.sql_expression_kind(&paren.expr),
            Expr::Group(group) => self.sql_expression_kind(&group.expr),
            Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
                last_path_name(&path.path)
                    .and_then(|name| self.lookup(&name))
                    .map(|info| info.sql_expression)
                    .unwrap_or(SqlExpressionKind::Nonliteral)
            }
            Expr::Macro(mac)
                if last_path_name(&mac.mac.path)
                    .is_some_and(|name| matches!(name.as_str(), "format" | "format_args")) =>
            {
                SqlExpressionKind::Interpolated
            }
            Expr::MethodCall(method) if normalized_ident(&method.method) == "sql" => {
                let mut targets = self.flow_of_expr(&method.receiver).targets();
                if let Expr::Path(path) = method.receiver.as_ref() {
                    targets.extend(targets_for_path(&path.path, self.symbols));
                }
                if targets.iter().any(|target| {
                    matches!(
                        target,
                        PersistenceTarget::PreparedStatement
                            | PersistenceTarget::LoginStatements
                            | PersistenceTarget::WorldStatements
                            | PersistenceTarget::CharStatements
                            | PersistenceTarget::HotfixStatements
                    )
                }) {
                    SqlExpressionKind::Static
                } else {
                    SqlExpressionKind::Nonliteral
                }
            }
            _ => SqlExpressionKind::Nonliteral,
        }
    }

    fn field_flow(&self, field: &ExprField) -> Flow {
        let name = match &field.member {
            Member::Named(ident) => normalized_ident(ident),
            Member::Unnamed(index) => index.index.to_string(),
        };
        let mut targets = TargetSet::new();
        for owner in self.nominal_types_of_expr(&field.base) {
            let field_targets = match &field.member {
                Member::Named(_) => self.symbols.field_targets.get(&(owner, name.clone())),
                Member::Unnamed(_) => self.symbols.tuple_field_targets.get(&(owner, name.clone())),
            };
            if let Some(field_targets) = field_targets {
                targets.extend(field_targets);
            }
        }
        if targets.is_empty()
            && matches!(field.member, Member::Named(_))
            && let Some(owners) = self.symbols.field_owners.get(&name)
            && !owners.is_empty()
            && owners.iter().all(|owner| {
                self.symbols
                    .field_targets
                    .get(&(owner.clone(), name.clone()))
                    .is_some_and(|targets| !targets.is_empty())
            })
        {
            for owner in owners {
                if let Some(field_targets) = self
                    .symbols
                    .field_targets
                    .get(&(owner.clone(), name.clone()))
                {
                    targets.extend(field_targets);
                }
            }
        }
        if !targets.is_empty() {
            return Flow::pools(&targets);
        }
        if let Some(target) = database_field_target(&name) {
            return Flow::pools(&BTreeSet::from([target]));
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
        let path_targets = targets_for_path(&path.path, self.symbols);
        if !path_targets.is_empty() {
            return Flow::pools(&path_targets);
        }
        let associated = self.associated_return_info(path).flow;
        if !associated.is_empty() {
            return associated;
        }
        self.symbols
            .function_returns
            .get(last)
            .map(|info| info.flow.clone())
            .unwrap_or_default()
    }

    fn flow_of_method(&self, method: &ExprMethodCall) -> Flow {
        let receiver = self.flow_of_expr(&method.receiver);
        let name = normalized_ident(&method.method);
        if let Some(target) = database_getter_target(&name) {
            return Flow::pools(&BTreeSet::from([target]));
        }
        match name.as_str() {
            "begin" => receiver.map_pool_stage(FlowStage::Transaction),
            "acquire" | "pool" => receiver.map_pool_stage(FlowStage::DerivedPool),
            "prepare" => receiver.map_pool_stage(FlowStage::Query),
            "query" | "direct_query" | "delay_query_holder_like_cpp" => {
                receiver.map_pool_stage(FlowStage::Query)
            }
            "execute" | "direct_execute" => receiver.map_pool_stage(FlowStage::Query),
            "bind" if receiver.has_stage(FlowStage::Query) => receiver,
            name if FLOW_PASSTHROUGH_METHODS.contains(&name) => receiver,
            _ => self.method_return_info(method).flow,
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
                // Conservatively retain the scrutinee flow because match-arm
                // bindings can return an adapter-derived value (for example
                // `Some(db) => Arc::clone(db)`). A future type-aware data-flow
                // pass may narrow decoded scalar arms without losing those
                // bindings.
                let mut flow = self.flow_of_expr(&match_expression.expr);
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
            // These expressions always evaluate to `()`; persistence used by
            // their bodies is inventoried at the actual call/store sites and
            // must not be misreported as their result value.
            Expr::ForLoop(_) | Expr::While(_) => Flow::default(),
            Expr::Macro(expression) => self.flow_of_macro(&expression.mac),
            _ => {
                // `syn::Expr` is non-exhaustive. Conservatively propagate the
                // flow of every direct child for syntax that has no more
                // precise rule above, so present and future wrappers cannot
                // silently launder a concrete persistence value.
                let mut collector = DirectChildFlowCollector {
                    analyzer: self,
                    flow: Flow::default(),
                    at_root: true,
                };
                collector.visit_expr(expression);
                collector.flow
            }
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
        if !self.allows_source_class(attributes, owner) {
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
            let fingerprint = normalized_tokens(mac);
            self.add_generated(
                PersistenceTarget::Sqlx,
                PersistenceOperation::Query,
                &name,
                &cfg,
                fingerprint.clone(),
            );
            if sql_is_advisory_lock(&fingerprint) {
                self.add_generated(
                    PersistenceTarget::Sqlx,
                    PersistenceOperation::AdvisoryLock,
                    &name,
                    &cfg,
                    fingerprint,
                );
            }
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
                self.add_generated(
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
            let fingerprint = normalized_tokens(mac);
            let mut sqlx_calls = Vec::new();
            sqlx_calls_in_tokens(mac.tokens.clone(), &mut sqlx_calls);
            for (callable, call_fingerprint) in sqlx_calls {
                self.add(
                    PersistenceTarget::Sqlx,
                    PersistenceOperation::Query,
                    &callable,
                    &cfg,
                    call_fingerprint.clone(),
                );
                if sql_is_advisory_lock(&call_fingerprint) {
                    self.add(
                        PersistenceTarget::Sqlx,
                        PersistenceOperation::AdvisoryLock,
                        &callable,
                        &cfg,
                        call_fingerprint,
                    );
                }
            }
            let advisory_methods = BTreeSet::from([
                "acquire_like_cpp".to_owned(),
                "release_like_cpp".to_owned(),
                "wait_until_lost_like_cpp".to_owned(),
            ]);
            let mut methods = Vec::new();
            persistence_methods_in_tokens(mac.tokens.clone(), &advisory_methods, &mut methods);
            methods.sort();
            for method in methods {
                let mut targets = TargetSet::new();
                for scope in &self.scopes {
                    for (local, info) in scope {
                        if tokens_contain_identifier(
                            mac.tokens.clone(),
                            &BTreeSet::from([local.clone()]),
                        ) {
                            targets.extend(info.flow.targets());
                        }
                    }
                }
                for target in targets {
                    self.add(
                        target,
                        PersistenceOperation::AdvisoryLock,
                        &method,
                        &cfg,
                        format!("opaque-macro-method:{method}"),
                    );
                }
            }
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
                    fingerprint.clone(),
                );
                if target == PersistenceTarget::Sqlx && sql_is_advisory_lock(&fingerprint) {
                    self.add(
                        target,
                        PersistenceOperation::AdvisoryLock,
                        &name,
                        &cfg,
                        fingerprint.clone(),
                    );
                }
            }
            self.record_pool_escape(
                &escaped,
                PersistenceOperation::ArgumentEscape,
                &format!("macro:{name}"),
                &cfg,
                fingerprint,
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
            if !self.allows_source_class(&typed.attrs, "function parameter") {
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

    fn register_generic_bounds(&mut self, generics: &syn::Generics) {
        for parameter in &generics.params {
            let syn::GenericParam::Type(parameter) = parameter else {
                continue;
            };
            let name = normalized_ident(&parameter.ident);
            for bound in &parameter.bounds {
                if let syn::TypeParamBound::Trait(bound) = bound {
                    let trait_path = self
                        .canonical_local_path_names(path_names(&bound.path))
                        .join("::");
                    self.generic_trait_bounds
                        .entry(name.clone())
                        .or_default()
                        .insert(trait_path);
                }
            }
        }
        if let Some(where_clause) = &generics.where_clause {
            for predicate in &where_clause.predicates {
                let syn::WherePredicate::Type(predicate) = predicate else {
                    continue;
                };
                let Type::Path(path) = &predicate.bounded_ty else {
                    continue;
                };
                let Some(name) = last_path_name(&path.path) else {
                    continue;
                };
                for bound in &predicate.bounds {
                    if let syn::TypeParamBound::Trait(bound) = bound {
                        let trait_path = self
                            .canonical_local_path_names(path_names(&bound.path))
                            .join("::");
                        self.generic_trait_bounds
                            .entry(name.clone())
                            .or_default()
                            .insert(trait_path);
                    }
                }
            }
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
    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        if !self.allows_source_class(&expression.attrs, "for-loop expression") {
            return;
        }
        let cfg = item_cfg(&self.cfg, &expression.attrs);
        self.visit_expr(&expression.expr);
        let iterator_info = self.info_from_expr(&expression.expr);
        self.record_pool_escape(
            &iterator_info.flow,
            PersistenceOperation::ArgumentEscape,
            "for_iter",
            &cfg,
            normalized_tokens(&expression.expr),
        );
        self.push_scope();
        self.register_local_uses(&expression.body.stmts);
        self.bind_pattern(&expression.pat, &iterator_info);
        for statement in &expression.body.stmts {
            self.visit_stmt(statement);
        }
        self.pop_scope();
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        if !self.allows_source_class(&expression.attrs, "while expression") {
            return;
        }
        self.visit_expr(&expression.cond);
        self.push_scope();
        self.register_local_uses(&expression.body.stmts);
        if let Expr::Let(let_expression) = expression.cond.as_ref() {
            let mut info = self.info_from_expr(&let_expression.expr);
            if info.flow.0.is_empty()
                && let Expr::MethodCall(method) = let_expression.expr.as_ref()
            {
                info = self.info_from_expr(&method.receiver);
            }
            self.bind_pattern(&let_expression.pat, &info);
        }
        for statement in &expression.body.stmts {
            self.visit_stmt(statement);
        }
        self.pop_scope();
    }

    fn visit_expr_async(&mut self, expression: &'ast syn::ExprAsync) {
        if !self.allows_source_class(&expression.attrs, "async expression") {
            return;
        }
        let cfg = item_cfg(&self.cfg, &expression.attrs);
        let flow = self.flow_in_block(&expression.block);
        self.record_pool_escape(
            &flow,
            PersistenceOperation::ArgumentEscape,
            "async_capture",
            &cfg,
            normalized_tokens(expression),
        );
        self.visit_block(&expression.block);
    }

    fn visit_expr_array(&mut self, expression: &'ast syn::ExprArray) {
        if !self.allows_source_class(&expression.attrs, "array expression") {
            return;
        }
        let cfg = item_cfg(&self.cfg, &expression.attrs);
        let flow = self.flow_of_expr(&Expr::Array(expression.clone()));
        self.record_pool_escape(
            &flow,
            PersistenceOperation::ArgumentEscape,
            "array_value",
            &cfg,
            normalized_tokens(expression),
        );
        for element in &expression.elems {
            self.visit_expr(element);
        }
    }

    fn visit_expr_loop(&mut self, expression: &'ast syn::ExprLoop) {
        if !self.allows_source_class(&expression.attrs, "loop expression") {
            return;
        }
        let cfg = item_cfg(&self.cfg, &expression.attrs);
        let flow = self.flow_in_block(&expression.body);
        self.record_pool_escape(
            &flow,
            PersistenceOperation::ArgumentEscape,
            "loop_value",
            &cfg,
            normalized_tokens(expression),
        );
        self.visit_block(&expression.body);
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.push_scope();
        self.register_local_uses(&block.stmts);
        for statement in &block.stmts {
            self.visit_stmt(statement);
        }
        self.pop_scope();
    }

    fn visit_local(&mut self, local: &'ast Local) {
        if !self.allows_source_class(&local.attrs, "local binding") {
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
        if !self.allows_source_class(&path.attrs, "expression path") {
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
        let symbol = last_path_name(&path.path).unwrap_or_default();
        for target in targets_for_path(&path.path, self.symbols) {
            self.add(
                target,
                PersistenceOperation::PathReference,
                &symbol,
                &cfg,
                canonical_path(&path.path),
            );
            if matches!(
                target,
                PersistenceTarget::LoginStatements
                    | PersistenceTarget::WorldStatements
                    | PersistenceTarget::CharStatements
                    | PersistenceTarget::HotfixStatements
            ) && is_generated_id_read_statement(&symbol)
            {
                self.add(
                    target,
                    PersistenceOperation::GeneratedIdRead,
                    &symbol,
                    &cfg,
                    canonical_path(&path.path),
                );
            }
        }
    }

    fn visit_expr_field(&mut self, field: &'ast ExprField) {
        if !self.allows_source_class(&field.attrs, "field expression") {
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
        if !self.allows_source_class(&call.attrs, "function call") {
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
        let has_path_targets = !path_targets.is_empty();
        if query {
            let fingerprint = canonical_call(call);
            self.add(
                PersistenceTarget::Sqlx,
                PersistenceOperation::Query,
                &name,
                &cfg,
                fingerprint.clone(),
            );
            if sql_is_advisory_lock(&fingerprint) {
                self.add(
                    PersistenceTarget::Sqlx,
                    PersistenceOperation::AdvisoryLock,
                    &name,
                    &cfg,
                    fingerprint,
                );
            }
            if let Some(argument) = call.args.first()
                && let kind @ (SqlExpressionKind::Nonliteral | SqlExpressionKind::Interpolated) =
                    self.sql_expression_kind(argument)
            {
                self.add(
                    PersistenceTarget::Sqlx,
                    match kind {
                        SqlExpressionKind::Interpolated => PersistenceOperation::InterpolatedSql,
                        SqlExpressionKind::Nonliteral => PersistenceOperation::NonliteralSql,
                        SqlExpressionKind::Static => unreachable!(),
                    },
                    &name,
                    &cfg,
                    normalized_tokens(argument),
                );
            }
        } else if let Some(operation) = PersistenceOperation::from_executor_method(&name)
            .filter(|_| rooted_sqlx || has_path_targets)
        {
            let mut targets = path_targets;
            for argument in &call.args {
                targets.extend(self.flow_of_expr(argument).targets());
            }
            if targets.is_empty() {
                targets.insert(PersistenceTarget::Sqlx);
            }
            for target in targets {
                let operation = match (name.as_str(), target) {
                    ("new", PersistenceTarget::PreparedStatement)
                    | ("new", PersistenceTarget::SqlQueryHolder)
                    | ("with_capacity_like_cpp", PersistenceTarget::PreparedStatement) => {
                        PersistenceOperation::StatementBuilder
                    }
                    ("new", PersistenceTarget::SqlTransaction) => {
                        PersistenceOperation::TransactionConstruct
                    }
                    ("new", _) => PersistenceOperation::PathReference,
                    _ => operation,
                };
                self.add(target, operation, &name, &cfg, canonical_call(call));
            }
        }

        let known_persistence_call = query
            || ((rooted_sqlx || has_path_targets)
                && PersistenceOperation::from_executor_method(&name).is_some());
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
        if !self.allows_source_class(&method.attrs, "method call") {
            return;
        }
        let cfg = item_cfg(&self.cfg, &method.attrs);
        let name = normalized_ident(&method.method);
        let receiver = self.flow_of_expr(&method.receiver);
        let validated_flow_passthrough = FLOW_PASSTHROUGH_METHODS.contains(&name.as_str())
            || (name == "bind" && receiver.has_stage(FlowStage::Query));
        let operation = if is_query_name(&name) && !receiver.0.is_empty() {
            Some(PersistenceOperation::Query)
        } else {
            PersistenceOperation::from_executor_method(&name)
        };
        let mut valid_persistence_method = false;
        if let Some(operation) = operation {
            let valid = match operation {
                PersistenceOperation::Commit | PersistenceOperation::Rollback => {
                    receiver.has_stage(FlowStage::Transaction)
                        || receiver
                            .targets()
                            .contains(&PersistenceTarget::SqlTransaction)
                        || receiver
                            .targets()
                            .contains(&PersistenceTarget::SqlxTransaction)
                        || (name == "commit_transaction" && !receiver.0.is_empty())
                }
                PersistenceOperation::Begin => !receiver.pool_targets().is_empty(),
                PersistenceOperation::Query
                | PersistenceOperation::PoolAccess
                | PersistenceOperation::PrepareStatement
                | PersistenceOperation::DirectQuery
                | PersistenceOperation::DirectExecute
                | PersistenceOperation::RawSql
                | PersistenceOperation::TransactionAppend
                | PersistenceOperation::GeneratedIdRead
                | PersistenceOperation::AdvisoryLock
                | PersistenceOperation::NonliteralSql
                | PersistenceOperation::InterpolatedSql => !receiver.0.is_empty(),
                _ => {
                    receiver.has_stage(FlowStage::Query)
                        || receiver.has_stage(FlowStage::Transaction)
                        || !receiver.pool_targets().is_empty()
                }
            };
            if valid {
                valid_persistence_method = true;
                let mut targets = receiver.targets();
                for argument in &method.args {
                    targets.extend(self.flow_of_expr(argument).targets());
                }
                for target in targets {
                    self.add(target, operation, &name, &cfg, canonical_method(method));
                    if matches!(
                        operation,
                        PersistenceOperation::DirectQuery | PersistenceOperation::DirectExecute
                    ) && let Some(argument) = method.args.first()
                        && let kind @ (SqlExpressionKind::Nonliteral
                        | SqlExpressionKind::Interpolated) = self.sql_expression_kind(argument)
                    {
                        self.add(
                            target,
                            match kind {
                                SqlExpressionKind::Interpolated => {
                                    PersistenceOperation::InterpolatedSql
                                }
                                SqlExpressionKind::Nonliteral => {
                                    PersistenceOperation::NonliteralSql
                                }
                                SqlExpressionKind::Static => unreachable!(),
                            },
                            &name,
                            &cfg,
                            normalized_tokens(argument),
                        );
                    }
                }
            }
        } else if !validated_flow_passthrough {
            self.record_pool_escape(
                &receiver,
                PersistenceOperation::ArgumentEscape,
                &format!("receiver:{name}"),
                &cfg,
                normalized_tokens(&method.receiver),
            );
        }

        if !valid_persistence_method && !validated_flow_passthrough {
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
        if !self.allows_source_class(&assignment.attrs, "assignment") {
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
        if !self.allows_source_class(&structure.attrs, "struct expression") {
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
        if !self.allows_source_class(&returned.attrs, "return expression") {
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

    fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
        if !self.allows_source_class(&expression.attrs, "match expression") {
            return;
        }
        self.visit_expr(&expression.expr);
        let scrutinee = self.info_from_expr(&expression.expr);
        for arm in &expression.arms {
            self.push_scope();
            self.bind_pattern(&arm.pat, &scrutinee);
            if let Some((_, guard)) = &arm.guard {
                self.visit_expr(guard);
            }
            self.visit_expr(&arm.body);
            self.pop_scope();
        }
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        if !self.allows_source_class(&expression.attrs, "if expression") {
            return;
        }
        self.visit_expr(&expression.cond);
        self.push_scope();
        if let Expr::Let(let_expression) = expression.cond.as_ref() {
            self.bind_pattern_from_expr(&let_expression.pat, &let_expression.expr);
        }
        self.visit_block(&expression.then_branch);
        self.pop_scope();
        if let Some((_, else_expression)) = &expression.else_branch {
            self.visit_expr(else_expression);
        }
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        let cfg = item_cfg(&self.cfg, &item.attrs);
        let symbol = item
            .ident
            .as_ref()
            .map(normalized_ident)
            .or_else(|| last_path_name(&item.mac.path))
            .unwrap_or_else(|| "macro".to_owned());
        let mut targets = targets_in_tokens(item.mac.tokens.clone(), self.symbols);
        for scope in &self.scopes {
            for (local, info) in scope {
                if tokens_contain_identifier(
                    item.mac.tokens.clone(),
                    &BTreeSet::from([local.clone()]),
                ) {
                    targets.extend(info.flow.targets());
                }
            }
        }
        if targets.is_empty() {
            return;
        }
        for target in targets {
            self.add_generated(
                target,
                PersistenceOperation::MacroReference,
                &symbol,
                &cfg,
                normalized_tokens(&item.mac),
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
        if !self.allows_source_class(&closure.attrs, "closure") {
            return;
        }
        let previous_cfg = self.cfg.clone();
        self.cfg = item_cfg(&self.cfg, &closure.attrs);
        self.push_scope();
        for input in &closure.inputs {
            self.bind_pattern(input, &VariableInfo::default());
        }
        self.visit_expr(&closure.body);
        self.pop_scope();
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
                    generated_input: false,
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
    for (index, field) in item_struct.fields.iter().enumerate() {
        if !source_class_allows(
            context.source_class,
            cfg,
            &field.attrs,
            errors,
            "struct field",
        ) {
            continue;
        }
        let symbol = field
            .ident
            .as_ref()
            .map(normalized_ident)
            .unwrap_or_else(|| index.to_string());
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
        if !source_class_allows(
            context.source_class,
            cfg,
            &variant.attrs,
            errors,
            "enum variant",
        ) {
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
        for (index, field) in variant.fields.iter().enumerate() {
            if !source_class_allows(
                context.source_class,
                &variant_cfg,
                &field.attrs,
                errors,
                "enum field",
            ) {
                continue;
            }
            let symbol = field
                .ident
                .as_ref()
                .map(normalized_ident)
                .unwrap_or_else(|| index.to_string());
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
    analyzer.register_local_uses(&function.block.stmts);
    analyzer.register_generic_bounds(&function.sig.generics);
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
                    generated_input: false,
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
        if !source_class_allows(
            context.source_class,
            &cfg,
            &method.attrs,
            errors,
            "impl method",
        ) {
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
                source_class: context.source_class,
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
        analyzer.register_local_uses(&method.block.stmts);
        analyzer.register_generic_bounds(&item_impl.generics);
        analyzer.register_generic_bounds(&method.sig.generics);
        let mut self_info = analyzer.info_from_type(&item_impl.self_ty);
        self_info.flow = Flow::default();
        analyzer.bind("Self".to_owned(), self_info.clone());
        if method.sig.receiver().is_some() {
            analyzer.bind("self".to_owned(), self_info);
        }
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
                generated_input: true,
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
    let symbols = collect_module_symbols(
        items,
        parent_symbols,
        context.package,
        context.module,
        &cfg,
        context.source_class,
        errors,
    );
    for item in items {
        match item {
            Item::Use(item_use) => {
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_use.attrs,
                    errors,
                    "use declaration",
                ) {
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
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &extern_crate.attrs,
                    errors,
                    "extern crate",
                ) {
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
                let local = extern_crate
                    .rename
                    .as_ref()
                    .map(|(_, rename)| normalized_ident(rename))
                    .unwrap_or_else(|| source.clone());
                let import_target = match source.as_str() {
                    "sqlx" => Some(PersistenceTarget::Sqlx),
                    "wow_database" => Some(PersistenceTarget::Database),
                    _ => None,
                };
                if let Some(target) = import_target {
                    accumulator.add(
                        &context,
                        NewAccess {
                            enclosing: "module",
                            target,
                            operation: PersistenceOperation::Import,
                            symbol: &local,
                            visibility: &extern_visibility,
                            cfg: &extern_cfg,
                            fingerprint: format!("extern crate {source} as {local}"),
                            generated_input: false,
                        },
                    );
                }
            }
            Item::Type(alias) => {
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &alias.attrs,
                    errors,
                    "type alias",
                ) {
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
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_struct.attrs,
                    errors,
                    "struct",
                ) {
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
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_enum.attrs,
                    errors,
                    "enum",
                ) {
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
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &function.attrs,
                    errors,
                    "function",
                ) {
                    continue;
                }
                analyze_function(
                    function,
                    RecordContext {
                        classification: context.classification,
                        source_class: context.source_class,
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
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_impl.attrs,
                    errors,
                    "impl",
                ) {
                    continue;
                }
                analyze_impl(
                    item_impl,
                    RecordContext {
                        classification: context.classification,
                        source_class: context.source_class,
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
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_const.attrs,
                    errors,
                    "const",
                ) {
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
                        source_class: context.source_class,
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
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_static.attrs,
                    errors,
                    "static",
                ) {
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
                        source_class: context.source_class,
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
            Item::Trait(item_trait) => {
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_trait.attrs,
                    errors,
                    "trait",
                ) {
                    continue;
                }
                let name = normalized_ident(&item_trait.ident);
                let targets = symbols.type_aliases.get(&name).cloned().unwrap_or_default();
                if context.package != "wow-database" || targets.is_empty() {
                    if syntax_mentions_persistence(item_trait, &symbols) {
                        errors.push(format!(
                            "{} contains persistence syntax in an unsupported item grammar; expose an explicit import/type/function/impl before baselining: {}",
                            context.module,
                            normalized_tokens(item_trait)
                        ));
                    }
                    continue;
                }
                let trait_cfg = item_cfg(&cfg, &item_trait.attrs);
                for target in targets {
                    accumulator.add(
                        &context,
                        NewAccess {
                            enclosing: &format!("trait {name}"),
                            target,
                            operation: PersistenceOperation::TypeReference,
                            symbol: &name,
                            visibility: &normalized_visibility(&item_trait.vis),
                            cfg: &trait_cfg,
                            fingerprint: normalized_tokens(item_trait),
                            generated_input: false,
                        },
                    );
                }
            }
            Item::Macro(item_macro) => {
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_macro.attrs,
                    errors,
                    "item macro",
                ) {
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
                if !source_class_allows(context.source_class, &cfg, attrs, errors, "inline module")
                {
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
                        source_class: context.source_class,
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

/// Parse and inventory already-classified production/test source mounts.
/// Source order is irrelevant and duplicate logical mounts fail closed. Each
/// mount is analyzed once with `cfg(test) = false` and once with
/// `cfg(test) = true`; the test pass retains only syntax that cannot exist in
/// production, so shared imports and helpers are not double-counted.
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
            left.inherited_cfg,
        )
            .cmp(&(
                right.classification,
                right.package,
                right.module,
                right.source_path,
                right.inherited_cfg,
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
        if !seen_mounts.insert((
            source.package,
            source.module,
            source.source_path,
            source.inherited_cfg,
        )) {
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
        let cfg = extend_cfg_context(source.inherited_cfg, &syntax.attrs);
        let production = cfg_context_allows_production(&cfg, &[]);
        let test = cfg_context_allows_test(&cfg, &[]);
        let (production, test) = match (production, test) {
            (Ok(production), Ok(test)) => (production, test),
            (production, test) => {
                if let Err(error) = production {
                    errors.push(format!(
                        "invalid file cfg (production) in persistence source {}: {error}",
                        source.source_path
                    ));
                }
                if let Err(error) = test {
                    errors.push(format!(
                        "invalid file cfg (test) in persistence source {}: {error}",
                        source.source_path
                    ));
                }
                continue;
            }
        };
        if !production && !test {
            errors.push(format!(
                "persistence source {} is unreachable in both production and test cfg",
                source.source_path
            ));
            continue;
        }
        for source_class in [
            PersistenceSourceClass::Production,
            PersistenceSourceClass::TestFixture,
        ] {
            let enabled = match source_class {
                PersistenceSourceClass::Production => production,
                PersistenceSourceClass::TestFixture => test,
            };
            if !enabled {
                continue;
            }
            analyze_module_items(
                &syntax.items,
                RecordContext {
                    classification: source.classification,
                    source_class,
                    package: source.package,
                    module: source.module,
                    source: source.source_path,
                },
                None,
                cfg.clone(),
                &mut accumulator,
                &mut errors,
            );
        }
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
        if !matches!(record.source_class.as_str(), "production" | "test_fixture") {
            return Err(format!(
                "{label} persistence baseline contains invalid source_class {:?}",
                record.source_class
            ));
        }
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
        "{} {} {} {} {}::{} {} {:?} {:?} {} [{}]",
        identity.classification,
        identity.source_class,
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

/// Render the large snapshot with one canonical compact JSON object per row.
pub(crate) fn render_persistence_access_baseline(
    baseline: &PersistenceAccessBaseline,
) -> Result<String, String> {
    validated_baseline_map("rendered", baseline)?;
    let mut output = format!(
        "{{\n  \"schema_version\": {},\n  \"accesses\": [\n",
        baseline.schema_version
    );
    for (index, access) in baseline.accesses.iter().enumerate() {
        output.push_str("    ");
        output.push_str(
            &serde_json::to_string(access)
                .map_err(|error| format!("cannot serialize persistence access row: {error}"))?,
        );
        if index + 1 != baseline.accesses.len() {
            output.push(',');
        }
        output.push('\n');
    }
    output.push_str("  ]\n}");
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inventory(source: &str) -> Result<PersistenceAccessBaseline, String> {
        inventory_for_package("fixture", source)
    }

    fn inventory_for_package(
        package: &str,
        source: &str,
    ) -> Result<PersistenceAccessBaseline, String> {
        inventory_persistence_accesses(&[ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package,
            module: "crate",
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
    fn same_named_nested_import_reaches_a_fixed_point() {
        let item_use: ItemUse = syn::parse_quote!(
            use bitflags::bitflags;
        );
        let mut symbols = ModuleSymbols::for_package("fixture");
        symbols.module_path = vec!["realm".to_owned()];

        assert!(apply_import_symbols(&item_use, &mut symbols));
        let aliases = symbols.path_aliases.clone();
        assert!(!apply_import_symbols(&item_use, &mut symbols));
        assert_eq!(symbols.path_aliases, aliases);
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
    fn persistence_inventory_tracks_renamed_database_extern_crates_without_false_positives() {
        let baseline = inventory(
            r#"
                extern crate wow_database as db;

                async fn leak(database: &db::CharacterDatabase) {
                    let mut tx = database.pool().begin().await.unwrap();
                    tx.rollback().await.unwrap();
                }
            "#,
        )
        .expect("renamed wow_database extern crate resolves");
        let found = operations(&baseline);
        for expected in [
            (
                PersistenceTarget::Database,
                PersistenceOperation::Import,
                "db".to_owned(),
            ),
            (
                PersistenceTarget::CharacterDatabase,
                PersistenceOperation::TypeReference,
                "database".to_owned(),
            ),
            (
                PersistenceTarget::CharacterDatabase,
                PersistenceOperation::PoolAccess,
                "pool".to_owned(),
            ),
            (
                PersistenceTarget::CharacterDatabase,
                PersistenceOperation::Begin,
                "begin".to_owned(),
            ),
            (
                PersistenceTarget::CharacterDatabase,
                PersistenceOperation::Rollback,
                "rollback".to_owned(),
            ),
        ] {
            assert!(
                found.contains(&expected),
                "missing {expected:?}: {found:#?}"
            );
        }
        assert!(baseline.accesses.iter().any(|row| {
            row.operation == PersistenceOperation::Import
                && row.fingerprint == "extern crate wow_database as db"
        }));

        let error = compare_persistence_access_baseline(&inventory("").unwrap(), &baseline)
            .expect_err("a renamed database extern crate must trip the non-growth ratchet");
        assert!(
            error.contains("untracked direct persistence access"),
            "{error}"
        );

        let unrelated = inventory(
            r#"
                extern crate unrelated as db;
                fn innocent(_: db::CharacterDatabase) {}
            "#,
        )
        .expect("an unrelated extern crate remains ordinary Rust syntax");
        assert!(unrelated.accesses.is_empty(), "{:#?}", unrelated.accesses);
    }

    #[test]
    fn persistence_inventory_tracks_grouped_namespace_self_aliases() {
        let baseline = inventory(
            r#"
                async fn leak(database: &db::CharacterDatabase) {
                    database.pool().begin().await.unwrap();
                }
                use wow_database::{self as db};
            "#,
        )
        .expect("grouped self rename resolves independent of item order");
        let found = operations(&baseline);
        for expected in [
            (
                PersistenceTarget::Database,
                PersistenceOperation::Import,
                "db".to_owned(),
            ),
            (
                PersistenceTarget::CharacterDatabase,
                PersistenceOperation::PoolAccess,
                "pool".to_owned(),
            ),
            (
                PersistenceTarget::CharacterDatabase,
                PersistenceOperation::Begin,
                "begin".to_owned(),
            ),
        ] {
            assert!(
                found.contains(&expected),
                "missing {expected:?}: {found:#?}"
            );
        }
        assert!(baseline.accesses.iter().any(|row| {
            row.operation == PersistenceOperation::Import
                && row.fingerprint == "wow_database::self as db"
        }));
        assert!(
            compare_persistence_access_baseline(&inventory("").unwrap(), &baseline)
                .unwrap_err()
                .contains("untracked direct persistence access")
        );

        let unrelated = inventory(
            r#"
                use unrelated::{self as db};
                fn innocent(_: db::CharacterDatabase) {}
            "#,
        )
        .unwrap();
        assert!(unrelated.accesses.is_empty(), "{:#?}", unrelated.accesses);
    }

    #[test]
    fn persistence_inventory_tracks_numeric_tuple_fields() {
        let baseline = inventory(
            r#"
                struct Holder(u8, wow_database::CharacterDatabase);
                enum Wrapped { Database(u8, wow_database::CharacterDatabase) }
                fn leak(holder: &Holder) { holder.1.pool(); }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "struct Holder"
                && row.operation == PersistenceOperation::TypeReference
                && row.symbol == "1"
        }));
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "enum Wrapped::Database"
                && row.operation == PersistenceOperation::TypeReference
                && row.symbol == "1"
        }));
        let found = operations(&baseline);
        assert!(found.contains(&(
            PersistenceTarget::CharacterDatabase,
            PersistenceOperation::PathReference,
            "1".to_owned(),
        )));
        assert!(found.contains(&(
            PersistenceTarget::CharacterDatabase,
            PersistenceOperation::PoolAccess,
            "pool".to_owned(),
        )));

        let collision = inventory(
            r#"
                struct DatabaseHolder(wow_database::CharacterDatabase);
                struct Innocent(u8);
                fn clean(value: &Innocent) { consume(value.0); }
            "#,
        )
        .unwrap();
        assert!(
            !collision.accesses.iter().any(|row| {
                row.enclosing == "fn clean"
                    && matches!(
                        row.operation,
                        PersistenceOperation::PathReference | PersistenceOperation::ArgumentEscape
                    )
            }),
            "tuple fields from unrelated owner types must not contaminate each other: {:#?}",
            collision.accesses
        );

        let method_name_collisions = inventory(
            r#"
                struct DbHolder(u8, wow_database::CharacterDatabase);
                struct Plain(u8, u8);
                struct DbFactory;
                impl DbFactory {
                    fn make(&self) -> DbHolder { unreachable!() }
                    fn get(&self) -> wow_database::CharacterDatabase { unreachable!() }
                }
                struct PlainFactory;
                impl PlainFactory {
                    fn make(&self) -> Plain { Plain(0, 0) }
                    fn get(&self) -> u8 { 0 }
                }
                fn clean(factory: &PlainFactory) {
                    consume(factory.make().1);
                    consume(factory.get());
                }
            "#,
        )
        .unwrap();
        assert!(
            !method_name_collisions
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean"),
            "methods with the same name on unrelated receiver types must not contaminate each other: {:#?}",
            method_name_collisions.accesses
        );

        let aliases_and_constructors = inventory(
            r#"
                struct Holder(u8, wow_database::CharacterDatabase);
                type Alias = Holder;
                fn alias(value: &Alias) { value.1.pool(); }
                fn constructed(database: wow_database::CharacterDatabase) {
                    let value = Holder(0, database);
                    value.1.pool();
                }
                fn make(database: wow_database::CharacterDatabase) -> Holder {
                    Holder(0, database)
                }
                fn returned(database: wow_database::CharacterDatabase) {
                    make(database).1.pool();
                }
                struct Factory(wow_database::CharacterDatabase);
                impl Factory {
                    fn make(&self) -> Holder { Holder(0, self.0.clone()) }
                    fn self_returned(&self) { self.make().1.pool(); }
                    fn associated_make() -> Holder { unreachable!() }
                    fn self_associated_returned() { Self::associated_make().1.pool(); }
                }
                fn method_returned(factory: &Factory) { factory.make().1.pool(); }
                fn associated_returned() { Factory::associated_make().1.pool(); }
                struct UnitFactory;
                impl UnitFactory {
                    fn make(&self) -> Holder { unreachable!() }
                }
                fn inline_unit_returned() { UnitFactory.make().1.pool(); }
                struct StructFactory {}
                impl StructFactory {
                    fn make(&self) -> Holder { unreachable!() }
                }
                fn inline_struct_returned() { StructFactory {}.make().1.pool(); }
                struct Outer { factory: Factory }
                fn field_receiver(value: &Outer) { value.factory.make().1.pool(); }
                struct TupleOuter(Factory);
                fn tuple_field_receiver(value: &TupleOuter) { value.0.make().1.pool(); }
                fn deref_receiver(factory: &Factory) { (*factory).make().1.pool(); }
                fn destructured_receiver(value: Outer) {
                    let Outer { factory } = value;
                    factory.make().1.pool();
                }
                fn tuple_destructured_receiver(value: TupleOuter) {
                    let TupleOuter(factory) = value;
                    factory.make().1.pool();
                }
                trait Maker { fn make() -> Holder; }
                impl Maker for Factory { fn make() -> Holder { unreachable!() } }
                fn ufcs_returned() { <Factory as Maker>::make().1.pool(); }
                struct DbOuter { database: wow_database::CharacterDatabase }
                fn destructured_database(value: DbOuter) {
                    let DbOuter { database } = value;
                    database.pool();
                }
                fn build_factory() -> Factory { unreachable!() }
                fn typed_local_receiver() {
                    let factory: Factory = build_factory();
                    factory.make().1.pool();
                }
                fn boxed_receiver(factory: Box<Factory>) { factory.make().1.pool(); }
                enum Receivers {
                    Tuple(Factory),
                    Named { factory: Factory },
                }
                fn enum_tuple_receiver(value: Receivers) {
                    if let Receivers::Tuple(factory) = value {
                        factory.make().1.pool();
                    }
                }
                fn enum_named_receiver(value: Receivers) {
                    if let Receivers::Named { factory } = value {
                        factory.make().1.pool();
                    }
                }
                struct Plan { statements: Vec<wow_database::PreparedStatement> }
                struct Planner;
                impl Planner {
                    fn plan(&self) -> Option<Plan> { None }
                    fn consume_plan(&self) {
                        let Some(plan) = self.plan() else { return };
                        consume(plan.statements);
                    }
                }
                struct Job { statement: wow_database::PreparedStatement }
                async fn consume_received(rx: &Receiver<Job>) {
                    while let Some(job) = rx.recv().await {
                        consume(job.statement);
                    }
                }
                fn consume_channel() {
                    let (_, mut rx) = unbounded_channel::<Job>();
                    async move {
                        while let Some(job) = rx.recv().await {
                            consume(job.statement);
                        }
                    };
                }
                fn nested_plan() -> Option<Option<Plan>> { None }
                fn consume_nested_plan() {
                    let Some(inner) = nested_plan() else { return };
                    let Some(plan) = inner else { return };
                    consume(plan.statements);
                }
                async fn consume_result(rx: &Receiver<Result<u8, Job>>) {
                    while let Some(Err(job)) = rx.recv().await {
                        consume(job.statement);
                    }
                }
                fn inferred_wrappers(job: Job) {
                    let wrapped = Some(job);
                    let Some(job) = wrapped else { return };
                    consume(job.statement);
                }
                fn inferred_err(job: Job) {
                    let wrapped: Result<u8, Job> = Err(job);
                    let Err(job) = wrapped else { return };
                    consume(job.statement);
                }
                fn inferred_tuple(factory: Factory) {
                    let pair = (factory, 0_u8);
                    let (factory, _) = pair;
                    factory.make().1.pool();
                }
                fn inferred_wrapped_tuple(factory: Factory) {
                    let wrapped = Some((factory, 0_u8));
                    let Some((factory, _)) = wrapped else { return };
                    factory.make().1.pool();
                }
                fn pair() -> (Factory, u8) { unreachable!() }
                fn tuple_returned() {
                    let (factory, _) = pair();
                    factory.make().1.pool();
                }
                impl Factory {
                    fn pair(&self) -> (Factory, u8) { unreachable!() }
                    fn method_tuple_returned(&self) {
                        let (factory, _) = self.pair();
                        factory.make().1.pool();
                    }
                }
                fn tuple_parameter(pair: (Factory, u8)) {
                    let (factory, _) = pair;
                    factory.make().1.pool();
                }
                fn referenced_tuple_parameter(pair: &(Factory, u8)) {
                    let (factory, _) = pair;
                    factory.make().1.pool();
                }
                fn wrapped_pair() -> Option<(Factory, u8)> { None }
                fn wrapped_tuple_returned() {
                    let Some((factory, _)) = wrapped_pair() else { return };
                    factory.make().1.pool();
                }
                fn reversed_pair() -> (u8, Factory) { unreachable!() }
                fn reversed_tuple_returned() {
                    let (_, factory) = reversed_pair();
                    factory.make().1.pool();
                }
                type PairAlias = (Factory, u8);
                type NestedPairAlias = PairAlias;
                fn aliased_pair() -> NestedPairAlias { unreachable!() }
                fn aliased_tuple_returned() {
                    let (factory, _) = aliased_pair();
                    factory.make().1.pool();
                }
                type WrappedPairAlias = Option<PairAlias>;
                fn aliased_wrapped_pair() -> WrappedPairAlias { None }
                fn aliased_wrapped_tuple_returned() {
                    let Some((factory, _)) = aliased_wrapped_pair() else { return };
                    factory.make().1.pool();
                }
                fn qualified_enum(value: Receivers) {
                    if let self::Receivers::Named { factory } = value {
                        factory.make().1.pool();
                    }
                }
            "#,
        )
        .unwrap();
        for enclosing in [
            "fn alias",
            "fn constructed",
            "fn returned",
            "fn method_returned",
            "impl Factory::self_returned",
            "impl Factory::self_associated_returned",
            "fn associated_returned",
            "fn inline_unit_returned",
            "fn inline_struct_returned",
            "fn field_receiver",
            "fn tuple_field_receiver",
            "fn deref_receiver",
            "fn destructured_receiver",
            "fn tuple_destructured_receiver",
            "fn ufcs_returned",
            "fn destructured_database",
            "fn typed_local_receiver",
            "fn boxed_receiver",
            "fn enum_tuple_receiver",
            "fn enum_named_receiver",
            "fn qualified_enum",
            "fn tuple_returned",
            "impl Factory::method_tuple_returned",
            "fn tuple_parameter",
            "fn referenced_tuple_parameter",
            "fn wrapped_tuple_returned",
            "fn reversed_tuple_returned",
            "fn aliased_tuple_returned",
            "fn aliased_wrapped_tuple_returned",
        ] {
            assert!(
                aliases_and_constructors.accesses.iter().any(|row| {
                    row.enclosing == enclosing
                        && row.target == PersistenceTarget::CharacterDatabase
                        && row.operation == PersistenceOperation::PoolAccess
                }),
                "tuple-field owner lost in {enclosing}: {:#?}",
                aliases_and_constructors.accesses
            );
        }
        for enclosing in [
            "impl Planner::consume_plan",
            "fn consume_received",
            "fn consume_channel",
            "fn consume_nested_plan",
            "fn consume_result",
            "fn inferred_wrappers",
            "fn inferred_err",
        ] {
            assert!(
                aliases_and_constructors.accesses.iter().any(|row| {
                    row.enclosing == enclosing
                        && row.target == PersistenceTarget::PreparedStatement
                        && row.operation == PersistenceOperation::PathReference
                }),
                "wrapper payload owner lost in {enclosing}: {:#?}",
                aliases_and_constructors.accesses
            );
        }
        for enclosing in ["fn inferred_tuple", "fn inferred_wrapped_tuple"] {
            assert!(aliases_and_constructors.accesses.iter().any(|row| {
                row.enclosing == enclosing
                    && row.target == PersistenceTarget::CharacterDatabase
                    && row.operation == PersistenceOperation::PoolAccess
            }));
        }

        let field_owner_collision = inventory(
            r#"
                struct Holder(wow_database::CharacterDatabase);
                struct DbFactory;
                impl DbFactory { fn make(&self) -> Holder { unreachable!() } }
                struct PlainFactory;
                impl PlainFactory { fn make(&self) -> u8 { 0 } }
                struct DbOuter { factory: DbFactory }
                struct PlainOuter { factory: PlainFactory }
                fn clean(value: &PlainOuter) { consume(value.factory.make()); }
                struct Wrapper<T>(T);
                impl<T> Wrapper<T> { fn make(&self) -> u8 { 0 } }
                fn generic_clean(value: &Wrapper<DbFactory>) { consume(value.make()); }
                trait PlainMaker { fn make() -> u8; }
                impl PlainMaker for PlainFactory { fn make() -> u8 { 0 } }
                fn ufcs_clean() { consume(<PlainFactory as PlainMaker>::make()); }
                enum DbEvent { Ready { factory: DbFactory } }
                enum PlainEvent { Ready { factory: PlainFactory } }
                fn enum_clean(value: PlainEvent) {
                    if let PlainEvent::Ready { factory } = value {
                        consume(factory.make());
                    }
                }
                struct DbJob { statement: wow_database::PreparedStatement }
                async fn result_sibling_clean(rx: &Receiver<Result<DbJob, u8>>) {
                    while let Some(Err(code)) = rx.recv().await {
                        consume(code);
                    }
                }
                fn db_pair() -> (DbFactory, u8) { unreachable!() }
                fn tuple_sibling_clean() {
                    let (_, code) = db_pair();
                    consume(code);
                }
                type DbPairAlias = (DbFactory, u8);
                fn aliased_db_pair() -> DbPairAlias { unreachable!() }
                fn aliased_tuple_sibling_clean() {
                    let (_, code) = aliased_db_pair();
                    consume(code);
                }
                struct TraitFactory;
                trait DbMaker { fn make(&self) -> Holder; }
                trait PlainTraitMaker { fn make(&self) -> u8; }
                impl DbMaker for TraitFactory {
                    fn make(&self) -> Holder { unreachable!() }
                }
                impl PlainTraitMaker for TraitFactory {
                    fn make(&self) -> u8 { 0 }
                }
                fn trait_ufcs_clean(factory: &TraitFactory) {
                    consume(<TraitFactory as PlainTraitMaker>::make(factory));
                }
                mod db_trait {
                    pub trait Maker { fn make(&self) -> super::Holder; }
                    pub trait Derived: Maker {}
                }
                mod plain_trait {
                    pub trait Maker { fn make(&self) -> u8; }
                    pub trait Derived: Maker {}
                }
                impl crate::db_trait::Maker for TraitFactory {
                    fn make(&self) -> Holder { unreachable!() }
                }
                impl plain_trait::Maker for TraitFactory {
                    fn make(&self) -> u8 { 0 }
                }
                fn qualified_trait_ufcs_persistent(factory: &TraitFactory) {
                    consume(<TraitFactory as db_trait::Maker>::make(factory).0.pool());
                }
                fn qualified_trait_ufcs_clean(factory: &TraitFactory) {
                    consume(<TraitFactory as plain_trait::Maker>::make(factory));
                }
                use plain_trait::Maker as ImportedPlainMaker;
                fn imported_trait_ufcs_clean(factory: &TraitFactory) {
                    consume(<TraitFactory as ImportedPlainMaker>::make(factory));
                }
                fn dyn_trait_persistent(factory: &dyn db_trait::Maker) {
                    consume(factory.make().0.pool());
                }
                fn generic_trait_persistent<T: db_trait::Maker>(factory: &T) {
                    consume(factory.make().0.pool());
                }
                fn where_trait_persistent<T>(factory: &T)
                where
                    T: db_trait::Maker,
                {
                    consume(factory.make().0.pool());
                }
                fn dyn_trait_clean(factory: &dyn plain_trait::Maker) {
                    consume(factory.make());
                }
                fn generic_trait_clean<T: plain_trait::Maker>(factory: &T) {
                    consume(factory.make());
                }
                fn dyn_supertrait_persistent(factory: &dyn db_trait::Derived) {
                    consume(factory.make().0.pool());
                }
                fn dyn_supertrait_clean(factory: &dyn plain_trait::Derived) {
                    consume(factory.make());
                }
                struct ImplBound<T>(T);
                impl<T> ImplBound<T>
                where
                    T: db_trait::Maker,
                {
                    fn impl_bound_persistent(&self) {
                        consume(self.0.make().0.pool());
                    }
                }
                mod db_method_scope {
                    use super::db_trait::Maker;
                    fn scoped_trait_method_persistent(factory: &super::TraitFactory) {
                        consume(factory.make().0.pool());
                    }
                }
                mod plain_method_scope {
                    use super::plain_trait::Maker;
                    fn scoped_trait_method_clean(factory: &super::TraitFactory) {
                        consume(factory.make());
                    }
                }
                mod module_anonymous_method_scope {
                    mod marker_trait {
                        pub trait Marker { fn marker(&self); }
                    }
                    impl marker_trait::Marker for super::TraitFactory {
                        fn marker(&self) {}
                    }
                    use super::db_trait::Maker as _;
                    use marker_trait::Marker as _;
                    fn anonymous_module_traits_are_additive(factory: &super::TraitFactory) {
                        factory.marker();
                        consume(factory.make().0.pool());
                    }
                }
                mod local_method_scope {
                    mod marker_trait {
                        pub trait Marker { fn marker(&self); }
                    }
                    impl marker_trait::Marker for super::TraitFactory {
                        fn marker(&self) {}
                    }
                    fn local_trait_method_persistent(factory: &super::TraitFactory) {
                        use super::db_trait::Maker;
                        consume(factory.make().0.pool());
                    }
                    fn nested_local_trait_method_persistent(factory: &super::TraitFactory) {
                        {
                            use super::db_trait::Maker;
                            consume(factory.make().0.pool());
                        }
                    }
                    fn local_trait_method_clean(factory: &super::TraitFactory) {
                        use super::plain_trait::Maker;
                        consume(factory.make());
                    }
                    fn disabled_local_trait_is_ignored(factory: &super::TraitFactory) {
                        use super::plain_trait::Maker;
                        #[cfg(any())]
                        use super::db_trait::Maker;
                        consume(factory.make());
                    }
                    fn disabled_anonymous_trait_is_ignored(factory: &super::TraitFactory) {
                        use super::plain_trait::Maker;
                        #[cfg(any())]
                        use super::db_trait::Maker as _;
                        consume(factory.make());
                    }
                    fn anonymous_local_traits_are_additive(factory: &super::TraitFactory) {
                        use super::db_trait::Maker as _;
                        use marker_trait::Marker as _;
                        factory.marker();
                        consume(factory.make().0.pool());
                    }
                    fn local_trait_scope_does_not_escape(factory: &super::TraitFactory) {
                        use super::plain_trait::Maker;
                        {
                            use super::db_trait::Maker;
                            consume(factory.make().0.pool());
                        }
                        consume(factory.make());
                    }
                }
                #[cfg(test)]
                fn test_only_local_trait(factory: &TraitFactory) {
                    #[cfg(test)]
                    use db_trait::Maker;
                    consume(factory.make().0.pool());
                }
            "#,
        )
        .unwrap();
        assert!(
            !field_owner_collision.accesses.iter().any(|row| matches!(
                row.enclosing.as_str(),
                "fn clean"
                    | "fn generic_clean"
                    | "fn ufcs_clean"
                    | "fn enum_clean"
                    | "fn result_sibling_clean"
                    | "fn tuple_sibling_clean"
                    | "fn aliased_tuple_sibling_clean"
                    | "fn trait_ufcs_clean"
                    | "fn qualified_trait_ufcs_clean"
                    | "fn imported_trait_ufcs_clean"
                    | "fn scoped_trait_method_clean"
                    | "fn local_trait_method_clean"
                    | "fn disabled_local_trait_is_ignored"
                    | "fn disabled_anonymous_trait_is_ignored"
                    | "fn dyn_trait_clean"
                    | "fn generic_trait_clean"
                    | "fn dyn_supertrait_clean"
            )),
            "named fields on unrelated owner types must not contaminate receiver identity: {:#?}",
            field_owner_collision.accesses
        );
        assert!(field_owner_collision.accesses.iter().any(|row| {
            row.enclosing == "fn qualified_trait_ufcs_persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(field_owner_collision.accesses.iter().any(|row| {
            row.enclosing == "fn scoped_trait_method_persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
        for enclosing in [
            "fn local_trait_method_persistent",
            "fn nested_local_trait_method_persistent",
            "fn local_trait_scope_does_not_escape",
            "fn anonymous_local_traits_are_additive",
            "fn anonymous_module_traits_are_additive",
            "fn dyn_trait_persistent",
            "fn generic_trait_persistent",
            "fn where_trait_persistent",
            "fn dyn_supertrait_persistent",
            "impl ImplBound < T >::impl_bound_persistent",
        ] {
            assert!(
                field_owner_collision.accesses.iter().any(|row| {
                    row.enclosing == enclosing
                        && row.target == PersistenceTarget::CharacterDatabase
                        && row.operation == PersistenceOperation::PoolAccess
                }),
                "missing bounded trait return in {enclosing}"
            );
        }
        assert_eq!(
            field_owner_collision
                .accesses
                .iter()
                .filter(|row| {
                    row.enclosing == "fn local_trait_scope_does_not_escape"
                        && row.target == PersistenceTarget::CharacterDatabase
                        && row.operation == PersistenceOperation::PoolAccess
                })
                .map(|row| row.count)
                .sum::<usize>(),
            1,
            "a nested persistent trait import must not contaminate its scalar sibling scope"
        );
        assert!(
            field_owner_collision.accesses.iter().any(|row| {
                row.enclosing == "fn test_only_local_trait"
                    && row.source_class == "test_fixture"
                    && row.target == PersistenceTarget::CharacterDatabase
                    && row.operation == PersistenceOperation::PoolAccess
            }),
            "cfg(test) local trait import was not isolated: {:#?}",
            field_owner_collision
                .accesses
                .iter()
                .filter(|row| row.enclosing == "fn test_only_local_trait")
                .collect::<Vec<_>>()
        );
        assert!(!field_owner_collision.accesses.iter().any(|row| {
            row.enclosing == "fn test_only_local_trait"
                && row.source_class == "production"
                && row.target == PersistenceTarget::CharacterDatabase
        }));
    }

    #[test]
    fn persistence_inventory_records_pool_escape_for_unvalidated_executor_names() {
        let baseline = inventory(
            r#"
                struct LocalExecutor;
                fn leak(local: &LocalExecutor, pool: &sqlx::PgPool) {
                    local.execute(pool);
                }
                fn valid(pool: &sqlx::PgPool) {
                    sqlx::query("SELECT 1").execute(pool);
                }
                fn bound(pool: &sqlx::PgPool) {
                    sqlx::query("SELECT ?").bind(1_u32).execute(pool);
                }
                struct LocalBinder;
                fn unrelated(local: &LocalBinder, pool: &sqlx::PgPool) {
                    local.bind(pool);
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn leak"
                && row.target == PersistenceTarget::PgPool
                && row.operation == PersistenceOperation::ArgumentEscape
                && row.symbol == "execute"
        }));
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn leak"
                && row.target == PersistenceTarget::PgPool
                && row.operation == PersistenceOperation::Execute
        }));
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn bound" && row.operation == PersistenceOperation::Execute
        }));
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn unrelated"
                && row.operation == PersistenceOperation::ArgumentEscape
                && row.symbol == "bind"
        }));
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn valid" && row.operation == PersistenceOperation::Execute
        }));
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn valid"
                && row.operation == PersistenceOperation::ArgumentEscape
                && row.symbol == "execute"
        }));
    }

    #[test]
    fn persistence_inventory_conservatively_propagates_unmodeled_expression_children() {
        let baseline = inventory(
            r#"
                fn array(pool: sqlx::PgPool) { consume([pool]); }
                fn index(pool: sqlx::PgPool) { consume([pool][0]); }
                async fn async_block(pool: sqlx::PgPool) { consume(async { pool }.await); }
                fn loop_value(pool: sqlx::PgPool) { consume(loop { break pool }); }
                fn for_binding(databases: Vec<wow_database::CharacterDatabase>) {
                    for database in databases { database.pool(); }
                }
                fn for_capture(pool: sqlx::PgPool) { for _ in [pool] {} }
                fn standalone_async(pool: sqlx::PgPool) { async move { pool }; }
                fn async_non_tail(pool: sqlx::PgPool, flag: bool) {
                    async move {
                        if flag { pool; 0_u8 } else { 0_u8 };
                        0_u8
                    };
                }
                fn loop_standalone(pool: sqlx::PgPool) { loop { break pool; }; }
                fn array_standalone(pool: sqlx::PgPool) { [pool]; }
                fn while_binding(mut databases: Vec<wow_database::CharacterDatabase>) {
                    while let Some(database) = databases.pop() { database.pool(); }
                }
                fn scalars(value: u32) {
                    consume([value]); consume([value][0]); consume(loop { break value });
                    for _ in [value] {} async move { value };
                }
            "#,
        )
        .unwrap();
        for enclosing in ["fn array", "fn index", "fn async_block", "fn loop_value"] {
            assert!(
                baseline.accesses.iter().any(|row| {
                    row.enclosing == enclosing
                        && row.target == PersistenceTarget::PgPool
                        && row.operation == PersistenceOperation::ArgumentEscape
                }),
                "missing propagated escape for {enclosing}"
            );
        }
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn for_binding"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
        for enclosing in [
            "fn for_capture",
            "fn standalone_async",
            "fn async_non_tail",
            "fn loop_standalone",
            "fn array_standalone",
        ] {
            assert!(
                baseline.accesses.iter().any(|row| {
                    row.enclosing == enclosing
                        && row.target == PersistenceTarget::PgPool
                        && row.operation == PersistenceOperation::ArgumentEscape
                }),
                "missing standalone wrapper escape for {enclosing}"
            );
        }
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn while_binding"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn scalars")
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
    fn persistence_inventory_resolves_typed_database_paths_getters_and_dynamic_sql() {
        let baseline = inventory(
            r#"
                use wow_database::{CharacterDatabase, DatabaseError as DbError, SqlTransaction};
                struct Session;
                impl Session {
                    fn character_db(&self) -> Option<&CharacterDatabase> { None }
                    async fn query(&self) {
                        let db = self.character_db().unwrap();
                        let sql = format!("SELECT {}", 1);
                        db.direct_query(&sql).await.unwrap();
                        let _tx = wow_database::SqlTransaction::new();
                        let _error = wow_database::DatabaseError::Query("x".into());
                    }
                }
                struct Store;
                impl Store {
                    fn transaction(&self) -> SqlTransaction { unreachable!() }
                    async fn commit(&self) {
                        let transaction = self.transaction();
                        transaction.commit_with_outcome_like_cpp().await.unwrap();
                    }
                }
                fn split_sql(content: &str) -> Vec<&str> { vec![content] }
                fn dynamic(content: &str) {
                    for statement in split_sql(content) {
                        sqlx::query(statement);
                    }
                }
            "#,
        )
        .expect("typed database imports, getters and qualified paths are explicit grammar");
        let found = operations(&baseline);
        for expected in [
            (
                PersistenceTarget::CharacterDatabase,
                PersistenceOperation::DirectQuery,
                "direct_query".to_owned(),
            ),
            (
                PersistenceTarget::CharacterDatabase,
                PersistenceOperation::InterpolatedSql,
                "direct_query".to_owned(),
            ),
            (
                PersistenceTarget::SqlTransaction,
                PersistenceOperation::TransactionConstruct,
                "new".to_owned(),
            ),
            (
                PersistenceTarget::DatabaseError,
                PersistenceOperation::PathReference,
                "Query".to_owned(),
            ),
            (
                PersistenceTarget::SqlTransaction,
                PersistenceOperation::Commit,
                "commit_with_outcome_like_cpp".to_owned(),
            ),
            (
                PersistenceTarget::Sqlx,
                PersistenceOperation::NonliteralSql,
                "query".to_owned(),
            ),
        ] {
            assert!(
                found.contains(&expected),
                "missing {expected:?} from {found:#?}"
            );
        }
        assert!(baseline.accesses.iter().any(|row| {
            row.target == PersistenceTarget::DatabaseError
                && row.operation == PersistenceOperation::Import
                && row.symbol == "DbError"
        }));
    }

    #[test]
    fn persistence_inventory_avoids_local_database_and_sqlx_variant_collisions() {
        let innocent = inventory(
            r#"
                enum LogFilter { Database }
                struct Database;
                fn local(_: Database) { let _ = LogFilter::Database; }
            "#,
        )
        .expect("local names are ordinary Rust symbols");
        assert!(innocent.accesses.is_empty(), "{:#?}", innocent.accesses);

        let sqlx = inventory(
            r#"
                fn classify(error: sqlx::Error) {
                    if let sqlx::Error::Database(inner) = error { drop(inner); }
                }
            "#,
        )
        .expect("sqlx variant paths stay in the sqlx target");
        assert!(
            sqlx.accesses
                .iter()
                .any(|row| row.target == PersistenceTarget::Sqlx)
        );
        assert!(
            !sqlx
                .accesses
                .iter()
                .any(|row| row.target == PersistenceTarget::Database),
            "{:#?}",
            sqlx.accesses
        );

        let adapter = inventory_for_package(
            "wow-database",
            "fn local(statement: PreparedStatement) -> SqlResult { todo!() }",
        )
        .expect("wow-database owns its unqualified concrete types");
        assert!(adapter.accesses.iter().any(|row| {
            row.target == PersistenceTarget::PreparedStatement
                && row.operation == PersistenceOperation::TypeReference
        }));
        let adapter_reexports = inventory_for_package(
            "wow-database",
            r#"
                pub use database::Database;
                use super::StatementDef;
            "#,
        )
        .expect("adapter-local re-exports resolve their imported leaf");
        for expected in [PersistenceTarget::Database, PersistenceTarget::StatementDef] {
            assert!(adapter_reexports.accesses.iter().any(|row| {
                row.target == expected && row.operation == PersistenceOperation::Import
            }));
        }

        let transaction_variant = inventory(
            r#"
                use sqlx::Transaction;
                use wow_database::DatabaseError;
                fn classify(error: DatabaseError) {
                    if let DatabaseError::Transaction(inner) = error { drop(inner); }
                }
            "#,
        )
        .expect("an imported sqlx type must not capture a same-named enum variant");
        assert!(
            !transaction_variant
                .accesses
                .iter()
                .any(|row| row.target == PersistenceTarget::SqlxTransaction
                    && row.symbol == "Transaction"
                    && row.operation != PersistenceOperation::Import),
            "{:#?}",
            transaction_variant.accesses
        );

        let generated_id = inventory(
            r#"
                use wow_database::CharStatements;
                fn allocator_seed() { let _ = CharStatements::SEL_MAX_ITEM_GUID; }
            "#,
        )
        .expect("MAX-ID statement reads are explicit inventory operations");
        assert!(generated_id.accesses.iter().any(|row| {
            row.target == PersistenceTarget::CharStatements
                && row.operation == PersistenceOperation::GeneratedIdRead
                && row.symbol == "SEL_MAX_ITEM_GUID"
        }));
    }

    #[test]
    fn persistence_inventory_tracks_head_shaped_transactions_fields_branches_and_locks() {
        let baseline = inventory(
            r#"
                use sqlx::Acquire;
                use wow_database::{CharacterDatabase, ItemGuidAllocatorAdvisoryLockLikeCpp};
                struct State { login_db: wow_database::LoginDatabase }
                struct Session;
                impl Session { fn char_db(&self) -> Option<&CharacterDatabase> { None } }
                async fn work(state: &State, session: &Session, db: &CharacterDatabase) {
                    state.login_db.direct_query("SELECT 1").await.unwrap();
                    let mut tx = db.pool().begin().await.map_err(map_error).context("begin").unwrap();
                    tx.rollback().await.unwrap();
                    let mut tx = db.pool().begin().await.unwrap();
                    tx.commit().await.unwrap();
                    if let Some(char_db) = session.char_db() {
                        char_db.execute(&char_db.prepare(TODO)).await.unwrap();
                    }
                    let lock = ItemGuidAllocatorAdvisoryLockLikeCpp::acquire_like_cpp(db.pool()).await.unwrap();
                    lock.wait_until_lost_like_cpp().await.unwrap();
                    lock.release_like_cpp().await.unwrap();
                }
            "#,
        )
        .expect("HEAD-shaped persistence flow remains visible");
        let found = operations(&baseline);
        for expected in [
            (
                PersistenceTarget::LoginDatabase,
                PersistenceOperation::DirectQuery,
                "direct_query",
            ),
            (
                PersistenceTarget::CharacterDatabase,
                PersistenceOperation::Rollback,
                "rollback",
            ),
            (
                PersistenceTarget::CharacterDatabase,
                PersistenceOperation::Commit,
                "commit",
            ),
            (
                PersistenceTarget::CharacterDatabase,
                PersistenceOperation::Execute,
                "execute",
            ),
            (
                PersistenceTarget::CharacterDatabase,
                PersistenceOperation::PoolAccess,
                "pool",
            ),
            (
                PersistenceTarget::ItemGuidAllocatorAdvisoryLockLikeCpp,
                PersistenceOperation::AdvisoryLock,
                "acquire_like_cpp",
            ),
            (
                PersistenceTarget::ItemGuidAllocatorAdvisoryLockLikeCpp,
                PersistenceOperation::AdvisoryLock,
                "wait_until_lost_like_cpp",
            ),
            (
                PersistenceTarget::ItemGuidAllocatorAdvisoryLockLikeCpp,
                PersistenceOperation::AdvisoryLock,
                "release_like_cpp",
            ),
        ] {
            assert!(
                found.contains(&(expected.0, expected.1, expected.2.to_owned())),
                "missing {expected:?} from {found:#?}"
            );
        }
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
        .expect("production and test satisfiability are classified");
        assert!(
            baseline
                .accesses
                .iter()
                .filter(|record| record.enclosing == "fn test_only")
                .all(|record| record.source_class == "test_fixture")
        );
        assert!(
            baseline
                .accesses
                .iter()
                .any(|record| record.enclosing == "fn test_only")
        );
        assert!(
            baseline
                .accesses
                .iter()
                .filter(|record| record.enclosing == "fn production_capable")
                .all(|record| record.source_class == "production")
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
    fn persistence_inventory_keeps_production_and_test_alias_graphs_separate() {
        let baseline = inventory(
            r#"
                #[cfg(not(test))]
                use sqlx::MySqlPool as Pool;
                #[cfg(test)]
                use sqlx::PgPool as Pool;

                #[cfg(not(test))]
                fn production_only(pool: Pool) { pool.begin(); }
                #[cfg(test)]
                fn test_only(pool: Pool) { pool.begin(); }

                #[cfg(test)]
                macro_rules! generated_test_query {
                    () => { sqlx::query("SELECT 1") };
                }
            "#,
        )
        .expect("mutually exclusive aliases resolve in their logical cfg views");

        let production_targets = baseline
            .accesses
            .iter()
            .filter(|record| record.enclosing == "fn production_only")
            .map(|record| (record.target, record.source_class.as_str()))
            .collect::<BTreeSet<_>>();
        assert!(production_targets.contains(&(PersistenceTarget::MySqlPool, "production")));
        assert!(
            !production_targets
                .iter()
                .any(|(target, _)| *target == PersistenceTarget::PgPool)
        );

        let test_targets = baseline
            .accesses
            .iter()
            .filter(|record| record.enclosing == "fn test_only")
            .map(|record| (record.target, record.source_class.as_str()))
            .collect::<BTreeSet<_>>();
        assert!(test_targets.contains(&(PersistenceTarget::PgPool, "test_fixture")));
        assert!(
            !test_targets
                .iter()
                .any(|(target, _)| *target == PersistenceTarget::MySqlPool)
        );

        assert!(baseline.accesses.iter().any(|record| {
            record.operation == PersistenceOperation::MacroReference
                && record.symbol == "generated_test_query"
                && record.source_class == "test_fixture"
        }));
    }

    #[test]
    fn persistence_inventory_accepts_test_only_mounts_and_ratchets_their_growth() {
        let test_cfg = vec!["cfg(test)".to_owned()];
        let test_mount = |source| ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::tests",
            source_path: "src/tests.rs",
            inherited_cfg: &test_cfg,
            source,
        };
        let expected = inventory_persistence_accesses(&[test_mount(
            "fn existing(pool: sqlx::PgPool) { pool.begin(); }",
        )])
        .expect("test-only source mounts are part of the baseline");
        assert!(!expected.accesses.is_empty());
        assert!(
            expected
                .accesses
                .iter()
                .all(|record| record.source_class == "test_fixture")
        );

        let actual = inventory_persistence_accesses(&[test_mount(
            r#"
                fn existing(pool: sqlx::PgPool) { pool.begin(); }
                fn added(pool: sqlx::PgPool) { pool.begin(); }
            "#,
        )])
        .unwrap();
        let error = compare_persistence_access_baseline(&expected, &actual)
            .expect_err("new test-only concrete persistence must trip the ratchet");
        assert!(
            error.contains("untracked direct persistence access"),
            "{error}"
        );
        assert!(error.contains("test_fixture"), "{error}");
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
