//! World modules.
//!
//! Grouped from the world_* siblings under #695; each module keeps its
//! items and its re-exported names.

pub mod auxiliary_catalog_adapter;
pub mod object_catalog_adapter;
pub mod query_catalog_adapter;
pub mod reference_catalog_adapter;
pub mod state_startup_adapter;

pub use auxiliary_catalog_adapter::*;
pub use object_catalog_adapter::*;
pub use query_catalog_adapter::*;
pub use reference_catalog_adapter::*;
pub use state_startup_adapter::*;
