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

use wow_loot::{
    LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP, LOOT_METHOD_GROUP_LIKE_CPP, LOOT_METHOD_MASTER_LIKE_CPP,
    LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP, LOOT_METHOD_PERSONAL_LIKE_CPP,
    LOOT_METHOD_ROUND_ROBIN_LIKE_CPP, creature_loot_is_allowed_to_player_like_cpp,
    loot_can_be_opened_by_player_like_cpp, loot_has_over_threshold_item_like_cpp,
    loot_is_looted_like_cpp, loot_item_is_looted_for_player_like_cpp,
    loot_player_has_unlooted_ffa_item_like_cpp, mark_loot_allowed_for_player_like_cpp,
    mark_loot_item_looted_for_player_like_cpp,
    prepare_represented_shared_creature_loot_generation_like_cpp,
    prepare_represented_shared_loot_generation_like_cpp,
    rebuild_represented_personal_loot_counts_like_cpp,
    rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp,
};

use wow_loot::{
    ROLL_VOTE_DISENCHANT_LIKE_CPP, ROLL_VOTE_GREED_LIKE_CPP, ROLL_VOTE_NEED_LIKE_CPP,
    ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP, ROLL_VOTE_NOT_VALID_LIKE_CPP, ROLL_VOTE_PASS_LIKE_CPP,
    represented_loot_roll_current_winner_like_cpp, represented_loot_roll_finish_winner_like_cpp,
};

#[cfg(test)]
use wow_loot::ROLL_FLAG_TYPE_NEED_LIKE_CPP;

mod authority;
mod claims;
mod combat_commands;
mod disenchant;
mod fanout;
mod generation;
mod handlers;
mod money;
mod object_state;
mod persistence;
mod persistence_workers;
mod player_view;
mod random_properties;
mod release_and_rolls;
mod reply_items;
mod request_cache;
mod requests;
mod rolls;
mod sources;
mod storage_plans;
mod visibility_commands;

pub use self::disenchant::*;
pub use self::object_state::*;
pub use self::persistence_workers::*;
pub use self::player_view::*;
pub use self::release_and_rolls::*;
pub use self::reply_items::*;

use std::collections::{HashMap, HashSet};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use rand::Rng;
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::session::directory::{PlayerRegistry, PrepareLootMoneyApplicationLikeCpp};
use crate::session::mailbox::{
    ApplyCreatureMeleeDamageLikeCppCommand, ApplyGroupJoinLikeCppCommand,
    ApplyGroupRemovalLikeCppCommand, ApplyLootMoneyLikeCppCommand, ApplyLootMoneyResultLikeCpp,
    CreatureAttackStartLikeCppCommand, CreatureAttackStopLikeCppCommand, KickLikeCppCommand,
    LootRollCommandIdentityLikeCpp, LootRollStoreWinnerCommand, LootRollVoteCommand,
    MasterLootGiveCommand, MasterLootGiveResult, NotifyLootMoneyRemovedLikeCppCommand,
    ReconcilePvpCombatExpiryLikeCppCommand, RefreshVisibleWorldCreaturesLikeCppCommand,
    SendAddonIfRegisteredLikeCppCommand, SendCreatureLootReleaseValuesUpdateLikeCppCommand,
    SendCreatureSpellCastIfVisibleLikeCppCommand, SendIfVisibleLikeCppCommand,
    SendPartyUpdateLikeCppCommand, SessionCommand,
    SyncChestGameobjectStateAndRefreshLikeCppCommand,
    SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    SyncGooberGameobjectStateAndRefreshLikeCppCommand,
};
use wow_constants::{
    ClientOpcodes, InventoryResult, ItemContext, ItemFieldFlags, ItemFlags, ItemFlags2,
    UnitDynFlags,
};
use wow_core::{ObjectGuid, guid::HighGuid};
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
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_END, INVENTORY_SLOT_ITEM_START, ItemPosCount,
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
    MasterLootItem, SLootRelease, SetLootSpecialization, StartLootRoll,
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

use crate::session::{
    DurableItemLootCompletionLikeCpp, DurableItemLootPersistenceGuardLikeCpp,
    DurableLootItemFanoutLikeCpp, InventoryItem, ItemValuationCatalogsLikeCpp,
    LootMoneyDeliveryAddressLikeCpp, LootMoneyPersistenceErrorLikeCpp,
    LootMoneyViewerFanoutLikeCpp, RepresentedGameObjectSpellCaster, RepresentedGameObjectUseEffect,
    RepresentedLootRollState, RepresentedLootRollVote,
    RepresentedQuestObjectiveProgressEventLikeCpp, SessionState, WorldSession,
    loot_money_durable_outcome_like_cpp,
};
use random_properties::{
    LootStoreRandomProperties, loot_store_data_can_stack_with_item,
    select_weighted_random_enchantment_like_cpp,
};
use storage_plans::{
    LootItemClaimCommitContextLikeCpp, PlannedDirectLootExistingStack,
    PlannedDisenchantExistingPush, PlannedDisenchantExistingStack, PlannedDisenchantGrant,
    PlannedDisenchantNewPush, PlannedLootNewStack,
};
use wow_conditions::{
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    QUEST_STATUS_NONE_LIKE_CPP, QUEST_STATUS_REWARDED_LIKE_CPP,
};
const MAX_NR_LOOT_ITEMS_LIKE_CPP: usize = 18;
const LOOT_ROLL_TIMEOUT_MS_LIKE_CPP: u32 = 60_000;
#[cfg(test)]
const ROLL_ALL_TYPE_NO_DISENCHANT_LIKE_CPP: u8 = 0x07;
const LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP: u8 = 0;
const LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP: u8 = 1;
const LOOT_SLOT_TYPE_LOCKED_LIKE_CPP: u8 = 2;
const DISENCHANT_LOOT_ROLL_CRITERIA_SPELL_LIKE_CPP: u32 = 13_262;
const LOOT_MODE_DEFAULT_LIKE_CPP: u16 = 0x01;
const LOOT_MODE_JUNK_FISH_LIKE_CPP: u16 = 0x8000;
const ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP: u32 = 0x0004;
const ITEM_FLAGS_CU_IGNORE_QUEST_STATUS_LIKE_CPP: u32 = 0x0002;
const MAX_LOOT_REFERENCE_FRAMES_LIKE_CPP: u32 = 64;
const CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP: i32 = 51;
const CONDITION_TYPE_MASK_LIKE_CPP: i32 = 52;
const TYPEID_PLAYER_LIKE_CPP: u32 = 6;
const PLAYER_TYPE_MASK_LIKE_CPP: u32 = 0x0001 | 0x0020 | 0x0040;
const LOCK_KEY_SKILL_LIKE_CPP: u8 = 2;
const LOCK_KEY_SPELL_LIKE_CPP: u8 = 3;
const SPELL_EFFECT_OPEN_LOCK_LIKE_CPP: u32 = 33;
const REMOTE_MASTER_LOOT_COMMAND_TIMEOUT: Duration = Duration::from_millis(250);

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

/// Unit fixtures that predate canonical map objects may exercise packet-cache
/// behavior locally. This is a compile-time false branch in production: live
/// Creature/GameObject claims fail closed without their map-owned authority.
const fn represented_local_loot_fixture_allowed_like_cpp() -> bool {
    cfg!(test)
}

impl<E: std::fmt::Debug> std::fmt::Debug for LootClaimPersistenceWorkerError<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Persistence(error) => formatter.debug_tuple("Persistence").field(error).finish(),
            Self::Claim(error) => formatter.debug_tuple("Claim").field(error).finish(),
        }
    }
}

#[cfg(test)]
#[path = "../loot_tests.rs"]
mod tests;
