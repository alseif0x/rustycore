//! World modules.
//!
//! Grouped from the world_* siblings under #695; each module keeps its
//! items and its re-exported names.

pub mod auxiliary_catalog;
pub mod object_catalog;
pub mod query_catalog;
pub mod reference_catalog;
pub mod runtime;

pub use auxiliary_catalog::*;
pub use object_catalog::*;
pub use query_catalog::*;
pub use reference_catalog::*;
pub use runtime::*;
