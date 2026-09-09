// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Exact syntax inventory for transitional legacy↔canonical bridges.
//!
//! A bridge is not identified from an enclosing function name alone. The
//! inventory resolves imports, aliases, types, typed values, known session
//! fields, calls, and method receivers and requires evidence from both the
//! canonical map/entity authority and legacy map/runtime authority sides.
//! Ordinary DTOs and state enums are not authority evidence by themselves; the
//! exact canonical mutable `wow_entities::Creature` is. The small set of
//! direction-bearing anchors curated in `runtime-ownership-ledger.json` is an
//! explicit exception: those definitions remain visible even when one side is
//! represented implicitly by `WorldSession`. Calling an anchor does not make
//! every caller a new bridge definition.
//!
//! This is a deliberately strict source guard rather than a Rust type checker.
//! Macros whose visible tokens contain both sides, a curated anchor, or a
//! bridge-shaped name must either be one of the transparent diagnostic/data
//! macros understood here or fail closed. `include!` source resolution remains
//! the caller's module-graph/generated-input responsibility; every other
//! item-generating bridge macro fails here. The checked-in comparator is an
//! exact set and multiplicity comparison, so a same-count swap cannot pass.

use std::collections::{BTreeMap, BTreeSet};

use quote::ToTokens;
use serde::{Deserialize, Serialize};
use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprCall, ExprField, ExprMacro, ExprMethodCall, FnArg, ImplItem, Item, ItemFn,
    ItemImpl, ItemMacro, ItemMod, Local, Member, Pat, Path, Signature, Type, UseTree,
};

use crate::ownership::{
    cfg_context_allows_production, cfg_context_allows_test, extend_cfg_context,
};

mod state_1;
mod state_2;
mod state_3;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;
#[allow(unused_imports)]
pub use state_3::*;

#[cfg(test)]
#[path = "bridge_access/tests/mod.rs"]
mod tests;
