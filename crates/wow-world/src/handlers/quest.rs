// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest system handlers.
//!
//! Implements:
//!   CMSG_QUEST_GIVER_STATUS_QUERY  → SMSG_QUEST_GIVER_STATUS
//!   CMSG_QUEST_GIVER_HELLO         → SMSG_QUEST_GIVER_QUEST_LIST_MESSAGE
//!   CMSG_QUEST_GIVER_QUERY_QUEST   → SMSG_QUEST_GIVER_QUEST_DETAILS
//!   CMSG_QUEST_GIVER_ACCEPT_QUEST  → save to DB + player quest-log update
//!   CMSG_QUEST_LOG_REMOVE_QUEST    → remove from DB
//!   CMSG_QUERY_QUEST_INFO          → SMSG_QUERY_QUEST_INFO_RESPONSE
//!
//! Legacy non-canonical note: Game/Handlers/QuestHandler.cs

use sqlx::Row;
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
        QUEST_PACKAGE_FILTER_LOOT_SPECIALIZATION_LIKE_CPP, QuestInfoEntry, QuestPackageItemEntry,
    },
    quest::QuestStore,
    reputation::reputation_rank_from_standing_like_cpp as reputation_rank_from_standing_data_like_cpp,
};
use wow_database::{
    CharStatements, CharacterDatabase, PreparedStatement, SqlTransaction, WorldDatabase,
    WorldStatements,
};
use wow_entities::{
    ItemPosCount, SendNewItemDelivery, SendNewItemDisplayText, SendNewItemInstancePlan,
    SendNewItemModifier, SendNewItemPlan, is_bag_pos,
};
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_network::SessionCommand;
use wow_network::player_registry::{
    SendRepeatableTurnInRequestItemsLikeCppCommand, SetQuestSharingInfoAndSendDetailsCommand,
};
use wow_packet::packets::misc::SetCurrency;
use wow_packet::packets::query::{
    QueryQuestCompletionNpcs, QuestCompletionNpc, QuestCompletionNpcResponse, QuestPoiBlobData,
    QuestPoiBlobPoint, QuestPoiData, QuestPoiQuery, QuestPoiQueryResponse,
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
use wow_packet::{ClientPacket, ServerPacket};

use crate::conditions::{
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    QUEST_STATUS_NONE_LIKE_CPP, QUEST_STATUS_REWARDED_LIKE_CPP,
};
use crate::handlers::character::ExtendedCostItemTurninChange;
use crate::session::{
    CurrencyGainSourceLikeCpp, InventoryItem, RepresentedAdventureMapStartQuestLikeCpp,
    RepresentedPushQuestToPartyOutcomeLikeCpp, RepresentedPushQuestToPartyOutcomeReasonLikeCpp,
    RepresentedQuestCompleteStatusUpdateLikeCpp, RepresentedQuestConfirmAcceptLikeCpp,
    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    RepresentedQuestObjectiveProgressEventLikeCpp, RepresentedQuestPushResultResponseLikeCpp,
    RepresentedQuestRewardMailLikeCpp, RepresentedQuestRewardReputationLikeCpp,
    RepresentedQuestRewardReputationSourceLikeCpp, RepresentedQuestRewardSpellCastLikeCpp,
    RepresentedQuestRewardSpellKindLikeCpp, RepresentedQuestRewardTalentPointsLikeCpp,
    RepresentedQuestRewardTitleLikeCpp, ReputationGainSourceLikeCpp,
    SeasonalQuestStatusDbRowLikeCpp, WorldSession,
};

fn quest_giver_creature_id_from_source_like_cpp(source_guid: ObjectGuid) -> i32 {
    if source_guid.is_any_type_creature() {
        i32::try_from(source_guid.entry()).unwrap_or(0)
    } else {
        0
    }
}

pub(crate) const QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP: u32 = 0x0001_0000;
pub(crate) const QUEST_FLAGS_PLAYER_CAST_COMPLETE_LIKE_CPP: u32 = 0x0020_0000;
pub(crate) const QUEST_FLAGS_SHARABLE_LIKE_CPP: u32 = 0x0000_0008;
const QUEST_FLAGS_COMPLETION_EVENT_LIKE_CPP: u32 = 0x0000_0002;
const QUEST_FLAGS_COMPLETION_AREA_TRIGGER_LIKE_CPP: u32 = 0x0000_0004;
const QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP: u32 = 0x0000_0400;
const QUEST_FLAGS_EX_REWARDS_IGNORE_CAPS_LIKE_CPP: u32 = 0x0080_0000;
const QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP: u32 = 0x0100_0000;
const QUEST_STATE_COMPLETE_LIKE_CPP: u32 = 0x0001;
const QUEST_STATE_FAIL_LIKE_CPP: u32 = 0x0002;
const QUEST_STATE_OBJECTIVE_FLAG_BASE_LIKE_CPP: u32 = 256;
pub(crate) const QUEST_PUSH_REASON_INVALID_LIKE_CPP: u8 = 1;
pub(crate) const QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP: u8 = 2;
const QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL: u8 = 0;
const QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL: u8 = 1;
const QUEST_OBJECTIVE_GAMEOBJECT_LIKE_CPP_LOCAL: u8 = 2;
const QUEST_OBJECTIVE_TALKTO_LIKE_CPP_LOCAL: u8 = 3;
const QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL: u8 = 4;
#[cfg(test)]
const QUEST_OBJECTIVE_MONEY_LIKE_CPP_LOCAL: u8 = 8;
const QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP_LOCAL: u8 = 9;
const QUEST_OBJECTIVE_WINPVPPETBATTLES_LIKE_CPP_LOCAL: u8 = 13;
const QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP_LOCAL: u8 = 14;
const QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL: u8 = 15;
const QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP_LOCAL: u8 = 16;
const QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP_LOCAL: u8 = 17;
const QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP_LOCAL: u8 = 18;
const QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL: u32 = 0x2;
const QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP_LOCAL: u32 = 0x4;
const QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL: u32 = 0x40;
const QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL: u32 = 0x1;
const QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP: u8 = 0;
const QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP: u8 = 1;
const QUEST_FLAGS_REMOVE_SURPLUS_ITEMS_LIKE_CPP: u32 = 0x0200_0000;
const QUEST_FLAGS_EX_NO_ITEM_REMOVAL_LIKE_CPP: u32 = 0x0000_0001;
const CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP: i32 = 3;

fn read_quest_giver_query_quest_like_cpp(
    pkt: &mut wow_packet::WorldPacket,
) -> Result<(ObjectGuid, u32, bool), wow_packet::PacketError> {
    let guid = pkt.read_packed_guid()?;
    let quest_id = pkt.read_uint32().unwrap_or(0);
    let respond_to_giver = pkt.read_bit().unwrap_or(false);
    Ok((guid, quest_id, respond_to_giver))
}

fn read_quest_giver_accept_quest_like_cpp(
    pkt: &mut wow_packet::WorldPacket,
) -> Result<(ObjectGuid, u32, bool), wow_packet::PacketError> {
    let guid = pkt.read_packed_guid()?;
    let quest_id = pkt.read_uint32().unwrap_or(0);
    let start_cheat = pkt.read_bit().unwrap_or(false);
    Ok((guid, quest_id, start_cheat))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QuestChoiceItemLikeCpp {
    loot_item_type: u8,
    item_id: u32,
    quantity: i32,
}

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

fn reputation_rank_from_standing_like_cpp(standing: i32) -> u8 {
    reputation_rank_from_standing_data_like_cpp(standing).as_u8()
}

fn calculate_pct_i32_f32_like_cpp(base: i32, pct: f32) -> i32 {
    (base as f32 * pct / 100.0) as i32
}

fn player_quest_level_like_cpp(quest: &wow_data::quest::QuestTemplate, player_level: u8) -> i32 {
    if quest.quest_level > 0 {
        quest.quest_level
    } else {
        i32::from(player_level).min(quest.quest_max_scaling_level)
    }
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

fn player_race_or_class_mask_like_cpp(id: u8) -> u32 {
    if id == 0 {
        return 0;
    }

    1_u32
        .checked_shl(u32::from(id.saturating_sub(1)))
        .unwrap_or(0)
}

fn represented_satisfy_quest_dependent_previous_quests_failed_like_cpp(
    quest_store: &wow_data::quest::QuestStore,
    quest: &wow_data::quest::QuestTemplate,
    receiver_rewarded_quests: &std::collections::HashSet<u32>,
) -> bool {
    if quest.dependent_previous_quests.is_empty() {
        return false;
    }

    for &prev_id in &quest.dependent_previous_quests {
        let Some(previous_quest) = quest_store.get(prev_id) else {
            // C++ ASSERTs because ObjectMgr validates this at startup. Rust fails closed
            // as the prerequisite branch rather than panicking in the sender loop.
            return true;
        };

        if receiver_rewarded_quests.contains(&prev_id) {
            if previous_quest.exclusive_group >= 0 {
                return false;
            }

            for exclusive_quest_id in quest_store
                .quests
                .values()
                .filter(|candidate| candidate.exclusive_group == previous_quest.exclusive_group)
                .map(|candidate| candidate.id)
            {
                if exclusive_quest_id != prev_id
                    && !receiver_rewarded_quests.contains(&exclusive_quest_id)
                {
                    return true;
                }
            }

            return false;
        }
    }

    true
}

fn represented_satisfy_quest_dependent_breadcrumb_quests_failed_like_cpp(
    quest: &wow_data::quest::QuestTemplate,
    receiver_active_quest_statuses: &std::collections::HashMap<u32, u8>,
) -> bool {
    quest
        .dependent_breadcrumb_quests
        .iter()
        .any(|breadcrumb_quest_id| {
            matches!(
                receiver_active_quest_statuses
                    .get(breadcrumb_quest_id)
                    .copied(),
                Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP)
                    | Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
                    | Some(QUEST_STATUS_FAILED_LIKE_CPP)
            )
        })
}

fn represented_can_take_quest_after_expansion_like_cpp(
    quest_store: &wow_data::quest::QuestStore,
    quest: &wow_data::quest::QuestTemplate,
    receiver: &wow_network::PlayerQuestSharingSnapshot,
) -> bool {
    // C++ anchor: `Player::CanTakeQuest`, Player.cpp:14093-14102, after the
    // push handler has already emitted dedicated messages for class/race/level,
    // reputation, prerequisite, daily/DF, and expansion gates. This bounded
    // helper keeps the remaining represented `false` cases that TrinityCore
    // groups under `QuestPushReason::Invalid` at the final CanTakeQuest gate.
    // Explicitly out of scope for this represented-partial slice: DisableMgr,
    // skill, timed, weekly/monthly, ConditionMgr, and full seasonal runtime.
    let receiver_status = receiver
        .active_quest_statuses
        .get(&quest.id)
        .copied()
        .unwrap_or(QUEST_STATUS_NONE_LIKE_CPP);
    if receiver.rewarded_quests.contains(&quest.id) || receiver_status != QUEST_STATUS_NONE_LIKE_CPP
    {
        return false;
    }

    if quest.exclusive_group <= 0 {
        return true;
    }

    for peer_quest in quest_store
        .quests
        .values()
        .filter(|candidate| candidate.exclusive_group == quest.exclusive_group)
    {
        if peer_quest.id == quest.id {
            continue;
        }

        if peer_quest.is_df_quest_like_cpp() && receiver.df_quests.contains(&peer_quest.id) {
            return false;
        }
        if peer_quest.is_daily_like_cpp()
            && receiver.daily_quests_completed.contains(&peer_quest.id)
        {
            return false;
        }

        if receiver
            .active_quest_statuses
            .get(&peer_quest.id)
            .copied()
            .unwrap_or(QUEST_STATUS_NONE_LIKE_CPP)
            != QUEST_STATUS_NONE_LIKE_CPP
        {
            return false;
        }

        if !(quest.is_repeatable() && peer_quest.is_repeatable())
            && receiver.rewarded_quests.contains(&peer_quest.id)
        {
            return false;
        }
    }

    true
}

fn represented_quest_completion_npc_response_like_cpp(
    quest_store: &wow_data::quest::QuestStore,
    raw_quest_ids: &[i32],
) -> Vec<QuestCompletionNpc> {
    raw_quest_ids
        .iter()
        .filter_map(|&raw_quest_id| {
            let quest_id = u32::try_from(raw_quest_id).ok()?;
            if quest_store.get(quest_id).is_none() {
                return None;
            }

            let mut npcs = Vec::new();
            for creature_entry in quest_store.creature_ender_entries_for_quest_like_cpp(quest_id) {
                let Ok(entry) = i32::try_from(creature_entry) else {
                    debug!(
                        quest_id,
                        creature_entry,
                        "QueryQuestCompletionNPCs: creature entry exceeds signed i32 response field"
                    );
                    continue;
                };
                npcs.push(entry);
            }

            for go_entry in quest_store.gameobject_ender_entries_for_quest_like_cpp(quest_id) {
                npcs.push((go_entry | 0x8000_0000) as i32);
            }

            Some(QuestCompletionNpc {
                quest_id: raw_quest_id,
                npcs,
            })
        })
        .collect()
}

async fn load_quest_poi_store_like_cpp(
    world_db: &WorldDatabase,
) -> Result<HashMap<i32, QuestPoiData>, sqlx::Error> {
    let point_rows = sqlx::query(
        "SELECT QuestID, Idx1, X, Y, Z \
         FROM quest_poi_points \
         ORDER BY QuestID DESC, Idx1, Idx2",
    )
    .fetch_all(world_db.pool())
    .await?;

    let mut all_points: HashMap<(i32, i32), Vec<QuestPoiBlobPoint>> = HashMap::new();
    for row in point_rows {
        let quest_id: i32 = row.try_get(0)?;
        let idx1: i32 = row.try_get(1)?;
        let x: i32 = row.try_get(2)?;
        let y: i32 = row.try_get(3)?;
        let z: i32 = row.try_get(4)?;
        all_points
            .entry((quest_id, idx1))
            .or_default()
            .push(QuestPoiBlobPoint { x, y, z });
    }

    let poi_rows = sqlx::query(
        "SELECT QuestID, BlobIndex, Idx1, ObjectiveIndex, QuestObjectiveID, QuestObjectID, \
             MapID, UiMapID, Priority, Flags, WorldEffectID, PlayerConditionID, \
             NavigationPlayerConditionID, SpawnTrackingID, AlwaysAllowMergingBlobs \
         FROM quest_poi \
         ORDER BY QuestID, Idx1",
    )
    .fetch_all(world_db.pool())
    .await?;

    let mut store: HashMap<i32, QuestPoiData> = HashMap::new();
    for row in poi_rows {
        let quest_id: i32 = row.try_get(0)?;
        let blob_index: i32 = row.try_get(1)?;
        let idx1: i32 = row.try_get(2)?;
        let Some(points) = all_points.get(&(quest_id, idx1)).cloned() else {
            debug!(
                quest_id,
                blob_index, "quest_poi references unknown quest points like C++; skipping blob"
            );
            continue;
        };

        store
            .entry(quest_id)
            .or_insert_with(|| QuestPoiData {
                quest_id,
                blobs: Vec::new(),
            })
            .blobs
            .push(QuestPoiBlobData {
                blob_index,
                objective_index: row.try_get(3)?,
                quest_objective_id: row.try_get(4)?,
                quest_object_id: row.try_get(5)?,
                map_id: row.try_get(6)?,
                ui_map_id: row.try_get(7)?,
                priority: row.try_get(8)?,
                flags: row.try_get(9)?,
                world_effect_id: row.try_get(10)?,
                player_condition_id: row.try_get(11)?,
                navigation_player_condition_id: row.try_get(12)?,
                spawn_tracking_id: row.try_get(13)?,
                points,
                always_allow_merging_blobs: row.try_get::<u8, _>(14)? != 0,
            });
    }

    Ok(store)
}

// ── Handler registrations ────────────────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AdventureMapStartQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_adventure_map_start_quest",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverStatusQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_status_query",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverHello,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_hello",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverQueryQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_query_quest",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverAcceptQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_accept_quest",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestLogRemoveQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_log_remove_quest",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryQuestInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_quest_info",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryQuestCompletionNpcs,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_quest_completion_npcs",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestPoiQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_poi_query",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverRequestReward,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_request_reward",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverCompleteQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_complete_quest",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverChooseReward,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_choose_reward",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverCloseQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_close_quest",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestWorldQuestUpdate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_world_quest_update",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestConfirmAccept,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_confirm_accept",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestPushResult,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_push_result",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::PushQuestToParty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_push_quest_to_party",
    }
}

// ── Handler implementations ──────────────────────────────────────────────────

/// TrinityCore `MAX_QUEST_LOG_SIZE`; explicit quest-log slots are 0..24.
pub(crate) const MAX_QUEST_LOG_SIZE_LIKE_CPP: u8 = 25;

impl WorldSession {
    async fn quest_poi_store_like_cpp(&mut self) -> Arc<HashMap<i32, QuestPoiData>> {
        if let Some(store) = &self.quest_poi_store_like_cpp {
            return Arc::clone(store);
        }

        let Some(world_db) = self.world_db().map(Arc::clone) else {
            warn!("QuestPOIQuery: world DB unavailable; sending empty C++ response");
            let store = Arc::new(HashMap::new());
            self.quest_poi_store_like_cpp = Some(Arc::clone(&store));
            return store;
        };

        let store = match load_quest_poi_store_like_cpp(world_db.as_ref()).await {
            Ok(store) => Arc::new(store),
            Err(err) => {
                warn!("QuestPOIQuery: failed to load quest POI store like C++: {err}");
                Arc::new(HashMap::new())
            }
        };

        self.quest_poi_store_like_cpp = Some(Arc::clone(&store));
        store
    }

    fn bind_player_quest_status_load_guid_like_cpp(
        stmt: &mut PreparedStatement,
        player_guid: ObjectGuid,
    ) {
        stmt.set_u64(0, player_guid.counter() as u64);
    }

    fn represented_accept_and_end_time_for_new_quest_like_cpp(
        quest: &wow_data::quest::QuestTemplate,
    ) -> (i64, i64) {
        let accept_time = GameTime::now().as_secs() as i64;
        let end_time = if quest.limit_time_secs > 0 {
            accept_time.saturating_add(quest.limit_time_secs)
        } else {
            0
        };
        (accept_time, end_time)
    }

    pub(crate) fn represented_quest_objective_complete_like_cpp(
        status: &PlayerQuestStatus,
        quest: &wow_data::quest::QuestTemplate,
        objective: &wow_data::quest::QuestObjective,
    ) -> bool {
        match objective.obj_type {
            QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_GAMEOBJECT_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_TALKTO_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_WINPVPPETBATTLES_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP_LOCAL => {
                let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                    return false;
                };
                status
                    .objective_counts
                    .get(storage_index)
                    .copied()
                    .unwrap_or(0)
                    >= objective.amount
            }
            QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL => {
                Self::represented_quest_objective_progress_bar_complete_like_cpp(status, quest)
            }
            // Other objective completion sources need live runtime data. This helper is only
            // used as a guard before represented item-objective progress, so fail closed.
            _ => false,
        }
    }

    fn represented_quest_objective_progress_bar_complete_like_cpp(
        status: &PlayerQuestStatus,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        let mut progress = 0.0_f32;
        for objective in &quest.objectives {
            if (objective.flags & QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL) == 0 {
                continue;
            }

            let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                continue;
            };
            let count = status
                .objective_counts
                .get(storage_index)
                .copied()
                .unwrap_or(0);
            progress += count as f32 * objective.progress_bar_weight;
            if progress >= 100.0 {
                return true;
            }
        }
        false
    }

    pub(crate) fn represented_quest_objective_completable_like_cpp(
        status: &PlayerQuestStatus,
        quest: &wow_data::quest::QuestTemplate,
        objective_index: usize,
    ) -> bool {
        let Some(objective) = quest.objectives.get(objective_index) else {
            return false;
        };

        if (objective.flags & QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL) != 0 {
            let Some((progress_bar_index, progress_bar_objective)) =
                quest.objectives.iter().enumerate().find(|(_, other)| {
                    other.obj_type == QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL
                        && (other.flags & QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL)
                            == 0
                })
            else {
                return false;
            };

            return Self::represented_quest_objective_completable_like_cpp(
                status,
                quest,
                progress_bar_index,
            ) && !Self::represented_quest_objective_complete_like_cpp(
                status,
                quest,
                progress_bar_objective,
            );
        }

        if objective_index == 0 {
            return true;
        }

        let mut previous_index = objective_index - 1;
        let mut objective_sequence_satisfied = true;
        let mut previous_sequenced_objective_complete = false;
        let mut previous_sequenced_objective_index = None;

        loop {
            let previous_objective = &quest.objectives[previous_index];
            if (previous_objective.flags & QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL) != 0 {
                previous_sequenced_objective_index = Some(previous_index);
                previous_sequenced_objective_complete =
                    Self::represented_quest_objective_complete_like_cpp(
                        status,
                        quest,
                        previous_objective,
                    );
                break;
            }

            if objective_sequence_satisfied {
                objective_sequence_satisfied = Self::represented_quest_objective_complete_like_cpp(
                    status,
                    quest,
                    previous_objective,
                ) || (previous_objective.flags
                    & (QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP_LOCAL
                        | QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL))
                    != 0;
            }

            if previous_index == 0 {
                break;
            }
            previous_index -= 1;
        }

        if (objective.flags & QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL) != 0 {
            if previous_sequenced_objective_index.is_none() {
                return objective_sequence_satisfied;
            }
            if !previous_sequenced_objective_complete || !objective_sequence_satisfied {
                return false;
            }
        } else if !previous_sequenced_objective_complete {
            if let Some(previous_sequenced_objective_index) = previous_sequenced_objective_index {
                if !Self::represented_quest_objective_completable_like_cpp(
                    status,
                    quest,
                    previous_sequenced_objective_index,
                ) {
                    return false;
                }
            }
        }

        true
    }

    pub(crate) fn represented_can_complete_quest_after_objective_like_cpp(
        status: &PlayerQuestStatus,
        quest: &wow_data::quest::QuestTemplate,
        ignored_objective_id: u32,
        quest_already_rewarded: bool,
    ) -> bool {
        if quest.id == 0 {
            return false;
        }

        if !quest.is_repeatable() && quest_already_rewarded {
            return false;
        }

        if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
            return false;
        }

        for objective in &quest.objectives {
            if ignored_objective_id != 0 && objective.id == ignored_objective_id {
                continue;
            }

            if (objective.flags
                & (QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP_LOCAL
                    | QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL))
                != 0
            {
                continue;
            }

            if !Self::represented_quest_objective_complete_like_cpp(status, quest, objective) {
                return false;
            }
        }

        if (quest.flags
            & (QUEST_FLAGS_COMPLETION_EVENT_LIKE_CPP
                | QUEST_FLAGS_COMPLETION_AREA_TRIGGER_LIKE_CPP))
            != 0
            && !status.explored
        {
            return false;
        }

        if quest.limit_time_secs > 0 && status.end_time_secs == 0 {
            return false;
        }

        true
    }

    fn complete_represented_quest_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        self.invalidate_player_quest_status_authority_like_cpp();
        let old_status = {
            let Some(status) = self.player_quests.get_mut(&quest.id) else {
                return false;
            };
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                return false;
            }

            let old_status = status.status;
            status.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
            old_status
        };
        self.record_represented_quest_complete_status_update_like_cpp(
            RepresentedQuestCompleteStatusUpdateLikeCpp {
                quest_id: quest.id,
                old_status,
                new_status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                send_quest_update_called: true,
                quest_slot_state_complete_represented: true,
                quest_slot_state_live_update_unrepresented: true,
                visible_gameobjects_or_spellclicks_refresh_unrepresented: true,
                spell_area_runtime_unrepresented: true,
                tracking_event_auto_reward_unrepresented: (quest.flags
                    & QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP)
                    != 0,
                quest_tracker_complete_time_unrepresented: true,
                script_status_change_unrepresented: true,
            },
        );
        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        self.sync_player_registry_state_like_cpp();
        true
    }

    pub(crate) async fn complete_represented_quest_after_add_if_ready_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        self.complete_represented_quest_after_objective_if_ready_like_cpp(quest, 0)
            .await
    }

    pub(crate) async fn complete_represented_quest_after_objective_if_ready_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        ignored_objective_id: u32,
    ) -> bool {
        let Some(status) = self.player_quests.get(&quest.id) else {
            return false;
        };
        let quest_already_rewarded = self.rewarded_quests.contains(&quest.id);
        if !Self::represented_can_complete_quest_after_objective_like_cpp(
            status,
            quest,
            ignored_objective_id,
            quest_already_rewarded,
        ) {
            return false;
        }

        if !self.complete_represented_quest_like_cpp(quest) {
            return false;
        }

        if (quest.flags & QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP) != 0 {
            let quest_giver_guid = self
                .player_guid()
                .unwrap_or(wow_core::ObjectGuid::new(0, 0));
            let choice = QuestChoiceItemLikeCpp {
                loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                item_id: 0,
                quantity: 0,
            };
            let rewarded = self
                .reward_represented_quest_like_cpp(quest, quest_giver_guid, choice)
                .await;
            if rewarded {
                if let Some(evidence) = self
                    .represented_quest_complete_status_updates_like_cpp
                    .iter_mut()
                    .rev()
                    .find(|evidence| evidence.quest_id == quest.id)
                {
                    evidence.tracking_event_auto_reward_unrepresented = false;
                }
                Box::pin(self.drain_represented_quest_objective_progress_like_cpp()).await;
            }
        }

        true
    }

    pub(crate) async fn save_represented_quest_status_like_cpp(&self, quest_id: u32) {
        if let Some(status) = self
            .player_quests
            .get(&quest_id)
            .map(|status| status.status)
        {
            self.save_quest_to_db(quest_id, status).await;
        }
    }

    pub(crate) async fn save_changed_represented_quest_statuses_like_cpp(
        &self,
        quest_ids: &mut Vec<u32>,
    ) {
        quest_ids.sort_unstable();
        quest_ids.dedup();
        for quest_id in quest_ids.drain(..) {
            self.save_represented_quest_status_like_cpp(quest_id).await;
        }
    }

    #[cfg(test)]
    pub(crate) fn represented_quest_statuses_for_save_like_cpp(&self) -> Vec<(u32, u8)> {
        let mut quests = self
            .player_quests
            .iter()
            .filter_map(|(quest_id, status)| {
                if self.rewarded_quests.contains(quest_id)
                    && self
                        .quest_store
                        .as_ref()
                        .and_then(|store| store.get(*quest_id))
                        .is_some_and(|quest| !quest.is_repeatable())
                {
                    return None;
                }

                Some((*quest_id, status.status))
            })
            .collect::<Vec<_>>();
        quests.sort_by_key(|(quest_id, _)| *quest_id);
        quests
    }

    pub(crate) fn remove_represented_active_rewarded_duplicates_like_cpp(&mut self) -> Vec<u32> {
        let mut duplicate_quest_ids = self
            .player_quests
            .keys()
            .filter(|quest_id| {
                self.rewarded_quests.contains(quest_id)
                    && self
                        .quest_store
                        .as_ref()
                        .and_then(|store| store.get(**quest_id))
                        .is_some_and(|quest| !quest.is_repeatable())
            })
            .copied()
            .collect::<Vec<_>>();
        duplicate_quest_ids.sort_unstable();
        duplicate_quest_ids.dedup();

        if !duplicate_quest_ids.is_empty() {
            self.invalidate_player_quest_status_authority_like_cpp();
        }

        for quest_id in &duplicate_quest_ids {
            self.player_quests.remove(quest_id);
        }

        if !duplicate_quest_ids.is_empty() {
            let mut remaining_slots = self
                .player_quests
                .iter()
                .map(|(quest_id, status)| (*quest_id, status.slot))
                .collect::<Vec<_>>();
            remaining_slots.sort_by_key(|(_, slot)| *slot);
            for (slot, (quest_id, _)) in remaining_slots.into_iter().enumerate() {
                if let Some(status) = self.player_quests.get_mut(&quest_id) {
                    status.slot =
                        u8::try_from(slot).unwrap_or(MAX_QUEST_LOG_SIZE_LIKE_CPP.saturating_sub(1));
                }
            }
        }

        duplicate_quest_ids
    }

    pub(crate) async fn quest_source_item_quest_log_item_id_like_cpp(
        &mut self,
        entry_id: u32,
    ) -> u32 {
        if let Some(quest_log_item_id) =
            self.item_template_addon_quest_log_item_id_like_cpp(entry_id)
        {
            return quest_log_item_id;
        }

        let Some(world_db) = self.world_db().map(Arc::clone) else {
            return 0;
        };

        let mut stmt = world_db.prepare(WorldStatements::SEL_ITEM_TEMPLATE_ADDON_LOOT_METADATA);
        stmt.set_u32(0, entry_id);

        let quest_log_item_id = match world_db.query(&stmt).await {
            Ok(result) if !result.is_empty() => result
                .try_read::<i32>(1)
                .unwrap_or(0)
                .try_into()
                .unwrap_or(0),
            Ok(_) => 0,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    entry_id,
                    ?error,
                    "QuestConfirmAccept: failed to load item_template_addon QuestLogItemId"
                );
                0
            }
        };
        self.cache_item_template_addon_quest_log_item_id_like_cpp(entry_id, quest_log_item_id);
        quest_log_item_id
    }

    pub(crate) async fn apply_quest_source_item_added_non_bound_objective_progress_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Vec<u32> {
        self.apply_quest_item_added_objective_progress_filtered_like_cpp(
            entry_id,
            quest_log_item_id,
            count,
            Some(false),
        )
        .await
    }

    /// C++ `Player::ItemAddedQuestCheck(entry, count)` without a bound-item
    /// filter, as used by bank withdrawals after `StoreItem`.
    pub(crate) async fn apply_quest_item_added_objective_progress_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Vec<u32> {
        self.apply_quest_item_added_objective_progress_filtered_like_cpp(
            entry_id,
            quest_log_item_id,
            count,
            None,
        )
        .await
    }

    async fn apply_quest_item_added_objective_progress_filtered_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
        bound_item_requirement: Option<bool>,
    ) -> Vec<u32> {
        use wow_packet::packets::quest::QuestUpdateComplete;

        let Some(quest_store) = self.quest_store.clone() else {
            return Vec::new();
        };
        let added_count = count;
        let count = i32::try_from(count).unwrap_or(i32::MAX);
        let entry_object_id = i32::try_from(entry_id).unwrap_or(i32::MAX);
        let mut objective_ids = vec![entry_object_id];
        let mut matching_entry_objectives = Vec::new();
        'matching_entry: for status in self.player_quests.values() {
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }
            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };
            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                let is_bound = (objective.flags2
                    & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL)
                    != 0;
                let passes_filter =
                    bound_item_requirement.is_none_or(|required_bound| required_bound == is_bound);
                let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                    continue;
                };
                let current = status
                    .objective_counts
                    .get(storage_index)
                    .copied()
                    .unwrap_or(0);
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                    || objective.object_id != entry_object_id
                    || !passes_filter
                    || current >= objective.amount
                    || !Self::represented_quest_objective_completable_like_cpp(
                        status,
                        quest,
                        objective_index,
                    )
                {
                    continue;
                }
                matching_entry_objectives.push(is_bound);
                if is_bound {
                    break 'matching_entry;
                }
            }
        }
        let should_update_quest_log_item = quest_log_item_id != 0
            && (matching_entry_objectives.len() != 1 || !matching_entry_objectives[0]);
        if should_update_quest_log_item {
            objective_ids.push(i32::try_from(quest_log_item_id).unwrap_or(i32::MAX));
        }

        let mut changed_quest_ids = Vec::new();
        let mut quests_to_complete = Vec::new();
        let mut objective_updates = Vec::new();
        'quests: for status in self.player_quests.values_mut() {
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }

            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };

            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL {
                    continue;
                }
                let is_bound = (objective.flags2
                    & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL)
                    != 0;
                if bound_item_requirement.is_some_and(|required_bound| required_bound != is_bound) {
                    continue;
                }
                if !objective_ids.contains(&objective.object_id) {
                    continue;
                }
                if !Self::represented_quest_objective_completable_like_cpp(
                    status,
                    quest,
                    objective_index,
                ) {
                    continue;
                }

                let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                    continue;
                };
                if status.objective_counts.len() <= storage_index {
                    status.objective_counts.resize(storage_index + 1, 0);
                }
                let current = status.objective_counts[storage_index];
                if current >= objective.amount {
                    continue;
                }
                status.objective_counts[storage_index] =
                    current.saturating_add(count).clamp(0, objective.amount);
                let new_count = status.objective_counts[storage_index];
                if !changed_quest_ids.contains(&status.quest_id) {
                    changed_quest_ids.push(status.quest_id);
                }
                if count > 0 {
                    objective_updates.push((new_count, is_bound));
                }
                let quest_already_rewarded = self.rewarded_quests.contains(&status.quest_id);
                if new_count >= objective.amount
                    && Self::represented_can_complete_quest_after_objective_like_cpp(
                        status,
                        quest,
                        objective.id,
                        quest_already_rewarded,
                    )
                {
                    quests_to_complete.push(status.quest_id);
                }
                if is_bound {
                    break 'quests;
                }
            }
        }
        for quest_id in quests_to_complete {
            if let Some(quest) = quest_store.get(quest_id).cloned() {
                let completed = self
                    .complete_represented_quest_after_add_if_ready_like_cpp(&quest)
                    .await;
                if completed
                    && self
                        .player_quests
                        .get(&quest_id)
                        .is_some_and(|status| status.status == QUEST_STATUS_COMPLETE_LIKE_CPP)
                {
                    self.send_packet(&QuestUpdateComplete { quest_id });
                }
            }
        }
        if objective_updates.len() == 1 && objective_updates[0].1 {
            self.send_quest_bound_item_update_like_cpp(
                entry_id,
                quest_log_item_id,
                added_count,
                u32::try_from(objective_updates[0].0.max(0)).unwrap_or(u32::MAX),
            );
        }
        self.sync_player_registry_state_like_cpp();
        changed_quest_ids
    }

    /// C++ `Player::SendQuestUpdateAddItem`: ITEM objectives never use
    /// `SMSG_QUEST_UPDATE_ADD_CREDIT`; a single quest-bound objective uses
    /// `SMSG_ITEM_PUSH_RESULT` display type 3 instead.
    fn send_quest_bound_item_update_like_cpp(
        &self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
        quantity_in_inventory: u32,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let delivery = if self
            .item_template_flags3(entry_id)
            .is_some_and(|flags| (flags & ItemFlags3::DontReportLootLogToParty as u32) != 0)
        {
            SendNewItemDelivery::Direct
        } else {
            SendNewItemDelivery::GroupBroadcast
        };

        self.send_new_item_plan(&SendNewItemPlan {
            player_guid,
            item_guid: ObjectGuid::EMPTY,
            item_entry: entry_id,
            item_instance: SendNewItemInstancePlan {
                item_id: entry_id,
                random_properties_seed: 0,
                random_properties_id: 0,
                modifications: Vec::new(),
            },
            slot: u8::from(wow_entities::INVENTORY_SLOT_BAG_0),
            slot_in_bag: 0,
            quest_log_item_id,
            quantity: count,
            quantity_in_inventory,
            dungeon_encounter_id: 0,
            battle_pet_species_id: 0,
            battle_pet_breed_id: 0,
            battle_pet_breed_quality: 0,
            battle_pet_level: 0,
            pushed: false,
            created: false,
            is_encounter_loot: false,
            display_text: SendNewItemDisplayText::QuestUpdateAddItem,
            delivery,
        });
    }

    fn apply_quest_item_removed_to_statuses_like_cpp(
        quest_store: &QuestStore,
        player_quests: &mut HashMap<u32, PlayerQuestStatus>,
        entry_id: u32,
        new_non_bank_item_count: u32,
    ) -> Vec<u32> {
        let Ok(object_id) = i32::try_from(entry_id) else {
            return Vec::new();
        };
        let new_item_count = i32::try_from(new_non_bank_item_count).unwrap_or(i32::MAX);
        let mut changed_quest_ids = Vec::new();

        for status in player_quests.values_mut() {
            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };
            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                    || objective.object_id != object_id
                    || !Self::represented_quest_objective_completable_like_cpp(
                        status,
                        quest,
                        objective_index,
                    )
                {
                    continue;
                }
                let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                    continue;
                };
                if new_item_count >= objective.amount {
                    continue;
                }
                if status.objective_counts.len() <= storage_index {
                    status.objective_counts.resize(storage_index + 1, 0);
                }
                if status.objective_counts[storage_index] == new_item_count
                    && status.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP
                {
                    continue;
                }
                status.objective_counts[storage_index] = new_item_count.max(0);
                status.status = QUEST_STATUS_INCOMPLETE_LIKE_CPP;
                changed_quest_ids.push(status.quest_id);
            }
        }
        changed_quest_ids.sort_unstable();
        changed_quest_ids.dedup();
        changed_quest_ids
    }

    fn apply_quest_item_added_non_bound_to_statuses_like_cpp(
        quest_store: &QuestStore,
        rewarded_quests: &HashSet<u32>,
        player_quests: &mut HashMap<u32, PlayerQuestStatus>,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Vec<u32> {
        let entry_object_id = i32::try_from(entry_id).unwrap_or(i32::MAX);
        let mut objective_ids = vec![entry_object_id];
        if quest_log_item_id != 0 {
            objective_ids.push(i32::try_from(quest_log_item_id).unwrap_or(i32::MAX));
        }
        let count = i32::try_from(count).unwrap_or(i32::MAX);
        let mut changed_quest_ids = Vec::new();
        let mut quests_to_complete = Vec::new();

        for status in player_quests.values_mut() {
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }
            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };
            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                    || (objective.flags2 & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL)
                        != 0
                    || !objective_ids.contains(&objective.object_id)
                    || !Self::represented_quest_objective_completable_like_cpp(
                        status,
                        quest,
                        objective_index,
                    )
                {
                    continue;
                }
                let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                    continue;
                };
                if status.objective_counts.len() <= storage_index {
                    status.objective_counts.resize(storage_index + 1, 0);
                }
                let current = status.objective_counts[storage_index];
                if current >= objective.amount {
                    continue;
                }
                let new_count = current.saturating_add(count).clamp(0, objective.amount);
                status.objective_counts[storage_index] = new_count;
                changed_quest_ids.push(status.quest_id);
                if new_count >= objective.amount
                    && Self::represented_can_complete_quest_after_objective_like_cpp(
                        status,
                        quest,
                        objective.id,
                        rewarded_quests.contains(&status.quest_id),
                    )
                {
                    quests_to_complete.push(status.quest_id);
                }
            }
        }
        for quest_id in quests_to_complete {
            if let Some(status) = player_quests.get_mut(&quest_id) {
                status.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
            }
        }
        changed_quest_ids.sort_unstable();
        changed_quest_ids.dedup();
        changed_quest_ids
    }

    fn apply_quest_item_added_bound_to_statuses_like_cpp(
        quest_store: &QuestStore,
        rewarded_quests: &HashSet<u32>,
        player_quests: &mut HashMap<u32, PlayerQuestStatus>,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Option<(u32, i32)> {
        let count = i32::try_from(count).unwrap_or(i32::MAX);
        let mut ordered_quest_ids = player_quests
            .values()
            .map(|status| (status.slot, status.quest_id))
            .collect::<Vec<_>>();
        ordered_quest_ids.sort_unstable();

        for object_id in [entry_id, quest_log_item_id] {
            if object_id == 0 {
                continue;
            }
            let object_id = i32::try_from(object_id).unwrap_or(i32::MAX);
            for &(_, quest_id) in &ordered_quest_ids {
                let Some(status) = player_quests.get_mut(&quest_id) else {
                    continue;
                };
                if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                    continue;
                }
                let Some(quest) = quest_store.get(status.quest_id) else {
                    continue;
                };
                for (objective_index, objective) in quest.objectives.iter().enumerate() {
                    if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                        || (objective.flags2
                            & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL)
                            == 0
                        || objective.object_id != object_id
                        || !Self::represented_quest_objective_completable_like_cpp(
                            status,
                            quest,
                            objective_index,
                        )
                    {
                        continue;
                    }
                    let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                        continue;
                    };
                    if status.objective_counts.len() <= storage_index {
                        status.objective_counts.resize(storage_index + 1, 0);
                    }
                    let current = status.objective_counts[storage_index];
                    if current >= objective.amount {
                        continue;
                    }
                    let new_count = current.saturating_add(count).clamp(0, objective.amount);
                    status.objective_counts[storage_index] = new_count;
                    if new_count >= objective.amount
                        && Self::represented_can_complete_quest_after_objective_like_cpp(
                            status,
                            quest,
                            objective.id,
                            rewarded_quests.contains(&status.quest_id),
                        )
                    {
                        status.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
                    }
                    return Some((status.quest_id, new_count));
                }
            }
        }
        None
    }

    /// C++ `Player::ItemRemovedQuestCheck`: after the inventory mutation,
    /// recompute matching item objectives from carried (non-bank) contents and
    /// move completed quests back to incomplete when the requirement is lost.
    pub(crate) fn apply_quest_item_removed_like_cpp(&mut self, entry_id: u32) -> Vec<u32> {
        self.invalidate_player_quest_status_authority_like_cpp();
        let Some(quest_store) = self.quest_store.clone() else {
            return Vec::new();
        };
        let new_non_bank_item_count = self.represented_non_bank_item_count_like_cpp(entry_id);
        let changed_quest_ids = Self::apply_quest_item_removed_to_statuses_like_cpp(
            quest_store.as_ref(),
            &mut self.player_quests,
            entry_id,
            new_non_bank_item_count,
        );
        let changed_slots = changed_quest_ids
            .iter()
            .filter_map(|quest_id| self.player_quests.get(quest_id).map(|status| status.slot))
            .collect::<Vec<_>>();
        for slot in changed_slots {
            self.send_represented_quest_log_slot_update_like_cpp(slot);
        }
        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        self.sync_player_registry_state_like_cpp();
        changed_quest_ids
    }

    pub(crate) fn apply_quest_item_added_non_bound_state_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Vec<u32> {
        self.invalidate_player_quest_status_authority_like_cpp();
        let Some(quest_store) = self.quest_store.clone() else {
            return Vec::new();
        };
        Self::apply_quest_item_added_non_bound_to_statuses_like_cpp(
            quest_store.as_ref(),
            &self.rewarded_quests,
            &mut self.player_quests,
            entry_id,
            quest_log_item_id,
            count,
        )
    }

    pub(crate) fn apply_quest_item_added_bound_state_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Vec<u32> {
        self.invalidate_player_quest_status_authority_like_cpp();
        let Some(quest_store) = self.quest_store.clone() else {
            return Vec::new();
        };
        let Some((quest_id, new_count)) = Self::apply_quest_item_added_bound_to_statuses_like_cpp(
            quest_store.as_ref(),
            &self.rewarded_quests,
            &mut self.player_quests,
            entry_id,
            quest_log_item_id,
            count,
        ) else {
            return Vec::new();
        };
        self.send_quest_bound_item_update_like_cpp(
            entry_id,
            quest_log_item_id,
            count,
            u32::try_from(new_count.max(0)).unwrap_or(u32::MAX),
        );
        vec![quest_id]
    }

    pub(crate) fn publish_quest_item_added_status_changes_like_cpp(
        &mut self,
        changed_quest_ids: &[u32],
    ) {
        use wow_packet::packets::quest::QuestUpdateComplete;

        let mut changed_slots = changed_quest_ids
            .iter()
            .filter_map(|quest_id| self.player_quests.get(quest_id).map(|status| status.slot))
            .collect::<Vec<_>>();
        changed_slots.sort_unstable();
        changed_slots.dedup();
        for slot in changed_slots {
            self.send_represented_quest_log_slot_update_like_cpp(slot);
        }
        for &quest_id in changed_quest_ids {
            if self
                .player_quests
                .get(&quest_id)
                .is_some_and(|status| status.status == QUEST_STATUS_COMPLETE_LIKE_CPP)
            {
                self.send_packet(&QuestUpdateComplete { quest_id });
            }
        }
        self.sync_player_registry_state_like_cpp();
    }

    /// Pure post-move quest snapshot used to persist the item move and its
    /// `ItemAddedQuestCheck` / `ItemRemovedQuestCheck` result atomically.
    pub(crate) fn plan_bank_item_quest_persistence_like_cpp(
        &self,
        entry_id: u32,
        quest_log_item_id: u32,
        moving_to_bank: bool,
        post_move_non_bank_count: u32,
        added_count: u32,
    ) -> Vec<PlayerQuestStatus> {
        let Some(quest_store) = self.quest_store.as_ref() else {
            return Vec::new();
        };
        let Ok(entry_object_id) = i32::try_from(entry_id) else {
            return Vec::new();
        };
        let mut planned = Vec::new();

        if moving_to_bank {
            let new_item_count = i32::try_from(post_move_non_bank_count).unwrap_or(i32::MAX);
            for current_status in self.player_quests.values() {
                let Some(quest) = quest_store.get(current_status.quest_id) else {
                    continue;
                };
                let mut status = current_status.clone();
                let mut changed = false;
                for (objective_index, objective) in quest.objectives.iter().enumerate() {
                    if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                        || objective.object_id != entry_object_id
                        || !Self::represented_quest_objective_completable_like_cpp(
                            &status,
                            quest,
                            objective_index,
                        )
                    {
                        continue;
                    }
                    let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                        continue;
                    };
                    if new_item_count >= objective.amount {
                        continue;
                    }
                    if status.objective_counts.len() <= storage_index {
                        status.objective_counts.resize(storage_index + 1, 0);
                    }
                    if status.objective_counts[storage_index] != new_item_count.max(0)
                        || status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP
                    {
                        status.objective_counts[storage_index] = new_item_count.max(0);
                        status.status = QUEST_STATUS_INCOMPLETE_LIKE_CPP;
                        changed = true;
                    }
                }
                if changed {
                    planned.push(status);
                }
            }
            return planned;
        }

        let mut matching_entry_objectives = Vec::new();
        'matching_entry: for status in self.player_quests.values() {
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }
            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };
            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                    continue;
                };
                let current = status
                    .objective_counts
                    .get(storage_index)
                    .copied()
                    .unwrap_or(0);
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                    || objective.object_id != entry_object_id
                    || current >= objective.amount
                    || !Self::represented_quest_objective_completable_like_cpp(
                        status,
                        quest,
                        objective_index,
                    )
                {
                    continue;
                }
                let is_bound = (objective.flags2
                    & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL)
                    != 0;
                matching_entry_objectives.push(is_bound);
                if is_bound {
                    break 'matching_entry;
                }
            }
        }
        let mut objective_ids = vec![entry_object_id];
        if quest_log_item_id != 0
            && (matching_entry_objectives.len() != 1 || !matching_entry_objectives[0])
        {
            objective_ids.push(i32::try_from(quest_log_item_id).unwrap_or(i32::MAX));
        }
        let added_count = i32::try_from(added_count).unwrap_or(i32::MAX);

        for current_status in self.player_quests.values() {
            if current_status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }
            let Some(quest) = quest_store.get(current_status.quest_id) else {
                continue;
            };
            let mut status = current_status.clone();
            let mut completed_objective_ids = Vec::new();
            let mut changed = false;
            let mut stop_after_status = false;
            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                    || !objective_ids.contains(&objective.object_id)
                    || !Self::represented_quest_objective_completable_like_cpp(
                        &status,
                        quest,
                        objective_index,
                    )
                {
                    continue;
                }
                let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                    continue;
                };
                if status.objective_counts.len() <= storage_index {
                    status.objective_counts.resize(storage_index + 1, 0);
                }
                let current = status.objective_counts[storage_index];
                if current >= objective.amount {
                    continue;
                }
                let new_count = current
                    .saturating_add(added_count)
                    .clamp(0, objective.amount);
                status.objective_counts[storage_index] = new_count;
                changed = true;
                if new_count >= objective.amount {
                    completed_objective_ids.push(objective.id);
                }
                if (objective.flags2 & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL) != 0
                {
                    stop_after_status = true;
                    break;
                }
            }
            let quest_already_rewarded = self.rewarded_quests.contains(&status.quest_id);
            if completed_objective_ids.iter().any(|objective_id| {
                Self::represented_can_complete_quest_after_objective_like_cpp(
                    &status,
                    quest,
                    *objective_id,
                    quest_already_rewarded,
                )
            }) {
                status.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
            }
            if changed {
                planned.push(status);
            }
            if stop_after_status {
                break;
            }
        }
        planned
    }

    /// Pure aggregate form of C++ `Player::ItemRemovedQuestCheck` for a set
    /// of removals that must commit in the same transaction as their items.
    pub(crate) fn begin_item_transfer_quest_persistence_like_cpp(
        &self,
        removed_entries_in_order: &[u32],
        post_removal_non_bank_counts: &[(u32, u32)],
    ) -> ItemTransferQuestPersistencePlanLikeCpp {
        let mut plan = ItemTransferQuestPersistencePlanLikeCpp {
            statuses: self.player_quests.clone(),
            changed_quest_ids: Vec::new(),
        };
        let Some(quest_store) = self.quest_store.as_ref() else {
            return plan;
        };
        let post_removal_counts = post_removal_non_bank_counts
            .iter()
            .copied()
            .collect::<HashMap<_, _>>();
        for &entry_id in removed_entries_in_order {
            let Some(&new_non_bank_item_count) = post_removal_counts.get(&entry_id) else {
                continue;
            };
            plan.changed_quest_ids
                .extend(Self::apply_quest_item_removed_to_statuses_like_cpp(
                    quest_store.as_ref(),
                    &mut plan.statuses,
                    entry_id,
                    new_non_bank_item_count,
                ));
        }
        plan
    }

    pub(crate) fn plan_item_transfer_withdrawal_quest_persistence_like_cpp(
        &self,
        plan: &mut ItemTransferQuestPersistencePlanLikeCpp,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> bool {
        let Some(quest_store) = self.quest_store.as_ref() else {
            return false;
        };
        if let Some((quest_id, _)) = Self::apply_quest_item_added_bound_to_statuses_like_cpp(
            quest_store.as_ref(),
            &self.rewarded_quests,
            &mut plan.statuses,
            entry_id,
            quest_log_item_id,
            count,
        ) {
            plan.changed_quest_ids.push(quest_id);
            return true;
        }
        plan.changed_quest_ids
            .extend(Self::apply_quest_item_added_non_bound_to_statuses_like_cpp(
                quest_store.as_ref(),
                &self.rewarded_quests,
                &mut plan.statuses,
                entry_id,
                quest_log_item_id,
                count,
            ));
        false
    }

    pub(crate) fn finish_item_transfer_quest_persistence_like_cpp(
        &self,
        mut plan: ItemTransferQuestPersistencePlanLikeCpp,
    ) -> Vec<PlayerQuestStatus> {
        plan.changed_quest_ids.sort_unstable();
        plan.changed_quest_ids.dedup();
        plan.changed_quest_ids
            .into_iter()
            .filter_map(|quest_id| plan.statuses.remove(&quest_id))
            .collect()
    }

    pub(crate) fn plan_item_transfer_quest_persistence_like_cpp(
        &self,
        removed_entries_in_order: &[u32],
        post_removal_non_bank_counts: &[(u32, u32)],
        added_items_in_order: &[(u32, u32, u32)],
    ) -> Vec<PlayerQuestStatus> {
        let mut plan = self.begin_item_transfer_quest_persistence_like_cpp(
            removed_entries_in_order,
            post_removal_non_bank_counts,
        );
        for &(entry_id, quest_log_item_id, count) in added_items_in_order {
            let _ = self.plan_item_transfer_withdrawal_quest_persistence_like_cpp(
                &mut plan,
                entry_id,
                quest_log_item_id,
                count,
            );
        }
        self.finish_item_transfer_quest_persistence_like_cpp(plan)
    }

    pub(crate) fn append_planned_quest_statuses_to_transaction_like_cpp(
        &self,
        transaction: &mut SqlTransaction,
        char_db: &CharacterDatabase,
        player_guid: u64,
        planned_statuses: &[PlayerQuestStatus],
    ) {
        for status in planned_statuses {
            for statement in self.represented_quest_status_save_statements_like_cpp(
                player_guid,
                status.quest_id,
                status.status,
                Some(status),
                |statement| char_db.prepare(statement),
            ) {
                transaction.append(statement);
            }
        }
    }

    /// C++ walks one objective-status index and stops at the first quest-bound
    /// item objective. Rust stores statuses in a `HashMap`, so two independent
    /// scans could select different quests. Use the explicit quest-log slot
    /// (then quest id as a deterministic duplicate-slot fallback) for both the
    /// durable plan and its post-commit application.
    fn quest_bound_item_objective_quest_order_like_cpp(&self) -> Vec<u32> {
        let mut quests = self
            .player_quests
            .values()
            .map(|status| (status.slot, status.quest_id))
            .collect::<Vec<_>>();
        quests.sort_unstable();
        quests.into_iter().map(|(_, quest_id)| quest_id).collect()
    }

    /// Pure form of the first C++ `Player::StoreNewItem` quest pass:
    /// `ItemAddedQuestCheck(itemId, count, true, &hadBoundItemObjective)`.
    ///
    /// `UpdateQuestObjectiveProgress` stops after the first matching
    /// `QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM` objective. When it changes
    /// one objective, `StoreNewItem` returns `nullptr` and no physical Item is
    /// created. Keeping this as a snapshot lets loot persist the objective and
    /// consume its object-owned claim in one SQL/authority transaction.
    pub(crate) fn plan_quest_source_item_bound_objective_persistence_like_cpp(
        &self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Option<QuestSourceItemBoundPersistencePlanLikeCpp> {
        let quest_store = self.quest_store.as_ref()?;
        let count_i32 = i32::try_from(count).unwrap_or(i32::MAX);
        let entry_object_id = i32::try_from(entry_id).unwrap_or(i32::MAX);
        let quest_log_object_id = i32::try_from(quest_log_item_id).unwrap_or(i32::MAX);
        let ordered_quest_ids = self.quest_bound_item_objective_quest_order_like_cpp();

        for object_id in [entry_object_id, quest_log_object_id] {
            if object_id == quest_log_object_id && quest_log_item_id == 0 {
                continue;
            }

            for quest_id in &ordered_quest_ids {
                let Some(current_status) = self.player_quests.get(quest_id) else {
                    continue;
                };
                if current_status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                    continue;
                }
                let Some(quest) = quest_store.get(current_status.quest_id) else {
                    continue;
                };

                for (objective_index, objective) in quest.objectives.iter().enumerate() {
                    if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                        || (objective.flags2
                            & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL)
                            == 0
                        || objective.object_id != object_id
                        || !Self::represented_quest_objective_completable_like_cpp(
                            current_status,
                            quest,
                            objective_index,
                        )
                    {
                        continue;
                    }

                    let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                        continue;
                    };
                    let current = current_status
                        .objective_counts
                        .get(storage_index)
                        .copied()
                        .unwrap_or(0);
                    if current >= objective.amount {
                        continue;
                    }

                    let mut planned_status = current_status.clone();
                    if planned_status.objective_counts.len() <= storage_index {
                        planned_status.objective_counts.resize(storage_index + 1, 0);
                    }
                    let new_count = current.saturating_add(count_i32).clamp(0, objective.amount);
                    planned_status.objective_counts[storage_index] = new_count;
                    let quest_already_rewarded = self.rewarded_quests.contains(&quest.id);
                    if new_count >= objective.amount
                        && Self::represented_can_complete_quest_after_objective_like_cpp(
                            &planned_status,
                            quest,
                            objective.id,
                            quest_already_rewarded,
                        )
                    {
                        planned_status.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
                    }

                    return Some(QuestSourceItemBoundPersistencePlanLikeCpp {
                        statuses: vec![planned_status],
                    });
                }
            }
        }

        None
    }

    async fn apply_quest_source_item_bound_objective_progress_for_object_like_cpp(
        &mut self,
        quest_store: &QuestStore,
        object_id: i32,
        count_i32: i32,
    ) -> Vec<(u32, i32)> {
        self.invalidate_player_quest_status_authority_like_cpp();
        let mut updated_counts = Vec::new();
        let mut quests_to_complete = Vec::new();
        let ordered_quest_ids = self.quest_bound_item_objective_quest_order_like_cpp();

        'quests: for quest_id in ordered_quest_ids {
            let Some(status) = self.player_quests.get_mut(&quest_id) else {
                continue;
            };
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }

            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };

            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL {
                    continue;
                }
                if (objective.flags2 & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL) == 0
                {
                    continue;
                }
                if objective.object_id != object_id {
                    continue;
                }
                if !Self::represented_quest_objective_completable_like_cpp(
                    status,
                    quest,
                    objective_index,
                ) {
                    continue;
                }

                let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                    continue;
                };
                if status.objective_counts.len() <= storage_index {
                    status.objective_counts.resize(storage_index + 1, 0);
                }
                let current = status.objective_counts[storage_index];
                if current >= objective.amount {
                    continue;
                }
                let new_count = current.saturating_add(count_i32).clamp(0, objective.amount);
                status.objective_counts[storage_index] = new_count;
                updated_counts.push((status.quest_id, new_count));
                let quest_already_rewarded = self.rewarded_quests.contains(&status.quest_id);
                if new_count >= objective.amount
                    && Self::represented_can_complete_quest_after_objective_like_cpp(
                        status,
                        quest,
                        objective.id,
                        quest_already_rewarded,
                    )
                {
                    quests_to_complete.push(status.quest_id);
                }
                // C++ `UpdateQuestObjectiveProgress` stops after the first
                // credited quest-bound Item objective.
                break 'quests;
            }
        }

        for quest_id in quests_to_complete {
            if let Some(quest) = quest_store.get(quest_id).cloned() {
                self.complete_represented_quest_after_add_if_ready_like_cpp(&quest)
                    .await;
            }
        }

        updated_counts
    }

    pub(crate) async fn apply_quest_source_item_bound_objective_preflight_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Option<QuestSourceItemBoundPreflightLikeCpp> {
        let Some(_player_guid) = self.player_guid() else {
            return None;
        };
        let Some(quest_store) = self.quest_store.clone() else {
            return None;
        };
        let count_i32 = i32::try_from(count).unwrap_or(i32::MAX);
        let entry_object_id = i32::try_from(entry_id).unwrap_or(i32::MAX);
        let mut updated_counts = self
            .apply_quest_source_item_bound_objective_progress_for_object_like_cpp(
                quest_store.as_ref(),
                entry_object_id,
                count_i32,
            )
            .await;

        if quest_log_item_id != 0 && updated_counts.len() != 1 {
            let quest_log_object_id = i32::try_from(quest_log_item_id).unwrap_or(i32::MAX);
            updated_counts.extend(
                self.apply_quest_source_item_bound_objective_progress_for_object_like_cpp(
                    quest_store.as_ref(),
                    quest_log_object_id,
                    count_i32,
                )
                .await,
            );
        }

        if updated_counts.is_empty() {
            return None;
        }

        self.sync_player_registry_state_like_cpp();
        let mut changed_quest_ids = Vec::new();
        for &(quest_id, _) in &updated_counts {
            if !changed_quest_ids.contains(&quest_id) {
                changed_quest_ids.push(quest_id);
            }
        }

        if updated_counts.len() != 1 {
            return Some(QuestSourceItemBoundPreflightLikeCpp {
                no_grant: false,
                changed_quest_ids,
            });
        }

        self.send_quest_bound_item_update_like_cpp(
            entry_id,
            quest_log_item_id,
            count,
            u32::try_from(updated_counts[0].1.max(0)).unwrap_or(u32::MAX),
        );
        Some(QuestSourceItemBoundPreflightLikeCpp {
            no_grant: true,
            changed_quest_ids,
        })
    }

    /// CMSG_ADVENTURE_MAP_START_QUEST.
    ///
    /// C++ `HandleAdventureMapStartQuest`:
    /// `QuestTemplate` lookup -> `sAdventureMapPOIStore` QuestID + PlayerCondition gate ->
    /// `Player::CanTakeQuest(quest, true)` -> `AddQuestAndCheckCompletion(quest, player)`.
    ///
    /// Rust keeps the same silent-return gates and records the accepted request until
    /// Adventure Map quest starts can call the same live AddQuestAndCheckCompletion path.
    pub async fn handle_adventure_map_start_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match AdventureMapStartQuest::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("AdventureMapStartQuest: bad packet: {error}");
                return;
            }
        };
        let Ok(quest_id) = u32::try_from(request.quest_id) else {
            return;
        };

        let Some(quest_store) = self.quest_store.clone() else {
            return;
        };
        let Some(quest) = quest_store.get(quest_id) else {
            return;
        };
        let Some(adventure_map_poi_store) = self.adventure_map_poi_store().cloned() else {
            return;
        };
        let Some(poi) = adventure_map_poi_store.find_start_quest_poi_like_cpp(quest_id, |id| {
            self.represented_meets_player_condition_id_like_cpp(id)
        }) else {
            return;
        };

        if !self.can_take_quest(quest) {
            return;
        }

        self.record_represented_adventure_map_start_quest_like_cpp(
            RepresentedAdventureMapStartQuestLikeCpp {
                quest_id,
                adventure_map_poi_id: poi.id,
                player_condition_id: poi.player_condition_id,
            },
        );
    }

    /// CMSG_QUEST_GIVER_STATUS_QUERY — returns the quest status icon for an NPC.
    /// Legacy non-canonical note: QuestHandler.HandleQuestGiverStatusQuery
    pub async fn handle_quest_giver_status_query(&mut self, mut pkt: wow_packet::WorldPacket) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("QuestGiverStatusQuery: failed to read GUID");
                return;
            }
        };

        let Some(source) = self.represented_quest_giver_status_query_source_like_cpp(guid) else {
            debug!(
                account = self.account_id,
                ?guid,
                "QuestGiverStatusQuery: represented ObjectAccessor mask UNIT|GAMEOBJECT miss"
            );
            return;
        };
        let status = self.get_represented_quest_giver_status_like_cpp(source);

        debug!(
            account = self.account_id,
            ?guid,
            source_entry = source.entry(),
            source_kind = source.kind_name(),
            status = status,
            "QuestGiverStatus represented source resolved"
        );

        self.send_packet(&QuestGiverStatus { guid, status });
    }

    /// CMSG_QUEST_GIVER_HELLO — player right-clicks a quest NPC.
    /// Opens the represented quest list dialog for an interactable questgiver Creature.
    /// C++ refs:
    /// - `WorldSession::HandleQuestgiverHelloOpcode`, `QuestHandler.cpp:76-103`.
    /// - `Player::PrepareQuestMenu`, `Player.cpp:13947-14004`.
    /// Remaining represented gaps: fake-death aura removal, `AI()->OnGossipHello`,
    /// full PlayerTalkClass ownership.
    pub async fn handle_quest_giver_hello(&mut self, mut pkt: wow_packet::WorldPacket) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("QuestGiverHello: failed to read GUID");
                return;
            }
        };

        let Some(access) =
            self.represented_npc_can_interact_with_like_cpp(guid, NPCFlags1::QUEST_GIVER.bits(), 0)
        else {
            debug!(
                account = self.account_id,
                ?guid,
                "QuestGiverHello: NPC not found or not interactable as questgiver"
            );
            return;
        };

        self.pause_interacted_creature_movement_like_cpp(guid);

        if (access.npc_flags & NPCFlags1::GOSSIP.bits()) != 0
            && let Some(world_db) = self.world_db().map(Arc::clone)
            && let Some(msg) = self
                .build_gossip_menu(&world_db, access.entry, access.npc_flags, guid)
                .await
        {
            debug!(
                account = self.account_id,
                creature_entry = access.entry,
                "QuestGiverHello sent DB-backed prepared gossip menu like C++"
            );
            self.send_packet(&msg);
            return;
        }

        if self.send_represented_creature_trainer_gossip_menu_like_cpp(
            guid,
            access.entry,
            access.npc_flags,
        ) {
            debug!(
                account = self.account_id,
                creature_entry = access.entry,
                "QuestGiverHello sent trainer fallback prepared gossip menu like C++"
            );
            return;
        }

        if self.use_represented_creature_questgiver_like_cpp(guid, access.entry) {
            debug!(
                account = self.account_id,
                creature_entry = access.entry,
                "QuestGiverHello represented Creature questgiver seam consumed"
            );
        }
    }

    /// CMSG_QUEST_GIVER_QUERY_QUEST — player clicks a quest name in the list.
    /// Shows full quest details (objectives, rewards) before acceptkng.
    /// Legacy non-canonical note: QuestHandler.HandleQuestGiverQueryQuest
    pub async fn handle_quest_giver_query_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let (guid, quest_id, respond_to_giver) =
            match read_quest_giver_query_quest_like_cpp(&mut pkt) {
                Ok(packet) => packet,
                Err(_) => {
                    warn!("QuestGiverQueryQuest: failed to read packet");
                    return;
                }
            };

        info!(
            account = self.account_id,
            ?guid,
            quest_id,
            respond_to_giver,
            "Received QuestGiverQueryQuest like C++"
        );
        if !self.send_represented_quest_giver_query_quest_like_cpp(guid, quest_id) {
            warn!(
                account = self.account_id,
                ?guid,
                quest_id,
                "QuestGiverQueryQuest produced no represented response"
            );
        }
    }

    /// CMSG_QUEST_GIVER_ACCEPT_QUEST — player clicks "Accept" in the quest details dialog.
    /// Saves quest to characters DB and confirms to the client.
    /// Legacy non-canonical note: QuestHandler.HandleQuestGiverAcceptQuest
    pub async fn handle_quest_giver_accept_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let (guid, quest_id, start_cheat) = match read_quest_giver_accept_quest_like_cpp(&mut pkt) {
            Ok(packet) => packet,
            Err(_) => {
                warn!("QuestGiverAcceptQuest: failed to read packet");
                return;
            }
        };

        info!(
            account = self.account_id,
            ?guid,
            quest_id,
            start_cheat,
            "Received QuestGiverAcceptQuest like C++"
        );

        // Validate represented C++ source/relation before any quest-log mutation or DB save.
        // C++ HandleQuestgiverAcceptQuestOpcode closes gossip and clears sharing info on
        // failure; this represented slice intentionally models that as no packet/no mutation.
        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => return,
        };
        if !self.represented_quest_giver_accept_source_allows_quest_like_cpp(
            guid,
            quest_id,
            &quest_store,
        ) {
            warn!(
                account = self.account_id,
                ?guid,
                quest_id,
                "AcceptQuest: represented source/relation guard rejected quest"
            );
            return;
        }
        let Some(quest) = quest_store.get(quest_id) else {
            warn!(
                account = self.account_id,
                quest_id, "AcceptQuest: unknown quest"
            );
            return;
        };

        // Full eligibility check: SatisfyQuestStatus + PrevQuestId + race/class/level
        // Legacy non-canonical note: Player.CanTakeQuest(quest, true)
        if !self.can_take_quest(quest) {
            warn!(
                account = self.account_id,
                quest_id,
                race = self.player_race_like_cpp(),
                class = self.player_class_like_cpp(),
                level = self.player_level_like_cpp(),
                "AcceptQuest: player does not meet requirements (CanTakeQuest failed)"
            );
            return;
        }

        // C++ Player::AddQuest uses FindQuestSlot(0) over explicit QuestLog slots.
        let Some(slot) = self.first_free_quest_slot_like_cpp() else {
            warn!(account = self.account_id, "Quest log full");
            return;
        };

        // Build objective counts (one slot per objective)
        let obj_count = quest.objectives.len();

        let (accept_time_secs, end_time_secs) =
            Self::represented_accept_and_end_time_for_new_quest_like_cpp(&quest);

        // Add to local state
        self.invalidate_player_quest_status_authority_like_cpp();
        self.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs,
                end_time_secs,
                objective_counts: vec![0; obj_count],
                slot,
            },
        );

        self.complete_represented_quest_after_add_if_ready_like_cpp(quest)
            .await;

        // Save to DB after AddQuestAndCheckCompletion-style completion, unless
        // RewardQuest already removed/rewarded the quest.
        if let Some(status) = self
            .player_quests
            .get(&quest_id)
            .map(|status| status.status)
        {
            self.save_quest_to_db(quest_id, status).await;
        }
        self.sync_player_registry_state_like_cpp();
        self.send_represented_quest_log_slot_update_like_cpp(slot);

        info!(account = self.account_id, quest_id, "Quest accepted");

        // Notify client — quest added popup
        self.send_packet(&QuestGiverQuestComplete {
            quest_id,
            xp: 0,
            money: 0,
            skill_line_id: 0,
            skill_points: 0,
            use_quest_reward_currency: false,
        });
    }

    /// CMSG_QUEST_GIVER_CLOSE_QUEST — acknowledged client close for auto-accept quest flow.
    /// C++ ref: `WorldSession::HandleQuestgiverCloseQuest`, `QuestHandler.cpp:591-601`.
    /// Represented seam only: records local `ScriptMgr::OnQuestAcknowledgeAutoAccept` evidence.
    pub async fn handle_quest_giver_close_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let quest_id = match pkt.read_uint32() {
            Ok(quest_id) => quest_id,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "QuestGiverCloseQuest: failed to read QuestID"
                );
                return;
            }
        };

        let _ = self.acknowledge_auto_accept_quest_like_cpp(quest_id);
    }

    pub(crate) fn acknowledge_auto_accept_quest_like_cpp(&mut self, quest_id: u32) -> bool {
        // C++ order: FindQuestSlot(QuestID), then GetQuestTemplate(QuestID), then
        // ScriptMgr::OnQuestAcknowledgeAutoAccept(player, quest).
        if self.find_quest_slot_like_cpp(quest_id).is_none() {
            debug!(
                account = self.account_id,
                quest_id, "QuestGiverCloseQuest: represented active quest log miss"
            );
            return false;
        }

        let Some(quest_store) = &self.quest_store else {
            debug!(
                account = self.account_id,
                quest_id, "QuestGiverCloseQuest: missing represented quest store"
            );
            return false;
        };

        if quest_store.get(quest_id).is_none() {
            debug!(
                account = self.account_id,
                quest_id, "QuestGiverCloseQuest: represented quest template miss"
            );
            return false;
        }

        self.represented_auto_accept_acknowledged_quests_like_cpp
            .push(quest_id);
        true
    }

    /// CMSG_REQUEST_WORLD_QUEST_UPDATE — current Trinity 3.4.3 handler sends an empty response.
    /// C++ refs: `WorldSession::HandleRequestWorldQuestUpdate`, `QuestHandler.cpp:780-788`;
    /// `RequestWorldQuestUpdate::Read`, `QuestPackets.h:655-661` (`Read() { }`, no payload consumption).
    pub async fn handle_request_world_quest_update(&mut self, _pkt: wow_packet::WorldPacket) {
        self.send_packet(&WorldQuestUpdateResponse {
            updates: Vec::new(),
        });
    }

    async fn add_quest_confirm_accept_local_state_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        let Some(slot) = self.first_free_quest_slot_like_cpp() else {
            return false;
        };

        let (accept_time_secs, end_time_secs) =
            Self::represented_accept_and_end_time_for_new_quest_like_cpp(quest);

        self.invalidate_player_quest_status_authority_like_cpp();
        self.player_quests.insert(
            quest.id,
            PlayerQuestStatus {
                quest_id: quest.id,
                status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs,
                end_time_secs,
                objective_counts: vec![0; quest.objectives.len()],
                slot,
            },
        );
        self.complete_represented_quest_after_add_if_ready_like_cpp(quest)
            .await;
        self.save_represented_quest_status_like_cpp(quest.id).await;
        self.sync_player_registry_state_like_cpp();
        true
    }

    async fn store_quest_source_item_like_cpp(
        &mut self,
        entry_id: u32,
        quantity: u32,
        dest: &[ItemPosCount],
    ) -> Option<QuestSourceItemStoreOutcomeLikeCpp> {
        let Some(player_guid) = self.player_guid() else {
            return None;
        };
        if dest.is_empty() {
            return None;
        }
        let quest_log_item_id = self
            .quest_source_item_quest_log_item_id_like_cpp(entry_id)
            .await;
        let completion_evidence_start = self
            .represented_quest_complete_status_updates_like_cpp()
            .len();
        if let Some(bound_preflight) = self
            .apply_quest_source_item_bound_objective_preflight_like_cpp(
                entry_id,
                quest_log_item_id,
                quantity,
            )
            .await
        {
            for quest_id in bound_preflight.changed_quest_ids {
                self.save_represented_quest_status_like_cpp(quest_id).await;
            }
            if bound_preflight.no_grant {
                self.save_represented_quest_statuses_completed_after_like_cpp(
                    completion_evidence_start,
                )
                .await;
                return Some(QuestSourceItemStoreOutcomeLikeCpp::BoundObjectiveNoGrant);
            }
        }

        #[derive(Clone, Copy)]
        struct ExistingStackUpdate {
            item_guid: ObjectGuid,
            new_count: u32,
            should_bind: bool,
            pos: u16,
        }

        #[derive(Clone, Copy)]
        struct NewStack {
            bag: u8,
            slot: u8,
            db_guid: u64,
            item_guid: ObjectGuid,
            stack_count: u32,
            max_durability: u32,
            item_flags: u32,
            contained_in: ObjectGuid,
        }

        let mut existing_updates: Vec<ExistingStackUpdate> = Vec::new();
        let mut new_stacks: Vec<NewStack> = Vec::new();
        let mut tx = SqlTransaction::new();
        let source_item_bonding = self
            .item_storage_template(entry_id)
            .map(|template| template.bonding);
        let mut last_item_guid = ObjectGuid::EMPTY;
        let mut last_bag = u8::from(wow_entities::INVENTORY_SLOT_BAG_0);
        let mut last_slot = 0;
        let mut last_count_in_stack = 0;
        let new_item_count = dest
            .iter()
            .filter(|dest| {
                let bag = (dest.pos >> 8) as u8;
                let slot = (dest.pos & 0x00FF) as u8;
                self.get_inventory_item_by_pos(bag, slot).is_none()
            })
            .count();
        let Some(allocated_new_item_guids) =
            self.allocate_item_instance_guids_like_cpp(new_item_count)
        else {
            warn!(
                account = self.account_id,
                entry_id,
                count = new_item_count,
                "QuestConfirmAccept: process-wide item GUID allocator is unavailable"
            );
            self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
            return None;
        };
        let mut allocated_new_item_guids = allocated_new_item_guids.into_iter();

        for dest in dest {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;

            if let Some(inv_item) = self.get_inventory_item_by_pos(bag, slot) {
                let Some(existing_item) =
                    self.inventory_item_objects_like_cpp().get(&inv_item.guid)
                else {
                    warn!(
                        account = self.account_id,
                        slot,
                        entry_id,
                        "QuestConfirmAccept: missing runtime item object for source item stack"
                    );
                    self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
                    return None;
                };
                let new_count = existing_item.count().saturating_add(dest.count);
                let existing_flags = existing_item.item_flags_bits();
                let should_bind = source_item_bonding.is_some_and(|bonding| {
                    matches!(bonding, ItemBondingType::OnAcquire | ItemBondingType::Quest)
                        || (bonding == ItemBondingType::OnEquip && is_bag_pos(dest.pos))
                });
                if let Some(char_db) = self.char_db() {
                    let mut upd_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
                    upd_count.set_u32(0, new_count);
                    upd_count.set_u64(1, inv_item.db_guid);
                    tx.append(upd_count);
                    if should_bind && !existing_item.is_soul_bound() {
                        let mut upd_flags =
                            char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_FLAGS);
                        upd_flags.set_u32(0, existing_flags | ItemFieldFlags::SOULBOUND.bits());
                        upd_flags.set_u64(1, inv_item.db_guid);
                        tx.append(upd_flags);
                    }
                }
                existing_updates.push(ExistingStackUpdate {
                    item_guid: inv_item.guid,
                    new_count,
                    should_bind,
                    pos: dest.pos,
                });
                last_item_guid = inv_item.guid;
                last_bag = bag;
                last_slot = slot;
                last_count_in_stack = new_count;
            } else {
                let (inventory_bag_db_guid, contained_in) = if bag
                    == u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
                {
                    (0, player_guid)
                } else if let Some(bag_inventory_item) = self.inventory_items_like_cpp().get(&bag) {
                    (bag_inventory_item.db_guid, bag_inventory_item.guid)
                } else {
                    warn!(
                        account = self.account_id,
                        bag,
                        slot,
                        entry_id,
                        "QuestConfirmAccept: represented source item destination references missing bag"
                    );
                    self.send_equip_error(InventoryResult::WrongBagType, None, None, 0, 0);
                    return None;
                };

                let Some((db_guid, item_guid)) = allocated_new_item_guids.next() else {
                    warn!(
                        account = self.account_id,
                        entry_id,
                        "QuestConfirmAccept: preallocated item GUID count did not match store plan"
                    );
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return None;
                };
                let max_durability = self.item_template_max_durability(entry_id);
                let should_bind = source_item_bonding.is_some_and(|bonding| {
                    matches!(bonding, ItemBondingType::OnAcquire | ItemBondingType::Quest)
                        || (bonding == ItemBondingType::OnEquip && is_bag_pos(dest.pos))
                });
                let item_flags = if should_bind {
                    ItemFieldFlags::SOULBOUND.bits()
                } else {
                    0
                };

                if let Some(char_db) = self.char_db() {
                    let mut ins_item = char_db.prepare(CharStatements::INS_ITEM_INSTANCE);
                    ins_item.set_u64(0, db_guid);
                    ins_item.set_u32(1, entry_id);
                    ins_item.set_u64(2, player_guid.counter() as u64);
                    ins_item.set_u32(3, dest.count);
                    ins_item.set_u32(4, max_durability);
                    tx.append(ins_item);
                    if item_flags != 0 {
                        let mut upd_flags =
                            char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_FLAGS);
                        upd_flags.set_u32(0, item_flags);
                        upd_flags.set_u64(1, db_guid);
                        tx.append(upd_flags);
                    }

                    let mut ins_inv = char_db.prepare(CharStatements::REP_CHAR_INVENTORY_ITEM);
                    ins_inv.set_u64(0, player_guid.counter() as u64);
                    ins_inv.set_u64(1, inventory_bag_db_guid);
                    ins_inv.set_u8(2, slot);
                    ins_inv.set_u64(3, db_guid);
                    tx.append(ins_inv);
                }

                new_stacks.push(NewStack {
                    bag,
                    slot,
                    db_guid,
                    item_guid,
                    stack_count: dest.count,
                    max_durability,
                    item_flags,
                    contained_in,
                });
                last_item_guid = item_guid;
                last_bag = bag;
                last_slot = slot;
                last_count_in_stack = dest.count;
            }
        }

        if let Some(char_db) = self.char_db().map(Arc::clone) {
            if let Err(error) = char_db.commit_transaction(tx).await {
                warn!(
                    account = self.account_id,
                    entry_id,
                    ?error,
                    "QuestConfirmAccept: source item StoreNewItem transaction failed"
                );
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return None;
            }
        }

        for update in &existing_updates {
            self.update_inventory_item_object_like_cpp(update.item_guid, |item| {
                item.set_count(update.new_count);
                if let Some(bonding) = source_item_bonding {
                    item.set_bonding(bonding);
                    if update.should_bind {
                        item.bind_if_stored(is_bag_pos(update.pos));
                    }
                }
            });
        }

        let inventory_type = self.item_template_inventory_type(entry_id);
        for stack in &new_stacks {
            if stack.bag == u8::from(wow_entities::INVENTORY_SLOT_BAG_0) {
                self.insert_inventory_item_like_cpp(
                    stack.slot,
                    InventoryItem {
                        guid: stack.item_guid,
                        entry_id,
                        db_guid: stack.db_guid,
                        inventory_type,
                    },
                );
            }
            let mut item_object = self.make_inventory_item_object(
                stack.item_guid,
                entry_id,
                player_guid,
                stack.stack_count,
                stack.max_durability,
                ItemContext::None,
                stack.slot,
            );
            if stack.bag != u8::from(wow_entities::INVENTORY_SLOT_BAG_0) {
                item_object.set_container_guid_and_slot(stack.contained_in, stack.bag);
            }
            if let Some(bonding) = source_item_bonding {
                item_object.set_bonding(bonding);
                item_object.bind_if_stored(is_bag_pos(wow_entities::make_item_pos(
                    stack.bag, stack.slot,
                )));
            }
            self.insert_inventory_item_object(item_object);
        }
        self.sync_object_accessor_player();

        let map_id = self.player_map_id_like_cpp();
        if !new_stacks.is_empty() {
            let item_creates = new_stacks
                .iter()
                .map(|stack| ItemCreateData {
                    item_guid: stack.item_guid,
                    entry_id: entry_id as i32,
                    owner_guid: player_guid,
                    contained_in: stack.contained_in,
                    stack_count: stack.stack_count,
                    dynamic_flags: stack.item_flags,
                    durability: stack.max_durability,
                    max_durability: stack.max_durability,
                    random_properties_seed: 0,
                    random_properties_id: 0,
                    enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
                    gems: Vec::new(),
                    context: ItemContext::None as u8,
                    container_slots: 0,
                    container_item_guids: [ObjectGuid::EMPTY; 36],
                })
                .collect();
            self.send_packet(&UpdateObject::create_items(item_creates, map_id));
        }

        for update in &existing_updates {
            self.send_packet(&UpdateObject::item_stack_count_update(
                update.item_guid,
                map_id,
                update.new_count,
            ));
        }

        if !new_stacks.is_empty() {
            let changed_slots: Vec<_> = new_stacks
                .iter()
                .filter(|stack| stack.bag == u8::from(wow_entities::INVENTORY_SLOT_BAG_0))
                .map(|stack| (stack.slot, stack.item_guid))
                .collect();
            if !changed_slots.is_empty() {
                self.send_player_values_update_from_entity_bridge(
                    &changed_slots,
                    &[],
                    &[],
                    &[],
                    None,
                );
            }
        }

        let quantity_in_inventory = self
            .represented_inventory_item_counts_like_cpp()
            .get(&entry_id)
            .copied()
            .unwrap_or(0);
        let changed_non_bound_quest_ids = self
            .apply_quest_source_item_added_non_bound_objective_progress_like_cpp(
                entry_id,
                quest_log_item_id,
                quantity,
            )
            .await;
        for quest_id in changed_non_bound_quest_ids {
            self.save_represented_quest_status_like_cpp(quest_id).await;
        }
        self.save_represented_quest_statuses_completed_after_like_cpp(completion_evidence_start)
            .await;

        self.send_new_item_plan(&SendNewItemPlan {
            player_guid,
            item_guid: last_item_guid,
            item_entry: entry_id,
            item_instance: SendNewItemInstancePlan {
                item_id: entry_id,
                random_properties_seed: 0,
                random_properties_id: 0,
                modifications: Vec::<SendNewItemModifier>::new(),
            },
            slot: last_bag,
            slot_in_bag: if last_count_in_stack == quantity {
                i16::from(last_slot)
            } else {
                -1
            },
            quest_log_item_id,
            quantity,
            quantity_in_inventory,
            battle_pet_species_id: 0,
            battle_pet_breed_id: 0,
            battle_pet_breed_quality: 0,
            battle_pet_level: 0,
            pushed: true,
            created: false,
            display_text: SendNewItemDisplayText::Normal,
            dungeon_encounter_id: 0,
            is_encounter_loot: false,
            delivery: SendNewItemDelivery::Direct,
        });
        Some(QuestSourceItemStoreOutcomeLikeCpp::StoredNewItem)
    }

    async fn store_quest_reward_item_like_cpp(
        &mut self,
        entry_id: u32,
        quantity: u32,
        dest: &[ItemPosCount],
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if dest.is_empty() {
            return false;
        }

        #[derive(Clone, Copy)]
        struct ExistingStackUpdate {
            item_guid: ObjectGuid,
            new_count: u32,
            should_bind: bool,
            pos: u16,
        }

        #[derive(Clone, Copy)]
        struct NewStack {
            bag: u8,
            slot: u8,
            db_guid: u64,
            item_guid: ObjectGuid,
            stack_count: u32,
            max_durability: u32,
            item_flags: u32,
            contained_in: ObjectGuid,
        }

        let item_bonding = self
            .item_storage_template(entry_id)
            .map(|template| template.bonding);
        let mut existing_updates = Vec::new();
        let mut new_stacks = Vec::new();
        let mut tx = SqlTransaction::new();
        let mut last_item_guid = ObjectGuid::EMPTY;
        let mut last_bag = u8::from(wow_entities::INVENTORY_SLOT_BAG_0);
        let mut last_slot = 0;
        let mut last_count_in_stack = 0;
        let new_item_count = dest
            .iter()
            .filter(|dest| {
                let bag = (dest.pos >> 8) as u8;
                let slot = (dest.pos & 0x00FF) as u8;
                self.get_inventory_item_by_pos(bag, slot).is_none()
            })
            .count();
        let Some(allocated_new_item_guids) =
            self.allocate_item_instance_guids_like_cpp(new_item_count)
        else {
            warn!(
                account = self.account_id,
                entry_id,
                count = new_item_count,
                "RewardQuest: process-wide item GUID allocator is unavailable"
            );
            self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
            return false;
        };
        let mut allocated_new_item_guids = allocated_new_item_guids.into_iter();

        for dest in dest {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;

            if let Some(inv_item) = self.get_inventory_item_by_pos(bag, slot) {
                let Some(existing_item) =
                    self.inventory_item_objects_like_cpp().get(&inv_item.guid)
                else {
                    warn!(
                        account = self.account_id,
                        slot,
                        entry_id,
                        "RewardQuest: missing runtime item object for reward item stack"
                    );
                    self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
                    return false;
                };
                let new_count = existing_item.count().saturating_add(dest.count);
                let existing_flags = existing_item.item_flags_bits();
                let should_bind = item_bonding.is_some_and(|bonding| {
                    matches!(bonding, ItemBondingType::OnAcquire | ItemBondingType::Quest)
                        || (bonding == ItemBondingType::OnEquip && is_bag_pos(dest.pos))
                });
                if let Some(char_db) = self.char_db() {
                    let mut upd_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
                    upd_count.set_u32(0, new_count);
                    upd_count.set_u64(1, inv_item.db_guid);
                    tx.append(upd_count);
                    if should_bind && !existing_item.is_soul_bound() {
                        let mut upd_flags =
                            char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_FLAGS);
                        upd_flags.set_u32(0, existing_flags | ItemFieldFlags::SOULBOUND.bits());
                        upd_flags.set_u64(1, inv_item.db_guid);
                        tx.append(upd_flags);
                    }
                }
                existing_updates.push(ExistingStackUpdate {
                    item_guid: inv_item.guid,
                    new_count,
                    should_bind,
                    pos: dest.pos,
                });
                last_item_guid = inv_item.guid;
                last_bag = bag;
                last_slot = slot;
                last_count_in_stack = new_count;
            } else {
                let (inventory_bag_db_guid, contained_in) = if bag
                    == u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
                {
                    (0, player_guid)
                } else if let Some(bag_inventory_item) = self.inventory_items_like_cpp().get(&bag) {
                    (bag_inventory_item.db_guid, bag_inventory_item.guid)
                } else {
                    warn!(
                        account = self.account_id,
                        bag,
                        slot,
                        entry_id,
                        "RewardQuest: represented reward item destination references missing bag"
                    );
                    self.send_equip_error(InventoryResult::WrongBagType, None, None, 0, 0);
                    return false;
                };

                let Some((db_guid, item_guid)) = allocated_new_item_guids.next() else {
                    warn!(
                        account = self.account_id,
                        entry_id,
                        "RewardQuest: preallocated item GUID count did not match store plan"
                    );
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                };
                let max_durability = self.item_template_max_durability(entry_id);
                let should_bind = item_bonding.is_some_and(|bonding| {
                    matches!(bonding, ItemBondingType::OnAcquire | ItemBondingType::Quest)
                        || (bonding == ItemBondingType::OnEquip && is_bag_pos(dest.pos))
                });
                let item_flags = if should_bind {
                    ItemFieldFlags::SOULBOUND.bits()
                } else {
                    0
                };

                if let Some(char_db) = self.char_db() {
                    let mut ins_item = char_db.prepare(CharStatements::INS_ITEM_INSTANCE);
                    ins_item.set_u64(0, db_guid);
                    ins_item.set_u32(1, entry_id);
                    ins_item.set_u64(2, player_guid.counter() as u64);
                    ins_item.set_u32(3, dest.count);
                    ins_item.set_u32(4, max_durability);
                    tx.append(ins_item);
                    if item_flags != 0 {
                        let mut upd_flags =
                            char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_FLAGS);
                        upd_flags.set_u32(0, item_flags);
                        upd_flags.set_u64(1, db_guid);
                        tx.append(upd_flags);
                    }

                    let mut ins_inv = char_db.prepare(CharStatements::REP_CHAR_INVENTORY_ITEM);
                    ins_inv.set_u64(0, player_guid.counter() as u64);
                    ins_inv.set_u64(1, inventory_bag_db_guid);
                    ins_inv.set_u8(2, slot);
                    ins_inv.set_u64(3, db_guid);
                    tx.append(ins_inv);
                }

                new_stacks.push(NewStack {
                    bag,
                    slot,
                    db_guid,
                    item_guid,
                    stack_count: dest.count,
                    max_durability,
                    item_flags,
                    contained_in,
                });
                last_item_guid = item_guid;
                last_bag = bag;
                last_slot = slot;
                last_count_in_stack = dest.count;
            }
        }

        if let Some(char_db) = self.char_db().map(Arc::clone) {
            if let Err(error) = char_db.commit_transaction(tx).await {
                warn!(
                    account = self.account_id,
                    entry_id,
                    ?error,
                    "RewardQuest: reward item StoreNewItem transaction failed"
                );
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }
        }

        for update in &existing_updates {
            self.update_inventory_item_object_like_cpp(update.item_guid, |item| {
                item.set_count(update.new_count);
                if let Some(bonding) = item_bonding {
                    item.set_bonding(bonding);
                    if update.should_bind {
                        item.bind_if_stored(is_bag_pos(update.pos));
                    }
                }
            });
        }

        let inventory_type = self.item_template_inventory_type(entry_id);
        for stack in &new_stacks {
            if stack.bag == u8::from(wow_entities::INVENTORY_SLOT_BAG_0) {
                self.insert_inventory_item_like_cpp(
                    stack.slot,
                    InventoryItem {
                        guid: stack.item_guid,
                        entry_id,
                        db_guid: stack.db_guid,
                        inventory_type,
                    },
                );
            }
            let mut item_object = self.make_inventory_item_object(
                stack.item_guid,
                entry_id,
                player_guid,
                stack.stack_count,
                stack.max_durability,
                ItemContext::QuestReward,
                stack.slot,
            );
            if stack.bag != u8::from(wow_entities::INVENTORY_SLOT_BAG_0) {
                item_object.set_container_guid_and_slot(stack.contained_in, stack.bag);
            }
            if let Some(bonding) = item_bonding {
                item_object.set_bonding(bonding);
                item_object.bind_if_stored(is_bag_pos(wow_entities::make_item_pos(
                    stack.bag, stack.slot,
                )));
            }
            self.insert_inventory_item_object(item_object);
        }
        self.sync_object_accessor_player();

        let map_id = self.player_map_id_like_cpp();
        if !new_stacks.is_empty() {
            let item_creates = new_stacks
                .iter()
                .map(|stack| ItemCreateData {
                    item_guid: stack.item_guid,
                    entry_id: entry_id as i32,
                    owner_guid: player_guid,
                    contained_in: stack.contained_in,
                    stack_count: stack.stack_count,
                    dynamic_flags: stack.item_flags,
                    durability: stack.max_durability,
                    max_durability: stack.max_durability,
                    random_properties_seed: 0,
                    random_properties_id: 0,
                    enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
                    gems: Vec::new(),
                    context: ItemContext::QuestReward as u8,
                    container_slots: 0,
                    container_item_guids: [ObjectGuid::EMPTY; 36],
                })
                .collect();
            self.send_packet(&UpdateObject::create_items(item_creates, map_id));
        }

        for update in &existing_updates {
            self.send_packet(&UpdateObject::item_stack_count_update(
                update.item_guid,
                map_id,
                update.new_count,
            ));
        }

        if !new_stacks.is_empty() {
            let changed_slots: Vec<_> = new_stacks
                .iter()
                .filter(|stack| stack.bag == u8::from(wow_entities::INVENTORY_SLOT_BAG_0))
                .map(|stack| (stack.slot, stack.item_guid))
                .collect();
            if !changed_slots.is_empty() {
                self.send_player_values_update_from_entity_bridge(
                    &changed_slots,
                    &[],
                    &[],
                    &[],
                    None,
                );
            }
        }

        let quantity_in_inventory = self
            .represented_inventory_item_counts_like_cpp()
            .get(&entry_id)
            .copied()
            .unwrap_or(0);
        self.send_new_item_plan(&SendNewItemPlan {
            player_guid,
            item_guid: last_item_guid,
            item_entry: entry_id,
            item_instance: SendNewItemInstancePlan {
                item_id: entry_id,
                random_properties_seed: 0,
                random_properties_id: 0,
                modifications: Vec::<SendNewItemModifier>::new(),
            },
            slot: last_bag,
            slot_in_bag: if last_count_in_stack == quantity {
                i16::from(last_slot)
            } else {
                -1
            },
            quest_log_item_id: 0,
            quantity,
            quantity_in_inventory,
            battle_pet_species_id: 0,
            battle_pet_breed_id: 0,
            battle_pet_breed_quality: 0,
            battle_pet_level: 0,
            pushed: true,
            created: false,
            display_text: SendNewItemDisplayText::Normal,
            dungeon_encounter_id: 0,
            is_encounter_loot: false,
            delivery: SendNewItemDelivery::Direct,
        });
        true
    }

    async fn store_fixed_quest_reward_items_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        for (item_id, count) in quest.reward_items.iter().zip(quest.reward_amounts.iter()) {
            if *item_id == 0 {
                continue;
            }

            let (result, dest, _) = self
                .plan_store_new_direct_inventory_item(*item_id, *count)
                .unwrap_or((InventoryResult::ItemNotFound, Vec::new(), None));
            if result != InventoryResult::Ok {
                self.send_quest_failed_like_cpp(quest.id, result);
                return false;
            }
            if !self
                .store_quest_reward_item_like_cpp(*item_id, *count, &dest)
                .await
            {
                return false;
            }
        }

        true
    }

    async fn store_chosen_quest_reward_item_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        if choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP || choice.item_id == 0
        {
            return true;
        }

        if self
            .item_store()
            .is_none_or(|store| store.get(choice.item_id).is_none())
        {
            return true;
        }

        for ((item_id, count), item_type) in quest
            .reward_choice_items
            .iter()
            .zip(quest.reward_choice_item_types.iter())
        {
            if *item_id == 0
                || *item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
                || *item_id != choice.item_id
            {
                continue;
            }

            let (result, dest, _) = self
                .plan_store_new_direct_inventory_item(*item_id, *count)
                .unwrap_or((InventoryResult::ItemNotFound, Vec::new(), None));
            if result != InventoryResult::Ok {
                self.send_quest_failed_like_cpp(quest.id, result);
                return false;
            }
            if !self
                .store_quest_reward_item_like_cpp(*item_id, *count, &dest)
                .await
            {
                return false;
            }
        }

        true
    }

    async fn store_quest_package_reward_entry_like_cpp(
        &mut self,
        entry: &QuestPackageItemEntry,
    ) -> bool {
        let Ok(item_id) = u32::try_from(entry.item_id) else {
            self.send_quest_package_reward_inventory_error_like_cpp(
                InventoryResult::ItemNotFound,
                0,
            );
            return false;
        };

        let (result, dest, _) = self
            .plan_store_new_direct_inventory_item(item_id, entry.item_quantity)
            .unwrap_or((InventoryResult::ItemNotFound, Vec::new(), None));
        if result != InventoryResult::Ok {
            self.send_quest_package_reward_inventory_error_like_cpp(result, item_id);
            return false;
        }

        self.store_quest_reward_item_like_cpp(item_id, entry.item_quantity, &dest)
            .await
    }

    async fn store_quest_package_reward_items_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        if quest.quest_package_id == 0
            || choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
            || choice.item_id == 0
        {
            return true;
        }

        // C++ gates `RewardQuestPackage` behind a non-null selected reward item template.
        if self
            .item_store()
            .is_none_or(|store| store.get(choice.item_id).is_none())
        {
            return true;
        }

        let Some(store) = &self.quest_package_item_store else {
            return true;
        };
        let Ok(choice_item_id) = i32::try_from(choice.item_id) else {
            return true;
        };

        let primary_entries = store
            .quest_package_items_like_cpp(quest.quest_package_id)
            .filter(|entry| entry.item_id == choice_item_id)
            .cloned()
            .collect::<Vec<_>>();
        let fallback_entries = store
            .quest_package_items_fallback_like_cpp(quest.quest_package_id)
            .filter(|entry| entry.item_id == choice_item_id)
            .cloned()
            .collect::<Vec<_>>();

        let mut has_filtered_quest_package_reward = false;
        for entry in primary_entries {
            if !self.represented_can_select_quest_package_item_like_cpp(&entry) {
                continue;
            }

            has_filtered_quest_package_reward = true;
            if !self.store_quest_package_reward_entry_like_cpp(&entry).await {
                return false;
            }
        }

        if !has_filtered_quest_package_reward {
            for entry in fallback_entries {
                if !self.store_quest_package_reward_entry_like_cpp(&entry).await {
                    return false;
                }
            }
        }

        true
    }

    fn quest_reward_currency_gain_source_like_cpp(
        quest: &wow_data::quest::QuestTemplate,
    ) -> CurrencyGainSourceLikeCpp {
        if (quest.flags_ex & QUEST_FLAGS_EX_REWARDS_IGNORE_CAPS_LIKE_CPP) != 0 {
            if (quest.flags_ex & QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP) != 0 {
                return CurrencyGainSourceLikeCpp::WorldQuestRewardIgnoreCaps;
            }

            return CurrencyGainSourceLikeCpp::QuestRewardIgnoreCaps;
        }

        if quest.is_daily_like_cpp() {
            CurrencyGainSourceLikeCpp::DailyQuestReward
        } else if quest.is_weekly_like_cpp() {
            CurrencyGainSourceLikeCpp::WeeklyQuestReward
        } else if (quest.flags_ex & QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP) != 0 {
            CurrencyGainSourceLikeCpp::WorldQuestReward
        } else {
            CurrencyGainSourceLikeCpp::QuestReward
        }
    }

    async fn grant_quest_reward_currency_like_cpp(
        &mut self,
        currency_id: u32,
        amount: u32,
        gain_source: CurrencyGainSourceLikeCpp,
    ) -> bool {
        let currency_snapshot = self.player_currencies_like_cpp().clone();
        let delta = match self.add_currency_quest_reward_like_cpp(currency_id, amount, gain_source)
        {
            Ok(delta) => delta,
            Err(()) => {
                self.set_player_currencies_like_cpp(currency_snapshot);
                return false;
            }
        };

        if let Some(char_db) = self.char_db().map(Arc::clone) {
            if let Some(player_guid) = self.player_guid() {
                let mut tx = SqlTransaction::new();
                self.append_player_currency_save_statements(&mut tx, player_guid.counter() as u64);
                if let Err(error) = char_db.commit_transaction(tx).await {
                    self.set_player_currencies_like_cpp(currency_snapshot);
                    warn!(
                        account = self.account_id,
                        currency_id,
                        ?error,
                        "ChooseReward: quest reward currency save failed"
                    );
                    return false;
                }
            }
        }

        if let Some(delta) = delta {
            let (Some(quantity), Some(amount)) = (
                i32::try_from(delta.quantity).ok(),
                i32::try_from(delta.amount).ok(),
            ) else {
                return true;
            };
            let mut packet = SetCurrency {
                type_id: delta.currency_id as i32,
                quantity,
                flags: 0,
                weekly_quantity: delta
                    .weekly_quantity
                    .and_then(|value| i32::try_from(value).ok()),
                tracked_quantity: None,
                max_quantity: delta
                    .max_quantity
                    .and_then(|value| i32::try_from(value).ok()),
                total_earned: delta
                    .total_earned
                    .and_then(|value| i32::try_from(value).ok()),
                suppress_chat_log: delta.suppress_chat_log,
                quantity_change: Some(amount),
                quantity_gain_source: Some(gain_source as i32),
                quantity_lost_source: None,
                first_craft_operation_id: None,
                next_recharge_time: None,
                recharge_cycle_start_time: None,
                overflown_currency_id: None,
            };
            packet.suppress_chat_log = delta.suppress_chat_log;
            self.send_packet(&packet);
        }

        true
    }

    async fn grant_quest_reward_currencies_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        let gain_source = Self::quest_reward_currency_gain_source_like_cpp(quest);

        if choice.loot_item_type == QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP
            && choice.item_id != 0
            && self
                .currency_types_store()
                .is_some_and(|store| store.has_record(choice.item_id))
        {
            for ((currency_id, count), item_type) in quest
                .reward_choice_items
                .iter()
                .zip(quest.reward_choice_item_types.iter())
            {
                if *currency_id == 0
                    || *item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP
                    || *currency_id != choice.item_id
                {
                    continue;
                }

                if !self
                    .grant_quest_reward_currency_like_cpp(*currency_id, *count, gain_source)
                    .await
                {
                    return false;
                }
            }
        }

        for (currency_id, count) in quest
            .reward_currencies
            .iter()
            .zip(quest.reward_currency_amounts.iter())
        {
            if *currency_id == 0 || *count == 0 {
                continue;
            }

            if !self
                .grant_quest_reward_currency_like_cpp(*currency_id, *count, gain_source)
                .await
            {
                return false;
            }
        }

        true
    }

    fn represented_direct_inventory_count_like_cpp(&self, item_entry: u32) -> u32 {
        self.inventory_items_like_cpp()
            .values()
            .filter(|item| item.entry_id == item_entry)
            .filter_map(|inventory_item| {
                self.inventory_item_objects_like_cpp()
                    .get(&inventory_item.guid)
                    .filter(|item| !item.is_in_trade())
                    .map(|item| item.count())
            })
            .fold(0u32, u32::saturating_add)
    }

    fn plan_quest_destroy_item_count_direct_like_cpp(
        &self,
        item_entry: u32,
        count: u32,
    ) -> Option<Vec<ExtendedCostItemTurninChange>> {
        let effective_count = if count == u32::MAX {
            self.represented_direct_inventory_count_like_cpp(item_entry)
        } else {
            count
        };

        if effective_count == 0 {
            return Some(Vec::new());
        }

        self.plan_destroy_item_count_direct_inventory(item_entry, effective_count)
    }

    async fn remove_quest_required_items_and_currencies_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let map_id = self.player_map_id_like_cpp();
        let mut item_changes = Vec::new();
        let currency_snapshot = self.player_currencies_like_cpp().clone();
        let mut currency_losses = Vec::new();

        for objective in &quest.objectives {
            match objective.obj_type {
                QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL => {
                    let Ok(item_entry) = u32::try_from(objective.object_id) else {
                        return false;
                    };
                    let count = if (quest.flags & QUEST_FLAGS_REMOVE_SURPLUS_ITEMS_LIKE_CPP) != 0 {
                        u32::MAX
                    } else {
                        u32::try_from(objective.amount).unwrap_or(u32::MAX)
                    };
                    let Some(mut changes) =
                        self.plan_quest_destroy_item_count_direct_like_cpp(item_entry, count)
                    else {
                        return false;
                    };
                    item_changes.append(&mut changes);
                }
                QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL => {
                    let (Ok(currency_id), Ok(amount)) = (
                        u32::try_from(objective.object_id),
                        u32::try_from(objective.amount),
                    ) else {
                        return false;
                    };
                    let before = self.player_currency_quantity(currency_id);
                    if !self.remove_currency(currency_id, amount) {
                        self.set_player_currencies_like_cpp(currency_snapshot);
                        return false;
                    }
                    let after = self.player_currency_quantity(currency_id);
                    let removed = before.saturating_sub(after);
                    if removed > 0 {
                        currency_losses.push((currency_id, after, removed));
                    }
                }
                _ => {}
            }
        }

        if (quest.flags_ex & QUEST_FLAGS_EX_NO_ITEM_REMOVAL_LIKE_CPP) == 0 {
            for (item_entry, count) in quest.item_drop.iter().zip(quest.item_drop_quantity.iter()) {
                if *item_entry == 0 {
                    continue;
                }
                let count = if *count == 0 { u32::MAX } else { *count };
                let Some(mut changes) =
                    self.plan_quest_destroy_item_count_direct_like_cpp(*item_entry, count)
                else {
                    self.set_player_currencies_like_cpp(currency_snapshot);
                    return false;
                };
                item_changes.append(&mut changes);
            }
        }

        if let Some(char_db) = self.char_db().map(Arc::clone) {
            let mut tx = SqlTransaction::new();
            Self::append_item_turnin_statements(
                char_db.as_ref(),
                &mut tx,
                player_guid,
                &item_changes,
            );
            self.append_player_currency_save_statements(&mut tx, player_guid.counter() as u64);
            if let Err(error) = char_db.commit_transaction(tx).await {
                self.set_player_currencies_like_cpp(currency_snapshot);
                warn!(
                    account = self.account_id,
                    quest_id = quest.id,
                    ?error,
                    "ChooseReward: quest objective item/currency removal save failed"
                );
                return false;
            }
        }

        self.apply_item_turnin_changes(player_guid, map_id, &item_changes);
        for (currency_id, quantity, removed) in currency_losses {
            let (Some(quantity), Some(removed)) =
                (i32::try_from(quantity).ok(), i32::try_from(removed).ok())
            else {
                continue;
            };
            self.send_packet(&SetCurrency {
                type_id: currency_id as i32,
                quantity,
                flags: 0,
                weekly_quantity: None,
                tracked_quantity: None,
                max_quantity: None,
                total_earned: None,
                suppress_chat_log: false,
                quantity_change: Some(-removed),
                quantity_gain_source: None,
                quantity_lost_source: Some(CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP),
                first_craft_operation_id: None,
                next_recharge_time: None,
                recharge_cycle_start_time: None,
                overflown_currency_id: None,
            });
        }

        true
    }

    fn remove_represented_timed_quest_like_cpp(&mut self, quest_id: u32) {
        if let Some(status) = self.player_quests.get_mut(&quest_id)
            && status.end_time_secs > 0
        {
            status.end_time_secs = 0;
            self.represented_timed_quest_removals_like_cpp
                .push(quest_id);
        }
    }

    fn apply_represented_quest_reward_skill_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        if quest.reward_skill_line_id != 0 {
            self.represented_quest_reward_skill_updates_like_cpp
                .push((quest.reward_skill_line_id, quest.reward_skill_points));
        }
    }

    fn record_represented_quest_reward_spell_casts_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        let caster_selection_unrepresented =
            (quest.flags & QUEST_FLAGS_PLAYER_CAST_COMPLETE_LIKE_CPP) == 0;
        if quest.reward_spell > 0 {
            self.represented_quest_reward_spell_casts_like_cpp.push(
                RepresentedQuestRewardSpellCastLikeCpp {
                    quest_id: quest.id,
                    spell_id: quest.reward_spell,
                    kind: RepresentedQuestRewardSpellKindLikeCpp::RewardSpell,
                    can_delay_teleport_like_cpp: self.represented_can_delay_teleport_like_cpp(),
                    spell_info_lookup_unrepresented: true,
                    caster_selection_unrepresented,
                    cast_spell_runtime_unrepresented: true,
                },
            );
            return;
        }

        let display_spells = quest.reward_display_spell;
        for (index, spell_id) in display_spells.into_iter().enumerate() {
            if spell_id == 0 {
                continue;
            }
            self.represented_quest_reward_spell_casts_like_cpp.push(
                RepresentedQuestRewardSpellCastLikeCpp {
                    quest_id: quest.id,
                    spell_id,
                    kind: RepresentedQuestRewardSpellKindLikeCpp::RewardDisplaySpell {
                        index: index as u8,
                    },
                    can_delay_teleport_like_cpp: self.represented_can_delay_teleport_like_cpp(),
                    spell_info_lookup_unrepresented: true,
                    caster_selection_unrepresented,
                    cast_spell_runtime_unrepresented: true,
                },
            );
        }
    }

    fn apply_represented_quest_title_and_talent_rewards_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        if quest.reward_title_id != 0 {
            self.represented_quest_reward_titles_like_cpp.push(
                RepresentedQuestRewardTitleLikeCpp {
                    quest_id: quest.id,
                    title_id: quest.reward_title_id,
                    char_title_lookup_unrepresented: true,
                    set_title_runtime_unrepresented: true,
                },
            );
        }

        if quest.reward_skill_points != 0 {
            self.represented_quest_reward_talent_points_like_cpp.push(
                RepresentedQuestRewardTalentPointsLikeCpp {
                    quest_id: quest.id,
                    points: quest.reward_skill_points,
                    init_talent_for_level_unrepresented: true,
                },
            );
        }
    }

    fn record_represented_quest_reward_mail_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: ObjectGuid,
    ) {
        if quest.reward_mail_template_id == 0 {
            return;
        }

        self.represented_quest_reward_mails_like_cpp
            .push(RepresentedQuestRewardMailLikeCpp {
                quest_id: quest.id,
                mail_template_id: quest.reward_mail_template_id,
                delay_secs: quest.reward_mail_delay_secs,
                sender_entry: (quest.reward_mail_sender_entry != 0)
                    .then_some(quest.reward_mail_sender_entry),
                quest_giver_guid: (quest.reward_mail_sender_entry == 0).then_some(quest_giver_guid),
                mail_template_lookup_unrepresented: true,
                mail_draft_runtime_unrepresented: true,
                character_db_transaction_unrepresented: true,
            });
    }

    fn record_represented_quest_reward_reputation_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        let source = if quest.is_daily_like_cpp() {
            RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest
        } else if quest.is_weekly_like_cpp() {
            RepresentedQuestRewardReputationSourceLikeCpp::WeeklyQuest
        } else if quest.is_monthly_like_cpp() {
            RepresentedQuestRewardReputationSourceLikeCpp::MonthlyQuest
        } else if quest.is_repeatable() {
            RepresentedQuestRewardReputationSourceLikeCpp::RepeatableQuest
        } else {
            RepresentedQuestRewardReputationSourceLikeCpp::Quest
        };
        let gain_source = match source {
            RepresentedQuestRewardReputationSourceLikeCpp::Quest => {
                ReputationGainSourceLikeCpp::Quest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest => {
                ReputationGainSourceLikeCpp::DailyQuest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::WeeklyQuest => {
                ReputationGainSourceLikeCpp::WeeklyQuest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::MonthlyQuest => {
                ReputationGainSourceLikeCpp::MonthlyQuest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::RepeatableQuest => {
                ReputationGainSourceLikeCpp::RepeatableQuest
            }
        };
        let faction_store = self.faction_store().map(Arc::clone);
        let quest_faction_reward_store = self.quest_faction_reward_store.as_ref().map(Arc::clone);
        let reputation_reward_rate_store = self.reputation_reward_rate_store().map(Arc::clone);
        let reputation_spillover_template_store =
            self.reputation_spillover_template_store().map(Arc::clone);
        let friendship_rep_reaction_store = self.friendship_rep_reaction_store().map(Arc::clone);
        let paragon_reputation_store = self.paragon_reputation_store().map(Arc::clone);
        let currency_types_store = self.currency_types_store().map(Arc::clone);

        for slot in 0..wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT {
            let faction_id = quest.reward_faction_ids[slot];
            if faction_id == 0 {
                continue;
            }
            let faction_entry = match faction_store.as_deref() {
                Some(store) => match store.get(faction_id).cloned() {
                    Some(entry) => Some(entry),
                    None => continue,
                },
                None => None,
            };
            let faction_lookup_missing = faction_entry.is_none();

            let reward_faction_override = quest.reward_faction_overrides[slot];
            let (base_reputation_before_gain, no_quest_bonus, quest_faction_reward_lookup) =
                if reward_faction_override != 0 {
                    (reward_faction_override / 100, true, false)
                } else if let Some(store) = quest_faction_reward_store.as_deref() {
                    let row = if quest.reward_faction_values[slot] < 0 {
                        2
                    } else {
                        1
                    };
                    let field = quest.reward_faction_values[slot].unsigned_abs() as usize;
                    let rep = store
                        .get(row)
                        .and_then(|entry| entry.difficulty.get(field).copied())
                        .map(i32::from)
                        .unwrap_or(0);
                    (rep, false, false)
                } else {
                    (0, false, true)
                };

            if base_reputation_before_gain == 0 && !quest_faction_reward_lookup {
                continue;
            }

            let quest_level_for_gain =
                player_quest_level_like_cpp(quest, self.player_level_like_cpp()).max(0) as u32;
            let reputation_rates = self.reputation_rates_like_cpp();
            let Some(percent_before_reward_rate) = self
                .reputation_gain_percent_before_reward_rate_like_cpp(
                    gain_source,
                    quest_level_for_gain,
                    base_reputation_before_gain,
                    faction_id,
                    no_quest_bonus,
                )
            else {
                continue;
            };
            let reputation_after_low_level_rate_like_cpp = calculate_pct_i32_f32_like_cpp(
                base_reputation_before_gain,
                percent_before_reward_rate,
            );
            if reputation_after_low_level_rate_like_cpp == 0 && !quest_faction_reward_lookup {
                continue;
            }

            let (
                reputation_after_reward_rate_like_cpp,
                percent_after_reward_rate_like_cpp,
                reputation_reward_rate_lookup,
            ) = if reputation_reward_rate_store.is_some() {
                if let Some(rate) =
                    self.reputation_reward_rate_for_source_like_cpp(gain_source, faction_id)
                {
                    if rate <= 0.0 {
                        continue;
                    }
                    let percent = percent_before_reward_rate * rate;
                    (
                        calculate_pct_i32_f32_like_cpp(base_reputation_before_gain, percent),
                        percent,
                        false,
                    )
                } else {
                    (
                        reputation_after_low_level_rate_like_cpp,
                        percent_before_reward_rate,
                        false,
                    )
                }
            } else {
                (
                    reputation_after_low_level_rate_like_cpp,
                    percent_before_reward_rate,
                    true,
                )
            };
            let reputation_after_recruit_a_friend_bonus_like_cpp = calculate_pct_i32_f32_like_cpp(
                base_reputation_before_gain,
                self.apply_recruit_a_friend_reputation_bonus_like_cpp(
                    gain_source,
                    percent_after_reward_rate_like_cpp,
                ),
            );
            if reputation_after_recruit_a_friend_bonus_like_cpp == 0 && !quest_faction_reward_lookup
            {
                continue;
            }

            let current_rank_for_cap = if quest.reward_faction_cap_in[slot] != 0
                && reputation_after_recruit_a_friend_bonus_like_cpp > 0
            {
                self.canonical_player_reputation_standing_like_cpp(faction_id)
                    .map(reputation_rank_from_standing_like_cpp)
            } else {
                None
            };
            if current_rank_for_cap.is_some_and(|current_rank| {
                i32::from(current_rank) >= quest.reward_faction_cap_in[slot]
            }) {
                continue;
            }

            let no_spillover = (quest.reward_faction_flags & (1u32 << slot)) != 0;
            let modify_reputation_runtime_unrepresented =
                if let (Some(faction_entry), Some(faction_store)) =
                    (faction_entry.as_ref(), faction_store.as_deref())
                {
                    let options = crate::reputation::mgr::SetReputationOptionsLikeCpp {
                        incremental: true,
                        spillover_only: false,
                        no_spillover,
                        reputation_gain_rate: reputation_rates.gain,
                        paragon_reward_quest_status_none_like_cpp: true,
                        renown_current_level_like_cpp: 0,
                        renown_currency_increased_cap_quantity_like_cpp: 0,
                        player_race: self.player_race_like_cpp(),
                        player_class: self.player_class_like_cpp(),
                    };
                    let db_spillover_template = reputation_spillover_template_store
                        .as_deref()
                        .and_then(|store| store.get(faction_id));
                    let outcome = self.reputation_mgr_like_cpp_mut().set_reputation_like_cpp(
                        faction_entry,
                        reputation_after_recruit_a_friend_bonus_like_cpp,
                        options,
                        faction_store,
                        db_spillover_template,
                        friendship_rep_reaction_store.as_deref(),
                        paragon_reputation_store.as_deref(),
                        currency_types_store.as_deref(),
                    );
                    if let Some(rep_list_id) = outcome.send_state_rep_list_id {
                        let packet = self
                            .reputation_mgr_like_cpp_mut()
                            .set_faction_standing_packet_like_cpp(Some(rep_list_id));
                        self.send_packet(&packet);
                    }
                    false
                } else {
                    true
                };

            self.represented_quest_reward_reputations_like_cpp.push(
                RepresentedQuestRewardReputationLikeCpp {
                    quest_id: quest.id,
                    slot: slot as u8,
                    faction_id,
                    reward_faction_value: quest.reward_faction_values[slot],
                    reward_faction_override,
                    reward_faction_cap_in: quest.reward_faction_cap_in[slot],
                    base_reputation_before_gain,
                    reputation_after_low_level_rate_like_cpp,
                    reputation_after_reward_rate_like_cpp,
                    no_quest_bonus,
                    no_spillover,
                    source,
                    faction_store_lookup_unrepresented: faction_lookup_missing,
                    quest_faction_reward_store_lookup_unrepresented: quest_faction_reward_lookup,
                    reputation_reward_rate_lookup_unrepresented: reputation_reward_rate_lookup,
                    gray_level_script_hook_unrepresented: true,
                    reputation_rank_cap_check_unrepresented: quest.reward_faction_cap_in[slot] != 0
                        && reputation_after_recruit_a_friend_bonus_like_cpp > 0
                        && current_rank_for_cap.is_none(),
                    calculate_reputation_gain_unrepresented: true,
                    modify_reputation_runtime_unrepresented,
                },
            );
        }
    }

    async fn apply_quest_reward_lockout_status_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        let now = GameTime::now().as_secs() as i64;
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        let mut save_daily = false;
        let mut save_weekly = false;
        let mut save_monthly = false;
        let mut save_seasonal = false;

        if quest.is_daily_like_cpp() || quest.is_df_quest_like_cpp() {
            self.last_daily_quest_time_like_cpp = now;
            if quest.is_df_quest_like_cpp() {
                self.df_quests_like_cpp.insert(quest.id);
            } else {
                self.daily_quests_completed_like_cpp.insert(quest.id);
            }
            save_daily = true;
        } else if quest.is_weekly_like_cpp() {
            self.weekly_quests_completed_like_cpp.insert(quest.id);
            save_weekly = true;
        } else if quest.is_monthly_like_cpp() {
            self.monthly_quests_completed_like_cpp.insert(quest.id);
            save_monthly = true;
        } else if quest.is_seasonal_like_cpp() {
            self.seasonal_quests_like_cpp
                .entry(quest.event_id_for_quest_like_cpp())
                .or_default()
                .insert(quest.id, now.max(0) as u64);
            self.seasonal_quest_changed_like_cpp = true;
            save_seasonal = true;
        }

        let Some(char_db) = self.char_db().map(Arc::clone) else {
            return;
        };

        let guid = player_guid.counter() as u64;
        let mut tx = SqlTransaction::new();

        if save_daily {
            let mut del = char_db.prepare(CharStatements::DEL_CHARACTER_QUESTSTATUS_DAILY);
            del.set_u64(0, guid);
            tx.append(del);

            for quest_id in &self.daily_quests_completed_like_cpp {
                let mut ins = char_db.prepare(CharStatements::INS_CHARACTER_QUESTSTATUS_DAILY);
                ins.set_u64(0, guid);
                ins.set_u32(1, *quest_id);
                ins.set_i64(2, self.last_daily_quest_time_like_cpp);
                tx.append(ins);
            }
            for quest_id in &self.df_quests_like_cpp {
                let mut ins = char_db.prepare(CharStatements::INS_CHARACTER_QUESTSTATUS_DAILY);
                ins.set_u64(0, guid);
                ins.set_u32(1, *quest_id);
                ins.set_i64(2, self.last_daily_quest_time_like_cpp);
                tx.append(ins);
            }
        }

        if save_weekly {
            let mut del = char_db.prepare(CharStatements::DEL_CHARACTER_QUESTSTATUS_WEEKLY);
            del.set_u64(0, guid);
            tx.append(del);

            for quest_id in &self.weekly_quests_completed_like_cpp {
                let mut ins = char_db.prepare(CharStatements::INS_CHARACTER_QUESTSTATUS_WEEKLY);
                ins.set_u64(0, guid);
                ins.set_u32(1, *quest_id);
                tx.append(ins);
            }
        }

        if save_monthly {
            let mut del = char_db.prepare(CharStatements::DEL_CHARACTER_QUESTSTATUS_MONTHLY);
            del.set_u64(0, guid);
            tx.append(del);

            for quest_id in &self.monthly_quests_completed_like_cpp {
                let mut ins = char_db.prepare(CharStatements::INS_CHARACTER_QUESTSTATUS_MONTHLY);
                ins.set_u64(0, guid);
                ins.set_u32(1, *quest_id);
                tx.append(ins);
            }
        }

        if save_seasonal {
            let mut del = char_db.prepare(CharStatements::DEL_CHARACTER_QUESTSTATUS_SEASONAL);
            del.set_u64(0, guid);
            tx.append(del);

            for (event_id, quests) in &self.seasonal_quests_like_cpp {
                for (quest_id, completed_time) in quests {
                    let Some(completed_time) = i64::try_from(*completed_time).ok() else {
                        continue;
                    };
                    let mut ins =
                        char_db.prepare(CharStatements::INS_CHARACTER_QUESTSTATUS_SEASONAL);
                    ins.set_u64(0, guid);
                    ins.set_u32(1, *quest_id);
                    ins.set_u32(2, u32::from(*event_id));
                    ins.set_i64(3, completed_time);
                    tx.append(ins);
                }
            }
        }

        if let Err(error) = char_db.commit_transaction(tx).await {
            warn!(
                account = self.account_id,
                quest_id = quest.id,
                ?error,
                "ChooseReward: represented reward lockout status save failed"
            );
        }
    }

    async fn save_represented_quest_statuses_completed_after_like_cpp(
        &mut self,
        completion_evidence_start: usize,
    ) {
        let completed_quest_ids: Vec<_> = self.represented_quest_complete_status_updates_like_cpp
            [completion_evidence_start..]
            .iter()
            .filter_map(|evidence| {
                (evidence.new_status == QUEST_STATUS_COMPLETE_LIKE_CPP).then_some(evidence.quest_id)
            })
            .collect();
        for quest_id in completed_quest_ids {
            self.save_represented_quest_status_like_cpp(quest_id).await;
        }
    }

    /// CMSG_QUEST_CONFIRM_ACCEPT — confirm accepting a shared quest.
    ///
    /// C++ anchor: `WorldSession::HandleQuestConfirmAccept`, `QuestHandler.cpp:499-531`.
    /// Represented-partial: validates against session-local pending sharing state, clears before
    /// quest-template lookup like C++, then records safe represented post-template gates.
    /// No-source-item quests and source-item no-grant branches consume only local quest-log insertion
    /// + Character DB status save + PlayerRegistry snapshot sync from `Player::AddQuest`. Real
    /// `StoreNewItem`/`SendNewItem`, criteria/completion, timed/PvP, scripts, and `SendQuestUpdate`
    /// packet fanout remain explicit no-mutation boundaries.
    pub async fn handle_quest_confirm_accept(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match QuestConfirmAccept::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "QuestConfirmAccept: failed to read signed QuestID"
                );
                return;
            }
        };

        let parsed_quest_id = packet.quest_id as u32;
        let Some(pending) = self.represented_pending_quest_sharing_like_cpp() else {
            debug!(
                account = self.account_id,
                raw_quest_id = packet.quest_id,
                parsed_quest_id,
                "QuestConfirmAccept: no represented pending shared quest"
            );
            return;
        };

        if pending.quest_id != parsed_quest_id {
            debug!(
                account = self.account_id,
                pending_quest_id = pending.quest_id,
                raw_quest_id = packet.quest_id,
                parsed_quest_id,
                "QuestConfirmAccept: represented pending quest id mismatch; pending state preserved"
            );
            return;
        }

        self.clear_represented_pending_quest_sharing_like_cpp();

        let Some(quest_store) = &self.quest_store else {
            debug!(
                account = self.account_id,
                parsed_quest_id,
                "QuestConfirmAccept: pending cleared before missing quest store like C++ order"
            );
            return;
        };

        let Some(quest) = quest_store.get(parsed_quest_id).cloned() else {
            debug!(
                account = self.account_id,
                parsed_quest_id,
                "QuestConfirmAccept: pending cleared before missing quest template like C++ order"
            );
            return;
        };

        let receiver_guid = self.player_guid();
        let record = |session: &mut WorldSession,
                      reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
                      can_add_source_item_unrepresented: bool,
                      can_add_source_item_result: Option<InventoryResult>,
                      add_quest_runtime_unrepresented: bool,
                      source_spell_unrepresented: bool,
                      represented_source_spell_id: Option<u32>,
                      represented_source_spell_self_casts: u8| {
            session.record_represented_quest_confirm_accept_like_cpp(
                RepresentedQuestConfirmAcceptLikeCpp {
                    receiver_guid,
                    sender_guid_before_clear: pending.sender_guid,
                    quest_id: parsed_quest_id,
                    raw_quest_id: packet.quest_id,
                    reason,
                    object_accessor_unrepresented: true,
                    party_runtime_unrepresented: true,
                    can_add_source_item_unrepresented,
                    can_add_source_item_result,
                    add_quest_runtime_unrepresented,
                    source_spell_unrepresented,
                    represented_source_spell_id,
                    represented_source_spell_self_casts,
                },
            );
        };

        let Some(player_registry) = self.player_registry().map(Arc::clone) else {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerMissing,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        };

        let Some(sender_active_status) =
            player_registry.quest_active_status(pending.sender_guid, parsed_quest_id)
        else {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerMissing,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        };

        let Some(receiver_guid) = receiver_guid else {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::NotInSameRaid,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        };

        let Some(group_registry) = self.group_registry().map(Arc::clone) else {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::NotInSameRaid,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        };

        let same_represented_group = group_registry.iter().any(|entry| {
            let members = &entry.value().members;
            members.contains(&receiver_guid) && members.contains(&pending.sender_guid)
        });
        if !same_represented_group {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::NotInSameRaid,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        }

        if !matches!(
            sender_active_status,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP | QUEST_STATUS_COMPLETE_LIKE_CPP)
        ) {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerNotActiveQuest,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        }

        if !self.can_take_quest(&quest) {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanTakeQuestFailed,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        }

        if self.first_free_quest_slot_like_cpp().is_none() {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestLogFull,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        }

        if quest.source_item_id > 0 {
            let Some(source_item_template) = self.item_storage_template(quest.source_item_id)
            else {
                let source_item_result = InventoryResult::ItemNotFound;
                self.send_equip_error(source_item_result, None, None, 0, 0);
                record(
                    self,
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed,
                    false,
                    Some(source_item_result),
                    false,
                    false,
                    None,
                    0,
                );
                return;
            };

            let source_item_count = quest.source_item_count.max(1);
            let (source_item_result, source_item_dest, _) = self
                .plan_store_new_direct_inventory_item(quest.source_item_id, source_item_count)
                .unwrap_or((InventoryResult::ItemNotFound, Vec::new(), None));

            if !matches!(
                source_item_result,
                InventoryResult::Ok | InventoryResult::ItemMaxCount
            ) {
                self.send_equip_error(
                    source_item_result,
                    None,
                    None,
                    0,
                    u32::from(source_item_template.item_limit_category),
                );
                record(
                    self,
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed,
                    false,
                    Some(source_item_result),
                    false,
                    false,
                    None,
                    0,
                );
                return;
            }

            let source_item_no_grant_reason = if self
                .item_template_start_quest_id(quest.source_item_id)
                .is_some_and(|start_quest_id| start_quest_id == quest.id as i32)
            {
                Some(RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemStartQuestNoGrant)
            } else if source_item_result == InventoryResult::ItemMaxCount {
                Some(RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemMaxCountNoGrant)
            } else {
                None
            };

            if let Some(source_item_no_grant_reason) = source_item_no_grant_reason {
                if !self
                    .add_quest_confirm_accept_local_state_like_cpp(&quest)
                    .await
                {
                    record(
                        self,
                        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestLogFull,
                        false,
                        None,
                        false,
                        false,
                        None,
                        0,
                    );
                    return;
                }

                let represented_source_spell_id =
                    (quest.source_spell_id > 0).then_some(quest.source_spell_id);
                let represented_source_spell_self_casts = u8::from(quest.source_spell_id > 0) * 2;
                record(
                    self,
                    source_item_no_grant_reason,
                    false,
                    Some(source_item_result),
                    false,
                    false,
                    represented_source_spell_id,
                    represented_source_spell_self_casts,
                );
                return;
            }

            if !self
                .add_quest_confirm_accept_local_state_like_cpp(&quest)
                .await
            {
                record(
                    self,
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestLogFull,
                    false,
                    None,
                    false,
                    false,
                    None,
                    0,
                );
                return;
            }

            let Some(source_item_store_outcome) = self
                .store_quest_source_item_like_cpp(
                    quest.source_item_id,
                    source_item_count,
                    &source_item_dest,
                )
                .await
            else {
                record(
                    self,
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::GiveQuestSourceItemStoreNewItemUnrepresented,
                    false,
                    Some(source_item_result),
                    true,
                    quest.source_spell_id > 0,
                    None,
                    0,
                );
                return;
            };

            let represented_source_spell_id =
                (quest.source_spell_id > 0).then_some(quest.source_spell_id);
            let represented_source_spell_self_casts = u8::from(quest.source_spell_id > 0) * 2;
            let source_item_store_reason = match source_item_store_outcome {
                QuestSourceItemStoreOutcomeLikeCpp::StoredNewItem => {
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemStoredNewItem
                }
                QuestSourceItemStoreOutcomeLikeCpp::BoundObjectiveNoGrant => {
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemBoundObjectiveNoGrant
                }
            };
            record(
                self,
                source_item_store_reason,
                false,
                Some(source_item_result),
                false,
                false,
                represented_source_spell_id,
                represented_source_spell_self_casts,
            );
            return;
        }

        if !self
            .add_quest_confirm_accept_local_state_like_cpp(&quest)
            .await
        {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestLogFull,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        }

        let represented_source_spell_id =
            (quest.source_spell_id > 0).then_some(quest.source_spell_id);
        let represented_source_spell_self_casts = u8::from(quest.source_spell_id > 0) * 2;
        record(
            self,
            RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
            false,
            None,
            false,
            false,
            represented_source_spell_id,
            represented_source_spell_self_casts,
        );
    }

    /// CMSG_PUSH_QUEST_TO_PARTY — sender-side bounded quest share preflight.
    ///
    /// C++ anchors:
    /// - `Opcodes.cpp:746`: `STATUS_LOGGEDIN`, `PROCESS_THREADUNSAFE`, `HandlePushQuestToParty`.
    /// - `QuestPackets.cpp:658-661`: packet reads one `uint32 QuestID`.
    /// - `QuestHandler.cpp:603-756`: template lookup, `CanShareQuest`, quest-pool active,
    ///   group presence, then receiver iteration.
    ///
    /// Represented-partial: this records sender-local evidence only. It never mutates DB/maps,
    /// never sets receiver pending sharing, and never fans out packets to other sessions.
    /// If the session has no real `player_guid`, Rust records the existing evidence only and
    /// does not fabricate an empty sender GUID for `SMSG_QUEST_PUSH_RESULT`.
    pub async fn handle_push_quest_to_party(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match PushQuestToParty::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "PushQuestToParty: failed to read QuestID"
                );
                return;
            }
        };

        let Some(quest_store) = self.quest_store.as_ref().map(Arc::clone) else {
            debug!(
                account = self.account_id,
                quest_id = packet.quest_id,
                "PushQuestToParty: missing QuestStore, silent return like missing ObjectMgr template path"
            );
            return;
        };

        let Some(quest) = quest_store.get(packet.quest_id) else {
            debug!(
                account = self.account_id,
                quest_id = packet.quest_id,
                "PushQuestToParty: missing quest template, silent return like C++"
            );
            return;
        };

        let sender_guid = self.player_guid();
        if !self.represented_can_share_quest_like_cpp(quest) {
            self.send_push_quest_result_to_sender_if_available_like_cpp(
                sender_guid,
                quest_push_reason::NOT_ALLOWED,
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotAllowed,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return;
        }

        let Some(quest_pool_store) = self.quest_pool_store.as_ref().map(Arc::clone) else {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::QuestPoolActiveCheckUnrepresented,
                    quest_pool_active_check_unrepresented: true,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return;
        };

        if !quest_pool_store.is_quest_active_like_cpp(packet.quest_id) {
            self.send_push_quest_result_to_sender_if_available_like_cpp(
                sender_guid,
                quest_push_reason::NOT_DAILY,
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotDaily,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return;
        }

        if self.group_guid.is_none() {
            self.send_push_quest_result_to_sender_if_available_like_cpp(
                sender_guid,
                quest_push_reason::NOT_IN_PARTY,
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotInParty,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
            return;
        }

        let Some(group_guid) = self.group_guid else {
            return;
        };

        let Some(group_registry) = self.group_registry().map(Arc::clone) else {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason:
                        RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: true,
                    receiver_fanout_unrepresented: true,
                },
            );
            return;
        };

        let Some(player_registry) = self.player_registry().map(Arc::clone) else {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason:
                        RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: true,
                    receiver_fanout_unrepresented: true,
                },
            );
            return;
        };

        let Some(group_info) = group_registry.get(&group_guid).map(|entry| entry.clone()) else {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason:
                        RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: true,
                    receiver_fanout_unrepresented: true,
                },
            );
            return;
        };

        let receiver_snapshots = group_info
            .members
            .iter()
            .copied()
            .filter(|member_guid| Some(*member_guid) != sender_guid)
            .filter_map(|member_guid| {
                player_registry
                    .quest_sharing_snapshot(member_guid)
                    .map(|receiver| (member_guid, receiver))
            })
            .collect::<Vec<_>>();

        if receiver_snapshots.is_empty() {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: true,
                },
            );
            return;
        }

        let mut blocked_by_unsupported_success_path = false;
        for (receiver_guid, receiver) in receiver_snapshots {
            if receiver.pending_quest_sharing.is_some() {
                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_BUSY_LIKE_CPP,
                    String::new(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverBusy,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if !receiver.is_alive {
                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_DEAD_LIKE_CPP,
                    String::new(),
                );
                if let Some(sender_guid) = sender_guid {
                    let _ = player_registry.send_current_packet(
                        receiver.registration,
                        QuestPushResultResponse {
                            sender_guid,
                            result: QUEST_PUSH_REASON_DEAD_TO_RECIPIENT_LIKE_CPP,
                            quest_title: quest.log_title.clone(),
                        }
                        .to_bytes(),
                    );
                }
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverDead,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if receiver.rewarded_quests.contains(&packet.quest_id) {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason:
                            RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverAlreadyDone,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if let Some(status) = receiver
                .active_quest_statuses
                .get(&packet.quest_id)
                .copied()
            {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                if status == QUEST_STATUS_INCOMPLETE_LIKE_CPP
                    || status == QUEST_STATUS_COMPLETE_LIKE_CPP
                {
                    self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                        receiver_guid,
                        QUEST_PUSH_REASON_ON_QUEST_LIKE_CPP,
                        String::new(),
                    );
                    let _ = player_registry.send_current_packet(
                        receiver.registration,
                        QuestPushResultResponse {
                            sender_guid: sender_guid_for_receiver_packet,
                            result: QUEST_PUSH_REASON_ON_QUEST_TO_RECIPIENT_LIKE_CPP,
                            quest_title: quest.log_title.clone(),
                        }
                        .to_bytes(),
                    );
                    self.record_represented_push_quest_to_party_outcome_like_cpp(
                        RepresentedPushQuestToPartyOutcomeLikeCpp {
                            sender_guid,
                            quest_id: packet.quest_id,
                            target_guid: Some(receiver_guid),
                            reason:
                                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverOnQuest,
                            quest_pool_active_check_unrepresented: false,
                            group_runtime_unrepresented: false,
                            receiver_fanout_unrepresented: false,
                        },
                    );
                    continue;
                }
            }

            // C++ `Player::SatisfyQuestLog(false)` checks `FindQuestSlot(0) <
            // MAX_QUEST_LOG_SIZE`; this represented cross-session seam uses
            // the receiver snapshot derived from `WorldSession.player_quests`
            // slots via `sync_player_registry_state_like_cpp()`.
            if receiver.active_quest_statuses.len() >= MAX_QUEST_LOG_SIZE_LIKE_CPP as usize {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_LOG_FULL_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_LOG_FULL_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverLogFull,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            // C++ `Player::SatisfyQuestDay(quest, false)` immediately follows
            // `SatisfyQuestLog(false)` in `WorldSession::HandlePushQuestToParty`.
            // Non-daily/non-DF quests pass this gate; already-completed daily
            // quests and represented DF quests send the same AlreadyDone pair
            // as the earlier rewarded/onquest branch.
            let already_satisfied_quest_day_like_cpp = if quest.is_df_quest_like_cpp() {
                receiver.df_quests.contains(&packet.quest_id)
            } else if quest.is_daily_like_cpp() {
                receiver.daily_quests_completed.contains(&packet.quest_id)
            } else {
                false
            };

            if already_satisfied_quest_day_like_cpp {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            // C++ then evaluates `Player::SatisfyQuestMinLevel(quest, false)`
            // followed by `SatisfyQuestMaxLevel(quest, false)`.  Receiver
            // `level` is a derived cross-session snapshot synchronized from
            // the receiver `WorldSession`, never source-of-truth in reverse.
            if quest.min_level > 0 && i32::from(receiver.level) < quest.min_level {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_LOW_LEVEL_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_LOW_LEVEL_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMinLevelLowLevel,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if quest.max_level > 0 && receiver.level > quest.max_level {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_HIGH_LEVEL_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_HIGH_LEVEL_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMaxLevelHighLevel,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            // C++ order then evaluates `Player::SatisfyQuestClass(quest, false)`
            // followed by `SatisfyQuestRace(quest, false)`. Receiver class/race
            // are read-only `PlayerRegistry` snapshots derived from the receiver
            // `WorldSession`; never sync registry state back into the session.
            let receiver_class_mask = player_race_or_class_mask_like_cpp(receiver.class);
            if quest.allowable_classes != 0 && (quest.allowable_classes & receiver_class_mask) == 0
            {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_CLASS_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_CLASS_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestClassWrongClass,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            let receiver_race_mask = u64::from(player_race_or_class_mask_like_cpp(receiver.race));
            if quest.allowable_races != 0 && (quest.allowable_races & receiver_race_mask) == 0 {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_RACE_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_RACE_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestRaceWrongRace,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            let receiver_reputation_standing_like_cpp = |faction_id: u32| -> i32 {
                receiver
                    .reputation_standings
                    .iter()
                    .find_map(|(stored_faction_id, standing)| {
                        (*stored_faction_id == faction_id).then_some(*standing)
                    })
                    .unwrap_or(0)
            };

            let reputation_failure_reason = if quest.required_min_rep_faction != 0
                && receiver_reputation_standing_like_cpp(quest.required_min_rep_faction)
                    < quest.required_min_rep_value
            {
                Some(RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction)
            } else if quest.required_max_rep_faction != 0
                && receiver_reputation_standing_like_cpp(quest.required_max_rep_faction)
                    >= quest.required_max_rep_value
            {
                Some(RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationHighFaction)
            } else {
                None
            };

            if let Some(reason) = reputation_failure_reason {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_LOW_FACTION_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_LOW_FACTION_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            // C++ `Player::SatisfyQuestDependentQuests(quest, false)` preserves this
            // sub-gate order: PreviousQuest, DependentPreviousQuests,
            // BreadcrumbQuest, DependentBreadcrumbQuests. Expansion, CanTakeQuest,
            // success fanout, SetQuestSharingInfo, details, and auto-accept stay
            // deliberately unsupported after these represented prerequisites.
            let previous_quest_prerequisite_failed = if quest.prev_quest_id > 0 {
                !receiver
                    .rewarded_quests
                    .contains(&quest.prev_quest_id.unsigned_abs())
            } else if quest.prev_quest_id < 0 {
                receiver
                    .active_quest_statuses
                    .get(&quest.prev_quest_id.unsigned_abs())
                    .copied()
                    != Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP)
            } else {
                false
            };

            let mut prerequisite_failure_reason =
                previous_quest_prerequisite_failed.then_some(
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite,
                );

            if prerequisite_failure_reason.is_none()
                && represented_satisfy_quest_dependent_previous_quests_failed_like_cpp(
                    &quest_store,
                    quest,
                    &receiver.rewarded_quests,
                )
            {
                prerequisite_failure_reason = Some(
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite,
                );
            }

            if prerequisite_failure_reason.is_none() && quest.breadcrumb_for_quest_id != 0 {
                // C++ `SatisfyQuestBreadcrumbQuest` depends on
                // `CanTakeQuest(target,false)`. Do not fake it here; keep the
                // success path blocked until real/represented CanTakeQuest is available.
                blocked_by_unsupported_success_path = true;
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: true,
                    },
                );
                continue;
            }

            if prerequisite_failure_reason.is_none()
                && represented_satisfy_quest_dependent_breadcrumb_quests_failed_like_cpp(
                    quest,
                    &receiver.active_quest_statuses,
                )
            {
                prerequisite_failure_reason = Some(
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite,
                );
            }

            if let Some(reason) = prerequisite_failure_reason {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if i32::from(receiver.active_expansion) < quest.expansion {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_EXPANSION_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_EXPANSION_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestExpansionRequiredExpansion,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if !represented_can_take_quest_after_expansion_like_cpp(&quest_store, quest, &receiver)
            {
                let Some(sender_guid_for_receiver_packet) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    continue;
                };

                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_INVALID_LIKE_CPP,
                    String::new(),
                );
                let _ = player_registry.send_current_packet(
                    receiver.registration,
                    QuestPushResultResponse {
                        sender_guid: sender_guid_for_receiver_packet,
                        result: QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP,
                        quest_title: quest.log_title.clone(),
                    }
                    .to_bytes(),
                );
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverCanTakeQuestInvalid,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            if quest.is_turn_in_like_cpp()
                && quest.is_repeatable()
                && !quest.is_daily_or_weekly_like_cpp()
            {
                let Some(sender_guid_for_receiver_command) = sender_guid else {
                    blocked_by_unsupported_success_path = true;
                    self.record_represented_push_quest_to_party_outcome_like_cpp(
                        RepresentedPushQuestToPartyOutcomeLikeCpp {
                            sender_guid,
                            quest_id: packet.quest_id,
                            target_guid: Some(receiver_guid),
                            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPromptCommandFailed,
                            quest_pool_active_check_unrepresented: false,
                            group_runtime_unrepresented: false,
                            receiver_fanout_unrepresented: true,
                        },
                    );
                    continue;
                };

                let command = SessionCommand::SendRepeatableTurnInRequestItemsLikeCpp(
                    SendRepeatableTurnInRequestItemsLikeCppCommand {
                        sender_guid: sender_guid_for_receiver_command,
                        quest: quest.clone(),
                    },
                );

                if player_registry
                    .try_send_current_command(receiver.registration, command)
                    .is_err()
                {
                    blocked_by_unsupported_success_path = true;
                    self.record_represented_push_quest_to_party_outcome_like_cpp(
                        RepresentedPushQuestToPartyOutcomeLikeCpp {
                            sender_guid,
                            quest_id: packet.quest_id,
                            target_guid: Some(receiver_guid),
                            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPromptCommandFailed,
                            quest_pool_active_check_unrepresented: false,
                            group_runtime_unrepresented: false,
                            receiver_fanout_unrepresented: true,
                        },
                    );
                    continue;
                }

                // C++ `HandlePushQuestToParty` sends Success to the sender before the
                // repeatable turn-in `SendQuestGiverRequestItems` receiver side effect.
                // Rust has an extra fallible queue hop, so emit represented Success only
                // after the receiver command has been accepted.
                self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                    receiver_guid,
                    QUEST_PUSH_REASON_SUCCESS_LIKE_CPP,
                    String::new(),
                );

                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPrompted,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: false,
                    },
                );
                continue;
            }

            let Some(sender_guid_for_receiver_command) = sender_guid else {
                blocked_by_unsupported_success_path = true;
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverQuestDetailsPromptCommandFailed,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: true,
                    },
                );
                continue;
            };

            let command = SessionCommand::SetQuestSharingInfoAndSendDetails(
                SetQuestSharingInfoAndSendDetailsCommand {
                    sender_guid: sender_guid_for_receiver_command,
                    quest: quest.clone(),
                },
            );

            if player_registry
                .try_send_current_command(receiver.registration, command)
                .is_err()
            {
                blocked_by_unsupported_success_path = true;
                self.record_represented_push_quest_to_party_outcome_like_cpp(
                    RepresentedPushQuestToPartyOutcomeLikeCpp {
                        sender_guid,
                        quest_id: packet.quest_id,
                        target_guid: Some(receiver_guid),
                        reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverQuestDetailsPromptCommandFailed,
                        quest_pool_active_check_unrepresented: false,
                        group_runtime_unrepresented: false,
                        receiver_fanout_unrepresented: true,
                    },
                );
                continue;
            }

            self.send_push_quest_result_to_sender_with_title_if_available_like_cpp(
                receiver_guid,
                QUEST_PUSH_REASON_SUCCESS_LIKE_CPP,
                String::new(),
            );
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: Some(receiver_guid),
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: false,
                },
            );
        }

        if blocked_by_unsupported_success_path {
            self.record_represented_push_quest_to_party_outcome_like_cpp(
                RepresentedPushQuestToPartyOutcomeLikeCpp {
                    sender_guid,
                    quest_id: packet.quest_id,
                    target_guid: sender_guid,
                    reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented,
                    quest_pool_active_check_unrepresented: false,
                    group_runtime_unrepresented: false,
                    receiver_fanout_unrepresented: true,
                },
            );
        }
    }

    fn represented_can_share_quest_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        quest.flags & QUEST_FLAGS_SHARABLE_LIKE_CPP != 0
            && self.player_quests.contains_key(&quest.id)
    }

    fn send_push_quest_result_to_sender_if_available_like_cpp(
        &self,
        sender_guid: Option<ObjectGuid>,
        result: u8,
    ) {
        if let Some(sender_guid) = sender_guid {
            self.send_packet(&QuestPushResultResponse {
                sender_guid,
                result,
                quest_title: String::new(),
            });
        }
    }

    fn send_push_quest_result_to_sender_with_title_if_available_like_cpp(
        &self,
        sender_guid: ObjectGuid,
        result: u8,
        quest_title: String,
    ) {
        self.send_packet(&QuestPushResultResponse {
            sender_guid,
            result,
            quest_title,
        });
    }

    /// CMSG_QUEST_PUSH_RESULT — response to a shared quest prompt.
    ///
    /// C++ anchor: `WorldSession::HandleQuestPushResult`, `QuestHandler.cpp:758-767`.
    /// Represented-partial: session-local pending sharing state is cleared like C++;
    /// matching sender responses are recorded as evidence because full `ObjectAccessor::FindPlayer`
    /// and party sender packet fanout are not represented in this bounded slice.
    pub async fn handle_quest_push_result(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match QuestPushResult::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "QuestPushResult: failed to read SenderGUID/QuestID/Result"
                );
                return;
            }
        };

        let Some(pending) = self.represented_pending_quest_sharing_like_cpp() else {
            debug!(
                account = self.account_id,
                sender_guid = ?packet.sender_guid,
                quest_id = packet.quest_id,
                result = packet.result,
                "QuestPushResult: no represented pending shared quest"
            );
            return;
        };

        self.clear_represented_pending_quest_sharing_like_cpp();

        if pending.sender_guid != packet.sender_guid {
            self.record_represented_quest_push_result_sender_mismatch_like_cpp();
            debug!(
                account = self.account_id,
                pending_sender_guid = ?pending.sender_guid,
                packet_sender_guid = ?packet.sender_guid,
                "QuestPushResult: represented sender mismatch, pending state cleared"
            );
            return;
        }

        let Some(receiver_guid) = self.player_guid() else {
            debug!(
                account = self.account_id,
                sender_guid = ?packet.sender_guid,
                "QuestPushResult: represented sender matched but no local receiver guid is available"
            );
            return;
        };

        self.record_represented_quest_push_result_response_like_cpp(
            RepresentedQuestPushResultResponseLikeCpp {
                receiver_guid,
                sender_guid: packet.sender_guid,
                parsed_quest_id: packet.quest_id,
                pending_quest_id: pending.quest_id,
                result: packet.result,
            },
        );
    }

    /// CMSG_QUEST_LOG_REMOVE_QUEST — abandon quest-log slot.

    /// Represented-partial seam: explicit QuestLog slot lookup + local active quest removal/DB delete.
    /// Remaining gaps: source-item gates/cleanup, no-abandon-once-begun, timed/PvP state,
    /// personal summons, quest tracker DB, ScriptMgr callbacks, and criteria update evidence.
    pub async fn handle_quest_log_remove_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let slot = match pkt.read_uint8() {
            Ok(slot) => slot,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "QuestLogRemoveQuest: failed to read Entry"
                );
                return;
            }
        };

        debug!(
            account = self.account_id,
            slot, "QuestLogRemoveQuest: represented slot-backed abandon request"
        );

        if slot >= MAX_QUEST_LOG_SIZE_LIKE_CPP {
            debug!(
                account = self.account_id,
                slot, "QuestLogRemoveQuest: slot outside MAX_QUEST_LOG_SIZE"
            );
            return;
        }

        let Some(qid) = self.get_quest_slot_quest_id_like_cpp(slot) else {
            debug!(
                account = self.account_id,
                slot,
                "QuestLogRemoveQuest: valid slot empty; criteria update remains an explicit gap"
            );
            return;
        };

        self.invalidate_player_quest_status_authority_like_cpp();
        self.player_quests.remove(&qid);
        self.delete_quest_from_db(qid).await;
        self.sync_player_registry_state_like_cpp();
        self.send_represented_quest_log_slot_update_like_cpp(slot);
        info!(
            account = self.account_id,
            quest_id = qid,
            slot,
            "Quest abandoned via represented explicit quest-log slot"
        );
    }

    pub(crate) fn first_free_quest_slot_like_cpp(&self) -> Option<u8> {
        (0..MAX_QUEST_LOG_SIZE_LIKE_CPP)
            .find(|&slot| !self.quest_slot_has_active_entry_like_cpp(slot))
    }

    fn quest_slot_has_active_entry_like_cpp(&self, slot: u8) -> bool {
        // C++ `QuestSlotOffset` stores the quest id independently from the status fields;
        // represented active slots are INCOMPLETE, COMPLETE, or FAILED.
        slot < MAX_QUEST_LOG_SIZE_LIKE_CPP
            && self.player_quests.values().any(|status| {
                status.slot == slot
                    && matches!(
                        status.status,
                        QUEST_STATUS_INCOMPLETE_LIKE_CPP
                            | QUEST_STATUS_COMPLETE_LIKE_CPP
                            | QUEST_STATUS_FAILED_LIKE_CPP
                    )
            })
    }

    pub(crate) fn get_quest_slot_quest_id_like_cpp(&self, slot: u8) -> Option<u32> {
        if slot >= MAX_QUEST_LOG_SIZE_LIKE_CPP {
            return None;
        }

        let mut matching_quest_id = None;
        for status in self.player_quests.values().filter(|status| {
            status.slot == slot
                && matches!(
                    status.status,
                    QUEST_STATUS_INCOMPLETE_LIKE_CPP
                        | QUEST_STATUS_COMPLETE_LIKE_CPP
                        | QUEST_STATUS_FAILED_LIKE_CPP
                )
        }) {
            if matching_quest_id.is_some() {
                return None;
            }

            matching_quest_id = Some(status.quest_id);
        }

        matching_quest_id
    }

    pub(crate) fn find_quest_slot_like_cpp(&self, quest_id: u32) -> Option<u8> {
        self.player_quests.get(&quest_id).and_then(|status| {
            (status.slot < MAX_QUEST_LOG_SIZE_LIKE_CPP
                && matches!(
                    status.status,
                    QUEST_STATUS_INCOMPLETE_LIKE_CPP
                        | QUEST_STATUS_COMPLETE_LIKE_CPP
                        | QUEST_STATUS_FAILED_LIKE_CPP
                ))
            .then_some(status.slot)
        })
    }

    pub(crate) fn quest_log_create_entries_like_cpp(&self) -> Vec<(u32, u32, i64, [u16; 24])> {
        (0..MAX_QUEST_LOG_SIZE_LIKE_CPP)
            .map(|slot| {
                let Some(quest_id) = self.get_quest_slot_quest_id_like_cpp(slot) else {
                    return (0, 0, 0, [0; 24]);
                };
                let Some(qs) = self.player_quests.get(&quest_id) else {
                    return (0, 0, 0, [0; 24]);
                };

                let quest = self
                    .quest_store
                    .as_ref()
                    .and_then(|store| store.get(qs.quest_id));
                let mut state_flags: u32 = match qs.status {
                    QUEST_STATUS_COMPLETE_LIKE_CPP => QUEST_STATE_COMPLETE_LIKE_CPP,
                    QUEST_STATUS_FAILED_LIKE_CPP => QUEST_STATE_FAIL_LIKE_CPP,
                    _ => 0,
                };
                let mut obj_progress = [0u16; 24];
                for (i, slot_progress) in obj_progress.iter_mut().enumerate() {
                    let count = qs.objective_counts.get(i).copied().unwrap_or(0);
                    let stores_flag = quest.is_some_and(|quest| {
                        quest.objectives.iter().any(|objective| {
                            objective.storage_index == i as i8
                                && objective.is_storing_flag_like_cpp()
                        })
                    });
                    if stores_flag {
                        if count != 0 {
                            state_flags |= QUEST_STATE_OBJECTIVE_FLAG_BASE_LIKE_CPP << i;
                        }
                        continue;
                    }
                    *slot_progress = count.min(u16::MAX as i32) as u16;
                }
                (qs.quest_id, state_flags, qs.end_time_secs, obj_progress)
            })
            .collect()
    }

    pub(crate) fn send_represented_quest_log_slot_update_like_cpp(&mut self, slot: u8) {
        if slot >= MAX_QUEST_LOG_SIZE_LIKE_CPP {
            return;
        }
        let Some(guid) = self.player_guid() else {
            return;
        };

        let Some((quest_id, state_flags, end_time, objective_progress)) = self
            .quest_log_create_entries_like_cpp()
            .get(slot as usize)
            .copied()
        else {
            return;
        };

        let mut data = PlayerDataValuesDeltaUpdate::default();
        data.player_data_mask[35 / 32] |= 1 << (35 % 32);
        let slot_bit = 36 + usize::from(slot);
        data.player_data_mask[slot_bit / 32] |= 1 << (slot_bit % 32);
        data.quest_log[slot as usize] = QuestLogValuesUpdate {
            // C++ Player::SetQuestSlot marks QuestID, StateFlags, EndTime,
            // and every ObjectiveProgress field changed for the slot.
            quest_log_mask: 0x1FFF_FFFF,
            end_time,
            quest_id: quest_id.min(i32::MAX as u32) as i32,
            state_flags,
            objective_progress,
        };

        self.send_packet(&UpdateObject::full_player_values_update(
            guid,
            self.player_map_id_like_cpp(),
            data,
        ));
    }

    /// CMSG_QUERY_QUEST_INFO — client asks for full quest template data by ID.
    /// Used to populate the quest log and tooltip.
    /// Legacy non-canonical note: QuestHandler.HandleQueryQuestInfo
    pub async fn handle_query_quest_info(&mut self, mut pkt: wow_packet::WorldPacket) {
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let _guid = pkt.read_packed_guid(); // requester GUID (usually player)

        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => {
                self.send_packet(&QueryQuestInfoResponse {
                    quest_id,
                    allow: false,
                    ..Default::default()
                });
                return;
            }
        };

        match quest_store.get(quest_id) {
            None => {
                self.send_packet(&QueryQuestInfoResponse {
                    quest_id,
                    allow: false,
                    ..Default::default()
                });
            }
            Some(quest) => {
                let objectives: Vec<QuestObjectiveInfo> = quest
                    .objectives
                    .iter()
                    .map(|obj| QuestObjectiveInfo {
                        id: obj.id,
                        obj_type: obj.obj_type,
                        storage_index: obj.storage_index,
                        object_id: obj.object_id,
                        amount: obj.amount,
                        flags: obj.flags,
                        flags2: obj.flags2,
                        progress_bar_weight: obj.progress_bar_weight,
                        description: obj.description.clone(),
                    })
                    .collect();

                self.send_packet(&QueryQuestInfoResponse {
                    quest_id,
                    allow: true,
                    quest_type: quest.quest_type,
                    quest_level: quest.quest_level,
                    quest_max_scaling_level: quest.quest_max_scaling_level,
                    min_level: quest.min_level,
                    quest_sort_id: quest.quest_sort_id,
                    quest_info_id: quest.quest_info_id,
                    suggested_group_num: quest.suggested_group_num,
                    reward_next_quest: quest.reward_next_quest,
                    reward_xp_difficulty: quest.reward_xp_difficulty,
                    reward_money_difficulty: quest.reward_money_difficulty,
                    flags: quest.flags,
                    flags_ex: quest.flags_ex,
                    flags_ex2: quest.flags_ex2,
                    reward_items: quest.reward_items,
                    reward_amounts: quest.reward_amounts,
                    reward_display_spell: quest.reward_display_spell,
                    reward_spell: quest.reward_spell,
                    reward_faction_ids: quest.reward_faction_ids,
                    reward_faction_values: quest.reward_faction_values,
                    reward_faction_overrides: quest.reward_faction_overrides,
                    reward_faction_cap_in: quest.reward_faction_cap_in,
                    reward_faction_flags: quest.reward_faction_flags,
                    objectives,
                    log_title: quest.log_title.clone(),
                    log_description: quest.log_description.clone(),
                    quest_description: quest.quest_description.clone(),
                    area_description: quest.area_description.clone(),
                    quest_completion_log: quest.quest_completion_log.clone(),
                });
            }
        }
    }

    /// CMSG_QUERY_QUEST_COMPLETION_NPCS — client asks for Creature/GO quest enders.
    /// C++ refs:
    /// - `WorldSession::HandleQueryQuestCompletionNPCs`, QueryHandler.cpp:252-278.
    /// - `QuestCompletionNPCResponse::Write`, QueryPackets.cpp:451-462.
    pub async fn handle_query_quest_completion_npcs(&mut self, query: QueryQuestCompletionNpcs) {
        let quests = self
            .quest_store
            .as_deref()
            .map_or_else(Vec::new, |quest_store| {
                represented_quest_completion_npc_response_like_cpp(quest_store, &query.quest_ids)
            });

        self.send_packet(&QuestCompletionNpcResponse { quests });
    }

    /// CMSG_QUEST_POI_QUERY — client asks for tracker POI blobs.
    ///
    /// C++ refs:
    /// - `QuestPOIQuery::Read`, QueryPackets.cpp:418-423.
    /// - `WorldSession::HandleQuestPOIQuery`, QueryHandler.cpp:280-298.
    /// - `ObjectMgr::LoadQuestPOI`, ObjectMgr.cpp:8337-8415.
    pub async fn handle_quest_poi_query(&mut self, query: QuestPoiQuery) {
        if query.missing_quest_count > i32::from(MAX_QUEST_LOG_SIZE_LIKE_CPP) {
            return;
        }

        let requested_count = query.missing_quest_count.max(0) as usize;
        let requested_count = requested_count.min(query.missing_quest_pois.len());
        let mut quest_ids = std::collections::HashSet::new();
        for quest_id in query.missing_quest_pois.iter().take(requested_count) {
            quest_ids.insert(*quest_id);
        }

        let poi_store = self.quest_poi_store_like_cpp().await;
        let mut quest_poi_data_stats = Vec::new();
        for quest_id in quest_ids {
            if quest_id <= 0 {
                continue;
            }

            let quest_id_u32 = quest_id as u32;
            if self.find_quest_slot_like_cpp(quest_id_u32).is_none() {
                continue;
            }

            if let Some(poi_data) = poi_store.get(&quest_id) {
                quest_poi_data_stats.push(poi_data.clone());
            }
        }

        self.send_packet_realm(&QuestPoiQueryResponse {
            quest_poi_data_stats,
        });
    }

    /// CMSG_QUEST_GIVER_REQUEST_REWARD — player talks to NPC to turn in a completed quest.
    /// Legacy non-canonical note: QuestHandler.HandleQuestgiverRequestReward
    /// Sent when player right-clicks a quest-ender NPC and has the quest in Complete status.
    /// Server responds with SMSG_QUEST_GIVER_OFFER_REWARD_MESSAGE (reward selection dialog).
    pub async fn handle_quest_giver_request_reward(&mut self, mut pkt: wow_packet::WorldPacket) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("QuestGiverRequestReward: failed to read GUID");
                return;
            }
        };
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);

        info!(
            account = self.account_id,
            ?guid,
            quest_id,
            "Received QuestGiverRequestReward like C++"
        );

        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => return,
        };
        let quest = match quest_store.get(quest_id) {
            Some(q) => q.clone(),
            None => {
                warn!(
                    account = self.account_id,
                    quest_id, "RequestReward: unknown quest"
                );
                return;
            }
        };

        if self.is_quest_disabled_like_cpp(quest_id) {
            debug!(
                account = self.account_id,
                quest_id, "RequestReward: quest disabled"
            );
            return;
        }

        if quest.flags & QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP == 0
            && !self.represented_quest_giver_involved_source_allows_quest_like_cpp(
                guid,
                quest_id,
                &quest_store,
            )
        {
            warn!(
                account = self.account_id,
                ?guid,
                quest_id,
                "RequestReward: represented involved source rejected"
            );
            return;
        }

        // C++: if (_player->CanCompleteQuest(questID)) _player->CompleteQuest(questID)
        let can_complete_now = self.player_quests.get(&quest_id).is_some_and(|status| {
            Self::represented_can_complete_quest_after_objective_like_cpp(
                status,
                &quest,
                0,
                self.rewarded_quests.contains(&quest_id),
            )
        });
        if can_complete_now {
            let completion_evidence_start = self
                .represented_quest_complete_status_updates_like_cpp
                .len();
            self.complete_represented_quest_after_add_if_ready_like_cpp(&quest)
                .await;
            self.save_represented_quest_statuses_completed_after_like_cpp(
                completion_evidence_start,
            )
            .await;
        }

        let is_complete = self
            .player_quests
            .get(&quest_id)
            .is_some_and(|qs| qs.status == QUEST_STATUS_COMPLETE_LIKE_CPP);

        if !is_complete {
            // Objectives not finished — silently ignore
            // (C# would send SMSG_QUEST_GIVER_REQUEST_ITEMS instead)
            warn!(
                account = self.account_id,
                quest_id, "RequestReward: quest not complete"
            );
            return;
        }

        // Build rewards block for the offer-reward dialog
        let mut rewards = QuestRewardsBlock::default();
        rewards.money = quest.reward_money_difficulty as i32;
        for i in 0..4 {
            rewards.items[i] = (quest.reward_items[i], quest.reward_amounts[i]);
        }
        for i in 0..3 {
            rewards.display_spells[i] = quest.reward_display_spell[i];
        }
        rewards.completion_spell = quest.reward_spell as i32;
        // Populate choice items for the dialog
        for i in 0..6 {
            rewards.choice_items[i] = (
                quest.reward_choice_items[i].0,
                quest.reward_choice_items[i].1,
            );
        }
        rewards.choice_item_types = quest.reward_choice_item_types;

        // C#: SendQuestGiverOfferReward(quest, questGiverGUID, true)
        self.send_packet(&QuestGiverOfferReward {
            giver_guid: guid,
            giver_creature_id: quest_giver_creature_id_from_source_like_cpp(guid),
            quest_id,
            quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
            suggested_party_members: quest.suggested_group_num,
            rewards,
            title: quest.log_title.clone(),
            reward_text: quest.quest_completion_log.clone(),
            auto_launched: false,
        });
    }

    /// CMSG_QUEST_GIVER_COMPLETE_QUEST — player talks to quest-ender NPC.
    /// If objectives are done: show reward dialog. Else: show "still need X" dialog.
    /// Legacy non-canonical note: QuestHandler.HandleQuestGiverCompleteQuest
    pub async fn handle_quest_giver_complete_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("QuestGiverCompleteQuest: failed to read GUID");
                return;
            }
        };
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let from_script: bool = pkt.read_bit().unwrap_or(false);

        info!(
            account = self.account_id,
            ?guid,
            quest_id,
            from_script,
            "Received QuestGiverCompleteQuest like C++"
        );

        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => return,
        };

        let quest = match quest_store.get(quest_id) {
            Some(q) => q,
            None => {
                warn!(
                    account = self.account_id,
                    quest_id, "QuestGiverCompleteQuest: unknown quest"
                );
                return;
            }
        };

        if self.is_quest_disabled_like_cpp(quest_id) {
            debug!(
                account = self.account_id,
                quest_id, "QuestGiverCompleteQuest: quest disabled"
            );
            return;
        }

        if quest.flags & QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP == 0 {
            if from_script
                || !self.represented_quest_giver_involved_source_allows_quest_like_cpp(
                    guid,
                    quest_id,
                    &quest_store,
                )
            {
                warn!(
                    account = self.account_id,
                    ?guid,
                    quest_id,
                    from_script,
                    "QuestGiverCompleteQuest: represented involved source rejected"
                );
                return;
            }
        } else if !from_script || self.player_guid() != Some(guid) {
            warn!(
                account = self.account_id,
                ?guid,
                quest_id,
                from_script,
                "QuestGiverCompleteQuest: auto-complete source is not script/player"
            );
            return;
        }

        // Check if player has the quest active
        if !self.has_quest(quest_id) {
            debug!(
                account = self.account_id,
                quest_id, "Player doesn't have quest"
            );
            return;
        }

        // Build rewards block
        let mut rewards = QuestRewardsBlock::default();
        rewards.money = quest.reward_money_difficulty as i32;
        for i in 0..4 {
            rewards.items[i] = (quest.reward_items[i], quest.reward_amounts[i]);
        }
        for i in 0..3 {
            rewards.display_spells[i] = quest.reward_display_spell[i];
        }
        rewards.completion_spell = quest.reward_spell as i32;
        for i in 0..6 {
            rewards.choice_items[i] = (
                quest.reward_choice_items[i].0,
                quest.reward_choice_items[i].1,
            );
        }
        rewards.choice_item_types = quest.reward_choice_item_types;

        // Check if all objectives are done — C++ GetQuestStatus == QUEST_STATUS_COMPLETE.
        let is_complete = self
            .player_quests
            .get(&quest_id)
            .is_some_and(|qs| qs.status == QUEST_STATUS_COMPLETE_LIKE_CPP);

        if !is_complete {
            // Not all objectives done — send "you still need X" dialog
            // Legacy non-canonical note: SendQuestGiverRequestItems(quest, guid, canComplete=false, false)
            self.send_packet(&QuestGiverRequestItems {
                giver_guid: guid,
                giver_creature_id: quest_giver_creature_id_from_source_like_cpp(guid),
                quest_id,
                comp_emote_delay: 0,
                comp_emote_type: 0,
                quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
                suggested_party_members: quest.suggested_group_num,
                money_to_get: 0,
                collect: Vec::new(),
                currency: Vec::new(),
                status_flags: 0xFD,
                title: quest.log_title.clone(),
                completion_text: quest.area_description.clone(),
                auto_launched: false,
            });
            return;
        }

        // All objectives done — show offer reward dialog
        self.send_packet(&QuestGiverOfferReward {
            giver_guid: guid,
            giver_creature_id: quest_giver_creature_id_from_source_like_cpp(guid),
            quest_id,
            quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
            suggested_party_members: quest.suggested_group_num,
            rewards,
            title: quest.log_title.clone(),
            reward_text: quest.quest_completion_log.clone(),
            auto_launched: false,
        });
    }

    fn read_quest_choice_item_like_cpp(
        pkt: &mut wow_packet::WorldPacket,
    ) -> Result<QuestChoiceItemLikeCpp, wow_packet::PacketError> {
        // C++ `QuestChoiceItem` starts with `ResetBitPos(); ReadBits(2)`, then
        // an `Item::ItemInstance`, then signed `Quantity`.
        pkt.reset_bits();
        let loot_item_type = pkt.read_bits(2)? as u8;

        let item_id = pkt.read_int32()? as u32;
        let _random_properties_seed = pkt.read_int32()?;
        let _random_properties_id = pkt.read_int32()?;

        let has_item_bonus = pkt.read_bit()?;
        pkt.reset_bits();

        let item_mod_count = pkt.read_bits(6)?;
        pkt.reset_bits();
        for _ in 0..item_mod_count {
            let _value = pkt.read_int32()?;
            let _modifier_type = pkt.read_uint8()?;
        }

        if has_item_bonus {
            let _context = pkt.read_uint8()?;
            let bonus_count = pkt.read_uint32()?;
            for _ in 0..bonus_count {
                let _bonus_id = pkt.read_uint32()?;
            }
        }

        let quantity = pkt.read_int32()?;

        Ok(QuestChoiceItemLikeCpp {
            loot_item_type,
            item_id,
            quantity,
        })
    }

    fn represented_reward_choice_matches_loaded_type_like_cpp(
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        quest
            .reward_choice_items
            .iter()
            .zip(quest.reward_choice_item_types.iter())
            .any(|((item_id, _quantity), item_type)| {
                *item_id != 0 && *item_id == choice.item_id && *item_type == choice.loot_item_type
            })
    }

    fn represented_reward_choice_template_exists_like_cpp(
        &self,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        match choice.loot_item_type {
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP => self
                .item_store()
                .is_some_and(|store| store.get(choice.item_id).is_some()),
            QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP => self
                .currency_types_store()
                .is_some_and(|store| store.has_record(choice.item_id)),
            _ => false,
        }
    }

    fn represented_can_select_quest_package_item_like_cpp(
        &self,
        quest_package_item: &QuestPackageItemEntry,
    ) -> bool {
        let Ok(item_id) = u32::try_from(quest_package_item.item_id) else {
            return false;
        };
        if self
            .item_store()
            .is_none_or(|store| store.get(item_id).is_none())
        {
            return false;
        }

        let Some(sparse) = self
            .item_stats_store()
            .and_then(|store| store.sparse_template(item_id))
        else {
            return false;
        };

        let player_team = crate::session::player_team_for_race_cpp(self.player_race_like_cpp());
        if ((sparse.flags[1] & ItemFlags2::FactionAlliance as u32) != 0
            && player_team != wow_constants::unit::Team::Alliance)
            || ((sparse.flags[1] & ItemFlags2::FactionHorde as u32) != 0
                && player_team != wow_constants::unit::Team::Horde)
        {
            return false;
        }

        match quest_package_item.display_type {
            QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP => true,
            QUEST_PACKAGE_FILTER_CLASS_LIKE_CPP => false,
            QUEST_PACKAGE_FILTER_LOOT_SPECIALIZATION_LIKE_CPP => false,
            _ => false,
        }
    }

    fn represented_quest_package_choice_matches_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        if choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
            || quest.quest_package_id == 0
        {
            return false;
        }

        let Some(store) = &self.quest_package_item_store else {
            return false;
        };
        let Ok(choice_item_id) = i32::try_from(choice.item_id) else {
            return false;
        };

        let primary_valid = store
            .quest_package_items_like_cpp(quest.quest_package_id)
            .filter(|entry| entry.item_id == choice_item_id)
            .any(|entry| self.represented_can_select_quest_package_item_like_cpp(entry));
        if primary_valid {
            return true;
        }

        store
            .quest_package_items_fallback_like_cpp(quest.quest_package_id)
            .any(|entry| entry.item_id == choice_item_id)
    }

    fn send_quest_failed_like_cpp(&self, quest_id: u32, reason: InventoryResult) {
        if quest_id == 0 {
            return;
        }

        self.send_packet(&QuestGiverQuestFailed {
            quest_id,
            reason: reason as u32,
        });
    }

    fn represented_quest_reward_inventory_plan_result_like_cpp(
        &self,
        item_id: u32,
        count: u32,
    ) -> InventoryResult {
        self.plan_store_new_direct_inventory_item(item_id, count)
            .map(|(result, _, _)| result)
            .unwrap_or(InventoryResult::ItemNotFound)
    }

    fn send_quest_package_reward_inventory_error_like_cpp(
        &self,
        result: InventoryResult,
        item_id: u32,
    ) {
        let limit_category = self
            .item_storage_template(item_id)
            .map(|template| u32::from(template.item_limit_category))
            .unwrap_or(0);
        self.send_equip_error(result, None, None, 0, limit_category);
    }

    fn represented_can_reward_quest_inventory_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        // C++ `Player::CanRewardQuest(quest, rewardType, rewardId, true)`.
        if choice.loot_item_type == QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP {
            for ((item_id, count), item_type) in quest
                .reward_choice_items
                .iter()
                .zip(quest.reward_choice_item_types.iter())
            {
                if *item_id == 0
                    || *item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
                    || *item_id != choice.item_id
                {
                    continue;
                }

                let result =
                    self.represented_quest_reward_inventory_plan_result_like_cpp(*item_id, *count);
                if result != InventoryResult::Ok {
                    self.send_quest_failed_like_cpp(quest.id, result);
                    return false;
                }
            }
        }

        for (item_id, count) in quest.reward_items.iter().zip(quest.reward_amounts.iter()) {
            if *item_id == 0 {
                continue;
            }

            let result =
                self.represented_quest_reward_inventory_plan_result_like_cpp(*item_id, *count);
            if result != InventoryResult::Ok {
                self.send_quest_failed_like_cpp(quest.id, result);
                return false;
            }
        }

        if quest.quest_package_id == 0
            || choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
        {
            return true;
        }

        let Some(store) = &self.quest_package_item_store else {
            return true;
        };
        let Ok(choice_item_id) = i32::try_from(choice.item_id) else {
            return true;
        };

        let mut has_filtered_quest_package_reward = false;
        for entry in store.quest_package_items_like_cpp(quest.quest_package_id) {
            if entry.item_id != choice_item_id
                || !self.represented_can_select_quest_package_item_like_cpp(entry)
            {
                continue;
            }

            has_filtered_quest_package_reward = true;
            let Ok(item_id) = u32::try_from(entry.item_id) else {
                self.send_quest_package_reward_inventory_error_like_cpp(
                    InventoryResult::ItemNotFound,
                    0,
                );
                return false;
            };
            let result = self.represented_quest_reward_inventory_plan_result_like_cpp(
                item_id,
                entry.item_quantity,
            );
            if result != InventoryResult::Ok {
                self.send_quest_package_reward_inventory_error_like_cpp(result, item_id);
                return false;
            }
        }

        if !has_filtered_quest_package_reward {
            for entry in store.quest_package_items_fallback_like_cpp(quest.quest_package_id) {
                if entry.item_id != choice_item_id {
                    continue;
                }

                let Ok(item_id) = u32::try_from(entry.item_id) else {
                    self.send_quest_package_reward_inventory_error_like_cpp(
                        InventoryResult::ItemNotFound,
                        0,
                    );
                    return false;
                };
                let result = self.represented_quest_reward_inventory_plan_result_like_cpp(
                    item_id,
                    entry.item_quantity,
                );
                if result != InventoryResult::Ok {
                    self.send_quest_package_reward_inventory_error_like_cpp(result, item_id);
                    return false;
                }
            }
        }

        true
    }

    async fn reward_represented_quest_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: ObjectGuid,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        let quest_id = quest.id;
        let choice_item_id = choice.item_id;
        self.set_represented_can_delay_teleport_like_cpp(true);

        macro_rules! reward_abort {
            () => {{
                self.set_represented_can_delay_teleport_like_cpp(false);
                return false;
            }};
        }

        if !self
            .remove_quest_required_items_and_currencies_like_cpp(quest)
            .await
        {
            debug!(
                account = self.account_id,
                quest_id,
                "RewardQuest: represented quest objective/item-drop removal failed before reward mutation"
            );
            reward_abort!();
        }

        self.remove_represented_timed_quest_like_cpp(quest_id);

        if !self.store_fixed_quest_reward_items_like_cpp(quest).await {
            debug!(
                account = self.account_id,
                quest_id,
                "RewardQuest: represented fixed reward item grant failed before reward mutation"
            );
            reward_abort!();
        }

        if !self
            .store_chosen_quest_reward_item_like_cpp(quest, choice)
            .await
        {
            debug!(
                account = self.account_id,
                quest_id,
                choice_item_id,
                "RewardQuest: represented chosen reward item grant failed before reward mutation"
            );
            reward_abort!();
        }

        if !self
            .store_quest_package_reward_items_like_cpp(quest, choice)
            .await
        {
            debug!(
                account = self.account_id,
                quest_id,
                choice_item_id,
                "RewardQuest: represented quest package item grant failed before reward mutation"
            );
            reward_abort!();
        }

        if !self
            .grant_quest_reward_currencies_like_cpp(quest, choice)
            .await
        {
            debug!(
                account = self.account_id,
                quest_id,
                choice_item_id,
                "RewardQuest: represented quest reward currency grant failed before reward mutation"
            );
            reward_abort!();
        }

        self.apply_represented_quest_reward_skill_like_cpp(quest);

        let money = quest.reward_money_difficulty;
        if money > 0 {
            match self
                .mutate_and_persist_player_gold_exclusive_like_cpp(|old_money| {
                    crate::session::loot_money_durable_outcome_like_cpp(old_money, money as u64).0
                })
                .await
            {
                Some((old_money, new_money)) => {
                    if old_money != new_money {
                        self.enqueue_represented_quest_objective_progress_like_cpp(
                            RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                                old_money,
                                new_money,
                            },
                        );
                    }
                }
                None => {
                    // Boundary: the represented reward path persists item and
                    // currency grants before reaching money and does not yet
                    // own C++ `Player::RewardQuest` as one durable transaction.
                    // Aborting here would leave the quest retryable after those
                    // grants and permit duplicates. Preserve the existing
                    // completion behavior; an ambiguous money COMMIT has
                    // already quarantined/kicked the session in the shared
                    // helper. Atomic quest reward persistence is separate debt.
                    warn!(
                        account = self.account_id,
                        quest_id,
                        money,
                        "Quest reward money was not durably established; preserving non-atomic represented reward completion to avoid duplicate retry"
                    );
                }
            }
        }

        self.apply_represented_quest_title_and_talent_rewards_like_cpp(quest);
        self.record_represented_quest_reward_mail_like_cpp(quest, quest_giver_guid);
        self.apply_quest_reward_lockout_status_like_cpp(quest).await;

        let xp = self.quest_xp_reward_like_cpp(quest);
        let rewarded_slot = self.find_quest_slot_like_cpp(quest_id);

        self.invalidate_player_quest_status_authority_like_cpp();
        self.player_quests.remove(&quest_id);
        if !quest.is_repeatable() {
            self.rewarded_quests.insert(quest_id);
            self.save_quest_to_db(quest_id, QUEST_STATUS_REWARDED_LIKE_CPP)
                .await;
        } else {
            self.delete_quest_from_db(quest_id).await;
        }
        self.sync_player_registry_state_like_cpp();
        if let Some(slot) = rewarded_slot {
            self.send_represented_quest_log_slot_update_like_cpp(slot);
        }

        info!(
            account = self.account_id,
            quest_id,
            xp,
            gold = money,
            repeatable = quest.is_repeatable(),
            "Quest rewarded"
        );

        let game_event_outcome = self
            .notify_game_event_quest_complete_like_cpp(quest_id)
            .await;
        debug!(
            account = self.account_id,
            quest_id,
            outcome = ?game_event_outcome,
            "Represented C++ GameEventMgr::HandleQuestComplete notification after quest reward"
        );

        self.send_packet(&QuestGiverQuestComplete {
            quest_id,
            xp,
            money,
            skill_line_id: quest.reward_skill_line_id,
            skill_points: quest.reward_skill_points,
            use_quest_reward_currency: false,
        });

        self.send_packet(&QuestUpdateComplete { quest_id });

        self.record_represented_quest_reward_reputation_like_cpp(quest);
        self.record_represented_quest_reward_spell_casts_like_cpp(quest);

        if xp > 0 {
            // C++ `Player::RewardQuest` calls `GiveXP(XP, nullptr)`: quest XP
            // does not consume rested XP. RAF remains mutually exclusive with
            // rested XP and may still apply inside `GiveXP`.
            self.give_xp(xp, ObjectGuid::EMPTY, 1.0).await;
        }

        self.set_represented_can_delay_teleport_like_cpp(false);

        true
    }

    /// CMSG_QUEST_GIVER_CHOOSE_REWARD — player clicks "Complete Quest" in reward dialog.
    /// Gives XP, gold, items. Removes quest from active log.
    /// Legacy non-canonical note: QuestHandler.HandleQuestGiverChooseReward
    pub async fn handle_quest_giver_choose_reward(&mut self, mut pkt: wow_packet::WorldPacket) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("ChooseReward: failed to read GUID");
                return;
            }
        };
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let choice = match Self::read_quest_choice_item_like_cpp(&mut pkt) {
            Ok(choice) => choice,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "ChooseReward: failed to read C++ QuestChoiceItem"
                );
                return;
            }
        };
        let choice_item_id = choice.item_id;

        info!(
            account = self.account_id,
            ?guid,
            quest_id,
            choice_item_id,
            choice_loot_type = choice.loot_item_type,
            "Received QuestGiverChooseReward like C++"
        );

        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => return,
        };
        let quest = match quest_store.get(quest_id) {
            Some(q) => q.clone(),
            None => {
                warn!(
                    account = self.account_id,
                    quest_id, "ChooseReward: unknown quest"
                );
                return;
            }
        };

        if self.is_quest_disabled_like_cpp(quest_id) {
            debug!(
                account = self.account_id,
                quest_id, "ChooseReward: quest disabled"
            );
            return;
        }

        // C++ `Player::CanRewardQuest`: player must have the quest active and COMPLETE.
        let quest_status = self.player_quests.get(&quest_id).map(|qs| qs.status);
        match quest_status {
            Some(QUEST_STATUS_COMPLETE_LIKE_CPP) => {}
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP) => {
                warn!(
                    account = self.account_id,
                    quest_id, "ChooseReward: quest not complete yet"
                );
                return;
            }
            _ => {
                warn!(
                    account = self.account_id,
                    quest_id, "ChooseReward: player doesn't have quest"
                );
                return;
            }
        }

        // Validate choice item — C# HandleQuestgiverChooseReward lines 255-310
        // If client sends a non-zero choice item, it must be in reward_choice_items.
        if choice_item_id != 0 {
            if choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
                && choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP
            {
                warn!(
                    account = self.account_id,
                    quest_id,
                    loot_item_type = choice.loot_item_type,
                    "ChooseReward: unsupported C++ LootItemType"
                );
                return;
            }
            if !self.represented_reward_choice_template_exists_like_cpp(choice) {
                warn!(
                    account = self.account_id,
                    quest_id,
                    choice_item_id,
                    loot_item_type = choice.loot_item_type,
                    "ChooseReward: selected reward item/currency template does not exist"
                );
                return;
            }
            let valid =
                Self::represented_reward_choice_matches_loaded_type_like_cpp(&quest, choice)
                    || self.represented_quest_package_choice_matches_like_cpp(&quest, choice);
            if !valid {
                warn!(
                    account = self.account_id,
                    quest_id,
                    choice_item_id,
                    loot_item_type = choice.loot_item_type,
                    "ChooseReward: choice item not valid for this quest (possible exploit)"
                );
                return;
            }
        }

        // C++ HandleQuestgiverChooseRewardOpcode keeps `object = _player` for auto-complete,
        // but non-auto-complete quests must resolve the packet source as an involved
        // Unit/GameObject and pass CanInteractWithQuestGiver before RewardQuest mutates state.
        // This represented-partial slice intentionally keeps bounded choice/package validation
        // only; full CanRewardQuest/RewardQuest side effects remain open.
        if quest.flags & QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP == 0
            && !self.represented_quest_giver_involved_source_allows_quest_like_cpp(
                guid,
                quest_id,
                &quest_store,
            )
        {
            warn!(
                account = self.account_id,
                ?guid,
                quest_id,
                "ChooseReward: represented involved source rejected"
            );
            return;
        }

        if !self.represented_can_reward_quest_inventory_like_cpp(&quest, choice) {
            debug!(
                account = self.account_id,
                quest_id,
                choice_item_id,
                "ChooseReward: represented reward inventory validation rejected like C++"
            );
            return;
        }

        let rewarded = self
            .reward_represented_quest_like_cpp(&quest, guid, choice)
            .await;
        if rewarded {
            Box::pin(self.drain_represented_quest_objective_progress_like_cpp()).await;
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Resolves CMSG_QUEST_GIVER_STATUS_QUERY through the represented equivalent of
    /// C++ `ObjectAccessor::GetObjectByTypeMask(*_player, guid, TYPEMASK_UNIT | TYPEMASK_GAMEOBJECT)`.
    /// Missing canonical objects and unsupported Player/Item/other GUID types fail closed with no packet.
    pub(crate) fn represented_quest_giver_status_query_source_like_cpp(
        &self,
        guid: wow_core::ObjectGuid,
    ) -> Option<RepresentedQuestGiverStatusSourceLikeCpp> {
        if guid.is_any_type_creature() {
            // C++ TYPEID_UNIT branch also checks Creature::IsHostileTo before computing
            // dialog status. Exact faction/hostility is not represented here yet; a
            // resolved canonical Creature is treated as non-hostile only for this
            // bounded represented status calculation.
            let access = self.canonical_creature_access_like_cpp(guid)?;
            return Some(RepresentedQuestGiverStatusSourceLikeCpp::Creature {
                entry: access.entry,
            });
        }

        if guid.is_game_object() {
            let access = self.canonical_gameobject_access_like_cpp(guid)?;
            return Some(RepresentedQuestGiverStatusSourceLikeCpp::GameObject {
                entry: access.entry,
            });
        }

        None
    }

    /// Bounded representation of C++ `Player::GetQuestDialogStatus(Object const*)`.
    /// Creature sources use Creature starter/ender relations; GameObject sources use
    /// GO starter/ender relations. Full AI dialog status, ConditionMgr, event overlays
    /// and important/covenant/journey DB2 classification remain documented migration gaps.
    pub(crate) fn get_represented_quest_giver_status_like_cpp(
        &self,
        source: RepresentedQuestGiverStatusSourceLikeCpp,
    ) -> u64 {
        let Some(store) = &self.quest_store else {
            return quest_giver_status::NONE;
        };

        let turn_in_quests = match source {
            RepresentedQuestGiverStatusSourceLikeCpp::Creature { entry } => {
                store.quests_for_ender(entry)
            }
            RepresentedQuestGiverStatusSourceLikeCpp::GameObject { entry } => {
                store.quests_for_gameobject_ender(entry)
            }
        };

        let mut result = quest_giver_status::NONE;

        for quest in turn_in_quests {
            match self.quest_status_like_cpp(quest.id) {
                QUEST_STATUS_COMPLETE_LIKE_CPP => {
                    result |= self.represented_quest_reward_complete_status_like_cpp(quest);
                }
                QUEST_STATUS_INCOMPLETE_LIKE_CPP => {
                    result |= self.represented_quest_reward_status_like_cpp(quest);
                }
                _ => {}
            }

            if quest.quest_type == 0
                && self.can_take_quest(quest)
                && quest.is_repeatable()
                && !quest.is_daily_or_weekly_like_cpp()
                && !quest.is_monthly_like_cpp()
            {
                if self.represented_quest_is_trivial_like_cpp(quest) {
                    result |= quest_giver_status::TRIVIAL_REPEATABLE_TURNIN;
                } else {
                    result |= quest_giver_status::REPEATABLE_TURNIN;
                }
            }
        }

        let start_quests = match source {
            RepresentedQuestGiverStatusSourceLikeCpp::Creature { entry } => {
                store.quests_for_starter(entry)
            }
            RepresentedQuestGiverStatusSourceLikeCpp::GameObject { entry } => {
                store.quests_for_gameobject_starter(entry)
            }
        };

        for quest in start_quests {
            if !self.represented_quest_available_conditions_meet_like_cpp(quest.id) {
                continue;
            }

            if self.quest_status_like_cpp(quest.id) != QUEST_STATUS_NONE_LIKE_CPP {
                continue;
            }

            if !self.can_see_start_quest_represented_bounded_like_cpp(quest) {
                continue;
            }

            if self.satisfy_quest_level_represented_like_cpp(quest) {
                result |= self.represented_quest_available_status_like_cpp(
                    quest,
                    self.represented_quest_is_trivial_like_cpp(quest),
                );
            } else {
                result |= self.represented_quest_future_status_like_cpp(quest);
            }
        }

        result
    }

    fn represented_quest_available_conditions_meet_like_cpp(&self, quest_id: u32) -> bool {
        let condition_store = if let Some(store) = self.condition_store() {
            Arc::clone(store)
        } else if let Some(store) = crate::conditions::condition_mgr_store_like_cpp() {
            store
        } else {
            return true;
        };

        if !crate::conditions::has_conditions_for_not_grouped_entry_like_cpp(
            condition_store.as_ref(),
            wow_constants::ConditionSourceType::QuestAvailable,
            quest_id,
        ) {
            return true;
        }

        let Some(player_object) = self.build_condition_player_object_like_cpp() else {
            return false;
        };

        let quest_statuses: Vec<_> = self
            .player_quests
            .iter()
            .map(
                |(&quest_id, status)| crate::conditions::ConditionQuestStatusSnapshot {
                    quest_id,
                    status: status.status,
                },
            )
            .collect();
        let quest_objective_progress: Vec<_> = self
            .quest_store
            .as_ref()
            .map(|store| {
                self.player_quests
                    .iter()
                    .filter_map(|(&quest_id, status)| {
                        store.get(quest_id).map(|quest| {
                            quest.objectives.iter().filter_map(move |objective| {
                                let storage_index =
                                    usize::try_from(objective.storage_index).ok()?;
                                let counter = status
                                    .objective_counts
                                    .get(storage_index)
                                    .copied()
                                    .unwrap_or(0);
                                Some(crate::conditions::ConditionQuestObjectiveProgressSnapshot {
                                    quest_id,
                                    objective_id: objective.id,
                                    counter,
                                })
                            })
                        })
                    })
                    .flatten()
                    .collect()
            })
            .unwrap_or_default();
        let rewarded_quest_ids: Vec<_> = self.rewarded_quests.iter().copied().collect();
        let daily_quest_ids: Vec<_> = self
            .daily_quests_completed_like_cpp
            .iter()
            .copied()
            .collect();
        let quest_snapshot = crate::conditions::ConditionPlayerQuestSnapshot {
            statuses: &quest_statuses,
            objective_progress: &quest_objective_progress,
            rewarded_quest_ids: &rewarded_quest_ids,
            daily_quest_ids: &daily_quest_ids,
        };
        let player_condition_context = self.represented_player_condition_context_like_cpp();
        let area_table_store = self.area_table_store().cloned();

        let mut source_info =
            crate::conditions::ConditionSourceInfo::from_targets(Some(&player_object), None, None);
        source_info.set_unit_target_snapshot(0, self.condition_player_unit_snapshot_like_cpp());
        source_info.set_player_target_snapshot(0, self.condition_player_snapshot_like_cpp());
        source_info.set_player_quest_target_snapshot(0, quest_snapshot);
        if let Some(store) = self.player_condition_store() {
            source_info.set_player_condition_store(store.as_ref());
            source_info.set_player_condition_context(0, player_condition_context.as_context(self));
        }

        crate::conditions::is_object_meeting_not_grouped_conditions_like_cpp(
            condition_store.as_ref(),
            wow_constants::ConditionSourceType::QuestAvailable,
            quest_id,
            &mut source_info,
            |condition, source_info| {
                crate::conditions::condition_meets_basic_like_cpp(
                    condition,
                    source_info,
                    |area_id, required_area_id| {
                        area_table_store.as_ref().is_some_and(|store| {
                            store.is_in_area_like_cpp(area_id, required_area_id)
                        })
                    },
                )
                .value()
                .unwrap_or(false)
            },
        )
    }

    fn quest_status_like_cpp(&self, quest_id: u32) -> u8 {
        if self.rewarded_quests.contains(&quest_id) {
            return QUEST_STATUS_REWARDED_LIKE_CPP;
        }

        self.player_quests
            .get(&quest_id)
            .map(|quest| quest.status)
            .unwrap_or(QUEST_STATUS_NONE_LIKE_CPP)
    }

    // SatisfyQuestSkill — Player.cpp:14098, 15015-15037
    fn satisfy_quest_skill_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        if quest.required_skill_id == 0 {
            return true;
        }
        let Ok(skill_u16) = u16::try_from(quest.required_skill_id) else {
            return true;
        };
        u32::from(self.player_skill_value_like_cpp(skill_u16)) >= quest.required_skill_points
    }

    // SatisfyQuestReputation — Player.cpp:14098, 15262-15289
    //
    // Mirrors C++ GetReputation(fId) = base + standing.
    // faction_store None or faction not found → treat reputation as 0 (C++ GetReputation returns 0
    // for unknown faction id, Player.cpp:15265 / ReputationMgr.cpp:118-124).
    fn satisfy_quest_reputation_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        if quest.required_min_rep_faction != 0 {
            let rep = self
                .faction_store()
                .and_then(|store| store.get(quest.required_min_rep_faction))
                .map(|faction_entry| {
                    self.reputation_mgr_like_cpp()
                        .reputation_for_faction_like_cpp(
                            faction_entry,
                            self.player_race_like_cpp(),
                            self.player_class_like_cpp(),
                        )
                })
                .unwrap_or(0);
            if rep < quest.required_min_rep_value {
                return false;
            }
        }

        if quest.required_max_rep_faction != 0 {
            let rep = self
                .faction_store()
                .and_then(|store| store.get(quest.required_max_rep_faction))
                .map(|faction_entry| {
                    self.reputation_mgr_like_cpp()
                        .reputation_for_faction_like_cpp(
                            faction_entry,
                            self.player_race_like_cpp(),
                            self.player_class_like_cpp(),
                        )
                })
                .unwrap_or(0);
            if rep >= quest.required_max_rep_value {
                return false;
            }
        }

        true
    }

    // SatisfyQuestExclusiveGroup — Player.cpp:15348-15391
    //
    // Only positive exclusive_group values restrict: a positive group means "take
    // at most one quest from this set".  Non-positive (0 or negative) groups are
    // unused/unrestricted → always true (Player.cpp:15351).
    //
    // quest_store None → fail-open: without the store we cannot enumerate peers,
    // so we conservatively allow the quest rather than silently blocking it.  The
    // same fail-open pattern is used throughout can_take_quest for missing stores.
    fn satisfy_quest_exclusive_group_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        // Player.cpp:15351 — non-positive exclusive_group never restricts
        if quest.exclusive_group <= 0 {
            return true;
        }

        let Some(quest_store) = &self.quest_store else {
            return true;
        };

        for peer in quest_store
            .quests
            .values()
            .filter(|c| c.exclusive_group == quest.exclusive_group)
        {
            // Player.cpp:15360 — skip the quest being evaluated
            if peer.id == quest.id {
                continue;
            }

            // Player.cpp:15366 — SatisfyQuestDay: daily/DF cooldown blocks the group
            // Mirrors the daily/DF pattern from the push path (quest.rs:271-278).
            if peer.is_df_quest_like_cpp() && self.df_quests_like_cpp.contains(&peer.id) {
                return false;
            }
            if peer.is_daily_like_cpp() && self.daily_quests_completed_like_cpp.contains(&peer.id) {
                return false;
            }

            // Player.cpp:15366 — SatisfyQuestWeek: weekly cooldown blocks the group
            if peer.is_weekly_like_cpp() && self.weekly_quests_completed_like_cpp.contains(&peer.id)
            {
                return false;
            }

            // Player.cpp:15366 — SatisfyQuestSeasonal: seasonal cooldown blocks the group
            // Mirrors the seasonal pattern from can_take_quest (quest.rs:5948-5963).
            if peer.is_seasonal_like_cpp() && !self.seasonal_quests_like_cpp.is_empty() {
                if let Some(bucket) = self
                    .seasonal_quests_like_cpp
                    .get(&peer.event_id_for_quest_like_cpp())
                {
                    if !bucket.is_empty() && bucket.contains_key(&peer.id) {
                        return false;
                    }
                }
            }

            // Player.cpp:15379 — alternative quest already active or rewarded (non-repeatable pair).
            //
            // C++: GetQuestStatus(peer) != QUEST_STATUS_NONE
            //   → in C++ GetQuestStatus returns REWARDED when rewarded, so this single
            //     term would also catch rewarded quests.  We model the two cases separately
            //     to keep the Rust representation explicit:
            //   Term 1: peer is currently active in player_quests (Incomplete/Complete/Failed).
            //   Term 2: peer was rewarded AND not both quests are repeatable (matching the
            //           C++ second OR operand: GetQuestRewardStatus + !IsRepeatable pair).
            if self.player_quests.contains_key(&peer.id) {
                return false;
            }
            if !(quest.is_repeatable() && peer.is_repeatable())
                && self.rewarded_quests.contains(&peer.id)
            {
                return false;
            }
        }

        true
    }

    fn represented_quest_info_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> Option<&QuestInfoEntry> {
        self.quest_info_store
            .as_ref()
            .and_then(|store| store.get(quest.quest_info_id as u32))
    }

    pub(crate) fn represented_quest_is_important_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        const QUEST_INFO_MODIFIER_IMPORTANT_LIKE_CPP: i32 = 0x400;
        self.represented_quest_info_like_cpp(quest)
            .is_some_and(|info| (info.modifiers & QUEST_INFO_MODIFIER_IMPORTANT_LIKE_CPP) != 0)
    }

    fn represented_quest_is_covenant_calling_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        const QUEST_TAG_TYPE_COVENANT_CALLING_LIKE_CPP: i8 = 15;
        self.represented_quest_info_like_cpp(quest)
            .is_some_and(|info| info.quest_type == QUEST_TAG_TYPE_COVENANT_CALLING_LIKE_CPP)
    }

    fn represented_quest_reward_complete_status_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> u64 {
        if self.represented_quest_is_important_like_cpp(quest) {
            if (quest.flags & wow_data::quest::QUEST_FLAGS_HIDE_REWARD_POI_LIKE_CPP) != 0 {
                quest_giver_status::IMPORTANT_QUEST_REWARD_COMPLETE_NO_POI
            } else {
                quest_giver_status::IMPORTANT_QUEST_REWARD_COMPLETE_POI
            }
        } else if self.represented_quest_is_covenant_calling_like_cpp(quest) {
            if (quest.flags & wow_data::quest::QUEST_FLAGS_HIDE_REWARD_POI_LIKE_CPP) != 0 {
                quest_giver_status::COVENANT_CALLING_REWARD_COMPLETE_NO_POI
            } else {
                quest_giver_status::COVENANT_CALLING_REWARD_COMPLETE_POI
            }
        } else if (quest.flags_ex & wow_data::quest::QUEST_FLAGS_EX_LEGENDARY_LIKE_CPP) != 0 {
            if (quest.flags & wow_data::quest::QUEST_FLAGS_HIDE_REWARD_POI_LIKE_CPP) != 0 {
                quest_giver_status::LEGENDARY_REWARD_COMPLETE_NO_POI
            } else {
                quest_giver_status::LEGENDARY_REWARD_COMPLETE_POI
            }
        } else if (quest.flags & wow_data::quest::QUEST_FLAGS_HIDE_REWARD_POI_LIKE_CPP) != 0 {
            quest_giver_status::REWARD_COMPLETE_NO_POI
        } else {
            quest_giver_status::REWARD_COMPLETE_POI
        }
    }

    fn represented_quest_reward_status_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> u64 {
        if self.represented_quest_is_important_like_cpp(quest) {
            quest_giver_status::IMPORTANT_REWARD
        } else if self.represented_quest_is_covenant_calling_like_cpp(quest) {
            quest_giver_status::COVENANT_CALLING_REWARD
        } else if (quest.flags_ex & wow_data::quest::QUEST_FLAGS_EX_LEGENDARY_LIKE_CPP) != 0 {
            quest_giver_status::LEGENDARY_REWARD
        } else {
            quest_giver_status::REWARD
        }
    }

    fn represented_quest_available_status_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
        trivial: bool,
    ) -> u64 {
        if self.represented_quest_is_important_like_cpp(quest) {
            if trivial {
                quest_giver_status::TRIVIAL_IMPORTANT_QUEST
            } else {
                quest_giver_status::IMPORTANT_QUEST
            }
        } else if self.represented_quest_is_covenant_calling_like_cpp(quest) {
            quest_giver_status::COVENANT_CALLING_QUEST
        } else if (quest.flags_ex & wow_data::quest::QUEST_FLAGS_EX_LEGENDARY_LIKE_CPP) != 0 {
            if trivial {
                quest_giver_status::TRIVIAL_LEGENDARY_QUEST
            } else {
                quest_giver_status::LEGENDARY_QUEST
            }
        } else if quest.is_daily_like_cpp() {
            if trivial {
                quest_giver_status::TRIVIAL_DAILY_QUEST
            } else {
                quest_giver_status::DAILY_QUEST
            }
        } else if trivial {
            quest_giver_status::TRIVIAL
        } else {
            quest_giver_status::QUEST
        }
    }

    fn represented_quest_future_status_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> u64 {
        if self.represented_quest_is_important_like_cpp(quest) {
            quest_giver_status::FUTURE_IMPORTANT_QUEST
        } else if (quest.flags_ex & wow_data::quest::QUEST_FLAGS_EX_LEGENDARY_LIKE_CPP) != 0 {
            quest_giver_status::FUTURE_LEGENDARY_QUEST
        } else {
            quest_giver_status::FUTURE
        }
    }

    fn represented_quest_is_trivial_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        self.player_level_like_cpp() as i32
            > quest
                .quest_level
                .saturating_add(self.quest_low_level_hide_diff_like_cpp as i32)
    }

    fn satisfy_quest_level_represented_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        let level = self.player_level_like_cpp();
        if quest.min_level > 0 && i32::from(level) < quest.min_level {
            return false;
        }

        if quest.max_level > 0 && level > quest.max_level {
            return false;
        }

        true
    }

    fn satisfy_quest_race_class_represented_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        quest.is_available_for(
            self.player_race_like_cpp(),
            self.player_class_like_cpp(),
            self.player_level_like_cpp()
                .max(quest.min_level.max(1).min(i32::from(u8::MAX)) as u8),
        )
    }

    fn can_see_start_quest_represented_bounded_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        if self.is_quest_disabled_like_cpp(quest.id) {
            return false;
        }

        if self.quest_status_like_cpp(quest.id) != QUEST_STATUS_NONE_LIKE_CPP {
            return false;
        }

        if quest.is_seasonal_like_cpp() && !self.seasonal_quests_like_cpp.is_empty() {
            if let Some(bucket) = self
                .seasonal_quests_like_cpp
                .get(&quest.event_id_for_quest_like_cpp())
            {
                if !bucket.is_empty() && bucket.contains_key(&quest.id) {
                    return false;
                }
            }
        }

        if quest.prev_quest_id != 0 {
            let prev_id = quest.prev_quest_id.unsigned_abs();
            if quest.prev_quest_id > 0 {
                if !self.rewarded_quests.contains(&prev_id) {
                    return false;
                }
            } else if !self
                .player_quests
                .get(&prev_id)
                .is_some_and(|qs| qs.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP)
            {
                return false;
            }
        }

        self.satisfy_quest_race_class_represented_like_cpp(quest)
            && i32::from(self.player_level_like_cpp())
                .saturating_add(self.quest_high_level_hide_diff_like_cpp as i32)
                >= quest.min_level
    }

    /// Check if the player currently has an active quest with the given ID.
    pub fn has_quest(&self, quest_id: u32) -> bool {
        self.player_quests.contains_key(&quest_id)
    }

    /// Full eligibility check before accepting a quest.
    /// C++ ref: Player::CanTakeQuest (Player.cpp:14093-14102) — gate order mirrors C++ exactly.
    pub fn can_take_quest(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        if self.is_quest_disabled_like_cpp(quest.id) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: quest disabled"
            );
            return false;
        }

        // SatisfyQuestStatus — C# lines 1624-1654
        // If quest is already rewarded (non-repeatable), cannot take again.
        if self.rewarded_quests.contains(&quest.id) && !quest.is_repeatable() {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: already rewarded"
            );
            return false;
        }
        // If quest is already active, cannot accept again.
        if self.player_quests.contains_key(&quest.id) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: already active"
            );
            return false;
        }

        // SatisfyQuestExclusiveGroup — Player.cpp:14096, Player.cpp:15348-15391
        // Inserted here to match C++ CanTakeQuest evaluation order: status → exclusive group.
        if !self.satisfy_quest_exclusive_group_like_cpp(quest) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: exclusive group blocked"
            );
            return false;
        }

        // SatisfyQuestRace + SatisfyQuestClass + SatisfyQuestLevel
        if !quest.is_available_for(
            self.player_race_like_cpp(),
            self.player_class_like_cpp(),
            self.player_level_like_cpp(),
        ) {
            return false;
        }

        // SatisfyQuestSkill — Player.cpp:14098, 15015-15037
        if !self.satisfy_quest_skill_like_cpp(quest) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: skill requirement not met"
            );
            return false;
        }

        // SatisfyQuestReputation — Player.cpp:14098, 15262-15289
        if !self.satisfy_quest_reputation_like_cpp(quest) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: reputation requirement not met"
            );
            return false;
        }

        // SatisfyQuestPreviousQuest — C# lines 1415-1440
        // prev_quest_id > 0 → previous quest must have been rewarded
        // prev_quest_id < 0 → previous quest must be currently active (Incomplete)
        if quest.prev_quest_id != 0 {
            let prev_id = quest.prev_quest_id.unsigned_abs();
            if quest.prev_quest_id > 0 {
                if !self.rewarded_quests.contains(&prev_id) {
                    debug!(
                        account = self.account_id,
                        quest_id = quest.id,
                        prev_id,
                        "CanTakeQuest: prev quest not rewarded"
                    );
                    return false;
                }
            } else {
                // negative: prev quest must be active
                let active = self
                    .player_quests
                    .get(&prev_id)
                    .is_some_and(|qs| qs.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP);
                if !active {
                    debug!(
                        account = self.account_id,
                        quest_id = quest.id,
                        prev_id,
                        "CanTakeQuest: negative prev quest not active"
                    );
                    return false;
                }
            }
        }

        // SatisfyQuestDependentPreviousQuests — Player.cpp:15090 / Player.cpp:15121-15177
        // Blocks acceptance if the scalar dependent-previous list is not satisfied.
        // Per C++ SatisfyQuestDependentQuests (Player.cpp:15088-15092), this cluster runs
        // after SatisfyQuestReputation, not before Race/Class/Level.
        if let Some(quest_store) = &self.quest_store {
            if represented_satisfy_quest_dependent_previous_quests_failed_like_cpp(
                quest_store,
                quest,
                &self.rewarded_quests,
            ) {
                debug!(
                    account = self.account_id,
                    quest_id = quest.id,
                    "CanTakeQuest: dependent previous quests not satisfied"
                );
                return false;
            }
        }

        // SatisfyQuestDependentBreadcrumbQuests — Player.cpp:15203-15222
        // Blocks acceptance if any breadcrumb quest listed in `dependent_breadcrumb_quests` is
        // currently INCOMPLETE/COMPLETE/FAILED in the player's log.
        // Note: BreadcrumbQuest (recursive single breadcrumb, Player.cpp:15179-15202) remains
        // unimplemented here without falsing.
        {
            let statuses: std::collections::HashMap<u32, u8> = self
                .player_quests
                .iter()
                .map(|(&qid, qs)| (qid, qs.status))
                .collect();
            if represented_satisfy_quest_dependent_breadcrumb_quests_failed_like_cpp(
                quest, &statuses,
            ) {
                debug!(
                    account = self.account_id,
                    quest_id = quest.id,
                    "CanTakeQuest: dependent breadcrumb in log"
                );
                return false;
            }
        }

        // SatisfyQuestDay — Player.cpp:15393-15407 (CanTakeQuest term Player.cpp:14093-14102).
        // DF (dungeon-finder) quests are gated by the DFQuests set; regular dailies by
        // DailyQuestsCompleted. Mirrors the completion-push split at quest.rs:2973-2979
        // and the exclusive-group peer pattern at quest.rs:5873-5879.
        if quest.is_df_quest_like_cpp() {
            if self.df_quests_like_cpp.contains(&quest.id) {
                debug!(
                    account = self.account_id,
                    quest_id = quest.id,
                    "CanTakeQuest: DF quest already completed"
                );
                return false;
            }
        } else if quest.is_daily_like_cpp()
            && self.daily_quests_completed_like_cpp.contains(&quest.id)
        {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: daily quest already completed"
            );
            return false;
        }

        // SatisfyQuestWeek — Player.cpp:15409-15418 (CanTakeQuest term Player.cpp:14093-14102).
        if quest.is_weekly_like_cpp() && self.weekly_quests_completed_like_cpp.contains(&quest.id) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: weekly quest on cooldown"
            );
            return false;
        }

        // SatisfyQuestMonth — Player.cpp:15445-15454 (CanTakeQuest term Player.cpp:14093-14102).
        if quest.is_monthly_like_cpp() && self.monthly_quests_completed_like_cpp.contains(&quest.id)
        {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: monthly quest on cooldown"
            );
            return false;
        }

        // SatisfyQuestSeasonal — C++ Player::SatisfyQuestSeasonal
        // Per C++ CanTakeQuest order (Player.cpp:14093-14102): Day/Week/Month (above) and
        // Seasonal precede Conditions; the dependent cluster (prev_quest_id,
        // DependentPreviousQuests, DependentBreadcrumbQuests) runs before this, as part of
        // SatisfyQuestDependentQuests. SatisfyQuestTimed remains a separate gap:
        // the session has no active-timed-quest set yet (see #QUESTS.15).
        if quest.is_seasonal_like_cpp() && !self.seasonal_quests_like_cpp.is_empty() {
            if let Some(bucket) = self
                .seasonal_quests_like_cpp
                .get(&quest.event_id_for_quest_like_cpp())
            {
                if !bucket.is_empty() && bucket.contains_key(&quest.id) {
                    debug!(
                        account = self.account_id,
                        quest_id = quest.id,
                        event_id = quest.event_id_for_quest_like_cpp(),
                        "CanTakeQuest: seasonal quest cooldown"
                    );
                    return false;
                }
            }
        }

        // SatisfyQuestConditions — C++ Player.cpp:14102
        if !self.represented_quest_available_conditions_meet_like_cpp(quest.id) {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: quest available conditions not met"
            );
            return false;
        }

        // SatisfyQuestExpansion — Player.cpp:15431-15443 (CanTakeQuest term Player.cpp:14102)
        if i32::from(self.expansion) < quest.expansion {
            debug!(
                account = self.account_id,
                quest_id = quest.id,
                "CanTakeQuest: required expansion"
            );
            return false;
        }

        true
    }

    pub(crate) fn is_quest_disabled_like_cpp(&self, quest_id: u32) -> bool {
        self.disable_mgr().is_some_and(|disable_mgr| {
            disable_mgr.is_disabled_for_like_cpp(DISABLE_TYPE_QUEST, quest_id, None, 0, None)
        })
    }

    /// Save quest status and represented objective counters to the characters database.
    ///
    /// C++ anchor: `Player::_SaveQuestStatus`, `Player.cpp:20160-20191`.
    /// The represented path keeps Rust's existing direct save timing, but mirrors the
    /// C++ objective persistence order for a saved quest: status row first, then delete
    /// stale objective rows for the quest, then replace nonzero objective counters.
    /// For Rust's combined rewarded migration path, preserve the rewarded row before
    /// deleting the stale active row.
    fn represented_quest_status_save_statements_like_cpp(
        &self,
        guid: u64,
        quest_id: u32,
        status: u8,
        status_snapshot: Option<&PlayerQuestStatus>,
        mut prepare: impl FnMut(CharStatements) -> PreparedStatement,
    ) -> Vec<PreparedStatement> {
        let mut statements = Vec::new();

        if status == QUEST_STATUS_REWARDED_LIKE_CPP {
            let mut rewarded = prepare(CharStatements::INS_CHAR_QUESTSTATUS_REWARDED);
            rewarded.set_u64(0, guid);
            rewarded.set_u32(1, quest_id);
            statements.push(rewarded);

            let mut del_status = prepare(CharStatements::DEL_CHAR_QUEST_STATUS);
            del_status.set_u64(0, guid);
            del_status.set_u32(1, quest_id);
            statements.push(del_status);

            let mut del_objectives =
                prepare(CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST);
            del_objectives.set_u64(0, guid);
            del_objectives.set_u32(1, quest_id);
            statements.push(del_objectives);

            return statements;
        }

        let saved_status = status_snapshot.or_else(|| self.player_quests.get(&quest_id));
        let represented_explored = saved_status.map(|status| status.explored).unwrap_or(false);
        let represented_accept_time = saved_status
            .map(|status| status.accept_time_secs)
            .unwrap_or(0);
        let represented_end_time = saved_status.map(|status| status.end_time_secs).unwrap_or(0);
        let mut stmt = prepare(CharStatements::INS_CHAR_QUEST_STATUS);
        stmt.set_u64(0, guid);
        stmt.set_u32(1, quest_id);
        stmt.set_u8(2, status);
        stmt.set_u8(3, u8::from(represented_explored));
        stmt.set_i64(4, represented_accept_time);
        stmt.set_i64(5, represented_end_time);
        statements.push(stmt);

        let mut del_objectives = prepare(CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST);
        del_objectives.set_u64(0, guid);
        del_objectives.set_u32(1, quest_id);
        statements.push(del_objectives);

        if let (Some(quest_store), Some(saved_status)) = (self.quest_store.as_ref(), saved_status)
            && let Some(quest) = quest_store.get(quest_id)
        {
            for objective in &quest.objectives {
                if objective.storage_index < 0 {
                    continue;
                }
                let storage_index = objective.storage_index as usize;
                let count = saved_status
                    .objective_counts
                    .get(storage_index)
                    .copied()
                    .unwrap_or(0);
                if count == 0 {
                    continue;
                }

                let Ok(objective_index) = u8::try_from(objective.storage_index) else {
                    continue;
                };
                let mut rep_objective = prepare(CharStatements::REP_CHAR_QUEST_STATUS_OBJECTIVES);
                rep_objective.set_u64(0, guid);
                rep_objective.set_u32(1, quest_id);
                rep_objective.set_u8(2, objective_index);
                rep_objective.set_i32(3, count);
                statements.push(rep_objective);
            }
        }

        statements
    }

    async fn save_quest_to_db(&self, quest_id: u32, status: u8) {
        let guid = match self.player_guid() {
            Some(g) => g.counter() as u64,
            None => return,
        };
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let mut tx = SqlTransaction::new();
        for stmt in self.represented_quest_status_save_statements_like_cpp(
            guid,
            quest_id,
            status,
            None,
            |statement| char_db.prepare(statement),
        ) {
            tx.append(stmt);
        }

        if let Err(e) = char_db.commit_transaction(tx).await {
            warn!(
                account = self.account_id,
                quest_id, "Failed to save quest status: {e}"
            );
        }
    }

    /// Delete a quest from the characters database (abandon).
    async fn delete_quest_from_db(&self, quest_id: u32) {
        use wow_database::CharStatements;

        let guid = match self.player_guid() {
            Some(g) => g.counter() as u64,
            None => return,
        };
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let mut tx = SqlTransaction::new();
        let mut stmt = char_db.prepare(CharStatements::DEL_CHAR_QUEST_STATUS);
        stmt.set_u64(0, guid);
        stmt.set_u32(1, quest_id);
        tx.append(stmt);

        let mut del_objectives =
            char_db.prepare(CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST);
        del_objectives.set_u64(0, guid);
        del_objectives.set_u32(1, quest_id);
        tx.append(del_objectives);

        if let Err(e) = char_db.commit_transaction(tx).await {
            warn!(
                account = self.account_id,
                quest_id, "Failed to delete quest: {e}"
            );
        }
    }

    /// Load all active quests for this player from the characters DB.
    pub(crate) async fn load_player_quests(&mut self) {
        use wow_database::CharStatements;

        self.begin_player_quest_status_authority_load_like_cpp();

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let mut stmt = char_db.prepare(CharStatements::SEL_CHAR_QUEST_STATUS);
        Self::bind_player_quest_status_load_guid_like_cpp(&mut stmt, player_guid);

        let result = match char_db.query(&stmt).await {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to load quest status: {e}"
                );
                return;
            }
        };

        self.player_quests.clear();
        self.rewarded_quests.clear();

        let mut quest_status_rows_coherent_like_cpp = true;
        let mut next_active_slot: u8 = 0;
        let mut stale_rewarded_active_rows = Vec::new();

        if !result.is_empty() {
            let mut result = result;
            loop {
                let row = (
                    result.try_read::<u32>(0),
                    result.try_read::<u8>(1),
                    result.try_read::<u8>(2),
                    result.try_read::<i64>(3),
                    result.try_read::<i64>(4),
                );
                let (
                    Some(quest_id),
                    Some(status),
                    Some(explored),
                    Some(accept_time_secs),
                    Some(end_time_secs),
                ) = row
                else {
                    quest_status_rows_coherent_like_cpp = false;
                    if !result.next_row() {
                        break;
                    }
                    continue;
                };
                let status = if status < 7 {
                    status
                } else {
                    QUEST_STATUS_INCOMPLETE_LIKE_CPP
                };
                let explored = explored != 0;

                if status == QUEST_STATUS_REWARDED_LIKE_CPP {
                    // Rewarded (C++ QuestStatus::QUEST_STATUS_REWARDED / m_RewardedQuests).
                    // Non-repeatable quests cannot be re-taken once rewarded.
                    self.rewarded_quests.insert(quest_id);
                    stale_rewarded_active_rows.push(quest_id);
                } else if next_active_slot < MAX_QUEST_LOG_SIZE_LIKE_CPP {
                    // Active or complete-but-not-turned-in.
                    // C++ _LoadQuestStatus assigns sequential visible slots in DB row order
                    // because the character DB status row has no persisted quest-log slot.
                    let slot = next_active_slot;
                    next_active_slot = next_active_slot.saturating_add(1);
                    let obj_count = self
                        .quest_store
                        .as_ref()
                        .and_then(|s| s.get(quest_id))
                        .map_or(0, |q| q.objectives.len());
                    if self.player_quests.contains_key(&quest_id) {
                        quest_status_rows_coherent_like_cpp = false;
                    }
                    self.player_quests.insert(
                        quest_id,
                        PlayerQuestStatus {
                            quest_id,
                            status,
                            explored,
                            accept_time_secs,
                            end_time_secs,
                            objective_counts: vec![0; obj_count],
                            slot,
                        },
                    );
                }

                if !result.next_row() {
                    break;
                }
            }
        }

        let mut objective_stmt = char_db.prepare(CharStatements::SEL_CHAR_QUEST_STATUS_OBJECTIVES);
        Self::bind_player_quest_status_load_guid_like_cpp(&mut objective_stmt, player_guid);

        match char_db.query(&objective_stmt).await {
            Ok(objective_rows) if !objective_rows.is_empty() => {
                let mut objective_rows = objective_rows;
                loop {
                    let quest_id: u32 = objective_rows.try_read::<u32>(0).unwrap_or(0);
                    let storage_index: u8 = objective_rows.try_read::<u8>(1).unwrap_or(0);
                    let data: i32 = objective_rows.try_read::<i32>(2).unwrap_or(0);

                    if let (Some(status), Some(quest)) = (
                        self.player_quests.get_mut(&quest_id),
                        self.quest_store
                            .as_ref()
                            .and_then(|store| store.get(quest_id)),
                    ) {
                        if let Some(objective) = quest.objectives.iter().find(|objective| {
                            u8::try_from(objective.storage_index).ok() == Some(storage_index)
                        }) {
                            let index = usize::from(storage_index);
                            if status.objective_counts.len() <= index {
                                status.objective_counts.resize(index + 1, 0);
                            }
                            status.objective_counts[index] = if objective.is_storing_flag_like_cpp()
                            {
                                i32::from(data != 0)
                            } else {
                                data
                            };
                        }
                    }

                    if !objective_rows.next_row() {
                        break;
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to load quest objective status: {e}"
                );
            }
        }

        let mut rewarded_rows_coherent_like_cpp = false;
        let mut rewarded_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_QUESTSTATUSREW);
        Self::bind_player_quest_status_load_guid_like_cpp(&mut rewarded_stmt, player_guid);
        match char_db.query(&rewarded_stmt).await {
            Ok(rewarded_rows) if !rewarded_rows.is_empty() => {
                rewarded_rows_coherent_like_cpp = true;
                let mut rewarded_rows = rewarded_rows;
                loop {
                    let Some(quest_id) = rewarded_rows.try_read::<u32>(0) else {
                        rewarded_rows_coherent_like_cpp = false;
                        if !rewarded_rows.next_row() {
                            break;
                        }
                        continue;
                    };
                    self.record_represented_rewarded_quest_row_like_cpp(quest_id);
                    if self
                        .represented_quest_can_increase_rewarded_counters_like_cpp(quest_id)
                        .is_some_and(|can_increase| can_increase)
                    {
                        self.rewarded_quests.insert(quest_id);
                    }

                    if !rewarded_rows.next_row() {
                        break;
                    }
                }
            }
            Ok(_) => rewarded_rows_coherent_like_cpp = true,
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to load rewarded quest status: {e}"
                );
            }
        }

        stale_rewarded_active_rows
            .extend(self.remove_represented_active_rewarded_duplicates_like_cpp());
        stale_rewarded_active_rows.sort_unstable();
        stale_rewarded_active_rows.dedup();
        for quest_id in stale_rewarded_active_rows {
            info!(
                account = self.account_id,
                quest_id,
                "QuestLoad: migrating stale active rewarded quest status before deleting active row like C++"
            );
            self.save_quest_to_db(quest_id, QUEST_STATUS_REWARDED_LIKE_CPP)
                .await;
        }

        if quest_status_rows_coherent_like_cpp && rewarded_rows_coherent_like_cpp {
            self.complete_player_quest_status_authority_load_like_cpp();
        }

        self.df_quests_like_cpp.clear();
        self.daily_quests_completed_like_cpp.clear();
        self.last_daily_quest_time_like_cpp = 0;
        let mut daily_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_QUESTSTATUS_DAILY);
        Self::bind_player_quest_status_load_guid_like_cpp(&mut daily_stmt, player_guid);
        match char_db.query(&daily_stmt).await {
            Ok(daily_rows) if !daily_rows.is_empty() => {
                let mut daily_rows = daily_rows;
                loop {
                    let quest_id = daily_rows.try_read::<u32>(0).unwrap_or(0);
                    let completed_time = daily_rows.try_read::<i64>(1).unwrap_or(0);
                    if let Some(quest) = self
                        .quest_store
                        .as_ref()
                        .and_then(|store| store.get(quest_id))
                    {
                        self.last_daily_quest_time_like_cpp = completed_time;
                        if quest.is_df_quest_like_cpp() {
                            self.df_quests_like_cpp.insert(quest_id);
                        } else {
                            self.daily_quests_completed_like_cpp.insert(quest_id);
                        }
                    }

                    if !daily_rows.next_row() {
                        break;
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to load daily quest status: {e}"
                );
            }
        }

        self.weekly_quests_completed_like_cpp.clear();
        let mut weekly_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_QUESTSTATUS_WEEKLY);
        Self::bind_player_quest_status_load_guid_like_cpp(&mut weekly_stmt, player_guid);
        match char_db.query(&weekly_stmt).await {
            Ok(weekly_rows) if !weekly_rows.is_empty() => {
                let mut weekly_rows = weekly_rows;
                loop {
                    let quest_id = weekly_rows.try_read::<u32>(0).unwrap_or(0);
                    if self
                        .quest_store
                        .as_ref()
                        .and_then(|store| store.get(quest_id))
                        .is_some()
                    {
                        self.weekly_quests_completed_like_cpp.insert(quest_id);
                    }

                    if !weekly_rows.next_row() {
                        break;
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to load weekly quest status: {e}"
                );
            }
        }

        self.monthly_quests_completed_like_cpp.clear();
        let mut monthly_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_QUESTSTATUS_MONTHLY);
        Self::bind_player_quest_status_load_guid_like_cpp(&mut monthly_stmt, player_guid);
        match char_db.query(&monthly_stmt).await {
            Ok(monthly_rows) if !monthly_rows.is_empty() => {
                let mut monthly_rows = monthly_rows;
                loop {
                    let quest_id = monthly_rows.try_read::<u32>(0).unwrap_or(0);
                    if self
                        .quest_store
                        .as_ref()
                        .and_then(|store| store.get(quest_id))
                        .is_some()
                    {
                        self.monthly_quests_completed_like_cpp.insert(quest_id);
                    }

                    if !monthly_rows.next_row() {
                        break;
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to load monthly quest status: {e}"
                );
            }
        }

        let mut seasonal_stmt = char_db.prepare(CharStatements::SEL_CHAR_QUEST_STATUS_SEASONAL);
        Self::bind_player_quest_status_load_guid_like_cpp(&mut seasonal_stmt, player_guid);

        let seasonal_rows = match char_db.query(&seasonal_stmt).await {
            Ok(result) => {
                let mut rows = Vec::new();
                if !result.is_empty() {
                    let mut result = result;
                    loop {
                        let quest_id = result.try_read::<u32>(0).unwrap_or_else(|| {
                            warn!(
                                account = self.account_id,
                                "Failed to read seasonal quest id"
                            );
                            0
                        });
                        let event_id = result.try_read::<u32>(1).unwrap_or_else(|| {
                            warn!(
                                account = self.account_id,
                                quest_id, "Failed to read seasonal quest event id"
                            );
                            u32::MAX
                        });
                        let completed_time = result.try_read::<i64>(2).unwrap_or_else(|| {
                            warn!(
                                account = self.account_id,
                                quest_id, event_id, "Failed to read seasonal quest completedTime"
                            );
                            -1
                        });
                        rows.push(SeasonalQuestStatusDbRowLikeCpp {
                            quest_id,
                            event_id,
                            completed_time,
                        });

                        if !result.next_row() {
                            break;
                        }
                    }
                }
                rows
            }
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to load seasonal quest status: {e}"
                );
                Vec::new()
            }
        };

        let quest_store = self.quest_store.as_ref().map(Arc::clone);
        let quest_v2_store = self.quest_v2_store.as_ref().map(Arc::clone);
        let seasonal_outcome = self.load_seasonal_quest_status_like_cpp(
            seasonal_rows,
            quest_store.as_deref(),
            quest_v2_store.as_deref(),
        );

        if seasonal_outcome.skipped_no_quest_store > 0
            || seasonal_outcome.skipped_missing_quest > 0
            || seasonal_outcome.skipped_event_out_of_range > 0
            || seasonal_outcome.skipped_negative_completed_time > 0
            || seasonal_outcome.completed_bit_skipped_no_quest_v2_store > 0
            || seasonal_outcome.completed_bit_skipped_zero_unique_bit > 0
            || seasonal_outcome.completed_bit_no_change_or_noop > 0
        {
            warn!(
                account = self.account_id,
                rows_seen = seasonal_outcome.rows_seen,
                skipped_no_quest_store = seasonal_outcome.skipped_no_quest_store,
                skipped_missing_quest = seasonal_outcome.skipped_missing_quest,
                skipped_event_out_of_range = seasonal_outcome.skipped_event_out_of_range,
                skipped_negative_completed_time = seasonal_outcome.skipped_negative_completed_time,
                completed_bit_skipped_no_quest_v2_store =
                    seasonal_outcome.completed_bit_skipped_no_quest_v2_store,
                completed_bit_skipped_zero_unique_bit =
                    seasonal_outcome.completed_bit_skipped_zero_unique_bit,
                completed_bit_no_change_or_noop = seasonal_outcome.completed_bit_no_change_or_noop,
                "Skipped seasonal quest status rows during login load"
            );
        }

        info!(
            account = self.account_id,
            active = self.player_quests.len(),
            rewarded = self.rewarded_quests.len(),
            df = self.df_quests_like_cpp.len(),
            daily = self.daily_quests_completed_like_cpp.len(),
            weekly = self.weekly_quests_completed_like_cpp.len(),
            monthly = self.monthly_quests_completed_like_cpp.len(),
            seasonal_inserted = seasonal_outcome.inserted,
            seasonal_replaced = seasonal_outcome.replaced,
            seasonal_completed_bit_set = seasonal_outcome.completed_bit_set,
            seasonal_completed_bit_skipped_no_quest_v2_store =
                seasonal_outcome.completed_bit_skipped_no_quest_v2_store,
            seasonal_completed_bit_skipped_zero_unique_bit =
                seasonal_outcome.completed_bit_skipped_zero_unique_bit,
            seasonal_completed_bit_no_change_or_noop =
                seasonal_outcome.completed_bit_no_change_or_noop,
            "Loaded player quests"
        );
    }
}

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

#[cfg(test)]
#[path = "quest_tests.rs"]
mod tests;

// ── PlayerQuestStatus ────────────────────────────────────────────────────────

/// Tracks one active quest for a player.
#[derive(Debug, Clone)]
pub struct PlayerQuestStatus {
    pub quest_id: u32,
    /// C++ QuestStatus values: 0=None, 1=Complete, 3=Incomplete, 5=Failed, 6=Rewarded.
    pub status: u8,
    pub explored: bool,
    /// TrinityCore QuestStatusData::AcceptTime, persisted as Unix seconds.
    pub accept_time_secs: i64,
    /// Represented ActivePlayerData::QuestLog[slot].EndTime persisted by _SaveQuestStatus.
    pub end_time_secs: i64,
    /// Progress per objective (indexed by objective.storage_index).
    /// value = current count toward the required amount.
    pub objective_counts: Vec<i32>,
    /// Represented TrinityCore QuestStatusData::Slot / ActivePlayerData::QuestLog index.
    pub slot: u8,
}

#[derive(Debug, Clone)]
pub(crate) struct ItemTransferQuestPersistencePlanLikeCpp {
    statuses: HashMap<u32, PlayerQuestStatus>,
    changed_quest_ids: Vec<u32>,
}
