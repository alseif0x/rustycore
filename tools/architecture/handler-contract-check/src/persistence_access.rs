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
//!
//! # What this grammar does not decide
//!
//! It reads statements that are *pinned* — a string literal, a `concat!`, a
//! name bound to one of those — and it does not evaluate what an expression
//! would build at run time. A `+` chain, a `format!` template, a branch, a
//! helper's return, and a projection deliberately yield no statement text.
//!
//! "Pinned" also means pinned *here*: a constant this package declares, or one
//! it imports within its own item registry. A constant owned by another
//! package is read as runtime-assembled, which over-reports rather than
//! under-reports — the call site still carries its row, and the reviewed
//! workflow annotation covering it states the affinity. Reaching across
//! packages would mean resolving their re-exports and globs too, which is the
//! same open-ended chase in a different register.
//!
//! That boundary is a design decision, not an omission. Deciding "which string
//! does this expression produce" has no natural stopping point: every answer
//! invites another shape to reconstruct, and each reconstruction has to be
//! kept faithful to MySQL's own lexing. The inventory therefore claims less
//! and proves more. A call site whose statement is assembled at run time is
//! still ratcheted — as `InterpolatedSql` or `NonliteralSql`, with its pool,
//! transaction, and escape flow intact — and the semantic policy requires a
//! reviewed workflow annotation to state its logical database, connection
//! affinity, and ordering. A curated sentence is a better authority for those
//! facts than an inference drawn from token text.

use std::collections::{BTreeMap, BTreeSet};

mod callable_reexports;
mod records;
use records::{AccessAccumulator, NewAccess, PersistenceSourceClass, RecordContext};
// Preserve the crate-local schema type path even when consumers infer row types.
#[allow(unused_imports)]
pub(crate) use records::PersistenceAccessRecord;
pub(crate) use records::{
    ClassifiedPersistenceSource, PersistenceAccessBaseline, PersistenceOperation,
    PersistenceTarget, compare_persistence_access_baseline, render_persistence_access_baseline,
};
mod sql_text;
use callable_reexports::{
    collect_local_callable_imports, collect_public_callable_reexports,
    inferred_return_with_unresolved_fallback, resolve_local_callable_imports,
    resolve_public_callable_reexports,
};
use sql_text::{
    is_standard_string_conversion, macro_shadows_before, query_macro_statement,
    sql_is_advisory_lock, standard_string_macro_of,
};

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
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

const QUERY_CONSTRUCTORS: &[&str] = &[
    "query",
    "query_as",
    "query_as_with",
    "query_file",
    "query_file_as",
    "query_scalar",
    "query_scalar_with",
    "query_with",
    "raw_sql",
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
    "and_then",
    "map",
    "map_err",
    "map_or",
    "map_or_else",
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
    generic_types: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for PersistenceOperationSyntax {
    fn visit_expr_method_call(&mut self, method: &'ast ExprMethodCall) {
        let name = normalized_ident(&method.method);
        if !matches!(name.as_str(), "new" | "open")
            && PersistenceOperation::from_executor_method(&name).is_some()
        {
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
            let owner = path
                .path
                .segments
                .iter()
                .nth_back(1)
                .map(|segment| normalized_ident(&segment.ident));
            if name != "open" || owner.is_some_and(|owner| self.generic_types.contains(&owner)) {
                self.symbols.insert(name);
            }
        }
        syn::visit::visit_expr_call(self, call);
    }
}

fn persistence_operations_in_syntax(item: &Item) -> BTreeSet<String> {
    let mut visitor = PersistenceOperationSyntax::default();
    visitor.visit_item(item);
    visitor.symbols
}

fn persistence_operations_in_block(
    block: &syn::Block,
    generic_types: BTreeSet<String>,
) -> BTreeSet<String> {
    let mut visitor = PersistenceOperationSyntax {
        generic_types,
        ..PersistenceOperationSyntax::default()
    };
    visitor.visit_block(block);
    visitor.symbols
}

#[derive(Default)]
struct CalledParameterInputs {
    parameters: BTreeMap<String, usize>,
    called: BTreeSet<usize>,
}

impl<'ast> Visit<'ast> for CalledParameterInputs {
    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(path) = call.func.as_ref()
            && path.qself.is_none()
            && path.path.segments.len() == 1
            && let Some(name) = last_path_name(&path.path)
            && let Some(index) = self.parameters.get(&name)
        {
            self.called.insert(*index);
        }
        syn::visit::visit_expr_call(self, call);
    }
}

fn called_parameter_inputs(function: &ItemFn) -> BTreeSet<usize> {
    let parameters = function
        .sig
        .inputs
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let FnArg::Typed(typed) = input else {
                return None;
            };
            let Pat::Ident(ident) = typed.pat.as_ref() else {
                return None;
            };
            Some((normalized_ident(&ident.ident), index))
        })
        .collect();
    let mut visitor = CalledParameterInputs {
        parameters,
        ..CalledParameterInputs::default()
    };
    visitor.visit_block(&function.block);
    visitor.called
}

#[derive(Default)]
struct MutableParameterWrites {
    parameters: BTreeMap<String, usize>,
    written: BTreeSet<usize>,
    receiver_fields: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for MutableParameterWrites {
    fn visit_expr_assign(&mut self, assignment: &'ast syn::ExprAssign) {
        if let Some((root, projections)) = assignment_place(&assignment.left)
            && root == "self"
            && let Some(PlaceProjection::Field(field)) = projections.first()
        {
            self.receiver_fields.insert(field.clone());
        }
        if let Some((root, _)) = assignment_place(&assignment.left)
            && let Some(index) = self.parameters.get(&root)
        {
            self.written.insert(*index);
        }
        if let Expr::Unary(unary) = assignment.left.as_ref()
            && matches!(unary.op, syn::UnOp::Deref(_))
            && let Some(name) = simple_assignment_name(&unary.expr)
            && let Some(index) = self.parameters.get(&name)
        {
            self.written.insert(*index);
        }
        syn::visit::visit_expr_assign(self, assignment);
    }
}

fn mutable_method_writes(method: &syn::ImplItemFn) -> (BTreeSet<String>, BTreeSet<usize>) {
    let receiver_is_mutable = method.sig.inputs.first().is_some_and(
        |input| matches!(input, FnArg::Receiver(receiver) if receiver.mutability.is_some()),
    );
    let parameters = method
        .sig
        .inputs
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let FnArg::Typed(typed) = input else {
                return None;
            };
            let Type::Reference(reference) = typed.ty.as_ref() else {
                return None;
            };
            if reference.mutability.is_none() {
                return None;
            }
            let Pat::Ident(ident) = typed.pat.as_ref() else {
                return None;
            };
            Some((normalized_ident(&ident.ident), index.saturating_sub(1)))
        })
        .collect();
    let mut visitor = MutableParameterWrites {
        parameters,
        ..MutableParameterWrites::default()
    };
    visitor.visit_block(&method.block);
    let receiver_fields = receiver_is_mutable
        .then_some(visitor.receiver_fields)
        .unwrap_or_default();
    (receiver_fields, visitor.written)
}

fn mutable_parameter_writes(function: &ItemFn) -> BTreeSet<usize> {
    let parameters = function
        .sig
        .inputs
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let FnArg::Typed(typed) = input else {
                return None;
            };
            let Type::Reference(reference) = typed.ty.as_ref() else {
                return None;
            };
            if reference.mutability.is_none() {
                return None;
            }
            let Pat::Ident(ident) = typed.pat.as_ref() else {
                return None;
            };
            Some((normalized_ident(&ident.ident), index))
        })
        .collect();
    let mut visitor = MutableParameterWrites {
        parameters,
        ..MutableParameterWrites::default()
    };
    visitor.visit_block(&function.block);
    visitor.written
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

/// The SQL an item initializer pins.
///
/// A macro is identified by the path it resolves to: somebody's
/// `other::concat!` expands to whatever it likes, so treating it as the
/// standard one would pin a statement that its definition can change.
fn source_sql_info(
    expression: &Expr,
    resolve_standard_macro: &dyn Fn(Vec<String>) -> Option<String>,
) -> (SqlExpressionKind, BTreeSet<String>) {
    match expression {
        Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Str(_)) => (
            SqlExpressionKind::Static,
            BTreeSet::from([normalized_tokens(expression)]),
        ),
        Expr::Reference(reference) => source_sql_info(&reference.expr, resolve_standard_macro),
        Expr::Paren(paren) => source_sql_info(&paren.expr, resolve_standard_macro),
        Expr::Group(group) => source_sql_info(&group.expr, resolve_standard_macro),
        Expr::Macro(mac) => {
            let leaf = last_path_name(&mac.mac.path).unwrap_or_default();
            // Only the standard compile-time macros pin a statement; a
            // namesake in another module is a nonliteral source.
            let name = match leaf.as_str() {
                "concat" | "stringify"
                    if resolve_standard_macro(path_names(&mac.mac.path)).is_none() =>
                {
                    String::new()
                }
                _ => leaf,
            };
            match name.as_str() {
                "env" => (SqlExpressionKind::Environment, BTreeSet::new()),
                "include_str" => (SqlExpressionKind::Included, BTreeSet::new()),
                "format" | "format_args" => (SqlExpressionKind::Interpolated, BTreeSet::new()),
                "stringify" => (
                    SqlExpressionKind::Static,
                    BTreeSet::from([normalized_tokens(expression)]),
                ),
                // The macro tokens already carry the arguments in order, so a
                // swapped pair of statements changes the source text itself.
                // The kind still comes from the arguments: a nested `env!` or
                // `include_str!` puts the statement outside the snapshot.
                "concat" => {
                    let Ok(arguments) =
                        syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
                            .parse2(mac.mac.tokens.clone())
                    else {
                        return (SqlExpressionKind::Nonliteral, BTreeSet::new());
                    };
                    let kind =
                        arguments
                            .iter()
                            .fold(SqlExpressionKind::Static, |kind, argument| {
                                kind.max(source_sql_info(argument, resolve_standard_macro).0)
                            });
                    let sources = match kind {
                        SqlExpressionKind::Static => {
                            BTreeSet::from([normalized_tokens(expression)])
                        }
                        _ => BTreeSet::new(),
                    };
                    (kind, sources)
                }
                _ => (SqlExpressionKind::Nonliteral, BTreeSet::new()),
            }
        }
        _ => (SqlExpressionKind::Nonliteral, BTreeSet::new()),
    }
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
    sql_sources: BTreeSet<String>,
    nominal_types: BTreeSet<String>,
    payload_variants: BTreeSet<Vec<NominalShape>>,
    tuple_items: Vec<VariableInfo>,
    field_items: BTreeMap<String, VariableInfo>,
    trait_bounds: BTreeSet<String>,
    type_generic_params: Vec<String>,
    callable_signatures: BTreeSet<CallableSignature>,
    closure_mutations: BTreeMap<String, VariableInfo>,
    mutable_pointees: BTreeSet<String>,
    mutable_places: BTreeSet<MutablePlace>,
    query_callable: bool,
    /// The SQLx executor method this value names, when it is one. A stored
    /// `sqlx::Executor::execute` still executes the SQL it is handed.
    executor_callable: Option<String>,
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
        self.sql_sources.extend(other.sql_sources.iter().cloned());
        self.mutable_pointees
            .extend(other.mutable_pointees.iter().cloned());
        self.mutable_places
            .extend(other.mutable_places.iter().cloned());
        self.query_callable |= other.query_callable;
        if self.executor_callable.is_none() {
            self.executor_callable = other.executor_callable.clone();
        }
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
        flattened.query_callable |= info.query_callable;
        if flattened.executor_callable.is_none() {
            flattened.executor_callable = info.executor_callable.clone();
        }
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
    function_called_inputs: BTreeMap<String, BTreeSet<usize>>,
    function_mutable_writes: BTreeMap<String, BTreeMap<usize, VariableInfo>>,
    method_returns: BTreeMap<(String, Option<String>, String), VariableInfo>,
    method_mutable_receivers: BTreeMap<(String, Option<String>, String), VariableInfo>,
    method_mutable_writes:
        BTreeMap<(String, Option<String>, String), BTreeMap<usize, VariableInfo>>,
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
    package_function_mutable_writes:
        std::sync::Arc<BTreeMap<String, BTreeMap<usize, VariableInfo>>>,
    package_function_generic_params: std::sync::Arc<BTreeMap<String, Vec<String>>>,
    package_function_generic_input_params: std::sync::Arc<BTreeMap<String, Vec<GenericInputSpec>>>,
    // Package-wide registries for inherent impl methods (keyed by canonical
    // crate-relative owner path): without them `factory.make()` only resolves
    // when the impl lives in the same module as the call.
    package_method_returns: std::sync::Arc<BTreeMap<(String, String), VariableInfo>>,
    package_method_mutable_receivers: std::sync::Arc<BTreeMap<(String, String), VariableInfo>>,
    package_method_mutable_writes:
        std::sync::Arc<BTreeMap<(String, String), BTreeMap<usize, VariableInfo>>>,
    package_method_generic_params: std::sync::Arc<BTreeMap<(String, String), Vec<String>>>,
    package_method_generic_input_params:
        std::sync::Arc<BTreeMap<(String, String), Vec<GenericInputSpec>>>,
    // Module constants/statics are value bindings, not lexical locals. Keep
    // both the current module's names and a package-wide canonical registry
    // so their declared persistence-bearing types survive path resolution.
    item_values: BTreeMap<String, VariableInfo>,
    package_item_values: std::sync::Arc<BTreeMap<String, VariableInfo>>,
    sqlx_namespaces: BTreeSet<String>,
    workspace_sqlx_namespaces: std::sync::Arc<BTreeSet<String>>,
    database_namespaces: BTreeSet<String>,
    query_callables: BTreeSet<String>,
    /// Names this module defines with `macro_rules!`. An unqualified builtin
    /// resolves to the same `module_path + name` shape as one of these, so the
    /// definitions have to be known to tell them apart.
    local_macro_definitions: BTreeSet<String>,
    /// Types this module declares. A `struct String` of one's own shadows the
    /// prelude type, and path resolution alone cannot tell them apart.
    local_type_definitions: BTreeSet<String>,
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
            function_called_inputs: BTreeMap::new(),
            function_mutable_writes: BTreeMap::new(),
            method_returns: BTreeMap::new(),
            method_mutable_receivers: BTreeMap::new(),
            method_mutable_writes: BTreeMap::new(),
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
            package_function_mutable_writes: std::sync::Arc::new(BTreeMap::new()),
            package_function_generic_params: std::sync::Arc::new(BTreeMap::new()),
            package_function_generic_input_params: std::sync::Arc::new(BTreeMap::new()),
            package_method_returns: std::sync::Arc::new(BTreeMap::new()),
            package_method_mutable_receivers: std::sync::Arc::new(BTreeMap::new()),
            package_method_mutable_writes: std::sync::Arc::new(BTreeMap::new()),
            package_method_generic_params: std::sync::Arc::new(BTreeMap::new()),
            package_method_generic_input_params: std::sync::Arc::new(BTreeMap::new()),
            item_values: BTreeMap::new(),
            package_item_values: std::sync::Arc::new(BTreeMap::new()),
            sqlx_namespaces: BTreeSet::from(["sqlx".to_owned()]),
            workspace_sqlx_namespaces: std::sync::Arc::new(BTreeSet::new()),
            database_namespaces: BTreeSet::from(["wow_database".to_owned()]),
            query_callables: BTreeSet::new(),
            local_macro_definitions: BTreeSet::new(),
            local_type_definitions: BTreeSet::new(),
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

fn sqlx_pool_options_target(names: &[String]) -> Option<PersistenceTarget> {
    names.iter().find_map(|name| match name.as_str() {
        "MySqlPoolOptions" => Some(PersistenceTarget::MySqlPool),
        "PgPoolOptions" => Some(PersistenceTarget::PgPool),
        _ => None,
    })
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
                    | ("Mutex", "new")
                    | ("RwLock", "new")
                    | ("Cell", "new")
                    | ("RefCell", "new")
                    | ("UnsafeCell", "new")
                    | ("ManuallyDrop", "new")
                    | ("Pin", "new")
                    | ("Option", "Some")
                    | ("Result", "Ok")
                    // An error payload carries persistence out of a function
                    // exactly as a success payload does, and `?` returns it.
                    | ("Result", "Err")
                    | ("ControlFlow", "Break")
                    | ("ControlFlow", "Continue")
            )
    ) || matches!(names, [name] if matches!(name.as_str(), "Some" | "Ok" | "Err" | "Break" | "Continue"))
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

fn is_standard_replacement(names: &[String]) -> bool {
    matches!(
        names,
        [root, module, function]
            if matches!(root.as_str(), "std" | "core")
                && module == "mem"
                && matches!(function.as_str(), "replace" | "take")
    )
}

fn targets_for_names(names: &[String], symbols: &ModuleSymbols) -> TargetSet {
    let mut targets = TargetSet::new();
    let Some(first) = names.first() else {
        return targets;
    };
    let last = names.last().expect("non-empty path");
    if path_is_sqlx(names, symbols) {
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
                sql_sources: BTreeSet::new(),
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
                mutable_pointees: BTreeSet::new(),
                mutable_places: BTreeSet::new(),
                query_callable: false,
                executor_callable: None,
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
        sql_sources: BTreeSet::new(),
        nominal_types: resolve_nominal_types(receiver_nominal_types_in_type(ty), symbols),
        payload_variants: payload_variants_in_type(ty, symbols),
        tuple_items: tuple_items_in_type(ty, symbols),
        field_items: BTreeMap::new(),
        trait_bounds: trait_bounds_in_type(ty, symbols),
        type_generic_params: Vec::new(),
        callable_signatures: BTreeSet::new(),
        closure_mutations: BTreeMap::new(),
        mutable_pointees: BTreeSet::new(),
        mutable_places: BTreeSet::new(),
        query_callable: false,
        executor_callable: None,
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

fn path_is_sqlx(names: &[String], symbols: &ModuleSymbols) -> bool {
    if names
        .first()
        .is_some_and(|first| symbols.sqlx_namespaces.contains(first))
    {
        return true;
    }
    if symbols.workspace_sqlx_namespaces.is_empty() {
        return false;
    }
    let canonical = canonical_path_names(names.to_vec(), symbols);
    (1..=canonical.len()).any(|length| {
        symbols
            .workspace_sqlx_namespaces
            .contains(&canonical[..length].join("::"))
    })
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
    path_is_sqlx(source, symbols)
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
    for (item_index, item) in items.iter().enumerate() {
        match item {
            // A trait's default associated constant is the value every impl
            // inherits unless it overrides it, so it is pinned under the trait.
            Item::Trait(item_trait)
                if source_class_allows(source_class, cfg, &item_trait.attrs, errors, "trait") =>
            {
                for associated in &item_trait.items {
                    let TraitItem::Const(associated) = associated else {
                        continue;
                    };
                    // Each default carries its own cfg. Without this, mutually
                    // exclusive `cfg(test)` and `cfg(not(test))` values are both
                    // registered in both views, and a test-only statement — an
                    // advisory lock included — enters the production inventory.
                    if !source_class_allows(
                        source_class,
                        cfg,
                        &associated.attrs,
                        errors,
                        "trait associated const",
                    ) {
                        continue;
                    }
                    let Some((_, default)) = &associated.default else {
                        continue;
                    };
                    let (kind, sources) = source_sql_info(default, &|path| {
                        standard_string_macro_of(
                            path,
                            &|path| canonical_path_names(path, symbols),
                            &symbols.module_path,
                            &macro_shadows_before(items, item_index),
                        )
                    });
                    if sources.is_empty() {
                        continue;
                    }
                    let mut info = variable_info_in_type(&associated.ty, symbols);
                    info.sql_expression = kind;
                    info.sql_sources = sources;
                    let mut path = module_path.to_vec();
                    path.push(normalized_ident(&item_trait.ident));
                    path.push(normalized_ident(&associated.ident));
                    output.entry(path.join("::")).or_default().union(&info);
                }
            }
            // `impl Statements { const SQL: &str = "…" }` pins a statement
            // exactly like a module constant, and the call site can only see
            // `Statements::SQL` unless it is registered under that name.
            Item::Impl(item_impl)
                if source_class_allows(source_class, cfg, &item_impl.attrs, errors, "impl") =>
            {
                for associated in &item_impl.items {
                    let ImplItem::Const(associated) = associated else {
                        continue;
                    };
                    if !source_class_allows(
                        source_class,
                        cfg,
                        &associated.attrs,
                        errors,
                        "impl associated const",
                    ) {
                        continue;
                    }
                    let (kind, sources) = source_sql_info(&associated.expr, &|path| {
                        standard_string_macro_of(
                            path,
                            &|path| canonical_path_names(path, symbols),
                            &symbols.module_path,
                            &macro_shadows_before(items, item_index),
                        )
                    });
                    if sources.is_empty() {
                        continue;
                    }
                    let mut info = variable_info_in_type(&associated.ty, symbols);
                    info.sql_expression = kind;
                    info.sql_sources = sources;
                    // Two traits may each define `SQL` for the same type with
                    // different values, so the key records which impl the value
                    // belongs to. An inherent impl keeps the bare name.
                    let qualifier = item_impl.trait_.as_ref().map(|(_, path, _)| {
                        canonical_path_names(path_names(path), symbols).join("::")
                    });
                    // The impl may target a type of another module, so the key
                    // comes from the self type's own path rather than from
                    // where the impl happens to be written.
                    let owners: Vec<Vec<String>> = match item_impl.self_ty.as_ref() {
                        Type::Path(path) => {
                            vec![canonical_path_names(path_names(&path.path), symbols)]
                        }
                        other => nominal_types_in_type(other)
                            .into_iter()
                            .map(|owner| {
                                let mut path = module_path.to_vec();
                                path.push(owner);
                                path
                            })
                            .collect(),
                    };
                    for owner in owners {
                        let member = normalized_ident(&associated.ident);
                        let key = match &qualifier {
                            Some(trait_path) => {
                                format!("<{} as {trait_path}>::{member}", owner.join("::"))
                            }
                            None => {
                                let mut path = owner.clone();
                                path.push(member.clone());
                                path.join("::")
                            }
                        };
                        output.entry(key).or_default().union(&info);
                    }
                }
            }
            Item::Const(item_const)
                if source_class_allows(source_class, cfg, &item_const.attrs, errors, "const") =>
            {
                let mut path = module_path.to_vec();
                path.push(normalized_ident(&item_const.ident));
                let mut info = variable_info_in_type(&item_const.ty, symbols);
                let (kind, sources) = source_sql_info(&item_const.expr, &|path| {
                    standard_string_macro_of(
                        path,
                        &|path| canonical_path_names(path, symbols),
                        &symbols.module_path,
                        &macro_shadows_before(items, item_index),
                    )
                });
                info.sql_expression = kind;
                info.sql_sources = sources;
                output.entry(path.join("::")).or_default().union(&info);
            }
            Item::Static(item_static)
                if source_class_allows(source_class, cfg, &item_static.attrs, errors, "static") =>
            {
                let mut path = module_path.to_vec();
                path.push(normalized_ident(&item_static.ident));
                let mut info = variable_info_in_type(&item_static.ty, symbols);
                let (kind, sources) = source_sql_info(&item_static.expr, &|path| {
                    standard_string_macro_of(
                        path,
                        &|path| canonical_path_names(path, symbols),
                        &symbols.module_path,
                        &macro_shadows_before(items, item_index),
                    )
                });
                info.sql_expression = kind;
                info.sql_sources = sources;
                output.entry(path.join("::")).or_default().union(&info);
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
    // A child module does not see the parent's items under their bare names,
    // so these sets are rebuilt for each module's own scope.
    symbols.local_type_definitions.clear();
    symbols.local_macro_definitions.clear();
    symbols
        .local_type_definitions
        .extend(items.iter().filter_map(|item| match item {
            Item::Struct(item_struct) => Some(normalized_ident(&item_struct.ident)),
            Item::Enum(item_enum) => Some(normalized_ident(&item_enum.ident)),
            Item::Type(item_type) => Some(normalized_ident(&item_type.ident)),
            _ => None,
        }));
    // An import shadows a prelude name as surely as a declaration does, unless
    // it is the standard item itself.
    for item in items {
        let Item::Use(item_use) = item else {
            continue;
        };
        let (leaves, _) = use_leaves(item_use);
        for leaf in leaves {
            if leaf
                .source
                .first()
                .is_some_and(|root| matches!(root.as_str(), "std" | "alloc" | "core"))
            {
                continue;
            }
            symbols.local_type_definitions.insert(leaf.local);
        }
    }

    symbols.module_path = module
        .split("::")
        .filter(|segment| *segment != "crate")
        .map(str::to_owned)
        .collect();
    symbols.traits_in_scope.clear();
    symbols.anonymous_traits_in_scope.clear();
    for _ in 0..=items.len() {
        let mut changed = false;
        for (item_index, item) in items.iter().enumerate() {
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
                    let mut info = variable_info_in_type(&item_const.ty, &symbols);
                    let (kind, sources) = source_sql_info(&item_const.expr, &|path| {
                        standard_string_macro_of(
                            path,
                            &|path| canonical_path_names(path, &symbols),
                            &symbols.module_path,
                            &macro_shadows_before(items, item_index),
                        )
                    });
                    info.sql_expression = kind;
                    info.sql_sources = sources;
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
                    let mut info = variable_info_in_type(&item_static.ty, &symbols);
                    let (kind, sources) = source_sql_info(&item_static.expr, &|path| {
                        standard_string_macro_of(
                            path,
                            &|path| canonical_path_names(path, &symbols),
                            &symbols.module_path,
                            &macro_shadows_before(items, item_index),
                        )
                    });
                    info.sql_expression = kind;
                    info.sql_sources = sources;
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
                    symbols
                        .local_macro_definitions
                        .insert(normalized_ident(name));
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
                let called_inputs = called_parameter_inputs(function);
                if !called_inputs.is_empty() {
                    symbols
                        .function_called_inputs
                        .insert(normalized_ident(&function.sig.ident), called_inputs);
                }
                let mutable_writes = mutable_parameter_writes(function)
                    .into_iter()
                    .filter_map(|index| {
                        let FnArg::Typed(typed) = function.sig.inputs.iter().nth(index)? else {
                            return None;
                        };
                        Some((index, variable_info_in_type(&typed.ty, &symbols)))
                    })
                    .collect::<BTreeMap<_, _>>();
                if !mutable_writes.is_empty() {
                    symbols
                        .function_mutable_writes
                        .insert(normalized_ident(&function.sig.ident), mutable_writes);
                }
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
                    let method_name = normalized_ident(&method.sig.ident);
                    let (receiver_fields, parameter_writes) = mutable_method_writes(method);
                    for receiver_type in &receiver_types {
                        let key = (
                            receiver_type.clone(),
                            trait_name.clone(),
                            method_name.clone(),
                        );
                        let mut receiver_write_info = VariableInfo::default();
                        for field in &receiver_fields {
                            let mut field_write_info = VariableInfo::default();
                            if let Some(targets) = symbols
                                .field_targets
                                .get(&(receiver_type.clone(), field.clone()))
                            {
                                field_write_info.flow.union(Flow::pools(targets));
                            }
                            if let Some(types) = symbols
                                .field_nominal_types
                                .get(&(receiver_type.clone(), field.clone()))
                            {
                                field_write_info.nominal_types.extend(types.iter().cloned());
                            }
                            if field_write_info != VariableInfo::default() {
                                receiver_write_info
                                    .field_items
                                    .entry(field.clone())
                                    .or_default()
                                    .union(&field_write_info);
                            }
                        }
                        if receiver_write_info != VariableInfo::default() {
                            symbols
                                .method_mutable_receivers
                                .entry(key.clone())
                                .or_default()
                                .union(&receiver_write_info);
                        }
                        for index in &parameter_writes {
                            let Some(FnArg::Typed(typed)) = method.sig.inputs.iter().nth(index + 1)
                            else {
                                continue;
                            };
                            let parameter_info = flatten_reachable_variable_info(
                                &variable_info_in_type(&typed.ty, &symbols),
                            );
                            symbols
                                .method_mutable_writes
                                .entry(key.clone())
                                .or_default()
                                .entry(*index)
                                .or_default()
                                .union(&parameter_info);
                        }
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

fn control_flow_sql_kind(tokens: TokenStream) -> SqlExpressionKind {
    tokens
        .into_iter()
        .fold(SqlExpressionKind::Nonliteral, |kind, token| {
            let candidate = match token {
                TokenTree::Ident(ident) => match normalized_ident(&ident).as_str() {
                    "include_str" => SqlExpressionKind::Included,
                    "env" => SqlExpressionKind::Environment,
                    "format" | "format_args" => SqlExpressionKind::Interpolated,
                    _ => SqlExpressionKind::Nonliteral,
                },
                TokenTree::Group(group) => control_flow_sql_kind(group.stream()),
                TokenTree::Punct(_) | TokenTree::Literal(_) => SqlExpressionKind::Nonliteral,
            };
            kind.max(candidate)
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
    /// Trait of the impl whose body this is, canonicalised exactly as the
    /// registration keys it. `Self::CONST` in a trait impl names
    /// `<Owner as Trait>::CONST`, which the inherent key does not reach (#204).
    active_trait: Option<String>,
    generic_trait_bounds: BTreeMap<String, BTreeSet<String>>,
    generic_trait_bound_args: BTreeMap<(String, String), Vec<VariableInfo>>,
    generic_trait_bound_associated: BTreeMap<(String, String), BTreeMap<String, VariableInfo>>,
    flow_cache: std::cell::RefCell<BTreeMap<(usize, u64), Flow>>,
    subtree_flow_cache: std::cell::RefCell<BTreeMap<(usize, u64), Flow>>,
    closure_effects: std::cell::RefCell<BTreeMap<usize, BTreeMap<String, VariableInfo>>>,
    closure_result_infos: std::cell::RefCell<BTreeMap<usize, VariableInfo>>,
    block_result_infos: std::cell::RefCell<BTreeMap<usize, VariableInfo>>,
    replacement_result_infos: std::cell::RefCell<BTreeMap<usize, VariableInfo>>,
    loop_flow_collectors: Vec<LoopFlowCollector>,
    block_exit_collectors: Vec<BlockExitCollector>,
    return_exit_collectors: Vec<Option<Vec<BTreeMap<String, VariableInfo>>>>,
    return_value_collectors: Vec<VariableInfo>,
    context_version: u64,
    suppress_records: bool,
    /// Macro shadows declared above the item whose body this analyzes.
    visible_macro_shadows: BTreeSet<String>,
}

#[derive(Default)]
struct LoopFlowCollector {
    label: Option<String>,
    exits: Option<Vec<BTreeMap<String, VariableInfo>>>,
    back_edges: Option<Vec<BTreeMap<String, VariableInfo>>>,
}

#[derive(Default)]
struct BlockExitCollector {
    label: String,
    exits: Option<Vec<BTreeMap<String, VariableInfo>>>,
    result: VariableInfo,
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
            active_trait: None,
            generic_trait_bounds: BTreeMap::new(),
            generic_trait_bound_args: BTreeMap::new(),
            generic_trait_bound_associated: BTreeMap::new(),
            flow_cache: std::cell::RefCell::new(BTreeMap::new()),
            subtree_flow_cache: std::cell::RefCell::new(BTreeMap::new()),
            closure_effects: std::cell::RefCell::new(BTreeMap::new()),
            closure_result_infos: std::cell::RefCell::new(BTreeMap::new()),
            block_result_infos: std::cell::RefCell::new(BTreeMap::new()),
            replacement_result_infos: std::cell::RefCell::new(BTreeMap::new()),
            loop_flow_collectors: Vec::new(),
            block_exit_collectors: Vec::new(),
            return_exit_collectors: Vec::new(),
            return_value_collectors: Vec::new(),
            context_version: 0,
            suppress_records: false,
            visible_macro_shadows: BTreeSet::new(),
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
        self.register_local_constants(&expression.body.stmts);
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
        self.register_local_constants(&expression.body.stmts);
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
        let explicit_label = label.map(|label| normalized_ident(&label.ident));
        let target = explicit_label
            .as_ref()
            .and_then(|label| {
                self.loop_flow_collectors
                    .iter()
                    .rposition(|collector| collector.label.as_deref() == Some(label.as_str()))
            })
            .or_else(|| {
                explicit_label
                    .is_none()
                    .then(|| self.loop_flow_collectors.len().checked_sub(1))
                    .flatten()
            });
        if target.is_none()
            && is_exit
            && let Some(label) = explicit_label
            && let Some(collector) = self
                .block_exit_collectors
                .iter_mut()
                .rev()
                .find(|collector| collector.label == label)
        {
            let scopes = self.scopes.clone();
            match &mut collector.exits {
                None => collector.exits = Some(scopes),
                Some(accumulated) => merge_scope_stacks(accumulated, &scopes),
            }
            return;
        }
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

    fn capture_return_exit(&mut self) {
        if let Some(exits) = self.return_exit_collectors.last_mut() {
            let scopes = self.scopes.clone();
            match exits {
                None => *exits = Some(scopes),
                Some(accumulated) => merge_scope_stacks(accumulated, &scopes),
            }
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

    /// Register the block's imports, then let them resolve through each other.
    ///
    /// Rust does not order imports, so `use mount as alias;` may precede
    /// `use std::include as mount;`. Resolving once would leave `alias`
    /// pointing at a name that is itself an alias, and a guard reading the
    /// resolved path would miss what the invocation really is.
    fn register_local_uses(&mut self, statements: &[Stmt]) {
        self.register_local_use_items(statements);
        // Chain the block's own aliases through each other. Only local entries
        // are substituted: rewriting them with module resolution would change
        // the written form that other lookups depend on.
        for _ in 0..8 {
            let Some(scope) = self.local_path_alias_scopes.last().cloned() else {
                break;
            };
            let expanded = scope
                .iter()
                .map(|(alias, source)| {
                    let mut names = source.clone();
                    if let Some(first) = names.first().cloned()
                        && first != *alias
                        && let Some(target) = scope.get(&first)
                        && target.first() != Some(&first)
                    {
                        names.splice(0..1, target.clone());
                    }
                    (alias.clone(), names)
                })
                .collect::<BTreeMap<_, _>>();
            if expanded == scope {
                break;
            }
            if let Some(scope) = self.local_path_alias_scopes.last_mut() {
                *scope = expanded;
            }
        }
    }

    fn register_local_use_items(&mut self, statements: &[Stmt]) {
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

    /// Bind a block's `const`/`static` items before its statements run.
    ///
    /// They are in scope throughout the block, so a query written above the
    /// declaration still reads the value; installing it only when the visitor
    /// reaches the declaration would record that query against an opaque path
    /// and let the literal change without moving a row.
    fn register_local_constants(&mut self, statements: &[Stmt]) {
        let items = statements
            .iter()
            .filter_map(|statement| match statement {
                Stmt::Item(item) => Some(item),
                _ => None,
            })
            .collect::<Vec<_>>();
        for (index, item) in items.iter().enumerate() {
            if !self.allows_source_class(item_attributes(item), "block-local item") {
                continue;
            }
            let shadows = items[..index]
                .iter()
                .filter_map(|item| match item {
                    Item::Macro(item_macro) => item_macro.ident.as_ref().map(normalized_ident),
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            let declared = match item {
                Item::Const(item_const) => Some((
                    normalized_ident(&item_const.ident),
                    &item_const.expr,
                    &item_const.ty,
                )),
                Item::Static(item_static) => Some((
                    normalized_ident(&item_static.ident),
                    &item_static.expr,
                    &item_static.ty,
                )),
                _ => None,
            };
            let Some((name, expression, declared_type)) = declared else {
                continue;
            };
            let (kind, sources) = source_sql_info(expression, &|path| {
                standard_string_macro_of(
                    path,
                    &|path| self.canonical_names(path),
                    &self.symbols.module_path,
                    &shadows,
                )
            });
            if kind == SqlExpressionKind::Nonliteral {
                continue;
            }
            let mut info = self.info_from_type(declared_type);
            info.sql_expression = kind;
            info.sql_sources = sources;
            self.bind(name, info);
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

    fn method_mutation_effects(
        &self,
        method: &ExprMethodCall,
    ) -> (VariableInfo, BTreeMap<usize, VariableInfo>) {
        let method_name = normalized_ident(&method.method);
        let receiver_types = self.nominal_types_of_expr(&method.receiver);
        let mut receiver_effect = VariableInfo::default();
        let mut parameter_effects = BTreeMap::<usize, VariableInfo>::new();
        for receiver_type in receiver_types {
            for ((owner, trait_name, candidate), info) in &self.symbols.method_mutable_receivers {
                if owner == &receiver_type
                    && candidate == &method_name
                    && trait_name
                        .as_ref()
                        .is_none_or(|trait_name| self.trait_is_in_scope(trait_name))
                {
                    receiver_effect.union(info);
                }
            }
            for ((owner, trait_name, candidate), effects) in &self.symbols.method_mutable_writes {
                if owner == &receiver_type
                    && candidate == &method_name
                    && trait_name
                        .as_ref()
                        .is_none_or(|trait_name| self.trait_is_in_scope(trait_name))
                {
                    for (index, info) in effects {
                        parameter_effects.entry(*index).or_default().union(info);
                    }
                }
            }
            let owner_key = if receiver_type.contains("::") {
                receiver_type
                    .strip_prefix("crate::")
                    .unwrap_or(&receiver_type)
                    .to_owned()
            } else {
                self.package_function_key(vec![receiver_type])
            };
            if let Some(info) = self
                .symbols
                .package_method_mutable_receivers
                .get(&(owner_key.clone(), method_name.clone()))
            {
                receiver_effect.union(info);
            }
            if let Some(effects) = self
                .symbols
                .package_method_mutable_writes
                .get(&(owner_key, method_name.clone()))
            {
                for (index, info) in effects {
                    parameter_effects.entry(*index).or_default().union(info);
                }
            }
        }
        (receiver_effect, parameter_effects)
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
            // The owner is the type the path resolves to, not the name it is
            // written with: `use sqlx::mysql::MySqlPoolOptions as Opt` still
            // builds a MySQL pool.
            let written = path_names(path);
            let owner = self
                .canonical_names(written[..written.len().saturating_sub(1)].to_vec())
                .last()
                .cloned()
                .unwrap_or_default();
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
            Expr::Reference(reference) => {
                let mut info = self.info_from_expr(&reference.expr);
                if reference.mutability.is_some()
                    && let Some(name) = simple_assignment_name(&reference.expr)
                {
                    info.mutable_pointees.insert(name);
                } else if reference.mutability.is_some()
                    && let Some((root, projections)) = assignment_place(&reference.expr)
                {
                    info.mutable_places
                        .insert(MutablePlace { root, projections });
                }
                return info;
            }
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
            let names = path_names(&path.path);
            if path_is_sqlx(&names, self.symbols)
                && names.last().is_some_and(|name| is_query_name(name))
            {
                return VariableInfo {
                    flow: Flow::query(),
                    query_callable: true,
                    ..VariableInfo::default()
                };
            }
            // `let run = sqlx::Executor::execute;` names an executor; the call
            // through `run` sends whatever SQL it is handed.
            // The trait may be imported inside this block, so the path has to
            // be resolved before it can be recognized.
            let canonical = self.canonical_names(names.clone());
            if (path_is_sqlx(&names, self.symbols) || path_is_sqlx(&canonical, self.symbols))
                && canonical
                    .iter()
                    .nth_back(1)
                    .is_some_and(|owner| owner == "Executor")
                && let Some(method) = canonical.last()
                && PersistenceOperation::from_executor_method(method).is_some()
            {
                return VariableInfo {
                    executor_callable: Some(method.clone()),
                    ..VariableInfo::default()
                };
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
            if let Some(info) = self
                .replacement_result_infos
                .borrow()
                .get(&(call as *const ExprCall as usize))
                .cloned()
            {
                return info;
            }
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
                    return self.preserve_opaque_argument_info(
                        self.apply_inferred_args(
                            &result,
                            params,
                            self.symbols.function_generic_input_params.get(&name),
                            &call.args,
                        ),
                        &call.args,
                    );
                }
                // Free functions declared in another source module resolve
                // through the package-wide canonical-path registry.
                let key = self.package_function_key(vec![name]);
                if let Some(info) = self.symbols.package_function_returns.get(&key) {
                    let params = self.symbols.package_function_generic_params.get(&key);
                    let result = self.apply_turbofish_args(info, params, turbofish);
                    return self.preserve_opaque_argument_info(
                        self.apply_inferred_args(
                            &result,
                            params,
                            self.symbols.package_function_generic_input_params.get(&key),
                            &call.args,
                        ),
                        &call.args,
                    );
                }
            }
            let return_info = self.preserve_opaque_argument_info(
                self.associated_return_info(path, Some(&call.args)),
                &call.args,
            );
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
                sql_sources: self.sql_sources(expression),
                nominal_types: return_info.nominal_types,
                payload_variants: return_info.payload_variants,
                tuple_items: return_info.tuple_items,
                field_items: return_info.field_items,
                trait_bounds: return_info.trait_bounds,
                type_generic_params: Vec::new(),
                callable_signatures: BTreeSet::new(),
                closure_mutations: BTreeMap::new(),
                mutable_pointees: BTreeSet::new(),
                mutable_places: BTreeSet::new(),
                query_callable: false,
                executor_callable: None,
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
                sql_sources: self.sql_sources(expression),
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
                sql_sources: self.sql_sources(expression),
                nominal_types,
                payload_variants: payload_variants_in_path(&path.path, self.symbols),
                tuple_items: Vec::new(),
                field_items: BTreeMap::new(),
                trait_bounds: BTreeSet::new(),
                type_generic_params: Vec::new(),
                callable_signatures: BTreeSet::new(),
                closure_mutations: BTreeMap::new(),
                mutable_pointees: BTreeSet::new(),
                mutable_places: BTreeSet::new(),
                query_callable: false,
                executor_callable: None,
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
            sql_sources: self.sql_sources(expression),
            nominal_types: BTreeSet::new(),
            payload_variants: BTreeSet::new(),
            tuple_items: Vec::new(),
            field_items: BTreeMap::new(),
            trait_bounds: BTreeSet::new(),
            type_generic_params: Vec::new(),
            callable_signatures: BTreeSet::new(),
            closure_mutations: BTreeMap::new(),
            mutable_pointees: BTreeSet::new(),
            mutable_places: BTreeSet::new(),
            query_callable: false,
            executor_callable: None,
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

    fn union_into_declared_projections(&self, aggregate: &mut VariableInfo, value: &VariableInfo) {
        for owner in aggregate.nominal_types.clone() {
            let suffix = format!("::{owner}");
            for (key, declared) in self
                .symbols
                .named_type_info
                .iter()
                .chain(self.symbols.workspace_named_type_info.iter())
            {
                if key != &owner && !key.ends_with(&suffix) {
                    continue;
                }
                if aggregate.tuple_items.len() < declared.tuple_items.len() {
                    aggregate
                        .tuple_items
                        .resize_with(declared.tuple_items.len(), VariableInfo::default);
                }
                for item in &mut aggregate.tuple_items {
                    item.union(value);
                }
                for field in declared.field_items.keys() {
                    aggregate
                        .field_items
                        .entry(field.clone())
                        .or_default()
                        .union(value);
                }
            }
        }
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
                typed_info
                    .sql_sources
                    .extend(info.sql_sources.iter().cloned());
                self.bind_pattern(&typed.pat, &typed_info);
            }
            Pat::Tuple(tuple) => {
                for (index, element) in tuple.elems.iter().enumerate() {
                    if matches!(element, Pat::Rest(_)) {
                        continue;
                    }
                    let source_index = tuple
                        .elems
                        .iter()
                        .position(|pattern| matches!(pattern, Pat::Rest(_)))
                        .filter(|rest| index > *rest)
                        .and_then(|_| {
                            info.tuple_items
                                .len()
                                .checked_sub(tuple.elems.len().saturating_sub(index))
                        })
                        .unwrap_or(index);
                    self.bind_pattern(element, info.tuple_items.get(source_index).unwrap_or(info));
                }
            }
            Pat::TupleStruct(tuple) => {
                let owner = self.pattern_owner(&tuple.path);
                for (index, element) in tuple.elems.iter().enumerate() {
                    if matches!(element, Pat::Rest(_)) {
                        continue;
                    }
                    let source_index = tuple
                        .elems
                        .iter()
                        .position(|pattern| matches!(pattern, Pat::Rest(_)))
                        .filter(|rest| index > *rest)
                        .and_then(|_| {
                            let declared_len = self
                                .symbols
                                .named_type_info
                                .iter()
                                .filter(|(key, _)| {
                                    *key == &owner || key.ends_with(&format!("::{owner}"))
                                })
                                .map(|(_, info)| info.tuple_items.len())
                                .max()
                                .into_iter()
                                .chain(
                                    self.symbols
                                        .tuple_field_targets
                                        .keys()
                                        .chain(self.symbols.field_nominal_types.keys())
                                        .filter(|(candidate, _)| candidate == &owner)
                                        .filter_map(|(_, field)| field.parse::<usize>().ok())
                                        .map(|index| index + 1),
                                )
                                .max()
                                .unwrap_or(tuple.elems.len());
                            declared_len.checked_sub(tuple.elems.len().saturating_sub(index))
                        })
                        .unwrap_or(index);
                    if self.has_declared_fields(&owner) {
                        let field_info =
                            self.declared_field_info(&owner, &source_index.to_string(), false);
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
                    info.sql_sources = self.sql_sources(expression);
                    self.bind_pattern(pattern, &info);
                }
            }
            _ => {
                let mut info = self.info_from_expr(expression);
                info.sql_expression = self.sql_expression_kind(expression);
                info.sql_sources = self.sql_sources(expression);
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
            // Control flow is classified from its own syntax: whether a branch
            // interpolates is visible without evaluating which branch runs.
            Expr::Block(_) | Expr::If(_) | Expr::Match(_) => {
                control_flow_sql_kind(expression.to_token_stream())
            }
            Expr::Path(path) => {
                // A pinned constant keeps its kind however it is named: a local
                // binding, a module item, an imported name, or a qualified
                // path. Reading only body scopes reported it as
                // runtime-assembled and forced a needless workflow
                // classification.
                self.pinned_path_info(path)
                    .map(|info| info.sql_expression)
                    .unwrap_or(SqlExpressionKind::Nonliteral)
            }
            Expr::Call(call)
                if matches!(call.func.as_ref(), Expr::Path(path)
                    if path.path.segments.iter().rev().take(2).map(|segment| normalized_ident(&segment.ident)).collect::<Vec<_>>()
                        == ["from", "String"]) =>
            {
                call.args
                    .first()
                    .map(|argument| self.sql_expression_kind(argument))
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
                    .standard_string_macro(path_names(&mac.mac.path))
                    .is_some_and(|name| name == "concat") =>
            {
                syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
                    .parse2(mac.mac.tokens.clone())
                    .ok()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .fold(SqlExpressionKind::Static, |kind, argument| {
                                kind.max(self.sql_expression_kind(argument))
                            })
                    })
                    .unwrap_or(SqlExpressionKind::Nonliteral)
            }
            Expr::Macro(mac)
                if self
                    .standard_string_macro(path_names(&mac.mac.path))
                    .is_some_and(|name| name == "stringify") =>
            {
                SqlExpressionKind::Static
            }
            Expr::MethodCall(method)
                if matches!(
                    normalized_ident(&method.method).as_str(),
                    "as_str" | "as_ref" | "to_owned" | "to_string"
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

    /// The SQL a query argument pins, when a single local rule can prove it.
    ///
    /// This grammar reads pinned statements; it does not evaluate what a Rust
    /// expression would build. A literal, a `concat!`, and a name bound to one
    /// of those are provable here. Anything assembled at run time — a `+`
    /// chain, a `format!` template, a branch, a helper's return, a projection —
    /// yields nothing, and the call site is then ratcheted as interpolated or
    /// nonliteral SQL whose connection affinity is a reviewed workflow
    /// annotation rather than a guess made from token text.
    /// The value a path names, wherever it is declared: a local binding, an
    /// item of this module, an imported name, or a qualified path. An imported
    /// or qualified name is resolved through the canonical package registry,
    /// which is where a constant declared in another module is recorded.
    fn pinned_path_info(&self, path: &syn::ExprPath) -> Option<&VariableInfo> {
        // `<Statements as Sql>::SQL` names the same constant as
        // `Statements::SQL`; the qualification says which impl, not which value.
        // `Self::SQL` inside an impl names that impl's type.
        let names = path_names(&path.path);
        if path.qself.is_none()
            && names.len() == 2
            && names[0] == "Self"
            && let Some(info) = self.lookup("Self")
        {
            let member = names[1].clone();
            // Resolution order matches Rust's: this impl's override, then the
            // trait's default, then an inherent constant of the same name.
            if let Some(active_trait) = &self.active_trait
                && let Some(found) = info.nominal_types.iter().find_map(|owner| {
                    let owner = self.package_function_key(vec![owner.clone()]);
                    self.symbols
                        .package_item_values
                        .get(&format!("<{owner} as {active_trait}>::{member}"))
                })
            {
                return Some(found);
            }
            if let Some(active_trait) = &self.active_trait
                && let Some(found) = self
                    .symbols
                    .package_item_values
                    .get(&format!("{active_trait}::{member}"))
            {
                return Some(found);
            }
            if let Some(found) = info.nominal_types.iter().find_map(|owner| {
                let owner = self.package_function_key(vec![owner.clone()]);
                self.symbols
                    .package_item_values
                    .get(&format!("{owner}::{member}"))
            }) {
                return Some(found);
            }
        }
        if let Some(qself) = &path.qself {
            // `<Statements as Sql>::SQL` names the value that `Sql` defines,
            // which another trait's impl for the same type may not share.
            let member = last_path_name(&path.path)?;
            let trait_path = (path.path.segments.len() > 1).then(|| {
                let names = path_names(&path.path);
                self.canonical_names(names[..names.len() - 1].to_vec())
                    .join("::")
            });
            return nominal_types_in_type(&qself.ty)
                .into_iter()
                .chain(receiver_nominal_types_in_type(&qself.ty))
                .find_map(|owner| {
                    let owner = self.package_function_key(vec![owner]);
                    let key = match &trait_path {
                        Some(trait_path) => format!("<{owner} as {trait_path}>::{member}"),
                        None => format!("{owner}::{member}"),
                    };
                    self.symbols
                        .package_item_values
                        .get(&key)
                        // An impl that does not override the constant inherits
                        // the trait's default.
                        .or_else(|| {
                            trait_path.as_ref().and_then(|trait_path| {
                                self.symbols
                                    .package_item_values
                                    .get(&format!("{trait_path}::{member}"))
                            })
                        })
                });
        }
        if names.len() == 1
            && let Some(name) = names.first()
            && let Some(info) = self
                .lookup(name)
                .or_else(|| self.symbols.item_values.get(name))
        {
            return Some(info);
        }
        self.symbols
            .package_item_values
            .get(&self.package_function_key(names))
    }

    /// Whether the statement `text` carries takes an advisory lock, reading it
    /// with this scope's knowledge.
    ///
    /// A compile-time string macro can be invoked under an alias
    /// (`use std::concat as c`), and the reader below recognizes only the macro
    /// it actually is. Rename them first so a pinned statement is not split
    /// into unrelated pieces.
    /// Whether the statement `fingerprint` carries takes an advisory lock,
    /// read with this scope's imports in hand.
    ///
    /// A compile-time string macro can be invoked under an alias
    /// (`use std::concat as c`), and only an *unqualified* name is the alias:
    /// `other::c!` is a different macro that happens to share a leaf name.
    /// The standard compile-time string macro a path names, if it names one.
    ///
    /// Only shadows declared above this body are in scope for it, so the set is
    /// the one recorded where the enclosing item was reached.
    fn standard_string_macro(&self, written: Vec<String>) -> Option<String> {
        standard_string_macro_of(
            written,
            &|path| self.canonical_names(path),
            &self.symbols.module_path,
            &self.visible_macro_shadows,
        )
    }

    fn statement_takes_advisory_lock(&self, fingerprint: &str) -> bool {
        sql_is_advisory_lock(fingerprint, &|path: Vec<String>| {
            self.standard_string_macro(path)
        })
    }

    /// The path a name really refers to: a block-local `use` alias first, then
    /// the module's own imports.
    fn canonical_names(&self, names: Vec<String>) -> Vec<String> {
        canonical_path_names(self.canonical_local_path_names(names), self.symbols)
    }

    fn sql_sources(&self, expression: &Expr) -> BTreeSet<String> {
        match expression {
            Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Str(_)) => {
                BTreeSet::from([normalized_tokens(expression)])
            }
            Expr::Reference(reference) => self.sql_sources(&reference.expr),
            Expr::Paren(paren) => self.sql_sources(&paren.expr),
            Expr::Group(group) => self.sql_sources(&group.expr),
            Expr::Path(path) => self
                .pinned_path_info(path)
                .map(|info| info.sql_sources.clone())
                .unwrap_or_default(),
            Expr::Call(call)
                if matches!(call.func.as_ref(), Expr::Path(path)
                    if path.path.segments.iter().rev().take(2).map(|segment| normalized_ident(&segment.ident)).collect::<Vec<_>>()
                        == ["from", "String"]) =>
            {
                call.args
                    .first()
                    .map(|argument| self.sql_sources(argument))
                    .unwrap_or_default()
            }
            // A `QueryBuilder` opens with the first fragment of one statement,
            // and the rest is pushed onto it.
            Expr::Call(call)
                if matches!(call.func.as_ref(), Expr::Path(path)
                    if path.path.segments.iter().rev().take(2).map(|segment| normalized_ident(&segment.ident)).collect::<Vec<_>>()
                        == ["new", "QueryBuilder"]) =>
            {
                call.args
                    .first()
                    .map(|argument| self.sql_sources(argument))
                    .unwrap_or_default()
            }
            // `concat!` and `stringify!` are pinned by their own tokens, which
            // already carry their arguments in order.
            Expr::Macro(mac)
                if self
                    .standard_string_macro(path_names(&mac.mac.path))
                    .is_some() =>
            {
                BTreeSet::from([normalized_tokens(expression)])
            }
            Expr::MethodCall(method)
                if matches!(
                    normalized_ident(&method.method).as_str(),
                    "as_str" | "as_ref" | "to_owned" | "to_string"
                ) =>
            {
                self.sql_sources(&method.receiver)
            }
            _ => BTreeSet::new(),
        }
    }

    fn fingerprint_with_sql_source(&self, base: String, argument: Option<&Expr>) -> String {
        let Some(argument) = argument else {
            return base;
        };
        fn is_indirect_with(
            expression: &Expr,
            resolve_path: &dyn Fn(Vec<String>) -> Vec<String>,
            local_types: &BTreeSet<String>,
        ) -> bool {
            let is_indirect =
                |expression: &Expr| is_indirect_with(expression, resolve_path, local_types);
            match expression {
                Expr::Path(_) => true,
                Expr::Reference(reference) => is_indirect(&reference.expr),
                Expr::Paren(paren) => is_indirect(&paren.expr),
                Expr::Group(group) => is_indirect(&group.expr),
                // `String::from(SQL)` is the same conversion in call form —
                // but only the standard one. A local type named `String` can
                // return anything.
                Expr::Call(call)
                    if matches!(call.func.as_ref(), Expr::Path(path)
                    if is_standard_string_conversion(
                        &path.path,
                        resolve_path,
                        local_types,
                    )) =>
                {
                    call.args.first().is_some_and(is_indirect)
                }
                // The same conversions `sql_sources` follows: a pinned source
                // is no less pinned for having been converted on the way in.
                Expr::MethodCall(method)
                    if matches!(
                        normalized_ident(&method.method).as_str(),
                        "as_str" | "as_ref" | "to_owned" | "to_string"
                    ) =>
                {
                    is_indirect(&method.receiver)
                }
                _ => false,
            }
        }
        let resolve_path = |path: Vec<String>| self.canonical_names(path);
        if !is_indirect_with(
            argument,
            &resolve_path,
            &self.symbols.local_type_definitions,
        ) {
            return base;
        }
        let sources = self.sql_sources(argument);
        if sources.is_empty() {
            base
        } else {
            format!(
                "{base}|sql-source:{}",
                sources.into_iter().collect::<Vec<_>>().join("|")
            )
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

    /// The flow a path names. An imported alias is resolved first, so
    /// `use sqlx::mysql::MySqlPoolOptions as Opt` keeps the provider that
    /// `Opt::new()` builds.
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
        Flow::pools(&targets_for_names(
            &self.canonical_names(path_names(&path.path)),
            self.symbols,
        ))
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
        // The result of an aliased constructor carries the same flow as the
        // constructor it names, or a chained `execute` loses its query stage.
        let canonical = self.canonical_names(names.clone());
        let last = canonical.last().map(String::as_str).unwrap_or_default();
        if is_standard_replacement(&self.canonical_local_path_names(names.clone()))
            && let Some(binding) = call.args.first().and_then(mutable_storage_receiver_name)
        {
            if let Some(info) = self
                .replacement_result_infos
                .borrow()
                .get(&(call as *const ExprCall as usize))
            {
                return info.flow.clone();
            }
            return self
                .lookup(&binding)
                .map(|info| info.flow.clone())
                .unwrap_or_default();
        }
        let rooted_sqlx =
            path_is_sqlx(&names, self.symbols) || path_is_sqlx(&canonical, self.symbols);
        if rooted_sqlx
            && last == "new"
            && let Some(target) = sqlx_pool_options_target(&canonical)
        {
            return Flow::pools(&BTreeSet::from([target]));
        }
        if (rooted_sqlx && is_query_name(last))
            || (names.len() == 1 && self.symbols.query_callables.contains(last))
            || (names.len() == 1 && self.lookup(last).is_some_and(|info| info.query_callable))
        {
            return Flow::query();
        }
        if rooted_sqlx
            && last == "new"
            && canonical
                .iter()
                .nth_back(1)
                .is_some_and(|owner| owner == "QueryBuilder")
        {
            return Flow::query();
        }
        // UFCS `begin` opens a transaction on its receiver; falling through to
        // the generic path targets would return a pool stage and the later
        // `commit` would no longer be recognized.
        if rooted_sqlx
            && last == "begin"
            && let Some(receiver) = call.args.first()
        {
            let flow = self.flow_of_expr(receiver);
            let opened = flow.map_pool_stage(FlowStage::Transaction);
            if !opened.is_empty() {
                return opened;
            }
            // A connection carries no pool stage, yet `begin` opens a
            // transaction on it just the same.
            let opened_on_targets = flow
                .targets()
                .iter()
                .map(|target| (*target, FlowStage::Transaction))
                .collect::<BTreeSet<_>>();
            if !opened_on_targets.is_empty() {
                return Flow(opened_on_targets);
            }
            return Flow(BTreeSet::from([(
                PersistenceTarget::Sqlx,
                FlowStage::Transaction,
            )]));
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
        // `use sqlx::mysql::MySqlPoolOptions as Opt` must not erase which
        // provider `Opt::new()` builds, or the pool the chain opens loses its
        // concrete identity.
        let path_targets = targets_for_names(&canonical, self.symbols);
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
                    .preserve_opaque_argument_info(
                        self.apply_inferred_args(
                            &result,
                            params,
                            self.symbols.function_generic_input_params.get(last),
                            &call.args,
                        ),
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
                    .preserve_opaque_argument_info(
                        self.apply_inferred_args(
                            &result,
                            params,
                            self.symbols.package_function_generic_input_params.get(&key),
                            &call.args,
                        ),
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
            // `QueryBuilder::build*` hands back the query it has assembled, so
            // what is chained onto it still executes SQL.
            "build" | "build_query_as" | "build_query_scalar"
                if receiver.targets().contains(&PersistenceTarget::Sqlx) =>
            {
                Flow::query()
            }
            "acquire" | "pool" => receiver.map_pool_stage(FlowStage::DerivedPool),
            "prepare" | "describe" => receiver.map_pool_stage(FlowStage::Query),
            "query" | "direct_query" | "delay_query_holder_like_cpp" => {
                receiver.map_pool_stage(FlowStage::Query)
            }
            "execute" | "direct_execute" => receiver.map_pool_stage(FlowStage::Query),
            "bind" if receiver.has_stage(FlowStage::Query) => receiver,
            // A modifier returns the query it was called on, so the executor
            // chained after it is still executing that query.
            "persistent" | "fetch_last_insert_id" | "try_map" | "map"
                if receiver.has_stage(FlowStage::Query) =>
            {
                receiver
            }
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
        // The result of an aliased constructor carries the same flow as the
        // constructor it names, or a chained `execute` loses its query stage.
        let canonical = self.canonical_names(names.clone());
        let last = canonical.last().map(String::as_str).unwrap_or_default();
        let rooted_sqlx =
            path_is_sqlx(&names, self.symbols) || path_is_sqlx(&canonical, self.symbols);
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

    fn record_persistence_escape(
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
        // An alias — module-level or block-local — hides which macro is being
        // invoked, and every decision below depends on knowing that: whether
        // this is a query macro at all, which argument carries the statement,
        // and whether the referenced SQL lives outside the snapshot.
        let canonical_names = self.canonical_names(names.clone());
        let canonical_name = canonical_names.last().cloned().unwrap_or_default();
        let rooted_sqlx =
            path_is_sqlx(&names, self.symbols) || path_is_sqlx(&canonical_names, self.symbols);
        let imported_query = names.len() == 1
            && (self.symbols.query_callables.contains(&name)
                || self.symbols.query_callables.contains(&canonical_name));
        let cfg = item_cfg(&self.cfg, attributes);
        if canonical_name == "include" && !is_pinned_wow_proto_include(&self.context, mac) {
            self.errors.push(format!(
                "{} contains include! whose Rust source is outside the persistence AST inventory; mount and parse the included source explicitly",
                self.enclosing
            ));
            return;
        }
        if matches!(name.as_str(), "write" | "writeln")
            && let Ok(expressions) =
                syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
                    .parse2(mac.tokens.clone())
            && let Some(place) = expressions.first().and_then(mutable_storage_place)
        {
            let current = self.info_from_expr(place);
            if !current.sql_sources.is_empty() {
                let mut sql_sources = current.sql_sources.clone();
                sql_sources.insert(normalized_tokens(mac));
                let mut sql_expression = current.sql_expression;
                for argument in expressions.iter().skip(1) {
                    sql_expression = sql_expression.max(self.sql_expression_kind(argument));
                    sql_sources.extend(self.sql_sources(argument));
                }
                union_into_assignment_place(
                    self,
                    place,
                    &VariableInfo {
                        sql_expression,
                        sql_sources,
                        ..VariableInfo::default()
                    },
                );
                return;
            }
        }
        if (rooted_sqlx && is_query_name(&canonical_name)) || imported_query {
            // Every `query_file*` variant, `_unchecked` included, reads SQL
            // this inventory cannot see.
            if canonical_name.starts_with("query_file") {
                // Name the macro that is actually invoked: under an alias,
                // reporting `q!` alone leaves the reader to guess which SQLx
                // macro put the SQL outside the snapshot.
                let invoked = (canonical_name != name)
                    .then(|| format!("{name}! ({canonical_name}!)"))
                    .unwrap_or_else(|| format!("{canonical_name}!"));
                self.errors.push(format!(
                    "{} uses {invoked} SQL whose referenced file is outside the persistence snapshot; mount and fingerprint the SQL file explicitly",
                    self.enclosing
                ));
                return;
            }
            let fingerprint = normalized_tokens(mac);
            self.add_generated(
                PersistenceTarget::Sqlx,
                PersistenceOperation::Query,
                &name,
                &cfg,
                fingerprint.clone(),
            );
            // Only one argument of a query macro is the statement; the rest are
            // bound values. `query!("SELECT ?", "GET_LOCK('x', 0)")` executes no
            // lock, so classifying the whole invocation would invent one.
            if query_macro_statement(&canonical_name, &mac.tokens)
                .is_some_and(|statement| self.statement_takes_advisory_lock(&statement))
            {
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
        // `migrate!` runs every `.sql` file of a directory this inventory does
        // not read, so a baselined invocation would let their contents — and
        // the statements they execute — change with no row moving.
        if rooted_sqlx && canonical_name == "migrate" {
            self.errors.push(format!(
                "{} runs migrate! SQL whose migration directory is outside the persistence snapshot; mount and fingerprint the migrations explicitly",
                self.enclosing
            ));
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
            let mut fingerprint = normalized_tokens(mac);
            let mut sql_sources = BTreeSet::new();
            let mut sql_kind = SqlExpressionKind::Static;
            if let Ok(expressions) =
                syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
                    .parse2(mac.tokens.clone())
            {
                for expression in expressions {
                    let sources = self.sql_sources(&expression);
                    if !sources.is_empty() {
                        sql_kind = sql_kind.max(self.sql_expression_kind(&expression));
                        sql_sources.extend(sources);
                    }
                }
            }
            if !sql_sources.is_empty() {
                fingerprint = format!(
                    "{fingerprint}|sql-source:{}",
                    sql_sources.into_iter().collect::<Vec<_>>().join("|")
                );
            }
            for target in targets {
                self.add(
                    target,
                    PersistenceOperation::MacroReference,
                    &name,
                    &cfg,
                    fingerprint.clone(),
                );
                if self.statement_takes_advisory_lock(&fingerprint) {
                    self.add(
                        target,
                        PersistenceOperation::AdvisoryLock,
                        &name,
                        &cfg,
                        fingerprint.clone(),
                    );
                }
                if matches!(sql_kind, SqlExpressionKind::Interpolated) {
                    self.add(
                        target,
                        PersistenceOperation::InterpolatedSql,
                        &name,
                        &cfg,
                        fingerprint.clone(),
                    );
                }
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
                if self.statement_takes_advisory_lock(&call_fingerprint) {
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
                if target == PersistenceTarget::Sqlx
                    && self.statement_takes_advisory_lock(&fingerprint)
                {
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
        inferred_return_with_unresolved_fallback(info, params, &substitutions, || {
            args.iter()
                .fold(VariableInfo::default(), |mut result, argument| {
                    result.union(&self.info_from_expr(argument));
                    result
                })
        })
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
        inferred_return_with_unresolved_fallback(info, params, &substitutions, || {
            args.iter()
                .fold(VariableInfo::default(), |mut result, argument| {
                    result.union(&self.info_from_expr(argument));
                    result
                })
        })
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

    fn preserve_opaque_argument_info(
        &self,
        mut result: VariableInfo,
        arguments: &syn::punctuated::Punctuated<Expr, syn::token::Comma>,
    ) -> VariableInfo {
        if result.flow.is_empty() && !result.trait_bounds.is_empty() {
            for argument in arguments {
                result.union(&self.info_from_expr(argument));
            }
        }
        result
    }

    fn is_unresolved_opaque_call(&self, expression: &Expr) -> bool {
        let expression = match expression {
            Expr::Await(value) => value.base.as_ref(),
            Expr::Try(value) => value.expr.as_ref(),
            Expr::Paren(value) => value.expr.as_ref(),
            Expr::Group(value) => value.expr.as_ref(),
            _ => expression,
        };
        let Expr::Call(call) = expression else {
            return false;
        };
        let Expr::Path(path) = call.func.as_ref() else {
            return false;
        };
        let names = path_names(&path.path);
        let local = names
            .last()
            .and_then(|name| self.symbols.function_returns.get(name));
        let package = self
            .symbols
            .package_function_returns
            .get(&self.package_function_key(names));
        local.or(package).is_some_and(|info| {
            info.flow.is_empty() && !info.trait_bounds.is_empty() && call.args.is_empty()
        })
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

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum PlaceProjection {
    Field(String),
    Index(Option<usize>),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct MutablePlace {
    root: String,
    projections: Vec<PlaceProjection>,
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

fn union_into_assignment_place(
    analyzer: &mut BodyAnalyzer<'_, '_>,
    place: &Expr,
    info: &VariableInfo,
) -> bool {
    if let Some(root) = simple_assignment_name(place) {
        let mut aggregate = analyzer.lookup(&root).cloned().unwrap_or_default();
        aggregate.union(info);
        analyzer.assign(&root, aggregate);
        return true;
    }
    let Some((root, projections)) = assignment_place(place) else {
        return false;
    };
    let mut aggregate = analyzer.lookup(&root).cloned().unwrap_or_default();
    let mut projected = VariableInfo::default();
    assign_place_projection(&mut projected, &projections, info);
    aggregate.union(&projected);
    analyzer.assign(&root, aggregate);
    true
}

fn is_assignment_binop(operation: &syn::BinOp) -> bool {
    matches!(
        operation,
        syn::BinOp::AddAssign(_)
            | syn::BinOp::SubAssign(_)
            | syn::BinOp::MulAssign(_)
            | syn::BinOp::DivAssign(_)
            | syn::BinOp::RemAssign(_)
            | syn::BinOp::BitXorAssign(_)
            | syn::BinOp::BitAndAssign(_)
            | syn::BinOp::BitOrAssign(_)
            | syn::BinOp::ShlAssign(_)
            | syn::BinOp::ShrAssign(_)
    )
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

fn mutable_storage_place(expression: &Expr) -> Option<&Expr> {
    match expression {
        Expr::Reference(reference) if reference.mutability.is_some() => Some(&reference.expr),
        Expr::Paren(paren) => mutable_storage_place(&paren.expr),
        Expr::Group(group) => mutable_storage_place(&group.expr),
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
        Expr::Struct(structure) => {
            let mut all_supported = true;
            for field in &structure.fields {
                let field_name = match &field.member {
                    Member::Named(ident) => normalized_ident(ident),
                    Member::Unnamed(index) => index.index.to_string(),
                };
                let field_info = info.field_items.get(&field_name).unwrap_or(info);
                if !assign_destructured_expr(analyzer, &field.expr, field_info) {
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
        if is_assignment_binop(&expression.op) {
            let info = self.info_from_expr(&expression.right);
            union_into_assignment_place(self, &expression.left, &info);
            let root = simple_assignment_name(&expression.left)
                .or_else(|| assignment_place(&expression.left).map(|(root, _)| root));
            if let Some(root) = root {
                let mut aggregate = self.lookup(&root).cloned().unwrap_or_default();
                self.union_into_declared_projections(&mut aggregate, &info);
                self.assign(&root, aggregate);
            }
            let cfg = item_cfg(&self.cfg, &expression.attrs);
            self.record_pool_escape(
                &info.flow,
                PersistenceOperation::StoreEscape,
                "compound_assignment",
                &cfg,
                normalized_tokens(expression),
            );
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
        self.return_exit_collectors.push(None);
        self.return_value_collectors.push(VariableInfo::default());
        self.visit_block(&expression.block);
        self.return_value_collectors
            .pop()
            .expect("async return value collector was installed");
        if let Some(exits) = self
            .return_exit_collectors
            .pop()
            .expect("async return collector was installed")
        {
            merge_scope_stacks(&mut self.scopes, &exits);
            self.bump_context();
        }
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
            if let Item::Const(item_const) = item {
                let (kind, sources) =
                    source_sql_info(&item_const.expr, &|path| self.standard_string_macro(path));
                if kind != SqlExpressionKind::Nonliteral {
                    let mut info = self.info_from_type(&item_const.ty);
                    info.sql_expression = kind;
                    info.sql_sources = sources;
                    self.bind(normalized_ident(&item_const.ident), info);
                    return;
                }
            }
            if let Item::Static(item_static) = item {
                let (kind, sources) =
                    source_sql_info(&item_static.expr, &|path| self.standard_string_macro(path));
                if kind != SqlExpressionKind::Nonliteral {
                    let mut info = self.info_from_type(&item_static.ty);
                    info.sql_expression = kind;
                    info.sql_sources = sources;
                    self.bind(normalized_ident(&item_static.ident), info);
                    return;
                }
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
        self.register_local_constants(&block.stmts);
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
        for argument in &call.args {
            if matches!(argument, Expr::Closure(_)) {
                self.visit_expr(argument);
            }
        }
        let cfg = item_cfg(&self.cfg, &call.attrs);
        let (
            name,
            canonical_name,
            executor_owner,
            rooted_sqlx,
            imported_query,
            path_targets,
            flow_passthrough,
        ) = match call.func.as_ref() {
            Expr::Path(path) => {
                let names = path_names(&path.path);
                // A block-local `use sqlx::query as q` renames the callee
                // without changing what it constructs, so the decision has
                // to be made on the path it resolves to.
                let canonical = self.canonical_names(names.clone());
                let canonical_name = canonical.last().cloned().unwrap_or_default();
                // The row keeps the name the source writes; only the
                // dispatch below follows the alias to what it constructs.
                let name = names.last().cloned().unwrap_or_default();
                // A local binding can name an executor, and the call
                // through it is that executor.
                let stored_executor = (names.len() == 1)
                    .then(|| {
                        self.lookup(&names[0])
                            .and_then(|info| info.executor_callable.clone())
                    })
                    .flatten();
                let canonical_name = stored_executor.clone().unwrap_or(canonical_name);
                let canonical = match &stored_executor {
                    Some(method) => {
                        vec!["sqlx".to_owned(), "Executor".to_owned(), method.clone()]
                    }
                    None => canonical,
                };
                let rooted_sqlx = path_is_sqlx(&names, self.symbols)
                    || path_is_sqlx(&canonical, self.symbols)
                    || stored_executor.is_some();
                let imported_query = names.len() == 1
                    && (self.symbols.query_callables.contains(&name)
                        || self.symbols.query_callables.contains(&canonical_name)
                        || self.lookup(&name).is_some_and(|info| info.query_callable));
                (
                    name,
                    canonical_name,
                    canonical
                        .iter()
                        .nth_back(1)
                        .is_some_and(|segment| segment == "Executor"),
                    rooted_sqlx,
                    imported_query,
                    targets_for_names(&canonical, self.symbols),
                    is_flow_passthrough_call(&names),
                )
            }
            // `(sqlx::query)(SQL)` names the same callable as `sqlx::query`;
            // the parentheses do not change what it constructs.
            callee => {
                let info = self.info_from_expr(callee);
                let executor = info.executor_callable.clone();
                let name = executor.clone().unwrap_or_else(|| {
                    info.query_callable
                        .then(|| "query".to_owned())
                        .unwrap_or_default()
                });
                (
                    name.clone(),
                    name,
                    executor.is_some(),
                    executor.is_some() || info.query_callable,
                    info.query_callable,
                    TargetSet::new(),
                    false,
                )
            }
        };
        let query_builder_constructor = rooted_sqlx
            && canonical_name == "new"
            && matches!(
                call.func.as_ref(),
                Expr::Path(path)
                    if self
                        .canonical_names(path_names(&path.path))
                        .iter()
                        .nth_back(1)
                        .is_some_and(|segment| segment == "QueryBuilder")
            );
        let query = (rooted_sqlx && (is_query_name(&name) || is_query_name(&canonical_name)))
            || imported_query
            || query_builder_constructor;
        let has_path_targets = !path_targets.is_empty();
        if query {
            let fingerprint = self.fingerprint_with_sql_source(
                canonical_call(call),
                call.args.first(),
            );
            self.add(
                PersistenceTarget::Sqlx,
                PersistenceOperation::Query,
                &name,
                &cfg,
                fingerprint.clone(),
            );
            if canonical_name == "raw_sql" {
                self.add(
                    PersistenceTarget::Sqlx,
                    PersistenceOperation::RawSql,
                    &name,
                    &cfg,
                    fingerprint.clone(),
                );
            }
            if self.statement_takes_advisory_lock(&fingerprint) {
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
        } else if let Some(operation) = PersistenceOperation::from_executor_method(&canonical_name)
            .filter(
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
            // The trait can be imported under another name, and `prepare`
            // receives its statement the same way the executors do.
            let ufcs_executor_sql = matches!(
                operation,
                PersistenceOperation::Execute
                    | PersistenceOperation::Fetch
                    | PersistenceOperation::FetchAll
                    | PersistenceOperation::FetchMany
                    | PersistenceOperation::FetchOne
                    | PersistenceOperation::FetchOptional
                    | PersistenceOperation::PrepareStatement
            ) && rooted_sqlx
                && executor_owner
                && call.args.first().is_some_and(|receiver| {
                    let flow = self.flow_of_expr(receiver);
                    !flow.is_empty() && !flow.has_stage(FlowStage::Query)
                })
                // The second argument is the statement only when it is not an
                // already built query, exactly as in the method form.
                && call.args.get(1).is_some_and(|argument| {
                    !self.flow_of_expr(argument).has_stage(FlowStage::Query)
                });
            let mut targets = path_targets;
            for argument in &call.args {
                targets.extend(self.flow_of_expr(argument).targets());
            }
            if targets.is_empty() {
                targets.insert(PersistenceTarget::Sqlx);
            }
            for target in targets {
                let prepared_statement_sql = name == "new"
                    && target == PersistenceTarget::PreparedStatement
                    && call.args.first().is_some();
                let raw_sql_argument = if prepared_statement_sql {
                    call.args.first()
                } else if ufcs_executor_sql {
                    call.args.get(1)
                } else {
                    None
                };
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
                let fingerprint = if raw_sql_argument.is_some() {
                    self.fingerprint_with_sql_source(canonical_call(call), raw_sql_argument)
                } else {
                    canonical_call(call)
                };
                self.add(target, operation, &name, &cfg, fingerprint.clone());
                if let Some(argument) = raw_sql_argument {
                    self.add(
                        target,
                        PersistenceOperation::RawSql,
                        &name,
                        &cfg,
                        fingerprint.clone(),
                    );
                    if self.statement_takes_advisory_lock(&fingerprint) {
                        self.add(
                            target,
                            PersistenceOperation::AdvisoryLock,
                            &name,
                            &cfg,
                            fingerprint.clone(),
                        );
                    }
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
                        SqlExpressionKind::Nonliteral => self.add(
                            target,
                            PersistenceOperation::NonliteralSql,
                            &name,
                            &cfg,
                            fingerprint.clone(),
                        ),
                        SqlExpressionKind::Interpolated => self.add(
                            target,
                            PersistenceOperation::InterpolatedSql,
                            &name,
                            &cfg,
                            fingerprint.clone(),
                        ),
                    }
                }
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
        if let Expr::Path(path) = call.func.as_ref()
            && is_standard_replacement(&self.canonical_local_path_names(path_names(&path.path)))
            && let Some(binding) = call.args.first().and_then(mutable_storage_receiver_name)
            && let Some(previous) = self.lookup(&binding).cloned()
        {
            self.replacement_result_infos
                .borrow_mut()
                .entry(call as *const ExprCall as usize)
                .or_default()
                .union(&previous);
        }
        if !flow_passthrough && !known_persistence_call {
            for argument in &call.args {
                let flow = self.flow_of_expr(argument);
                self.record_persistence_escape(
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
        if let Expr::Path(path) = call.func.as_ref()
            && is_standard_replacement(&self.canonical_local_path_names(path_names(&path.path)))
            && name == "take"
            && let Some(receiver_name) = call.args.first().and_then(mutable_storage_receiver_name)
        {
            self.assign(&receiver_name, VariableInfo::default());
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
        if let Expr::Path(path) = call.func.as_ref() {
            let names = path_names(&path.path);
            let callee = names.last().cloned().unwrap_or_default();
            let package_key = self.package_function_key(names.clone());
            let effects = (names.len() == 1)
                .then(|| self.symbols.function_mutable_writes.get(&callee))
                .flatten()
                .or_else(|| {
                    self.symbols
                        .package_function_mutable_writes
                        .get(&package_key)
                })
                .cloned()
                .unwrap_or_default();
            for (index, effect) in effects {
                let Some(place) = call.args.get(index).and_then(mutable_storage_place) else {
                    continue;
                };
                let effect = self.apply_inferred_args(
                    &effect,
                    self.symbols
                        .function_generic_params
                        .get(&callee)
                        .or_else(|| {
                            self.symbols
                                .package_function_generic_params
                                .get(&package_key)
                        }),
                    self.symbols
                        .function_generic_input_params
                        .get(&callee)
                        .or_else(|| {
                            self.symbols
                                .package_function_generic_input_params
                                .get(&package_key)
                        }),
                    &call.args,
                );
                union_into_assignment_place(self, place, &effect);
            }
        }
        let callee_info = self.info_from_expr(&call.func);
        for (captured, info) in self.closure_mutations_for_args(&callee_info, &argument_infos) {
            self.assign(&captured, info);
        }
        if !known_persistence_call && !flow_passthrough {
            let before_callback = self.scopes.clone();
            let called_inputs = match call.func.as_ref() {
                Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
                    last_path_name(&path.path)
                        .and_then(|name| self.symbols.function_called_inputs.get(&name))
                        .cloned()
                        .unwrap_or_default()
                }
                _ => BTreeSet::new(),
            };
            for index in called_inputs {
                let Some(info) = argument_infos.get(index) else {
                    continue;
                };
                for (captured, mutation) in self.closure_mutations_for_args(info, &[]) {
                    self.assign(&captured, mutation);
                }
            }
            let after_callback = self.scopes.clone();
            self.scopes = before_callback;
            merge_scope_stacks(&mut self.scopes, &after_callback);
            self.bump_context();
        }
        self.visit_expr(&call.func);
        for argument in &call.args {
            if !matches!(argument, Expr::Closure(_)) {
                self.visit_expr(argument);
            }
        }
    }

    fn visit_expr_method_call(&mut self, method: &'ast ExprMethodCall) {
        if !self.allows_source_class(&method.attrs, "method call") {
            return;
        }
        for argument in &method.args {
            if matches!(argument, Expr::Closure(_)) {
                self.visit_expr(argument);
            }
        }
        let cfg = item_cfg(&self.cfg, &method.attrs);
        let name = normalized_ident(&method.method);
        let receiver = self.flow_of_expr(&method.receiver);
        if receiver.is_empty()
            && self.is_unresolved_opaque_call(&method.receiver)
            && matches!(
                name.as_str(),
                "pool"
                    | "acquire"
                    | "begin"
                    | "prepare"
                    | "query"
                    | "execute"
                    | "fetch"
                    | "fetch_all"
                    | "fetch_many"
                    | "fetch_one"
                    | "fetch_optional"
            )
        {
            self.errors.push(format!(
                "{} invokes persistence-shaped method {name} on a zero-argument opaque return whose concrete flow is not represented",
                self.enclosing
            ));
        }
        let mut bound_parameter = false;
        let validated_flow_passthrough = FLOW_PASSTHROUGH_METHODS.contains(&name.as_str())
            || (name == "bind" && receiver.has_stage(FlowStage::Query));
        let operation = if is_query_name(&name) && !receiver.0.is_empty() {
            Some(PersistenceOperation::Query)
        } else if matches!(name.as_str(), "push" | "push_unseparated" | "separated")
            && receiver.targets().contains(&PersistenceTarget::Sqlx)
        {
            Some(PersistenceOperation::RawSql)
        } else if matches!(
            name.as_str(),
            "push_bind" | "push_bind_unseparated" | "push_bindings" | "push_values" | "push_tuples"
        ) && (receiver.targets().contains(&PersistenceTarget::Sqlx)
            || receiver.has_stage(FlowStage::Query))
        {
            // A bind changes what the builder sends, so the call carries a row
            // — but the value is a parameter, not a statement, and it is kept
            // out of SQL-content classification below.
            bound_parameter = true;
            Some(PersistenceOperation::RawSql)
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
                // An executor is handed either a statement or an already built
                // query. `pool.execute(sqlx::query("…"))` is the second, and
                // reading it as raw SQL would report ordinary typed execution
                // as dynamic.
                let executor_consumes_raw_sql = matches!(
                    operation,
                    PersistenceOperation::Execute
                        | PersistenceOperation::Fetch
                        | PersistenceOperation::FetchAll
                        | PersistenceOperation::FetchMany
                        | PersistenceOperation::FetchOne
                        | PersistenceOperation::FetchOptional
                ) && !receiver.has_stage(FlowStage::Query)
                    && method.args.first().is_some_and(|argument| {
                        !self.flow_of_expr(argument).has_stage(FlowStage::Query)
                    });
                let mut targets = receiver.targets();
                for argument in &method.args {
                    targets.extend(self.flow_of_expr(argument).targets());
                }
                for target in targets {
                    let fingerprint = self
                        .fingerprint_with_sql_source(canonical_method(method), method.args.first());
                    self.add(target, operation, &name, &cfg, fingerprint.clone());
                    if executor_consumes_raw_sql
                        || operation == PersistenceOperation::PrepareStatement
                    {
                        self.add(
                            target,
                            PersistenceOperation::RawSql,
                            &name,
                            &cfg,
                            fingerprint.clone(),
                        );
                    }
                    if (matches!(
                        operation,
                        PersistenceOperation::DirectQuery
                            | PersistenceOperation::RawSql
                            // `prepare` receives the statement itself, so its
                            // text belongs to the inventory like any other.
                            | PersistenceOperation::PrepareStatement
                    ) || executor_consumes_raw_sql)
                        && !bound_parameter
                    {
                        if self.statement_takes_advisory_lock(&fingerprint) {
                            self.add(
                                target,
                                PersistenceOperation::AdvisoryLock,
                                &name,
                                &cfg,
                                fingerprint.clone(),
                            );
                        }
                    }
                    if (matches!(
                        operation,
                        PersistenceOperation::DirectQuery
                            | PersistenceOperation::DirectExecute
                            | PersistenceOperation::RawSql
                            // A prepared statement is supplied the same way any
                            // other raw SQL is, so it is classified the same:
                            // dynamic text is recorded, and text the snapshot
                            // cannot see is refused.
                            | PersistenceOperation::PrepareStatement
                    ) || executor_consumes_raw_sql)
                        && !bound_parameter
                    {
                        let Some(argument) = method.args.first() else {
                            continue;
                        };
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
            if receiver.has_stage(FlowStage::Transaction) {
                self.record_persistence_escape(
                    &receiver,
                    PersistenceOperation::ArgumentEscape,
                    &format!("receiver:{name}"),
                    &cfg,
                    normalized_tokens(&method.receiver),
                );
            } else {
                self.record_pool_escape(
                    &receiver,
                    PersistenceOperation::ArgumentEscape,
                    &format!("receiver:{name}"),
                    &cfg,
                    normalized_tokens(&method.receiver),
                );
            }
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
            "push"
                | "push_back"
                | "push_front"
                | "insert"
                | "get_or_insert"
                | "get_or_insert_with"
                | "extend"
                | "append"
                | "replace"
        ) {
            let mut stored = VariableInfo::default();
            for argument in &method.args {
                stored.union(&self.info_from_expr(argument));
            }
            union_into_assignment_place(self, &method.receiver, &stored);
        }
        let sql_mutation_argument = match name.as_str() {
            "push_str" | "push" | "extend" | "append" => method.args.first(),
            "insert_str" | "insert" | "replace_range" => method.args.get(1),
            "clear" | "truncate" | "remove" | "pop" | "retain" | "drain" | "split_off" => None,
            _ => None,
        };
        if matches!(
            name.as_str(),
            "push_str"
                | "push"
                | "extend"
                | "append"
                | "insert_str"
                | "insert"
                | "replace_range"
                | "clear"
                | "truncate"
                | "remove"
                | "pop"
                | "retain"
                | "drain"
                | "split_off"
        ) {
            let mut sql_sources = BTreeSet::from([normalized_tokens(method)]);
            if let Some(argument) = sql_mutation_argument {
                sql_sources.extend(self.sql_sources(argument));
            }
            let appended = VariableInfo {
                sql_expression: sql_mutation_argument
                    .map(|argument| self.sql_expression_kind(argument))
                    .unwrap_or(SqlExpressionKind::Nonliteral),
                sql_sources,
                ..VariableInfo::default()
            };
            union_into_assignment_place(self, &method.receiver, &appended);
        }
        let (receiver_effect, parameter_effects) = self.method_mutation_effects(method);
        if receiver_effect != VariableInfo::default() {
            union_into_assignment_place(self, &method.receiver, &receiver_effect);
        }
        for (index, effect) in parameter_effects {
            let Some(place) = method.args.get(index).and_then(mutable_storage_place) else {
                continue;
            };
            union_into_assignment_place(self, place, &effect);
        }
        self.visit_expr(&method.receiver);
        for argument in &method.args {
            if !matches!(argument, Expr::Closure(_)) {
                self.visit_expr(argument);
            }
        }
        if CLOSURE_INVOKING_METHODS.contains(&name.as_str())
            || (!valid_persistence_method
                && method
                    .args
                    .iter()
                    .any(|argument| matches!(argument, Expr::Closure(_))))
        {
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
            if let Expr::Unary(unary) = assignment.left.as_ref()
                && matches!(unary.op, syn::UnOp::Deref(_))
            {
                let pointee = self.info_from_expr(&unary.expr);
                for name in pointee.mutable_pointees {
                    self.assign(&name, info.clone());
                }
                for place in pointee.mutable_places {
                    let mut aggregate = self.lookup(&place.root).cloned().unwrap_or_default();
                    assign_place_projection(&mut aggregate, &place.projections, &info);
                    self.assign(&place.root, aggregate);
                }
            }
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
            let info = self.info_from_expr(expression);
            let flow = info.flow.clone();
            let cfg = item_cfg(&self.cfg, &returned.attrs);
            self.record_pool_escape(
                &flow,
                PersistenceOperation::ReturnEscape,
                "pool",
                &cfg,
                normalized_tokens(expression),
            );
            if let Some(result) = self.return_value_collectors.last_mut() {
                result.union(&info);
            }
        }
        self.capture_return_exit();
    }

    fn visit_expr_try(&mut self, expression: &'ast syn::ExprTry) {
        if !self.allows_source_class(&expression.attrs, "try expression") {
            return;
        }
        self.visit_expr(&expression.expr);
        // On `Err`, `?` returns the operand's residual from the function, so
        // persistence carried there leaves by the same door as an explicit
        // `return`.
        //
        // Known over-report: this records the operand's whole flow, so a pool
        // held in the *success* payload is reported as escaping although `?`
        // consumes it. Extracting only the residual needs shape information
        // that is present for some operands and absent for others, and two
        // attempts at it each lost a real escape. Reporting an escape that does
        // not happen is the lesser fault for a ratchet whose purpose is to
        // notice change; missing one is the fault that matters.
        let flow = self.info_from_expr(&expression.expr).flow;
        let cfg = item_cfg(&self.cfg, &expression.attrs);
        self.record_pool_escape(
            &flow,
            PersistenceOperation::ReturnEscape,
            "pool",
            &cfg,
            normalized_tokens(&expression.expr),
        );
        // `?` can return from the surrounding closure/async body immediately
        // after evaluating its operand. Preserve that exit state before later
        // statements on the success path can clear the captured binding.
        self.capture_return_exit();
    }

    fn visit_expr_break(&mut self, expression: &'ast syn::ExprBreak) {
        if !self.allows_source_class(&expression.attrs, "break expression") {
            return;
        }
        if let Some(value) = &expression.expr {
            self.visit_expr(value);
            if let Some(label) = &expression.label {
                let label = normalized_ident(&label.ident);
                let info = self.info_from_expr(value);
                if let Some(collector) = self
                    .block_exit_collectors
                    .iter_mut()
                    .rev()
                    .find(|collector| collector.label == label)
                {
                    collector.result.union(&info);
                }
            }
        }
        self.capture_loop_control(expression.label.as_ref(), true);
    }

    fn visit_expr_continue(&mut self, expression: &'ast syn::ExprContinue) {
        if !self.allows_source_class(&expression.attrs, "continue expression") {
            return;
        }
        self.capture_loop_control(expression.label.as_ref(), false);
    }

    fn visit_expr_block(&mut self, expression: &'ast syn::ExprBlock) {
        if !self.allows_source_class(&expression.attrs, "block expression") {
            return;
        }
        let Some(label) = &expression.label else {
            self.visit_block(&expression.block);
            return;
        };
        self.block_exit_collectors.push(BlockExitCollector {
            label: normalized_ident(&label.name.ident),
            ..BlockExitCollector::default()
        });
        self.visit_block(&expression.block);
        let collector = self
            .block_exit_collectors
            .pop()
            .expect("labeled block collector was installed");
        self.block_result_infos
            .borrow_mut()
            .entry(&expression.block as *const syn::Block as usize)
            .or_default()
            .union(&collector.result);
        if let Some(exits) = collector.exits {
            merge_scope_stacks(&mut self.scopes, &exits);
            self.bump_context();
        }
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
            let arm_entry_scopes = next_arm_scopes.clone();
            self.scopes = next_arm_scopes.clone();
            self.push_scope();
            self.bind_pattern(&arm.pat, &scrutinee);
            if let Some((_, guard)) = &arm.guard {
                self.visit_expr(guard);
                next_arm_scopes = self.scopes.clone();
                next_arm_scopes.pop();
                // The pattern can fail before the guard runs, so a later arm
                // must also represent the path on which the guard never
                // executed and its side effects never happened.
                merge_scope_stacks(&mut next_arm_scopes, &arm_entry_scopes);
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
        self.return_exit_collectors.push(None);
        self.return_value_collectors.push(VariableInfo::default());
        self.visit_expr(&closure.body);
        let mut result_info = self.info_from_expr(&closure.body);
        let returned = self
            .return_value_collectors
            .pop()
            .expect("closure return value collector was installed");
        result_info.union(&returned);
        self.closure_result_infos
            .borrow_mut()
            .insert(closure as *const ExprClosure as usize, result_info);
        if let Some(exits) = self
            .return_exit_collectors
            .pop()
            .expect("closure return collector was installed")
        {
            merge_scope_stacks(&mut self.scopes, &exits);
            self.bump_context();
        }
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
    visible_macro_shadows: BTreeSet<String>,
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
        analyzer.visible_macro_shadows = visible_macro_shadows.clone();
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
    visible_macro_shadows: BTreeSet<String>,
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
        .then(|| {
            persistence_operations_in_block(
                &function.block,
                generic_type_param_names(&function.sig.generics)
                    .into_iter()
                    .collect(),
            )
        })
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
    analyzer.visible_macro_shadows = visible_macro_shadows;
    analyzer.register_local_uses(&function.block.stmts);
    analyzer.register_local_callables(&function.block.stmts);
    analyzer.register_local_constants(&function.block.stmts);
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
    visible_macro_shadows: BTreeSet<String>,
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
        analyzer.visible_macro_shadows = visible_macro_shadows.clone();
        analyzer.register_local_uses(&method.block.stmts);
        analyzer.register_local_callables(&method.block.stmts);
        analyzer.register_local_constants(&method.block.stmts);
        analyzer.register_generic_bounds(&item_impl.generics);
        analyzer.register_generic_bounds(&method.sig.generics);
        analyzer.active_trait = item_impl
            .trait_
            .as_ref()
            .map(|(_, path, _)| canonical_path_names(path_names(path), symbols).join("::"));
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
    // An alias does not change what the macro brings in, so the guard below
    // reads the path it resolves to.
    let path_name = canonical_path_names(path_names(&item_macro.mac.path), symbols)
        .last()
        .cloned()
        .unwrap_or_default();
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
    for (item_index, item) in items.iter().enumerate() {
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
                    macro_shadows_before(items, item_index),
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
                    macro_shadows_before(items, item_index),
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
                        macro_shadows_before(items, item_index),
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

/// Refresh only the cache entries a finished package can have changed.
///
/// Both caches are pure functions of the registry, and analyzing one package
/// rewrites only that package's own entries. Rebuilding them from scratch once
/// per package re-cloned and re-merged the entire registry O(packages) times
/// per fixpoint iteration; the merged workspace view additionally has to be
/// rebuilt only for consumers that can actually see the changed provider.
fn refresh_named_type_caches(
    registries: &BTreeMap<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>,
    dependencies: &WorkspaceDependencyAliases,
    changed_package: &str,
    workspace_cache: &mut BTreeMap<
        (String, PersistenceSourceClass),
        std::sync::Arc<BTreeMap<String, VariableInfo>>,
    >,
    package_cache: &mut BTreeMap<
        (String, PersistenceSourceClass),
        std::sync::Arc<BTreeMap<String, VariableInfo>>,
    >,
) {
    for source_class in [
        PersistenceSourceClass::Production,
        PersistenceSourceClass::TestFixture,
    ] {
        let key = (changed_package.to_owned(), source_class);
        match registries.get(&key) {
            Some(info) => {
                package_cache.insert(key, std::sync::Arc::new(info.clone()));
            }
            None => {
                package_cache.remove(&key);
            }
        }
    }
    let changed_crate = changed_package.replace('-', "_");
    for (aliases_by_package, source_class) in [
        (&dependencies.production, PersistenceSourceClass::Production),
        (&dependencies.test, PersistenceSourceClass::TestFixture),
    ] {
        for (consumer, aliases) in aliases_by_package {
            let roots = aliases.values().cloned().collect::<BTreeSet<_>>();
            if !roots.contains(&changed_crate) {
                continue;
            }
            workspace_cache.insert(
                (consumer.clone(), source_class),
                std::sync::Arc::new(workspace_named_type_info(registries, source_class, &roots)),
            );
        }
    }
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

fn resolve_public_sqlx_namespace_reexports(
    reexports: &BTreeMap<(String, PersistenceSourceClass), Vec<(String, String)>>,
) -> BTreeMap<(String, PersistenceSourceClass), BTreeSet<String>> {
    let mut registries = reexports
        .keys()
        .cloned()
        .map(|key| (key, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    let pass_limit = reexports.values().map(Vec::len).sum::<usize>() + 1;
    for _ in 0..pass_limit {
        let before = registries.clone();
        for (consumer_key, aliases) in reexports {
            for (export, source) in aliases {
                let direct = source == "sqlx"
                    || before
                        .get(consumer_key)
                        .is_some_and(|known| known.contains(source));
                let forwarded = source.split_once("::").is_some_and(|(root, remainder)| {
                    before.iter().any(|((provider, source_class), known)| {
                        *source_class == consumer_key.1
                            && provider.replace('-', "_") == root
                            && known.contains(remainder)
                    })
                });
                if direct || forwarded {
                    registries
                        .entry(consumer_key.clone())
                        .or_default()
                        .insert(export.clone());
                }
            }
        }
        if registries == before {
            break;
        }
    }
    registries
}

fn workspace_sqlx_namespace_cache(
    registries: &BTreeMap<(String, PersistenceSourceClass), BTreeSet<String>>,
    dependencies: &WorkspaceDependencyAliases,
) -> BTreeMap<(String, PersistenceSourceClass), std::sync::Arc<BTreeSet<String>>> {
    dependencies
        .production
        .iter()
        .map(|(package, aliases)| {
            let key = (package.clone(), PersistenceSourceClass::Production);
            let mut namespaces = registries.get(&key).cloned().unwrap_or_default();
            for provider_root in aliases.values() {
                if let Some((_, provider_namespaces)) =
                    registries.iter().find(|((provider, source_class), _)| {
                        *source_class == PersistenceSourceClass::Production
                            && provider.replace('-', "_") == *provider_root
                    })
                {
                    namespaces.extend(
                        provider_namespaces
                            .iter()
                            .map(|path| format!("{provider_root}::{path}")),
                    );
                }
            }
            (key, std::sync::Arc::new(namespaces))
        })
        .chain(dependencies.test.iter().map(|(package, aliases)| {
            let key = (package.clone(), PersistenceSourceClass::TestFixture);
            let mut namespaces = registries.get(&key).cloned().unwrap_or_default();
            for provider_root in aliases.values() {
                if let Some((_, provider_namespaces)) =
                    registries.iter().find(|((provider, source_class), _)| {
                        *source_class == PersistenceSourceClass::TestFixture
                            && provider.replace('-', "_") == *provider_root
                    })
                {
                    namespaces.extend(
                        provider_namespaces
                            .iter()
                            .map(|path| format!("{provider_root}::{path}")),
                    );
                }
            }
            (key, std::sync::Arc::new(namespaces))
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
        let mut next = named_type_registries.clone();
        let mut workspace_cache = workspace_named_type_info_cache(&next, dependencies);
        let mut package_cache = package_named_type_info_cache(&next);
        let mut package_start = 0;
        while package_start < ordered.len() {
            let package = ordered[package_start].package;
            let package_end = ordered[package_start..]
                .iter()
                .position(|source| source.package != package)
                .map_or(ordered.len(), |offset| package_start + offset);
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
            refresh_named_type_caches(
                &next,
                dependencies,
                package,
                &mut workspace_cache,
                &mut package_cache,
            );
        }
        let converged = next == named_type_registries;
        named_type_registries = next;
        if converged {
            break;
        }
    }
    let mut callable_reexports =
        BTreeMap::<(String, PersistenceSourceClass), Vec<(String, String)>>::new();
    let mut local_callable_imports = BTreeMap::new();
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
            collect_local_callable_imports(
                &syntax.items,
                &symbols,
                &cfg,
                source_class,
                &mut errors,
                local_callable_imports.entry(key.clone()).or_default(),
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
    let sqlx_namespace_registries = resolve_public_sqlx_namespace_reexports(&callable_reexports);
    let sqlx_namespace_cache =
        workspace_sqlx_namespace_cache(&sqlx_namespace_registries, dependencies);
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
    let mut function_mutable_write_registries = BTreeMap::<
        (String, PersistenceSourceClass),
        BTreeMap<String, BTreeMap<usize, VariableInfo>>,
    >::new();
    let mut function_generic_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<String, Vec<String>>>::new();
    let mut function_generic_input_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<String, Vec<GenericInputSpec>>>::new(
        );
    let mut method_registries = BTreeMap::<
        (String, PersistenceSourceClass),
        BTreeMap<(String, String), VariableInfo>,
    >::new();
    let mut method_mutable_receiver_registries = BTreeMap::<
        (String, PersistenceSourceClass),
        BTreeMap<(String, String), VariableInfo>,
    >::new();
    let mut method_mutable_write_registries = BTreeMap::<
        (String, PersistenceSourceClass),
        BTreeMap<(String, String), BTreeMap<usize, VariableInfo>>,
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
            base.workspace_sqlx_namespaces = sqlx_namespace_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeSet::new()));
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
            let function_mutable_write_registry = function_mutable_write_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for (name, effects) in &symbols.function_mutable_writes {
                let canonical = if module_prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{module_prefix}::{name}")
                };
                let target = function_mutable_write_registry
                    .entry(canonical)
                    .or_default();
                for (index, info) in effects {
                    target.entry(*index).or_default().union(info);
                }
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
            let mutable_receiver_registry = method_mutable_receiver_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for ((owner, trait_name, method), info) in &symbols.method_mutable_receivers {
                if trait_name.is_some() {
                    continue;
                }
                let canonical_owner = if module_prefix.is_empty() || owner.contains("::") {
                    owner.clone()
                } else {
                    format!("{module_prefix}::{owner}")
                };
                mutable_receiver_registry
                    .entry((canonical_owner, method.clone()))
                    .or_default()
                    .union(info);
            }
            let mutable_write_registry = method_mutable_write_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for ((owner, trait_name, method), effects) in &symbols.method_mutable_writes {
                if trait_name.is_some() {
                    continue;
                }
                let canonical_owner = if module_prefix.is_empty() || owner.contains("::") {
                    owner.clone()
                } else {
                    format!("{module_prefix}::{owner}")
                };
                let target = mutable_write_registry
                    .entry((canonical_owner, method.clone()))
                    .or_default();
                for (index, info) in effects {
                    target.entry(*index).or_default().union(info);
                }
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
            base.workspace_sqlx_namespaces = sqlx_namespace_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeSet::new()));
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
    let mut function_registry_cache = dependency_scoped_registry_cache(
        &function_registries,
        &named_type_registries,
        dependencies,
        |provider_root, key| format!("{provider_root}::{key}"),
        qualify_dependency_info,
    );
    let mut function_mutable_write_registry_cache = dependency_scoped_registry_cache(
        &function_mutable_write_registries,
        &named_type_registries,
        dependencies,
        |provider_root, key| format!("{provider_root}::{key}"),
        |provider_root, named, effects| {
            effects
                .iter()
                .map(|(index, info)| (*index, qualify_dependency_info(provider_root, named, info)))
                .collect()
        },
    );
    let mut function_generic_registry_cache = dependency_scoped_registry_cache(
        &function_generic_registries,
        &named_type_registries,
        dependencies,
        |provider_root, key| format!("{provider_root}::{key}"),
        |_, _, value| value.clone(),
    );
    let mut function_generic_input_registry_cache = dependency_scoped_registry_cache(
        &function_generic_input_registries,
        &named_type_registries,
        dependencies,
        |provider_root, key| format!("{provider_root}::{key}"),
        |_, _, value| value.clone(),
    );
    resolve_local_callable_imports(&local_callable_imports, &mut function_registry_cache);
    resolve_local_callable_imports(
        &local_callable_imports,
        &mut function_mutable_write_registry_cache,
    );
    resolve_local_callable_imports(
        &local_callable_imports,
        &mut function_generic_registry_cache,
    );
    resolve_local_callable_imports(
        &local_callable_imports,
        &mut function_generic_input_registry_cache,
    );
    let method_registry_cache = dependency_scoped_registry_cache(
        &method_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (owner, method)| (format!("{provider_root}::{owner}"), method.clone()),
        qualify_dependency_info,
    );
    let method_mutable_receiver_registry_cache = dependency_scoped_registry_cache(
        &method_mutable_receiver_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (owner, method)| (format!("{provider_root}::{owner}"), method.clone()),
        qualify_dependency_info,
    );
    let method_mutable_write_registry_cache = dependency_scoped_registry_cache(
        &method_mutable_write_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (owner, method)| (format!("{provider_root}::{owner}"), method.clone()),
        |provider_root, named, effects| {
            effects
                .iter()
                .map(|(index, info)| (*index, qualify_dependency_info(provider_root, named, info)))
                .collect()
        },
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
            package_symbols.workspace_sqlx_namespaces = sqlx_namespace_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeSet::new()));
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
            package_symbols.package_function_mutable_writes = function_mutable_write_registry_cache
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
            package_symbols.package_method_mutable_receivers =
                method_mutable_receiver_registry_cache
                    .get(&registry_key)
                    .cloned()
                    .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_method_mutable_writes = method_mutable_write_registry_cache
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

#[cfg(test)]
mod tests;
