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
//!   CMSG_QUEST_GIVER_ACCEPT_QUEST  → save to DB + SMSG_QUEST_GIVER_QUEST_COMPLETE
//!   CMSG_QUEST_LOG_REMOVE_QUEST    → remove from DB
//!   CMSG_QUERY_QUEST_INFO          → SMSG_QUERY_QUEST_INFO_RESPONSE
//!
//! C# ref: Game/Handlers/QuestHandler.cs

use std::sync::Arc;
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
use wow_database::{CharStatements, PreparedStatement, SqlTransaction, WorldStatements};
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
    QueryQuestCompletionNpcs, QuestCompletionNpc, QuestCompletionNpcResponse,
};
use wow_packet::packets::quest::{
    PushQuestToParty, QueryQuestInfoResponse, QuestConfirmAccept, QuestGiverOfferReward,
    QuestGiverQuestComplete, QuestGiverQuestFailed, QuestGiverRequestItems, QuestGiverStatus,
    QuestObjectiveInfo, QuestPushResult, QuestPushResultResponse, QuestRewardsBlock,
    QuestUpdateComplete, WorldQuestUpdateResponse, quest_giver_status, quest_push_reason,
};
use wow_packet::packets::update::{ItemCreateData, UpdateObject};
use wow_packet::{ClientPacket, ServerPacket};

use crate::conditions::{
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    QUEST_STATUS_NONE_LIKE_CPP, QUEST_STATUS_REWARDED_LIKE_CPP,
};
use crate::handlers::character::ExtendedCostItemTurninChange;
use crate::session::{
    CurrencyGainSourceLikeCpp, InventoryItem, RepresentedPushQuestToPartyOutcomeLikeCpp,
    RepresentedPushQuestToPartyOutcomeReasonLikeCpp, RepresentedQuestCompleteStatusUpdateLikeCpp,
    RepresentedQuestConfirmAcceptLikeCpp, RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    RepresentedQuestObjectiveProgressEventLikeCpp, RepresentedQuestPushResultResponseLikeCpp,
    RepresentedQuestRewardMailLikeCpp, RepresentedQuestRewardReputationLikeCpp,
    RepresentedQuestRewardReputationSourceLikeCpp, RepresentedQuestRewardSpellCastLikeCpp,
    RepresentedQuestRewardSpellKindLikeCpp, RepresentedQuestRewardTalentPointsLikeCpp,
    RepresentedQuestRewardTitleLikeCpp, ReputationGainSourceLikeCpp,
    SeasonalQuestStatusDbRowLikeCpp, WorldSession,
};

pub(crate) const QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP: u32 = 0x0001_0000;
pub(crate) const QUEST_FLAGS_PLAYER_CAST_COMPLETE_LIKE_CPP: u32 = 0x0020_0000;
pub(crate) const QUEST_FLAGS_SHARABLE_LIKE_CPP: u32 = 0x0000_0008;
const QUEST_FLAGS_COMPLETION_EVENT_LIKE_CPP: u32 = 0x0000_0002;
const QUEST_FLAGS_COMPLETION_AREA_TRIGGER_LIKE_CPP: u32 = 0x0000_0004;
const QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP: u32 = 0x0000_0400;
const QUEST_FLAGS_EX_REWARDS_IGNORE_CAPS_LIKE_CPP: u32 = 0x0080_0000;
const QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP: u32 = 0x0100_0000;
pub(crate) const QUEST_PUSH_REASON_INVALID_LIKE_CPP: u8 = 1;
pub(crate) const QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP: u8 = 2;
const QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL: u8 = 0;
const QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL: u8 = 1;
const QUEST_OBJECTIVE_GAMEOBJECT_LIKE_CPP_LOCAL: u8 = 2;
const QUEST_OBJECTIVE_TALKTO_LIKE_CPP_LOCAL: u8 = 3;
const QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL: u8 = 4;
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
struct QuestSourceItemBoundPreflightLikeCpp {
    no_grant: bool,
    changed_quest_ids: Vec<u32>,
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
    receiver: &wow_network::PlayerBroadcastInfo,
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

// ── Handler registrations ────────────────────────────────────────────────────

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

    async fn save_represented_quest_status_like_cpp(&mut self, quest_id: u32) {
        if let Some(status) = self
            .player_quests
            .get(&quest_id)
            .map(|status| status.status)
        {
            self.save_quest_to_db(quest_id, status).await;
        }
    }

    async fn quest_source_item_quest_log_item_id_like_cpp(&mut self, entry_id: u32) -> u32 {
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

    async fn apply_quest_source_item_added_non_bound_objective_progress_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Vec<u32> {
        let Some(quest_store) = self.quest_store.clone() else {
            return Vec::new();
        };
        let count = i32::try_from(count).unwrap_or(i32::MAX);
        let mut objective_ids = vec![i32::try_from(entry_id).unwrap_or(i32::MAX)];
        if quest_log_item_id != 0 {
            objective_ids.push(i32::try_from(quest_log_item_id).unwrap_or(i32::MAX));
        }

        let mut changed_quest_ids = Vec::new();
        let mut quests_to_complete = Vec::new();
        for status in self.player_quests.values_mut() {
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
                if (objective.flags2 & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL) != 0
                {
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
            }
        }
        for quest_id in quests_to_complete {
            if let Some(quest) = quest_store.get(quest_id).cloned() {
                self.complete_represented_quest_after_add_if_ready_like_cpp(&quest)
                    .await;
            }
        }
        self.sync_player_registry_state_like_cpp();
        changed_quest_ids
    }

    async fn apply_quest_source_item_bound_objective_progress_for_object_like_cpp(
        &mut self,
        quest_store: &QuestStore,
        object_id: i32,
        count_i32: i32,
    ) -> Vec<(u32, i32)> {
        let mut updated_counts = Vec::new();
        let mut quests_to_complete = Vec::new();

        for status in self.player_quests.values_mut() {
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

    async fn apply_quest_source_item_bound_objective_preflight_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Option<QuestSourceItemBoundPreflightLikeCpp> {
        let Some(player_guid) = self.player_guid() else {
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
            quantity_in_inventory: u32::try_from(updated_counts[0].1.max(0)).unwrap_or(u32::MAX),
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
        Some(QuestSourceItemBoundPreflightLikeCpp {
            no_grant: true,
            changed_quest_ids,
        })
    }

    /// CMSG_QUEST_GIVER_STATUS_QUERY — returns the quest status icon for an NPC.
    /// C# ref: QuestHandler.HandleQuestGiverStatusQuery
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
    /// Remaining represented gaps: fake-death aura removal, movement pause/home position,
    /// `AI()->OnGossipHello`, `PrepareGossipMenu`, `SendPreparedGossip` / auto-open / PlayerTalkClass.
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

        if self.use_represented_creature_questgiver_like_cpp(guid, access.entry) {
            debug!(
                account = self.account_id,
                creature_entry = access.entry,
                "QuestGiverHello represented Creature questgiver seam consumed"
            );
        }
    }

    /// CMSG_QUEST_GIVER_QUERY_QUEST — player clicks a quest name in the list.
    /// Shows full quest details (objectives, rewards) before accepting.
    /// C# ref: QuestHandler.HandleQuestGiverQueryQuest
    pub async fn handle_quest_giver_query_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("QuestGiverQueryQuest: failed to read GUID");
                return;
            }
        };
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let _resend_offer: bool = pkt.read_uint8().unwrap_or(0) != 0;

        let _ = self.send_represented_quest_giver_query_quest_like_cpp(guid, quest_id);
    }

    /// CMSG_QUEST_GIVER_ACCEPT_QUEST — player clicks "Accept" in the quest details dialog.
    /// Saves quest to characters DB and confirms to the client.
    /// C# ref: QuestHandler.HandleQuestGiverAcceptQuest
    pub async fn handle_quest_giver_accept_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("QuestGiverAcceptQuest: failed to read GUID");
                return;
            }
        };
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let _start_cheat: bool = pkt.read_uint8().unwrap_or(0) != 0;

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
            debug!(
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
        // C# ref: Player.CanTakeQuest(quest, true)
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
        let mut next_item_guid = if dest.iter().any(|dest| {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;
            self.get_inventory_item_by_pos(bag, slot).is_none()
        }) {
            if let Some(char_db) = self.char_db().map(Arc::clone) {
                let max_guid_stmt = char_db.prepare(CharStatements::SEL_MAX_ITEM_GUID);
                match char_db.query(&max_guid_stmt).await {
                    Ok(row) => row.try_read::<u64>(0).unwrap_or(0).saturating_add(1),
                    Err(error) => {
                        warn!(
                            account = self.account_id,
                            entry_id,
                            ?error,
                            "QuestConfirmAccept: failed to allocate DB item guid for source item"
                        );
                        self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                        return None;
                    }
                }
            } else {
                self.inventory_items_like_cpp()
                    .values()
                    .map(|item| item.db_guid)
                    .chain(
                        self.inventory_item_objects_like_cpp()
                            .keys()
                            .map(|guid| guid.counter() as u64),
                    )
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1)
            }
        } else {
            0
        };

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

                let db_guid = next_item_guid;
                next_item_guid = next_item_guid.saturating_add(1);
                let item_guid = ObjectGuid::create_item(self.realm_id(), db_guid as i64);
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
                    context: ItemContext::None as u8,
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
        let mut next_item_guid = if dest.iter().any(|dest| {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;
            self.get_inventory_item_by_pos(bag, slot).is_none()
        }) {
            if let Some(char_db) = self.char_db().map(Arc::clone) {
                let max_guid_stmt = char_db.prepare(CharStatements::SEL_MAX_ITEM_GUID);
                match char_db.query(&max_guid_stmt).await {
                    Ok(row) => row.try_read::<u64>(0).unwrap_or(0).saturating_add(1),
                    Err(error) => {
                        warn!(
                            account = self.account_id,
                            entry_id,
                            ?error,
                            "RewardQuest: failed to allocate DB item guid for reward item"
                        );
                        self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                        return false;
                    }
                }
            } else {
                self.inventory_items_like_cpp()
                    .values()
                    .map(|item| item.db_guid)
                    .chain(
                        self.inventory_item_objects_like_cpp()
                            .keys()
                            .map(|guid| guid.counter() as u64),
                    )
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1)
            }
        } else {
            0
        };

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

                let db_guid = next_item_guid;
                next_item_guid = next_item_guid.saturating_add(1);
                let item_guid = ObjectGuid::create_item(self.realm_id(), db_guid as i64);
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
                    context: ItemContext::QuestReward as u8,
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

        let Some(sender_snapshot) = player_registry
            .get(&pending.sender_guid)
            .map(|entry| entry.clone())
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

        let sender_active_status = sender_snapshot
            .active_quest_statuses
            .get(&parsed_quest_id)
            .copied();
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
                    .get(&member_guid)
                    .map(|receiver| (member_guid, receiver.clone()))
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
                    let _ = receiver.send_tx.send(
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
                let _ = receiver.send_tx.send(
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
                    let _ = receiver.send_tx.send(
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
                let _ = receiver.send_tx.send(
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
                let _ = receiver.send_tx.send(
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
                let _ = receiver.send_tx.send(
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
                let _ = receiver.send_tx.send(
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
                let _ = receiver.send_tx.send(
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
                let _ = receiver.send_tx.send(
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
                let _ = receiver.send_tx.send(
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
                let _ = receiver.send_tx.send(
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
                let _ = receiver.send_tx.send(
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
                let _ = receiver.send_tx.send(
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

                if receiver.command_tx.try_send(command).is_err() {
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

            if receiver.command_tx.try_send(command).is_err() {
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

        self.player_quests.remove(&qid);
        self.delete_quest_from_db(qid).await;
        self.sync_player_registry_state_like_cpp();
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
        // represented active slots are COMPLETE or INCOMPLETE only for this bounded helper.
        slot < MAX_QUEST_LOG_SIZE_LIKE_CPP
            && self.player_quests.values().any(|status| {
                status.slot == slot
                    && matches!(
                        status.status,
                        QUEST_STATUS_COMPLETE_LIKE_CPP | QUEST_STATUS_INCOMPLETE_LIKE_CPP
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
                    QUEST_STATUS_COMPLETE_LIKE_CPP | QUEST_STATUS_INCOMPLETE_LIKE_CPP
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
                    QUEST_STATUS_COMPLETE_LIKE_CPP | QUEST_STATUS_INCOMPLETE_LIKE_CPP
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

                let state_flags: u32 = if qs.status == QUEST_STATUS_COMPLETE_LIKE_CPP {
                    1
                } else {
                    0
                };
                let mut obj_progress = [0u16; 24];
                for (i, &count) in qs.objective_counts.iter().enumerate().take(24) {
                    obj_progress[i] = count.min(u16::MAX as i32) as u16;
                }
                (qs.quest_id, state_flags, 0i64, obj_progress)
            })
            .collect()
    }

    /// CMSG_QUERY_QUEST_INFO — client asks for full quest template data by ID.
    /// Used to populate the quest log and tooltip.
    /// C# ref: QuestHandler.HandleQueryQuestInfo
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

    /// CMSG_QUEST_GIVER_REQUEST_REWARD — player talks to NPC to turn in a completed quest.
    /// C# ref: QuestHandler.HandleQuestgiverRequestReward
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
            debug!(
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
            debug!(
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

        // C#: SendQuestGiverOfferReward(quest, questGiverGUID, true)
        self.send_packet(&QuestGiverOfferReward {
            giver_guid: guid,
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
    /// C# ref: QuestHandler.HandleQuestGiverCompleteQuest
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
                debug!(
                    account = self.account_id,
                    ?guid,
                    quest_id,
                    from_script,
                    "QuestGiverCompleteQuest: represented involved source rejected"
                );
                return;
            }
        } else if !from_script || self.player_guid() != Some(guid) {
            debug!(
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

        // Check if all objectives are done — C++ GetQuestStatus == QUEST_STATUS_COMPLETE.
        let is_complete = self
            .player_quests
            .get(&quest_id)
            .is_some_and(|qs| qs.status == QUEST_STATUS_COMPLETE_LIKE_CPP);

        if !is_complete {
            // Not all objectives done — send "you still need X" dialog
            // C# ref: SendQuestGiverRequestItems(quest, guid, canComplete=false, false)
            self.send_packet(&QuestGiverRequestItems {
                giver_guid: guid,
                giver_creature_id: i32::try_from(guid.entry()).unwrap_or(0),
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

        if !self
            .remove_quest_required_items_and_currencies_like_cpp(quest)
            .await
        {
            debug!(
                account = self.account_id,
                quest_id,
                "RewardQuest: represented quest objective/item-drop removal failed before reward mutation"
            );
            return false;
        }

        self.remove_represented_timed_quest_like_cpp(quest_id);

        if !self.store_fixed_quest_reward_items_like_cpp(quest).await {
            debug!(
                account = self.account_id,
                quest_id,
                "RewardQuest: represented fixed reward item grant failed before reward mutation"
            );
            return false;
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
            return false;
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
            return false;
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
            return false;
        }

        self.apply_represented_quest_reward_skill_like_cpp(quest);

        let money = quest.reward_money_difficulty;
        if money > 0 {
            let old_money = self.player_gold_like_cpp();
            let new_money = old_money.saturating_add(money as u64);
            self.enqueue_represented_quest_objective_progress_like_cpp(
                RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                    old_money,
                    new_money,
                },
            );
            self.set_player_gold_like_cpp(new_money);
            self.save_player_gold().await;
        }

        self.apply_represented_quest_title_and_talent_rewards_like_cpp(quest);
        self.record_represented_quest_reward_mail_like_cpp(quest, quest_giver_guid);
        self.apply_quest_reward_lockout_status_like_cpp(quest).await;

        let xp = self.calculate_quest_xp(quest.reward_xp_difficulty, quest.quest_level);

        self.player_quests.remove(&quest_id);
        if !quest.is_repeatable() {
            self.rewarded_quests.insert(quest_id);
            self.save_quest_to_db(quest_id, QUEST_STATUS_REWARDED_LIKE_CPP)
                .await;
        } else {
            self.delete_quest_from_db(quest_id).await;
        }
        self.sync_player_registry_state_like_cpp();

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
            let player_guid = self
                .player_guid()
                .unwrap_or(wow_core::ObjectGuid::new(0, 0));
            self.give_xp(xp, player_guid, false).await;
        }

        true
    }

    /// CMSG_QUEST_GIVER_CHOOSE_REWARD — player clicks "Complete Quest" in reward dialog.
    /// Gives XP, gold, items. Removes quest from active log.
    /// C# ref: QuestHandler.HandleQuestGiverChooseReward
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
            debug!(
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

    fn represented_quest_is_important_like_cpp(
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
    /// C# ref: Player.CanTakeQuest (SatisfyQuestStatus + SatisfyQuestRace/Class/Level + PrevQuest)
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

        // SatisfyQuestSeasonal — C++ Player::SatisfyQuestSeasonal
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

        // SatisfyQuestRace + SatisfyQuestClass + SatisfyQuestLevel
        if !quest.is_available_for(
            self.player_race_like_cpp(),
            self.player_class_like_cpp(),
            self.player_level_like_cpp(),
        ) {
            return false;
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

    fn is_quest_disabled_like_cpp(&self, quest_id: u32) -> bool {
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
    async fn save_quest_to_db(&self, quest_id: u32, status: u8) {
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
        let represented_explored = self
            .player_quests
            .get(&quest_id)
            .map(|status| status.explored)
            .unwrap_or(false);
        let represented_accept_time = self
            .player_quests
            .get(&quest_id)
            .map(|status| status.accept_time_secs)
            .unwrap_or(0);
        let represented_end_time = self
            .player_quests
            .get(&quest_id)
            .map(|status| status.end_time_secs)
            .unwrap_or(0);
        let mut stmt = char_db.prepare(CharStatements::INS_CHAR_QUEST_STATUS);
        stmt.set_u64(0, guid);
        stmt.set_u32(1, quest_id);
        stmt.set_u8(2, status);
        stmt.set_u8(3, u8::from(represented_explored));
        stmt.set_i64(4, represented_accept_time);
        stmt.set_i64(5, represented_end_time);
        tx.append(stmt);

        let mut del_objectives =
            char_db.prepare(CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST);
        del_objectives.set_u64(0, guid);
        del_objectives.set_u32(1, quest_id);
        tx.append(del_objectives);

        if let (Some(quest_store), Some(saved_status)) =
            (self.quest_store.as_ref(), self.player_quests.get(&quest_id))
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
                let mut rep_objective =
                    char_db.prepare(CharStatements::REP_CHAR_QUEST_STATUS_OBJECTIVES);
                rep_objective.set_u64(0, guid);
                rep_objective.set_u32(1, quest_id);
                rep_objective.set_u8(2, objective_index);
                rep_objective.set_i32(3, count);
                tx.append(rep_objective);
            }
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

        let mut next_active_slot: u8 = 0;

        if !result.is_empty() {
            let mut result = result;
            loop {
                let quest_id: u32 = result.try_read::<u32>(0).unwrap_or(0);
                let status: u8 = result.try_read::<u8>(1).unwrap_or(0);
                let explored: bool = result.try_read::<u8>(2).unwrap_or(0) != 0;
                let accept_time_secs: i64 = result.try_read::<i64>(3).unwrap_or(0);
                let end_time_secs: i64 = result.try_read::<i64>(4).unwrap_or(0);

                if status == QUEST_STATUS_REWARDED_LIKE_CPP {
                    // Rewarded (C++ QuestStatus::QUEST_STATUS_REWARDED / m_RewardedQuests).
                    // Non-repeatable quests cannot be re-taken once rewarded.
                    self.rewarded_quests.insert(quest_id);
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
mod tests {
    use super::*;
    use crate::session::InventoryItem;
    use wow_constants::{
        ComparisonType, ConditionSourceType, ConditionType, InventoryType, ItemBondingType,
        ItemClass, ItemContext,
    };
    use wow_core::guid::HighGuid;
    use wow_core::{ObjectGuid, Position};
    use wow_data::quest::{
        QUEST_FLAGS_DAILY_LIKE_CPP, QUEST_ITEM_DROP_COUNT, QUEST_REWARD_CHOICES_COUNT,
        QUEST_REWARD_CURRENCY_COUNT, QUEST_REWARD_DISPLAY_SPELL_COUNT, QUEST_REWARD_ITEM_COUNT,
        QUEST_REWARD_REPUTATIONS_COUNT, QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP, QuestObjective,
        QuestPoolMemberRowLikeCpp, QuestPoolSavedActiveRowLikeCpp, QuestPoolStoreLikeCpp,
        QuestStore, QuestTemplate,
    };
    use wow_data::{
        Condition, ConditionEntriesByTypeStore, CurrencyTypesEntry, CurrencyTypesStore,
        ItemLimitCategoryEntry, ItemLimitCategoryStore, ItemRecord, ItemSparseTemplateEntry,
        ItemStatsStore, ItemStore,
        progression_rewards::{
            FactionEntry, FactionStore, QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP,
            QuestFactionRewardEntry, QuestFactionRewardStore, QuestInfoEntry, QuestInfoStore,
            QuestPackageItemEntry, QuestPackageItemStore,
        },
        reputation::{ReputationRewardRateEntryLikeCpp, ReputationRewardRateStoreLikeCpp},
    };
    use wow_database::{PreparedStatement, SqlParam, StatementDef};
    use wow_entities::{ITEM_LIMIT_CATEGORY_MODE_HAVE, Player, PlayerReputationRecord};
    use wow_network::{GroupInfo, GroupRegistry, PendingInvites, PlayerRegistry};
    use wow_packet::WorldPacket;
    use wow_packet::packets::item::InventoryChangeFailure;
    use wow_packet::packets::quest::QuestGiverQuestFailed;

    fn make_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
        let (_pkt_tx, pkt_rx) = flume::bounded(8);
        let (send_tx, send_rx) = flume::bounded(8);
        let mut session = WorldSession::new(
            1,
            "QuestStatusTest".into(),
            0,
            2,
            9,
            54261,
            vec![0; 40],
            "enUS".into(),
            pkt_rx,
            send_tx,
        );
        session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        session.set_player_position_like_cpp(Position::new(10.0, 0.0, 0.0, 0.0));
        (session, send_rx)
    }

    #[test]
    fn player_quest_status_load_binds_full_u64_guid_like_cpp() {
        let guid = ObjectGuid::create_player(1, i64::from(u32::MAX) + 42);

        for statement in [
            CharStatements::SEL_CHAR_QUEST_STATUS,
            CharStatements::SEL_CHAR_QUEST_STATUS_OBJECTIVES,
            CharStatements::SEL_CHAR_QUEST_STATUS_SEASONAL,
        ] {
            let mut stmt = PreparedStatement::new(statement.sql());
            WorldSession::bind_player_quest_status_load_guid_like_cpp(&mut stmt, guid);
            assert_eq!(stmt.params().len(), 1);
            assert!(matches!(
                stmt.params()[0],
                SqlParam::U64(value) if value == guid.counter() as u64
            ));
        }
    }

    fn quest_template(id: u32) -> QuestTemplate {
        QuestTemplate {
            id,
            quest_type: 2,
            quest_level: 1,
            quest_max_scaling_level: 0,
            quest_package_id: 0,
            min_level: 1,
            quest_sort_id: 0,
            quest_info_id: 0,
            suggested_group_num: 0,
            reward_next_quest: 0,
            reward_xp_difficulty: 0,
            reward_xp_multiplier: 1.0,
            reward_money_difficulty: 0,
            reward_money_multiplier: 1.0,
            reward_bonus_money: 0,
            reward_display_spell: [0; QUEST_REWARD_DISPLAY_SPELL_COUNT],
            reward_spell: 0,
            reward_honor: 0,
            reward_title_id: 0,
            reward_skill_line_id: 0,
            reward_skill_points: 0,
            reward_mail_template_id: 0,
            reward_mail_delay_secs: 0,
            reward_mail_sender_entry: 0,
            reward_faction_ids: [0; QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_values: [0; QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_overrides: [0; QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_cap_in: [0; QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_flags: 0,
            source_item_id: 0,
            source_item_count: 0,
            source_spell_id: 0,
            limit_time_secs: 0,
            expansion: 0,
            flags: 0,
            flags_ex: 0,
            flags_ex2: 0,
            special_flags: 0,
            event_id_for_quest: 0,
            reward_items: [0; QUEST_REWARD_ITEM_COUNT],
            reward_amounts: [0; QUEST_REWARD_ITEM_COUNT],
            reward_currencies: [0; QUEST_REWARD_CURRENCY_COUNT],
            reward_currency_amounts: [0; QUEST_REWARD_CURRENCY_COUNT],
            item_drop: [0; QUEST_ITEM_DROP_COUNT],
            item_drop_quantity: [0; QUEST_ITEM_DROP_COUNT],
            log_title: format!("Quest {id}"),
            log_description: String::new(),
            quest_description: String::new(),
            area_description: String::new(),
            quest_completion_log: String::new(),
            objectives: Vec::new(),
            allowable_races: 0,
            allowable_classes: 0,
            max_level: 0,
            prev_quest_id: 0,
            next_quest_id: 0,
            exclusive_group: 0,
            breadcrumb_for_quest_id: 0,
            dependent_previous_quests: Vec::new(),
            dependent_breadcrumb_quests: Vec::new(),
            required_min_rep_faction: 0,
            required_min_rep_value: 0,
            required_max_rep_faction: 0,
            required_max_rep_value: 0,
            reward_choice_items: [(0, 0); QUEST_REWARD_CHOICES_COUNT],
            reward_choice_item_types: [0; QUEST_REWARD_CHOICES_COUNT],
        }
    }

    fn quest_info_entry_like_cpp(id: u32, quest_type: i8, modifiers: i32) -> QuestInfoEntry {
        QuestInfoEntry {
            id,
            info_name: String::new(),
            quest_type,
            modifiers,
            profession: 0,
        }
    }

    fn store_with_quests(ids: &[u32]) -> QuestStore {
        QuestStore::from_quests_like_cpp(ids.iter().copied().map(quest_template))
    }

    fn quest_template_with_objective_count(id: u32, objective_count: usize) -> QuestTemplate {
        let mut quest = quest_template(id);
        quest.objectives = (0..objective_count)
            .map(|index| QuestObjective {
                id: id * 10 + index as u32,
                quest_id: id,
                obj_type: 0,
                order: index as u8,
                storage_index: index as i8,
                object_id: 1000 + index as i32,
                amount: 1,
                flags: 0,
                flags2: 0,
                progress_bar_weight: 0.0,
                description: String::new(),
            })
            .collect();
        quest
    }

    fn store_with_sharable_quest_objectives(id: u32, objective_count: usize) -> QuestStore {
        let mut quest = quest_template_with_objective_count(id, objective_count);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        QuestStore::from_quests_like_cpp([quest])
    }

    fn store_with_sharable_timed_quest_objectives(
        id: u32,
        objective_count: usize,
        limit_time_secs: i64,
    ) -> QuestStore {
        let mut quest = quest_template_with_objective_count(id, objective_count);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        quest.limit_time_secs = limit_time_secs;
        QuestStore::from_quests_like_cpp([quest])
    }

    fn quest_template_with_source_item(
        id: u32,
        source_item_id: u32,
        source_item_count: u32,
        source_spell_id: u32,
    ) -> QuestTemplate {
        let mut quest = quest_template(id);
        quest.source_item_id = source_item_id;
        quest.source_item_count = source_item_count;
        quest.source_spell_id = source_spell_id;
        quest
    }

    fn store_with_source_item_quest(
        quest_id: u32,
        source_item_id: u32,
        source_item_count: u32,
        source_spell_id: u32,
    ) -> QuestStore {
        QuestStore::from_quests_like_cpp([quest_template_with_source_item(
            quest_id,
            source_item_id,
            source_item_count,
            source_spell_id,
        )])
    }

    fn install_source_item_template(
        session: &mut WorldSession,
        entry: u32,
        stackable: i32,
        max_count: u32,
    ) {
        install_source_item_template_with_start_quest_limit_category_and_flags3(
            session, entry, stackable, max_count, 0, 0, 0,
        );
    }

    fn install_source_item_template_with_flags3(
        session: &mut WorldSession,
        entry: u32,
        stackable: i32,
        max_count: u32,
        flags3: u32,
    ) {
        install_source_item_template_with_start_quest_limit_category_and_flags3(
            session, entry, stackable, max_count, 0, 0, flags3,
        );
    }

    fn install_source_item_template_with_start_quest(
        session: &mut WorldSession,
        entry: u32,
        stackable: i32,
        max_count: u32,
        start_quest_id: i32,
    ) {
        install_source_item_template_with_start_quest_and_limit_category(
            session,
            entry,
            stackable,
            max_count,
            start_quest_id,
            0,
        );
    }

    fn install_source_item_template_with_limit_category(
        session: &mut WorldSession,
        entry: u32,
        stackable: i32,
        max_count: u32,
        limit_category: u16,
    ) {
        install_source_item_template_with_start_quest_and_limit_category(
            session,
            entry,
            stackable,
            max_count,
            0,
            limit_category,
        );
    }

    fn install_source_item_template_with_start_quest_and_limit_category(
        session: &mut WorldSession,
        entry: u32,
        stackable: i32,
        max_count: u32,
        start_quest_id: i32,
        limit_category: u16,
    ) {
        install_source_item_template_with_start_quest_limit_category_and_flags3(
            session,
            entry,
            stackable,
            max_count,
            start_quest_id,
            limit_category,
            0,
        );
    }

    fn install_source_item_template_with_start_quest_limit_category_and_flags3(
        session: &mut WorldSession,
        entry: u32,
        stackable: i32,
        max_count: u32,
        start_quest_id: i32,
        limit_category: u16,
        flags3: u32,
    ) {
        install_source_item_template_with_start_quest_limit_category_flags3_and_bonding(
            session,
            entry,
            stackable,
            max_count,
            start_quest_id,
            limit_category,
            flags3,
            ItemBondingType::None,
        );
    }

    fn install_source_item_template_with_start_quest_limit_category_flags3_and_bonding(
        session: &mut WorldSession,
        entry: u32,
        stackable: i32,
        max_count: u32,
        start_quest_id: i32,
        limit_category: u16,
        flags3: u32,
        bonding: ItemBondingType,
    ) {
        session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
            id: entry,
            class_id: ItemClass::Consumable as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::NonEquip as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
        }])));
        session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
            entry,
            ItemSparseTemplateEntry {
                flags: [0, 0, flags3, 0],
                bag_family: 0,
                start_quest_id,
                stackable,
                max_count: i32::try_from(max_count).unwrap_or(i32::MAX),
                lock_id: 0,
                required_reputation_rank: 0,
                sell_price: 0,
                buy_price: 0,
                vendor_stack_count: 1,
                price_variance: 1.0,
                price_random_value: 1.0,
                max_durability: 0,
                limit_category,
                instance_bound: 0,
                zone_bound: [0, 0],
                required_reputation_faction: 0,
                allowable_class: -1,
                required_expansion: 0,
                bonding: bonding as u8,
                container_slots: 0,
                inventory_type: InventoryType::NonEquip as i8,
            },
        )])));
    }

    fn insert_direct_inventory_item(
        session: &mut WorldSession,
        player_guid: ObjectGuid,
        slot: u8,
        entry: u32,
        count: u32,
        db_guid: u64,
    ) {
        let item_guid = ObjectGuid::create_item(1, db_guid as i64);
        session.insert_inventory_item_like_cpp(
            slot,
            InventoryItem {
                guid: item_guid,
                entry_id: entry,
                db_guid,
                inventory_type: None,
            },
        );
        let item = session.make_inventory_item_object(
            item_guid,
            entry,
            player_guid,
            count,
            0,
            ItemContext::None,
            slot,
        );
        session.insert_inventory_item_object(item);
    }

    fn install_have_limit_category_like_cpp(
        session: &mut WorldSession,
        category_id: u32,
        quantity: u8,
    ) {
        session.set_item_limit_category_store(Arc::new(ItemLimitCategoryStore::from_entries([
            ItemLimitCategoryEntry {
                id: category_id,
                name: format!("Have Limit {category_id}"),
                quantity,
                flags: ITEM_LIMIT_CATEGORY_MODE_HAVE,
            },
        ])));
    }

    #[test]
    fn query_quest_completion_builds_creature_then_masked_go_entries_like_cpp() {
        let mut store = store_with_quests(&[77]);
        store.ender_quests.entry(1234).or_default().push(77);
        store.ender_quests.entry(12).or_default().push(77);
        store
            .gameobject_ender_quests
            .entry(0x5678)
            .or_default()
            .push(77);

        let response = represented_quest_completion_npc_response_like_cpp(&store, &[77]);

        assert_eq!(response.len(), 1);
        assert_eq!(response[0].quest_id, 77);
        assert_eq!(response[0].npcs, vec![12, 1234, 0x8000_5678u32 as i32]);
    }

    #[test]
    fn query_quest_completion_skips_negative_missing_and_oversized_creature_entries_like_cpp() {
        let mut store = store_with_quests(&[5]);
        store
            .ender_quests
            .entry(i32::MAX as u32 + 1)
            .or_default()
            .push(5);
        store
            .gameobject_ender_quests
            .entry(u32::MAX)
            .or_default()
            .push(5);

        let response = represented_quest_completion_npc_response_like_cpp(&store, &[-1, 999, 5]);

        assert_eq!(response.len(), 1);
        assert_eq!(response[0].quest_id, 5);
        assert_eq!(response[0].npcs, vec![-1]);
    }

    fn creature_guid(entry: u32, counter: i64) -> ObjectGuid {
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, entry, counter)
    }

    fn gameobject_guid(entry: u32, counter: i64) -> ObjectGuid {
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, entry, counter)
    }

    fn insert_creature(manager: &mut wow_map::MapManager, guid: ObjectGuid, entry: u32) {
        let mut creature = wow_entities::Creature::new(false);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(entry);
        creature.unit_mut().world_mut().set_map(571, 0).unwrap();
        creature
            .unit_mut()
            .world_mut()
            .relocate(Position::new(10.0, 0.0, 0.0, 0.0));
        creature.unit_mut().set_level(80);
        creature.set_ai_identity_runtime(1, 35, NPCFlags1::QUEST_GIVER.bits(), 0);
        manager
            .create_world_map(571, 0)
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_creature(creature).unwrap(),
            )
            .unwrap();
    }

    fn insert_gameobject(manager: &mut wow_map::MapManager, guid: ObjectGuid, entry: u32) {
        let mut gameobject = wow_entities::GameObject::new();
        gameobject.world_mut().object_mut().create(guid);
        gameobject.world_mut().object_mut().set_entry(entry);
        gameobject.world_mut().set_map(571, 0).unwrap();
        gameobject
            .world_mut()
            .relocate(Position::new(10.0, 0.0, 0.0, 0.0));
        gameobject.world_mut().object_mut().add_to_world();
        manager
            .create_world_map(571, 0)
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
            )
            .unwrap();
    }

    fn insert_player_with_reputation(
        manager: &mut wow_map::MapManager,
        guid: ObjectGuid,
        faction_id: u32,
        standing: i32,
    ) {
        let mut player = Player::new(Some(1), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_map(571, 0).unwrap();
        player
            .unit_mut()
            .world_mut()
            .relocate(Position::new(10.0, 0.0, 0.0, 0.0));
        player
            .gameplay_state_mut()
            .reputations
            .push(PlayerReputationRecord {
                faction_id,
                standing,
                flags: 0,
            });
        manager
            .create_world_map(571, 0)
            .map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
            .unwrap();
    }

    fn attach_map_manager(session: &mut WorldSession, manager: wow_map::MapManager) {
        session.set_canonical_map_manager(Arc::new(std::sync::Mutex::new(manager)));
    }

    async fn run_status_query(session: &mut WorldSession, guid: ObjectGuid) {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&guid);
        session.handle_quest_giver_status_query(pkt).await;
    }

    fn add_active_quest(session: &mut WorldSession, quest_id: u32) {
        let slot = session.first_free_quest_slot_like_cpp().unwrap_or(0);
        add_active_quest_in_slot(session, quest_id, slot);
    }

    fn add_active_quest_in_slot(session: &mut WorldSession, quest_id: u32, slot: u8) {
        add_active_quest_in_slot_with_status(
            session,
            quest_id,
            slot,
            QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        );
    }

    fn add_active_quest_in_slot_with_status(
        session: &mut WorldSession,
        quest_id: u32,
        slot: u8,
        status: u8,
    ) {
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot,
            },
        );
    }

    #[test]
    fn represented_quest_objective_completable_accepts_cpp_storing_value_previous_types() {
        let quest_id = 7100;
        let mut quest = quest_template(quest_id);
        quest.objectives = vec![
            QuestObjective {
                id: quest_id * 10,
                quest_id,
                obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
                order: 0,
                storage_index: 0,
                object_id: 44,
                amount: 1,
                flags: 0,
                flags2: 0,
                progress_bar_weight: 0.0,
                description: String::new(),
            },
            QuestObjective {
                id: quest_id * 10 + 1,
                quest_id,
                obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
                order: 1,
                storage_index: 1,
                object_id: 55,
                amount: 1,
                flags: QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL,
                flags2: 0,
                progress_bar_weight: 0.0,
                description: String::new(),
            },
        ];
        let status = PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![1, 0],
            slot: 0,
        };

        assert!(WorldSession::represented_quest_objective_completable_like_cpp(&status, &quest, 1));
    }

    #[test]
    fn represented_objective_negative_storage_index_does_not_alias_slot_zero_like_cpp() {
        let quest_id = 7101;
        let mut quest = quest_template(quest_id);
        let objective = QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: -1,
            object_id: 55,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        };
        quest.objectives = vec![objective.clone()];
        let status = PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![1],
            slot: 0,
        };

        assert!(
            !WorldSession::represented_quest_objective_complete_like_cpp(
                &status, &quest, &objective
            )
        );
    }

    async fn run_close_quest(session: &mut WorldSession, quest_id: u32) {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(quest_id);
        session.handle_quest_giver_close_quest(pkt).await;
    }

    async fn run_remove_quest_slot(session: &mut WorldSession, slot: u8) {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint8(slot);
        session.handle_quest_log_remove_quest(pkt).await;
    }

    async fn run_request_world_quest_update(session: &mut WorldSession) {
        session
            .handle_request_world_quest_update(WorldPacket::new_empty())
            .await;
    }

    async fn run_quest_confirm_accept(session: &mut WorldSession, quest_id: i32) {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(quest_id);
        session.handle_quest_confirm_accept(pkt).await;
    }

    fn write_cpp_item_instance_like_cpp(
        pkt: &mut WorldPacket,
        item_id: i32,
        random_properties_seed: i32,
        random_properties_id: i32,
        item_mods: &[(i32, u8)],
        item_bonus_ids: Option<&[u32]>,
    ) {
        pkt.write_int32(item_id);
        pkt.write_int32(random_properties_seed);
        pkt.write_int32(random_properties_id);
        pkt.write_bit(item_bonus_ids.is_some());
        pkt.flush_bits();
        pkt.write_bits(item_mods.len() as u32, 6);
        pkt.flush_bits();
        for (value, modifier_type) in item_mods {
            pkt.write_int32(*value);
            pkt.write_uint8(*modifier_type);
        }
        if let Some(item_bonus_ids) = item_bonus_ids {
            pkt.write_uint8(0);
            pkt.write_uint32(item_bonus_ids.len() as u32);
            for bonus_id in item_bonus_ids {
                pkt.write_uint32(*bonus_id);
            }
        }
    }

    fn write_cpp_quest_choice_item_like_cpp(
        pkt: &mut WorldPacket,
        loot_item_type: u8,
        item_id: i32,
        quantity: i32,
    ) {
        pkt.reset_bits();
        pkt.write_bits(u32::from(loot_item_type), 2);
        write_cpp_item_instance_like_cpp(pkt, item_id, 0, 0, &[], None);
        pkt.write_int32(quantity);
    }

    fn quest_giver_choose_reward_packet_like_cpp(
        source_guid: ObjectGuid,
        quest_id: u32,
        loot_item_type: u8,
        item_id: u32,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&source_guid);
        pkt.write_uint32(quest_id);
        write_cpp_quest_choice_item_like_cpp(
            &mut pkt,
            loot_item_type,
            item_id as i32,
            if item_id == 0 { 0 } else { 1 },
        );
        pkt
    }

    fn quest_giver_request_reward_packet_like_cpp(
        source_guid: ObjectGuid,
        quest_id: u32,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&source_guid);
        pkt.write_uint32(quest_id);
        pkt
    }

    fn currency_entry_like_cpp(id: u32) -> CurrencyTypesEntry {
        CurrencyTypesEntry {
            id,
            category_id: 0,
            inventory_icon_file_id: 0,
            spell_weight: 0,
            spell_category: 0,
            max_qty: 0,
            max_earnable_per_week: 0,
            quality: 0,
            faction_id: 0,
            award_condition_id: 0,
            flags: wow_constants::CurrencyTypesFlags::empty(),
            flags_b: wow_constants::CurrencyTypesFlagsB::empty(),
        }
    }

    fn install_test_item_template_with_flags2_like_cpp(
        session: &mut WorldSession,
        entry: u32,
        flags2: u32,
    ) {
        session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
            id: entry,
            class_id: ItemClass::Consumable as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::NonEquip as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
        }])));
        session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
            entry,
            ItemSparseTemplateEntry {
                flags: [0, flags2, 0, 0],
                bag_family: 0,
                start_quest_id: 0,
                stackable: 1,
                max_count: 0,
                lock_id: 0,
                required_reputation_rank: 0,
                sell_price: 0,
                buy_price: 0,
                vendor_stack_count: 1,
                price_variance: 0.0,
                price_random_value: 0.0,
                max_durability: 0,
                limit_category: 0,
                instance_bound: 0,
                zone_bound: [0; 2],
                required_reputation_faction: 0,
                allowable_class: -1,
                required_expansion: 0,
                bonding: ItemBondingType::None as u8,
                container_slots: 0,
                inventory_type: InventoryType::NonEquip as i8,
            },
        )])));
    }

    #[test]
    fn quest_giver_choose_reward_choice_parser_reads_cpp_wire_item_choice() {
        let guid = ObjectGuid::create_player(1, 42);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&guid);
        pkt.write_uint32(7001);
        write_cpp_quest_choice_item_like_cpp(
            &mut pkt,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            19019,
            3,
        );

        assert_eq!(pkt.read_packed_guid().unwrap(), guid);
        assert_eq!(pkt.read_uint32().unwrap(), 7001);
        assert_eq!(
            WorldSession::read_quest_choice_item_like_cpp(&mut pkt).unwrap(),
            QuestChoiceItemLikeCpp {
                loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                item_id: 19019,
                quantity: 3,
            }
        );
        assert!(pkt.is_empty());
    }

    #[test]
    fn quest_giver_choose_reward_choice_parser_skips_cpp_item_mods_and_bonus() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bits(u32::from(QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP), 2);
        write_cpp_item_instance_like_cpp(&mut pkt, 392, 11, 22, &[(7, 1), (8, 2)], Some(&[91, 92]));
        pkt.write_int32(5);

        let choice = WorldSession::read_quest_choice_item_like_cpp(&mut pkt).unwrap();

        assert_eq!(
            choice,
            QuestChoiceItemLikeCpp {
                loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
                item_id: 392,
                quantity: 5,
            }
        );
        assert!(pkt.is_empty());
    }

    #[test]
    fn quest_giver_choose_reward_choice_parser_rejects_truncated_cpp_wire() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bits(u32::from(QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP), 2);
        write_cpp_item_instance_like_cpp(&mut pkt, 19019, 0, 0, &[], None);

        assert!(WorldSession::read_quest_choice_item_like_cpp(&mut pkt).is_err());
    }

    #[test]
    fn quest_giver_choose_reward_choice_validation_matches_loaded_cpp_type() {
        let mut quest = quest_template(7002);
        quest.reward_choice_items[0] = (19019, 1);
        quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
        quest.reward_choice_items[1] = (392, 5);
        quest.reward_choice_item_types[1] = QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP;

        assert!(
            WorldSession::represented_reward_choice_matches_loaded_type_like_cpp(
                &quest,
                QuestChoiceItemLikeCpp {
                    loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                    item_id: 19019,
                    quantity: 1,
                }
            )
        );
        assert!(
            WorldSession::represented_reward_choice_matches_loaded_type_like_cpp(
                &quest,
                QuestChoiceItemLikeCpp {
                    loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
                    item_id: 392,
                    quantity: 5,
                }
            )
        );
        assert!(
            !WorldSession::represented_reward_choice_matches_loaded_type_like_cpp(
                &quest,
                QuestChoiceItemLikeCpp {
                    loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                    item_id: 392,
                    quantity: 5,
                }
            )
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_rejects_missing_reward_item_template_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7003;
        let reward_item_id = 19019;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.reward_choice_items[0] = (reward_item_id, 1);
        quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
        session.set_player_gold_like_cpp(5);
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                reward_item_id,
            ))
            .await;

        assert_eq!(
            session
                .player_quests
                .get(&quest_id)
                .map(|status| status.status),
            Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
        );
        assert!(!session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 5);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_accepts_existing_reward_currency_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7004;
        let currency_id = 392;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.reward_choice_items[0] = (currency_id, 5);
        quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP;
        session.set_player_gold_like_cpp(5);
        session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
            currency_entry_like_cpp(currency_id),
        ])));
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
                currency_id,
            ))
            .await;

        assert!(!session.player_quests.contains_key(&quest_id));
        assert!(session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 42);
        assert_eq!(session.player_currency_quantity(currency_id), 5);
        assert_eq!(
            send_rx.try_recv().unwrap(),
            wow_packet::packets::misc::SetCurrency {
                type_id: currency_id as i32,
                quantity: 5,
                flags: 0,
                weekly_quantity: None,
                tracked_quantity: None,
                max_quantity: None,
                total_earned: None,
                suppress_chat_log: false,
                quantity_change: Some(5),
                quantity_gain_source: Some(CurrencyGainSourceLikeCpp::QuestReward as i32),
                quantity_lost_source: None,
                first_craft_operation_id: None,
                next_recharge_time: None,
                recharge_cycle_start_time: None,
                overflown_currency_id: None,
            }
            .to_bytes()
        );
        let opcodes = std::iter::from_fn(|| send_rx.try_recv().ok())
            .map(|bytes| wow_packet::WorldPacket::from_bytes(&bytes).server_opcode())
            .collect::<Vec<_>>();
        assert_eq!(
            opcodes,
            vec![
                Some(wow_constants::ServerOpcodes::QuestGiverQuestComplete),
                Some(wow_constants::ServerOpcodes::QuestUpdateComplete),
            ]
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_fixed_currency_rewards_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7014;
        let currency_id = 393;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.reward_currencies[0] = currency_id;
        quest.reward_currency_amounts[0] = 7;
        session.set_player_gold_like_cpp(5);
        session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
            currency_entry_like_cpp(currency_id),
        ])));
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert!(!session.player_quests.contains_key(&quest_id));
        assert!(session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 42);
        assert_eq!(session.player_currency_quantity(currency_id), 7);
        assert_eq!(
            send_rx.try_recv().unwrap(),
            wow_packet::packets::misc::SetCurrency {
                type_id: currency_id as i32,
                quantity: 7,
                flags: 0,
                weekly_quantity: None,
                tracked_quantity: None,
                max_quantity: None,
                total_earned: None,
                suppress_chat_log: false,
                quantity_change: Some(7),
                quantity_gain_source: Some(CurrencyGainSourceLikeCpp::DailyQuestReward as i32),
                quantity_lost_source: None,
                first_craft_operation_id: None,
                next_recharge_time: None,
                recharge_cycle_start_time: None,
                overflown_currency_id: None,
            }
            .to_bytes()
        );
        let opcodes = std::iter::from_fn(|| send_rx.try_recv().ok())
            .map(|bytes| wow_packet::WorldPacket::from_bytes(&bytes).server_opcode())
            .collect::<Vec<_>>();
        assert_eq!(
            opcodes,
            vec![
                Some(wow_constants::ServerOpcodes::QuestGiverQuestComplete),
                Some(wow_constants::ServerOpcodes::QuestUpdateComplete),
            ]
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_removes_timed_quest_before_rewards_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7020;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        session.set_player_gold_like_cpp(5);
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 100,
                end_time_secs: 700,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session.represented_timed_quest_removals_like_cpp(),
            &[quest_id]
        );
        assert!(!session.player_quests.contains_key(&quest_id));
        assert!(session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 42);
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_non_timed_quest_records_no_timed_removal_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7021;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 100,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert!(
            session
                .represented_timed_quest_removals_like_cpp()
                .is_empty()
        );
        assert!(!session.player_quests.contains_key(&quest_id));
        assert!(session.rewarded_quests.contains(&quest_id));
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_emits_reward_skill_fields_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7022;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.reward_skill_line_id = 333;
        quest.reward_skill_points = 5;
        session.set_player_gold_like_cpp(5);
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session.represented_quest_reward_skill_updates_like_cpp(),
            &[(333, 5)]
        );
        let complete = send_rx.try_recv().unwrap();
        assert_eq!(
            wow_packet::WorldPacket::from_bytes(&complete).server_opcode(),
            Some(wow_constants::ServerOpcodes::QuestGiverQuestComplete)
        );
        assert_eq!(&complete[18..22], &333u32.to_le_bytes());
        assert_eq!(&complete[22..26], &5u32.to_le_bytes());
        assert_eq!(session.player_gold_like_cpp(), 42);
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_records_title_and_talent_rewards_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7025;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_title_id = 77;
        quest.reward_skill_points = 3;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session.represented_quest_reward_titles_like_cpp(),
            &[RepresentedQuestRewardTitleLikeCpp {
                quest_id,
                title_id: 77,
                char_title_lookup_unrepresented: true,
                set_title_runtime_unrepresented: true,
            }]
        );
        assert_eq!(
            session.represented_quest_reward_talent_points_like_cpp(),
            &[RepresentedQuestRewardTalentPointsLikeCpp {
                quest_id,
                points: 3,
                init_talent_for_level_unrepresented: true,
            }]
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_records_reward_mail_sender_entry_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7026;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_mail_template_id = 55;
        quest.reward_mail_delay_secs = 900;
        quest.reward_mail_sender_entry = 1234;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session.represented_quest_reward_mails_like_cpp(),
            &[RepresentedQuestRewardMailLikeCpp {
                quest_id,
                mail_template_id: 55,
                delay_secs: 900,
                sender_entry: Some(1234),
                quest_giver_guid: None,
                mail_template_lookup_unrepresented: true,
                mail_draft_runtime_unrepresented: true,
                character_db_transaction_unrepresented: true,
            }]
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_records_reward_mail_quest_giver_sender_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7027;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_mail_template_id = 56;
        quest.reward_mail_delay_secs = 30;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session.represented_quest_reward_mails_like_cpp(),
            &[RepresentedQuestRewardMailLikeCpp {
                quest_id,
                mail_template_id: 56,
                delay_secs: 30,
                sender_entry: None,
                quest_giver_guid: Some(player_guid),
                mail_template_lookup_unrepresented: true,
                mail_draft_runtime_unrepresented: true,
                character_db_transaction_unrepresented: true,
            }]
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_records_reward_reputation_override_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7028;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
        quest.reward_faction_ids[2] = 930;
        quest.reward_faction_values[2] = 7;
        quest.reward_faction_overrides[2] = 1200;
        quest.reward_faction_cap_in[2] = 5;
        quest.reward_faction_flags = 1 << 2;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session.represented_quest_reward_reputations_like_cpp(),
            &[RepresentedQuestRewardReputationLikeCpp {
                quest_id,
                slot: 2,
                faction_id: 930,
                reward_faction_value: 7,
                reward_faction_override: 1200,
                reward_faction_cap_in: 5,
                base_reputation_before_gain: 12,
                reputation_after_low_level_rate_like_cpp: 12,
                reputation_after_reward_rate_like_cpp: 12,
                no_quest_bonus: true,
                no_spillover: true,
                source: RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest,
                faction_store_lookup_unrepresented: true,
                quest_faction_reward_store_lookup_unrepresented: false,
                reputation_reward_rate_lookup_unrepresented: true,
                gray_level_script_hook_unrepresented: true,
                reputation_rank_cap_check_unrepresented: true,
                calculate_reputation_gain_unrepresented: true,
                modify_reputation_runtime_unrepresented: true,
            }]
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_records_reward_reputation_db2_lookup_gap_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7029;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_faction_ids[0] = 76;
        quest.reward_faction_values[0] = -4;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session.represented_quest_reward_reputations_like_cpp(),
            &[RepresentedQuestRewardReputationLikeCpp {
                quest_id,
                slot: 0,
                faction_id: 76,
                reward_faction_value: -4,
                reward_faction_override: 0,
                reward_faction_cap_in: 0,
                base_reputation_before_gain: 0,
                reputation_after_low_level_rate_like_cpp: 0,
                reputation_after_reward_rate_like_cpp: 0,
                no_quest_bonus: false,
                no_spillover: false,
                source: RepresentedQuestRewardReputationSourceLikeCpp::Quest,
                faction_store_lookup_unrepresented: true,
                quest_faction_reward_store_lookup_unrepresented: true,
                reputation_reward_rate_lookup_unrepresented: true,
                gray_level_script_hook_unrepresented: true,
                reputation_rank_cap_check_unrepresented: false,
                calculate_reputation_gain_unrepresented: true,
                modify_reputation_runtime_unrepresented: true,
            }]
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_resolves_reward_reputation_db2_value_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7030;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_faction_ids[0] = 76;
        quest.reward_faction_values[0] = -4;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.set_faction_store(Arc::new(FactionStore::from_entries([
            FactionEntry::for_test_like_cpp(76, 5),
        ])));
        session.set_quest_faction_reward_store(Arc::new(QuestFactionRewardStore::from_entries([
            QuestFactionRewardEntry {
                id: 2,
                difficulty: [0, 5, 10, 15, 250, 350, 500, 750, 1000, 1500],
            },
        ])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session.represented_quest_reward_reputations_like_cpp(),
            &[RepresentedQuestRewardReputationLikeCpp {
                quest_id,
                slot: 0,
                faction_id: 76,
                reward_faction_value: -4,
                reward_faction_override: 0,
                reward_faction_cap_in: 0,
                base_reputation_before_gain: 250,
                reputation_after_low_level_rate_like_cpp: 250,
                reputation_after_reward_rate_like_cpp: 250,
                no_quest_bonus: false,
                no_spillover: false,
                source: RepresentedQuestRewardReputationSourceLikeCpp::Quest,
                faction_store_lookup_unrepresented: false,
                quest_faction_reward_store_lookup_unrepresented: false,
                reputation_reward_rate_lookup_unrepresented: true,
                gray_level_script_hook_unrepresented: true,
                reputation_rank_cap_check_unrepresented: false,
                calculate_reputation_gain_unrepresented: true,
                modify_reputation_runtime_unrepresented: false,
            }]
        );
        assert_eq!(
            session
                .reputation_mgr_like_cpp()
                .get_state(5)
                .expect("quest reward faction state")
                .standing,
            250
        );
        let mut pkt = loop {
            let bytes = send_rx
                .try_recv()
                .expect("set faction standing packet from quest reputation");
            let pkt = wow_packet::WorldPacket::from_bytes(&bytes);
            if pkt.server_opcode() == Some(wow_constants::ServerOpcodes::SetFactionStanding) {
                break pkt;
            }
        };
        pkt.skip_opcode();
        assert_eq!(pkt.read_float().expect("achievement bonus"), 0.0);
        assert_eq!(pkt.read_uint32().expect("faction count"), 1);
        assert_eq!(pkt.read_int32().expect("reputation list id"), 5);
        assert_eq!(pkt.read_int32().expect("standing"), 250);
        assert!(!pkt.read_bit().expect("show visual"));
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_skips_missing_reward_reputation_faction_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7031;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_faction_ids[0] = 999_999;
        quest.reward_faction_overrides[0] = 1200;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.set_faction_store(Arc::new(FactionStore::from_entries([
            FactionEntry::for_test_like_cpp(76, 5),
        ])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert!(
            session
                .represented_quest_reward_reputations_like_cpp()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_skips_reward_reputation_at_rank_cap_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7032;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_faction_ids[0] = 76;
        quest.reward_faction_overrides[0] = 1200;
        quest.reward_faction_cap_in[0] = 5;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.set_faction_store(Arc::new(FactionStore::from_entries([
            FactionEntry::for_test_like_cpp(76, 5),
        ])));
        let mut manager = wow_map::MapManager::default();
        insert_player_with_reputation(&mut manager, player_guid, 76, 9000);
        attach_map_manager(&mut session, manager);
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert!(
            session
                .represented_quest_reward_reputations_like_cpp()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_records_reward_reputation_below_rank_cap_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7033;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_faction_ids[0] = 76;
        quest.reward_faction_overrides[0] = 1200;
        quest.reward_faction_cap_in[0] = 6;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.set_faction_store(Arc::new(FactionStore::from_entries([
            FactionEntry::for_test_like_cpp(76, 5),
        ])));
        let mut manager = wow_map::MapManager::default();
        insert_player_with_reputation(&mut manager, player_guid, 76, 9000);
        attach_map_manager(&mut session, manager);
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session.represented_quest_reward_reputations_like_cpp(),
            &[RepresentedQuestRewardReputationLikeCpp {
                quest_id,
                slot: 0,
                faction_id: 76,
                reward_faction_value: 0,
                reward_faction_override: 1200,
                reward_faction_cap_in: 6,
                base_reputation_before_gain: 12,
                reputation_after_low_level_rate_like_cpp: 12,
                reputation_after_reward_rate_like_cpp: 12,
                no_quest_bonus: true,
                no_spillover: false,
                source: RepresentedQuestRewardReputationSourceLikeCpp::Quest,
                faction_store_lookup_unrepresented: false,
                quest_faction_reward_store_lookup_unrepresented: false,
                reputation_reward_rate_lookup_unrepresented: true,
                gray_level_script_hook_unrepresented: true,
                reputation_rank_cap_check_unrepresented: false,
                calculate_reputation_gain_unrepresented: true,
                modify_reputation_runtime_unrepresented: false,
            }]
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_applies_reputation_reward_rate_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7034;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
        quest.reward_faction_ids[0] = 76;
        quest.reward_faction_overrides[0] = 1200;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.set_faction_store(Arc::new(FactionStore::from_entries([
            FactionEntry::for_test_like_cpp(76, 5),
        ])));
        session.set_reputation_reward_rate_store(Arc::new(
            ReputationRewardRateStoreLikeCpp::from_rows_like_cpp(
                [wow_data::reputation::ReputationRewardRateRowLikeCpp {
                    faction_id: 76,
                    rates: ReputationRewardRateEntryLikeCpp {
                        quest_rate: 1.0,
                        quest_daily_rate: 1.5,
                        quest_weekly_rate: 1.0,
                        quest_monthly_rate: 1.0,
                        quest_repeatable_rate: 1.0,
                        creature_rate: 1.0,
                        spell_rate: 1.0,
                    },
                }],
                session.faction_store().unwrap(),
            )
            .0,
        ));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session.represented_quest_reward_reputations_like_cpp(),
            &[RepresentedQuestRewardReputationLikeCpp {
                quest_id,
                slot: 0,
                faction_id: 76,
                reward_faction_value: 0,
                reward_faction_override: 1200,
                reward_faction_cap_in: 0,
                base_reputation_before_gain: 12,
                reputation_after_low_level_rate_like_cpp: 12,
                reputation_after_reward_rate_like_cpp: 18,
                no_quest_bonus: true,
                no_spillover: false,
                source: RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest,
                faction_store_lookup_unrepresented: false,
                quest_faction_reward_store_lookup_unrepresented: false,
                reputation_reward_rate_lookup_unrepresented: false,
                gray_level_script_hook_unrepresented: true,
                reputation_rank_cap_check_unrepresented: false,
                calculate_reputation_gain_unrepresented: true,
                modify_reputation_runtime_unrepresented: false,
            }]
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_applies_low_level_quest_reputation_rate_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7036;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.quest_level = 20;
        quest.reward_faction_ids[0] = 76;
        quest.reward_faction_overrides[0] = 1200;
        session.set_player_level_like_cpp(80);
        session.set_reputation_rates_like_cpp(wow_network::ReputationRatesLikeCpp {
            low_level_quest: 0.5,
            ..wow_network::ReputationRatesLikeCpp::default()
        });
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.set_faction_store(Arc::new(FactionStore::from_entries([
            FactionEntry::for_test_like_cpp(76, 5),
        ])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session.represented_quest_reward_reputations_like_cpp(),
            &[RepresentedQuestRewardReputationLikeCpp {
                quest_id,
                slot: 0,
                faction_id: 76,
                reward_faction_value: 0,
                reward_faction_override: 1200,
                reward_faction_cap_in: 0,
                base_reputation_before_gain: 12,
                reputation_after_low_level_rate_like_cpp: 6,
                reputation_after_reward_rate_like_cpp: 6,
                no_quest_bonus: true,
                no_spillover: false,
                source: RepresentedQuestRewardReputationSourceLikeCpp::Quest,
                faction_store_lookup_unrepresented: false,
                quest_faction_reward_store_lookup_unrepresented: false,
                reputation_reward_rate_lookup_unrepresented: true,
                gray_level_script_hook_unrepresented: true,
                reputation_rank_cap_check_unrepresented: false,
                calculate_reputation_gain_unrepresented: true,
                modify_reputation_runtime_unrepresented: false,
            }]
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_skips_zero_reputation_reward_rate_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7035;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_faction_ids[0] = 76;
        quest.reward_faction_overrides[0] = 1200;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.set_faction_store(Arc::new(FactionStore::from_entries([
            FactionEntry::for_test_like_cpp(76, 5),
        ])));
        session.set_reputation_reward_rate_store(Arc::new(
            ReputationRewardRateStoreLikeCpp::from_rows_like_cpp(
                [wow_data::reputation::ReputationRewardRateRowLikeCpp {
                    faction_id: 76,
                    rates: ReputationRewardRateEntryLikeCpp {
                        quest_rate: 0.0,
                        quest_daily_rate: 1.0,
                        quest_weekly_rate: 1.0,
                        quest_monthly_rate: 1.0,
                        quest_repeatable_rate: 1.0,
                        creature_rate: 1.0,
                        spell_rate: 1.0,
                    },
                }],
                session.faction_store().unwrap(),
            )
            .0,
        ));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert!(
            session
                .represented_quest_reward_reputations_like_cpp()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_records_reward_spell_cast_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7023;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_spell = 12_345;
        quest.reward_display_spell = [22_001, 22_002, 0];
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session.represented_quest_reward_spell_casts_like_cpp(),
            &[RepresentedQuestRewardSpellCastLikeCpp {
                quest_id,
                spell_id: 12_345,
                kind: RepresentedQuestRewardSpellKindLikeCpp::RewardSpell,
                spell_info_lookup_unrepresented: true,
                caster_selection_unrepresented: true,
                cast_spell_runtime_unrepresented: true,
            }]
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_records_display_spells_only_without_reward_spell_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7024;
        let mut quest = quest_template(quest_id);
        quest.flags =
            QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_PLAYER_CAST_COMPLETE_LIKE_CPP;
        quest.reward_display_spell = [22_001, 0, 22_003];
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session.represented_quest_reward_spell_casts_like_cpp(),
            &[
                RepresentedQuestRewardSpellCastLikeCpp {
                    quest_id,
                    spell_id: 22_001,
                    kind: RepresentedQuestRewardSpellKindLikeCpp::RewardDisplaySpell { index: 0 },
                    spell_info_lookup_unrepresented: true,
                    caster_selection_unrepresented: false,
                    cast_spell_runtime_unrepresented: true,
                },
                RepresentedQuestRewardSpellCastLikeCpp {
                    quest_id,
                    spell_id: 22_003,
                    kind: RepresentedQuestRewardSpellKindLikeCpp::RewardDisplaySpell { index: 2 },
                    spell_info_lookup_unrepresented: true,
                    caster_selection_unrepresented: false,
                    cast_spell_runtime_unrepresented: true,
                },
            ]
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_sets_daily_lockout_status_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7015;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert!(session.daily_quests_completed_like_cpp.contains(&quest_id));
        assert!(!session.df_quests_like_cpp.contains(&quest_id));
        assert!(session.last_daily_quest_time_like_cpp > 0);
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_sets_df_lockout_in_daily_table_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7016;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
        quest.special_flags = QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert!(!session.daily_quests_completed_like_cpp.contains(&quest_id));
        assert!(session.df_quests_like_cpp.contains(&quest_id));
        assert!(session.last_daily_quest_time_like_cpp > 0);
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_sets_weekly_and_monthly_lockouts_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let weekly_id = 7017;
        let monthly_id = 7018;
        let mut weekly = quest_template(weekly_id);
        weekly.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | 0x0000_8000;
        let mut monthly = quest_template(monthly_id);
        monthly.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        monthly.special_flags = 0x0000_0010;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([
            weekly, monthly,
        ])));
        for quest_id in [weekly_id, monthly_id] {
            session.player_quests.insert(
                quest_id,
                PlayerQuestStatus {
                    quest_id,
                    status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                    explored: false,
                    accept_time_secs: 0,
                    end_time_secs: 0,
                    objective_counts: Vec::new(),
                    slot: 0,
                },
            );
            session
                .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                    player_guid,
                    quest_id,
                    QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                    0,
                ))
                .await;
        }

        assert!(
            session
                .weekly_quests_completed_like_cpp
                .contains(&weekly_id)
        );
        assert!(
            !session
                .weekly_quests_completed_like_cpp
                .contains(&monthly_id)
        );
        assert!(
            session
                .monthly_quests_completed_like_cpp
                .contains(&monthly_id)
        );
        assert!(
            !session
                .monthly_quests_completed_like_cpp
                .contains(&weekly_id)
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_sets_seasonal_lockout_status_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7019;
        let event_id = 9;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.quest_sort_id = -376;
        quest.event_id_for_quest = event_id;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert!(
            session
                .seasonal_quests_like_cpp
                .get(&event_id)
                .is_some_and(|quests| quests.contains_key(&quest_id))
        );
        assert!(session.seasonal_quest_changed_like_cpp);
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_removes_item_objective_before_rewards_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7015;
        let required_item_id = 19_028;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.objectives.push(QuestObjective {
            id: 1,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: required_item_id as i32,
            amount: 2,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        session.set_player_gold_like_cpp(5);
        install_source_item_template(&mut session, required_item_id, 20, 0);
        insert_direct_inventory_item(&mut session, player_guid, 23, required_item_id, 5, 9911);
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert!(!session.player_quests.contains_key(&quest_id));
        assert!(session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 42);
        let item = session
            .inventory_items_like_cpp()
            .values()
            .find(|item| item.entry_id == required_item_id)
            .expect("partial objective item stack should remain");
        assert_eq!(
            session
                .inventory_item_objects_like_cpp()
                .get(&item.guid)
                .map(|item| item.count()),
            Some(3)
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_removes_currency_objective_before_rewards_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7016;
        let currency_id = 394;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.objectives.push(QuestObjective {
            id: 1,
            quest_id,
            obj_type: QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: currency_id as i32,
            amount: 4,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        session.set_player_gold_like_cpp(5);
        session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
            currency_entry_like_cpp(currency_id),
        ])));
        assert!(
            session
                .add_currency_quest_reward_like_cpp(
                    currency_id,
                    10,
                    CurrencyGainSourceLikeCpp::QuestReward,
                )
                .unwrap()
                .is_some()
        );
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert!(!session.player_quests.contains_key(&quest_id));
        assert!(session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 42);
        assert_eq!(session.player_currency_quantity(currency_id), 6);
        assert_eq!(
            send_rx.try_recv().unwrap(),
            wow_packet::packets::misc::SetCurrency {
                type_id: currency_id as i32,
                quantity: 6,
                flags: 0,
                weekly_quantity: None,
                tracked_quantity: None,
                max_quantity: None,
                total_earned: None,
                suppress_chat_log: false,
                quantity_change: Some(-4),
                quantity_gain_source: None,
                quantity_lost_source: Some(CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP),
                first_craft_operation_id: None,
                next_recharge_time: None,
                recharge_cycle_start_time: None,
                overflown_currency_id: None,
            }
            .to_bytes()
        );
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_accepts_quest_package_primary_everyone_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7005;
        let reward_item_id = 19_019;
        let package_id = 77;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.quest_package_id = package_id;
        session.set_player_gold_like_cpp(5);
        install_test_item_template_with_flags2_like_cpp(&mut session, reward_item_id, 0);
        session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
            QuestPackageItemEntry {
                id: 1,
                package_id: package_id as u16,
                item_id: reward_item_id as i32,
                item_quantity: 1,
                display_type: QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP,
            },
        ])));
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                reward_item_id,
            ))
            .await;

        assert!(!session.player_quests.contains_key(&quest_id));
        assert!(session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 42);
        let reward_item = session
            .inventory_items_like_cpp()
            .values()
            .find(|item| item.entry_id == reward_item_id)
            .expect("primary package reward item should be in direct inventory");
        assert_eq!(
            session
                .inventory_item_objects_like_cpp()
                .get(&reward_item.guid)
                .map(|item| item.count()),
            Some(1)
        );

        let mut saw_item_push = false;
        let mut saw_quest_complete = false;
        while let Ok(bytes) = send_rx.try_recv() {
            let mut packet = WorldPacket::from_bytes(&bytes);
            match packet.read_uint16().unwrap() {
                opcode if opcode == wow_constants::ServerOpcodes::ItemPushResult as u16 => {
                    saw_item_push = true;
                    assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
                    assert_eq!(
                        packet.read_uint8().unwrap(),
                        u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
                    );
                    let slot_in_bag = packet.read_int32().unwrap();
                    assert!(slot_in_bag >= 0);
                    assert_eq!(packet.read_int32().unwrap(), 0);
                    assert_eq!(packet.read_int32().unwrap(), 1);
                    assert_eq!(packet.read_int32().unwrap(), 1);
                }
                opcode
                    if opcode == wow_constants::ServerOpcodes::QuestGiverQuestComplete as u16 =>
                {
                    saw_quest_complete = true;
                }
                _ => {}
            }
        }
        assert!(saw_item_push);
        assert!(saw_quest_complete);
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_accepts_quest_package_fallback_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7006;
        let reward_item_id = 19_020;
        let package_id = 78;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.quest_package_id = package_id;
        session.set_player_gold_like_cpp(5);
        install_test_item_template_with_flags2_like_cpp(&mut session, reward_item_id, 0);
        session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
            QuestPackageItemEntry {
                id: 1,
                package_id: package_id as u16,
                item_id: reward_item_id as i32,
                item_quantity: 1,
                display_type: QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP,
            },
        ])));
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                reward_item_id,
            ))
            .await;

        assert!(!session.player_quests.contains_key(&quest_id));
        assert!(session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 42);
        let reward_item = session
            .inventory_items_like_cpp()
            .values()
            .find(|item| item.entry_id == reward_item_id)
            .expect("fallback package reward item should be in direct inventory");
        assert_eq!(
            session
                .inventory_item_objects_like_cpp()
                .get(&reward_item.guid)
                .map(|item| item.count()),
            Some(1)
        );

        let mut saw_item_push = false;
        let mut saw_quest_complete = false;
        while let Ok(bytes) = send_rx.try_recv() {
            let mut packet = WorldPacket::from_bytes(&bytes);
            match packet.read_uint16().unwrap() {
                opcode if opcode == wow_constants::ServerOpcodes::ItemPushResult as u16 => {
                    saw_item_push = true;
                    assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
                    assert_eq!(
                        packet.read_uint8().unwrap(),
                        u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
                    );
                    let slot_in_bag = packet.read_int32().unwrap();
                    assert!(slot_in_bag >= 0);
                    assert_eq!(packet.read_int32().unwrap(), 0);
                    assert_eq!(packet.read_int32().unwrap(), 1);
                    assert_eq!(packet.read_int32().unwrap(), 1);
                }
                opcode
                    if opcode == wow_constants::ServerOpcodes::QuestGiverQuestComplete as u16 =>
                {
                    saw_quest_complete = true;
                }
                _ => {}
            }
        }
        assert!(saw_item_push);
        assert!(saw_quest_complete);
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_rejects_quest_package_wrong_faction_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7007;
        let reward_item_id = 19_021;
        let package_id = 79;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.quest_package_id = package_id;
        session.set_player_gold_like_cpp(5);
        install_test_item_template_with_flags2_like_cpp(
            &mut session,
            reward_item_id,
            ItemFlags2::FactionHorde as u32,
        );
        session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
            QuestPackageItemEntry {
                id: 1,
                package_id: package_id as u16,
                item_id: reward_item_id as i32,
                item_quantity: 1,
                display_type: QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP,
            },
        ])));
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                reward_item_id,
            ))
            .await;

        assert_eq!(
            session
                .player_quests
                .get(&quest_id)
                .map(|status| status.status),
            Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
        );
        assert!(!session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 5);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_direct_choice_inventory_failure_sends_quest_failed_like_cpp()
    {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7008;
        let reward_item_id = 19_022;
        let limit_category = 44;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.reward_choice_items[0] = (reward_item_id, 1);
        quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
        session.set_player_gold_like_cpp(5);
        install_source_item_template_with_limit_category(
            &mut session,
            reward_item_id,
            20,
            0,
            limit_category as u16,
        );
        install_have_limit_category_like_cpp(&mut session, limit_category, 1);
        insert_direct_inventory_item(&mut session, player_guid, 23, reward_item_id, 1, 9907);
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                reward_item_id,
            ))
            .await;

        assert_eq!(
            session
                .player_quests
                .get(&quest_id)
                .map(|status| status.status),
            Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
        );
        assert!(!session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 5);
        assert_eq!(
            send_rx.try_recv().unwrap(),
            QuestGiverQuestFailed {
                quest_id,
                reason: InventoryResult::ItemMaxLimitCategoryCountExceededIs as u32,
            }
            .to_bytes()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_fixed_reward_inventory_failure_sends_quest_failed_like_cpp()
    {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7009;
        let reward_item_id = 19_023;
        let limit_category = 45;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.reward_items[0] = reward_item_id;
        quest.reward_amounts[0] = 1;
        session.set_player_gold_like_cpp(5);
        install_source_item_template_with_limit_category(
            &mut session,
            reward_item_id,
            20,
            0,
            limit_category as u16,
        );
        install_have_limit_category_like_cpp(&mut session, limit_category, 1);
        insert_direct_inventory_item(&mut session, player_guid, 23, reward_item_id, 1, 9908);
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session
                .player_quests
                .get(&quest_id)
                .map(|status| status.status),
            Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
        );
        assert!(!session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 5);
        assert_eq!(
            send_rx.try_recv().unwrap(),
            QuestGiverQuestFailed {
                quest_id,
                reason: InventoryResult::ItemMaxLimitCategoryCountExceededIs as u32,
            }
            .to_bytes()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_fixed_reward_stores_and_pushes_item_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7012;
        let reward_item_id = 19_026;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.reward_items[0] = reward_item_id;
        quest.reward_amounts[0] = 2;
        session.set_player_gold_like_cpp(5);
        install_source_item_template(&mut session, reward_item_id, 20, 0);
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert!(!session.player_quests.contains_key(&quest_id));
        assert!(session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 42);
        let reward_item = session
            .inventory_items_like_cpp()
            .values()
            .find(|item| item.entry_id == reward_item_id)
            .expect("fixed reward item should be in direct inventory");
        assert_eq!(
            session
                .inventory_item_objects_like_cpp()
                .get(&reward_item.guid)
                .map(|item| item.count()),
            Some(2)
        );

        let mut saw_item_push = false;
        let mut saw_quest_complete = false;
        while let Ok(bytes) = send_rx.try_recv() {
            let mut packet = WorldPacket::from_bytes(&bytes);
            match packet.read_uint16().unwrap() {
                opcode if opcode == wow_constants::ServerOpcodes::ItemPushResult as u16 => {
                    saw_item_push = true;
                    assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
                    assert_eq!(
                        packet.read_uint8().unwrap(),
                        u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
                    );
                    let slot_in_bag = packet.read_int32().unwrap();
                    assert!(slot_in_bag >= 0);
                    assert_eq!(packet.read_int32().unwrap(), 0);
                    assert_eq!(packet.read_int32().unwrap(), 2);
                    assert_eq!(packet.read_int32().unwrap(), 2);
                }
                opcode
                    if opcode == wow_constants::ServerOpcodes::QuestGiverQuestComplete as u16 =>
                {
                    saw_quest_complete = true;
                }
                _ => {}
            }
        }
        assert!(saw_item_push);
        assert!(saw_quest_complete);
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_chosen_item_stores_and_pushes_item_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7013;
        let reward_item_id = 19_027;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.reward_choice_items[0] = (reward_item_id, 3);
        quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
        session.set_player_gold_like_cpp(5);
        install_source_item_template(&mut session, reward_item_id, 20, 0);
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                reward_item_id,
            ))
            .await;

        assert!(!session.player_quests.contains_key(&quest_id));
        assert!(session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 42);
        let reward_item = session
            .inventory_items_like_cpp()
            .values()
            .find(|item| item.entry_id == reward_item_id)
            .expect("chosen reward item should be in direct inventory");
        assert_eq!(
            session
                .inventory_item_objects_like_cpp()
                .get(&reward_item.guid)
                .map(|item| item.count()),
            Some(3)
        );

        let mut saw_item_push = false;
        let mut saw_quest_complete = false;
        while let Ok(bytes) = send_rx.try_recv() {
            let mut packet = WorldPacket::from_bytes(&bytes);
            match packet.read_uint16().unwrap() {
                opcode if opcode == wow_constants::ServerOpcodes::ItemPushResult as u16 => {
                    saw_item_push = true;
                    assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
                    assert_eq!(
                        packet.read_uint8().unwrap(),
                        u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
                    );
                    let slot_in_bag = packet.read_int32().unwrap();
                    assert!(slot_in_bag >= 0);
                    assert_eq!(packet.read_int32().unwrap(), 0);
                    assert_eq!(packet.read_int32().unwrap(), 3);
                    assert_eq!(packet.read_int32().unwrap(), 3);
                }
                opcode
                    if opcode == wow_constants::ServerOpcodes::QuestGiverQuestComplete as u16 =>
                {
                    saw_quest_complete = true;
                }
                _ => {}
            }
        }
        assert!(saw_item_push);
        assert!(saw_quest_complete);
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_package_primary_inventory_failure_sends_equip_error_like_cpp()
     {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7010;
        let reward_item_id = 19_024;
        let package_id = 80;
        let limit_category = 46;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.quest_package_id = package_id;
        session.set_player_gold_like_cpp(5);
        install_source_item_template_with_limit_category(
            &mut session,
            reward_item_id,
            20,
            0,
            limit_category as u16,
        );
        install_have_limit_category_like_cpp(&mut session, limit_category, 1);
        insert_direct_inventory_item(&mut session, player_guid, 23, reward_item_id, 1, 9909);
        session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
            QuestPackageItemEntry {
                id: 1,
                package_id: package_id as u16,
                item_id: reward_item_id as i32,
                item_quantity: 1,
                display_type: QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP,
            },
        ])));
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                reward_item_id,
            ))
            .await;

        assert_eq!(
            session
                .player_quests
                .get(&quest_id)
                .map(|status| status.status),
            Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
        );
        assert!(!session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 5);
        assert_eq!(
            send_rx.try_recv().unwrap(),
            InventoryChangeFailure::error(InventoryResult::ItemMaxLimitCategoryCountExceededIs)
                .with_limit_category(limit_category)
                .to_bytes()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_giver_choose_reward_package_fallback_inventory_failure_sends_equip_error_like_cpp()
     {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 7011;
        let reward_item_id = 19_025;
        let package_id = 81;
        let limit_category = 47;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.quest_package_id = package_id;
        session.set_player_gold_like_cpp(5);
        install_source_item_template_with_limit_category(
            &mut session,
            reward_item_id,
            20,
            0,
            limit_category as u16,
        );
        install_have_limit_category_like_cpp(&mut session, limit_category, 1);
        insert_direct_inventory_item(&mut session, player_guid, 23, reward_item_id, 1, 9910);
        session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
            QuestPackageItemEntry {
                id: 1,
                package_id: package_id as u16,
                item_id: reward_item_id as i32,
                item_quantity: 1,
                display_type: QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP,
            },
        ])));
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                reward_item_id,
            ))
            .await;

        assert_eq!(
            session
                .player_quests
                .get(&quest_id)
                .map(|status| status.status),
            Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
        );
        assert!(!session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 5);
        assert_eq!(
            send_rx.try_recv().unwrap(),
            InventoryChangeFailure::error(InventoryResult::ItemMaxLimitCategoryCountExceededIs)
                .with_limit_category(limit_category)
                .to_bytes()
        );
        assert!(send_rx.try_recv().is_err());
    }

    async fn run_quest_push_result(
        session: &mut WorldSession,
        sender_guid: ObjectGuid,
        quest_id: u32,
        result: u8,
    ) {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&sender_guid);
        pkt.write_uint32(quest_id);
        pkt.write_uint8(result);
        session.handle_quest_push_result(pkt).await;
    }

    async fn run_push_quest_to_party(session: &mut WorldSession, quest_id: u32) {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(quest_id);
        session.handle_push_quest_to_party(pkt).await;
    }

    fn store_with_sharable_quest(id: u32) -> QuestStore {
        let mut quest = quest_template(id);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        QuestStore::from_quests_like_cpp([quest])
    }

    fn store_with_sharable_quest_levels(id: u32, min_level: i32, max_level: u8) -> QuestStore {
        let mut quest = quest_template(id);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        quest.min_level = min_level;
        quest.max_level = max_level;
        QuestStore::from_quests_like_cpp([quest])
    }

    fn store_with_sharable_quest_class_race(
        id: u32,
        allowable_classes: u32,
        allowable_races: u64,
    ) -> QuestStore {
        let mut quest = quest_template(id);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        quest.allowable_classes = allowable_classes;
        quest.allowable_races = allowable_races;
        QuestStore::from_quests_like_cpp([quest])
    }

    fn store_with_sharable_quest_reputation(
        id: u32,
        min_faction: u32,
        min_value: i32,
        max_faction: u32,
        max_value: i32,
    ) -> QuestStore {
        let mut quest = quest_template(id);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        quest.required_min_rep_faction = min_faction;
        quest.required_min_rep_value = min_value;
        quest.required_max_rep_faction = max_faction;
        quest.required_max_rep_value = max_value;
        QuestStore::from_quests_like_cpp([quest])
    }

    fn store_with_sharable_quest_previous(id: u32, prev_quest_id: i32) -> QuestStore {
        let mut quest = quest_template(id);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        quest.prev_quest_id = prev_quest_id;
        QuestStore::from_quests_like_cpp([quest])
    }

    fn store_with_daily_sharable_quests(ids: &[u32]) -> QuestStore {
        let quests = ids.iter().map(|id| {
            let mut quest = quest_template(*id);
            quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
            quest
        });
        QuestStore::from_quests_like_cpp(quests)
    }

    fn store_with_df_sharable_quest(id: u32) -> QuestStore {
        let mut quest = quest_template(id);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        quest.special_flags |= QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP;
        QuestStore::from_quests_like_cpp([quest])
    }

    fn quest_pool_store_with_active_saved(
        quest_store: &QuestStore,
        pool_id: u32,
        member_quest_ids: &[u32],
        active_quest_ids: &[u32],
    ) -> QuestPoolStoreLikeCpp {
        QuestPoolStoreLikeCpp::from_rows_like_cpp(
            quest_store,
            member_quest_ids
                .iter()
                .enumerate()
                .map(|(pool_index, quest_id)| QuestPoolMemberRowLikeCpp {
                    quest_id: *quest_id,
                    pool_id,
                    pool_index: pool_index as u32,
                    num_active: Some(active_quest_ids.len() as u32),
                }),
            active_quest_ids
                .iter()
                .map(|quest_id| QuestPoolSavedActiveRowLikeCpp {
                    pool_id,
                    quest_id: *quest_id,
                }),
        )
    }

    fn recv_world_quest_update_count(send_rx: &flume::Receiver<Vec<u8>>) -> u32 {
        let bytes = send_rx
            .try_recv()
            .expect("world quest update response packet");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            wow_constants::ServerOpcodes::WorldQuestUpdateResponse as u16
        );
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        pkt.read_uint32().unwrap()
    }

    fn recv_push_quest_result_response(
        send_rx: &flume::Receiver<Vec<u8>>,
    ) -> (ObjectGuid, u8, String) {
        let bytes = send_rx
            .try_recv()
            .expect("quest push result response packet");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            wow_constants::ServerOpcodes::QuestPushResult as u16
        );
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        let sender_guid = pkt.read_packed_guid().unwrap();
        let result = pkt.read_uint8().unwrap();
        let title_len = pkt.read_bits(9).unwrap() as usize;
        let quest_title = pkt.read_string(title_len).unwrap();
        assert_eq!(pkt.remaining(), 0);
        assert!(send_rx.try_recv().is_err());
        (sender_guid, result, quest_title)
    }

    fn recv_quest_giver_quest_details_contains_quest_id(
        send_rx: &flume::Receiver<Vec<u8>>,
        quest_id: u32,
    ) {
        let bytes = send_rx
            .try_recv()
            .expect("quest giver quest details packet");
        assert_eq!(
            wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
            Some(wow_constants::ServerOpcodes::QuestGiverQuestDetails)
        );
        assert!(
            bytes
                .windows(4)
                .any(|window| window == quest_id.to_le_bytes())
        );
        assert!(send_rx.try_recv().is_err());
    }

    fn recv_quest_giver_request_items_like_cpp(
        send_rx: &flume::Receiver<Vec<u8>>,
        quest_id: u32,
    ) -> (Vec<(i32, i32, u32)>, bool) {
        let bytes = send_rx
            .try_recv()
            .expect("quest giver request items packet");
        let mut pkt = wow_packet::WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.server_opcode(),
            Some(wow_constants::ServerOpcodes::QuestGiverRequestItems)
        );
        pkt.skip_opcode();
        let _giver_guid = pkt.read_packed_guid().expect("giver guid");
        let giver_creature_id = pkt.read_int32().expect("giver creature id");
        assert_eq!(pkt.read_int32().expect("quest id"), quest_id as i32);
        let _comp_emote_delay = pkt.read_int32().expect("comp emote delay");
        let _comp_emote_type = pkt.read_int32().expect("comp emote type");
        for _ in 0..3 {
            let _ = pkt.read_uint32().expect("quest flags");
        }
        let _suggested_party_members = pkt.read_int32().expect("suggested party members");
        let _money_to_get = pkt.read_int32().expect("money to get");
        let collect_count = pkt.read_int32().expect("collect count");
        let currency_count = pkt.read_int32().expect("currency count");
        let _status_flags = pkt.read_int32().expect("status flags");
        let mut collect = Vec::new();
        for _ in 0..collect_count {
            collect.push((
                pkt.read_int32().expect("collect object id"),
                pkt.read_int32().expect("collect amount"),
                pkt.read_uint32().expect("collect flags"),
            ));
        }
        for _ in 0..currency_count {
            let _currency_id = pkt.read_int32().expect("currency id");
            let _currency_amount = pkt.read_int32().expect("currency amount");
        }
        let auto_launched = pkt.read_bit().expect("auto launched bit");
        assert_eq!(
            pkt.read_int32().expect("repeated giver creature id"),
            giver_creature_id
        );
        assert_eq!(
            pkt.read_uint32()
                .expect("conditional completion text count"),
            0
        );
        assert!(send_rx.try_recv().is_err());
        (collect, auto_launched)
    }

    #[tokio::test]
    async fn quest_giver_request_reward_completes_ready_quest_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = session.player_guid().expect("player guid");
        let quest_id = 9021;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        add_active_quest(&mut session, quest_id);

        session
            .handle_quest_giver_request_reward(quest_giver_request_reward_packet_like_cpp(
                player_guid,
                quest_id,
            ))
            .await;

        assert_eq!(
            session
                .player_quests
                .get(&quest_id)
                .expect("quest should still be active before choose-reward")
                .status,
            QUEST_STATUS_COMPLETE_LIKE_CPP
        );
        assert_complete_status_update_like_cpp(&session, quest_id, false);
        recv_quest_giver_offer_reward_contains_quest_id(&send_rx, quest_id);
    }

    fn recv_quest_giver_offer_reward_contains_quest_id(
        send_rx: &flume::Receiver<Vec<u8>>,
        quest_id: u32,
    ) {
        let bytes = send_rx.try_recv().expect("quest giver offer reward packet");
        assert_eq!(
            wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
            Some(wow_constants::ServerOpcodes::QuestGiverOfferRewardMessage)
        );
        assert!(
            bytes
                .windows(4)
                .any(|window| window == quest_id.to_le_bytes())
        );
        assert!(send_rx.try_recv().is_err());
    }

    fn assert_success_command_queued_like_cpp(
        sender_rx: &flume::Receiver<Vec<u8>>,
        receiver_rx: &flume::Receiver<Vec<u8>>,
        receiver_session: &mut WorldSession,
        receiver_guid: ObjectGuid,
        sender_guid: ObjectGuid,
        quest_id: u32,
    ) {
        assert_eq!(
            recv_push_quest_result_response(sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_SUCCESS_LIKE_CPP,
                String::new()
            )
        );
        assert!(receiver_rx.try_recv().is_err());
        let commands = receiver_session.drain_session_commands();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            SessionCommand::SetQuestSharingInfoAndSendDetails(command) => {
                assert_eq!(command.sender_guid, sender_guid);
                assert_eq!(command.quest.id, quest_id);
            }
            other => panic!("unexpected session command: {other:?}"),
        }
    }

    fn recv_status(send_rx: &flume::Receiver<Vec<u8>>) -> (ObjectGuid, u64) {
        let bytes = send_rx.try_recv().expect("quest giver status packet");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            wow_constants::ServerOpcodes::QuestGiverStatus as u16
        );
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        let guid = pkt.read_packed_guid().unwrap();
        let status = pkt.read_uint64().unwrap();
        (guid, status)
    }

    fn recv_status_multiple(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<(ObjectGuid, u64)> {
        let bytes = send_rx
            .try_recv()
            .expect("quest giver status multiple packet");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            wow_constants::ServerOpcodes::QuestGiverStatusMultiple as u16
        );
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        let count = pkt.read_int32().unwrap();
        assert!(count >= 0);
        let mut statuses = Vec::new();
        for _ in 0..count {
            statuses.push((pkt.read_packed_guid().unwrap(), pkt.read_uint64().unwrap()));
        }
        statuses
    }

    fn mark_visible(session: &mut WorldSession, guid: ObjectGuid) {
        session.client_visible_guids_like_cpp.insert(guid);
    }

    fn mark_visible_gameobject_questgiver(session: &mut WorldSession, guid: ObjectGuid) {
        let mut state = crate::session::RepresentedGameObjectUseState::default();
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8);
        session
            .represented_gameobject_use_states
            .insert(guid, state);
        mark_visible(session, guid);
    }

    fn assert_confirm_accept_outcome(
        session: &WorldSession,
        receiver_guid: Option<ObjectGuid>,
        sender_guid: ObjectGuid,
        quest_id: u32,
        raw_quest_id: i32,
        reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    ) {
        let success_boundary = matches!(
            reason,
            RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::AddQuestRuntimeUnrepresented
        );
        assert_eq!(
            session.represented_quest_confirm_accepts_like_cpp(),
            &[RepresentedQuestConfirmAcceptLikeCpp {
                receiver_guid,
                sender_guid_before_clear: sender_guid,
                quest_id,
                raw_quest_id,
                reason,
                object_accessor_unrepresented: true,
                party_runtime_unrepresented: true,
                can_add_source_item_unrepresented: false,
                can_add_source_item_result: None,
                add_quest_runtime_unrepresented: success_boundary,
                source_spell_unrepresented: false,
                represented_source_spell_id: None,
                represented_source_spell_self_casts: 0,
            }]
        );
    }

    fn assert_complete_status_update_like_cpp(
        session: &WorldSession,
        quest_id: u32,
        tracking_event_auto_reward_unrepresented: bool,
    ) {
        assert_eq!(
            session.represented_quest_complete_status_updates_like_cpp(),
            &[RepresentedQuestCompleteStatusUpdateLikeCpp {
                quest_id,
                old_status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                new_status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                send_quest_update_called: true,
                quest_slot_state_complete_represented: true,
                quest_slot_state_live_update_unrepresented: true,
                visible_gameobjects_or_spellclicks_refresh_unrepresented: true,
                spell_area_runtime_unrepresented: true,
                tracking_event_auto_reward_unrepresented,
                quest_tracker_complete_time_unrepresented: true,
                script_status_change_unrepresented: true,
            }]
        );
    }

    fn install_confirm_accept_sender_snapshot(
        session: &mut WorldSession,
        sender_guid: ObjectGuid,
        quest_id: u32,
        same_group: bool,
        sender_active_status: Option<u8>,
    ) -> (WorldSession, flume::Receiver<Vec<u8>>) {
        let player_registry = Arc::new(PlayerRegistry::default());
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_loaded_player_name_like_cpp("Receiver".to_string());
        session.register_in_player_registry();

        let (mut sender_session, sender_rx) = make_session();
        sender_session.set_player_guid(Some(sender_guid));
        sender_session.set_loaded_player_name_like_cpp("Sender".to_string());
        sender_session.set_player_registry(player_registry);
        if let Some(status) = sender_active_status {
            add_active_quest_in_slot_with_status(&mut sender_session, quest_id, 0, status);
        }
        sender_session.register_in_player_registry();
        sender_session.sync_player_registry_state_like_cpp();

        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(sender_guid);
        if same_group {
            if let Some(receiver_guid) = session.player_guid() {
                group.add_member(receiver_guid);
            }
        }
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        (sender_session, sender_rx)
    }

    #[tokio::test]
    async fn quest_confirm_accept_short_packet_does_not_clear_pending_state_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 81);
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7001);

        session
            .handle_quest_confirm_accept(WorldPacket::from_bytes(&[0x59, 0x1B, 0x00]))
            .await;

        assert_eq!(
            session.represented_pending_quest_sharing_like_cpp(),
            Some(crate::session::RepresentedPendingQuestSharingLikeCpp {
                sender_guid,
                quest_id: 7001,
            })
        );
        assert!(
            session
                .represented_quest_confirm_accepts_like_cpp()
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_no_pending_valid_packet_is_noop_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_quest_store(Arc::new(store_with_quests(&[7002])));

        run_quest_confirm_accept(&mut session, 7002).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert!(
            session
                .represented_quest_confirm_accepts_like_cpp()
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_mismatch_preserves_pending_state_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 82);
        session.set_quest_store(Arc::new(store_with_quests(&[7003])));
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7003);

        run_quest_confirm_accept(&mut session, 7004).await;

        assert_eq!(
            session.represented_pending_quest_sharing_like_cpp(),
            Some(crate::session::RepresentedPendingQuestSharingLikeCpp {
                sender_guid,
                quest_id: 7003,
            })
        );
        assert!(
            session
                .represented_quest_confirm_accepts_like_cpp()
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_match_missing_template_clears_without_evidence_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 83);
        session.set_quest_store(Arc::new(store_with_quests(&[7005])));
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7006);

        run_quest_confirm_accept(&mut session, 7006).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert!(
            session
                .represented_quest_confirm_accepts_like_cpp()
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_match_template_records_original_player_missing_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 84);
        let receiver_guid = ObjectGuid::create_player(1, 42);
        session.set_quest_store(Arc::new(store_with_quests(&[7007])));
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7007);

        run_quest_confirm_accept(&mut session, 7007).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert_confirm_accept_outcome(
            &session,
            Some(receiver_guid),
            sender_guid,
            7007,
            7007,
            RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerMissing,
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_negative_raw_id_compares_as_u32_bit_pattern_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 85);
        let quest_id = u32::MAX;
        session.set_quest_store(Arc::new(store_with_quests(&[quest_id])));
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);

        run_quest_confirm_accept(&mut session, -1).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert_confirm_accept_outcome(
            &session,
            Some(ObjectGuid::create_player(1, 42)),
            sender_guid,
            quest_id,
            -1,
            RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerMissing,
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_sender_exists_not_same_group_records_not_in_same_raid_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 86);
        session.set_quest_store(Arc::new(store_with_quests(&[7008])));
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7008);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            7008,
            false,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, 7008).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert_confirm_accept_outcome(
            &session,
            Some(ObjectGuid::create_player(1, 42)),
            sender_guid,
            7008,
            7008,
            RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::NotInSameRaid,
        );
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_same_group_sender_not_active_records_original_not_active_like_cpp()
     {
        let (mut session, send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 87);
        session.set_quest_store(Arc::new(store_with_quests(&[7009])));
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7009);
        let (_sender_session, sender_rx) =
            install_confirm_accept_sender_snapshot(&mut session, sender_guid, 7009, true, None);

        run_quest_confirm_accept(&mut session, 7009).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert_confirm_accept_outcome(
            &session,
            Some(ObjectGuid::create_player(1, 42)),
            sender_guid,
            7009,
            7009,
            RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerNotActiveQuest,
        );
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_same_group_sender_active_can_take_failed_records_receiver_gate_like_cpp()
     {
        let (mut session, send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 88);
        let quest_id = 7010;
        session.set_quest_store(Arc::new(store_with_quests(&[quest_id])));
        session.rewarded_quests.insert(quest_id);
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert_confirm_accept_outcome(
            &session,
            Some(ObjectGuid::create_player(1, 42)),
            sender_guid,
            quest_id,
            quest_id as i32,
            RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanTakeQuestFailed,
        );
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_can_take_ok_log_full_records_can_add_log_full_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 89);
        let quest_id = 7011;
        session.set_quest_store(Arc::new(store_with_quests(&[quest_id])));
        for slot in 0..MAX_QUEST_LOG_SIZE_LIKE_CPP {
            add_active_quest_in_slot_with_status(
                &mut session,
                80_000 + u32::from(slot),
                slot,
                QUEST_STATUS_COMPLETE_LIKE_CPP,
            );
        }
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_COMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert_confirm_accept_outcome(
            &session,
            Some(ObjectGuid::create_player(1, 42)),
            sender_guid,
            quest_id,
            quest_id as i32,
            RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestLogFull,
        );
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_no_source_side_effects_adds_local_quest_state_like_cpp() {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 90);
        let quest_id = 7012;
        session.set_quest_store(Arc::new(store_with_sharable_timed_quest_objectives(
            quest_id, 3, 600,
        )));
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        let status = session
            .player_quests
            .get(&quest_id)
            .expect("receiver quest log should receive bounded local AddQuest state");
        assert_eq!(status.quest_id, quest_id);
        assert_eq!(status.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
        assert!(!status.explored);
        assert!(status.accept_time_secs > 0);
        assert_eq!(status.end_time_secs, status.accept_time_secs + 600);
        assert_eq!(status.objective_counts, vec![0, 0, 0]);
        assert_eq!(status.slot, 0);
        let registry = session.player_registry().expect("test installs registry");
        let snapshot = registry
            .get(&receiver_guid)
            .expect("receiver snapshot should sync after quest insertion");
        assert_eq!(
            snapshot.active_quest_statuses.get(&quest_id),
            Some(&QUEST_STATUS_INCOMPLETE_LIKE_CPP)
        );
        assert_eq!(
            snapshot.active_quest_objective_counts.get(&quest_id),
            Some(&vec![0, 0, 0])
        );
        assert_confirm_accept_outcome(
            &session,
            Some(receiver_guid),
            sender_guid,
            quest_id,
            quest_id as i32,
            RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
        );
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_first_free_slot_skips_occupied_slot_like_cpp() {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 190);
        let occupied_quest_id = 8000;
        let quest_id = 70120;
        session.set_quest_store(Arc::new(store_with_sharable_quest_objectives(quest_id, 1)));
        add_active_quest_in_slot_with_status(
            &mut session,
            occupied_quest_id,
            0,
            QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        );
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        let occupied_status = session
            .player_quests
            .get(&occupied_quest_id)
            .expect("pre-existing quest should remain in slot 0");
        assert_eq!(occupied_status.slot, 0);
        let status = session
            .player_quests
            .get(&quest_id)
            .expect("accepted quest should be inserted into first free slot");
        assert_eq!(status.slot, 1);
        assert_eq!(status.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
        let registry = session.player_registry().expect("test installs registry");
        let snapshot = registry
            .get(&receiver_guid)
            .expect("receiver snapshot should sync after quest insertion");
        assert_eq!(
            snapshot.active_quest_statuses.get(&quest_id),
            Some(&QUEST_STATUS_INCOMPLETE_LIKE_CPP)
        );
        assert_confirm_accept_outcome(
            &session,
            Some(receiver_guid),
            sender_guid,
            quest_id,
            quest_id as i32,
            RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
        );
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_spell_records_two_self_casts_like_cpp() {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 191);
        let quest_id = 70121;
        let mut quest = quest_template_with_objective_count(quest_id, 2);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        quest.source_spell_id = 12_345;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        let status = session
            .player_quests
            .get(&quest_id)
            .expect("source-spell-only quest should still insert represented local AddQuest state");
        assert_eq!(status.quest_id, quest_id);
        assert_eq!(status.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
        assert!(!status.explored);
        assert_eq!(status.objective_counts, vec![0, 0]);
        assert_eq!(status.slot, 0);
        let registry = session.player_registry().expect("test installs registry");
        let snapshot = registry
            .get(&receiver_guid)
            .expect("receiver snapshot should sync after source-spell quest insertion");
        assert_eq!(
            snapshot.active_quest_statuses.get(&quest_id),
            Some(&QUEST_STATUS_INCOMPLETE_LIKE_CPP)
        );
        assert_eq!(
            session.represented_quest_confirm_accepts_like_cpp(),
            &[RepresentedQuestConfirmAcceptLikeCpp {
                receiver_guid: Some(receiver_guid),
                sender_guid_before_clear: sender_guid,
                quest_id,
                raw_quest_id: quest_id as i32,
                reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
                object_accessor_unrepresented: true,
                party_runtime_unrepresented: true,
                can_add_source_item_unrepresented: false,
                can_add_source_item_result: None,
                add_quest_runtime_unrepresented: false,
                source_spell_unrepresented: false,
                represented_source_spell_id: Some(12_345),
                represented_source_spell_self_casts: 2,
            }]
        );
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_start_quest_no_grant_adds_local_state_like_cpp() {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 90);
        let quest_id = 7012;
        let source_item_id = 9000;
        let source_spell_id = 12_344;
        session.set_quest_store(Arc::new(store_with_source_item_quest(
            quest_id,
            source_item_id,
            2,
            source_spell_id,
        )));
        install_source_item_template_with_start_quest(
            &mut session,
            source_item_id,
            20,
            0,
            quest_id as i32,
        );
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert!(session.player_quests.contains_key(&quest_id));
        assert_eq!(
            session
                .represented_inventory_item_counts_like_cpp()
                .get(&source_item_id)
                .copied()
                .unwrap_or(0),
            0
        );
        assert_eq!(
            session.represented_quest_confirm_accepts_like_cpp(),
            &[RepresentedQuestConfirmAcceptLikeCpp {
                receiver_guid: Some(receiver_guid),
                sender_guid_before_clear: sender_guid,
                quest_id,
                raw_quest_id: quest_id as i32,
                reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemStartQuestNoGrant,
                object_accessor_unrepresented: true,
                party_runtime_unrepresented: true,
                can_add_source_item_unrepresented: false,
                can_add_source_item_result: Some(InventoryResult::Ok),
                add_quest_runtime_unrepresented: false,
                source_spell_unrepresented: false,
                represented_source_spell_id: Some(source_spell_id),
                represented_source_spell_self_casts: 2,
            }]
        );
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_with_space_stores_and_pushes_item_like_cpp() {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 91);
        let quest_id = 7013;
        let source_item_id = 9001;
        let source_spell_id = 12_346;
        let quest_log_item_id = 9101;
        let mut quest =
            quest_template_with_source_item(quest_id, source_item_id, 2, source_spell_id);
        quest.objectives.push(QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: quest_log_item_id as i32,
            amount: 2,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        install_source_item_template(&mut session, source_item_id, 20, 0);
        session.cache_item_template_addon_quest_log_item_id_like_cpp(
            source_item_id,
            quest_log_item_id,
        );
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert_eq!(
            session
                .player_quests
                .get(&quest_id)
                .expect("source-item quest should still add local quest state")
                .status,
            QUEST_STATUS_COMPLETE_LIKE_CPP
        );
        assert_eq!(
            session
                .player_quests
                .get(&quest_id)
                .expect("source-item quest should still add local quest state")
                .objective_counts,
            vec![2]
        );
        let stored_source_item_count: u32 = session
            .inventory_items_like_cpp()
            .values()
            .filter(|item| item.entry_id == source_item_id)
            .filter_map(|item| session.inventory_item_objects_like_cpp().get(&item.guid))
            .map(|item| item.count())
            .sum();
        let stored_source_item_slot = session
            .inventory_items_like_cpp()
            .iter()
            .find_map(|(&slot, item)| (item.entry_id == source_item_id).then_some(slot))
            .expect("source item should have a direct inventory slot");
        assert_eq!(stored_source_item_count, 2);
        assert_eq!(
            session.represented_quest_confirm_accepts_like_cpp(),
            &[RepresentedQuestConfirmAcceptLikeCpp {
                receiver_guid: Some(receiver_guid),
                sender_guid_before_clear: sender_guid,
                quest_id,
                raw_quest_id: quest_id as i32,
                reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemStoredNewItem,
                object_accessor_unrepresented: true,
                party_runtime_unrepresented: true,
                can_add_source_item_unrepresented: false,
                can_add_source_item_result: Some(InventoryResult::Ok),
                add_quest_runtime_unrepresented: false,
                source_spell_unrepresented: false,
                represented_source_spell_id: Some(source_spell_id),
                represented_source_spell_self_casts: 2,
            }]
        );
        assert_complete_status_update_like_cpp(&session, quest_id, false);
        let mut sent = Vec::new();
        while let Ok(packet) = send_rx.try_recv() {
            sent.push(packet);
        }
        assert!(
            sent.len() >= 3,
            "StoreNewItem(update=true) should create item/update player and SendNewItem"
        );
        let mut saw_item_push = false;
        for bytes in &sent {
            let mut packet = WorldPacket::from_bytes(bytes);
            if packet.read_uint16().unwrap() != wow_constants::ServerOpcodes::ItemPushResult as u16
            {
                continue;
            }
            saw_item_push = true;
            assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
            assert_eq!(
                packet.read_uint8().unwrap(),
                u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
            );
            assert_eq!(
                packet.read_int32().unwrap(),
                i32::from(stored_source_item_slot)
            );
            assert_eq!(packet.read_int32().unwrap(), quest_log_item_id as i32);
            assert_eq!(packet.read_int32().unwrap(), 2);
            assert_eq!(packet.read_int32().unwrap(), 2);
        }
        assert!(
            saw_item_push,
            "source item grant should send ItemPushResult"
        );
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_full_backpack_stores_in_represented_bag_like_cpp() {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 98);
        let quest_id = 7020;
        let source_item_id = 9007;
        let bag_item_id = 9107;
        let filler_item_id = 9108;
        let bag_guid = ObjectGuid::create_item(1, 9_107);

        session.set_quest_store(Arc::new(store_with_source_item_quest(
            quest_id,
            source_item_id,
            3,
            0,
        )));
        session.set_item_store(Arc::new(ItemStore::from_records([
            ItemRecord {
                id: source_item_id,
                class_id: ItemClass::Consumable as u8,
                subclass_id: 0,
                material: 0,
                inventory_type: InventoryType::NonEquip as i8,
                sheathe_type: 0,
                random_select: 0,
                random_suffix_group_id: 0,
            },
            ItemRecord {
                id: bag_item_id,
                class_id: ItemClass::Container as u8,
                subclass_id: 0,
                material: 0,
                inventory_type: InventoryType::Bag as i8,
                sheathe_type: 0,
                random_select: 0,
                random_suffix_group_id: 0,
            },
            ItemRecord {
                id: filler_item_id,
                class_id: ItemClass::Consumable as u8,
                subclass_id: 0,
                material: 0,
                inventory_type: InventoryType::NonEquip as i8,
                sheathe_type: 0,
                random_select: 0,
                random_suffix_group_id: 0,
            },
        ])));
        let sparse = |inventory_type: InventoryType, stackable: i32, container_slots: u8| {
            ItemSparseTemplateEntry {
                flags: [0, 0, 0, 0],
                bag_family: 0,
                start_quest_id: 0,
                stackable,
                max_count: 0,
                lock_id: 0,
                required_reputation_rank: 0,
                sell_price: 0,
                buy_price: 0,
                vendor_stack_count: 1,
                price_variance: 1.0,
                price_random_value: 1.0,
                max_durability: 0,
                limit_category: 0,
                instance_bound: 0,
                zone_bound: [0, 0],
                required_reputation_faction: 0,
                allowable_class: -1,
                required_expansion: 0,
                bonding: ItemBondingType::None as u8,
                container_slots,
                inventory_type: inventory_type as i8,
            }
        };
        session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
            (source_item_id, sparse(InventoryType::NonEquip, 20, 0)),
            (bag_item_id, sparse(InventoryType::Bag, 1, 4)),
            (filler_item_id, sparse(InventoryType::NonEquip, 1, 0)),
        ])));

        session.insert_inventory_item_like_cpp(
            wow_entities::INVENTORY_SLOT_BAG_START,
            InventoryItem {
                guid: bag_guid,
                entry_id: bag_item_id,
                db_guid: 9_107,
                inventory_type: Some(InventoryType::Bag as u8),
            },
        );
        let bag_item = session.make_inventory_item_object(
            bag_guid,
            bag_item_id,
            receiver_guid,
            1,
            0,
            ItemContext::None,
            wow_entities::INVENTORY_SLOT_BAG_START,
        );
        session.insert_inventory_item_object(bag_item);
        for slot_offset in 0..wow_entities::INVENTORY_DEFAULT_SIZE {
            insert_direct_inventory_item(
                &mut session,
                receiver_guid,
                wow_entities::INVENTORY_SLOT_ITEM_START + slot_offset,
                filler_item_id,
                1,
                9_200 + u64::from(slot_offset),
            );
        }

        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert!(session.player_quests.contains_key(&quest_id));
        assert!(
            session
                .inventory_items_like_cpp()
                .values()
                .all(|item| item.entry_id != source_item_id)
        );
        let child = session
            .inventory_item_objects_like_cpp()
            .values()
            .find(|item| item.object().entry() == source_item_id)
            .expect("source item should be created inside represented bag");
        assert_eq!(child.container_guid(), bag_guid);
        assert_eq!(child.bag_slot(), wow_entities::INVENTORY_SLOT_BAG_START);
        assert_eq!(child.slot(), 0);
        assert_eq!(child.count(), 3);
        assert_eq!(
            session
                .represented_inventory_item_counts_like_cpp()
                .get(&source_item_id)
                .copied(),
            Some(3)
        );

        let mut saw_item_push = false;
        while let Ok(bytes) = send_rx.try_recv() {
            let mut packet = WorldPacket::from_bytes(&bytes);
            if packet.read_uint16().unwrap() != wow_constants::ServerOpcodes::ItemPushResult as u16
            {
                continue;
            }
            saw_item_push = true;
            assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
            assert_eq!(
                packet.read_uint8().unwrap(),
                wow_entities::INVENTORY_SLOT_BAG_START
            );
            assert_eq!(packet.read_int32().unwrap(), 0);
            assert_eq!(packet.read_int32().unwrap(), 0);
            assert_eq!(packet.read_int32().unwrap(), 3);
            assert_eq!(packet.read_int32().unwrap(), 3);
        }
        assert!(saw_item_push);
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_merges_existing_stack_inside_represented_bag_like_cpp()
     {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 99);
        let quest_id = 7021;
        let source_item_id = 9008;
        let bag_item_id = 9109;
        let filler_item_id = 9110;
        let bag_guid = ObjectGuid::create_item(1, 9_109);
        let child_guid = ObjectGuid::create_item(1, 9_110);

        session.set_quest_store(Arc::new(store_with_source_item_quest(
            quest_id,
            source_item_id,
            2,
            0,
        )));
        session.set_item_store(Arc::new(ItemStore::from_records([
            ItemRecord {
                id: source_item_id,
                class_id: ItemClass::Consumable as u8,
                subclass_id: 0,
                material: 0,
                inventory_type: InventoryType::NonEquip as i8,
                sheathe_type: 0,
                random_select: 0,
                random_suffix_group_id: 0,
            },
            ItemRecord {
                id: bag_item_id,
                class_id: ItemClass::Container as u8,
                subclass_id: 0,
                material: 0,
                inventory_type: InventoryType::Bag as i8,
                sheathe_type: 0,
                random_select: 0,
                random_suffix_group_id: 0,
            },
            ItemRecord {
                id: filler_item_id,
                class_id: ItemClass::Consumable as u8,
                subclass_id: 0,
                material: 0,
                inventory_type: InventoryType::NonEquip as i8,
                sheathe_type: 0,
                random_select: 0,
                random_suffix_group_id: 0,
            },
        ])));
        let sparse = |inventory_type: InventoryType, stackable: i32, container_slots: u8| {
            ItemSparseTemplateEntry {
                flags: [0, 0, 0, 0],
                bag_family: 0,
                start_quest_id: 0,
                stackable,
                max_count: 0,
                lock_id: 0,
                required_reputation_rank: 0,
                sell_price: 0,
                buy_price: 0,
                vendor_stack_count: 1,
                price_variance: 1.0,
                price_random_value: 1.0,
                max_durability: 0,
                limit_category: 0,
                instance_bound: 0,
                zone_bound: [0, 0],
                required_reputation_faction: 0,
                allowable_class: -1,
                required_expansion: 0,
                bonding: ItemBondingType::None as u8,
                container_slots,
                inventory_type: inventory_type as i8,
            }
        };
        session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
            (source_item_id, sparse(InventoryType::NonEquip, 20, 0)),
            (bag_item_id, sparse(InventoryType::Bag, 1, 4)),
            (filler_item_id, sparse(InventoryType::NonEquip, 1, 0)),
        ])));

        session.insert_inventory_item_like_cpp(
            wow_entities::INVENTORY_SLOT_BAG_START,
            InventoryItem {
                guid: bag_guid,
                entry_id: bag_item_id,
                db_guid: 9_109,
                inventory_type: Some(InventoryType::Bag as u8),
            },
        );
        let bag_item = session.make_inventory_item_object(
            bag_guid,
            bag_item_id,
            receiver_guid,
            1,
            0,
            ItemContext::None,
            wow_entities::INVENTORY_SLOT_BAG_START,
        );
        session.insert_inventory_item_object(bag_item);
        let mut child = session.make_inventory_item_object(
            child_guid,
            source_item_id,
            receiver_guid,
            18,
            0,
            ItemContext::None,
            0,
        );
        child.set_container_guid_and_slot(bag_guid, wow_entities::INVENTORY_SLOT_BAG_START);
        session.insert_inventory_item_object(child);
        for slot_offset in 0..wow_entities::INVENTORY_DEFAULT_SIZE {
            insert_direct_inventory_item(
                &mut session,
                receiver_guid,
                wow_entities::INVENTORY_SLOT_ITEM_START + slot_offset,
                filler_item_id,
                1,
                9_300 + u64::from(slot_offset),
            );
        }

        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        let child = session
            .inventory_item_objects_like_cpp()
            .get(&child_guid)
            .expect("existing represented bag stack should remain");
        assert_eq!(child.count(), 20);
        assert_eq!(child.container_guid(), bag_guid);
        assert_eq!(child.bag_slot(), wow_entities::INVENTORY_SLOT_BAG_START);
        assert_eq!(child.slot(), 0);
        assert_eq!(
            session
                .represented_inventory_item_counts_like_cpp()
                .get(&source_item_id)
                .copied(),
            Some(20)
        );

        let mut saw_item_push = false;
        while let Ok(bytes) = send_rx.try_recv() {
            let mut packet = WorldPacket::from_bytes(&bytes);
            if packet.read_uint16().unwrap() != wow_constants::ServerOpcodes::ItemPushResult as u16
            {
                continue;
            }
            saw_item_push = true;
            assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
            assert_eq!(
                packet.read_uint8().unwrap(),
                wow_entities::INVENTORY_SLOT_BAG_START
            );
            assert_eq!(packet.read_int32().unwrap(), -1);
            assert_eq!(packet.read_int32().unwrap(), 0);
            assert_eq!(packet.read_int32().unwrap(), 2);
            assert_eq!(packet.read_int32().unwrap(), 20);
        }
        assert!(saw_item_push);
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_binds_on_acquire_like_cpp_store_item() {
        let (mut session, _send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 203);
        let quest_id = 7122;
        let source_item_id = 9210;
        let quest = quest_template_with_source_item(quest_id, source_item_id, 1, 0);
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        install_source_item_template_with_start_quest_limit_category_flags3_and_bonding(
            &mut session,
            source_item_id,
            20,
            0,
            0,
            0,
            0,
            ItemBondingType::OnAcquire,
        );
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        let stored = session
            .inventory_items_like_cpp()
            .values()
            .find(|item| item.entry_id == source_item_id)
            .and_then(|item| session.inventory_item_objects_like_cpp().get(&item.guid))
            .expect("source item should be stored as runtime item object");
        assert_eq!(stored.bonding(), ItemBondingType::OnAcquire);
        assert!(stored.is_soul_bound());
        assert_eq!(
            stored.item_flags_bits() & ItemFieldFlags::SOULBOUND.bits(),
            ItemFieldFlags::SOULBOUND.bits()
        );
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_bound_objective_updates_quest_without_creating_item_like_cpp()
     {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 191);
        let quest_id = 7113;
        let source_item_id = 9201;
        let source_spell_id = 12_347;
        let quest_log_item_id = 9301;
        let mut quest =
            quest_template_with_source_item(quest_id, source_item_id, 2, source_spell_id);
        quest.objectives.push(QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: quest_log_item_id as i32,
            amount: 2,
            flags: 0,
            flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        install_source_item_template(&mut session, source_item_id, 20, 0);
        session.cache_item_template_addon_quest_log_item_id_like_cpp(
            source_item_id,
            quest_log_item_id,
        );
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        let status = session
            .player_quests
            .get(&quest_id)
            .expect("bound source-item quest should still add local quest state");
        assert_eq!(status.status, QUEST_STATUS_COMPLETE_LIKE_CPP);
        assert_eq!(status.objective_counts, vec![2]);
        let stored_source_item_count: u32 = session
            .inventory_items_like_cpp()
            .values()
            .filter(|item| item.entry_id == source_item_id)
            .filter_map(|item| session.inventory_item_objects_like_cpp().get(&item.guid))
            .map(|item| item.count())
            .sum();
        assert_eq!(stored_source_item_count, 0);
        assert_eq!(
            session.represented_quest_confirm_accepts_like_cpp(),
            &[RepresentedQuestConfirmAcceptLikeCpp {
                receiver_guid: Some(receiver_guid),
                sender_guid_before_clear: sender_guid,
                quest_id,
                raw_quest_id: quest_id as i32,
                reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemBoundObjectiveNoGrant,
                object_accessor_unrepresented: true,
                party_runtime_unrepresented: true,
                can_add_source_item_unrepresented: false,
                can_add_source_item_result: Some(InventoryResult::Ok),
                add_quest_runtime_unrepresented: false,
                source_spell_unrepresented: false,
                represented_source_spell_id: Some(source_spell_id),
                represented_source_spell_self_casts: 2,
            }]
        );
        assert_complete_status_update_like_cpp(&session, quest_id, false);

        let sent = send_rx.try_recv().expect("bound item update packet");
        assert!(send_rx.try_recv().is_err());
        let mut packet = WorldPacket::from_bytes(&sent);
        assert_eq!(
            packet.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::ItemPushResult as u16
        );
        assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
        assert_eq!(
            packet.read_uint8().unwrap(),
            u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
        );
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), quest_log_item_id as i32);
        assert_eq!(packet.read_int32().unwrap(), 2);
        assert_eq!(packet.read_int32().unwrap(), 2);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_uint32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert!(!packet.read_bit().unwrap());
        assert!(!packet.read_bit().unwrap());
        assert_eq!(packet.read_bits(3).unwrap(), 3);
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_tracking_event_source_item_objective_auto_rewards_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 194);
        let quest_id = 7119;
        let source_item_id = 9204;
        let mut quest = quest_template_with_source_item(quest_id, source_item_id, 1, 0);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP | QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP;
        quest.objectives.push(QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: source_item_id as i32,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        install_source_item_template(&mut session, source_item_id, 20, 0);
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert!(!session.player_quests.contains_key(&quest_id));
        assert!(session.rewarded_quests.contains(&quest_id));
        assert_complete_status_update_like_cpp(&session, quest_id, false);

        let mut opcodes = Vec::new();
        while let Ok(bytes) = send_rx.try_recv() {
            if let Some(opcode) = wow_packet::WorldPacket::from_bytes(&bytes).server_opcode() {
                opcodes.push(opcode);
            }
        }
        assert!(opcodes.contains(&wow_constants::ServerOpcodes::QuestGiverQuestComplete));
        assert!(opcodes.contains(&wow_constants::ServerOpcodes::QuestUpdateComplete));
        assert!(opcodes.contains(&wow_constants::ServerOpcodes::ItemPushResult));
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_bound_objective_broadcasts_to_group_like_cpp() {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 193);
        let other_guid = ObjectGuid::create_player(1, 194);
        let quest_id = 7115;
        let source_item_id = 9203;
        let quest_log_item_id = 9303;
        let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
        quest.objectives.push(QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: quest_log_item_id as i32,
            amount: 2,
            flags: 0,
            flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        install_source_item_template(&mut session, source_item_id, 20, 0);
        session.cache_item_template_addon_quest_log_item_id_like_cpp(
            source_item_id,
            quest_log_item_id,
        );
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);

        let player_registry = Arc::new(PlayerRegistry::default());
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_loaded_player_name_like_cpp("Receiver".to_string());
        session.register_in_player_registry();

        let (mut sender_session, sender_rx) = make_session();
        sender_session.set_player_guid(Some(sender_guid));
        sender_session.set_loaded_player_name_like_cpp("Sender".to_string());
        sender_session.set_player_registry(Arc::clone(&player_registry));
        add_active_quest_in_slot_with_status(
            &mut sender_session,
            quest_id,
            0,
            QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        );
        sender_session.register_in_player_registry();
        sender_session.sync_player_registry_state_like_cpp();

        let (mut other_session, other_rx) = make_session();
        other_session.set_player_guid(Some(other_guid));
        other_session.set_loaded_player_name_like_cpp("Other".to_string());
        other_session.set_player_registry(player_registry);
        other_session.register_in_player_registry();

        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(sender_guid);
        group.add_member(receiver_guid);
        group.add_member(other_guid);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        let self_packet = send_rx.try_recv().expect("receiver group packet");
        assert_eq!(sender_rx.try_recv().unwrap(), self_packet);
        assert_eq!(other_rx.try_recv().unwrap(), self_packet);
        assert!(send_rx.try_recv().is_err());
        let mut packet = WorldPacket::from_bytes(&self_packet);
        assert_eq!(
            packet.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::ItemPushResult as u16
        );
        assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
        assert_eq!(
            packet.read_uint8().unwrap(),
            u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
        );
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), quest_log_item_id as i32);
        assert_eq!(packet.read_int32().unwrap(), 2);
        assert_eq!(packet.read_int32().unwrap(), 2);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_uint32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert!(!packet.read_bit().unwrap());
        assert!(!packet.read_bit().unwrap());
        assert_eq!(packet.read_bits(3).unwrap(), 3);
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_bound_objective_dont_report_flag_sends_direct_like_cpp()
     {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 195);
        let other_guid = ObjectGuid::create_player(1, 196);
        let quest_id = 7116;
        let source_item_id = 9204;
        let quest_log_item_id = 9304;
        let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
        quest.objectives.push(QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: quest_log_item_id as i32,
            amount: 2,
            flags: 0,
            flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        install_source_item_template_with_flags3(
            &mut session,
            source_item_id,
            20,
            0,
            ItemFlags3::DontReportLootLogToParty as u32,
        );
        session.cache_item_template_addon_quest_log_item_id_like_cpp(
            source_item_id,
            quest_log_item_id,
        );
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let player_registry = Arc::new(PlayerRegistry::default());
        session.set_player_registry(Arc::clone(&player_registry));
        session.set_loaded_player_name_like_cpp("Receiver".to_string());
        session.register_in_player_registry();

        let (mut sender_session, sender_rx) = make_session();
        sender_session.set_player_guid(Some(sender_guid));
        sender_session.set_loaded_player_name_like_cpp("Sender".to_string());
        sender_session.set_player_registry(Arc::clone(&player_registry));
        add_active_quest_in_slot_with_status(
            &mut sender_session,
            quest_id,
            0,
            QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        );
        sender_session.register_in_player_registry();
        sender_session.sync_player_registry_state_like_cpp();

        let (mut other_session, other_rx) = make_session();
        other_session.set_player_guid(Some(other_guid));
        other_session.set_loaded_player_name_like_cpp("Other".to_string());
        other_session.set_player_registry(player_registry);
        other_session.register_in_player_registry();

        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(sender_guid);
        group.add_member(receiver_guid);
        group.add_member(other_guid);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        let sent = send_rx.try_recv().expect("direct bound item update packet");
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
        assert!(other_rx.try_recv().is_err());
        let mut packet = WorldPacket::from_bytes(&sent);
        assert_eq!(
            packet.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::ItemPushResult as u16
        );
        assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
        assert_eq!(
            packet.read_uint8().unwrap(),
            u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
        );
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), quest_log_item_id as i32);
        assert_eq!(packet.read_int32().unwrap(), 2);
        assert_eq!(packet.read_int32().unwrap(), 2);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_uint32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert!(!packet.read_bit().unwrap());
        assert!(!packet.read_bit().unwrap());
        assert_eq!(packet.read_bits(3).unwrap(), 3);
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_multiple_bound_objectives_still_creates_item_like_cpp()
     {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 192);
        let quest_id = 7114;
        let source_item_id = 9202;
        let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
        quest.objectives.push(QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: source_item_id as i32,
            amount: 2,
            flags: 0,
            flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        quest.objectives.push(QuestObjective {
            id: quest_id * 10 + 1,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 1,
            storage_index: 1,
            object_id: source_item_id as i32,
            amount: 2,
            flags: 0,
            flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        install_source_item_template(&mut session, source_item_id, 20, 0);
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        let status = session
            .player_quests
            .get(&quest_id)
            .expect("source-item quest should add local quest state");
        assert_eq!(status.objective_counts, vec![2, 2]);
        let stored_source_item_count: u32 = session
            .inventory_items_like_cpp()
            .values()
            .filter(|item| item.entry_id == source_item_id)
            .filter_map(|item| session.inventory_item_objects_like_cpp().get(&item.guid))
            .map(|item| item.count())
            .sum();
        assert_eq!(stored_source_item_count, 2);
        assert_eq!(
            session.represented_quest_confirm_accepts_like_cpp(),
            &[RepresentedQuestConfirmAcceptLikeCpp {
                receiver_guid: Some(receiver_guid),
                sender_guid_before_clear: sender_guid,
                quest_id,
                raw_quest_id: quest_id as i32,
                reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemStoredNewItem,
                object_accessor_unrepresented: true,
                party_runtime_unrepresented: true,
                can_add_source_item_unrepresented: false,
                can_add_source_item_result: Some(InventoryResult::Ok),
                add_quest_runtime_unrepresented: false,
                source_spell_unrepresented: false,
                represented_source_spell_id: None,
                represented_source_spell_self_casts: 0,
            }]
        );
        let sent: Vec<_> = std::iter::from_fn(|| send_rx.try_recv().ok()).collect();
        assert!(
            sent.iter().any(|bytes| {
                let mut packet = WorldPacket::from_bytes(bytes);
                packet.read_uint16().ok()
                    == Some(wow_constants::ServerOpcodes::ItemPushResult as u16)
            }),
            "multiple bound objectives do not trigger C++ no-grant path, so SendNewItem still runs"
        );
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_sequenced_objective_waits_for_previous_like_cpp() {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 197);
        let quest_id = 7117;
        let source_item_id = 9205;
        let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
        quest.objectives.push(QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: 9901,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        quest.objectives.push(QuestObjective {
            id: quest_id * 10 + 1,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 1,
            storage_index: 1,
            object_id: source_item_id as i32,
            amount: 2,
            flags: QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        install_source_item_template(&mut session, source_item_id, 20, 0);
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        let status = session
            .player_quests
            .get(&quest_id)
            .expect("source-item quest should add local quest state");
        assert_eq!(status.objective_counts, vec![0, 0]);
        let stored_source_item_count: u32 = session
            .inventory_items_like_cpp()
            .values()
            .filter(|item| item.entry_id == source_item_id)
            .filter_map(|item| session.inventory_item_objects_like_cpp().get(&item.guid))
            .map(|item| item.count())
            .sum();
        assert_eq!(stored_source_item_count, 2);
        let sent: Vec<_> = std::iter::from_fn(|| send_rx.try_recv().ok()).collect();
        assert!(
            sent.iter().any(|bytes| {
                let mut packet = WorldPacket::from_bytes(bytes);
                packet.read_uint16().ok()
                    == Some(wow_constants::ServerOpcodes::ItemPushResult as u16)
            }),
            "source item is still granted; only sequenced objective progress is blocked"
        );
        assert!(sender_rx.try_recv().is_err());
        assert_eq!(session.player_guid(), Some(receiver_guid));
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_optional_previous_allows_sequenced_objective_like_cpp()
     {
        let (mut session, _send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 198);
        let quest_id = 7118;
        let source_item_id = 9206;
        let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
        quest.objectives.push(QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: 9902,
            amount: 1,
            flags: QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP_LOCAL,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        quest.objectives.push(QuestObjective {
            id: quest_id * 10 + 1,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 1,
            storage_index: 1,
            object_id: source_item_id as i32,
            amount: 2,
            flags: QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        install_source_item_template(&mut session, source_item_id, 20, 0);
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        let status = session
            .player_quests
            .get(&quest_id)
            .expect("source-item quest should add local quest state");
        assert_eq!(status.objective_counts, vec![0, 2]);
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_progress_bar_part_objective_progresses_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 199);
        let quest_id = 7119;
        let source_item_id = 9207;
        let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
        quest.objectives.push(QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: source_item_id as i32,
            amount: 2,
            flags: QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL,
            flags2: 0,
            progress_bar_weight: 50.0,
            description: String::new(),
        });
        quest.objectives.push(QuestObjective {
            id: quest_id * 10 + 1,
            quest_id,
            obj_type: QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL,
            order: 1,
            storage_index: 1,
            object_id: 0,
            amount: 100,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        install_source_item_template(&mut session, source_item_id, 20, 0);
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        let status = session
            .player_quests
            .get(&quest_id)
            .expect("source-item quest should add local quest state");
        assert_eq!(status.objective_counts, vec![2, 0]);
        assert!(sender_rx.try_recv().is_err());
    }

    #[test]
    fn represented_progress_bar_part_objective_stops_when_progress_bar_complete_like_cpp() {
        let quest_id = 7120;
        let mut quest = quest_template(quest_id);
        quest.objectives = vec![
            QuestObjective {
                id: quest_id * 10,
                quest_id,
                obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
                order: 0,
                storage_index: 0,
                object_id: 99,
                amount: 2,
                flags: QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL,
                flags2: 0,
                progress_bar_weight: 50.0,
                description: String::new(),
            },
            QuestObjective {
                id: quest_id * 10 + 1,
                quest_id,
                obj_type: QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL,
                order: 1,
                storage_index: 1,
                object_id: 0,
                amount: 100,
                flags: 0,
                flags2: 0,
                progress_bar_weight: 0.0,
                description: String::new(),
            },
        ];
        let status = PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![2, 0],
            slot: 0,
        };

        assert!(
            !WorldSession::represented_quest_objective_completable_like_cpp(&status, &quest, 0)
        );
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_zero_count_normalizes_to_one_and_fails_full_inventory_like_cpp()
     {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 96);
        let quest_id = 7018;
        let source_item_id = 9005;
        let filler_item_id = 9105;
        session.set_quest_store(Arc::new(store_with_source_item_quest(
            quest_id,
            source_item_id,
            0,
            0,
        )));
        install_source_item_template(&mut session, source_item_id, 1, 0);
        for slot in 35..59 {
            insert_direct_inventory_item(
                &mut session,
                receiver_guid,
                slot,
                filler_item_id,
                1,
                91_000 + u64::from(slot),
            );
        }
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert!(!session.player_quests.contains_key(&quest_id));
        let outcomes = session.represented_quest_confirm_accepts_like_cpp();
        assert_eq!(outcomes.len(), 1);
        let outcome = &outcomes[0];
        assert_eq!(outcome.receiver_guid, Some(receiver_guid));
        assert_eq!(outcome.sender_guid_before_clear, sender_guid);
        assert_eq!(outcome.quest_id, quest_id);
        assert_eq!(outcome.raw_quest_id, quest_id as i32);
        assert_eq!(
            outcome.reason,
            RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed
        );
        assert!(!outcome.add_quest_runtime_unrepresented);
        let source_item_result = outcome
            .can_add_source_item_result
            .expect("zero ProvidedItemCount must normalize to one and reach planner failure");
        assert_ne!(source_item_result, InventoryResult::Ok);
        assert_ne!(source_item_result, InventoryResult::ItemMaxCount);
        assert_eq!(
            send_rx.try_recv().unwrap(),
            InventoryChangeFailure::error(source_item_result).to_bytes()
        );
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_at_max_count_allows_can_add_gate_like_cpp() {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 92);
        let quest_id = 7014;
        let source_item_id = 9002;
        session.set_quest_store(Arc::new(store_with_source_item_quest(
            quest_id,
            source_item_id,
            1,
            0,
        )));
        install_source_item_template(&mut session, source_item_id, 20, 1);
        insert_direct_inventory_item(&mut session, receiver_guid, 23, source_item_id, 1, 9002);
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert!(session.player_quests.contains_key(&quest_id));
        assert_eq!(
            session.represented_quest_confirm_accepts_like_cpp(),
            &[RepresentedQuestConfirmAcceptLikeCpp {
                receiver_guid: Some(receiver_guid),
                sender_guid_before_clear: sender_guid,
                quest_id,
                raw_quest_id: quest_id as i32,
                reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemMaxCountNoGrant,
                object_accessor_unrepresented: true,
                party_runtime_unrepresented: true,
                can_add_source_item_unrepresented: false,
                can_add_source_item_result: Some(InventoryResult::ItemMaxCount),
                add_quest_runtime_unrepresented: false,
                source_spell_unrepresented: false,
                represented_source_spell_id: None,
                represented_source_spell_self_casts: 0,
            }]
        );
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_missing_source_item_proto_fails_can_add_gate_like_cpp() {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 93);
        let quest_id = 7015;
        let source_item_id = 9003;
        session.set_quest_store(Arc::new(store_with_source_item_quest(
            quest_id,
            source_item_id,
            1,
            0,
        )));
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert_eq!(
            session.represented_quest_confirm_accepts_like_cpp(),
            &[RepresentedQuestConfirmAcceptLikeCpp {
                receiver_guid: Some(receiver_guid),
                sender_guid_before_clear: sender_guid,
                quest_id,
                raw_quest_id: quest_id as i32,
                reason:
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed,
                object_accessor_unrepresented: true,
                party_runtime_unrepresented: true,
                can_add_source_item_unrepresented: false,
                can_add_source_item_result: Some(InventoryResult::ItemNotFound),
                add_quest_runtime_unrepresented: false,
                source_spell_unrepresented: false,
                represented_source_spell_id: None,
                represented_source_spell_self_casts: 0,
            }]
        );
        assert_eq!(
            send_rx.try_recv().unwrap(),
            InventoryChangeFailure::error(InventoryResult::ItemNotFound).to_bytes()
        );
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_limit_category_missing_db2_entry_fails_like_cpp() {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 95);
        let quest_id = 7017;
        let source_item_id = 9004;
        session.set_quest_store(Arc::new(store_with_source_item_quest(
            quest_id,
            source_item_id,
            1,
            0,
        )));
        install_source_item_template_with_limit_category(&mut session, source_item_id, 20, 0, 44);
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert!(!session.player_quests.contains_key(&quest_id));
        assert_eq!(
            session.represented_quest_confirm_accepts_like_cpp(),
            &[RepresentedQuestConfirmAcceptLikeCpp {
                receiver_guid: Some(receiver_guid),
                sender_guid_before_clear: sender_guid,
                quest_id,
                raw_quest_id: quest_id as i32,
                reason:
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed,
                object_accessor_unrepresented: true,
                party_runtime_unrepresented: true,
                can_add_source_item_unrepresented: false,
                can_add_source_item_result: Some(InventoryResult::NotEquippable),
                add_quest_runtime_unrepresented: false,
                source_spell_unrepresented: false,
                represented_source_spell_id: None,
                represented_source_spell_self_casts: 0,
            }]
        );
        assert_eq!(
            send_rx.try_recv().unwrap(),
            InventoryChangeFailure::error(InventoryResult::NotEquippable).to_bytes()
        );
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_source_item_start_quest_still_respects_limit_category_like_cpp() {
        let (mut session, send_rx) = make_session();
        let receiver_guid = session.player_guid().unwrap();
        let sender_guid = ObjectGuid::create_player(1, 97);
        let quest_id = 7019;
        let source_item_id = 9006;
        session.set_quest_store(Arc::new(store_with_source_item_quest(
            quest_id,
            source_item_id,
            1,
            0,
        )));
        install_source_item_template_with_start_quest_and_limit_category(
            &mut session,
            source_item_id,
            20,
            0,
            quest_id as i32,
            44,
        );
        session.set_item_limit_category_store(Arc::new(ItemLimitCategoryStore::from_entries([
            ItemLimitCategoryEntry {
                id: 44,
                name: "Quest Source Have Limit".into(),
                quantity: 1,
                flags: ITEM_LIMIT_CATEGORY_MODE_HAVE,
            },
        ])));
        insert_direct_inventory_item(&mut session, receiver_guid, 23, source_item_id, 1, 9906);
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert!(!session.player_quests.contains_key(&quest_id));
        assert_eq!(
            session.represented_quest_confirm_accepts_like_cpp(),
            &[RepresentedQuestConfirmAcceptLikeCpp {
                receiver_guid: Some(receiver_guid),
                sender_guid_before_clear: sender_guid,
                quest_id,
                raw_quest_id: quest_id as i32,
                reason:
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed,
                object_accessor_unrepresented: true,
                party_runtime_unrepresented: true,
                can_add_source_item_unrepresented: false,
                can_add_source_item_result: Some(
                    InventoryResult::ItemMaxLimitCategoryCountExceededIs
                ),
                add_quest_runtime_unrepresented: false,
                source_spell_unrepresented: false,
                represented_source_spell_id: None,
                represented_source_spell_self_casts: 0,
            }]
        );
        assert_eq!(
            send_rx.try_recv().unwrap(),
            InventoryChangeFailure::error(InventoryResult::ItemMaxLimitCategoryCountExceededIs)
                .with_limit_category(44)
                .to_bytes()
        );
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_without_source_item_does_not_overclaim_source_gate_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 94);
        let quest_id = 7016;
        session.set_quest_store(Arc::new(store_with_source_item_quest(quest_id, 0, 0, 0)));
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert_confirm_accept_outcome(
            &session,
            Some(ObjectGuid::create_player(1, 42)),
            sender_guid,
            quest_id,
            quest_id as i32,
            RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
        );
        assert!(session.player_quests.contains_key(&quest_id));
        assert_eq!(
            session
                .player_quests
                .get(&quest_id)
                .expect("no-objective shared quest should be locally tracked")
                .status,
            QUEST_STATUS_COMPLETE_LIKE_CPP
        );
        assert_complete_status_update_like_cpp(&session, quest_id, false);
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_confirm_accept_tracking_event_auto_rewards_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 201);
        let quest_id = 7121;
        let mut quest = quest_template(quest_id);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP | QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP;
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
        let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
            &mut session,
            sender_guid,
            quest_id,
            true,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        );

        run_quest_confirm_accept(&mut session, quest_id as i32).await;

        assert_confirm_accept_outcome(
            &session,
            Some(ObjectGuid::create_player(1, 42)),
            sender_guid,
            quest_id,
            quest_id as i32,
            RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
        );
        assert!(!session.player_quests.contains_key(&quest_id));
        assert!(session.rewarded_quests.contains(&quest_id));
        assert_complete_status_update_like_cpp(&session, quest_id, false);
        let complete = send_rx
            .try_recv()
            .expect("tracking event auto reward should send quest complete");
        assert_eq!(
            wow_packet::WorldPacket::from_bytes(&complete).server_opcode(),
            Some(wow_constants::ServerOpcodes::QuestGiverQuestComplete)
        );
        let update = send_rx
            .try_recv()
            .expect("tracking event auto reward should send quest update complete");
        assert_eq!(
            wow_packet::WorldPacket::from_bytes(&update).server_opcode(),
            Some(wow_constants::ServerOpcodes::QuestUpdateComplete)
        );
        assert!(send_rx.try_recv().is_err());
        assert!(sender_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_push_short_packet_does_not_clear_pending_state_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 77);
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7001);

        session
            .handle_quest_push_result(WorldPacket::from_bytes(&[0x00]))
            .await;

        assert_eq!(
            session.represented_pending_quest_sharing_like_cpp(),
            Some(crate::session::RepresentedPendingQuestSharingLikeCpp {
                sender_guid,
                quest_id: 7001,
            })
        );
        assert!(
            session
                .represented_quest_push_result_responses_like_cpp()
                .is_empty()
        );
        assert_eq!(
            session.represented_quest_push_result_sender_mismatch_count_like_cpp(),
            0
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_push_no_pending_valid_packet_is_noop_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 78);

        run_quest_push_result(&mut session, sender_guid, 7002, 3).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert!(
            session
                .represented_quest_push_result_responses_like_cpp()
                .is_empty()
        );
        assert_eq!(
            session.represented_quest_push_result_sender_mismatch_count_like_cpp(),
            0
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_push_pending_sender_match_clears_and_records_response_evidence_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = ObjectGuid::create_player(1, 79);
        let receiver_guid = session.player_guid().unwrap();
        session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7003);

        run_quest_push_result(&mut session, sender_guid, 8003, 6).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert_eq!(
            session.represented_quest_push_result_responses_like_cpp(),
            &[RepresentedQuestPushResultResponseLikeCpp {
                receiver_guid,
                sender_guid,
                parsed_quest_id: 8003,
                pending_quest_id: 7003,
                result: 6,
            }]
        );
        assert_eq!(
            session.represented_quest_push_result_sender_mismatch_count_like_cpp(),
            0
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_push_pending_sender_mismatch_clears_without_response_evidence_like_cpp() {
        let (mut session, send_rx) = make_session();
        let pending_sender_guid = ObjectGuid::create_player(1, 80);
        let packet_sender_guid = ObjectGuid::create_player(1, 81);
        session.set_represented_pending_quest_sharing_like_cpp(pending_sender_guid, 7004);

        run_quest_push_result(&mut session, packet_sender_guid, 7004, 4).await;

        assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
        assert!(
            session
                .represented_quest_push_result_responses_like_cpp()
                .is_empty()
        );
        assert_eq!(
            session.represented_quest_push_result_sender_mismatch_count_like_cpp(),
            1
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[test]
    fn quest_push_inventory_registration_and_dispatcher_contract_like_cpp() {
        let entry = inventory::iter::<PacketHandlerEntry>
            .into_iter()
            .find(|entry| entry.opcode == ClientOpcodes::QuestPushResult)
            .expect("QuestPushResult handler registration");

        assert_eq!(entry.status, SessionStatus::LoggedIn);
        assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
        assert_eq!(entry.handler_name, "handle_quest_push_result");
        assert!(include_str!("../session.rs").contains("ClientOpcodes::QuestPushResult =>"));
        assert!(include_str!("../session.rs").contains("self.handle_quest_push_result(pkt).await"));
    }

    #[test]
    fn quest_push_packet_parser_reads_sender_quest_id_result_in_cpp_order() {
        let sender_guid = ObjectGuid::create_player(1, 82);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&sender_guid);
        pkt.write_uint32(7005);
        pkt.write_uint8(9);

        let parsed = QuestPushResult::read(&mut pkt).expect("valid QuestPushResult");

        assert_eq!(parsed.sender_guid, sender_guid);
        assert_eq!(parsed.quest_id, 7005);
        assert_eq!(parsed.result, 9);
    }

    #[tokio::test]
    async fn push_quest_to_party_malformed_packet_records_no_evidence_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_quest_store(Arc::new(store_with_sharable_quest(7101)));
        add_active_quest(&mut session, 7101);

        session
            .handle_push_quest_to_party(WorldPacket::from_bytes(&[0x9F, 0x34, 0x00]))
            .await;

        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .is_empty()
        );
        assert!(
            session
                .represented_pending_quest_sharing_like_cpp()
                .is_none()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn push_quest_to_party_missing_quest_template_returns_silently_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_quest_store(Arc::new(store_with_quests(&[7102])));

        run_push_quest_to_party(&mut session, 7103).await;

        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn push_quest_to_party_unshareable_or_not_in_log_records_not_allowed_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = session.player_guid();
        session.set_quest_store(Arc::new(store_with_sharable_quest(7104)));

        run_push_quest_to_party(&mut session, 7104).await;

        assert_eq!(
            session.represented_push_quest_to_party_outcomes_like_cpp(),
            &[RepresentedPushQuestToPartyOutcomeLikeCpp {
                sender_guid,
                quest_id: 7104,
                target_guid: sender_guid,
                reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotAllowed,
                quest_pool_active_check_unrepresented: false,
                group_runtime_unrepresented: false,
                receiver_fanout_unrepresented: false,
            }]
        );
        assert_eq!(
            recv_push_quest_result_response(&send_rx),
            (
                sender_guid.expect("test session has player guid"),
                quest_push_reason::NOT_ALLOWED,
                String::new()
            )
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_shareable_sender_without_pool_store_still_blocks_before_group_like_cpp()
     {
        let (mut session, send_rx) = make_session();
        let sender_guid = session.player_guid();
        session.set_quest_store(Arc::new(store_with_sharable_quest(7105)));
        add_active_quest(&mut session, 7105);

        run_push_quest_to_party(&mut session, 7105).await;

        assert_eq!(
            session.represented_push_quest_to_party_outcomes_like_cpp(),
            &[RepresentedPushQuestToPartyOutcomeLikeCpp {
                sender_guid,
                quest_id: 7105,
                target_guid: sender_guid,
                reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::QuestPoolActiveCheckUnrepresented,
                quest_pool_active_check_unrepresented: true,
                group_runtime_unrepresented: false,
                receiver_fanout_unrepresented: false,
            }]
        );
        assert!(
            session
                .represented_pending_quest_sharing_like_cpp()
                .is_none()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn push_quest_to_party_inactive_pooled_quest_records_not_daily_before_group_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = session.player_guid();
        let quest_store = store_with_daily_sharable_quests(&[7106, 7107]);
        let quest_pool_store =
            quest_pool_store_with_active_saved(&quest_store, 77, &[7106, 7107], &[7107]);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, 7106);
        session.group_guid = Some(99);

        run_push_quest_to_party(&mut session, 7106).await;

        assert_eq!(
            session.represented_push_quest_to_party_outcomes_like_cpp(),
            &[RepresentedPushQuestToPartyOutcomeLikeCpp {
                sender_guid,
                quest_id: 7106,
                target_guid: sender_guid,
                reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotDaily,
                quest_pool_active_check_unrepresented: false,
                group_runtime_unrepresented: false,
                receiver_fanout_unrepresented: false,
            }]
        );
        assert_eq!(
            recv_push_quest_result_response(&send_rx),
            (
                sender_guid.expect("test session has player guid"),
                quest_push_reason::NOT_DAILY,
                String::new()
            )
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_active_pooled_quest_passes_pool_check_to_not_in_party_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = session.player_guid();
        let quest_store = store_with_daily_sharable_quests(&[7108, 7109]);
        let quest_pool_store =
            quest_pool_store_with_active_saved(&quest_store, 78, &[7108, 7109], &[7108]);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, 7108);

        run_push_quest_to_party(&mut session, 7108).await;

        assert_eq!(
            session.represented_push_quest_to_party_outcomes_like_cpp(),
            &[RepresentedPushQuestToPartyOutcomeLikeCpp {
                sender_guid,
                quest_id: 7108,
                target_guid: sender_guid,
                reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotInParty,
                quest_pool_active_check_unrepresented: false,
                group_runtime_unrepresented: false,
                receiver_fanout_unrepresented: false,
            }]
        );
        assert_eq!(
            recv_push_quest_result_response(&send_rx),
            (
                sender_guid.expect("test session has player guid"),
                quest_push_reason::NOT_IN_PARTY,
                String::new()
            )
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_non_pooled_quest_passes_pool_check_to_group_boundary_like_cpp() {
        let (mut session, send_rx) = make_session();
        let sender_guid = session.player_guid();
        let quest_store = store_with_sharable_quest(7110);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, 7110);
        session.group_guid = Some(99);

        run_push_quest_to_party(&mut session, 7110).await;

        assert_eq!(
            session.represented_push_quest_to_party_outcomes_like_cpp(),
            &[RepresentedPushQuestToPartyOutcomeLikeCpp {
                sender_guid,
                quest_id: 7110,
                target_guid: sender_guid,
                reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented,
                quest_pool_active_check_unrepresented: false,
                group_runtime_unrepresented: true,
                receiver_fanout_unrepresented: true,
            }]
        );
        assert!(send_rx.try_recv().is_err());
    }

    fn install_represented_party(
        session: &mut WorldSession,
        sender_guid: ObjectGuid,
        receiver_guid: ObjectGuid,
    ) -> (Arc<PlayerRegistry>, WorldSession, flume::Receiver<Vec<u8>>) {
        let player_registry = Arc::new(PlayerRegistry::default());
        let (mut receiver_session, receiver_rx) = make_session();
        receiver_session.set_player_guid(Some(receiver_guid));
        receiver_session.set_loaded_player_name_like_cpp("Receiver".to_string());
        receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        receiver_session.set_player_position_like_cpp(Position::new(11.0, 0.0, 0.0, 0.0));
        receiver_session.set_player_registry(Arc::clone(&player_registry));
        receiver_session.register_in_player_registry();

        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(sender_guid);
        group.add_member(receiver_guid);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry.clone());
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
        (player_registry, receiver_session, receiver_rx)
    }

    #[tokio::test]
    async fn push_quest_to_party_grouped_receiver_on_quest_emits_on_quest_pair_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 43);
        let quest_store = store_with_sharable_quest(7111);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, 7111);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        add_active_quest(&mut receiver_session, 7111);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, 7111).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_ON_QUEST_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_ON_QUEST_TO_RECIPIENT_LIKE_CPP,
                "Quest 7111".to_string()
            )
        );
        assert!(
            !session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented
                ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_grouped_receiver_rewarded_emits_already_done_pair_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 44);
        let quest_store = store_with_sharable_quest(7112);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, 7112);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.rewarded_quests.insert(7112);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, 7112).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
                "Quest 7112".to_string()
            )
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_grouped_receiver_log_full_emits_log_full_pair_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 144);
        let shared_quest_id = 7116;
        let quest_store = store_with_sharable_quest(shared_quest_id);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        for slot in 0..MAX_QUEST_LOG_SIZE_LIKE_CPP {
            add_active_quest_in_slot(&mut receiver_session, 8000 + u32::from(slot), slot);
        }
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_LOG_FULL_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_LOG_FULL_TO_RECIPIENT_LIKE_CPP,
                "Quest 7116".to_string()
            )
        );
        assert!(
            !session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
                ))
        );
        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverLogFull
                ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_grouped_receiver_daily_completed_emits_already_done_pair_like_cpp()
    {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 244);
        let shared_quest_id = 7117;
        let quest_store = store_with_daily_sharable_quests(&[shared_quest_id]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session
            .set_represented_daily_quest_completed_like_cpp_for_test(shared_quest_id, true);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
                "Quest 7117".to_string()
            )
        );
        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone
            ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_grouped_receiver_df_completed_emits_already_done_pair_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 245);
        let shared_quest_id = 7118;
        let quest_store = store_with_df_sharable_quest(shared_quest_id);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.set_represented_df_quest_like_cpp_for_test(shared_quest_id, true);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
                "Quest 7118".to_string()
            )
        );
        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone
            ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_non_daily_non_df_ignores_unrelated_daily_snapshot_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 246);
        let shared_quest_id = 7119;
        let quest_store = store_with_sharable_quest(shared_quest_id);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.set_represented_daily_quest_completed_like_cpp_for_test(9001, true);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_success_command_queued_like_cpp(
            &sender_rx,
            &receiver_rx,
            &mut receiver_session,
            receiver_guid,
            sender_guid,
            shared_quest_id,
        );
        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
        );
        assert!(
            !session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone
            ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_grouped_receiver_low_level_emits_low_level_pair_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 248);
        let shared_quest_id = 7121;
        let quest_store = store_with_sharable_quest_levels(shared_quest_id, 10, 0);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.set_player_level_like_cpp(4);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_LOW_LEVEL_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_LOW_LEVEL_TO_RECIPIENT_LIKE_CPP,
                "Quest 7121".to_string()
            )
        );
        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMinLevelLowLevel
                ))
        );
        assert!(
            !session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
                ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_grouped_receiver_high_level_emits_high_level_pair_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 249);
        let shared_quest_id = 7122;
        let quest_store = store_with_sharable_quest_levels(shared_quest_id, 1, 40);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.set_player_level_like_cpp(80);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_HIGH_LEVEL_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_HIGH_LEVEL_TO_RECIPIENT_LIKE_CPP,
                "Quest 7122".to_string()
            )
        );
        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMaxLevelHighLevel
                ))
        );
        assert!(
            !session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
                ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_receiver_max_level_zero_does_not_block_high_level_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 250);
        let shared_quest_id = 7123;
        let quest_store = store_with_sharable_quest_levels(shared_quest_id, 1, 0);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.set_player_level_like_cpp(80);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_success_command_queued_like_cpp(
            &sender_rx,
            &receiver_rx,
            &mut receiver_session,
            receiver_guid,
            sender_guid,
            shared_quest_id,
        );
        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
                ))
        );
        assert!(
            !session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMaxLevelHighLevel
                ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_grouped_receiver_wrong_class_emits_class_pair_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 251);
        let shared_quest_id = 7124;
        let quest_store = store_with_sharable_quest_class_race(shared_quest_id, 1 << (2 - 1), 0);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        receiver_session.sync_player_registry_state_like_cpp();
        assert_eq!(
            player_registry
                .get(&receiver_guid)
                .expect("receiver snapshot")
                .class,
            1
        );

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_CLASS_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_CLASS_TO_RECIPIENT_LIKE_CPP,
                "Quest 7124".to_string()
            )
        );
        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestClassWrongClass
                ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_grouped_receiver_wrong_race_emits_race_pair_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 252);
        let shared_quest_id = 7125;
        let quest_store = store_with_sharable_quest_class_race(shared_quest_id, 0, 1 << (2 - 1));
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        receiver_session.sync_player_registry_state_like_cpp();
        assert_eq!(
            player_registry
                .get(&receiver_guid)
                .expect("receiver snapshot")
                .race,
            1
        );

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_RACE_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_RACE_TO_RECIPIENT_LIKE_CPP,
                "Quest 7125".to_string()
            )
        );
        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestRaceWrongRace
                ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_receiver_class_precedes_race_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 253);
        let shared_quest_id = 7126;
        let quest_store =
            store_with_sharable_quest_class_race(shared_quest_id, 1 << (2 - 1), 1 << (2 - 1));
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_CLASS_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_CLASS_TO_RECIPIENT_LIKE_CPP,
                "Quest 7126".to_string()
            )
        );
        assert!(
            !session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestRaceWrongRace
                ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_zero_class_and_race_masks_do_not_block_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 254);
        let shared_quest_id = 7127;
        let quest_store = store_with_sharable_quest_class_race(shared_quest_id, 0, 0);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_success_command_queued_like_cpp(
            &sender_rx,
            &receiver_rx,
            &mut receiver_session,
            receiver_guid,
            sender_guid,
            shared_quest_id,
        );
        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
                ))
        );
        assert!(
            !session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestClassWrongClass
                        | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestRaceWrongRace
                ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_grouped_receiver_low_min_reputation_emits_low_faction_pair_like_cpp()
     {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 255);
        let shared_quest_id = 7128;
        let quest_store = store_with_sharable_quest_reputation(shared_quest_id, 72, 100, 0, 0);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (player_registry, receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        player_registry
            .get_mut(&receiver_guid)
            .expect("receiver snapshot")
            .reputation_standings = vec![(72, 99)];
        receiver_session.sync_player_registry_state_like_cpp();
        player_registry
            .get_mut(&receiver_guid)
            .expect("receiver snapshot")
            .reputation_standings = vec![(72, 99)];

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_LOW_FACTION_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_LOW_FACTION_TO_RECIPIENT_LIKE_CPP,
                "Quest 7128".to_string()
            )
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction)));
    }

    #[tokio::test]
    async fn push_quest_to_party_grouped_receiver_equal_max_reputation_emits_low_faction_pair_like_cpp()
     {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 256);
        let shared_quest_id = 7129;
        let quest_store = store_with_sharable_quest_reputation(shared_quest_id, 0, 0, 72, 100);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (player_registry, receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.sync_player_registry_state_like_cpp();
        player_registry
            .get_mut(&receiver_guid)
            .expect("receiver snapshot")
            .reputation_standings = vec![(72, 100)];

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_LOW_FACTION_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_LOW_FACTION_TO_RECIPIENT_LIKE_CPP,
                "Quest 7129".to_string()
            )
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationHighFaction)));
    }

    #[tokio::test]
    async fn push_quest_to_party_zero_reputation_factions_do_not_block_with_missing_snapshot_like_cpp()
     {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 257);
        let shared_quest_id = 7130;
        let quest_store = store_with_sharable_quest_reputation(shared_quest_id, 0, 999, 0, -1);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_success_command_queued_like_cpp(
            &sender_rx,
            &receiver_rx,
            &mut receiver_session,
            receiver_guid,
            sender_guid,
            shared_quest_id,
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
        assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationHighFaction)));
    }

    #[tokio::test]
    async fn push_quest_to_party_positive_prev_missing_rewarded_emits_prerequisite_pair_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 259);
        let shared_quest_id = 7132;
        let quest_store = store_with_sharable_quest_previous(shared_quest_id, 9001);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
                "Quest 7132".to_string()
            )
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
        assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
    }

    #[tokio::test]
    async fn push_quest_to_party_positive_prev_rewarded_passes_to_unrepresented_boundary_like_cpp()
    {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 260);
        let shared_quest_id = 7133;
        let quest_store = store_with_sharable_quest_previous(shared_quest_id, 9002);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.rewarded_quests.insert(9002);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_success_command_queued_like_cpp(
            &sender_rx,
            &receiver_rx,
            &mut receiver_session,
            receiver_guid,
            sender_guid,
            shared_quest_id,
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
        assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
    }

    #[tokio::test]
    async fn push_quest_to_party_negative_prev_missing_active_incomplete_emits_prerequisite_pair_like_cpp()
     {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 261);
        let shared_quest_id = 7134;
        let quest_store = store_with_sharable_quest_previous(shared_quest_id, -9003);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
                "Quest 7134".to_string()
            )
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
    }

    #[tokio::test]
    async fn push_quest_to_party_negative_prev_active_incomplete_passes_to_unrepresented_boundary_like_cpp()
     {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 262);
        let shared_quest_id = 7135;
        let quest_store = store_with_sharable_quest_previous(shared_quest_id, -9004);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        add_active_quest(&mut receiver_session, 9004);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_success_command_queued_like_cpp(
            &sender_rx,
            &receiver_rx,
            &mut receiver_session,
            receiver_guid,
            sender_guid,
            shared_quest_id,
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
        assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
    }

    #[tokio::test]
    async fn push_quest_to_party_dependent_previous_missing_rewarded_emits_prerequisite_pair_like_cpp()
     {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 470);
        let shared_quest_id = 7607;
        let prev_id = 9607;
        let mut shared_quest = quest_template(shared_quest_id);
        shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        let mut previous_quest = quest_template(prev_id);
        previous_quest.next_quest_id = shared_quest_id;
        previous_quest.exclusive_group = 0;
        let quest_store = QuestStore::from_quests_like_cpp([shared_quest, previous_quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
                "Quest 7607".to_string()
            )
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));
    }

    #[tokio::test]
    async fn push_quest_to_party_dependent_previous_rewarded_nonnegative_group_passes_to_unrepresented_boundary_like_cpp()
     {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 471);
        let shared_quest_id = 7608;
        let prev_id = 9608;
        let mut shared_quest = quest_template(shared_quest_id);
        shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        let mut previous_quest = quest_template(prev_id);
        previous_quest.next_quest_id = shared_quest_id;
        previous_quest.exclusive_group = 0;
        let quest_store = QuestStore::from_quests_like_cpp([shared_quest, previous_quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.rewarded_quests.insert(prev_id);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_success_command_queued_like_cpp(
            &sender_rx,
            &receiver_rx,
            &mut receiver_session,
            receiver_guid,
            sender_guid,
            shared_quest_id,
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
        assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));
    }

    #[tokio::test]
    async fn push_quest_to_party_dependent_previous_negative_exclusive_group_requires_all_other_members_like_cpp()
     {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 472);
        let shared_quest_id = 7609;
        let prev_id = 9609;
        let sibling_id = 9610;
        let mut shared_quest = quest_template(shared_quest_id);
        shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        let mut previous_quest = quest_template(prev_id);
        previous_quest.next_quest_id = shared_quest_id;
        previous_quest.exclusive_group = -90;
        let mut sibling_quest = quest_template(sibling_id);
        sibling_quest.exclusive_group = -90;
        let quest_store =
            QuestStore::from_quests_like_cpp([shared_quest, previous_quest, sibling_quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.rewarded_quests.insert(prev_id);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
                "Quest 7609".to_string()
            )
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));

        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let mut shared_quest = quest_template(shared_quest_id);
        shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        let mut previous_quest = quest_template(prev_id);
        previous_quest.next_quest_id = shared_quest_id;
        previous_quest.exclusive_group = -90;
        let mut sibling_quest = quest_template(sibling_id);
        sibling_quest.exclusive_group = -90;
        let quest_store =
            QuestStore::from_quests_like_cpp([shared_quest, previous_quest, sibling_quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.rewarded_quests.insert(prev_id);
        receiver_session.rewarded_quests.insert(sibling_id);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_success_command_queued_like_cpp(
            &sender_rx,
            &receiver_rx,
            &mut receiver_session,
            receiver_guid,
            sender_guid,
            shared_quest_id,
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
        assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));
    }

    #[tokio::test]
    async fn push_quest_to_party_dependent_breadcrumb_active_status_emits_prerequisite_and_absent_passes_like_cpp()
     {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 473);
        let shared_quest_id = 7610;
        let breadcrumb_id = 9611;
        let mut shared_quest = quest_template(shared_quest_id);
        shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        let mut breadcrumb_quest = quest_template(breadcrumb_id);
        breadcrumb_quest.breadcrumb_for_quest_id = shared_quest_id as i32;
        let quest_store = QuestStore::from_quests_like_cpp([shared_quest, breadcrumb_quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        add_active_quest_in_slot_with_status(
            &mut receiver_session,
            breadcrumb_id,
            2,
            QUEST_STATUS_FAILED_LIKE_CPP,
        );
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
                "Quest 7610".to_string()
            )
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite)));

        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let mut shared_quest = quest_template(shared_quest_id);
        shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        let mut breadcrumb_quest = quest_template(breadcrumb_id);
        breadcrumb_quest.breadcrumb_for_quest_id = shared_quest_id as i32;
        let quest_store = QuestStore::from_quests_like_cpp([shared_quest, breadcrumb_quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_success_command_queued_like_cpp(
            &sender_rx,
            &receiver_rx,
            &mut receiver_session,
            receiver_guid,
            sender_guid,
            shared_quest_id,
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
        assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite)));
    }

    #[tokio::test]
    async fn push_quest_to_party_breadcrumb_for_quest_remains_unrepresented_without_prerequisite_pair_like_cpp()
     {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 474);
        let shared_quest_id = 7611;
        let mut shared_quest = quest_template(shared_quest_id);
        shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        shared_quest.breadcrumb_for_quest_id = 9991;
        let target_quest = quest_template(9991);
        let quest_store = QuestStore::from_quests_like_cpp([shared_quest, target_quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert!(sender_rx.try_recv().is_err());
        assert!(receiver_rx.try_recv().is_err());
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented)));
        assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
    }

    #[tokio::test]
    async fn push_quest_to_party_reputation_precedes_previous_prerequisite_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 263);
        let shared_quest_id = 7136;
        let mut quest = quest_template(shared_quest_id);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        quest.required_min_rep_faction = 72;
        quest.required_min_rep_value = 100;
        quest.prev_quest_id = 9005;
        let quest_store = QuestStore::from_quests_like_cpp([quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (player_registry, receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.sync_player_registry_state_like_cpp();
        player_registry
            .get_mut(&receiver_guid)
            .expect("receiver snapshot")
            .reputation_standings = vec![(72, 99)];

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_LOW_FACTION_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_LOW_FACTION_TO_RECIPIENT_LIKE_CPP,
                "Quest 7136".to_string()
            )
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction)));
        assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
    }

    #[tokio::test]
    async fn push_quest_to_party_class_precedes_reputation_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 258);
        let shared_quest_id = 7131;
        let mut quest = quest_template(shared_quest_id);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        quest.allowable_classes = 1 << (2 - 1);
        quest.required_min_rep_faction = 72;
        quest.required_min_rep_value = 100;
        let quest_store = QuestStore::from_quests_like_cpp([quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        receiver_session.sync_player_registry_state_like_cpp();
        player_registry
            .get_mut(&receiver_guid)
            .expect("receiver snapshot")
            .reputation_standings = vec![(72, -42000)];

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_CLASS_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_CLASS_TO_RECIPIENT_LIKE_CPP,
                "Quest 7131".to_string()
            )
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestClassWrongClass)));
        assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction)));
    }

    #[tokio::test]
    async fn push_quest_to_party_daily_precedes_low_level_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 251);
        let shared_quest_id = 7124;
        let mut quest = quest_template(shared_quest_id);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
        quest.min_level = 80;
        let quest_store = QuestStore::from_quests_like_cpp([quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.set_player_level_like_cpp(1);
        receiver_session
            .set_represented_daily_quest_completed_like_cpp_for_test(shared_quest_id, true);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
                "Quest 7124".to_string()
            )
        );
        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone
                ))
        );
        assert!(
            !session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMinLevelLowLevel
                ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_receiver_level_snapshot_syncs_from_world_session_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 252);
        let shared_quest_id = 7125;
        let quest_store = store_with_sharable_quest_levels(shared_quest_id, 20, 0);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        assert_eq!(
            player_registry.get(&receiver_guid).map(|info| info.level),
            Some(80)
        );
        receiver_session.set_player_level_like_cpp(19);
        receiver_session.sync_player_registry_state_like_cpp();
        assert_eq!(
            player_registry.get(&receiver_guid).map(|info| info.level),
            Some(19)
        );

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_LOW_LEVEL_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_LOW_LEVEL_TO_RECIPIENT_LIKE_CPP,
                "Quest 7125".to_string()
            )
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_log_full_precedes_daily_completed_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 247);
        let shared_quest_id = 7120;
        let quest_store = store_with_daily_sharable_quests(&[shared_quest_id]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        for slot in 0..MAX_QUEST_LOG_SIZE_LIKE_CPP {
            add_active_quest_in_slot(&mut receiver_session, 8100 + u32::from(slot), slot);
        }
        receiver_session
            .set_represented_daily_quest_completed_like_cpp_for_test(shared_quest_id, true);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_LOG_FULL_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_LOG_FULL_TO_RECIPIENT_LIKE_CPP,
                "Quest 7120".to_string()
            )
        );
        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverLogFull
                ))
        );
        assert!(
            !session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone
            ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_low_receiver_expansion_emits_expansion_pair_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 248);
        let shared_quest_id = 7121;
        let mut quest = quest_template(shared_quest_id);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        quest.expansion = 2;
        let quest_store = QuestStore::from_quests_like_cpp([quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.expansion = 1;
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_EXPANSION_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_EXPANSION_TO_RECIPIENT_LIKE_CPP,
                "Quest 7121".to_string()
            )
        );
        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestExpansionRequiredExpansion
                ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_success_prompts_receiver_details_and_sets_pending_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 249);
        let shared_quest_id = 7122;
        let mut quest = quest_template(shared_quest_id);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        quest.expansion = 2;
        let quest_store = QuestStore::from_quests_like_cpp([quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.expansion = 2;
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_SUCCESS_LIKE_CPP,
                String::new()
            )
        );
        assert!(receiver_rx.try_recv().is_err());
        let commands = receiver_session.drain_session_commands();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            SessionCommand::SetQuestSharingInfoAndSendDetails(command) => {
                assert_eq!(command.sender_guid, sender_guid);
                assert_eq!(command.quest.id, shared_quest_id);
            }
            other => panic!("unexpected session command: {other:?}"),
        }
        receiver_session
            .session_command_tx()
            .try_send(commands.into_iter().next().expect("command"))
            .expect("requeue command for processing");
        receiver_session
            .process_represented_session_commands_like_cpp()
            .await;
        assert_eq!(
            receiver_session.represented_pending_quest_sharing_like_cpp(),
            Some(crate::session::RepresentedPendingQuestSharingLikeCpp {
                sender_guid,
                quest_id: shared_quest_id,
            })
        );
        recv_quest_giver_quest_details_contains_quest_id(&receiver_rx, shared_quest_id);
        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| outcome.target_guid == Some(receiver_guid)
                    && matches!(
                        outcome.reason,
                        RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
                    )
                    && !outcome.receiver_fanout_unrepresented)
        );
        assert!(
            !session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestExpansionRequiredExpansion
                        | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented
                ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_repeatable_turn_in_success_prompts_request_items_without_pending_like_cpp()
     {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 611);
        let shared_quest_id = 76110;
        let mut quest = quest_template(shared_quest_id);
        quest.quest_type = 0;
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        quest.special_flags |= 0x0000_0001;
        quest.objectives.push(QuestObjective {
            id: 1,
            quest_id: shared_quest_id,
            obj_type: 1,
            order: 0,
            storage_index: 0,
            object_id: 49211,
            amount: 3,
            flags: 0xA5,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        let quest_for_assertion = quest.clone();
        let quest_store = QuestStore::from_quests_like_cpp([quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_SUCCESS_LIKE_CPP,
                String::new()
            )
        );
        assert!(receiver_rx.try_recv().is_err());
        let commands = receiver_session.drain_session_commands();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            SessionCommand::SendRepeatableTurnInRequestItemsLikeCpp(command) => {
                assert_eq!(command.sender_guid, sender_guid);
                assert_eq!(command.quest.id, shared_quest_id);
            }
            other => panic!("unexpected session command: {other:?}"),
        }
        receiver_session
            .session_command_tx()
            .try_send(commands.into_iter().next().expect("command"))
            .expect("requeue command for processing");
        receiver_session
            .process_represented_session_commands_like_cpp()
            .await;

        assert_eq!(
            receiver_session.represented_pending_quest_sharing_like_cpp(),
            None
        );
        let (collect, auto_launched) =
            recv_quest_giver_request_items_like_cpp(&receiver_rx, shared_quest_id);
        assert_eq!(collect, vec![(49211, 3, 0xA5)]);
        assert!(auto_launched);
        assert!(
            !receiver_session
                .can_complete_repeatable_quest_represented_bounded_like_cpp(&quest_for_assertion)
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(
            |outcome| outcome.target_guid == Some(receiver_guid)
                && matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPrompted
                )
                && !outcome.receiver_fanout_unrepresented
        ));
        assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(
            |outcome| outcome.target_guid == Some(receiver_guid)
                && matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
                )
        ));
    }

    #[tokio::test]
    async fn push_quest_to_party_repeatable_turn_in_command_queue_failure_sends_no_success_like_cpp()
     {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 612);
        let shared_quest_id = 76120;
        let mut quest = quest_template(shared_quest_id);
        quest.quest_type = 0;
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        quest.special_flags |= 0x0000_0001;
        quest.objectives.push(QuestObjective {
            id: 1,
            quest_id: shared_quest_id,
            obj_type: 1,
            order: 0,
            storage_index: 0,
            object_id: 49212,
            amount: 3,
            flags: 0xA5,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        let quest_for_dummy_commands = quest.clone();
        let quest_store = QuestStore::from_quests_like_cpp([quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.sync_player_registry_state_like_cpp();

        for _ in 0..256 {
            receiver_session
                .session_command_tx()
                .try_send(SessionCommand::SetQuestSharingInfoAndSendDetails(
                    SetQuestSharingInfoAndSendDetailsCommand {
                        sender_guid,
                        quest: quest_for_dummy_commands.clone(),
                    },
                ))
                .expect("fill receiver command queue fixture");
        }

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert!(sender_rx.try_recv().is_err());
        assert!(receiver_rx.try_recv().is_err());
        assert_eq!(receiver_session.drain_session_commands().len(), 256);
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(
            |outcome| outcome.target_guid == Some(receiver_guid)
                && matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPromptCommandFailed
                )
                && outcome.receiver_fanout_unrepresented
        ));
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(
            |outcome| outcome.target_guid == Some(sender_guid)
                && matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented
                )
                && outcome.receiver_fanout_unrepresented
        ));
        assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(
            |outcome| outcome.target_guid == Some(receiver_guid)
                && matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPrompted
                )
        ));
    }

    #[tokio::test]
    async fn push_quest_to_party_receiver_unknown_status_after_expansion_emits_invalid_pair_like_cpp()
     {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 609);
        let shared_quest_id = 76090;
        let mut quest = quest_template(shared_quest_id);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        quest.expansion = 2;
        let quest_store = QuestStore::from_quests_like_cpp([quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.expansion = 2;
        add_active_quest_in_slot_with_status(&mut receiver_session, shared_quest_id, 2, 0xFE);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_INVALID_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP,
                "Quest 76090".to_string()
            )
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| outcome.target_guid == Some(receiver_guid) && matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverCanTakeQuestInvalid)));
        assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| outcome.target_guid == Some(receiver_guid) && matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
    }

    #[tokio::test]
    async fn push_quest_to_party_receiver_positive_exclusive_group_active_peer_emits_invalid_pair_like_cpp()
     {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 610);
        let shared_quest_id = 76091;
        let peer_quest_id = 76092;
        let mut shared_quest = quest_template(shared_quest_id);
        shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        shared_quest.exclusive_group = 609;
        let mut peer_quest = quest_template(peer_quest_id);
        peer_quest.exclusive_group = 609;
        let quest_store = QuestStore::from_quests_like_cpp([shared_quest, peer_quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        add_active_quest(&mut receiver_session, peer_quest_id);
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_INVALID_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP,
                "Quest 76091".to_string()
            )
        );
        assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| outcome.target_guid == Some(receiver_guid) && matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverCanTakeQuestInvalid)));
        assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| outcome.target_guid == Some(receiver_guid) && matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
    }

    #[tokio::test]
    async fn push_quest_to_party_prerequisite_precedes_expansion_gate_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 250);
        let shared_quest_id = 7123;
        let mut quest = quest_template(shared_quest_id);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        quest.prev_quest_id = 9001;
        quest.expansion = 2;
        let quest_store = QuestStore::from_quests_like_cpp([quest]);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.expansion = 1;
        receiver_session.sync_player_registry_state_like_cpp();

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
                "Quest 7123".to_string()
            )
        );
        assert!(
            session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite
                ))
        );
        assert!(
            !session
                .represented_push_quest_to_party_outcomes_like_cpp()
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestExpansionRequiredExpansion
                ))
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_grouped_receiver_busy_emits_sender_only_busy_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 45);
        let quest_store = store_with_sharable_quest(7113);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, 7113);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session
            .set_represented_pending_quest_sharing_like_cpp(ObjectGuid::create_player(1, 77), 9000);

        run_push_quest_to_party(&mut session, 7113).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_BUSY_LIKE_CPP,
                String::new()
            )
        );
        assert!(receiver_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn push_quest_to_party_grouped_receiver_dead_emits_dead_pair_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 46);
        let quest_store = store_with_sharable_quest(7114);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, 7114);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.set_player_alive_like_cpp(false);

        run_push_quest_to_party(&mut session, 7114).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_DEAD_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_DEAD_TO_RECIPIENT_LIKE_CPP,
                "Quest 7114".to_string()
            )
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_grouped_receiver_dead_observes_runtime_under_map_sync_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 146);
        let quest_store = store_with_sharable_quest(7114);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, 7114);
        let (player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.set_player_health_like_cpp(1_000, 1_000);
        let mut movement_info = wow_packet::packets::movement::MovementInfo::default();
        movement_info.position.z = -501.0;

        let event = receiver_session.handle_under_map_like_cpp(&movement_info);

        assert!(event.is_some());
        assert!(!receiver_session.player_is_alive_like_cpp());
        assert!(
            !player_registry
                .get(&receiver_guid)
                .expect("receiver registry snapshot")
                .is_alive
        );

        run_push_quest_to_party(&mut session, 7114).await;

        assert_eq!(
            recv_push_quest_result_response(&sender_rx),
            (
                receiver_guid,
                QUEST_PUSH_REASON_DEAD_LIKE_CPP,
                String::new()
            )
        );
        assert_eq!(
            recv_push_quest_result_response(&receiver_rx),
            (
                sender_guid,
                QUEST_PUSH_REASON_DEAD_TO_RECIPIENT_LIKE_CPP,
                "Quest 7114".to_string()
            )
        );
    }

    #[tokio::test]
    async fn push_quest_to_party_missing_group_registry_keeps_explicit_blocker_like_cpp() {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid();
        let quest_store = store_with_sharable_quest(7115);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, 7115);
        session.group_guid = Some(1234);

        run_push_quest_to_party(&mut session, 7115).await;

        assert_eq!(
            session.represented_push_quest_to_party_outcomes_like_cpp(),
            &[RepresentedPushQuestToPartyOutcomeLikeCpp {
                sender_guid,
                quest_id: 7115,
                target_guid: sender_guid,
                reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented,
                quest_pool_active_check_unrepresented: false,
                group_runtime_unrepresented: true,
                receiver_fanout_unrepresented: true,
            }]
        );
        assert!(sender_rx.try_recv().is_err());
    }

    #[test]
    fn push_quest_to_party_registration_and_dispatch_are_wired_like_cpp() {
        let entry = inventory::iter::<PacketHandlerEntry>
            .into_iter()
            .find(|entry| entry.opcode == ClientOpcodes::PushQuestToParty)
            .expect("PushQuestToParty handler registration");

        assert_eq!(entry.status, SessionStatus::LoggedIn);
        assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
        assert_eq!(entry.handler_name, "handle_push_quest_to_party");
        assert!(include_str!("../session.rs").contains("ClientOpcodes::PushQuestToParty =>"));
        assert!(
            include_str!("../session.rs").contains("self.handle_push_quest_to_party(pkt).await")
        );
    }

    #[tokio::test]
    async fn request_world_quest_update_empty_payload_sends_empty_response_like_cpp() {
        let (mut session, send_rx) = make_session();

        run_request_world_quest_update(&mut session).await;

        assert_eq!(recv_world_quest_update_count(&send_rx), 0);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn request_world_quest_update_with_payload_ignores_bytes_and_sends_empty_response_like_cpp()
     {
        let (mut session, send_rx) = make_session();
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint8(1);

        session.handle_request_world_quest_update(pkt).await;

        assert_eq!(recv_world_quest_update_count(&send_rx), 0);
        assert!(send_rx.try_recv().is_err());
    }

    #[test]
    fn request_world_quest_update_inventory_entry_matches_cpp_status_and_processing() {
        let entry = inventory::iter::<PacketHandlerEntry>
            .into_iter()
            .find(|entry| entry.opcode == ClientOpcodes::RequestWorldQuestUpdate)
            .expect("RequestWorldQuestUpdate handler registration");

        assert_eq!(entry.status, SessionStatus::LoggedIn);
        assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
        assert_eq!(entry.handler_name, "handle_request_world_quest_update");
    }

    #[tokio::test]
    async fn quest_giver_status_query_missing_noncanonical_guid_sends_no_packet_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_quest_store(Arc::new(store_with_quests(&[1001])));
        attach_map_manager(&mut session, wow_map::MapManager::default());

        run_status_query(&mut session, creature_guid(9001, 1)).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_giver_status_query_unsupported_player_or_item_guid_sends_no_packet_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_quest_store(Arc::new(store_with_quests(&[1001])));
        attach_map_manager(&mut session, wow_map::MapManager::default());

        run_status_query(&mut session, ObjectGuid::create_player(1, 99)).await;
        run_status_query(&mut session, ObjectGuid::create_item(1, 100)).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_giver_status_query_canonical_creature_starter_sends_available_like_cpp() {
        let (mut session, send_rx) = make_session();
        let mut store = store_with_quests(&[1001]);
        store.starter_quests.entry(9001).or_default().push(1001);
        session.set_quest_store(Arc::new(store));
        let guid = creature_guid(9001, 1);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9001);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::TRIVIAL));
    }

    #[tokio::test]
    async fn quest_giver_status_query_canonical_creature_starter_sends_quest_when_not_trivial_like_cpp()
     {
        let (mut session, send_rx) = make_session();
        let mut quest = quest_template(1006);
        quest.quest_level = 80;
        let mut store = QuestStore::from_quests_like_cpp([quest]);
        store.starter_quests.entry(9006).or_default().push(1006);
        session.set_quest_store(Arc::new(store));
        let guid = creature_guid(9006, 6);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9006);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::QUEST));
    }

    #[tokio::test]
    async fn quest_giver_status_query_uses_configured_low_level_hide_diff_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_quest_low_level_hide_diff_like_cpp(5);
        let mut quest = quest_template(1013);
        quest.quest_level = 75;
        let mut store = QuestStore::from_quests_like_cpp([quest]);
        store.starter_quests.entry(9013).or_default().push(1013);
        session.set_quest_store(Arc::new(store));
        let guid = creature_guid(9013, 13);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9013);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::QUEST));
    }

    #[tokio::test]
    async fn quest_giver_status_query_canonical_creature_starter_sends_future_when_visible_but_low_level_like_cpp()
     {
        let (mut session, send_rx) = make_session();
        let mut quest = quest_template(1007);
        quest.quest_level = 85;
        quest.min_level = 85;
        let mut store = QuestStore::from_quests_like_cpp([quest]);
        store.starter_quests.entry(9007).or_default().push(1007);
        session.set_quest_store(Arc::new(store));
        let guid = creature_guid(9007, 7);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9007);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::FUTURE));
    }

    #[tokio::test]
    async fn quest_giver_status_query_uses_configured_high_level_hide_diff_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_quest_high_level_hide_diff_like_cpp(2);
        let mut quest = quest_template(1014);
        quest.quest_level = 85;
        quest.min_level = 83;
        let mut store = QuestStore::from_quests_like_cpp([quest]);
        store.starter_quests.entry(9014).or_default().push(1014);
        session.set_quest_store(Arc::new(store));
        let guid = creature_guid(9014, 14);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9014);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::NONE));
    }

    #[tokio::test]
    async fn quest_giver_status_query_starter_respects_quest_available_conditions_like_cpp() {
        let (mut session, send_rx) = make_session();
        let quest_id = 1008;
        let mut quest = quest_template(quest_id);
        quest.quest_level = 80;
        let mut store = QuestStore::from_quests_like_cpp([quest]);
        store.starter_quests.entry(9008).or_default().push(quest_id);
        session.set_quest_store(Arc::new(store));
        session.set_condition_store(Arc::new(
            ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
                source_type: ConditionSourceType::QuestAvailable,
                source_entry: quest_id as i32,
                condition_type: ConditionType::Level,
                condition_value1: 90,
                condition_value2: ComparisonType::HighEq as u32,
                ..Condition::default()
            }]),
        ));
        let guid = creature_guid(9008, 8);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9008);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::NONE));
    }

    #[tokio::test]
    async fn quest_giver_status_query_starter_allows_passing_quest_available_conditions_like_cpp() {
        let (mut session, send_rx) = make_session();
        let quest_id = 1009;
        let mut quest = quest_template(quest_id);
        quest.quest_level = 80;
        let mut store = QuestStore::from_quests_like_cpp([quest]);
        store.starter_quests.entry(9009).or_default().push(quest_id);
        session.set_quest_store(Arc::new(store));
        session.set_condition_store(Arc::new(
            ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
                source_type: ConditionSourceType::QuestAvailable,
                source_entry: quest_id as i32,
                condition_type: ConditionType::Level,
                condition_value1: 80,
                condition_value2: ComparisonType::HighEq as u32,
                ..Condition::default()
            }]),
        ));
        let guid = creature_guid(9009, 9);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9009);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::QUEST));
    }

    #[tokio::test]
    async fn quest_giver_status_query_starter_allows_objective_progress_condition_like_cpp() {
        let (mut session, send_rx) = make_session();
        let active_quest_id = 1015;
        let starter_quest_id = 1016;
        let active_quest = quest_template_with_objective_count(active_quest_id, 1);
        let objective_id = active_quest.objectives[0].id;
        let mut starter_quest = quest_template(starter_quest_id);
        starter_quest.quest_level = 80;
        let mut store = QuestStore::from_quests_like_cpp([active_quest, starter_quest]);
        store
            .starter_quests
            .entry(9016)
            .or_default()
            .push(starter_quest_id);
        session.set_quest_store(Arc::new(store));
        session.player_quests.insert(
            active_quest_id,
            PlayerQuestStatus {
                quest_id: active_quest_id,
                status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: vec![2],
                slot: 0,
            },
        );
        session.set_condition_store(Arc::new(
            ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
                source_type: ConditionSourceType::QuestAvailable,
                source_entry: starter_quest_id as i32,
                condition_type: ConditionType::QuestObjectiveProgress,
                condition_value1: objective_id,
                condition_value3: 2,
                ..Condition::default()
            }]),
        ));
        let guid = creature_guid(9016, 16);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9016);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::QUEST));
    }

    #[tokio::test]
    async fn quest_giver_status_query_starter_rejects_objective_progress_mismatch_like_cpp() {
        let (mut session, send_rx) = make_session();
        let active_quest_id = 1017;
        let starter_quest_id = 1018;
        let active_quest = quest_template_with_objective_count(active_quest_id, 1);
        let objective_id = active_quest.objectives[0].id;
        let mut starter_quest = quest_template(starter_quest_id);
        starter_quest.quest_level = 80;
        let mut store = QuestStore::from_quests_like_cpp([active_quest, starter_quest]);
        store
            .starter_quests
            .entry(9018)
            .or_default()
            .push(starter_quest_id);
        session.set_quest_store(Arc::new(store));
        session.player_quests.insert(
            active_quest_id,
            PlayerQuestStatus {
                quest_id: active_quest_id,
                status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: vec![2],
                slot: 0,
            },
        );
        session.set_condition_store(Arc::new(
            ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
                source_type: ConditionSourceType::QuestAvailable,
                source_entry: starter_quest_id as i32,
                condition_type: ConditionType::QuestObjectiveProgress,
                condition_value1: objective_id,
                condition_value3: 3,
                ..Condition::default()
            }]),
        ));
        let guid = creature_guid(9018, 18);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9018);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::NONE));
    }

    #[tokio::test]
    async fn quest_giver_status_query_canonical_creature_completed_ender_sends_can_reward_like_cpp()
    {
        let (mut session, send_rx) = make_session();
        let mut store = store_with_quests(&[1002]);
        store.ender_quests.entry(9002).or_default().push(1002);
        session.set_quest_store(Arc::new(store));
        session.player_quests.insert(
            1002,
            PlayerQuestStatus {
                quest_id: 1002,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );
        let guid = creature_guid(9002, 2);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9002);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(
            recv_status(&send_rx),
            (guid, quest_giver_status::CAN_REWARD)
        );
    }

    #[tokio::test]
    async fn quest_giver_status_query_important_starter_uses_quest_info_modifiers_like_cpp() {
        let (mut session, send_rx) = make_session();
        let quest_id = 1010;
        let mut quest = quest_template(quest_id);
        quest.quest_level = 80;
        quest.quest_info_id = 710;
        let mut store = QuestStore::from_quests_like_cpp([quest]);
        store.starter_quests.entry(9010).or_default().push(quest_id);
        session.set_quest_store(Arc::new(store));
        session.set_quest_info_store(Arc::new(QuestInfoStore::from_entries([
            quest_info_entry_like_cpp(710, 2, 0x400),
        ])));
        let guid = creature_guid(9010, 10);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9010);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(
            recv_status(&send_rx),
            (guid, quest_giver_status::IMPORTANT_QUEST)
        );
    }

    #[tokio::test]
    async fn quest_giver_status_query_important_low_level_uses_future_important_like_cpp() {
        let (mut session, send_rx) = make_session();
        let quest_id = 1011;
        let mut quest = quest_template(quest_id);
        quest.quest_level = 85;
        quest.min_level = 85;
        quest.quest_info_id = 711;
        let mut store = QuestStore::from_quests_like_cpp([quest]);
        store.starter_quests.entry(9011).or_default().push(quest_id);
        session.set_quest_store(Arc::new(store));
        session.set_quest_info_store(Arc::new(QuestInfoStore::from_entries([
            quest_info_entry_like_cpp(711, 2, 0x400),
        ])));
        let guid = creature_guid(9011, 11);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9011);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(
            recv_status(&send_rx),
            (guid, quest_giver_status::FUTURE_IMPORTANT_QUEST)
        );
    }

    #[tokio::test]
    async fn quest_giver_status_query_covenant_completed_ender_uses_quest_info_tag_like_cpp() {
        let (mut session, send_rx) = make_session();
        let quest_id = 1012;
        let mut quest = quest_template(quest_id);
        quest.quest_info_id = 712;
        let mut store = QuestStore::from_quests_like_cpp([quest]);
        store.ender_quests.entry(9012).or_default().push(quest_id);
        session.set_quest_store(Arc::new(store));
        session.set_quest_info_store(Arc::new(QuestInfoStore::from_entries([
            quest_info_entry_like_cpp(712, 15, 0),
        ])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );
        let guid = creature_guid(9012, 12);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9012);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(
            recv_status(&send_rx),
            (
                guid,
                quest_giver_status::COVENANT_CALLING_REWARD_COMPLETE_POI
            )
        );
    }

    #[tokio::test]
    async fn quest_giver_status_query_canonical_gameobject_starter_uses_go_relation_like_cpp() {
        let (mut session, send_rx) = make_session();
        let mut store = store_with_quests(&[1003]);
        assert!(store.insert_gameobject_starter_relation_like_cpp(9103, 1003));
        session.set_quest_store(Arc::new(store));
        let guid = gameobject_guid(9103, 3);
        let mut manager = wow_map::MapManager::default();
        insert_gameobject(&mut manager, guid, 9103);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::TRIVIAL));
    }

    #[tokio::test]
    async fn quest_giver_status_query_canonical_gameobject_completed_ender_uses_go_relation_like_cpp()
     {
        let (mut session, send_rx) = make_session();
        let mut store = store_with_quests(&[1004]);
        assert!(store.insert_gameobject_ender_relation_like_cpp(9104, 1004));
        session.set_quest_store(Arc::new(store));
        session.player_quests.insert(
            1004,
            PlayerQuestStatus {
                quest_id: 1004,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );
        let guid = gameobject_guid(9104, 4);
        let mut manager = wow_map::MapManager::default();
        insert_gameobject(&mut manager, guid, 9104);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(
            recv_status(&send_rx),
            (guid, quest_giver_status::CAN_REWARD)
        );
    }

    #[tokio::test]
    async fn quest_giver_status_query_gameobject_ignores_creature_relation_for_same_entry_like_cpp()
    {
        let (mut session, send_rx) = make_session();
        let mut store = store_with_quests(&[1005]);
        store.starter_quests.entry(9105).or_default().push(1005);
        store.ender_quests.entry(9105).or_default().push(1005);
        session.set_quest_store(Arc::new(store));
        session.player_quests.insert(
            1005,
            PlayerQuestStatus {
                quest_id: 1005,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );
        let guid = gameobject_guid(9105, 5);
        let mut manager = wow_map::MapManager::default();
        insert_gameobject(&mut manager, guid, 9105);
        attach_map_manager(&mut session, manager);

        run_status_query(&mut session, guid).await;

        assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::NONE));
    }

    #[tokio::test]
    async fn quest_giver_status_multiple_empty_visible_set_sends_zero_count_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_quest_store(Arc::new(store_with_quests(&[2001])));
        attach_map_manager(&mut session, wow_map::MapManager::default());

        session.handle_quest_giver_status_multiple_query().await;

        assert!(recv_status_multiple(&send_rx).is_empty());
    }

    #[tokio::test]
    async fn quest_giver_status_multiple_visible_canonical_creature_starter_sends_available_like_cpp()
     {
        let (mut session, send_rx) = make_session();
        let mut store = store_with_quests(&[2002]);
        store.starter_quests.entry(9202).or_default().push(2002);
        session.set_quest_store(Arc::new(store));
        let guid = creature_guid(9202, 202);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9202);
        attach_map_manager(&mut session, manager);
        mark_visible(&mut session, guid);

        session.handle_quest_giver_status_multiple_query().await;

        assert_eq!(
            recv_status_multiple(&send_rx),
            vec![(guid, quest_giver_status::TRIVIAL)]
        );
    }

    #[tokio::test]
    async fn quest_giver_status_multiple_visible_gameobject_starter_uses_go_relation_like_cpp() {
        let (mut session, send_rx) = make_session();
        let mut store = store_with_quests(&[2003]);
        assert!(store.insert_gameobject_starter_relation_like_cpp(9203, 2003));
        store.starter_quests.entry(9203).or_default().push(2999);
        session.set_quest_store(Arc::new(store));
        let guid = gameobject_guid(9203, 203);
        let mut manager = wow_map::MapManager::default();
        insert_gameobject(&mut manager, guid, 9203);
        attach_map_manager(&mut session, manager);
        mark_visible_gameobject_questgiver(&mut session, guid);

        session.handle_quest_giver_status_multiple_query().await;

        assert_eq!(
            recv_status_multiple(&send_rx),
            vec![(guid, quest_giver_status::TRIVIAL)]
        );
    }

    #[tokio::test]
    async fn quest_giver_status_multiple_skips_missing_player_item_and_non_questgiver_go_like_cpp()
    {
        let (mut session, send_rx) = make_session();
        let mut store = store_with_quests(&[2004]);
        store.starter_quests.entry(9204).or_default().push(2004);
        assert!(store.insert_gameobject_starter_relation_like_cpp(9204, 2004));
        session.set_quest_store(Arc::new(store));
        let accepted_guid = creature_guid(9204, 204);
        let missing_guid = creature_guid(9204, 205);
        let player_guid = ObjectGuid::create_player(1, 204);
        let item_guid = ObjectGuid::create_item(1, 204);
        let non_questgiver_go = gameobject_guid(9204, 206);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, accepted_guid, 9204);
        insert_gameobject(&mut manager, non_questgiver_go, 9204);
        attach_map_manager(&mut session, manager);
        for guid in [
            accepted_guid,
            missing_guid,
            player_guid,
            item_guid,
            non_questgiver_go,
        ] {
            mark_visible(&mut session, guid);
        }
        let mut state = crate::session::RepresentedGameObjectUseState::default();
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8);
        session
            .represented_gameobject_use_states
            .insert(non_questgiver_go, state);

        session.handle_quest_giver_status_multiple_query().await;

        assert_eq!(
            recv_status_multiple(&send_rx),
            vec![(accepted_guid, quest_giver_status::TRIVIAL)]
        );
    }

    #[tokio::test]
    async fn quest_giver_close_active_existing_template_records_acknowledge_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_quest_store(Arc::new(store_with_quests(&[5901])));
        add_active_quest(&mut session, 5901);

        run_close_quest(&mut session, 5901).await;

        assert_eq!(
            session.represented_auto_accept_acknowledged_quests_like_cpp,
            vec![5901]
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_giver_close_missing_active_quest_records_no_acknowledge_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_quest_store(Arc::new(store_with_quests(&[5902])));

        run_close_quest(&mut session, 5902).await;

        assert!(
            session
                .represented_auto_accept_acknowledged_quests_like_cpp
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_giver_close_missing_template_records_no_acknowledge_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_quest_store(Arc::new(store_with_quests(&[5904])));
        add_active_quest(&mut session, 5903);

        run_close_quest(&mut session, 5903).await;

        assert!(
            session
                .represented_auto_accept_acknowledged_quests_like_cpp
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_giver_close_short_packet_records_no_acknowledge_and_sends_no_packet_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_quest_store(Arc::new(store_with_quests(&[5905])));
        add_active_quest(&mut session, 5905);

        session
            .handle_quest_giver_close_quest(WorldPacket::from_bytes(&[0x05, 0x17]))
            .await;

        assert!(
            session
                .represented_auto_accept_acknowledged_quests_like_cpp
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[test]
    fn quest_giver_close_inventory_registration_matches_dispatch_contract_like_cpp() {
        let entry = inventory::iter::<PacketHandlerEntry>
            .into_iter()
            .find(|entry| entry.opcode == ClientOpcodes::QuestGiverCloseQuest)
            .expect("QuestGiverCloseQuest handler registration");

        assert_eq!(entry.status, SessionStatus::LoggedIn);
        assert_eq!(entry.processing, PacketProcessing::Inplace);
        assert_eq!(entry.handler_name, "handle_quest_giver_close_quest");
    }

    #[tokio::test]
    async fn quest_log_remove_short_packet_does_not_remove_like_cpp() {
        let (mut session, send_rx) = make_session();
        add_active_quest_in_slot(&mut session, 5911, 0);

        session
            .handle_quest_log_remove_quest(WorldPacket::from_bytes(&[]))
            .await;

        assert!(session.player_quests.contains_key(&5911));
        assert_eq!(session.get_quest_slot_quest_id_like_cpp(0), Some(5911));
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_log_remove_slot_outside_max_does_not_remove_like_cpp() {
        let (mut session, send_rx) = make_session();
        add_active_quest_in_slot(&mut session, 5912, 0);

        run_remove_quest_slot(&mut session, 25).await;

        assert!(session.player_quests.contains_key(&5912));
        assert_eq!(session.get_quest_slot_quest_id_like_cpp(0), Some(5912));
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_log_remove_valid_slot_removes_only_that_slot_like_cpp() {
        let (mut session, send_rx) = make_session();
        add_active_quest_in_slot(&mut session, 880_001, 7);
        add_active_quest_in_slot(&mut session, 17, 3);

        run_remove_quest_slot(&mut session, 7).await;

        assert!(!session.player_quests.contains_key(&880_001));
        assert!(session.player_quests.contains_key(&17));
        assert_eq!(session.get_quest_slot_quest_id_like_cpp(7), None);
        assert_eq!(session.get_quest_slot_quest_id_like_cpp(3), Some(17));
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_log_remove_empty_valid_slot_does_not_remove_other_quest_like_cpp() {
        let (mut session, send_rx) = make_session();
        add_active_quest_in_slot(&mut session, 5914, 4);

        run_remove_quest_slot(&mut session, 3).await;

        assert!(session.player_quests.contains_key(&5914));
        assert_eq!(session.get_quest_slot_quest_id_like_cpp(4), Some(5914));
        assert_eq!(session.get_quest_slot_quest_id_like_cpp(3), None);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_log_remove_duplicate_slot_fails_closed_and_removes_none_like_cpp() {
        let (mut session, send_rx) = make_session();
        add_active_quest_in_slot(&mut session, 5915, 2);
        add_active_quest_in_slot(&mut session, 5916, 2);

        run_remove_quest_slot(&mut session, 2).await;

        assert!(session.player_quests.contains_key(&5915));
        assert!(session.player_quests.contains_key(&5916));
        assert_eq!(session.get_quest_slot_quest_id_like_cpp(2), None);
        assert_eq!(session.first_free_quest_slot_like_cpp(), Some(0));
        assert!(send_rx.try_recv().is_err());
    }

    #[test]
    fn quest_log_create_entries_preserve_explicit_slot_holes_like_cpp() {
        let (mut session, _send_rx) = make_session();
        add_active_quest_in_slot(&mut session, 5916, 9);
        add_active_quest_in_slot(&mut session, 5915, 2);

        let entries = session.quest_log_create_entries_like_cpp();

        assert_eq!(entries.len(), MAX_QUEST_LOG_SIZE_LIKE_CPP as usize);
        assert_eq!(entries[0], (0, 0, 0, [0; 24]));
        assert_eq!(entries[2].0, 5915);
        assert_eq!(entries[9].0, 5916);
    }

    #[test]
    fn quest_log_create_entries_duplicate_slot_is_empty_fail_closed_like_cpp() {
        let (mut session, _send_rx) = make_session();
        add_active_quest_in_slot(&mut session, 5915, 2);
        add_active_quest_in_slot(&mut session, 5916, 2);

        let entries = session.quest_log_create_entries_like_cpp();

        assert_eq!(entries.len(), MAX_QUEST_LOG_SIZE_LIKE_CPP as usize);
        assert_eq!(entries[2], (0, 0, 0, [0; 24]));
        assert!(session.player_quests.contains_key(&5915));
        assert!(session.player_quests.contains_key(&5916));
    }

    #[test]
    fn quest_log_remove_inventory_registration_and_dispatcher_contract_like_cpp() {
        let entry = inventory::iter::<PacketHandlerEntry>
            .into_iter()
            .find(|entry| entry.opcode == ClientOpcodes::QuestLogRemoveQuest)
            .expect("QuestLogRemoveQuest handler registration");

        assert_eq!(entry.status, SessionStatus::LoggedIn);
        assert_eq!(entry.processing, PacketProcessing::Inplace);
        assert_eq!(entry.handler_name, "handle_quest_log_remove_quest");
        assert!(include_str!("../session.rs").contains("ClientOpcodes::QuestLogRemoveQuest =>"));
        assert!(
            include_str!("../session.rs").contains("self.handle_quest_log_remove_quest(pkt).await")
        );
    }

    #[test]
    fn can_take_quest_blocks_when_quest_available_condition_not_met_like_cpp() {
        // NEGATIVA: nivel requerido 90, jugador nivel 80 → condición no se cumple.
        let (mut session, _send_rx) = make_session();
        let quest_id = 7570u32;
        let quest = quest_template(quest_id);
        let store = QuestStore::from_quests_like_cpp([quest.clone()]);
        session.set_quest_store(Arc::new(store));
        session.set_condition_store(Arc::new(
            ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
                source_type: ConditionSourceType::QuestAvailable,
                source_entry: quest_id as i32,
                condition_type: ConditionType::Level,
                condition_value1: 90,
                condition_value2: ComparisonType::HighEq as u32,
                ..Condition::default()
            }]),
        ));

        // Sin la condición la quest sería aceptable (nivel 1, min_level 1, raza/clase sin filtro).
        // Con la condición de nivel 90, el jugador (80) no la cumple.
        assert!(!session.can_take_quest(&quest));

        // POSITIVA: nivel requerido 80 (alcanzable para el jugador en nivel 80).
        let quest_id2 = 7571u32;
        let quest2 = quest_template(quest_id2);
        let store2 = QuestStore::from_quests_like_cpp([quest2.clone()]);
        session.set_quest_store(Arc::new(store2));
        session.set_condition_store(Arc::new(
            ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
                source_type: ConditionSourceType::QuestAvailable,
                source_entry: quest_id2 as i32,
                condition_type: ConditionType::Level,
                condition_value1: 80,
                condition_value2: ComparisonType::HighEq as u32,
                ..Condition::default()
            }]),
        ));

        assert!(session.can_take_quest(&quest2));
    }

    #[test]
    fn can_take_quest_blocks_when_session_expansion_below_required_like_cpp() {
        // NEGATIVA: expansión de sesión 1 < expansión requerida 2 → rechaza.
        let (mut session, _send_rx) = make_session();
        let mut quest = quest_template(9900u32);
        quest.expansion = 2;
        let store = QuestStore::from_quests_like_cpp([quest.clone()]);
        session.set_quest_store(Arc::new(store));
        session.expansion = 1;
        assert!(!session.can_take_quest(&quest));

        // POSITIVA límite: expansión de sesión == expansión requerida → acepta.
        let (mut session2, _send_rx2) = make_session();
        let mut quest2 = quest_template(9901u32);
        quest2.expansion = 2;
        let store2 = QuestStore::from_quests_like_cpp([quest2.clone()]);
        session2.set_quest_store(Arc::new(store2));
        session2.expansion = 2;
        assert!(session2.can_take_quest(&quest2));
    }

    // ── SatisfyQuestExclusiveGroup tests ────────────────────────────────────

    #[test]
    fn can_take_quest_exclusive_group_blocks_when_peer_rewarded_non_repeatable_like_cpp() {
        // NEGATIVA: peer del mismo exclusive_group (>0) ya rewarded (no repetible)
        // → Player.cpp:15379 segundo término: !(repeatable && repeatable) && rewarded → false.
        let (mut session, _send_rx) = make_session();

        let mut peer = quest_template(9910u32);
        peer.exclusive_group = 5;
        // is_repeatable() false por defecto (quest_type=2, special_flags=0)

        let mut quest = quest_template(9911u32);
        quest.exclusive_group = 5;

        let store = QuestStore::from_quests_like_cpp([peer.clone(), quest.clone()]);
        session.set_quest_store(Arc::new(store));

        // El peer ya fue recompensado (no repetible).
        session.rewarded_quests.insert(peer.id);

        // La quest objetivo no está ni activa ni rewarded.
        assert!(!session.can_take_quest(&quest));
    }

    #[test]
    fn can_take_quest_exclusive_group_blocks_when_peer_active_like_cpp() {
        // NEGATIVA-2: peer del mismo exclusive_group activo en player_quests (status != NONE)
        // → Player.cpp:15379 primer término: GetQuestStatus(peer) != NONE → false.
        let (mut session, _send_rx) = make_session();

        let mut peer = quest_template(9912u32);
        peer.exclusive_group = 7;

        let mut quest = quest_template(9913u32);
        quest.exclusive_group = 7;

        let store = QuestStore::from_quests_like_cpp([peer.clone(), quest.clone()]);
        session.set_quest_store(Arc::new(store));

        // El peer está activo (Incomplete).
        add_active_quest(&mut session, peer.id);

        // La quest objetivo aún no ha sido aceptada.
        assert!(!session.can_take_quest(&quest));
    }

    #[test]
    fn can_take_quest_exclusive_group_positive_no_conflicting_peer_allows_like_cpp() {
        // POSITIVA: exclusive_group > 0 pero ningún peer está activo ni rewarded → true.
        let (mut session, _send_rx) = make_session();

        let mut peer = quest_template(9914u32);
        peer.exclusive_group = 9;

        let mut quest = quest_template(9915u32);
        quest.exclusive_group = 9;

        let store = QuestStore::from_quests_like_cpp([peer.clone(), quest.clone()]);
        session.set_quest_store(Arc::new(store));

        // Ningún peer activo ni rewarded → debe permitir.
        assert!(session.can_take_quest(&quest));
    }

    #[test]
    fn can_take_quest_exclusive_group_zero_never_blocks_like_cpp() {
        // POSITIVA: exclusive_group <= 0 → Player.cpp:15351 → siempre true.
        let (mut session, _send_rx) = make_session();

        let mut quest = quest_template(9916u32);
        quest.exclusive_group = 0;

        let store = QuestStore::from_quests_like_cpp([quest.clone()]);
        session.set_quest_store(Arc::new(store));

        assert!(session.can_take_quest(&quest));

        // También verifica group negativo.
        let (mut session2, _send_rx2) = make_session();
        let mut quest2 = quest_template(9917u32);
        quest2.exclusive_group = -3;

        let store2 = QuestStore::from_quests_like_cpp([quest2.clone()]);
        session2.set_quest_store(Arc::new(store2));

        assert!(session2.can_take_quest(&quest2));
    }
}

// ── PlayerQuestStatus ────────────────────────────────────────────────────────

/// Tracks one active quest for a player.
#[derive(Debug, Clone)]
pub struct PlayerQuestStatus {
    pub quest_id: u32,
    /// 0=None, 1=Incomplete, 2=Complete, 3=Failed
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
