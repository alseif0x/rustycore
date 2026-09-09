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

mod state_1;
mod state_2;
mod state_3;
mod state_4;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;
#[allow(unused_imports)]
pub use state_3::*;
#[allow(unused_imports)]
pub use state_4::*;

#[cfg(test)]
#[path = "registry_access/tests/mod.rs"]
mod tests;
