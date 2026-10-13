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
// Test-only: its production readers moved to `wow-world-character` (#1263 F5, #1363 A5).
#[cfg(test)]
mod enumeration_support;
mod gossip;
mod items;
mod login_host;
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

use std::collections::{BTreeMap, HashMap, HashSet};
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
    ConditionSourceType, InventoryResult, ItemContext, TypeId, TypeMask, UnitStandStateType,
};
// Test-only: their production readers moved to `wow-world-inventory` (#1363 slice 8b)
// and `wow-world-character` (#1363 slice 8d).
#[cfg(test)]
use wow_constants::{InventoryType, ItemFieldFlags, ItemVendorType, PowerType};
// Test-only: its production readers were the character/account registrations that
// moved to `wow-world-application` under #1263 F5; the character scenarios still
// reach these three names through `super::*`.
#[cfg(test)]
use wow_constants::ClientOpcodes;
use wow_core::{ObjectGuid, Position};
use wow_crypto::rsa_sign::rsa_sign_connect_to;
#[cfg(test)]
use wow_data::{PlayerCreateInfoLikeCpp, PlayerCreatePositionLikeCpp};
// Test-only: its production caller moved to `wow-world-character` (#1263 F5, #1363 A5).
#[cfg(test)]
use wow_data::PlayerStatSystemProjectionLikeCpp;
use wow_data::{
    ConditionEntriesByTypeStore, ConditionId, CurrencyTypesStore, HotfixRecordStatus,
    ItemExtendedCostStore, PlayerConditionContextLikeCpp, PlayerConditionStore,
    PlayerSpellBonusInputLikeCpp, PlayerStatSystemInputLikeCpp,
    calculate_player_stat_system_like_cpp, hotfix_locale_mask,
    is_player_meeting_condition_like_cpp,
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
    GAMEOBJECT_TYPE_QUESTGIVER, INVENTORY_DEFAULT_SIZE, SendNewItemPlan, WorldObject,
};
// Test-only: their production readers moved to `wow-world-inventory` (#1363 slice 8b)
// and `wow-world-character` (#1363 slice 8d).
#[cfg(test)]
use wow_entities::{INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_START};
// Test-only: the production registration source that used these two moved to the
// application crate's character/account area registrar under #1263 F5.
#[cfg(test)]
use wow_handler::{PacketProcessing, SessionStatus};
#[cfg(test)]
pub(crate) use wow_world_lifecycle::login_transport::{
    GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_LIKE_CPP,
    player_visibility_create_update_from_snapshot_like_cpp,
};

// The character/account packet registrations that used to reach
// `crate::session::registry::PacketHandlerEntry` from here moved to the
// application crate's explicit area registrar under #1263 F5, so this module no
// longer needs the entry type in scope.
use wow_packet::packets::auth::{
    ConnectTo, ConnectToAddress, ConnectToKey, ConnectToSerial, ResumeComms,
};
use wow_packet::packets::misc::*;
use wow_packet::packets::quest::QuestGiverStatusMultiple;
use wow_packet::packets::spell::{SpellCastVisual, SpellTargetData};
use wow_packet::packets::update::*;
// Explicit provenance for child modules that reach these names through `super::{}`:
// a glob import leaves the ownership checker without a defining source.
use wow_packet::WorldPacket;
use wow_packet::packets::misc::BindPointUpdate;
use wow_packet::packets::update::PlayerCombatStats;

use crate::handlers::quest::RepresentedQuestGiverStatusSourceLikeCpp;
use crate::session::{
    CreatureSpawnCatalogsLikeCpp, PlayerBootstrapCatalogsLikeCpp, REST_STATE_NORMAL_LIKE_CPP,
    RepresentedBankItemMoveLikeCpp, RepresentedGameObjectUseState, RepresentedHomebindLikeCpp,
    RepresentedQuestObjectiveProgressEventLikeCpp, RepresentedVoidStorageItemLikeCpp,
    SpellCastMetadata, SupportFeaturePolicyLikeCpp,
};
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
use login_support::*;
use login_transport_support::{MapTransportCreateLikeCpp, TransportCreatePositionLikeCpp};
#[cfg(test)]
use wow_entities::GAMEOBJECT_TYPE_GOOBER;
use wow_progression::mgr::CharacterReputationRowLikeCpp;

// Test-only: the scenarios reach these through `super::*`; their production
// readers moved to `wow-world-character` (#1363 slice 8d).
#[cfg(test)]
use {
    login_transport_support::{
        compose_init_self_create_blocks_like_cpp, transport_route_contains_saved_map_like_cpp,
        validate_persisted_transport_login_like_cpp,
    },
    wow_constants::character::RESPONSE_SUCCESS_LIKE_CPP,
    wow_core::guid::HighGuid,
    wow_packet::packets::{character::*, movement::TransportInfo},
    wow_world_character::{motd_lines_like_cpp, void_storage_login_context_like_cpp},
    wow_world_core::corpse_load::MapCorpseLoadOutcomeLikeCpp,
    wow_world_inventory::loaded_item_support::*,
};

// ── Handler registration ────────────────────────────────────────────

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
};
// Test-only: the StorageMove free helpers moved to `wow-world-application`
// (#1263 F4) and their production callers are that owner's own body.
#[cfg(test)]
pub(crate) use wow_world_application::{
    bank_store_destination_applies_obtain_spells_like_cpp,
    bank_store_item_added_quest_count_like_cpp, inventory_storage_move_quest_directions_like_cpp,
};

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
// Test-only since #1363 slice 8d (the rename result uses the character crate's own).
#[cfg(test)]
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

#[cfg(test)]
use wow_world_application::item_spell_charges_db_string;

// Test-only (#1363 slice 8c): the production readers of these names moved to
// `wow-world-inventory` with the item handler bodies; the scenarios still use them.
#[cfg(test)]
use std::collections::BTreeSet;
#[cfg(test)]
use wow_entities::{INVENTORY_SLOT_BAG_0, NULL_BAG, NULL_SLOT, SwapItemPreflightResult};
#[cfg(test)]
use wow_packet::packets::item::*;
#[cfg(test)]
use wow_world_inventory::{
    DestroyQuestItemLikeCpp, InventoryEquipChildPlanLikeCpp, autostore_bank_quest_checks_like_cpp,
    autostore_bank_target_like_cpp, bind_inventory_item_for_destination_like_cpp,
    item_dynamic_flags_changed_like_cpp, item_storage_mutable_persistence_like_cpp,
};

#[cfg(test)]
#[path = "../../../unit_tests/handlers/character_vendor_atomicity_tests.rs"]
mod vendor_atomicity_tests;

#[cfg(test)]
#[path = "../../../unit_tests/handlers/character_tests.rs"]
pub(crate) mod tests;
