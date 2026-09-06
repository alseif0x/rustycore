// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot handlers, organised by feature.
//!
//! Issue #224 split the former 13,606-line `handlers/loot.rs` into private
//! feature modules. The logical owner, every registration, opcode and
//! dispatcher arm are unchanged; this module keeps the shared constants,
//! helper types and free functions the features build on.

mod authority;
mod claims;
mod fanout;
mod generation;
mod handlers;
mod money;
mod persistence;
mod requests;
mod rolls;
mod sources;

use std::collections::{HashMap, HashSet};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use rand::{
    Rng,
    distributions::{Distribution, WeightedIndex},
};
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::session::directory::{PlayerRegistry, PrepareLootMoneyApplicationLikeCpp};
use crate::session::mailbox::{
    ApplyCreatureMeleeDamageLikeCppCommand, ApplyGroupJoinLikeCppCommand,
    ApplyGroupRemovalLikeCppCommand, ApplyLootMoneyLikeCppCommand, ApplyLootMoneyResultLikeCpp,
    CancelRepresentedTradeLikeCppCommand, CreatureAttackStartLikeCppCommand,
    CreatureAttackStopLikeCppCommand, KickLikeCppCommand, LootRollCommandIdentityLikeCpp,
    LootRollStoreWinnerCommand, LootRollVoteCommand, MasterLootGiveCommand, MasterLootGiveResult,
    NotifyLootMoneyRemovedLikeCppCommand, ReconcilePvpCombatExpiryLikeCppCommand,
    RefreshVisibleWorldCreaturesLikeCppCommand, SendAddonIfRegisteredLikeCppCommand,
    SendCreatureLootReleaseValuesUpdateLikeCppCommand,
    SendCreatureSpellCastIfVisibleLikeCppCommand, SendIfVisibleLikeCppCommand,
    SendPartyUpdateLikeCppCommand, SendRepeatableTurnInRequestItemsLikeCppCommand,
    SendRepresentedDuelCountdownLikeCppCommand, SendRepresentedDuelRequestedLikeCppCommand,
    SendRepresentedTradeStatusLikeCppCommand, SessionCommand,
    SetQuestSharingInfoAndSendDetailsCommand, SyncChestGameobjectStateAndRefreshLikeCppCommand,
    SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    SyncGooberGameobjectStateAndRefreshLikeCppCommand, UnacceptRepresentedTradeLikeCppCommand,
};
use wow_constants::{
    ClientOpcodes, InventoryResult, InventoryType, ItemContext, ItemFieldFlags, ItemFlags,
    ItemFlags2, ItemQuality, UnitDynFlags,
};
use wow_core::{ObjectGuid, guid::HighGuid};
use wow_data::{ItemRandomEnchantmentTemplateEntry, ItemRandomPropertyTemplateEntry};
use wow_entities::{
    AccessorObjectKind, CORPSE_DYNFLAG_LOOTABLE, GAMEOBJECT_TYPE_AREADAMAGE,
    GAMEOBJECT_TYPE_BARBER_CHAIR, GAMEOBJECT_TYPE_BINDER, GAMEOBJECT_TYPE_CAMERA,
    GAMEOBJECT_TYPE_CHAIR, GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_DESTRUCTIBLE_BUILDING,
    GAMEOBJECT_TYPE_DOOR, GAMEOBJECT_TYPE_DUNGEON_DIFFICULTY, GAMEOBJECT_TYPE_FISHING_HOLE,
    GAMEOBJECT_TYPE_FISHING_NODE, GAMEOBJECT_TYPE_FLAGDROP, GAMEOBJECT_TYPE_FLAGSTAND,
    GAMEOBJECT_TYPE_GATHERING_NODE, GAMEOBJECT_TYPE_GOOBER, GAMEOBJECT_TYPE_GUILD_BANK,
    GAMEOBJECT_TYPE_MAILBOX, GAMEOBJECT_TYPE_MAP_OBJECT, GAMEOBJECT_TYPE_MINI_GAME,
    GAMEOBJECT_TYPE_QUESTGIVER, GAMEOBJECT_TYPE_TEXT, GO_DYNFLAG_LO_NO_INTERACT,
    GameObjectLootSource, GatheringNodeUseSource, GoState, INVENTORY_DEFAULT_SIZE,
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_END, INVENTORY_SLOT_ITEM_START, Item, ItemPosCount,
    LootState, MAX_MONEY_AMOUNT, is_bag_pos, make_item_pos,
};
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_loot::{
    GeneratedLootItem, LootClaimCommitError, LootClaimError, LootClaimLease, LootClaimPayload,
    LootConditionId, LootConditionRowLikeCpp, LootFillError, LootFillOptions,
    LootItemRandomProperties, LootItemTemplateMetadata, LootStoreItem, LootStoreItemContext,
    LootStoreKind, LootTemplate, OwnedLootAuthority, OwnedLootAuthorityLifecycle, OwnedLootScope,
    OwnedLootSnapshot, condition_compare_values_like_cpp, loot_condition_reference_ids_like_cpp,
    loot_condition_reference_self_references_like_cpp,
    loot_condition_row_normalize_without_external_stores_like_cpp,
    loot_conditions_allow_player_with_references_like_cpp_representable,
    loot_item_ui_type_for_player_like_cpp,
};
use wow_packet::ServerPacket;
use wow_packet::packets::item::{
    ItemExpirePurchaseRefund, ItemInstance, ItemModList, ItemPushResult, ItemPushResultDisplayType,
};
use wow_packet::packets::loot::{
    AELootTargets, AELootTargetsAck, CoinRemoved, CreatureLoot, LOOT_ERROR_DIDNT_KILL_LIKE_CPP,
    LOOT_ERROR_MASTER_INV_FULL_LIKE_CPP, LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
    LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP, LOOT_ERROR_NO_LOOT_LIKE_CPP,
    LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP, LOOT_ERROR_TOO_FAR_LIKE_CPP,
    LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP, LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
    LOOT_TYPE_CHEST_LIKE_CPP, LOOT_TYPE_CORPSE_LIKE_CPP, LOOT_TYPE_DISENCHANTING_LIKE_CPP,
    LOOT_TYPE_FISHING_JUNK_LIKE_CPP, LOOT_TYPE_FISHING_LIKE_CPP, LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
    LOOT_TYPE_INSIGNIA_LIKE_CPP, LOOT_TYPE_MILLING_LIKE_CPP, LOOT_TYPE_PROSPECTING_LIKE_CPP,
    LOOT_TYPE_SKINNING_LIKE_CPP, LootAllPassed, LootEntry, LootEntryFlags, LootItemData,
    LootItemPkt, LootList, LootMoney, LootMoneyNotify, LootRelease, LootReleaseAll, LootRemoved,
    LootResponse, LootRoll, LootRollBroadcast, LootRollWon, LootUnit, MasterLootCandidateList,
    MasterLootItem, NotNormalLootItem, SLootRelease, SetLootSpecialization, StartLootRoll,
};
use wow_packet::packets::update::{ItemCreateData, ItemEnchantmentValuesUpdate, UpdateObject};
use wow_persistence::{
    PersistenceOutcomeLikeCpp, StoredItemMoneyPersistenceAttemptLikeCpp,
    StoredItemMoneyPersistenceRequestLikeCpp, StoredItemMoneyReconciliationLikeCpp,
    StoredItemMoneyRollbackKindLikeCpp,
};
#[cfg(test)]
use wow_persistence::{
    STORED_ITEM_MONEY_SOURCE_ROWS_EXPECTED_LIKE_CPP, StoredItemMoneyPersistenceOutcomeLikeCpp,
    classify_stored_item_money_reconciliation_like_cpp,
    stored_item_money_zero_without_source_outcome_like_cpp,
};

use crate::conditions::{
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    QUEST_STATUS_NONE_LIKE_CPP, QUEST_STATUS_REWARDED_LIKE_CPP,
};
use crate::session::{
    DurableItemLootCompletionLikeCpp, DurableItemLootPersistenceGuardLikeCpp,
    DurableLootItemFanoutLikeCpp, InventoryItem, ItemValuationCatalogsLikeCpp,
    LootMoneyDeliveryAddressLikeCpp, LootMoneyPersistenceErrorLikeCpp,
    LootMoneyViewerFanoutLikeCpp, RepresentedGameObjectSpellCaster, RepresentedGameObjectUseEffect,
    RepresentedLootRollState, RepresentedLootRollVote,
    RepresentedQuestObjectiveProgressEventLikeCpp, SessionState, WorldSession,
    loot_money_durable_outcome_like_cpp,
};

const LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP: u8 = 0;
const LOOT_METHOD_ROUND_ROBIN_LIKE_CPP: u8 = 1;
const LOOT_METHOD_MASTER_LIKE_CPP: u8 = 2;
const LOOT_METHOD_GROUP_LIKE_CPP: u8 = 3;
const LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP: u8 = 4;
const LOOT_METHOD_PERSONAL_LIKE_CPP: u8 = 5;
const MAX_NR_LOOT_ITEMS_LIKE_CPP: usize = 18;
const LOOT_ROLL_TIMEOUT_MS_LIKE_CPP: u32 = 60_000;
#[cfg(test)]
const ROLL_ALL_TYPE_NO_DISENCHANT_LIKE_CPP: u8 = 0x07;
const ROLL_ALL_TYPE_MASK_LIKE_CPP: u8 = 0x0F;
const ROLL_FLAG_TYPE_NEED_LIKE_CPP: u8 = 0x02;
const ROLL_FLAG_TYPE_DISENCHANT_LIKE_CPP: u8 = 0x08;
const LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP: u8 = 0;
const LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP: u8 = 1;
const LOOT_SLOT_TYPE_LOCKED_LIKE_CPP: u8 = 2;
const DISENCHANT_LOOT_ROLL_CRITERIA_SPELL_LIKE_CPP: u32 = 13_262;
const LOOT_MODE_DEFAULT_LIKE_CPP: u16 = 0x01;
const LOOT_MODE_JUNK_FISH_LIKE_CPP: u16 = 0x8000;
const ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP: u32 = 0x0004;
const ITEM_FLAGS_CU_IGNORE_QUEST_STATUS_LIKE_CPP: u32 = 0x0002;
const MAX_LOOT_REFERENCE_FRAMES_LIKE_CPP: u32 = 64;
const ROLL_VOTE_PASS_LIKE_CPP: u8 = 0;
const ROLL_VOTE_NEED_LIKE_CPP: u8 = 1;
const ROLL_VOTE_GREED_LIKE_CPP: u8 = 2;
const ROLL_VOTE_DISENCHANT_LIKE_CPP: u8 = 3;
const ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP: u8 = 4;
const ROLL_VOTE_NOT_VALID_LIKE_CPP: u8 = 5;
const CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP: i32 = 51;
const CONDITION_TYPE_MASK_LIKE_CPP: i32 = 52;
const TYPEID_PLAYER_LIKE_CPP: u32 = 6;
const PLAYER_TYPE_MASK_LIKE_CPP: u32 = 0x0001 | 0x0020 | 0x0040;
const LOCK_KEY_SKILL_LIKE_CPP: u8 = 2;
const LOCK_KEY_SPELL_LIKE_CPP: u8 = 3;
const SPELL_EFFECT_OPEN_LOCK_LIKE_CPP: u32 = 33;
const REMOTE_MASTER_LOOT_COMMAND_TIMEOUT: Duration = Duration::from_millis(250);

#[derive(Clone)]
struct AuthoritativeLootReleaseLikeCpp {
    authority: OwnedLootAuthority,
    selected_generation: u64,
    loot: CreatureLoot,
    whole_object_fully_looted: bool,
    whole_object_fully_skinned: bool,
    object_generation: u64,
    lifecycle_revision: u64,
    require_no_viewers: bool,
}

// ── Handler registrations ─────────────────────────────────────────

// ── Handler implementations ───────────────────────────────────────

fn durable_loot_item_fanout_viewers_like_cpp(
    precommit_viewers: &[ObjectGuid],
    committed_viewers: &[ObjectGuid],
) -> HashSet<ObjectGuid> {
    precommit_viewers
        .iter()
        .chain(committed_viewers)
        .copied()
        .collect()
}

fn master_loot_error_for_inventory_result_like_cpp(result: InventoryResult) -> Option<u8> {
    match result {
        InventoryResult::Ok => None,
        InventoryResult::ItemMaxCount => Some(LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP),
        InventoryResult::InvFull => Some(LOOT_ERROR_MASTER_INV_FULL_LIKE_CPP),
        _ => Some(LOOT_ERROR_MASTER_OTHER_LIKE_CPP),
    }
}

// ── Loot generation ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ItemTemplateAddonLootMetadataLikeCpp {
    flags_cu: u32,
    quest_log_item_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepresentedLootPlayerContext {
    race: u8,
    class: u8,
    gender: u8,
    level: u8,
    known_spells: Vec<i32>,
    active_quest_statuses: HashMap<u32, u8>,
    active_quest_objective_counts: HashMap<u32, Vec<i32>>,
    rewarded_quests: HashSet<u32>,
    inventory_item_counts: HashMap<u32, u32>,
    is_current: bool,
}

impl RepresentedLootPlayerContext {
    fn quest_status(&self, quest_id: u32) -> u8 {
        self.active_quest_statuses
            .get(&quest_id)
            .copied()
            .or_else(|| {
                self.rewarded_quests
                    .contains(&quest_id)
                    .then_some(QUEST_STATUS_REWARDED_LIKE_CPP)
            })
            .unwrap_or(QUEST_STATUS_NONE_LIKE_CPP)
    }

    fn inventory_item_count(&self, item_id: u32) -> u32 {
        self.inventory_item_counts
            .get(&item_id)
            .copied()
            .unwrap_or(0)
    }
}

impl ItemTemplateAddonLootMetadataLikeCpp {
    fn ignores_quest_status(self) -> bool {
        self.flags_cu & ITEM_FLAGS_CU_IGNORE_QUEST_STATUS_LIKE_CPP != 0
    }

    fn follows_loot_rules(self) -> bool {
        self.flags_cu & ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP != 0
    }
}

fn player_class_mask_like_cpp(class_id: u8) -> Option<u32> {
    if (1..=13).contains(&class_id) {
        Some(1_u32 << (class_id - 1))
    } else {
        None
    }
}

fn player_race_mask_like_cpp(race_id: u8) -> Option<u32> {
    let bit = match race_id {
        1..=11 => race_id - 1,
        22 => 21,
        24..=32 => race_id - 1,
        34 => 11,
        35 => 12,
        36 => 13,
        37 => 14,
        52 => 16,
        70 => 15,
        _ => return None,
    };
    Some(1_u32 << bit)
}

fn player_team_for_race_cpp_representable(race: u8) -> u32 {
    match race {
        2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 31 | 35 | 36 | 70 => 67,
        _ => 469,
    }
}

fn represented_item_faction_flags_block_player_like_cpp(flags2: Option<u32>, race: u8) -> bool {
    let Some(flags2) = flags2 else {
        return false;
    };

    let team = player_team_for_race_cpp_representable(race);
    ((flags2 & ItemFlags2::FactionHorde as u32) != 0 && team != 67)
        || ((flags2 & ItemFlags2::FactionAlliance as u32) != 0 && team != 469)
}

fn player_quest_status_mask_like_cpp(status: Option<u8>, rewarded: bool) -> u32 {
    if rewarded {
        return 0x40;
    }

    match status {
        None => 0x01,
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP) => 0x02,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP) => 0x08,
        Some(QUEST_STATUS_FAILED_LIKE_CPP) => 0x20,
        _ => 0,
    }
}

fn generated_creature_loot_item_to_entry_like_cpp(
    item: GeneratedLootItem,
    addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
) -> LootEntry {
    LootEntry {
        loot_list_id: item.loot_list_id as u8,
        item_id: item.item_id,
        quantity: item.count,
        random_properties_id: item.random_properties_id,
        random_properties_seed: item.random_properties_seed,
        item_context: item.context,
        flags: LootEntryFlags {
            follow_loot_rules: !item.needs_quest || addon_metadata.follows_loot_rules(),
            freeforall: item.free_for_all,
            blocked: item.is_blocked,
            counted: item.is_counted,
            under_threshold: item.is_under_threshold,
            needs_quest: item.needs_quest,
        },
        allowed_looters: Vec::new(),
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: item.is_looted,
    }
}

fn generated_shared_gameobject_loot_item_to_entry_like_cpp<FAllowed>(
    item: GeneratedLootItem,
    addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
    allowed_looters: &[ObjectGuid],
    mut item_allowed_for_player: FAllowed,
) -> LootEntry
where
    FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
{
    let store_item_context = item.store_item_context;
    let mut entry = generated_creature_loot_item_to_entry_like_cpp(item, addon_metadata);
    for looter in allowed_looters {
        if item_allowed_for_player(store_item_context, *looter) {
            entry.add_allowed_looter_like_cpp(*looter);
        }
    }
    entry
}

#[derive(Debug, Clone)]
struct RepresentedCreatureLootStateLikeCpp {
    is_alive: bool,
    position: wow_core::Position,
    level: u8,
    entry: u32,
    loot_id: u32,
    gold_min: u32,
    gold_max: u32,
    dungeon_encounter_id: u32,
    tappers: Vec<ObjectGuid>,
    loot_lifecycle_revision: u64,
}

#[derive(Debug, Clone)]
struct RepresentedGameObjectLootInstallObservationLikeCpp {
    authority: OwnedLootAuthority,
    object_generation: u64,
    loot_lifecycle_revision: u64,
}

#[derive(Debug, Clone, Copy)]
struct RepresentedGameObjectLootStateLikeCpp {
    position: Option<wow_core::Position>,
    display_id: Option<u32>,
    scale: f32,
    rotation: [f32; 4],
    go_type: Option<u8>,
    interact_radius_override: Option<u32>,
    lock_id: Option<u32>,
    owner_guid: Option<ObjectGuid>,
}

pub(crate) fn represented_gameobject_interaction_distance_like_cpp(
    go_type: Option<u8>,
    interact_radius_override: Option<u32>,
) -> f32 {
    // C++ ref: GameObject.cpp GetInteractionDistance().
    // Spell-lock range remains with the typed GameObject/SpellInfo port.
    if let Some(override_hundredths) = interact_radius_override.filter(|value| *value != 0) {
        return override_hundredths as f32 / 100.0;
    }

    match go_type.map(u32::from) {
        Some(GAMEOBJECT_TYPE_AREADAMAGE) => 0.0,
        Some(GAMEOBJECT_TYPE_QUESTGIVER)
        | Some(GAMEOBJECT_TYPE_TEXT)
        | Some(GAMEOBJECT_TYPE_FLAGSTAND)
        | Some(GAMEOBJECT_TYPE_FLAGDROP)
        | Some(GAMEOBJECT_TYPE_MINI_GAME) => 5.5555553,
        Some(GAMEOBJECT_TYPE_BINDER) => 10.0,
        Some(GAMEOBJECT_TYPE_CHAIR) | Some(GAMEOBJECT_TYPE_BARBER_CHAIR) => 3.0,
        Some(GAMEOBJECT_TYPE_FISHING_NODE) => 100.0,
        Some(GAMEOBJECT_TYPE_FISHING_HOLE) => 20.0 + wow_movement::CONTACT_DISTANCE_LIKE_CPP,
        Some(GAMEOBJECT_TYPE_CAMERA)
        | Some(GAMEOBJECT_TYPE_MAP_OBJECT)
        | Some(GAMEOBJECT_TYPE_DUNGEON_DIFFICULTY)
        | Some(GAMEOBJECT_TYPE_DESTRUCTIBLE_BUILDING)
        | Some(GAMEOBJECT_TYPE_DOOR) => 5.0,
        Some(GAMEOBJECT_TYPE_GUILD_BANK) | Some(GAMEOBJECT_TYPE_MAILBOX) => 10.0,
        _ => 5.0,
    }
}

fn represented_gameobject_display_box_contains_like_cpp(
    go_position: wow_core::Position,
    player_position: wow_core::Position,
    display_info: &wow_data::GameObjectDisplayInfoEntry,
    scale: f32,
    rotation: [f32; 4],
    radius: f32,
) -> bool {
    let min_x = display_info.geo_box_min.x * scale - radius;
    let min_y = display_info.geo_box_min.y * scale - radius;
    let min_z = display_info.geo_box_min.z * scale - radius;
    let max_x = display_info.geo_box_max.x * scale + radius;
    let max_y = display_info.geo_box_max.y * scale + radius;
    let max_z = display_info.geo_box_max.z * scale + radius;

    let dx = player_position.x - go_position.x;
    let dy = player_position.y - go_position.y;
    let dz = player_position.z - go_position.z;
    let [qx, qy, qz, qw] = rotation;
    let iqx = -qx;
    let iqy = -qy;
    let iqz = -qz;

    let tx = 2.0 * (iqy * dz - iqz * dy);
    let ty = 2.0 * (iqz * dx - iqx * dz);
    let tz = 2.0 * (iqx * dy - iqy * dx);
    let local_x = dx + qw * tx + (iqy * tz - iqz * ty);
    let local_y = dy + qw * ty + (iqz * tx - iqx * tz);
    let local_z = dz + qw * tz + (iqx * ty - iqy * tx);

    local_x >= min_x
        && local_x <= max_x
        && local_y >= min_y
        && local_y <= max_y
        && local_z >= min_z
        && local_z <= max_z
}

#[cfg(test)]
fn represented_loot_object_guid_like_cpp(owner: ObjectGuid) -> ObjectGuid {
    if owner.is_empty() {
        return ObjectGuid::EMPTY;
    }

    ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner.realm_id(),
        owner.map_id(),
        0,
        0,
        owner.counter(),
    )
}

/// Unit fixtures that predate canonical map objects may exercise packet-cache
/// behavior locally. This is a compile-time false branch in production: live
/// Creature/GameObject claims fail closed without their map-owned authority.
const fn represented_local_loot_fixture_allowed_like_cpp() -> bool {
    cfg!(test)
}

fn loot_type_for_client_like_cpp(loot_type: u8) -> u8 {
    match loot_type {
        LOOT_TYPE_PROSPECTING_LIKE_CPP | LOOT_TYPE_MILLING_LIKE_CPP => {
            LOOT_TYPE_DISENCHANTING_LIKE_CPP
        }
        LOOT_TYPE_INSIGNIA_LIKE_CPP => LOOT_TYPE_SKINNING_LIKE_CPP,
        LOOT_TYPE_FISHINGHOLE_LIKE_CPP | LOOT_TYPE_FISHING_JUNK_LIKE_CPP => {
            LOOT_TYPE_FISHING_LIKE_CPP
        }
        _ => loot_type,
    }
}

fn loot_is_looted_like_cpp(loot: &CreatureLoot) -> bool {
    loot.coins == 0 && loot.unlooted_count == 0
}

fn direct_item_count_after_loot_release_like_cpp(
    current_count: u32,
    maximum_destroy_count: Option<u32>,
) -> u32 {
    let destroy_count = maximum_destroy_count
        .unwrap_or(current_count)
        .min(current_count);
    current_count.saturating_sub(destroy_count)
}

fn mark_loot_allowed_for_player_like_cpp(loot: &mut CreatureLoot, player_guid: ObjectGuid) {
    if !player_guid.is_empty() && !loot.allowed_looters.contains(&player_guid) {
        loot.allowed_looters.push(player_guid);
    }

    for entry in &mut loot.items {
        if entry.allowed_looters.is_empty() || entry.flags.freeforall {
            entry.add_allowed_looter_like_cpp(player_guid);
        }
    }

    let existing_ffa_item_ids: Vec<u8> = loot
        .player_ffa_items
        .iter()
        .find(|(player, _)| *player == player_guid)
        .map(|(_, items)| items.iter().map(|item| item.loot_list_id).collect())
        .unwrap_or_default();
    let mut ffa_items = Vec::new();
    for entry in &mut loot.items {
        if entry.flags.freeforall
            && entry.has_allowed_looter_like_cpp(player_guid)
            && !existing_ffa_item_ids.contains(&entry.loot_list_id)
        {
            ffa_items.push(NotNormalLootItem {
                loot_list_id: entry.loot_list_id,
                is_looted: false,
            });
            loot.unlooted_count = loot.unlooted_count.saturating_add(1);
        } else if !entry.flags.freeforall
            && entry.has_allowed_looter_like_cpp(player_guid)
            && !entry.flags.counted
        {
            entry.flags.counted = true;
            loot.unlooted_count = loot.unlooted_count.saturating_add(1);
        }
    }

    if !ffa_items.is_empty() {
        match loot
            .player_ffa_items
            .iter_mut()
            .find(|(player, _)| *player == player_guid)
        {
            Some((_, existing)) => existing.extend(ffa_items),
            None => loot.player_ffa_items.push((player_guid, ffa_items)),
        }
    }
}

/// Completes the shared C++ `Loot::FillLoot` visibility/count state before the
/// generation is published through the object-owned authority. Applying this
/// only to the session cache after publication is unsafe: reconciliation would
/// immediately restore the older authoritative snapshot and lose the tap list.
fn prepare_represented_shared_loot_generation_like_cpp(
    loot: &mut CreatureLoot,
    allowed_looters: &[ObjectGuid],
) {
    for looter in allowed_looters {
        if !looter.is_empty() && !loot.allowed_looters.contains(looter) {
            loot.allowed_looters.push(*looter);
        }
    }
    rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(loot);
}

fn prepare_represented_shared_creature_loot_generation_like_cpp(
    loot: &mut CreatureLoot,
    allowed_looters: &[ObjectGuid],
) {
    for looter in allowed_looters {
        mark_loot_allowed_for_player_like_cpp(loot, *looter);
    }
    rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(loot);
}

#[cfg(test)]
fn assign_represented_personal_loot_items_like_cpp<R: Rng + ?Sized>(
    loot: &mut CreatureLoot,
    tappers: &[ObjectGuid],
    rng: &mut R,
) {
    if tappers.is_empty() {
        return;
    }

    loot.unlooted_count = 0;
    loot.player_ffa_items.clear();

    for entry in &mut loot.items {
        entry.allowed_looters.clear();
        entry.flags.counted = false;

        let chosen_tapper = tappers[rng.gen_range(0..tappers.len())];
        entry.add_allowed_looter_like_cpp(chosen_tapper);
    }

    rebuild_represented_personal_loot_counts_like_cpp(loot);
}

fn rebuild_represented_personal_loot_counts_like_cpp(loot: &mut CreatureLoot) {
    loot.unlooted_count = 0;
    loot.player_ffa_items.clear();

    for entry in &mut loot.items {
        entry.ffa_looted_by.clear();
        entry.flags.counted = false;

        if entry.flags.freeforall {
            for looter in &entry.allowed_looters {
                match loot
                    .player_ffa_items
                    .iter_mut()
                    .find(|(player, _)| player == looter)
                {
                    Some((_, existing)) => existing.push(NotNormalLootItem {
                        loot_list_id: entry.loot_list_id,
                        is_looted: false,
                    }),
                    None => loot.player_ffa_items.push((
                        *looter,
                        vec![NotNormalLootItem {
                            loot_list_id: entry.loot_list_id,
                            is_looted: false,
                        }],
                    )),
                }
                loot.unlooted_count = loot.unlooted_count.saturating_add(1);
            }
        } else if !entry.allowed_looters.is_empty() {
            entry.flags.counted = true;
            loot.unlooted_count = loot.unlooted_count.saturating_add(1);
        }
    }
}

/// Rebuild a player-scoped authority pool without resurrecting entries already
/// consumed in the session view. The generation helper above intentionally
/// starts fresh; authority synchronization can also run during release, after
/// `taken`/`ffa_looted_by` have already changed.
fn rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(loot: &mut CreatureLoot) {
    loot.unlooted_count = 0;
    loot.player_ffa_items.clear();

    for entry in &mut loot.items {
        if entry.flags.freeforall {
            entry.flags.counted = false;
            for looter in &entry.allowed_looters {
                let is_looted = entry.ffa_looted_by.contains(looter);
                let item = NotNormalLootItem {
                    loot_list_id: entry.loot_list_id,
                    is_looted,
                };
                match loot
                    .player_ffa_items
                    .iter_mut()
                    .find(|(player, _)| player == looter)
                {
                    Some((_, items)) => items.push(item),
                    None => loot.player_ffa_items.push((*looter, vec![item])),
                }
                if !is_looted {
                    loot.unlooted_count = loot.unlooted_count.saturating_add(1);
                }
            }
        } else {
            entry.flags.counted = !entry.allowed_looters.is_empty();
            if entry.flags.counted && !entry.taken {
                loot.unlooted_count = loot.unlooted_count.saturating_add(1);
            }
        }
    }
}

fn loot_player_has_unlooted_ffa_item_like_cpp(
    loot: &CreatureLoot,
    player_guid: ObjectGuid,
    loot_list_id: u8,
) -> bool {
    loot.player_ffa_items
        .iter()
        .find(|(player, _)| *player == player_guid)
        .is_some_and(|(_, items)| {
            items
                .iter()
                .any(|item| item.loot_list_id == loot_list_id && !item.is_looted)
        })
}

fn loot_item_is_looted_for_player_like_cpp(
    loot: &CreatureLoot,
    entry: &LootEntry,
    player_guid: ObjectGuid,
) -> bool {
    if entry.flags.freeforall {
        !loot_player_has_unlooted_ffa_item_like_cpp(loot, player_guid, entry.loot_list_id)
    } else {
        entry.taken
    }
}

fn mark_loot_item_looted_for_player_like_cpp(
    loot: &mut CreatureLoot,
    loot_list_id: u8,
    player_guid: ObjectGuid,
) {
    let should_decrement = loot
        .items
        .iter()
        .find(|entry| entry.loot_list_id == loot_list_id)
        .is_some_and(|entry| !loot_item_is_looted_for_player_like_cpp(loot, entry, player_guid));

    if let Some(entry) = loot
        .items
        .iter_mut()
        .find(|entry| entry.loot_list_id == loot_list_id)
    {
        entry.mark_looted_for_player_like_cpp(player_guid);
        if entry.flags.freeforall {
            if let Some((_, items)) = loot
                .player_ffa_items
                .iter_mut()
                .find(|(player, _)| *player == player_guid)
                && let Some(item) = items
                    .iter_mut()
                    .find(|item| item.loot_list_id == loot_list_id)
            {
                item.is_looted = true;
            }
        }
        if should_decrement {
            loot.unlooted_count = loot.unlooted_count.saturating_sub(1);
        }
    }
}

fn represented_loot_response_items_like_cpp(
    loot: &CreatureLoot,
    player_guid: ObjectGuid,
) -> Vec<LootItemData> {
    loot.items
        .iter()
        .filter_map(|entry| {
            let ui_type = loot_item_ui_type_for_player_like_cpp(
                player_guid,
                &entry.allowed_looters,
                loot_item_is_looted_for_player_like_cpp(loot, entry, player_guid),
                entry.flags.freeforall,
                loot_player_has_unlooted_ffa_item_like_cpp(loot, player_guid, entry.loot_list_id),
                entry.flags.needs_quest,
                entry.flags.follow_loot_rules,
                loot.loot_method,
                loot.round_robin_player,
                loot.loot_master,
                entry.flags.under_threshold,
                entry.flags.blocked,
                entry.roll_winner,
            )?;

            Some(LootItemData {
                item_type: 0,
                ui_type,
                can_trade_to_tap_list: false,
                loot: ItemInstance {
                    item_id: entry.item_id as i32,
                    ..ItemInstance::default()
                },
                loot_list_id: entry.loot_list_id,
                quantity: entry.quantity,
                loot_item_type: 0,
            })
        })
        .collect()
}

fn looted_corpse_decay_secs_like_cpp(
    is_fully_skinned: bool,
    corpse_delay_secs: u32,
    ignore_decay_ratio: bool,
    corpse_decay_looted_rate: f32,
) -> u32 {
    if is_fully_skinned {
        return 0;
    }

    let rate = if ignore_decay_ratio {
        1.0
    } else {
        corpse_decay_looted_rate.max(0.0)
    };
    ((corpse_delay_secs as f32) * rate) as u32
}

fn loot_can_be_opened_by_player_like_cpp(loot: &CreatureLoot, player_guid: ObjectGuid) -> bool {
    if loot_is_looted_like_cpp(loot) {
        return false;
    }

    loot_has_item_for_all_like_cpp(loot, player_guid)
        || loot_has_item_for_player_like_cpp(loot, player_guid)
}

/// Exact represented branch order of C++ `Player::isAllowedToLoot` for a
/// creature and the pool selected by `Creature::GetLootForPlayer`.
fn creature_loot_is_allowed_to_player_like_cpp(
    creature_is_dead: bool,
    player_has_pending_bind: bool,
    loot: &CreatureLoot,
    player_guid: ObjectGuid,
) -> bool {
    if !creature_is_dead || player_has_pending_bind || loot_is_looted_like_cpp(loot) {
        return false;
    }
    if !loot.allowed_looters.contains(&player_guid)
        || (!loot_has_item_for_all_like_cpp(loot, player_guid)
            && !loot_has_item_for_player_like_cpp(loot, player_guid))
    {
        return false;
    }

    match loot.loot_method {
        LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP | LOOT_METHOD_PERSONAL_LIKE_CPP => true,
        LOOT_METHOD_ROUND_ROBIN_LIKE_CPP => {
            loot.round_robin_player.is_empty()
                || loot.round_robin_player == player_guid
                || loot_has_item_for_player_like_cpp(loot, player_guid)
        }
        LOOT_METHOD_MASTER_LIKE_CPP
        | LOOT_METHOD_GROUP_LIKE_CPP
        | LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP => {
            loot.round_robin_player.is_empty()
                || loot.round_robin_player == player_guid
                || loot_has_over_threshold_item_like_cpp(loot)
                || loot_has_item_for_player_like_cpp(loot, player_guid)
        }
        _ => false,
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreatureLootReleaseCommandQueueOutcomeLikeCpp {
    Queued,
    Retrying,
    Disconnected,
}

#[cfg(test)]
fn queue_creature_loot_release_command_reliably_like_cpp(
    command_tx: &flume::Sender<SessionCommand>,
    command: SessionCommand,
) -> CreatureLootReleaseCommandQueueOutcomeLikeCpp {
    match command_tx.try_send(command) {
        Ok(()) => CreatureLootReleaseCommandQueueOutcomeLikeCpp::Queued,
        Err(flume::TrySendError::Disconnected(_)) => {
            CreatureLootReleaseCommandQueueOutcomeLikeCpp::Disconnected
        }
        Err(flume::TrySendError::Full(command)) => {
            let command_tx = command_tx.clone();
            // Never await another session from the source session loop: two
            // full queues could otherwise wait on each other forever. The
            // detached retry retains the exact command until capacity opens;
            // receiver-side authority/lifecycle gates coalesce its meaning to
            // the current corpse generation and reject stale respawn reuse.
            tokio::spawn(async move {
                if command_tx.send_async(command).await.is_err() {
                    tracing::debug!(
                        "loot-release DynamicFlags retry ended after target session disconnected"
                    );
                }
            });
            CreatureLootReleaseCommandQueueOutcomeLikeCpp::Retrying
        }
    }
}

fn loot_has_over_threshold_item_like_cpp(loot: &CreatureLoot) -> bool {
    loot.items
        .iter()
        .any(|entry| !entry.taken && entry.is_over_threshold_like_cpp())
}

fn connected_roll_looters_like_cpp(
    entry: &LootEntry,
    player_guid: ObjectGuid,
    current_map_id: u16,
    current_instance_id: u32,
    player_registry: Option<&PlayerRegistry>,
) -> Vec<ObjectGuid> {
    let mut looters = Vec::new();

    for looter in &entry.allowed_looters {
        if *looter == player_guid {
            looters.push(*looter);
            continue;
        }

        let Some(registry) = player_registry else {
            continue;
        };
        let Some(player) = registry.loot_presence(*looter) else {
            continue;
        };
        if player.map_id == current_map_id && player.instance_id == current_instance_id {
            looters.push(*looter);
        }
    }

    looters.sort_by_key(|guid| (guid.high_value(), guid.low_value()));
    looters.dedup();
    looters
}

fn represented_max_enchanting_skill_like_cpp(
    looters: &[ObjectGuid],
    current_player_guid: ObjectGuid,
    current_player_enchanting_skill: Option<u16>,
    player_registry: Option<&PlayerRegistry>,
) -> u16 {
    looters
        .iter()
        .filter_map(|looter| {
            if *looter == current_player_guid {
                current_player_enchanting_skill
            } else {
                player_registry.and_then(|registry| registry.loot_enchanting_skill(*looter))
            }
        })
        .max()
        .unwrap_or(0)
}

fn start_loot_roll_packet_like_cpp(
    loot_obj: ObjectGuid,
    map_id: u16,
    loot_method: u8,
    entry: &LootEntry,
    valid_rolls: u8,
    dungeon_encounter_id: i32,
) -> StartLootRoll {
    StartLootRoll {
        loot_obj,
        map_id: map_id as i32,
        roll_time_ms: LOOT_ROLL_TIMEOUT_MS_LIKE_CPP,
        method: loot_method,
        valid_rolls,
        loot_roll_ineligible_reason: [0; 4],
        item: LootItemData {
            item_type: 0,
            ui_type: LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP,
            can_trade_to_tap_list: entry.allowed_looters.len() > 1,
            loot: ItemInstance {
                item_id: entry.item_id as i32,
                random_properties_id: entry.random_properties_id,
                random_properties_seed: entry.random_properties_seed,
                ..ItemInstance::default()
            },
            loot_list_id: entry.loot_list_id,
            quantity: entry.quantity,
            loot_item_type: 0,
        },
        dungeon_encounter_id,
    }
}

fn loot_roll_broadcast_item_like_cpp(entry: &LootEntry, ui_type: u8) -> LootItemData {
    LootItemData {
        item_type: 0,
        ui_type,
        can_trade_to_tap_list: entry.allowed_looters.len() > 1,
        loot: ItemInstance {
            item_id: entry.item_id as i32,
            random_properties_id: entry.random_properties_id,
            random_properties_seed: entry.random_properties_seed,
            ..ItemInstance::default()
        },
        loot_list_id: entry.loot_list_id,
        quantity: entry.quantity,
        loot_item_type: 0,
    }
}

fn roll_chance_with_rate_like_cpp<R: Rng + ?Sized>(chance: f32, rate: f32, rng: &mut R) -> bool {
    if chance >= 100.0 {
        return true;
    }
    rng.gen_range(0.0f32..100.0f32) < chance * rate
}

fn referenced_loot_max_count_like_cpp(max_count: u8, rate: f32) -> u32 {
    ((max_count as f32) * rate) as u32
}

fn represented_disenchant_loot_plain_row_can_roll_like_cpp(
    row: &LootStoreItem,
    item_exists: bool,
) -> bool {
    row.can_roll_as_plain_entry_like_cpp(item_exists, LOOT_MODE_DEFAULT_LIKE_CPP)
}

fn represented_disenchant_loot_reference_row_can_roll_like_cpp(row: &LootStoreItem) -> bool {
    row.can_roll_as_reference_entry_like_cpp(LOOT_MODE_DEFAULT_LIKE_CPP)
}

fn add_loot_item_stacks_like_cpp(
    loot_items: &mut Vec<LootEntry>,
    item_id: u32,
    mut count: u32,
    max_stack_size: u32,
    flags: LootEntryFlags,
) {
    while count > 0 && loot_items.len() < MAX_NR_LOOT_ITEMS_LIKE_CPP {
        let quantity = count.min(max_stack_size);
        loot_items.push(LootEntry {
            loot_list_id: loot_items.len() as u8,
            item_id,
            quantity,
            random_properties_id: 0,
            random_properties_seed: 0,
            item_context: 0,
            flags,
            allowed_looters: Vec::new(),
            roll_winner: ObjectGuid::EMPTY,
            ffa_looted_by: Vec::new(),
            taken: false,
        });
        count = count.saturating_sub(max_stack_size);
    }
}

#[derive(Debug, Clone)]
struct DisenchantLootTemplateFrame {
    template: LootTemplate,
    entry_index: usize,
    group_index: usize,
    requested_group_id: u8,
}

fn disenchant_loot_template_frame_like_cpp(
    rows: Vec<LootStoreItem>,
    requested_group_id: u8,
) -> DisenchantLootTemplateFrame {
    let mut template = LootTemplate::default();
    for row in rows {
        template.add_entry_like_cpp(row);
    }

    DisenchantLootTemplateFrame {
        template,
        entry_index: 0,
        group_index: 0,
        requested_group_id,
    }
}

#[derive(Debug, Clone, Copy)]
enum DisenchantLootTemplateTable {
    Disenchant,
    Reference,
}

impl DisenchantLootTemplateTable {
    fn name(self) -> &'static str {
        match self {
            Self::Disenchant => "disenchant_loot_template",
            Self::Reference => "reference_loot_template",
        }
    }
}

fn represented_loot_roll_finish_winner_like_cpp(
    state: &RepresentedLootRollState,
) -> Option<Option<(ObjectGuid, RepresentedLootRollVote)>> {
    let mut winner = None;
    let mut has_need = false;

    for (player_guid, vote) in &state.voters {
        match vote.vote {
            ROLL_VOTE_NEED_LIKE_CPP => {
                if !has_need
                    || winner.is_none_or(|(_, current): (ObjectGuid, RepresentedLootRollVote)| {
                        vote.roll_number > current.roll_number
                    })
                {
                    has_need = true;
                    winner = Some((*player_guid, *vote));
                }
            }
            ROLL_VOTE_GREED_LIKE_CPP | ROLL_VOTE_DISENCHANT_LIKE_CPP => {
                if !has_need
                    && winner.is_none_or(|(_, current): (ObjectGuid, RepresentedLootRollVote)| {
                        vote.roll_number > current.roll_number
                    })
                {
                    winner = Some((*player_guid, *vote));
                }
            }
            ROLL_VOTE_PASS_LIKE_CPP | ROLL_VOTE_NOT_VALID_LIKE_CPP => {}
            ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP => return None,
            _ => {}
        }
    }

    Some(winner)
}

fn represented_loot_roll_current_winner_like_cpp(
    state: &RepresentedLootRollState,
) -> Option<(ObjectGuid, RepresentedLootRollVote)> {
    let mut winner = None;
    let mut has_need = false;

    for (player_guid, vote) in &state.voters {
        match vote.vote {
            ROLL_VOTE_NEED_LIKE_CPP => {
                if !has_need
                    || winner.is_none_or(|(_, current): (ObjectGuid, RepresentedLootRollVote)| {
                        vote.roll_number > current.roll_number
                    })
                {
                    has_need = true;
                    winner = Some((*player_guid, *vote));
                }
            }
            ROLL_VOTE_GREED_LIKE_CPP | ROLL_VOTE_DISENCHANT_LIKE_CPP => {
                if !has_need
                    && winner.is_none_or(|(_, current): (ObjectGuid, RepresentedLootRollVote)| {
                        vote.roll_number > current.roll_number
                    })
                {
                    winner = Some((*player_guid, *vote));
                }
            }
            ROLL_VOTE_PASS_LIKE_CPP
            | ROLL_VOTE_NOT_VALID_LIKE_CPP
            | ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP => {}
            _ => {}
        }
    }

    winner
}

fn loot_has_item_for_all_like_cpp(loot: &CreatureLoot, player_guid: ObjectGuid) -> bool {
    if loot.coins > 0 {
        return true;
    }

    loot.items.iter().any(|entry| {
        !entry.taken
            && entry.flags.follow_loot_rules
            && !entry.flags.freeforall
            && entry.has_allowed_looter_like_cpp(player_guid)
    })
}

fn loot_has_item_for_player_like_cpp(loot: &CreatureLoot, player_guid: ObjectGuid) -> bool {
    loot.items.iter().any(|entry| {
        !loot_item_is_looted_for_player_like_cpp(loot, entry, player_guid)
            && entry.has_allowed_looter_like_cpp(player_guid)
            && (!entry.flags.follow_loot_rules || entry.flags.freeforall)
    })
}

fn loot_item_context(context: u8) -> ItemContext {
    <ItemContext as num_traits::FromPrimitive>::from_u8(context).unwrap_or(ItemContext::None)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LootStoreRandomProperties {
    id: i32,
    seed: i32,
}

fn loot_store_data_can_stack_with_item(
    loot_entry: &LootEntry,
    random_properties: LootStoreRandomProperties,
    item: &Item,
) -> bool {
    let data = item.data();
    data.random_properties_id == random_properties.id
        && data.property_seed == random_properties.seed
        && u8::try_from(data.context).unwrap_or(0) == loot_entry.item_context
}

impl WorldSession {
    fn generate_loot_store_random_properties_with_rng_like_cpp<R: Rng + ?Sized>(
        &self,
        item_id: u32,
        rng: &mut R,
    ) -> LootStoreRandomProperties {
        // C++ Player::StoreLootItem calls ItemEnchantmentMgr::GenerateRandomProperties(itemid).
        let random_select = self.item_template_random_select(item_id);
        let random_suffix = self.item_template_random_suffix_group_id(item_id);
        if random_select == 0 && random_suffix == 0 {
            return LootStoreRandomProperties { id: 0, seed: 0 };
        }

        if random_select != 0 {
            let Some(random_properties_id) =
                self.select_random_enchantment_from_group_like_cpp(u32::from(random_select), rng)
            else {
                return LootStoreRandomProperties { id: 0, seed: 0 };
            };

            if self
                .item_random_properties_store()
                .and_then(|store| store.get(random_properties_id))
                .is_none()
            {
                return LootStoreRandomProperties { id: 0, seed: 0 };
            }

            return LootStoreRandomProperties {
                id: i32::try_from(random_properties_id).unwrap_or(0),
                seed: 0,
            };
        }

        let Some(random_suffix_id) =
            self.select_random_enchantment_from_group_like_cpp(u32::from(random_suffix), rng)
        else {
            return LootStoreRandomProperties { id: 0, seed: 0 };
        };

        if self
            .item_random_suffix_store()
            .and_then(|store| store.get(random_suffix_id))
            .is_none()
        {
            return LootStoreRandomProperties { id: 0, seed: 0 };
        }

        let seed = self
            .item_random_property_template(item_id)
            .map(|template| self.random_property_points_like_cpp(template))
            .unwrap_or(0);

        LootStoreRandomProperties {
            id: -i32::try_from(random_suffix_id).unwrap_or(0),
            seed,
        }
    }

    fn select_random_enchantment_from_group_like_cpp<R: Rng + ?Sized>(
        &self,
        group_id: u32,
        rng: &mut R,
    ) -> Option<u32> {
        let group = self
            .item_random_enchantment_template_store()
            .and_then(|store| store.group(group_id))?;
        select_weighted_random_enchantment_like_cpp(group, rng)
    }

    fn random_property_points_like_cpp(&self, template: ItemRandomPropertyTemplateEntry) -> i32 {
        let prop_index =
            match <InventoryType as num_traits::FromPrimitive>::from_i8(template.inventory_type) {
                Some(InventoryType::NonEquip)
                | Some(InventoryType::Bag)
                | Some(InventoryType::Tabard)
                | Some(InventoryType::Ammo)
                | Some(InventoryType::Quiver)
                | Some(InventoryType::Relic)
                | None => return 0,
                Some(InventoryType::Head)
                | Some(InventoryType::Body)
                | Some(InventoryType::Chest)
                | Some(InventoryType::Legs)
                | Some(InventoryType::Weapon2Hand)
                | Some(InventoryType::Robe) => 0,
                Some(InventoryType::Shoulders)
                | Some(InventoryType::Waist)
                | Some(InventoryType::Feet)
                | Some(InventoryType::Hands)
                | Some(InventoryType::Trinket) => 1,
                Some(InventoryType::Neck)
                | Some(InventoryType::Wrists)
                | Some(InventoryType::Finger)
                | Some(InventoryType::Shield)
                | Some(InventoryType::Cloak)
                | Some(InventoryType::Holdable) => 2,
                Some(InventoryType::Weapon)
                | Some(InventoryType::WeaponMainhand)
                | Some(InventoryType::WeaponOffhand) => 3,
                Some(InventoryType::Ranged)
                | Some(InventoryType::Thrown)
                | Some(InventoryType::RangedRight) => 4,
                _ => return 0,
            };

        let Some(points) = self
            .rand_prop_points_store()
            .and_then(|store| store.get(u32::from(template.item_level)))
        else {
            return 0;
        };

        match <ItemQuality as num_traits::FromPrimitive>::from_i8(template.quality) {
            Some(ItemQuality::Uncommon) => points.good[prop_index] as i32,
            Some(ItemQuality::Rare) | Some(ItemQuality::Heirloom) => {
                points.superior[prop_index] as i32
            }
            Some(ItemQuality::Epic)
            | Some(ItemQuality::Legendary)
            | Some(ItemQuality::Artifact) => points.epic[prop_index] as i32,
            _ => 0,
        }
    }
}

fn select_weighted_random_enchantment_like_cpp<R: Rng + ?Sized>(
    group: &[ItemRandomEnchantmentTemplateEntry],
    rng: &mut R,
) -> Option<u32> {
    let valid_rows = group
        .iter()
        .filter(|row| (0.000001..=100.0).contains(&row.chance))
        .collect::<Vec<_>>();
    let weights = valid_rows.iter().map(|row| row.chance).collect::<Vec<_>>();
    let distribution = WeightedIndex::new(weights).ok()?;
    Some(valid_rows[distribution.sample(rng)].enchantment_id)
}

#[derive(Debug, Clone)]
struct PlannedLootNewStack {
    slot: u8,
    entry_id: u32,
    count: u32,
    max_durability: u32,
    dynamic_flags: u32,
    random_properties_id: i32,
    random_properties_seed: i32,
    item_context: u8,
}

/// Everything needed to mirror C++ `Player::StoreLootItem`'s post-store wire
/// boundary. SQL and the object-owned claim are settled by the detached
/// worker; the session publishes the stored-item update before choosing the
/// direct-loot or disenchant-specific removal/`ItemPushResult` order.
#[derive(Debug, Clone, Copy)]
struct LootItemClaimCommitContextLikeCpp {
    owner_guid: ObjectGuid,
    loot_obj: ObjectGuid,
    loot_list_id: u8,
    player_guid: ObjectGuid,
    free_for_all: bool,
}

#[derive(Debug, Clone)]
struct PlannedDisenchantExistingStack {
    slot: u8,
    item_guid: ObjectGuid,
    db_guid: u64,
    new_count: u32,
    dynamic_flags: u32,
    flags_changed: bool,
}

#[derive(Debug, Clone)]
struct PlannedDirectLootExistingStack {
    slot: u8,
    item_guid: ObjectGuid,
    db_guid: u64,
    new_count: u32,
    added_count: u32,
    dynamic_flags: u32,
    flags_changed: bool,
}

#[derive(Debug, Clone)]
struct PlannedDisenchantExistingPush {
    slot: u8,
    item_guid: ObjectGuid,
    added_count: u32,
    new_count: u32,
}

#[derive(Debug, Clone)]
struct PlannedDisenchantNewPush {
    stack_index: usize,
    added_count: u32,
    new_count: u32,
}

#[derive(Debug, Clone)]
struct PlannedDisenchantGrant {
    entry: LootEntry,
    random_properties: LootStoreRandomProperties,
    existing_pushes: Vec<PlannedDisenchantExistingPush>,
    new_pushes: Vec<PlannedDisenchantNewPush>,
}

/// Own a loot lease in the same detached task that crosses the durable
/// persistence boundary.  Tokio does not cancel a spawned task when the
/// caller drops its `JoinHandle`, so packet/session cancellation cannot turn a
/// successful SQL commit back into an available object-owned claim.
enum LootClaimPersistenceWorkerError<E> {
    Persistence(E),
    Claim(LootClaimCommitError),
}

impl<E: std::fmt::Debug> std::fmt::Debug for LootClaimPersistenceWorkerError<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Persistence(error) => formatter.debug_tuple("Persistence").field(error).finish(),
            Self::Claim(error) => formatter.debug_tuple("Claim").field(error).finish(),
        }
    }
}

fn queue_stored_item_money_indeterminate_kick_like_cpp(command_tx: &flume::Sender<SessionCommand>) {
    let kick = SessionCommand::KickLikeCpp(KickLikeCppCommand {
        reason: "stored Item money COMMIT outcome is unknown; relog required".to_string(),
    });
    if let Err(error) = command_tx.try_send(kick) {
        let kick = error.into_inner();
        let command_tx = command_tx.clone();
        tokio::spawn(async move {
            let _ = command_tx.send_async(kick).await;
        });
    }
}

fn spawn_loot_claim_persistence_worker_like_cpp<F, E>(
    persistence: F,
    claim: Option<LootClaimLease>,
    durable_item_completion: Option<(
        DurableItemLootPersistenceGuardLikeCpp,
        DurableItemLootCompletionLikeCpp,
    )>,
) -> Result<
    tokio::task::JoinHandle<Result<(), LootClaimPersistenceWorkerError<E>>>,
    LootClaimCommitError,
>
where
    F: std::future::Future<Output = Result<(), E>> + Send + 'static,
    E: Send + 'static,
{
    let persistence_guard = claim
        .as_ref()
        .map(LootClaimLease::begin_persistence_guard_like_cpp)
        .transpose()?;
    drop(claim);
    Ok(tokio::spawn(async move {
        let mut durable_item_completion = durable_item_completion;
        persistence
            .await
            .map_err(LootClaimPersistenceWorkerError::Persistence)?;
        if let Some(mut guard) = persistence_guard {
            let (_, committed_snapshot) = guard
                .commit_with_snapshot_like_cpp()
                .map_err(LootClaimPersistenceWorkerError::Claim)?;
            if let (Some(snapshot), Some((_, completion))) =
                (committed_snapshot, durable_item_completion.as_ref())
                && let Some(fanout) = completion.item_fanout.as_ref()
            {
                // Publish the serialization cut before exposing the durable
                // completion to the session. Sampling the authority later can
                // include an opener that already saw the consumed slot.
                let _ = fanout.committed_snapshot.set(snapshot);
            }
        }
        if let Some((guard, completion)) = durable_item_completion.as_mut() {
            guard.mark_committed_like_cpp(completion.clone());
        }
        Ok(())
    }))
}

/// Outcome-aware persistence worker for consume-and-grant item transactions.
/// An unknown COMMIT cannot be treated as rollback: the old object allocation
/// is quarantined permanently and the player is kicked to reload whichever
/// durable state the concrete adapter ultimately kept.
fn spawn_loot_item_persistence_worker_like_cpp<F>(
    persistence: F,
    claim: Option<LootClaimLease>,
    durable_item_completion: Option<(
        DurableItemLootPersistenceGuardLikeCpp,
        DurableItemLootCompletionLikeCpp,
    )>,
    command_tx: flume::Sender<SessionCommand>,
) -> Result<
    tokio::task::JoinHandle<Result<(), LootClaimPersistenceWorkerError<String>>>,
    LootClaimCommitError,
>
where
    F: std::future::Future<Output = PersistenceOutcomeLikeCpp> + Send + 'static,
{
    let mut persistence_guard = claim
        .as_ref()
        .map(LootClaimLease::begin_persistence_guard_like_cpp)
        .transpose()?;
    drop(claim);
    Ok(tokio::spawn(async move {
        let mut durable_item_completion = durable_item_completion;
        match persistence.await {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                return Err(LootClaimPersistenceWorkerError::Persistence(reason));
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                if let Some(guard) = persistence_guard.as_mut() {
                    let _ = guard.quarantine_commit_unknown_like_cpp();
                }
                let kick = SessionCommand::KickLikeCpp(KickLikeCppCommand {
                    reason: "loot item COMMIT outcome is unknown; relog required".to_string(),
                });
                if let Err(send_error) = command_tx.try_send(kick) {
                    let kick = send_error.into_inner();
                    tokio::spawn(async move {
                        let _ = command_tx.send_async(kick).await;
                    });
                }
                return Err(LootClaimPersistenceWorkerError::Persistence(reason));
            }
        }
        if let Some(mut guard) = persistence_guard {
            let (_, committed_snapshot) = guard
                .commit_with_snapshot_like_cpp()
                .map_err(LootClaimPersistenceWorkerError::Claim)?;
            if let (Some(snapshot), Some((_, completion))) =
                (committed_snapshot, durable_item_completion.as_ref())
                && let Some(fanout) = completion.item_fanout.as_ref()
            {
                let _ = fanout.committed_snapshot.set(snapshot);
            }
        }
        if let Some((guard, completion)) = durable_item_completion.as_mut() {
            guard.mark_committed_like_cpp(completion.clone());
        }
        Ok(())
    }))
}

#[cfg(test)]
#[path = "../loot_tests.rs"]
mod tests;
