// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented quest state, persistence, quest-giver dialog and publication
//! bodies moved out of the World session (#1363 slice 6a).
//!
//! C++ runs these on the Player (`Player::_LoadQuestStatus`,
//! `_LoadSeasonalQuestStatus`, `ResetSeasonalQuestStatus`, `SetQuestStatus`,
//! `PrepareQuestMenu`, `SendPreparedQuest`, `CanRewardQuest`,
//! `GetQuestXPReward`, `LearnQuestRewardedSpells`) and on `PlayerMenu`
//! (`SendQuestGiverRequestItems`, `SendQuestGiverQuestDetails`,
//! `SendQuestGiverOfferReward`). Their Rust bodies now live here, unchanged
//! except for the access path: each [`QuestSessionHostLikeCpp`] method is the
//! same-named `WorldSession` operation or field, called at the exact point the
//! body called it before the move. Slice 6b added the item-objective progress
//! (`ItemAddedQuestCheck`, `ItemRemovedQuestCheck`), the quest-source item
//! grant, the reward-inventory admission and the eligibility gates
//! (`CanTakeQuest`, `CanSeeStartQuest`); the inventory operations those reach
//! form [`QuestItemStorageHostLikeCpp`]. The objective drain and the reward
//! commit lend the World session's owners and stay with the host, and the
//! packet handlers follow in slice 6c.

mod dialog;
mod eligibility;
mod giver;
mod item_mutations;
mod objectives;
mod persistence;
mod reward_validation;
mod rewards;
mod source_items;
mod state;

pub use dialog::*;
pub use eligibility::*;
pub use giver::*;
pub use item_mutations::*;
pub use objectives::*;
pub use persistence::*;
pub use reward_validation::*;
pub use rewards::*;
pub use source_items::*;
pub use state::*;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use tracing::{debug, info, warn};
use wow_conditions::{
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    QUEST_STATUS_REWARDED_LIKE_CPP,
};
use wow_constants::item::ItemFlags3;
use wow_constants::quest::{
    QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP, QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
};
use wow_constants::quest::{
    QUEST_OBJECTIVE_CURRENCY_LIKE_CPP, QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP,
    QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP as QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_ITEM_LIKE_CPP,
    QUEST_OBJECTIVE_ITEM_LIKE_CPP as QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_MONEY_LIKE_CPP,
};
use wow_constants::unit::NPCFlags1;
use wow_constants::{InventoryResult, ItemBondingType, ItemContext, ItemFieldFlags};
use wow_core::ObjectGuid;
use wow_data::progression_rewards::{QuestPackageItemEntry, QuestV2Store};
use wow_data::quest::QuestStore;
use wow_entities::{
    Item, ItemObjectUpdateLikeCpp, ItemPosCount, PlayerInventoryItem as InventoryItem,
    PlayerQuestGameplayState, PlayerQuestStatusRecord as PlayerQuestStatus,
    QUESTS_COMPLETED_BITS_PER_BLOCK, QUESTS_COMPLETED_BITS_SIZE, SendNewItemDelivery,
    SendNewItemDisplayText, SendNewItemInstancePlan, SendNewItemModifier, SendNewItemPlan,
    is_bag_pos,
};
use wow_packet::packets::quest::{
    QuestGiverOfferReward, QuestGiverQuestDetails, QuestGiverQuestFailed, QuestGiverQuestList,
    QuestGiverRequestItems, QuestGiverRequestItemsCollect, QuestGiverRequestItemsCurrency,
    QuestListEntry, QuestObjectiveSimple,
};
use wow_packet::packets::update::{ItemCreateData, ItemEnchantmentValuesUpdate, UpdateObject};
use wow_world_core::session::{
    HubMut, HubRef, RepresentedGameObjectAccessLikeCpp, SessionHubHostLikeCpp,
};
use wow_world_entities::{RepresentedGameObjectUseEffect, WorldEntitiesState};
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_player_conditions::PlayerConditionProjectionCxLikeCpp;

use crate::{
    MAX_QUEST_LOG_SIZE_LIKE_CPP, QuestSourceItemBoundPersistencePlanLikeCpp,
    QuestSourceItemBoundPreflightLikeCpp, RepresentedQuestGiverStatusSourceLikeCpp,
    SessionQuestState, quest_giver_creature_id_from_source_like_cpp,
};

/// The World operations and fields the moved quest bodies still reach.
///
/// Each method is the same-named `WorldSession` operation or field the body
/// called before the move; `world_test_consumer_like_cpp` is the World
/// crate's `cfg!(test)`, which the bodies passed to the quest-state APIs and
/// which gates their former `#[cfg(test)]` fixture steps.
pub trait QuestSessionHostLikeCpp: SessionHubHostLikeCpp {
    /// The World `quest_state` field.
    fn quest_state(&self) -> &SessionQuestState;
    /// The World `split_quest_state_ref` split.
    fn split_quest_state_ref(&self) -> (&SessionQuestState, HubRef<'_>);
    /// The World `split_quest_state_mut` split.
    fn split_quest_state_mut(&mut self) -> (&mut SessionQuestState, HubMut<'_>);
    /// The World `lifecycle` field.
    fn lifecycle(&self) -> &SessionLifecycleState;
    /// The World `world_entities` field.
    fn world_entities_mut(&mut self) -> &mut WorldEntitiesState;
    /// The World crate's `cfg!(test)`: only the World test build's session
    /// returns its fixtures from `test_fixtures_ref_like_cpp` (#1404 review).
    fn world_test_consumer_like_cpp(&self) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        let world_test = self.test_fixtures_ref_like_cpp().is_some();
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let world_test = false;
        world_test
    }
    fn player_quest_gameplay_snapshot_like_cpp(&self) -> Option<PlayerQuestGameplayState>;
    fn mutate_player_quest_gameplay_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut PlayerQuestGameplayState) -> R,
    ) -> Option<R>;
    fn invalidate_player_quest_status_authority_like_cpp(&mut self);
    fn represented_inventory_item_counts_like_cpp(&self) -> Option<HashMap<u32, u32>>;
    /// C++ `Player::CanInteractWithQuestGiver(TYPEID_GAMEOBJECT)`, over the
    /// World quest-state context.
    fn represented_gameobject_questgiver_can_interact_with_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectAccessLikeCpp>;
    /// C++ `Player::CompleteQuest` after an objective update: the World
    /// objective-drain operation, which lends the session's owners.
    fn complete_represented_quest_after_objective_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
        ignored_objective_id: u32,
    ) -> impl Future<Output = bool> + Send;
    fn sync_player_registry_state_like_cpp(&self);
    fn update_visible_gameobjects_or_spell_clicks_like_cpp(&self) -> usize;
    fn send_represented_quest_log_slot_update_like_cpp(&mut self, slot: u8);
    fn player_condition_projection_cx_like_cpp(&self) -> PlayerConditionProjectionCxLikeCpp<'_>;
    fn player_target_spell_is_hit_inert_like_cpp(&self, spell_id: u32, difficulty_id: u8) -> bool;
}

/// The World inventory and item operations the moved item-objective, source-item
/// and reward-inventory bodies reach; each is the same-named `WorldSession`
/// operation (`make_inventory_item_object` is the World inventory context's).
pub trait QuestItemStorageHostLikeCpp: QuestSessionHostLikeCpp {
    fn quest_source_item_quest_log_item_id_like_cpp(
        &mut self,
        entry_id: u32,
    ) -> impl Future<Output = u32> + Send;
    fn get_inventory_item_by_pos(&self, bag: u8, slot: u8) -> Option<InventoryItem>;
    fn resolved_inventory_item_like_cpp(&self, slot: u8) -> Option<InventoryItem>;
    fn resolved_inventory_item_object_like_cpp(&self, guid: ObjectGuid) -> Option<Item>;
    fn represented_non_bank_item_count_like_cpp(&self, entry_id: u32) -> Option<u32>;
    fn apply_inventory_item_object_updates_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        updates: &[ItemObjectUpdateLikeCpp],
    ) -> bool;
    fn insert_inventory_item_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem>;
    fn insert_inventory_item_object(&mut self, item: Item) -> Option<Item>;
    #[allow(clippy::too_many_arguments)]
    fn make_inventory_item_object(
        &self,
        item_guid: ObjectGuid,
        entry_id: u32,
        owner_guid: ObjectGuid,
        count: u32,
        durability: u32,
        context: ItemContext,
        slot: u8,
    ) -> Item;
    fn send_new_item_plan(&self, plan: &SendNewItemPlan);
    fn send_player_values_update_from_entity_bridge(
        &self,
        inv_slot_changes: &[(u8, ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) -> bool;
    fn plan_store_new_direct_inventory_item(
        &self,
        entry_id: u32,
        count: u32,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)>;
}
