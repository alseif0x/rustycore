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

mod analyze_items_1;
mod analyze_items_2;
mod body;
mod body_ops_1;
mod body_ops_2;
mod body_ops_3;
mod body_ops_4;
mod body_ops_5;
mod body_support;
mod body_visit;
mod caches;
mod callables;
mod flow;
mod imports;
mod inventory;
mod operation_syntax;
mod records_scan;
mod symbol_collection;
mod symbols;
mod syntax;
mod type_targets;
#[allow(unused_imports)]
use analyze_items_1::*;
#[allow(unused_imports)]
use analyze_items_2::*;
#[allow(unused_imports)]
use body::*;
#[allow(unused_imports)]
use body_ops_1::*;
#[allow(unused_imports)]
use body_ops_2::*;
#[allow(unused_imports)]
use body_ops_3::*;
#[allow(unused_imports)]
use body_ops_4::*;
#[allow(unused_imports)]
use body_ops_5::*;
#[allow(unused_imports)]
use body_support::*;
#[allow(unused_imports)]
use body_visit::*;
#[allow(unused_imports)]
use caches::*;
#[allow(unused_imports)]
use callables::*;
#[allow(unused_imports)]
use flow::*;
#[allow(unused_imports)]
use imports::*;
#[allow(unused_imports)]
pub(crate) use inventory::*;
#[allow(unused_imports)]
use operation_syntax::*;
#[allow(unused_imports)]
use records_scan::*;
#[allow(unused_imports)]
use symbol_collection::*;
#[allow(unused_imports)]
use symbols::*;
#[allow(unused_imports)]
use syntax::*;
#[allow(unused_imports)]
use type_targets::*;

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

type TargetSet = BTreeSet<PersistenceTarget>;

#[cfg(test)]
mod tests;
