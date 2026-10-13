// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handlers, organised by feature.
//!
//! Issue #224 split the former 20,272-line `handlers/character.rs` into
//! private feature modules. The logical owner, every registration, opcode and
//! dispatcher arm are unchanged; this module keeps the shared constants,
//! helper types and free functions the features build on.

mod account;
mod bank;
mod character_handler_host;
// The production-linked rename contract in `tests/` compiles without cfg(test),
// so these entry points are also visible under the test-fixtures feature.
#[cfg(any(test, feature = "test-fixtures"))]
mod character_handler_shims;
mod condition_objects;
mod creation_support;
mod entry_zone;
// Test-only: its production readers moved to `wow-world-character` (#1263 F5, #1363 A5).
#[cfg(test)]
mod enumeration_support;
mod gossip;
mod item_load_support;
mod items;
mod lifecycle;
mod login_support;
mod login_transport_support;
mod query;
mod session_state;
mod spell_rules;
mod stats;
#[cfg(test)]
#[path = "../../../unit_tests/handlers/character/stats_queries/test_only.rs"]
mod stats_queries;
mod stats_update;
mod vendor;
mod visibility;
mod world_entry;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::Arc;

use rand::Rng;
// Test-only: its production caller moved to `wow-world-character` (#1263 F5, #1363 A5).
#[cfg(test)]
pub(crate) use stats::RepresentedPlayerGearStatsLikeCpp;
use tracing::{debug, info, trace, warn};
// Test-only: their production readers moved to `wow-world-entities` (#1363 slice 8a).
#[cfg(test)]
use wow_constants::CreatureFlagsExtra;
#[cfg(test)]
use wow_constants::movement::MovementFlag;
use wow_constants::unit::NPCFlags1;
#[cfg(test)]
use wow_constants::unit::{UNIT_FLAGS2_ALLOWED_LIKE_CPP, UNIT_FLAGS3_ALLOWED_LIKE_CPP, UnitFlags};
use wow_constants::{
    ConditionSourceType, InventoryResult, InventoryType, ItemContext, ItemFieldFlags, ItemFlags,
    ItemUpdateState, PowerType, TypeId, TypeMask, UnitStandStateType,
};
// Test-only: its production readers moved to `wow-world-inventory` (#1363 slice 8b).
#[cfg(test)]
use wow_constants::ItemVendorType;
// Test-only: its production readers were the character/account registrations that
// moved to `wow-world-application` under #1263 F5; the character scenarios still
// reach these three names through `super::*`.
#[cfg(test)]
use wow_constants::ClientOpcodes;
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_crypto::rsa_sign::rsa_sign_connect_to;
#[cfg(test)]
use wow_data::PlayerCreatePositionLikeCpp;
// Test-only: its production caller moved to `wow-world-character` (#1263 F5, #1363 A5).
#[cfg(test)]
use wow_data::PlayerStatSystemProjectionLikeCpp;
use wow_data::{
    ConditionEntriesByTypeStore, ConditionId, CurrencyTypesStore, HotfixRecordStatus,
    ItemExtendedCostStore, PlayerConditionContextLikeCpp, PlayerConditionStore,
    PlayerCreateInfoLikeCpp, PlayerSpellBonusInputLikeCpp, PlayerStatSystemInputLikeCpp,
    TaxiPathNodeEntry, TaxiPathNodeStore, calculate_player_stat_system_like_cpp,
    hotfix_locale_mask, is_player_meeting_condition_like_cpp,
};
pub(crate) use wow_world_inventory::ExtendedCostItemTurninChange;
// Test-only types whose production users moved to `wow-world-inventory`.
#[cfg(test)]
pub(crate) use std::f32::consts::PI;
#[cfg(test)]
pub(crate) use wow_constants::{EnchantmentSlot, ItemBondingType};
#[cfg(test)]
pub(crate) use wow_entities::SocketedGem;
// Test-only: its production readers moved to `wow-world-entities` (#1363 slice 8a).
#[cfg(test)]
use wow_entities::MovementGeneratorType;
use wow_entities::{
    BANK_SLOT_BAG_END, BANK_SLOT_BAG_START, GAMEOBJECT_TYPE_QUESTGIVER, INVENTORY_DEFAULT_SIZE,
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START, NULL_BAG, NULL_SLOT,
    REAGENT_BAG_SLOT_END, REAGENT_BAG_SLOT_START, SendNewItemPlan, SwapItemPreflightResult,
    WorldObject, is_bank_pos, is_child_equipment_pos, is_equipment_pos, is_inventory_pos,
    item_can_go_into_bag,
};
// Test-only: their production readers moved to `wow-world-inventory` (#1363 slice 8b).
#[cfg(test)]
use wow_entities::INVENTORY_SLOT_ITEM_START;
// Test-only: the production registration source that used these two moved to the
// application crate's character/account area registrar under #1263 F5.
#[cfg(test)]
use wow_handler::{PacketProcessing, SessionStatus};
#[cfg(test)]
pub(crate) use wow_world_lifecycle::login_transport::GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_LIKE_CPP;

// The character/account packet registrations that used to reach
// `crate::session::registry::PacketHandlerEntry` from here moved to the
// application crate's explicit area registrar under #1263 F5, so this module no
// longer needs the entry type in scope.
use wow_packet::packets::auth::{
    ConnectTo, ConnectToAddress, ConnectToKey, ConnectToSerial, ResumeComms,
};
use wow_packet::packets::character::*;
use wow_packet::packets::chat::ChatServerMessage;
use wow_packet::packets::item::*;
use wow_packet::packets::loot::LootReleaseAll;
use wow_packet::packets::misc::*;
use wow_packet::packets::movement::TransportInfo;
use wow_packet::packets::quest::QuestGiverStatusMultiple;
use wow_packet::packets::spell::{SpellCastVisual, SpellTargetData};
use wow_packet::packets::update::*;
// Explicit provenance for child modules that reach these names through `super::{}`:
// a glob import leaves the ownership checker without a defining source.
use wow_packet::packets::misc::BindPointUpdate;
use wow_packet::packets::update::{PlayerCombatStats, UpdateBlock, UpdateObject};
use wow_packet::{ClientPacket, WorldPacket};
use wow_persistence::{
    PlayerInitialWorldStateRowsLikeCpp, PlayerLoginTransportLoadOutcomeLikeCpp,
    PlayerLoginTransportLoadRequestLikeCpp,
};

use crate::handlers::quest::RepresentedQuestGiverStatusSourceLikeCpp;
use crate::session::{
    ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP, CreatureSpawnCatalogsLikeCpp, GLOBAL_CACHE_MASK_LIKE_CPP,
    PlayerBootstrapCatalogsLikeCpp, REST_STATE_NORMAL_LIKE_CPP,
    RepresentedAutoUnequipOffhandLikeCpp, RepresentedBankItemMoveLikeCpp,
    RepresentedGameObjectUseState, RepresentedHomebindLikeCpp,
    RepresentedQuestObjectiveProgressEventLikeCpp, RepresentedVoidStorageItemLikeCpp,
    SpellCastMetadata, SupportFeaturePolicyLikeCpp,
};
use crate::terrain::zone_and_area_for_position_like_cpp;
// Test-only: its production caller moved to `wow-world-character` (#1263 F5, #1363 A5).
#[cfg(test)]
use crate::session::REST_STATE_RAF_LINKED_LIKE_CPP;
#[cfg(test)]
use crate::session::{RepresentedAlterAppearanceLikeCpp, RepresentedConfirmBarbersChoiceLikeCpp};
pub(crate) use creation_support::default_display_id;
use creation_support::{default_health_mana, restored_saved_health_like_cpp};
// Test-only: their production callers moved to `wow-world-character` (#1263 F5, #1363 A5).
#[cfg(test)]
use creation_support::{default_character_power1_like_cpp, start_position, start_zone};
#[cfg(test)]
use enumeration_support::{enum_character_flags_like_cpp, enum_character_pet_data_like_cpp};
use item_load_support::*;
use login_support::*;
pub(crate) use login_transport_support::player_visibility_create_update_from_snapshot_like_cpp;
use login_transport_support::{
    InitTransportsPlanLikeCpp, MapTransportCreateLikeCpp, PersistedTransportLoginLikeCpp,
    TransportCreatePositionLikeCpp, compose_init_self_create_blocks_like_cpp,
    map_transport_create_block_like_cpp, map_transport_create_from_load_row_like_cpp,
    object_guid_from_db_binary_like_cpp, transport_position_for_login_like_cpp,
    transport_route_contains_saved_map_like_cpp, validate_persisted_transport_login_like_cpp,
};
#[cfg(test)]
use wow_entities::GAMEOBJECT_TYPE_GOOBER;
use wow_progression::mgr::CharacterReputationRowLikeCpp;

// ── Handler registration ────────────────────────────────────────────

const DEFAULT_MOTD_LIKE_CPP: &str = "Welcome to a Trinity Core Server.";
const DIRECT_VENDOR_MASK_LIKE_CPP: u32 = 0x80 | 0x100 | 0x200 | 0x400 | 0x800;
const DIRECT_TRAINER_MASK_LIKE_CPP: u32 = 0x10 | 0x20 | 0x40;
const DIRECT_FLIGHT_MASTER_LIKE_CPP: u32 = 0x2000;
const DIRECT_AUCTIONEER_LIKE_CPP: u32 = 0x200000;
const DIRECT_BANKER_LIKE_CPP: u32 = 0x20000;
const DIRECT_TABARD_DESIGNER_LIKE_CPP: u32 = 0x80000;
const DIRECT_STABLE_MASTER_LIKE_CPP: u32 = 0x400000;
const DIRECT_GUILD_BANKER_LIKE_CPP: u32 = 0x800000;
const DIRECT_INTERACTION_MASK_LIKE_CPP: u32 = DIRECT_VENDOR_MASK_LIKE_CPP
    | DIRECT_TRAINER_MASK_LIKE_CPP
    | DIRECT_FLIGHT_MASTER_LIKE_CPP
    | DIRECT_AUCTIONEER_LIKE_CPP
    | DIRECT_BANKER_LIKE_CPP
    | DIRECT_TABARD_DESIGNER_LIKE_CPP
    | DIRECT_STABLE_MASTER_LIKE_CPP
    | DIRECT_GUILD_BANKER_LIKE_CPP;

fn npc_has_direct_interaction_like_cpp(npc_flags: u32) -> bool {
    npc_flags & DIRECT_INTERACTION_MASK_LIKE_CPP != 0
}
const DEFAULT_GOSSIP_MESSAGE_LIKE_CPP: i32 = 0x00FF_FFFF;
const TRAINER_NPC_FLAGS_MASK_LIKE_CPP: u32 = 0x10 | 0x20 | 0x40;
const GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP: i32 = -1;
const GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP: u8 = 3;
const GOSSIP_OPTION_TRAINER_TEXT_LIKE_CPP: &str = "I would like to train.";

pub(crate) use wow_world_core::corpse_load::MapCorpseLoadOutcomeLikeCpp;

fn motd_lines_like_cpp(motd: &str) -> Vec<String> {
    // C++ `World::SetMotd` uses `boost::split` on `@` with token compression
    // disabled, so empty and trailing lines remain part of the login burst.
    motd.split('@').map(ToOwned::to_owned).collect()
}

fn void_storage_login_context_like_cpp(
    random_properties_id: i32,
    _selected_context_column: u8,
) -> u8 {
    // Audited 3.4.3 `Player::_LoadVoidStorage` constructs ItemContext from
    // fields[5] even though CHAR_SEL_CHAR_VOID_STORAGE selects `context` as
    // fields[7]. Keep that executable C++ behavior; the unused argument makes
    // the query/implementation mismatch explicit instead of hiding column 7.
    random_properties_id as u8
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DirectInventoryPositionUpdateLikeCpp {
    slot: u8,
    item_db_guid: u64,
}

use wow_world_application::InventorySwapTargetLikeCpp;
// The StorageMove vocabulary moved to its application owner (#1263 F4); these
// re-exports keep the handler tree's existing paths resolving unchanged.
pub(crate) use wow_world_application::{
    InventoryStorageQuestChecksLikeCpp, InventoryStorageTargetLikeCpp,
    autostore_bank_target_like_cpp,
};
// Test-only: the StorageMove free helpers moved to `wow-world-application`
// (#1263 F4) and their production callers are that owner's own body.
#[cfg(test)]
pub(crate) use wow_world_application::{
    bank_store_destination_applies_obtain_spells_like_cpp,
    bank_store_item_added_quest_count_like_cpp, inventory_storage_move_quest_directions_like_cpp,
};

fn autostore_bank_quest_checks_like_cpp(
    target: InventoryStorageTargetLikeCpp,
) -> InventoryStorageQuestChecksLikeCpp {
    if target == InventoryStorageTargetLikeCpp::Inventory {
        InventoryStorageQuestChecksLikeCpp::AutoStoreBankItemAdded
    } else {
        // C++ HandleAutoStoreBankItemOpcode intentionally does not call
        // ItemRemovedQuestCheck in its inventory-to-bank branch.
        InventoryStorageQuestChecksLikeCpp::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InventoryEquipChildPlanLikeCpp {
    child_guid: ObjectGuid,
    destination_slot: u8,
    displaced_storage: Option<(u8, u8, InventoryStorageTargetLikeCpp)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InventorySwapStepLikeCpp {
    Done,
    ChildRedirect {
        first_src: u16,
        first_dst: u16,
        second_src: u16,
        second_dst: u16,
    },
}

#[cfg(test)]
fn plan_direct_inventory_swap_persistence_like_cpp(
    src: u8,
    dst: u8,
    src_item: Option<&InventoryItem>,
    dst_item: Option<&InventoryItem>,
) -> Vec<DirectInventoryPositionUpdateLikeCpp> {
    let mut updates = Vec::with_capacity(2);
    if let Some(item) = src_item {
        updates.push(DirectInventoryPositionUpdateLikeCpp {
            slot: dst,
            item_db_guid: item.db_guid,
        });
    }
    if let Some(item) = dst_item {
        updates.push(DirectInventoryPositionUpdateLikeCpp {
            slot: src,
            item_db_guid: item.db_guid,
        });
    }
    updates
}

fn creature_has_trainer_flag_like_cpp(npc_flags: u32) -> bool {
    (npc_flags & TRAINER_NPC_FLAGS_MASK_LIKE_CPP) != 0
}

fn represented_trainer_gossip_option_like_cpp() -> wow_packet::packets::gossip::ClientGossipOption {
    wow_packet::packets::gossip::ClientGossipOption {
        gossip_option_id: GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP,
        option_npc: GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP,
        option_flags: 0,
        option_cost: 0,
        option_language: 0,
        flags: 0,
        order_index: 0,
        status: 0,
        text: GOSSIP_OPTION_TRAINER_TEXT_LIKE_CPP.to_string(),
        confirm: String::new(),
        spell_id: None,
        override_icon_id: None,
    }
}

fn represented_trainer_gossip_option_info_like_cpp() -> crate::session::GossipOptionInfo {
    crate::session::GossipOptionInfo {
        gossip_option_id: GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP,
        menu_id: 0,
        order_index: 0,
        option_npc: GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP,
        action_menu_id: 0,
    }
}

fn add_represented_trainer_gossip_option_if_missing_like_cpp(
    gossip_options: &mut Vec<wow_packet::packets::gossip::ClientGossipOption>,
    stored_options: &mut Vec<crate::session::GossipOptionInfo>,
    npc_flags: u32,
) -> bool {
    if !creature_has_trainer_flag_like_cpp(npc_flags) {
        return false;
    }

    if gossip_options
        .iter()
        .any(|option| option.option_npc == GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP)
    {
        return false;
    }

    gossip_options.push(represented_trainer_gossip_option_like_cpp());
    stored_options.push(represented_trainer_gossip_option_info_like_cpp());
    true
}
fn primary_power_type_for_class_like_cpp(class_id: u8) -> PowerType {
    wow_world_core::session::primary_power_type_for_player_class_like_cpp(class_id)
}

fn primary_max_power_for_class_like_cpp(class_id: u8, max_mana: i64) -> i32 {
    wow_world_inventory::primary_max_power_for_class_like_cpp(class_id, max_mana)
}

fn loaded_inventory_slot_count_with_legacy_rust_compat(saved_slots: u8) -> u8 {
    // C++ loads the saved value directly, but TrinityCore's schema defaults
    // inventorySlots to the base backpack size. Older RustyCore builds
    // explicitly inserted zero before this field was wired; keep those
    // already-created characters playable without an out-of-band migration.
    if saved_slots == 0 {
        INVENTORY_DEFAULT_SIZE
    } else {
        saved_slots
    }
}

#[cfg(test)]
const TACT_KEY_TABLE_HASH_LIKE_CPP: u32 = 0xDF2F_53CF;
const QUEST_GIVER_STATUS_TRACKED_QUERY_MAX_GUIDS_LIKE_CPP: u32 = 1000;
pub(crate) use wow_constants::character::RESPONSE_SUCCESS_LIKE_CPP;
const CHAR_CREATE_ERROR_LIKE_CPP: u8 = 25;
pub(crate) use wow_constants::character::{
    AT_LOGIN_CHANGE_FACTION_LIKE_CPP, AT_LOGIN_CHANGE_RACE_LIKE_CPP, AT_LOGIN_FIRST_LIKE_CPP,
    AT_LOGIN_RENAME_LIKE_CPP, AT_LOGIN_RESURRECT_LIKE_CPP, CHAR_CUSTOMIZE_FLAG_CUSTOMIZE_LIKE_CPP,
    CHAR_CUSTOMIZE_FLAG_FACTION_LIKE_CPP, CHAR_CUSTOMIZE_FLAG_RACE_LIKE_CPP,
    CHARACTER_FLAG_DECLINED_LIKE_CPP, CHARACTER_FLAG_GHOST_LIKE_CPP,
    CHARACTER_FLAG_RENAME_LIKE_CPP, CLASS_DEATH_KNIGHT_LIKE_CPP, CLASS_HUNTER_LIKE_CPP,
    CLASS_WARLOCK_LIKE_CPP, PLAYER_FLAGS_GHOST_LIKE_CPP,
};
// Test-only: their production readers moved to `wow-world-character` (#1263 F5, #1363 A5).
#[cfg(test)]
pub(crate) use wow_constants::character::{
    AT_LOGIN_CUSTOMIZE_LIKE_CPP, CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP,
};
pub(crate) use wow_constants::character::{
    CHAR_NAME_INVALID_CHARACTER_LIKE_CPP, CHAR_NAME_NO_NAME_LIKE_CPP, CHAR_NAME_TOO_LONG_LIKE_CPP,
    CHAR_NAME_TOO_SHORT_LIKE_CPP,
};
/// C++ `Player::Create` seeds the represented RAF rest state from the account's
/// recruiter role.
///
/// The body moved to the `wow-world-character` owner (#1263 F5, #1363 A5);
/// the character scenarios still call this entry point.
#[cfg(test)]
fn initial_character_rest_state_like_cpp(is_a_recruiter: bool, recruiter_id: u32) -> u8 {
    wow_world_character::initial_character_rest_state_like_cpp(is_a_recruiter, recruiter_id)
}

use wow_packet::packets::gossip::*;
#[cfg(test)]
use wow_packet::packets::query::*;

use crate::session::{InventoryItem, WorldSession};

// ── Hardcoded data ──────────────────────────────────────────────────

/// Reverse-map an equipment slot (0-18) to its InventoryType.
///
/// Used as a fallback when Item.db2 store is not available.
fn slot_to_inventory_type(slot: u8) -> Option<u8> {
    match slot {
        0 => Some(1),        // Head
        1 => Some(2),        // Neck
        2 => Some(3),        // Shoulders
        3 => Some(4),        // Body (Shirt)
        4 => Some(5),        // Chest
        5 => Some(6),        // Waist
        6 => Some(7),        // Legs
        7 => Some(8),        // Feet
        8 => Some(9),        // Wrists
        9 => Some(10),       // Hands
        10 | 11 => Some(11), // Finger (Ring)
        12 | 13 => Some(12), // Trinket
        14 => Some(16),      // Cloak
        15 => Some(21),      // MainHand (WeaponMainHand)
        16 => Some(22),      // OffHand (WeaponOffHand)
        17 => Some(15),      // Ranged
        18 => Some(19),      // Tabard
        _ => None,
    }
}

/// Parse a space-separated equipment cache string into VisualItemInfo array.
///
/// C++ `EnumCharactersResult::CharacterInfo` parses `equipmentCache` as five
/// fields per slot: InvType, DisplayID, DisplayEnchantID, Subclass, and
/// SecondaryItemModifiedAppearanceID. The body moved to the
/// `wow-world-character` owner (#1263 F5, #1363 A5); the character scenarios
/// still call this entry point.
#[cfg(test)]
fn parse_equipment_cache(cache: &str) -> [VisualItemInfo; 34] {
    wow_world_character::parse_equipment_cache(cache)
}

use wow_world_application::{
    bind_inventory_item_for_destination_like_cpp, item_dynamic_flags_changed_like_cpp,
};

// Test-only: the spawn-row helpers moved to `wow_world_entities::visibility_session`
// with the visibility bodies (#1363 slice 8a); the creature scenarios still call them.
#[cfg(test)]
use wow_world_entities::{
    WAYPOINT_MOTION_TYPE_LIKE_CPP, choose_creature_flags_like_cpp,
    creature_create_movement_flags_like_cpp, creature_create_position_after_hover_offset_like_cpp,
    creature_movement_generator_type_from_db_like_cpp,
    normalize_creature_template_speed_run_like_cpp,
    normalize_creature_template_speed_walk_like_cpp, normalized_creature_wander_distance_like_cpp,
    spawn_difficulties_contains_spawn_mode_like_cpp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DestroyItemCountAction {
    FullStack,
    PartialStack { new_count: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DestroyQuestItemLikeCpp {
    bag: u8,
    slot: u8,
    entry_id: u32,
    count: u32,
}

fn destroy_item_count_action(current_count: u32, requested_count: u32) -> DestroyItemCountAction {
    if requested_count != 0 && current_count > requested_count {
        return DestroyItemCountAction::PartialStack {
            new_count: current_count - requested_count,
        };
    }

    DestroyItemCountAction::FullStack
}

#[cfg(test)]
use wow_world_application::item_spell_charges_db_string;
use wow_world_application::item_storage_mutable_persistence_like_cpp;

#[cfg(test)]
#[path = "../../../unit_tests/handlers/character_vendor_atomicity_tests.rs"]
mod vendor_atomicity_tests;

#[cfg(test)]
#[path = "../../../unit_tests/handlers/character_tests.rs"]
pub(crate) mod tests;
