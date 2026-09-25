//! ConditionMgr evaluation context and source-specific condition queries.
//!
//! This crate owns the read-only application boundary for C++ `ConditionMgr`
//! evaluation. Runtime/session callers construct snapshots and provide the
//! condition callback; immutable rows remain owned by `wow-data`.

mod context;
mod evaluate;
mod loot;
mod source_queries;
mod spell_click;
mod store;

pub use context::*;
pub use evaluate::*;
pub use loot::*;
pub use source_queries::*;
pub use spell_click::*;
pub use store::{
    clear_condition_mgr_store_like_cpp, condition_mgr_store_like_cpp,
    set_condition_mgr_store_like_cpp,
};

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
