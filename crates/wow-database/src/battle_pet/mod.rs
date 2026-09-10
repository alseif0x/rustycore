//! Battle pet modules.
//!
//! Grouped from the battle_pet_* siblings under #695; each module keeps its
//! items and its re-exported names.

pub mod account_adapter;
pub mod purchase_adapter;
pub mod selection_catalog_adapter;

pub use account_adapter::*;
pub use purchase_adapter::*;
pub use selection_catalog_adapter::*;
