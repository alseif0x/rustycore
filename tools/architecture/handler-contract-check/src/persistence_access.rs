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

#[derive(Clone, Debug, Default)]
struct VariableInfo {
    flow: Flow,
    sql_expression: SqlExpressionKind,
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
    let mut targets = targets_for_names(&leaf.source, symbols);
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
        let source_is_sqlx = source_is_sqlx(&leaf.source, symbols);
        let source_is_database = source_is_database(&leaf.source, symbols);
        if leaf.source.len() == 1 && source_is_sqlx {
            changed |= symbols.sqlx_namespaces.insert(leaf.local.clone());
        }
        if leaf.source.len() == 1 && source_is_database {
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

fn collect_module_symbols(
    items: &[Item],
    parent: Option<&ModuleSymbols>,
    package: &str,
    cfg: &[String],
    source_class: PersistenceSourceClass,
    errors: &mut Vec<String>,
) -> ModuleSymbols {
    let mut symbols = parent
        .cloned()
        .unwrap_or_else(|| ModuleSymbols::for_package(package));
    for _ in 0..=items.len() {
        let mut changed = false;
        for item in items {
            match item {
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
            Item::Struct(item_struct)
                if source_class_allows(source_class, cfg, &item_struct.attrs, errors, "struct") =>
            {
                for field in &item_struct.fields {
                    if !source_class_allows(source_class, cfg, &field.attrs, errors, "struct field")
                    {
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
                    for field in &variant.fields {
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
            Item::Fn(function)
                if source_class_allows(source_class, cfg, &function.attrs, errors, "function") =>
            {
                if let ReturnType::Type(_, ty) = &function.sig.output {
                    let targets = targets_in_type(ty, &symbols);
                    if !targets.is_empty() {
                        symbols
                            .function_returns
                            .insert(normalized_ident(&function.sig.ident), Flow::pools(&targets));
                    }
                }
            }
            Item::Impl(item_impl)
                if source_class_allows(source_class, cfg, &item_impl.attrs, errors, "impl") =>
            {
                for item in &item_impl.items {
                    let ImplItem::Fn(method) = item else {
                        continue;
                    };
                    if !source_class_allows(source_class, cfg, &method.attrs, errors, "impl method")
                    {
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
        VariableInfo {
            flow: Flow::pools(&targets_in_type(ty, self.symbols)),
            sql_expression: SqlExpressionKind::Static,
        }
    }

    fn info_from_expr(&self, expression: &Expr) -> VariableInfo {
        VariableInfo {
            flow: self.flow_of_expr(expression),
            sql_expression: self.sql_expression_kind(expression),
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
        if let Some(targets) = self.symbols.field_targets.get(&name) {
            return Flow::pools(targets);
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
        self.symbols
            .function_returns
            .get(last)
            .cloned()
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
        let operation = if is_query_name(&name) && !receiver.0.is_empty() {
            Some(PersistenceOperation::Query)
        } else {
            PersistenceOperation::from_executor_method(&name)
        };
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
            self.scopes.push(BTreeMap::new());
            self.bind_pattern(&arm.pat, &scrutinee);
            if let Some((_, guard)) = &arm.guard {
                self.visit_expr(guard);
            }
            self.visit_expr(&arm.body);
            self.scopes.pop();
        }
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        if !self.allows_source_class(&expression.attrs, "if expression") {
            return;
        }
        self.visit_expr(&expression.cond);
        self.scopes.push(BTreeMap::new());
        if let Expr::Let(let_expression) = expression.cond.as_ref() {
            self.bind_pattern_from_expr(&let_expression.pat, &let_expression.expr);
        }
        self.visit_block(&expression.then_branch);
        self.scopes.pop();
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
    for field in &item_struct.fields {
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
        for field in &variant.fields {
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
