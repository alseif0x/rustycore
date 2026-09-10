//! Hotfix persistence adapters.
//!
//! Grouped from the *_hotfix_adapter siblings under #703; each module keeps its
//! items and its re-exported names.

pub mod chr_specialization_adapter;
pub mod creature_display_adapter;
pub mod difficulty_adapter;
pub mod lfg_dungeons_adapter;
pub mod skill_catalog_adapter;

pub use chr_specialization_adapter::*;
pub use creature_display_adapter::*;
pub use difficulty_adapter::*;
pub use lfg_dungeons_adapter::*;
pub use skill_catalog_adapter::*;
