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
#[cfg(test)]
mod bank_test_support;
mod condition_objects;
mod corpse_loading;
mod creation_support;
mod creature_spawn;
mod entry_zone;
mod enumeration_support;
mod gossip;
mod inventory_plan;
mod item_actions;
mod item_load_support;
mod items;
mod lifecycle;
mod login_context;
mod login_support;
mod login_transport_support;
mod pets;
mod query;
mod session_state;
mod spell_rules;
mod stats;
mod stats_queries;
mod stats_update;
mod trainer_gossip;
mod vendor;
mod vendor_admission;
mod visibility;
mod world_entry;

use self::corpse_loading::*;
use self::creature_spawn::*;
pub(in crate::handlers::character) use self::creature_spawn::CreatureAddonCreateFieldsLikeCpp;
use self::inventory_plan::*;
use self::item_actions::*;
pub(in crate::handlers) use self::item_actions::ExtendedCostItemTurninChange;
use self::login_context::*;
use self::trainer_gossip::*;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::f32::consts::PI;
use std::sync::Arc;

use rand::Rng;
pub(crate) use stats::RepresentedPlayerGearStatsLikeCpp;
use tracing::{debug, info, trace, warn};
use wow_constants::movement::MovementFlag;
use wow_constants::unit::{
    NPCFlags1, SheathState, UNIT_FLAGS_ALLOWED_LIKE_CPP, UNIT_FLAGS2_ALLOWED_LIKE_CPP,
    UNIT_FLAGS3_ALLOWED_LIKE_CPP, UnitFlags,
};
use wow_constants::{
    ClientOpcodes, ConditionSourceType, CreatureFlagsExtra, EnchantmentSlot, InventoryResult,
    InventoryType, ItemBondingType, ItemContext, ItemExtendedCostFlags, ItemFieldFlags, ItemFlags,
    ItemFlags2, ItemModifier, ItemUpdateState, ItemVendorType, PowerType, Team, TypeId, TypeMask,
    UnitStandStateType,
};
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_crypto::rsa_sign::rsa_sign_connect_to;
#[cfg(test)]
use wow_data::PlayerCreatePositionLikeCpp;
use wow_data::{
    ConditionEntriesByTypeStore, ConditionId, CurrencyTypesStore, HotfixRecordStatus,
    ItemExtendedCostStore, PlayerConditionContextLikeCpp, PlayerConditionStore,
    PlayerCreateInfoLikeCpp, PlayerSpellBonusInputLikeCpp, PlayerStatSystemInputLikeCpp,
    PlayerStatSystemProjectionLikeCpp, TaxiPathNodeEntry, TaxiPathNodeStore,
    calculate_player_stat_system_like_cpp, hotfix_locale_mask,
    is_player_meeting_condition_like_cpp,
};
use wow_entities::{
    BANK_SLOT_BAG_END, BANK_SLOT_BAG_START, BUYBACK_SLOT_START, Corpse, CorpseCustomizationChoice,
    CorpseType, CreatureAddonLifecycleRecordLikeCpp, GAMEOBJECT_TYPE_FISHING_HOLE,
    GAMEOBJECT_TYPE_QUESTGIVER, GameObjectTemplateData, INVENTORY_DEFAULT_SIZE,
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START,
    INVENTORY_SLOT_ITEM_START, InventoryStorageMovePlanLikeCpp, MAX_BAG_SIZE, MAX_MONEY_AMOUNT,
    MovementGeneratorType, NULL_BAG, NULL_SLOT, PlayerEffectiveCombatStatsLikeCpp,
    REAGENT_BAG_SLOT_END, REAGENT_BAG_SLOT_START, SendNewItemDelivery, SendNewItemDisplayText,
    SendNewItemInstancePlan, SendNewItemModifier, SendNewItemPlan, SocketedGem,
    SwapItemPreflightResult, WorldObject, is_bank_pos, is_child_equipment_pos, is_equipment_pos,
    is_inventory_pos, item_can_go_into_bag, normalize_creature_chase_movement_type_like_cpp,
    normalize_creature_random_movement_type_like_cpp,
};
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::packets::auth::{
    ConnectTo, ConnectToAddress, ConnectToFailed, ConnectToKey, ConnectToSerial, ResumeComms,
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
use wow_packet::packets::update::{
    ItemCreateData, PlayerCombatStats, UpdateBlock, UpdateObject, UpdateType,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_persistence::{
    PlayerInitialWorldStateRowsLikeCpp, PlayerLoginTransportLoadOutcomeLikeCpp,
    PlayerLoginTransportLoadRequestLikeCpp, PlayerLoginTransportLoadRowLikeCpp,
};

use crate::handlers::quest::RepresentedQuestGiverStatusSourceLikeCpp;
use crate::map_manager::zone_and_area_for_position_like_cpp;
use crate::session::{
    ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP, CharacterPetAuraEffectRowLikeCpp,
    CharacterPetAuraRowLikeCpp, CharacterPetDeclinedNamesRowLikeCpp,
    CharacterPetSpellChargeRowLikeCpp, CharacterPetSpellCooldownRowLikeCpp,
    CharacterPetSpellRowLikeCpp, CharacterPetStableRowLikeCpp, CreatureSpawnCatalogsLikeCpp,
    GLOBAL_CACHE_MASK_LIKE_CPP, PlayerBootstrapCatalogsLikeCpp, REST_STATE_NORMAL_LIKE_CPP,
    REST_STATE_RAF_LINKED_LIKE_CPP, RepresentedAlterAppearanceLikeCpp,
    RepresentedAutoUnequipOffhandLikeCpp, RepresentedBankItemMoveLikeCpp,
    RepresentedConfirmBarbersChoiceLikeCpp, RepresentedGameObjectUseState,
    RepresentedHomebindLikeCpp, RepresentedQuestObjectiveProgressEventLikeCpp,
    RepresentedVoidStorageItemLikeCpp, SpellCastMetadata, SupportFeaturePolicyLikeCpp,
};
pub(crate) use creation_support::default_display_id;
use creation_support::{
    default_character_power1_like_cpp, default_health_mana, max_health_u32_like_cpp,
    restored_saved_health_like_cpp, start_position, start_zone,
};
use enumeration_support::{
    EnumCharacterFlagsLikeCpp, enum_character_effective_player_flags_like_cpp,
    enum_character_flags_like_cpp, enum_character_pet_data_like_cpp,
};
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
const WORLDSTATE_ANY_MAP_LIKE_CPP: i32 = -1;
const DEFAULT_GOSSIP_MESSAGE_LIKE_CPP: i32 = 0x00FF_FFFF;
const TRAINER_NPC_FLAGS_MASK_LIKE_CPP: u32 = 0x10 | 0x20 | 0x40;
const GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP: i32 = -1;
const GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP: u8 = 3;
const GOSSIP_OPTION_TRAINER_TEXT_LIKE_CPP: &str = "I would like to train.";
const ITEM_ENCHANTMENT_DB_FIELDS: usize = 3;

const WAYPOINT_MOTION_TYPE_LIKE_CPP: u8 = 2;
const TACT_KEY_TABLE_HASH_LIKE_CPP: u32 = 0xDF2F_53CF;
const QUEST_GIVER_STATUS_TRACKED_QUERY_MAX_GUIDS_LIKE_CPP: u32 = 1000;
const MAX_AREA_SPIRIT_HEALER_RANGE_LIKE_CPP: f32 = 20.0;
// C++ ObjectDefines.h: DEFAULT_VISIBILITY_DISTANCE = VISIBILITY_DISTANCE_NORMAL = 100 yards.
// Wider values here make the SQL fallback load whole areas and can crash the 3.4.3 client.
const DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP: f32 = crate::map_manager::VISIBILITY_RADIUS;
pub(crate) use wow_constants::character::RESPONSE_SUCCESS_LIKE_CPP;
const CHAR_CREATE_ERROR_LIKE_CPP: u8 = 25;
const CHAR_CREATE_NAME_IN_USE_LIKE_CPP: u8 = 27;
pub(crate) use wow_constants::character::{
    CHAR_NAME_INVALID_CHARACTER_LIKE_CPP, CHAR_NAME_NO_NAME_LIKE_CPP, CHAR_NAME_TOO_LONG_LIKE_CPP,
    CHAR_NAME_TOO_SHORT_LIKE_CPP,
};
const CLASS_HUNTER_LIKE_CPP: u8 = 3;
const CLASS_DEATH_KNIGHT_LIKE_CPP: u8 = 6;
const CLASS_WARLOCK_LIKE_CPP: u8 = 9;
const PLAYER_FLAGS_GHOST_LIKE_CPP: u32 = 0x0000_0010;
const AT_LOGIN_RENAME_LIKE_CPP: u16 = 0x001;
const AT_LOGIN_CUSTOMIZE_LIKE_CPP: u16 = 0x008;
const AT_LOGIN_FIRST_LIKE_CPP: u16 = 0x020;
const AT_LOGIN_CHANGE_FACTION_LIKE_CPP: u16 = 0x040;
const AT_LOGIN_CHANGE_RACE_LIKE_CPP: u16 = 0x080;
const AT_LOGIN_RESURRECT_LIKE_CPP: u16 = 0x100;
const CHARACTER_FLAG_LOCKED_FOR_TRANSFER_LIKE_CPP: u32 = 0x0000_0004;
const CHARACTER_FLAG_GHOST_LIKE_CPP: u32 = 0x0000_2000;
const CHARACTER_FLAG_RENAME_LIKE_CPP: u32 = 0x0000_4000;
const CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP: u32 = 0x0100_0000;
const CHARACTER_FLAG_DECLINED_LIKE_CPP: u32 = 0x0200_0000;
const CHAR_CUSTOMIZE_FLAG_CUSTOMIZE_LIKE_CPP: u32 = 0x0000_0001;
const CHAR_CUSTOMIZE_FLAG_FACTION_LIKE_CPP: u32 = 0x0001_0000;
const CHAR_CUSTOMIZE_FLAG_RACE_LIKE_CPP: u32 = 0x0010_0000;
const GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_LIKE_CPP: u8 = 15;
const TAXI_PATH_NODE_FLAG_TELEPORT_LIKE_CPP: i32 = 0x1;
const TAXI_PATH_NODE_FLAG_STOP_LIKE_CPP: i32 = 0x2;

use wow_packet::packets::gossip::*;
use wow_packet::packets::query::*;

use crate::session::{InventoryItem, WorldSession};

// ── Hardcoded data ──────────────────────────────────────────────────

/// Maximum characters per account.
const MAX_CHARACTERS_PER_ACCOUNT: u32 = 10;

#[cfg(test)]
#[path = "../character_vendor_atomicity_tests.rs"]
mod vendor_atomicity_tests;

#[cfg(test)]
#[path = "../character_tests.rs"]
pub(crate) mod tests;
