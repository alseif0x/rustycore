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
//! body called it before the move. The quest eligibility, objective progress,
//! source-item storage, reward commit and the packet handlers stay with the
//! host for now (slice 6b).

mod dialog;
mod giver;
mod item_mutations;
mod persistence;
mod rewards;
mod state;

pub use dialog::*;
pub use giver::*;
pub use item_mutations::*;
pub use persistence::*;
pub use rewards::*;
pub use state::*;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use tracing::{debug, info, warn};
use wow_conditions::{
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    QUEST_STATUS_REWARDED_LIKE_CPP,
};
use wow_constants::quest::{
    QUEST_OBJECTIVE_CURRENCY_LIKE_CPP, QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP,
    QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP as QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_ITEM_LIKE_CPP,
    QUEST_OBJECTIVE_ITEM_LIKE_CPP as QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_MONEY_LIKE_CPP,
};
use wow_constants::unit::NPCFlags1;
use wow_core::ObjectGuid;
use wow_data::progression_rewards::QuestV2Store;
use wow_entities::{
    PlayerQuestGameplayState, PlayerQuestStatusRecord as PlayerQuestStatus,
    QUESTS_COMPLETED_BITS_PER_BLOCK, QUESTS_COMPLETED_BITS_SIZE,
};
use wow_packet::packets::quest::{
    QuestGiverOfferReward, QuestGiverQuestDetails, QuestGiverQuestList, QuestGiverRequestItems,
    QuestGiverRequestItemsCollect, QuestGiverRequestItemsCurrency, QuestListEntry,
    QuestObjectiveSimple,
};
use wow_world_core::session::{
    HubMut, HubRef, RepresentedGameObjectAccessLikeCpp, SessionHubHostLikeCpp,
};
use wow_world_entities::{RepresentedGameObjectUseEffect, WorldEntitiesState};
use wow_world_lifecycle::SessionLifecycleState;

use crate::{
    MAX_QUEST_LOG_SIZE_LIKE_CPP, QuestSourceItemBoundPersistencePlanLikeCpp, SessionQuestState,
    quest_giver_creature_id_from_source_like_cpp,
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
    /// The World crate's `cfg!(test)`.
    fn world_test_consumer_like_cpp(&self) -> bool;
    fn player_quest_gameplay_snapshot_like_cpp(&self) -> Option<PlayerQuestGameplayState>;
    fn mutate_player_quest_gameplay_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut PlayerQuestGameplayState) -> R,
    ) -> Option<R>;
    fn invalidate_player_quest_status_authority_like_cpp(&mut self);
    /// C++ `Player::CanTakeQuest`, over the World eligibility context.
    fn can_take_quest(&self, quest: &wow_data::quest::QuestTemplate) -> bool;
    fn represented_quest_is_important_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool;
    fn represented_inventory_item_counts_like_cpp(&self) -> Option<HashMap<u32, u32>>;
    /// C++ `Player::CanInteractWithQuestGiver(TYPEID_GAMEOBJECT)`, over the
    /// World quest-state context.
    fn represented_gameobject_questgiver_can_interact_with_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectAccessLikeCpp>;
}
