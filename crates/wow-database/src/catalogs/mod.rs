//! Catalog persistence adapters.
//!
//! Grouped from the *_catalog_adapter siblings under #703; each module keeps its
//! items and its re-exported names.

pub mod area_trigger_template_adapter;
pub mod area_trigger_world_adapter;
pub mod canonical_spawn_adapter;
pub mod condition_disable_adapter;
pub mod exploration_base_xp_adapter;
pub mod gameplay_rule_adapter;
pub mod gossip_adapter;
pub mod item_random_enchantment_adapter;
pub mod item_template_addon_adapter;
pub mod jump_charge_adapter;
pub mod lfg_world_adapter;
pub mod loot_template_adapter;
pub mod mount_adapter;
pub mod phase_hotfix_adapter;
pub mod phase_world_adapter;
pub mod reputation_adapter;
pub mod reserved_name_adapter;
pub mod trainer_adapter;
pub mod vehicle_adapter;
pub mod vendor_adapter;
pub mod visibility_spawn_adapter;

pub use area_trigger_template_adapter::*;
pub use area_trigger_world_adapter::*;
pub use canonical_spawn_adapter::*;
pub use condition_disable_adapter::*;
pub use exploration_base_xp_adapter::*;
pub use gameplay_rule_adapter::*;
pub use gossip_adapter::*;
pub use item_random_enchantment_adapter::*;
pub use item_template_addon_adapter::*;
pub use jump_charge_adapter::*;
pub use lfg_world_adapter::*;
pub use loot_template_adapter::*;
pub use mount_adapter::*;
pub use phase_hotfix_adapter::*;
pub use phase_world_adapter::*;
pub use reputation_adapter::*;
pub use reserved_name_adapter::*;
pub use trainer_adapter::*;
pub use vehicle_adapter::*;
pub use vendor_adapter::*;
pub use visibility_spawn_adapter::*;
