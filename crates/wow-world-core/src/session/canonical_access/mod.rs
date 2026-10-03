mod operations;
mod equipment_sets;
mod instance_locks;
mod group_owner;
mod instance_player;
mod group_difficulty;
mod inventory;
mod item_modifiers;
mod item_sets;
mod collections;
mod spell_acquisition;
mod inventory_projection;
mod acquisition_owner;
mod registry_hydration;
mod quest_reward_owner;
mod player_stats;
mod trainer_npc;

pub use equipment_sets::OwnedEquipmentSetsAccessLikeCpp;
pub use instance_locks::InstanceLockManagerAccessLikeCpp;
pub use group_owner::PlayerGroupOwnerAccessLikeCpp;
pub use instance_player::InstancePlayerAccessLikeCpp;
pub use group_difficulty::GroupDifficultyAccessLikeCpp;
pub use acquisition_owner::PlayerAcquisitionOwnerAccessLikeCpp;
pub use inventory::OwnedInventoryAccessLikeCpp;
pub use item_modifiers::OwnedItemModifiersAccessLikeCpp;
pub use item_sets::OwnedItemSetAccessLikeCpp;
pub use collections::OwnedCollectionsAccessLikeCpp;
pub use spell_acquisition::OwnedSpellAcquisitionAccessLikeCpp;
pub use inventory_projection::InventoryPlayerProjectionLikeCpp;
pub use registry_hydration::PlayerRegistryHydrationAccessLikeCpp;
pub use quest_reward_owner::{OwnedPlayerCurrencyAccessLikeCpp, QuestRewardPlayerAccessLikeCpp};
pub use player_stats::PlayerStatsAccessLikeCpp;
pub use trainer_npc::NpcInteractionAccessLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use player_stats::{StatsAuraFixtureRefs, StatsCombatFixtureRefs, StatsFixtureRefs};
#[cfg(any(test, feature = "test-fixtures"))]
pub use trainer_npc::NpcInteractionFixtureRefsLikeCpp;

mod equipment_set_use;
pub use equipment_set_use::{EquipmentSetCombatAccessLikeCpp, EquipmentSetUseAccessLikeCpp};
