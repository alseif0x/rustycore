//! Player modules.
//!
//! Grouped from the player_* siblings under #695; each module keeps its
//! items and its re-exported names.

pub mod base_stats;
pub mod choice;
pub mod creation_catalog;
pub mod economy;
pub mod inventory;
pub mod lifecycle;
pub mod login;
pub mod name_query;
pub mod quest;
pub mod save;

pub use base_stats::*;
pub use choice::*;
pub use creation_catalog::*;
pub use economy::*;
pub use inventory::*;
pub use lifecycle::*;
pub use login::*;
pub use name_query::*;
pub use quest::*;
pub use save::*;
