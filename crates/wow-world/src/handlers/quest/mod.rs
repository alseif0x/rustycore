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

mod eligibility;
mod handlers;
mod objectives;
mod persistence;
mod rewards;
mod sharing;
mod state;

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
        QUEST_PACKAGE_FILTER_LOOT_SPECIALIZATION_LIKE_CPP, QuestInfoEntry, QuestPackageItemEntry,
    },
    quest::QuestStore,
    reputation::reputation_rank_from_standing_like_cpp as reputation_rank_from_standing_data_like_cpp,
};
pub use wow_entities::PlayerQuestStatusRecord as PlayerQuestStatus;
use wow_entities::{
    ItemPosCount, SendNewItemDelivery, SendNewItemDisplayText, SendNewItemInstancePlan,
    SendNewItemModifier, SendNewItemPlan, is_bag_pos,
};
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ServerPacket;
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
    receiver: &crate::session::directory::PlayerQuestSharingSnapshot,
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

fn build_quest_poi_store_like_cpp(
    point_rows: Vec<wow_persistence::QuestPoiPointLoadRowLikeCpp>,
    poi_rows: Vec<wow_persistence::QuestPoiBlobLoadRowLikeCpp>,
) -> HashMap<i32, QuestPoiData> {
    let mut all_points: HashMap<(i32, i32), Vec<QuestPoiBlobPoint>> = HashMap::new();
    for row in point_rows {
        all_points
            .entry((row.quest_id, row.idx1))
            .or_default()
            .push(QuestPoiBlobPoint {
                x: row.x,
                y: row.y,
                z: row.z,
            });
    }

    let mut store: HashMap<i32, QuestPoiData> = HashMap::new();
    for row in poi_rows {
        let Some(points) = all_points.get(&(row.quest_id, row.idx1)).cloned() else {
            debug!(
                quest_id = row.quest_id,
                blob_index = row.blob_index,
                "quest_poi references unknown quest points like C++; skipping blob"
            );
            continue;
        };

        store
            .entry(row.quest_id)
            .or_insert_with(|| QuestPoiData {
                quest_id: row.quest_id,
                blobs: Vec::new(),
            })
            .blobs
            .push(QuestPoiBlobData {
                blob_index: row.blob_index,
                objective_index: row.objective_index,
                quest_objective_id: row.quest_objective_id,
                quest_object_id: row.quest_object_id,
                map_id: row.map_id,
                ui_map_id: row.ui_map_id,
                priority: row.priority,
                flags: row.flags,
                world_effect_id: row.world_effect_id,
                player_condition_id: row.player_condition_id,
                navigation_player_condition_id: row.navigation_player_condition_id,
                spawn_tracking_id: row.spawn_tracking_id,
                points,
                always_allow_merging_blobs: row.always_allow_merging_blobs,
            });
    }

    store
}

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

pub(crate) const MAX_QUEST_LOG_SIZE_LIKE_CPP: u8 = 25;

#[cfg(test)]
#[path = "../quest_tests.rs"]
mod tests;

// ── PlayerQuestStatus ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct ItemTransferQuestPersistencePlanLikeCpp {
    statuses: HashMap<u32, PlayerQuestStatus>,
    changed_quest_ids: Vec<u32>,
}
