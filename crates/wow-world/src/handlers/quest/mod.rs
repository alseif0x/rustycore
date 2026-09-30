// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest handlers, organised by feature.
//!
//! Issue #224 split the former 8,274-line `handlers/quest.rs` into private
//! feature modules. The logical owner, every registration, opcode and
//! dispatcher arm are unchanged; this module keeps the shared constants,
//! helper types and free functions the features build on.

pub(crate) mod dialog_status;
mod eligibility;
mod handlers;
mod objectives;
mod persistence;
mod rewards;
mod sharing;
mod source_items;
mod state;
mod presentation;
pub(crate) use presentation::{quest_giver_creature_id_from_source_like_cpp, represented_quest_completion_npc_response_like_cpp};
pub(crate) use eligibility::represented_satisfy_quest_dependent_previous_quests_failed_like_cpp;
use eligibility::{player_race_or_class_mask_like_cpp, represented_satisfy_quest_dependent_previous_quests_failed_with_rules, represented_satisfy_quest_dependent_breadcrumb_quests_failed_like_cpp, represented_can_take_quest_after_expansion_like_cpp};
use rewards::{player_quest_level_like_cpp, reputation_rank_from_standing_like_cpp};

use crate::session::mailbox::SessionCommand;
use crate::session::mailbox::{
    SendRepeatableTurnInRequestItemsLikeCppCommand, SetQuestSharingInfoAndSendDetailsCommand,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tracing::{debug, info, warn};
use wow_constants::item::ItemFlags3;
use wow_constants::unit::NPCFlags1;
use wow_constants::{
    ClientOpcodes, InventoryResult, ItemBondingType, ItemContext, ItemFieldFlags, ItemFlags2,
};
use wow_core::{GameTime, ObjectGuid};
use wow_data::{
    DISABLE_TYPE_QUEST,
    progression_rewards::{
        QUEST_PACKAGE_FILTER_CLASS_LIKE_CPP, QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP,
        QUEST_PACKAGE_FILTER_LOOT_SPECIALIZATION_LIKE_CPP, QuestPackageItemEntry,
    },
    quest::QuestStore,
    reputation::reputation_rank_from_standing_like_cpp as reputation_rank_from_standing_data_like_cpp,
};
pub use wow_entities::PlayerQuestStatusRecord as PlayerQuestStatus;
use wow_entities::{
    ItemPosCount, PlayerQuestGameplayState, SendNewItemDelivery, SendNewItemDisplayText,
    SendNewItemInstancePlan, SendNewItemModifier, SendNewItemPlan, is_bag_pos,
};
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_data_model::quest_poi::QuestPoiData;
use wow_packet::ServerPacket;
use wow_packet::packets::misc::SetCurrency;
use wow_packet::packets::query::{
    QueryQuestCompletionNpcs, QuestCompletionNpc, QuestCompletionNpcResponse, QuestPoiQuery,
    QuestPoiQueryResponse,
};
use wow_packet::packets::quest::{
    AdventureMapStartQuest, PushQuestToParty, QueryQuestInfoResponse, QuestConfirmAccept,
    QuestGiverOfferReward, QuestGiverQuestComplete, QuestGiverQuestFailed, QuestGiverRequestItems,
    QuestGiverStatus, QuestObjectiveInfo, QuestPushResult, QuestPushResultResponse,
    QuestRewardsBlock, QuestUpdateComplete, WorldQuestUpdateResponse, quest_giver_status,
    quest_push_reason,
};
use wow_packet::packets::update::{
    ItemCreateData, ItemEnchantmentValuesUpdate, PlayerDataValuesDeltaUpdate, QuestLogValuesUpdate,
    UpdateObject,
};

use crate::handlers::character::ExtendedCostItemTurninChange;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::RepresentedQuestRewardReputationLikeCpp;
use crate::session::{
    CurrencyGainSourceLikeCpp, InventoryItem, RepresentedAdventureMapStartQuestLikeCpp,
    RepresentedPushQuestToPartyOutcomeLikeCpp, RepresentedPushQuestToPartyOutcomeReasonLikeCpp,
    RepresentedQuestCompleteStatusUpdateLikeCpp, RepresentedQuestConfirmAcceptLikeCpp,
    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    RepresentedQuestObjectiveProgressEventLikeCpp, RepresentedQuestPushResultResponseLikeCpp,
    RepresentedQuestRewardReputationSourceLikeCpp, ReputationGainSourceLikeCpp,
    SeasonalQuestStatusDbRowLikeCpp, WorldSession,
};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::{
    RepresentedQuestRewardMailLikeCpp, RepresentedQuestRewardTalentPointsLikeCpp,
    RepresentedQuestRewardTitleLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::{
    RepresentedQuestRewardSpellCastLikeCpp, RepresentedQuestRewardSpellKindLikeCpp,
};
use wow_conditions::{
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    QUEST_STATUS_NONE_LIKE_CPP, QUEST_STATUS_REWARDED_LIKE_CPP,
};

pub(crate) const QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP: u32 = 0x0001_0000;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) const QUEST_FLAGS_PLAYER_CAST_COMPLETE_LIKE_CPP: u32 = 0x0020_0000;
pub(crate) const QUEST_FLAGS_SHARABLE_LIKE_CPP: u32 = 0x0000_0008;
pub(crate) const QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP: u32 = 0x0000_0400;
pub(crate) use wow_constants::quest::{
    QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP, QUEST_FLAGS_EX_REWARDS_IGNORE_CAPS_LIKE_CPP,
};
use wow_constants::quest::{
    QUEST_STATE_COMPLETE as QUEST_STATE_COMPLETE_LIKE_CPP,
    QUEST_STATE_FAIL as QUEST_STATE_FAIL_LIKE_CPP,
    QUEST_STATE_OBJECTIVE_FLAG_BASE as QUEST_STATE_OBJECTIVE_FLAG_BASE_LIKE_CPP,
};
pub(crate) const QUEST_PUSH_REASON_INVALID_LIKE_CPP: u8 = 1;
pub(crate) const QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP: u8 = 2;
pub(crate) const QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL: u8 = 4;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) const QUEST_OBJECTIVE_MONEY_LIKE_CPP_LOCAL: u8 = 8;
pub(crate) use wow_constants::quest::{
    QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM as QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
    QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY as QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
};
const QUEST_FLAGS_REMOVE_SURPLUS_ITEMS_LIKE_CPP: u32 = 0x0200_0000;
const QUEST_FLAGS_EX_NO_ITEM_REMOVAL_LIKE_CPP: u32 = 0x0000_0001;
pub(crate) const CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP: i32 = 3;

use wow_packet::packets::quest::QuestChoiceItem as QuestChoiceItemLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuestSourceItemStoreOutcomeLikeCpp {
    StoredNewItem,
    BoundObjectiveNoGrant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QuestSourceItemBoundPreflightLikeCpp {
    pub(crate) no_grant: bool,
    pub(crate) changed_quest_ids: Vec<u32>,
}

/// Durable snapshot of C++ `StoreNewItem`'s first, quest-bound
/// `ItemAddedQuestCheck` pass. A single matching bound objective consumes the
/// loot award as quest credit without materialising an inventory Item.
#[derive(Debug, Clone)]
pub(crate) struct QuestSourceItemBoundPersistencePlanLikeCpp {
    pub(crate) statuses: Vec<PlayerQuestStatus>,
}

pub(crate) const QUEST_PUSH_REASON_BUSY_LIKE_CPP: u8 = 5;
pub(crate) const QUEST_PUSH_REASON_DEAD_LIKE_CPP: u8 = 6;
pub(crate) const QUEST_PUSH_REASON_DEAD_TO_RECIPIENT_LIKE_CPP: u8 = 7;
pub(crate) const QUEST_PUSH_REASON_LOG_FULL_LIKE_CPP: u8 = 8;
pub(crate) const QUEST_PUSH_REASON_LOG_FULL_TO_RECIPIENT_LIKE_CPP: u8 = 9;
pub(crate) const QUEST_PUSH_REASON_ON_QUEST_LIKE_CPP: u8 = 10;
pub(crate) const QUEST_PUSH_REASON_ON_QUEST_TO_RECIPIENT_LIKE_CPP: u8 = 11;
pub(crate) const QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP: u8 = 12;
pub(crate) const QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP: u8 = 13;
pub(crate) const QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP: u8 = 20;
pub(crate) const QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP: u8 = 21;
pub(crate) const QUEST_PUSH_REASON_LOW_LEVEL_LIKE_CPP: u8 = 22;
pub(crate) const QUEST_PUSH_REASON_LOW_LEVEL_TO_RECIPIENT_LIKE_CPP: u8 = 23;
pub(crate) const QUEST_PUSH_REASON_HIGH_LEVEL_LIKE_CPP: u8 = 24;
pub(crate) const QUEST_PUSH_REASON_HIGH_LEVEL_TO_RECIPIENT_LIKE_CPP: u8 = 25;
pub(crate) const QUEST_PUSH_REASON_CLASS_LIKE_CPP: u8 = 26;
pub(crate) const QUEST_PUSH_REASON_CLASS_TO_RECIPIENT_LIKE_CPP: u8 = 27;
pub(crate) const QUEST_PUSH_REASON_RACE_LIKE_CPP: u8 = 28;
pub(crate) const QUEST_PUSH_REASON_RACE_TO_RECIPIENT_LIKE_CPP: u8 = 29;
pub(crate) const QUEST_PUSH_REASON_LOW_FACTION_LIKE_CPP: u8 = 30;
pub(crate) const QUEST_PUSH_REASON_LOW_FACTION_TO_RECIPIENT_LIKE_CPP: u8 = 31;
pub(crate) const QUEST_PUSH_REASON_EXPANSION_LIKE_CPP: u8 = 32;
pub(crate) const QUEST_PUSH_REASON_EXPANSION_TO_RECIPIENT_LIKE_CPP: u8 = 33;
pub(crate) const QUEST_PUSH_REASON_SUCCESS_LIKE_CPP: u8 = 0;

// ── Handler registrations ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedQuestGiverStatusSourceLikeCpp {
    Creature { entry: u32 },
    GameObject { entry: u32 },
}

impl RepresentedQuestGiverStatusSourceLikeCpp {
    fn entry(self) -> u32 {
        match self {
            Self::Creature { entry } | Self::GameObject { entry } => entry,
        }
    }

    fn kind_name(self) -> &'static str {
        match self {
            Self::Creature { .. } => "Creature",
            Self::GameObject { .. } => "GameObject",
        }
    }
}

pub(crate) use wow_constants::quest::MAX_QUEST_LOG_SIZE as MAX_QUEST_LOG_SIZE_LIKE_CPP;

use wow_constants::quest::{
    QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP as QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_ITEM_LIKE_CPP as QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
};
#[cfg(test)]
use wow_constants::quest::{
    QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP as QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP as QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP as QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_MONSTER_LIKE_CPP as QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP as QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL,
};

// ── PlayerQuestStatus ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct ItemTransferQuestPersistencePlanLikeCpp {
    statuses: HashMap<u32, PlayerQuestStatus>,
    changed_quest_ids: Vec<u32>,
}

#[cfg(feature = "test-fixtures")]
pub(crate) async fn quest_poi_store_for_test(
    session: &mut WorldSession,
) -> Arc<HashMap<i32, QuestPoiData>> {
    session.quest_poi_store_like_cpp().await
}

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) async fn save_quest_to_db_for_test(session: &WorldSession, quest_id: u32, status: u8) {
    session.save_quest_to_db(quest_id, status).await;
}
