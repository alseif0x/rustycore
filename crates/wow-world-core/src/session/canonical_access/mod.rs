mod loot_release;
mod operations;
pub use loot_release::{
    LootReleaseAccessLikeCpp, LootReleaseOwnerAccessLikeCpp, LootReleaseStatsInputsLikeCpp,
    looted_corpse_decay_secs_like_cpp,
};
mod aura_removal;
pub use aura_removal::{AuraConditionAccessBuilderLikeCpp, AuraNpcAccessBuilderLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
pub use aura_removal::{
    AuraDismountPetFixtureRefsLikeCpp, AuraMountControlFixtureRefsLikeCpp,
    AuraRemovalFixtureRefsLikeCpp,
};
pub use aura_removal::{
    AuraMountControlAccessLikeCpp, AuraStatsAccessBuilderLikeCpp, PlayerAuraRemovalAccessLikeCpp,
};
mod equipment_sets;
mod group_difficulty;
mod group_owner;
mod instance_locks;
mod instance_player;
mod inventory_valuation;
mod item_enchantment;
mod item_modifiers;
mod owned_inventory;
pub use item_enchantment::{
    OwnedItemEnchantmentAccessLikeCpp, apply_enchantment_template_from_store_like_cpp,
};
mod acquisition_owner;
mod collections;
mod equipment_set_use;
mod inventory_projection;
mod item_sets;
mod player_condition;
mod player_stats;
mod quest_eligibility;
mod quest_objectives;
mod quest_reward_owner;
mod registry_hydration;
mod spell_acquisition;
mod trainer_interaction;
mod trainer_npc;
mod xp_gain;

pub use acquisition_owner::PlayerAcquisitionOwnerAccessLikeCpp;
pub use collections::OwnedCollectionsAccessLikeCpp;
pub use equipment_set_use::EquipmentSetCombatAccessLikeCpp;
pub use equipment_set_use::EquipmentSetUseAccessLikeCpp;
pub use equipment_sets::OwnedEquipmentSetsAccessLikeCpp;
pub use group_difficulty::GroupDifficultyAccessLikeCpp;
pub use group_owner::PlayerGroupOwnerAccessLikeCpp;
pub use instance_locks::InstanceLockManagerAccessLikeCpp;
pub use instance_player::InstancePlayerAccessLikeCpp;
pub use inventory_projection::InventoryPlayerProjectionLikeCpp;
pub use inventory_valuation::InventoryValuationAccessLikeCpp;
pub use item_modifiers::OwnedItemModifiersAccessLikeCpp;
pub use item_sets::OwnedItemSetAccessLikeCpp;
pub use owned_inventory::OwnedInventoryAccessLikeCpp;
pub use player_condition::PlayerConditionAccessLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use player_condition::PlayerConditionFixtureRefsLikeCpp;
pub use player_stats::PlayerStatsAccessLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use player_stats::{StatsAuraFixtureRefs, StatsCombatFixtureRefs, StatsFixtureRefs};
pub use quest_eligibility::QuestEligibilityAccessLikeCpp;
pub use quest_objectives::QuestObjectiveAccessLikeCpp;
pub use quest_reward_owner::{OwnedPlayerCurrencyAccessLikeCpp, QuestRewardPlayerAccessLikeCpp};
pub use registry_hydration::PlayerRegistryHydrationAccessLikeCpp;
pub use spell_acquisition::OwnedSpellAcquisitionAccessLikeCpp;
pub use trainer_interaction::TrainerInteractionRoleAccessLikeCpp;
pub use trainer_npc::NpcInteractionAccessLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use trainer_npc::NpcInteractionFixtureRefsLikeCpp;
pub use xp_gain::CoreXPGainAccessLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use xp_gain::CoreXPGainFixtureRefsLikeCpp;
