mod operations;
mod loot_release;
pub use loot_release::{LootReleaseAccessLikeCpp, LootReleaseOwnerAccessLikeCpp, LootReleaseStatsInputsLikeCpp, looted_corpse_decay_secs_like_cpp};
mod aura_removal;
pub use aura_removal::{PlayerAuraRemovalAccessLikeCpp, AuraMountControlAccessLikeCpp, AuraStatsAccessBuilderLikeCpp};
pub use aura_removal::{AuraNpcAccessBuilderLikeCpp, AuraConditionAccessBuilderLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
pub use aura_removal::{AuraRemovalFixtureRefsLikeCpp, AuraMountControlFixtureRefsLikeCpp, AuraDismountPetFixtureRefsLikeCpp};
mod equipment_sets;
mod instance_locks;
mod group_owner;
mod instance_player;
mod group_difficulty;
mod inventory;
mod inventory_valuation;
mod item_modifiers;
mod item_enchantment;
pub use item_enchantment::{OwnedItemEnchantmentAccessLikeCpp, apply_enchantment_template_from_store_like_cpp};
mod item_sets;
mod collections;
mod spell_acquisition;
mod inventory_projection;
mod acquisition_owner;
mod trainer_interaction;
mod registry_hydration;
mod quest_reward_owner;
mod player_stats;
mod quest_objectives;
mod quest_eligibility;
mod trainer_npc;
mod equipment_set_use;
mod player_condition;
mod xp_gain;

pub use equipment_sets::OwnedEquipmentSetsAccessLikeCpp;
pub use instance_locks::InstanceLockManagerAccessLikeCpp;
pub use group_owner::PlayerGroupOwnerAccessLikeCpp;
pub use instance_player::InstancePlayerAccessLikeCpp;
pub use group_difficulty::GroupDifficultyAccessLikeCpp;
pub use acquisition_owner::PlayerAcquisitionOwnerAccessLikeCpp;
pub use trainer_interaction::TrainerInteractionRoleAccessLikeCpp;
pub use inventory::OwnedInventoryAccessLikeCpp;
pub use inventory_valuation::InventoryValuationAccessLikeCpp;
pub use item_modifiers::OwnedItemModifiersAccessLikeCpp;
pub use item_sets::OwnedItemSetAccessLikeCpp;
pub use collections::OwnedCollectionsAccessLikeCpp;
pub use spell_acquisition::OwnedSpellAcquisitionAccessLikeCpp;
pub use inventory_projection::InventoryPlayerProjectionLikeCpp;
pub use registry_hydration::PlayerRegistryHydrationAccessLikeCpp;
pub use quest_reward_owner::{OwnedPlayerCurrencyAccessLikeCpp, QuestRewardPlayerAccessLikeCpp};
pub use player_stats::PlayerStatsAccessLikeCpp;
pub use quest_objectives::QuestObjectiveAccessLikeCpp;
pub use quest_eligibility::QuestEligibilityAccessLikeCpp;
pub use trainer_npc::NpcInteractionAccessLikeCpp;
pub use equipment_set_use::EquipmentSetCombatAccessLikeCpp;
pub use equipment_set_use::EquipmentSetUseAccessLikeCpp;
pub use player_condition::PlayerConditionAccessLikeCpp;
pub use xp_gain::CoreXPGainAccessLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use player_stats::{StatsAuraFixtureRefs, StatsCombatFixtureRefs, StatsFixtureRefs};
#[cfg(any(test, feature = "test-fixtures"))]
pub use trainer_npc::NpcInteractionFixtureRefsLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use player_condition::PlayerConditionFixtureRefsLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use xp_gain::CoreXPGainFixtureRefsLikeCpp;
