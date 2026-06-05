// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handlers: enum, create, delete, and player login.

use std::collections::HashSet;
use std::sync::Arc;

use rand::Rng;
use tracing::{debug, info, trace, warn};
use wow_constants::unit::NPCFlags1;
use wow_constants::{
    ClientOpcodes, ConditionSourceType, InventoryResult, InventoryType, ItemBondingType,
    ItemContext, ItemExtendedCostFlags, ItemFieldFlags, ItemFlags, ItemFlags2, ItemUpdateState,
    ItemVendorType, Team, TypeId, TypeMask, UnitStandStateType,
};
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_crypto::rsa_sign::rsa_sign_connect_to;
use wow_data::{
    ConditionEntriesByTypeStore, ConditionId, CurrencyTypesStore, HotfixRecordStatus,
    ItemExtendedCostStore, PlayerConditionContextLikeCpp, PlayerConditionStore, hotfix_locale_mask,
    is_player_meeting_condition_like_cpp,
};
use wow_database::{
    CharStatements, CharacterDatabase, LoginStatements, SqlTransaction, WorldDatabase,
    WorldStatements,
};
use wow_entities::{
    BANK_SLOT_BAG_END, BANK_SLOT_BAG_START, BUYBACK_SLOT_START, GAMEOBJECT_TYPE_FISHING_HOLE,
    GAMEOBJECT_TYPE_QUESTGIVER, GameObjectTemplateData, INVENTORY_DEFAULT_SIZE,
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START,
    INVENTORY_SLOT_ITEM_START, MAX_BAG_SIZE, MAX_GAMEOBJECT_DATA, MovementGeneratorType, NULL_BAG,
    NULL_SLOT, REAGENT_BAG_SLOT_END, REAGENT_BAG_SLOT_START, WorldObject, is_equipment_pos,
    is_inventory_pos,
};
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::WorldPacket;
use wow_packet::packets::auth::{
    ConnectTo, ConnectToAddress, ConnectToFailed, ConnectToKey, ConnectToSerial, ResumeComms,
};
use wow_packet::packets::character::*;
use wow_packet::packets::item::*;
use wow_packet::packets::loot::LootReleaseAll;
use wow_packet::packets::misc::*;
use wow_packet::packets::quest::QuestGiverStatusMultiple;
use wow_packet::packets::update::*;

use crate::handlers::quest::RepresentedQuestGiverStatusSourceLikeCpp;
use crate::reputation::mgr::CharacterReputationRowLikeCpp;
use crate::session::RepresentedGameObjectUseState;

// ── Handler registration ────────────────────────────────────────────

const GO_SPAWN_TEMPLATE_DATA_START: usize = 16;
const GO_SPAWN_PHASE_USE_FLAGS_COLUMN: usize = GO_SPAWN_TEMPLATE_DATA_START + MAX_GAMEOBJECT_DATA;
const GO_SPAWN_PHASE_ID_COLUMN: usize = GO_SPAWN_PHASE_USE_FLAGS_COLUMN + 1;
const GO_SPAWN_PHASE_GROUP_COLUMN: usize = GO_SPAWN_PHASE_USE_FLAGS_COLUMN + 2;
const GO_SPAWN_TERRAIN_SWAP_MAP_COLUMN: usize = GO_SPAWN_PHASE_USE_FLAGS_COLUMN + 3;
const GO_SPAWN_EFFECTIVE_FLAGS_COLUMN: usize = GO_SPAWN_PHASE_USE_FLAGS_COLUMN + 4;
const GO_SPAWN_EFFECTIVE_FACTION_COLUMN: usize = GO_SPAWN_PHASE_USE_FLAGS_COLUMN + 5;
const GO_SPAWN_OVERRIDE_SOURCE_KNOWN_COLUMN: usize = GO_SPAWN_PHASE_USE_FLAGS_COLUMN + 6;
const CREATURE_SPAWN_EFFECTIVE_MOVEMENT_TYPE_COLUMN: usize = 35;
const CREATURE_SPAWN_WAYPOINT_PATH_ID_COLUMN: usize = 36;
const WAYPOINT_MOTION_TYPE_LIKE_CPP: u8 = 2;
const TACT_KEY_TABLE_HASH_LIKE_CPP: u32 = 0xD3F6_1A9E;
const QUEST_GIVER_STATUS_TRACKED_QUERY_MAX_GUIDS_LIKE_CPP: u32 = 1000;

fn creature_movement_generator_type_from_db_like_cpp(
    db_movement_type: u8,
) -> MovementGeneratorType {
    match db_movement_type {
        WAYPOINT_MOTION_TYPE_LIKE_CPP => MovementGeneratorType::Waypoint,
        _ => MovementGeneratorType::Idle,
    }
}

fn represented_go_state_from_i8_like_cpp(state: i8) -> Option<wow_entities::GoState> {
    match state {
        0 => Some(wow_entities::GoState::Active),
        1 => Some(wow_entities::GoState::Ready),
        2 => Some(wow_entities::GoState::Destroyed),
        24 => Some(wow_entities::GoState::TransportActive),
        25 => Some(wow_entities::GoState::TransportStopped),
        _ => None,
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::EnumCharacters,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_enum_characters",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CreateCharacter,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_create_character",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CharDelete,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_char_delete",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::PlayerLogin,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_player_login",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ConnectToFailed,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_connect_to_failed",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GetUndeleteCharacterCooldownStatus,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_get_undelete_cooldown_status",
    }
}

// ── Stub registrations for character-select opcodes ──────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ServerTimeOffsetRequest,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_server_time_offset_request",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestPlayedTime,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_played_time",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePayGetProductList,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pay_stub",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePayGetPurchaseList,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pay_stub",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateVasPurchaseStates,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_vas_stub",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SocialContractRequest,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_social_contract_stub",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DbQueryBulk,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_db_query_bulk",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::HotfixRequest,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_hotfix_request",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TimeSyncResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_time_sync_response",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TimeSyncResponseDropped,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_time_sync_response",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TimeSyncResponseFailed,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_time_sync_response",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LogoutRequest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_logout_request",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LogoutCancel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_logout_cancel",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryCreature,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_creature",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryGameObject,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_game_object",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryPageText,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_page_text",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryPlayerNames,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_player_names",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryRealmName,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_realm_name",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::Ping,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_ping",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TalkToGossip,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_gossip_hello",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GossipSelectOption,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_gossip_select_option",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryNpcText,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_npc_text",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ListInventory,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_list_inventory",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BuyItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_buy_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BuyBackItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_buy_back_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SellItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_sell_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ItemPurchaseRefund,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_item_purchase_refund",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionHelloRequest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_hello_request",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BankerActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_banker_activate",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BinderActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_binder_activate",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TabardVendorActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_tabard_vendor_activate",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SpiritHealerActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_spirit_healer_activate",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RepairItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_repair_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestStabledPets,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_stabled_pets",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverStatusMultipleQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_giver_status_multiple_query",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverStatusTrackedQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_status_tracked_query",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SwapInvItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_swap_inv_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutoEquipItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_auto_equip_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SwapItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_swap_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutoStoreBagItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_auto_store_bag_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DestroyItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_destroy_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ShowTradeSkill,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_show_trade_skill",
    }
}

use wow_packet::packets::gossip::*;
use wow_packet::packets::query::*;

use crate::session::{InventoryItem, PendingCreatureSpawn, WorldSession};

// ── Hardcoded data ──────────────────────────────────────────────────

/// Default start position for a race.
/// Returns (map_id, x, y, z, orientation).
fn start_position(race: u8) -> (i32, f32, f32, f32, f32) {
    match race {
        1 => (0, -8949.95, -132.493, 83.5312, 0.0),       // Human
        2 => (1, -618.518, -4251.67, 38.718, 0.0),        // Orc
        3 => (0, -6240.32, 331.033, 382.758, 6.17716),    // Dwarf
        4 => (1, 10311.3, 832.463, 1326.41, 5.69632),     // NightElf
        5 => (0, 1676.71, 1678.31, 121.67, 2.70526),      // Undead
        6 => (1, -2917.58, -257.98, 52.9968, 0.0),        // Tauren
        7 => (0, -6240.32, 331.033, 382.758, 0.0),        // Gnome
        8 => (1, -618.518, -4251.67, 38.718, 0.0),        // Troll
        10 => (530, 10349.6, -6357.29, 33.4026, 5.31605), // BloodElf
        11 => (530, -3961.64, -13931.2, 100.615, 2.08364), // Draenei
        22 => (0, -8949.95, -132.493, 83.5312, 0.0),      // Worgen → Human
        _ => (0, -8949.95, -132.493, 83.5312, 0.0),       // Default: Human
    }
}

/// Default display ID for a race/sex combination.
pub(crate) fn default_display_id(race: u8, sex: u8) -> u32 {
    match (race, sex) {
        (1, 0) => 49,
        (1, 1) => 50, // Human M/F
        (2, 0) => 51,
        (2, 1) => 52, // Orc
        (3, 0) => 53,
        (3, 1) => 54, // Dwarf
        (4, 0) => 55,
        (4, 1) => 56, // NightElf
        (5, 0) => 57,
        (5, 1) => 58, // Undead
        (6, 0) => 59,
        (6, 1) => 60, // Tauren
        (7, 0) => 1563,
        (7, 1) => 1564, // Gnome
        (8, 0) => 1478,
        (8, 1) => 1479, // Troll
        (10, 0) => 15476,
        (10, 1) => 15475, // BloodElf
        (11, 0) => 16125,
        (11, 1) => 16126, // Draenei
        _ => 49,          // Default: Human Male
    }
}

/// Default zone ID for a starting position.
fn start_zone(race: u8) -> i32 {
    match race {
        1 | 22 => 12, // Human / Worgen: Elwynn Forest
        2 | 8 => 14,  // Orc / Troll: Durotar
        3 | 7 => 1,   // Dwarf / Gnome: Dun Morogh
        4 => 141,     // NightElf: Teldrassil
        5 => 85,      // Undead: Tirisfal Glades
        6 => 215,     // Tauren: Mulgore
        10 => 3430,   // BloodElf: Eversong Woods
        11 => 3524,   // Draenei: Azuremyst Isle
        _ => 12,
    }
}

/// Default starting health and mana for a level 1 character by class.
fn default_health_mana(class: u8) -> (u32, u32) {
    match class {
        1 => (50, 0),   // Warrior — no mana
        2 => (52, 79),  // Paladin
        3 => (46, 85),  // Hunter (uses focus at high level, mana at 1)
        4 => (45, 0),   // Rogue — no mana
        5 => (52, 160), // Priest
        6 => (130, 0),  // Death Knight — no mana (runic power)
        7 => (47, 73),  // Shaman
        8 => (42, 200), // Mage
        9 => (43, 200), // Warlock
        11 => (54, 60), // Druid
        _ => (50, 100), // Default
    }
}

/// Maximum characters per account.
const MAX_CHARACTERS_PER_ACCOUNT: u32 = 10;

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
/// C# format: 5 values per slot (InvType, DisplayId, DisplayEnchantId, Subclass,
/// SecondaryItemModifiedAppearanceID), space-separated, up to 34 slots.
fn parse_equipment_cache(cache: &str) -> [VisualItemInfo; 34] {
    let mut equipment = [VisualItemInfo::default(); 34];
    if cache.is_empty() {
        return equipment;
    }

    let parts: Vec<&str> = cache.split_whitespace().collect();
    let fields_per_slot = 5;

    for slot in 0..34 {
        let base = slot * fields_per_slot;
        if base + fields_per_slot > parts.len() {
            break;
        }
        equipment[slot] = VisualItemInfo {
            inv_type: parts[base].parse().unwrap_or(0),
            display_id: parts[base + 1].parse().unwrap_or(0),
            display_enchant_id: parts[base + 2].parse().unwrap_or(0),
            subclass: parts[base + 3].parse().unwrap_or(0),
            secondary_item_modified_appearance_id: parts[base + 4].parse().unwrap_or(0),
        };
    }

    equipment
}

const MAX_MONEY_AMOUNT: u64 = 99_999_999_999;
const MAX_VENDOR_ITEMS_CPP: usize = 150;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VendorBuyItem {
    item_id: u32,
    item_type: i32,
    max_count: u32,
    incr_time: u32,
    player_condition_id: u32,
    has_vendor_conditions: bool,
    extended_cost: u32,
    buy_price: u64,
    max_durability: u32,
    buy_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VendorBuyTemplateBlock {
    BuyError(BuyResult),
    Silent,
}

fn vendor_buy_quantity_and_price(buy_price: u64, buy_count: u32, quantity: u32) -> (u32, u64) {
    if buy_price == 0 || quantity == 0 {
        return (quantity, 0);
    }

    let buy_price_per_item = buy_price as f64 / buy_count.max(1) as f64;
    let max_count = (MAX_MONEY_AMOUNT as f64 / buy_price_per_item) as u32;
    let quantity = quantity.min(max_count);
    let price = ((buy_price_per_item * quantity as f64) as u64).max(1);

    (quantity, price)
}

fn player_money_gain_like_cpp(current_money: u64, amount: u64) -> Option<u64> {
    if amount == 0 {
        return Some(current_money);
    }

    let max_gain = MAX_MONEY_AMOUNT.checked_sub(amount)?;
    if current_money <= max_gain {
        Some(current_money + amount)
    } else {
        None
    }
}

fn vendor_buy_packet_quantity_to_cpp_count(quantity: i32) -> u32 {
    u32::from((quantity as u8).max(1))
}

fn vendor_buy_currency_packet_quantity_to_cpp_count(quantity: i32) -> u32 {
    (quantity as u32).max(1)
}

fn vendor_list_reaches_cpp_item_limit(count: usize) -> bool {
    count >= MAX_VENDOR_ITEMS_CPP
}

fn vendor_list_should_skip_currency_row(
    currency_store: Option<&CurrencyTypesStore>,
    item_id: i32,
    extended_cost: i32,
) -> bool {
    if extended_cost == 0 {
        return true;
    }

    !vendor_currency_type_is_known(currency_store, item_id as u32)
}

fn vendor_currency_type_is_known(
    currency_store: Option<&CurrencyTypesStore>,
    currency_id: u32,
) -> bool {
    currency_store.is_some_and(|store| store.has_record(currency_id))
}

fn vendor_buy_currency_quantity_block_result(
    max_count: u32,
    quantity: u32,
) -> Option<InventoryResult> {
    if max_count == 0 || quantity % max_count != 0 {
        Some(InventoryResult::CantBuyQuantity)
    } else {
        None
    }
}

fn vendor_buy_muid_to_cpp_slot(muid: i32) -> Option<u32> {
    let muid = muid as u32;
    if muid > 0 { Some(muid - 1) } else { None }
}

fn vendor_player_condition_failed_id_like_cpp(
    player_condition_id: u32,
    store: Option<&PlayerConditionStore>,
    context: Option<PlayerConditionContextLikeCpp<'_>>,
) -> i32 {
    if player_condition_id == 0 {
        return 0;
    }

    let (Some(store), Some(context)) = (store, context) else {
        return player_condition_id as i32;
    };

    let Some(condition) = store.get(player_condition_id) else {
        return 0;
    };

    if is_player_meeting_condition_like_cpp(condition, &context) {
        0
    } else {
        player_condition_id as i32
    }
}

fn vendor_buy_player_condition_block_result_like_cpp(
    player_condition_id: u32,
    store: Option<&PlayerConditionStore>,
    context: Option<PlayerConditionContextLikeCpp<'_>>,
) -> Option<InventoryResult> {
    if vendor_player_condition_failed_id_like_cpp(player_condition_id, store, context) == 0 {
        None
    } else {
        Some(InventoryResult::ItemLocked)
    }
}

fn vendor_conditions_block_result(has_vendor_conditions: bool) -> Option<BuyResult> {
    if has_vendor_conditions {
        Some(BuyResult::CantFindItem)
    } else {
        None
    }
}

fn vendor_buy_required_reputation_block_result(
    required_reputation_faction: Option<u16>,
    required_reputation_rank: Option<i32>,
    player_reputation_rank: i32,
) -> Option<BuyResult> {
    if required_reputation_faction.unwrap_or(0) != 0
        && player_reputation_rank < required_reputation_rank.unwrap_or(0)
    {
        Some(BuyResult::ReputationRequire)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VendorExtendedCostBlock {
    Equip(InventoryResult),
    Buy(BuyResult),
    Silent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtendedCostItemTurninChange {
    Update {
        slot: u8,
        item_guid: ObjectGuid,
        db_guid: u64,
        new_count: u32,
    },
    Delete {
        slot: u8,
        item_guid: ObjectGuid,
        db_guid: u64,
    },
}

fn vendor_buy_extended_cost_block_result(
    extended_cost_store: Option<&ItemExtendedCostStore>,
    currency_store: Option<&CurrencyTypesStore>,
    has_item_count: impl Fn(u32, u32) -> bool,
    has_currency: impl Fn(u32, u32) -> bool,
    allow_currency_only_success: bool,
    extended_cost: u32,
    buy_count: u32,
    quantity: u32,
) -> Option<VendorExtendedCostBlock> {
    if extended_cost == 0 {
        return None;
    }

    if quantity % buy_count.max(1) != 0 {
        return Some(VendorExtendedCostBlock::Equip(
            InventoryResult::CantBuyQuantity,
        ));
    }

    let Some(extended_cost_entry) = extended_cost_store.and_then(|store| store.get(extended_cost))
    else {
        return Some(VendorExtendedCostBlock::Silent);
    };
    let stacks = quantity / buy_count.max(1);

    for (item_id, item_count) in extended_cost_entry
        .item_id
        .iter()
        .copied()
        .zip(extended_cost_entry.item_count.iter().copied())
    {
        if item_id == 0 {
            continue;
        }

        let Ok(item_id) = u32::try_from(item_id) else {
            return Some(VendorExtendedCostBlock::Equip(
                InventoryResult::VendorMissingTurnins,
            ));
        };
        let amount = u32::from(item_count).wrapping_mul(stacks);
        if !has_item_count(item_id, amount) {
            return Some(VendorExtendedCostBlock::Equip(
                InventoryResult::VendorMissingTurnins,
            ));
        }
    }

    for (i, currency_id) in extended_cost_entry.currency_id.iter().copied().enumerate() {
        if currency_id == 0 {
            continue;
        }

        let currency_id = u32::from(currency_id);
        if !vendor_currency_type_is_known(currency_store, currency_id) {
            return Some(VendorExtendedCostBlock::Buy(BuyResult::CantFindItem));
        }

        if item_extended_cost_currency_requires_season_earned(extended_cost_entry.flags, i)
            || !has_currency(
                currency_id,
                extended_cost_entry.currency_count[i].wrapping_mul(stacks),
            )
        {
            return Some(VendorExtendedCostBlock::Equip(
                InventoryResult::VendorMissingTurnins,
            ));
        }
    }

    if extended_cost_entry.required_arena_rating != 0 {
        return Some(VendorExtendedCostBlock::Equip(
            InventoryResult::CantEquipRank,
        ));
    }

    if extended_cost_entry.min_faction_id != 0 {
        return Some(VendorExtendedCostBlock::Buy(BuyResult::ReputationRequire));
    }

    if extended_cost_entry.requires_guild() || extended_cost_entry.required_achievement != 0 {
        return Some(VendorExtendedCostBlock::Equip(
            InventoryResult::VendorMissingTurnins,
        ));
    }

    if allow_currency_only_success {
        None
    } else {
        Some(VendorExtendedCostBlock::Equip(
            InventoryResult::VendorMissingTurnins,
        ))
    }
}

fn vendor_buy_extended_cost_item_costs(
    extended_cost_store: Option<&ItemExtendedCostStore>,
    extended_cost: u32,
    buy_count: u32,
    quantity: u32,
) -> Vec<(u32, u32)> {
    if extended_cost == 0 {
        return Vec::new();
    }
    let Some(extended_cost_entry) = extended_cost_store.and_then(|store| store.get(extended_cost))
    else {
        return Vec::new();
    };
    let stacks = quantity / buy_count.max(1);
    extended_cost_entry
        .item_id
        .iter()
        .copied()
        .zip(extended_cost_entry.item_count.iter().copied())
        .filter(|(item_id, _)| *item_id > 0)
        .map(|(item_id, count)| {
            (
                u32::try_from(item_id).unwrap_or(0),
                u32::from(count).wrapping_mul(stacks),
            )
        })
        .collect()
}

fn vendor_buy_extended_cost_currency_costs(
    extended_cost_store: Option<&ItemExtendedCostStore>,
    extended_cost: u32,
    buy_count: u32,
    quantity: u32,
) -> Vec<(u32, u32)> {
    if extended_cost == 0 {
        return Vec::new();
    }
    let Some(extended_cost_entry) = extended_cost_store.and_then(|store| store.get(extended_cost))
    else {
        return Vec::new();
    };
    let stacks = quantity / buy_count.max(1);
    extended_cost_entry
        .currency_id
        .iter()
        .copied()
        .zip(extended_cost_entry.currency_count.iter().copied())
        .filter(|(currency_id, _)| *currency_id != 0)
        .map(|(currency_id, count)| (u32::from(currency_id), count.wrapping_mul(stacks)))
        .collect()
}

fn item_extended_cost_currency_requires_season_earned(
    flags: ItemExtendedCostFlags,
    currency_index: usize,
) -> bool {
    match currency_index {
        0 => flags.contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_1),
        1 => flags.contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_2),
        2 => flags.contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_3),
        3 => flags.contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_4),
        4 => flags.contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_5),
        _ => false,
    }
}

fn vendor_buy_direct_store_block_result(
    bag: u8,
    slot: u8,
    _quantity: u32,
) -> Option<InventoryResult> {
    if (bag == NULL_BAG && slot == NULL_SLOT) || is_inventory_pos(bag, slot) {
        return None;
    }

    if is_equipment_pos(bag, slot) {
        return Some(InventoryResult::NotEquippable);
    }

    Some(InventoryResult::WrongSlot)
}

fn vendor_buy_stock_refill_count(
    current_count: u32,
    elapsed_secs: u64,
    incr_time: u32,
    buy_count: u32,
    max_count: u32,
) -> (u32, bool) {
    if max_count == 0 || current_count >= max_count || incr_time == 0 {
        // C++ assumes nonzero incrtime for finite stock; keep invalid DB rows from dividing by zero.
        return (current_count.min(max_count), current_count >= max_count);
    }

    let increments = elapsed_secs / u64::from(incr_time);
    if increments == 0 {
        return (current_count, false);
    }

    let restored = increments.saturating_mul(u64::from(buy_count.max(1)));
    let new_count = u64::from(current_count).saturating_add(restored);
    if new_count >= u64::from(max_count) {
        (max_count, true)
    } else {
        (new_count as u32, false)
    }
}

fn vendor_list_should_skip_sold_out(
    max_count: i32,
    current_count: u32,
    is_game_master: bool,
) -> bool {
    max_count > 0 && current_count == 0 && !is_game_master
}

fn vendor_list_item_refundable(
    item_flags: Option<ItemFlags>,
    max_stack_size: Option<u32>,
    extended_cost: i32,
) -> bool {
    extended_cost > 0
        && max_stack_size == Some(1)
        && item_flags.is_some_and(|flags| flags.contains(ItemFlags::ITEM_PURCHASE_RECORD))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadedItemRefundDecision {
    None,
    Valid {
        paid_money: u64,
        paid_extended_cost: u16,
    },
    Clear {
        new_flags: u32,
    },
}

fn loaded_item_refund_decision(
    item_flags: u32,
    played_time: u32,
    paid_money: Option<u64>,
    paid_extended_cost: Option<u16>,
) -> LoadedItemRefundDecision {
    let flags = ItemFieldFlags::from_bits_retain(item_flags);
    if !flags.contains(ItemFieldFlags::REFUNDABLE) {
        return LoadedItemRefundDecision::None;
    }

    let new_flags = (flags & !ItemFieldFlags::REFUNDABLE).bits();
    if played_time > 2 * 60 * 60 {
        return LoadedItemRefundDecision::Clear { new_flags };
    }

    match (paid_money, paid_extended_cost) {
        (Some(paid_money), Some(paid_extended_cost)) => LoadedItemRefundDecision::Valid {
            paid_money,
            paid_extended_cost,
        },
        _ => LoadedItemRefundDecision::Clear { new_flags },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DestroyItemCountAction {
    FullStack,
    PartialStack { new_count: u32 },
}

fn destroy_item_count_action(current_count: u32, requested_count: u32) -> DestroyItemCountAction {
    if requested_count != 0 && current_count > requested_count {
        return DestroyItemCountAction::PartialStack {
            new_count: current_count - requested_count,
        };
    }

    DestroyItemCountAction::FullStack
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SellItemAmountAction {
    Invalid,
    FullStack { amount: u32 },
    PartialStack { amount: u32, remaining: u32 },
}

fn sell_item_amount_action(current_count: u32, requested_amount: i32) -> SellItemAmountAction {
    let amount = if requested_amount == 0 {
        current_count
    } else {
        let Ok(amount) = u32::try_from(requested_amount) else {
            return SellItemAmountAction::Invalid;
        };
        amount
    };

    if amount == 0 || amount > current_count {
        return SellItemAmountAction::Invalid;
    }

    if amount < current_count {
        SellItemAmountAction::PartialStack {
            amount,
            remaining: current_count - amount,
        }
    } else {
        SellItemAmountAction::FullStack { amount }
    }
}

fn item_spell_charges_db_string(charges: &[i32]) -> String {
    let mut out = String::new();
    for charge in charges {
        out.push_str(&charge.to_string());
        out.push(' ');
    }
    out
}

fn item_is_currently_looted_like_cpp(item: &wow_entities::Item) -> bool {
    item.loot_generated()
}

fn item_is_not_empty_bag_like_cpp(
    inventory_type: Option<InventoryType>,
    contains_items: bool,
) -> bool {
    matches!(inventory_type, Some(InventoryType::Bag)) && contains_items
}

fn append_item_refund_clear_statements(
    char_db: &CharacterDatabase,
    tx: &mut SqlTransaction,
    item_db_guid: u64,
    new_flags: u32,
) {
    let mut del_refund = char_db.prepare(CharStatements::DEL_ITEM_REFUND_INSTANCE);
    del_refund.set_u64(0, item_db_guid);
    tx.append(del_refund);

    let mut upd_flags = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_FLAGS);
    upd_flags.set_u32(0, new_flags);
    upd_flags.set_u64(1, item_db_guid);
    tx.append(upd_flags);
}

fn append_item_refund_insert_statements(
    char_db: &CharacterDatabase,
    tx: &mut SqlTransaction,
    item_db_guid: u64,
    player_db_guid: u64,
    paid_money: u64,
    paid_extended_cost: u16,
) {
    let mut del_refund = char_db.prepare(CharStatements::DEL_ITEM_REFUND_INSTANCE);
    del_refund.set_u64(0, item_db_guid);
    tx.append(del_refund);

    let mut ins_refund = char_db.prepare(CharStatements::INS_ITEM_REFUND_INSTANCE);
    ins_refund.set_u64(0, item_db_guid);
    ins_refund.set_u64(1, player_db_guid);
    ins_refund.set_u64(2, paid_money);
    ins_refund.set_u16(3, paid_extended_cost);
    tx.append(ins_refund);
}

fn player_class_mask(player_class: u8) -> u32 {
    player_class
        .checked_sub(1)
        .and_then(|shift| 1u32.checked_shl(u32::from(shift)))
        .unwrap_or(0)
}

fn vendor_list_should_skip_allowed_class(
    allowable_class: Option<i16>,
    bonding: Option<u8>,
    player_class: u8,
    is_game_master: bool,
) -> bool {
    if is_game_master || bonding != Some(ItemBondingType::OnAcquire as u8) {
        return false;
    }

    let Some(allowable_class) = allowable_class else {
        return false;
    };
    (i32::from(allowable_class) & player_class_mask(player_class) as i32) == 0
}

fn player_team_for_race_cpp(race: u8) -> Team {
    match race {
        // C++ resolves this from ChrRacesEntry::Alliance: 1 = Horde, 0 = Alliance.
        2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 31 | 35 | 36 | 70 => Team::Horde,
        _ => Team::Alliance,
    }
}

fn vendor_list_should_skip_faction_flags(
    flags2: Option<u32>,
    team: Team,
    is_game_master: bool,
) -> bool {
    if is_game_master {
        return false;
    }

    let Some(flags2) = flags2 else {
        return false;
    };
    ((flags2 & ItemFlags2::FactionHorde as u32) != 0 && team == Team::Alliance)
        || ((flags2 & ItemFlags2::FactionAlliance as u32) != 0 && team == Team::Horde)
}

fn vendor_buy_template_block_result(
    allowable_class: Option<i16>,
    bonding: Option<u8>,
    flags2: Option<u32>,
    player_class: u8,
    player_race: u8,
    is_game_master: bool,
) -> Option<VendorBuyTemplateBlock> {
    if vendor_list_should_skip_allowed_class(allowable_class, bonding, player_class, is_game_master)
    {
        return Some(VendorBuyTemplateBlock::BuyError(BuyResult::CantFindItem));
    }

    if vendor_list_should_skip_faction_flags(
        flags2,
        player_team_for_race_cpp(player_race),
        is_game_master,
    ) {
        return Some(VendorBuyTemplateBlock::Silent);
    }

    None
}

fn vendor_buy_direct_inventory_destination(
    player_guid: ObjectGuid,
    buy: &BuyItem,
) -> Option<(u8, u8)> {
    let slot = buy.slot as u8;
    if slot as usize > MAX_BAG_SIZE && slot != NULL_SLOT {
        return None;
    }

    let bag = if buy.container_guid == player_guid {
        INVENTORY_SLOT_BAG_0
    } else {
        NULL_BAG
    };

    Some((bag, slot))
}

// ── Handler implementations ─────────────────────────────────────────

fn is_represented_bag_slot(slot: u8) -> bool {
    (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot)
        || (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&slot)
        || (REAGENT_BAG_SLOT_START..REAGENT_BAG_SLOT_END).contains(&slot)
}

impl WorldSession {
    fn vendor_stock_now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    }

    fn vendor_item_current_count(
        &mut self,
        vendor_guid: ObjectGuid,
        item_id: u32,
        max_count: u32,
        incr_time: u32,
        buy_count: u32,
    ) -> u32 {
        if max_count == 0 {
            return 0;
        }

        let key = (vendor_guid, item_id);
        let now = Self::vendor_stock_now_secs();
        let Some(count) = self.vendor_item_counts.get(&key).copied() else {
            return max_count;
        };

        let elapsed = now.saturating_sub(count.last_increment_time);
        let (new_count, full) =
            vendor_buy_stock_refill_count(count.count, elapsed, incr_time, buy_count, max_count);
        if full {
            self.vendor_item_counts.remove(&key);
            max_count
        } else {
            if let Some(count) = self.vendor_item_counts.get_mut(&key) {
                count.count = new_count;
                if incr_time > 0 && elapsed >= u64::from(incr_time) {
                    count.last_increment_time = now;
                }
                count.count
            } else {
                new_count
            }
        }
    }

    fn update_vendor_item_current_count(
        &mut self,
        vendor_guid: ObjectGuid,
        item_id: u32,
        max_count: u32,
        incr_time: u32,
        buy_count: u32,
        used_count: u32,
    ) -> u32 {
        if max_count == 0 {
            return 0;
        }

        let current =
            self.vendor_item_current_count(vendor_guid, item_id, max_count, incr_time, buy_count);
        let new_count = current.saturating_sub(used_count);
        self.vendor_item_counts.insert(
            (vendor_guid, item_id),
            crate::session::VendorItemCount {
                count: new_count,
                last_increment_time: Self::vendor_stock_now_secs(),
            },
        );
        new_count
    }

    async fn resolve_vendor_buy_item_by_cpp_slot(
        &self,
        world_db: &WorldDatabase,
        root_entry: u32,
        vendor_slot: u32,
        expected_item_id: u32,
    ) -> Option<VendorBuyItem> {
        let mut raw_slot = 0u32;
        let mut expanded = std::collections::HashSet::<u32>::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(root_entry);

        while let Some(vendor_entry) = queue.pop_front() {
            if !expanded.insert(vendor_entry) {
                continue;
            }

            let mut stmt = world_db.prepare(WorldStatements::SEL_VENDOR_ITEMS);
            stmt.set_u32(0, root_entry);
            stmt.set_u32(1, vendor_entry);
            let mut result = match world_db.query(&stmt).await {
                Ok(result) => result,
                Err(e) => {
                    warn!("BuyItem: vendor item query failed for entry {vendor_entry}: {e}");
                    continue;
                }
            };

            loop {
                let item_id: i32 = result.try_read(0).unwrap_or(0);
                if item_id > 0 {
                    let current_slot = raw_slot;
                    raw_slot = raw_slot.saturating_add(1);
                    let item_type = result
                        .try_read::<u8>(3)
                        .unwrap_or(ItemVendorType::Item as u8)
                        as i32;
                    let item_known = self
                        .item_store()
                        .map_or(true, |store| store.get(item_id as u32).is_some());
                    let currency_known = item_type == ItemVendorType::Currency as i32
                        && vendor_currency_type_is_known(
                            self.currency_types_store().map(|store| store.as_ref()),
                            item_id as u32,
                        );
                    if (item_known || currency_known) && current_slot == vendor_slot {
                        let row_item_id = item_id as u32;
                        if row_item_id != expected_item_id {
                            return None;
                        }

                        return Some(VendorBuyItem {
                            item_id: row_item_id,
                            item_type,
                            max_count: result.try_read::<u32>(1).unwrap_or(0),
                            incr_time: result.try_read::<u32>(10).unwrap_or(0),
                            player_condition_id: result.try_read::<u32>(11).unwrap_or(0),
                            has_vendor_conditions: result
                                .try_read::<u8>(12)
                                .map(|value| value != 0)
                                .unwrap_or(false),
                            extended_cost: result.try_read::<u32>(2).unwrap_or(0),
                            buy_price: result
                                .try_read::<i64>(5)
                                .map(|v| v as u64)
                                .or_else(|| result.try_read::<u64>(5))
                                .unwrap_or(0),
                            max_durability: result.try_read::<u32>(7).unwrap_or(0),
                            buy_count: result.try_read::<u32>(8).unwrap_or(1),
                        });
                    }
                } else if item_id < 0 {
                    queue.push_back((-item_id) as u32);
                }

                if !result.next_row() {
                    break;
                }
            }
        }

        None
    }

    /// Handle CMSG_ENUM_CHARACTERS — list characters for this account.
    pub async fn handle_enum_characters(&mut self) {
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => {
                warn!("No character database for account {}", self.account_id);
                self.send_packet(&EnumCharactersResult {
                    success: false,
                    characters: vec![],
                    race_unlock_data: vec![],
                });
                return;
            }
        };

        let mut stmt = char_db.prepare(CharStatements::SEL_ENUM);
        stmt.set_u32(0, self.account_id);

        let result = match char_db.query(&stmt).await {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    "Failed to query characters for account {}: {e}",
                    self.account_id
                );
                self.send_packet(&EnumCharactersResult {
                    success: false,
                    characters: vec![],
                    race_unlock_data: vec![],
                });
                return;
            }
        };

        let mut characters = Vec::new();
        let mut legit_guids = Vec::new();

        if !result.is_empty() {
            let mut result = result;
            loop {
                let guid_low: u64 = result.read(0); // bigint(20) unsigned
                let name: String = result.read_string(1);
                let race: u8 = result.read(2);
                let class: u8 = result.read(3);
                let gender: u8 = result.read(4);
                let level: u8 = result.read(5);
                let zone: i32 = result.try_read::<u16>(6).unwrap_or(0) as i32; // smallint unsigned
                let map: i32 = result.try_read::<u16>(7).unwrap_or(0) as i32; // smallint unsigned
                let pos_x: f32 = result.try_read(8).unwrap_or(0.0);
                let pos_y: f32 = result.try_read(9).unwrap_or(0.0);
                let pos_z: f32 = result.try_read(10).unwrap_or(0.0);
                let _guild_id: u64 = result.try_read(11).unwrap_or(0); // bigint unsigned via IFNULL
                let player_flags: u32 = result.try_read(12).unwrap_or(0);
                let at_login_flags: u16 = result.try_read(13).unwrap_or(0); // smallint unsigned
                let equipment_cache: String = result.try_read(14).unwrap_or_default();
                let last_login_build: u32 = result.try_read(15).unwrap_or(54261);

                let realm_id = self.realm_id();
                let guid = ObjectGuid::create_player(realm_id, guid_low as i64);

                // ── Convert PlayerFlags → CharacterFlags (matching C# exactly) ──
                // C# does NOT pass raw playerFlags as CharacterFlags.
                // Only specific bits are mapped:
                let mut char_flags: u32 = 0;
                // PlayerFlags::Resting (0x20) → CharacterFlags::Resting (0x02)
                if (player_flags & 0x20) != 0 {
                    char_flags |= 0x02;
                }
                // PlayerFlags::Ghost (0x10) → CharacterFlags::Ghost (0x2000)
                // But suppress if AtLoginFlags::Resurrect (0x100) is set
                if (player_flags & 0x10) != 0 && (at_login_flags & 0x100) == 0 {
                    char_flags |= 0x2000;
                }
                // AtLoginFlags::Rename (0x01) → CharacterFlags::Rename (0x4000)
                if (at_login_flags & 0x01) != 0 {
                    char_flags |= 0x4000;
                }

                // ── CharacterCustomizeFlags (Flags2) from AtLoginFlags ──
                let char_flags2: u32 = if (at_login_flags & 0x08) != 0 {
                    1 // CharacterCustomizeFlags::Customize
                } else if (at_login_flags & 0x40) != 0 {
                    2 // CharacterCustomizeFlags::Faction
                } else if (at_login_flags & 0x80) != 0 {
                    4 // CharacterCustomizeFlags::Race
                } else {
                    0
                };

                // Only add to legit list if not locked
                // CharacterFlags::CharacterLockedForTransfer (0x04) |
                // CharacterFlags::LockedByBilling (0x01000000)
                if (char_flags & (0x04 | 0x0100_0000)) == 0 {
                    legit_guids.push(guid);
                }

                let char_info = CharacterInfo {
                    guid,
                    guild_club_member_id: 0,
                    name,
                    list_position: characters.len() as u8,
                    race_id: race,
                    class_id: class,
                    sex_id: gender,
                    experience_level: level,
                    zone_id: zone,
                    map_id: map,
                    position: Position::new(pos_x, pos_y, pos_z, 0.0),
                    guild_guid: ObjectGuid::EMPTY,
                    flags: char_flags,
                    flags2: char_flags2,
                    flags3: 0,
                    flags4: 0,
                    first_login: (at_login_flags & 0x20) != 0, // AT_LOGIN_FIRST
                    pet_display_id: 0,
                    pet_level: 0,
                    pet_family: 0,
                    profession_ids: [0; 2],
                    equipment: parse_equipment_cache(&equipment_cache),
                    last_played_time: 0,
                    spec_id: 0,
                    last_login_version: last_login_build as i32,
                    override_select_screen_file_data_id: 0,
                };

                characters.push(char_info);

                if !result.next_row() {
                    break;
                }
            }
        }

        self.set_legit_characters(legit_guids);

        debug!(
            "Sending {} characters to account {}",
            characters.len(),
            self.account_id
        );

        // Build RaceUnlockData — from race_unlock_requirement table.
        // All WotLK races: expansion 0 (Classic) or 1 (TBC).
        // HasExpansion = true if account expansion >= required expansion.
        let account_exp = self.account_expansion;
        let race_unlock_data: Vec<RaceUnlock> = [
            (1u8, 0u8), // Human — Classic
            (2, 0),     // Orc
            (3, 0),     // Dwarf
            (4, 0),     // Night Elf
            (5, 0),     // Undead
            (6, 0),     // Tauren
            (7, 0),     // Gnome
            (8, 0),     // Troll
            (10, 1),    // Blood Elf — TBC
            (11, 1),    // Draenei — TBC
        ]
        .iter()
        .map(|&(race_id, required_exp)| RaceUnlock {
            race_id,
            has_expansion: account_exp >= required_exp,
            has_achievement: false,
            has_heritage_armor: false,
            is_locked: false,
        })
        .collect();

        self.send_packet(&EnumCharactersResult {
            success: true,
            characters,
            race_unlock_data,
        });
    }

    /// Handle CMSG_CREATE_CHARACTER — create a new character.
    pub async fn handle_create_character(&mut self, pkt: CreateCharacter) {
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => {
                self.send_packet(&CreateChar {
                    code: response_codes::CHAR_CREATE_ERROR,
                    guid: ObjectGuid::EMPTY,
                });
                return;
            }
        };

        // Validate name length
        if pkt.name.len() < 2 || pkt.name.len() > 12 {
            self.send_packet(&CreateChar {
                code: response_codes::CHAR_CREATE_ERROR,
                guid: ObjectGuid::EMPTY,
            });
            return;
        }

        // Validate name characters (alphanumeric only)
        if !pkt.name.chars().all(|c| c.is_ascii_alphabetic()) {
            self.send_packet(&CreateChar {
                code: response_codes::CHAR_CREATE_ERROR,
                guid: ObjectGuid::EMPTY,
            });
            return;
        }

        // Check name uniqueness
        let mut name_stmt = char_db.prepare(CharStatements::SEL_CHECK_NAME);
        name_stmt.set_string(0, &pkt.name);

        if let Ok(result) = char_db.query(&name_stmt).await {
            if !result.is_empty() {
                self.send_packet(&CreateChar {
                    code: response_codes::CHAR_CREATE_NAME_IN_USE,
                    guid: ObjectGuid::EMPTY,
                });
                return;
            }
        }

        // Check account character limit
        let mut count_stmt = char_db.prepare(CharStatements::SEL_SUM_CHARS);
        count_stmt.set_u32(0, self.account_id);

        if let Ok(result) = char_db.query(&count_stmt).await {
            if !result.is_empty() {
                let count: i64 = result.try_read(0).unwrap_or(0);
                if count >= MAX_CHARACTERS_PER_ACCOUNT as i64 {
                    self.send_packet(&CreateChar {
                        code: response_codes::CHAR_CREATE_ACCOUNT_LIMIT,
                        guid: ObjectGuid::EMPTY,
                    });
                    return;
                }
            }
        }

        // Generate new GUID
        let new_guid_counter = match self.guid_generator() {
            Some(generator) => generator.generate(),
            None => {
                warn!("No GUID generator available");
                self.send_packet(&CreateChar {
                    code: response_codes::CHAR_CREATE_ERROR,
                    guid: ObjectGuid::EMPTY,
                });
                return;
            }
        };

        // Get start position
        let (map_id, x, y, z, o) = start_position(pkt.race);
        let zone = start_zone(pkt.race);
        let sex = if pkt.sex < 0 { 0u8 } else { pkt.sex as u8 };

        // Default health/power for a fresh level 1 character
        let (health, mana) = default_health_mana(pkt.class);

        let create_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Insert character — columns match the real 3.4.3 characters table
        let mut ins_stmt = char_db.prepare(CharStatements::INS_CHARACTER);
        ins_stmt.set_u64(0, new_guid_counter as u64); // guid (bigint unsigned)
        ins_stmt.set_u32(1, self.account_id); // account
        ins_stmt.set_string(2, &pkt.name); // name
        ins_stmt.set_u8(3, pkt.race); // race
        ins_stmt.set_u8(4, pkt.class); // class
        ins_stmt.set_u8(5, sex); // gender
        ins_stmt.set_u8(6, 1); // level
        ins_stmt.set_u64(7, 0); // money (bigint unsigned)
        ins_stmt.set_i32(8, zone); // zone (smallint unsigned)
        ins_stmt.set_i32(9, map_id); // map (smallint unsigned)
        ins_stmt.set_f32(10, x); // position_x
        ins_stmt.set_f32(11, y); // position_y
        ins_stmt.set_f32(12, z); // position_z
        ins_stmt.set_f32(13, o); // orientation
        ins_stmt.set_string(14, ""); // taximask (text NOT NULL)
        ins_stmt.set_i64(15, create_time); // createTime (bigint)
        ins_stmt.set_u8(16, 0); // createMode
        ins_stmt.set_u32(17, 0); // playerFlags
        ins_stmt.set_u32(18, 0x20); // at_login (AT_LOGIN_FIRST)
        ins_stmt.set_u32(19, health); // health
        ins_stmt.set_u32(20, mana); // power1 (mana)
        ins_stmt.set_u32(21, self.build); // lastLoginBuild

        match char_db.execute(&ins_stmt).await {
            Ok(_) => {
                // Insert customizations into character_customizations table
                for c in &pkt.customizations {
                    let mut cust_stmt = char_db.prepare(CharStatements::INS_CHAR_CUSTOMIZATION);
                    cust_stmt.set_u64(0, new_guid_counter as u64);
                    cust_stmt.set_i32(1, c.option_id);
                    cust_stmt.set_i32(2, c.choice_id);
                    if let Err(e) = char_db.execute(&cust_stmt).await {
                        warn!("Failed to insert customization for guid {new_guid_counter}: {e}");
                    }
                }

                let guid = ObjectGuid::create_player(self.realm_id(), new_guid_counter);
                info!(
                    "Character '{}' created (guid={}, {} customizations) for account {}",
                    pkt.name,
                    new_guid_counter,
                    pkt.customizations.len(),
                    self.account_id
                );

                // Insert initial action buttons from playercreateinfo_action
                if let Some(world_db) = self.world_db().map(Arc::clone) {
                    let action_stmt =
                        world_db.prepare(WorldStatements::SEL_PLAYER_CREATEINFO_ACTION);
                    if let Ok(mut action_result) = world_db.query(&action_stmt).await {
                        let mut action_count = 0u32;
                        loop {
                            let a_race: u8 = action_result.read(0);
                            let a_class: u8 = action_result.read(1);
                            if a_race == pkt.race && a_class == pkt.class {
                                let button: u8 = action_result.read(2);
                                let action: i32 = action_result.try_read(3).unwrap_or(0);
                                let btn_type: u8 = action_result.try_read(4).unwrap_or(0);
                                if action > 0 {
                                    let mut ins =
                                        char_db.prepare(CharStatements::INS_CHARACTER_ACTION);
                                    ins.set_u64(0, new_guid_counter as u64);
                                    ins.set_u8(1, button);
                                    ins.set_i32(2, action);
                                    ins.set_u8(3, btn_type);
                                    if let Err(e) = char_db.execute(&ins).await {
                                        warn!("Failed to insert action button {button}: {e}");
                                    } else {
                                        action_count += 1;
                                    }
                                }
                            }
                            if !action_result.next_row() {
                                break;
                            }
                        }
                        if action_count > 0 {
                            info!(
                                "Inserted {action_count} initial action buttons for '{}'",
                                pkt.name
                            );
                        }
                    }
                }

                // Update realmcharacters count in login DB
                self.update_realm_characters(&char_db).await;

                self.send_packet(&CreateChar {
                    code: response_codes::CHAR_CREATE_SUCCESS,
                    guid,
                });
            }
            Err(e) => {
                warn!("Failed to create character: {e}");
                self.send_packet(&CreateChar {
                    code: response_codes::CHAR_CREATE_ERROR,
                    guid: ObjectGuid::EMPTY,
                });
            }
        }
    }

    /// Handle CMSG_CHAR_DELETE — delete a character.
    pub async fn handle_char_delete(&mut self, pkt: CharDelete) {
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => {
                self.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_FAILED,
                });
                return;
            }
        };

        // Verify the character belongs to this account
        if !self.is_legit_character(&pkt.guid) {
            warn!(
                "Account {} tried to delete non-owned character {:?}",
                self.account_id, pkt.guid
            );
            self.send_packet(&DeleteChar {
                code: response_codes::CHAR_DELETE_FAILED,
            });
            return;
        }

        // Double-check in DB
        let mut check_stmt = char_db.prepare(CharStatements::SEL_CHAR_DEL_CHECK);
        check_stmt.set_u32(0, pkt.guid.counter() as u32);
        check_stmt.set_u32(1, self.account_id);

        if let Ok(result) = char_db.query(&check_stmt).await {
            if result.is_empty() {
                self.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_FAILED,
                });
                return;
            }
        }

        // Delete
        let mut del_stmt = char_db.prepare(CharStatements::DEL_CHARACTER);
        del_stmt.set_u32(0, pkt.guid.counter() as u32);

        match char_db.execute(&del_stmt).await {
            Ok(_) => {
                info!(
                    "Character {:?} deleted for account {}",
                    pkt.guid, self.account_id
                );
                self.remove_legit_character(&pkt.guid);

                // Update realmcharacters count in login DB
                self.update_realm_characters(&char_db).await;

                self.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_SUCCESS,
                });
            }
            Err(e) => {
                warn!("Failed to delete character: {e}");
                self.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_FAILED,
                });
            }
        }
    }

    /// Handle CMSG_PLAYER_LOGIN — initiate ConnectTo flow.
    ///
    /// Instead of sending the login sequence directly, we send SMSG_CONNECT_TO
    /// to redirect the client to the instance port. The login sequence is sent
    /// after the client reconnects via `handle_continue_player_login`.
    pub async fn handle_player_login(&mut self, pkt: PlayerLogin) {
        // Verify character ownership
        if !self.is_legit_character(&pkt.guid) {
            warn!(
                "Account {} tried to login with non-owned character {:?}",
                self.account_id, pkt.guid
            );
            return;
        }

        // Store the loading character GUID
        self.set_player_loading(Some(pkt.guid));

        // Build ConnectTo and register with SessionManager
        self.send_connect_to(ConnectToSerial::WorldAttempt1);
    }

    /// Build and send SMSG_CONNECT_TO to the client.
    fn send_connect_to(&mut self, serial: ConnectToSerial) {
        let session_mgr = match self.session_mgr() {
            Some(mgr) => Arc::clone(mgr),
            None => {
                warn!(
                    "No session manager for ConnectTo flow (account {}), sending login directly",
                    self.account_id
                );
                self.fallback_direct_login();
                return;
            }
        };

        // Generate ConnectToKey
        let key = ConnectToKey {
            account_id: self.account_id,
            connection_type: 1, // Instance
            key: rand::thread_rng().gen_range(0..0x7FFF_FFFF_u32),
        };
        let key_raw = key.raw();
        self.set_connect_to_key(Some(key_raw));
        self.set_connect_to_serial(Some(serial));

        // Register in SessionManager — returns oneshot receiver for instance link
        let rx = session_mgr.register(self.account_id, key_raw, self.session_key.clone());
        self.set_instance_link_rx(Some(rx));

        // Build the ConnectTo payload
        let addr = self.instance_address();
        let port = self.instance_port();

        // Build where_buffer for RSA signature: [type(1B)][ip(4B)]
        let mut where_buffer = Vec::with_capacity(5);
        where_buffer.push(1u8); // IPv4
        where_buffer.extend_from_slice(&addr);

        let signature = rsa_sign_connect_to(&where_buffer, 1, port);

        let connect_to = ConnectTo {
            signature,
            address: ConnectToAddress::IPv4(addr),
            port,
            serial,
            con: 1, // Instance
            key: key_raw,
        };

        info!(
            "Sending ConnectTo (serial={:?}) to account {} for instance {}:{port}",
            serial,
            self.account_id,
            format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
        );

        self.send_packet(&connect_to);
    }

    /// Handle CMSG_SERVER_TIME_OFFSET_REQUEST — respond with current realm time.
    pub async fn handle_server_time_offset_request(&mut self) {
        self.send_packet(&ServerTimeOffset::now());
    }

    /// Handle CMSG_REQUEST_PLAYED_TIME (0x327A).
    ///
    /// C# ref: `MiscHandler.HandlePlayedTime`.
    /// Client sends this when the player types `/played`.
    /// We respond with total and level played time in seconds.
    /// `trigger_event` mirrors the client flag (TriggerScriptEvent).
    pub async fn handle_request_played_time(&mut self, trigger_event: bool) {
        use wow_packet::packets::misc::PlayedTime;

        // Session time elapsed since login (seconds).
        let session_secs: u32 = self
            .login_time
            .map(|t| t.elapsed().as_secs() as u32)
            .unwrap_or(0);

        // Add session time on top of DB-loaded base values.
        let total_time = self.total_played_time.saturating_add(session_secs);
        let level_time = self.level_played_time.saturating_add(session_secs);

        self.send_packet(&PlayedTime {
            total_time,
            level_time,
            trigger_event,
        });
    }

    /// Handle CMSG_GET_UNDELETE_CHARACTER_COOLDOWN_STATUS.
    ///
    /// The client sends this when it wants to know if character undelete is
    /// available. We always respond with "no cooldown" (undelete available).
    pub async fn handle_get_undelete_cooldown_status(&mut self) {
        self.send_packet(&wow_packet::packets::misc::UndeleteCooldownStatusResponse::no_cooldown());
    }

    /// Handle CMSG_DB_QUERY_BULK — client requests DB2 records.
    ///
    /// DB2 records are served from the startup hotfix blob cache, which is
    /// populated from local DB2 files plus the C++ `hotfixes.hotfix_blob` table.
    pub async fn handle_db_query_bulk(&mut self, query: wow_packet::packets::misc::DbQueryBulk) {
        info!(
            "DbQueryBulk: table=0x{:08X}, {} records {:?} for account {}",
            query.table_hash,
            query.queries.len(),
            query.queries,
            self.account_id
        );
        // Status 1 = Valid (send blob), Status 3 = Invalid (client uses its own DB2 cache).
        let cache = self.hotfix_blob_cache().map(Arc::clone);
        for record_id in &query.queries {
            if let Some(ref c) = cache {
                if let Some(blob) = c.get(query.table_hash, *record_id) {
                    let mut data = blob.to_vec();
                    if let Some(optional_entries) =
                        c.get_optional_data(query.table_hash, *record_id, &self.locale)
                    {
                        for optional_data in optional_entries {
                            data.extend_from_slice(&optional_data.key.to_le_bytes());
                            data.extend_from_slice(&optional_data.data);
                        }
                    }
                    info!(
                        "DbQueryBulk: FOUND blob table=0x{:08X} record={} ({} bytes)",
                        query.table_hash,
                        record_id,
                        data.len()
                    );
                    self.send_packet(&DBReply::found(query.table_hash, *record_id, data));
                    continue;
                }
            }

            // Not found anywhere → send Invalid(3) so the client uses its local DB2 copy.
            // RecordRemoved(2) would tell the client to DELETE the record from its cache,
            // which is wrong for items that exist in the client's DB2 but not on the server.
            if query.table_hash == TACT_KEY_TABLE_HASH_LIKE_CPP {
                debug!(
                    "DbQueryBulk: NOT_FOUND TactKey.db2 record={} → Invalid(3), client may use local DB2 cache",
                    record_id
                );
            } else {
                info!(
                    "DbQueryBulk: NOT_FOUND table=0x{:08X} record={} → Invalid(3)",
                    query.table_hash, record_id
                );
            }
            self.send_packet(&DBReply::not_found(query.table_hash, *record_id));
        }
    }

    /// Handle CMSG_HOTFIX_REQUEST — client requests hotfix data.
    pub async fn handle_hotfix_request(&mut self, req: wow_packet::packets::misc::HotfixRequest) {
        debug!(
            "HotfixRequest: client_build={}, data_build={}, {} hotfixes for account {}",
            req.client_build,
            req.data_build,
            req.hotfixes.len(),
            self.account_id
        );

        let Some(cache) = self.hotfix_blob_cache().map(Arc::clone) else {
            self.send_packet(&HotfixConnect::empty());
            return;
        };

        let mut response = HotfixConnect::empty();
        let locale_mask = hotfix_locale_mask(&self.locale);
        for push_id in &req.hotfixes {
            let Some(push) = cache.hotfix_push(*push_id) else {
                continue;
            };

            for record in &push.records {
                if record.available_locales_mask & locale_mask == 0 {
                    continue;
                }

                let mut status = record.status as u8;
                let mut size = 0u32;

                if record.status == HotfixRecordStatus::Valid {
                    if let Some(blob) = cache.get(record.table_hash, record.record_id) {
                        let start = response.content.len();
                        response.content.extend_from_slice(blob);
                        if let Some(optional_entries) = cache.get_optional_data(
                            record.table_hash,
                            record.record_id,
                            &self.locale,
                        ) {
                            for optional_data in optional_entries {
                                response
                                    .content
                                    .extend_from_slice(&optional_data.key.to_le_bytes());
                                response.content.extend_from_slice(&optional_data.data);
                            }
                        }
                        size = (response.content.len() - start) as u32;
                    } else {
                        status = if cache.has_table(record.table_hash) {
                            HotfixRecordStatus::RecordRemoved as u8
                        } else {
                            HotfixRecordStatus::Invalid as u8
                        };
                    }
                }

                response.hotfixes.push(HotfixConnectData {
                    id: HotfixId {
                        push_id: record.id.push_id,
                        unique_id: record.id.unique_id,
                    },
                    table_hash: record.table_hash,
                    record_id: record.record_id,
                    size,
                    status,
                });
            }
        }

        self.send_packet(&response);
    }

    /// Handle CMSG_TIME_SYNC_RESPONSE — client's response to our TimeSyncRequest.
    ///
    /// We acknowledge the response to keep the client's time sync state healthy.
    /// The periodic timer in `update()` handles sending the next request.
    pub async fn handle_time_sync_response(
        &mut self,
        resp: wow_packet::packets::misc::TimeSyncResponse,
    ) {
        trace!(
            "TimeSyncResponse: seq={}, client_time={} for account {}",
            resp.sequence_index, resp.client_time, self.account_id
        );
        self.record_time_sync_response_like_cpp(resp.sequence_index, resp.client_time);
    }

    /// Handle CMSG_LOGOUT_REQUEST — player wants to log out.
    ///
    /// C# logic: if player is in combat or in a duel, deny logout.
    /// Otherwise, if in a resting zone or GM, instant logout.
    /// Else, 20-second countdown.
    ///
    /// For now we always allow instant logout (simplified).
    pub async fn handle_logout_request(&mut self, req: LogoutRequest) {
        info!(
            "LogoutRequest (idle={}) from account {}",
            req.idle_logout, self.account_id
        );

        if !self.active_loot_guid.is_empty() {
            self.send_packet(&LootReleaseAll);
        }

        self.set_player_logout_like_cpp(true);

        // Always allow instant logout for now (no combat/duel checks)
        self.send_packet(&LogoutResponse::instant_ok());

        // Complete logout immediately
        self.logout_time = None;

        // Trinity clears buyback slots before SaveToDB; persisted buyback items must not survive logout.
        self.clear_buyback_on_logout().await;
        self.save_current_player_to_db_like_cpp().await;
        self.save_account_mounts_like_cpp().await;
        self.save_account_toys_like_cpp().await;
        self.save_account_heirlooms_like_cpp().await;
        self.save_account_item_appearances_like_cpp().await;

        if let Some(player_guid) = self.player_guid() {
            self.close_active_loot_windows_like_cpp(player_guid);
        }

        // Mark character offline in DB
        self.mark_character_offline().await;

        // Notify other players that this player has left before removing from registry.
        self.broadcast_destroy_player_to_others();
        // Remove from broadcast registry before clearing player_guid.
        self.unregister_from_player_registry();
        self.unregister_from_object_accessor();

        // Send LogoutComplete → client returns to character select
        self.set_state(crate::session::SessionState::Authed);
        self.send_packet(&LogoutComplete);
        self.set_player_guid(None);

        // Clear inventory state
        self.clear_all_inventory_runtime_like_cpp();
        self.clear_player_currencies_like_cpp();
        self.set_active_loot_guid(ObjectGuid::EMPTY);

        // ── Restore realm socket as primary ──────────────────────────
        // After ConnectTo, send_tx/packet_rx point to the instance socket.
        // On logout the client returns to character select on the REALM
        // connection. If we don't swap back, the next PlayerLogin sends
        // ConnectTo on the dead instance socket → client stuck at 90%.
        self.restore_realm_channels();
        self.set_player_logout_like_cpp(false);

        info!("Player logged out for account {}", self.account_id);
    }

    /// Handle CMSG_LOGOUT_CANCEL — player cancels a pending logout.
    pub async fn handle_logout_cancel(&mut self) {
        info!("LogoutCancel from account {}", self.account_id);
        self.logout_time = None;
        self.send_packet(&LogoutCancelAck);
    }

    /// Save accumulated played time (`totaltime` + `leveltime`) back to the
    /// characters database.  Called on logout so time is not lost.
    pub(crate) async fn save_played_time(&self) {
        let guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        // Compute current total values: base (from DB at login) + session elapsed.
        let session_secs: u32 = self
            .login_time
            .map(|t| t.elapsed().as_secs() as u32)
            .unwrap_or(0);
        let total_time = self.total_played_time.saturating_add(session_secs);
        let level_time = self.level_played_time.saturating_add(session_secs);

        let mut stmt = char_db.prepare(CharStatements::UPD_CHAR_PLAYED_TIME);
        stmt.set_u32(0, total_time);
        stmt.set_u32(1, level_time);
        stmt.set_u32(2, guid.counter() as u32);
        if let Err(e) = char_db.execute(&stmt).await {
            warn!(
                "Failed to save played time for guid {}: {e}",
                guid.counter()
            );
        } else {
            info!(
                "Saved played time: total={}s level={}s for guid {}",
                total_time,
                level_time,
                guid.counter()
            );
        }
    }

    /// Mark the current character as offline in the database.
    async fn mark_character_offline(&self) {
        let guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let mut stmt = char_db.prepare(CharStatements::UPD_CHAR_OFFLINE);
        stmt.set_u32(0, guid.counter() as u32);
        if let Err(e) = char_db.execute(&stmt).await {
            warn!("Failed to mark character offline: {e}");
        }
    }

    async fn clear_buyback_on_logout(&mut self) {
        let guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };
        if self.buyback_items_like_cpp().is_empty() {
            self.clear_buyback_runtime_like_cpp();
            return;
        }

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let mut tx = SqlTransaction::new();
        for item in self.buyback_items_like_cpp().values() {
            let mut del_inv = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
            del_inv.set_u64(0, guid.counter() as u64);
            del_inv.set_u64(1, item.db_guid);
            tx.append(del_inv);

            let mut del_item = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
            del_item.set_u64(0, item.db_guid);
            tx.append(del_item);
        }

        if let Err(e) = char_db.commit_transaction(tx).await {
            warn!(
                "Failed to clear buyback items on logout for guid {}: {e}",
                guid.counter()
            );
            return;
        }

        let removed_guids: Vec<_> = self
            .buyback_items_like_cpp()
            .values()
            .map(|item| item.guid)
            .collect();
        for item_guid in removed_guids {
            self.remove_inventory_item_object(item_guid);
        }
        self.clear_buyback_runtime_like_cpp();
        self.sync_object_accessor_player();
    }

    async fn save_account_mounts_like_cpp(&self) {
        let Some(login_db) = self.login_db() else {
            return;
        };

        for mount in self.account_mount_rows_like_cpp() {
            let Ok(mount_spell_id) = u32::try_from(mount.spell_id) else {
                continue;
            };
            let mut stmt = login_db.prepare(LoginStatements::REP_ACCOUNT_MOUNTS);
            stmt.set_u32(0, self.battlenet_account_id());
            stmt.set_u32(1, mount_spell_id);
            stmt.set_u8(2, mount.flags);
            if let Err(error) = login_db.execute(&stmt).await {
                warn!(
                    account = self.account_id,
                    bnet_account = self.battlenet_account_id(),
                    mount_spell_id,
                    "Failed to save account mount flags: {error}"
                );
            }
        }
    }

    async fn save_account_toys_like_cpp(&self) {
        let Some(login_db) = self.login_db() else {
            return;
        };

        for (item_id, is_favorite, has_fanfare) in self.account_toy_rows_like_cpp() {
            let mut stmt = login_db.prepare(LoginStatements::REP_ACCOUNT_TOYS);
            stmt.set_u32(0, self.battlenet_account_id());
            stmt.set_u32(1, item_id);
            stmt.set_bool(2, is_favorite);
            stmt.set_bool(3, has_fanfare);
            if let Err(error) = login_db.execute(&stmt).await {
                warn!(
                    account = self.account_id,
                    bnet_account = self.battlenet_account_id(),
                    item_id,
                    "Failed to save account toy flags: {error}"
                );
            }
        }
    }

    async fn save_account_heirlooms_like_cpp(&self) {
        let Some(login_db) = self.login_db() else {
            return;
        };

        for (item_id, flags) in self.account_heirloom_rows_like_cpp() {
            let mut stmt = login_db.prepare(LoginStatements::REP_ACCOUNT_HEIRLOOMS);
            stmt.set_u32(0, self.battlenet_account_id());
            stmt.set_u32(1, item_id);
            stmt.set_u32(2, flags);
            if let Err(error) = login_db.execute(&stmt).await {
                warn!(
                    account = self.account_id,
                    bnet_account = self.battlenet_account_id(),
                    item_id,
                    "Failed to save account heirloom flags: {error}"
                );
            }
        }
    }

    async fn save_account_item_appearances_like_cpp(&mut self) {
        let Some(login_db) = self.login_db().map(Arc::clone) else {
            return;
        };
        let plan = self.account_item_appearance_save_plan_like_cpp();
        if plan.is_empty() {
            return;
        }

        let bnet_account_id = self.battlenet_account_id();
        let mut tx = SqlTransaction::new();
        for (block_index, appearance_mask) in plan.appearance_blocks {
            let mut stmt = login_db.prepare(LoginStatements::INS_BNET_ITEM_APPEARANCES);
            stmt.set_u32(0, bnet_account_id);
            stmt.set_u32(1, block_index);
            stmt.set_u32(2, appearance_mask);
            tx.append(stmt);
        }
        for item_modified_appearance_id in plan.favorite_inserts {
            let mut stmt = login_db.prepare(LoginStatements::INS_BNET_ITEM_FAVORITE_APPEARANCE);
            stmt.set_u32(0, bnet_account_id);
            stmt.set_u32(1, item_modified_appearance_id);
            tx.append(stmt);
        }
        for item_modified_appearance_id in plan.favorite_deletes {
            let mut stmt = login_db.prepare(LoginStatements::DEL_BNET_ITEM_FAVORITE_APPEARANCE);
            stmt.set_u32(0, bnet_account_id);
            stmt.set_u32(1, item_modified_appearance_id);
            tx.append(stmt);
        }

        if let Err(error) = login_db.commit_transaction(tx).await {
            warn!(
                account = self.account_id,
                bnet_account = bnet_account_id,
                "Failed to save account item appearances: {error}"
            );
        }
    }

    /// Handle ConnectToFailed — client couldn't connect to instance port.
    ///
    /// Retry with the next serial, or fall back to direct login if all retries
    /// are exhausted.
    pub async fn handle_connect_to_failed(&mut self, pkt: ConnectToFailed) {
        warn!(
            "ConnectToFailed (serial={:?}) from account {}",
            pkt.serial, self.account_id
        );

        // Clean up the pending entry from SessionManager
        if let Some(mgr) = self.session_mgr() {
            mgr.remove(self.account_id);
        }
        self.set_instance_link_rx(None);

        // Try next serial
        if let Some(next_serial) = pkt.serial.next() {
            info!("Retrying ConnectTo with serial {:?}", next_serial);
            self.send_connect_to(next_serial);
        } else {
            warn!(
                "All ConnectTo retries exhausted for account {}, falling back to direct login",
                self.account_id
            );
            self.fallback_direct_login();
        }
    }

    /// Continue the player login after the instance socket is connected.
    ///
    /// Called when the `instance_link_rx` oneshot delivers the new channels.
    /// Sends ResumeComms and the full login sequence on the instance socket.
    pub async fn handle_continue_player_login(&mut self) {
        let guid: ObjectGuid = match self.player_loading() {
            Some(g) => g,
            None => {
                warn!("handle_continue_player_login called but no player_loading set");
                return;
            }
        };
        self.set_player_loading(None);
        self.set_connect_to_key(None);
        self.set_connect_to_serial(None);

        // Send ResumeComms only when using ConnectTo flow (instance socket).
        // In direct login (no session_mgr), the client didn't go through ConnectTo
        // and doesn't expect ResumeComms — sending it causes a disconnect.
        if self.session_mgr().is_some() {
            self.send_packet(&ResumeComms);
        }

        // Load character from DB and send login sequence
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => {
                warn!("No character database for continue login");
                return;
            }
        };

        let mut stmt = char_db.prepare(CharStatements::SEL_CHARACTER);
        stmt.set_u32(0, guid.counter() as u32);

        let result = match char_db.query(&stmt).await {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to load character {:?}: {e}", guid);
                return;
            }
        };

        if result.is_empty() {
            warn!("Character {:?} not found in database", guid);
            return;
        }

        let name: String = result.read_string(2);
        // Store character name for chat messages.
        self.set_loaded_player_name_like_cpp(name.clone());
        let race: u8 = result.read(3);
        let class: u8 = result.read(4);
        let gender: u8 = result.read(5);
        let level: u8 = result.read(6);
        let zone: i32 = result.try_read::<u16>(7).unwrap_or(0) as i32; // smallint unsigned
        let map_id: i32 = result.try_read::<u16>(8).unwrap_or(0) as i32; // smallint unsigned
        let pos_x: f32 = result.try_read(9).unwrap_or(0.0);
        let pos_y: f32 = result.try_read(10).unwrap_or(0.0);
        let pos_z: f32 = result.try_read(11).unwrap_or(0.0);
        let orientation: f32 = result.try_read(12).unwrap_or(0.0);

        let position = Position::new(pos_x, pos_y, pos_z, orientation);
        let display_id = default_display_id(race, gender);

        // Load played time + money from DB.
        // Cols: 15=totaltime, 16=leveltime, 17=money (bigint unsigned).
        self.total_played_time = result.try_read::<u32>(15).unwrap_or(0);
        self.level_played_time = result.try_read::<u32>(16).unwrap_or(0);
        self.set_player_gold_like_cpp(result.try_read::<u64>(17).unwrap_or(0));
        self.set_player_xp_like_cpp(result.try_read::<u32>(18).unwrap_or(0));
        self.set_player_guid(Some(guid));
        self.set_loaded_player_identity_like_cpp(map_id as u16, race, class, level, gender);
        self.refresh_next_level_xp();
        if self.ensure_login_player_controller_like_cpp(
            guid,
            name.clone(),
            position,
            map_id as u16,
            race,
            class,
            level,
            gender,
        ) {
            let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
        }

        self.load_account_toys_like_cpp().await;
        self.load_account_heirlooms_like_cpp().await;
        self.load_account_item_appearances_like_cpp().await;
        let account_mounts = self.load_account_mounts_like_cpp().await;

        // Load equipped items for visible display + inventory objects
        let mut visible_items = [(0i32, 0u16, 0u16); 19];
        let mut inv_slots = [ObjectGuid::EMPTY; 141];
        let mut item_creates: Vec<wow_packet::packets::update::ItemCreateData> = Vec::new();
        let realm_id = self.realm_id();
        self.clear_inventory_items_and_objects_like_cpp();
        self.clear_player_currencies_like_cpp();
        {
            let mut eq_stmt = char_db.prepare(CharStatements::SEL_CHAR_EQUIPMENT);
            eq_stmt.set_u64(0, guid.counter() as u64);
            let mut refund_cleanup_tx = SqlTransaction::new();
            match char_db.query(&eq_stmt).await {
                Ok(mut eq_result) => {
                    if !eq_result.is_empty() {
                        loop {
                            let slot: u8 = eq_result.read(0);
                            let item_entry: u32 = eq_result.try_read(1).unwrap_or(0);
                            let item_db_guid: u64 = eq_result.try_read(2).unwrap_or(0);
                            let item_count: u32 = eq_result.try_read(3).unwrap_or(1);
                            let item_durability: u32 = eq_result.try_read(4).unwrap_or(0);
                            let item_context = eq_result
                                .try_read::<u8>(5)
                                .and_then(<ItemContext as num_traits::FromPrimitive>::from_u8)
                                .unwrap_or(ItemContext::None);
                            let item_flags = eq_result.try_read::<u32>(6).unwrap_or(0);
                            let item_played_time = eq_result.try_read::<u32>(7).unwrap_or(0);
                            let refund_decision = loaded_item_refund_decision(
                                item_flags,
                                item_played_time,
                                eq_result.try_read::<u64>(8),
                                eq_result.try_read::<u16>(9),
                            );
                            if item_entry > 0 && (slot as usize) < 141 {
                                let item_max_durability = self
                                    .item_template_max_durability(item_entry)
                                    .max(item_durability);
                                let item_guid =
                                    ObjectGuid::create_item(realm_id, item_db_guid as i64);
                                let stored_flags = match refund_decision {
                                    LoadedItemRefundDecision::Clear { new_flags } => {
                                        append_item_refund_clear_statements(
                                            char_db.as_ref(),
                                            &mut refund_cleanup_tx,
                                            item_db_guid,
                                            new_flags,
                                        );
                                        new_flags
                                    }
                                    LoadedItemRefundDecision::None
                                    | LoadedItemRefundDecision::Valid { .. } => item_flags,
                                };
                                inv_slots[slot as usize] = item_guid;
                                item_creates.push(wow_packet::packets::update::ItemCreateData {
                                    item_guid,
                                    entry_id: item_entry as i32,
                                    owner_guid: guid,
                                    contained_in: guid,
                                    stack_count: item_count,
                                    dynamic_flags: stored_flags,
                                    durability: item_durability,
                                    max_durability: item_max_durability,
                                    random_properties_seed: 0,
                                    random_properties_id: 0,
                                    context: 0,
                                });
                                let inventory_type =
                                    self.item_template_inventory_type(item_entry).or_else(|| {
                                        if slot < 19 {
                                            slot_to_inventory_type(slot)
                                        } else {
                                            None
                                        }
                                    });
                                let inventory_item = InventoryItem {
                                    guid: item_guid,
                                    entry_id: item_entry,
                                    db_guid: item_db_guid,
                                    inventory_type,
                                };
                                if WorldSession::is_buyback_slot(slot) {
                                    self.insert_buyback_item_like_cpp(slot, inventory_item);
                                } else {
                                    self.insert_inventory_item_like_cpp(slot, inventory_item);
                                }
                                let mut item_object = self.make_inventory_item_object(
                                    item_guid,
                                    item_entry,
                                    guid,
                                    item_count,
                                    item_durability,
                                    item_context,
                                    slot,
                                );
                                item_object.set_create_played_time(item_played_time);
                                item_object.replace_all_item_flags(
                                    ItemFieldFlags::from_bits_retain(stored_flags),
                                );
                                if let LoadedItemRefundDecision::Valid {
                                    paid_money,
                                    paid_extended_cost,
                                } = refund_decision
                                {
                                    item_object.set_refund_recipient(guid);
                                    item_object.set_paid_money(paid_money);
                                    item_object
                                        .set_paid_extended_cost(u32::from(paid_extended_cost));
                                }
                                self.apply_loaded_inventory_item_collection_hooks_like_cpp(
                                    &item_object,
                                );
                                item_object.set_state(ItemUpdateState::Unchanged);
                                self.insert_inventory_item_object(item_object);
                                // Slots 0-18 also populate VisibleItems for character model
                                if (slot as usize) < 19 {
                                    visible_items[slot as usize] = (item_entry as i32, 0, 0);
                                }
                            }
                            if !eq_result.next_row() {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to load equipment for {:?}: {}", guid, e);
                }
            }
            if !refund_cleanup_tx.is_empty() {
                if let Err(e) = char_db.commit_transaction(refund_cleanup_tx).await {
                    warn!(
                        "Failed to clean expired/missing item refund metadata for {:?}: {}",
                        guid, e
                    );
                }
            }

            // ── Load represented bag contents (nested items) ──
            // C++ `Player::_LoadInventory` loads child rows after their top-level
            // bag rows. `character_inventory.bag` stores the bag item GUID, so the
            // query joins back to the represented bag row and returns its top-level slot.
            {
                let mut bag_stmt = char_db.prepare(CharStatements::SEL_CHAR_BAG_CONTENTS);
                bag_stmt.set_u64(0, guid.counter() as u64);
                match char_db.query(&bag_stmt).await {
                    Ok(mut bag_result) => {
                        if !bag_result.is_empty() {
                            loop {
                                let bag_slot: u8 = bag_result.read(0);
                                let inner_slot: u8 = bag_result.read(1);
                                let item_entry: u32 = bag_result.try_read(2).unwrap_or(0);
                                let item_db_guid: u64 = bag_result.try_read(3).unwrap_or(0);
                                let item_count: u32 = bag_result.try_read(4).unwrap_or(1);
                                let item_durability: u32 = bag_result.try_read(5).unwrap_or(0);
                                let item_context = bag_result
                                    .try_read::<u8>(6)
                                    .and_then(<ItemContext as num_traits::FromPrimitive>::from_u8)
                                    .unwrap_or(ItemContext::None);
                                let item_flags = bag_result.try_read::<u32>(7).unwrap_or(0);
                                let item_played_time = bag_result.try_read::<u32>(8).unwrap_or(0);
                                if item_entry > 0 && is_represented_bag_slot(bag_slot) {
                                    if let Some(bag_item_guid) = self
                                        .inventory_items_like_cpp()
                                        .get(&bag_slot)
                                        .map(|bag_item| bag_item.guid)
                                    {
                                        let item_guid =
                                            ObjectGuid::create_item(realm_id, item_db_guid as i64);
                                        let mut item_object = self.make_inventory_item_object(
                                            item_guid,
                                            item_entry,
                                            guid,
                                            item_count,
                                            item_durability,
                                            item_context,
                                            inner_slot,
                                        );
                                        item_object.set_create_played_time(item_played_time);
                                        item_object.replace_all_item_flags(
                                            ItemFieldFlags::from_bits_retain(item_flags),
                                        );
                                        item_object
                                            .set_container_guid_and_slot(bag_item_guid, bag_slot);
                                        self.apply_loaded_inventory_item_collection_hooks_like_cpp(
                                            &item_object,
                                        );
                                        item_object.set_state(ItemUpdateState::Unchanged);
                                        self.insert_inventory_item_object(item_object);
                                    } else {
                                        warn!(
                                            "Skipping bag content {:?}/{} for {:?}: missing represented bag slot {}",
                                            ObjectGuid::create_item(realm_id, item_db_guid as i64),
                                            inner_slot,
                                            guid,
                                            bag_slot
                                        );
                                    }
                                }
                                if !bag_result.next_row() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to load bag contents for {:?}: {}", guid, e);
                    }
                }
            }

            // inventory_type is now loaded from the canonical ItemTemplate bridge.
            // No SQL cache needed.
        }
        self.sync_player_inventory_like_cpp();

        // ── Load character currencies from character_currency ──
        // C++ `Player::_LoadCurrency` skips rows not found in sCurrencyTypesStore.
        {
            let mut currency_stmt = char_db.prepare(CharStatements::SEL_PLAYER_CURRENCY);
            currency_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&currency_stmt).await {
                Ok(mut currency_result) => {
                    if !currency_result.is_empty() {
                        loop {
                            let currency_id: u32 =
                                u32::from(currency_result.try_read::<u16>(0).unwrap_or(0));
                            let known_currency = self
                                .currency_types_store()
                                .is_some_and(|store| store.has_record(currency_id));
                            if known_currency {
                                let mut currencies = self.player_currencies_like_cpp().clone();
                                currencies.entry(currency_id).or_insert_with(|| {
                                    crate::session::PlayerCurrency {
                                        state: crate::session::PlayerCurrencyState::Unchanged,
                                        quantity: currency_result.try_read(1).unwrap_or(0),
                                        weekly_quantity: currency_result.try_read(2).unwrap_or(0),
                                        tracked_quantity: currency_result.try_read(3).unwrap_or(0),
                                        increased_cap_quantity: currency_result
                                            .try_read(4)
                                            .unwrap_or(0),
                                        earned_quantity: currency_result.try_read(5).unwrap_or(0),
                                        flags: currency_result.try_read(6).unwrap_or(0),
                                    }
                                });
                                self.set_player_currencies_like_cpp(currencies);
                            }
                            if !currency_result.next_row() {
                                break;
                            }
                        }
                    }
                    info!(
                        "Loaded {} currencies for {:?}",
                        self.player_currencies_like_cpp().len(),
                        guid
                    );
                    self.sync_player_currencies_like_cpp();
                }
                Err(e) => {
                    warn!("Failed to load currencies for {:?}: {}", guid, e);
                }
            }
        }

        // ── Load known spells from character_spell ──
        // Column types: spell=int unsigned, active=tinyint unsigned, disabled=tinyint unsigned
        let mut known_spells: Vec<i32> = Vec::new();
        {
            let mut spell_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_SPELL);
            spell_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&spell_stmt).await {
                Ok(mut spell_result) => {
                    if !spell_result.is_empty() {
                        loop {
                            let spell_id: u32 = spell_result.try_read(0).unwrap_or(0);
                            let active: u8 = spell_result.try_read(1).unwrap_or(1);
                            let _disabled: u8 = spell_result.try_read(2).unwrap_or(0);
                            if spell_id > 0 && active != 0 {
                                known_spells.push(spell_id as i32);
                            }
                            if !spell_result.next_row() {
                                break;
                            }
                        }
                    }
                    info!("Loaded {} DB spells for {:?}", known_spells.len(), guid);
                }
                Err(e) => {
                    warn!("Failed to load spells for {:?}: {}", guid, e);
                }
            }
        }

        // ── Load character skill IDs from character_skills table ──
        // These are used to filter DBC auto-learned spells (weapons, languages,
        // racials, worn armor type). This matches C# behavior where
        // LearnSkillRewardedSpells() only runs for skills the character actually has.
        let mut known_skill_ids = std::collections::HashSet::<u16>::new();
        let mut skill_values = std::collections::HashMap::<u16, u16>::new();
        {
            let mut skill_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_SKILLS);
            skill_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&skill_stmt).await {
                Ok(mut skill_result) => {
                    if !skill_result.is_empty() {
                        loop {
                            let skill_id: u16 = skill_result.try_read(0).unwrap_or(0);
                            let skill_value: u16 = skill_result.try_read(1).unwrap_or(0);
                            if skill_id > 0 {
                                known_skill_ids.insert(skill_id);
                                skill_values.insert(skill_id, skill_value);
                            }
                            if !skill_result.next_row() {
                                break;
                            }
                        }
                    }
                    info!(
                        "Loaded {} known skill IDs for {:?}",
                        known_skill_ids.len(),
                        guid
                    );
                }
                Err(e) => {
                    warn!("Failed to load character_skills for {:?}: {}", guid, e);
                }
            }
        }
        self.set_player_skill_values_like_cpp(skill_values);

        // ── Merge DBC auto-learned spells + build SkillInfo ──
        // Only supplement from DBC if character has NO spells in DB (new character).
        // Existing characters should rely entirely on their character_spell table.
        let db_count = known_spells.len();
        let mut skill_info_tuples: Vec<(u16, u16, u16, u16, u16, i16, u16)> = Vec::new();
        if let Some(skill_store) = self.skill_store() {
            // Always supplement with DBC auto-learned spells (acquire_method 1 & 2 only).
            // This covers racial abilities, languages, and weapon passives that are
            // auto-granted from skills the character has in character_skills.
            // Class trainer spells (acquire_method 0) come from character_spell DB.
            let dbc_spells =
                skill_store.starting_spells(race, class, level, Some(&known_skill_ids));
            let racial = skill_store.racial_spells(race);
            for spell_id in dbc_spells.into_iter().chain(racial.into_iter()) {
                if !known_spells.contains(&spell_id) {
                    known_spells.push(spell_id);
                }
            }
            info!(
                "Total spells for {:?}: {} ({} from DB, {} from DBC)",
                guid,
                known_spells.len(),
                db_count,
                known_spells.len() - db_count
            );

            // Build SkillInfo entries for the UpdateObject SkillInfo array.
            // C#: LearnDefaultSkills → SetSkill writes skill slots.
            let skill_entries = skill_store.starting_skill_info(race, class, level);
            for entry in &skill_entries {
                skill_info_tuples.push((
                    entry.skill_id,
                    entry.step,
                    entry.rank,
                    entry.starting_rank,
                    entry.max_rank,
                    entry.temp_bonus,
                    entry.perm_bonus,
                ));
            }
            info!("Loaded {} skill slots for {:?}", skill_entries.len(), guid);
        }

        // Store final known_spells in session for later use (ShowTradeSkill, etc.)
        self.set_known_spells_like_cpp(known_spells.clone());

        // ── Load action buttons from character_action ──
        // Column types: button=tinyint unsigned, action=int unsigned, type=tinyint unsigned
        let mut action_buttons = [0i64; 180];
        let mut action_count = 0u32;
        {
            let mut action_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_ACTIONS_SPEC);
            action_stmt.set_u64(0, guid.counter() as u64);
            action_stmt.set_u8(1, 0); // spec = 0
            action_stmt.set_u8(2, 0); // traitConfigId = 0
            match char_db.query(&action_stmt).await {
                Ok(mut action_result) => {
                    if !action_result.is_empty() {
                        loop {
                            let button: u8 = action_result.read(0);
                            let action: u32 = action_result.try_read(1).unwrap_or(0);
                            let btn_type: u8 = action_result.try_read(2).unwrap_or(0);
                            if (button as usize) < 180 && action > 0 {
                                action_buttons[button as usize] =
                                    wow_packet::packets::misc::UpdateActionButtons::pack_button(
                                        action as i32,
                                        btn_type,
                                    );
                                action_count += 1;
                            }
                            if !action_result.next_row() {
                                break;
                            }
                        }
                    }
                    info!("Loaded {} action buttons for {:?}", action_count, guid);
                }
                Err(e) => {
                    warn!("Failed to load action buttons for {:?}: {}", guid, e);
                }
            }
        }

        // Store current map and character info for VALUES updates + stat recalculation
        self.set_loaded_player_identity_like_cpp(map_id as u16, race, class, level, gender);
        self.refresh_next_level_xp();
        // NOTE: known_spells is stored below after DBC merge (see "Merge DBC auto-learned spells")

        // C++ login query set includes CHAR_SEL_CHARACTER_REPUTATION and
        // ReputationMgr::LoadFromDB reinitializes from Faction.db2 before merging rows.
        {
            let mut reputation_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_REPUTATION);
            reputation_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&reputation_stmt).await {
                Ok(mut reputation_result) => {
                    let mut rows = Vec::new();
                    if !reputation_result.is_empty() {
                        loop {
                            rows.push(CharacterReputationRowLikeCpp {
                                faction_id: reputation_result.try_read(0).unwrap_or(0),
                                standing: reputation_result.try_read(1).unwrap_or(0),
                                flags: reputation_result.try_read(2).unwrap_or(0),
                            });
                            if !reputation_result.next_row() {
                                break;
                            }
                        }
                    }
                    if self.load_character_reputation_rows_like_cpp(rows) {
                        info!("Loaded character reputation rows for {:?}", guid);
                    } else {
                        warn!(
                            "Skipped character reputation load for {:?}: missing Faction.db2 store",
                            guid
                        );
                    }
                }
                Err(e) => {
                    warn!("Failed to load character reputation for {:?}: {}", guid, e);
                }
            }
        }

        // Sum gear stat bonuses from equipped items (slots 0-18)
        let (gear_stats, gear_ap, gear_rap, gear_health, gear_mana) =
            if let Some(iss) = self.item_stats_store() {
                let mut bonuses = [0i32; 5]; // STR, AGI, STA, INT, SPI
                let mut g_ap = 0i32;
                let mut g_rap = 0i32;
                let mut g_health = 0i32;
                let mut g_mana = 0i32;
                for (&slot, inv_item) in self.inventory_items_like_cpp() {
                    if slot < 19 {
                        // only equipped gear slots affect stats
                        if let Some(entry) = iss.get(inv_item.entry_id) {
                            let [s, a, st, i, sp] = entry.base_stat_bonuses();
                            bonuses[0] += s;
                            bonuses[1] += a;
                            bonuses[2] += st;
                            bonuses[3] += i;
                            bonuses[4] += sp;
                            g_ap += entry.attack_power_bonus();
                            g_rap += entry.ranged_attack_power_bonus();
                            g_health += entry.health_bonus();
                            g_mana += entry.mana_bonus();
                        }
                    }
                }
                (bonuses, g_ap, g_rap, g_health, g_mana)
            } else {
                ([0i32; 5], 0, 0, 0, 0)
            };

        // Compute real stats from player_levelstats + gear bonuses
        let combat = if let Some(store) = self.player_stats() {
            if let Some(ls) = store.get(race, class, level) {
                // Total stats = base + gear
                let total_str = ls.strength as i32 + gear_stats[0];
                let total_agi = ls.agility as i32 + gear_stats[1];
                let total_sta = ls.stamina as i32 + gear_stats[2];
                let total_int = ls.intellect as i32 + gear_stats[3];
                let total_spi = ls.spirit as i32 + gear_stats[4];

                // MaxHealth from total STA
                let sta64 = total_sta as i64;
                let base_hp = ls.base_health as i64;
                let hp_bonus = sta64.min(20) + (sta64 - 20).max(0) * 10 + gear_health as i64;
                let max_health = base_hp + hp_bonus;

                // MaxMana from total INT
                let int64 = total_int as i64;
                let base_mp = ls.base_mana as i64;
                let mp_bonus = int64.min(20) + (int64 - 20).max(0) * 15 + gear_mana as i64;
                let max_mana = base_mp + mp_bonus;

                // Armor from total AGI
                let base_armor = total_agi * 2;

                // Attack power from total stats + gear AP
                let melee_ap = match class {
                    1 | 2 | 6 => total_str * 2 - 20,
                    3 | 4 => total_str + total_agi - 20,
                    7 | 11 => total_str * 2 - 20,
                    _ => (total_str - 10).max(0),
                }
                .max(0)
                    + gear_ap;

                let ranged_ap = match class {
                    3 => total_agi * 2 - 20,
                    1 | 4 => total_agi - 10,
                    _ => 0,
                }
                .max(0)
                    + gear_rap;

                // Damage from total AP
                let ap_f = melee_ap as f32;
                let base_dmg = ap_f / 14.0 * 2.0;
                let min_d = (base_dmg + 1.0).max(1.0);
                let max_d = min_d + 1.0;

                let rap_f = ranged_ap as f32;
                let (min_rd, max_rd) = if rap_f > 0.0 {
                    let rd = rap_f / 14.0 * 2.8;
                    ((rd + 1.0).max(1.0), rd + 3.0)
                } else {
                    (0.0, 0.0)
                };

                PlayerCombatStats {
                    health: max_health,
                    max_health,
                    stats: [total_str, total_agi, total_sta, total_int, total_spi],
                    base_armor,
                    max_mana,
                    attack_power: melee_ap,
                    ranged_attack_power: ranged_ap,
                    min_damage: min_d,
                    max_damage: max_d,
                    min_ranged_damage: min_rd,
                    max_ranged_damage: max_rd,
                    dodge_pct: ls.dodge_pct(class, level),
                    parry_pct: ls.parry_pct(class),
                    crit_pct: ls.crit_pct(class, level),
                    ranged_crit_pct: ls.crit_pct(class, level),
                    spell_crit_pct: ls.spell_crit_pct(class, level),
                }
            } else {
                warn!(
                    "No player_levelstats for race={race} class={class} level={level}, using fallback"
                );
                let (h, m) = default_health_mana(class);
                PlayerCombatStats {
                    health: h as i64,
                    max_health: h as i64,
                    max_mana: m as i64,
                    ..PlayerCombatStats::default()
                }
            }
        } else {
            let (h, m) = default_health_mana(class);
            PlayerCombatStats {
                health: h as i64,
                max_health: h as i64,
                max_mana: m as i64,
                ..PlayerCombatStats::default()
            }
        };

        info!(
            "Player '{}' ({:?}) continuing login at map {} ({}, {}, {}), {} equipped items, \
             HP={} Mana={} AP={} STR/AGI/STA/INT/SPI={:?} Armor={} Dodge={:.1}% Crit={:.1}%",
            name,
            guid,
            map_id,
            pos_x,
            pos_y,
            pos_z,
            item_creates.len(),
            combat.max_health,
            combat.max_mana,
            combat.attack_power,
            combat.stats,
            combat.base_armor,
            combat.dodge_pct,
            combat.crit_pct
        );

        // Load active quests from characters DB
        self.load_player_quests().await;

        self.send_login_sequence(
            guid,
            race,
            class,
            gender,
            level,
            display_id,
            &position,
            map_id,
            zone,
            visible_items,
            inv_slots,
            item_creates,
            combat,
            known_spells,
            action_buttons,
            skill_info_tuples,
            account_mounts,
        );

        // Mark online in DB
        let mut online_stmt = char_db.prepare(CharStatements::UPD_CHAR_ONLINE);
        online_stmt.set_u32(0, guid.counter() as u32);
        let _ = char_db.execute(&online_stmt).await;
    }

    /// Fallback: skip ConnectTo and trigger direct login on the realm socket.
    ///
    /// Used when no session manager is configured or all ConnectTo retries fail.
    /// Sets a flag so that `process_pending` will call `handle_continue_player_login`.
    fn fallback_direct_login(&mut self) {
        // player_loading is already set — create a dummy oneshot that fires immediately
        let (tx, rx) = tokio::sync::oneshot::channel();
        let link = wow_network::session_mgr::InstanceLink {
            send_tx: self.send_tx().clone(),
            pkt_rx: None, // None = keep using realm socket's packet_rx
        };
        let _ = tx.send(link);
        self.set_instance_link_rx(Some(rx));
        info!(
            "Fallback: direct login scheduled for account {}",
            self.account_id
        );
    }

    /// Send nearby creatures to the client as UpdateObject packets.
    ///
    /// Queries the world database for creatures within visibility range
    /// on the player's map, builds CreatureCreateData for each, and sends
    /// a batched UpdateObject.
    pub async fn send_nearby_creatures(&mut self, map_id: u16, position: &Position, zone_id: u32) {
        const VISIBILITY_RANGE: f32 = 800.0;

        let map_creatures = self.visible_world_creatures_from_map_like_cpp(map_id, position);
        if self.has_world_map_manager_like_cpp() {
            if map_creatures.is_empty() {
                self.client_visible_guids_like_cpp
                    .retain(|guid| !guid.is_any_type_creature());
                self.last_visibility_pos = Some(*position);
                return;
            }

            let mut blocks = Vec::with_capacity(map_creatures.len());
            let mut visible = HashSet::with_capacity(map_creatures.len());
            for creature in &map_creatures {
                let mut create_data = creature.create_data.clone();
                create_data.health = i64::from(creature.current_hp());
                create_data.max_health = i64::from(creature.max_hp());
                create_data.level = creature.level();
                create_data.npc_flags = creature.npc_flags_mask_like_cpp();
                create_data.npc_flags = self
                    .represented_viewer_dependent_creature_npc_flags_like_cpp(
                        creature.guid(),
                        create_data.npc_flags,
                    );
                create_data.zone_id = zone_id;
                blocks.push(UpdateObject::create_creature_block(
                    create_data,
                    &creature.position(),
                ));
                visible.insert(creature.guid());
            }

            self.client_visible_guids_like_cpp
                .retain(|guid| !guid.is_any_type_creature());
            self.client_visible_guids_like_cpp.extend(visible);
            self.last_visibility_pos = Some(*position);
            self.send_packet(&UpdateObject::create_creatures(blocks, map_id));
            debug!(
                "Sent {} map-owned creatures to account {} on map {}",
                map_creatures.len(),
                self.account_id,
                map_id
            );
            return;
        }

        let world_db = match self.world_db() {
            Some(db) => Arc::clone(db),
            None => {
                warn!("No world database — skipping creature spawn");
                return;
            }
        };

        let x_min = position.x - VISIBILITY_RANGE;
        let x_max = position.x + VISIBILITY_RANGE;
        let y_min = position.y - VISIBILITY_RANGE;
        let y_max = position.y + VISIBILITY_RANGE;

        let mut stmt = world_db.prepare(WorldStatements::SEL_CREATURES_IN_RANGE);
        stmt.set_u16(0, map_id);
        stmt.set_f32(1, x_min);
        stmt.set_f32(2, x_max);
        stmt.set_f32(3, y_min);
        stmt.set_f32(4, y_max);

        let result =
            match tokio::time::timeout(std::time::Duration::from_secs(5), world_db.query(&stmt))
                .await
            {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    warn!("Failed to query creatures for map {map_id}: {e}");
                    return;
                }
                Err(_) => {
                    warn!("Creature query timed out for map {map_id}");
                    return;
                }
            };

        if result.is_empty() {
            return;
        }

        let realm_id = self.realm_id();
        let mut blocks = Vec::new();
        let mut visible_guids = Vec::new();
        let mut result = result;

        loop {
            // BIGINT UNSIGNED may fail as u64 in sqlx — read as i64 first, cast to u64
            let spawn_guid: u64 = result
                .try_read::<i64>(0)
                .map(|v| v as u64)
                .or_else(|| result.try_read::<u64>(0))
                .unwrap_or(0);
            let entry: u32 = result.try_read(1).unwrap_or(0);
            let pos_x: f32 = result.try_read(2).unwrap_or(0.0);
            let pos_y: f32 = result.try_read(3).unwrap_or(0.0);
            let pos_z: f32 = result.try_read(4).unwrap_or(0.0);
            let orientation: f32 = result.try_read(5).unwrap_or(0.0);
            let cur_health: u32 = result.try_read(6).unwrap_or(100);
            let _cur_mana: u32 = result.try_read(7).unwrap_or(0);
            let model_id: u32 = result.try_read(8).unwrap_or(0);
            let min_level: u8 = result.try_read::<Option<u8>>(9).flatten().unwrap_or(1);
            let _max_level: u8 = result.try_read::<Option<u8>>(10).flatten().unwrap_or(1);
            let faction: i32 = result.try_read::<u16>(11).unwrap_or(35) as i32;
            // BIGINT UNSIGNED may fail as u64 in sqlx — read as i64 first
            let npc_flags: u64 = result
                .try_read::<i64>(12)
                .map(|v| v as u64)
                .or_else(|| result.try_read::<u64>(12))
                .unwrap_or(0);
            let unit_flags: u32 = result.try_read(13).unwrap_or(0);
            let unit_flags2: u32 = result.try_read(14).unwrap_or(0);
            let unit_flags3: u32 = result.try_read(15).unwrap_or(0);
            let speed_walk: f32 = result.try_read(16).unwrap_or(1.0);
            let speed_run: f32 = result.try_read(17).unwrap_or(1.14286);
            let scale: f32 = result.try_read(18).unwrap_or(1.0);
            let unit_class: u8 = result.try_read(19).unwrap_or(1);
            let flags_extra: u32 = result.try_read(20).unwrap_or(0);
            let base_attack_time: u32 = result.try_read(21).unwrap_or(2000);
            let _ranged_attack_time: u32 = result.try_read(22).unwrap_or(0);
            let template_display_id: u32 =
                result.try_read::<Option<u32>>(23).flatten().unwrap_or(0);
            let loot_id: u32 = result.try_read::<Option<u32>>(24).flatten().unwrap_or(0);
            let skin_loot_id: u32 = result.try_read::<Option<u32>>(25).flatten().unwrap_or(0);
            let gold_min: u32 = result.try_read::<Option<u32>>(26).flatten().unwrap_or(0);
            let gold_max: u32 = result.try_read::<Option<u32>>(27).flatten().unwrap_or(0);
            let phase_use_flags: u8 = result
                .try_read::<u8>(28)
                .or_else(|| result.try_read::<i16>(28).map(|value| value.max(0) as u8))
                .unwrap_or(0);
            let phase_id: u16 = result
                .try_read::<u16>(29)
                .or_else(|| result.try_read::<i32>(29).map(|value| value.max(0) as u16))
                .unwrap_or(0);
            let phase_group_id: u32 = result
                .try_read::<u32>(30)
                .or_else(|| result.try_read::<i32>(30).map(|value| value.max(0) as u32))
                .unwrap_or(0);
            let terrain_swap_map: i32 = result.try_read(31).unwrap_or(-1);
            let ground_movement_type: u8 = result
                .try_read::<Option<u8>>(32)
                .flatten()
                .or_else(|| result.try_read::<u8>(32))
                .or_else(|| result.try_read::<i16>(32).map(|value| value.max(0) as u8))
                .unwrap_or(wow_constants::CreatureGroundMovementType::Run as u8);
            let swim_allowed: bool = result
                .try_read::<Option<u8>>(33)
                .flatten()
                .or_else(|| result.try_read::<u8>(33))
                .or_else(|| result.try_read::<i16>(33).map(|value| value.max(0) as u8))
                .unwrap_or(1)
                != 0;
            let flight_movement_type: u8 = result
                .try_read::<Option<u8>>(34)
                .flatten()
                .or_else(|| result.try_read::<u8>(34))
                .or_else(|| result.try_read::<i16>(34).map(|value| value.max(0) as u8))
                .unwrap_or(0);
            let default_movement_type = result
                .try_read::<Option<u8>>(CREATURE_SPAWN_EFFECTIVE_MOVEMENT_TYPE_COLUMN)
                .flatten()
                .or_else(|| result.try_read::<u8>(CREATURE_SPAWN_EFFECTIVE_MOVEMENT_TYPE_COLUMN))
                .or_else(|| {
                    result
                        .try_read::<i16>(CREATURE_SPAWN_EFFECTIVE_MOVEMENT_TYPE_COLUMN)
                        .map(|value| value.max(0) as u8)
                })
                .map(creature_movement_generator_type_from_db_like_cpp)
                .unwrap_or(MovementGeneratorType::Idle);
            let waypoint_path_id: u32 = result
                .try_read::<Option<u32>>(CREATURE_SPAWN_WAYPOINT_PATH_ID_COLUMN)
                .flatten()
                .or_else(|| result.try_read::<u32>(CREATURE_SPAWN_WAYPOINT_PATH_ID_COLUMN))
                .or_else(|| {
                    result
                        .try_read::<i64>(CREATURE_SPAWN_WAYPOINT_PATH_ID_COLUMN)
                        .map(|value| value.max(0) as u32)
                })
                .unwrap_or(0);

            let display_id = if model_id > 0 {
                model_id
            } else if template_display_id > 0 {
                template_display_id
            } else {
                if !result.next_row() {
                    break;
                }
                continue;
            };

            let (target_phase_shift, _) = self.db_spawn_phase_shift_like_cpp(
                map_id,
                phase_use_flags,
                phase_id,
                phase_group_id,
                terrain_swap_map,
            );
            if !self.can_see_phase_shift_like_cpp(&target_phase_shift) {
                if !result.next_row() {
                    break;
                }
                continue;
            }

            let health = if cur_health > 0 {
                cur_health as i64
            } else {
                100
            };

            let guid = ObjectGuid::create_world_object(
                HighGuid::Creature,
                0,
                realm_id,
                map_id,
                1,
                entry,
                spawn_guid as i64,
            );

            let creature_pos = Position::new(pos_x, pos_y, pos_z, orientation);
            let create_data = CreatureCreateData {
                guid,
                entry,
                display_id,
                native_display_id: display_id,
                health,
                max_health: health,
                level: min_level,
                faction_template: faction,
                npc_flags,
                unit_flags,
                unit_flags2,
                unit_flags3,
                damage_school: wow_constants::spell::SpellSchools::Normal as u8,
                scale,
                unit_class,
                base_attack_time,
                ranged_attack_time: base_attack_time,
                zone_id,
                speed_walk_rate: speed_walk,
                speed_run_rate: speed_run,
                ai_anim_kit_id: 0,
                movement_anim_kit_id: 0,
                melee_anim_kit_id: 0,
            };

            // Register through canonical map state when available; the legacy
            // per-session AI object remains a compatibility facade/cache.
            let aggro_radius = self
                .creature_aggro_radius_for_faction_template_like_cpp(faction.max(0) as u32, 15.0);
            let min_dmg = (min_level as u32).saturating_sub(1) * 3 + 5;
            let max_dmg = min_dmg + min_dmg / 2;
            self.register_world_creature_with_flags_extra_movement_and_default_motion_like_cpp(
                map_id,
                creature_pos,
                create_data.clone(),
                min_dmg,
                max_dmg,
                aggro_radius,
                loot_id,
                skin_loot_id,
                gold_min,
                gold_max,
                None,
                0,
                phase_use_flags,
                phase_id,
                phase_group_id,
                terrain_swap_map,
                flags_extra,
                ground_movement_type,
                swim_allowed,
                flight_movement_type,
                default_movement_type,
                waypoint_path_id,
            );

            let mut viewer_create_data = create_data.clone();
            viewer_create_data.npc_flags = self
                .represented_viewer_dependent_creature_npc_flags_like_cpp(
                    guid,
                    viewer_create_data.npc_flags,
                );
            blocks.push(UpdateObject::create_creature_block(
                viewer_create_data,
                &creature_pos,
            ));
            visible_guids.push(guid);

            if !result.next_row() {
                break;
            }
        }

        if blocks.is_empty() {
            return;
        }

        let count = blocks.len();
        // Mirror C++ Player::m_clientGUIDs semantics: this is the exact set
        // of creatures sent to this client, not every creature loaded on map.
        self.client_visible_guids_like_cpp
            .retain(|guid| !guid.is_any_type_creature());
        self.client_visible_guids_like_cpp
            .extend(visible_guids.iter().copied());
        self.last_visibility_pos = Some(*position);
        let update = UpdateObject::create_creatures(blocks, map_id);
        self.send_packet(&update);
        let mob_count = visible_guids
            .iter()
            .filter(|g| {
                self.mutate_world_creature(**g, |creature| creature.npc_flags() == 0)
                    .unwrap_or(false)
            })
            .count();
        let npc_count = visible_guids.len().saturating_sub(mob_count);
        debug!(
            "Sent {} creatures ({} mobs / {} npcs) to account {} on map {}",
            count, mob_count, npc_count, self.account_id, map_id
        );
    }

    /// Dynamic visibility update — called when the player moves significantly.
    ///
    /// Queries the DB for all creatures/GOs in the new range, diffs against
    /// the current visible set, and sends:
    ///  - SMSG_UPDATE_OBJECT (CreateObject2) for newly visible objects
    ///  - SMSG_UPDATE_OBJECT (OutOfRange) for objects that left the range
    ///
    /// Threshold: only triggers if the player moved more than 50 yards from
    /// the last visibility update position.
    pub async fn update_visibility(&mut self) {
        use std::collections::HashSet;

        // ── Position & threshold check ──────────────────────────────────
        self.sync_represented_farsight_clear_from_canonical_like_cpp();
        let pos = match self.represented_visibility_source_position_like_cpp() {
            Some(p) => p,
            None => return,
        };
        if let Some(last) = self.last_visibility_pos {
            let dx = pos.x - last.x;
            let dy = pos.y - last.y;
            if dx * dx + dy * dy < 50.0 * 50.0 {
                return; // haven't moved enough yet
            }
        }

        let map_id = self.player_map_id_like_cpp();
        let realm_id = self.realm_id();

        const RANGE: f32 = 800.0;
        let x_min = pos.x - RANGE;
        let x_max = pos.x + RANGE;
        let y_min = pos.y - RANGE;
        let y_max = pos.y + RANGE;

        let map_creatures = self.visible_world_creatures_from_map_like_cpp(map_id, &pos);
        let canonical_gameobjects =
            self.visible_gameobjects_from_canonical_map_like_cpp(map_id, &pos, RANGE);
        let canonical_dynamic_objects =
            self.visible_dynamic_objects_from_canonical_map_like_cpp(map_id, &pos, RANGE);
        let has_map_visibility_source = self.has_world_map_manager_like_cpp()
            || canonical_gameobjects.is_some()
            || canonical_dynamic_objects.is_some();

        if has_map_visibility_source {
            let mut new_visible_creatures: HashSet<ObjectGuid> = HashSet::new();
            let mut new_creature_blocks: Vec<UpdateBlock> = Vec::new();
            for creature in &map_creatures {
                let guid = creature.guid();
                new_visible_creatures.insert(guid);
                if !self.client_visible_guids_like_cpp.contains(&guid) {
                    let mut create_data = creature.create_data.clone();
                    create_data.health = i64::from(creature.current_hp());
                    create_data.max_health = i64::from(creature.max_hp());
                    create_data.level = creature.level();
                    create_data.npc_flags = creature.npc_flags_mask_like_cpp();
                    create_data.npc_flags = self
                        .represented_viewer_dependent_creature_npc_flags_like_cpp(
                            guid,
                            create_data.npc_flags,
                        );
                    new_creature_blocks.push(UpdateObject::create_creature_block(
                        create_data,
                        &creature.position(),
                    ));
                }
            }

            let removed_creatures: Vec<ObjectGuid> = self
                .client_visible_guids_like_cpp
                .iter()
                .filter(|g| g.is_any_type_creature() && !new_visible_creatures.contains(g))
                .copied()
                .collect();

            if !new_creature_blocks.is_empty() {
                debug!(
                    "Visibility update: {} map-owned creatures",
                    new_creature_blocks.len()
                );
                self.send_packet(&UpdateObject::create_creatures(new_creature_blocks, map_id));
            }
            if !removed_creatures.is_empty() {
                debug!(
                    "Visibility update: {} map-owned creatures out of range",
                    removed_creatures.len()
                );
                self.send_packet(&UpdateObject::out_of_range_objects(
                    removed_creatures,
                    map_id,
                ));
            }
            self.client_visible_guids_like_cpp
                .retain(|guid| !guid.is_any_type_creature());
            self.client_visible_guids_like_cpp
                .extend(new_visible_creatures.iter().copied());

            if let Some(gameobjects) = canonical_gameobjects {
                let new_visible_gos: HashSet<_> = gameobjects.iter().map(|go| go.guid).collect();
                let new_go_blocks = gameobjects
                    .into_iter()
                    .filter(|go| !self.client_visible_guids_like_cpp.contains(&go.guid))
                    .map(UpdateObject::create_gameobject_block)
                    .collect::<Vec<_>>();
                let removed_gos: Vec<ObjectGuid> = self
                    .client_visible_guids_like_cpp
                    .iter()
                    .filter(|g| g.is_game_object() && !new_visible_gos.contains(g))
                    .copied()
                    .collect();
                for guid in &removed_gos {
                    self.represented_gameobject_phase_shifts.remove(guid);
                }

                if !new_go_blocks.is_empty() {
                    debug!(
                        "Visibility update: {} canonical game objects",
                        new_go_blocks.len()
                    );
                    self.send_packet(&UpdateObject::create_world_objects(new_go_blocks, map_id));
                }
                if !removed_gos.is_empty() {
                    debug!(
                        "Visibility update: {} canonical game objects out of range",
                        removed_gos.len()
                    );
                    self.send_packet(&UpdateObject::out_of_range_objects(
                        removed_gos.clone(),
                        map_id,
                    ));
                }
                for guid in &removed_gos {
                    self.client_visible_guids_like_cpp.remove(guid);
                }
                self.client_visible_guids_like_cpp
                    .extend(new_visible_gos.iter().copied());
            }

            if let Some(dynamic_objects) = canonical_dynamic_objects {
                let new_visible_dynamic_objects: HashSet<_> = dynamic_objects
                    .iter()
                    .map(|dynamic_object| dynamic_object.guid)
                    .collect();
                let new_dynamic_object_blocks = dynamic_objects
                    .into_iter()
                    .filter(|dynamic_object| {
                        !self
                            .client_visible_guids_like_cpp
                            .contains(&dynamic_object.guid)
                    })
                    .map(UpdateObject::create_dynamic_object_block)
                    .collect::<Vec<_>>();
                let removed_dynamic_objects: Vec<ObjectGuid> = self
                    .client_visible_guids_like_cpp
                    .iter()
                    .filter(|g| g.is_dynamic_object() && !new_visible_dynamic_objects.contains(g))
                    .copied()
                    .collect();

                if !new_dynamic_object_blocks.is_empty() {
                    debug!(
                        "Visibility update: {} canonical dynamic objects",
                        new_dynamic_object_blocks.len()
                    );
                    self.send_packet(&UpdateObject::create_world_objects(
                        new_dynamic_object_blocks,
                        map_id,
                    ));
                }
                if !removed_dynamic_objects.is_empty() {
                    debug!(
                        "Visibility update: {} canonical dynamic objects out of range",
                        removed_dynamic_objects.len()
                    );
                    self.send_packet(&UpdateObject::out_of_range_objects(
                        removed_dynamic_objects.clone(),
                        map_id,
                    ));
                }
                for guid in &removed_dynamic_objects {
                    self.client_visible_guids_like_cpp.remove(guid);
                }
                self.client_visible_guids_like_cpp
                    .extend(new_visible_dynamic_objects.iter().copied());
            }

            self.last_visibility_pos = Some(pos);
            debug!(
                "Visibility updated at ({:.1}, {:.1}): {} creatures / {} GOs in range",
                pos.x,
                pos.y,
                self.client_visible_guids_like_cpp
                    .iter()
                    .filter(|guid| guid.is_any_type_creature())
                    .count(),
                self.client_visible_guids_like_cpp
                    .iter()
                    .filter(|guid| guid.is_game_object())
                    .count()
            );
            return;
        }

        // ── CREATURES ───────────────────────────────────────────────────
        let world_db = match self.world_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };
        let mut stmt = world_db.prepare(WorldStatements::SEL_CREATURES_IN_RANGE);
        stmt.set_u16(0, map_id);
        stmt.set_f32(1, x_min);
        stmt.set_f32(2, x_max);
        stmt.set_f32(3, y_min);
        stmt.set_f32(4, y_max);

        let cr =
            match tokio::time::timeout(std::time::Duration::from_secs(5), world_db.query(&stmt))
                .await
            {
                Ok(Ok(r)) => r,
                _ => return,
            };

        let mut new_visible_creatures: HashSet<ObjectGuid> = HashSet::new();
        let mut new_creature_blocks: Vec<UpdateBlock> = Vec::new();

        if !cr.is_empty() {
            let mut cr = cr;
            loop {
                let spawn_guid: u64 = cr
                    .try_read::<i64>(0)
                    .map(|v| v as u64)
                    .or_else(|| cr.try_read::<u64>(0))
                    .unwrap_or(0);
                let entry: u32 = cr.try_read(1).unwrap_or(0);
                let pos_x: f32 = cr.try_read(2).unwrap_or(0.0);
                let pos_y: f32 = cr.try_read(3).unwrap_or(0.0);
                let pos_z: f32 = cr.try_read(4).unwrap_or(0.0);
                let orientation: f32 = cr.try_read(5).unwrap_or(0.0);
                let cur_health: u32 = cr.try_read(6).unwrap_or(100);
                let model_id: u32 = cr.try_read(8).unwrap_or(0);
                let min_level: u8 = cr.try_read::<Option<u8>>(9).flatten().unwrap_or(1);
                let faction: i32 = cr.try_read::<u16>(11).unwrap_or(35) as i32;
                let npc_flags: u64 = cr
                    .try_read::<i64>(12)
                    .map(|v| v as u64)
                    .or_else(|| cr.try_read::<u64>(12))
                    .unwrap_or(0);
                let unit_flags: u32 = cr.try_read(13).unwrap_or(0);
                let unit_flags2: u32 = cr.try_read(14).unwrap_or(0);
                let unit_flags3: u32 = cr.try_read(15).unwrap_or(0);
                let speed_walk: f32 = cr.try_read(16).unwrap_or(1.0);
                let speed_run: f32 = cr.try_read(17).unwrap_or(1.14286);
                let scale: f32 = cr.try_read(18).unwrap_or(1.0);
                let unit_class: u8 = cr.try_read(19).unwrap_or(1);
                let flags_extra: u32 = cr.try_read(20).unwrap_or(0);
                let base_attack_time: u32 = cr.try_read(21).unwrap_or(2000);
                let template_display_id: u32 =
                    cr.try_read::<Option<u32>>(23).flatten().unwrap_or(0);
                let loot_id: u32 = cr.try_read::<Option<u32>>(24).flatten().unwrap_or(0);
                let skin_loot_id: u32 = cr.try_read::<Option<u32>>(25).flatten().unwrap_or(0);
                let gold_min: u32 = cr.try_read::<Option<u32>>(26).flatten().unwrap_or(0);
                let gold_max: u32 = cr.try_read::<Option<u32>>(27).flatten().unwrap_or(0);
                let phase_use_flags: u8 = cr
                    .try_read::<u8>(28)
                    .or_else(|| cr.try_read::<i16>(28).map(|value| value.max(0) as u8))
                    .unwrap_or(0);
                let phase_id: u16 = cr
                    .try_read::<u16>(29)
                    .or_else(|| cr.try_read::<i32>(29).map(|value| value.max(0) as u16))
                    .unwrap_or(0);
                let phase_group_id: u32 = cr
                    .try_read::<u32>(30)
                    .or_else(|| cr.try_read::<i32>(30).map(|value| value.max(0) as u32))
                    .unwrap_or(0);
                let terrain_swap_map: i32 = cr.try_read(31).unwrap_or(-1);
                let ground_movement_type: u8 = cr
                    .try_read::<Option<u8>>(32)
                    .flatten()
                    .or_else(|| cr.try_read::<u8>(32))
                    .or_else(|| cr.try_read::<i16>(32).map(|value| value.max(0) as u8))
                    .unwrap_or(wow_constants::CreatureGroundMovementType::Run as u8);
                let swim_allowed: bool = cr
                    .try_read::<Option<u8>>(33)
                    .flatten()
                    .or_else(|| cr.try_read::<u8>(33))
                    .or_else(|| cr.try_read::<i16>(33).map(|value| value.max(0) as u8))
                    .unwrap_or(1)
                    != 0;
                let flight_movement_type: u8 = cr
                    .try_read::<Option<u8>>(34)
                    .flatten()
                    .or_else(|| cr.try_read::<u8>(34))
                    .or_else(|| cr.try_read::<i16>(34).map(|value| value.max(0) as u8))
                    .unwrap_or(0);
                let default_movement_type = cr
                    .try_read::<Option<u8>>(CREATURE_SPAWN_EFFECTIVE_MOVEMENT_TYPE_COLUMN)
                    .flatten()
                    .or_else(|| cr.try_read::<u8>(CREATURE_SPAWN_EFFECTIVE_MOVEMENT_TYPE_COLUMN))
                    .or_else(|| {
                        cr.try_read::<i16>(CREATURE_SPAWN_EFFECTIVE_MOVEMENT_TYPE_COLUMN)
                            .map(|value| value.max(0) as u8)
                    })
                    .map(creature_movement_generator_type_from_db_like_cpp)
                    .unwrap_or(MovementGeneratorType::Idle);
                let waypoint_path_id: u32 = cr
                    .try_read::<Option<u32>>(CREATURE_SPAWN_WAYPOINT_PATH_ID_COLUMN)
                    .flatten()
                    .or_else(|| cr.try_read::<u32>(CREATURE_SPAWN_WAYPOINT_PATH_ID_COLUMN))
                    .or_else(|| {
                        cr.try_read::<i64>(CREATURE_SPAWN_WAYPOINT_PATH_ID_COLUMN)
                            .map(|value| value.max(0) as u32)
                    })
                    .unwrap_or(0);

                let display_id = if model_id > 0 {
                    model_id
                } else if template_display_id > 0 {
                    template_display_id
                } else {
                    if !cr.next_row() {
                        break;
                    }
                    continue;
                };

                let (target_phase_shift, _) = self.db_spawn_phase_shift_like_cpp(
                    map_id,
                    phase_use_flags,
                    phase_id,
                    phase_group_id,
                    terrain_swap_map,
                );
                if !self.can_see_phase_shift_like_cpp(&target_phase_shift) {
                    if !cr.next_row() {
                        break;
                    }
                    continue;
                }

                let health = if cur_health > 0 {
                    cur_health as i64
                } else {
                    100
                };

                let guid = ObjectGuid::create_world_object(
                    HighGuid::Creature,
                    0,
                    realm_id,
                    map_id,
                    1,
                    entry,
                    spawn_guid as i64,
                );
                new_visible_creatures.insert(guid);

                // Only create a new block if this creature isn't already visible.
                if !self.client_visible_guids_like_cpp.contains(&guid) {
                    let creature_pos = Position::new(pos_x, pos_y, pos_z, orientation);
                    let create_data = CreatureCreateData {
                        guid,
                        entry,
                        display_id,
                        native_display_id: display_id,
                        health,
                        max_health: health,
                        level: min_level,
                        faction_template: faction,
                        npc_flags,
                        unit_flags,
                        unit_flags2,
                        unit_flags3,
                        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
                        scale,
                        unit_class,
                        base_attack_time,
                        ranged_attack_time: base_attack_time,
                        zone_id: 0,
                        speed_walk_rate: speed_walk,
                        speed_run_rate: speed_run,
                        ai_anim_kit_id: 0,
                        movement_anim_kit_id: 0,
                        melee_anim_kit_id: 0,
                    };

                    // Register in AI tracker
                    let aggro_radius = self.creature_aggro_radius_for_faction_template_like_cpp(
                        faction.max(0) as u32,
                        15.0,
                    );
                    let min_dmg = (min_level as u32).saturating_sub(1) * 3 + 5;
                    let max_dmg = min_dmg + min_dmg / 2;
                    self.register_world_creature_with_flags_extra_movement_and_default_motion_like_cpp(
                        map_id,
                        creature_pos,
                        create_data.clone(),
                        min_dmg,
                        max_dmg,
                        aggro_radius,
                        loot_id,
                        skin_loot_id,
                        gold_min,
                        gold_max,
                        None,
                        0,
                        phase_use_flags,
                        phase_id,
                        phase_group_id,
                        terrain_swap_map,
                        flags_extra,
                        ground_movement_type,
                        swim_allowed,
                        flight_movement_type,
                        default_movement_type,
                        waypoint_path_id,
                    );

                    let mut viewer_create_data = create_data.clone();
                    viewer_create_data.npc_flags = self
                        .represented_viewer_dependent_creature_npc_flags_like_cpp(
                            guid,
                            viewer_create_data.npc_flags,
                        );
                    new_creature_blocks.push(UpdateObject::create_creature_block(
                        viewer_create_data,
                        &creature_pos,
                    ));
                }

                if !cr.next_row() {
                    break;
                }
            }
        }

        // Creatures that left range → out-of-range
        let removed_creatures: Vec<ObjectGuid> = self
            .client_visible_guids_like_cpp
            .iter()
            .filter(|g| g.is_any_type_creature() && !new_visible_creatures.contains(g))
            .cloned()
            .collect();

        if !new_creature_blocks.is_empty() {
            debug!(
                "Visibility update: {} new creatures",
                new_creature_blocks.len()
            );
            self.send_packet(&UpdateObject::create_creatures(new_creature_blocks, map_id));
        }
        if !removed_creatures.is_empty() {
            debug!(
                "Visibility update: {} creatures out of range",
                removed_creatures.len()
            );
            self.send_packet(&UpdateObject::out_of_range_objects(
                removed_creatures,
                map_id,
            ));
        }
        self.client_visible_guids_like_cpp
            .retain(|guid| !guid.is_any_type_creature());
        self.client_visible_guids_like_cpp
            .extend(new_visible_creatures.iter().copied());

        // ── GAME OBJECTS ────────────────────────────────────────────────
        let mut go_stmt = world_db.prepare(WorldStatements::SEL_GAMEOBJECTS_IN_RANGE);
        go_stmt.set_u16(0, map_id);
        go_stmt.set_f32(1, x_min);
        go_stmt.set_f32(2, x_max);
        go_stmt.set_f32(3, y_min);
        go_stmt.set_f32(4, y_max);

        let go_result =
            match tokio::time::timeout(std::time::Duration::from_secs(5), world_db.query(&go_stmt))
                .await
            {
                Ok(Ok(r)) => r,
                _ => {
                    self.last_visibility_pos = Some(pos);
                    return;
                }
            };

        let mut new_visible_gos: HashSet<ObjectGuid> = HashSet::new();
        let mut new_go_blocks: Vec<UpdateBlock> = Vec::new();

        if !go_result.is_empty() {
            let mut go_result = go_result;
            loop {
                let spawn_guid: u64 = go_result
                    .try_read::<i64>(0)
                    .map(|v| v as u64)
                    .or_else(|| go_result.try_read::<u64>(0))
                    .unwrap_or(0);
                let entry: u32 = go_result.try_read(1).unwrap_or(0);
                let pos_x: f32 = go_result.try_read(2).unwrap_or(0.0);
                let pos_y: f32 = go_result.try_read(3).unwrap_or(0.0);
                let pos_z: f32 = go_result.try_read(4).unwrap_or(0.0);
                let orientation: f32 = go_result.try_read(5).unwrap_or(0.0);
                let rot0: f32 = go_result.try_read(6).unwrap_or(0.0);
                let rot1: f32 = go_result.try_read(7).unwrap_or(0.0);
                let rot2: f32 = go_result.try_read(8).unwrap_or(0.0);
                let rot3: f32 = go_result.try_read(9).unwrap_or(0.0);
                let anim_progress: u8 = go_result.try_read(10).unwrap_or(255);
                let state: i8 = go_result.try_read::<u8>(11).unwrap_or(1) as i8;
                let go_type: u8 = go_result.try_read(12).unwrap_or(0);
                let display_id: u32 = go_result.try_read(13).unwrap_or(0);
                let scale: f32 = go_result.try_read(15).unwrap_or(1.0);
                let mut template_data = [0_u32; MAX_GAMEOBJECT_DATA];
                for (index, value) in template_data.iter_mut().enumerate() {
                    *value = go_result
                        .try_read::<i32>(GO_SPAWN_TEMPLATE_DATA_START + index)
                        .and_then(|raw| u32::try_from(raw).ok())
                        .unwrap_or(0);
                }
                let data2 = template_data[2];
                let data3 = template_data[3];
                let template = GameObjectTemplateData::new(u32::from(go_type), template_data);
                let phase_use_flags: u8 = go_result
                    .try_read::<u8>(GO_SPAWN_PHASE_USE_FLAGS_COLUMN)
                    .or_else(|| {
                        go_result
                            .try_read::<i16>(GO_SPAWN_PHASE_USE_FLAGS_COLUMN)
                            .map(|value| value.max(0) as u8)
                    })
                    .unwrap_or(0);
                let phase_id: u16 = go_result
                    .try_read::<u16>(GO_SPAWN_PHASE_ID_COLUMN)
                    .or_else(|| {
                        go_result
                            .try_read::<i32>(GO_SPAWN_PHASE_ID_COLUMN)
                            .map(|value| value.max(0) as u16)
                    })
                    .unwrap_or(0);
                let phase_group_id: u32 = go_result
                    .try_read::<u32>(GO_SPAWN_PHASE_GROUP_COLUMN)
                    .or_else(|| {
                        go_result
                            .try_read::<i32>(GO_SPAWN_PHASE_GROUP_COLUMN)
                            .map(|value| value.max(0) as u32)
                    })
                    .unwrap_or(0);
                let terrain_swap_map: i32 = go_result
                    .try_read(GO_SPAWN_TERRAIN_SWAP_MAP_COLUMN)
                    .unwrap_or(-1);
                let effective_flags: u32 = go_result
                    .try_read::<u32>(GO_SPAWN_EFFECTIVE_FLAGS_COLUMN)
                    .or_else(|| {
                        go_result
                            .try_read::<i64>(GO_SPAWN_EFFECTIVE_FLAGS_COLUMN)
                            .and_then(|value| u32::try_from(value).ok())
                    })
                    .unwrap_or(0);
                let effective_faction: u32 = go_result
                    .try_read::<u32>(GO_SPAWN_EFFECTIVE_FACTION_COLUMN)
                    .or_else(|| {
                        go_result
                            .try_read::<i64>(GO_SPAWN_EFFECTIVE_FACTION_COLUMN)
                            .and_then(|value| u32::try_from(value).ok())
                    })
                    .unwrap_or(0);
                let override_source_known = go_result
                    .try_read::<u8>(GO_SPAWN_OVERRIDE_SOURCE_KNOWN_COLUMN)
                    .map(|value| value != 0)
                    .or_else(|| {
                        go_result
                            .try_read::<i64>(GO_SPAWN_OVERRIDE_SOURCE_KNOWN_COLUMN)
                            .map(|value| value != 0)
                    })
                    .unwrap_or(false);

                if display_id == 0 {
                    if !go_result.next_row() {
                        break;
                    }
                    continue;
                }

                let (target_phase_shift, _) = self.db_spawn_phase_shift_like_cpp(
                    map_id,
                    phase_use_flags,
                    phase_id,
                    phase_group_id,
                    terrain_swap_map,
                );
                if !self.can_see_phase_shift_like_cpp(&target_phase_shift) {
                    if !go_result.next_row() {
                        break;
                    }
                    continue;
                }

                let guid = ObjectGuid::create_world_object(
                    HighGuid::GameObject,
                    0,
                    realm_id,
                    map_id,
                    1,
                    entry,
                    spawn_guid as i64,
                );
                if self.represented_gameobject_is_per_player_despawned_like_cpp(guid) {
                    if !go_result.next_row() {
                        break;
                    }
                    continue;
                }
                new_visible_gos.insert(guid);
                self.record_represented_gameobject_db_phase_shift_like_cpp(
                    guid,
                    map_id,
                    phase_use_flags,
                    phase_id,
                    phase_group_id,
                    terrain_swap_map,
                );

                if !self.client_visible_guids_like_cpp.contains(&guid) {
                    let go_pos = Position::new(pos_x, pos_y, pos_z, orientation);
                    let dynamic_flags = self
                        .represented_gameobject_dynamic_flags_for_player_like_cpp(
                            entry,
                            &RepresentedGameObjectUseState {
                                go_type: Some(go_type),
                                go_state: represented_go_state_from_i8_like_cpp(state),
                                ..Default::default()
                            },
                        );
                    let create_data = GameObjectCreateData {
                        guid,
                        entry,
                        dynamic_flags,
                        display_id,
                        go_type,
                        position: go_pos,
                        rotation: [rot0, rot1, rot2, rot3],
                        anim_progress,
                        state,
                        created_by: ObjectGuid::EMPTY,
                        faction_template: effective_faction as i32,
                        gameobject_flags: effective_flags,
                        scale,
                    };
                    new_go_blocks.push(UpdateObject::create_gameobject_block(create_data));
                    self.record_represented_gameobject_runtime_state_like_cpp(
                        map_id, guid, entry, go_pos, go_type,
                    );
                } else {
                    self.record_represented_gameobject_runtime_state_like_cpp(
                        map_id,
                        guid,
                        entry,
                        Position::new(pos_x, pos_y, pos_z, orientation),
                        go_type,
                    );
                }
                self.record_represented_gameobject_override_like_cpp(
                    guid,
                    effective_flags,
                    effective_faction,
                    override_source_known,
                );
                if u32::from(go_type) == GAMEOBJECT_TYPE_FISHING_HOLE {
                    let max_opens = if data2 <= data3 {
                        rand::thread_rng().gen_range(data2..=data3)
                    } else {
                        data2
                    };
                    self.record_represented_fishing_hole_max_opens_like_cpp(guid, max_opens);
                    self.record_represented_fishing_hole_radius_like_cpp(guid, template_data[0]);
                }
                self.record_represented_gameobject_interact_radius_override_like_cpp(
                    guid,
                    template.get_interact_radius_override_like_cpp(),
                );
                self.record_represented_gameobject_lock_id_like_cpp(
                    guid,
                    template.get_lock_id_like_cpp(),
                );
                self.record_represented_gameobject_display_model_like_cpp(
                    guid,
                    display_id,
                    scale,
                    [rot0, rot1, rot2, rot3],
                );
                self.record_represented_gameobject_anim_progress_like_cpp(guid, anim_progress);

                if !go_result.next_row() {
                    break;
                }
            }
        }

        let removed_gos: Vec<ObjectGuid> = self
            .client_visible_guids_like_cpp
            .iter()
            .filter(|g| g.is_game_object() && !new_visible_gos.contains(g))
            .cloned()
            .collect();
        for guid in &removed_gos {
            self.represented_gameobject_phase_shifts.remove(guid);
        }

        if !new_go_blocks.is_empty() {
            debug!(
                "Visibility update: {} new game objects",
                new_go_blocks.len()
            );
            self.send_packet(&UpdateObject::create_world_objects(new_go_blocks, map_id));
        }
        if !removed_gos.is_empty() {
            debug!(
                "Visibility update: {} game objects out of range",
                removed_gos.len()
            );
            self.send_packet(&UpdateObject::out_of_range_objects(
                removed_gos.clone(),
                map_id,
            ));
        }
        for guid in &removed_gos {
            self.client_visible_guids_like_cpp.remove(guid);
        }
        self.client_visible_guids_like_cpp
            .extend(new_visible_gos.iter().copied());

        // ── Update position marker ──────────────────────────────────────
        self.last_visibility_pos = Some(pos);
        debug!(
            "Visibility updated at ({:.1}, {:.1}): {} creatures / {} GOs in range",
            pos.x,
            pos.y,
            self.client_visible_guids_like_cpp
                .iter()
                .filter(|guid| guid.is_any_type_creature())
                .count(),
            self.client_visible_guids_like_cpp
                .iter()
                .filter(|guid| guid.is_game_object())
                .count()
        );
    }

    /// Handle CMSG_QUERY_CREATURE — client requests creature template data.
    ///
    /// The client sends this automatically after receiving an UpdateObject with
    /// unknown creature entries. Without a response, NPC names don't display
    /// and interaction menus don't work.
    pub async fn handle_query_creature(&mut self, query: QueryCreature) {
        // If already responded, skip — client caches locally after first response
        if self.creature_query_cache.contains(&query.creature_id) {
            return;
        }
        self.creature_query_cache.insert(query.creature_id);

        let world_db = match self.world_db() {
            Some(db) => Arc::clone(db),
            None => {
                self.send_packet(&QueryCreatureResponse {
                    creature_id: query.creature_id,
                    allow: false,
                    stats: None,
                });
                return;
            }
        };

        // Query creature template
        let mut stmt = world_db.prepare(WorldStatements::SEL_CREATURE_QUERY_RESPONSE);
        stmt.set_u32(0, query.creature_id);

        let result = match world_db.query(&stmt).await {
            Ok(r) => r,
            Err(e) => {
                debug!(
                    "Failed to query creature template {}: {e}",
                    query.creature_id
                );
                self.send_packet(&QueryCreatureResponse {
                    creature_id: query.creature_id,
                    allow: false,
                    stats: None,
                });
                return;
            }
        };

        if result.is_empty() {
            self.send_packet(&QueryCreatureResponse {
                creature_id: query.creature_id,
                allow: false,
                stats: None,
            });
            return;
        }

        // Parse template fields
        let name: String = result.read_string(1);
        let _female_name: String = result.read_string(2);
        let subname: String = result.read_string(3);
        let title_alt: String = result.read_string(4);
        let icon_name: String = result.read_string(5);
        let creature_type: i32 = result.try_read(6).unwrap_or(0);
        let creature_family: i32 = result.try_read(7).unwrap_or(0);
        let classification: i32 = result.try_read(8).unwrap_or(0);
        let kill_credit1: i32 = result.try_read(9).unwrap_or(0);
        let kill_credit2: i32 = result.try_read(10).unwrap_or(0);
        let civilian: bool = result.try_read::<u8>(11).unwrap_or(0) != 0;
        let racial_leader: bool = result.try_read::<u8>(12).unwrap_or(0) != 0;
        let movement_id: i32 = result.try_read(13).unwrap_or(0);
        let required_expansion: i32 = result.try_read(14).unwrap_or(0);
        let vignette_id: i32 = result.try_read(15).unwrap_or(0);
        let unit_class: i32 = result.try_read::<u8>(16).unwrap_or(1) as i32;
        let widget_set_id: i32 = result.try_read(17).unwrap_or(0);
        let widget_set_unit_condition_id: i32 = result.try_read(18).unwrap_or(0);
        // LEFT JOIN nullable fields from creature_template_difficulty
        let hp_multi: f32 = result.try_read::<Option<f32>>(19).flatten().unwrap_or(1.0);
        let energy_multi: f32 = result.try_read::<Option<f32>>(20).flatten().unwrap_or(1.0);
        let creature_difficulty_id: i32 = result.try_read::<Option<i32>>(21).flatten().unwrap_or(0);
        let type_flags: u32 = result.try_read::<Option<u32>>(22).flatten().unwrap_or(0);
        let type_flags2: u32 = result.try_read::<Option<u32>>(23).flatten().unwrap_or(0);

        // Override name/subname/title_alt with localized versions when not English
        let locale = &self.locale;
        let (name, subname, title_alt) = if !locale.is_empty() && locale != "enUS" {
            let mut loc_stmt = world_db.prepare(WorldStatements::SEL_CREATURE_TEMPLATE_LOCALE);
            loc_stmt.set_u32(0, query.creature_id);
            loc_stmt.set_string(1, locale);
            match world_db.query(&loc_stmt).await {
                Ok(r) if !r.is_empty() => {
                    let loc_name: String = r.read_string(0);
                    // col 1 = NameAlt (female name)
                    let loc_subname: String = r.read_string(2);
                    let loc_title_alt: String = r.read_string(3);
                    (
                        if loc_name.is_empty() { name } else { loc_name },
                        if loc_subname.is_empty() {
                            subname
                        } else {
                            loc_subname
                        },
                        if loc_title_alt.is_empty() {
                            title_alt
                        } else {
                            loc_title_alt
                        },
                    )
                }
                Ok(_) => (name, subname, title_alt),
                Err(e) => {
                    warn!(
                        "Failed to query creature locale for {}: {e}",
                        query.creature_id
                    );
                    (name, subname, title_alt)
                }
            }
        } else {
            (name, subname, title_alt)
        };

        // Query display models
        let mut display_stmt = world_db.prepare(WorldStatements::SEL_CREATURE_DISPLAY_MODELS);
        display_stmt.set_u32(0, query.creature_id);

        let mut displays = Vec::new();
        let mut total_probability: f32 = 0.0;

        if let Ok(disp_result) = world_db.query(&display_stmt).await {
            if !disp_result.is_empty() {
                let mut disp_result = disp_result;
                loop {
                    let display_id: u32 = disp_result.try_read(0).unwrap_or(0);
                    let scale: f32 = disp_result.try_read(1).unwrap_or(1.0);
                    let probability: f32 = disp_result.try_read(2).unwrap_or(1.0);
                    total_probability += probability;
                    displays.push(CreatureXDisplay {
                        creature_display_id: display_id,
                        scale,
                        probability,
                    });
                    if !disp_result.next_row() {
                        break;
                    }
                }
            }
        }

        let mut names: [String; 4] = Default::default();
        names[0] = name;

        let stats = CreatureStats {
            title: subname,
            title_alt,
            cursor_name: icon_name,
            civilian,
            leader: racial_leader,
            names,
            name_alts: Default::default(),
            flags: [type_flags, type_flags2],
            creature_type,
            creature_family,
            classification,
            proxy_creature_ids: [kill_credit1, kill_credit2],
            display: CreatureDisplayStats {
                displays,
                total_probability,
            },
            hp_multi,
            energy_multi,
            quest_items: Vec::new(),
            creature_movement_info_id: movement_id,
            health_scaling_expansion: 0,
            required_expansion,
            vignette_id,
            unit_class,
            creature_difficulty_id,
            widget_set_id,
            widget_set_unit_condition_id,
        };

        self.send_packet(&QueryCreatureResponse {
            creature_id: query.creature_id,
            allow: true,
            stats: Some(stats),
        });
    }

    /// Handle CMSG_QUERY_GAME_OBJECT — client requests gameobject template data.
    pub async fn handle_query_game_object(
        &mut self,
        query: wow_packet::packets::query::QueryGameObject,
    ) {
        let world_db = match self.world_db() {
            Some(db) => Arc::clone(db),
            None => {
                self.send_packet(&QueryGameObjectResponse {
                    game_object_id: query.game_object_id,
                    guid: query.guid,
                    allow: false,
                    stats: None,
                });
                return;
            }
        };

        let mut stmt = world_db.prepare(WorldStatements::SEL_GAMEOBJECT_TEMPLATE_BY_ENTRY);
        stmt.set_u32(0, query.game_object_id);

        let result = match world_db.query(&stmt).await {
            Ok(r) => r,
            Err(e) => {
                debug!(
                    "Failed to query gameobject template {}: {e}",
                    query.game_object_id
                );
                self.send_packet(&QueryGameObjectResponse {
                    game_object_id: query.game_object_id,
                    guid: query.guid,
                    allow: false,
                    stats: None,
                });
                return;
            }
        };

        if result.is_empty() {
            self.send_packet(&QueryGameObjectResponse {
                game_object_id: query.game_object_id,
                guid: query.guid,
                allow: false,
                stats: None,
            });
            return;
        }

        let go_type: i32 = result.try_read(1).unwrap_or(0);
        let display_id: i32 = result.try_read(2).unwrap_or(0);
        let mut name: String = result.read_string(3);
        let icon_name: String = result.read_string(4);
        let mut cast_bar_caption: String = result.read_string(5);
        let mut unk_string: String = result.read_string(6);
        let size: f32 = result.try_read(7).unwrap_or(1.0);

        // Data0..Data34 at columns 8..42, matching C++ MAX_GAMEOBJECT_DATA.
        let mut data = [0i32; 35];
        for i in 0..35 {
            data[i] = result.try_read(8 + i).unwrap_or(0);
        }
        let content_tuning_id = result.try_read(43).unwrap_or(0);

        let locale = &self.locale;
        if !locale.is_empty() && locale != "enUS" {
            let mut loc_stmt = world_db.prepare(WorldStatements::SEL_GAMEOBJECT_TEMPLATE_LOCALE);
            loc_stmt.set_u32(0, query.game_object_id);
            loc_stmt.set_string(1, locale);
            match world_db.query(&loc_stmt).await {
                Ok(r) if !r.is_empty() => {
                    let loc_name: String = r.read_string(0);
                    let loc_cast_bar_caption: String = r.read_string(1);
                    let loc_unk_string: String = r.read_string(2);
                    if !loc_name.is_empty() {
                        name = loc_name;
                    }
                    if !loc_cast_bar_caption.is_empty() {
                        cast_bar_caption = loc_cast_bar_caption;
                    }
                    if !loc_unk_string.is_empty() {
                        unk_string = loc_unk_string;
                    }
                }
                Ok(_) => {}
                Err(e) => debug!(
                    "Failed to query gameobject locale {} {}: {e}",
                    query.game_object_id, locale
                ),
            }
        }

        let mut quest_items = Vec::new();
        let mut quest_item_stmt = world_db.prepare(WorldStatements::SEL_GAMEOBJECT_QUEST_ITEMS);
        quest_item_stmt.set_u32(0, query.game_object_id);
        match world_db.query(&quest_item_stmt).await {
            Ok(mut quest_item_result) if !quest_item_result.is_empty() => loop {
                let item_id: i32 = quest_item_result.try_read::<i32>(0).unwrap_or(0);
                if item_id > 0 {
                    quest_items.push(item_id);
                }
                if !quest_item_result.next_row() {
                    break;
                }
            },
            Ok(_) => {}
            Err(e) => debug!(
                "Failed to query gameobject quest items {}: {e}",
                query.game_object_id
            ),
        }

        let mut names: [String; 4] = Default::default();
        names[0] = name;

        let stats = GameObjectStats {
            names,
            icon_name,
            cast_bar_caption,
            unk_string,
            go_type,
            display_id,
            data,
            size,
            quest_items,
            content_tuning_id,
        };

        self.send_packet(&QueryGameObjectResponse {
            game_object_id: query.game_object_id,
            guid: query.guid,
            allow: true,
            stats: Some(stats),
        });
    }

    pub async fn handle_query_page_text(&mut self, query: QueryPageText) {
        let world_db = match self.world_db() {
            Some(db) => Arc::clone(db),
            None => {
                self.send_packet(&QueryPageTextResponse {
                    page_text_id: query.page_text_id,
                    allow: false,
                    pages: Vec::new(),
                });
                return;
            }
        };

        let mut pages = Vec::new();
        let mut page_id = query.page_text_id;
        let mut visited = HashSet::new();

        while page_id != 0 && visited.insert(page_id) && pages.len() < 100 {
            let mut stmt = world_db.prepare(WorldStatements::SEL_PAGE_TEXT);
            stmt.set_u32(0, page_id);
            let result = match world_db.query(&stmt).await {
                Ok(result) => result,
                Err(e) => {
                    debug!("Failed to query page text {page_id}: {e}");
                    break;
                }
            };
            if result.is_empty() {
                break;
            }

            let id: u32 = result.try_read(0).unwrap_or(page_id);
            let mut text: String = result.read_string(1);
            let next_page_id: u32 = result.try_read(2).unwrap_or(0);
            let player_condition_id: i32 = result.try_read(3).unwrap_or(0);
            let flags: u8 = result.try_read(4).unwrap_or(0);

            let locale = &self.locale;
            if !locale.is_empty() && locale != "enUS" {
                let mut loc_stmt = world_db.prepare(WorldStatements::SEL_PAGE_TEXT_LOCALE);
                loc_stmt.set_u32(0, id);
                loc_stmt.set_string(1, locale);
                match world_db.query(&loc_stmt).await {
                    Ok(locale_result) if !locale_result.is_empty() => {
                        let locale_text: String = locale_result.read_string(0);
                        if !locale_text.is_empty() {
                            text = locale_text;
                        }
                    }
                    Ok(_) => {}
                    Err(e) => debug!("Failed to query page text locale {id} {locale}: {e}"),
                }
            }

            pages.push(PageTextInfo {
                id,
                next_page_id,
                player_condition_id,
                flags,
                text,
            });
            page_id = next_page_id;
        }

        self.send_packet(&QueryPageTextResponse {
            page_text_id: query.page_text_id,
            allow: !pages.is_empty(),
            pages,
        });
    }

    /// Send nearby gameobjects to the client as UpdateObject packets.
    pub async fn send_nearby_gameobjects(
        &mut self,
        map_id: u16,
        position: &Position,
        _zone_id: u32,
    ) {
        const VISIBILITY_RANGE: f32 = 800.0;

        if let Some(gameobjects) =
            self.visible_gameobjects_from_canonical_map_like_cpp(map_id, position, VISIBILITY_RANGE)
        {
            if gameobjects.is_empty() {
                self.client_visible_guids_like_cpp
                    .retain(|guid| !guid.is_game_object());
                return;
            }

            let go_guids: HashSet<_> = gameobjects.iter().map(|go| go.guid).collect();
            let blocks = gameobjects
                .into_iter()
                .map(UpdateObject::create_gameobject_block)
                .collect::<Vec<_>>();
            let count = blocks.len();
            self.client_visible_guids_like_cpp
                .retain(|guid| !guid.is_game_object());
            self.client_visible_guids_like_cpp
                .extend(go_guids.iter().copied());
            self.send_packet(&UpdateObject::create_world_objects(blocks, map_id));
            debug!(
                "Sent {} canonical gameobjects to account {} on map {}",
                count, self.account_id, map_id
            );
            return;
        }

        let world_db = match self.world_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let x_min = position.x - VISIBILITY_RANGE;
        let x_max = position.x + VISIBILITY_RANGE;
        let y_min = position.y - VISIBILITY_RANGE;
        let y_max = position.y + VISIBILITY_RANGE;

        let mut stmt = world_db.prepare(WorldStatements::SEL_GAMEOBJECTS_IN_RANGE);
        stmt.set_u16(0, map_id);
        stmt.set_f32(1, x_min);
        stmt.set_f32(2, x_max);
        stmt.set_f32(3, y_min);
        stmt.set_f32(4, y_max);

        let result =
            match tokio::time::timeout(std::time::Duration::from_secs(5), world_db.query(&stmt))
                .await
            {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    warn!("Failed to query gameobjects for map {map_id}: {e}");
                    return;
                }
                Err(_) => {
                    warn!("Gameobject query timed out for map {map_id}");
                    return;
                }
            };

        if result.is_empty() {
            return;
        }

        let realm_id = self.realm_id();
        let mut blocks = Vec::new();
        let mut go_guids: Vec<wow_core::ObjectGuid> = Vec::new();
        let mut result = result;

        loop {
            let spawn_guid: u64 = result
                .try_read::<i64>(0)
                .map(|v| v as u64)
                .or_else(|| result.try_read::<u64>(0))
                .unwrap_or(0);
            let entry: u32 = result.try_read(1).unwrap_or(0);
            let pos_x: f32 = result.try_read(2).unwrap_or(0.0);
            let pos_y: f32 = result.try_read(3).unwrap_or(0.0);
            let pos_z: f32 = result.try_read(4).unwrap_or(0.0);
            let orientation: f32 = result.try_read(5).unwrap_or(0.0);
            let rot0: f32 = result.try_read(6).unwrap_or(0.0);
            let rot1: f32 = result.try_read(7).unwrap_or(0.0);
            let rot2: f32 = result.try_read(8).unwrap_or(0.0);
            let rot3: f32 = result.try_read(9).unwrap_or(0.0);
            let anim_progress: u8 = result.try_read(10).unwrap_or(0);
            let state: i8 = result.try_read::<u8>(11).unwrap_or(1) as i8;
            let go_type: u8 = result.try_read::<u8>(12).unwrap_or(0);
            let display_id: u32 = result.try_read(13).unwrap_or(0);
            let _name: String = result.read_string(14);
            let scale: f32 = result.try_read(15).unwrap_or(1.0);
            let mut template_data = [0_u32; MAX_GAMEOBJECT_DATA];
            for (index, value) in template_data.iter_mut().enumerate() {
                *value = result
                    .try_read::<i32>(GO_SPAWN_TEMPLATE_DATA_START + index)
                    .and_then(|raw| u32::try_from(raw).ok())
                    .unwrap_or(0);
            }
            let data2 = template_data[2];
            let data3 = template_data[3];
            let template = GameObjectTemplateData::new(u32::from(go_type), template_data);
            let phase_use_flags: u8 = result
                .try_read::<u8>(GO_SPAWN_PHASE_USE_FLAGS_COLUMN)
                .or_else(|| {
                    result
                        .try_read::<i16>(GO_SPAWN_PHASE_USE_FLAGS_COLUMN)
                        .map(|value| value.max(0) as u8)
                })
                .unwrap_or(0);
            let phase_id: u16 = result
                .try_read::<u16>(GO_SPAWN_PHASE_ID_COLUMN)
                .or_else(|| {
                    result
                        .try_read::<i32>(GO_SPAWN_PHASE_ID_COLUMN)
                        .map(|value| value.max(0) as u16)
                })
                .unwrap_or(0);
            let phase_group_id: u32 = result
                .try_read::<u32>(GO_SPAWN_PHASE_GROUP_COLUMN)
                .or_else(|| {
                    result
                        .try_read::<i32>(GO_SPAWN_PHASE_GROUP_COLUMN)
                        .map(|value| value.max(0) as u32)
                })
                .unwrap_or(0);
            let terrain_swap_map: i32 = result
                .try_read(GO_SPAWN_TERRAIN_SWAP_MAP_COLUMN)
                .unwrap_or(-1);
            let effective_flags: u32 = result
                .try_read::<u32>(GO_SPAWN_EFFECTIVE_FLAGS_COLUMN)
                .or_else(|| {
                    result
                        .try_read::<i64>(GO_SPAWN_EFFECTIVE_FLAGS_COLUMN)
                        .and_then(|value| u32::try_from(value).ok())
                })
                .unwrap_or(0);
            let effective_faction: u32 = result
                .try_read::<u32>(GO_SPAWN_EFFECTIVE_FACTION_COLUMN)
                .or_else(|| {
                    result
                        .try_read::<i64>(GO_SPAWN_EFFECTIVE_FACTION_COLUMN)
                        .and_then(|value| u32::try_from(value).ok())
                })
                .unwrap_or(0);
            let override_source_known = result
                .try_read::<u8>(GO_SPAWN_OVERRIDE_SOURCE_KNOWN_COLUMN)
                .map(|value| value != 0)
                .or_else(|| {
                    result
                        .try_read::<i64>(GO_SPAWN_OVERRIDE_SOURCE_KNOWN_COLUMN)
                        .map(|value| value != 0)
                })
                .unwrap_or(false);

            // Skip gameobjects with no display
            if display_id == 0 {
                if !result.next_row() {
                    break;
                }
                continue;
            }

            let (target_phase_shift, _) = self.db_spawn_phase_shift_like_cpp(
                map_id,
                phase_use_flags,
                phase_id,
                phase_group_id,
                terrain_swap_map,
            );
            if !self.can_see_phase_shift_like_cpp(&target_phase_shift) {
                if !result.next_row() {
                    break;
                }
                continue;
            }

            let guid = ObjectGuid::create_world_object(
                HighGuid::GameObject,
                0,
                realm_id,
                map_id,
                1,
                entry,
                spawn_guid as i64,
            );

            let go_pos = Position::new(pos_x, pos_y, pos_z, orientation);
            let dynamic_flags = self.represented_gameobject_dynamic_flags_for_player_like_cpp(
                entry,
                &RepresentedGameObjectUseState {
                    go_type: Some(go_type),
                    go_state: represented_go_state_from_i8_like_cpp(state),
                    ..Default::default()
                },
            );
            let create_data = GameObjectCreateData {
                guid,
                entry,
                dynamic_flags,
                display_id,
                go_type,
                position: go_pos,
                rotation: [rot0, rot1, rot2, rot3],
                anim_progress,
                state,
                created_by: ObjectGuid::EMPTY,
                faction_template: effective_faction as i32,
                gameobject_flags: effective_flags,
                scale,
            };

            blocks.push(UpdateObject::create_gameobject_block(create_data));
            go_guids.push(guid);
            self.record_represented_gameobject_db_phase_shift_like_cpp(
                guid,
                map_id,
                phase_use_flags,
                phase_id,
                phase_group_id,
                terrain_swap_map,
            );
            self.record_represented_gameobject_runtime_state_like_cpp(
                map_id, guid, entry, go_pos, go_type,
            );
            self.record_represented_gameobject_override_like_cpp(
                guid,
                effective_flags,
                effective_faction,
                override_source_known,
            );
            if u32::from(go_type) == GAMEOBJECT_TYPE_FISHING_HOLE {
                let max_opens = if data2 <= data3 {
                    rand::thread_rng().gen_range(data2..=data3)
                } else {
                    data2
                };
                self.record_represented_fishing_hole_max_opens_like_cpp(guid, max_opens);
                self.record_represented_fishing_hole_radius_like_cpp(guid, template_data[0]);
            }
            self.record_represented_gameobject_interact_radius_override_like_cpp(
                guid,
                template.get_interact_radius_override_like_cpp(),
            );
            self.record_represented_gameobject_lock_id_like_cpp(
                guid,
                template.get_lock_id_like_cpp(),
            );
            self.record_represented_gameobject_display_model_like_cpp(
                guid,
                display_id,
                scale,
                [rot0, rot1, rot2, rot3],
            );
            self.record_represented_gameobject_anim_progress_like_cpp(guid, anim_progress);

            if !result.next_row() {
                break;
            }
        }

        if blocks.is_empty() {
            return;
        }

        self.client_visible_guids_like_cpp
            .retain(|guid| !guid.is_game_object());
        self.client_visible_guids_like_cpp
            .extend(go_guids.iter().copied());
        let count = blocks.len();
        let update = UpdateObject::create_world_objects(blocks, map_id);
        self.send_packet(&update);
        debug!(
            "Sent {} gameobjects to account {} on map {}",
            count, self.account_id, map_id
        );
    }

    /// Handle CMSG_PING — respond with Pong containing the serial.
    pub async fn handle_ping(&mut self, ping: wow_packet::packets::auth::Ping) {
        trace!(
            "Ping: serial={}, latency={}ms for account {}",
            ping.serial, ping.latency, self.account_id
        );
        self.send_packet(&wow_packet::packets::auth::Pong {
            serial: ping.serial,
        });
    }

    /// Handle CMSG_GOSSIP_HELLO / TalkToGossip — player right-clicks an NPC.
    ///
    /// For now, we send an empty gossip message with a default NPC text.
    /// This allows the client to show the gossip window.
    /// Handle CMSG_QUERY_PLAYER_NAMES — client requests player name data.
    ///
    /// The client sends this after receiving UpdateObject for a player whose
    /// name isn't cached. Without a response, the player's nameplate is blank.
    pub async fn handle_query_player_names(&mut self, query: QueryPlayerNames) {
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => {
                // Send failure response for all queried players
                let players = query
                    .players
                    .iter()
                    .map(|guid| NameCacheLookupResult {
                        player: *guid,
                        result: 1, // Failure
                        data: None,
                    })
                    .collect();
                self.send_packet_realm(&QueryPlayerNamesResponse { players });
                return;
            }
        };

        let mut results = Vec::new();

        for guid in &query.players {
            let counter = guid.counter();

            let mut stmt = char_db.prepare(CharStatements::SEL_CHARACTER);
            stmt.set_u64(0, counter as u64);

            let db_result = match char_db.query(&stmt).await {
                Ok(r) => r,
                Err(_) => {
                    results.push(NameCacheLookupResult {
                        player: *guid,
                        result: 1,
                        data: None,
                    });
                    continue;
                }
            };

            if db_result.is_empty() {
                results.push(NameCacheLookupResult {
                    player: *guid,
                    result: 1,
                    data: None,
                });
                continue;
            }

            let name: String = db_result.read_string(2);
            let race: u8 = db_result.read(3);
            let class: u8 = db_result.read(4);
            let sex: u8 = db_result.read(5);
            let level: u8 = db_result.read(6);

            // Build account GUIDs (simplified — just use account_id)
            let account_id_val = self.account_id as i64;
            let account_guid = ObjectGuid::new((HighGuid::WowAccount as i64) << 58, account_id_val);
            let bnet_guid = ObjectGuid::new((HighGuid::BNetAccount as i64) << 58, account_id_val);

            // Use the session VRA (region << 24 | battlegroup << 16 | realmId)
            // to match what every other packet sends. The wrong formula caused
            // "Unknown Entity" because the client rejected the mismatched VRA.
            let vra = self.virtual_realm_address();

            results.push(NameCacheLookupResult {
                player: *guid,
                result: 0, // Success
                data: Some(PlayerGuidLookupData {
                    name,
                    race,
                    sex,
                    class,
                    level,
                    guid_actual: *guid,
                    account_id: account_guid,
                    bnet_account_id: bnet_guid,
                    virtual_realm_address: vra,
                    ..Default::default()
                }),
            });
        }

        debug!(
            "QueryPlayerNames: {} queries, {} found for account {}",
            query.players.len(),
            results.iter().filter(|r| r.result == 0).count(),
            self.account_id
        );
        self.send_packet_realm(&QueryPlayerNamesResponse { players: results });
    }

    pub fn handle_query_realm_name(&mut self, query: QueryRealmName) {
        let our_vra = self.virtual_realm_address();
        let is_local = query.virtual_realm_address == our_vra;

        // Query the realm name from the DB — for now hardcode from realmlist
        // TODO: query login_db for realmlist.name by realm_id
        let realm_name = "Trinity".to_string();

        debug!(
            "QueryRealmName: VRA=0x{:08X}, ours=0x{:08X}, local={}",
            query.virtual_realm_address, our_vra, is_local
        );

        let resp = RealmQueryResponse {
            virtual_realm_address: query.virtual_realm_address,
            lookup_state: 0, // Success
            realm_name_actual: realm_name.clone(),
            realm_name_normalized: realm_name,
            is_local,
        };
        self.send_packet_realm(&resp);
    }

    pub async fn handle_gossip_hello(&mut self, hello: Hello) {
        info!(
            "GossipHello for {:?} from account {}",
            hello.unit, self.account_id
        );

        const GOSSIP_FLAG: u32 = 0x1;

        let (npc_flags, entry) = self
            .mutate_world_creature(hello.unit, |creature| {
                (creature.npc_flags(), creature.entry())
            })
            .unwrap_or((0, 0));

        info!(
            "GossipHello npc_flags=0x{:X} entry={} for {:?}",
            npc_flags, entry, hello.unit
        );

        // If the NPC has Gossip flag AND we have a world DB, try to load the gossip menu.
        if npc_flags & GOSSIP_FLAG != 0 && entry != 0 {
            if let Some(world_db) = self.world_db().map(Arc::clone) {
                if let Some(msg) = self.build_gossip_menu(&world_db, entry, hello.unit).await {
                    info!(
                        "Sending GossipMessage with {} options for entry {}",
                        msg.gossip_options.len(),
                        entry
                    );
                    self.send_packet(&msg);
                    return;
                }
            }
        }

        // No gossip menu found — fall back to direct interaction based on NPC flags.
        self.handle_npc_direct_interaction(hello).await;
    }

    pub(crate) fn build_condition_player_object_like_cpp(&self) -> Option<WorldObject> {
        let mut player = WorldObject::new(
            false,
            TypeId::Player,
            TypeMask::OBJECT | TypeMask::UNIT | TypeMask::PLAYER,
        );
        player.object_mut().create(self.player_guid()?);
        let _ = player.set_map(u32::from(self.player_map_id_like_cpp()), 0);
        if let Some(position) = self.player_position_like_cpp() {
            player.relocate(position);
        }
        Some(player)
    }

    pub(crate) fn build_condition_creature_object_like_cpp(
        &mut self,
        npc_guid: ObjectGuid,
    ) -> Option<(WorldObject, crate::conditions::ConditionUnitSnapshot)> {
        self.mutate_world_creature(npc_guid, |creature| {
            let mut source =
                WorldObject::new(false, TypeId::Unit, TypeMask::OBJECT | TypeMask::UNIT);
            source.object_mut().create(creature.guid());
            source.object_mut().set_entry(creature.entry());
            let _ = source.set_map(creature.map_id(), creature.instance_id());
            source.relocate(creature.position());
            *source.phase_shift_mut() = creature.phase_shift().clone();
            let snapshot = crate::conditions::ConditionUnitSnapshot {
                level: u32::from(creature.level()),
                health: u64::from(creature.current_hp()),
                max_health: u64::from(creature.max_hp()),
                class_mask: 0,
                race: 0,
                creature_type: None,
                is_alive: creature.is_alive(),
                is_charmed: false,
                in_water: false,
                unit_state: 0,
                stand_state: UnitStandStateType::Stand as u32,
            };
            (source, snapshot)
        })
    }

    pub(crate) fn condition_player_unit_snapshot_like_cpp(
        &self,
    ) -> crate::conditions::ConditionUnitSnapshot {
        crate::conditions::ConditionUnitSnapshot {
            level: u32::from(self.player_level_like_cpp()),
            health: 1,
            max_health: 1,
            class_mask: player_class_mask(self.player_class_like_cpp()),
            race: self.player_race_like_cpp(),
            creature_type: None,
            is_alive: self.player_is_alive_like_cpp(),
            is_charmed: false,
            in_water: false,
            unit_state: 0,
            stand_state: UnitStandStateType::Stand as u32,
        }
    }

    pub(crate) fn condition_player_snapshot_like_cpp(
        &self,
    ) -> crate::conditions::ConditionPlayerSnapshot {
        crate::conditions::ConditionPlayerSnapshot {
            team: player_team_for_race_cpp(self.player_race_like_cpp()) as u32,
            native_gender: u32::from(self.player_gender_like_cpp()),
            drunken_state: 0,
            can_be_game_master: false,
            is_game_master: false,
            pet_type: None,
            is_in_flight: false,
        }
    }

    fn gossip_conditions_meet_like_cpp(
        &mut self,
        condition_store: &ConditionEntriesByTypeStore,
        source_type: ConditionSourceType,
        source_group: u32,
        source_entry: i32,
        npc_guid: ObjectGuid,
    ) -> bool {
        let Some(conditions) = condition_store
            .conditions_for_like_cpp(source_type, ConditionId::new(source_group, source_entry, 0))
        else {
            return true;
        };

        let Some(player_object) = self.build_condition_player_object_like_cpp() else {
            warn!(
                "Gossip condition check failed closed: missing player object for {:?}",
                source_type
            );
            return false;
        };
        let Some((source_object, source_unit_snapshot)) =
            self.build_condition_creature_object_like_cpp(npc_guid)
        else {
            warn!(
                "Gossip condition check failed closed: missing source object for {:?}",
                source_type
            );
            return false;
        };

        let player_unit_snapshot = self.condition_player_unit_snapshot_like_cpp();
        let player_snapshot = self.condition_player_snapshot_like_cpp();
        let player_condition_store = self.player_condition_store().cloned();
        let player_condition_context = self.represented_player_condition_context_like_cpp();

        let mut source_info = crate::conditions::ConditionSourceInfo::from_targets(
            Some(&player_object),
            Some(&source_object),
            None,
        );
        source_info.set_unit_target_snapshot(0, player_unit_snapshot);
        source_info.set_player_target_snapshot(0, player_snapshot);
        source_info.set_unit_target_snapshot(1, source_unit_snapshot);
        if let Some(store) = player_condition_store.as_ref() {
            source_info.set_player_condition_store(store.as_ref());
            source_info.set_player_condition_context(0, player_condition_context.as_context(self));
        }

        crate::conditions::is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            |condition, source_info| match crate::conditions::condition_meets_basic_like_cpp(
                condition,
                source_info,
                |current_area, required_area| current_area == required_area,
            ) {
                crate::conditions::ConditionMeetResult::Evaluated(value) => value,
                crate::conditions::ConditionMeetResult::Unsupported => {
                    warn!(
                        "Gossip condition check failed closed: unsupported {:?} for {:?} {}:{}",
                        condition.condition_type, source_type, source_group, source_entry
                    );
                    false
                }
            },
        )
    }

    fn gossip_menu_text_conditions_meet_like_cpp(
        &mut self,
        condition_store: &ConditionEntriesByTypeStore,
        menu_id: u32,
        text_id: u32,
        npc_guid: ObjectGuid,
    ) -> bool {
        if condition_store
            .conditions_for_like_cpp(
                ConditionSourceType::GossipMenu,
                ConditionId::new(menu_id, text_id as i32, 0),
            )
            .is_some()
        {
            return self.gossip_conditions_meet_like_cpp(
                condition_store,
                ConditionSourceType::GossipMenu,
                menu_id,
                text_id as i32,
                npc_guid,
            );
        }

        self.gossip_conditions_meet_like_cpp(
            condition_store,
            ConditionSourceType::GossipMenu,
            menu_id,
            0,
            npc_guid,
        )
    }

    fn vendor_item_conditions_meet_like_cpp(
        condition_store: &ConditionEntriesByTypeStore,
        creature_entry: u32,
        item_id: u32,
        player_object: Option<&WorldObject>,
        vendor_object: Option<&WorldObject>,
        player_unit_snapshot: crate::conditions::ConditionUnitSnapshot,
        player_snapshot: crate::conditions::ConditionPlayerSnapshot,
        vendor_unit_snapshot: Option<crate::conditions::ConditionUnitSnapshot>,
        player_condition_store: Option<&PlayerConditionStore>,
        player_condition_context: Option<PlayerConditionContextLikeCpp<'_>>,
    ) -> bool {
        crate::conditions::is_object_meeting_vendor_item_conditions_like_cpp(
            condition_store,
            creature_entry,
            item_id,
            player_object,
            vendor_object,
            |condition, source_info| {
                source_info.set_unit_target_snapshot(0, player_unit_snapshot);
                source_info.set_player_target_snapshot(0, player_snapshot);
                if let Some(vendor_unit_snapshot) = vendor_unit_snapshot {
                    source_info.set_unit_target_snapshot(1, vendor_unit_snapshot);
                }
                if let (Some(store), Some(context)) =
                    (player_condition_store, player_condition_context)
                {
                    source_info.set_player_condition_store(store);
                    source_info.set_player_condition_context(0, context);
                }
                match crate::conditions::condition_meets_basic_like_cpp(
                    condition,
                    source_info,
                    |current_area, required_area| current_area == required_area,
                ) {
                    crate::conditions::ConditionMeetResult::Evaluated(value) => value,
                    crate::conditions::ConditionMeetResult::Unsupported => false,
                }
            },
        )
    }

    /// Build a GossipMessage from the database for a creature entry.
    /// Returns None if no gossip menu exists.
    async fn build_gossip_menu(
        &mut self,
        world_db: &Arc<WorldDatabase>,
        entry: u32,
        npc_guid: wow_core::ObjectGuid,
    ) -> Option<GossipMessage> {
        use crate::session::GossipOptionInfo;
        use wow_packet::packets::gossip::ClientGossipOption;

        // 1. Get MenuID from creature_template_gossip
        let mut stmt = world_db.prepare(WorldStatements::SEL_CREATURE_GOSSIP_MENU);
        stmt.set_u32(0, entry);
        let menu_result: wow_database::SqlResult =
            tokio::time::timeout(std::time::Duration::from_secs(2), world_db.query(&stmt))
                .await
                .ok()?
                .ok()?;
        if menu_result.is_empty() {
            return None;
        }
        let menu_id: u32 = menu_result.try_read(0)?;

        let condition_store = self.condition_store().cloned();

        // 2. Get TextID from gossip_menu, then resolve BroadcastTextID from npc_text.
        // C++ Player::GetGossipTextId iterates every gossip_menu row and keeps the last row whose
        // attached GossipMenu conditions meet for (player, source).
        let mut stmt = world_db.prepare(WorldStatements::SEL_GOSSIP_MENU_TEXTS);
        stmt.set_u32(0, menu_id);
        let mut text_result: wow_database::SqlResult =
            tokio::time::timeout(std::time::Duration::from_secs(2), world_db.query(&stmt))
                .await
                .ok()?
                .ok()?;
        let npc_text_id: u32 = if text_result.is_empty() {
            1
        } else {
            let mut selected = 1;
            loop {
                let text_id = text_result.try_read::<u32>(0).unwrap_or(1);
                let meets = condition_store.as_ref().is_none_or(|store| {
                    self.gossip_menu_text_conditions_meet_like_cpp(
                        store.as_ref(),
                        menu_id,
                        text_id,
                        npc_guid,
                    )
                });
                if meets {
                    selected = text_id;
                }
                if !text_result.next_row() {
                    break;
                }
            }
            selected
        };

        // Resolve BroadcastTextID from npc_text (C# uses BroadcastTextID, NOT TextID)
        let broadcast_text_id: Option<i32> = {
            let mut stmt = world_db.prepare(WorldStatements::SEL_NPC_TEXT);
            stmt.set_u32(0, npc_text_id);
            match tokio::time::timeout(std::time::Duration::from_secs(2), world_db.query(&stmt))
                .await
            {
                Ok(Ok(r)) if !r.is_empty() => r.try_read::<u32>(0).map(|v| v as i32),
                _ => None,
            }
        };
        info!(
            "Gossip menu_id={} npc_text_id={} broadcast_text_id={:?}",
            menu_id, npc_text_id, broadcast_text_id
        );

        // 3. Get options from gossip_menu_option
        let mut stmt = world_db.prepare(WorldStatements::SEL_GOSSIP_MENU_OPTIONS);
        stmt.set_u32(0, menu_id);
        let mut opt_result: wow_database::SqlResult =
            match tokio::time::timeout(std::time::Duration::from_secs(2), world_db.query(&stmt))
                .await
            {
                Ok(Ok(r)) => r,
                _ => return None,
            };

        if opt_result.is_empty() {
            return None;
        }

        // Collect raw option rows first, then resolve localized text.
        struct RawOption {
            gossip_option_id: i32,
            option_id: u32,
            option_npc: u8,
            option_text: String,
            action_menu_id: u32,
            box_money: u32,
            box_text: String,
            spell_id: Option<i32>,
            override_icon_id: Option<i32>,
            broadcast_text_id: u32,
        }
        let mut raw_options = Vec::new();
        loop {
            raw_options.push(RawOption {
                gossip_option_id: opt_result.try_read(0).unwrap_or(0),
                option_id: opt_result.try_read(1).unwrap_or(0),
                option_npc: opt_result.try_read(2).unwrap_or(0),
                option_text: opt_result.read_string(3),
                action_menu_id: opt_result.try_read(4).unwrap_or(0),
                box_money: opt_result.try_read(6).unwrap_or(0),
                box_text: opt_result.read_string(7),
                spell_id: opt_result.try_read(8),
                override_icon_id: opt_result.try_read(9),
                broadcast_text_id: opt_result.try_read::<u32>(10).unwrap_or(0),
            });
            if !opt_result.next_row() {
                break;
            }
        }

        // Resolve localized text for each option via OptionBroadcastTextID.
        let locale = self.locale.clone();
        info!(
            "Gossip locale='{}' for {} options",
            locale,
            raw_options.len()
        );
        let mut gossip_options = Vec::new();
        let mut stored_options = Vec::new();
        for opt in &raw_options {
            if let Some(store) = condition_store.as_ref()
                && !self.gossip_conditions_meet_like_cpp(
                    store.as_ref(),
                    ConditionSourceType::GossipMenuOption,
                    menu_id,
                    opt.option_id as i32,
                    npc_guid,
                )
            {
                continue;
            }

            let mut text = opt.option_text.clone();

            if opt.broadcast_text_id != 0 && locale != "enUS" {
                let mut stmt = world_db.prepare(WorldStatements::SEL_BROADCAST_TEXT_LOCALE);
                stmt.set_u32(0, opt.broadcast_text_id);
                stmt.set_string(1, &locale);
                if let Ok(Ok(r)) =
                    tokio::time::timeout(std::time::Duration::from_secs(2), world_db.query(&stmt))
                        .await
                {
                    if !r.is_empty() {
                        let localized: String = r.read_string(0);
                        if !localized.is_empty() {
                            text = localized;
                        }
                    }
                }
            }

            gossip_options.push(ClientGossipOption {
                gossip_option_id: opt.gossip_option_id,
                option_npc: opt.option_npc,
                option_flags: 0,
                option_cost: opt.box_money as i32,
                option_language: 0,
                flags: 0,
                order_index: opt.option_id as i32,
                status: 0,
                text,
                confirm: opt.box_text.clone(),
                spell_id: opt.spell_id,
                override_icon_id: opt.override_icon_id,
            });

            stored_options.push(GossipOptionInfo {
                gossip_option_id: opt.gossip_option_id,
                option_npc: opt.option_npc,
                action_menu_id: opt.action_menu_id,
            });
        }

        // Store gossip state for when the player selects an option.
        self.gossip_options = stored_options;
        self.gossip_source_guid = Some(npc_guid);

        Some(GossipMessage {
            gossip_guid: npc_guid,
            gossip_id: menu_id as i32,
            friendship_faction_id: 0,
            text_id: None,
            broadcast_text_id,
            gossip_options,
            gossip_text: Vec::new(),
        })
    }

    /// Direct interaction for NPCs without gossip menus (banker, auctioneer, etc.).
    async fn handle_npc_direct_interaction(&mut self, hello: Hello) {
        use wow_packet::packets::misc::{AuctionHelloResponse, NpcInteractionOpenResult};

        const VENDOR_MASK: u32 = 0x80 | 0x100 | 0x200 | 0x400 | 0x800;
        const TRAINER_MASK: u32 = 0x10 | 0x20 | 0x40;
        const FLIGHT_MASTER: u32 = 0x2000;
        const AUCTIONEER: u32 = 0x200000;
        const BANKER: u32 = 0x20000;
        const TABARD_DESIGNER: u32 = 0x80000;
        const STABLE_MASTER: u32 = 0x400000;
        const GUILD_BANKER: u32 = 0x800000;

        let npc_flags = self
            .mutate_world_creature(hello.unit, |creature| creature.npc_flags())
            .unwrap_or(0);

        if npc_flags & VENDOR_MASK != 0 {
            self.handle_list_inventory(hello).await;
        } else if npc_flags & TRAINER_MASK != 0 {
            self.handle_trainer_list(hello).await;
        } else if npc_flags & AUCTIONEER != 0 {
            self.send_packet(&AuctionHelloResponse::open(hello.unit));
        } else if npc_flags & BANKER != 0 {
            self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 8));
        } else if npc_flags & FLIGHT_MASTER != 0 {
            self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 6));
        } else if npc_flags & TABARD_DESIGNER != 0 {
            self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 14));
        } else if npc_flags & STABLE_MASTER != 0 {
            self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 22));
        } else if npc_flags & GUILD_BANKER != 0 {
            self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 10));
        } else {
            self.send_packet(&GossipMessage::empty(hello.unit, 0, 1));
        }
    }

    /// Handle CMSG_GOSSIP_SELECT_OPTION — player selects a gossip menu option.
    ///
    /// Routes to the appropriate handler based on the option's OptionNpc value:
    /// 1=Vendor, 3=Trainer, 5=Binder, etc.
    pub async fn handle_gossip_select_option(
        &mut self,
        select: wow_packet::packets::gossip::GossipSelectOption,
    ) {
        use wow_packet::packets::misc::NpcInteractionOpenResult;

        info!(
            "GossipSelectOption: gossip_id={}, option_id={} from account {}",
            select.gossip_id, select.gossip_option_id, self.account_id
        );

        // Find the selected option in our stored gossip data.
        let opt = self
            .gossip_options
            .iter()
            .find(|o| o.gossip_option_id == select.gossip_option_id);
        let (option_npc, _action_menu_id) = match opt {
            Some(o) => (o.option_npc, o.action_menu_id),
            None => {
                warn!(
                    "GossipSelectOption: unknown gossip_option_id={} — closing.",
                    select.gossip_option_id
                );
                self.send_packet(&GossipComplete {
                    suppress_sound: false,
                });
                return;
            }
        };

        let npc_guid = self.gossip_source_guid.unwrap_or(select.gossip_unit);
        info!(
            "GossipSelectOption: OptionNpc={} for {:?}",
            option_npc, npc_guid
        );

        // Close the gossip window before opening the interaction.
        self.send_packet(&GossipComplete {
            suppress_sound: false,
        });

        let hello = Hello { unit: npc_guid };
        match option_npc {
            1 => {
                // Vendor
                self.handle_list_inventory(hello).await;
            }
            2 => {
                // Taxinode / Flight Master
                self.send_packet(&NpcInteractionOpenResult::new(npc_guid, 6));
            }
            3 => {
                // Trainer
                self.handle_trainer_list(hello).await;
            }
            5 => {
                // Binder (Innkeeper)
                self.send_packet(&NpcInteractionOpenResult::new(npc_guid, 20));
            }
            6 => {
                // Banker
                self.send_packet(&NpcInteractionOpenResult::new(npc_guid, 8));
            }
            8 => {
                // Guild Tabard Vendor
                self.send_packet(&NpcInteractionOpenResult::new(npc_guid, 14));
            }
            9 => {
                // Battlemaster
                info!("Battlemaster interaction (stub)");
            }
            10 => {
                // Auctioneer
                use wow_packet::packets::misc::AuctionHelloResponse;
                self.send_packet(&AuctionHelloResponse::open(npc_guid));
            }
            12 => {
                // Stable Master
                self.send_packet(&NpcInteractionOpenResult::new(npc_guid, 22));
            }
            _ => {
                info!(
                    "GossipSelectOption: unhandled OptionNpc={} — ignored",
                    option_npc
                );
            }
        }
    }

    // ── NPC activation handlers ───────────────────────────────────────────────

    /// CMSG_AUCTION_HELLO_REQUEST — player talks to an auctioneer.
    /// C# ref: AuctionHandler.HandleAuctionHello → SendAuctionHello
    pub async fn handle_auction_hello_request(&mut self, mut pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::AuctionHelloResponse;
        let guid = pkt
            .read_packed_guid()
            .unwrap_or(wow_core::ObjectGuid::EMPTY);
        info!(
            "AuctionHelloRequest from {:?} account {}",
            guid, self.account_id
        );
        self.send_packet(&AuctionHelloResponse::open(guid));
    }

    /// CMSG_BANKER_ACTIVATE — player talks to a banker.
    /// C# ref: BankHandler.HandleBankerActivate → SendShowBank → NpcInteractionOpenResult(Banker=8)
    pub async fn handle_banker_activate(&mut self, hello: Hello) {
        use wow_packet::packets::misc::NpcInteractionOpenResult;
        info!(
            "BankerActivate {:?} account {}",
            hello.unit, self.account_id
        );
        self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 8)); // Banker
    }

    /// CMSG_BINDER_ACTIVATE — player sets hearthstone at innkeeper.
    /// C# ref: NPCHandler.HandleBinderActivate → SendBindPoint → NpcInteractionOpenResult(Binder=20)
    pub async fn handle_binder_activate(&mut self, hello: Hello) {
        use wow_packet::packets::misc::NpcInteractionOpenResult;
        info!(
            "BinderActivate {:?} account {}",
            hello.unit, self.account_id
        );
        // TODO: actually set hearthstone bind point in DB.
        self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 20)); // Binder
    }

    /// CMSG_TABARD_VENDOR_ACTIVATE — player talks to a tabard designer.
    /// C# ref: NPCHandler.HandleTabardVendorActivate → NpcInteractionOpenResult(GuildTabardVendor=14)
    pub async fn handle_tabard_vendor_activate(&mut self, mut pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::NpcInteractionOpenResult;
        let guid = pkt
            .read_packed_guid()
            .unwrap_or(wow_core::ObjectGuid::EMPTY);
        info!(
            "TabardVendorActivate {:?} account {}",
            guid, self.account_id
        );
        self.send_packet(&NpcInteractionOpenResult::new(guid, 14)); // GuildTabardVendor
    }

    /// CMSG_SPIRIT_HEALER_ACTIVATE — ghost uses spirit healer.
    /// C# ref: NPCHandler.HandleSpiritHealerActivate → SendSpiritResurrect
    /// TODO: full resurrection logic (durability loss, corpse spawn, teleport).
    pub async fn handle_spirit_healer_activate(&mut self, _pkt: wow_packet::WorldPacket) {
        info!("SpiritHealerActivate account {} (stub)", self.account_id);
    }

    /// CMSG_REPAIR_ITEM — player repairs item at a repair vendor.
    /// C++ ref: WorldSession::HandleRepairItemOpcode.
    pub async fn handle_repair_item(&mut self, repair: RepairItem) {
        let Some(repair_npc) = self.represented_npc_can_interact_with_like_cpp(
            repair.npc_guid,
            NPCFlags1::REPAIR.bits(),
            0,
        ) else {
            debug!(
                npc_guid = ?repair.npc_guid,
                account = self.account_id,
                "RepairItem rejected: NPC missing, out of range, dead, or lacks REPAIR flag"
            );
            return;
        };

        self.remove_represented_feign_death_if_needed_like_cpp();

        // C++ uses GetReputationPriceDiscount(unit) and RATE_REPAIRCOST.
        let discount_mod = self.reputation_price_discount_for_faction_template_like_cpp(
            repair_npc.faction_template_id,
        );
        let repair_cost_rate = self.repair_cost_rate_like_cpp();

        if !repair.item_guid.is_empty() {
            let repaired = self
                .repair_inventory_item_durability_like_cpp(
                    repair.item_guid,
                    true,
                    discount_mod,
                    repair_cost_rate,
                )
                .await;
            debug!(
                npc_guid = ?repair.npc_guid,
                item_guid = ?repair.item_guid,
                repaired,
                account = self.account_id,
                "RepairItem single-item represented runtime"
            );
            return;
        }

        if repair.use_guild_bank {
            let repaired = self
                .repair_all_inventory_item_durability_with_guild_bank_like_cpp(
                    discount_mod,
                    repair_cost_rate,
                )
                .await;
            debug!(
                npc_guid = ?repair.npc_guid,
                repaired,
                account = self.account_id,
                "RepairItem all-items represented guild-bank runtime"
            );
            return;
        }

        let repaired = self
            .repair_all_inventory_item_durability_with_player_money_like_cpp(
                discount_mod,
                repair_cost_rate,
            )
            .await;
        debug!(
            npc_guid = ?repair.npc_guid,
            repaired,
            account = self.account_id,
            "RepairItem all-items represented runtime"
        );
    }

    /// CMSG_REQUEST_STABLED_PETS — player opens stable master UI.
    /// C# ref: NPCHandler.HandleRequestStabledPets
    /// TODO: query character_pet table and send PetStableList.
    pub async fn handle_request_stabled_pets(&mut self, _pkt: wow_packet::WorldPacket) {
        info!("RequestStabledPets account {} (stub)", self.account_id);
    }

    /// Handle CMSG_QUERY_NPC_TEXT — client requests NPC text for gossip.
    pub async fn handle_query_npc_text(&mut self, query: QueryNpcText) {
        debug!(
            "QueryNpcText: text_id={} for account {}",
            query.text_id, self.account_id
        );

        // For now, respond with a default "found" response.
        // BroadcastTextID=0 tells the client to use local DB2 data for text.
        self.send_packet(&QueryNpcTextResponse::with_text(query.text_id, 0));
    }

    /// Handle CMSG_LIST_INVENTORY — player opens vendor window.
    ///
    /// Queries npc_vendor for the creature's items (including reference vendors, item_id < 0)
    /// and sends SMSG_VENDOR_INVENTORY. Entry is resolved from the visibility tracker or,
    /// if missing, from world.creature by GUID (fallback when NPC not in tracker).
    pub async fn handle_list_inventory(&mut self, hello: Hello) {
        let vendor_guid = hello.unit;
        info!(
            "ListInventory for {:?} from account {}",
            vendor_guid, self.account_id
        );

        let world_db = match self.world_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        // Resolve creature entry: first from map-owned creature state, then fallback from DB by spawn GUID.
        let entry = match self.mutate_world_creature(vendor_guid, |creature| creature.entry()) {
            Some(entry) => entry,
            None => {
                let mut stmt = world_db.prepare(WorldStatements::SEL_CREATURE_ENTRY_BY_GUID);
                stmt.set_u64(0, vendor_guid.low_value() as u64);
                let fallback = match tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    world_db.query(&stmt),
                )
                .await
                {
                    Ok(Ok(r)) if !r.is_empty() => r.try_read::<u32>(0),
                    _ => None,
                };
                match fallback {
                    Some(e) => {
                        info!("Vendor entry {} resolved from DB (GUID not in tracker)", e);
                        e
                    }
                    None => {
                        info!(
                            "Vendor GUID {:?} not in tracker and not found in creature table",
                            vendor_guid
                        );
                        self.send_packet(&VendorInventory {
                            vendor_guid,
                            reason: 0,
                            items: vec![],
                        });
                        return;
                    }
                }
            }
        };

        // Load all items: direct rows + expand reference vendors (npc_vendor.item < 0).
        let mut items = Vec::new();
        let mut raw_slot = 0i32;
        let mut expanded = std::collections::HashSet::<u32>::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(entry);
        let condition_store = self.condition_store().cloned();
        let player_condition_store = self.player_condition_store().cloned();
        let player_condition_context = self.represented_player_condition_context_like_cpp();
        let player_condition_object = self.build_condition_player_object_like_cpp();
        let vendor_condition_object = self.build_condition_creature_object_like_cpp(vendor_guid);
        let player_unit_snapshot = self.condition_player_unit_snapshot_like_cpp();
        let player_snapshot = self.condition_player_snapshot_like_cpp();

        'vendor_expansion: while let Some(vendor_entry) = queue.pop_front() {
            if !expanded.insert(vendor_entry) {
                continue; // already expanded (avoid cycles)
            }
            let mut stmt = world_db.prepare(WorldStatements::SEL_VENDOR_ITEMS);
            stmt.set_u32(0, entry);
            stmt.set_u32(1, vendor_entry);

            let mut result = match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                world_db.query(&stmt),
            )
            .await
            {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    warn!("Vendor query failed for entry {vendor_entry}: {e}");
                    continue;
                }
                Err(_) => {
                    warn!("Vendor query timed out for entry {vendor_entry}");
                    continue;
                }
            };

            loop {
                let item_id: i32 = result.try_read(0).unwrap_or(0);
                let maxcount: i32 = result.try_read(1).unwrap_or(0);
                let extended_cost: i32 = result.try_read::<u32>(2).unwrap_or(0) as i32;
                let item_type: i32 = result.try_read::<u8>(3).unwrap_or(1) as i32;
                let buy_price: u64 = result
                    .try_read::<i64>(5)
                    .map(|v| v as u64)
                    .or_else(|| result.try_read::<u64>(5))
                    .unwrap_or(0);
                let durability: i32 = result.try_read::<i64>(7).map(|v| v as i32).unwrap_or(0);
                let stack_count: i32 = result.try_read::<i64>(8).map(|v| v as i32).unwrap_or(1);
                let do_not_filter: bool = result.try_read::<u8>(9).map(|v| v != 0).unwrap_or(false);
                let incr_time: u32 = result.try_read::<u32>(10).unwrap_or(0);
                let player_condition_id: u32 = result.try_read::<u32>(11).unwrap_or(0);
                let has_vendor_conditions: bool = result
                    .try_read::<u8>(12)
                    .map(|value| value != 0)
                    .unwrap_or(false);

                // Solo enviar items con ID válido; 0 o negativo el cliente lo muestra como ? y nombre vacío
                // Además filtrar items que no existen en Item.db2 — igual que C#:
                //   ObjectManager::GetItemTemplate → null si no está en ItemStorage (Item.db2)
                //   → "non-existed item, ignore"
                // Items 58260, 58274, etc. no están en Item.db2 de este cliente → se omiten.
                if item_id > 0 {
                    let muid = raw_slot.saturating_add(1);
                    raw_slot = raw_slot.saturating_add(1);
                    if item_type == ItemVendorType::Currency as i32 {
                        if vendor_list_should_skip_currency_row(
                            self.currency_types_store().map(|store| store.as_ref()),
                            item_id,
                            extended_cost,
                        ) {
                            if !result.next_row() {
                                break;
                            }
                            continue;
                        }
                        items.push(VendorItem {
                            muid,
                            item_id,
                            item_type,
                            quantity: 0,
                            price: 0,
                            durability: 0,
                            stack_count: maxcount,
                            extended_cost,
                            player_condition_failed: vendor_player_condition_failed_id_like_cpp(
                                player_condition_id,
                                player_condition_store.as_deref(),
                                Some(player_condition_context.as_context(self)),
                            ),
                            locked: false,
                            do_not_filter,
                            refundable: false,
                        });
                        if vendor_list_reaches_cpp_item_limit(items.len()) {
                            break 'vendor_expansion;
                        }
                        if !result.next_row() {
                            break;
                        }
                        continue;
                    }
                    let item_known = self
                        .item_store()
                        .map_or(true, |s| s.get(item_id as u32).is_some());
                    if !item_known {
                        info!(
                            "Vendor item {} not in Item.db2 (entry {}), skipping",
                            item_id, vendor_entry
                        );
                        if !result.next_row() {
                            break;
                        }
                        continue;
                    }
                    let current_count = self.vendor_item_current_count(
                        vendor_guid,
                        item_id as u32,
                        maxcount.max(0) as u32,
                        incr_time,
                        stack_count.max(1) as u32,
                    );
                    if vendor_list_should_skip_sold_out(maxcount, current_count, self.security > 0)
                    {
                        if !result.next_row() {
                            break;
                        }
                        continue;
                    }
                    let template = self.item_storage_template(item_id as u32);
                    let sparse_template = self
                        .item_stats_store()
                        .and_then(|store| store.sparse_template(item_id as u32));
                    if vendor_list_should_skip_allowed_class(
                        sparse_template.map(|template| template.allowable_class),
                        sparse_template.map(|template| template.bonding),
                        self.player_class_like_cpp(),
                        self.security > 0,
                    ) {
                        if !result.next_row() {
                            break;
                        }
                        continue;
                    }
                    if vendor_list_should_skip_faction_flags(
                        sparse_template.map(|template| template.flags[1]),
                        player_team_for_race_cpp(self.player_race_like_cpp()),
                        self.security > 0,
                    ) {
                        if !result.next_row() {
                            break;
                        }
                        continue;
                    }
                    if has_vendor_conditions {
                        let Some(store) = condition_store.as_ref() else {
                            if !result.next_row() {
                                break;
                            }
                            continue;
                        };
                        let (vendor_object, vendor_unit_snapshot) = vendor_condition_object
                            .as_ref()
                            .map(|(object, snapshot)| (Some(object), Some(*snapshot)))
                            .unwrap_or((None, None));
                        if !Self::vendor_item_conditions_meet_like_cpp(
                            store.as_ref(),
                            entry,
                            item_id as u32,
                            player_condition_object.as_ref(),
                            vendor_object,
                            player_unit_snapshot,
                            player_snapshot,
                            vendor_unit_snapshot,
                            player_condition_store.as_deref(),
                            Some(player_condition_context.as_context(self)),
                        ) {
                            warn!(
                                "Vendor item condition not met for creature entry {} item {}",
                                entry, item_id
                            );
                            if !result.next_row() {
                                break;
                            }
                            continue;
                        }
                    }
                    let refundable = vendor_list_item_refundable(
                        template.as_ref().map(|template| template.flags),
                        template.as_ref().map(|template| template.max_stack_size),
                        extended_cost,
                    );
                    items.push(VendorItem {
                        muid,
                        item_id,
                        item_type,
                        quantity: if maxcount == 0 {
                            -1
                        } else {
                            current_count as i32
                        },
                        price: buy_price,
                        durability,
                        stack_count: stack_count.max(1),
                        extended_cost,
                        player_condition_failed: vendor_player_condition_failed_id_like_cpp(
                            player_condition_id,
                            player_condition_store.as_deref(),
                            Some(player_condition_context.as_context(self)),
                        ),
                        locked: false,
                        do_not_filter,
                        refundable,
                    });
                    if vendor_list_reaches_cpp_item_limit(items.len()) {
                        break 'vendor_expansion;
                    }
                } else if item_id < 0 {
                    let ref_entry = (-item_id) as u32;
                    queue.push_back(ref_entry);
                }

                if !result.next_row() {
                    break;
                }
            }
        }

        let item_ids: Vec<i32> = items.iter().map(|i| i.item_id).collect();
        info!(
            "Sending vendor inventory: {} items for entry {} (item_ids: {:?})",
            items.len(),
            entry,
            item_ids
        );
        self.send_packet(&VendorInventory {
            vendor_guid,
            reason: 0,
            items,
        });
    }

    fn has_item_count_direct_inventory(&self, item_entry: u32, count: u32) -> bool {
        if count == 0 {
            return true;
        }

        let mut current_count = 0_u32;
        let mut slots: Vec<_> = self.inventory_items_like_cpp().iter().collect();
        slots.sort_by_key(|&(slot, _)| {
            let slot = *slot;
            if slot >= 19 {
                u16::from(slot)
            } else {
                1000 + u16::from(slot)
            }
        });

        for (_, inventory_item) in slots {
            if inventory_item.entry_id != item_entry {
                continue;
            }
            let Some(item) = self
                .inventory_item_objects_like_cpp()
                .get(&inventory_item.guid)
            else {
                continue;
            };
            if item.is_in_trade() {
                continue;
            }
            current_count = current_count.saturating_add(item.count());
            if current_count >= count {
                return true;
            }
        }

        false
    }

    pub(crate) fn plan_destroy_item_count_direct_inventory(
        &self,
        item_entry: u32,
        count: u32,
    ) -> Option<Vec<ExtendedCostItemTurninChange>> {
        if count == 0 {
            return Some(Vec::new());
        }

        let mut remaining = count;
        let mut changes = Vec::new();
        let mut slots: Vec<_> = self.inventory_items_like_cpp().iter().collect();
        slots.sort_by_key(|&(slot, _)| {
            let slot = *slot;
            if slot >= 19 {
                u16::from(slot)
            } else {
                1000 + u16::from(slot)
            }
        });

        for (&slot, inventory_item) in slots {
            if inventory_item.entry_id != item_entry {
                continue;
            }
            let Some(item) = self
                .inventory_item_objects_like_cpp()
                .get(&inventory_item.guid)
            else {
                continue;
            };
            if item.is_in_trade() {
                continue;
            }

            let item_count = item.count();
            if item_count <= remaining {
                remaining -= item_count;
                changes.push(ExtendedCostItemTurninChange::Delete {
                    slot,
                    item_guid: inventory_item.guid,
                    db_guid: inventory_item.db_guid,
                });
            } else {
                changes.push(ExtendedCostItemTurninChange::Update {
                    slot,
                    item_guid: inventory_item.guid,
                    db_guid: inventory_item.db_guid,
                    new_count: item_count - remaining,
                });
                remaining = 0;
            }

            if remaining == 0 {
                return Some(changes);
            }
        }

        None
    }

    pub(crate) fn append_item_turnin_statements(
        char_db: &wow_database::CharacterDatabase,
        tx: &mut SqlTransaction,
        player_guid: ObjectGuid,
        changes: &[ExtendedCostItemTurninChange],
    ) {
        for change in changes {
            match *change {
                ExtendedCostItemTurninChange::Update {
                    db_guid, new_count, ..
                } => {
                    let mut stmt = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
                    stmt.set_u32(0, new_count);
                    stmt.set_u64(1, db_guid);
                    tx.append(stmt);
                }
                ExtendedCostItemTurninChange::Delete { db_guid, .. } => {
                    let mut del_inv = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
                    del_inv.set_u64(0, player_guid.counter() as u64);
                    del_inv.set_u64(1, db_guid);
                    tx.append(del_inv);

                    let mut del_item = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
                    del_item.set_u64(0, db_guid);
                    tx.append(del_item);
                }
            }
        }
    }

    pub(crate) fn apply_item_turnin_changes(
        &mut self,
        _player_guid: ObjectGuid,
        map_id: u16,
        changes: &[ExtendedCostItemTurninChange],
    ) {
        let mut cleared_slots = Vec::new();
        let mut visible_item_changes = Vec::new();
        let mut virtual_item_changes = Vec::new();
        let mut send_stat_update = false;

        for change in changes {
            match *change {
                ExtendedCostItemTurninChange::Update {
                    item_guid,
                    new_count,
                    ..
                } => {
                    self.update_inventory_item_object_like_cpp(item_guid, |item| {
                        item.set_count(new_count);
                    });
                    self.send_packet(&UpdateObject::item_stack_count_update(
                        item_guid, map_id, new_count,
                    ));
                }
                ExtendedCostItemTurninChange::Delete {
                    slot, item_guid, ..
                } => {
                    self.remove_inventory_item_like_cpp(slot);
                    self.remove_inventory_item_object(item_guid);
                    cleared_slots.push((slot, ObjectGuid::EMPTY));
                    if (slot as usize) < 19 {
                        visible_item_changes.push((slot, 0i32, 0u16, 0u16));
                        send_stat_update = true;
                    }
                    if (15..=17).contains(&slot) {
                        virtual_item_changes.push((slot - 15, 0i32, 0u16, 0u16));
                    }
                }
            }
        }

        if !cleared_slots.is_empty() {
            self.sync_object_accessor_player();
            self.send_player_values_update_from_entity_bridge(
                &cleared_slots,
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );
        }
        if send_stat_update {
            self.send_stat_update();
        }
    }

    /// Handle CMSG_BUY_ITEM — player buys an item from a vendor.
    ///
    /// C# ref: `ItemHandler.HandleBuyItem` → `Player.BuyItemFromVendorSlot`.
    /// Simplified: no reputation discount, no extended cost, no stack logic.
    pub async fn handle_buy_item(&mut self, buy: BuyItem) {
        use wow_packet::packets::update::{ItemCreateData, UpdateObject};

        debug!(
            "BuyItem: item={} qty={} muid={} from {:?}",
            buy.item_id, buy.quantity, buy.muid, buy.vendor_guid
        );

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };
        let realm_id = self.realm_id();
        let map_id = self.player_map_id_like_cpp();
        let vendor_slot = match vendor_buy_muid_to_cpp_slot(buy.muid) {
            Some(slot) => slot,
            None => return,
        };

        // ── Get vendor NPC entry from creature GUID ──
        let vendor_entry = match self.mutate_world_creature(buy.vendor_guid, |c| c.entry()) {
            Some(entry) => entry,
            None => {
                warn!("BuyItem: vendor {:?} not in creatures", buy.vendor_guid);
                self.send_buy_error(
                    BuyResult::DistanceTooFar,
                    Some(buy.vendor_guid),
                    buy.muid as u32,
                );
                return;
            }
        };

        let world_db = match self.world_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let condition_store = self.condition_store().cloned();
        let player_condition_store = self.player_condition_store().cloned();
        let player_condition_context = self.represented_player_condition_context_like_cpp();
        if let Some(store) = condition_store.as_ref() {
            let player_condition_object = self.build_condition_player_object_like_cpp();
            let vendor_condition_object =
                self.build_condition_creature_object_like_cpp(buy.vendor_guid);
            let (vendor_object, vendor_unit_snapshot) = vendor_condition_object
                .as_ref()
                .map(|(object, snapshot)| (Some(object), Some(*snapshot)))
                .unwrap_or((None, None));
            if !Self::vendor_item_conditions_meet_like_cpp(
                store.as_ref(),
                vendor_entry,
                buy.item_id as u32,
                player_condition_object.as_ref(),
                vendor_object,
                self.condition_player_unit_snapshot_like_cpp(),
                self.condition_player_snapshot_like_cpp(),
                vendor_unit_snapshot,
                player_condition_store.as_deref(),
                Some(player_condition_context.as_context(self)),
            ) {
                warn!(
                    "BuyItem: conditions not met for creature entry {} item {}",
                    vendor_entry, buy.item_id
                );
                self.send_buy_error(
                    BuyResult::CantFindItem,
                    Some(buy.vendor_guid),
                    buy.item_id as u32,
                );
                return;
            }
        }

        if buy.item_type == ItemVendorType::Currency as i32 {
            if !vendor_currency_type_is_known(
                self.currency_types_store().map(|store| store.as_ref()),
                buy.item_id as u32,
            ) {
                self.send_buy_error(BuyResult::CantFindItem, None, buy.item_id as u32);
                return;
            }

            let quantity = vendor_buy_currency_packet_quantity_to_cpp_count(buy.quantity);
            let vendor_item = match self
                .resolve_vendor_buy_item_by_cpp_slot(
                    world_db.as_ref(),
                    vendor_entry,
                    vendor_slot,
                    buy.item_id as u32,
                )
                .await
            {
                Some(item) if item.item_type == ItemVendorType::Currency as i32 => item,
                _ => {
                    self.send_buy_error(
                        BuyResult::CantFindItem,
                        Some(buy.vendor_guid),
                        buy.item_id as u32,
                    );
                    return;
                }
            };

            if let Some(result) = vendor_buy_player_condition_block_result_like_cpp(
                vendor_item.player_condition_id,
                player_condition_store.as_deref(),
                Some(player_condition_context.as_context(self)),
            ) {
                self.send_equip_error(result, None, None, 0, 0);
                return;
            }

            if let Some(result) =
                vendor_buy_currency_quantity_block_result(vendor_item.max_count, quantity)
            {
                self.send_equip_error(result, None, None, 0, 0);
                return;
            }

            if vendor_item.extended_cost == 0 {
                self.send_buy_error(BuyResult::CantFindItem, None, buy.item_id as u32);
                return;
            }

            match vendor_buy_extended_cost_block_result(
                self.item_extended_cost_store().map(|store| store.as_ref()),
                self.currency_types_store().map(|store| store.as_ref()),
                |item_id, amount| self.has_item_count_direct_inventory(item_id, amount),
                |currency_id, amount| self.has_currency(currency_id, amount),
                true,
                vendor_item.extended_cost,
                vendor_item.max_count,
                quantity,
            ) {
                Some(VendorExtendedCostBlock::Equip(result)) => {
                    self.send_equip_error(result, None, None, 0, 0);
                }
                Some(VendorExtendedCostBlock::Buy(result)) => {
                    self.send_buy_error(result, Some(buy.vendor_guid), buy.item_id as u32);
                }
                Some(VendorExtendedCostBlock::Silent) | None => {}
            }

            let extended_cost_item_costs = vendor_buy_extended_cost_item_costs(
                self.item_extended_cost_store().map(|store| store.as_ref()),
                vendor_item.extended_cost,
                vendor_item.max_count,
                quantity,
            );
            let extended_cost_currency_costs = vendor_buy_extended_cost_currency_costs(
                self.item_extended_cost_store().map(|store| store.as_ref()),
                vendor_item.extended_cost,
                vendor_item.max_count,
                quantity,
            );
            let char_db = match self.char_db() {
                Some(db) => Arc::clone(db),
                None => return,
            };
            let mut item_turnin_changes = Vec::new();
            for &(item_id, amount) in &extended_cost_item_costs {
                let Some(mut changes) =
                    self.plan_destroy_item_count_direct_inventory(item_id, amount)
                else {
                    self.send_equip_error(InventoryResult::VendorMissingTurnins, None, None, 0, 0);
                    return;
                };
                item_turnin_changes.append(&mut changes);
            }
            let currency_snapshot = self.player_currencies_like_cpp().clone();
            let currency_gain = match self.add_currency_vendor(buy.item_id as u32, quantity) {
                Ok(delta) => delta,
                Err(()) => {
                    self.set_player_currencies_like_cpp(currency_snapshot);
                    self.send_equip_error(InventoryResult::VendorMissingTurnins, None, None, 0, 0);
                    return;
                }
            };
            for &(currency_id, amount) in &extended_cost_currency_costs {
                if i32::try_from(amount).is_err() || !self.remove_currency(currency_id, amount) {
                    self.set_player_currencies_like_cpp(currency_snapshot);
                    self.send_equip_error(InventoryResult::VendorMissingTurnins, None, None, 0, 0);
                    return;
                }
            }

            let mut tx = SqlTransaction::new();
            Self::append_item_turnin_statements(
                char_db.as_ref(),
                &mut tx,
                player_guid,
                &item_turnin_changes,
            );
            self.append_player_currency_save_statements(&mut tx, player_guid.counter() as u64);
            if let Err(e) = char_db.commit_transaction(tx).await {
                self.set_player_currencies_like_cpp(currency_snapshot);
                warn!("BuyItem: currency vendor transaction failed: {e}");
                self.send_buy_error(
                    BuyResult::CantFindItem,
                    Some(buy.vendor_guid),
                    buy.item_id as u32,
                );
                return;
            }

            if let Some(delta) = currency_gain {
                let (Some(quantity), Some(amount)) = (
                    i32::try_from(delta.quantity).ok(),
                    i32::try_from(delta.amount).ok(),
                ) else {
                    return;
                };
                let mut packet =
                    SetCurrency::vendor_gain(delta.currency_id as i32, quantity, amount);
                packet.weekly_quantity = delta
                    .weekly_quantity
                    .and_then(|value| i32::try_from(value).ok());
                packet.max_quantity = delta
                    .max_quantity
                    .and_then(|value| i32::try_from(value).ok());
                packet.total_earned = delta
                    .total_earned
                    .and_then(|value| i32::try_from(value).ok());
                packet.suppress_chat_log = delta.suppress_chat_log;
                self.send_packet(&packet);
            }
            self.apply_item_turnin_changes(player_guid, map_id, &item_turnin_changes);
            for &(currency_id, amount) in &extended_cost_currency_costs {
                let Some(quantity) = i32::try_from(self.player_currency_quantity(currency_id)).ok()
                else {
                    continue;
                };
                let Some(amount) = i32::try_from(amount).ok() else {
                    continue;
                };
                self.send_packet(&SetCurrency::vendor_loss(
                    currency_id as i32,
                    quantity,
                    amount,
                ));
            }
            return;
        }

        if buy.item_type != ItemVendorType::Item as i32 {
            warn!("BuyItem: unsupported item type {}", buy.item_type);
            return;
        }

        // ── Validate: player alive ──
        let quantity = vendor_buy_packet_quantity_to_cpp_count(buy.quantity);
        let (store_bag, store_slot) =
            match vendor_buy_direct_inventory_destination(player_guid, &buy) {
                Some(destination) => destination,
                None => {
                    warn!(
                        "BuyItem: rejected slot {} above C++ MAX_BAG_SIZE {}",
                        buy.slot, MAX_BAG_SIZE
                    );
                    return;
                }
            };

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let vendor_item = match self
            .resolve_vendor_buy_item_by_cpp_slot(
                world_db.as_ref(),
                vendor_entry,
                vendor_slot,
                buy.item_id as u32,
            )
            .await
        {
            Some(item) if item.item_type == ItemVendorType::Item as i32 => item,
            _ => {
                warn!(
                    "BuyItem: vendor slot {} item {} not found for vendor {}",
                    vendor_slot, buy.item_id, vendor_entry
                );
                self.send_buy_error(
                    BuyResult::CantFindItem,
                    Some(buy.vendor_guid),
                    buy.muid as u32,
                );
                return;
            }
        };
        let sparse_template = self
            .item_stats_store()
            .and_then(|store| store.sparse_template(buy.item_id as u32));
        let allowable_class = sparse_template.map(|template| template.allowable_class);
        let bonding = sparse_template.map(|template| template.bonding);
        let flags2 = sparse_template.map(|template| template.flags[1]);
        let required_reputation_faction =
            sparse_template.map(|template| template.required_reputation_faction);
        let required_reputation_rank =
            sparse_template.map(|template| template.required_reputation_rank);
        if let Some(block) = vendor_buy_template_block_result(
            allowable_class,
            bonding,
            flags2,
            self.player_class_like_cpp(),
            self.player_race_like_cpp(),
            self.security > 0,
        ) {
            match block {
                VendorBuyTemplateBlock::BuyError(result) => {
                    self.send_buy_error(result, None, buy.item_id as u32);
                }
                VendorBuyTemplateBlock::Silent => {}
            }
            return;
        }
        if condition_store.is_none()
            && let Some(result) = vendor_conditions_block_result(vendor_item.has_vendor_conditions)
        {
            self.send_buy_error(result, Some(buy.vendor_guid), buy.item_id as u32);
            return;
        }
        if let Some(result) = vendor_buy_player_condition_block_result_like_cpp(
            vendor_item.player_condition_id,
            player_condition_store.as_deref(),
            Some(player_condition_context.as_context(self)),
        ) {
            self.send_equip_error(result, None, None, 0, 0);
            return;
        }
        let vendor_current_count = self.vendor_item_current_count(
            buy.vendor_guid,
            vendor_item.item_id,
            vendor_item.max_count,
            vendor_item.incr_time,
            vendor_item.buy_count,
        );
        if vendor_item.max_count != 0 && vendor_current_count < quantity {
            self.send_buy_error(
                BuyResult::ItemAlreadySold,
                Some(buy.vendor_guid),
                buy.muid as u32,
            );
            return;
        }
        if let Some(result) = vendor_buy_required_reputation_block_result(
            required_reputation_faction,
            required_reputation_rank,
            -1,
        ) {
            self.send_buy_error(result, Some(buy.vendor_guid), buy.item_id as u32);
            return;
        }
        if let Some(result) = vendor_buy_extended_cost_block_result(
            self.item_extended_cost_store().map(|store| store.as_ref()),
            self.currency_types_store().map(|store| store.as_ref()),
            |item_id, amount| self.has_item_count_direct_inventory(item_id, amount),
            |currency_id, amount| self.has_currency(currency_id, amount),
            true,
            vendor_item.extended_cost,
            vendor_item.buy_count,
            quantity,
        ) {
            match result {
                VendorExtendedCostBlock::Equip(result) => {
                    self.send_equip_error(result, None, None, 0, 0);
                }
                VendorExtendedCostBlock::Buy(result) => {
                    self.send_buy_error(result, Some(buy.vendor_guid), buy.item_id as u32);
                }
                VendorExtendedCostBlock::Silent => {}
            }
            return;
        }
        let extended_cost_item_costs = vendor_buy_extended_cost_item_costs(
            self.item_extended_cost_store().map(|store| store.as_ref()),
            vendor_item.extended_cost,
            vendor_item.buy_count,
            quantity,
        );
        let extended_cost_currency_costs = vendor_buy_extended_cost_currency_costs(
            self.item_extended_cost_store().map(|store| store.as_ref()),
            vendor_item.extended_cost,
            vendor_item.buy_count,
            quantity,
        );
        if let Some(result) = vendor_buy_direct_store_block_result(store_bag, store_slot, quantity)
        {
            self.send_equip_error(result, None, None, 0, 0);
            return;
        }

        let (quantity, buy_price): (u32, u64) =
            vendor_buy_quantity_and_price(vendor_item.buy_price, vendor_item.buy_count, quantity);
        let max_durability = vendor_item.max_durability;
        let refund_template = self.item_storage_template(buy.item_id as u32);
        let creates_refund_metadata = vendor_list_item_refundable(
            refund_template.as_ref().map(|template| template.flags),
            refund_template
                .as_ref()
                .map(|template| template.max_stack_size),
            vendor_item.extended_cost as i32,
        );

        // ── Check gold ──
        if self.player_gold_like_cpp() < buy_price {
            self.send_buy_error(
                BuyResult::NotEnoughtMoney,
                Some(buy.vendor_guid),
                buy.muid as u32,
            );
            return;
        }

        let (store_result, store_dest, _) = match self.plan_store_new_direct_inventory_item_at(
            buy.item_id as u32,
            quantity,
            store_bag,
            store_slot,
        ) {
            Some(plan) => plan,
            None => {
                self.send_buy_error(
                    BuyResult::CantFindItem,
                    Some(buy.vendor_guid),
                    buy.muid as u32,
                );
                return;
            }
        };
        if store_result != InventoryResult::Ok {
            self.send_equip_error(store_result, None, None, 0, 0);
            return;
        }

        let needs_new_items = store_dest.iter().any(|dest| {
            let slot = (dest.pos & 0x00FF) as u8;
            !self.inventory_items_like_cpp().contains_key(&slot)
        });
        let mut next_item_guid = if needs_new_items {
            let max_guid_stmt = char_db.prepare(CharStatements::SEL_MAX_ITEM_GUID);
            match char_db.query(&max_guid_stmt).await {
                Ok(r) => r.try_read::<u64>(0).unwrap_or(0) + 1,
                Err(_) => 1,
            }
        } else {
            0
        };

        let mut tx = SqlTransaction::new();
        let old_gold = self.player_gold_like_cpp();
        let new_gold = old_gold.saturating_sub(buy_price);
        let mut upd_money = char_db.prepare(CharStatements::UPD_CHAR_MONEY);
        upd_money.set_u64(0, new_gold);
        upd_money.set_u64(1, player_guid.counter() as u64);
        tx.append(upd_money);

        let mut existing_updates = Vec::new();
        let mut new_stacks = Vec::new();
        for dest in &store_dest {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;
            if bag != u8::from(INVENTORY_SLOT_BAG_0) {
                warn!(
                    "BuyItem: direct inventory plan produced unsupported bag {}",
                    bag
                );
                self.send_equip_error(InventoryResult::WrongBagType, None, None, 0, 0);
                return;
            }

            if let Some(inv_item) = self.inventory_items_like_cpp().get(&slot) {
                let Some(existing_item) =
                    self.inventory_item_objects_like_cpp().get(&inv_item.guid)
                else {
                    warn!("BuyItem: missing runtime item object for slot {}", slot);
                    self.send_buy_error(
                        BuyResult::CantFindItem,
                        Some(buy.vendor_guid),
                        buy.muid as u32,
                    );
                    return;
                };
                let new_count = existing_item.count().saturating_add(dest.count);
                let mut upd_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
                upd_count.set_u32(0, new_count);
                upd_count.set_u64(1, inv_item.db_guid);
                tx.append(upd_count);
                existing_updates.push((slot, inv_item.guid, new_count));
            } else {
                let db_guid = next_item_guid;
                next_item_guid += 1;
                let item_guid = ObjectGuid::create_item(realm_id, db_guid as i64);

                let mut ins_item = char_db.prepare(CharStatements::INS_ITEM_INSTANCE);
                ins_item.set_u64(0, db_guid);
                ins_item.set_u32(1, buy.item_id as u32);
                ins_item.set_u64(2, player_guid.counter() as u64);
                ins_item.set_u32(3, dest.count);
                ins_item.set_u32(4, max_durability);
                tx.append(ins_item);

                let mut ins_inv = char_db.prepare(CharStatements::INS_CHAR_INVENTORY);
                ins_inv.set_u64(0, player_guid.counter() as u64);
                ins_inv.set_u8(1, slot);
                ins_inv.set_u64(2, db_guid);
                tx.append(ins_inv);

                new_stacks.push((slot, db_guid, item_guid, dest.count));
            }
        }
        let refund_item_db_guid = creates_refund_metadata
            .then(|| new_stacks.last().map(|&(_, db_guid, _, _)| db_guid))
            .flatten();
        if let Some(refund_item_db_guid) = refund_item_db_guid {
            let mut upd_flags = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_FLAGS);
            upd_flags.set_u32(0, ItemFieldFlags::REFUNDABLE.bits());
            upd_flags.set_u64(1, refund_item_db_guid);
            tx.append(upd_flags);
            append_item_refund_insert_statements(
                char_db.as_ref(),
                &mut tx,
                refund_item_db_guid,
                player_guid.counter() as u64,
                buy_price,
                vendor_item.extended_cost as u16,
            );
        }

        let mut item_turnin_changes = Vec::new();
        for &(item_id, amount) in &extended_cost_item_costs {
            let Some(mut changes) = self.plan_destroy_item_count_direct_inventory(item_id, amount)
            else {
                self.send_equip_error(InventoryResult::VendorMissingTurnins, None, None, 0, 0);
                return;
            };
            item_turnin_changes.append(&mut changes);
        }
        Self::append_item_turnin_statements(
            char_db.as_ref(),
            &mut tx,
            player_guid,
            &item_turnin_changes,
        );

        let currency_snapshot = self.player_currencies_like_cpp().clone();
        for &(currency_id, amount) in &extended_cost_currency_costs {
            if i32::try_from(amount).is_err() || !self.remove_currency(currency_id, amount) {
                self.set_player_currencies_like_cpp(currency_snapshot);
                self.send_equip_error(InventoryResult::VendorMissingTurnins, None, None, 0, 0);
                return;
            }
        }
        self.append_player_currency_save_statements(&mut tx, player_guid.counter() as u64);

        if let Err(e) = char_db.commit_transaction(tx).await {
            self.set_player_currencies_like_cpp(currency_snapshot);
            warn!("BuyItem: store transaction failed: {e}");
            self.send_buy_error(
                BuyResult::CantFindItem,
                Some(buy.vendor_guid),
                buy.muid as u32,
            );
            return;
        }

        self.apply_player_money_change_like_cpp(old_gold, new_gold)
            .await;
        self.apply_item_turnin_changes(player_guid, map_id, &item_turnin_changes);
        for &(currency_id, amount) in &extended_cost_currency_costs {
            let Some(quantity) = i32::try_from(self.player_currency_quantity(currency_id)).ok()
            else {
                continue;
            };
            let Some(amount) = i32::try_from(amount).ok() else {
                continue;
            };
            self.send_packet(&SetCurrency::vendor_loss(
                currency_id as i32,
                quantity,
                amount,
            ));
        }

        for &(_, item_guid, new_count) in &existing_updates {
            self.update_inventory_item_object_like_cpp(item_guid, |item| {
                item.set_count(new_count);
            });
        }

        let inv_type = self.item_template_inventory_type(buy.item_id as u32);
        let mut collection_updates = Vec::new();
        for &(slot, db_guid, item_guid, stack_count) in &new_stacks {
            self.insert_inventory_item_like_cpp(
                slot,
                crate::session::InventoryItem {
                    guid: item_guid,
                    entry_id: buy.item_id as u32,
                    db_guid,
                    inventory_type: inv_type,
                },
            );
            let mut item_object = self.make_inventory_item_object(
                item_guid,
                buy.item_id as u32,
                player_guid,
                stack_count,
                max_durability,
                ItemContext::Vendor,
                slot,
            );
            if refund_item_db_guid == Some(db_guid) {
                item_object.set_item_flag(ItemFieldFlags::REFUNDABLE);
                item_object.set_refund_recipient(player_guid);
                item_object.set_paid_money(buy_price);
                item_object.set_paid_extended_cost(vendor_item.extended_cost as u32);
            }
            collection_updates.extend(self.on_item_added_to_collection_like_cpp(&item_object));
            self.insert_inventory_item_object(item_object);
        }
        self.sync_object_accessor_player();

        let changed_slots: Vec<_> = new_stacks
            .iter()
            .map(|&(slot, _, item_guid, _)| (slot, item_guid))
            .collect();

        info!(
            "BuyItem: player {:?} bought item {} across {} destination(s) for {} copper (remaining: {})",
            player_guid,
            buy.item_id,
            store_dest.len(),
            buy_price,
            self.player_gold_like_cpp()
        );
        let new_quantity = if vendor_item.max_count == 0 {
            -1
        } else {
            self.update_vendor_item_current_count(
                buy.vendor_guid,
                vendor_item.item_id,
                vendor_item.max_count,
                vendor_item.incr_time,
                vendor_item.buy_count,
                quantity,
            ) as i32
        };

        // ── Send BuySucceeded ──
        self.send_packet(&BuySucceeded {
            vendor_guid: buy.vendor_guid,
            muid: buy.muid,
            new_quantity,
            quantity_bought: quantity as i32,
        });

        if !new_stacks.is_empty() {
            let item_creates = new_stacks
                .iter()
                .map(|&(_, _, item_guid, stack_count)| ItemCreateData {
                    item_guid,
                    entry_id: buy.item_id,
                    owner_guid: player_guid,
                    contained_in: player_guid,
                    stack_count,
                    dynamic_flags: 0,
                    durability: max_durability,
                    max_durability,
                    random_properties_seed: 0,
                    random_properties_id: 0,
                    context: 0,
                })
                .collect();
            self.send_packet(&UpdateObject::create_items(item_creates, map_id));
        }

        for &(_, item_guid, new_count) in &existing_updates {
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid, map_id, new_count,
            ));
        }

        self.send_player_values_update_from_entity_bridge(
            &changed_slots,
            &[],
            &[],
            &[],
            Some(self.player_gold_like_cpp()),
        );
        for update in &collection_updates {
            self.send_player_values_update_like_cpp(update);
        }
    }

    /// Handle CMSG_BUY_BACK_ITEM — player buys back an item from a vendor.
    ///
    /// C++ ref: `WorldSession::HandleBuybackItem`.
    pub async fn handle_buy_back_item(&mut self, buyback: BuyBackItem) {
        use wow_packet::packets::update::UpdateObject;

        debug!(
            "BuyBackItem: slot={} from vendor {:?}",
            buyback.slot, buyback.vendor_guid
        );

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };
        let map_id = self.player_map_id_like_cpp();
        if self
            .mutate_world_creature(buyback.vendor_guid, |_| ())
            .is_none()
        {
            self.send_sell_error(SellResult::CantFindVendor, None, ObjectGuid::EMPTY);
            return;
        }

        let Ok(buyback_slot) = u8::try_from(buyback.slot) else {
            self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
            return;
        };
        if !WorldSession::is_buyback_slot(buyback_slot) {
            self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
            return;
        }

        let buyback_item = match self.buyback_items_like_cpp().get(&buyback_slot).cloned() {
            Some(item) => item,
            None => {
                self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
                return;
            }
        };
        let Some(runtime_item) = self
            .inventory_item_objects_like_cpp()
            .get(&buyback_item.guid)
            .cloned()
        else {
            self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
            return;
        };

        let buyback_index = (buyback_slot - BUYBACK_SLOT_START) as usize;
        let price = u64::from(self.buyback_price_like_cpp()[buyback_index]);
        if self.player_gold_like_cpp() < price {
            self.send_buy_error(
                BuyResult::NotEnoughtMoney,
                Some(buyback.vendor_guid),
                buyback_item.entry_id,
            );
            return;
        }

        let (store_result, store_dest, _) = match self.plan_store_new_direct_inventory_item_at(
            buyback_item.entry_id,
            runtime_item.count(),
            NULL_BAG,
            NULL_SLOT,
        ) {
            Some(plan) => plan,
            None => {
                self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
                return;
            }
        };
        if store_result != InventoryResult::Ok {
            self.send_equip_error(store_result, Some(buyback_item.guid), None, 0, 0);
            return;
        }

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };
        let mut tx = SqlTransaction::new();
        let old_gold = self.player_gold_like_cpp();
        let new_gold = old_gold.saturating_sub(price);
        let mut upd_money = char_db.prepare(CharStatements::UPD_CHAR_MONEY);
        upd_money.set_u64(0, new_gold);
        upd_money.set_u64(1, player_guid.counter() as u64);
        tx.append(upd_money);

        let mut existing_updates = Vec::new();
        let mut moved_slot = None;
        let mut moved_count = 0u32;
        for dest in &store_dest {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;
            if bag != u8::from(INVENTORY_SLOT_BAG_0) {
                self.send_equip_error(
                    InventoryResult::WrongBagType,
                    Some(buyback_item.guid),
                    None,
                    0,
                    0,
                );
                return;
            }

            if let Some(inv_item) = self.inventory_items_like_cpp().get(&slot) {
                let Some(existing_item) =
                    self.inventory_item_objects_like_cpp().get(&inv_item.guid)
                else {
                    self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
                    return;
                };
                let new_count = existing_item.count().saturating_add(dest.count);
                let mut upd_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
                upd_count.set_u32(0, new_count);
                upd_count.set_u64(1, inv_item.db_guid);
                tx.append(upd_count);
                existing_updates.push((slot, inv_item.guid, new_count));
            } else {
                if moved_slot.is_some() {
                    self.send_equip_error(
                        InventoryResult::NoSlotAvailable,
                        Some(buyback_item.guid),
                        None,
                        0,
                        0,
                    );
                    return;
                }
                let mut upd_slot = char_db.prepare(CharStatements::UPD_CHAR_INVENTORY_SLOT);
                upd_slot.set_u8(0, slot);
                upd_slot.set_u64(1, player_guid.counter() as u64);
                upd_slot.set_u64(2, buyback_item.db_guid);
                tx.append(upd_slot);
                if runtime_item.count() != dest.count {
                    let mut upd_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
                    upd_count.set_u32(0, dest.count);
                    upd_count.set_u64(1, buyback_item.db_guid);
                    tx.append(upd_count);
                }
                moved_slot = Some(slot);
                moved_count = dest.count;
            }
        }

        if moved_slot.is_none() {
            let mut del_inv = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
            del_inv.set_u64(0, player_guid.counter() as u64);
            del_inv.set_u64(1, buyback_item.db_guid);
            tx.append(del_inv);

            let mut del_item = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
            del_item.set_u64(0, buyback_item.db_guid);
            tx.append(del_item);
        }

        if let Err(e) = char_db.commit_transaction(tx).await {
            warn!("BuyBackItem: transaction failed: {e}");
            self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
            return;
        }

        self.apply_player_money_change_like_cpp(old_gold, new_gold)
            .await;
        self.remove_buyback_item_like_cpp(buyback_slot);
        self.clear_buyback_slot_metadata_like_cpp(buyback_slot);
        if self
            .buyback_items_like_cpp()
            .contains_key(&self.current_buyback_slot_like_cpp())
        {
            self.set_current_buyback_slot_like_cpp(buyback_slot);
        }

        for &(_, item_guid, new_count) in &existing_updates {
            self.update_inventory_item_object_like_cpp(item_guid, |item| {
                item.set_count(new_count);
            });
        }

        let mut inv_slot_changes = vec![(buyback_slot, ObjectGuid::EMPTY)];
        if let Some(slot) = moved_slot {
            self.insert_inventory_item_like_cpp(
                slot,
                InventoryItem {
                    guid: buyback_item.guid,
                    entry_id: buyback_item.entry_id,
                    db_guid: buyback_item.db_guid,
                    inventory_type: buyback_item.inventory_type,
                },
            );
            self.set_inventory_item_object_slot(buyback_item.guid, slot);
            self.update_inventory_item_object_like_cpp(buyback_item.guid, |item_object| {
                item_object.set_count(moved_count);
            });
            inv_slot_changes.push((slot, buyback_item.guid));
        } else {
            self.remove_inventory_item_object(buyback_item.guid);
        }
        self.sync_object_accessor_player();

        for &(_, item_guid, new_count) in &existing_updates {
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid, map_id, new_count,
            ));
        }
        if moved_slot.is_some() && moved_count != runtime_item.count() {
            self.send_packet(&UpdateObject::item_stack_count_update(
                buyback_item.guid,
                map_id,
                moved_count,
            ));
        }
        self.send_player_values_update_from_entity_bridge(
            &inv_slot_changes,
            &[],
            &[],
            &[(buyback_slot, 0, 0)],
            Some(self.player_gold_like_cpp()),
        );
    }

    /// Handle CMSG_SELL_ITEM — player sells an item to a vendor.
    ///
    /// C# ref: `ItemHandler.HandleSellItem` → `Player.SellItemToVendor`.
    pub async fn handle_sell_item(&mut self, sell: SellItem) {
        use wow_packet::packets::update::UpdateObject;

        debug!(
            "SellItem: item={:?} from account {}",
            sell.item_guid, self.account_id
        );

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };
        let map_id = self.player_map_id_like_cpp();

        // ── Find item in inventory by GUID ──
        let (slot, item) = match self
            .inventory_items_like_cpp()
            .iter()
            .find(|(_, item)| item.guid == sell.item_guid)
            .map(|(&s, item)| (s, item.clone()))
        {
            Some(pair) => pair,
            None => {
                warn!("SellItem: item {:?} not in inventory", sell.item_guid);
                self.send_sell_error(
                    SellResult::YouDontOwnThatItem,
                    Some(sell.vendor_guid),
                    sell.item_guid,
                );
                return;
            }
        };

        // Equipped items (slots 0-18) can't be sold without unequipping first
        if slot < 19 {
            self.send_sell_error(
                SellResult::CantSellItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        }

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let Some(runtime_item) = self
            .inventory_item_objects_like_cpp()
            .get(&item.guid)
            .cloned()
        else {
            self.send_sell_error(
                SellResult::CantFindItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        };
        let item_inventory_type = self
            .item_storage_template(item.entry_id)
            .map(|template| template.inventory_type);
        if item_is_not_empty_bag_like_cpp(
            item_inventory_type,
            self.direct_item_contains_items(item.guid),
        ) {
            self.send_sell_error(
                SellResult::CantSellItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        }
        if self.is_active_loot_guid(item.guid) || item_is_currently_looted_like_cpp(&runtime_item) {
            self.send_sell_error(
                SellResult::CantSellItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        }
        if runtime_item.is_refundable() {
            return;
        }
        let sell_amount = match sell_item_amount_action(runtime_item.count(), sell.amount) {
            SellItemAmountAction::Invalid => {
                self.send_sell_error(
                    SellResult::CantSellItem,
                    Some(sell.vendor_guid),
                    sell.item_guid,
                );
                return;
            }
            action => action,
        };
        let sold_count = match sell_amount {
            SellItemAmountAction::FullStack { amount }
            | SellItemAmountAction::PartialStack { amount, .. } => amount,
            SellItemAmountAction::Invalid => unreachable!(),
        };

        // ── Get sell price from item_sparse directly ──
        let sell_price: u64 = {
            let world_db = match self.world_db() {
                Some(db) => Arc::clone(db),
                None => return,
            };
            let mut stmt = world_db.prepare(WorldStatements::SEL_ITEM_SELL_PRICE);
            stmt.set_u32(0, item.entry_id);
            match world_db.query(&stmt).await {
                Ok(r) if !r.is_empty() => r.try_read::<u64>(0).unwrap_or(0),
                _ => 0,
            }
        };
        if sell_price == 0 {
            self.send_sell_error(
                SellResult::CantSellItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        }

        let money = sell_price.saturating_mul(u64::from(sold_count));
        let old_gold = self.player_gold_like_cpp();
        let Some(new_gold) = player_money_gain_like_cpp(old_gold, money) else {
            self.send_sell_error(
                SellResult::CantSellItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        };
        let buyback_slot = self.select_buyback_slot_cpp();
        let old_buyback = self.buyback_items_like_cpp().get(&buyback_slot).cloned();
        let buyback_price = sell_price
            .saturating_mul(u64::from(sold_count))
            .min(u64::from(u32::MAX)) as u32;
        let buyback_timestamp = self
            .login_time
            .map(|login_time| login_time.elapsed().as_secs())
            .unwrap_or(0)
            .saturating_add(30 * 3600)
            .min(u64::from(u32::MAX)) as i64;

        let mut tx = SqlTransaction::new();
        if let Some(old_buyback) = &old_buyback {
            let mut del_old_inv = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
            del_old_inv.set_u64(0, player_guid.counter() as u64);
            del_old_inv.set_u64(1, old_buyback.db_guid);
            tx.append(del_old_inv);

            let mut del_old_item = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
            del_old_item.set_u64(0, old_buyback.db_guid);
            tx.append(del_old_item);
        }

        let mut new_buyback_stack = None;
        match sell_amount {
            SellItemAmountAction::FullStack { .. } => {
                let mut upd_slot = char_db.prepare(CharStatements::UPD_CHAR_INVENTORY_SLOT);
                upd_slot.set_u8(0, buyback_slot);
                upd_slot.set_u64(1, player_guid.counter() as u64);
                upd_slot.set_u64(2, item.db_guid);
                tx.append(upd_slot);
            }
            SellItemAmountAction::PartialStack { remaining, amount } => {
                let mut upd_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
                upd_count.set_u32(0, remaining);
                upd_count.set_u64(1, item.db_guid);
                tx.append(upd_count);

                let max_guid_stmt = char_db.prepare(CharStatements::SEL_MAX_ITEM_GUID);
                let new_db_guid = match char_db.query(&max_guid_stmt).await {
                    Ok(r) => r.try_read::<u64>(0).unwrap_or(0) + 1,
                    Err(_) => 1,
                };
                let new_item_guid = ObjectGuid::create_item(self.realm_id(), new_db_guid as i64);
                let cloned_item =
                    runtime_item.clone_item_for_store(new_item_guid, Some(player_guid), amount);
                let cloned_data = cloned_item.data();
                let charges = item_spell_charges_db_string(&cloned_data.spell_charges);

                let mut ins_item = char_db.prepare(CharStatements::INS_ITEM_INSTANCE_CLONE);
                ins_item.set_u64(0, new_db_guid);
                ins_item.set_u32(1, item.entry_id);
                ins_item.set_u64(2, player_guid.counter() as u64);
                ins_item.set_u64(3, cloned_data.creator.counter() as u64);
                ins_item.set_u64(4, cloned_data.gift_creator.counter() as u64);
                ins_item.set_u32(5, cloned_item.count());
                ins_item.set_u32(6, cloned_data.expiration);
                ins_item.set_string(7, charges);
                ins_item.set_u32(8, cloned_data.dynamic_flags);
                ins_item.set_u32(9, cloned_data.durability);
                ins_item.set_u32(10, cloned_data.create_played_time);
                ins_item.set_i32(11, cloned_data.random_properties_id);
                ins_item.set_i32(12, cloned_data.property_seed);
                ins_item.set_u8(13, u8::try_from(cloned_data.context).unwrap_or(0));
                tx.append(ins_item);

                let mut ins_inv = char_db.prepare(CharStatements::INS_CHAR_INVENTORY);
                ins_inv.set_u64(0, player_guid.counter() as u64);
                ins_inv.set_u8(1, buyback_slot);
                ins_inv.set_u64(2, new_db_guid);
                tx.append(ins_inv);

                new_buyback_stack = Some((new_db_guid, cloned_item, remaining));
            }
            SellItemAmountAction::Invalid => unreachable!(),
        }

        // ── Add gold + save to DB ──
        let mut upd_money = char_db.prepare(CharStatements::UPD_CHAR_MONEY);
        upd_money.set_u64(0, new_gold);
        upd_money.set_u64(1, player_guid.counter() as u64);
        tx.append(upd_money);

        if let Err(e) = char_db.commit_transaction(tx).await {
            warn!("SellItem: transaction failed: {e}");
            self.send_sell_error(
                SellResult::CantSellItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        }

        self.apply_player_money_change_like_cpp(old_gold, new_gold)
            .await;
        if let Some(old_buyback) = old_buyback {
            self.remove_buyback_item_like_cpp(buyback_slot);
            self.remove_inventory_item_object(old_buyback.guid);
        }
        self.set_buyback_slot_metadata_like_cpp(buyback_slot, buyback_price, buyback_timestamp);
        self.advance_buyback_slot_cpp();

        let mut created_buyback_item = None;
        let mut stack_update = None;
        if let Some((new_db_guid, cloned_item, remaining)) = new_buyback_stack {
            let new_item_guid = cloned_item.object().guid();
            let stack_count = cloned_item.count();
            let durability = cloned_item.data().durability;
            let max_durability = cloned_item.data().max_durability;
            self.update_inventory_item_object_like_cpp(item.guid, |item_object| {
                item_object.set_count(remaining);
            });
            stack_update = Some((item.guid, remaining));
            self.insert_buyback_item_like_cpp(
                buyback_slot,
                InventoryItem {
                    guid: new_item_guid,
                    entry_id: item.entry_id,
                    db_guid: new_db_guid,
                    inventory_type: item.inventory_type,
                },
            );
            self.insert_inventory_item_object(cloned_item);
            self.set_inventory_item_object_slot(new_item_guid, buyback_slot);
            created_buyback_item = Some((new_item_guid, stack_count, durability, max_durability));
        } else {
            self.remove_inventory_item_like_cpp(slot);
            self.insert_buyback_item_like_cpp(
                buyback_slot,
                InventoryItem {
                    guid: item.guid,
                    entry_id: item.entry_id,
                    db_guid: item.db_guid,
                    inventory_type: item.inventory_type,
                },
            );
            self.set_inventory_item_object_slot(item.guid, buyback_slot);
        }
        self.sync_object_accessor_player();

        info!(
            "SellItem: player {:?} sold {}x item {} from slot {} for {} copper (total: {})",
            player_guid,
            sold_count,
            item.entry_id,
            slot,
            money,
            self.player_gold_like_cpp()
        );

        if let Some((item_guid, stack_count, durability, max_durability)) = created_buyback_item {
            self.send_packet(&UpdateObject::create_items(
                vec![ItemCreateData {
                    item_guid,
                    entry_id: item.entry_id as i32,
                    owner_guid: player_guid,
                    contained_in: player_guid,
                    stack_count,
                    dynamic_flags: 0,
                    durability,
                    max_durability,
                    random_properties_seed: 0,
                    random_properties_id: 0,
                    context: 0,
                }],
                map_id,
            ));
        }
        if let Some((item_guid, new_count)) = stack_update {
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid, map_id, new_count,
            ));
        }

        let mut inv_slot_changes = Vec::new();
        if matches!(sell_amount, SellItemAmountAction::FullStack { .. }) {
            inv_slot_changes.push((slot, ObjectGuid::EMPTY));
        }
        let buyback_guid = self
            .buyback_items_like_cpp()
            .get(&buyback_slot)
            .map(|item| item.guid)
            .unwrap_or(ObjectGuid::EMPTY);
        inv_slot_changes.push((buyback_slot, buyback_guid));
        self.send_player_values_update_from_entity_bridge(
            &inv_slot_changes,
            &[],
            &[],
            &[(buyback_slot, buyback_price, buyback_timestamp)],
            Some(self.player_gold_like_cpp()),
        );
    }

    /// Handle CMSG_ITEM_PURCHASE_REFUND.
    ///
    /// C++ ref: `ItemHandler.HandleItemRefund` -> `Player::RefundItem`.
    pub async fn handle_item_purchase_refund(&mut self, refund: ItemPurchaseRefund) {
        const REFUND_RESULT_OK: u8 = 0;
        const REFUND_RESULT_ERR_GENERIC: u8 = 10;

        #[derive(Debug, Clone)]
        struct PlannedNewStack {
            slot: u8,
            entry_id: u32,
            count: u32,
            max_durability: u32,
        }

        let player_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let map_id = self.player_map_id_like_cpp();

        let Some((refund_slot, refund_inv_item)) = self
            .inventory_items_like_cpp()
            .iter()
            .find(|(_, item)| item.guid == refund.item_guid)
            .map(|(&slot, item)| (slot, item.clone()))
        else {
            warn!(
                "ItemPurchaseRefund: item {:?} not in inventory",
                refund.item_guid
            );
            return;
        };

        let Some(refund_item) = self
            .inventory_item_objects_like_cpp()
            .get(&refund.item_guid)
            .cloned()
        else {
            warn!(
                "ItemPurchaseRefund: item {:?} missing runtime object",
                refund.item_guid
            );
            return;
        };

        if self.is_active_loot_guid(refund.item_guid)
            || item_is_currently_looted_like_cpp(&refund_item)
        {
            return;
        }
        if !refund_item.is_refundable() {
            return;
        }

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0);

        if refund_item.is_refund_expired_at(now_secs)
            || refund_item.refund_recipient() != player_guid
        {
            let new_flags = refund_item.item_flags_bits() & !ItemFieldFlags::REFUNDABLE.bits();
            let mut tx = SqlTransaction::new();
            append_item_refund_clear_statements(
                char_db.as_ref(),
                &mut tx,
                refund_inv_item.db_guid,
                new_flags,
            );
            if let Err(e) = char_db.commit_transaction(tx).await {
                warn!("ItemPurchaseRefund: refund cleanup transaction failed: {e}");
                return;
            }

            self.update_inventory_item_object_like_cpp(refund.item_guid, |item| {
                item.set_not_refundable();
            });
            self.sync_object_accessor_player();
            self.send_packet(&ItemExpirePurchaseRefund {
                item_guid: refund.item_guid,
            });

            if refund_item.is_refund_expired_at(now_secs) {
                self.send_packet(&ItemPurchaseRefundResult {
                    item_guid: refund.item_guid,
                    result: REFUND_RESULT_ERR_GENERIC,
                    contents: None,
                });
            }
            return;
        }

        let Some(extended_cost) = self
            .item_extended_cost_store()
            .and_then(|store| store.get(refund_item.paid_extended_cost()))
            .copied()
        else {
            return;
        };

        let contents = super::misc::item_purchase_contents_from_extended_cost(
            &extended_cost,
            refund_item.paid_money(),
        );

        let mut item_costs = Vec::new();
        for i in 0..5 {
            let item_id = extended_cost.item_id[i] as u32;
            let count = extended_cost.item_count[i] as u32;
            if item_id != 0 && count != 0 {
                item_costs.push((item_id, count));
            }
        }

        let mut currency_costs = Vec::new();
        for i in 0..5 {
            let season_earned = match i {
                0 => extended_cost
                    .flags
                    .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_1),
                1 => extended_cost
                    .flags
                    .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_2),
                2 => extended_cost
                    .flags
                    .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_3),
                3 => extended_cost
                    .flags
                    .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_4),
                4 => extended_cost
                    .flags
                    .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_5),
                _ => false,
            };
            if season_earned {
                continue;
            }
            let currency_id = extended_cost.currency_id[i] as u32;
            let count = extended_cost.currency_count[i] as u32;
            if currency_id != 0 && count != 0 {
                currency_costs.push((currency_id, count));
            }
        }

        let mut planned_existing_counts =
            std::collections::HashMap::<u8, (ObjectGuid, u64, u32)>::new();
        let mut planned_new_stacks = Vec::<PlannedNewStack>::new();
        for &(entry_id, count) in &item_costs {
            let (store_result, store_dest, _) =
                match self.plan_store_new_direct_inventory_item(entry_id, count) {
                    Some(plan) => plan,
                    None => {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    }
                };
            if store_result != InventoryResult::Ok {
                self.send_packet(&ItemPurchaseRefundResult {
                    item_guid: refund.item_guid,
                    result: REFUND_RESULT_ERR_GENERIC,
                    contents: Some(contents),
                });
                return;
            }

            for dest in store_dest {
                let bag = (dest.pos >> 8) as u8;
                let slot = (dest.pos & 0x00FF) as u8;
                if bag != u8::from(INVENTORY_SLOT_BAG_0) {
                    self.send_packet(&ItemPurchaseRefundResult {
                        item_guid: refund.item_guid,
                        result: REFUND_RESULT_ERR_GENERIC,
                        contents: Some(contents),
                    });
                    return;
                }

                let max_stack = self
                    .item_storage_template(entry_id)
                    .map(|template| template.max_stack_size)
                    .unwrap_or(1)
                    .max(1);

                if let Some(existing) = self.inventory_items_like_cpp().get(&slot) {
                    let Some(existing_object) =
                        self.inventory_item_objects_like_cpp().get(&existing.guid)
                    else {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    };
                    let base_count = planned_existing_counts
                        .get(&slot)
                        .map(|(_, _, count)| *count)
                        .unwrap_or_else(|| existing_object.count());
                    let new_count = base_count.saturating_add(dest.count);
                    if existing.entry_id != entry_id || new_count > max_stack {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    }
                    planned_existing_counts
                        .insert(slot, (existing.guid, existing.db_guid, new_count));
                    continue;
                }

                if let Some(new_stack) = planned_new_stacks
                    .iter_mut()
                    .find(|stack| stack.slot == slot)
                {
                    if new_stack.entry_id == entry_id
                        && new_stack.count.saturating_add(dest.count) <= max_stack
                    {
                        new_stack.count = new_stack.count.saturating_add(dest.count);
                        continue;
                    }

                    let backpack_end =
                        INVENTORY_SLOT_ITEM_START.saturating_add(INVENTORY_DEFAULT_SIZE);
                    let Some(alt_slot) = (INVENTORY_SLOT_ITEM_START..backpack_end).find(|slot| {
                        !self.inventory_items_like_cpp().contains_key(slot)
                            && !planned_new_stacks.iter().any(|stack| stack.slot == *slot)
                    }) else {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    };
                    let Some((InventoryResult::Ok, alt_dest, _)) = self
                        .plan_store_new_direct_inventory_item_at(
                            entry_id,
                            dest.count,
                            u8::from(INVENTORY_SLOT_BAG_0),
                            alt_slot,
                        )
                    else {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    };
                    if alt_dest.len() != 1 || (alt_dest[0].pos & 0x00FF) as u8 != alt_slot {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    }
                    planned_new_stacks.push(PlannedNewStack {
                        slot: alt_slot,
                        entry_id,
                        count: dest.count,
                        max_durability: self.item_template_max_durability(entry_id),
                    });
                    continue;
                }

                planned_new_stacks.push(PlannedNewStack {
                    slot,
                    entry_id,
                    count: dest.count,
                    max_durability: self.item_template_max_durability(entry_id),
                });
            }
        }

        let mut tx = SqlTransaction::new();
        let mut del_refund = char_db.prepare(CharStatements::DEL_ITEM_REFUND_INSTANCE);
        del_refund.set_u64(0, refund_inv_item.db_guid);
        tx.append(del_refund);

        let mut del_inv = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
        del_inv.set_u64(0, player_guid.counter() as u64);
        del_inv.set_u64(1, refund_inv_item.db_guid);
        tx.append(del_inv);

        let mut del_item = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
        del_item.set_u64(0, refund_inv_item.db_guid);
        tx.append(del_item);

        let old_money = self.player_gold_like_cpp();
        let money_gain = player_money_gain_like_cpp(old_money, refund_item.paid_money());
        let money_overflow = money_gain.is_none();
        let new_gold = money_gain.unwrap_or(old_money);
        let mut upd_money = char_db.prepare(CharStatements::UPD_CHAR_MONEY);
        upd_money.set_u64(0, new_gold);
        upd_money.set_u64(1, player_guid.counter() as u64);
        tx.append(upd_money);

        for &(_, db_guid, new_count) in planned_existing_counts.values() {
            let mut upd_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
            upd_count.set_u32(0, new_count);
            upd_count.set_u64(1, db_guid);
            tx.append(upd_count);
        }

        let realm_id = self.realm_id();
        let mut created_new_stacks = Vec::new();
        if !planned_new_stacks.is_empty() {
            let max_guid_stmt = char_db.prepare(CharStatements::SEL_MAX_ITEM_GUID);
            let mut next_item_guid = match char_db.query(&max_guid_stmt).await {
                Ok(r) => r.try_read::<u64>(0).unwrap_or(0) + 1,
                Err(_) => 1,
            };

            for stack in &planned_new_stacks {
                let db_guid = next_item_guid;
                next_item_guid += 1;
                let item_guid = ObjectGuid::create_item(realm_id, db_guid as i64);

                let mut ins_item = char_db.prepare(CharStatements::INS_ITEM_INSTANCE);
                ins_item.set_u64(0, db_guid);
                ins_item.set_u32(1, stack.entry_id);
                ins_item.set_u64(2, player_guid.counter() as u64);
                ins_item.set_u32(3, stack.count);
                ins_item.set_u32(4, stack.max_durability);
                tx.append(ins_item);

                let mut ins_inv = char_db.prepare(CharStatements::INS_CHAR_INVENTORY);
                ins_inv.set_u64(0, player_guid.counter() as u64);
                ins_inv.set_u8(1, stack.slot);
                ins_inv.set_u64(2, db_guid);
                tx.append(ins_inv);

                created_new_stacks.push((stack.clone(), db_guid, item_guid));
            }
        }

        let currency_snapshot = self.player_currencies_like_cpp().clone();
        let mut currency_deltas = Vec::new();
        for &(currency_id, amount) in &currency_costs {
            match self.add_currency_item_refund(currency_id, amount) {
                Ok(Some(delta)) => currency_deltas.push(delta),
                Ok(None) => {}
                Err(()) => {
                    self.set_player_currencies_like_cpp(currency_snapshot);
                    self.send_packet(&ItemPurchaseRefundResult {
                        item_guid: refund.item_guid,
                        result: REFUND_RESULT_ERR_GENERIC,
                        contents: Some(contents),
                    });
                    return;
                }
            }
        }
        self.append_player_currency_save_statements(&mut tx, player_guid.counter() as u64);

        if let Err(e) = char_db.commit_transaction(tx).await {
            self.set_player_currencies_like_cpp(currency_snapshot);
            warn!("ItemPurchaseRefund: refund transaction failed: {e}");
            self.send_packet(&ItemPurchaseRefundResult {
                item_guid: refund.item_guid,
                result: REFUND_RESULT_ERR_GENERIC,
                contents: Some(contents),
            });
            return;
        }

        self.apply_player_money_change_like_cpp(old_money, new_gold)
            .await;
        if money_overflow {
            self.send_equip_error(InventoryResult::TooMuchGold, None, None, 0, 0);
        }
        self.remove_inventory_item_like_cpp(refund_slot);
        self.remove_inventory_item_object(refund.item_guid);

        for &(item_guid, _, new_count) in planned_existing_counts.values() {
            self.update_inventory_item_object_like_cpp(item_guid, |item| {
                item.set_count(new_count);
            });
        }

        for (stack, db_guid, item_guid) in &created_new_stacks {
            self.insert_inventory_item_like_cpp(
                stack.slot,
                InventoryItem {
                    guid: *item_guid,
                    entry_id: stack.entry_id,
                    db_guid: *db_guid,
                    inventory_type: self.item_template_inventory_type(stack.entry_id),
                },
            );
            let item_object = self.make_inventory_item_object(
                *item_guid,
                stack.entry_id,
                player_guid,
                stack.count,
                stack.max_durability,
                ItemContext::None,
                stack.slot,
            );
            self.insert_inventory_item_object(item_object);
        }
        self.sync_object_accessor_player();

        self.send_packet(&ItemPurchaseRefundResult {
            item_guid: refund.item_guid,
            result: REFUND_RESULT_OK,
            contents: Some(contents),
        });
        self.send_packet(&ItemExpirePurchaseRefund {
            item_guid: refund.item_guid,
        });

        for delta in currency_deltas {
            let Some(type_id) = i32::try_from(delta.currency_id).ok() else {
                continue;
            };
            let Some(quantity) = i32::try_from(delta.quantity).ok() else {
                continue;
            };
            let Some(amount) = i32::try_from(delta.amount).ok() else {
                continue;
            };
            self.send_packet(&SetCurrency::item_refund_gain(
                type_id,
                quantity,
                amount,
                delta
                    .weekly_quantity
                    .and_then(|value| i32::try_from(value).ok()),
                delta
                    .max_quantity
                    .and_then(|value| i32::try_from(value).ok()),
                delta
                    .total_earned
                    .and_then(|value| i32::try_from(value).ok()),
                delta.suppress_chat_log,
            ));
        }

        if !created_new_stacks.is_empty() {
            let item_creates = created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| ItemCreateData {
                    item_guid: *item_guid,
                    entry_id: stack.entry_id as i32,
                    owner_guid: player_guid,
                    contained_in: player_guid,
                    stack_count: stack.count,
                    dynamic_flags: 0,
                    durability: stack.max_durability,
                    max_durability: stack.max_durability,
                    random_properties_seed: 0,
                    random_properties_id: 0,
                    context: 0,
                })
                .collect();
            self.send_packet(&UpdateObject::create_items(item_creates, map_id));
        }

        for &(item_guid, _, new_count) in planned_existing_counts.values() {
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid, map_id, new_count,
            ));
        }

        let mut changed_slots = Vec::new();
        changed_slots.push((refund_slot, ObjectGuid::EMPTY));
        changed_slots.extend(
            created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| (stack.slot, *item_guid)),
        );
        self.send_player_values_update_from_entity_bridge(
            &changed_slots,
            &[],
            &[],
            &[],
            Some(self.player_gold_like_cpp()),
        );

        if refund_slot < 19 {
            self.send_stat_update();
        }
    }

    /// Handle CMSG_QUEST_GIVER_STATUS_MULTIPLE_QUERY — client asks quest status for visible questgivers.
    ///
    /// C++ anchors:
    /// - `Player::SendQuestGiverStatusMultiple`, `Player.cpp:16804-16837`.
    /// - `QuestGiverStatusMultiple::Write`, `QuestPackets.cpp:64-74`.
    ///
    /// Ownership/sync: represented `client_visible_guids_like_cpp` + canonical map access + read-only
    /// `QuestStore` relations -> one outbound packet only. This handler must not mutate map,
    /// QuestStore, ObjectAccessor/GameEvent, or player state. Exact Creature hostility/faction remains
    /// a documented gap; represented Creature NPC QUEST_GIVER flag is enforced when available.
    pub async fn handle_quest_giver_status_multiple_query(&mut self) {
        trace!(
            "QuestGiverStatusMultipleQuery from account {}",
            self.account_id
        );

        let visible_guids: Vec<ObjectGuid> =
            self.client_visible_guids_like_cpp.iter().copied().collect();
        let statuses = self.collect_quest_giver_status_multiple_like_cpp(visible_guids);
        self.send_packet(&QuestGiverStatusMultiple { statuses });
    }

    /// Handle CMSG_QUEST_GIVER_STATUS_TRACKED_QUERY — client supplies questgiver GUIDs to query.
    ///
    /// C++ anchors:
    /// - `QuestGiverStatusTrackedQuery::Read`, `QuestPackets.cpp:40-54`.
    /// - `WorldSession::HandleQuestgiverStatusTrackedQueryOpcode`, `QuestHandler.cpp:775-778`.
    /// - `Player::SendQuestGiverStatusMultiple`, `Player.cpp:16809-16837`.
    ///
    /// Ownership/sync: client packet GUID set -> represented canonical Creature/GameObject access +
    /// read-only `QuestStore` status -> one outbound packet only. This must not read the visible GUID
    /// cache and must not mutate map, QuestStore, ObjectAccessor/GameEvent, player quest state, or
    /// represented visibility state.
    pub async fn handle_quest_giver_status_tracked_query(&mut self, mut pkt: WorldPacket) {
        trace!(
            "QuestGiverStatusTrackedQuery from account {}",
            self.account_id
        );

        let guid_count = match pkt.read_uint32() {
            Ok(guid_count) => guid_count,
            Err(e) => {
                warn!("Malformed QuestGiverStatusTrackedQuery count: {e}");
                return;
            }
        };

        if guid_count > QUEST_GIVER_STATUS_TRACKED_QUERY_MAX_GUIDS_LIKE_CPP {
            warn!(
                guid_count,
                max = QUEST_GIVER_STATUS_TRACKED_QUERY_MAX_GUIDS_LIKE_CPP,
                "QuestGiverStatusTrackedQuery exceeds C++ max capacity"
            );
            return;
        }

        let mut quest_giver_guids = HashSet::with_capacity(guid_count as usize);
        for _ in 0..guid_count {
            match pkt.read_packed_guid() {
                Ok(guid) => {
                    quest_giver_guids.insert(guid);
                }
                Err(e) => {
                    warn!("Malformed QuestGiverStatusTrackedQuery packed GUID: {e}");
                    return;
                }
            }
        }

        let statuses = self.collect_quest_giver_status_multiple_like_cpp(quest_giver_guids);
        self.send_packet(&QuestGiverStatusMultiple { statuses });
    }

    fn collect_quest_giver_status_multiple_like_cpp(
        &self,
        guids: impl IntoIterator<Item = ObjectGuid>,
    ) -> Vec<(ObjectGuid, u64)> {
        let mut statuses = Vec::new();

        for guid in guids {
            if guid.is_any_type_creature() {
                let Some(access) = self.canonical_creature_access_like_cpp(guid) else {
                    continue;
                };
                if (access.npc_flags & NPCFlags1::QUEST_GIVER.bits()) == 0 {
                    continue;
                }

                let status = self.get_represented_quest_giver_status_like_cpp(
                    RepresentedQuestGiverStatusSourceLikeCpp::Creature {
                        entry: access.entry,
                    },
                );
                statuses.push((guid, status));
                continue;
            }

            if guid.is_game_object() {
                let Some(access) = self.canonical_gameobject_access_like_cpp(guid) else {
                    continue;
                };
                let Some(state) = self.represented_gameobject_use_states.get(&guid) else {
                    continue;
                };
                if state.go_type.map(u32::from) != Some(GAMEOBJECT_TYPE_QUESTGIVER) {
                    continue;
                }

                let status = self.get_represented_quest_giver_status_like_cpp(
                    RepresentedQuestGiverStatusSourceLikeCpp::GameObject {
                        entry: access.entry,
                    },
                );
                statuses.push((guid, status));
            }
        }

        statuses
    }

    /// Send SMSG_QUEST_GIVER_STATUS for a single NPC.
    fn send_quest_giver_status(&self, guid: ObjectGuid, status: u32) {
        use wow_constants::ServerOpcodes;
        let mut pkt = wow_packet::WorldPacket::new_server(ServerOpcodes::QuestGiverStatus);
        pkt.write_packed_guid(&guid);
        pkt.write_uint32(status);
        self.send_raw_packet(&pkt.into_data());
    }

    // ── Item equip/swap handlers ─────────────────────────────────────

    /// Handle CMSG_SWAP_INV_ITEM: drag-and-drop item between two inventory slots.
    pub async fn handle_swap_inv_item(&mut self, swap: SwapInvItem) {
        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => {
                warn!("handle_swap_inv_item: no player_guid");
                return;
            }
        };

        let src = swap.src_slot;
        let dst = swap.dst_slot;
        debug!(
            "SwapInvItem: slot {} ↔ slot {} for {:?}",
            src, dst, player_guid
        );

        // Both slots must be in valid range (0-140)
        if src as usize >= 141 || dst as usize >= 141 {
            self.send_packet(&InventoryChangeFailure::error(
                InventoryResult::InternalBagError,
            ));
            return;
        }

        // Can't swap to same slot
        if src == dst {
            return;
        }

        let src_item = self.inventory_items_like_cpp().get(&src).cloned();
        let dst_item = self.inventory_items_like_cpp().get(&dst).cloned();

        // At least one slot must have an item
        if src_item.is_none() && dst_item.is_none() {
            self.send_packet(&InventoryChangeFailure::error(InventoryResult::SlotEmpty));
            return;
        }

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        // Perform the swap in memory
        if let Some(ref item) = src_item {
            self.insert_inventory_item_like_cpp(dst, item.clone());
            self.set_inventory_item_object_slot(item.guid, dst);
        } else {
            self.remove_inventory_item_like_cpp(dst);
        }
        if let Some(ref item) = dst_item {
            self.insert_inventory_item_like_cpp(src, item.clone());
            self.set_inventory_item_object_slot(item.guid, src);
        } else {
            self.remove_inventory_item_like_cpp(src);
        }
        self.sync_object_accessor_player();

        // Update DB
        if let Some(ref item) = src_item {
            let mut stmt = char_db.prepare(CharStatements::UPD_CHAR_INVENTORY_SLOT);
            stmt.set_u8(0, dst);
            stmt.set_u64(1, player_guid.counter() as u64);
            stmt.set_u64(2, item.db_guid);
            let _ = char_db.execute(&stmt).await;
        }
        if let Some(ref item) = dst_item {
            let mut stmt = char_db.prepare(CharStatements::UPD_CHAR_INVENTORY_SLOT);
            stmt.set_u8(0, src);
            stmt.set_u64(1, player_guid.counter() as u64);
            stmt.set_u64(2, item.db_guid);
            let _ = char_db.execute(&stmt).await;
        }

        let source_moved_bag_has_active_loot = is_represented_bag_slot(src)
            && src_item.as_ref().is_some_and(|item| {
                self.represented_bag_contains_active_item_loot_like_cpp(item.guid)
            });
        let destination_moved_bag_has_active_loot = is_represented_bag_slot(dst)
            && dst_item.as_ref().is_some_and(|item| {
                self.represented_bag_contains_active_item_loot_like_cpp(item.guid)
            });
        if source_moved_bag_has_active_loot || destination_moved_bag_has_active_loot {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }

        // Build VALUES update changes
        let mut inv_slot_changes = Vec::new();
        let mut visible_item_changes = Vec::new();
        let mut virtual_item_changes = Vec::new();

        // Source slot
        let src_new_guid = if let Some(ref item) = dst_item {
            item.guid
        } else {
            ObjectGuid::EMPTY
        };
        inv_slot_changes.push((src, src_new_guid));

        // Destination slot
        let dst_new_guid = if let Some(ref item) = src_item {
            item.guid
        } else {
            ObjectGuid::EMPTY
        };
        inv_slot_changes.push((dst, dst_new_guid));

        // VisibleItems: equipment slots 0-18
        for &slot in &[src, dst] {
            if (slot as usize) < 19 {
                let (item_id, app, vis) = match self.inventory_items_like_cpp().get(&slot) {
                    Some(item) => (item.entry_id as i32, 0u16, 0u16),
                    None => (0, 0, 0),
                };
                visible_item_changes.push((slot, item_id, app, vis));
            }
        }

        // VirtualItems: weapon slots 15/16/17 → indices 0/1/2
        for &slot in &[src, dst] {
            if slot >= 15 && slot <= 17 {
                let idx = slot - 15;
                let (item_id, app, vis) = match self.inventory_items_like_cpp().get(&slot) {
                    Some(item) => (item.entry_id as i32, 0u16, 0u16),
                    None => (0, 0, 0),
                };
                virtual_item_changes.push((idx, item_id, app, vis));
            }
        }

        self.send_player_values_update_from_entity_bridge(
            &inv_slot_changes,
            &visible_item_changes,
            &virtual_item_changes,
            &[],
            None,
        );

        // If any affected slot is a gear slot (0-18), recalculate and send stats
        if src < 19 || dst < 19 {
            self.send_stat_update();
        }

        info!(
            "Swapped items: slot {} ↔ slot {} for {:?}",
            src, dst, player_guid
        );
    }

    /// Handle CMSG_AUTO_EQUIP_ITEM: right-click to auto-equip/unequip an item.
    pub async fn handle_auto_equip_item(&mut self, equip: AutoEquipItem) {
        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        let src_slot = equip.slot;
        debug!(
            "AutoEquipItem: slot {} (pack_slot {}) for {:?}",
            src_slot, equip.pack_slot, player_guid
        );

        let src_item = match self.inventory_items_like_cpp().get(&src_slot).cloned() {
            Some(item) => item,
            None => {
                self.send_packet(&InventoryChangeFailure::error(InventoryResult::SlotEmpty));
                return;
            }
        };

        // Determine destination slot
        let dst_slot = if src_slot < 19 {
            // Already equipped → find first free backpack slot to unequip
            match self.find_free_backpack_slot() {
                Some(slot) => slot,
                None => {
                    self.send_packet(&InventoryChangeFailure::error(InventoryResult::InvFull));
                    return;
                }
            }
        } else {
            // In backpack → find target equipment slot using ItemTemplate::GetInventoryType().
            let inv_type = match src_item.inventory_type {
                Some(t) => t,
                None => {
                    warn!(
                        "AutoEquipItem: no inventory_type for entry {} — not in cache",
                        src_item.entry_id
                    );
                    self.send_packet(&InventoryChangeFailure::error(
                        InventoryResult::NotEquippable,
                    ));
                    return;
                }
            };
            // Build occupied map from currently equipped gear and bag slots.
            let occupied: std::collections::HashMap<u8, ()> = self
                .inventory_items_like_cpp()
                .keys()
                .filter(|&&s| s < 19 || (30..34).contains(&s))
                .map(|&s| (s, ()))
                .collect();
            match equip_slot_for_inventory_type(inv_type, &occupied) {
                Some(slot) => slot,
                None => {
                    warn!(
                        "AutoEquipItem: inv_type {} has no valid equipment slot",
                        inv_type
                    );
                    self.send_packet(&InventoryChangeFailure::error(
                        InventoryResult::NotEquippable,
                    ));
                    return;
                }
            }
        };

        // Perform the swap using the same logic as SwapInvItem
        let swap = SwapInvItem {
            inv_update: InvUpdate { items: Vec::new() },
            src_slot,
            dst_slot,
        };
        self.handle_swap_inv_item(swap).await;
    }

    /// Handle CMSG_SWAP_ITEM: container-aware swap between two positions.
    ///
    /// C# reads: ContainerSlotB, ContainerSlotA, SlotB, SlotA.
    /// ContainerSlot=255 means player's direct inventory.
    /// For simplicity, we only support 255 (player inventory) for now.
    pub async fn handle_swap_item(&mut self, swap: wow_packet::packets::item::SwapItem) {
        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        debug!(
            "SwapItem: A=({},{}) B=({},{}) for {:?}",
            swap.container_slot_a, swap.slot_a, swap.container_slot_b, swap.slot_b, player_guid
        );

        // Only support player's direct inventory (container=255) for now
        if swap.container_slot_a != 255 || swap.container_slot_b != 255 {
            warn!("SwapItem with non-255 containers not supported yet");
            self.send_packet(&InventoryChangeFailure::error(
                InventoryResult::InternalBagError,
            ));
            return;
        }

        // Delegate to the existing swap logic
        let inner = SwapInvItem {
            inv_update: InvUpdate { items: Vec::new() },
            src_slot: swap.slot_a,
            dst_slot: swap.slot_b,
        };
        self.handle_swap_inv_item(inner).await;
    }

    /// Handle CMSG_AUTO_STORE_BAG_ITEM: right-click to store item in bag/backpack.
    ///
    /// This is used by the client when right-clicking equipped items to unequip them,
    /// or to move items between containers.
    pub async fn handle_auto_store_bag_item(
        &mut self,
        store: wow_packet::packets::item::AutoStoreBagItem,
    ) {
        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        debug!(
            "AutoStoreBagItem: src container={} slot={} dst container={} for {:?}",
            store.container_slot_a, store.slot_a, store.container_slot_b, player_guid
        );

        // Only support player's direct inventory (container=255) for now
        if store.container_slot_a != 255 {
            warn!("AutoStoreBagItem with non-255 source container not supported yet");
            self.send_packet(&InventoryChangeFailure::error(
                InventoryResult::InternalBagError,
            ));
            return;
        }

        let src_slot = store.slot_a;

        // Check source has an item
        if !self.inventory_items_like_cpp().contains_key(&src_slot) {
            self.send_packet(&InventoryChangeFailure::error(InventoryResult::SlotEmpty));
            return;
        }

        // Find a free backpack slot
        let dst_slot = match self.find_free_backpack_slot() {
            Some(slot) => slot,
            None => {
                self.send_packet(&InventoryChangeFailure::error(InventoryResult::InvFull));
                return;
            }
        };

        // Delegate to the existing swap logic (move from src to empty dst)
        let inner = SwapInvItem {
            inv_update: InvUpdate { items: Vec::new() },
            src_slot,
            dst_slot,
        };
        self.handle_swap_inv_item(inner).await;
    }

    /// Handle CMSG_DESTROY_ITEM: delete an item from inventory.
    pub async fn handle_destroy_item(
        &mut self,
        destroy: wow_packet::packets::item::DestroyItemPkt,
    ) {
        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        debug!(
            "DestroyItem: container={} slot={} count={} for {:?}",
            destroy.container_id, destroy.slot_num, destroy.count, player_guid
        );

        // Only support player's direct inventory (container=255) for now
        if destroy.container_id != 255 {
            warn!("DestroyItem with non-255 container not supported yet");
            self.send_packet(&InventoryChangeFailure::error(
                InventoryResult::InternalBagError,
            ));
            return;
        }

        let slot = destroy.slot_num;
        let item = match self.inventory_items_like_cpp().get(&slot).cloned() {
            Some(item) => item,
            None => {
                self.send_packet(&InventoryChangeFailure::error(InventoryResult::SlotEmpty));
                return;
            }
        };

        let runtime_item = self
            .inventory_item_objects_like_cpp()
            .get(&item.guid)
            .cloned();
        let item_proto = self.item_storage_template(item.entry_id);
        let unequip_result = self.can_destroy_direct_item_like_cpp(
            slot,
            runtime_item.as_ref(),
            item_proto.as_ref(),
            self.direct_item_contains_items(item.guid),
        );
        if unequip_result != InventoryResult::Ok {
            self.send_packet(&InventoryChangeFailure::error(unequip_result));
            return;
        }

        if self
            .item_template_flags(item.entry_id)
            .is_some_and(|flags| flags.contains(ItemFlags::NO_USER_DESTROY))
        {
            self.send_packet(&InventoryChangeFailure::error(
                InventoryResult::DropBoundItem,
            ));
            return;
        }

        // Delete from DB
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let count_action = runtime_item
            .as_ref()
            .map(|item_object| {
                destroy_item_count_action(
                    item_object.count(),
                    u32::try_from(destroy.count).unwrap_or(u32::MAX),
                )
            })
            .unwrap_or(DestroyItemCountAction::FullStack);

        if let DestroyItemCountAction::PartialStack { new_count } = count_action {
            let mut upd_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
            upd_count.set_u32(0, new_count);
            upd_count.set_u64(1, item.db_guid);
            if let Err(e) = char_db.execute(&upd_count).await {
                warn!("DestroyItem: update partial stack count failed: {e}");
                self.send_packet(&InventoryChangeFailure::error(
                    InventoryResult::InternalBagError,
                ));
                return;
            }

            self.update_inventory_item_object_like_cpp(item.guid, |item_object| {
                item_object.set_count(new_count);
            });
            self.sync_object_accessor_player();
            self.send_packet(&UpdateObject::item_stack_count_update(
                item.guid,
                self.player_map_id_like_cpp(),
                new_count,
            ));
            info!(
                "Destroyed partial item entry={} at slot {} count={} for {:?}",
                item.entry_id, slot, destroy.count, player_guid
            );
            return;
        }

        let destroyed_entry_id = item.entry_id;
        if self
            .destroy_direct_inventory_full_stack_like_cpp(slot, item, runtime_item, "DestroyItem")
            .await
        {
            info!(
                "Destroyed item entry={} at slot {} for {:?}",
                destroyed_entry_id, slot, player_guid
            );
        }
    }

    /// C++ `Player::DestroyItem(..., update=true)` for a direct inventory full-stack item.
    pub(crate) async fn destroy_direct_inventory_full_stack_like_cpp(
        &mut self,
        slot: u8,
        item: crate::session::InventoryItem,
        runtime_item: Option<wow_entities::Item>,
        context: &str,
    ) -> bool {
        self.destroy_inventory_full_stack_by_pos_like_cpp(
            INVENTORY_SLOT_BAG_0,
            slot,
            item,
            runtime_item,
            context,
        )
        .await
    }

    /// C++ `Player::DestroyItem(bag, slot, update=true)` for a full-stack item.
    pub(crate) async fn destroy_inventory_full_stack_by_pos_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item: crate::session::InventoryItem,
        runtime_item: Option<wow_entities::Item>,
        context: &str,
    ) -> bool {
        let player_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return false,
        };
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return false,
        };

        let mut tx = SqlTransaction::new();
        let should_expire_refund = runtime_item
            .as_ref()
            .is_some_and(|item_object| item_object.is_refundable());
        if should_expire_refund {
            let mut del_refund = char_db.prepare(CharStatements::DEL_ITEM_REFUND_INSTANCE);
            del_refund.set_u64(0, item.db_guid);
            tx.append(del_refund);
        }

        let mut del_inv = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
        del_inv.set_u64(0, player_guid.counter() as u64);
        del_inv.set_u64(1, item.db_guid);
        tx.append(del_inv);

        let mut del_item = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
        del_item.set_u64(0, item.db_guid);
        tx.append(del_item);

        if let Err(e) = char_db.commit_transaction(tx).await {
            warn!("{context}: delete transaction failed: {e}");
            self.send_packet(&InventoryChangeFailure::error(
                InventoryResult::InternalBagError,
            ));
            return false;
        }

        self.remove_fully_looted_runtime_item(bag, slot, item.guid);

        if should_expire_refund {
            self.send_packet(&ItemExpirePurchaseRefund {
                item_guid: item.guid,
            });
        }

        if bag == INVENTORY_SLOT_BAG_0 {
            let inv_slot_changes = vec![(slot, ObjectGuid::EMPTY)];
            let mut visible_item_changes = Vec::new();
            let mut virtual_item_changes = Vec::new();

            if (slot as usize) < 19 {
                visible_item_changes.push((slot, 0i32, 0u16, 0u16));
            }
            if (15..=17).contains(&slot) {
                virtual_item_changes.push((slot - 15, 0i32, 0u16, 0u16));
            }

            self.send_player_values_update_from_entity_bridge(
                &inv_slot_changes,
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );

            if slot < 19 {
                self.send_stat_update();
            }
        }

        true
    }

    /// Find the first empty slot in the default backpack (slots 35-58).
    ///
    /// C# InventorySlots: ItemStart=35, ItemEnd=59 (24 backpack slots).
    fn find_free_backpack_slot(&self) -> Option<u8> {
        for slot in 35..59u8 {
            if !self.inventory_items_like_cpp().contains_key(&slot) {
                return Some(slot);
            }
        }
        None
    }

    /// Recalculate all stats from base + gear and send a VALUES update to the client.
    ///
    /// Called after equip/desequip changes to gear slots (0-18).
    pub(crate) fn send_stat_update(&self) {
        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        let race = self.player_race_like_cpp();
        let class = self.player_class_like_cpp();
        let level = self.player_level_like_cpp();

        if race == 0 || class == 0 || level == 0 {
            return; // Not fully logged in yet
        }

        // Sum gear stat bonuses from equipped items (slots 0-18)
        let (
            gear_stats,
            gear_ap,
            gear_rap,
            gear_health,
            gear_mana,
            gear_combat_ratings,
            gear_spell_power,
            gear_armor,
        ) = if let Some(iss) = self.item_stats_store() {
            let mut bonuses = [0i32; 5];
            let mut g_ap = 0i32;
            let mut g_rap = 0i32;
            let mut g_health = 0i32;
            let mut g_mana = 0i32;
            let mut g_cr = [0i32; 25];
            let mut g_sp = 0i32;
            let mut g_armor = 0i32;
            for (&slot, inv_item) in self.inventory_items_like_cpp() {
                if slot < 19 {
                    if let Some(entry) = iss.get(inv_item.entry_id) {
                        let [s, a, st, i, sp] = entry.base_stat_bonuses();
                        bonuses[0] += s;
                        bonuses[1] += a;
                        bonuses[2] += st;
                        bonuses[3] += i;
                        bonuses[4] += sp;
                        g_ap += entry.attack_power_bonus();
                        g_rap += entry.ranged_attack_power_bonus();
                        g_health += entry.health_bonus();
                        g_mana += entry.mana_bonus();
                        let cr = entry.combat_rating_bonuses();
                        for j in 0..25 {
                            g_cr[j] += cr[j];
                        }
                        g_sp += entry.spell_power_bonus();
                        g_armor += entry.armor;
                    }
                }
            }
            (bonuses, g_ap, g_rap, g_health, g_mana, g_cr, g_sp, g_armor)
        } else {
            ([0i32; 5], 0, 0, 0, 0, [0i32; 25], 0, 0)
        };

        // Compute total stats from base + gear
        let store = match self.player_stats() {
            Some(s) => s.clone(),
            None => return,
        };
        let ls = match store.get(race, class, level) {
            Some(ls) => ls,
            None => return,
        };

        let total_str = ls.strength as i32 + gear_stats[0];
        let total_agi = ls.agility as i32 + gear_stats[1];
        let total_sta = ls.stamina as i32 + gear_stats[2];
        let total_int = ls.intellect as i32 + gear_stats[3];
        let total_spi = ls.spirit as i32 + gear_stats[4];

        // MaxHealth from total STA
        let sta64 = total_sta as i64;
        let base_hp = ls.base_health as i64;
        let hp_bonus = sta64.min(20) + (sta64 - 20).max(0) * 10 + gear_health as i64;
        let max_health = base_hp + hp_bonus;

        // MaxMana from total INT
        let int64 = total_int as i64;
        let base_mp = ls.base_mana as i64;
        let mp_bonus = int64.min(20) + (int64 - 20).max(0) * 15 + gear_mana as i64;
        let max_mana = base_mp + mp_bonus;

        // Armor = AGI contribution + item armor
        let total_armor = total_agi * 2 + gear_armor;

        // Attack power
        let melee_ap = match class {
            1 | 2 | 6 => total_str * 2 - 20,
            3 | 4 => total_str + total_agi - 20,
            7 | 11 => total_str * 2 - 20,
            _ => (total_str - 10).max(0),
        }
        .max(0)
            + gear_ap;

        let ranged_ap = match class {
            3 => total_agi * 2 - 20,
            1 | 4 => total_agi - 10,
            _ => 0,
        }
        .max(0)
            + gear_rap;

        // Damage
        let ap_f = melee_ap as f32;
        let base_dmg = ap_f / 14.0 * 2.0;
        let min_d = (base_dmg + 1.0).max(1.0);
        let max_d = min_d + 1.0;

        let rap_f = ranged_ap as f32;
        let (min_rd, max_rd) = if rap_f > 0.0 {
            let rd = rap_f / 14.0 * 2.8;
            ((rd + 1.0).max(1.0), rd + 3.0)
        } else {
            (0.0, 0.0)
        };

        // Power for slot 0 (mana/rage/energy/runic)
        let power0 = match class {
            1 => 1000,            // Warrior: rage
            4 => 100,             // Rogue: energy
            6 => 1000,            // DK: runic power
            _ => max_mana as i32, // Casters: mana
        };

        // CombatRatings[32]: copy 25 used indices, rest 0
        let mut combat_ratings = [0i32; 32];
        combat_ratings[..25].copy_from_slice(&gear_combat_ratings);

        // ── Percentage calculations (WotLK level 80 formulas) ──
        let lvl = level as f32;

        // Crit from AGI: class-dependent AGI-to-crit ratio at level 80
        let agi_crit_ratio = match class {
            4 => 40.0,     // Rogue
            3 => 53.0,     // Hunter
            11 => 45.5,    // Druid
            7 => 80.0,     // Shaman
            2 => 59.5,     // Paladin
            1 | 6 => 62.5, // Warrior, DK
            _ => 80.0,     // Casters (Mage/Warlock/Priest)
        };
        let crit_from_agi = total_agi as f32 / agi_crit_ratio;

        // Crit from rating: ~45.91 rating per 1% at level 80
        let crit_rating_per_pct = if lvl >= 80.0 {
            45.91
        } else {
            (lvl * 0.574).max(1.0)
        };
        let crit_from_rating = gear_combat_ratings[8] as f32 / crit_rating_per_pct as f32;

        // Base crit varies by class (roughly)
        let base_crit = match class {
            4 => 3.5, // Rogue
            3 => 3.6, // Hunter
            1 => 3.2, // Warrior
            2 => 3.3, // Paladin
            6 => 3.2, // DK
            _ => 1.8, // Casters
        };
        let melee_crit_pct = (base_crit + crit_from_agi + crit_from_rating).min(100.0);

        // Spell crit from INT: class-dependent INT-to-spell-crit ratio
        let int_crit_ratio = match class {
            8 => 80.0,  // Mage
            9 => 82.0,  // Warlock
            5 => 80.0,  // Priest
            7 => 80.0,  // Shaman
            11 => 80.0, // Druid
            2 => 80.0,  // Paladin
            _ => 160.0, // Non-casters
        };
        let spell_crit_from_int = total_int as f32 / int_crit_ratio;
        let spell_crit_from_rating = gear_combat_ratings[10] as f32 / crit_rating_per_pct as f32;
        let base_spell_crit = match class {
            8 => 0.91,  // Mage
            9 => 1.70,  // Warlock
            5 => 1.24,  // Priest
            7 => 2.20,  // Shaman
            11 => 1.85, // Druid
            2 => 3.33,  // Paladin
            _ => 0.0,
        };
        let spell_crit_pct =
            (base_spell_crit as f32 + spell_crit_from_int + spell_crit_from_rating).min(100.0);

        // Dodge from AGI + rating
        let dodge_from_agi = total_agi as f32 / agi_crit_ratio; // simplified: same ratio
        let dodge_rating_per_pct = if lvl >= 80.0 {
            39.35
        } else {
            (lvl * 0.492).max(1.0)
        };
        let dodge_from_rating = gear_combat_ratings[2] as f32 / dodge_rating_per_pct as f32;
        let dodge_pct = (dodge_from_agi + dodge_from_rating + 5.0).min(100.0); // 5% base

        // Parry from STR + rating (for classes that can parry)
        let parry_rating_per_pct = if lvl >= 80.0 {
            49.18
        } else {
            (lvl * 0.615).max(1.0)
        };
        let parry_from_rating = gear_combat_ratings[3] as f32 / parry_rating_per_pct as f32;
        let parry_pct = match class {
            1 | 2 | 4 | 6 => (5.0 + parry_from_rating).min(100.0), // 5% base for melee
            _ => parry_from_rating.min(100.0),
        };

        // Block from rating (only shield users)
        let block_rating_per_pct = if lvl >= 80.0 {
            16.39
        } else {
            (lvl * 0.205).max(1.0)
        };
        let block_from_rating = gear_combat_ratings[4] as f32 / block_rating_per_pct as f32;
        let block_pct = match class {
            1 | 2 | 7 => (5.0 + block_from_rating).min(100.0), // 5% base
            _ => block_from_rating.min(100.0),
        };

        // SpellCritPercentage[7]: index 0=Physical (same as melee), 1-6=spell schools
        let mut spell_crit_arr = [0.0f32; 7];
        spell_crit_arr[0] = melee_crit_pct;
        for i in 1..7 {
            spell_crit_arr[i] = spell_crit_pct;
        }

        // ── Mana regen (WotLK spirit-based formula) ──
        // spirit_regen = 0.001 + sqrt(INT) * SPI * class_coeff
        let class_regen_coeff: f32 = match class {
            2 => 0.044,  // Paladin
            3 => 0.030,  // Hunter
            5 => 0.033,  // Priest
            7 => 0.044,  // Shaman
            8 => 0.035,  // Mage
            9 => 0.033,  // Warlock
            11 => 0.044, // Druid
            _ => 0.0,    // Warrior, Rogue, DK (no mana)
        };
        let spirit_regen = if class_regen_coeff > 0.0 {
            0.001 + (total_int as f32).max(0.0).sqrt() * total_spi as f32 * class_regen_coeff
        } else {
            0.0
        };

        // ── Expertise from rating ──
        // CombatRating::Expertise = index 23, 15.77 rating per expertise at level 80
        let expertise_rating_per_pct = if lvl >= 80.0 {
            15.77
        } else {
            (lvl * 0.197).max(1.0)
        };
        let expertise_value = gear_combat_ratings[23] as f32 / expertise_rating_per_pct;

        // ── Dodge/Parry from attribute (for tooltip display) ──
        let dodge_from_attr = dodge_from_agi;
        let parry_from_attr = 0.0; // No STR-to-parry in WotLK without talent

        // ── Shield block value (from STR, for shield classes) ──
        let shield_block_value = match class {
            1 | 2 | 7 => ((total_str as f32 * 0.5 - 10.0).max(0.0)) as i32,
            _ => 0,
        };

        let changes = PlayerStatChanges {
            health: max_health,
            max_health,
            min_damage: min_d,
            max_damage: max_d,
            base_mana: power0,
            base_health: max_health as i32,
            attack_power: melee_ap,
            ranged_attack_power: ranged_ap,
            min_ranged_damage: min_rd,
            max_ranged_damage: max_rd,
            power0,
            max_power0: power0,
            stats: [total_str, total_agi, total_sta, total_int, total_spi],
            stat_pos_buff: gear_stats,
            armor: total_armor,
            combat_ratings,
            spell_power: gear_spell_power,
            block_pct,
            dodge_pct,
            parry_pct,
            crit_pct: melee_crit_pct,
            ranged_crit_pct: melee_crit_pct,
            spell_crit_pct: spell_crit_arr,
            // Mana regen
            mana_regen: spirit_regen,
            mana_regen_combat: 0.0, // No talents = no in-combat spirit regen
            mana_regen_mp5: 0.0,    // No MP5 auras without talent system
            // Expertise
            mainhand_expertise: expertise_value,
            offhand_expertise: expertise_value,
            // Extended parent 38 fields
            ranged_expertise: 0.0,
            combat_rating_expertise: expertise_value,
            dodge_from_attr,
            parry_from_attr,
            offhand_crit_pct: melee_crit_pct,
            shield_block: shield_block_value,
            shield_block_crit_pct: 0.0,
            mod_healing_pct: 1.0,
            mod_healing_done_pct: 1.0,
            mod_periodic_healing_pct: 1.0,
            mod_spell_power_pct: 1.0,
        };

        debug!(
            "Stat update for {:?}: HP={} AP={} STR/AGI/STA/INT/SPI={:?} Armor={} SP={} Crit={:.1}% SCrit={:.1}% Dodge={:.1}% Parry={:.1}% Exp={:.1} ManaRegen={:.1}",
            player_guid,
            max_health,
            melee_ap,
            [total_str, total_agi, total_sta, total_int, total_spi],
            total_armor,
            gear_spell_power,
            melee_crit_pct,
            spell_crit_pct,
            dodge_pct,
            parry_pct,
            expertise_value,
            spirit_regen
        );

        let update =
            UpdateObject::player_stat_update(player_guid, self.player_map_id_like_cpp(), changes);
        self.send_packet(&update);
    }

    /// Update the realmcharacters count in the login database.
    ///
    /// Counts how many characters this account has on the character DB, then
    /// upserts the count into `realmcharacters` in the login DB.
    async fn update_realm_characters(&self, char_db: &wow_database::CharacterDatabase) {
        let login_db = match self.login_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        // Count characters for this account
        let mut count_stmt = char_db.prepare(CharStatements::SEL_SUM_CHARS);
        count_stmt.set_u32(0, self.account_id);

        let num_chars: u8 = match char_db.query(&count_stmt).await {
            Ok(result) => {
                if result.is_empty() {
                    0
                } else {
                    result.try_read::<i64>(0).unwrap_or(0) as u8
                }
            }
            Err(_) => return,
        };

        // REPLACE INTO realmcharacters (numchars, acctid, realmid)
        let mut rep_stmt = login_db.prepare(LoginStatements::REP_REALM_CHARACTERS);
        rep_stmt.set_u8(0, num_chars);
        rep_stmt.set_u32(1, self.account_id);
        rep_stmt.set_u32(2, self.realm_id() as u32);

        if let Err(e) = login_db.execute(&rep_stmt).await {
            warn!("Failed to update realmcharacters: {e}");
        } else {
            debug!(
                "Updated realmcharacters: account={} realm={} count={}",
                self.account_id,
                self.realm_id(),
                num_chars
            );
        }
    }

    async fn load_account_mounts_like_cpp(&mut self) -> Vec<AccountMount> {
        self.set_account_mounts_like_cpp(Vec::new());
        let Some(login_db) = self.login_db() else {
            return Vec::new();
        };

        let mut stmt = login_db.prepare(LoginStatements::SEL_ACCOUNT_MOUNTS);
        stmt.set_u32(0, self.battlenet_account_id());

        let mut result = match login_db.query(&stmt).await {
            Ok(result) => result,
            Err(e) => {
                warn!(
                    account = self.account_id,
                    bnet_account = self.battlenet_account_id(),
                    "Failed to load account mounts: {e}"
                );
                return Vec::new();
            }
        };

        if result.is_empty() {
            return Vec::new();
        }

        let mut mounts = Vec::new();
        loop {
            let spell_id = result.try_read::<i32>(0).unwrap_or(0);
            let flags = result.try_read::<u8>(1).unwrap_or(0);
            let has_mount = spell_id > 0
                && self.mount_store().is_none_or(|store| {
                    store
                        .get_by_source_spell_id_like_cpp(spell_id as u32)
                        .is_some()
                });
            if has_mount {
                mounts.push(AccountMount { spell_id, flags });
            }

            if !result.next_row() {
                break;
            }
        }

        self.set_account_mounts_like_cpp(mounts.clone());
        mounts
    }

    async fn load_account_toys_like_cpp(&mut self) {
        let Some(login_db) = self.login_db() else {
            self.load_represented_account_toys_like_cpp([]);
            return;
        };

        let bnet_account_id = self.battlenet_account_id();
        let mut stmt = login_db.prepare(LoginStatements::SEL_ACCOUNT_TOYS);
        stmt.set_u32(0, bnet_account_id);
        let rows = match login_db.query(&stmt).await {
            Ok(mut result) => {
                let mut rows = Vec::new();
                if !result.is_empty() {
                    loop {
                        let item_id = result.try_read::<i32>(0).unwrap_or(0);
                        let is_favorite = result.try_read::<bool>(1).unwrap_or(false);
                        let has_fanfare = result.try_read::<bool>(2).unwrap_or(false);
                        if let Ok(item_id) = u32::try_from(item_id) {
                            rows.push((item_id, is_favorite, has_fanfare));
                        }
                        if !result.next_row() {
                            break;
                        }
                    }
                }
                rows
            }
            Err(error) => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account toys: {error}"
                );
                Vec::new()
            }
        };

        self.load_represented_account_toys_like_cpp(rows);
    }

    async fn load_account_heirlooms_like_cpp(&mut self) {
        let Some(login_db) = self.login_db() else {
            self.load_represented_account_heirlooms_like_cpp([]);
            return;
        };

        let bnet_account_id = self.battlenet_account_id();
        let mut stmt = login_db.prepare(LoginStatements::SEL_ACCOUNT_HEIRLOOMS);
        stmt.set_u32(0, bnet_account_id);
        let rows = match login_db.query(&stmt).await {
            Ok(mut result) => {
                let mut rows = Vec::new();
                if !result.is_empty() {
                    loop {
                        let item_id = result.try_read::<i32>(0).unwrap_or(0);
                        let flags = result.try_read::<u32>(1).unwrap_or(0);
                        if let Ok(item_id) = u32::try_from(item_id) {
                            rows.push((item_id, flags));
                        }
                        if !result.next_row() {
                            break;
                        }
                    }
                }
                rows
            }
            Err(error) => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account heirlooms: {error}"
                );
                Vec::new()
            }
        };

        self.load_represented_account_heirlooms_like_cpp(rows);
    }

    async fn load_account_item_appearances_like_cpp(&mut self) {
        let Some(login_db) = self.login_db() else {
            self.load_represented_account_item_appearances_like_cpp([], []);
            return;
        };

        let bnet_account_id = self.battlenet_account_id();
        let mut appearance_stmt = login_db.prepare(LoginStatements::SEL_BNET_ITEM_APPEARANCES);
        appearance_stmt.set_u32(0, bnet_account_id);
        let appearance_blocks = match login_db.query(&appearance_stmt).await {
            Ok(mut result) => {
                let mut blocks = Vec::new();
                if !result.is_empty() {
                    loop {
                        let block_index = result.try_read::<i32>(0).unwrap_or(0);
                        let appearance_mask = result.try_read::<u32>(1).unwrap_or(0);
                        if let Ok(block_index) = u32::try_from(block_index) {
                            blocks.push((block_index, appearance_mask));
                        }
                        if !result.next_row() {
                            break;
                        }
                    }
                }
                blocks
            }
            Err(error) => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account item appearances: {error}"
                );
                Vec::new()
            }
        };

        let mut favorite_stmt =
            login_db.prepare(LoginStatements::SEL_BNET_ITEM_FAVORITE_APPEARANCES);
        favorite_stmt.set_u32(0, bnet_account_id);
        let favorite_appearances = match login_db.query(&favorite_stmt).await {
            Ok(mut result) => {
                let mut favorites = Vec::new();
                if !result.is_empty() {
                    loop {
                        let item_modified_appearance_id = result.try_read::<i32>(0).unwrap_or(0);
                        if let Ok(item_modified_appearance_id) =
                            u32::try_from(item_modified_appearance_id)
                        {
                            favorites.push(item_modified_appearance_id);
                        }
                        if !result.next_row() {
                            break;
                        }
                    }
                }
                favorites
            }
            Err(error) => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account favorite item appearances: {error}"
                );
                Vec::new()
            }
        };

        self.load_represented_account_item_appearances_like_cpp(
            appearance_blocks,
            favorite_appearances,
        );
    }

    /// Send the player login packet sequence to the client.
    ///
    /// Follows the exact C# RustyCore order:
    /// HandlePlayerLogin → SendInitialPacketsBeforeAddToMap → AddToMap →
    /// SendInitialPacketsAfterAddToMap.
    ///
    /// Note: AuthResponse, SetTimeZone, FeatureSystemStatusGlueScreen,
    /// AccountDataTimes(global), and TutorialFlags are already sent during
    /// session init (see `send_session_init_packets`).
    fn send_login_sequence(
        &mut self,
        guid: ObjectGuid,
        race: u8,
        class: u8,
        sex: u8,
        level: u8,
        display_id: u32,
        position: &Position,
        map_id: i32,
        zone_id: i32,
        visible_items: [(i32, u16, u16); 19],
        inv_slots: [ObjectGuid; 141],
        item_creates: Vec<wow_packet::packets::update::ItemCreateData>,
        combat: PlayerCombatStats,
        known_spells: Vec<i32>,
        action_buttons: [i64; 180],
        skill_info: Vec<(u16, u16, u16, u16, u16, i16, u16)>,
        account_mounts: Vec<AccountMount>,
    ) {
        // ── Phase 1: HandlePlayerLogin packets ──

        // 1. DungeonDifficultySet — C# sends this BEFORE LoginVerifyWorld
        self.send_packet(&DungeonDifficultySet::normal());

        // 2. LoginVerifyWorld — confirms map + position
        self.send_packet(&LoginVerifyWorld {
            map_id,
            position: *position,
            reason: 0,
        });

        // 3. AccountDataTimes (per-character)
        self.send_packet(&AccountDataTimes::for_player(guid));

        // 4. FeatureSystemStatus (in-game version, different from glue screen)
        self.send_packet(&FeatureSystemStatus::default_wotlk());

        // 5. BattlePetJournalLockAcquired (empty packet — journal access granted)
        self.send_packet(&BattlePetJournalLockAcquired);

        // ── Phase 2: SendInitialPacketsBeforeAddToMap ──

        // 6. TimeSyncRequest (critical — client needs time sync)
        //    Also initializes the periodic timer (5s first, then 10s).
        self.reset_time_sync_like_cpp();
        self.send_time_sync();

        // 7. ContactList (social/friends — empty)
        self.send_packet(&ContactList::all());

        // 8. BindPointUpdate (hearthstone location = start position)
        self.send_packet(&BindPointUpdate {
            x: position.x,
            y: position.y,
            z: position.z,
            map_id,
            area_id: zone_id,
        });

        // 8b. SetProficiency — weapon and armor proficiency masks
        //     Sent during LoadFromDB when proficiency spells are applied.
        self.send_packet(&SetProficiency::default_weapons(class));
        self.send_packet(&SetProficiency::default_armor(class));

        // 9. UpdateTalentData (empty for fresh character)
        self.send_packet(&UpdateTalentData);

        // 10. SendKnownSpells — populated from character_spell table
        info!("Sending {} known spells for {:?}", known_spells.len(), guid);
        self.send_packet(&SendKnownSpells {
            initial_login: true,
            known_spells,
            favorite_spells: Vec::new(),
        });

        // 11. SendUnlearnSpells (empty)
        self.send_packet(&SendUnlearnSpells);

        // 12. SendSpellHistory (empty — no cooldowns)
        self.send_packet(&SendSpellHistory);

        // 13. SendSpellCharges (empty)
        self.send_packet(&SendSpellCharges);

        // 14. ActiveGlyphs (empty with full update)
        self.send_packet(&ActiveGlyphs {
            is_full_update: true,
        });

        // 15. UpdateActionButtons — populated from character_action table
        self.send_packet(&UpdateActionButtons {
            buttons: action_buttons,
            reason: 0, // Initialization
        });

        // 16. InitializeFactions (1000 factions, all neutral)
        let initialize_factions = self
            .reputation_mgr_like_cpp_mut()
            .initialize_factions_packet_like_cpp();
        self.send_packet(&initialize_factions);

        // 17. SetupCurrency (empty)
        self.send_packet(&SetupCurrency::empty());

        // 18. LoadEquipmentSet (empty)
        self.send_packet(&LoadEquipmentSet);

        // 19. AllAccountCriteria (empty)
        self.send_packet(&AllAccountCriteria);

        // 20. AllAchievementData (empty)
        self.send_packet(&AllAchievementData);

        // 21. LoginSetTimeSpeed
        self.send_packet(&LoginSetTimeSpeed::now());

        // 22. WorldServerInfo
        self.send_packet(&WorldServerInfo::default_open_world());

        // 22b. SetFlatSpellModifier + SetPctSpellModifier (empty for fresh char)
        //      C# sends via SendSpellModifiers() at Player.cs line 5584.
        //      For fresh chars these are empty, but we send them anyway to
        //      ensure the client's spell modifier arrays are initialized.
        self.send_raw_packet(&SetSpellModifier::flat_empty().to_bytes());
        self.send_raw_packet(&SetSpellModifier::pct_empty().to_bytes());

        // 23. AccountMountUpdate
        self.send_packet(&AccountMountUpdate::full(account_mounts));

        // 24. AccountToyUpdate
        self.send_account_toys_like_cpp();

        // 25. AccountHeirloomUpdate
        self.send_account_heirlooms_like_cpp();

        // 26. AccountTransmogUpdate favorite appearances
        self.send_favorite_appearances_like_cpp();

        // 27. InitialSetup (expansion level)
        self.send_packet(&InitialSetup::wotlk());

        // 27b. MoveSetActiveMover — CRITICAL: tells the client which unit it
        //      controls for movement. Without this, `m_mover` is null and the
        //      client crashes with ACCESS_VIOLATION when processing movement.
        //      C# sends via SetMovedUnit(this) at Player.cs line 5610.
        self.send_packet(&MoveSetActiveMover { mover_guid: guid });

        // ── Phase 3: AddToMap → UpdateObject ──

        // 26. UpdateObject — items + player in a SINGLE packet.
        //     C# sends all item CREATE blocks followed by the player CREATE
        //     in one UpdateObject. Items must come first so the client has
        //     them when it processes InvSlots, but everything must be in
        //     the same packet for forward-referenced Owner GUIDs to resolve.
        {
            // Build quest log for the UpdateObject (25 slots max).
            // C# ref: QuestLog.WriteCreate — sent with PartyMember flag for self-view.
            // StateFlags: 0=None, 1=Complete (QuestSlotStateMask)
            let quest_log: Vec<(u32, u32, i64, [u16; 24])> =
                self.quest_log_create_entries_like_cpp();
            let account_toys = self.account_toy_active_player_rows_like_cpp();
            let account_heirlooms = self.account_heirloom_active_player_rows_like_cpp();

            let mut player_pkt = UpdateObject::create_player(
                guid,
                race,
                class,
                sex,
                level,
                display_id,
                position,
                map_id as u16,
                zone_id as u32,
                true,
                visible_items,
                inv_slots,
                combat,
                skill_info,
                self.player_gold_like_cpp(),
                quest_log,
            );
            player_pkt
                .set_player_collection_dynamic_fields_like_cpp(account_toys, account_heirlooms);

            if !item_creates.is_empty() {
                info!(
                    "Sending {} item CREATE blocks + player in single UpdateObject",
                    item_creates.len()
                );
                // Prepend item blocks before the player block
                let mut all_blocks: Vec<UpdateBlock> = item_creates
                    .into_iter()
                    .map(|data| {
                        let g = data.item_guid;
                        UpdateBlock::CreateItem {
                            guid: g,
                            create_data: data,
                        }
                    })
                    .collect();
                all_blocks.append(&mut player_pkt.blocks);
                player_pkt.blocks = all_blocks;
                player_pkt.num_updates = player_pkt.blocks.len() as u32;
            }

            self.send_packet(&player_pkt);
        }

        // ── Phase 3b: Send nearby creatures + gameobjects ──
        // Query world DB for objects near the player and send UpdateObject.
        // This must be async, so we store the params and do it in the caller.
        self.pending_creature_spawn = Some(PendingCreatureSpawn {
            map_id: map_id as u16,
            position: *position,
            zone_id: zone_id as u32,
        });

        // ── Phase 4: SendInitialPacketsAfterAddToMap ──

        // 27. InitWorldStates (zone state variables — empty for now)
        self.send_packet(&InitWorldStates::new(map_id, zone_id));

        // 28. LoadCufProfiles (empty — no saved profiles)
        self.send_packet(&LoadCufProfiles);

        // 29. AuraUpdate (empty — no auras on fresh character)
        self.send_packet(&AuraUpdate::empty_for(guid));

        // 30. PhaseShiftChange — tells the client which phase the player is in.
        //     Without this the client ignores all world objects (creatures, GOs).
        //     C#: PhasingHandler.OnMapChange(this) → SendToPlayer → PhaseShiftChange
        //     Default player has no special phases: flags = Unphased (0x08).
        self.send_packet(&PhaseShiftChange::default_for(guid));

        // 30. Set session state to LoggedIn, store player GUID and initial position.
        self.set_state(crate::session::SessionState::LoggedIn);
        let attached_controller = self.ensure_login_player_controller_like_cpp(
            guid,
            self.player_name_like_cpp()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("Player{}", guid.counter())),
            *position,
            map_id as u16,
            race,
            class,
            level,
            sex,
        );
        if attached_controller {
            let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
        }
        self.set_player_health_like_cpp(
            combat.health.max(0).min(u32::MAX as i64) as u32,
            combat.max_health.max(1).min(u32::MAX as i64) as u32,
        );
        self.login_time = Some(std::time::Instant::now());
        // Clear per-session loot/visibility state for fresh login. Creatures
        // remain map-owned, matching C++ Map ownership.
        self.client_visible_guids_like_cpp.clear();
        self.loot_table.clear();
        self.set_active_loot_guid(ObjectGuid::EMPTY);
        self.combat_target = None;
        self.in_combat = false;

        // Register in the shared player registry so other sessions can
        // broadcast chat / emotes / movement packets to us.
        self.register_in_player_registry();
        self.sync_object_accessor_player();

        // 31. Broadcast this player's CREATE block to all other players on the same map.
        //     Each other player receives an UpdateObject with this player's CREATE block.
        self.broadcast_create_player_to_others();

        // 32. Receive CREATE blocks from all other players on the same map.
        //     This player receives UpdateObject packets for each other player.
        self.receive_other_players_on_map();

        // 33. Send full stat VALUES update so all character panel tabs
        //     (Melee, Ranged, Spell, Defense) display correct values on login.
        //     The CREATE packet has basic defaults; this overwrites them with
        //     fully computed stats (mana regen, expertise, shield block, etc.).
        self.send_stat_update();

        info!(
            "Login sequence complete for {:?} (37 packets including broadcasts)",
            guid
        );
    }

    // ── ShowTradeSkill ───────────────────────────────────────────────────────

    /// Handle `CMSG_SHOW_TRADE_SKILL` (0x36CA) — player opens a profession window.
    ///
    /// Responds with `SMSG_SHOW_TRADE_SKILL_RESPONSE` (0x2774) containing the
    /// known recipe spell IDs for the requested skill.
    pub async fn handle_show_trade_skill(
        &mut self,
        show: wow_packet::packets::misc::ShowTradeSkill,
    ) {
        use wow_packet::packets::misc::ShowTradeSkillResponse;

        let skill_id = show.skill_id;
        let level = self.player_level_like_cpp();

        let skill_rank = (level as i32) * 5;
        let skill_max_rank = skill_rank;

        let known = if let Some(store) = self.skill_store() {
            store.trade_skill_spells(skill_id, &self.known_spells_like_cpp())
        } else {
            Vec::new()
        };

        info!(
            "ShowTradeSkill skill_id={} spell_id={} caster={:?} — {} known recipes",
            skill_id,
            show.spell_id,
            show.caster_guid,
            known.len()
        );

        let response = ShowTradeSkillResponse {
            caster_guid: show.caster_guid,
            spell_id: show.spell_id,
            skill_line_id: skill_id,
            skill_rank,
            skill_max_rank,
            known_ability_spell_ids: known,
        };
        self.send_raw_packet(&response.to_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_data::quest::{
        QUEST_ITEM_DROP_COUNT, QUEST_REWARD_CHOICES_COUNT, QUEST_REWARD_DISPLAY_SPELL_COUNT,
        QUEST_REWARD_ITEM_COUNT, QUEST_REWARD_REPUTATIONS_COUNT, QuestStore, QuestTemplate,
    };
    use wow_packet::WorldPacket;
    use wow_packet::packets::loot::{
        CreatureLoot, LOOT_TYPE_CORPSE_LIKE_CPP, LootEntry, LootEntryFlags,
    };
    use wow_packet::packets::quest::quest_giver_status;

    fn make_session_with_send_capacity(
        capacity: usize,
    ) -> (WorldSession, flume::Receiver<Vec<u8>>) {
        let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
        let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
        (
            WorldSession::new(
                1,
                "TestAccount".into(),
                0,
                2,
                9,
                54261,
                vec![0u8; 40],
                "esES".into(),
                pkt_rx,
                send_tx,
            ),
            send_rx,
        )
    }

    fn make_quest_status_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
        let (mut session, send_rx) = make_session_with_send_capacity(8);
        session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        session.set_player_position_like_cpp(Position::new(10.0, 0.0, 0.0, 0.0));
        (session, send_rx)
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
            reward_currencies: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
            reward_currency_amounts: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
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
            required_skill_id: 0,
            required_skill_points: 0,
            reward_choice_items: [(0, 0); QUEST_REWARD_CHOICES_COUNT],
            reward_choice_item_types: [0; QUEST_REWARD_CHOICES_COUNT],
        }
    }

    fn store_with_quests(ids: &[u32]) -> QuestStore {
        QuestStore::from_quests_like_cpp(ids.iter().copied().map(quest_template))
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

    fn attach_map_manager(session: &mut WorldSession, manager: wow_map::MapManager) {
        session.set_canonical_map_manager(Arc::new(std::sync::Mutex::new(manager)));
    }

    fn mark_gameobject_questgiver(session: &mut WorldSession, guid: ObjectGuid) {
        let mut state = crate::session::RepresentedGameObjectUseState::default();
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8);
        session
            .represented_gameobject_use_states
            .insert(guid, state);
    }

    fn tracked_query_packet(guids: &[ObjectGuid]) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(guids.len() as u32);
        for guid in guids {
            pkt.write_packed_guid(guid);
        }
        pkt
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

    #[test]
    fn start_positions_are_valid() {
        for race in [1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 22] {
            let (map, x, y, z, _o) = start_position(race);
            assert!(map >= 0, "Race {race} has invalid map");
            // Positions should be non-zero (except possibly orientation)
            assert!(
                x != 0.0 || y != 0.0 || z != 0.0,
                "Race {race} has zero position"
            );
        }
    }

    #[test]
    fn display_ids_are_valid() {
        for race in [1, 2, 3, 4, 5, 6, 7, 8, 10, 11] {
            for sex in [0u8, 1] {
                let id = default_display_id(race, sex);
                assert!(id > 0, "Race {race} sex {sex} has zero display ID");
            }
        }
    }

    #[tokio::test]
    async fn quest_giver_status_tracked_supplied_creature_not_visible_sends_available_like_cpp() {
        let (mut session, send_rx) = make_quest_status_session();
        let mut store = store_with_quests(&[3001]);
        store.starter_quests.entry(9301).or_default().push(3001);
        session.set_quest_store(Arc::new(store));
        let guid = creature_guid(9301, 301);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9301);
        attach_map_manager(&mut session, manager);
        assert!(!session.client_visible_guids_like_cpp.contains(&guid));

        session
            .handle_quest_giver_status_tracked_query(tracked_query_packet(&[guid]))
            .await;

        assert_eq!(
            recv_status_multiple(&send_rx),
            vec![(guid, quest_giver_status::TRIVIAL)]
        );
    }

    #[tokio::test]
    async fn quest_giver_status_tracked_supplied_gameobject_uses_uint64_status_like_cpp() {
        let (mut session, send_rx) = make_quest_status_session();
        let mut store = store_with_quests(&[3002]);
        assert!(store.insert_gameobject_starter_relation_like_cpp(9302, 3002));
        session.set_quest_store(Arc::new(store));
        let guid = gameobject_guid(9302, 302);
        let mut manager = wow_map::MapManager::default();
        insert_gameobject(&mut manager, guid, 9302);
        attach_map_manager(&mut session, manager);
        mark_gameobject_questgiver(&mut session, guid);

        session
            .handle_quest_giver_status_tracked_query(tracked_query_packet(&[guid]))
            .await;

        assert_eq!(
            recv_status_multiple(&send_rx),
            vec![(guid, quest_giver_status::TRIVIAL)]
        );
    }

    #[tokio::test]
    async fn quest_giver_status_tracked_duplicate_guid_emits_single_status_like_cpp_set() {
        let (mut session, send_rx) = make_quest_status_session();
        let mut store = store_with_quests(&[3003]);
        store.starter_quests.entry(9303).or_default().push(3003);
        session.set_quest_store(Arc::new(store));
        let guid = creature_guid(9303, 303);
        let mut manager = wow_map::MapManager::default();
        insert_creature(&mut manager, guid, 9303);
        attach_map_manager(&mut session, manager);

        session
            .handle_quest_giver_status_tracked_query(tracked_query_packet(&[guid, guid]))
            .await;

        assert_eq!(recv_status_multiple(&send_rx).len(), 1);
    }

    #[tokio::test]
    async fn quest_giver_status_tracked_count_over_cpp_max_sends_no_packet() {
        let (mut session, send_rx) = make_quest_status_session();
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(QUEST_GIVER_STATUS_TRACKED_QUERY_MAX_GUIDS_LIKE_CPP + 1);

        session.handle_quest_giver_status_tracked_query(pkt).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_giver_status_tracked_short_payload_sends_no_packet() {
        let (mut session, send_rx) = make_quest_status_session();
        let guid = creature_guid(9304, 304);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(1);
        pkt.write_packed_guid(&guid);
        let mut bytes = pkt.into_data();
        bytes.pop();

        session
            .handle_quest_giver_status_tracked_query(WorldPacket::from_bytes(&bytes))
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn quest_giver_status_tracked_unsupported_missing_guid_sends_empty_multiple_like_cpp() {
        let (mut session, send_rx) = make_quest_status_session();
        attach_map_manager(&mut session, wow_map::MapManager::default());
        session.set_quest_store(Arc::new(store_with_quests(&[3005])));
        let missing_guid = creature_guid(9305, 305);
        let player_guid = ObjectGuid::create_player(1, 305);
        let item_guid = ObjectGuid::create_item(1, 305);

        session
            .handle_quest_giver_status_tracked_query(tracked_query_packet(&[
                missing_guid,
                player_guid,
                item_guid,
            ]))
            .await;

        assert!(recv_status_multiple(&send_rx).is_empty());
    }

    #[tokio::test]
    async fn tact_key_db_query_bulk_miss_returns_invalid_like_cpp_client_cache_fallback() {
        let (mut session, send_rx) = make_session_with_send_capacity(1);

        session
            .handle_db_query_bulk(wow_packet::packets::misc::DbQueryBulk {
                table_hash: TACT_KEY_TABLE_HASH_LIKE_CPP,
                queries: vec![3909],
            })
            .await;

        let bytes = send_rx.try_recv().expect("db reply");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            wow_constants::ServerOpcodes::DbReply as u16
        );
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint32().unwrap(), TACT_KEY_TABLE_HASH_LIKE_CPP);
        assert_eq!(pkt.read_int32().unwrap(), 3909);
        let _timestamp = pkt.read_int32().unwrap();
        assert_eq!(pkt.read_bits(3).unwrap(), 3);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
    }

    #[tokio::test]
    async fn query_page_text_without_world_db_sends_cpp_deny_shape() {
        let (mut session, send_rx) = make_session_with_send_capacity(1);

        session
            .handle_query_page_text(QueryPageText {
                page_text_id: 123,
                item_guid: ObjectGuid::EMPTY,
            })
            .await;

        let bytes = send_rx.try_recv().expect("query page text response");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            wow_constants::ServerOpcodes::QueryPageTextResponse as u16
        );
        assert_eq!(&bytes[2..6], &123_u32.to_le_bytes());
        assert_eq!(bytes[6], 0x00);
        assert_eq!(bytes.len(), 7);
    }

    #[tokio::test]
    async fn logout_releases_active_loot_views_like_cpp_remove_from_world() {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 19_030);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(loot_guid);
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_logout_request(LogoutRequest { idle_logout: false })
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootReleaseAll as u16
        );
        assert_eq!(sent.remaining(), 0);

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LogoutResponse as u16
        );

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRelease as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
        assert_eq!(sent.read_packed_guid().unwrap(), player_guid);

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LogoutComplete as u16
        );
        assert!(!session.is_active_loot_guid(loot_guid));
        assert!(session.loot_table.contains_key(&loot_guid));
    }

    #[test]
    fn start_zones_are_valid() {
        for race in [1, 2, 3, 4, 5, 6, 7, 8, 10, 11] {
            let zone = start_zone(race);
            assert!(zone > 0, "Race {race} has invalid zone");
        }
    }

    #[test]
    fn parse_equipment_cache_empty() {
        let eq = parse_equipment_cache("");
        for slot in &eq {
            assert_eq!(slot.display_id, 0);
            assert_eq!(slot.inv_type, 0);
        }
    }

    #[test]
    fn vendor_buy_price_uses_cpp_buy_count_unit_price() {
        assert_eq!(vendor_buy_quantity_and_price(500, 5, 1), (1, 100));
        assert_eq!(vendor_buy_quantity_and_price(500, 5, 3), (3, 300));
        assert_eq!(vendor_buy_quantity_and_price(500, 0, 2), (2, 1000));
        assert_eq!(vendor_buy_quantity_and_price(0, 5, 3), (3, 0));
        assert_eq!(vendor_buy_quantity_and_price(1, 5, 1), (1, 1));
    }

    #[test]
    fn vendor_buy_price_clamps_count_to_cpp_max_money_amount() {
        let unit_price = (MAX_MONEY_AMOUNT / 2) + 1;

        assert_eq!(
            vendor_buy_quantity_and_price(unit_price, 1, 3),
            (1, unit_price)
        );
    }

    #[test]
    fn vendor_buy_packet_quantity_uses_cpp_uint8_count_conversion() {
        assert_eq!(vendor_buy_packet_quantity_to_cpp_count(0), 1);
        assert_eq!(vendor_buy_packet_quantity_to_cpp_count(1), 1);
        assert_eq!(vendor_buy_packet_quantity_to_cpp_count(256), 1);
        assert_eq!(vendor_buy_packet_quantity_to_cpp_count(-1), 255);
    }

    #[test]
    fn vendor_buy_currency_preflight_matches_cpp_quantity_guards() {
        assert_eq!(vendor_buy_currency_packet_quantity_to_cpp_count(0), 1);
        assert_eq!(vendor_buy_currency_packet_quantity_to_cpp_count(5), 5);
        assert_eq!(
            vendor_buy_currency_quantity_block_result(5, 3),
            Some(InventoryResult::CantBuyQuantity)
        );
        assert_eq!(vendor_buy_currency_quantity_block_result(5, 10), None);
        assert_eq!(
            vendor_buy_currency_quantity_block_result(0, 10),
            Some(InventoryResult::CantBuyQuantity)
        );
    }

    #[test]
    fn vendor_buy_muid_uses_cpp_one_based_uint32_slot_conversion() {
        assert_eq!(vendor_buy_muid_to_cpp_slot(0), None);
        assert_eq!(vendor_buy_muid_to_cpp_slot(1), Some(0));
        assert_eq!(vendor_buy_muid_to_cpp_slot(2), Some(1));
        assert_eq!(vendor_buy_muid_to_cpp_slot(-1), Some(u32::MAX - 1));
    }

    #[test]
    fn vendor_list_item_limit_matches_cpp_cap() {
        assert!(!vendor_list_reaches_cpp_item_limit(149));
        assert!(vendor_list_reaches_cpp_item_limit(150));
        assert!(vendor_list_reaches_cpp_item_limit(151));
    }

    #[test]
    fn vendor_list_currency_rows_match_cpp_basic_guards() {
        let store = CurrencyTypesStore::from_entries([wow_data::CurrencyTypesEntry {
            id: 395,
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
        }]);
        assert!(vendor_list_should_skip_currency_row(Some(&store), 395, 0,));
        assert!(!vendor_list_should_skip_currency_row(Some(&store), 395, 10,));
        assert!(vendor_list_should_skip_currency_row(
            Some(&store),
            999_999,
            10
        ));
        assert!(vendor_list_should_skip_currency_row(None, 395, 10));
    }

    #[test]
    fn vendor_player_condition_id_evaluates_player_condition_store_like_cpp() {
        let store = PlayerConditionStore::from_entries([
            wow_data::PlayerConditionEntry {
                id: 42,
                class_mask: 0,
                ..Default::default()
            },
            wow_data::PlayerConditionEntry {
                id: 43,
                class_mask: 1 << 1,
                ..Default::default()
            },
        ]);
        let context = PlayerConditionContextLikeCpp {
            class_mask: 1,
            ..Default::default()
        };

        assert_eq!(
            vendor_player_condition_failed_id_like_cpp(0, Some(&store), Some(context)),
            0
        );
        assert_eq!(
            vendor_player_condition_failed_id_like_cpp(42, Some(&store), Some(context)),
            0
        );
        assert_eq!(
            vendor_player_condition_failed_id_like_cpp(43, Some(&store), Some(context)),
            43
        );
        assert_eq!(
            vendor_player_condition_failed_id_like_cpp(999, Some(&store), Some(context)),
            0
        );
        assert_eq!(
            vendor_buy_player_condition_block_result_like_cpp(42, Some(&store), Some(context)),
            None
        );
        assert_eq!(
            vendor_buy_player_condition_block_result_like_cpp(43, Some(&store), Some(context)),
            Some(InventoryResult::ItemLocked)
        );
        assert_eq!(
            vendor_buy_player_condition_block_result_like_cpp(42, None, Some(context)),
            Some(InventoryResult::ItemLocked)
        );
    }

    #[test]
    fn vendor_condition_presence_fails_closed_until_condition_mgr_exists() {
        assert_eq!(vendor_conditions_block_result(false), None);
        assert_eq!(
            vendor_conditions_block_result(true),
            Some(BuyResult::CantFindItem)
        );
    }

    #[test]
    fn vendor_required_reputation_fails_closed_until_reputation_mgr_exists() {
        assert_eq!(
            vendor_buy_required_reputation_block_result(None, None, -1),
            None
        );
        assert_eq!(
            vendor_buy_required_reputation_block_result(Some(72), Some(5), -1),
            Some(BuyResult::ReputationRequire)
        );
        assert_eq!(
            vendor_buy_required_reputation_block_result(Some(72), Some(5), 5),
            None
        );
    }

    #[test]
    fn vendor_buy_extended_cost_fails_closed_like_cpp_preflight() {
        let currency_store = CurrencyTypesStore::from_entries([wow_data::CurrencyTypesEntry {
            id: 395,
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
        }]);
        let extended_cost_store =
            ItemExtendedCostStore::from_entries([wow_data::ItemExtendedCostEntry {
                id: 12,
                required_arena_rating: 0,
                arena_bracket: 0,
                flags: wow_constants::ItemExtendedCostFlags::empty(),
                min_faction_id: 0,
                min_reputation: 0,
                required_achievement: 0,
                item_id: [0; wow_data::MAX_ITEM_EXT_COST_ITEMS],
                item_count: [0; wow_data::MAX_ITEM_EXT_COST_ITEMS],
                currency_id: [395, 0, 0, 0, 0],
                currency_count: [10, 0, 0, 0, 0],
            }]);

        assert_eq!(
            vendor_buy_extended_cost_block_result(
                None,
                None,
                |_, _| false,
                |_, _| false,
                false,
                0,
                5,
                3
            ),
            None
        );
        assert_eq!(
            vendor_buy_extended_cost_block_result(
                Some(&extended_cost_store),
                Some(&currency_store),
                |_, _| false,
                |_, _| false,
                false,
                12,
                5,
                3
            ),
            Some(VendorExtendedCostBlock::Equip(
                InventoryResult::CantBuyQuantity
            ))
        );
        assert_eq!(
            vendor_buy_extended_cost_block_result(
                Some(&extended_cost_store),
                Some(&currency_store),
                |_, _| true,
                |currency_id, amount| currency_id == 395 && amount >= 20,
                false,
                12,
                5,
                10
            ),
            Some(VendorExtendedCostBlock::Equip(
                InventoryResult::VendorMissingTurnins
            ))
        );
        assert_eq!(
            vendor_buy_extended_cost_block_result(
                Some(&extended_cost_store),
                Some(&currency_store),
                |_, _| true,
                |currency_id, amount| currency_id == 395 && amount >= 20,
                true,
                12,
                5,
                10
            ),
            None
        );
        assert_eq!(
            vendor_buy_extended_cost_currency_costs(Some(&extended_cost_store), 12, 5, 10),
            vec![(395, 20)]
        );
        let item_turnin_store =
            ItemExtendedCostStore::from_entries([wow_data::ItemExtendedCostEntry {
                id: 13,
                required_arena_rating: 0,
                arena_bracket: 0,
                flags: wow_constants::ItemExtendedCostFlags::empty(),
                min_faction_id: 0,
                min_reputation: 0,
                required_achievement: 0,
                item_id: [700, 0, 0, 0, 0],
                item_count: [3, 0, 0, 0, 0],
                currency_id: [0; wow_data::MAX_ITEM_EXT_COST_CURRENCIES],
                currency_count: [0; wow_data::MAX_ITEM_EXT_COST_CURRENCIES],
            }]);
        assert_eq!(
            vendor_buy_extended_cost_block_result(
                Some(&item_turnin_store),
                Some(&currency_store),
                |item_id, amount| item_id == 700 && amount == 6,
                |_, _| true,
                true,
                13,
                5,
                10
            ),
            None
        );
        assert_eq!(
            vendor_buy_extended_cost_block_result(
                Some(&item_turnin_store),
                Some(&currency_store),
                |_, _| false,
                |_, _| true,
                true,
                13,
                5,
                10
            ),
            Some(VendorExtendedCostBlock::Equip(
                InventoryResult::VendorMissingTurnins
            ))
        );
        assert_eq!(
            vendor_buy_extended_cost_item_costs(Some(&item_turnin_store), 13, 5, 10),
            vec![(700, 6)]
        );
        let checked_currency_amount = std::cell::Cell::new(false);
        assert_eq!(
            vendor_buy_extended_cost_block_result(
                Some(&extended_cost_store),
                Some(&currency_store),
                |_, _| true,
                |currency_id, amount| {
                    checked_currency_amount.set(true);
                    assert_eq!(currency_id, 395);
                    assert_eq!(amount, 20);
                    false
                },
                true,
                12,
                5,
                10
            ),
            Some(VendorExtendedCostBlock::Equip(
                InventoryResult::VendorMissingTurnins
            ))
        );
        assert!(checked_currency_amount.get());
        assert_eq!(
            vendor_buy_extended_cost_block_result(
                Some(&extended_cost_store),
                None,
                |_, _| true,
                |_, _| true,
                true,
                12,
                5,
                10
            ),
            Some(VendorExtendedCostBlock::Buy(BuyResult::CantFindItem))
        );
        assert_eq!(
            vendor_buy_extended_cost_block_result(
                Some(&extended_cost_store),
                Some(&currency_store),
                |_, _| true,
                |_, _| true,
                true,
                99,
                5,
                10
            ),
            Some(VendorExtendedCostBlock::Silent)
        );
    }

    #[test]
    fn vendor_buy_direct_store_preflight_matches_cpp_store_branch() {
        assert_eq!(
            vendor_buy_direct_store_block_result(NULL_BAG, NULL_SLOT, 1),
            None
        );
        assert_eq!(
            vendor_buy_direct_store_block_result(INVENTORY_SLOT_BAG_0, 35, 1),
            None
        );
        assert_eq!(
            vendor_buy_direct_store_block_result(NULL_BAG, 35, 1),
            Some(InventoryResult::WrongSlot)
        );
        assert_eq!(
            vendor_buy_direct_store_block_result(INVENTORY_SLOT_BAG_0, 0, 1),
            Some(InventoryResult::NotEquippable)
        );
    }

    #[test]
    fn vendor_buy_stock_refill_matches_cpp_increment_and_full_reset() {
        assert_eq!(vendor_buy_stock_refill_count(2, 20, 10, 5, 20), (12, false));
        assert_eq!(vendor_buy_stock_refill_count(18, 10, 10, 5, 20), (20, true));
        assert_eq!(vendor_buy_stock_refill_count(2, 9, 10, 5, 20), (2, false));
    }

    #[test]
    fn extended_cost_item_turnin_plan_matches_cpp_destroy_order() {
        let (_pkt_tx, pkt_rx) = flume::bounded::<wow_packet::WorldPacket>(8);
        let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(8);
        let mut session = WorldSession::new(
            1,
            "TestAccount".into(),
            0,
            2,
            9,
            54261,
            vec![0u8; 40],
            "esES".into(),
            pkt_rx,
            send_tx,
        );
        let player_guid = ObjectGuid::create_player(1, 1);
        session.set_player_guid(Some(player_guid));

        for (slot, db_guid, count) in [(35, 10_u64, 4_u32), (36, 11_u64, 5_u32)] {
            let item_guid = ObjectGuid::create_item(1, db_guid as i64);
            session.insert_inventory_item_like_cpp(
                slot,
                InventoryItem {
                    guid: item_guid,
                    entry_id: 700,
                    db_guid,
                    inventory_type: None,
                },
            );
            let item = session.make_inventory_item_object(
                item_guid,
                700,
                player_guid,
                count,
                0,
                ItemContext::Vendor,
                slot,
            );
            session.insert_inventory_item_object(item);
        }

        assert!(session.has_item_count_direct_inventory(700, 9));
        assert!(!session.has_item_count_direct_inventory(700, 10));
        assert_eq!(
            session.plan_destroy_item_count_direct_inventory(700, 6),
            Some(vec![
                ExtendedCostItemTurninChange::Delete {
                    slot: 35,
                    item_guid: ObjectGuid::create_item(1, 10),
                    db_guid: 10,
                },
                ExtendedCostItemTurninChange::Update {
                    slot: 36,
                    item_guid: ObjectGuid::create_item(1, 11),
                    db_guid: 11,
                    new_count: 3,
                },
            ])
        );
    }

    #[test]
    fn vendor_item_current_count_updates_like_cpp() {
        let (_pkt_tx, pkt_rx) = flume::bounded::<wow_packet::WorldPacket>(8);
        let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(8);
        let mut session = WorldSession::new(
            1,
            "TestAccount".into(),
            0,
            2,
            9,
            54261,
            vec![0u8; 40],
            "esES".into(),
            pkt_rx,
            send_tx,
        );
        let vendor_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 7, 1);

        assert_eq!(
            session.vendor_item_current_count(vendor_guid, 700, 5, 60, 1),
            5
        );
        assert_eq!(
            session.update_vendor_item_current_count(vendor_guid, 700, 5, 60, 1, 2),
            3
        );
        assert_eq!(
            session.vendor_item_current_count(vendor_guid, 700, 5, 60, 1),
            3
        );

        if let Some(count) = session.vendor_item_counts.get_mut(&(vendor_guid, 700)) {
            count.last_increment_time = WorldSession::vendor_stock_now_secs().saturating_sub(120);
        }

        assert_eq!(
            session.vendor_item_current_count(vendor_guid, 700, 5, 60, 1),
            5
        );
        assert!(!session.vendor_item_counts.contains_key(&(vendor_guid, 700)));
    }

    #[test]
    fn vendor_list_sold_out_filter_matches_cpp_gm_branch() {
        assert!(vendor_list_should_skip_sold_out(5, 0, false));
        assert!(!vendor_list_should_skip_sold_out(5, 0, true));
        assert!(!vendor_list_should_skip_sold_out(5, 1, false));
        assert!(!vendor_list_should_skip_sold_out(0, 0, false));
    }

    #[test]
    fn vendor_list_refundable_flag_matches_cpp_template_guard() {
        assert!(vendor_list_item_refundable(
            Some(ItemFlags::ITEM_PURCHASE_RECORD),
            Some(1),
            42
        ));
        assert!(!vendor_list_item_refundable(
            Some(ItemFlags::ITEM_PURCHASE_RECORD),
            Some(2),
            42
        ));
        assert!(!vendor_list_item_refundable(
            Some(ItemFlags::ITEM_PURCHASE_RECORD),
            Some(1),
            0
        ));
        assert!(!vendor_list_item_refundable(None, Some(1), 42));
    }

    #[test]
    fn loaded_refund_metadata_matches_cpp_load_cleanup() {
        let refundable_flags = (ItemFieldFlags::SOULBOUND | ItemFieldFlags::REFUNDABLE).bits();
        assert_eq!(
            loaded_item_refund_decision(refundable_flags, 7_200, Some(123), Some(45)),
            LoadedItemRefundDecision::Valid {
                paid_money: 123,
                paid_extended_cost: 45,
            }
        );
        assert_eq!(
            loaded_item_refund_decision(refundable_flags, 7_201, Some(123), Some(45)),
            LoadedItemRefundDecision::Clear {
                new_flags: ItemFieldFlags::SOULBOUND.bits(),
            }
        );
        assert_eq!(
            loaded_item_refund_decision(refundable_flags, 10, None, Some(45)),
            LoadedItemRefundDecision::Clear {
                new_flags: ItemFieldFlags::SOULBOUND.bits(),
            }
        );
        assert_eq!(
            loaded_item_refund_decision(ItemFieldFlags::SOULBOUND.bits(), 10, Some(123), Some(45)),
            LoadedItemRefundDecision::None
        );
    }

    #[test]
    fn destroy_item_count_action_matches_cpp_direct_item_branch() {
        assert_eq!(
            destroy_item_count_action(5, 0),
            DestroyItemCountAction::FullStack
        );
        assert_eq!(
            destroy_item_count_action(5, 5),
            DestroyItemCountAction::FullStack
        );
        assert_eq!(
            destroy_item_count_action(5, 7),
            DestroyItemCountAction::FullStack
        );
        assert_eq!(
            destroy_item_count_action(5, 2),
            DestroyItemCountAction::PartialStack { new_count: 3 }
        );
    }

    #[test]
    fn sell_item_amount_action_matches_cpp_amount_branch() {
        assert_eq!(
            sell_item_amount_action(5, 0),
            SellItemAmountAction::FullStack { amount: 5 }
        );
        assert_eq!(
            sell_item_amount_action(5, 5),
            SellItemAmountAction::FullStack { amount: 5 }
        );
        assert_eq!(
            sell_item_amount_action(5, 2),
            SellItemAmountAction::PartialStack {
                amount: 2,
                remaining: 3,
            }
        );
        assert_eq!(sell_item_amount_action(5, 6), SellItemAmountAction::Invalid);
        assert_eq!(
            sell_item_amount_action(5, -1),
            SellItemAmountAction::Invalid
        );
    }

    #[test]
    fn player_money_gain_like_cpp_enforces_max_money_amount() {
        assert_eq!(player_money_gain_like_cpp(0, 0), Some(0));
        assert_eq!(
            player_money_gain_like_cpp(MAX_MONEY_AMOUNT - 1, 1),
            Some(MAX_MONEY_AMOUNT)
        );
        assert_eq!(player_money_gain_like_cpp(MAX_MONEY_AMOUNT, 1), None);
        assert_eq!(player_money_gain_like_cpp(MAX_MONEY_AMOUNT - 10, 11), None);
        assert_eq!(player_money_gain_like_cpp(0, MAX_MONEY_AMOUNT + 1), None);
    }

    #[test]
    fn item_currently_looted_guard_uses_runtime_loot_generated_state() {
        let mut item = wow_entities::Item::default();
        assert!(!item_is_currently_looted_like_cpp(&item));

        item.set_loot_generated(true);
        assert!(item_is_currently_looted_like_cpp(&item));
    }

    #[test]
    fn sell_non_empty_bag_guard_matches_cpp_is_not_empty_bag() {
        assert!(item_is_not_empty_bag_like_cpp(
            Some(InventoryType::Bag),
            true
        ));
        assert!(!item_is_not_empty_bag_like_cpp(
            Some(InventoryType::Bag),
            false
        ));
        assert!(!item_is_not_empty_bag_like_cpp(
            Some(InventoryType::Chest),
            true
        ));
        assert!(!item_is_not_empty_bag_like_cpp(None, true));
    }

    #[test]
    fn vendor_list_allowed_class_filter_matches_cpp_bind_on_acquire_branch() {
        let warrior_mask = 1i16 << (1 - 1);
        let mage_mask = 1i16 << (8 - 1);

        assert!(!vendor_list_should_skip_allowed_class(
            Some(warrior_mask),
            Some(ItemBondingType::OnAcquire as u8),
            1,
            false,
        ));
        assert!(vendor_list_should_skip_allowed_class(
            Some(warrior_mask),
            Some(ItemBondingType::OnAcquire as u8),
            8,
            false,
        ));
        assert!(!vendor_list_should_skip_allowed_class(
            Some(warrior_mask),
            Some(ItemBondingType::OnEquip as u8),
            8,
            false,
        ));
        assert!(!vendor_list_should_skip_allowed_class(
            Some(warrior_mask),
            Some(ItemBondingType::OnAcquire as u8),
            8,
            true,
        ));
        assert!(!vendor_list_should_skip_allowed_class(
            Some(warrior_mask | mage_mask),
            Some(ItemBondingType::OnAcquire as u8),
            8,
            false,
        ));
        assert!(!vendor_list_should_skip_allowed_class(
            Some(-1),
            Some(ItemBondingType::OnAcquire as u8),
            8,
            false,
        ));
    }

    #[test]
    fn vendor_list_faction_filter_matches_cpp_team_branch() {
        assert_eq!(player_team_for_race_cpp(1), Team::Alliance);
        assert_eq!(player_team_for_race_cpp(2), Team::Horde);
        assert_eq!(player_team_for_race_cpp(11), Team::Alliance);
        assert_eq!(player_team_for_race_cpp(10), Team::Horde);

        assert!(vendor_list_should_skip_faction_flags(
            Some(ItemFlags2::FactionHorde as u32),
            Team::Alliance,
            false,
        ));
        assert!(!vendor_list_should_skip_faction_flags(
            Some(ItemFlags2::FactionHorde as u32),
            Team::Horde,
            false,
        ));
        assert!(vendor_list_should_skip_faction_flags(
            Some(ItemFlags2::FactionAlliance as u32),
            Team::Horde,
            false,
        ));
        assert!(!vendor_list_should_skip_faction_flags(
            Some(ItemFlags2::FactionAlliance as u32),
            Team::Horde,
            true,
        ));
        assert!(!vendor_list_should_skip_faction_flags(
            None,
            Team::Alliance,
            false
        ));
    }

    #[test]
    fn vendor_buy_template_gates_match_cpp_error_shapes() {
        let warrior_mask = 1i16 << (1 - 1);

        assert_eq!(
            vendor_buy_template_block_result(
                Some(warrior_mask),
                Some(ItemBondingType::OnAcquire as u8),
                None,
                8,
                1,
                false,
            ),
            Some(VendorBuyTemplateBlock::BuyError(BuyResult::CantFindItem))
        );
        assert_eq!(
            vendor_buy_template_block_result(
                Some(warrior_mask),
                Some(ItemBondingType::OnAcquire as u8),
                None,
                8,
                1,
                true,
            ),
            None
        );
        assert_eq!(
            vendor_buy_template_block_result(
                None,
                None,
                Some(ItemFlags2::FactionHorde as u32),
                1,
                1,
                false,
            ),
            Some(VendorBuyTemplateBlock::Silent)
        );
        assert_eq!(
            vendor_buy_template_block_result(
                None,
                None,
                Some(ItemFlags2::FactionHorde as u32),
                1,
                2,
                false,
            ),
            None
        );
    }

    #[test]
    fn vendor_buy_destination_maps_player_container_like_cpp() {
        let player_guid = ObjectGuid::create_player(1, 42);
        let buy = BuyItem {
            vendor_guid: ObjectGuid::EMPTY,
            container_guid: player_guid,
            quantity: 1,
            muid: 1,
            slot: 35,
            item_type: 0,
            item_id: 700,
        };

        assert_eq!(
            vendor_buy_direct_inventory_destination(player_guid, &buy),
            Some((INVENTORY_SLOT_BAG_0, 35))
        );
    }

    #[test]
    fn vendor_buy_destination_rejects_cpp_slot_over_max_bag_size() {
        let player_guid = ObjectGuid::create_player(1, 42);
        let buy = BuyItem {
            vendor_guid: ObjectGuid::EMPTY,
            container_guid: player_guid,
            quantity: 1,
            muid: 1,
            slot: (MAX_BAG_SIZE + 1) as i32,
            item_type: 0,
            item_id: 700,
        };

        assert_eq!(
            vendor_buy_direct_inventory_destination(player_guid, &buy),
            None
        );
    }

    #[test]
    fn vendor_buy_destination_uses_cpp_uint8_slot_conversion() {
        let player_guid = ObjectGuid::create_player(1, 42);
        let buy = BuyItem {
            vendor_guid: ObjectGuid::EMPTY,
            container_guid: player_guid,
            quantity: 1,
            muid: 1,
            slot: 256,
            item_type: 0,
            item_id: 700,
        };

        assert_eq!(
            vendor_buy_direct_inventory_destination(player_guid, &buy),
            Some((INVENTORY_SLOT_BAG_0, 0))
        );
    }

    #[test]
    fn parse_equipment_cache_real_data() {
        // Real data from DB: first slot has inv_type=0, next few slots have gear
        let cache = "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 4 2470 0 0 0 20 33257 0 1 0";
        let eq = parse_equipment_cache(cache);
        // Slot 0: all zeros
        assert_eq!(eq[0].display_id, 0);
        // Slot 3: inv_type=4, display_id=2470
        assert_eq!(eq[3].inv_type, 4);
        assert_eq!(eq[3].display_id, 2470);
        // Slot 4: inv_type=20, display_id=33257, subclass=1
        assert_eq!(eq[4].inv_type, 20);
        assert_eq!(eq[4].display_id, 33257);
        assert_eq!(eq[4].subclass, 1);
    }

    #[test]
    fn player_flags_to_char_flags_resting() {
        // PlayerFlags::Resting = 0x20 → CharacterFlags::Resting = 0x02
        let player_flags: u32 = 0x20;
        let mut char_flags: u32 = 0;
        if (player_flags & 0x20) != 0 {
            char_flags |= 0x02;
        }
        assert_eq!(char_flags, 0x02);
    }

    #[test]
    fn player_flags_to_char_flags_ghost() {
        // PlayerFlags::Ghost = 0x10 → CharacterFlags::Ghost = 0x2000
        let player_flags: u32 = 0x10;
        let at_login_flags: u16 = 0;
        let mut char_flags: u32 = 0;
        if (player_flags & 0x10) != 0 && (at_login_flags & 0x100) == 0 {
            char_flags |= 0x2000;
        }
        assert_eq!(char_flags, 0x2000);
    }

    #[test]
    fn player_flags_ghost_suppressed_by_resurrect() {
        // Ghost flag suppressed when AtLoginFlags::Resurrect (0x100) is set
        let player_flags: u32 = 0x10;
        let at_login_flags: u16 = 0x100;
        let mut char_flags: u32 = 0;
        if (player_flags & 0x10) != 0 && (at_login_flags & 0x100) == 0 {
            char_flags |= 0x2000;
        }
        assert_eq!(char_flags, 0); // Ghost NOT set
    }

    #[test]
    fn raw_player_flags_not_passed_directly() {
        // Verify that raw playerFlags (e.g. AFK=0x02) don't leak into CharacterFlags
        let player_flags: u32 = 0x02; // PlayerFlags::AFK
        let mut char_flags: u32 = 0;
        // Only map known flags
        if (player_flags & 0x20) != 0 {
            char_flags |= 0x02;
        }
        if (player_flags & 0x10) != 0 {
            char_flags |= 0x2000;
        }
        // AFK (0x02) should NOT map to anything in CharacterFlags
        assert_eq!(char_flags, 0);
    }
}
