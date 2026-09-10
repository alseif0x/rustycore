//! Game modules.
//!
//! Grouped from the game_* siblings under #695; each module keeps its
//! items and its re-exported names.

pub mod event_persistence_adapter;
pub mod event_world_catalog_adapter;
pub mod tele_catalog_adapter;

pub use event_persistence_adapter::*;
pub use event_world_catalog_adapter::*;
pub use tele_catalog_adapter::*;
