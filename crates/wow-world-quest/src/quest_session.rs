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
//! form [`QuestItemStorageHostLikeCpp`]. Slice 6c added the quest-giver,
//! acceptance, reward and sharing packet entry points
//! (`HandleQuestgiver*Opcode`, `HandleQuestConfirmAccept`,
//! `HandlePushQuestToParty`, `HandleQuestPOIQuery`,
//! `HandleAdventureMapStartQuest`); the session operations only they reach form
//! [`QuestHandlerSessionHostLikeCpp`]. The objective drain and the reward
//! commit lend the World session's owners and stay host operations.

mod accept_handlers;
mod dialog;
mod eligibility;
mod giver;
mod giver_handlers;
mod item_mutations;
mod objectives;
mod persistence;
mod reward_handlers;
mod reward_validation;
mod rewards;
mod sharing;
mod source_items;
mod spell_effect;
mod state;
pub mod unit_update_publication;

pub use accept_handlers::*;
pub use dialog::*;
pub use eligibility::*;
pub use giver::*;
pub use giver_handlers::*;
pub use item_mutations::*;
pub use objectives::*;
pub use persistence::*;
pub use reward_handlers::*;
pub use reward_validation::*;
pub use rewards::*;
pub use sharing::*;
pub use source_items::*;
pub use spell_effect::*;
pub use state::*;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use tracing::{debug, info, warn};
use wow_conditions::{
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    QUEST_STATUS_NONE_LIKE_CPP, QUEST_STATUS_REWARDED_LIKE_CPP,
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
use wow_packet::packets::gossip::GossipMessage;
use wow_packet::packets::query::{
    QuestPoiBlobData, QuestPoiBlobPoint, QuestPoiData, QuestPoiQuery, QuestPoiQueryResponse,
};
use wow_packet::packets::quest::{
    AdventureMapStartQuest, PushQuestToParty, QuestConfirmAccept, QuestGiverQuestComplete,
    QuestGiverStatus, QuestPushResultResponse, QuestRewardsBlock, quest_push_reason,
};
use wow_packet::packets::quest::{
    QuestGiverOfferReward, QuestGiverQuestDetails, QuestGiverQuestFailed, QuestGiverQuestList,
    QuestGiverRequestItems, QuestGiverRequestItemsCollect, QuestGiverRequestItemsCurrency,
    QuestListEntry, QuestObjectiveSimple,
};
use wow_packet::packets::update::{ItemCreateData, ItemEnchantmentValuesUpdate, UpdateObject};
use wow_packet::{ClientPacket, ServerPacket};
use wow_world_core::session::mailbox::{
    SendRepeatableTurnInRequestItemsLikeCppCommand, SessionCommand,
    SetQuestSharingInfoAndSendDetailsCommand,
};
use wow_world_core::session::{
    HubMut, HubRef, RepresentedGameObjectAccessLikeCpp, SessionCatalogs, SessionHubHostLikeCpp,
};
use wow_world_entities::{RepresentedGameObjectUseEffect, WorldEntitiesState};
use wow_world_instances::RepresentedAdventureMapStartQuestLikeCpp;
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_player_conditions::PlayerConditionProjectionCxLikeCpp;

// Only the pending-sharing fixture branch names it (#1363 slice 6c).
#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedPendingQuestSharingLikeCpp;
use crate::{
    MAX_QUEST_LOG_SIZE_LIKE_CPP, QuestSourceItemBoundPersistencePlanLikeCpp,
    QuestSourceItemBoundPreflightLikeCpp, RepresentedPushQuestToPartyOutcomeLikeCpp,
    RepresentedPushQuestToPartyOutcomeReasonLikeCpp, RepresentedQuestConfirmAcceptLikeCpp,
    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp, RepresentedQuestGiverStatusSourceLikeCpp,
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
    /// Hook port for `player.quest_accepted` (#1444 H1): AzerothCore
    /// `OnPlayerQuestAccept` (`PlayerQuest.cpp:426`). The World adapter
    /// dispatches it to the trusted linked modules; the default does nothing.
    /// Called after the accepted status was saved, outside any split borrow.
    fn quest_accepted_hook_like_cpp(
        &mut self,
        _event: &crate::module_hooks::QuestAcceptedHookEventLikeCpp,
    ) {
    }
    /// Whether any linked module wants `player.quest_accepted`; asked before
    /// the event is built. The default, used by every test host, wants nothing.
    fn quest_accepted_hook_wanted_like_cpp(&self) -> bool {
        false
    }
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
    // The quest-status authority reset with the spell-hit authority
    // invalidation (#1363 slice 12d; the World inherent of that name, moved
    // here as the provided method).
    fn invalidate_player_quest_status_authority_like_cpp(&mut self) {
        let (quest_state, hub) = self.split_quest_state_mut();
        let _ = crate::mutate_player_quest_gameplay_like_cpp(hub, quest_state, |state| {
            state.set_status_authority_complete_like_cpp(false);
        });
        self.core()
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
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
    fn player_condition_projection_cx_like_cpp(&self) -> PlayerConditionProjectionCxLikeCpp<'_>;
    fn player_target_spell_is_hit_inert_like_cpp(&self, spell_id: u32, difficulty_id: u8) -> bool;
    /// The World spell-click visibility of one creature for this viewer (the
    /// many-field condition builder the moved viewer-dependent NPC flags call).
    fn represented_can_see_spell_click_on_creature_like_cpp(
        &self,
        creature_guid: ObjectGuid,
    ) -> wow_world_entities::spell_click_values::RepresentedCanSeeSpellClickOutcomeLikeCpp;
}

/// The World inventory and item operations the moved item-objective, source-item
/// and reward-inventory bodies reach; each is the same-named `WorldSession`
/// operation (`make_inventory_item_object` is the World inventory context's).
pub trait QuestItemStorageHostLikeCpp: QuestSessionHostLikeCpp {
    /// The World `catalogs` field, written by the item-template-addon
    /// quest-log item cache and the quest POI store load.
    fn catalogs_mut(&mut self) -> &mut SessionCatalogs;
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

/// The World operations only the moved quest packet entry points reach
/// (#1363 slice 6c); each is the same-named `WorldSession` field, split or
/// operation. The reward commit and the objective drain lend the session's
/// owners, so they stay World operations.
pub trait QuestHandlerSessionHostLikeCpp: QuestItemStorageHostLikeCpp {
    /// The World `quest_state` field, written by the represented outcome
    /// records and the pending-sharing fixture.
    fn quest_state_mut(&mut self) -> &mut SessionQuestState;
    /// The World `world_entities` field.
    fn world_entities(&self) -> &WorldEntitiesState;
    /// The World `split_world_entities_mut` split.
    fn split_world_entities_mut(&mut self) -> (&mut WorldEntitiesState, HubMut<'_>);
    fn resolved_group_guid_like_cpp(&self) -> Option<u64>;
    /// `wow_world_character::build_gossip_menu` over the World session.
    fn build_gossip_menu(
        &mut self,
        entry: u32,
        npc_flags: u32,
        npc_guid: ObjectGuid,
    ) -> impl Future<Output = Option<GossipMessage>> + Send;
    /// `wow_world_character::send_represented_creature_trainer_gossip_menu_like_cpp`
    /// over the World session.
    fn send_represented_creature_trainer_gossip_menu_like_cpp(
        &mut self,
        npc_guid: ObjectGuid,
        entry: u32,
        npc_flags: u32,
    ) -> bool;
    fn record_represented_adventure_map_start_quest_like_cpp(
        &mut self,
        request: RepresentedAdventureMapStartQuestLikeCpp,
    );
    /// C++ `Player::RewardQuest`: the World reward commit.
    fn reward_represented_quest_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: ObjectGuid,
        choice: QuestChoiceItemLikeCpp,
    ) -> impl Future<Output = bool> + Send;
    /// The World objective drain.
    fn drain_represented_quest_objective_progress_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send;
}
