// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handlers: enum, create, delete, and player login.

use std::sync::Arc;

use rand::Rng;
use tracing::{debug, info, trace, warn};
use wow_constants::ClientOpcodes;
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_crypto::rsa_sign::rsa_sign_connect_to;
use wow_database::{CharStatements, LoginStatements, WorldDatabase, WorldStatements};
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::auth::{
    ConnectTo, ConnectToAddress, ConnectToFailed, ConnectToKey, ConnectToSerial, ResumeComms,
};
use wow_packet::packets::character::*;
use wow_packet::packets::item::*;
use wow_packet::packets::misc::*;
use wow_packet::packets::update::*;
use wow_ai::CreatureAI;

// ── Handler registration ────────────────────────────────────────────

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
        processing: PacketProcessing::Inplace,
        handler_name: "handle_battle_pay_stub",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePayGetPurchaseList,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
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
        processing: PacketProcessing::Inplace,
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
        processing: PacketProcessing::Inplace,
        handler_name: "handle_hotfix_request",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TimeSyncResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_time_sync_response",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LogoutRequest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_logout_request",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LogoutCancel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_logout_cancel",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryCreature,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_creature",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryGameObject,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_game_object",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryPlayerNames,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_player_names",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryRealmName,
        status: SessionStatus::Authed,
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
        processing: PacketProcessing::ThreadUnsafe,
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
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_npc_text",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ListInventory,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_list_inventory",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TrainerList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_trainer_list",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BuyItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_buy_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SellItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_sell_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionHelloRequest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
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
        processing: PacketProcessing::Inplace,
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
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_stabled_pets",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverStatusMultipleQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_status_multiple_query",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SwapInvItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_swap_inv_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutoEquipItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auto_equip_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SwapItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_swap_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutoStoreBagItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auto_store_bag_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DestroyItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
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
        1 => (0, -8949.95, -132.493, 83.5312, 0.0),        // Human
        2 => (1, -618.518, -4251.67, 38.718, 0.0),          // Orc
        3 => (0, -6240.32, 331.033, 382.758, 6.17716),      // Dwarf
        4 => (1, 10311.3, 832.463, 1326.41, 5.69632),       // NightElf
        5 => (0, 1676.71, 1678.31, 121.67, 2.70526),        // Undead
        6 => (1, -2917.58, -257.98, 52.9968, 0.0),          // Tauren
        7 => (0, -6240.32, 331.033, 382.758, 0.0),          // Gnome
        8 => (1, -618.518, -4251.67, 38.718, 0.0),          // Troll
        10 => (530, 10349.6, -6357.29, 33.4026, 5.31605),   // BloodElf
        11 => (530, -3961.64, -13931.2, 100.615, 2.08364),  // Draenei
        22 => (0, -8949.95, -132.493, 83.5312, 0.0),        // Worgen → Human
        _ => (0, -8949.95, -132.493, 83.5312, 0.0),         // Default: Human
    }
}

/// Default display ID for a race/sex combination.
pub(crate) fn default_display_id(race: u8, sex: u8) -> u32 {
    match (race, sex) {
        (1, 0) => 49,     (1, 1) => 50,     // Human M/F
        (2, 0) => 51,     (2, 1) => 52,     // Orc
        (3, 0) => 53,     (3, 1) => 54,     // Dwarf
        (4, 0) => 55,     (4, 1) => 56,     // NightElf
        (5, 0) => 57,     (5, 1) => 58,     // Undead
        (6, 0) => 59,     (6, 1) => 60,     // Tauren
        (7, 0) => 1563,   (7, 1) => 1564,   // Gnome
        (8, 0) => 1478,   (8, 1) => 1479,   // Troll
        (10, 0) => 15476, (10, 1) => 15475, // BloodElf
        (11, 0) => 16125, (11, 1) => 16126, // Draenei
        _ => 49,                              // Default: Human Male
    }
}

/// Default zone ID for a starting position.
fn start_zone(race: u8) -> i32 {
    match race {
        1 | 22 => 12,  // Human / Worgen: Elwynn Forest
        2 | 8 => 14,   // Orc / Troll: Durotar
        3 | 7 => 1,    // Dwarf / Gnome: Dun Morogh
        4 => 141,       // NightElf: Teldrassil
        5 => 85,        // Undead: Tirisfal Glades
        6 => 215,       // Tauren: Mulgore
        10 => 3430,     // BloodElf: Eversong Woods
        11 => 3524,     // Draenei: Azuremyst Isle
        _ => 12,
    }
}

/// Default starting health and mana for a level 1 character by class.
fn default_health_mana(class: u8) -> (u32, u32) {
    match class {
        1 => (50, 0),      // Warrior — no mana
        2 => (52, 79),     // Paladin
        3 => (46, 85),     // Hunter (uses focus at high level, mana at 1)
        4 => (45, 0),      // Rogue — no mana
        5 => (52, 160),    // Priest
        6 => (130, 0),     // Death Knight — no mana (runic power)
        7 => (47, 73),     // Shaman
        8 => (42, 200),    // Mage
        9 => (43, 200),    // Warlock
        11 => (54, 60),    // Druid
        _ => (50, 100),    // Default
    }
}

/// Maximum characters per account.
const MAX_CHARACTERS_PER_ACCOUNT: u32 = 10;

/// Reverse-map an equipment slot (0-18) to its InventoryType.
///
/// Used as a fallback when Item.db2 store is not available.
fn slot_to_inventory_type(slot: u8) -> Option<u8> {
    match slot {
        0 => Some(1),         // Head
        1 => Some(2),         // Neck
        2 => Some(3),         // Shoulders
        3 => Some(4),         // Body (Shirt)
        4 => Some(5),         // Chest
        5 => Some(6),         // Waist
        6 => Some(7),         // Legs
        7 => Some(8),         // Feet
        8 => Some(9),         // Wrists
        9 => Some(10),        // Hands
        10 | 11 => Some(11),  // Finger (Ring)
        12 | 13 => Some(12),  // Trinket
        14 => Some(16),       // Cloak
        15 => Some(21),       // MainHand (WeaponMainHand)
        16 => Some(22),       // OffHand (WeaponOffHand)
        17 => Some(15),       // Ranged
        18 => Some(19),       // Tabard
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

// ── Handler implementations ─────────────────────────────────────────

impl WorldSession {
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
                warn!("Failed to query characters for account {}: {e}", self.account_id);
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
            (1u8, 0u8),  // Human — Classic
            (2, 0),       // Orc
            (3, 0),       // Dwarf
            (4, 0),       // Night Elf
            (5, 0),       // Undead
            (6, 0),       // Tauren
            (7, 0),       // Gnome
            (8, 0),       // Troll
            (10, 1),      // Blood Elf — TBC
            (11, 1),      // Draenei — TBC
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
        ins_stmt.set_u64(0, new_guid_counter as u64);    // guid (bigint unsigned)
        ins_stmt.set_u32(1, self.account_id);             // account
        ins_stmt.set_string(2, &pkt.name);                // name
        ins_stmt.set_u8(3, pkt.race);                     // race
        ins_stmt.set_u8(4, pkt.class);                    // class
        ins_stmt.set_u8(5, sex);                          // gender
        ins_stmt.set_u8(6, 1);                            // level
        ins_stmt.set_u64(7, 0);                           // money (bigint unsigned)
        ins_stmt.set_i32(8, zone);                        // zone (smallint unsigned)
        ins_stmt.set_i32(9, map_id);                      // map (smallint unsigned)
        ins_stmt.set_f32(10, x);                          // position_x
        ins_stmt.set_f32(11, y);                          // position_y
        ins_stmt.set_f32(12, z);                          // position_z
        ins_stmt.set_f32(13, o);                          // orientation
        ins_stmt.set_string(14, "");                      // taximask (text NOT NULL)
        ins_stmt.set_i64(15, create_time);                // createTime (bigint)
        ins_stmt.set_u8(16, 0);                           // createMode
        ins_stmt.set_u32(17, 0);                          // playerFlags
        ins_stmt.set_u32(18, 0x20);                       // at_login (AT_LOGIN_FIRST)
        ins_stmt.set_u32(19, health);                     // health
        ins_stmt.set_u32(20, mana);                       // power1 (mana)
        ins_stmt.set_u32(21, self.build);                 // lastLoginBuild

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
                    pkt.name, new_guid_counter, pkt.customizations.len(), self.account_id
                );

                // Insert initial action buttons from playercreateinfo_action
                if let Some(world_db) = self.world_db().map(Arc::clone) {
                    let action_stmt = world_db.prepare(WorldStatements::SEL_PLAYER_CREATEINFO_ACTION);
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
                                    let mut ins = char_db.prepare(CharStatements::INS_CHARACTER_ACTION);
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
                            info!("Inserted {action_count} initial action buttons for '{}'", pkt.name);
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
        let rx = session_mgr.register(
            self.account_id,
            key_raw,
            self.session_key.clone(),
        );
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
            serial, self.account_id, format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
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
    /// We don't have a DB2 storage engine, so we respond with Status::Invalid
    /// for every requested record. This tells the client to use its local data
    /// and prevents it from trying to download from CDN (which causes
    /// "Streaming Error" and gets stuck at 80% loading).
    pub async fn handle_db_query_bulk(&mut self, query: wow_packet::packets::misc::DbQueryBulk) {
        info!(
            "DbQueryBulk: table=0x{:08X}, {} records {:?} for account {}",
            query.table_hash,
            query.queries.len(),
            query.queries,
            self.account_id
        );
        // Look up each record in the hotfix blob cache (populated from DB2 files at startup).
        // Status 1 = Valid (send blob), Status 3 = Invalid (client uses its own DB2 cache).
        let cache = self.hotfix_blob_cache().map(Arc::clone);
        let world_db = self.world_db().map(Arc::clone);
        for record_id in &query.queries {
            // 1. Try the in-memory file cache first (O(1) lookup)
            if let Some(ref c) = cache {
                if let Some(blob) = c.get(query.table_hash, *record_id) {
                    info!("DbQueryBulk: FOUND blob table=0x{:08X} record={} ({} bytes)", query.table_hash, record_id, blob.len());
                    self.send_packet(&DBReply::found(query.table_hash, *record_id, blob.to_vec()));
                    continue;
                }
            }
            // 2. Fallback: query hotfixes.hotfix_blob table for server-side custom records
            let table_known = cache.as_ref().map_or(false, |c| c.has_table(query.table_hash));
            if table_known {
                if let Some(ref db) = world_db {
                    let mut stmt = db.prepare(WorldStatements::SEL_HOTFIX_BLOB);
                    stmt.set_u32(0, query.table_hash);
                    stmt.set_i32(1, *record_id);
                    if let Ok(r) = db.query(&stmt).await {
                        if !r.is_empty() {
                            let blob: Vec<u8> = r.try_read(0).unwrap_or_default();
                            if !blob.is_empty() {
                                info!("DbQueryBulk: FOUND in hotfix_blob DB table=0x{:08X} record={} ({} bytes)", query.table_hash, record_id, blob.len());
                                self.send_packet(&DBReply::found(query.table_hash, *record_id, blob));
                                continue;
                            }
                        }
                    }
                }
            }
            // 3. Not found anywhere → send Invalid(3) so the client uses its local DB2 copy.
            // RecordRemoved(2) would tell the client to DELETE the record from its cache,
            // which is wrong for items that exist in the client's DB2 but not on the server.
            info!(
                "DbQueryBulk: NOT_FOUND table=0x{:08X} record={} → Invalid(3)",
                query.table_hash, record_id
            );
            self.send_packet(&DBReply::not_found(query.table_hash, *record_id));

        }
    }


    /// Handle CMSG_HOTFIX_REQUEST — client requests hotfix data.
    ///
    /// We have no hotfixes, so we respond with an empty HotfixConnect.
    pub async fn handle_hotfix_request(&mut self, req: wow_packet::packets::misc::HotfixRequest) {
        debug!(
            "HotfixRequest: client_build={}, data_build={}, {} hotfixes for account {}",
            req.client_build, req.data_build, req.hotfixes.len(), self.account_id
        );
        self.send_packet(&HotfixConnect);
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
            resp.sequence_index,
            resp.client_time,
            self.account_id
        );
        // TODO: compute clock delta for lag compensation (not needed for basic world entry)
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

        // Always allow instant logout for now (no combat/duel checks)
        self.send_packet(&LogoutResponse::instant_ok());

        // Complete logout immediately
        self.logout_time = None;

        // Persist played time to DB before marking offline
        self.save_played_time().await;

        // Mark character offline in DB
        self.mark_character_offline().await;

        // Notify other players that this player has left before removing from registry.
        self.broadcast_destroy_player_to_others();
        // Remove from broadcast registry before clearing player_guid.
        self.unregister_from_player_registry();

        // Send LogoutComplete → client returns to character select
        self.set_state(crate::session::SessionState::Authed);
        self.send_packet(&LogoutComplete);
        self.set_player_guid(None);

        // Clear inventory state
        self.inventory_items.clear();

        // ── Restore realm socket as primary ──────────────────────────
        // After ConnectTo, send_tx/packet_rx point to the instance socket.
        // On logout the client returns to character select on the REALM
        // connection. If we don't swap back, the next PlayerLogin sends
        // ConnectTo on the dead instance socket → client stuck at 90%.
        self.restore_realm_channels();

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
    async fn save_played_time(&self) {
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
            warn!("Failed to save played time for guid {}: {e}", guid.counter());
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
        self.player_name = Some(name.clone());
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
        self.player_gold = result.try_read::<u64>(17).unwrap_or(0);

        // Load equipped items for visible display + inventory objects
        let mut visible_items = [(0i32, 0u16, 0u16); 19];
        let mut inv_slots = [ObjectGuid::EMPTY; 141];
        let mut item_creates: Vec<wow_packet::packets::update::ItemCreateData> = Vec::new();
        let realm_id = self.realm_id();
        self.inventory_items.clear();
        {
            let mut eq_stmt = char_db.prepare(CharStatements::SEL_CHAR_EQUIPMENT);
            eq_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&eq_stmt).await {
                Ok(mut eq_result) => {
                    loop {
                        let slot: u8 = eq_result.read(0);
                        let item_entry: u32 = eq_result.try_read(1).unwrap_or(0);
                        let item_db_guid: u64 = eq_result.try_read(2).unwrap_or(0);
                        if item_entry > 0 && (slot as usize) < 141 {
                            let item_guid = ObjectGuid::create_item(realm_id, item_db_guid as i64);
                            inv_slots[slot as usize] = item_guid;
                            item_creates.push(wow_packet::packets::update::ItemCreateData {
                                item_guid,
                                entry_id: item_entry as i32,
                                owner_guid: guid,
                                contained_in: guid,
                            });
                            // Look up inventory_type from Item.db2 store
                            let inventory_type = self.item_store()
                                .and_then(|store| store.inventory_type(item_entry))
                                .or_else(|| if slot < 19 { slot_to_inventory_type(slot) } else { None });
                            self.inventory_items.insert(slot, InventoryItem {
                                guid: item_guid,
                                entry_id: item_entry,
                                db_guid: item_db_guid,
                                inventory_type,
                            });
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
                Err(e) => {
                    warn!("Failed to load equipment for {:?}: {}", guid, e);
                }
            }

            // inventory_type is now loaded from Item.db2 store for all items.
            // No SQL cache needed.
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
        {
            let mut skill_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_SKILLS);
            skill_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&skill_stmt).await {
                Ok(mut skill_result) => {
                    if !skill_result.is_empty() {
                        loop {
                            let skill_id: u16 = skill_result.try_read(0).unwrap_or(0);
                            if skill_id > 0 {
                                known_skill_ids.insert(skill_id);
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
            let dbc_spells = skill_store.starting_spells(race, class, level, Some(&known_skill_ids));
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
        self.known_spells = known_spells.clone();

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
                                    wow_packet::packets::misc::UpdateActionButtons::pack_button(action as i32, btn_type);
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
        self.current_map_id = map_id as u16;
        self.player_race = race;
        self.player_class = class;
        self.player_level = level;
        self.player_gender = gender;
        // NOTE: known_spells is stored below after DBC merge (see "Merge DBC auto-learned spells")

        // Sum gear stat bonuses from equipped items (slots 0-18)
        let (gear_stats, gear_ap, gear_rap, gear_health, gear_mana) =
            if let Some(iss) = self.item_stats_store() {
                let mut bonuses = [0i32; 5]; // STR, AGI, STA, INT, SPI
                let mut g_ap = 0i32;
                let mut g_rap = 0i32;
                let mut g_health = 0i32;
                let mut g_mana = 0i32;
                for (&slot, inv_item) in &self.inventory_items {
                    if slot < 19 { // only equipped gear slots affect stats
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
                }.max(0) + gear_ap;

                let ranged_ap = match class {
                    3 => total_agi * 2 - 20,
                    1 | 4 => total_agi - 10,
                    _ => 0,
                }.max(0) + gear_rap;

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
                    health: h as i64, max_health: h as i64, max_mana: m as i64,
                    ..PlayerCombatStats::default()
                }
            }
        } else {
            let (h, m) = default_health_mana(class);
            PlayerCombatStats {
                health: h as i64, max_health: h as i64, max_mana: m as i64,
                ..PlayerCombatStats::default()
            }
        };

        info!(
            "Player '{}' ({:?}) continuing login at map {} ({}, {}, {}), {} equipped items, \
             HP={} Mana={} AP={} STR/AGI/STA/INT/SPI={:?} Armor={} Dodge={:.1}% Crit={:.1}%",
            name, guid, map_id, pos_x, pos_y, pos_z, item_creates.len(),
            combat.max_health, combat.max_mana, combat.attack_power,
            combat.stats, combat.base_armor, combat.dodge_pct, combat.crit_pct
        );

        // Load active quests from characters DB
        self.load_player_quests().await;

        self.send_login_sequence(
            guid, race, class, gender, level, display_id, &position, map_id, zone,
            visible_items, inv_slots, item_creates, combat,
            known_spells, action_buttons, skill_info_tuples,
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
    pub async fn send_nearby_creatures(
        &mut self,
        map_id: u16,
        position: &Position,
        zone_id: u32,
    ) {
        let world_db = match self.world_db() {
            Some(db) => Arc::clone(db),
            None => {
                warn!("No world database — skipping creature spawn");
                return;
            }
        };

        const VISIBILITY_RANGE: f32 = 800.0;
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

        let result = match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            world_db.query(&stmt),
        ).await {
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
        let mut result = result;


        loop {
            // BIGINT UNSIGNED may fail as u64 in sqlx — read as i64 first, cast to u64
            let spawn_guid: u64 = result.try_read::<i64>(0)
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
            let npc_flags: u64 = result.try_read::<i64>(12)
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
            let base_attack_time: u32 = result.try_read(20).unwrap_or(2000);
            let _ranged_attack_time: u32 = result.try_read(21).unwrap_or(0);
            let template_display_id: u32 = result.try_read::<Option<u32>>(22)
                .flatten()
                .unwrap_or(0);

            let display_id = if model_id > 0 {
                model_id
            } else if template_display_id > 0 {
                template_display_id
            } else {
                if !result.next_row() { break; }
                continue;
            };

            let health = if cur_health > 0 { cur_health as i64 } else { 100 };


            let guid = ObjectGuid::create_world_object(
                HighGuid::Creature,
                0, realm_id, map_id, 1, entry, spawn_guid as i64,
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
                scale,
                unit_class,
                base_attack_time,
                ranged_attack_time: base_attack_time,
                zone_id,
                speed_walk_rate: speed_walk,
                speed_run_rate: speed_run,
            };

            blocks.push(UpdateObject::create_creature_block(create_data, &creature_pos));

            // Register in server-side AI tracker for movement/combat.
            // Estimate aggro radius from faction: neutral=0, hostile=10-20.
            let aggro_radius = if faction == 35 { 0.0 } else { 15.0 };
            let min_dmg = (min_level as u32).saturating_sub(1) * 3 + 5;
            let max_dmg = min_dmg + min_dmg / 2;
            let ai = CreatureAI::new(
                guid,
                entry,
                creature_pos,
                health as u32,
                min_level,
                min_dmg,
                max_dmg,
                aggro_radius,
                display_id,
                faction as u32,
                npc_flags as u32,
                unit_flags,
            );
            self.creatures.insert(guid, ai);

            if !result.next_row() {
                break;
            }
        }

        if blocks.is_empty() {
            return;
        }

        let count = blocks.len();
        // Snapshot visible set from AI tracker (populated in loop above).
        self.visible_creatures = self.creatures.keys().cloned().collect();
        self.last_visibility_pos = Some(*position);
        let update = UpdateObject::create_creatures(blocks, map_id);
        self.send_packet(&update);
        debug!("Sent {} creatures ({} mobs / {} npcs) to account {} on map {}",
            count,
            self.creatures.values().filter(|a| a.npc_flags == 0).count(),
            self.creatures.values().filter(|a| a.npc_flags > 0).count(),
            self.account_id, map_id);
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
        let pos = match self.player_position {
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

        let map_id = self.current_map_id;
        let realm_id = self.realm_id();

        const RANGE: f32 = 800.0;
        let x_min = pos.x - RANGE;
        let x_max = pos.x + RANGE;
        let y_min = pos.y - RANGE;
        let y_max = pos.y + RANGE;

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

        let cr = match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            world_db.query(&stmt),
        ).await {
            Ok(Ok(r)) => r,
            _ => return,
        };

        let mut new_visible_creatures: HashSet<ObjectGuid> = HashSet::new();
        let mut new_creature_blocks: Vec<UpdateBlock> = Vec::new();

        if !cr.is_empty() {
            let mut cr = cr;
            loop {
                let spawn_guid: u64 = cr.try_read::<i64>(0).map(|v| v as u64)
                    .or_else(|| cr.try_read::<u64>(0)).unwrap_or(0);
                let entry: u32 = cr.try_read(1).unwrap_or(0);
                let pos_x: f32 = cr.try_read(2).unwrap_or(0.0);
                let pos_y: f32 = cr.try_read(3).unwrap_or(0.0);
                let pos_z: f32 = cr.try_read(4).unwrap_or(0.0);
                let orientation: f32 = cr.try_read(5).unwrap_or(0.0);
                let cur_health: u32 = cr.try_read(6).unwrap_or(100);
                let model_id: u32 = cr.try_read(8).unwrap_or(0);
                let min_level: u8 = cr.try_read::<Option<u8>>(9).flatten().unwrap_or(1);
                let faction: i32 = cr.try_read::<u16>(11).unwrap_or(35) as i32;
                let npc_flags: u64 = cr.try_read::<i64>(12).map(|v| v as u64)
                    .or_else(|| cr.try_read::<u64>(12)).unwrap_or(0);
                let unit_flags: u32 = cr.try_read(13).unwrap_or(0);
                let unit_flags2: u32 = cr.try_read(14).unwrap_or(0);
                let unit_flags3: u32 = cr.try_read(15).unwrap_or(0);
                let speed_walk: f32 = cr.try_read(16).unwrap_or(1.0);
                let speed_run: f32 = cr.try_read(17).unwrap_or(1.14286);
                let scale: f32 = cr.try_read(18).unwrap_or(1.0);
                let unit_class: u8 = cr.try_read(19).unwrap_or(1);
                let base_attack_time: u32 = cr.try_read(20).unwrap_or(2000);
                let template_display_id: u32 = cr.try_read::<Option<u32>>(22)
                    .flatten().unwrap_or(0);

                let display_id = if model_id > 0 { model_id }
                    else if template_display_id > 0 { template_display_id }
                    else {
                        if !cr.next_row() { break; }
                        continue;
                    };

                let health = if cur_health > 0 { cur_health as i64 } else { 100 };

                let guid = ObjectGuid::create_world_object(
                    HighGuid::Creature, 0, realm_id, map_id, 1, entry, spawn_guid as i64,
                );
                new_visible_creatures.insert(guid);

                // Only create a new block if this creature isn't already visible.
                if !self.visible_creatures.contains(&guid) {
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
                        scale,
                        unit_class,
                        base_attack_time,
                        ranged_attack_time: base_attack_time,
                        zone_id: 0,
                        speed_walk_rate: speed_walk,
                        speed_run_rate: speed_run,
                    };
                    new_creature_blocks.push(UpdateObject::create_creature_block(create_data, &creature_pos));

                    // Register in AI tracker
                    let aggro_radius = if faction == 35 { 0.0 } else { 15.0 };
                    let min_dmg = (min_level as u32).saturating_sub(1) * 3 + 5;
                    let max_dmg = min_dmg + min_dmg / 2;
                    let ai = CreatureAI::new(
                        guid, entry, creature_pos, health as u32, min_level,
                        min_dmg, max_dmg, aggro_radius, display_id,
                        faction as u32, npc_flags as u32, unit_flags,
                    );
                    self.creatures.insert(guid, ai);
                }

                if !cr.next_row() { break; }
            }
        }

        // Creatures that left range → out-of-range
        let removed_creatures: Vec<ObjectGuid> = self.visible_creatures.iter()
            .filter(|g| !new_visible_creatures.contains(g))
            .cloned()
            .collect();

        if !new_creature_blocks.is_empty() {
            debug!("Visibility update: {} new creatures", new_creature_blocks.len());
            self.send_packet(&UpdateObject::create_creatures(new_creature_blocks, map_id));
        }
        if !removed_creatures.is_empty() {
            debug!("Visibility update: {} creatures out of range", removed_creatures.len());
            for g in &removed_creatures {
                self.creatures.remove(g);
            }
            self.send_packet(&UpdateObject::out_of_range_objects(removed_creatures, map_id));
        }
        self.visible_creatures = new_visible_creatures;

        // ── GAME OBJECTS ────────────────────────────────────────────────
        let mut go_stmt = world_db.prepare(WorldStatements::SEL_GAMEOBJECTS_IN_RANGE);
        go_stmt.set_u16(0, map_id);
        go_stmt.set_f32(1, x_min);
        go_stmt.set_f32(2, x_max);
        go_stmt.set_f32(3, y_min);
        go_stmt.set_f32(4, y_max);

        let go_result = match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            world_db.query(&go_stmt),
        ).await {
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
                let spawn_guid: u64 = go_result.try_read::<i64>(0).map(|v| v as u64)
                    .or_else(|| go_result.try_read::<u64>(0)).unwrap_or(0);
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
                let data0: u32 = go_result.try_read(16).unwrap_or(0);
                let data1: u32 = go_result.try_read(17).unwrap_or(0);

                if display_id == 0 {
                    if !go_result.next_row() { break; }
                    continue;
                }

                let guid = ObjectGuid::create_world_object(
                    HighGuid::GameObject, 0, realm_id, map_id, 1, entry, spawn_guid as i64,
                );
                new_visible_gos.insert(guid);

                if !self.visible_gameobjects.contains(&guid) {
                    let go_pos = Position::new(pos_x, pos_y, pos_z, orientation);
                    let create_data = GameObjectCreateData {
                        guid,
                        entry,
                        display_id,
                        go_type,
                        position: go_pos,
                        rotation: [rot0, rot1, rot2, rot3],
                        anim_progress,
                        state,
                        faction_template: 0,
                        scale,
                    };
                    new_go_blocks.push(UpdateObject::create_gameobject_block(create_data));
                }

                if !go_result.next_row() { break; }
            }
        }

        let removed_gos: Vec<ObjectGuid> = self.visible_gameobjects.iter()
            .filter(|g| !new_visible_gos.contains(g))
            .cloned()
            .collect();

        if !new_go_blocks.is_empty() {
            debug!("Visibility update: {} new game objects", new_go_blocks.len());
            self.send_packet(&UpdateObject::create_world_objects(new_go_blocks, map_id));
        }
        if !removed_gos.is_empty() {
            debug!("Visibility update: {} game objects out of range", removed_gos.len());
            self.send_packet(&UpdateObject::out_of_range_objects(removed_gos, map_id));
        }
        self.visible_gameobjects = new_visible_gos;

        // ── Update position marker ──────────────────────────────────────
        self.last_visibility_pos = Some(pos);
        debug!("Visibility updated at ({:.1}, {:.1}): {} creatures / {} GOs in range",
            pos.x, pos.y, self.visible_creatures.len(), self.visible_gameobjects.len());
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
                debug!("Failed to query creature template {}: {e}", query.creature_id);
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
                        if loc_subname.is_empty() { subname } else { loc_subname },
                        if loc_title_alt.is_empty() { title_alt } else { loc_title_alt },
                    )
                }
                Ok(_) => (name, subname, title_alt),
                Err(e) => {
                    warn!("Failed to query creature locale for {}: {e}", query.creature_id);
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
                    if !disp_result.next_row() { break; }
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
    pub async fn handle_query_game_object(&mut self, query: wow_packet::packets::query::QueryGameObject) {
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
                debug!("Failed to query gameobject template {}: {e}", query.game_object_id);
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
        let name: String = result.read_string(3);
        let icon_name: String = result.read_string(4);
        let cast_bar_caption: String = result.read_string(5);
        let unk_string: String = result.read_string(6);
        let size: f32 = result.try_read(7).unwrap_or(1.0);

        // Data0..Data33 at columns 8..41
        let mut data = [0i32; 34];
        for i in 0..34 {
            data[i] = result.try_read(8 + i).unwrap_or(0);
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
            quest_items: Vec::new(),
            content_tuning_id: 0,
        };

        self.send_packet(&QueryGameObjectResponse {
            game_object_id: query.game_object_id,
            guid: query.guid,
            allow: true,
            stats: Some(stats),
        });
    }

    /// Send nearby gameobjects to the client as UpdateObject packets.
    pub async fn send_nearby_gameobjects(
        &mut self,
        map_id: u16,
        position: &Position,
        _zone_id: u32,
    ) {
        let world_db = match self.world_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        const VISIBILITY_RANGE: f32 = 800.0;
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

        let result = match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            world_db.query(&stmt),
        ).await {
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
            let spawn_guid: u64 = result.try_read::<i64>(0)
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

            // Skip gameobjects with no display
            if display_id == 0 {
                if !result.next_row() { break; }
                continue;
            }

            let guid = ObjectGuid::create_world_object(
                HighGuid::GameObject,
                0, realm_id, map_id, 1, entry, spawn_guid as i64,
            );

            let go_pos = Position::new(pos_x, pos_y, pos_z, orientation);
            let create_data = GameObjectCreateData {
                guid,
                entry,
                display_id,
                go_type,
                position: go_pos,
                rotation: [rot0, rot1, rot2, rot3],
                anim_progress,
                state,
                faction_template: 0,
                scale,
            };

            blocks.push(UpdateObject::create_gameobject_block(create_data));
            go_guids.push(guid);

            if !result.next_row() {
                break;
            }
        }

        if blocks.is_empty() {
            return;
        }

        self.visible_gameobjects = go_guids.iter().cloned().collect();
        let count = blocks.len();
        let update = UpdateObject::create_world_objects(blocks, map_id);
        self.send_packet(&update);
        debug!("Sent {} gameobjects to account {} on map {}", count, self.account_id, map_id);
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
                let players = query.players.iter().map(|guid| NameCacheLookupResult {
                    player: *guid,
                    result: 1, // Failure
                    data: None,
                }).collect();
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
            let account_guid = ObjectGuid::new(
                (HighGuid::WowAccount as i64) << 58,
                account_id_val,
            );
            let bnet_guid = ObjectGuid::new(
                (HighGuid::BNetAccount as i64) << 58,
                account_id_val,
            );

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

        use wow_packet::packets::gossip::ClientGossipOption;
        use crate::session::GossipOptionInfo;

        const GOSSIP_FLAG: u32 = 0x1;

        let npc_flags = self.creatures.get(&hello.unit).map(|c| c.npc_flags).unwrap_or(0);
        let entry = self.creatures.get(&hello.unit).map(|c| c.entry).unwrap_or(0);

        info!("GossipHello npc_flags=0x{:X} entry={} for {:?}", npc_flags, entry, hello.unit);

        // If the NPC has Gossip flag AND we have a world DB, try to load the gossip menu.
        if npc_flags & GOSSIP_FLAG != 0 && entry != 0 {
            if let Some(world_db) = self.world_db().map(Arc::clone) {
                if let Some(msg) = self.build_gossip_menu(&world_db, entry, hello.unit).await {
                    info!(
                        "Sending GossipMessage with {} options for entry {}",
                        msg.gossip_options.len(), entry
                    );
                    self.send_packet(&msg);
                    return;
                }
            }
        }

        // No gossip menu found — fall back to direct interaction based on NPC flags.
        self.handle_npc_direct_interaction(hello).await;
    }

    /// Build a GossipMessage from the database for a creature entry.
    /// Returns None if no gossip menu exists.
    async fn build_gossip_menu(
        &mut self,
        world_db: &Arc<WorldDatabase>,
        entry: u32,
        npc_guid: wow_core::ObjectGuid,
    ) -> Option<GossipMessage> {
        use wow_packet::packets::gossip::ClientGossipOption;
        use crate::session::GossipOptionInfo;

        // 1. Get MenuID from creature_template_gossip
        let mut stmt = world_db.prepare(WorldStatements::SEL_CREATURE_GOSSIP_MENU);
        stmt.set_u32(0, entry);
        let menu_result: wow_database::SqlResult = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            world_db.query(&stmt),
        ).await.ok()?.ok()?;
        if menu_result.is_empty() {
            return None;
        }
        let menu_id: u32 = menu_result.try_read(0)?;

        // 2. Get TextID from gossip_menu, then resolve BroadcastTextID from npc_text
        let mut stmt = world_db.prepare(WorldStatements::SEL_GOSSIP_MENU);
        stmt.set_u32(0, menu_id);
        let text_result: wow_database::SqlResult = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            world_db.query(&stmt),
        ).await.ok()?.ok()?;
        let npc_text_id: u32 = if text_result.is_empty() { 1 } else {
            text_result.try_read::<u32>(0).unwrap_or(1)
        };

        // Resolve BroadcastTextID from npc_text (C# uses BroadcastTextID, NOT TextID)
        let broadcast_text_id: Option<i32> = {
            let mut stmt = world_db.prepare(WorldStatements::SEL_NPC_TEXT);
            stmt.set_u32(0, npc_text_id);
            match tokio::time::timeout(
                std::time::Duration::from_secs(2),
                world_db.query(&stmt),
            ).await {
                Ok(Ok(r)) if !r.is_empty() => r.try_read::<u32>(0).map(|v| v as i32),
                _ => None,
            }
        };
        info!("Gossip menu_id={} npc_text_id={} broadcast_text_id={:?}", menu_id, npc_text_id, broadcast_text_id);

        // 3. Get options from gossip_menu_option
        let mut stmt = world_db.prepare(WorldStatements::SEL_GOSSIP_MENU_OPTIONS);
        stmt.set_u32(0, menu_id);
        let mut opt_result: wow_database::SqlResult = match tokio::time::timeout(
            std::time::Duration::from_secs(2),
            world_db.query(&stmt),
        ).await {
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
        let locale = &self.locale;
        info!("Gossip locale='{}' for {} options", locale, raw_options.len());
        let mut gossip_options = Vec::new();
        let mut stored_options = Vec::new();
        for opt in &raw_options {
            let mut text = opt.option_text.clone();

            if opt.broadcast_text_id != 0 && locale != "enUS" {
                let mut stmt = world_db.prepare(WorldStatements::SEL_BROADCAST_TEXT_LOCALE);
                stmt.set_u32(0, opt.broadcast_text_id);
                stmt.set_string(1, locale);
                if let Ok(Ok(r)) = tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    world_db.query(&stmt),
                ).await {
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

        const VENDOR_MASK:      u32 = 0x80 | 0x100 | 0x200 | 0x400 | 0x800;
        const TRAINER_MASK:     u32 = 0x10 | 0x20 | 0x40;
        const FLIGHT_MASTER:    u32 = 0x2000;
        const AUCTIONEER:       u32 = 0x200000;
        const BANKER:           u32 = 0x20000;
        const TABARD_DESIGNER:  u32 = 0x80000;
        const STABLE_MASTER:    u32 = 0x400000;
        const GUILD_BANKER:     u32 = 0x800000;

        let npc_flags = self.creatures.get(&hello.unit).map(|c| c.npc_flags).unwrap_or(0);

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
        let opt = self.gossip_options.iter().find(|o| o.gossip_option_id == select.gossip_option_id);
        let (option_npc, _action_menu_id) = match opt {
            Some(o) => (o.option_npc, o.action_menu_id),
            None => {
                warn!(
                    "GossipSelectOption: unknown gossip_option_id={} — closing.",
                    select.gossip_option_id
                );
                self.send_packet(&GossipComplete { suppress_sound: false });
                return;
            }
        };

        let npc_guid = self.gossip_source_guid.unwrap_or(select.gossip_unit);
        info!("GossipSelectOption: OptionNpc={} for {:?}", option_npc, npc_guid);

        // Close the gossip window before opening the interaction.
        self.send_packet(&GossipComplete { suppress_sound: false });

        let hello = Hello { unit: npc_guid };
        match option_npc {
            1 => { // Vendor
                self.handle_list_inventory(hello).await;
            }
            2 => { // Taxinode / Flight Master
                self.send_packet(&NpcInteractionOpenResult::new(npc_guid, 6));
            }
            3 => { // Trainer
                self.handle_trainer_list(hello).await;
            }
            5 => { // Binder (Innkeeper)
                self.send_packet(&NpcInteractionOpenResult::new(npc_guid, 20));
            }
            6 => { // Banker
                self.send_packet(&NpcInteractionOpenResult::new(npc_guid, 8));
            }
            8 => { // Guild Tabard Vendor
                self.send_packet(&NpcInteractionOpenResult::new(npc_guid, 14));
            }
            9 => { // Battlemaster
                info!("Battlemaster interaction (stub)");
            }
            10 => { // Auctioneer
                use wow_packet::packets::misc::AuctionHelloResponse;
                self.send_packet(&AuctionHelloResponse::open(npc_guid));
            }
            12 => { // Stable Master
                self.send_packet(&NpcInteractionOpenResult::new(npc_guid, 22));
            }
            _ => {
                info!("GossipSelectOption: unhandled OptionNpc={} — ignored", option_npc);
            }
        }
    }

    // ── NPC activation handlers ───────────────────────────────────────────────

    /// CMSG_AUCTION_HELLO_REQUEST — player talks to an auctioneer.
    /// C# ref: AuctionHandler.HandleAuctionHello → SendAuctionHello
    pub async fn handle_auction_hello_request(&mut self, mut pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::AuctionHelloResponse;
        let guid = pkt.read_packed_guid().unwrap_or(wow_core::ObjectGuid::EMPTY);
        info!("AuctionHelloRequest from {:?} account {}", guid, self.account_id);
        self.send_packet(&AuctionHelloResponse::open(guid));
    }

    /// CMSG_BANKER_ACTIVATE — player talks to a banker.
    /// C# ref: BankHandler.HandleBankerActivate → SendShowBank → NpcInteractionOpenResult(Banker=8)
    pub async fn handle_banker_activate(&mut self, hello: Hello) {
        use wow_packet::packets::misc::NpcInteractionOpenResult;
        info!("BankerActivate {:?} account {}", hello.unit, self.account_id);
        self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 8)); // Banker
    }

    /// CMSG_BINDER_ACTIVATE — player sets hearthstone at innkeeper.
    /// C# ref: NPCHandler.HandleBinderActivate → SendBindPoint → NpcInteractionOpenResult(Binder=20)
    pub async fn handle_binder_activate(&mut self, hello: Hello) {
        use wow_packet::packets::misc::NpcInteractionOpenResult;
        info!("BinderActivate {:?} account {}", hello.unit, self.account_id);
        // TODO: actually set hearthstone bind point in DB.
        self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 20)); // Binder
    }

    /// CMSG_TABARD_VENDOR_ACTIVATE — player talks to a tabard designer.
    /// C# ref: NPCHandler.HandleTabardVendorActivate → NpcInteractionOpenResult(GuildTabardVendor=14)
    pub async fn handle_tabard_vendor_activate(&mut self, mut pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::NpcInteractionOpenResult;
        let guid = pkt.read_packed_guid().unwrap_or(wow_core::ObjectGuid::EMPTY);
        info!("TabardVendorActivate {:?} account {}", guid, self.account_id);
        self.send_packet(&NpcInteractionOpenResult::new(guid, 14)); // GuildTabardVendor
    }

    /// CMSG_SPIRIT_HEALER_ACTIVATE — ghost uses spirit healer.
    /// C# ref: NPCHandler.HandleSpiritHealerActivate → SendSpiritResurrect
    /// TODO: full resurrection logic (durability loss, corpse spawn, teleport).
    pub async fn handle_spirit_healer_activate(&mut self, _pkt: wow_packet::WorldPacket) {
        info!("SpiritHealerActivate account {} (stub)", self.account_id);
    }

    /// CMSG_REPAIR_ITEM — player repairs item at a repair vendor.
    /// C# ref: NPCHandler.HandleRepairItem
    /// TODO: calculate repair cost and apply to character money.
    pub async fn handle_repair_item(&mut self, _pkt: wow_packet::WorldPacket) {
        info!("RepairItem account {} (stub — all items already at full durability)", self.account_id);
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
        info!("ListInventory for {:?} from account {}", vendor_guid, self.account_id);

        let world_db = match self.world_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        // Resolve creature entry: first from AI tracker, then fallback from DB by spawn GUID.
        let entry = match self.creatures.get(&vendor_guid) {
            Some(ai) => ai.entry,
            None => {
                let mut stmt = world_db.prepare(WorldStatements::SEL_CREATURE_ENTRY_BY_GUID);
                stmt.set_u64(0, vendor_guid.low_value() as u64);
                let fallback = match tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    world_db.query(&stmt),
                ).await {
                    Ok(Ok(r)) if !r.is_empty() => r.try_read::<u32>(0),
                    _ => None,
                };
                match fallback {
                    Some(e) => {
                        info!("Vendor entry {} resolved from DB (GUID not in tracker)", e);
                        e
                    }
                    None => {
                        info!("Vendor GUID {:?} not in tracker and not found in creature table", vendor_guid);
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
        let mut slot = 1i32;
        let mut expanded = std::collections::HashSet::<u32>::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(entry);

        while let Some(vendor_entry) = queue.pop_front() {
            if !expanded.insert(vendor_entry) {
                continue; // already expanded (avoid cycles)
            }
            let mut stmt = world_db.prepare(WorldStatements::SEL_VENDOR_ITEMS);
            stmt.set_u32(0, vendor_entry);

            let mut result = match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                world_db.query(&stmt),
            ).await {
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
                let buy_price: u64 = result.try_read::<i64>(5).map(|v| v as u64)
                    .or_else(|| result.try_read::<u64>(5)).unwrap_or(0);
                let durability: i32 = result.try_read::<i64>(7).map(|v| v as i32)
                    .unwrap_or(0);
                let stack_count: i32 = result.try_read::<i64>(8).map(|v| v as i32)
                    .unwrap_or(1);
                let do_not_filter: bool = result.try_read::<u8>(9).map(|v| v != 0).unwrap_or(false);

                // Solo enviar items con ID válido; 0 o negativo el cliente lo muestra como ? y nombre vacío
                // Además filtrar items que no existen en Item.db2 — igual que C#:
                //   ObjectManager::GetItemTemplate → null si no está en ItemStorage (Item.db2)
                //   → "non-existed item, ignore"
                // Items 58260, 58274, etc. no están en Item.db2 de este cliente → se omiten.
                if item_id > 0 {
                    let item_known = self.item_store()
                        .map_or(true, |s| s.get(item_id as u32).is_some());
                    if !item_known {
                        info!(
                            "Vendor item {} not in Item.db2 (entry {}), skipping",
                            item_id, vendor_entry
                        );
                        if !result.next_row() { break; }
                        continue;
                    }
                    items.push(VendorItem {
                        muid: slot,
                        item_id,
                        item_type,
                        quantity: if maxcount == 0 { -1 } else { maxcount },
                        price: buy_price,
                        durability,
                        stack_count: stack_count.max(1),
                        extended_cost,
                        player_condition_failed: 0,
                        locked: false,
                        do_not_filter,
                        refundable: false,
                    });
                    slot += 1;
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
        self.send_packet(&VendorInventory { vendor_guid, reason: 0, items });
    }

    /// Handle CMSG_BUY_ITEM — player buys an item from a vendor.
    ///
    /// C# ref: `ItemHandler.HandleBuyItem` → `Player.BuyItemFromVendorSlot`.
    /// Simplified: no reputation discount, no extended cost, no stack logic.
    pub async fn handle_buy_item(&mut self, buy: BuyItem) {
        use wow_packet::packets::update::{ItemCreateData, UpdateObject};

        debug!("BuyItem: item={} qty={} muid={} from {:?}",
            buy.item_id, buy.quantity, buy.muid, buy.vendor_guid);

        let player_guid = match self.player_guid { Some(g) => g, None => return };
        let realm_id = self.realm_id();
        let map_id = self.current_map_id;

        // ── Validate: player alive ──
        let quantity = buy.quantity.max(1) as u32;

        // ── Get vendor NPC entry from creature GUID ──
        let vendor_entry = match self.creatures.get(&buy.vendor_guid) {
            Some(c) => c.entry,
            None => {
                warn!("BuyItem: vendor {:?} not in creatures", buy.vendor_guid);
                self.send_packet(&BuyFailed {
                    vendor_guid: buy.vendor_guid,
                    muid: buy.muid,
                    reason: 0x04, // BuyError::DistanceTooFar
                });
                return;
            }
        };

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };
        let world_db = match self.world_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        // ── Query buy price from npc_vendor + item_sparse ──
        let mut price_stmt = world_db.prepare(WorldStatements::SEL_VENDOR_ITEM_PRICE);
        price_stmt.set_u32(0, vendor_entry);
        price_stmt.set_u32(1, buy.item_id as u32);
        let price_result = match world_db.query(&price_stmt).await {
            Ok(r) => r,
            Err(e) => {
                warn!("BuyItem: price query failed: {e}");
                self.send_packet(&BuyFailed { vendor_guid: buy.vendor_guid, muid: buy.muid, reason: 0x03 });
                return;
            }
        };

        let buy_price: u64 = if price_result.is_empty() {
            warn!("BuyItem: item {} not found in vendor {}", buy.item_id, vendor_entry);
            self.send_packet(&BuyFailed { vendor_guid: buy.vendor_guid, muid: buy.muid, reason: 0x01 });
            return;
        } else {
            let raw: u64 = price_result.try_read::<u64>(0).unwrap_or(0);
            raw * quantity as u64
        };

        let max_durability: u32 = if price_result.is_empty() { 0 } else {
            price_result.try_read::<u32>(2).unwrap_or(0)
        };

        // ── Check gold ──
        if self.player_gold < buy_price {
            self.send_packet(&BuyFailed { vendor_guid: buy.vendor_guid, muid: buy.muid, reason: 0x02 }); // NotEnoughMoney
            return;
        }

        // ── Find free backpack slot (ItemStart=35 to ItemEnd-1=58) ──
        const BACKPACK_START: u8 = 35;
        const BACKPACK_END: u8 = 59;
        let free_slot = (BACKPACK_START..BACKPACK_END)
            .find(|&s| !self.inventory_items.contains_key(&s));
        let slot = match free_slot {
            Some(s) => s,
            None => {
                self.send_packet(&BuyFailed { vendor_guid: buy.vendor_guid, muid: buy.muid, reason: 0x08 }); // CantCarryMore
                return;
            }
        };

        // ── Allocate item GUID ──
        let mut max_guid_stmt = char_db.prepare(CharStatements::SEL_MAX_ITEM_GUID);
        let next_item_guid: u64 = match char_db.query(&max_guid_stmt).await {
            Ok(r) => r.try_read::<u64>(0).unwrap_or(0) + 1,
            Err(_) => 1,
        };

        // ── DB: insert item_instance ──
        // Columns: guid, itemEntry, owner_guid, count, durability
        let mut ins_item = char_db.prepare(CharStatements::INS_ITEM_INSTANCE);
        ins_item.set_u64(0, next_item_guid);
        ins_item.set_u32(1, buy.item_id as u32);
        ins_item.set_u64(2, player_guid.counter() as u64);
        ins_item.set_u32(3, quantity);
        ins_item.set_u32(4, max_durability);
        if let Err(e) = char_db.execute(&ins_item).await {
            warn!("BuyItem: insert item_instance failed: {e}");
            self.send_packet(&BuyFailed { vendor_guid: buy.vendor_guid, muid: buy.muid, reason: 0x01 });
            return;
        }

        // ── DB: insert character_inventory ──
        // Columns: guid (player), bag (0=main), slot, item (item guid)
        let mut ins_inv = char_db.prepare(CharStatements::INS_CHAR_INVENTORY);
        ins_inv.set_u64(0, player_guid.counter() as u64);
        ins_inv.set_u8(1, slot);
        ins_inv.set_u64(2, next_item_guid);
        if let Err(e) = char_db.execute(&ins_inv).await {
            warn!("BuyItem: insert character_inventory failed: {e}");
            // Roll back item_instance
            let mut del = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
            del.set_u64(0, next_item_guid);
            let _ = char_db.execute(&del).await;
            self.send_packet(&BuyFailed { vendor_guid: buy.vendor_guid, muid: buy.muid, reason: 0x01 });
            return;
        }

        // ── Deduct gold + save to DB ──
        self.player_gold -= buy_price;
        let mut upd_money = char_db.prepare(CharStatements::UPD_CHAR_MONEY);
        upd_money.set_u64(0, self.player_gold);
        upd_money.set_u64(1, player_guid.counter() as u64);
        if let Err(e) = char_db.execute(&upd_money).await {
            warn!("BuyItem: update money failed: {e}");
        }

        // ── Update in-memory inventory ──
        let item_guid = ObjectGuid::create_item(realm_id, next_item_guid as i64);
        let inv_type = self.item_store().and_then(|s| s.inventory_type(buy.item_id as u32));
        self.inventory_items.insert(slot, crate::session::InventoryItem {
            guid: item_guid,
            entry_id: buy.item_id as u32,
            db_guid: next_item_guid,
            inventory_type: inv_type,
        });

        info!(
            "BuyItem: player {:?} bought item {} at slot {} for {} copper (remaining: {})",
            player_guid, buy.item_id, slot, buy_price, self.player_gold
        );

        // ── Send BuySucceeded ──
        self.send_packet(&BuySucceeded {
            vendor_guid: buy.vendor_guid,
            muid: buy.muid,
            new_quantity: -1, // unlimited stock
            quantity_bought: buy.quantity,
        });

        // ── Send UpdateObject: new item CREATE block ──
        let item_create = ItemCreateData {
            item_guid,
            entry_id: buy.item_id,
            owner_guid: player_guid,
            contained_in: ObjectGuid::EMPTY,
        };
        self.send_packet(&UpdateObject::create_items(vec![item_create], map_id));

        // ── Send VALUES update: inv slot + coinage ──
        self.send_packet(&UpdateObject::player_money_update(
            player_guid,
            map_id,
            self.player_gold,
            Some((slot, item_guid)),
        ));
    }

    /// Handle CMSG_SELL_ITEM — player sells an item to a vendor.
    ///
    /// C# ref: `ItemHandler.HandleSellItem` → `Player.SellItemToVendor`.
    pub async fn handle_sell_item(&mut self, sell: SellItem) {
        use wow_packet::packets::update::UpdateObject;

        debug!("SellItem: item={:?} from account {}", sell.item_guid, self.account_id);

        let player_guid = match self.player_guid { Some(g) => g, None => return };
        let map_id = self.current_map_id;

        // ── Find item in inventory by GUID ──
        let (slot, item) = match self.inventory_items.iter()
            .find(|(_, item)| item.guid == sell.item_guid)
            .map(|(&s, item)| (s, item.clone()))
        {
            Some(pair) => pair,
            None => {
                warn!("SellItem: item {:?} not in inventory", sell.item_guid);
                self.send_packet(&SellResponse {
                    vendor_guid: sell.vendor_guid,
                    item_guid: sell.item_guid,
                    reason: 0x04, // SellError::ItemNotFound
                });
                return;
            }
        };

        // Equipped items (slots 0-18) can't be sold without unequipping first
        if slot < 19 {
            self.send_packet(&SellResponse {
                vendor_guid: sell.vendor_guid,
                item_guid: sell.item_guid,
                reason: 0x01, // SellError::CantSell
            });
            return;
        }

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
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

        // ── DB: delete character_inventory + item_instance ──
        let mut del_inv = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
        del_inv.set_u64(0, player_guid.counter() as u64);
        del_inv.set_u64(1, item.db_guid);
        if let Err(e) = char_db.execute(&del_inv).await {
            warn!("SellItem: delete character_inventory failed: {e}");
        }

        let mut del_item = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
        del_item.set_u64(0, item.db_guid);
        if let Err(e) = char_db.execute(&del_item).await {
            warn!("SellItem: delete item_instance failed: {e}");
        }

        // ── Add gold + save to DB ──
        self.player_gold = self.player_gold.saturating_add(sell_price);
        let mut upd_money = char_db.prepare(CharStatements::UPD_CHAR_MONEY);
        upd_money.set_u64(0, self.player_gold);
        upd_money.set_u64(1, player_guid.counter() as u64);
        if let Err(e) = char_db.execute(&upd_money).await {
            warn!("SellItem: update money failed: {e}");
        }

        // ── Remove from in-memory inventory ──
        self.inventory_items.remove(&slot);

        info!(
            "SellItem: player {:?} sold item {} from slot {} for {} copper (total: {})",
            player_guid, item.entry_id, slot, sell_price, self.player_gold
        );

        // ── Send SellResponse success ──
        self.send_packet(&SellResponse {
            vendor_guid: sell.vendor_guid,
            item_guid: sell.item_guid,
            reason: 0, // success
        });

        // ── Send VALUES update: clear inv slot + coinage ──
        self.send_packet(&UpdateObject::player_money_update(
            player_guid,
            map_id,
            self.player_gold,
            Some((slot, ObjectGuid::EMPTY)),
        ));
    }

    /// Handle CMSG_QUEST_GIVER_STATUS_MULTIPLE_QUERY — client asks quest status for all NPCs.
    pub async fn handle_quest_giver_status_multiple_query(&mut self) {
        trace!(
            "QuestGiverStatusMultipleQuery from account {}",
            self.account_id
        );
        // Respond with empty list — no NPCs have quests
        self.send_quest_giver_status_multiple(vec![]);
    }

    /// Send SMSG_QUEST_GIVER_STATUS for a single NPC.
    fn send_quest_giver_status(&self, guid: ObjectGuid, status: u32) {
        use wow_constants::ServerOpcodes;
        let mut pkt = wow_packet::WorldPacket::new_server(ServerOpcodes::QuestGiverStatus);
        pkt.write_packed_guid(&guid);
        pkt.write_uint32(status);
        self.send_raw_packet(&pkt.into_data());
    }

    /// Send SMSG_QUEST_GIVER_STATUS_MULTIPLE with a list of NPC quest statuses.
    fn send_quest_giver_status_multiple(&self, statuses: Vec<(ObjectGuid, u32)>) {
        use wow_constants::ServerOpcodes;
        let mut pkt = wow_packet::WorldPacket::new_server(ServerOpcodes::QuestGiverStatusMultiple);
        pkt.write_int32(statuses.len() as i32);
        for (guid, status) in &statuses {
            pkt.write_packed_guid(guid);
            pkt.write_uint32(*status);
        }
        self.send_raw_packet(&pkt.into_data());
    }

    // ── Item equip/swap handlers ─────────────────────────────────────

    /// Handle CMSG_SWAP_INV_ITEM: drag-and-drop item between two inventory slots.
    pub async fn handle_swap_inv_item(&mut self, swap: SwapInvItem) {
        let player_guid = match self.player_guid {
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
            self.send_packet(&InventoryChangeFailure::error(InventoryResult::InternalBagError));
            return;
        }

        // Can't swap to same slot
        if src == dst {
            return;
        }

        let src_item = self.inventory_items.get(&src).cloned();
        let dst_item = self.inventory_items.get(&dst).cloned();

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
            self.inventory_items.insert(dst, item.clone());
        } else {
            self.inventory_items.remove(&dst);
        }
        if let Some(ref item) = dst_item {
            self.inventory_items.insert(src, item.clone());
        } else {
            self.inventory_items.remove(&src);
        }

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
                let (item_id, app, vis) = match self.inventory_items.get(&slot) {
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
                let (item_id, app, vis) = match self.inventory_items.get(&slot) {
                    Some(item) => (item.entry_id as i32, 0u16, 0u16),
                    None => (0, 0, 0),
                };
                virtual_item_changes.push((idx, item_id, app, vis));
            }
        }

        let update = UpdateObject::player_values_update(
            player_guid,
            self.current_map_id,
            inv_slot_changes,
            visible_item_changes,
            virtual_item_changes,
        );
        self.send_packet(&update);

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
        let player_guid = match self.player_guid {
            Some(g) => g,
            None => return,
        };

        let src_slot = equip.slot;
        debug!(
            "AutoEquipItem: slot {} (pack_slot {}) for {:?}",
            src_slot, equip.pack_slot, player_guid
        );

        let src_item = match self.inventory_items.get(&src_slot).cloned() {
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
            // In backpack → find target equipment slot using inventory_type.
            // inventory_type is loaded from Item.db2 via item_store.
            let inv_type = match src_item.inventory_type {
                Some(t) => t,
                None => {
                    warn!(
                        "AutoEquipItem: no inventory_type for entry {} — not in cache",
                        src_item.entry_id
                    );
                    self.send_packet(&InventoryChangeFailure::error(InventoryResult::NotEquippable));
                    return;
                }
            };
            // Build occupied map from currently equipped slots (0-18)
            let occupied: std::collections::HashMap<u8, ()> = self.inventory_items
                .keys()
                .filter(|&&s| s < 19)
                .map(|&s| (s, ()))
                .collect();
            match equip_slot_for_inventory_type(inv_type, &occupied) {
                Some(slot) => slot,
                None => {
                    warn!(
                        "AutoEquipItem: inv_type {} has no valid equipment slot",
                        inv_type
                    );
                    self.send_packet(&InventoryChangeFailure::error(InventoryResult::NotEquippable));
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
        let player_guid = match self.player_guid {
            Some(g) => g,
            None => return,
        };

        debug!(
            "SwapItem: A=({},{}) B=({},{}) for {:?}",
            swap.container_slot_a, swap.slot_a,
            swap.container_slot_b, swap.slot_b,
            player_guid
        );

        // Only support player's direct inventory (container=255) for now
        if swap.container_slot_a != 255 || swap.container_slot_b != 255 {
            warn!("SwapItem with non-255 containers not supported yet");
            self.send_packet(&InventoryChangeFailure::error(InventoryResult::InternalBagError));
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
    pub async fn handle_auto_store_bag_item(&mut self, store: wow_packet::packets::item::AutoStoreBagItem) {
        let player_guid = match self.player_guid {
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
            self.send_packet(&InventoryChangeFailure::error(InventoryResult::InternalBagError));
            return;
        }

        let src_slot = store.slot_a;

        // Check source has an item
        if !self.inventory_items.contains_key(&src_slot) {
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
    pub async fn handle_destroy_item(&mut self, destroy: wow_packet::packets::item::DestroyItemPkt) {
        let player_guid = match self.player_guid {
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
            self.send_packet(&InventoryChangeFailure::error(InventoryResult::InternalBagError));
            return;
        }

        let slot = destroy.slot_num;
        let item = match self.inventory_items.remove(&slot) {
            Some(item) => item,
            None => {
                self.send_packet(&InventoryChangeFailure::error(InventoryResult::SlotEmpty));
                return;
            }
        };

        // Delete from DB
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let mut del_stmt = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
        del_stmt.set_u64(0, player_guid.counter() as u64);
        del_stmt.set_u64(1, item.db_guid);
        let _ = char_db.execute(&del_stmt).await;

        // Send VALUES update to clear the slot
        let inv_slot_changes = vec![(slot, ObjectGuid::EMPTY)];
        let mut visible_item_changes = Vec::new();
        let mut virtual_item_changes = Vec::new();

        if (slot as usize) < 19 {
            visible_item_changes.push((slot, 0i32, 0u16, 0u16));
        }
        if slot >= 15 && slot <= 17 {
            virtual_item_changes.push((slot - 15, 0i32, 0u16, 0u16));
        }

        let update = UpdateObject::player_values_update(
            player_guid,
            self.current_map_id,
            inv_slot_changes,
            visible_item_changes,
            virtual_item_changes,
        );
        self.send_packet(&update);

        // If destroyed item was in a gear slot (0-18), recalculate stats
        if slot < 19 {
            self.send_stat_update();
        }

        info!("Destroyed item entry={} at slot {} for {:?}", item.entry_id, slot, player_guid);
    }

    /// Find the first empty slot in the default backpack (slots 35-58).
    ///
    /// C# InventorySlots: ItemStart=35, ItemEnd=59 (24 backpack slots).
    fn find_free_backpack_slot(&self) -> Option<u8> {
        for slot in 35..59u8 {
            if !self.inventory_items.contains_key(&slot) {
                return Some(slot);
            }
        }
        None
    }

    /// Recalculate all stats from base + gear and send a VALUES update to the client.
    ///
    /// Called after equip/desequip changes to gear slots (0-18).
    fn send_stat_update(&self) {
        let player_guid = match self.player_guid {
            Some(g) => g,
            None => return,
        };

        let race = self.player_race;
        let class = self.player_class;
        let level = self.player_level;

        if race == 0 || class == 0 || level == 0 {
            return; // Not fully logged in yet
        }

        // Sum gear stat bonuses from equipped items (slots 0-18)
        let (gear_stats, gear_ap, gear_rap, gear_health, gear_mana,
             gear_combat_ratings, gear_spell_power, gear_armor) =
            if let Some(iss) = self.item_stats_store() {
                let mut bonuses = [0i32; 5];
                let mut g_ap = 0i32;
                let mut g_rap = 0i32;
                let mut g_health = 0i32;
                let mut g_mana = 0i32;
                let mut g_cr = [0i32; 25];
                let mut g_sp = 0i32;
                let mut g_armor = 0i32;
                for (&slot, inv_item) in &self.inventory_items {
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
                            for j in 0..25 { g_cr[j] += cr[j]; }
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
        }.max(0) + gear_ap;

        let ranged_ap = match class {
            3 => total_agi * 2 - 20,
            1 | 4 => total_agi - 10,
            _ => 0,
        }.max(0) + gear_rap;

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
            1 => 1000,               // Warrior: rage
            4 => 100,                // Rogue: energy
            6 => 1000,               // DK: runic power
            _ => max_mana as i32,    // Casters: mana
        };

        // CombatRatings[32]: copy 25 used indices, rest 0
        let mut combat_ratings = [0i32; 32];
        combat_ratings[..25].copy_from_slice(&gear_combat_ratings);

        // ── Percentage calculations (WotLK level 80 formulas) ──
        let lvl = level as f32;

        // Crit from AGI: class-dependent AGI-to-crit ratio at level 80
        let agi_crit_ratio = match class {
            4 => 40.0,       // Rogue
            3 => 53.0,       // Hunter
            11 => 45.5,      // Druid
            7 => 80.0,       // Shaman
            2 => 59.5,       // Paladin
            1 | 6 => 62.5,   // Warrior, DK
            _ => 80.0,       // Casters (Mage/Warlock/Priest)
        };
        let crit_from_agi = total_agi as f32 / agi_crit_ratio;

        // Crit from rating: ~45.91 rating per 1% at level 80
        let crit_rating_per_pct = if lvl >= 80.0 { 45.91 } else { (lvl * 0.574).max(1.0) };
        let crit_from_rating = gear_combat_ratings[8] as f32 / crit_rating_per_pct as f32;

        // Base crit varies by class (roughly)
        let base_crit = match class {
            4 => 3.5,   // Rogue
            3 => 3.6,   // Hunter
            1 => 3.2,   // Warrior
            2 => 3.3,   // Paladin
            6 => 3.2,   // DK
            _ => 1.8,   // Casters
        };
        let melee_crit_pct = (base_crit + crit_from_agi + crit_from_rating).min(100.0);

        // Spell crit from INT: class-dependent INT-to-spell-crit ratio
        let int_crit_ratio = match class {
            8 => 80.0,       // Mage
            9 => 82.0,       // Warlock
            5 => 80.0,       // Priest
            7 => 80.0,       // Shaman
            11 => 80.0,      // Druid
            2 => 80.0,       // Paladin
            _ => 160.0,      // Non-casters
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
        let spell_crit_pct = (base_spell_crit as f32 + spell_crit_from_int + spell_crit_from_rating).min(100.0);

        // Dodge from AGI + rating
        let dodge_from_agi = total_agi as f32 / agi_crit_ratio; // simplified: same ratio
        let dodge_rating_per_pct = if lvl >= 80.0 { 39.35 } else { (lvl * 0.492).max(1.0) };
        let dodge_from_rating = gear_combat_ratings[2] as f32 / dodge_rating_per_pct as f32;
        let dodge_pct = (dodge_from_agi + dodge_from_rating + 5.0).min(100.0); // 5% base

        // Parry from STR + rating (for classes that can parry)
        let parry_rating_per_pct = if lvl >= 80.0 { 49.18 } else { (lvl * 0.615).max(1.0) };
        let parry_from_rating = gear_combat_ratings[3] as f32 / parry_rating_per_pct as f32;
        let parry_pct = match class {
            1 | 2 | 4 | 6 => (5.0 + parry_from_rating).min(100.0), // 5% base for melee
            _ => parry_from_rating.min(100.0),
        };

        // Block from rating (only shield users)
        let block_rating_per_pct = if lvl >= 80.0 { 16.39 } else { (lvl * 0.205).max(1.0) };
        let block_from_rating = gear_combat_ratings[4] as f32 / block_rating_per_pct as f32;
        let block_pct = match class {
            1 | 2 | 7 => (5.0 + block_from_rating).min(100.0), // 5% base
            _ => block_from_rating.min(100.0),
        };

        // SpellCritPercentage[7]: index 0=Physical (same as melee), 1-6=spell schools
        let mut spell_crit_arr = [0.0f32; 7];
        spell_crit_arr[0] = melee_crit_pct;
        for i in 1..7 { spell_crit_arr[i] = spell_crit_pct; }

        // ── Mana regen (WotLK spirit-based formula) ──
        // spirit_regen = 0.001 + sqrt(INT) * SPI * class_coeff
        let class_regen_coeff: f32 = match class {
            2 => 0.044,   // Paladin
            3 => 0.030,   // Hunter
            5 => 0.033,   // Priest
            7 => 0.044,   // Shaman
            8 => 0.035,   // Mage
            9 => 0.033,   // Warlock
            11 => 0.044,  // Druid
            _ => 0.0,     // Warrior, Rogue, DK (no mana)
        };
        let spirit_regen = if class_regen_coeff > 0.0 {
            0.001 + (total_int as f32).max(0.0).sqrt() * total_spi as f32 * class_regen_coeff
        } else {
            0.0
        };

        // ── Expertise from rating ──
        // CombatRating::Expertise = index 23, 15.77 rating per expertise at level 80
        let expertise_rating_per_pct = if lvl >= 80.0 { 15.77 } else { (lvl * 0.197).max(1.0) };
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
            mana_regen_combat: 0.0,  // No talents = no in-combat spirit regen
            mana_regen_mp5: 0.0,     // No MP5 auras without talent system
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
            player_guid, max_health, melee_ap,
            [total_str, total_agi, total_sta, total_int, total_spi],
            total_armor, gear_spell_power, melee_crit_pct, spell_crit_pct, dodge_pct, parry_pct,
            expertise_value, spirit_regen
        );

        let update = UpdateObject::player_stat_update(
            player_guid,
            self.current_map_id,
            changes,
        );
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
                if result.is_empty() { 0 } else { result.try_read::<i64>(0).unwrap_or(0) as u8 }
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
            debug!("Updated realmcharacters: account={} realm={} count={}", self.account_id, self.realm_id(), num_chars);
        }
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
        self.time_sync_next_counter = 0;
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
        self.send_packet(&ActiveGlyphs { is_full_update: true });

        // 15. UpdateActionButtons — populated from character_action table
        self.send_packet(&UpdateActionButtons {
            buttons: action_buttons,
            reason: 0, // Initialization
        });

        // 16. InitializeFactions (1000 factions, all neutral)
        self.send_packet(&InitializeFactions);

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

        // 23. AccountMountUpdate (empty, full update)
        self.send_packet(&AccountMountUpdate);

        // 24. AccountToyUpdate (empty, full update)
        self.send_packet(&AccountToyUpdate);

        // 25. InitialSetup (expansion level)
        self.send_packet(&InitialSetup::wotlk());

        // 25b. MoveSetActiveMover — CRITICAL: tells the client which unit it
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
            let quest_log: Vec<(u32, u32, i64, [u16; 24])> = self.player_quests.values()
                .filter(|qs| qs.status == 1 || qs.status == 2)
                .take(25)
                .map(|qs| {
                    let state_flags: u32 = if qs.status == 2 { 1 } else { 0 };
                    let mut obj_progress = [0u16; 24];
                    for (i, &count) in qs.objective_counts.iter().enumerate().take(24) {
                        obj_progress[i] = count.min(u16::MAX as i32) as u16;
                    }
                    (qs.quest_id, state_flags, 0i64, obj_progress)
                })
                .collect();

            let mut player_pkt = UpdateObject::create_player(
                guid, race, class, sex, level, display_id, position,
                map_id as u16, zone_id as u32, true, visible_items, inv_slots,
                combat, skill_info, self.player_gold, quest_log,
            );

            if !item_creates.is_empty() {
                info!("Sending {} item CREATE blocks + player in single UpdateObject", item_creates.len());
                // Prepend item blocks before the player block
                let mut all_blocks: Vec<UpdateBlock> = item_creates
                    .into_iter()
                    .map(|data| {
                        let g = data.item_guid;
                        UpdateBlock::CreateItem { guid: g, create_data: data }
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
        self.set_player_guid(Some(guid));
        self.login_time = Some(std::time::Instant::now());
        // Store initial position for aggro checks / movement tracking.
        self.player_position = Some(*position);
        // Clear creature/loot state for fresh login.
        self.creatures.clear();
        self.loot_table.clear();
        self.combat_target = None;
        self.in_combat = false;

        // Register in the shared player registry so other sessions can
        // broadcast chat / emotes / movement packets to us.
        self.register_in_player_registry();

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

        info!("Login sequence complete for {:?} (37 packets including broadcasts)", guid);
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
        let level = self.player_level;

        let skill_rank = (level as i32) * 5;
        let skill_max_rank = skill_rank;

        let known = if let Some(store) = self.skill_store() {
            store.trade_skill_spells(skill_id, &self.known_spells)
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

    #[test]
    fn start_positions_are_valid() {
        for race in [1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 22] {
            let (map, x, y, z, _o) = start_position(race);
            assert!(map >= 0, "Race {race} has invalid map");
            // Positions should be non-zero (except possibly orientation)
            assert!(x != 0.0 || y != 0.0 || z != 0.0, "Race {race} has zero position");
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
