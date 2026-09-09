// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Syntax-level ratchets for the world-session ownership refactor.
//!
//! This module deliberately records exact source surfaces rather than line
//! counts. It cannot prove runtime ordering or semantic writer authority; those
//! remain behavior-test contracts. It does prevent an unreviewed field,
//! external impl, setter, visible method, construction-bag field, or factory
//! fan-out change from entering through ordinary Rust syntax.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use syn::visit::Visit;
use syn::{Expr, ImplItem, Item, ItemImpl, ItemStruct, Type, UseTree, Visibility};

use crate::bridge_access::{
    BridgeAccessBaseline, BridgeSource, compare_bridge_access_baseline, inventory_bridge_accesses,
    validate_curated_bridge_anchors,
};
use crate::ownership::{
    SourceMountContext, audit_package_source_mounts, cfg_context_allows_production,
    cfg_context_allows_test, extend_cfg_context, read_spliced_source, workspace_dependency_aliases,
    workspace_source_mounts,
};
use crate::persistence_access::{
    ClassifiedPersistenceSource, PersistenceAccessBaseline, compare_persistence_access_baseline,
    inventory_persistence_accesses_with_dependencies, render_persistence_access_baseline,
};
use crate::registry_access::{
    ProductionRegistrySource, RegistryAccessBaseline, compare_registry_access_baseline,
    inventory_registry_accesses,
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
#[path = "session_ownership/tests/mod.rs"]
mod tests;
