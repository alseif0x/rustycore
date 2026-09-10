//! World modules.
//!
//! Grouped from the world_* siblings under #697; each module keeps its
//! items and its re-exported names.

pub mod id_store;
pub mod query_catalog;
pub mod safe_locs;
pub mod spawn_id_store;
pub mod state_expression;

pub use id_store::*;
pub use query_catalog::*;
pub use safe_locs::*;
pub use spawn_id_store::*;
pub use state_expression::*;
