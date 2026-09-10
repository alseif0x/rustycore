//! Player modules.
//!
//! Grouped from the player_* siblings under #695; each module keeps its
//! items and its re-exported names.

pub mod base_stats_adapter;
pub mod choice_catalog_adapter;
pub mod creation_catalog_adapter;
pub mod inventory_adapter;
pub mod lifecycle_adapter;
pub mod money_transaction_adapter;
pub mod name_query_adapter;
pub mod quest_adapter;
pub mod spell_acquisition_adapter;

pub use base_stats_adapter::*;
pub use choice_catalog_adapter::*;
pub use creation_catalog_adapter::*;
pub use inventory_adapter::*;
pub use lifecycle_adapter::*;
pub use money_transaction_adapter::*;
pub use name_query_adapter::*;
pub use quest_adapter::*;
pub use spell_acquisition_adapter::*;
