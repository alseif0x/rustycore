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
use syn::parse::Parser;
use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprCall, ExprClosure, ExprField, ExprMacro, ExprMethodCall, ExprReturn,
    ExprStruct, FnArg, ImplItem, Item, ItemEnum, ItemFn, ItemImpl, ItemMod, ItemStruct, ItemTrait,
    ItemType, ItemUse, Local, Member, Pat, ReturnType, Stmt, TraitItem, Type, UseTree, Visibility,
};

use crate::ownership::{
    WorkspaceDependencyAliases, cfg_context_allows_production, cfg_context_allows_test,
    extend_cfg_context,
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
    "unwrap",
    "context",
    "with_context",
];

/// Combinators whose result may be produced by their arguments (a closure or
/// a fallback value), not only by the receiver. Treating them as
/// receiver-only passthroughs would hide a persistence value created inside
/// the argument, e.g. `Some(0_u8).map(|_| database).unwrap().pool()`.
const FLOW_TRANSFORMING_METHODS: &[&str] = &[
    "map",
    "map_err",
    "ok_or",
    "ok_or_else",
    "or",
    "or_else",
    "unwrap_or",
    "unwrap_or_else",
];

const CLOSURE_INVOKING_METHODS: &[&str] = &[
    "and_then",
    "filter",
    "filter_map",
    "flat_map",
    "for_each",
    "get_or_init",
    "get_or_insert_with",
    "inspect",
    "inspect_err",
    "is_ok_and",
    "is_some_and",
    "map",
    "map_err",
    "map_or",
    "map_or_else",
    "ok_or_else",
    "or_else",
    "unwrap_or_else",
    "with_context",
];

const OPAQUE_PERSISTENCE_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "bail",
    "debug",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "error",
    "ensure",
    "format",
    "format_args",
    "info",
    "join",
    "matches",
    "panic",
    "select",
    "trace",
    "try_join",
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
            "acquire" | "pool" => Some(Self::PoolAccess),
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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
    transactions: Vec<Vec<AccessIdentity>>,
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
        *self.rows.entry(identity.clone()).or_insert(0) += 1;
        if let Some(transaction) = self.transactions.last_mut() {
            transaction.push(identity);
        }
    }

    fn begin_transaction(&mut self) {
        self.transactions.push(Vec::new());
    }

    fn commit_transaction(&mut self) {
        let committed = self
            .transactions
            .pop()
            .expect("persistence access transaction is active");
        if let Some(parent) = self.transactions.last_mut() {
            parent.extend(committed);
        }
    }

    fn rollback_transaction(&mut self) {
        let rolled_back = self
            .transactions
            .pop()
            .expect("persistence access transaction is active");
        for identity in rolled_back.into_iter().rev() {
            let count = self
                .rows
                .get_mut(&identity)
                .expect("transactional persistence row was recorded");
            *count -= 1;
            if *count == 0 {
                self.rows.remove(&identity);
            }
        }
    }

    fn contains_symbol(&self, enclosing: &str, symbol: &str) -> bool {
        self.rows
            .keys()
            .any(|row| row.enclosing == enclosing && row.symbol == symbol)
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

/// Names of the type parameters declared by a signature (`fn make<T, U>`),
/// in declaration order, so an explicit turbofish at the call site can be
/// mapped positionally onto the recorded return.
fn generic_type_param_names(generics: &syn::Generics) -> Vec<String> {
    generics
        .params
        .iter()
        .filter_map(|parameter| match parameter {
            syn::GenericParam::Type(parameter) => Some(normalized_ident(&parameter.ident)),
            _ => None,
        })
        .collect()
}

const RECEIVER_INPUT_MARKER: &str = "$receiver";

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
struct GenericInputSpec {
    params: BTreeSet<String>,
    tuple_paths: BTreeMap<String, Vec<Vec<usize>>>,
}

fn projected_generic_argument(
    argument: &VariableInfo,
    input: &GenericInputSpec,
    param: &str,
) -> VariableInfo {
    let Some(paths) = input.tuple_paths.get(param) else {
        return argument.clone();
    };
    let mut result = VariableInfo::default();
    let mut projected_any = false;
    for path in paths {
        let mut projected = argument;
        let mut complete = true;
        for index in path {
            let Some(item) = projected.tuple_items.get(*index) else {
                complete = false;
                break;
            };
            projected = item;
        }
        if complete {
            projected_any = true;
            result.union(projected);
        }
    }
    if projected_any {
        result
    } else {
        argument.clone()
    }
}

fn collect_generic_tuple_paths(
    ty: &Type,
    params: &BTreeSet<String>,
    path: &mut Vec<usize>,
    output: &mut BTreeMap<String, Vec<Vec<usize>>>,
) {
    match ty {
        Type::Path(type_path) => {
            if let Some(name) = last_path_name(&type_path.path)
                && params.contains(&name)
                && !path.is_empty()
            {
                output.entry(name).or_default().push(path.clone());
            }
            for segment in &type_path.path.segments {
                if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
                    for argument in &arguments.args {
                        if let syn::GenericArgument::Type(inner) = argument {
                            collect_generic_tuple_paths(inner, params, path, output);
                        }
                    }
                }
            }
        }
        Type::Tuple(tuple) => {
            for (index, element) in tuple.elems.iter().enumerate() {
                path.push(index);
                collect_generic_tuple_paths(element, params, path, output);
                path.pop();
            }
        }
        Type::Reference(reference) => {
            collect_generic_tuple_paths(&reference.elem, params, path, output);
        }
        Type::Ptr(pointer) => collect_generic_tuple_paths(&pointer.elem, params, path, output),
        Type::Paren(paren) => collect_generic_tuple_paths(&paren.elem, params, path, output),
        Type::Group(group) => collect_generic_tuple_paths(&group.elem, params, path, output),
        _ => {}
    }
}

fn generic_params_by_input(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
    params: &[String],
) -> Vec<GenericInputSpec> {
    let all_params = params.iter().cloned().collect::<BTreeSet<_>>();
    inputs
        .iter()
        .map(|input| match input {
            FnArg::Typed(typed) => {
                let params = params
                    .iter()
                    .filter(|param| {
                        tokens_contain_identifier(
                            typed.ty.to_token_stream(),
                            &BTreeSet::from([(*param).clone()]),
                        )
                    })
                    .cloned()
                    .collect();
                let mut tuple_paths = BTreeMap::new();
                collect_generic_tuple_paths(
                    &typed.ty,
                    &all_params,
                    &mut Vec::new(),
                    &mut tuple_paths,
                );
                GenericInputSpec {
                    params,
                    tuple_paths,
                }
            }
            // Keep the receiver in the formal-input sequence. A method-call
            // expression omits it and skips this marker below, while UFCS
            // supplies it explicitly and therefore keeps later generic
            // arguments aligned with their declared inputs.
            FnArg::Receiver(_) => GenericInputSpec {
                params: BTreeSet::from([RECEIVER_INPUT_MARKER.to_owned()]),
                tuple_paths: BTreeMap::new(),
            },
        })
        .collect()
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
    // Valid MySQL spells these functions in any case (`SELECT get_lock(...)`);
    // a case-sensitive test would lose the AdvisoryLock identity and with it
    // the connection-affinity fact the semantic ledger must preserve.
    let normalized = fingerprint.to_ascii_uppercase();
    ["GET_LOCK", "RELEASE_LOCK", "IS_USED_LOCK"]
        .iter()
        .any(|needle| normalized.contains(needle))
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

#[derive(Default)]
struct PersistenceOperationSyntax {
    symbols: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for PersistenceOperationSyntax {
    fn visit_expr_method_call(&mut self, method: &'ast ExprMethodCall) {
        let name = normalized_ident(&method.method);
        if name != "new" && PersistenceOperation::from_executor_method(&name).is_some() {
            self.symbols.insert(name);
        }
        syn::visit::visit_expr_method_call(self, method);
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(path) = call.func.as_ref()
            && path.path.segments.len() >= 2
            && let Some(name) = last_path_name(&path.path)
            && name != "new"
            && PersistenceOperation::from_executor_method(&name).is_some()
        {
            self.symbols.insert(name);
        }
        syn::visit::visit_expr_call(self, call);
    }
}

fn persistence_operations_in_syntax(item: &Item) -> BTreeSet<String> {
    let mut visitor = PersistenceOperationSyntax::default();
    visitor.visit_item(item);
    visitor.symbols
}

fn persistence_operations_in_block(block: &syn::Block) -> BTreeSet<String> {
    let mut visitor = PersistenceOperationSyntax::default();
    visitor.visit_block(block);
    visitor.symbols
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

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
enum SqlExpressionKind {
    #[default]
    Static,
    Nonliteral,
    Included,
    Environment,
    Interpolated,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CallableSignature {
    generic_params: Vec<String>,
    generic_inputs: Vec<GenericInputSpec>,
}

fn closure_pattern_info(
    pattern: &Pat,
    input_index: usize,
    path: &mut Vec<usize>,
    generic_params: &mut Vec<String>,
    input: &mut GenericInputSpec,
) -> VariableInfo {
    match pattern {
        Pat::Ident(_) => {
            let suffix = path
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join("_");
            let marker = if suffix.is_empty() {
                format!("$closure_arg_{input_index}")
            } else {
                format!("$closure_arg_{input_index}_{suffix}")
            };
            generic_params.push(marker.clone());
            input.params.insert(marker.clone());
            if !path.is_empty() {
                input
                    .tuple_paths
                    .entry(marker.clone())
                    .or_default()
                    .push(path.clone());
            }
            VariableInfo {
                nominal_types: BTreeSet::from([marker]),
                ..VariableInfo::default()
            }
        }
        Pat::Tuple(tuple) => VariableInfo {
            tuple_items: tuple
                .elems
                .iter()
                .enumerate()
                .map(|(index, element)| {
                    path.push(index);
                    let info =
                        closure_pattern_info(element, input_index, path, generic_params, input);
                    path.pop();
                    info
                })
                .collect(),
            ..VariableInfo::default()
        },
        Pat::Reference(reference) => {
            closure_pattern_info(&reference.pat, input_index, path, generic_params, input)
        }
        Pat::Type(typed) => {
            closure_pattern_info(&typed.pat, input_index, path, generic_params, input)
        }
        Pat::Paren(paren) => {
            closure_pattern_info(&paren.pat, input_index, path, generic_params, input)
        }
        Pat::Wild(_) => VariableInfo::default(),
        _ => {
            // Non-tuple projections do not have a structural path model yet.
            // Retain the complete argument conservatively instead of losing it.
            let marker = format!("$closure_arg_{input_index}");
            if !generic_params.contains(&marker) {
                generic_params.push(marker.clone());
            }
            input.params.insert(marker.clone());
            VariableInfo {
                nominal_types: BTreeSet::from([marker]),
                ..VariableInfo::default()
            }
        }
    }
}

fn closure_callable_model(closure: &ExprClosure) -> (CallableSignature, Vec<VariableInfo>) {
    let mut generic_params = Vec::new();
    let mut generic_inputs = Vec::new();
    let mut parameter_infos = Vec::new();
    for (input_index, pattern) in closure.inputs.iter().enumerate() {
        let mut input = GenericInputSpec::default();
        let info = closure_pattern_info(
            pattern,
            input_index,
            &mut Vec::new(),
            &mut generic_params,
            &mut input,
        );
        generic_inputs.push(input);
        parameter_infos.push(info);
    }
    (
        CallableSignature {
            generic_params,
            generic_inputs,
        },
        parameter_infos,
    )
}

fn closure_callable_signature(closure: &ExprClosure) -> CallableSignature {
    closure_callable_model(closure).0
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct VariableInfo {
    flow: Flow,
    sql_expression: SqlExpressionKind,
    nominal_types: BTreeSet<String>,
    payload_variants: BTreeSet<Vec<NominalShape>>,
    tuple_items: Vec<VariableInfo>,
    field_items: BTreeMap<String, VariableInfo>,
    trait_bounds: BTreeSet<String>,
    type_generic_params: Vec<String>,
    callable_signatures: BTreeSet<CallableSignature>,
    closure_mutations: BTreeMap<String, VariableInfo>,
}

impl VariableInfo {
    fn union(&mut self, other: &Self) {
        self.flow.union(other.flow.clone());
        self.nominal_types
            .extend(other.nominal_types.iter().cloned());
        self.payload_variants
            .extend(other.payload_variants.iter().cloned());
        self.trait_bounds.extend(other.trait_bounds.iter().cloned());
        if self.type_generic_params.is_empty() {
            self.type_generic_params = other.type_generic_params.clone();
        }
        self.callable_signatures
            .extend(other.callable_signatures.iter().cloned());
        for (name, mutation) in &other.closure_mutations {
            self.closure_mutations
                .entry(name.clone())
                .or_default()
                .union(mutation);
        }
        for (field, other_info) in &other.field_items {
            self.field_items
                .entry(field.clone())
                .or_default()
                .union(other_info);
        }
        self.sql_expression = self.sql_expression.max(other.sql_expression);
        if self.tuple_items.len() < other.tuple_items.len() {
            self.tuple_items
                .resize_with(other.tuple_items.len(), VariableInfo::default);
        }
        for (item, other_item) in self.tuple_items.iter_mut().zip(&other.tuple_items) {
            item.union(other_item);
        }
    }

    fn substitute_self(
        &mut self,
        receiver_types: &BTreeSet<String>,
        trait_bounds: &BTreeSet<String>,
    ) -> bool {
        let mut replaced = self.nominal_types.remove("Self");
        if replaced {
            self.nominal_types.extend(receiver_types.iter().cloned());
        }
        let mut variants = BTreeSet::new();
        for variant in std::mem::take(&mut self.payload_variants) {
            let mut shapes = variant;
            let mut variant_replaced = false;
            for shape in &mut shapes {
                variant_replaced |= shape.substitute_self(receiver_types);
            }
            if variant_replaced {
                replaced = true;
            }
            variants.insert(shapes);
        }
        self.payload_variants = variants;
        for item in &mut self.tuple_items {
            replaced |= item.substitute_self(receiver_types, trait_bounds);
        }
        for item in self.field_items.values_mut() {
            replaced |= item.substitute_self(receiver_types, trait_bounds);
        }
        if replaced {
            self.trait_bounds.extend(trait_bounds.iter().cloned());
        }
        replaced
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct NominalShape {
    nominal_types: BTreeSet<String>,
    arguments: Vec<NominalShape>,
}

impl NominalShape {
    fn substitute_self(&mut self, receiver_types: &BTreeSet<String>) -> bool {
        let mut replaced = self.nominal_types.remove("Self");
        if replaced {
            self.nominal_types.extend(receiver_types.iter().cloned());
        }
        for argument in &mut self.arguments {
            replaced |= argument.substitute_self(receiver_types);
        }
        replaced
    }
}

/// Collapse an arbitrarily deep value shape into its reachable persistence
/// facts without recursively cloning that shape. Generic recursive types can
/// otherwise grow one structural layer per workspace fixed-point pass and
/// eventually overflow the stack before the registry converges.
fn flatten_reachable_variable_info(root: &VariableInfo) -> VariableInfo {
    let mut flattened = VariableInfo::default();
    let mut pending = vec![root];
    while let Some(info) = pending.pop() {
        flattened.flow.union(info.flow.clone());
        flattened
            .nominal_types
            .extend(info.nominal_types.iter().cloned());
        flattened
            .trait_bounds
            .extend(info.trait_bounds.iter().cloned());
        if flattened.type_generic_params.is_empty() {
            flattened.type_generic_params = info.type_generic_params.clone();
        }
        flattened
            .callable_signatures
            .extend(info.callable_signatures.iter().cloned());
        flattened.sql_expression = flattened.sql_expression.max(info.sql_expression);
        pending.extend(info.tuple_items.iter());
        pending.extend(info.field_items.values());
        pending.extend(info.closure_mutations.values());

        let mut shapes = info
            .payload_variants
            .iter()
            .flat_map(|variant| variant.iter())
            .collect::<Vec<_>>();
        while let Some(shape) = shapes.pop() {
            flattened
                .nominal_types
                .extend(shape.nominal_types.iter().cloned());
            shapes.extend(shape.arguments.iter());
        }
    }
    flattened
}

/// Replaces generic parameter names with the bound's recorded type
/// arguments, recursively (e.g. a `Maker<T>` return resolved through
/// `M: Maker<Holder>` must surface `Holder`, not the nominal `T`). The
/// recorded argument carries its full `VariableInfo`, so the named-type
/// registry data (flow, fields, payloads) of the concrete type arrives with
/// the substitution.
fn substitute_shape_params(
    shape: &mut NominalShape,
    map: &BTreeMap<String, VariableInfo>,
    merged: &mut VariableInfo,
) -> bool {
    let mut replaced = false;
    let original = std::mem::take(&mut shape.nominal_types);
    shape.nominal_types = original
        .into_iter()
        .flat_map(|name| match map.get(&name) {
            Some(replacement) => {
                replaced = true;
                // A concrete generic argument may itself contain this nominal
                // payload recursively. Flatten its reachable information
                // before merging so workspace named-type discovery has a
                // finite abstract state while remaining conservative.
                let widened = flatten_reachable_variable_info(replacement);
                merged.union(&widened);
                replacement
                    .nominal_types
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
            }
            None => vec![name],
        })
        .collect();
    for argument in &mut shape.arguments {
        replaced |= substitute_shape_params(argument, map, merged);
    }
    replaced
}

fn substitute_nominal_params(
    info: &mut VariableInfo,
    map: &BTreeMap<String, VariableInfo>,
) -> bool {
    let mut replaced = false;
    let original = std::mem::take(&mut info.nominal_types);
    let mut resolved_names = BTreeSet::new();
    let mut merged = VariableInfo::default();
    for name in original {
        match map.get(&name) {
            Some(replacement) => {
                replaced = true;
                resolved_names.extend(replacement.nominal_types.iter().cloned());
                merged.union(replacement);
            }
            None => {
                resolved_names.insert(name);
            }
        }
    }
    info.nominal_types = resolved_names;
    info.union(&merged);
    for item in &mut info.tuple_items {
        replaced |= substitute_nominal_params(item, map);
    }
    for item in info.field_items.values_mut() {
        replaced |= substitute_nominal_params(item, map);
    }
    for mutation in info.closure_mutations.values_mut() {
        replaced |= substitute_nominal_params(mutation, map);
    }
    let variants = std::mem::take(&mut info.payload_variants);
    let mut payload_replacements = VariableInfo::default();
    info.payload_variants = variants
        .into_iter()
        .map(|shapes| {
            shapes
                .into_iter()
                .map(|mut shape| {
                    replaced |= substitute_shape_params(&mut shape, map, &mut payload_replacements);
                    shape
                })
                .collect()
        })
        .collect();
    info.union(&payload_replacements);
    replaced
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
    // Generic parameter name lists, recorded next to the return registries so
    // an explicit turbofish at the call site can be substituted into the
    // recorded return instead of letting `make::<CharacterDatabase>()` bypass
    // both ratchets.
    function_generic_params: BTreeMap<String, Vec<String>>,
    function_generic_input_params: BTreeMap<String, Vec<GenericInputSpec>>,
    method_generic_params: BTreeMap<(String, Option<String>, String), Vec<String>>,
    method_generic_input_params: BTreeMap<(String, Option<String>, String), Vec<GenericInputSpec>>,
    trait_method_returns: std::sync::Arc<BTreeMap<(String, String), VariableInfo>>,
    trait_supertraits: std::sync::Arc<BTreeMap<String, BTreeSet<String>>>,
    trait_generic_params: std::sync::Arc<BTreeMap<String, Vec<String>>>,
    trait_method_generic_params: std::sync::Arc<BTreeMap<(String, String), Vec<String>>>,
    trait_method_generic_input_params:
        std::sync::Arc<BTreeMap<(String, String), Vec<GenericInputSpec>>>,
    // Current-package paths stay unqualified (`dto::Holder`) while the
    // workspace registry is shared and crate-qualified
    // (`provider_crate::dto::Holder`).
    named_type_info: std::sync::Arc<BTreeMap<String, VariableInfo>>,
    workspace_named_type_info: std::sync::Arc<BTreeMap<String, VariableInfo>>,
    dependency_crate_aliases: std::sync::Arc<BTreeMap<String, String>>,
    package_function_returns: std::sync::Arc<BTreeMap<String, VariableInfo>>,
    package_function_generic_params: std::sync::Arc<BTreeMap<String, Vec<String>>>,
    package_function_generic_input_params: std::sync::Arc<BTreeMap<String, Vec<GenericInputSpec>>>,
    // Package-wide registries for inherent impl methods (keyed by canonical
    // crate-relative owner path): without them `factory.make()` only resolves
    // when the impl lives in the same module as the call.
    package_method_returns: std::sync::Arc<BTreeMap<(String, String), VariableInfo>>,
    package_method_generic_params: std::sync::Arc<BTreeMap<(String, String), Vec<String>>>,
    package_method_generic_input_params:
        std::sync::Arc<BTreeMap<(String, String), Vec<GenericInputSpec>>>,
    // Module constants/statics are value bindings, not lexical locals. Keep
    // both the current module's names and a package-wide canonical registry
    // so their declared persistence-bearing types survive path resolution.
    item_values: BTreeMap<String, VariableInfo>,
    package_item_values: std::sync::Arc<BTreeMap<String, VariableInfo>>,
    sqlx_namespaces: BTreeSet<String>,
    database_namespaces: BTreeSet<String>,
    query_callables: BTreeSet<String>,
    // `macro_rules!` definitions whose body already reaches concrete
    // persistence: the definition is baselined once, and this registry makes
    // every later invocation leave its own row too.
    persistence_macros: BTreeMap<String, TargetSet>,
    // Persistence-generating macro_rules definitions can be invoked from a
    // different physical source module through #[macro_use], #[macro_export],
    // or a macro import. The leaf-name union intentionally fails closed.
    package_persistence_macros: std::sync::Arc<BTreeMap<String, TargetSet>>,
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
            function_generic_params: BTreeMap::new(),
            function_generic_input_params: BTreeMap::new(),
            method_generic_params: BTreeMap::new(),
            method_generic_input_params: BTreeMap::new(),
            trait_method_returns: std::sync::Arc::new(BTreeMap::new()),
            trait_supertraits: std::sync::Arc::new(BTreeMap::new()),
            trait_generic_params: std::sync::Arc::new(BTreeMap::new()),
            trait_method_generic_params: std::sync::Arc::new(BTreeMap::new()),
            trait_method_generic_input_params: std::sync::Arc::new(BTreeMap::new()),
            named_type_info: std::sync::Arc::new(BTreeMap::new()),
            workspace_named_type_info: std::sync::Arc::new(BTreeMap::new()),
            dependency_crate_aliases: std::sync::Arc::new(BTreeMap::new()),
            package_function_returns: std::sync::Arc::new(BTreeMap::new()),
            package_function_generic_params: std::sync::Arc::new(BTreeMap::new()),
            package_function_generic_input_params: std::sync::Arc::new(BTreeMap::new()),
            package_method_returns: std::sync::Arc::new(BTreeMap::new()),
            package_method_generic_params: std::sync::Arc::new(BTreeMap::new()),
            package_method_generic_input_params: std::sync::Arc::new(BTreeMap::new()),
            item_values: BTreeMap::new(),
            package_item_values: std::sync::Arc::new(BTreeMap::new()),
            sqlx_namespaces: BTreeSet::from(["sqlx".to_owned()]),
            database_namespaces: BTreeSet::from(["wow_database".to_owned()]),
            query_callables: BTreeSet::new(),
            persistence_macros: BTreeMap::new(),
            package_persistence_macros: std::sync::Arc::new(BTreeMap::new()),
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
        || is_standard_identity(names)
}

fn is_standard_identity(names: &[String]) -> bool {
    matches!(
        names,
        [root, module, function]
            if matches!(root.as_str(), "std" | "core")
                && module == "convert"
                && function == "identity"
    )
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
                field_items: BTreeMap::new(),
                trait_bounds: trait_bounds_in_type(element, symbols),
                type_generic_params: Vec::new(),
                callable_signatures: BTreeSet::new(),
                closure_mutations: BTreeMap::new(),
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

fn instantiate_named_type_info(
    named: &VariableInfo,
    path: &syn::TypePath,
    symbols: &ModuleSymbols,
) -> VariableInfo {
    let mut instantiated = named.clone();
    let arguments = path
        .path
        .segments
        .last()
        .and_then(|segment| match &segment.arguments {
            syn::PathArguments::AngleBracketed(arguments) => Some(arguments),
            _ => None,
        })
        .into_iter()
        .flat_map(|arguments| &arguments.args)
        .filter_map(|argument| match argument {
            syn::GenericArgument::Type(ty) => Some(variable_info_in_type(ty, symbols)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if !arguments.is_empty() {
        let substitutions = instantiated
            .type_generic_params
            .iter()
            .cloned()
            .zip(arguments)
            .collect::<BTreeMap<_, _>>();
        substitute_nominal_params(&mut instantiated, &substitutions);
    }
    instantiated.type_generic_params.clear();
    instantiated
}

fn variable_info_in_type(ty: &Type, symbols: &ModuleSymbols) -> VariableInfo {
    let mut info = VariableInfo {
        flow: Flow::pools(&targets_in_type(ty, symbols)),
        sql_expression: SqlExpressionKind::Static,
        nominal_types: resolve_nominal_types(receiver_nominal_types_in_type(ty), symbols),
        payload_variants: payload_variants_in_type(ty, symbols),
        tuple_items: tuple_items_in_type(ty, symbols),
        field_items: BTreeMap::new(),
        trait_bounds: trait_bounds_in_type(ty, symbols),
        type_generic_params: Vec::new(),
        callable_signatures: BTreeSet::new(),
        closure_mutations: BTreeMap::new(),
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
    if let Type::Path(path) = ty {
        let names = path_names(&path.path);
        if names
            .first()
            .is_some_and(|name| matches!(name.as_str(), "crate" | "self" | "super"))
        {
            let canonical = canonical_path_names(names, symbols).join("::");
            if !canonical.is_empty() {
                info.nominal_types.insert(canonical);
            }
        }
    }
    let canonical = match ty {
        Type::Path(path) => Some(canonical_path_names(path_names(&path.path), symbols).join("::")),
        Type::Reference(reference) => return variable_info_in_type(&reference.elem, symbols),
        Type::Ptr(pointer) => return variable_info_in_type(&pointer.elem, symbols),
        Type::Paren(paren) => return variable_info_in_type(&paren.elem, symbols),
        Type::Group(group) => return variable_info_in_type(&group.elem, symbols),
        _ => None,
    };
    if let Some(canonical) = &canonical
        && let Some(named) = symbols.named_type_info.get(canonical)
        && let Type::Path(path) = ty
    {
        info.union(&instantiate_named_type_info(named, path, symbols));
    }
    if let Some(canonical) = &canonical
        && let Some(named) = symbols.workspace_named_type_info.get(canonical)
        && let Type::Path(path) = ty
    {
        info.union(&instantiate_named_type_info(named, path, symbols));
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
        if names.first().is_some_and(|name| name == "crate") {
            names.remove(0);
        }
        base.clear();
        absolute = true;
    }
    // Only direct Cargo dependencies may introduce an external crate root.
    // Rewrite dependency renames to the provider's canonical registry root;
    // unrelated workspace packages must remain ordinary relative paths.
    if !absolute
        && let Some(first) = names.first_mut()
        && let Some(provider_root) = symbols.dependency_crate_aliases.get(first)
    {
        *first = provider_root.clone();
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

fn collect_public_callable_reexports(
    items: &[Item],
    parent_symbols: &ModuleSymbols,
    cfg: &[String],
    source_class: PersistenceSourceClass,
    errors: &mut Vec<String>,
    output: &mut Vec<(String, String)>,
) {
    let mut symbols = parent_symbols.clone();
    // Resolve sibling aliases before interpreting a public re-export. This
    // mirrors module symbol collection without treating the re-export itself
    // as proof that its source is callable.
    for _ in 0..=items.len() {
        let mut changed = false;
        for item in items {
            if let Item::Use(item_use) = item
                && source_class_allows(
                    source_class,
                    cfg,
                    &item_use.attrs,
                    errors,
                    "use declaration",
                )
            {
                changed |= apply_import_symbols(item_use, &mut symbols);
            }
        }
        if !changed {
            break;
        }
    }
    for item in items {
        match item {
            Item::Use(item_use)
                if matches!(item_use.vis, Visibility::Public(_))
                    && source_class_allows(
                        source_class,
                        cfg,
                        &item_use.attrs,
                        errors,
                        "public use declaration",
                    ) =>
            {
                let (leaves, globs) = use_leaves(item_use);
                for leaf in leaves {
                    let mut export = symbols.module_path.clone();
                    export.push(leaf.local);
                    let mut source = canonical_path_names(leaf.source, &symbols);
                    if source.first().is_some_and(|segment| segment == "crate") {
                        source.remove(0);
                    }
                    output.push((export.join("::"), source.join("::")));
                }
                for glob in globs {
                    let mut export = symbols.module_path.clone();
                    export.push("*".to_owned());
                    let mut source = canonical_path_names(glob, &symbols);
                    if source.first().is_some_and(|segment| segment == "crate") {
                        source.remove(0);
                    }
                    source.push("*".to_owned());
                    output.push((export.join("::"), source.join("::")));
                }
            }
            Item::Mod(item_mod)
                if source_class_allows(
                    source_class,
                    cfg,
                    &item_mod.attrs,
                    errors,
                    "inline module",
                ) =>
            {
                if let Some((_, nested)) = &item_mod.content {
                    let mut nested_symbols = symbols.clone();
                    nested_symbols
                        .module_path
                        .push(normalized_ident(&item_mod.ident));
                    let nested_cfg = item_cfg(cfg, &item_mod.attrs);
                    collect_public_callable_reexports(
                        nested,
                        &nested_symbols,
                        &nested_cfg,
                        source_class,
                        errors,
                        output,
                    );
                }
            }
            _ => {}
        }
    }
}

fn collect_public_named_type_paths(
    items: &[Item],
    parent_symbols: &ModuleSymbols,
    cfg: &[String],
    source_class: PersistenceSourceClass,
    errors: &mut Vec<String>,
    output: &mut BTreeSet<String>,
) {
    for item in items {
        match item {
            Item::Type(alias)
                if matches!(alias.vis, Visibility::Public(_))
                    && source_class_allows(
                        source_class,
                        cfg,
                        &alias.attrs,
                        errors,
                        "public type alias",
                    ) =>
            {
                let mut path = parent_symbols.module_path.clone();
                path.push(normalized_ident(&alias.ident));
                output.insert(path.join("::"));
            }
            Item::Struct(item_struct)
                if matches!(item_struct.vis, Visibility::Public(_))
                    && source_class_allows(
                        source_class,
                        cfg,
                        &item_struct.attrs,
                        errors,
                        "public struct",
                    ) =>
            {
                let mut path = parent_symbols.module_path.clone();
                path.push(normalized_ident(&item_struct.ident));
                output.insert(path.join("::"));
            }
            Item::Enum(item_enum)
                if matches!(item_enum.vis, Visibility::Public(_))
                    && source_class_allows(
                        source_class,
                        cfg,
                        &item_enum.attrs,
                        errors,
                        "public enum",
                    ) =>
            {
                let mut path = parent_symbols.module_path.clone();
                path.push(normalized_ident(&item_enum.ident));
                output.insert(path.join("::"));
                for variant in &item_enum.variants {
                    if source_class_allows(
                        source_class,
                        cfg,
                        &variant.attrs,
                        errors,
                        "public enum variant",
                    ) {
                        let mut variant_path = path.clone();
                        variant_path.push(normalized_ident(&variant.ident));
                        output.insert(variant_path.join("::"));
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
                ) =>
            {
                if let Some((_, nested)) = &item_mod.content {
                    let mut nested_symbols = parent_symbols.clone();
                    nested_symbols
                        .module_path
                        .push(normalized_ident(&item_mod.ident));
                    collect_public_named_type_paths(
                        nested,
                        &nested_symbols,
                        &item_cfg(cfg, &item_mod.attrs),
                        source_class,
                        errors,
                        output,
                    );
                }
            }
            _ => {}
        }
    }
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
        let alias_source = if leaf.namespace_self {
            let mut absolute = vec!["crate".to_owned()];
            absolute.extend(canonical_source.iter().cloned());
            absolute
        } else {
            canonical_source.clone()
        };
        if leaf.local != "_" && symbols.path_aliases.get(&leaf.local) != Some(&alias_source) {
            symbols
                .path_aliases
                .insert(leaf.local.clone(), alias_source);
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
                let generic_params: Vec<String> = item_trait
                    .generics
                    .params
                    .iter()
                    .filter_map(|parameter| match parameter {
                        syn::GenericParam::Type(parameter) => {
                            Some(normalized_ident(&parameter.ident))
                        }
                        _ => None,
                    })
                    .collect();
                if !generic_params.is_empty() {
                    std::sync::Arc::make_mut(&mut symbols.trait_generic_params)
                        .insert(trait_path.clone(), generic_params);
                }
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
                    let method_generic_params = generic_type_param_names(&method.sig.generics);
                    if !method_generic_params.is_empty() {
                        std::sync::Arc::make_mut(&mut symbols.trait_method_generic_input_params)
                            .insert(
                                (trait_path.clone(), normalized_ident(&method.sig.ident)),
                                generic_params_by_input(&method.sig.inputs, &method_generic_params),
                            );
                        std::sync::Arc::make_mut(&mut symbols.trait_method_generic_params).insert(
                            (trait_path.clone(), normalized_ident(&method.sig.ident)),
                            method_generic_params,
                        );
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
                            std::sync::Arc::make_mut(&mut symbols.trait_method_returns)
                                .entry((trait_path.clone(), normalized_ident(&method.sig.ident)))
                                .or_default()
                                .union(&info);
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

fn collect_nested_item_values(
    items: &[Item],
    module_path: &[String],
    package: &str,
    cfg: &[String],
    source_class: PersistenceSourceClass,
    symbols: &ModuleSymbols,
    output: &mut BTreeMap<String, VariableInfo>,
    errors: &mut Vec<String>,
) {
    for item in items {
        match item {
            Item::Const(item_const)
                if source_class_allows(source_class, cfg, &item_const.attrs, errors, "const") =>
            {
                let mut path = module_path.to_vec();
                path.push(normalized_ident(&item_const.ident));
                output
                    .entry(path.join("::"))
                    .or_default()
                    .union(&variable_info_in_type(&item_const.ty, symbols));
            }
            Item::Static(item_static)
                if source_class_allows(source_class, cfg, &item_static.attrs, errors, "static") =>
            {
                let mut path = module_path.to_vec();
                path.push(normalized_ident(&item_static.ident));
                output
                    .entry(path.join("::"))
                    .or_default()
                    .union(&variable_info_in_type(&item_static.ty, symbols));
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
                let child_symbols = collect_module_symbols(
                    &item_mod.content.as_ref().expect("checked content").1,
                    Some(symbols),
                    package,
                    &child_path.join("::"),
                    &child_cfg,
                    source_class,
                    errors,
                );
                collect_nested_item_values(
                    &item_mod.content.as_ref().expect("checked content").1,
                    &child_path,
                    package,
                    &child_cfg,
                    source_class,
                    &child_symbols,
                    output,
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
                Item::Const(item_const)
                    if source_class_allows(
                        source_class,
                        cfg,
                        &item_const.attrs,
                        errors,
                        "const",
                    ) =>
                {
                    let info = variable_info_in_type(&item_const.ty, &symbols);
                    let entry = symbols
                        .item_values
                        .entry(normalized_ident(&item_const.ident))
                        .or_default();
                    let before = entry.clone();
                    entry.union(&info);
                    changed |= *entry != before;
                }
                Item::Static(item_static)
                    if source_class_allows(
                        source_class,
                        cfg,
                        &item_static.attrs,
                        errors,
                        "static",
                    ) =>
                {
                    let info = variable_info_in_type(&item_static.ty, &symbols);
                    let entry = symbols
                        .item_values
                        .entry(normalized_ident(&item_static.ident))
                        .or_default();
                    let before = entry.clone();
                    entry.union(&info);
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
            Item::Macro(item_macro)
                if source_class_allows(
                    source_class,
                    cfg,
                    &item_macro.attrs,
                    errors,
                    "macro definition",
                ) =>
            {
                // A `macro_rules!` whose body reaches concrete persistence is
                // inventoried at its definition site, but that row covers the
                // definition only: register the macro name so `audit_macro`
                // inventories every later invocation instead of letting a new
                // call site slip past both ratchets unannotated.
                if let Some(name) = &item_macro.ident {
                    let targets = targets_in_tokens(item_macro.mac.tokens.clone(), &symbols);
                    if !targets.is_empty() {
                        symbols
                            .persistence_macros
                            .insert(normalized_ident(name), targets);
                    }
                }
            }
            Item::Fn(function)
                if source_class_allows(source_class, cfg, &function.attrs, errors, "function") =>
            {
                let generic_params = generic_type_param_names(&function.sig.generics);
                if !generic_params.is_empty() {
                    symbols.function_generic_input_params.insert(
                        normalized_ident(&function.sig.ident),
                        generic_params_by_input(&function.sig.inputs, &generic_params),
                    );
                    symbols
                        .function_generic_params
                        .insert(normalized_ident(&function.sig.ident), generic_params);
                }
                if let ReturnType::Type(_, ty) = &function.sig.output {
                    let mut return_info = variable_info_in_type(ty, &symbols);
                    return_info.sql_expression = SqlExpressionKind::Nonliteral;
                    if !return_info.flow.is_empty()
                        || !return_info.nominal_types.is_empty()
                        || !return_info.payload_variants.is_empty()
                        || !return_info.tuple_items.is_empty()
                        || !return_info.trait_bounds.is_empty()
                    {
                        symbols
                            .function_returns
                            .entry(normalized_ident(&function.sig.ident))
                            .or_default()
                            .union(&return_info);
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
                let mut receiver_types = nominal_types_in_type(&item_impl.self_ty)
                    .into_iter()
                    .flat_map(|nominal| {
                        symbols
                            .nominal_type_aliases
                            .get(&nominal)
                            .cloned()
                            .unwrap_or_else(|| BTreeSet::from([nominal]))
                    })
                    .collect::<BTreeSet<_>>();
                if let Type::Path(path) = item_impl.self_ty.as_ref() {
                    let mut names = path_names(&path.path);
                    if names.first().is_some_and(|name| name == "crate") {
                        names.remove(0);
                    }
                    if names.len() > 1 {
                        receiver_types.insert(names.join("::"));
                    }
                }
                let associated_types = item_impl
                    .items
                    .iter()
                    .filter_map(|item| {
                        let ImplItem::Type(associated) = item else {
                            return None;
                        };
                        source_class_allows(
                            source_class,
                            cfg,
                            &associated.attrs,
                            errors,
                            "impl associated type",
                        )
                        .then(|| {
                            (
                                normalized_ident(&associated.ident),
                                variable_info_in_type(&associated.ty, &symbols),
                            )
                        })
                    })
                    .collect::<BTreeMap<_, _>>();
                if let Some(trait_name) = &trait_name {
                    let trait_arguments = item_impl
                        .trait_
                        .as_ref()
                        .and_then(|(_, path, _)| path.segments.last())
                        .and_then(|segment| match &segment.arguments {
                            syn::PathArguments::AngleBracketed(arguments) => Some(arguments),
                            _ => None,
                        });
                    let trait_params = symbols
                        .trait_generic_params
                        .iter()
                        .find(|(candidate, _)| {
                            *candidate == trait_name
                                || candidate.ends_with(&format!("::{trait_name}"))
                                || trait_name.ends_with(&format!("::{candidate}"))
                        })
                        .map(|(_, params)| params.clone())
                        .unwrap_or_default();
                    let trait_substitutions = trait_arguments
                        .map(|arguments| {
                            trait_params
                                .iter()
                                .zip(arguments.args.iter().filter_map(|argument| match argument {
                                    syn::GenericArgument::Type(ty) => {
                                        Some(variable_info_in_type(ty, &symbols))
                                    }
                                    _ => None,
                                }))
                                .map(|(param, info)| (param.clone(), info))
                                .collect::<BTreeMap<_, _>>()
                        })
                        .unwrap_or_default();
                    let inherited = symbols
                        .trait_method_returns
                        .iter()
                        .filter(|((candidate_trait, _), _)| {
                            candidate_trait == trait_name
                                || candidate_trait.ends_with(&format!("::{trait_name}"))
                                || trait_name.ends_with(&format!("::{candidate_trait}"))
                        })
                        .map(|((_, method), info)| (method.clone(), info.clone()))
                        .collect::<Vec<_>>();
                    for (method_name, mut return_info) in inherited {
                        substitute_nominal_params(&mut return_info, &trait_substitutions);
                        substitute_nominal_params(&mut return_info, &associated_types);
                        let generic_params = symbols
                            .trait_method_generic_params
                            .iter()
                            .find(|((candidate_trait, candidate_method), _)| {
                                candidate_method == &method_name
                                    && (candidate_trait == trait_name
                                        || candidate_trait.ends_with(&format!("::{trait_name}"))
                                        || trait_name.ends_with(&format!("::{candidate_trait}")))
                            })
                            .map(|(_, params)| params.clone());
                        let generic_input_params = symbols
                            .trait_method_generic_input_params
                            .iter()
                            .find(|((candidate_trait, candidate_method), _)| {
                                candidate_method == &method_name
                                    && (candidate_trait == trait_name
                                        || candidate_trait.ends_with(&format!("::{trait_name}"))
                                        || trait_name.ends_with(&format!("::{candidate_trait}")))
                            })
                            .map(|(_, params)| params.clone());
                        for receiver_type in &receiver_types {
                            let key = (
                                receiver_type.clone(),
                                Some(trait_name.clone()),
                                method_name.clone(),
                            );
                            symbols
                                .method_returns
                                .entry(key.clone())
                                .or_default()
                                .union(&return_info);
                            if let Some(generic_params) = &generic_params {
                                symbols
                                    .method_generic_params
                                    .insert(key.clone(), generic_params.clone());
                            }
                            if let Some(generic_input_params) = &generic_input_params {
                                symbols
                                    .method_generic_input_params
                                    .insert(key, generic_input_params.clone());
                            }
                        }
                    }
                }
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
                        // `Self::Product` is represented by syn as a nominal
                        // projection. Resolve the implementation's associated
                        // binding before recording this concrete method.
                        substitute_nominal_params(&mut return_info, &associated_types);
                        return_info.sql_expression = SqlExpressionKind::Nonliteral;
                        let generic_params = generic_type_param_names(&method.sig.generics);
                        if !generic_params.is_empty() {
                            let method_name = normalized_ident(&method.sig.ident);
                            let input_params =
                                generic_params_by_input(&method.sig.inputs, &generic_params);
                            for receiver_type in &receiver_types {
                                symbols.method_generic_input_params.insert(
                                    (
                                        receiver_type.clone(),
                                        trait_name.clone(),
                                        method_name.clone(),
                                    ),
                                    input_params.clone(),
                                );
                                symbols.method_generic_params.insert(
                                    (
                                        receiver_type.clone(),
                                        trait_name.clone(),
                                        method_name.clone(),
                                    ),
                                    generic_params.clone(),
                                );
                            }
                        }
                        if !return_info.flow.is_empty()
                            || !return_info.nominal_types.is_empty()
                            || !return_info.payload_variants.is_empty()
                            || !return_info.tuple_items.is_empty()
                            || !return_info.trait_bounds.is_empty()
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
    let field_targets = symbols.field_targets.clone();
    let tuple_field_targets = symbols.tuple_field_targets.clone();
    let field_nominal_types = symbols.field_nominal_types.clone();
    for info in std::sync::Arc::make_mut(&mut symbols.trait_method_returns).values_mut() {
        for owner in info.nominal_types.clone() {
            for ((candidate_owner, field), targets) in &field_targets {
                if candidate_owner == &owner {
                    info.field_items
                        .entry(field.clone())
                        .or_default()
                        .flow
                        .union(Flow::pools(targets));
                }
            }
            for ((candidate_owner, field), targets) in &tuple_field_targets {
                if candidate_owner == &owner {
                    info.field_items
                        .entry(field.clone())
                        .or_default()
                        .flow
                        .union(Flow::pools(targets));
                }
            }
            for ((candidate_owner, field), nominal_types) in &field_nominal_types {
                if candidate_owner == &owner {
                    info.field_items
                        .entry(field.clone())
                        .or_default()
                        .nominal_types
                        .extend(nominal_types.iter().cloned());
                }
            }
        }
    }
    symbols
}

fn collect_named_type_info(
    items: &[Item],
    parent: &ModuleSymbols,
    package: &str,
    module: &str,
    cfg: &[String],
    source_class: PersistenceSourceClass,
    errors: &mut Vec<String>,
    output: &mut BTreeMap<String, VariableInfo>,
) {
    let symbols = collect_module_symbols(
        items,
        Some(parent),
        package,
        module,
        cfg,
        source_class,
        errors,
    );
    for item in items {
        match item {
            Item::Type(alias)
                if source_class_allows(source_class, cfg, &alias.attrs, errors, "type alias") =>
            {
                let mut path = symbols.module_path.clone();
                path.push(normalized_ident(&alias.ident));
                output
                    .entry(path.join("::"))
                    .or_default()
                    .union(&VariableInfo {
                        type_generic_params: generic_type_param_names(&alias.generics),
                        ..variable_info_in_type(&alias.ty, &symbols)
                    });
            }
            Item::Struct(item_struct)
                if source_class_allows(source_class, cfg, &item_struct.attrs, errors, "struct") =>
            {
                let mut path = symbols.module_path.clone();
                path.push(normalized_ident(&item_struct.ident));
                let entry = output.entry(path.join("::")).or_default();
                entry.type_generic_params = generic_type_param_names(&item_struct.generics);
                for (index, field) in item_struct.fields.iter().enumerate() {
                    if source_class_allows(source_class, cfg, &field.attrs, errors, "struct field")
                    {
                        let name = field
                            .ident
                            .as_ref()
                            .map(normalized_ident)
                            .unwrap_or_else(|| index.to_string());
                        let field_info = variable_info_in_type(&field.ty, &symbols);
                        // Moving, returning, or passing the whole nominal
                        // value also moves every persistence-bearing field.
                        entry.flow.union(field_info.flow.clone());
                        entry
                            .field_items
                            .entry(name)
                            .or_default()
                            .union(&field_info);
                    }
                }
            }
            Item::Enum(item_enum)
                if source_class_allows(source_class, cfg, &item_enum.attrs, errors, "enum") =>
            {
                let mut path = symbols.module_path.clone();
                path.push(normalized_ident(&item_enum.ident));
                let enum_path = path.join("::");
                let enum_generic_params = generic_type_param_names(&item_enum.generics);
                output
                    .entry(enum_path.clone())
                    .or_default()
                    .type_generic_params = enum_generic_params.clone();
                let mut enum_flow = Flow::default();
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
                    // Each variant is also registered as `Enum::Variant` with
                    // its own fields, so a pattern such as
                    // `Product::Database(database)` can recover the payload
                    // through `declared_field_info` across source files.
                    let mut variant_path = symbols.module_path.clone();
                    variant_path.push(normalized_ident(&item_enum.ident));
                    variant_path.push(normalized_ident(&variant.ident));
                    let variant_entry = output.entry(variant_path.join("::")).or_default();
                    variant_entry.type_generic_params = enum_generic_params.clone();
                    let mut shapes = Vec::new();
                    for (index, field) in variant.fields.iter().enumerate() {
                        if !source_class_allows(
                            source_class,
                            cfg,
                            &field.attrs,
                            errors,
                            "enum variant field",
                        ) {
                            continue;
                        }
                        let field_info = variable_info_in_type(&field.ty, &symbols);
                        let name = field
                            .ident
                            .as_ref()
                            .map(normalized_ident)
                            .unwrap_or_else(|| index.to_string());
                        variant_entry
                            .field_items
                            .entry(name)
                            .or_default()
                            .union(&field_info);
                        variant_entry.flow.union(field_info.flow.clone());
                        enum_flow.union(field_info.flow.clone());
                        shapes.push(nominal_shape_in_type(&field.ty, &symbols).unwrap_or(
                            NominalShape {
                                nominal_types: BTreeSet::new(),
                                arguments: Vec::new(),
                            },
                        ));
                    }
                    if !shapes.is_empty() {
                        output
                            .entry(enum_path.clone())
                            .or_default()
                            .payload_variants
                            .insert(shapes);
                    }
                }
                output.entry(enum_path).or_default().flow.union(enum_flow);
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
                let child_module = format!("{module}::{}", normalized_ident(&item_mod.ident));
                collect_named_type_info(
                    &item_mod.content.as_ref().expect("checked content").1,
                    &symbols,
                    package,
                    &child_module,
                    &item_cfg(cfg, &item_mod.attrs),
                    source_class,
                    errors,
                    output,
                );
            }
            _ => {}
        }
    }
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

fn tokens_contain_callable_invocation(tokens: TokenStream, names: &BTreeSet<String>) -> bool {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    for (index, token) in tokens.iter().enumerate() {
        if let TokenTree::Group(group) = token
            && tokens_contain_callable_invocation(group.stream(), names)
        {
            return true;
        }
        if let TokenTree::Ident(ident) = token
            && names.contains(&normalized_ident(ident))
            && matches!(
                tokens.get(index + 1),
                Some(TokenTree::Group(group))
                    if group.delimiter() == proc_macro2::Delimiter::Parenthesis
            )
        {
            return true;
        }
    }
    false
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

/// Extracts concrete persistence targets from fully qualified adapter paths
/// (`wow_database::CharacterDatabase::open(...)`) inside an opaque token
/// stream, reusing the same name mapping as ordinary paths. Without this an
/// allowed opaque macro such as `assert!(wow_database::...)` would emit
/// neither a database row nor a fail-closed error.
fn database_targets_in_tokens(tokens: TokenStream, symbols: &ModuleSymbols) -> TargetSet {
    let mut targets = TargetSet::new();
    let trees = tokens.into_iter().collect::<Vec<_>>();
    let mut index = 0;
    while index < trees.len() {
        if let TokenTree::Group(group) = &trees[index] {
            targets.extend(database_targets_in_tokens(group.stream(), symbols));
        }
        if let TokenTree::Ident(ident) = &trees[index] {
            let root = normalized_ident(ident);
            if symbols.database_namespaces.contains(&root) {
                let mut names = vec![root];
                let mut cursor = index + 1;
                while cursor + 2 < trees.len()
                    && matches!(&trees[cursor], TokenTree::Punct(punct) if punct.as_char() == ':')
                    && matches!(&trees[cursor + 1], TokenTree::Punct(punct) if punct.as_char() == ':')
                {
                    let TokenTree::Ident(segment) = &trees[cursor + 2] else {
                        break;
                    };
                    names.push(normalized_ident(segment));
                    cursor += 3;
                }
                if names.len() > 1 {
                    targets.extend(targets_for_names(&names, symbols));
                }
            }
        }
        index += 1;
    }
    targets
}

fn targets_in_tokens(tokens: TokenStream, symbols: &ModuleSymbols) -> TargetSet {
    let mut targets = TargetSet::new();
    if tokens_contain_path_root(tokens.clone(), &symbols.sqlx_namespaces) {
        targets.insert(PersistenceTarget::Sqlx);
    }
    targets.extend(database_targets_in_tokens(tokens.clone(), symbols));
    for (name, alias_targets) in &symbols.type_aliases {
        if tokens_contain_identifier(tokens.clone(), &BTreeSet::from([name.clone()])) {
            targets.extend(alias_targets);
        }
    }
    targets
}

fn cfg_predicate_allows_source(
    predicate: &syn::Meta,
    source_class: PersistenceSourceClass,
) -> bool {
    let attribute: Attribute = syn::parse_quote!(#[cfg(#predicate)]);
    match source_class {
        PersistenceSourceClass::Production => {
            cfg_context_allows_production(&[], &[attribute]).unwrap_or(false)
        }
        PersistenceSourceClass::TestFixture => {
            cfg_context_allows_test(&[], &[attribute]).unwrap_or(false)
        }
    }
}

fn targets_in_attribute_meta(
    meta: &syn::Meta,
    symbols: &ModuleSymbols,
    source_class: PersistenceSourceClass,
    inherited_cfg: &[String],
) -> Vec<(PersistenceTarget, Vec<String>)> {
    if meta.path().is_ident("cfg") {
        return Vec::new();
    }
    if meta.path().is_ident("cfg_attr") {
        let syn::Meta::List(list) = meta else {
            return Vec::new();
        };
        let Ok(items) = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
            .parse2(list.tokens.clone())
        else {
            return Vec::new();
        };
        let mut items = items.iter();
        let Some(predicate) = items.next() else {
            return Vec::new();
        };
        if !cfg_predicate_allows_source(predicate, source_class) {
            return Vec::new();
        }
        let conditional: Attribute = syn::parse_quote!(#[cfg(#predicate)]);
        let mut nested_cfg = inherited_cfg.to_vec();
        nested_cfg.push(conditional.meta.to_token_stream().to_string());
        nested_cfg.sort();
        nested_cfg.dedup();
        let mut targets = Vec::new();
        for nested in items {
            targets.extend(targets_in_attribute_meta(
                nested,
                symbols,
                source_class,
                &nested_cfg,
            ));
        }
        return targets;
    }
    let mut targets = targets_for_path(meta.path(), symbols);
    targets.extend(targets_in_tokens(meta.to_token_stream(), symbols));
    targets
        .into_iter()
        .map(|target| (target, inherited_cfg.to_vec()))
        .collect()
}

fn targets_in_attributes(
    attribute: &Attribute,
    symbols: &ModuleSymbols,
    source_class: PersistenceSourceClass,
) -> Vec<(PersistenceTarget, Vec<String>)> {
    targets_in_attribute_meta(&attribute.meta, symbols, source_class, &[])
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
        for (target, conditional_cfg) in
            targets_in_attributes(attribute, symbols, context.source_class)
        {
            let mut cfg = record.cfg.to_vec();
            cfg.extend(conditional_cfg);
            cfg.sort();
            cfg.dedup();
            accumulator.add(
                context,
                NewAccess {
                    enclosing: record.enclosing,
                    target,
                    operation: PersistenceOperation::MacroReference,
                    symbol: &symbol,
                    visibility: record.visibility,
                    cfg: &cfg,
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
    generic_trait_bound_args: BTreeMap<(String, String), Vec<VariableInfo>>,
    generic_trait_bound_associated: BTreeMap<(String, String), BTreeMap<String, VariableInfo>>,
    flow_cache: std::cell::RefCell<BTreeMap<(usize, u64), Flow>>,
    subtree_flow_cache: std::cell::RefCell<BTreeMap<(usize, u64), Flow>>,
    closure_effects: std::cell::RefCell<BTreeMap<usize, BTreeMap<String, VariableInfo>>>,
    closure_result_infos: std::cell::RefCell<BTreeMap<usize, VariableInfo>>,
    block_result_infos: std::cell::RefCell<BTreeMap<usize, VariableInfo>>,
    loop_flow_collectors: Vec<LoopFlowCollector>,
    context_version: u64,
    suppress_records: bool,
}

#[derive(Default)]
struct LoopFlowCollector {
    label: Option<String>,
    exits: Option<Vec<BTreeMap<String, VariableInfo>>>,
    back_edges: Option<Vec<BTreeMap<String, VariableInfo>>>,
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
            self.flow.union(self.analyzer.subtree_flow(expression));
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
            generic_trait_bound_args: BTreeMap::new(),
            generic_trait_bound_associated: BTreeMap::new(),
            flow_cache: std::cell::RefCell::new(BTreeMap::new()),
            subtree_flow_cache: std::cell::RefCell::new(BTreeMap::new()),
            closure_effects: std::cell::RefCell::new(BTreeMap::new()),
            closure_result_infos: std::cell::RefCell::new(BTreeMap::new()),
            block_result_infos: std::cell::RefCell::new(BTreeMap::new()),
            loop_flow_collectors: Vec::new(),
            context_version: 0,
            suppress_records: false,
        }
    }

    fn analyze_without_records(&mut self, analyze: impl FnOnce(&mut Self)) {
        let previous = self.suppress_records;
        let error_count = self.errors.len();
        self.suppress_records = true;
        analyze(self);
        self.suppress_records = previous;
        self.errors.truncate(error_count);
    }

    fn bump_context(&mut self) {
        self.context_version = self.context_version.wrapping_add(1);
        self.flow_cache.get_mut().clear();
        self.subtree_flow_cache.get_mut().clear();
    }

    fn visit_loop_block(&mut self, block: &syn::Block, label: Option<String>) -> LoopFlowCollector {
        self.loop_flow_collectors.push(LoopFlowCollector {
            label,
            ..LoopFlowCollector::default()
        });
        self.visit_block(block);
        self.loop_flow_collectors
            .pop()
            .expect("loop flow collector was installed")
    }

    fn visit_for_loop_body(
        &mut self,
        expression: &syn::ExprForLoop,
        iterator_info: &VariableInfo,
        label: Option<String>,
    ) -> LoopFlowCollector {
        self.loop_flow_collectors.push(LoopFlowCollector {
            label,
            ..LoopFlowCollector::default()
        });
        self.push_scope();
        self.register_local_uses(&expression.body.stmts);
        self.register_local_callables(&expression.body.stmts);
        self.bind_pattern(&expression.pat, iterator_info);
        for statement in &expression.body.stmts {
            self.visit_stmt(statement);
        }
        self.pop_scope();
        self.loop_flow_collectors
            .pop()
            .expect("for-loop flow collector was installed")
    }

    fn visit_while_loop_body(
        &mut self,
        expression: &syn::ExprWhile,
        label: Option<String>,
    ) -> LoopFlowCollector {
        self.loop_flow_collectors.push(LoopFlowCollector {
            label,
            ..LoopFlowCollector::default()
        });
        self.push_scope();
        self.register_local_uses(&expression.body.stmts);
        self.register_local_callables(&expression.body.stmts);
        visit_let_chain_condition(self, &expression.cond, true);
        // Evaluating a false condition exits the loop with every mutation
        // performed by that condition, before the body can overwrite it.
        self.capture_loop_control(None, true);
        for statement in &expression.body.stmts {
            self.visit_stmt(statement);
        }
        self.pop_scope();
        self.loop_flow_collectors
            .pop()
            .expect("while-loop flow collector was installed")
    }

    fn capture_loop_control(&mut self, label: Option<&syn::Lifetime>, is_exit: bool) {
        let target = label
            .map(|label| normalized_ident(&label.ident))
            .and_then(|label| {
                self.loop_flow_collectors
                    .iter()
                    .rposition(|collector| collector.label.as_deref() == Some(label.as_str()))
            })
            .or_else(|| self.loop_flow_collectors.len().checked_sub(1));
        let Some(target) = target else {
            return;
        };
        let scopes = self.scopes.clone();
        let collector = &mut self.loop_flow_collectors[target];
        let destination = if is_exit {
            &mut collector.exits
        } else {
            &mut collector.back_edges
        };
        match destination {
            None => *destination = Some(scopes),
            Some(accumulated) => merge_scope_stacks(accumulated, &scopes),
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

    /// Canonical lookup key into the package-wide free-function registry.
    /// Local block imports expand to their absolute `crate::…` source while
    /// module-level aliases already drop the leading `crate`, so normalize
    /// both forms to the registry's crate-relative shape.
    fn package_function_key(&self, names: Vec<String>) -> String {
        let mut names = self.canonical_local_path_names(names);
        if names.first().is_some_and(|first| first == "crate") {
            names.remove(0);
        }
        names.join("::")
    }

    fn register_local_uses(&mut self, statements: &[Stmt]) {
        self.bump_context();
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

    fn register_local_callables(&mut self, statements: &[Stmt]) {
        for function in statements.iter().filter_map(|statement| match statement {
            Stmt::Item(Item::Fn(function)) => Some(function),
            _ => None,
        }) {
            if !source_class_allows(
                self.context.source_class,
                &self.cfg,
                &function.attrs,
                self.errors,
                "block-local function",
            ) {
                continue;
            }
            let ReturnType::Type(_, ty) = &function.sig.output else {
                continue;
            };
            let mut info = self.info_from_type(ty);
            info.sql_expression = SqlExpressionKind::Nonliteral;
            let generic_params = generic_type_param_names(&function.sig.generics);
            if !generic_params.is_empty() {
                info.callable_signatures.insert(CallableSignature {
                    generic_inputs: generic_params_by_input(&function.sig.inputs, &generic_params),
                    generic_params,
                });
            }
            if !info.flow.is_empty()
                || !info.nominal_types.is_empty()
                || !info.payload_variants.is_empty()
                || !info.tuple_items.is_empty()
                || !info.field_items.is_empty()
                || !info.trait_bounds.is_empty()
                || !info.callable_signatures.is_empty()
            {
                self.bind(normalized_ident(&function.sig.ident), info);
            }
        }
    }

    fn push_scope(&mut self) {
        self.bump_context();
        self.scopes.push(BTreeMap::new());
        self.local_path_alias_scopes.push(BTreeMap::new());
        self.anonymous_trait_scopes.push(BTreeSet::new());
    }

    fn pop_scope(&mut self) {
        self.bump_context();
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
        if self.suppress_records {
            return;
        }
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
        if self.suppress_records {
            return;
        }
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
        if allowed && !self.suppress_records {
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
        self.bump_context();
        self.scopes
            .last_mut()
            .expect("body analyzer always has a scope")
            .insert(name, info);
    }

    fn assign(&mut self, name: &str, info: VariableInfo) {
        self.bump_context();
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
        // The receiver binding itself may carry trait bounds (e.g. `self` in
        // a default trait body is bound to its own trait), not only generic
        // parameters of the enclosing function.
        trait_bounds.extend(self.shallow_trait_bounds_of_expr(&method.receiver));
        if receiver_types.is_empty() {
            trait_bounds.extend(self.shallow_trait_bounds_of_expr(&method.receiver));
        }
        for receiver_type in &receiver_types {
            if let Some(info) =
                self.symbols
                    .method_returns
                    .get(&(receiver_type.clone(), None, method_name.clone()))
            {
                let key = (receiver_type.clone(), None, method_name.clone());
                let params = self.symbols.method_generic_params.get(&key);
                let info = self.apply_turbofish_args(info, params, method.turbofish.as_ref());
                let info = self.apply_inferred_method_args(
                    &info,
                    params,
                    self.symbols.method_generic_input_params.get(&key),
                    &method.args,
                );
                result.union(&info);
                continue;
            }
            let mut trait_impl_hit = false;
            for ((owner, trait_name, candidate), info) in &self.symbols.method_returns {
                if owner == receiver_type
                    && trait_name
                        .as_ref()
                        .is_some_and(|trait_name| self.trait_is_in_scope(trait_name))
                    && candidate == &method_name
                {
                    let key = (owner.clone(), trait_name.clone(), candidate.clone());
                    let params = self.symbols.method_generic_params.get(&key);
                    let info = self.apply_turbofish_args(info, params, method.turbofish.as_ref());
                    let info = self.apply_inferred_method_args(
                        &info,
                        params,
                        self.symbols.method_generic_input_params.get(&key),
                        &method.args,
                    );
                    result.union(&info);
                    trait_impl_hit = true;
                }
            }
            if !trait_impl_hit {
                // Inherent impl declared in another source module: resolve
                // through the package-wide registry keyed by canonical owner
                // path, the same way free functions already resolve.
                let owner_key = if receiver_type.contains("::") {
                    receiver_type
                        .strip_prefix("crate::")
                        .unwrap_or(receiver_type)
                        .to_owned()
                } else {
                    self.package_function_key(vec![receiver_type.clone()])
                };
                if let Some(info) = self
                    .symbols
                    .package_method_returns
                    .get(&(owner_key.clone(), method_name.clone()))
                {
                    let key = (owner_key.clone(), method_name.clone());
                    let params = self.symbols.package_method_generic_params.get(&key);
                    let info = self.apply_turbofish_args(info, params, method.turbofish.as_ref());
                    let info = self.apply_inferred_method_args(
                        &info,
                        params,
                        self.symbols.package_method_generic_input_params.get(&key),
                        &method.args,
                    );
                    result.union(&info);
                }
            }
        }
        let expanded_trait_bounds = self.expand_trait_bounds(trait_bounds);
        for trait_bound in &expanded_trait_bounds {
            if let Some(info) = self
                .symbols
                .trait_method_returns
                .get(&(trait_bound.clone(), method_name.clone()))
            {
                let info = self.apply_bound_generic_args(info, trait_bound, &receiver_types);
                let key = (trait_bound.clone(), method_name.clone());
                let params = self.symbols.trait_method_generic_params.get(&key);
                let info = self.apply_turbofish_args(&info, params, method.turbofish.as_ref());
                let info = self.apply_inferred_method_args(
                    &info,
                    params,
                    self.symbols.trait_method_generic_input_params.get(&key),
                    &method.args,
                );
                result.union(&info);
            }
        }
        result.substitute_self(&receiver_types, &expanded_trait_bounds);
        if method_name == "recv" {
            result
                .payload_variants
                .extend(self.info_from_expr(&method.receiver).payload_variants);
        }
        if result.flow.is_empty()
            && result.nominal_types.is_empty()
            && result.payload_variants.is_empty()
            && result.tuple_items.is_empty()
            && result.trait_bounds.is_empty()
        {
            // The callee is not recorded anywhere (external or unmodelled).
            // A persistence-bearing turbofish can still select the return
            // type (`factory.make::<CharacterDatabase>()`), so keep the
            // argument visible instead of dropping the call's result.
            result = self.apply_turbofish_args(&result, None, method.turbofish.as_ref());
        }
        result
    }

    fn expand_trait_bounds(&self, trait_bounds: BTreeSet<String>) -> BTreeSet<String> {
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
        expanded_trait_bounds
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
            Expr::Call(call) => match call.func.as_ref() {
                Expr::Path(path) if path.path.segments.len() == 1 => last_path_name(&path.path)
                    .and_then(|name| {
                        self.lookup(&name)
                            .or_else(|| self.symbols.function_returns.get(&name))
                    })
                    .map(|info| info.trait_bounds.clone())
                    .unwrap_or_default(),
                Expr::Path(path) => {
                    self.associated_return_info(path, Some(&call.args))
                        .trait_bounds
                }
                _ => BTreeSet::new(),
            },
            Expr::MethodCall(method) => self.method_return_info(method).trait_bounds,
            _ => BTreeSet::new(),
        }
    }

    fn associated_return_info(
        &self,
        expression: &syn::ExprPath,
        call_args: Option<&syn::punctuated::Punctuated<Expr, syn::token::Comma>>,
    ) -> VariableInfo {
        let path = &expression.path;
        if expression.qself.is_none() && path.segments.len() < 2 {
            return VariableInfo::default();
        }
        let method_name = normalized_ident(&path.segments.last().expect("path has a method").ident);
        // An explicit turbofish on the callee (`Factory::make::<T>()`)
        // selects generic returns; it must be substituted into the recorded
        // return below instead of being dropped.
        let turbofish = path
            .segments
            .last()
            .and_then(|segment| match &segment.arguments {
                syn::PathArguments::AngleBracketed(arguments) => Some(arguments),
                _ => None,
            });
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
        let explicit_owner_key = expression.qself.is_none().then(|| {
            let names = path_names(path);
            self.package_function_key(names[..names.len().saturating_sub(1)].to_vec())
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
                let mut owners = self
                    .symbols
                    .nominal_type_aliases
                    .get(&owner)
                    .cloned()
                    .unwrap_or_else(|| BTreeSet::from([owner]));
                if let Some(owner_key) = &explicit_owner_key {
                    owners.insert(owner_key.clone());
                }
                owners
            }
        };
        let mut result = VariableInfo::default();
        for receiver_type in &receiver_types {
            let key = (
                receiver_type.clone(),
                trait_name.clone(),
                method_name.clone(),
            );
            if let Some(info) = self.symbols.method_returns.get(&key) {
                let params = self.symbols.method_generic_params.get(&key);
                let info = self.apply_turbofish_args(info, params, turbofish);
                let info = self.apply_optional_inferred_args(
                    &info,
                    params,
                    self.symbols.method_generic_input_params.get(&key),
                    call_args,
                );
                result.union(&info);
            }
        }
        let mut bounds = BTreeSet::new();
        if let Some(trait_name) = trait_name {
            bounds.insert(trait_name);
        }
        for receiver_type in &receiver_types {
            if let Some(generic_bounds) = self.generic_trait_bounds.get(receiver_type) {
                bounds.extend(generic_bounds.iter().cloned());
            }
        }
        let expanded = self.expand_trait_bounds(bounds);
        for trait_bound in &expanded {
            if let Some(info) = self
                .symbols
                .trait_method_returns
                .get(&(trait_bound.clone(), method_name.clone()))
            {
                let info = self.apply_bound_generic_args(info, trait_bound, &receiver_types);
                let key = (trait_bound.clone(), method_name.clone());
                let params = self.symbols.trait_method_generic_params.get(&key);
                let info = self.apply_turbofish_args(&info, params, turbofish);
                let info = self.apply_optional_inferred_args(
                    &info,
                    params,
                    self.symbols.trait_method_generic_input_params.get(&key),
                    call_args,
                );
                result.union(&info);
            }
        }
        if result.flow.is_empty()
            && result.nominal_types.is_empty()
            && result.payload_variants.is_empty()
            && result.tuple_items.is_empty()
            && result.trait_bounds.is_empty()
        {
            // Inherent impl declared in another module
            // (`crate::dto::Factory::make()`): the owner path is the callee
            // path minus its method segment, resolved package-wide. No early
            // return: `-> Self` returns still need the substitute below, and
            // a persistence-typed owner path contributes its flow like the
            // ordinary call fallback does.
            if let Some(owner_key) = &explicit_owner_key
                && let Some(info) = self
                    .symbols
                    .package_method_returns
                    .get(&(owner_key.clone(), method_name.clone()))
            {
                let key = (owner_key.clone(), method_name.clone());
                let params = self.symbols.package_method_generic_params.get(&key);
                let mut info = self.apply_turbofish_args(info, params, turbofish);
                info = self.apply_optional_inferred_args(
                    &info,
                    params,
                    self.symbols.package_method_generic_input_params.get(&key),
                    call_args,
                );
                info.flow
                    .union(Flow::pools(&targets_for_path(path, self.symbols)));
                result.union(&info);
            }
        }
        if result.flow.is_empty()
            && result.nominal_types.is_empty()
            && result.payload_variants.is_empty()
            && result.tuple_items.is_empty()
            && result.trait_bounds.is_empty()
        {
            // Qualified free-function calls (`crate::factory::database()`)
            // resolve through the package-wide canonical-path registry, the
            // same way trait returns already do.
            let key = self.package_function_key(path_names(path));
            if let Some(info) = self.symbols.package_function_returns.get(&key) {
                let params = self.symbols.package_function_generic_params.get(&key);
                let info = self.apply_turbofish_args(info, params, turbofish);
                return self.apply_optional_inferred_args(
                    &info,
                    params,
                    self.symbols.package_function_generic_input_params.get(&key),
                    call_args,
                );
            }
        }
        if result.substitute_self(&receiver_types, &expanded) {
            // A dependency method may declare `-> Self`. Substitution must
            // recover that exact owner's registered fields (for example
            // `wow_database::DbUpdater::new(...).pool`) without broadly
            // expanding ordinary short return names such as `Holder`, which
            // can legitimately occur in many modules and dependencies.
            for receiver_type in &receiver_types {
                let dependency_owner = receiver_type.split("::").next().is_some_and(|root| {
                    self.symbols
                        .dependency_crate_aliases
                        .values()
                        .any(|provider_root| provider_root == root)
                });
                if dependency_owner
                    && let Some(named) = self
                        .symbols
                        .workspace_named_type_info
                        .get(receiver_type)
                        .cloned()
                {
                    result.union(&named);
                }
            }
        }
        if result.flow.is_empty()
            && result.nominal_types.is_empty()
            && result.payload_variants.is_empty()
            && result.tuple_items.is_empty()
            && result.trait_bounds.is_empty()
        {
            // Unrecorded associated callee: a persistence-bearing turbofish
            // can still select the return type, so keep the argument visible.
            result = self.apply_turbofish_args(&result, None, turbofish);
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
            Expr::Block(block) => {
                return self
                    .block_result_infos
                    .borrow()
                    .get(&(&block.block as *const syn::Block as usize))
                    .cloned()
                    .unwrap_or_else(|| implicit_tail_info(&block.block, self));
            }
            // A closure value carries what its body would produce when later
            // called through the binding (e.g. `let factory = || database;`).
            Expr::Closure(closure) => {
                let mut info = self
                    .closure_result_infos
                    .borrow()
                    .get(&(closure as *const ExprClosure as usize))
                    .cloned()
                    .unwrap_or_else(|| self.info_from_expr(&closure.body));
                info.callable_signatures
                    .insert(closure_callable_signature(closure));
                info.closure_mutations = self
                    .closure_effects
                    .borrow()
                    .get(&(closure as *const ExprClosure as usize))
                    .cloned()
                    .unwrap_or_default();
                return info;
            }
            _ => {}
        }
        if let Expr::Path(path) = expression
            && path.qself.is_none()
            && path.path.segments.len() == 1
            && let Some(name) = last_path_name(&path.path)
        {
            if let Some(info) = self.lookup(&name) {
                if let Some(syn::PathArguments::AngleBracketed(turbofish)) =
                    path.path.segments.last().map(|segment| &segment.arguments)
                    && !info.callable_signatures.is_empty()
                {
                    let mut result = VariableInfo::default();
                    for signature in &info.callable_signatures {
                        result.union(&self.apply_turbofish_args(
                            info,
                            Some(&signature.generic_params),
                            Some(turbofish),
                        ));
                    }
                    return result;
                }
                return info.clone();
            }
            if let Some(info) = self.symbols.item_values.get(&name) {
                return info.clone();
            }
        }
        if let Expr::Path(path) = expression {
            let key = self.package_function_key(path_names(&path.path));
            if let Some(info) = self.symbols.package_item_values.get(&key) {
                return info.clone();
            }
            // Function items carry their declared return information when
            // stored in a local (`let factory = crate::make; factory()`).
            // Resolve the same local/package/associated registries used by a
            // direct call before the lexical binding hides the declaration.
            if path.path.segments.len() == 1
                && let Some(name) = last_path_name(&path.path)
                && let Some(info) = self.symbols.function_returns.get(&name)
            {
                let params = self.symbols.function_generic_params.get(&name);
                let turbofish = path.path.segments.last().and_then(|segment| {
                    if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
                        Some(arguments)
                    } else {
                        None
                    }
                });
                let mut info = self.apply_turbofish_args(info, params, turbofish);
                if let Some(params) = params {
                    info.callable_signatures.insert(CallableSignature {
                        generic_params: params.clone(),
                        generic_inputs: self
                            .symbols
                            .function_generic_input_params
                            .get(&name)
                            .cloned()
                            .unwrap_or_default(),
                    });
                }
                return info;
            }
            if let Some(info) = self.symbols.package_function_returns.get(&key) {
                let params = self.symbols.package_function_generic_params.get(&key);
                let turbofish = path.path.segments.last().and_then(|segment| {
                    if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
                        Some(arguments)
                    } else {
                        None
                    }
                });
                let mut info = self.apply_turbofish_args(info, params, turbofish);
                if let Some(params) = params {
                    info.callable_signatures.insert(CallableSignature {
                        generic_params: params.clone(),
                        generic_inputs: self
                            .symbols
                            .package_function_generic_input_params
                            .get(&key)
                            .cloned()
                            .unwrap_or_default(),
                    });
                }
                return info;
            }
            let associated = self.associated_return_info(path, None);
            if !associated.flow.is_empty()
                || !associated.nominal_types.is_empty()
                || !associated.payload_variants.is_empty()
                || !associated.tuple_items.is_empty()
                || !associated.field_items.is_empty()
                || !associated.trait_bounds.is_empty()
            {
                return associated;
            }
        }
        if let Expr::Call(call) = expression
            && let Expr::Path(path) = call.func.as_ref()
        {
            if is_standard_identity(&self.canonical_local_path_names(path_names(&path.path)))
                && let Some(argument) = call.args.first()
            {
                // `std::convert::identity` is shape-preserving as well as a
                // flow passthrough. Returning the argument's complete value
                // information keeps tuple/field projections sound.
                return self.info_from_expr(argument);
            }
            if path.path.segments.len() == 1
                && let Some(name) = last_path_name(&path.path)
            {
                let turbofish =
                    path.path
                        .segments
                        .last()
                        .and_then(|segment| match &segment.arguments {
                            syn::PathArguments::AngleBracketed(arguments) => Some(arguments),
                            _ => None,
                        });
                // Resolve single-segment callees through the current lexical
                // scope before falling back to module-level `function_returns`:
                // a local closure or function-valued binding may carry the
                // persistence return flow.
                if let Some(info) = self.lookup(&name) {
                    if info.callable_signatures.is_empty() {
                        let mut result = info.clone();
                        result.closure_mutations.clear();
                        for argument in &call.args {
                            result.union(&self.info_from_expr(argument));
                        }
                        return result;
                    }
                    let mut result = VariableInfo::default();
                    let mut return_info = info.clone();
                    return_info.callable_signatures.clear();
                    return_info.closure_mutations.clear();
                    for signature in &info.callable_signatures {
                        let explicit = self.apply_turbofish_args(
                            &return_info,
                            Some(&signature.generic_params),
                            turbofish,
                        );
                        result.union(&self.apply_inferred_args(
                            &explicit,
                            Some(&signature.generic_params),
                            Some(&signature.generic_inputs),
                            &call.args,
                        ));
                    }
                    return result;
                }
                if let Some(info) = self.symbols.function_returns.get(&name) {
                    let params = self.symbols.function_generic_params.get(&name);
                    let result = self.apply_turbofish_args(info, params, turbofish);
                    return self.apply_inferred_args(
                        &result,
                        params,
                        self.symbols.function_generic_input_params.get(&name),
                        &call.args,
                    );
                }
                // Free functions declared in another source module resolve
                // through the package-wide canonical-path registry.
                let key = self.package_function_key(vec![name]);
                if let Some(info) = self.symbols.package_function_returns.get(&key) {
                    let params = self.symbols.package_function_generic_params.get(&key);
                    let result = self.apply_turbofish_args(info, params, turbofish);
                    return self.apply_inferred_args(
                        &result,
                        params,
                        self.symbols.package_function_generic_input_params.get(&key),
                        &call.args,
                    );
                }
            }
            let return_info = self.associated_return_info(path, Some(&call.args));
            if !return_info.flow.is_empty()
                || !return_info.nominal_types.is_empty()
                || !return_info.payload_variants.is_empty()
                || !return_info.tuple_items.is_empty()
                || !return_info.trait_bounds.is_empty()
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
        if let Expr::Call(call) = expression
            && !matches!(call.func.as_ref(), Expr::Path(_))
        {
            let mut callable = self.info_from_expr(&call.func);
            for argument in &call.args {
                callable.union(&self.info_from_expr(argument));
            }
            if !callable.flow.is_empty()
                || !callable.nominal_types.is_empty()
                || !callable.payload_variants.is_empty()
                || !callable.tuple_items.is_empty()
                || !callable.field_items.is_empty()
                || !callable.trait_bounds.is_empty()
            {
                return callable;
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
                field_items: return_info.field_items,
                trait_bounds: return_info.trait_bounds,
                type_generic_params: Vec::new(),
                callable_signatures: BTreeSet::new(),
                closure_mutations: BTreeMap::new(),
            };
        }
        if let Expr::Field(field) = expression {
            let field_name = match &field.member {
                Member::Named(ident) => normalized_ident(ident),
                Member::Unnamed(index) => index.index.to_string(),
            };
            let base_info = self.info_from_expr(&field.base);
            if let Some(info) = base_info.field_items.get(&field_name) {
                return info.clone();
            }
        }
        if let Expr::Struct(structure) = expression {
            let key = self.package_function_key(path_names(&structure.path));
            let mut info = VariableInfo {
                flow: self.flow_of_expr(expression),
                sql_expression: self.sql_expression_kind(expression),
                nominal_types: BTreeSet::from([key.clone()]),
                ..VariableInfo::default()
            };
            if let Some(named) = self.symbols.named_type_info.get(&key) {
                info.union(named);
            }
            if let Some(named) = self.symbols.workspace_named_type_info.get(&key) {
                info.union(named);
            }
            // Explicit initializers are authoritative for this value. In
            // particular, a clean boolean field must not inherit aggregate
            // flow from a persistence-bearing sibling.
            for field in &structure.fields {
                let name = match &field.member {
                    Member::Named(ident) => normalized_ident(ident),
                    Member::Unnamed(index) => index.index.to_string(),
                };
                let mut field_info = self.info_from_expr(&field.expr);
                // Scalar-reading methods intentionally discard aggregate
                // SqlResult flow in ordinary expressions. A struct field is
                // a durable projection boundary, however, so retain the
                // initializer subtree on that exact field without tainting
                // its siblings.
                field_info.flow.union(self.produced_value_flow(&field.expr));
                info.field_items.insert(name, field_info);
            }
            if let Some(rest) = &structure.rest {
                let rest = self.info_from_expr(rest);
                for (name, field) in rest.field_items {
                    info.field_items.entry(name).or_insert(field);
                }
                info.flow.union(rest.flow);
            }
            return info;
        }
        if let Expr::Call(call) = expression
            && let Expr::Path(path) = call.func.as_ref()
            && let Some(owner) = last_path_name(&path.path)
        {
            let is_constructor_method = path.qself.is_none()
                && path.path.segments.len() >= 2
                && matches!(owner.as_str(), "default" | "new");
            let nominal_types = if is_constructor_method {
                path.path
                    .segments
                    .iter()
                    .nth_back(1)
                    .map(|segment| BTreeSet::from([normalized_ident(&segment.ident)]))
                    .unwrap_or_default()
            } else {
                BTreeSet::from([owner])
            };
            let mut info = VariableInfo {
                flow: self.flow_of_expr(expression),
                sql_expression: self.sql_expression_kind(expression),
                nominal_types,
                payload_variants: payload_variants_in_path(&path.path, self.symbols),
                tuple_items: Vec::new(),
                field_items: BTreeMap::new(),
                trait_bounds: BTreeSet::new(),
                type_generic_params: Vec::new(),
                callable_signatures: BTreeSet::new(),
                closure_mutations: BTreeMap::new(),
            };
            let mut names = path_names(&path.path);
            if is_constructor_method {
                names.pop();
            }
            let key = self.package_function_key(names);
            if let Some(named) = self.symbols.named_type_info.get(&key) {
                info.union(named);
            }
            if let Some(named) = self.symbols.workspace_named_type_info.get(&key) {
                info.union(named);
            }
            for argument in &call.args {
                info.union(&self.info_from_expr(argument));
            }
            return info;
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
            field_items: BTreeMap::new(),
            trait_bounds: BTreeSet::new(),
            type_generic_params: Vec::new(),
            callable_signatures: BTreeSet::new(),
            closure_mutations: BTreeMap::new(),
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
                let info = self.info_from_expr(expression);
                if !info.nominal_types.is_empty() {
                    return info.nominal_types;
                }
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
            Expr::Call(call) => {
                let info = self.info_from_expr(expression);
                if !info.nominal_types.is_empty() {
                    return info.nominal_types;
                }
                match call.func.as_ref() {
                    Expr::Path(path) if path.path.segments.len() == 1 => last_path_name(&path.path)
                        .map(|name| {
                            self.symbols
                                .function_returns
                                .get(&name)
                                .map(|info| info.nominal_types.clone())
                                .unwrap_or_else(|| BTreeSet::from([name]))
                        })
                        .unwrap_or_default(),
                    Expr::Path(path) => {
                        self.associated_return_info(path, Some(&call.args))
                            .nominal_types
                    }
                    _ => BTreeSet::new(),
                }
            }
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
        // Enum variants and structs declared in another source file register
        // their fields in the package-wide named-type registry under
        // `crate::dto::Product::Database`; a match pattern may name them by
        // the full path or by an imported short owner, so accept the exact
        // owner key or any canonical key with that owner suffix.
        info.union(&self.named_field_info(owner, field));
        info
    }

    fn named_field_info(&self, owner: &str, field: &str) -> VariableInfo {
        let mut info = VariableInfo::default();
        let suffix = format!("::{owner}");
        for (key, named_info) in self.symbols.named_type_info.iter() {
            if (key == &owner || key.ends_with(&suffix))
                && let Some(field_info) = named_info.field_items.get(field)
            {
                info.union(field_info);
            }
        }
        let mut workspace_keys = BTreeSet::from([owner.to_owned()]);
        workspace_keys.insert(
            canonical_path_names(owner.split("::").map(str::to_owned).collect(), self.symbols)
                .join("::"),
        );
        for key in workspace_keys {
            if let Some(field_info) = self
                .symbols
                .workspace_named_type_info
                .get(&key)
                .and_then(|named_info| named_info.field_items.get(field))
            {
                info.union(field_info);
            }
        }
        info
    }

    fn named_field_is_declared(&self, owner: &str, field: &str) -> bool {
        let suffix = format!("::{owner}");
        if self.symbols.named_type_info.iter().any(|(key, info)| {
            (key == owner || key.ends_with(&suffix)) && info.field_items.contains_key(field)
        }) {
            return true;
        }
        let canonical =
            canonical_path_names(owner.split("::").map(str::to_owned).collect(), self.symbols)
                .join("::");
        [owner, canonical.as_str()].into_iter().any(|key| {
            self.symbols
                .workspace_named_type_info
                .get(key)
                .is_some_and(|info| info.field_items.contains_key(field))
        })
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
        }) || {
            let suffix = format!("::{owner}");
            let local = self.symbols.named_type_info.iter().any(|(key, info)| {
                (key == &owner || key.ends_with(&suffix)) && !info.field_items.is_empty()
            });
            let canonical =
                canonical_path_names(owner.split("::").map(str::to_owned).collect(), self.symbols)
                    .join("::");
            local
                || [owner, canonical.as_str()].into_iter().any(|key| {
                    self.symbols
                        .workspace_named_type_info
                        .get(key)
                        .is_some_and(|info| !info.field_items.is_empty())
                })
        }
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
            trait_bounds: info.trait_bounds.clone(),
            ..VariableInfo::default()
        };
        payload
            .trait_bounds
            .extend(info.trait_bounds.iter().cloned());
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
                typed_info.sql_expression = typed_info.sql_expression.max(info.sql_expression);
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
                if self
                    .canonical_local_path_names(path_names(&mac.mac.path))
                    .last()
                    .is_some_and(|name| matches!(name.as_str(), "format" | "format_args")) =>
            {
                SqlExpressionKind::Interpolated
            }
            Expr::Macro(mac)
                if self
                    .canonical_local_path_names(path_names(&mac.mac.path))
                    .last()
                    .is_some_and(|name| name == "include_str") =>
            {
                SqlExpressionKind::Included
            }
            Expr::Macro(mac)
                if self
                    .canonical_local_path_names(path_names(&mac.mac.path))
                    .last()
                    .is_some_and(|name| name == "env") =>
            {
                SqlExpressionKind::Environment
            }
            Expr::Macro(mac)
                if self
                    .canonical_local_path_names(path_names(&mac.mac.path))
                    .last()
                    .is_some_and(|name| matches!(name.as_str(), "concat" | "stringify")) =>
            {
                SqlExpressionKind::Static
            }
            Expr::MethodCall(method)
                if matches!(
                    normalized_ident(&method.method).as_str(),
                    "as_str" | "as_ref"
                ) =>
            {
                self.sql_expression_kind(&method.receiver)
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
        let base_info = self.info_from_expr(&field.base);
        if let Some(info) = base_info.field_items.get(&name) {
            // A declared projection is authoritative even when it is clean.
            // Falling through for an empty field inherited the aggregate's
            // persistence flow and tainted sibling booleans/counters.
            return info.flow.clone();
        }
        let mut targets = TargetSet::new();
        let mut declared = false;
        for owner in self.nominal_types_of_expr(&field.base) {
            let field_targets = match &field.member {
                Member::Named(_) => self
                    .symbols
                    .field_targets
                    .get(&(owner.clone(), name.clone())),
                Member::Unnamed(_) => self
                    .symbols
                    .tuple_field_targets
                    .get(&(owner.clone(), name.clone())),
            };
            if let Some(field_targets) = field_targets {
                declared = true;
                targets.extend(field_targets);
            }
            declared |= self
                .symbols
                .field_nominal_types
                .contains_key(&(owner.clone(), name.clone()));
            declared |= self.named_field_is_declared(&owner, &name);
        }
        if declared && targets.is_empty() {
            return Flow::default();
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
                if let Some(info) = self.symbols.item_values.get(&name) {
                    return info.flow.clone();
                }
            }
        }
        let key = self.package_function_key(path_names(&path.path));
        if let Some(info) = self.symbols.package_item_values.get(&key) {
            return info.flow.clone();
        }
        Flow::pools(&targets_for_path(&path.path, self.symbols))
    }

    fn flow_of_call(&self, call: &ExprCall) -> Flow {
        let Expr::Path(path) = call.func.as_ref() else {
            let mut flow = self.info_from_expr(&call.func).flow;
            for argument in &call.args {
                flow.union(self.flow_of_expr(argument));
            }
            return flow;
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
        if is_flow_passthrough_call(&names)
            || is_standard_identity(&self.canonical_local_path_names(names.clone()))
        {
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
        let associated = self.associated_return_info(path, Some(&call.args)).flow;
        if !associated.is_empty() {
            return associated;
        }
        if names.len() == 1 {
            let turbofish =
                path.path
                    .segments
                    .last()
                    .and_then(|segment| match &segment.arguments {
                        syn::PathArguments::AngleBracketed(arguments) => Some(arguments),
                        _ => None,
                    });
            if let Some(info) = self.lookup(last) {
                if !info.callable_signatures.is_empty() {
                    let mut result = VariableInfo::default();
                    let mut return_info = info.clone();
                    return_info.callable_signatures.clear();
                    for signature in &info.callable_signatures {
                        let explicit = self.apply_turbofish_args(
                            &return_info,
                            Some(&signature.generic_params),
                            turbofish,
                        );
                        result.union(&self.apply_inferred_args(
                            &explicit,
                            Some(&signature.generic_params),
                            Some(&signature.generic_inputs),
                            &call.args,
                        ));
                    }
                    return result.flow;
                }
                let mut flow = info.flow.clone();
                for argument in &call.args {
                    flow.union(self.flow_of_expr(argument));
                }
                return flow;
            }
            if let Some(info) = self.symbols.function_returns.get(last) {
                let params = self.symbols.function_generic_params.get(last);
                let result = self.apply_turbofish_args(info, params, turbofish);
                return self
                    .apply_inferred_args(
                        &result,
                        params,
                        self.symbols.function_generic_input_params.get(last),
                        &call.args,
                    )
                    .flow;
            }
            // Free functions declared in another source module resolve
            // through the package-wide canonical-path registry.
            let key = self.package_function_key(names.clone());
            if let Some(info) = self.symbols.package_function_returns.get(&key) {
                let params = self.symbols.package_function_generic_params.get(&key);
                let result = self.apply_turbofish_args(info, params, turbofish);
                return self
                    .apply_inferred_args(
                        &result,
                        params,
                        self.symbols.package_function_generic_input_params.get(&key),
                        &call.args,
                    )
                    .flow;
            }
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
            name if FLOW_TRANSFORMING_METHODS.contains(&name) => {
                let mut flow = receiver;
                for argument in &method.args {
                    // The closure/fallback argument may produce the result
                    // value; union its produced flow, including closure
                    // bodies, instead of treating the combinator as a
                    // receiver-only passthrough.
                    flow.union(self.subtree_flow(argument));
                }
                flow
            }
            name if FLOW_PASSTHROUGH_METHODS.contains(&name) => receiver,
            _ => {
                // An unmodelled (external) method can transform or wrap a
                // capability-carrying receiver without dropping the value,
                // e.g. `Some(database).iter().next()` later unwrapped into
                // `.pool()`. Keep only capability flow: query-stage flow and
                // data containers (row results, fields, holders) reading
                // scalars are ordinary data access already inventoried at the
                // query/execute call sites, and unioning them would drown the
                // ratchet in value_alias noise.
                let mut flow = Flow(
                    receiver
                        .0
                        .iter()
                        .filter(|(target, stage)| {
                            !matches!(stage, FlowStage::Query)
                                && !matches!(
                                    target,
                                    PersistenceTarget::SqlResult
                                        | PersistenceTarget::SqlFields
                                        | PersistenceTarget::SqlQueryHolder
                                        | PersistenceTarget::SqlQueryHolderResult
                                )
                        })
                        .copied()
                        .collect(),
                );
                flow.union(self.method_return_info(method).flow);
                flow
            }
        }
    }

    /// Flow carried by the value an expression produces, excluding unrelated
    /// side effects evaluated before that value. This is narrower than
    /// `subtree_flow` and is used at named-field projection boundaries.
    fn produced_value_flow(&self, expression: &Expr) -> Flow {
        let direct = self.flow_of_expr(expression);
        if !direct.is_empty() {
            return direct;
        }
        match expression {
            Expr::Reference(reference) => self.produced_value_flow(&reference.expr),
            Expr::Paren(paren) => self.produced_value_flow(&paren.expr),
            Expr::Group(group) => self.produced_value_flow(&group.expr),
            Expr::Try(try_expression) => self.produced_value_flow(&try_expression.expr),
            Expr::Await(await_expression) => self.produced_value_flow(&await_expression.base),
            Expr::Cast(cast) => self.produced_value_flow(&cast.expr),
            Expr::Unary(unary) => self.produced_value_flow(&unary.expr),
            Expr::MethodCall(method) => {
                let name = normalized_ident(&method.method);
                if matches!(name.as_str(), "try_read" | "read" | "read_string")
                    || FLOW_PASSTHROUGH_METHODS.contains(&name.as_str())
                    || FLOW_TRANSFORMING_METHODS.contains(&name.as_str())
                {
                    let mut flow = self.produced_value_flow(&method.receiver);
                    if FLOW_TRANSFORMING_METHODS.contains(&name.as_str()) {
                        for argument in &method.args {
                            flow.union(self.produced_value_flow(argument));
                        }
                    }
                    flow
                } else {
                    Flow::default()
                }
            }
            Expr::Block(block) => block
                .block
                .stmts
                .last()
                .and_then(|statement| match statement {
                    Stmt::Expr(tail, None) => Some(self.produced_value_flow(tail)),
                    _ => None,
                })
                .unwrap_or_default(),
            Expr::If(if_expression) => {
                let mut flow = if_expression
                    .then_branch
                    .stmts
                    .last()
                    .and_then(|statement| match statement {
                        Stmt::Expr(tail, None) => Some(self.produced_value_flow(tail)),
                        _ => None,
                    })
                    .unwrap_or_default();
                if let Some((_, alternative)) = &if_expression.else_branch {
                    flow.union(self.produced_value_flow(alternative));
                }
                flow
            }
            Expr::Match(match_expression) => {
                let mut flow = Flow::default();
                for arm in &match_expression.arms {
                    flow.union(self.produced_value_flow(&arm.body));
                }
                flow
            }
            Expr::Tuple(tuple) => {
                let mut flow = Flow::default();
                for element in &tuple.elems {
                    flow.union(self.produced_value_flow(element));
                }
                flow
            }
            _ => Flow::default(),
        }
    }

    fn flow_of_expr(&self, expression: &Expr) -> Flow {
        let key = (expression as *const Expr as usize, self.context_version);
        if let Some(flow) = self.flow_cache.borrow().get(&key) {
            return flow.clone();
        }
        let flow = self.compute_flow_of_expr(expression);
        self.flow_cache.borrow_mut().insert(key, flow.clone());
        flow
    }

    fn subtree_flow(&self, expression: &Expr) -> Flow {
        let key = (expression as *const Expr as usize, self.context_version);
        if let Some(flow) = self.subtree_flow_cache.borrow().get(&key) {
            return flow.clone();
        }
        let mut flow = self.flow_of_expr(expression);
        if let Expr::Field(field) = expression {
            let mut root = field.base.as_ref();
            while let Expr::Field(parent) = root {
                root = parent.base.as_ref();
            }
            if !matches!(root, Expr::Path(_)) {
                flow.union(self.subtree_flow(root));
            }
            self.subtree_flow_cache
                .borrow_mut()
                .insert(key, flow.clone());
            return flow;
        }
        let mut collector = DirectChildFlowCollector {
            analyzer: self,
            flow: Flow::default(),
            at_root: true,
        };
        collector.visit_expr(expression);
        flow.union(collector.flow);
        self.subtree_flow_cache
            .borrow_mut()
            .insert(key, flow.clone());
        flow
    }

    fn compute_flow_of_expr(&self, expression: &Expr) -> Flow {
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
        } else if let Some(targets) = (names.len() == 1)
            .then(|| self.symbols.persistence_macros.get(last))
            .flatten()
            .or_else(|| {
                let key = self.package_function_key(names.clone());
                self.symbols
                    .package_persistence_macros
                    .get(&key)
                    .or_else(|| self.symbols.package_persistence_macros.get(last))
            })
        {
            // A registered macro's definition already proves its concrete
            // targets. Preserve that result flow as well as auditing the call
            // site so `database!().pool()` cannot disappear behind the
            // definition's existing baseline row.
            Flow::pools(targets)
        } else if matches!(last, "vec" | "join" | "try_join" | "select") {
            // `vec!` is whitelisted as an opaque macro but it produces a
            // value: its result flow is the union of every in-scope
            // persistence value named in its input. Without this,
            // `let values = vec![database]; values[0].pool()` escapes both
            // ratchets.
            let mut flow = Flow::default();
            for scope in &self.scopes {
                for (local, info) in scope {
                    if !info.flow.is_empty()
                        && tokens_contain_identifier(
                            mac.tokens.clone(),
                            &BTreeSet::from([local.clone()]),
                        )
                    {
                        flow.union(info.flow.clone());
                    }
                }
            }
            for (callable, info) in self.symbols.package_function_returns.iter() {
                let leaf = callable.rsplit("::").next().unwrap_or(callable);
                if !info.flow.is_empty()
                    && tokens_contain_callable_invocation(
                        mac.tokens.clone(),
                        &BTreeSet::from([leaf.to_owned()]),
                    )
                {
                    flow.union(info.flow.clone());
                }
            }
            flow
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
        if name == "include" && !is_pinned_wow_proto_include(&self.context, mac) {
            self.errors.push(format!(
                "{} contains include! whose Rust source is outside the persistence AST inventory; mount and parse the included source explicitly",
                self.enclosing
            ));
            return;
        }
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
        let registered_macro_targets = (names.len() == 1)
            .then(|| self.symbols.persistence_macros.get(&name))
            .flatten()
            .or_else(|| {
                let key = self.package_function_key(names.clone());
                self.symbols
                    .package_persistence_macros
                    .get(&key)
                    .or_else(|| self.symbols.package_persistence_macros.get(&name))
            })
            .cloned();
        if let Some(targets) = registered_macro_targets {
            // Invocation of a registered persistence-generating `macro_rules!`:
            // the definition row covers the body, but each call site must
            // leave its own row so adding another invocation cannot bypass
            // both ratchets without touching the generated artifacts.
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
        let mut argument_flow = Flow::default();
        if let Ok(expressions) =
            syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
                .parse2(mac.tokens.clone())
        {
            for expression in expressions {
                argument_flow.union(self.subtree_flow(&expression));
            }
        }
        let known = self.known_persistence_names();
        if !tokens_contain_identifier(mac.tokens.clone(), &known) && argument_flow.is_empty() {
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
            let parsed = syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
                .parse2(mac.tokens.clone());
            if let Ok(expressions) = parsed {
                for expression in expressions {
                    escaped.union(self.subtree_flow(&expression));
                }
                targets.extend(escaped.targets());
            } else {
                // Macros with custom grammars (`select!`, pattern arms, etc.)
                // remain fail-closed: any referenced persistent local escapes.
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
            let mut info = self.info_from_type(&typed.ty);
            // Parameter values are supplied at runtime even when their type
            // is a source-known `&str`; only literal-producing expressions
            // can retain the default Static classification.
            info.sql_expression = SqlExpressionKind::Nonliteral;
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

    fn register_bound_args(&mut self, name: &str, trait_path: &str, bound: &syn::TraitBound) {
        let Some(segment) = bound.path.segments.last() else {
            return;
        };
        let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
            return;
        };
        let args: Vec<VariableInfo> = arguments
            .args
            .iter()
            .filter_map(|argument| match argument {
                // The argument's full info unions the package-wide named-type
                // registry, so substituting it later brings the concrete
                // type's flow, fields, and payload shapes with it.
                syn::GenericArgument::Type(inner) => {
                    Some(variable_info_in_type(inner, self.symbols))
                }
                _ => None,
            })
            .collect();
        if !args.is_empty() {
            self.generic_trait_bound_args
                .insert((name.to_owned(), trait_path.to_owned()), args);
        }
        let associated = arguments
            .args
            .iter()
            .filter_map(|argument| {
                let syn::GenericArgument::AssocType(binding) = argument else {
                    return None;
                };
                Some((
                    normalized_ident(&binding.ident),
                    variable_info_in_type(&binding.ty, self.symbols),
                ))
            })
            .collect::<BTreeMap<_, _>>();
        if !associated.is_empty() {
            self.generic_trait_bound_associated
                .entry((name.to_owned(), trait_path.to_owned()))
                .or_default()
                .extend(associated);
        }
    }

    fn register_generic_bounds(&mut self, generics: &syn::Generics) {
        self.bump_context();
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
                        .insert(trait_path.clone());
                    self.register_bound_args(&name, &trait_path, bound);
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
                            .insert(trait_path.clone());
                        self.register_bound_args(&name, &trait_path, bound);
                    }
                }
            }
        }
    }

    /// Substitutes a trait's generic parameters with the type arguments the
    /// receiver's bound recorded (`M: Maker<Holder>` makes a `-> T` trait
    /// return surface `Holder`, including `where` predicates).
    fn apply_bound_generic_args(
        &self,
        info: &VariableInfo,
        trait_bound: &str,
        receiver_types: &BTreeSet<String>,
    ) -> VariableInfo {
        let mut info = info.clone();
        let params = self
            .symbols
            .trait_generic_params
            .get(trait_bound)
            .cloned()
            .unwrap_or_default();
        for receiver_type in receiver_types {
            if let Some(args) = self
                .generic_trait_bound_args
                .get(&(receiver_type.clone(), trait_bound.to_owned()))
            {
                let map: BTreeMap<String, VariableInfo> =
                    params.iter().cloned().zip(args.iter().cloned()).collect();
                substitute_nominal_params(&mut info, &map);
            }
            if let Some(associated) = self
                .generic_trait_bound_associated
                .get(&(receiver_type.clone(), trait_bound.to_owned()))
            {
                substitute_nominal_params(&mut info, associated);
            }
        }
        info
    }

    /// Applies an explicit method/function turbofish (`make::<T>()`) to a
    /// recorded return. When the callee's generic parameter list is known,
    /// the arguments are substituted positionally into the return. When it is
    /// not recorded (external or unmodelled callee), a persistence-bearing
    /// turbofish argument is unioned into the result instead: the argument
    /// may select the return type, so dropping it would let the call bypass
    /// both ratchets.
    fn apply_turbofish_args(
        &self,
        info: &VariableInfo,
        params: Option<&Vec<String>>,
        turbofish: Option<&syn::AngleBracketedGenericArguments>,
    ) -> VariableInfo {
        let Some(turbofish) = turbofish else {
            return info.clone();
        };
        let args: Vec<VariableInfo> = turbofish
            .args
            .iter()
            .filter_map(|argument| match argument {
                syn::GenericArgument::Type(inner) => {
                    Some(variable_info_in_type(inner, self.symbols))
                }
                _ => None,
            })
            .collect();
        if args.is_empty() {
            return info.clone();
        }
        let mut info = info.clone();
        let substituted = params.is_some_and(|params| {
            let map: BTreeMap<String, VariableInfo> =
                params.iter().cloned().zip(args.iter().cloned()).collect();
            substitute_nominal_params(&mut info, &map)
        });
        if !substituted {
            for argument in &args {
                if !argument.flow.is_empty() || !argument.trait_bounds.is_empty() {
                    info.union(argument);
                }
            }
        }
        info
    }

    fn apply_inferred_args(
        &self,
        info: &VariableInfo,
        params: Option<&Vec<String>>,
        input_params: Option<&Vec<GenericInputSpec>>,
        args: &syn::punctuated::Punctuated<Expr, syn::token::Comma>,
    ) -> VariableInfo {
        let (Some(params), Some(input_params)) = (params, input_params) else {
            return info.clone();
        };
        let mut substitutions = BTreeMap::<String, VariableInfo>::new();
        for (argument, formal_input) in args.iter().zip(input_params) {
            if formal_input.params.is_empty() {
                continue;
            }
            let argument_info = self.info_from_expr(argument);
            for param in &formal_input.params {
                substitutions
                    .entry(param.clone())
                    .or_default()
                    .union(&projected_generic_argument(
                        &argument_info,
                        formal_input,
                        param,
                    ));
            }
        }
        substitutions.retain(|param, _| params.contains(param));
        let mut result = info.clone();
        substitute_nominal_params(&mut result, &substitutions);
        result
    }

    fn apply_inferred_method_args(
        &self,
        info: &VariableInfo,
        params: Option<&Vec<String>>,
        input_params: Option<&Vec<GenericInputSpec>>,
        args: &syn::punctuated::Punctuated<Expr, syn::token::Comma>,
    ) -> VariableInfo {
        let input_params = input_params.map(|inputs| {
            if inputs
                .first()
                .is_some_and(|input| input.params.contains(RECEIVER_INPUT_MARKER))
            {
                &inputs[1..]
            } else {
                inputs.as_slice()
            }
        });
        let Some(params) = params else {
            return info.clone();
        };
        let Some(input_params) = input_params else {
            return info.clone();
        };
        let mut substitutions = BTreeMap::<String, VariableInfo>::new();
        for (argument, formal_input) in args.iter().zip(input_params) {
            let argument_info = self.info_from_expr(argument);
            for param in &formal_input.params {
                substitutions
                    .entry(param.clone())
                    .or_default()
                    .union(&projected_generic_argument(
                        &argument_info,
                        formal_input,
                        param,
                    ));
            }
        }
        substitutions.retain(|param, _| params.contains(param));
        let mut result = info.clone();
        substitute_nominal_params(&mut result, &substitutions);
        result
    }

    fn apply_optional_inferred_args(
        &self,
        info: &VariableInfo,
        params: Option<&Vec<String>>,
        input_params: Option<&Vec<GenericInputSpec>>,
        args: Option<&syn::punctuated::Punctuated<Expr, syn::token::Comma>>,
    ) -> VariableInfo {
        args.map(|args| self.apply_inferred_args(info, params, input_params, args))
            .unwrap_or_else(|| info.clone())
    }

    fn closure_mutations_for_args(
        &self,
        info: &VariableInfo,
        arguments: &[VariableInfo],
    ) -> BTreeMap<String, VariableInfo> {
        if info.callable_signatures.is_empty() {
            return info.closure_mutations.clone();
        }
        let mut instantiated = BTreeMap::<String, VariableInfo>::new();
        for signature in &info.callable_signatures {
            let mut substitutions = BTreeMap::<String, VariableInfo>::new();
            for (argument, formal_input) in arguments.iter().zip(&signature.generic_inputs) {
                for param in &formal_input.params {
                    substitutions
                        .entry(param.clone())
                        .or_default()
                        .union(&projected_generic_argument(argument, formal_input, param));
                }
            }
            substitutions.retain(|param, _| signature.generic_params.contains(param));
            for (captured, mutation) in &info.closure_mutations {
                let mut mutation = mutation.clone();
                substitute_nominal_params(&mut mutation, &substitutions);
                instantiated
                    .entry(captured.clone())
                    .or_default()
                    .union(&mutation);
            }
        }
        instantiated
    }
}

fn item_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::ExternCrate(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Macro(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::TraitAlias(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

fn implicit_tail_info(block: &syn::Block, analyzer: &BodyAnalyzer<'_, '_>) -> VariableInfo {
    if let Some(info) = analyzer
        .block_result_infos
        .borrow()
        .get(&(block as *const syn::Block as usize))
    {
        return info.clone();
    }
    match block.stmts.last() {
        Some(Stmt::Expr(expression, None)) => analyzer.info_from_expr(expression),
        _ => VariableInfo::default(),
    }
}

fn implicit_tail_flow(block: &syn::Block, analyzer: &BodyAnalyzer<'_, '_>) -> Flow {
    implicit_tail_info(block, analyzer).flow
}

/// Unions two scope stacks captured after mutually exclusive match arms. Both
/// stacks descend from the same pre-match snapshot, so zip is exact: every
/// outer local assigned by any arm keeps the union of all arm outcomes.
fn merge_scope_stacks(
    accumulated: &mut [BTreeMap<String, VariableInfo>],
    incoming: &[BTreeMap<String, VariableInfo>],
) {
    for (accumulated_scope, incoming_scope) in accumulated.iter_mut().zip(incoming.iter()) {
        for (name, info) in incoming_scope {
            accumulated_scope
                .entry(name.clone())
                .or_default()
                .union(info);
        }
    }
}

fn collect_shape_nominals(shape: &NominalShape, output: &mut BTreeSet<String>) {
    output.extend(shape.nominal_types.iter().cloned());
    for argument in &shape.arguments {
        collect_shape_nominals(argument, output);
    }
}

/// Loop-carried values can grow without a finite structural fixed point (for
/// example `node = Node::Branch(Box::new(node))`). Once several exact passes
/// have failed to stabilize, collapse nested projections into the containing
/// value while retaining every reachable flow, nominal type, trait bound and
/// callable effect. This is a conservative widening: later projections may
/// produce extra rows, but persistence can no longer disappear at arbitrary
/// runtime iteration depths and the abstract loop state remains finite.
fn widen_loop_variable(info: &mut VariableInfo) {
    let mut nested = VariableInfo::default();
    for item in &mut info.tuple_items {
        widen_loop_variable(item);
        nested.union(item);
    }
    for item in info.field_items.values_mut() {
        widen_loop_variable(item);
        nested.union(item);
    }
    for mutation in info.closure_mutations.values_mut() {
        widen_loop_variable(mutation);
        nested.union(mutation);
    }
    for shapes in &info.payload_variants {
        for shape in shapes {
            collect_shape_nominals(shape, &mut nested.nominal_types);
        }
    }
    info.union(&nested);
    info.payload_variants.clear();
    info.tuple_items.clear();
    info.field_items.clear();
}

fn widen_loop_scopes(scopes: &mut [BTreeMap<String, VariableInfo>]) {
    for scope in scopes {
        for info in scope.values_mut() {
            widen_loop_variable(info);
        }
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

#[derive(Clone, Debug)]
enum PlaceProjection {
    Field(String),
    Index(Option<usize>),
}

fn assignment_place(expression: &Expr) -> Option<(String, Vec<PlaceProjection>)> {
    fn collect(expression: &Expr, projections: &mut Vec<PlaceProjection>) -> Option<String> {
        match expression {
            Expr::Path(_) => simple_assignment_name(expression),
            Expr::Field(field) => {
                let root = collect(&field.base, projections)?;
                let name = match &field.member {
                    Member::Named(ident) => normalized_ident(ident),
                    Member::Unnamed(index) => index.index.to_string(),
                };
                projections.push(PlaceProjection::Field(name));
                Some(root)
            }
            Expr::Index(index) => {
                let root = collect(&index.expr, projections)?;
                let numeric = match index.index.as_ref() {
                    Expr::Lit(literal) => match &literal.lit {
                        syn::Lit::Int(value) => value.base10_parse().ok(),
                        _ => None,
                    },
                    _ => None,
                };
                projections.push(PlaceProjection::Index(numeric));
                Some(root)
            }
            Expr::Paren(paren) => collect(&paren.expr, projections),
            Expr::Group(group) => collect(&group.expr, projections),
            _ => None,
        }
    }

    let mut projections = Vec::new();
    let root = collect(expression, &mut projections)?;
    (!projections.is_empty()).then_some((root, projections))
}

fn assign_place_projection(
    aggregate: &mut VariableInfo,
    projections: &[PlaceProjection],
    value: &VariableInfo,
) {
    let Some((projection, remaining)) = projections.split_first() else {
        *aggregate = value.clone();
        return;
    };
    match projection {
        PlaceProjection::Field(name) => {
            assign_place_projection(
                aggregate.field_items.entry(name.clone()).or_default(),
                remaining,
                value,
            );
        }
        PlaceProjection::Index(index) => {
            // An index write makes the collection persistence-bearing even
            // when the exact runtime index is unknown. Retain an exact tuple/
            // array projection as well when a literal index is available.
            aggregate.union(value);
            if let Some(index) = index {
                if aggregate.tuple_items.len() <= *index {
                    aggregate
                        .tuple_items
                        .resize_with(index + 1, VariableInfo::default);
                }
                assign_place_projection(&mut aggregate.tuple_items[*index], remaining, value);
            }
        }
    }
}

fn mutable_storage_receiver_name(expression: &Expr) -> Option<String> {
    match expression {
        Expr::Reference(reference) if reference.mutability.is_some() => {
            simple_assignment_name(&reference.expr)
        }
        Expr::Paren(paren) => mutable_storage_receiver_name(&paren.expr),
        Expr::Group(group) => mutable_storage_receiver_name(&group.expr),
        _ => None,
    }
}

fn visit_let_chain_condition(
    analyzer: &mut BodyAnalyzer<'_, '_>,
    expression: &Expr,
    method_receiver_fallback: bool,
) {
    match expression {
        Expr::Let(let_expression) => {
            analyzer.visit_expr(&let_expression.expr);
            let mut info = analyzer.info_from_expr(&let_expression.expr);
            if method_receiver_fallback
                && info.flow.is_empty()
                && let Expr::MethodCall(method) = let_expression.expr.as_ref()
            {
                info = analyzer.info_from_expr(&method.receiver);
                analyzer.bind_pattern(&let_expression.pat, &info);
            } else {
                analyzer.bind_pattern_from_expr(&let_expression.pat, &let_expression.expr);
            }
        }
        Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_)) => {
            visit_let_chain_condition(analyzer, &binary.left, method_receiver_fallback);
            let pre_rhs_scopes = analyzer.scopes.clone();
            visit_let_chain_condition(analyzer, &binary.right, method_receiver_fallback);
            merge_scope_stacks(&mut analyzer.scopes, &pre_rhs_scopes);
            analyzer.bump_context();
        }
        Expr::Paren(paren) => {
            visit_let_chain_condition(analyzer, &paren.expr, method_receiver_fallback);
        }
        Expr::Group(group) => {
            visit_let_chain_condition(analyzer, &group.expr, method_receiver_fallback);
        }
        _ => analyzer.visit_expr(expression),
    }
}

fn assign_destructured_expr(
    analyzer: &mut BodyAnalyzer<'_, '_>,
    expression: &Expr,
    info: &VariableInfo,
) -> bool {
    match expression {
        Expr::Path(_) => {
            let Some(name) = simple_assignment_name(expression) else {
                return false;
            };
            analyzer.assign(&name, info.clone());
            true
        }
        Expr::Tuple(tuple) => {
            let mut all_supported = true;
            for (index, element) in tuple.elems.iter().enumerate() {
                let item = info.tuple_items.get(index).unwrap_or(info);
                if !assign_destructured_expr(analyzer, element, item) {
                    all_supported = false;
                }
            }
            all_supported
        }
        Expr::Array(array) => {
            let mut all_supported = true;
            for (index, element) in array.elems.iter().enumerate() {
                if !assign_destructured_expr(
                    analyzer,
                    element,
                    info.tuple_items.get(index).unwrap_or(info),
                ) {
                    all_supported = false;
                }
            }
            all_supported
        }
        Expr::Paren(paren) => assign_destructured_expr(analyzer, &paren.expr, info),
        Expr::Group(group) => assign_destructured_expr(analyzer, &group.expr, info),
        _ => false,
    }
}

impl<'ast> Visit<'ast> for BodyAnalyzer<'_, '_> {
    fn visit_expr_binary(&mut self, expression: &'ast syn::ExprBinary) {
        if !self.allows_source_class(&expression.attrs, "binary expression") {
            return;
        }
        self.visit_expr(&expression.left);
        if matches!(expression.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) {
            // The right side of `&&`/`||` may not execute. Audit it, but
            // retain both the pre-RHS and post-RHS binding states so a
            // conditional assignment cannot erase persistence flow that is
            // still reachable through the short-circuit path.
            let pre_rhs_scopes = self.scopes.clone();
            self.visit_expr(&expression.right);
            merge_scope_stacks(&mut self.scopes, &pre_rhs_scopes);
            self.bump_context();
        } else {
            self.visit_expr(&expression.right);
        }
    }

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
        // The body may run zero times (empty iterator), so its assignments
        // to outer locals cannot be applied unconditionally: conservatively
        // union the pre-loop state with the post-body state.
        let mut accumulated_scopes = self.scopes.clone();
        let label = expression
            .label
            .as_ref()
            .map(|label| normalized_ident(&label.name.ident));
        let initial_error_count = self.errors.len();
        self.accumulator.begin_transaction();
        let first_flow = self.visit_for_loop_body(expression, &iterator_info, label.clone());
        let first_post_body_scopes = self.scopes.clone();
        let mut first_next = accumulated_scopes.clone();
        merge_scope_stacks(&mut first_next, &first_post_body_scopes);
        if let Some(back_edges) = &first_flow.back_edges {
            merge_scope_stacks(&mut first_next, back_edges);
        }
        if first_next == accumulated_scopes {
            self.accumulator.commit_transaction();
            self.scopes = accumulated_scopes;
            if let Some(exits) = first_flow.exits {
                merge_scope_stacks(&mut self.scopes, &exits);
            }
            self.bump_context();
            return;
        }
        self.accumulator.rollback_transaction();
        self.errors.truncate(initial_error_count);
        accumulated_scopes = first_next;
        let mut stabilized = false;
        for iteration in 0..32 {
            self.scopes = accumulated_scopes.clone();
            let mut flow = None;
            self.analyze_without_records(|analyzer| {
                flow =
                    Some(analyzer.visit_for_loop_body(expression, &iterator_info, label.clone()));
            });
            let flow = flow.expect("for-loop analysis produced flow state");
            let post_body_scopes = self.scopes.clone();
            let mut next = accumulated_scopes.clone();
            merge_scope_stacks(&mut next, &post_body_scopes);
            if let Some(back_edges) = &flow.back_edges {
                merge_scope_stacks(&mut next, back_edges);
            }
            if iteration >= 7 {
                widen_loop_scopes(&mut next);
            }
            if next == accumulated_scopes {
                stabilized = true;
                break;
            }
            accumulated_scopes = next;
        }
        if !stabilized {
            self.errors.push(format!(
                "{} for-loop persistence flow did not reach a fixed point",
                self.enclosing
            ));
        }
        self.scopes = accumulated_scopes.clone();
        let flow = self.visit_for_loop_body(expression, &iterator_info, label);
        self.scopes = accumulated_scopes;
        if let Some(exits) = flow.exits {
            merge_scope_stacks(&mut self.scopes, &exits);
        }
        self.bump_context();
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        if !self.allows_source_class(&expression.attrs, "while expression") {
            return;
        }
        // The body may run zero times (false condition), so its assignments
        // to outer locals cannot be applied unconditionally: conservatively
        // union the pre-loop state with the post-body state.
        let mut accumulated_scopes = self.scopes.clone();
        let label = expression
            .label
            .as_ref()
            .map(|label| normalized_ident(&label.name.ident));
        let initial_error_count = self.errors.len();
        self.accumulator.begin_transaction();
        let first_flow = self.visit_while_loop_body(expression, label.clone());
        let first_post_body_scopes = self.scopes.clone();
        let mut first_next = accumulated_scopes.clone();
        merge_scope_stacks(&mut first_next, &first_post_body_scopes);
        if let Some(back_edges) = &first_flow.back_edges {
            merge_scope_stacks(&mut first_next, back_edges);
        }
        if first_next == accumulated_scopes {
            self.accumulator.commit_transaction();
            self.scopes = accumulated_scopes;
            if let Some(exits) = first_flow.exits {
                merge_scope_stacks(&mut self.scopes, &exits);
            }
            self.bump_context();
            return;
        }
        self.accumulator.rollback_transaction();
        self.errors.truncate(initial_error_count);
        accumulated_scopes = first_next;
        let mut stabilized = false;
        for iteration in 0..32 {
            self.scopes = accumulated_scopes.clone();
            let mut flow = None;
            self.analyze_without_records(|analyzer| {
                flow = Some(analyzer.visit_while_loop_body(expression, label.clone()));
            });
            let flow = flow.expect("while-loop analysis produced flow state");
            let post_body_scopes = self.scopes.clone();
            let mut next = accumulated_scopes.clone();
            merge_scope_stacks(&mut next, &post_body_scopes);
            if let Some(back_edges) = &flow.back_edges {
                merge_scope_stacks(&mut next, back_edges);
            }
            if iteration >= 7 {
                widen_loop_scopes(&mut next);
            }
            if next == accumulated_scopes {
                stabilized = true;
                break;
            }
            accumulated_scopes = next;
        }
        if !stabilized {
            self.errors.push(format!(
                "{} while-loop persistence flow did not reach a fixed point",
                self.enclosing
            ));
        }
        self.scopes = accumulated_scopes.clone();
        let flow = self.visit_while_loop_body(expression, label);
        self.scopes = accumulated_scopes;
        if let Some(exits) = flow.exits {
            merge_scope_stacks(&mut self.scopes, &exits);
        }
        self.bump_context();
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
        // Constructing a future does not execute its body. Audit the body for
        // captures and accesses, but keep both the declaration-time bindings
        // and the bindings that would result if the future were later polled.
        let declaration_scopes = self.scopes.clone();
        self.push_scope();
        self.visit_block(&expression.block);
        self.pop_scope();
        merge_scope_stacks(&mut self.scopes, &declaration_scopes);
        self.bump_context();
    }

    fn visit_expr_array(&mut self, expression: &'ast syn::ExprArray) {
        if !self.allows_source_class(&expression.attrs, "array expression") {
            return;
        }
        let cfg = item_cfg(&self.cfg, &expression.attrs);
        let mut flow = Flow::default();
        for element in &expression.elems {
            flow.union(self.subtree_flow(element));
        }
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
        let mut accumulated_scopes = self.scopes.clone();
        let label = expression
            .label
            .as_ref()
            .map(|label| normalized_ident(&label.name.ident));
        let initial_error_count = self.errors.len();
        self.accumulator.begin_transaction();
        let first_flow = self.visit_loop_block(&expression.body, label.clone());
        let first_post_body_scopes = self.scopes.clone();
        let mut first_next = accumulated_scopes.clone();
        merge_scope_stacks(&mut first_next, &first_post_body_scopes);
        if let Some(back_edges) = &first_flow.back_edges {
            merge_scope_stacks(&mut first_next, back_edges);
        }
        if first_next == accumulated_scopes {
            self.accumulator.commit_transaction();
            self.scopes = accumulated_scopes;
            if let Some(exits) = first_flow.exits {
                merge_scope_stacks(&mut self.scopes, &exits);
            }
            self.bump_context();
            return;
        }
        self.accumulator.rollback_transaction();
        self.errors.truncate(initial_error_count);
        accumulated_scopes = first_next;
        let mut stabilized = false;
        for iteration in 0..32 {
            self.scopes = accumulated_scopes.clone();
            self.loop_flow_collectors.push(LoopFlowCollector {
                label: label.clone(),
                ..LoopFlowCollector::default()
            });
            self.analyze_without_records(|analyzer| analyzer.visit_block(&expression.body));
            let flow = self
                .loop_flow_collectors
                .pop()
                .expect("loop flow collector was installed");
            let post_body_scopes = self.scopes.clone();
            let mut next = accumulated_scopes.clone();
            merge_scope_stacks(&mut next, &post_body_scopes);
            if let Some(back_edges) = &flow.back_edges {
                merge_scope_stacks(&mut next, back_edges);
            }
            if iteration >= 7 {
                widen_loop_scopes(&mut next);
            }
            if next == accumulated_scopes {
                stabilized = true;
                break;
            }
            accumulated_scopes = next;
        }
        if !stabilized {
            self.errors.push(format!(
                "{} loop persistence flow did not reach a fixed point",
                self.enclosing
            ));
        }
        self.scopes = accumulated_scopes.clone();
        let flow = self.visit_loop_block(&expression.body, label);
        self.scopes = accumulated_scopes;
        if let Some(exits) = flow.exits {
            merge_scope_stacks(&mut self.scopes, &exits);
        }
        self.bump_context();
    }

    fn visit_stmt(&mut self, statement: &'ast syn::Stmt) {
        if let syn::Stmt::Item(item) = statement {
            // Block-local `use` declarations are already modelled by
            // `register_local_uses`, which imports their aliases into the
            // lexical path scopes before any statement runs, so they neither
            // hide persistence nor need new inventory rows. Block-local item
            // macros keep the ordinary macro analysis.
            if matches!(item, Item::Use(_) | Item::Macro(_)) {
                syn::visit::visit_stmt(self, statement);
                return;
            }
            // Other block-local item declarations never reach
            // `collect_module_symbols`, so a local alias such as
            // `type Alias = sqlx::MySqlPool;` would let the body reach
            // concrete persistence without any inventory row. Fail closed on
            // any block-local item that mentions persistence instead of
            // silently delegating to the default traversal.
            if !self.allows_source_class(item_attributes(item), "block-local item") {
                return;
            }
            if !targets_in_tokens(item.to_token_stream(), self.symbols).is_empty()
                || syntax_mentions_persistence(item, self.symbols)
            {
                self.errors.push(format!(
                    "{} contains a block-local item that mentions concrete persistence; hoist the declaration to module scope so the symbol collector can model it: {}",
                    self.context.module,
                    normalized_tokens(item)
                ));
            } else if matches!(item, Item::Fn(_)) {
                let operations = persistence_operations_in_syntax(item);
                if !operations.is_empty() {
                    self.errors.push(format!(
                        "{} contains a block-local function with persistence-shaped operations ({}) whose generic receiver cannot be audited at call sites; hoist it to module scope",
                        self.context.module,
                        operations.into_iter().collect::<Vec<_>>().join(", ")
                    ));
                }
            }
            return;
        }
        syn::visit::visit_stmt(self, statement);
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.push_scope();
        self.register_local_uses(&block.stmts);
        self.register_local_callables(&block.stmts);
        for statement in &block.stmts {
            self.visit_stmt(statement);
        }
        if let Some(Stmt::Expr(expression, None)) = block.stmts.last() {
            let info = self.info_from_expr(expression);
            self.block_result_infos
                .borrow_mut()
                .entry(block as *const syn::Block as usize)
                .or_default()
                .union(&info);
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
                let continuing_scopes = self.scopes.clone();
                self.visit_expr(diverge);
                self.scopes = continuing_scopes;
                self.bump_context();
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
        // A nested projection is an implementation detail of the complete
        // field path. Auditing every prefix made `value.inner.clean` record
        // `value.inner` as a persistence reference merely because a sibling
        // field in `inner` was persistent. Still visit the non-field root so
        // calls, indices, and other side-effecting bases remain visible.
        let mut root = field.base.as_ref();
        while let Expr::Field(parent) = root {
            root = parent.base.as_ref();
        }
        self.visit_expr(root);
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
            if let Some(argument) = call.args.first() {
                match self.sql_expression_kind(argument) {
                    SqlExpressionKind::Static => {}
                    SqlExpressionKind::Included => self.errors.push(format!(
                        "{} passes include_str! SQL whose content is outside the persistence snapshot; mount and fingerprint the included SQL source explicitly",
                        self.enclosing
                    )),
                    SqlExpressionKind::Environment => self.errors.push(format!(
                        "{} passes env! SQL whose expanded content is outside the persistence snapshot; pin the SQL in reviewed source",
                        self.enclosing
                    )),
                    kind @ (SqlExpressionKind::Nonliteral | SqlExpressionKind::Interpolated) => {
                        self.add(
                            PersistenceTarget::Sqlx,
                            match kind {
                                SqlExpressionKind::Interpolated => {
                                    PersistenceOperation::InterpolatedSql
                                }
                                SqlExpressionKind::Nonliteral => {
                                    PersistenceOperation::NonliteralSql
                                }
                                SqlExpressionKind::Static
                                | SqlExpressionKind::Included
                                | SqlExpressionKind::Environment => {
                                    unreachable!()
                                }
                            },
                            &name,
                            &cfg,
                            normalized_tokens(argument),
                        );
                    }
                }
            }
        } else if let Some(operation) = PersistenceOperation::from_executor_method(&name).filter(
            |_| {
                rooted_sqlx
                    || has_path_targets
                    || (matches!(call.func.as_ref(), Expr::Path(path) if path.path.segments.len() >= 2)
                        && call
                            .args
                            .first()
                            .is_some_and(|receiver| !self.flow_of_expr(receiver).is_empty()))
            },
        )
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
            || (PersistenceOperation::from_executor_method(&name).is_some()
                && (rooted_sqlx
                    || has_path_targets
                    || (matches!(call.func.as_ref(), Expr::Path(path) if path.path.segments.len() >= 2)
                        && call
                            .args
                            .first()
                            .is_some_and(|receiver| !self.flow_of_expr(receiver).is_empty()))));
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
        if matches!(
            name.as_str(),
            "push" | "push_back" | "push_front" | "insert" | "extend" | "append"
        ) && matches!(call.func.as_ref(), Expr::Path(path) if path.path.segments.len() >= 2)
            && let Some(receiver_name) = call.args.first().and_then(mutable_storage_receiver_name)
        {
            let mut stored = self.lookup(&receiver_name).cloned().unwrap_or_default();
            let before = stored.clone();
            for argument in call.args.iter().skip(1) {
                stored.union(&self.info_from_expr(argument));
            }
            if stored != before {
                self.assign(&receiver_name, stored);
            }
        }
        if matches!(call.func.as_ref(), Expr::Path(path) if path.path.segments.len() >= 2)
            && matches!(name.as_str(), "replace" | "write")
            && let Some(receiver_name) = call.args.first().and_then(mutable_storage_receiver_name)
            && let Some(replacement) = call.args.iter().nth(1)
        {
            self.assign(&receiver_name, self.info_from_expr(replacement));
        }
        if matches!(call.func.as_ref(), Expr::Path(path) if path.path.segments.len() >= 2)
            && name == "swap"
            && let (Some(left), Some(right)) = (
                call.args.first().and_then(mutable_storage_receiver_name),
                call.args.get(1).and_then(mutable_storage_receiver_name),
            )
        {
            let mut merged = self.lookup(&left).cloned().unwrap_or_default();
            merged.union(&self.lookup(&right).cloned().unwrap_or_default());
            self.assign(&left, merged.clone());
            self.assign(&right, merged);
        }
        let argument_infos = call
            .args
            .iter()
            .map(|argument| self.info_from_expr(argument))
            .collect::<Vec<_>>();
        for (index, argument) in call.args.iter().enumerate() {
            let Some(binding) = mutable_storage_receiver_name(argument) else {
                continue;
            };
            let mut updated = self.lookup(&binding).cloned().unwrap_or_default();
            for (other_index, info) in argument_infos.iter().enumerate() {
                if other_index != index {
                    updated.union(info);
                }
            }
            self.assign(&binding, updated);
        }
        if let Expr::Path(path) = call.func.as_ref()
            && path.qself.is_none()
            && path.path.segments.len() == 1
            && let Some(callee) = last_path_name(&path.path)
        {
            let mutations = self
                .lookup(&callee)
                .map(|info| self.closure_mutations_for_args(info, &argument_infos))
                .unwrap_or_default();
            for (captured, info) in mutations {
                self.assign(&captured, info);
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
                    {
                        match self.sql_expression_kind(argument) {
                            SqlExpressionKind::Static => {}
                            SqlExpressionKind::Included => self.errors.push(format!(
                                "{} passes include_str! SQL whose content is outside the persistence snapshot; mount and fingerprint the included SQL source explicitly",
                                self.enclosing
                            )),
                            SqlExpressionKind::Environment => self.errors.push(format!(
                                "{} passes env! SQL whose expanded content is outside the persistence snapshot; pin the SQL in reviewed source",
                                self.enclosing
                            )),
                            kind @ (SqlExpressionKind::Nonliteral
                            | SqlExpressionKind::Interpolated) => self.add(
                                target,
                                match kind {
                                    SqlExpressionKind::Interpolated => {
                                        PersistenceOperation::InterpolatedSql
                                    }
                                    SqlExpressionKind::Nonliteral => {
                                        PersistenceOperation::NonliteralSql
                                    }
                                    SqlExpressionKind::Static
                                    | SqlExpressionKind::Included
                                    | SqlExpressionKind::Environment => {
                                        unreachable!()
                                    }
                                },
                                &name,
                                &cfg,
                                normalized_tokens(argument),
                            ),
                        }
                    }
                }
            }
        }
        if !valid_persistence_method && !validated_flow_passthrough {
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
        if matches!(
            name.as_str(),
            "push" | "push_back" | "push_front" | "insert" | "extend" | "append" | "replace"
        ) && let Some(receiver_name) = simple_assignment_name(&method.receiver)
        {
            let mut stored = self.lookup(&receiver_name).cloned().unwrap_or_default();
            let before = stored.clone();
            for argument in &method.args {
                stored.union(&self.info_from_expr(argument));
            }
            if stored != before {
                self.assign(&receiver_name, stored);
            }
        }
        self.visit_expr(&method.receiver);
        for argument in &method.args {
            self.visit_expr(argument);
        }
        if CLOSURE_INVOKING_METHODS.contains(&name.as_str()) {
            let pre_call_scopes = self.scopes.clone();
            let mut callback_argument = self.info_from_expr(&method.receiver);
            for argument in &method.args {
                if !matches!(argument, Expr::Closure(_)) {
                    callback_argument.union(&self.info_from_expr(argument));
                }
            }
            for argument in &method.args {
                let info = self.info_from_expr(argument);
                let mutations = self
                    .closure_mutations_for_args(&info, std::slice::from_ref(&callback_argument));
                for (captured, info) in mutations {
                    self.assign(&captured, info);
                }
            }
            let post_call_scopes = self.scopes.clone();
            self.scopes = pre_call_scopes;
            merge_scope_stacks(&mut self.scopes, &post_call_scopes);
            self.bump_context();
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
        } else if assign_destructured_expr(self, &assignment.left, &info) {
            self.record_flow(
                &info.flow,
                PersistenceOperation::ValueAlias,
                "destructuring_assignment",
                &cfg,
                normalized_tokens(assignment),
            );
        } else {
            if let Some((root, projections)) = assignment_place(&assignment.left) {
                let mut aggregate = self.lookup(&root).cloned().unwrap_or_default();
                assign_place_projection(&mut aggregate, &projections, &info);
                self.assign(&root, aggregate);
            }
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

    fn visit_expr_break(&mut self, expression: &'ast syn::ExprBreak) {
        if !self.allows_source_class(&expression.attrs, "break expression") {
            return;
        }
        if let Some(value) = &expression.expr {
            self.visit_expr(value);
        }
        self.capture_loop_control(expression.label.as_ref(), true);
    }

    fn visit_expr_continue(&mut self, expression: &'ast syn::ExprContinue) {
        if !self.allows_source_class(&expression.attrs, "continue expression") {
            return;
        }
        self.capture_loop_control(expression.label.as_ref(), false);
    }

    fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
        if !self.allows_source_class(&expression.attrs, "match expression") {
            return;
        }
        self.visit_expr(&expression.expr);
        let scrutinee = self.info_from_expr(&expression.expr);
        // Arms are mutually exclusive: visiting them against one shared
        // scope lets a later arm overwrite (or clear) an earlier arm's
        // assignment to an outer local. Snapshot the pre-match state and
        // conservatively union every arm's resulting flow instead.
        let pre_match_scopes = self.scopes.clone();
        let mut next_arm_scopes = pre_match_scopes.clone();
        let mut merged: Option<Vec<BTreeMap<String, VariableInfo>>> = None;
        for arm in &expression.arms {
            self.scopes = next_arm_scopes.clone();
            self.push_scope();
            self.bind_pattern(&arm.pat, &scrutinee);
            if let Some((_, guard)) = &arm.guard {
                self.visit_expr(guard);
                next_arm_scopes = self.scopes.clone();
                next_arm_scopes.pop();
            }
            self.visit_expr(&arm.body);
            self.pop_scope();
            let post_arm = self.scopes.clone();
            match &mut merged {
                None => merged = Some(post_arm),
                Some(accumulated) => merge_scope_stacks(accumulated, &post_arm),
            }
        }
        self.scopes = merged.unwrap_or(pre_match_scopes);
        merge_scope_stacks(&mut self.scopes, &next_arm_scopes);
        self.bump_context();
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        if !self.allows_source_class(&expression.attrs, "if expression") {
            return;
        }
        // The then/else branches are mutually exclusive: visiting them
        // sequentially against one shared scope lets the else branch erase
        // flow the then branch assigned to an outer local. Like match arms,
        // snapshot the pre-if state and conservatively union both outcomes,
        // including the no-`else` path.
        self.push_scope();
        visit_let_chain_condition(self, &expression.cond, false);
        let mut post_condition_scopes = self.scopes.clone();
        post_condition_scopes.pop();
        self.visit_block(&expression.then_branch);
        self.pop_scope();
        let post_then = self.scopes.clone();
        self.scopes = post_condition_scopes;
        if let Some((_, else_expression)) = &expression.else_branch {
            self.visit_expr(else_expression);
        }
        let post_else = self.scopes.clone();
        self.scopes = post_then;
        merge_scope_stacks(&mut self.scopes, &post_else);
        self.bump_context();
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
        let declaration_scopes = self.scopes.clone();
        self.cfg = item_cfg(&self.cfg, &closure.attrs);
        self.push_scope();
        let (_, parameter_infos) = closure_callable_model(closure);
        for (input, info) in closure.inputs.iter().zip(&parameter_infos) {
            self.bind_pattern(input, info);
        }
        self.visit_expr(&closure.body);
        let result_info = self.info_from_expr(&closure.body);
        self.closure_result_infos
            .borrow_mut()
            .insert(closure as *const ExprClosure as usize, result_info);
        self.pop_scope();
        let mut effects = BTreeMap::new();
        for (before_scope, after_scope) in declaration_scopes.iter().zip(&self.scopes).rev() {
            for (name, after) in after_scope {
                if before_scope.get(name).is_some_and(|before| before != after) {
                    effects.entry(name.clone()).or_insert_with(|| after.clone());
                }
            }
        }
        self.closure_effects
            .borrow_mut()
            .insert(closure as *const ExprClosure as usize, effects);
        self.scopes = declaration_scopes;
        self.bump_context();
        self.cfg = previous_cfg;
    }
}

fn analyze_trait_default_bodies(
    item_trait: &ItemTrait,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let trait_name = normalized_ident(&item_trait.ident);
    let trait_canonical = {
        let mut path = symbols.module_path.clone();
        path.push(trait_name.clone());
        path.join("::")
    };
    for trait_item in &item_trait.items {
        let TraitItem::Fn(method) = trait_item else {
            continue;
        };
        let Some(default_body) = &method.default else {
            continue;
        };
        if !source_class_allows(
            context.source_class,
            cfg,
            &method.attrs,
            errors,
            "trait default method",
        ) {
            continue;
        }
        let method_name = normalized_ident(&method.sig.ident);
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
            format!("trait {trait_name}::{method_name}"),
            normalized_visibility(&item_trait.vis),
            item_cfg(cfg, &method.attrs),
        );
        analyzer.register_generic_bounds(&item_trait.generics);
        analyzer.register_generic_bounds(&method.sig.generics);
        analyzer.register_parameters(&method.sig.inputs);
        analyzer.bind(
            "self".to_owned(),
            VariableInfo {
                nominal_types: BTreeSet::from([trait_name.clone()]),
                trait_bounds: BTreeSet::from([trait_canonical.clone()]),
                ..VariableInfo::default()
            },
        );
        analyzer.visit_block(default_body);
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
    let generic_operations = function
        .sig
        .generics
        .params
        .iter()
        .any(|param| matches!(param, syn::GenericParam::Type(_)))
        .then(|| persistence_operations_in_block(&function.block))
        .unwrap_or_default();
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
        enclosing.clone(),
        visibility,
        cfg.clone(),
    );
    analyzer.register_local_uses(&function.block.stmts);
    analyzer.register_local_callables(&function.block.stmts);
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
    drop(analyzer);
    let unresolved = generic_operations
        .into_iter()
        .filter(|operation| !accumulator.contains_symbol(&enclosing, operation))
        .collect::<Vec<_>>();
    if !unresolved.is_empty() {
        errors.push(format!(
            "{enclosing} is generic and contains persistence-shaped operations ({}) that cannot be instantiated from declared flow; expose a typed module-level adapter instead",
            unresolved.join(", ")
        ));
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
    let trait_name = item_impl
        .trait_
        .as_ref()
        .map(|(_, trait_path, _)| canonical_path_names(path_names(trait_path), symbols).join("::"));
    let impl_enclosing = trait_name
        .as_ref()
        .map(|trait_name| format!("impl {trait_name} for {self_name}"))
        .unwrap_or_else(|| format!("impl {self_name}"));
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
        let enclosing = trait_name
            .as_ref()
            .map(|trait_name| {
                format!(
                    "impl {trait_name} for {self_name}::{}",
                    normalized_ident(&method.sig.ident)
                )
            })
            .unwrap_or_else(|| {
                format!("impl {self_name}::{}", normalized_ident(&method.sig.ident))
            });
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
        analyzer.register_local_callables(&method.block.stmts);
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
    if path_name == "include" && !is_pinned_wow_proto_include(context, &item_macro.mac) {
        errors.push(format!(
            "{} contains include! whose Rust source is outside the persistence AST inventory; mount and parse the included source explicitly",
            context.module
        ));
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

fn is_pinned_wow_proto_include(context: &RecordContext<'_>, mac: &syn::Macro) -> bool {
    if context.package != "wow-proto" || context.source != "crates/wow-proto/src/lib.rs" {
        return false;
    }
    let suffix = match context.module {
        "crate::bgs::protocol" => "/bgs.protocol.rs",
        "crate::bgs::protocol::account::v1" => "/bgs.protocol.account.v1.rs",
        "crate::bgs::protocol::authentication::v1" => "/bgs.protocol.authentication.v1.rs",
        "crate::bgs::protocol::challenge::v1" => "/bgs.protocol.challenge.v1.rs",
        "crate::bgs::protocol::connection::v1" => "/bgs.protocol.connection.v1.rs",
        "crate::bgs::protocol::game_utilities::v1" => "/bgs.protocol.game_utilities.v1.rs",
        _ => return false,
    };
    let expected: syn::ItemMacro = syn::parse_str(&format!(
        "include!(concat!(env!(\"OUT_DIR\"), {suffix:?}));"
    ))
    .expect("pinned wow-proto include syntax is valid");
    normalized_tokens(mac) == normalized_tokens(&expected.mac)
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
                let visibility = normalized_visibility(&item_const.vis);
                add_attribute_records(
                    accumulator,
                    &context,
                    &symbols,
                    &item_const.attrs,
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
                    &item_const.ty,
                    &enclosing,
                    &normalized_ident(&item_const.ident),
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
                    // A default trait body is ordinary executable code:
                    // analyze it before anything else, because it can reach
                    // persistence through a nominal wrapper
                    // (`self.holder().0.pool()`) even when the trait
                    // signature never names a concrete database type.
                    analyze_trait_default_bodies(
                        item_trait,
                        &context,
                        &symbols,
                        &cfg,
                        accumulator,
                        errors,
                    );
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
fn workspace_named_type_info(
    registries: &BTreeMap<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>,
    source_class: PersistenceSourceClass,
    provider_roots: &BTreeSet<String>,
) -> BTreeMap<String, VariableInfo> {
    let mut workspace = BTreeMap::<String, VariableInfo>::new();
    for ((provider, candidate_class), named_types) in registries {
        if *candidate_class != source_class {
            continue;
        }
        let crate_name = provider.replace('-', "_");
        if !provider_roots.contains(&crate_name) {
            continue;
        }
        for (path, info) in named_types {
            workspace
                .entry(format!("{crate_name}::{path}"))
                .or_default()
                .union(info);
        }
    }
    workspace
}

fn workspace_named_type_info_cache(
    registries: &BTreeMap<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>,
    dependencies: &WorkspaceDependencyAliases,
) -> BTreeMap<(String, PersistenceSourceClass), std::sync::Arc<BTreeMap<String, VariableInfo>>> {
    dependencies
        .production
        .iter()
        .map(|(package, aliases)| {
            let roots = aliases.values().cloned().collect();
            (
                (package.clone(), PersistenceSourceClass::Production),
                std::sync::Arc::new(workspace_named_type_info(
                    registries,
                    PersistenceSourceClass::Production,
                    &roots,
                )),
            )
        })
        .chain(dependencies.test.iter().map(|(package, aliases)| {
            let roots = aliases.values().cloned().collect();
            (
                (package.clone(), PersistenceSourceClass::TestFixture),
                std::sync::Arc::new(workspace_named_type_info(
                    registries,
                    PersistenceSourceClass::TestFixture,
                    &roots,
                )),
            )
        }))
        .collect()
}

fn dependency_sorted_packages(
    sources: &[ClassifiedPersistenceSource<'_>],
    dependencies: &WorkspaceDependencyAliases,
) -> Vec<String> {
    let mut remaining = sources
        .iter()
        .map(|source| source.package.to_owned())
        .collect::<BTreeSet<_>>();
    let package_by_root = remaining
        .iter()
        .map(|package| (package.replace('-', "_"), package.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut ordered = Vec::new();
    while !remaining.is_empty() {
        let ready = remaining.iter().find(|package| {
            dependencies
                .production
                .get(*package)
                .into_iter()
                .chain(dependencies.test.get(*package))
                .flat_map(|aliases| aliases.values())
                .filter_map(|root| package_by_root.get(root))
                .all(|provider| !remaining.contains(provider) || provider == *package)
        });
        // A dependency cycle cannot be topologically ordered. Pick its stable
        // first member; the outer fixed-point loop still converges the cycle.
        let package = ready
            .cloned()
            .unwrap_or_else(|| remaining.first().expect("remaining is non-empty").clone());
        remaining.remove(&package);
        ordered.push(package);
    }
    ordered
}

fn package_named_type_info_cache(
    registries: &BTreeMap<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>,
) -> BTreeMap<(String, PersistenceSourceClass), std::sync::Arc<BTreeMap<String, VariableInfo>>> {
    registries
        .iter()
        .map(|(key, info)| (key.clone(), std::sync::Arc::new(info.clone())))
        .collect()
}

fn dependency_alias_cache(
    dependencies: &WorkspaceDependencyAliases,
) -> BTreeMap<(String, PersistenceSourceClass), std::sync::Arc<BTreeMap<String, String>>> {
    dependencies
        .production
        .iter()
        .map(|(package, aliases)| {
            (
                (package.clone(), PersistenceSourceClass::Production),
                std::sync::Arc::new(aliases.clone()),
            )
        })
        .chain(dependencies.test.iter().map(|(package, aliases)| {
            (
                (package.clone(), PersistenceSourceClass::TestFixture),
                std::sync::Arc::new(aliases.clone()),
            )
        }))
        .collect()
}

fn qualify_dependency_shape(
    shape: &mut NominalShape,
    provider_root: &str,
    provider_named_types: &BTreeMap<String, VariableInfo>,
) {
    shape.nominal_types = std::mem::take(&mut shape.nominal_types)
        .into_iter()
        .map(|name| {
            if provider_named_types.contains_key(&name) {
                format!("{provider_root}::{name}")
            } else {
                name
            }
        })
        .collect();
    for argument in &mut shape.arguments {
        qualify_dependency_shape(argument, provider_root, provider_named_types);
    }
}

fn qualify_dependency_info(
    provider_root: &str,
    provider_named_types: &BTreeMap<String, VariableInfo>,
    info: &VariableInfo,
) -> VariableInfo {
    let mut qualified = info.clone();
    qualified.nominal_types = std::mem::take(&mut qualified.nominal_types)
        .into_iter()
        .map(|name| {
            if provider_named_types.contains_key(&name) {
                format!("{provider_root}::{name}")
            } else {
                name
            }
        })
        .collect();
    qualified.payload_variants = std::mem::take(&mut qualified.payload_variants)
        .into_iter()
        .map(|mut variant| {
            for shape in &mut variant {
                qualify_dependency_shape(shape, provider_root, provider_named_types);
            }
            variant
        })
        .collect();
    for item in &mut qualified.tuple_items {
        *item = qualify_dependency_info(provider_root, provider_named_types, item);
    }
    for item in qualified.field_items.values_mut() {
        *item = qualify_dependency_info(provider_root, provider_named_types, item);
    }
    qualified
}

fn resolve_public_named_type_reexports(
    reexports: &BTreeMap<(String, PersistenceSourceClass), Vec<(String, String)>>,
    dependencies: &WorkspaceDependencyAliases,
    registries: &mut BTreeMap<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>,
    public_paths: &mut BTreeMap<(String, PersistenceSourceClass), BTreeSet<String>>,
) {
    let pass_limit = reexports.values().map(Vec::len).sum::<usize>() + 1;
    for _ in 0..pass_limit {
        let before = registries.clone();
        let snapshot = registries.clone();
        let public_snapshot = public_paths.clone();
        for (consumer_key, aliases) in reexports {
            let (consumer, source_class) = (&consumer_key.0, consumer_key.1);
            let dependency_roots = match source_class {
                PersistenceSourceClass::Production => dependencies.production.get(consumer),
                PersistenceSourceClass::TestFixture => dependencies.test.get(consumer),
            };
            for (export, source) in aliases {
                let glob = (source == "*" || source.ends_with("::*"))
                    && (export == "*" || export.ends_with("::*"));
                let source = if source == "*" {
                    ""
                } else {
                    source.strip_suffix("::*").unwrap_or(source)
                };
                let export = if export == "*" {
                    ""
                } else {
                    export.strip_suffix("::*").unwrap_or(export)
                };
                let mut source_parts = source.split("::");
                let source_root = source_parts.next().unwrap_or_default();
                let dependency_root = dependency_roots
                    .into_iter()
                    .flat_map(|aliases| aliases.values())
                    .find(|root| root.as_str() == source_root);
                let (provider_key, provider_entry, provider_root) =
                    if let Some(provider_root) = dependency_root {
                        let provider = snapshot.keys().find_map(|(candidate, candidate_class)| {
                            (*candidate_class == source_class
                                && candidate.replace('-', "_") == provider_root.as_str())
                            .then_some(candidate.clone())
                        });
                        let Some(provider) = provider else {
                            continue;
                        };
                        (
                            (provider, source_class),
                            source_parts.collect::<Vec<_>>().join("::"),
                            Some(provider_root.as_str()),
                        )
                    } else {
                        (consumer_key.clone(), source.to_owned(), None)
                    };
                let Some(provider_registry) = snapshot.get(&provider_key) else {
                    continue;
                };
                let provider_public = public_snapshot
                    .get(&provider_key)
                    .cloned()
                    .unwrap_or_default();
                let entries = if glob {
                    let prefix = (!provider_entry.is_empty())
                        .then(|| format!("{provider_entry}::"))
                        .unwrap_or_default();
                    provider_registry
                        .iter()
                        .filter_map(|(entry, info)| {
                            if !provider_public.contains(entry) {
                                return None;
                            }
                            entry.strip_prefix(&prefix).map(|suffix| {
                                let exported = if export.is_empty() {
                                    suffix.to_owned()
                                } else {
                                    format!("{export}::{suffix}")
                                };
                                (exported, info.clone())
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    let mut entries = Vec::new();
                    if provider_public.contains(&provider_entry)
                        && let Some(info) = provider_registry.get(&provider_entry)
                    {
                        entries.push((export.to_owned(), info.clone()));
                    }
                    let prefix = format!("{provider_entry}::");
                    entries.extend(provider_registry.iter().filter_map(|(entry, info)| {
                        if !provider_public.contains(entry) {
                            return None;
                        }
                        entry.strip_prefix(&prefix).map(|suffix| {
                            let exported = if export.is_empty() {
                                suffix.to_owned()
                            } else {
                                format!("{export}::{suffix}")
                            };
                            (exported, info.clone())
                        })
                    }));
                    entries
                };
                for (exported, source_info) in entries {
                    let info = if let Some(provider_root) = provider_root {
                        qualify_dependency_info(provider_root, provider_registry, &source_info)
                    } else {
                        source_info
                    };
                    registries
                        .entry(consumer_key.clone())
                        .or_default()
                        .entry(exported.clone())
                        .or_default()
                        .union(&info);
                    public_paths
                        .entry(consumer_key.clone())
                        .or_default()
                        .insert(exported);
                }
            }
        }
        if registries == &before {
            break;
        }
    }
}

fn resolve_public_callable_reexports(
    reexports: &BTreeMap<(String, PersistenceSourceClass), Vec<(String, String)>>,
    named_type_registries: &BTreeMap<
        (String, PersistenceSourceClass),
        BTreeMap<String, VariableInfo>,
    >,
    dependencies: &WorkspaceDependencyAliases,
    function_registries: &mut BTreeMap<
        (String, PersistenceSourceClass),
        BTreeMap<String, VariableInfo>,
    >,
    generic_registries: &mut BTreeMap<
        (String, PersistenceSourceClass),
        BTreeMap<String, Vec<String>>,
    >,
    generic_input_registries: &mut BTreeMap<
        (String, PersistenceSourceClass),
        BTreeMap<String, Vec<GenericInputSpec>>,
    >,
) {
    let pass_limit = reexports.values().map(Vec::len).sum::<usize>() + 1;
    for _ in 0..pass_limit {
        let before = function_registries.clone();
        let function_snapshot = function_registries.clone();
        let generic_snapshot = generic_registries.clone();
        let generic_input_snapshot = generic_input_registries.clone();
        for (consumer_key, aliases) in reexports {
            let (consumer, source_class) = (&consumer_key.0, consumer_key.1);
            let dependency_roots = match source_class {
                PersistenceSourceClass::Production => dependencies.production.get(consumer),
                PersistenceSourceClass::TestFixture => dependencies.test.get(consumer),
            };
            for (export, source) in aliases {
                let glob = (source == "*" || source.ends_with("::*"))
                    && (export == "*" || export.ends_with("::*"));
                let source = if source == "*" {
                    ""
                } else {
                    source.strip_suffix("::*").unwrap_or(source)
                };
                let export = if export == "*" {
                    ""
                } else {
                    export.strip_suffix("::*").unwrap_or(export)
                };
                let mut source_parts = source.split("::");
                let source_root = source_parts.next().unwrap_or_default();
                let dependency_root = dependency_roots
                    .into_iter()
                    .flat_map(|aliases| aliases.values())
                    .find(|root| root.as_str() == source_root);
                let (provider_key, provider_entry, provider_root) =
                    if let Some(provider_root) = dependency_root {
                        let provider =
                            function_snapshot
                                .keys()
                                .find_map(|(candidate, candidate_class)| {
                                    (*candidate_class == source_class
                                        && candidate.replace('-', "_") == provider_root.as_str())
                                    .then_some(candidate.clone())
                                });
                        let Some(provider) = provider else {
                            continue;
                        };
                        let entry = source_parts.collect::<Vec<_>>().join("::");
                        (
                            (provider, source_class),
                            entry,
                            Some(provider_root.as_str()),
                        )
                    } else {
                        (consumer_key.clone(), source.to_owned(), None)
                    };
                let Some(provider_registry) = function_snapshot.get(&provider_key) else {
                    continue;
                };
                let entries = if glob {
                    let prefix = (!provider_entry.is_empty())
                        .then(|| format!("{provider_entry}::"))
                        .unwrap_or_default();
                    provider_registry
                        .iter()
                        .filter_map(|(entry, info)| {
                            entry.strip_prefix(&prefix).map(|suffix| {
                                let exported = if export.is_empty() {
                                    suffix.to_owned()
                                } else {
                                    format!("{export}::{suffix}")
                                };
                                (exported, entry.clone(), info.clone())
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    let mut entries = provider_registry
                        .get(&provider_entry)
                        .cloned()
                        .map(|info| vec![(export.to_owned(), provider_entry.clone(), info)])
                        .unwrap_or_default();
                    let prefix = if provider_entry.is_empty() {
                        String::new()
                    } else {
                        format!("{provider_entry}::")
                    };
                    entries.extend(provider_registry.iter().filter_map(|(entry, info)| {
                        entry.strip_prefix(&prefix).map(|suffix| {
                            let exported = if export.is_empty() {
                                suffix.to_owned()
                            } else {
                                format!("{export}::{suffix}")
                            };
                            (exported, entry.clone(), info.clone())
                        })
                    }));
                    entries
                };
                for (exported, source_entry, source_info) in entries {
                    let info = if let Some(provider_root) = provider_root {
                        let named = named_type_registries
                            .get(&provider_key)
                            .cloned()
                            .unwrap_or_default();
                        qualify_dependency_info(provider_root, &named, &source_info)
                    } else {
                        source_info
                    };
                    function_registries
                        .entry(consumer_key.clone())
                        .or_default()
                        .entry(exported.clone())
                        .or_default()
                        .union(&info);
                    if let Some(params) = generic_snapshot
                        .get(&provider_key)
                        .and_then(|registry| registry.get(&source_entry))
                    {
                        generic_registries
                            .entry(consumer_key.clone())
                            .or_default()
                            .entry(exported.clone())
                            .or_insert_with(|| params.clone());
                    }
                    if let Some(inputs) = generic_input_snapshot
                        .get(&provider_key)
                        .and_then(|registry| registry.get(&source_entry))
                    {
                        generic_input_registries
                            .entry(consumer_key.clone())
                            .or_default()
                            .entry(exported)
                            .or_insert_with(|| inputs.clone());
                    }
                }
            }
        }
        if function_registries == &before {
            break;
        }
    }
}

fn resolve_public_macro_reexports(
    reexports: &BTreeMap<(String, PersistenceSourceClass), Vec<(String, String)>>,
    dependencies: &WorkspaceDependencyAliases,
    registries: &mut BTreeMap<(String, PersistenceSourceClass), BTreeMap<String, TargetSet>>,
) {
    let pass_limit = reexports.values().map(Vec::len).sum::<usize>() + 1;
    for _ in 0..pass_limit {
        let before = registries.clone();
        let snapshot = registries.clone();
        for (consumer_key, aliases) in reexports {
            let (consumer, source_class) = (&consumer_key.0, consumer_key.1);
            let dependency_roots = match source_class {
                PersistenceSourceClass::Production => dependencies.production.get(consumer),
                PersistenceSourceClass::TestFixture => dependencies.test.get(consumer),
            };
            for (export, source) in aliases {
                let glob = (source == "*" || source.ends_with("::*"))
                    && (export == "*" || export.ends_with("::*"));
                let source = if source == "*" {
                    ""
                } else {
                    source.strip_suffix("::*").unwrap_or(source)
                };
                let export = if export == "*" {
                    ""
                } else {
                    export.strip_suffix("::*").unwrap_or(export)
                };
                let mut source_parts = source.split("::");
                let source_root = source_parts.next().unwrap_or_default();
                let dependency_root = dependency_roots
                    .into_iter()
                    .flat_map(|aliases| aliases.values())
                    .find(|root| root.as_str() == source_root);
                let (provider_key, provider_entry) = if let Some(provider_root) = dependency_root {
                    let provider = snapshot.keys().find_map(|(candidate, candidate_class)| {
                        (*candidate_class == source_class
                            && candidate.replace('-', "_") == provider_root.as_str())
                        .then_some(candidate.clone())
                    });
                    let Some(provider) = provider else {
                        continue;
                    };
                    (
                        (provider, source_class),
                        source_parts.collect::<Vec<_>>().join("::"),
                    )
                } else {
                    (consumer_key.clone(), source.to_owned())
                };
                let Some(provider_registry) = snapshot.get(&provider_key) else {
                    continue;
                };
                let entries = if glob {
                    let prefix = (!provider_entry.is_empty())
                        .then(|| format!("{provider_entry}::"))
                        .unwrap_or_default();
                    provider_registry
                        .iter()
                        .filter_map(|(entry, targets)| {
                            entry.strip_prefix(&prefix).map(|suffix| {
                                let exported = if export.is_empty() {
                                    suffix.to_owned()
                                } else {
                                    format!("{export}::{suffix}")
                                };
                                (exported, targets.clone())
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    let mut entries = provider_registry
                        .get(&provider_entry)
                        .cloned()
                        .map(|targets| vec![(export.to_owned(), targets)])
                        .unwrap_or_default();
                    let prefix = if provider_entry.is_empty() {
                        String::new()
                    } else {
                        format!("{provider_entry}::")
                    };
                    entries.extend(provider_registry.iter().filter_map(|(entry, targets)| {
                        entry.strip_prefix(&prefix).map(|suffix| {
                            let exported = if export.is_empty() {
                                suffix.to_owned()
                            } else {
                                format!("{export}::{suffix}")
                            };
                            (exported, targets.clone())
                        })
                    }));
                    entries
                };
                for (exported, targets) in entries {
                    registries
                        .entry(consumer_key.clone())
                        .or_default()
                        .entry(exported)
                        .or_default()
                        .extend(targets);
                }
            }
        }
        if registries == &before {
            break;
        }
    }
}

type TraitSignatureRegistry = (
    BTreeMap<(String, String), VariableInfo>,
    BTreeMap<String, BTreeSet<String>>,
    BTreeMap<String, Vec<String>>,
    BTreeMap<(String, String), Vec<String>>,
    BTreeMap<(String, String), Vec<GenericInputSpec>>,
);

fn resolve_public_trait_reexports(
    reexports: &BTreeMap<(String, PersistenceSourceClass), Vec<(String, String)>>,
    named_type_registries: &BTreeMap<
        (String, PersistenceSourceClass),
        BTreeMap<String, VariableInfo>,
    >,
    dependencies: &WorkspaceDependencyAliases,
    registries: &mut BTreeMap<(String, PersistenceSourceClass), TraitSignatureRegistry>,
) {
    let pass_limit = reexports.values().map(Vec::len).sum::<usize>() + 1;
    for _ in 0..pass_limit {
        let before = registries.clone();
        let snapshot = registries.clone();
        for (consumer_key, aliases) in reexports {
            let (consumer, source_class) = (&consumer_key.0, consumer_key.1);
            let dependency_roots = match source_class {
                PersistenceSourceClass::Production => dependencies.production.get(consumer),
                PersistenceSourceClass::TestFixture => dependencies.test.get(consumer),
            };
            for (export, source) in aliases {
                let glob = (source == "*" || source.ends_with("::*"))
                    && (export == "*" || export.ends_with("::*"));
                let source = if source == "*" {
                    ""
                } else {
                    source.strip_suffix("::*").unwrap_or(source)
                };
                let export = if export == "*" {
                    ""
                } else {
                    export.strip_suffix("::*").unwrap_or(export)
                };
                let mut source_parts = source.split("::");
                let source_root = source_parts.next().unwrap_or_default();
                let dependency_root = dependency_roots
                    .into_iter()
                    .flat_map(|aliases| aliases.values())
                    .find(|root| root.as_str() == source_root);
                let (provider_key, source_trait, provider_root) =
                    if let Some(provider_root) = dependency_root {
                        let provider = snapshot.keys().find_map(|(candidate, candidate_class)| {
                            (*candidate_class == source_class
                                && candidate.replace('-', "_") == provider_root.as_str())
                            .then_some(candidate.clone())
                        });
                        let Some(provider) = provider else {
                            continue;
                        };
                        (
                            (provider, source_class),
                            source_parts.collect::<Vec<_>>().join("::"),
                            Some(provider_root.as_str()),
                        )
                    } else {
                        (consumer_key.clone(), source.to_owned(), None)
                    };
                let Some(provider) = snapshot.get(&provider_key) else {
                    continue;
                };
                let mut provider_traits = provider
                    .0
                    .keys()
                    .map(|(trait_path, _)| trait_path.clone())
                    .collect::<BTreeSet<_>>();
                provider_traits.extend(provider.1.keys().cloned());
                provider_traits.extend(provider.2.keys().cloned());
                provider_traits.extend(provider.3.keys().map(|(path, _)| path.clone()));
                provider_traits.extend(provider.4.keys().map(|(path, _)| path.clone()));
                let source_prefix = (!source_trait.is_empty())
                    .then(|| format!("{source_trait}::"))
                    .unwrap_or_default();
                let candidates = provider_traits
                    .into_iter()
                    .filter_map(|trait_path| {
                        if glob {
                            let suffix = trait_path.strip_prefix(&source_prefix)?.to_owned();
                            let exported = if export.is_empty() {
                                suffix
                            } else {
                                format!("{export}::{suffix}")
                            };
                            Some((exported, trait_path))
                        } else {
                            (trait_path == source_trait).then(|| (export.to_owned(), trait_path))
                        }
                    })
                    .collect::<Vec<_>>();
                for (exported_trait, source_trait) in candidates {
                    let mut pending = vec![source_trait.clone()];
                    let mut inherited = BTreeSet::new();
                    while let Some(trait_path) = pending.pop() {
                        if !inherited.insert(trait_path.clone()) {
                            continue;
                        }
                        if let Some(supertraits) = provider.1.get(&trait_path) {
                            pending.extend(supertraits.iter().cloned());
                        }
                    }
                    let target = registries.entry(consumer_key.clone()).or_default();
                    for inherited_trait in inherited {
                        for ((trait_path, method), info) in &provider.0 {
                            if trait_path != &inherited_trait {
                                continue;
                            }
                            let info = if let Some(provider_root) = provider_root {
                                let named = named_type_registries
                                    .get(&provider_key)
                                    .cloned()
                                    .unwrap_or_default();
                                qualify_dependency_info(provider_root, &named, info)
                            } else {
                                info.clone()
                            };
                            target
                                .0
                                .entry((exported_trait.clone(), method.clone()))
                                .or_default()
                                .union(&info);
                            if let Some(params) =
                                provider.3.get(&(trait_path.clone(), method.clone()))
                            {
                                target
                                    .3
                                    .entry((exported_trait.clone(), method.clone()))
                                    .or_insert_with(|| params.clone());
                            }
                            if let Some(inputs) =
                                provider.4.get(&(trait_path.clone(), method.clone()))
                            {
                                target
                                    .4
                                    .entry((exported_trait.clone(), method.clone()))
                                    .or_insert_with(|| inputs.clone());
                            }
                        }
                    }
                    if let Some(params) = provider.2.get(&source_trait) {
                        target
                            .2
                            .entry(exported_trait)
                            .or_insert_with(|| params.clone());
                    }
                }
            }
        }
        if registries == &before {
            break;
        }
    }
}

fn dependency_scoped_trait_supertrait_cache(
    registries: &BTreeMap<(String, PersistenceSourceClass), BTreeMap<String, BTreeSet<String>>>,
    trait_names: &BTreeMap<(String, PersistenceSourceClass), BTreeSet<String>>,
    dependencies: &WorkspaceDependencyAliases,
) -> BTreeMap<(String, PersistenceSourceClass), std::sync::Arc<BTreeMap<String, BTreeSet<String>>>>
{
    let mut consumers = registries.keys().cloned().collect::<BTreeSet<_>>();
    consumers.extend(
        dependencies
            .production
            .keys()
            .cloned()
            .map(|package| (package, PersistenceSourceClass::Production)),
    );
    consumers.extend(
        dependencies
            .test
            .keys()
            .cloned()
            .map(|package| (package, PersistenceSourceClass::TestFixture)),
    );
    consumers
        .into_iter()
        .map(|key| {
            let (package, source_class) = (&key.0, key.1);
            let mut scoped = registries.get(&key).cloned().unwrap_or_default();
            let aliases = match source_class {
                PersistenceSourceClass::Production => dependencies.production.get(package),
                PersistenceSourceClass::TestFixture => dependencies.test.get(package),
            };
            for provider_root in aliases
                .into_iter()
                .flat_map(|aliases| aliases.values())
                .collect::<BTreeSet<_>>()
            {
                let provider = registries.keys().find_map(|(candidate, candidate_class)| {
                    (*candidate_class == source_class
                        && candidate.replace('-', "_") == provider_root.as_str())
                    .then_some(candidate)
                });
                let Some(provider) = provider else {
                    continue;
                };
                let provider_key = (provider.clone(), source_class);
                let provider_names = trait_names.get(&provider_key);
                if let Some(entries) = registries.get(&provider_key) {
                    for (trait_path, supertraits) in entries {
                        scoped.insert(
                            format!("{provider_root}::{trait_path}"),
                            supertraits
                                .iter()
                                .map(|supertrait| {
                                    if provider_names
                                        .is_some_and(|names| names.contains(supertrait))
                                    {
                                        format!("{provider_root}::{supertrait}")
                                    } else {
                                        supertrait.clone()
                                    }
                                })
                                .collect(),
                        );
                    }
                }
            }
            (key, std::sync::Arc::new(scoped))
        })
        .collect()
}

fn dependency_scoped_registry_cache<K, V, QualifyKey, QualifyValue>(
    registries: &BTreeMap<(String, PersistenceSourceClass), BTreeMap<K, V>>,
    named_type_registries: &BTreeMap<
        (String, PersistenceSourceClass),
        BTreeMap<String, VariableInfo>,
    >,
    dependencies: &WorkspaceDependencyAliases,
    qualify_key: QualifyKey,
    qualify_value: QualifyValue,
) -> BTreeMap<(String, PersistenceSourceClass), std::sync::Arc<BTreeMap<K, V>>>
where
    K: Clone + Ord,
    V: Clone,
    QualifyKey: Fn(&str, &K) -> K,
    QualifyValue: Fn(&str, &BTreeMap<String, VariableInfo>, &V) -> V,
{
    let mut consumers = registries.keys().cloned().collect::<BTreeSet<_>>();
    consumers.extend(
        dependencies
            .production
            .keys()
            .cloned()
            .map(|package| (package, PersistenceSourceClass::Production)),
    );
    consumers.extend(
        dependencies
            .test
            .keys()
            .cloned()
            .map(|package| (package, PersistenceSourceClass::TestFixture)),
    );
    consumers
        .into_iter()
        .map(|key| {
            let (package, source_class) = (&key.0, key.1);
            let mut scoped = registries.get(&key).cloned().unwrap_or_default();
            let aliases = match source_class {
                PersistenceSourceClass::Production => dependencies.production.get(package),
                PersistenceSourceClass::TestFixture => dependencies.test.get(package),
            };
            for provider_root in aliases
                .into_iter()
                .flat_map(|aliases| aliases.values())
                .collect::<BTreeSet<_>>()
            {
                let provider = registries.keys().find_map(|(candidate, candidate_class)| {
                    (*candidate_class == source_class
                        && candidate.replace('-', "_") == provider_root.as_str())
                    .then_some(candidate)
                });
                let Some(provider) = provider else {
                    continue;
                };
                let provider_key = (provider.clone(), source_class);
                let provider_named_types = named_type_registries
                    .get(&provider_key)
                    .cloned()
                    .unwrap_or_default();
                if let Some(entries) = registries.get(&provider_key) {
                    for (entry_key, value) in entries {
                        scoped.insert(
                            qualify_key(provider_root, entry_key),
                            qualify_value(provider_root, &provider_named_types, value),
                        );
                    }
                }
            }
            (key, std::sync::Arc::new(scoped))
        })
        .collect()
}

#[cfg(test)]
pub(crate) fn inventory_persistence_accesses(
    sources: &[ClassifiedPersistenceSource<'_>],
) -> Result<PersistenceAccessBaseline, String> {
    inventory_persistence_accesses_with_dependencies(
        sources,
        &WorkspaceDependencyAliases::default(),
    )
}

pub(crate) fn inventory_persistence_accesses_with_dependencies(
    sources: &[ClassifiedPersistenceSource<'_>],
    dependencies: &WorkspaceDependencyAliases,
) -> Result<PersistenceAccessBaseline, String> {
    let mut ordered = sources.to_vec();
    let package_order = dependency_sorted_packages(sources, dependencies);
    let package_order = package_order
        .into_iter()
        .enumerate()
        .map(|(index, package)| (package, index))
        .collect::<BTreeMap<_, _>>();
    ordered.sort_by(|left, right| {
        (
            package_order.get(left.package),
            left.classification,
            left.package,
            left.module,
            left.source_path,
            left.inherited_cfg,
        )
            .cmp(&(
                package_order.get(right.package),
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
    let dependency_aliases = dependency_alias_cache(dependencies);
    let mut named_type_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>::new();
    for _ in 0..=ordered.len() {
        let before = named_type_registries.clone();
        let mut next = before.clone();
        let mut package_start = 0;
        while package_start < ordered.len() {
            let package = ordered[package_start].package;
            let package_end = ordered[package_start..]
                .iter()
                .position(|source| source.package != package)
                .map_or(ordered.len(), |offset| package_start + offset);
            let workspace_cache = workspace_named_type_info_cache(&next, dependencies);
            let package_cache = package_named_type_info_cache(&next);
            for source in &ordered[package_start..package_end] {
                let Ok(syntax) = syn::parse_file(source.source) else {
                    continue;
                };
                let cfg = extend_cfg_context(source.inherited_cfg, &syntax.attrs);
                for source_class in [
                    PersistenceSourceClass::Production,
                    PersistenceSourceClass::TestFixture,
                ] {
                    if !source_class_allows(source_class, &cfg, &[], &mut errors, "source file") {
                        continue;
                    }
                    let key = (source.package.to_owned(), source_class);
                    let mut base = ModuleSymbols::for_package(source.package);
                    base.named_type_info = package_cache
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
                    base.workspace_named_type_info = workspace_cache
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
                    base.dependency_crate_aliases = dependency_aliases
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
                    let output = next.entry(key).or_default();
                    collect_named_type_info(
                        &syntax.items,
                        &base,
                        source.package,
                        source.module,
                        &cfg,
                        source_class,
                        &mut errors,
                        output,
                    );
                }
            }
            package_start = package_end;
        }
        named_type_registries = next;
        if named_type_registries == before {
            break;
        }
    }
    let mut callable_reexports =
        BTreeMap::<(String, PersistenceSourceClass), Vec<(String, String)>>::new();
    let mut public_named_type_paths =
        BTreeMap::<(String, PersistenceSourceClass), BTreeSet<String>>::new();
    let initial_named_type_workspace_cache =
        workspace_named_type_info_cache(&named_type_registries, dependencies);
    let initial_named_type_package_cache = package_named_type_info_cache(&named_type_registries);
    for source in &ordered {
        let Ok(syntax) = syn::parse_file(source.source) else {
            continue;
        };
        let cfg = extend_cfg_context(source.inherited_cfg, &syntax.attrs);
        for source_class in [
            PersistenceSourceClass::Production,
            PersistenceSourceClass::TestFixture,
        ] {
            if !source_class_allows(source_class, &cfg, &[], &mut errors, "source file") {
                continue;
            }
            let key = (source.package.to_owned(), source_class);
            let mut base = ModuleSymbols::for_package(source.package);
            base.named_type_info = initial_named_type_package_cache
                .get(&key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.workspace_named_type_info = initial_named_type_workspace_cache
                .get(&key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.dependency_crate_aliases = dependency_aliases
                .get(&key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            let symbols = collect_module_symbols(
                &syntax.items,
                Some(&base),
                source.package,
                source.module,
                &cfg,
                source_class,
                &mut errors,
            );
            collect_public_callable_reexports(
                &syntax.items,
                &symbols,
                &cfg,
                source_class,
                &mut errors,
                callable_reexports.entry(key.clone()).or_default(),
            );
            collect_public_named_type_paths(
                &syntax.items,
                &symbols,
                &cfg,
                source_class,
                &mut errors,
                public_named_type_paths.entry(key).or_default(),
            );
        }
    }
    resolve_public_named_type_reexports(
        &callable_reexports,
        dependencies,
        &mut named_type_registries,
        &mut public_named_type_paths,
    );
    let named_type_workspace_cache =
        workspace_named_type_info_cache(&named_type_registries, dependencies);
    let named_type_package_cache = package_named_type_info_cache(&named_type_registries);
    let mut trait_registries =
        BTreeMap::<(String, PersistenceSourceClass), TraitSignatureRegistry>::new();
    let mut function_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>::new();
    let mut function_generic_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<String, Vec<String>>>::new();
    let mut function_generic_input_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<String, Vec<GenericInputSpec>>>::new(
        );
    let mut method_registries = BTreeMap::<
        (String, PersistenceSourceClass),
        BTreeMap<(String, String), VariableInfo>,
    >::new();
    let mut method_generic_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<(String, String), Vec<String>>>::new(
        );
    let mut method_generic_input_registries = BTreeMap::<
        (String, PersistenceSourceClass),
        BTreeMap<(String, String), Vec<GenericInputSpec>>,
    >::new();
    let mut item_value_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>::new();
    let mut macro_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<String, TargetSet>>::new();
    // Trait declarations and their consumers may live in different physical
    // files (`mod maker;`). Build a package-wide, cfg-aware signature registry
    // before analyzing any body so source order cannot hide bounded returns.
    // Free functions get the same canonical-path treatment: a qualified call
    // such as `crate::factory::database()` must resolve its return flow no
    // matter which file declares the function.
    for source in &ordered {
        let Ok(syntax) = syn::parse_file(source.source) else {
            continue;
        };
        let cfg = extend_cfg_context(source.inherited_cfg, &syntax.attrs);
        for source_class in [
            PersistenceSourceClass::Production,
            PersistenceSourceClass::TestFixture,
        ] {
            if !source_class_allows(source_class, &cfg, &[], &mut errors, "source file") {
                continue;
            }
            let mut base = ModuleSymbols::for_package(source.package);
            base.named_type_info = named_type_package_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.workspace_named_type_info = named_type_workspace_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.dependency_crate_aliases = dependency_aliases
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            let mut symbols = collect_module_symbols(
                &syntax.items,
                Some(&base),
                source.package,
                source.module,
                &cfg,
                source_class,
                &mut errors,
            );
            collect_nested_trait_returns(
                &syntax.items,
                &symbols.module_path.clone(),
                &cfg,
                source_class,
                &mut symbols,
                &mut errors,
            );
            let function_registry = function_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            let module_prefix = symbols.module_path.join("::");
            let item_value_registry = item_value_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            collect_nested_item_values(
                &syntax.items,
                &symbols.module_path,
                source.package,
                &cfg,
                source_class,
                &symbols,
                item_value_registry,
                &mut errors,
            );
            let macro_registry = macro_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for (name, targets) in &symbols.persistence_macros {
                macro_registry
                    .entry(name.clone())
                    .or_default()
                    .extend(targets.iter().copied());
            }
            for (name, info) in symbols.function_returns.iter() {
                let canonical = if module_prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{module_prefix}::{name}")
                };
                function_registry.entry(canonical).or_default().union(info);
            }
            let function_generic_registry = function_generic_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for (name, generic_params) in symbols.function_generic_params.iter() {
                let canonical = if module_prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{module_prefix}::{name}")
                };
                function_generic_registry
                    .entry(canonical)
                    .or_insert_with(|| generic_params.clone());
            }
            let function_generic_input_registry = function_generic_input_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for (name, input_params) in symbols.function_generic_input_params.iter() {
                let canonical = if module_prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{module_prefix}::{name}")
                };
                function_generic_input_registry
                    .entry(canonical)
                    .or_insert_with(|| input_params.clone());
            }
            let registry = trait_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for (key, info) in symbols.trait_method_returns.iter() {
                registry.0.entry(key.clone()).or_default().union(info);
            }
            for (trait_path, supertraits) in symbols.trait_supertraits.iter() {
                registry
                    .1
                    .entry(trait_path.clone())
                    .or_default()
                    .extend(supertraits.iter().cloned());
            }
            for (trait_path, generic_params) in symbols.trait_generic_params.iter() {
                registry
                    .2
                    .entry(trait_path.clone())
                    .or_insert_with(|| generic_params.clone());
            }
            for (key, generic_params) in symbols.trait_method_generic_params.iter() {
                registry
                    .3
                    .entry(key.clone())
                    .or_insert_with(|| generic_params.clone());
            }
            for (key, input_params) in symbols.trait_method_generic_input_params.iter() {
                registry
                    .4
                    .entry(key.clone())
                    .or_insert_with(|| input_params.clone());
            }
            // Inherent (non-trait) impl methods also get a package-wide
            // registry keyed by canonical owner path, so a call like
            // `factory.make()` resolves when the impl lives in another module.
            let method_registry = method_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for ((owner, trait_name, method), info) in symbols.method_returns.iter() {
                if trait_name.is_some() {
                    continue;
                }
                let canonical_owner = if module_prefix.is_empty() || owner.contains("::") {
                    owner.clone()
                } else {
                    format!("{module_prefix}::{owner}")
                };
                method_registry
                    .entry((canonical_owner, method.clone()))
                    .or_default()
                    .union(info);
            }
            let method_generic_registry = method_generic_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for ((owner, trait_name, method), generic_params) in
                symbols.method_generic_params.iter()
            {
                if trait_name.is_some() {
                    continue;
                }
                let canonical_owner = if module_prefix.is_empty() || owner.contains("::") {
                    owner.clone()
                } else {
                    format!("{module_prefix}::{owner}")
                };
                method_generic_registry
                    .entry((canonical_owner, method.clone()))
                    .or_insert_with(|| generic_params.clone());
            }
            let method_generic_input_registry = method_generic_input_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for ((owner, trait_name, method), input_params) in
                symbols.method_generic_input_params.iter()
            {
                if trait_name.is_some() {
                    continue;
                }
                let canonical_owner = if module_prefix.is_empty() || owner.contains("::") {
                    owner.clone()
                } else {
                    format!("{module_prefix}::{owner}")
                };
                method_generic_input_registry
                    .entry((canonical_owner, method.clone()))
                    .or_insert_with(|| input_params.clone());
            }
        }
    }
    resolve_public_callable_reexports(
        &callable_reexports,
        &named_type_registries,
        dependencies,
        &mut function_registries,
        &mut function_generic_registries,
        &mut function_generic_input_registries,
    );
    let mut item_value_generic_registries = BTreeMap::new();
    let mut item_value_generic_input_registries = BTreeMap::new();
    resolve_public_callable_reexports(
        &callable_reexports,
        &named_type_registries,
        dependencies,
        &mut item_value_registries,
        &mut item_value_generic_registries,
        &mut item_value_generic_input_registries,
    );
    resolve_public_macro_reexports(&callable_reexports, dependencies, &mut macro_registries);
    resolve_public_trait_reexports(
        &callable_reexports,
        &named_type_registries,
        dependencies,
        &mut trait_registries,
    );
    let trait_method_registries = trait_registries
        .iter()
        .map(|(key, registry)| (key.clone(), registry.0.clone()))
        .collect::<BTreeMap<_, _>>();
    let trait_supertrait_registries = trait_registries
        .iter()
        .map(|(key, registry)| (key.clone(), registry.1.clone()))
        .collect::<BTreeMap<_, _>>();
    let trait_generic_registries = trait_registries
        .iter()
        .map(|(key, registry)| (key.clone(), registry.2.clone()))
        .collect::<BTreeMap<_, _>>();
    let trait_method_generic_registries = trait_registries
        .iter()
        .map(|(key, registry)| (key.clone(), registry.3.clone()))
        .collect::<BTreeMap<_, _>>();
    let trait_method_generic_input_registries = trait_registries
        .iter()
        .map(|(key, registry)| (key.clone(), registry.4.clone()))
        .collect::<BTreeMap<_, _>>();
    let trait_names = trait_registries
        .iter()
        .map(|(key, registry)| {
            let mut names = registry
                .0
                .keys()
                .map(|(trait_path, _)| trait_path.clone())
                .collect::<BTreeSet<_>>();
            names.extend(registry.1.keys().cloned());
            names.extend(registry.2.keys().cloned());
            names.extend(registry.3.keys().map(|(trait_path, _)| trait_path.clone()));
            names.extend(registry.4.keys().map(|(trait_path, _)| trait_path.clone()));
            (key.clone(), names)
        })
        .collect::<BTreeMap<_, _>>();
    let trait_method_registry_cache = dependency_scoped_registry_cache(
        &trait_method_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (trait_path, method)| {
            (format!("{provider_root}::{trait_path}"), method.clone())
        },
        qualify_dependency_info,
    );
    let trait_supertrait_registry_cache = dependency_scoped_trait_supertrait_cache(
        &trait_supertrait_registries,
        &trait_names,
        dependencies,
    );
    let trait_generic_registry_cache = dependency_scoped_registry_cache(
        &trait_generic_registries,
        &named_type_registries,
        dependencies,
        |provider_root, trait_path| format!("{provider_root}::{trait_path}"),
        |_, _, value| value.clone(),
    );
    let trait_method_generic_registry_cache = dependency_scoped_registry_cache(
        &trait_method_generic_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (trait_path, method)| {
            (format!("{provider_root}::{trait_path}"), method.clone())
        },
        |_, _, value| value.clone(),
    );
    let trait_method_generic_input_registry_cache = dependency_scoped_registry_cache(
        &trait_method_generic_input_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (trait_path, method)| {
            (format!("{provider_root}::{trait_path}"), method.clone())
        },
        |_, _, value| value.clone(),
    );
    // Trait declarations and implementations can live in different files.
    // Once the first pass has the complete trait-return registry, revisit
    // impls so associated bindings can instantiate inherited default method
    // returns and publish them under the concrete receiver package-wide.
    for source in &ordered {
        let Ok(syntax) = syn::parse_file(source.source) else {
            continue;
        };
        let cfg = extend_cfg_context(source.inherited_cfg, &syntax.attrs);
        for source_class in [
            PersistenceSourceClass::Production,
            PersistenceSourceClass::TestFixture,
        ] {
            if !source_class_allows(source_class, &cfg, &[], &mut errors, "source file") {
                continue;
            }
            let mut base = ModuleSymbols::for_package(source.package);
            base.named_type_info = named_type_package_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.workspace_named_type_info = named_type_workspace_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.dependency_crate_aliases = dependency_aliases
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            let registry_key = (source.package.to_owned(), source_class);
            base.trait_method_returns = trait_method_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.trait_supertraits = trait_supertrait_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.trait_generic_params = trait_generic_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.trait_method_generic_params = trait_method_generic_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.trait_method_generic_input_params = trait_method_generic_input_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            let symbols = collect_module_symbols(
                &syntax.items,
                Some(&base),
                source.package,
                source.module,
                &cfg,
                source_class,
                &mut errors,
            );
            let module_prefix = symbols.module_path.join("::");
            let method_registry = method_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for ((owner, _, method), info) in &symbols.method_returns {
                let canonical_owner = if module_prefix.is_empty() || owner.contains("::") {
                    owner.clone()
                } else {
                    format!("{module_prefix}::{owner}")
                };
                method_registry
                    .entry((canonical_owner, method.clone()))
                    .or_default()
                    .union(info);
            }
        }
    }
    let function_registry_cache = dependency_scoped_registry_cache(
        &function_registries,
        &named_type_registries,
        dependencies,
        |provider_root, key| format!("{provider_root}::{key}"),
        qualify_dependency_info,
    );
    let function_generic_registry_cache = dependency_scoped_registry_cache(
        &function_generic_registries,
        &named_type_registries,
        dependencies,
        |provider_root, key| format!("{provider_root}::{key}"),
        |_, _, value| value.clone(),
    );
    let function_generic_input_registry_cache = dependency_scoped_registry_cache(
        &function_generic_input_registries,
        &named_type_registries,
        dependencies,
        |provider_root, key| format!("{provider_root}::{key}"),
        |_, _, value| value.clone(),
    );
    let method_registry_cache = dependency_scoped_registry_cache(
        &method_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (owner, method)| (format!("{provider_root}::{owner}"), method.clone()),
        qualify_dependency_info,
    );
    let method_generic_registry_cache = dependency_scoped_registry_cache(
        &method_generic_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (owner, method)| (format!("{provider_root}::{owner}"), method.clone()),
        |_, _, value| value.clone(),
    );
    let method_generic_input_registry_cache = dependency_scoped_registry_cache(
        &method_generic_input_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (owner, method)| (format!("{provider_root}::{owner}"), method.clone()),
        |_, _, value| value.clone(),
    );
    let item_value_registry_cache = dependency_scoped_registry_cache(
        &item_value_registries,
        &named_type_registries,
        dependencies,
        |provider_root, key| format!("{provider_root}::{key}"),
        qualify_dependency_info,
    );
    let macro_registry_cache = dependency_scoped_registry_cache(
        &macro_registries,
        &named_type_registries,
        dependencies,
        |provider_root, key| format!("{provider_root}::{key}"),
        |_, _, value| value.clone(),
    );
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
            let mut package_symbols = ModuleSymbols::for_package(source.package);
            package_symbols.named_type_info = named_type_package_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.workspace_named_type_info = named_type_workspace_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.dependency_crate_aliases = dependency_aliases
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            let registry_key = (source.package.to_owned(), source_class);
            package_symbols.trait_method_returns = trait_method_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.trait_supertraits = trait_supertrait_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.trait_generic_params = trait_generic_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.trait_method_generic_params = trait_method_generic_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.trait_method_generic_input_params =
                trait_method_generic_input_registry_cache
                    .get(&registry_key)
                    .cloned()
                    .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_function_returns = function_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_function_generic_params = function_generic_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_function_generic_input_params =
                function_generic_input_registry_cache
                    .get(&registry_key)
                    .cloned()
                    .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_method_returns = method_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_method_generic_params = method_generic_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_method_generic_input_params =
                method_generic_input_registry_cache
                    .get(&registry_key)
                    .cloned()
                    .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_item_values = item_value_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_persistence_macros = macro_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            analyze_module_items(
                &syntax.items,
                RecordContext {
                    classification: source.classification,
                    source_class,
                    package: source.package,
                    module: source.module,
                    source: source.source_path,
                },
                Some(&package_symbols),
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
    fn persistence_inventory_resolves_trait_signatures_across_source_files() {
        let dto = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::dto",
            source_path: "src/dto.rs",
            inherited_cfg: &[],
            source: r#"
                pub type Db = wow_database::CharacterDatabase;
                pub struct Holder(pub Db);
                pub struct Plain(pub u8);
            "#,
        };
        let declaration = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::maker",
            source_path: "src/maker.rs",
            inherited_cfg: &[],
            source: r#"
                use crate::dto::{Holder, Plain};
                pub trait Maker { fn make(&self) -> Holder; }
                pub trait PlainMaker { fn make(&self) -> Plain; }
            "#,
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::consumer",
            source_path: "src/consumer.rs",
            inherited_cfg: &[],
            source: r#"
                fn cross_file_persistent(factory: &dyn crate::maker::Maker) {
                    consume(factory.make().0.pool());
                }
                fn cross_file_clean(factory: &dyn crate::maker::PlainMaker) {
                    consume(factory.make().0);
                }
            "#,
        };

        let baseline = inventory_persistence_accesses(&[consumer, declaration, dto]).unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn cross_file_persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn cross_file_clean")
        );
    }

    #[test]
    fn persistence_inventory_unions_cfg_alternative_trait_signatures() {
        let baseline = inventory(
            r#"
                struct Holder(wow_database::CharacterDatabase);

                #[cfg(feature = "database")]
                trait Maker { fn make(&self) -> Holder; }

                #[cfg(not(feature = "database"))]
                trait Maker { fn make(&self) -> u8; }

                #[cfg(feature = "database")]
                fn persistent(factory: &dyn Maker) {
                    consume(factory.make().0.pool());
                }
            "#,
        )
        .unwrap();

        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_unions_cfg_alternative_function_signatures() {
        let baseline = inventory(
            r#"
                struct Holder(wow_database::CharacterDatabase);

                #[cfg(feature = "database")]
                fn make() -> Holder { todo!() }

                #[cfg(not(feature = "database"))]
                fn make() -> u8 { 0 }

                #[cfg(feature = "database")]
                fn persistent() {
                    consume(make().0.pool());
                }
            "#,
        )
        .unwrap();

        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
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
                struct MixedFields {
                    database: wow_database::CharacterDatabase,
                    clean: bool,
                }
                fn declared_clean_field(value: MixedFields) {
                    consume(value.clean);
                }
                fn declared_persistent_field(value: MixedFields) {
                    consume(value.database.pool());
                }
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
                    pub trait Chained: Maker + Sized { fn again(&self) -> Option<Self>; }
                }
                mod plain_trait {
                    pub trait Maker { fn make(&self) -> u8; }
                    pub trait Derived: Maker {}
                    pub trait Chained: Maker + Sized { fn again(&self) -> Option<Self>; }
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
                fn self_return_persistent<T: db_trait::Chained>(factory: &T) {
                    let Some(next) = factory.again() else { return };
                    consume(next.make().0.pool());
                }
                fn self_return_clean<T: plain_trait::Chained>(factory: &T) {
                    let Some(next) = factory.again() else { return };
                    consume(next.make());
                }
                fn ufcs_self_return_persistent<T: db_trait::Chained>(factory: &T) {
                    let Some(next) = T::again(factory) else { return };
                    consume(next.make().0.pool());
                }
                fn persistent_provider() -> impl db_trait::Maker { unreachable!() }
                fn plain_provider() -> impl plain_trait::Maker { unreachable!() }
                fn impl_trait_function_persistent() {
                    consume(persistent_provider().make().0.pool());
                }
                fn impl_trait_function_clean() {
                    consume(plain_provider().make());
                }
                struct Provider;
                impl Provider {
                    fn persistent(&self) -> impl db_trait::Maker { unreachable!() }
                    fn plain(&self) -> impl plain_trait::Maker { unreachable!() }
                }
                fn impl_trait_method_persistent(provider: &Provider) {
                    consume(provider.persistent().make().0.pool());
                }
                fn impl_trait_method_clean(provider: &Provider) {
                    consume(provider.plain().make());
                }
                mod namespace_self_scope {
                    use crate::db_trait::{self};
                    fn namespace_self_persistent(factory: &dyn db_trait::Maker) {
                        consume(factory.make().0.pool());
                    }
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
                    | "fn declared_clean_field"
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
                    | "fn self_return_clean"
                    | "fn impl_trait_function_clean"
                    | "fn impl_trait_method_clean"
            )),
            "named fields on unrelated owner types must not contaminate receiver identity: {:#?}",
            field_owner_collision.accesses
        );
        assert!(field_owner_collision.accesses.iter().any(|row| {
            row.enclosing == "fn declared_persistent_field"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
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
            "fn self_return_persistent",
            "fn ufcs_self_return_persistent",
            "fn impl_trait_function_persistent",
            "fn impl_trait_method_persistent",
            "fn namespace_self_persistent",
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
    fn persistence_inventory_records_rejected_named_persistence_receiver_as_escape() {
        let baseline = inventory(
            r#"
                struct Holder(wow_database::CharacterDatabase);
                impl Holder { fn commit(&self) {} }
                fn persistent(holder: Holder) { holder.commit(); }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::ArgumentEscape
                && row.symbol == "receiver:commit"
        }));
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::Commit
        }));
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

    #[test]
    fn persistence_inventory_tracks_transforming_combinator_arguments() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    consume(Some(0_u8).map(|_| database).unwrap().pool());
                }
                fn clean() {
                    consume(Some(0_u8).map(|_| 1_u8).unwrap());
                }
            "#,
        )
        .unwrap();

        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean")
        );
    }

    #[test]
    fn persistence_inventory_resolves_local_closure_callables() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let factory = || database;
                    consume(factory().pool());
                }
            "#,
        )
        .unwrap();

        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_fails_closed_on_block_local_items() {
        let error = inventory(
            r#"
                fn leak(pool: Alias) {
                    type Alias = sqlx::MySqlPool;
                    pool.acquire();
                }
            "#,
        )
        .expect_err("block-local persistence alias must fail closed");
        assert!(
            error.contains("block-local item"),
            "unexpected error: {error}"
        );

        inventory(
            r#"
                fn clean() {
                    struct Local(u8);
                    let _ = Local(0);
                }
            "#,
        )
        .expect("block-local items without persistence stay allowed");
    }

    #[test]
    fn persistence_inventory_registers_enum_variant_payloads_across_source_files() {
        let dto = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::dto",
            source_path: "src/dto.rs",
            inherited_cfg: &[],
            source: r#"
                pub type Db = wow_database::CharacterDatabase;
                pub enum Product { Database(Db), Plain(u8) }
            "#,
        };
        let declaration = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::maker",
            source_path: "src/maker.rs",
            inherited_cfg: &[],
            source: r#"
                pub trait Maker { fn make(&self) -> crate::dto::Product; }
            "#,
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::consumer",
            source_path: "src/consumer.rs",
            inherited_cfg: &[],
            source: r#"
                fn cross_file_persistent(factory: &dyn crate::maker::Maker) {
                    match factory.make() {
                        crate::dto::Product::Database(database) => consume(database.pool()),
                        crate::dto::Product::Plain(_) => {}
                    }
                }
            "#,
        };

        let baseline = inventory_persistence_accesses(&[consumer, declaration, dto]).unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn cross_file_persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_unions_match_arm_assignments() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase, pick: u8) {
                    let mut value = None;
                    match pick {
                        0 => value = Some(database),
                        _ => value = None,
                    }
                    if let Some(database) = value {
                        consume(database.pool());
                    }
                }
            "#,
        )
        .unwrap();

        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_carries_failed_match_guard_mutations_to_later_arms() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    match true {
                        true if { slot = Some(database); false } => {}
                        _ => consume(slot.unwrap().pool()),
                    }
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_analyzes_default_trait_method_bodies() {
        let dto = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::dto",
            source_path: "src/dto.rs",
            inherited_cfg: &[],
            source: r#"
                pub type Db = wow_database::CharacterDatabase;
                pub struct Holder(pub Db);
            "#,
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::consumer",
            source_path: "src/consumer.rs",
            inherited_cfg: &[],
            source: r#"
                use crate::dto::Holder;
                trait Access {
                    fn holder(&self) -> Holder;
                    fn leak(&self) {
                        consume(self.holder().0.pool());
                    }
                }
            "#,
        };

        let baseline = inventory_persistence_accesses(&[consumer, dto]).unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "trait Access::leak"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_resolves_free_functions_across_source_files() {
        let factory = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::factory",
            source_path: "src/factory.rs",
            inherited_cfg: &[],
            source: r#"
                pub fn database() -> wow_database::CharacterDatabase {
                    unimplemented!()
                }
            "#,
        };
        let qualified = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::qualified",
            source_path: "src/qualified.rs",
            inherited_cfg: &[],
            source: r#"
                fn persistent_qualified() {
                    consume(crate::factory::database().pool());
                }
            "#,
        };
        let imported = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::imported",
            source_path: "src/imported.rs",
            inherited_cfg: &[],
            source: r#"
                use crate::factory::database;
                fn persistent_imported() {
                    consume(database().pool());
                }
            "#,
        };

        let baseline = inventory_persistence_accesses(&[qualified, imported, factory]).unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent_qualified"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent_imported"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_substitutes_generic_bound_type_arguments() {
        let dto = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::dto",
            source_path: "src/dto.rs",
            inherited_cfg: &[],
            source: r#"
                pub type Db = wow_database::CharacterDatabase;
                pub struct Holder(pub Db);
            "#,
        };
        let declaration = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::maker",
            source_path: "src/maker.rs",
            inherited_cfg: &[],
            source: r#"
                pub trait Maker<T> { fn make(&self) -> T; }
            "#,
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::consumer",
            source_path: "src/consumer.rs",
            inherited_cfg: &[],
            source: r#"
                use crate::dto::Holder;
                fn cross_file_persistent<M: crate::maker::Maker<Holder>>(factory: &M) {
                    consume(factory.make().0.pool());
                }
                fn cross_file_where<M>(factory: &M)
                where
                    M: crate::maker::Maker<Holder>,
                {
                    consume(factory.make().0.pool());
                }
            "#,
        };

        let baseline = inventory_persistence_accesses(&[consumer, declaration, dto]).unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn cross_file_persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn cross_file_where"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_unions_if_else_branch_assignments() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase, pick: bool) {
                    let mut value = None;
                    if pick { value = Some(database); } else { value = None; }
                    if let Some(database) = value {
                        consume(database.pool());
                    }
                }
                fn persistent_no_else(database: wow_database::CharacterDatabase, pick: bool) {
                    let mut value = None;
                    if pick { value = Some(database); }
                    if let Some(database) = value {
                        consume(database.pool());
                    }
                }
                fn clean(pick: bool) {
                    let mut value = None;
                    if pick { value = Some(1_u8); } else { value = None; }
                    if let Some(value) = value {
                        consume(value);
                    }
                }
            "#,
        )
        .unwrap();

        for enclosing in ["fn persistent", "fn persistent_no_else"] {
            assert!(
                baseline.accesses.iter().any(|row| {
                    row.enclosing == enclosing
                        && row.target == PersistenceTarget::CharacterDatabase
                        && row.operation == PersistenceOperation::PoolAccess
                }),
                "missing pool-access row for {enclosing}"
            );
        }
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean")
        );
    }

    #[test]
    fn persistence_inventory_retains_pre_loop_flow_for_zero_iteration_paths() {
        let baseline = inventory(
            r#"
                fn persistent_for(database: wow_database::CharacterDatabase, items: Vec<u8>) {
                    let mut value = Some(database);
                    for _ in items { value = None; }
                    if let Some(database) = value {
                        consume(database.pool());
                    }
                }
                fn persistent_while(database: wow_database::CharacterDatabase, running: bool) {
                    let mut value = Some(database);
                    while running { value = None; }
                    if let Some(database) = value {
                        consume(database.pool());
                    }
                }
            "#,
        )
        .unwrap();

        for enclosing in ["fn persistent_for", "fn persistent_while"] {
            assert!(
                baseline.accesses.iter().any(|row| {
                    row.enclosing == enclosing
                        && row.target == PersistenceTarget::CharacterDatabase
                        && row.operation == PersistenceOperation::PoolAccess
                }),
                "missing pool-access row for {enclosing}"
            );
        }
    }

    #[test]
    fn persistence_inventory_propagates_vec_macro_result_flow() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let values = vec![database];
                    consume(values[0].pool());
                }
                fn clean() {
                    let values = vec![1_u8];
                    consume(values[0]);
                }
            "#,
        )
        .unwrap();

        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean"
                    && row.operation == PersistenceOperation::PoolAccess)
        );
    }

    #[test]
    fn persistence_inventory_substitutes_turbofish_into_recorded_returns() {
        let baseline = inventory(
            r#"
                struct Factory;
                impl Factory { fn make<T>(&self) -> T { unreachable!() } }
                fn make_selected<T>() -> T { unreachable!() }
                pub struct External;
                fn persistent_method() {
                    consume(Factory.make::<wow_database::CharacterDatabase>().pool());
                }
                fn persistent_free() {
                    consume(make_selected::<wow_database::WorldDatabase>().pool());
                }
                fn persistent_unknown(external: &External) {
                    consume(external.make::<wow_database::LoginDatabase>().pool());
                }
                fn clean() {
                    consume(Factory.make::<u8>());
                }
            "#,
        )
        .unwrap();

        let pool_row = |enclosing: &str, target: PersistenceTarget| {
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing
                    && row.target == target
                    && row.operation == PersistenceOperation::PoolAccess
            })
        };
        assert!(pool_row(
            "fn persistent_method",
            PersistenceTarget::CharacterDatabase
        ));
        assert!(pool_row(
            "fn persistent_free",
            PersistenceTarget::WorldDatabase
        ));
        assert!(pool_row(
            "fn persistent_unknown",
            PersistenceTarget::LoginDatabase
        ));
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean")
        );
    }

    #[test]
    fn persistence_inventory_resolves_database_paths_in_opaque_macro_tokens() {
        let baseline = inventory(
            r#"
                fn persistent() {
                    assert!(wow_database::CharacterDatabase::open("dsn").is_ok());
                }
                fn clean() {
                    assert!(true);
                }
            "#,
        )
        .unwrap();

        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::MacroReference
        }));
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean")
        );
    }

    #[test]
    fn persistence_inventory_matches_advisory_lock_sql_case_insensitively() {
        let baseline = inventory(
            r#"
                fn persistent() {
                    sqlx::query("select get_lock('k', 0)");
                }
                fn clean() {
                    sqlx::query("select 1");
                }
            "#,
        )
        .unwrap();

        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::Sqlx
                && row.operation == PersistenceOperation::AdvisoryLock
        }));
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn clean" && row.operation == PersistenceOperation::AdvisoryLock
        }));
    }

    #[test]
    fn persistence_inventory_resolves_inherent_methods_across_source_files() {
        let dto = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::dto",
            source_path: "src/dto.rs",
            inherited_cfg: &[],
            source: r#"
                pub type Db = wow_database::CharacterDatabase;
                pub struct Factory;
                impl Factory { pub fn make(&self) -> Db { unreachable!() } }
            "#,
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::consumer",
            source_path: "src/consumer.rs",
            inherited_cfg: &[],
            source: r#"
                use crate::dto::Factory;
                fn persistent(factory: &Factory) {
                    consume(factory.make().pool());
                }
                fn persistent_qualified(factory: &Factory) {
                    consume(crate::dto::Factory::make(factory).pool());
                }
            "#,
        };

        let baseline = inventory_persistence_accesses(&[consumer, dto]).unwrap();
        for enclosing in ["fn persistent", "fn persistent_qualified"] {
            assert!(
                baseline.accesses.iter().any(|row| {
                    row.enclosing == enclosing
                        && row.target == PersistenceTarget::CharacterDatabase
                        && row.operation == PersistenceOperation::PoolAccess
                }),
                "missing pool-access row for {enclosing}"
            );
        }
    }

    #[test]
    fn persistence_inventory_preserves_receiver_flow_through_unmodeled_methods() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let wrapped = Some(database).iter().next();
                    consume(wrapped.unwrap().pool());
                }
                fn clean() {
                    let wrapped = Some(1_u8).iter().next();
                    consume(wrapped.unwrap());
                }
            "#,
        )
        .unwrap();

        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean")
        );
    }

    #[test]
    fn persistence_inventory_inventories_every_registered_macro_invocation() {
        let baseline = inventory(
            r#"
                macro_rules! hidden_query { () => { sqlx::query("SELECT 1") } }
                fn persistent() {
                    consume(hidden_query!());
                }
                fn clean() {
                    consume(1_u8);
                }
            "#,
        )
        .unwrap();

        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::Sqlx
                && row.operation == PersistenceOperation::MacroReference
                && row.symbol == "hidden_query"
        }));
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean")
        );
    }

    #[test]
    fn persistence_inventory_inventories_registered_macros_across_source_files() {
        let definitions = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::macros",
            source_path: "src/macros.rs",
            inherited_cfg: &[],
            source: r#"
                macro_rules! hidden_query { () => { sqlx::query("SELECT 1") } }
            "#,
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::consumer",
            source_path: "src/consumer.rs",
            inherited_cfg: &[],
            source: r#"
                fn persistent() { consume(hidden_query!()); }
                fn clean() { consume(1_u8); }
            "#,
        };

        let baseline = inventory_persistence_accesses(&[consumer, definitions]).unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::Sqlx
                && row.operation == PersistenceOperation::MacroReference
                && row.symbol == "hidden_query"
        }));
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean")
        );
    }

    #[test]
    fn persistence_inventory_substitutes_impl_associated_type_returns() {
        let baseline = inventory(
            r#"
                struct Holder(wow_database::CharacterDatabase);
                struct PlainHolder(u8);
                trait Maker {
                    type Product;
                    fn make(&self) -> Self::Product;
                }
                struct Factory;
                impl Maker for Factory {
                    type Product = Holder;
                    fn make(&self) -> Self::Product { unreachable!() }
                }
                struct PlainFactory;
                impl Maker for PlainFactory {
                    type Product = PlainHolder;
                    fn make(&self) -> Self::Product { unreachable!() }
                }
                fn persistent(factory: &Factory) { consume(factory.make().0.pool()); }
                fn clean(factory: &PlainFactory) { consume(factory.make().0); }
            "#,
        )
        .unwrap();

        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean")
        );
    }

    #[test]
    fn persistence_inventory_records_nominal_container_argument_escapes() {
        let baseline = inventory(
            r#"
                struct Holder(wow_database::CharacterDatabase);
                struct PlainHolder(u8);
                fn persistent(database: wow_database::CharacterDatabase) {
                    let holder = Holder(database);
                    send(holder);
                }
                fn clean() { send(PlainHolder(1_u8)); }
            "#,
        )
        .unwrap();

        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::ArgumentEscape
                && row.symbol == "send"
        }));
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean")
        );
    }

    #[test]
    fn persistence_inventory_propagates_arbitrary_callable_result_flow() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let factory = || database;
                    consume((factory)().pool());
                }
                fn clean() {
                    let factory = || 1_u8;
                    consume((factory)());
                }
            "#,
        )
        .unwrap();

        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean")
        );
    }

    #[test]
    fn persistence_inventory_tracks_module_values_across_source_files() {
        let values = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::values",
            source_path: "src/values.rs",
            inherited_cfg: &[],
            source: r#"
                static POOL: std::sync::OnceLock<sqlx::MySqlPool> = std::sync::OnceLock::new();
                const DATABASE: wow_database::CharacterDatabase = unreachable!();
                static CLEAN: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
            "#,
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::consumer",
            source_path: "src/consumer.rs",
            inherited_cfg: &[],
            source: r#"
                fn persistent_pool() { consume(crate::values::POOL.get().unwrap().acquire()); }
                fn persistent_database() { consume(crate::values::DATABASE.pool()); }
                fn clean() { consume(crate::values::CLEAN.get()); }
            "#,
        };

        let baseline = inventory_persistence_accesses(&[consumer, values]).unwrap();
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == "fn persistent_pool"
                    && row.target == PersistenceTarget::MySqlPool
                    && row.operation == PersistenceOperation::PoolAccess
            }),
            "{:#?}",
            baseline.accesses
        );
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent_database"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean")
        );
    }

    #[test]
    fn persistence_inventory_applies_associated_bindings_to_inherited_default_methods() {
        let baseline = inventory(
            r#"
                struct Holder(wow_database::CharacterDatabase);
                trait Maker {
                    type Product;
                    fn make(&self) -> Self::Product { unreachable!() }
                }
                struct Factory;
                impl Maker for Factory { type Product = Holder; }
                fn persistent(factory: &Factory) { consume(factory.make().0.pool()); }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_tracks_inline_module_values_across_source_files() {
        let values = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::values",
            source_path: "src/values.rs",
            inherited_cfg: &[],
            source: r#"
                mod nested {
                    static POOL: std::sync::OnceLock<sqlx::MySqlPool> = std::sync::OnceLock::new();
                }
            "#,
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::consumer",
            source_path: "src/consumer.rs",
            inherited_cfg: &[],
            source: r#"
                fn persistent() {
                    consume(crate::values::nested::POOL.get().unwrap().acquire());
                }
            "#,
        };
        let baseline = inventory_persistence_accesses(&[consumer, values]).unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::MySqlPool
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_propagates_arguments_through_callable_results() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let identity = |value| value;
                    consume(identity(database).pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_preserves_state_reachable_through_loop_breaks() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase, stop: bool) {
                    let mut value = Some(database);
                    loop {
                        if stop { break; }
                        value = None;
                    }
                    consume(value.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_preserves_mutations_at_loop_break_exits() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase, stop: bool) {
                    let mut slot = None;
                    loop {
                        slot = Some(&database);
                        if stop { break; }
                        slot = None;
                    }
                    consume(slot.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_preserves_continue_states_as_loop_back_edges() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase, repeat: bool) {
                    let mut slot = None;
                    loop {
                        if let Some(db) = slot { consume(db.pool()); break; }
                        slot = Some(&database);
                        if repeat { continue; }
                        slot = None;
                    }
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_preserves_for_and_while_break_exits() {
        let baseline = inventory(
            r#"
                fn in_while(database: wow_database::CharacterDatabase, running: bool, stop: bool) {
                    let mut slot = None;
                    while running {
                        slot = Some(&database);
                        if stop { break; }
                        slot = None;
                    }
                    if let Some(db) = slot { consume(db.pool()); }
                }
                fn in_for(database: wow_database::CharacterDatabase, values: Vec<u8>, stop: bool) {
                    let mut slot = None;
                    for _ in values {
                        slot = Some(&database);
                        if stop { break; }
                        slot = None;
                    }
                    if let Some(db) = slot { consume(db.pool()); }
                }
            "#,
        )
        .unwrap();
        for enclosing in ["fn in_while", "fn in_for"] {
            assert!(baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
            }));
        }
    }

    #[test]
    fn persistence_inventory_routes_labeled_breaks_to_their_target_loop() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase, clear: bool) {
                    let mut slot = None;
                    'outer: loop {
                        loop {
                            slot = Some(&database);
                            break 'outer;
                        }
                        if clear { slot = None; }
                    }
                    consume(slot.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_preserves_false_while_condition_mutations() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    while { slot = Some(&database); false } { slot = None; }
                    consume(slot.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_preserves_join_macro_result_flow() {
        let baseline = inventory(
            r#"
                async fn persistent(database: wow_database::CharacterDatabase) {
                    let (database,) = tokio::join!(async { database });
                    consume(database.pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_instantiates_generic_container_flow() {
        let baseline = inventory(
            r#"
                struct Holder<T>(T);
                fn persistent(database: wow_database::CharacterDatabase) {
                    let holder = Holder(database);
                    send(holder);
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::ArgumentEscape
                && row.symbol == "send"
        }));
    }

    #[test]
    fn persistence_inventory_updates_destructuring_assignment_bindings() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut value = None;
                    (value,) = (Some(database),);
                    consume(value.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_updates_supported_places_in_mixed_assignments() {
        let baseline = inventory(
            r#"
                struct Holder { field: u8 }
                fn persistent(
                    database: wow_database::CharacterDatabase,
                    mut holder: Holder,
                ) {
                    let mut value = None;
                    (holder.field, value) = (1_u8, Some(database));
                    consume(value.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_resolves_inline_module_value_aliases() {
        let baseline = inventory(
            r#"
                mod nested {
                    type Db = wow_database::CharacterDatabase;
                    static DATABASE: Db = unreachable!();
                    fn persistent() { consume(DATABASE.pool()); }
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_substitutes_generic_trait_arguments_in_default_returns() {
        let baseline = inventory(
            r#"
                struct Holder(wow_database::CharacterDatabase);
                trait Maker<T> { fn make(&self) -> T { unreachable!() } }
                struct Factory;
                impl Maker<Holder> for Factory {}
                fn persistent(factory: &Factory) { consume(factory.make().0.pool()); }
            "#,
        )
        .unwrap();
        assert!(
            baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn persistent"
                    && row.operation == PersistenceOperation::PoolAccess)
        );
    }

    #[test]
    fn persistence_inventory_propagates_inferred_generic_function_arguments() {
        let baseline = inventory(
            r#"
                fn identity<T>(value: T) -> T { value }
                fn persistent(database: wow_database::CharacterDatabase) {
                    consume(identity(database).pool());
                }
            "#,
        )
        .unwrap();
        assert!(
            baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn persistent"
                    && row.operation == PersistenceOperation::PoolAccess)
        );
    }

    #[test]
    fn persistence_inventory_maps_inferred_generics_to_their_formal_inputs() {
        let baseline = inventory(
            r#"
                fn first<T, U>(first: T, _second: U) -> T { first }
                fn clean(database: wow_database::CharacterDatabase) {
                    consume(first(1_u8, database).pool());
                }
            "#,
        )
        .unwrap();
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean"
                    && row.operation == PersistenceOperation::PoolAccess)
        );
    }

    #[test]
    fn persistence_inventory_propagates_inferred_generic_method_arguments() {
        let baseline = inventory(
            r#"
                struct Factory;
                impl Factory { fn identity<T>(&self, value: T) -> T { value } }
                fn persistent(factory: &Factory, database: wow_database::CharacterDatabase) {
                    consume(factory.identity(database).pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_aligns_inferred_generic_arguments_for_ufcs_methods() {
        let baseline = inventory(
            r#"
                struct Factory;
                impl Factory { fn identity<T>(&self, value: T) -> T { value } }
                fn persistent(factory: &Factory, database: wow_database::CharacterDatabase) {
                    consume(Factory::identity(factory, database).pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_does_not_infer_ufcs_return_from_receiver() {
        let baseline = inventory(
            r#"
                struct Factory(wow_database::CharacterDatabase);
                impl Factory { fn identity<T>(&self, value: T) -> T { value } }
                fn clean(factory: &Factory) {
                    consume(Factory::identity(factory, 1_u8).pool());
                }
            "#,
        )
        .unwrap();
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_propagates_qualified_generic_function_arguments() {
        let factory = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::factory",
            source_path: "src/factory.rs",
            inherited_cfg: &[],
            source: "fn identity<T>(value: T) -> T { value }",
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::consumer",
            source_path: "src/consumer.rs",
            inherited_cfg: &[],
            source: r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    consume(crate::factory::identity(database).pool());
                }
            "#,
        };
        let baseline = inventory_persistence_accesses(&[consumer, factory]).unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_resolves_named_type_fields_across_packages() {
        let provider = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-a",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: "pub struct Holder(pub wow_database::CharacterDatabase);",
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "consumer-b",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                fn persistent(holder: provider_alias::Holder) {
                    consume(holder.0.pool());
                }
            "#,
        };
        let dependencies = WorkspaceDependencyAliases {
            production: BTreeMap::from([(
                "consumer-b".to_owned(),
                BTreeMap::from([("provider_alias".to_owned(), "provider_a".to_owned())]),
            )]),
            test: BTreeMap::new(),
        };
        let baseline =
            inventory_persistence_accesses_with_dependencies(&[consumer, provider], &dependencies)
                .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_does_not_resolve_unrelated_workspace_named_types() {
        let provider = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "unrelated-provider",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: "pub struct Holder(pub wow_database::CharacterDatabase);",
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "consumer-b",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                struct Holder(u8);
                fn clean(holder: Holder) {
                    consume(holder.0.pool());
                }
            "#,
        };
        let baseline = inventory_persistence_accesses_with_dependencies(
            &[consumer, provider],
            &WorkspaceDependencyAliases::default(),
        )
        .unwrap();
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_keeps_clean_dependency_field_projections_clean() {
        let provider = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-a",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                pub struct Inner {
                    pub database: wow_database::CharacterDatabase,
                    pub clean: bool,
                }
            "#,
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "consumer-b",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                struct Outer { inner: provider_a::Inner }
                fn clean(value: Outer) {
                    consume(value.inner.clean);
                    assert!(!value.inner.clean);
                }
                fn persistent(value: Outer) { consume(value.inner.database.pool()); }
            "#,
        };
        let aliases = BTreeMap::from([("provider_a".to_owned(), "provider_a".to_owned())]);
        let dependencies = WorkspaceDependencyAliases {
            production: BTreeMap::from([("consumer-b".to_owned(), aliases.clone())]),
            test: BTreeMap::from([("consumer-b".to_owned(), aliases)]),
        };
        let baseline =
            inventory_persistence_accesses_with_dependencies(&[consumer, provider], &dependencies)
                .unwrap();
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean"),
            "clean dependency field projection was tainted: {:#?}",
            baseline.accesses
        );
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_propagates_inferred_generic_trait_method_arguments() {
        let baseline = inventory(
            r#"
                trait Identity { fn identity<T>(&self, value: T) -> T { value } }
                struct Factory;
                impl Identity for Factory {}
                fn persistent(factory: &Factory, database: wow_database::CharacterDatabase) {
                    consume(factory.identity(database).pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_resolves_registered_callables_inside_join_macros() {
        let baseline = inventory(
            r#"
                fn database() -> wow_database::CharacterDatabase { unreachable!() }
                async fn persistent() {
                    let database = tokio::join!(async { database() }).0;
                    consume(database.pool());
                }
            "#,
        )
        .unwrap();
        assert!(
            baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn persistent"
                    && row.operation == PersistenceOperation::PoolAccess)
        );
    }

    #[test]
    fn persistence_inventory_does_not_treat_macro_identifiers_as_function_calls() {
        let baseline = inventory(
            r#"
                fn error() -> wow_database::CharacterDatabase { unreachable!() }
                async fn clean(message: &str) {
                    let value = tokio::join!(async { tracing::error!(message) }).0;
                    consume(value.pool());
                }
            "#,
        )
        .unwrap();
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean"
                    && row.operation == PersistenceOperation::PoolAccess)
        );
    }

    #[test]
    fn persistence_inventory_does_not_apply_diverging_let_else_state() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase, maybe: Option<u8>) {
                    let mut value = Some(database);
                    let Some(_) = maybe else { value = None; return; };
                    consume(value.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        assert!(
            baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn persistent"
                    && row.operation == PersistenceOperation::PoolAccess)
        );
    }

    #[test]
    fn persistence_inventory_does_not_apply_closure_side_effects_at_declaration() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut value = Some(database);
                    let clear = || { value = None; };
                    drop(clear);
                    consume(value.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        assert!(
            baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn persistent"
                    && row.operation == PersistenceOperation::PoolAccess)
        );
    }

    #[test]
    fn persistence_inventory_does_not_apply_async_side_effects_at_future_creation() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut value = Some(database);
                    let future = async { value = None; };
                    drop(future);
                    consume(value.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        assert!(
            baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn persistent"
                    && row.operation == PersistenceOperation::PoolAccess)
        );
    }

    #[test]
    fn persistence_inventory_retains_values_inserted_into_mutable_containers() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut values = Vec::new();
                    values.push(database);
                    consume(values[0].pool());
                }
            "#,
        )
        .unwrap();
        assert!(
            baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn persistent"
                    && row.operation == PersistenceOperation::PoolAccess)
        );
    }

    #[test]
    fn persistence_inventory_retains_values_inserted_through_ufcs() {
        let baseline = inventory(
            r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut values = Vec::new();
                    Vec::push(&mut values, database);
                    consume(values[0].pool());
                }
                fn clean(database: wow_database::CharacterDatabase) {
                    let mut values = Vec::new();
                    Vec::push(&mut values, Clean);
                    consume(values[0].pool());
                    drop(database);
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_binds_let_chain_patterns_in_while_body() {
        let baseline = inventory(
            r#"
                struct Holder(wow_database::CharacterDatabase);
                fn persistent(maybe: Option<Holder>, enabled: bool) {
                    while let Some(holder) = maybe && enabled {
                        consume(holder.0.pool());
                        break;
                    }
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_applies_closure_mutations_only_when_invoked() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let install = || { slot = Some(database); };
                    install();
                    consume(slot.unwrap().pool());
                }
                fn clean(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let install = || { slot = Some(database); };
                    drop(install);
                    consume(slot.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_binds_let_chain_values_in_later_conditions() {
        let baseline = inventory(
            r#"
                struct Holder(wow_database::CharacterDatabase);
                fn persistent(maybe: Option<Holder>) {
                    if let Some(holder) = maybe && holder.0.pool().is_closed() {}
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_projects_nested_generic_tuple_slots_independently() {
        let baseline = inventory(
            r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                fn first<T, U>(value: (T, U)) -> T { value.0 }
                fn second<T, U>(value: (T, U)) -> U { value.1 }
                fn clean(database: wow_database::CharacterDatabase) {
                    consume(first((Clean, database)).pool());
                }
                fn persistent(database: wow_database::CharacterDatabase) {
                    consume(second((Clean, database)).pool());
                }
            "#,
        )
        .unwrap();
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_applies_turbofish_to_function_item_aliases() {
        let baseline = inventory(
            r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                fn make<T>() -> T { unreachable!() }
                fn persistent() {
                    let factory = make::<wow_database::CharacterDatabase>;
                    consume(factory().pool());
                }
                fn clean() {
                    let factory = make::<Clean>;
                    consume(factory().pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_applies_closure_mutations_for_combinators_conditionally() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    Some(()).map(|_| slot = Some(database));
                    consume(slot.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_substitutes_associated_types_in_generic_bounds() {
        let baseline = inventory(
            r#"
                trait Maker { type Output; fn make(&self) -> Self::Output; }
                fn persistent<T: Maker<Output = wow_database::CharacterDatabase>>(value: &T) {
                    consume(value.make().pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_propagates_standard_replacement_writes() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    std::mem::replace(&mut slot, Some(database));
                    consume(slot.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_propagates_mutable_argument_writes_conservatively() {
        let baseline = inventory(
            r#"
                fn install<T>(slot: &mut Option<T>, value: T) { *slot = Some(value); }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    install(&mut slot, database);
                    consume(slot.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_recomputes_loop_back_edges_to_a_fixed_point() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let source = &database;
                    let mut slot = None;
                    loop {
                        if let Some(db) = slot { consume(db.pool()); break; }
                        slot = Some(source);
                    }
                }
            "#,
        )
        .unwrap();
        let pool_rows = baseline
            .accesses
            .iter()
            .filter(|row| {
                row.enclosing == "fn persistent"
                    && row.operation == PersistenceOperation::PoolAccess
            })
            .collect::<Vec<_>>();
        assert_eq!(pool_rows.len(), 1);
        assert_eq!(pool_rows[0].count, 1);
    }

    #[test]
    fn persistence_inventory_widens_recursively_growing_loop_values() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut value = database;
                    loop {
                        value = (value,);
                        consume(&value);
                    }
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::ValueAlias
        }));
    }

    #[test]
    fn persistence_inventory_propagates_assignments_into_places() {
        let baseline = inventory(
            r#"
                struct Holder<T> { value: T }
                fn field(database: wow_database::CharacterDatabase) {
                    let mut holder = Holder { value: None };
                    holder.value = Some(database);
                    consume(holder.value.unwrap().pool());
                }
                fn index(database: wow_database::CharacterDatabase) {
                    let mut values = [None];
                    values[0] = Some(database);
                    consume(values[0].unwrap().pool());
                }
            "#,
        )
        .unwrap();
        for enclosing in ["fn field", "fn index"] {
            assert!(baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
            }));
            assert!(baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::StoreEscape
            }));
        }
    }

    #[test]
    fn persistence_inventory_parameterizes_closure_mutation_effects() {
        let baseline = inventory(
            r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let mut install = |value| slot = Some(value);
                    install(database);
                    consume(slot.unwrap().pool());
                }
                fn clean(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let mut install = |value| slot = Some(value);
                    install(Clean);
                    consume(slot.unwrap().pool());
                    drop(database);
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_preserves_skipped_let_chain_state() {
        let baseline = inventory(
            r#"
                fn persistent(
                    database: wow_database::CharacterDatabase,
                    maybe: Option<()>,
                ) {
                    let mut slot = Some(database);
                    if let Some(_) = maybe && { slot = None; true } {}
                    consume(slot.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_retains_block_local_tail_values() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let wrapped = { let alias = database; alias };
                    consume(wrapped.pool());
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_rejects_unmounted_include_sources() {
        let error = inventory(r#"include!("db_impl.rs");"#).unwrap_err();
        assert!(
            error.contains("include! whose Rust source is outside"),
            "{error}"
        );
        let body_error = inventory(r#"fn hidden() { include!("db_impl.rs"); }"#).unwrap_err();
        assert!(
            body_error.contains("include! whose Rust source is outside"),
            "{body_error}"
        );

        let pinned = |suffix: &str| {
            let source = format!(
                "pub mod bgs {{ pub mod protocol {{ include!(concat!(env!(\"OUT_DIR\"), {suffix:?})); }} }}"
            );
            inventory_persistence_accesses(&[ClassifiedPersistenceSource {
                classification: "direct_application_or_domain_access",
                package: "wow-proto",
                module: "crate",
                source_path: "crates/wow-proto/src/lib.rs",
                inherited_cfg: &[],
                source: &source,
            }])
        };
        pinned("/bgs.protocol.rs").expect("the exact pinned generated include is accepted");
        let changed = pinned("/unreviewed.rs").unwrap_err();
        assert!(
            changed.contains("include! whose Rust source is outside"),
            "{changed}"
        );
    }

    #[test]
    fn persistence_inventory_classifies_compile_time_string_macros_as_static_sql() {
        let baseline = inventory(
            r#"
                fn persistent() {
                    consume(sqlx::query(concat!("SELECT ", "* FROM account")));
                }
                fn aliased() {
                    use std::concat as static_sql;
                    consume(sqlx::query(static_sql!("SELECT ", "1")));
                }
            "#,
        )
        .unwrap();
        for enclosing in ["fn persistent", "fn aliased"] {
            assert!(baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::Query
            }));
            assert!(!baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::NonliteralSql
            }));
        }
    }

    #[test]
    fn persistence_inventory_rejects_unmounted_include_str_sql() {
        for source in [
            r#"fn direct() { consume(sqlx::query(include_str!("query.sql"))); }"#,
            r#"fn aliased() { let sql = include_str!("query.sql"); consume(sqlx::query(sql)); }"#,
        ] {
            let error = inventory(source).unwrap_err();
            assert!(error.contains("include_str! SQL"), "{error}");
        }
    }

    #[test]
    fn persistence_inventory_rejects_environment_sourced_sql() {
        let error =
            inventory(r#"fn hidden() { consume(sqlx::query(env!("QUERY"))); }"#).unwrap_err();
        assert!(error.contains("passes env! SQL"), "{error}");
    }

    #[test]
    fn persistence_inventory_classifies_sql_parameters_as_nonliteral() {
        let baseline = inventory(
            r#"
                fn dynamic(db: wow_database::CharacterDatabase, query: &str) {
                    db.direct_query(query);
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn dynamic" && row.operation == PersistenceOperation::NonliteralSql
        }));
    }

    #[test]
    fn persistence_inventory_rejects_returning_calls_hidden_in_unknown_macros() {
        let error = inventory(
            r#"
                macro_rules! forward { ($value:expr) => { $value.pool() } }
                fn make_database() -> wow_database::CharacterDatabase { unreachable!() }
                fn hidden() { forward!(make_database()); }
            "#,
        )
        .unwrap_err();
        assert!(error.contains("unknown macro forward"), "{error}");
    }

    #[test]
    fn persistence_inventory_rejects_generic_persistence_in_block_local_functions() {
        let error = inventory(
            r#"
                trait HasPool { fn pool(&self); }
                fn hidden(database: wow_database::CharacterDatabase) {
                    fn use_pool<T: HasPool>(value: T) { value.pool(); }
                    use_pool(database);
                }
            "#,
        )
        .unwrap_err();
        assert!(
            error.contains("block-local function with persistence-shaped operations (pool)"),
            "{error}"
        );
    }

    #[test]
    fn persistence_inventory_rejects_generic_module_persistence_operations() {
        let error = inventory(
            r#"
                trait HasPool { fn pool(&self); }
                fn use_pool<T: HasPool>(value: T) { value.pool(); }
                fn hidden(database: wow_database::CharacterDatabase) { use_pool(database); }
            "#,
        )
        .unwrap_err();
        assert!(
            error.contains(
                "fn use_pool is generic and contains persistence-shaped operations (pool)"
            ),
            "{error}"
        );
    }

    #[test]
    fn persistence_inventory_classifies_trait_ufcs_from_receiver_flow() {
        let baseline = inventory(
            r#"
                trait PoolProvider { fn pool(&self); }
                fn persistent(database: wow_database::CharacterDatabase) {
                    PoolProvider::pool(&database);
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.operation == PersistenceOperation::PoolAccess
                && row.target == PersistenceTarget::CharacterDatabase
        }));
    }

    #[test]
    fn persistence_inventory_distinguishes_same_named_trait_impl_workflows() {
        let baseline = inventory(
            r#"
                trait LoginStore { fn save(&self); }
                trait CharacterStore { fn save(&self); }
                struct Worker {
                    login: wow_database::LoginDatabase,
                    character: wow_database::CharacterDatabase,
                }
                impl LoginStore for Worker {
                    fn save(&self) { consume(self.login.pool()); }
                }
                impl CharacterStore for Worker {
                    fn save(&self) { consume(self.character.pool()); }
                }
            "#,
        )
        .unwrap();
        let workflows = baseline
            .accesses
            .iter()
            .filter(|row| row.operation == PersistenceOperation::PoolAccess)
            .map(|row| row.enclosing.clone())
            .collect::<BTreeSet<_>>();
        assert!(
            workflows
                .iter()
                .any(|name| name.contains("LoginStore for Worker::save"))
        );
        assert!(
            workflows
                .iter()
                .any(|name| name.contains("CharacterStore for Worker::save"))
        );
    }

    #[test]
    fn persistence_inventory_inspects_generated_attributes_nested_in_cfg_attr() {
        let baseline = inventory(
            r#"
                #[cfg_attr(test, derive(sqlx::FromRow))]
                struct Row;
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.source_class == "test_fixture"
                && row.operation == PersistenceOperation::MacroReference
                && row.generated_input
        }));
        assert!(!baseline.accesses.iter().any(|row| {
            row.source_class == "production"
                && row.operation == PersistenceOperation::MacroReference
                && row.generated_input
        }));
    }

    #[test]
    fn persistence_inventory_preserves_constant_visibility() {
        let baseline = inventory(
            r#"
                pub const DATABASE: Option<wow_database::CharacterDatabase> = None;
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "const DATABASE"
                && row.operation == PersistenceOperation::TypeReference
                && row.visibility == "pub"
        }));
    }

    #[test]
    fn persistence_inventory_models_clean_block_local_callables() {
        let baseline = inventory(
            r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                fn inferred(database: wow_database::CharacterDatabase) {
                    fn pass<T>(value: T) -> T { value }
                    let wrapped = pass(database);
                    consume(wrapped.pool());
                }
                fn explicit_persistent(database: wow_database::CharacterDatabase) {
                    fn make<T, U>(_input: U) -> T { unreachable!() }
                    consume(make::<wow_database::CharacterDatabase, _>(database).pool());
                }
                fn explicit_clean(database: wow_database::CharacterDatabase) {
                    fn make<T, U>(_input: U) -> T { unreachable!() }
                    consume(make::<Clean, _>(database).pool());
                }
            "#,
        )
        .unwrap();
        for enclosing in ["fn inferred", "fn explicit_persistent"] {
            assert!(
                baseline.accesses.iter().any(|row| {
                    row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
                }),
                "missing pool access for {enclosing}: {:#?}",
                baseline.accesses
            );
        }
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn explicit_clean"
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_projects_destructured_closure_parameters() {
        let baseline = inventory(
            r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                fn clean(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let install = |(value, _)| slot = Some(value);
                    install((Clean, database));
                    consume(slot.unwrap().pool());
                }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let install = |(value, _)| slot = Some(value);
                    install((database, Clean));
                    consume(slot.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_registers_default_trait_method_parameters() {
        let baseline = inventory(
            r#"
                struct Holder(wow_database::CharacterDatabase);
                trait Access {
                    fn leak(&self, holder: Holder) { consume(holder.0.pool()); }
                }
            "#,
        )
        .unwrap();
        assert!(
            baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "trait Access::leak"
                    && row.operation == PersistenceOperation::PoolAccess)
        );
    }

    #[test]
    fn persistence_inventory_canonicalizes_qualified_inherent_impl_owners() {
        let dto = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::dto",
            source_path: "src/dto.rs",
            inherited_cfg: &[],
            source: "struct Holder(wow_database::CharacterDatabase); struct Factory;",
        };
        let extensions = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::extensions",
            source_path: "src/extensions.rs",
            inherited_cfg: &[],
            source: "impl crate::dto::Factory { fn make(&self) -> crate::dto::Holder { unreachable!() } }",
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "fixture",
            module: "crate::consumer",
            source_path: "src/consumer.rs",
            inherited_cfg: &[],
            source: "fn persistent(factory: crate::dto::Factory) { consume(factory.make().0.pool()); }",
        };
        let baseline = inventory_persistence_accesses(&[consumer, dto, extensions]).unwrap();
        assert!(
            baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn persistent"
                    && row.operation == PersistenceOperation::PoolAccess)
        );
    }

    #[test]
    fn persistence_inventory_resolves_dependency_callable_returns() {
        let provider = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-a",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                pub struct Holder(pub wow_database::CharacterDatabase);
                pub struct Factory;
                impl Factory {
                    pub fn make() -> Holder { unreachable!() }
                }
                pub struct Constructed(pub wow_database::CharacterDatabase);
                impl Constructed {
                    pub fn new(database: wow_database::CharacterDatabase) -> Self {
                        Self(database)
                    }
                }
                pub fn make() -> Holder { unreachable!() }
            "#,
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "consumer-b",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                fn free_function() { consume(provider_alias::make().0.pool()); }
                fn associated_function() {
                    consume(provider_alias::Factory::make().0.pool());
                }
                fn self_constructor(database: wow_database::CharacterDatabase) {
                    let value = provider_alias::Constructed::new(database);
                    consume(value.0.pool());
                }
            "#,
        };
        let dependencies = WorkspaceDependencyAliases {
            production: BTreeMap::from([(
                "consumer-b".to_owned(),
                BTreeMap::from([("provider_alias".to_owned(), "provider_a".to_owned())]),
            )]),
            test: BTreeMap::new(),
        };
        let baseline =
            inventory_persistence_accesses_with_dependencies(&[consumer, provider], &dependencies)
                .unwrap();
        for enclosing in [
            "fn free_function",
            "fn associated_function",
            "fn self_constructor",
        ] {
            assert!(baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
            }));
        }
    }

    #[test]
    fn persistence_inventory_preserves_function_item_return_flow() {
        let baseline = inventory(
            r#"
                fn database() -> wow_database::CharacterDatabase { unreachable!() }
                struct Factory;
                impl Factory {
                    fn database() -> wow_database::CharacterDatabase { unreachable!() }
                }
                fn free_alias() {
                    let factory = crate::database;
                    consume(factory().pool());
                }
                fn associated_alias() {
                    let factory = Factory::database;
                    consume(factory().pool());
                }
            "#,
        )
        .unwrap();
        for enclosing in ["fn free_alias", "fn associated_alias"] {
            assert!(baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
            }));
        }
    }

    #[test]
    fn persistence_inventory_propagates_dependency_macro_result_flow() {
        let provider = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-a",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                #[macro_export]
                macro_rules! hidden_database {
                    () => { wow_database::CharacterDatabase::default() };
                }
            "#,
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "consumer-b",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                fn persistent() {
                    consume(provider_alias::hidden_database!().pool());
                }
            "#,
        };
        let dependencies = WorkspaceDependencyAliases {
            production: BTreeMap::from([(
                "consumer-b".to_owned(),
                BTreeMap::from([("provider_alias".to_owned(), "provider_a".to_owned())]),
            )]),
            test: BTreeMap::new(),
        };
        let baseline =
            inventory_persistence_accesses_with_dependencies(&[consumer, provider], &dependencies)
                .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent"
                && row.operation == PersistenceOperation::MacroReference
        }));
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_propagates_macros_and_values_through_facades() {
        let provider = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-a",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                #[macro_export]
                macro_rules! hidden_database {
                    () => { wow_database::CharacterDatabase::default() };
                }
                pub static DATABASE: wow_database::CharacterDatabase = todo!();
            "#,
        };
        let named_facade = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-b",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                pub use provider_a_alias::{
                    DATABASE as EXPORTED_DATABASE,
                    hidden_database as exported_database,
                };
            "#,
        };
        let module_facade = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-module",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: "pub use provider_a_alias as source;",
        };
        let glob_facade = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-glob",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: "pub use provider_a_alias::*;",
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "consumer-c",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                fn named_macro() { consume(provider_b_alias::exported_database!().pool()); }
                fn named_value() { consume(provider_b_alias::EXPORTED_DATABASE.pool()); }
                fn module_macro() {
                    consume(provider_module_alias::source::hidden_database!().pool());
                }
                fn module_value() {
                    consume(provider_module_alias::source::DATABASE.pool());
                }
                fn glob_macro() { consume(provider_glob_alias::hidden_database!().pool()); }
                fn glob_value() { consume(provider_glob_alias::DATABASE.pool()); }
            "#,
        };
        let dependencies = WorkspaceDependencyAliases {
            production: BTreeMap::from([
                (
                    "provider-b".to_owned(),
                    BTreeMap::from([("provider_a_alias".to_owned(), "provider_a".to_owned())]),
                ),
                (
                    "provider-module".to_owned(),
                    BTreeMap::from([("provider_a_alias".to_owned(), "provider_a".to_owned())]),
                ),
                (
                    "provider-glob".to_owned(),
                    BTreeMap::from([("provider_a_alias".to_owned(), "provider_a".to_owned())]),
                ),
                (
                    "consumer-c".to_owned(),
                    BTreeMap::from([
                        ("provider_b_alias".to_owned(), "provider_b".to_owned()),
                        (
                            "provider_module_alias".to_owned(),
                            "provider_module".to_owned(),
                        ),
                        ("provider_glob_alias".to_owned(), "provider_glob".to_owned()),
                    ]),
                ),
            ]),
            test: BTreeMap::new(),
        };
        let baseline = inventory_persistence_accesses_with_dependencies(
            &[consumer, glob_facade, module_facade, named_facade, provider],
            &dependencies,
        )
        .unwrap();
        for enclosing in [
            "fn named_macro",
            "fn named_value",
            "fn module_macro",
            "fn module_value",
            "fn glob_macro",
            "fn glob_value",
        ] {
            assert!(
                baseline.accesses.iter().any(|row| {
                    row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
                }),
                "missing facade flow for {enclosing}"
            );
        }
    }

    #[test]
    fn persistence_inventory_does_not_import_unrelated_callables_or_macros() {
        let provider = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "unrelated-provider",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                pub struct Holder(pub wow_database::CharacterDatabase);
                pub fn make() -> Holder { unreachable!() }
                #[macro_export]
                macro_rules! hidden_database {
                    () => { wow_database::CharacterDatabase::default() };
                }
            "#,
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "consumer-b",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                fn clean() {
                    consume(unrelated_provider::make().0.pool());
                    consume(unrelated_provider::hidden_database!().pool());
                }
            "#,
        };
        let baseline = inventory_persistence_accesses_with_dependencies(
            &[consumer, provider],
            &WorkspaceDependencyAliases::default(),
        )
        .unwrap();
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean")
        );
    }

    #[test]
    fn persistence_inventory_preserves_state_across_short_circuit_rhs() {
        let baseline = inventory(
            r#"
                fn or_path(database: wow_database::CharacterDatabase, stop: bool) {
                    let mut value = Some(database);
                    stop || { value = None; true };
                    consume(value.unwrap().pool());
                }
                fn and_path(database: wow_database::CharacterDatabase, proceed: bool) {
                    let mut value = Some(database);
                    proceed && { value = None; true };
                    consume(value.unwrap().pool());
                }
                fn unconditional(database: wow_database::CharacterDatabase) {
                    let mut value = Some(database);
                    value = None;
                    consume(value.unwrap().pool());
                }
            "#,
        )
        .unwrap();
        for enclosing in ["fn or_path", "fn and_path"] {
            assert!(baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
            }));
        }
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn unconditional" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_resolves_callable_returns_through_dependency_reexports() {
        let provider = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-a",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                pub struct Holder(pub wow_database::CharacterDatabase);
                struct Hidden(pub wow_database::CharacterDatabase);
                pub fn make() -> Holder { unreachable!() }
            "#,
        };
        let reexporter = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-b",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: "pub use provider_a_alias::{make as create, Holder as ExportedHolder};",
        };
        let glob_reexporter = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-glob",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: "pub use provider_a_alias::*;",
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "consumer-c",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                fn persistent() { consume(provider_b_alias::create().0.pool()); }
                fn globbed() { consume(provider_glob_alias::make().0.pool()); }
                fn named_type(holder: provider_b_alias::ExportedHolder) {
                    consume(holder.0.pool());
                }
                fn globbed_type(holder: provider_glob_alias::Holder) {
                    consume(holder.0.pool());
                }
                fn private_glob(holder: provider_glob_alias::Hidden) {
                    consume(holder.0.pool());
                }
            "#,
        };
        let dependencies = WorkspaceDependencyAliases {
            production: BTreeMap::from([
                (
                    "provider-b".to_owned(),
                    BTreeMap::from([("provider_a_alias".to_owned(), "provider_a".to_owned())]),
                ),
                (
                    "consumer-c".to_owned(),
                    BTreeMap::from([
                        ("provider_b_alias".to_owned(), "provider_b".to_owned()),
                        ("provider_glob_alias".to_owned(), "provider_glob".to_owned()),
                    ]),
                ),
                (
                    "provider-glob".to_owned(),
                    BTreeMap::from([("provider_a_alias".to_owned(), "provider_a".to_owned())]),
                ),
            ]),
            test: BTreeMap::new(),
        };
        let baseline = inventory_persistence_accesses_with_dependencies(
            &[consumer, reexporter, glob_reexporter, provider],
            &dependencies,
        )
        .unwrap();
        for enclosing in [
            "fn persistent",
            "fn globbed",
            "fn named_type",
            "fn globbed_type",
        ] {
            assert!(
                baseline.accesses.iter().any(|row| {
                    row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
                }),
                "missing pool flow for {enclosing}"
            );
        }
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn private_glob" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_imports_dependency_trait_return_registries() {
        let provider = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-a",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                pub struct Holder(pub wow_database::CharacterDatabase);
                pub trait Base {
                    fn make(&self) -> Holder;
                    fn pass<T>(&self, value: T) -> T;
                }
                pub trait Maker: Base {}
            "#,
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "consumer-b",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                fn declared<T: provider_alias::Maker>(value: &T) {
                    consume(value.make().0.pool());
                }
                fn inferred<T: provider_alias::Maker>(value: &T, database: wow_database::CharacterDatabase) {
                    consume(value.pass(database).pool());
                }
            "#,
        };
        let facade = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-b",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: "pub use provider_alias::Maker;",
        };
        let downstream = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "consumer-c",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                fn reexported<T: facade_alias::Maker>(value: &T) {
                    consume(value.make().0.pool());
                }
            "#,
        };
        let dependencies = WorkspaceDependencyAliases {
            production: BTreeMap::from([
                (
                    "consumer-b".to_owned(),
                    BTreeMap::from([("provider_alias".to_owned(), "provider_a".to_owned())]),
                ),
                (
                    "provider-b".to_owned(),
                    BTreeMap::from([("provider_alias".to_owned(), "provider_a".to_owned())]),
                ),
                (
                    "consumer-c".to_owned(),
                    BTreeMap::from([("facade_alias".to_owned(), "provider_b".to_owned())]),
                ),
            ]),
            test: BTreeMap::new(),
        };
        let baseline = inventory_persistence_accesses_with_dependencies(
            &[consumer, downstream, facade, provider],
            &dependencies,
        )
        .unwrap();
        for enclosing in ["fn declared", "fn inferred", "fn reexported"] {
            assert!(baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
            }));
        }
    }

    #[test]
    fn persistence_inventory_qualifies_test_only_dependency_supertraits() {
        let provider = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-a",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                #[cfg(test)]
                pub struct Holder(pub wow_database::CharacterDatabase);
                #[cfg(test)]
                pub trait Base { fn make(&self) -> Holder; }
                #[cfg(test)]
                pub trait Maker: Base {}
            "#,
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "consumer-b",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                #[cfg(test)]
                fn test_only<T: provider_alias::Maker>(value: &T) {
                    consume(value.make().0.pool());
                }
            "#,
        };
        let dependencies = WorkspaceDependencyAliases {
            production: BTreeMap::new(),
            test: BTreeMap::from([(
                "consumer-b".to_owned(),
                BTreeMap::from([("provider_alias".to_owned(), "provider_a".to_owned())]),
            )]),
        };
        let baseline =
            inventory_persistence_accesses_with_dependencies(&[consumer, provider], &dependencies)
                .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn test_only"
                && row.source_class == "test_fixture"
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_preserves_standard_identity_flow() {
        let baseline = inventory(
            r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    consume(std::convert::identity(database).pool());
                }
                use std::convert::identity;
                fn imported(database: wow_database::CharacterDatabase) {
                    consume(identity(database).pool());
                }
                mod custom {
                    pub fn identity(value: bool) -> bool { value }
                }
                fn clean(value: bool) {
                    consume(custom::identity(value));
                }
            "#,
        )
        .unwrap();
        for enclosing in ["fn persistent", "fn imported"] {
            assert!(baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
            }));
        }
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn clean")
        );
    }

    #[test]
    fn persistence_inventory_retains_struct_literal_nominal_fields() {
        let baseline = inventory(
            r#"
                struct Holder {
                    database: wow_database::CharacterDatabase,
                    clean: bool,
                }
                fn clean(database: wow_database::CharacterDatabase) {
                    let holder = Holder { database, clean: false };
                    consume(holder.clean);
                }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let holder = Holder { database, clean: false };
                    consume(holder.database.pool());
                }
                fn side_effect(database: wow_database::CharacterDatabase) {
                    let holder = Holder {
                        database: wow_database::CharacterDatabase::default(),
                        clean: { drop(database); false },
                    };
                    consume(holder.clean);
                }
                struct Decoded {
                    value: u32,
                    clean: bool,
                }
                fn decoded(result: wow_database::SqlResult) {
                    let decoded = Decoded {
                        value: result.try_read(0).unwrap_or(0),
                        clean: false,
                    };
                    consume(decoded.value);
                }
            "#,
        )
        .unwrap();
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn clean"
                && matches!(
                    row.operation,
                    PersistenceOperation::ArgumentEscape | PersistenceOperation::PoolAccess
                )
        }));
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn side_effect"
                && row.symbol == "consume"
                && row.operation == PersistenceOperation::ArgumentEscape
        }));
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn decoded"
                && row.target == PersistenceTarget::SqlResult
                && row.operation == PersistenceOperation::ArgumentEscape
        }));
    }

    #[test]
    fn persistence_inventory_resolves_public_module_alias_named_types() {
        let provider = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-a",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                pub mod dto {
                    pub struct Holder(pub wow_database::CharacterDatabase);
                    struct Hidden(pub wow_database::CharacterDatabase);
                }
            "#,
        };
        let facade = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "provider-b",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: "pub use provider_a_alias::dto as types;",
        };
        let consumer = ClassifiedPersistenceSource {
            classification: "database_adapter_core",
            package: "consumer-c",
            module: "crate",
            source_path: "src/lib.rs",
            inherited_cfg: &[],
            source: r#"
                fn persistent(holder: facade_alias::types::Holder) {
                    consume(holder.0.pool());
                }
                fn private(holder: facade_alias::types::Hidden) {
                    consume(holder.0.pool());
                }
            "#,
        };
        let dependencies = WorkspaceDependencyAliases {
            production: BTreeMap::from([
                (
                    "provider-b".to_owned(),
                    BTreeMap::from([("provider_a_alias".to_owned(), "provider_a".to_owned())]),
                ),
                (
                    "consumer-c".to_owned(),
                    BTreeMap::from([("facade_alias".to_owned(), "provider_b".to_owned())]),
                ),
            ]),
            test: BTreeMap::new(),
        };
        let baseline = inventory_persistence_accesses_with_dependencies(
            &[consumer, facade, provider],
            &dependencies,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn private" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_infers_generic_function_item_alias_arguments() {
        let baseline = inventory(
            r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                fn first<T, U>(value: T, _other: U) -> T { value }
                fn clean(database: wow_database::CharacterDatabase) {
                    let first_alias = first;
                    first_alias(Clean, database).pool();
                }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let first_alias = first;
                    first_alias(database, Clean).pool();
                }
            "#,
        )
        .unwrap();
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_binds_let_chain_patterns_in_then_branch() {
        let baseline = inventory(
            r#"
                struct Holder(wow_database::CharacterDatabase);
                fn persistent(maybe: Option<Holder>, enabled: bool) {
                    if let Some(holder) = maybe && enabled {
                        consume(holder.0.pool());
                    }
                }
            "#,
        )
        .unwrap();
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_instantiates_generic_named_type_fields() {
        let baseline = inventory(
            r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                struct Holder<T>(T);
                fn clean(holder: Holder<Clean>) { holder.0.pool(); }
                fn persistent(holder: Holder<wow_database::CharacterDatabase>) {
                    holder.0.pool();
                }
            "#,
        )
        .unwrap();
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        }));
    }

    #[test]
    fn persistence_inventory_preserves_interpolated_sql_through_string_views() {
        let baseline = inventory(
            r#"
                fn dynamic(id: u32) {
                    sqlx::query(format!("SELECT {id}").as_str());
                    sqlx::query(format!("SELECT {id}").as_ref());
                }
            "#,
        )
        .unwrap();
        assert_eq!(
            baseline
                .accesses
                .iter()
                .filter(|row| {
                    row.enclosing == "fn dynamic"
                        && row.operation == PersistenceOperation::InterpolatedSql
                })
                .count(),
            2
        );
    }
}
