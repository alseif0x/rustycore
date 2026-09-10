//! Spell modules.
//!
//! Grouped from the spell_* siblings under #695; each module keeps its
//! items and its re-exported names.

pub mod acquisition;
pub mod acquisition_startup;
pub mod core_db2_hotfix;
pub mod info_key_hotfix;
pub mod world_catalog;

pub use acquisition::*;
pub use acquisition_startup::*;
pub use core_db2_hotfix::*;
pub use info_key_hotfix::*;
pub use world_catalog::*;
