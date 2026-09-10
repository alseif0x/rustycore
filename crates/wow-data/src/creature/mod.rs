//! Creature modules.
//!
//! Grouped from the creature_* siblings under #697; each module keeps its
//! items and its re-exported names.

pub mod display;
pub mod equipment;
pub mod model_info;
pub mod template;

pub use display::*;
pub use equipment::*;
pub use model_info::*;
pub use template::*;
