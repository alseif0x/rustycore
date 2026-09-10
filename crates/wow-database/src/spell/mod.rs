//! Spell modules.
//!
//! Grouped from the spell_* siblings under #695; each module keeps its
//! items and its re-exported names.

pub mod acquisition_startup_adapter;
pub mod core_db2_hotfix_adapter;
pub mod info_key_hotfix_adapter;
pub mod world_catalog_adapter;

pub use acquisition_startup_adapter::*;
pub use core_db2_hotfix_adapter::*;
pub use info_key_hotfix_adapter::*;
pub use world_catalog_adapter::*;
