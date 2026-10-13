// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-session loot operations (#1363 slices 5a and 5b; moved from the
//! World `handlers/loot` tree): loot sources (creature corpses, chests,
//! gathering nodes, fishing), loot generation and condition admission, the
//! request player context and quest gates, the owned-loot claim, cache and
//! authority facades, group rolls, the loot-window requests, item storage and
//! disenchant, durable persistence and its workers, and the loot fanout.
//!
//! Each body takes the World session through [`LootSessionHostLikeCpp`]; the
//! registrations, release and the money paths stay with the host.

mod authority;
mod claims;
mod fanout;
mod generation;
mod host;
mod persistence;
mod random_properties;
mod request_cache;
mod request_context;
mod requests;
mod rolls;
mod sources;

pub use host::LootSessionHostLikeCpp;

pub use authority::*;
pub use claims::*;
pub use fanout::*;
pub use generation::*;
pub use persistence::*;
pub use random_properties::*;
pub use request_cache::*;
pub use request_context::*;
pub use requests::*;
pub use rolls::*;
pub use sources::*;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use rand::Rng;

use tracing::{debug, warn};

use wow_constants::{InventoryResult, ItemContext, ItemFlags, ItemFlags2};
use wow_core::ObjectGuid;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_core::guid::HighGuid;
use wow_entities::{
    AccessorObjectKind, GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_GATHERING_NODE,
    GAMEOBJECT_TYPE_GOOBER, GO_DYNFLAG_LO_NO_INTERACT, GameObjectLootSource,
    GatheringNodeUseSource, GoState, LootState,
};
use wow_loot::{
    GeneratedLootItem, LOOT_METHOD_MASTER_LIKE_CPP, LootClaimLease, LootConditionId,
    LootConditionRowLikeCpp, LootFillError, LootFillOptions, LootItemRandomProperties,
    LootItemTemplateMetadata, LootStoreItem, LootStoreItemContext, LootStoreKind, LootTemplate,
    OwnedLootAuthority, OwnedLootAuthorityLifecycle, OwnedLootScope,
    condition_compare_values_like_cpp, loot_condition_reference_ids_like_cpp,
    loot_conditions_allow_player_with_references_like_cpp_representable, loot_is_looted_like_cpp,
    mark_loot_allowed_for_player_like_cpp,
    prepare_represented_shared_creature_loot_generation_like_cpp,
    prepare_represented_shared_loot_generation_like_cpp,
    rebuild_represented_personal_loot_counts_like_cpp,
    rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp,
};
use wow_packet::packets::loot::{
    CreatureLoot, LOOT_ERROR_MASTER_INV_FULL_LIKE_CPP, LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
    LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP, LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
    LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP, LOOT_TYPE_CHEST_LIKE_CPP, LOOT_TYPE_CORPSE_LIKE_CPP,
    LOOT_TYPE_FISHING_JUNK_LIKE_CPP, LOOT_TYPE_FISHING_LIKE_CPP, LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
    LootEntry, LootEntryFlags, LootRemoved, LootResponse, MasterLootCandidateList,
};
use wow_world_core::session::mailbox::{
    SessionCommand, SyncChestGameobjectStateAndRefreshLikeCppCommand,
    SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    SyncGooberGameobjectStateAndRefreshLikeCppCommand,
};
use wow_world_core::session::state::hub_support::RepresentedLootPlayerContext;
use wow_world_core::session::{
    ItemValuationCatalogsLikeCpp, OwnedLootAuthorityLookupOutcomeLikeCpp, SessionState,
};
use wow_world_entities::{RepresentedGameObjectSpellCaster, RepresentedGameObjectUseEffect};
use wow_world_lifecycle::loot_template_rules::{
    ItemTemplateAddonLootMetadataLikeCpp, player_class_mask_like_cpp,
    referenced_loot_max_count_like_cpp, roll_chance_with_rate_like_cpp,
};
use wow_world_loot::storage_plans::LootItemClaimCommitContextLikeCpp;
use wow_world_loot::{
    LootStoreRandomProperties, RepresentedCreatureLootStateLikeCpp, loot_type_for_client_like_cpp,
    represented_loot_response_items_like_cpp,
};

use wow_conditions::{
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    QUEST_STATUS_NONE_LIKE_CPP,
};

// #1363 slice 5b: the rolls, requests, item storage, persistence and fanout
// bodies.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use wow_constants::ItemFieldFlags;
use wow_entities::{
    INVENTORY_DEFAULT_SIZE, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_END,
    INVENTORY_SLOT_ITEM_START, ItemPosCount, MAX_MONEY_AMOUNT,
    PlayerInventoryItem as InventoryItem, make_item_pos,
};
use wow_loot::{
    LOOT_METHOD_GROUP_LIKE_CPP, LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP, LootClaimCommitError,
    LootClaimError, LootClaimPayload, ROLL_VOTE_DISENCHANT_LIKE_CPP, ROLL_VOTE_GREED_LIKE_CPP,
    ROLL_VOTE_NEED_LIKE_CPP, ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP, ROLL_VOTE_NOT_VALID_LIKE_CPP,
    ROLL_VOTE_PASS_LIKE_CPP, RepresentedLootRollVote, loot_has_over_threshold_item_like_cpp,
    loot_item_is_looted_for_player_like_cpp, mark_loot_item_looted_for_player_like_cpp,
    represented_loot_roll_current_winner_like_cpp, represented_loot_roll_finish_winner_like_cpp,
};
use wow_packet::ServerPacket;
use wow_packet::packets::loot::{
    LootAllPassed, LootList, LootMoneyNotify, LootRoll, LootRollBroadcast, LootRollWon,
};
use wow_packet::packets::update::{ItemCreateData, ItemEnchantmentValuesUpdate, UpdateObject};
use wow_persistence::PersistenceOutcomeLikeCpp;
use wow_world_core::session::mailbox::{
    KickLikeCppCommand, LootRollCommandIdentityLikeCpp, MasterLootGiveResult,
};
use wow_world_lifecycle::{
    DurableItemLootCompletionLikeCpp, DurableItemLootPersistenceGuardLikeCpp,
    DurableLootItemFanoutLikeCpp,
};
use wow_world_loot::storage_plans::{
    PlannedDirectLootExistingStack, PlannedDisenchantExistingPush, PlannedDisenchantExistingStack,
    PlannedDisenchantGrant, PlannedDisenchantNewPush, PlannedLootNewStack,
};
use wow_world_loot::{
    LOOT_ROLL_TIMEOUT_MS_LIKE_CPP, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP,
    LOOT_SLOT_TYPE_LOCKED_LIKE_CPP, LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP, RepresentedLootRollState,
    connected_roll_looters_like_cpp, loot_item_context, loot_roll_broadcast_item_like_cpp,
    loot_store_data_can_stack_with_item, represented_max_enchanting_skill_like_cpp,
    start_loot_roll_packet_like_cpp,
};
use wow_world_quest::RepresentedQuestObjectiveProgressEventLikeCpp;

pub const LOOT_MODE_DEFAULT_LIKE_CPP: u16 = 0x01;
pub const LOOT_MODE_JUNK_FISH_LIKE_CPP: u16 = 0x8000;
const MAX_LOOT_REFERENCE_FRAMES_LIKE_CPP: u32 = 64;
const CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP: i32 = 51;
const CONDITION_TYPE_MASK_LIKE_CPP: i32 = 52;
const TYPEID_PLAYER_LIKE_CPP: u32 = 6;
const PLAYER_TYPE_MASK_LIKE_CPP: u32 = 0x0001 | 0x0020 | 0x0040;

pub fn master_loot_error_for_inventory_result_like_cpp(result: InventoryResult) -> Option<u8> {
    match result {
        InventoryResult::Ok => None,
        InventoryResult::ItemMaxCount => Some(LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP),
        InventoryResult::InvFull => Some(LOOT_ERROR_MASTER_INV_FULL_LIKE_CPP),
        _ => Some(LOOT_ERROR_MASTER_OTHER_LIKE_CPP),
    }
}

// ── Loot generation ───────────────────────────────────────────────

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

pub fn generated_creature_loot_item_to_entry_like_cpp(
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

pub fn generated_shared_gameobject_loot_item_to_entry_like_cpp<FAllowed>(
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
pub struct RepresentedGameObjectLootInstallObservationLikeCpp {
    authority: OwnedLootAuthority,
    object_generation: u64,
    loot_lifecycle_revision: u64,
}

#[cfg(any(test, feature = "test-fixtures"))]
pub fn represented_loot_object_guid_like_cpp(owner: ObjectGuid) -> ObjectGuid {
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

pub fn represented_disenchant_loot_plain_row_can_roll_like_cpp(
    row: &LootStoreItem,
    item_exists: bool,
) -> bool {
    row.can_roll_as_plain_entry_like_cpp(item_exists, LOOT_MODE_DEFAULT_LIKE_CPP)
}

pub fn represented_disenchant_loot_reference_row_can_roll_like_cpp(row: &LootStoreItem) -> bool {
    row.can_roll_as_reference_entry_like_cpp(LOOT_MODE_DEFAULT_LIKE_CPP)
}

#[derive(Debug, Clone)]
pub struct DisenchantLootTemplateFrame {
    pub template: LootTemplate,
    entry_index: usize,
    group_index: usize,
    requested_group_id: u8,
}

pub fn disenchant_loot_template_frame_like_cpp(
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
