// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Account-scoped character enumeration, offline marking and account collections.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use wow_database::{CharStatements, LoginStatements, PreparedStatement, SqlTransaction};

use wow_persistence::{PersistenceOutcomeLikeCpp, PlayerOfflineMarkLikeCpp};

use super::*;

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
        opcode: ClientOpcodes::CharacterRenameRequest,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_character_rename_request",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CharCustomize,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_char_customize",
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
        opcode: ClientOpcodes::OpeningCinematic,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_opening_cinematic",
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

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AlterAppearance,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_alter_appearance",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ConfirmBarbersChoice,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_confirm_barbers_choice",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetPlayerDeclinedNames,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_player_declined_names",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SaveEquipmentSet,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_save_equipment_set",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AssignEquipmentSetSpec,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_assign_equipment_set_spec",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DeleteEquipmentSet,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_delete_equipment_set",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UseEquipmentSet,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_use_equipment_set",
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
        opcode: ClientOpcodes::QueryCorpseLocationFromClient,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_corpse_location",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryCorpseTransport,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_corpse_transport",
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
        opcode: ClientOpcodes::ItemTextQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_item_text_query",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryPetName,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_pet_name",
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
        opcode: ClientOpcodes::AutobankItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_autobank_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutostoreBankItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_autostore_bank_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BuyBankSlot,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_buy_bank_slot",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChangeBankBagSlotFlag,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_change_bank_bag_slot_flag",
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
        opcode: ClientOpcodes::AreaSpiritHealerQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_area_spirit_healer_query",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AreaSpiritHealerQueue,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_area_spirit_healer_queue",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::HearthAndResurrect,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_hearth_and_resurrect",
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
        opcode: ClientOpcodes::AutoEquipItemSlot,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_auto_equip_item_slot",
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
        opcode: ClientOpcodes::CancelTempEnchantment,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_temp_enchantment",
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

        let (expire_bans_stmt, enum_stmt) =
            enum_character_query_statements_like_cpp(self.declined_names_used_like_cpp());
        let expire_bans_stmt = char_db.prepare(expire_bans_stmt);
        if let Err(e) = char_db.execute(&expire_bans_stmt).await {
            warn!(
                "Failed to expire elapsed character bans before enum for account {}: {e}",
                self.account_id
            );
        }

        let mut stmt = char_db.prepare(enum_stmt);
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
                let guild_id: u64 = result.try_read(11).unwrap_or(0); // nullable gm.guildid
                let player_flags: u32 = result.try_read(12).unwrap_or(0);
                let at_login_flags: u16 = result.try_read(13).unwrap_or(0); // smallint unsigned
                let pet_entry: u32 = result.try_read(14).unwrap_or(0);
                let pet_display_id: u32 = result.try_read(15).unwrap_or(0);
                let pet_level: u32 = result.try_read(16).unwrap_or(0);
                let equipment_cache: String = result.try_read(17).unwrap_or_default();
                let banned_guid: u64 = result.try_read(18).unwrap_or(0);
                let list_slot: u8 = result.try_read(19).unwrap_or(characters.len() as u8);
                let last_played_time: i64 = result.try_read(20).unwrap_or(0);
                let active_talent_group: i16 = result.try_read::<u8>(21).unwrap_or(0) as i16;
                let last_login_build: u32 = result.try_read(22).unwrap_or(54261);
                let declined_genitive = self
                    .declined_names_used_like_cpp()
                    .then(|| result.try_read::<String>(28).unwrap_or_default())
                    .unwrap_or_default();

                let realm_id = self.realm_id();
                let guid = ObjectGuid::create_player(realm_id, guid_low as i64);

                let enum_flags = enum_character_flags_like_cpp(
                    player_flags,
                    at_login_flags,
                    banned_guid,
                    (!declined_genitive.is_empty()).then_some(declined_genitive.as_str()),
                    self.declined_names_used_like_cpp(),
                );
                let (pet_display_id, pet_level, pet_family) = enum_character_pet_data_like_cpp(
                    player_flags,
                    at_login_flags,
                    class,
                    pet_entry,
                    pet_display_id,
                    pet_level,
                    self.creature_template_lifecycle_store_like_cpp()
                        .map(Arc::as_ref),
                );

                // Only add to legit list if not locked
                if (enum_flags.flags
                    & (CHARACTER_FLAG_LOCKED_FOR_TRANSFER_LIKE_CPP
                        | CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP))
                    == 0
                {
                    legit_guids.push(guid);
                }

                let char_info = CharacterInfo {
                    guid,
                    guild_club_member_id: 0,
                    name,
                    list_position: list_slot,
                    race_id: race,
                    class_id: class,
                    sex_id: gender,
                    experience_level: level,
                    zone_id: zone,
                    map_id: map,
                    position: Position::new(pos_x, pos_y, pos_z, 0.0),
                    guild_guid: if guild_id == 0 {
                        ObjectGuid::EMPTY
                    } else {
                        ObjectGuid::create_guild(HighGuid::Guild, realm_id, guild_id as i64)
                    },
                    flags: enum_flags.flags,
                    flags2: enum_flags.flags2,
                    flags3: 0,
                    flags4: 0,
                    first_login: enum_flags.first_login,
                    pet_display_id,
                    pet_level,
                    pet_family,
                    profession_ids: [0; 2],
                    equipment: parse_equipment_cache(&equipment_cache),
                    last_played_time,
                    spec_id: active_talent_group,
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

    /// Build and send SMSG_CONNECT_TO to the client.
    pub(super) fn send_connect_to(&mut self, serial: ConnectToSerial) {
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

    /// Handle CMSG_HOTFIX_REQUEST — client requests hotfix data.
    pub async fn handle_hotfix_request(&mut self, req: wow_packet::packets::misc::HotfixRequest) {
        info!(
            "HotfixRequest: client_build={}, data_build={}, {} hotfixes for account {}, first={:?}, last={:?}",
            req.client_build,
            req.data_build,
            req.hotfixes.len(),
            self.account_id,
            req.hotfixes.first(),
            req.hotfixes.last()
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
                    if let Some(blob) = cache.get_hotfix_blob(record.table_hash, record.record_id) {
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
                        // C++ known-store hotfixes use DB2StorageBase::WriteRecord, not raw WDC4
                        // bytes. Until Rust has that typed serializer, fail closed so the client
                        // keeps its local DB2 cache instead of parsing a malformed Valid payload.
                        status = HotfixRecordStatus::Invalid as u8;
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

    /// Mark the current character as offline (#200: through the lifecycle port).
    pub(crate) async fn mark_character_offline(&self) {
        let Some(guid) = self.player_guid() else {
            return;
        };
        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            return;
        };

        match port
            .mark_offline_like_cpp(PlayerOfflineMarkLikeCpp::Character {
                guid_low: guid.counter() as u32,
            })
            .await
        {
            PersistenceOutcomeLikeCpp::Applied { .. } => {
                info!("Marked character offline for guid {}", guid.counter());
            }
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to mark character offline: {reason}");
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!("Character offline mark outcome is unknown: {reason}");
            }
        }
    }

    pub(crate) fn build_character_account_offline_statement_like_cpp(
        account_id: u32,
    ) -> PreparedStatement {
        let mut stmt = PreparedStatement::for_statement(CharStatements::UPD_ACCOUNT_ONLINE);
        stmt.set_u32(0, account_id);
        stmt
    }

    /// Trinity marks every character for the active account offline after
    /// `SMSG_LOGOUT_COMPLETE` because one account can only have one online
    /// character.  See C++ `WorldSession::LogoutPlayer`.
    pub(crate) async fn mark_character_account_offline_like_cpp(&self) {
        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            warn!(
                account = self.account_id,
                "Character account offline save skipped: lifecycle persistence port unavailable"
            );
            return;
        };

        match port
            .mark_offline_like_cpp(PlayerOfflineMarkLikeCpp::CharacterAccount {
                account_id: self.account_id,
            })
            .await
        {
            PersistenceOutcomeLikeCpp::Applied { rows } => {
                info!(
                    account = self.account_id,
                    rows, "Marked character account offline like C++"
                );
            }
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    "Failed to mark character account offline like C++: {reason}"
                );
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    account = self.account_id,
                    "Character account offline mark outcome is unknown: {reason}"
                );
            }
        }
    }

    /// Mark the account as offline in the login database when the whole
    /// WorldSession is being destroyed, matching C++ `WorldSession::~WorldSession`.
    pub(crate) async fn mark_login_account_offline_on_disconnect_like_cpp(&self) {
        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            warn!(
                account = self.account_id,
                "Disconnect account offline save skipped: lifecycle persistence port unavailable"
            );
            return;
        };

        match port
            .mark_offline_like_cpp(PlayerOfflineMarkLikeCpp::LoginAccount {
                account_id: self.account_id,
            })
            .await
        {
            PersistenceOutcomeLikeCpp::Applied { .. } => {
                info!(
                    account = self.account_id,
                    "Marked login account offline on disconnect"
                );
            }
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    "Failed to mark login account offline on disconnect: {reason}"
                );
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    account = self.account_id,
                    "Login account offline mark outcome is unknown: {reason}"
                );
            }
        }
    }

    pub(crate) async fn save_account_mounts_like_cpp(&self) {
        let Some(login_db) = self.login_db().map(Arc::clone) else {
            return;
        };
        let save_rows = self.account_mount_save_rows_like_cpp();
        if save_rows.is_empty() {
            return;
        }

        let mut tx = SqlTransaction::new();
        for row in save_rows {
            let mut stmt = login_db.prepare(LoginStatements::REP_ACCOUNT_MOUNTS);
            stmt.set_u32(0, row.bnet_account_id);
            stmt.set_u32(1, row.mount_spell_id);
            stmt.set_u8(2, row.flags);
            tx.append(stmt);
        }

        if let Err(error) = login_db.commit_transaction(tx).await {
            warn!(
                account = self.account_id,
                bnet_account = self.battlenet_account_id(),
                "Failed to save account mount flags: {error}"
            );
        }
    }

    pub(crate) async fn save_account_toys_like_cpp(&self) {
        let Some(login_db) = self.login_db().map(Arc::clone) else {
            return;
        };
        let save_rows = self.account_toy_save_rows_like_cpp();
        if save_rows.is_empty() {
            return;
        }

        let mut tx = SqlTransaction::new();
        for row in save_rows {
            let mut stmt = login_db.prepare(LoginStatements::REP_ACCOUNT_TOYS);
            stmt.set_u32(0, row.bnet_account_id);
            stmt.set_u32(1, row.item_id);
            stmt.set_bool(2, row.is_favorite);
            stmt.set_bool(3, row.has_fanfare);
            tx.append(stmt);
        }

        if let Err(error) = login_db.commit_transaction(tx).await {
            warn!(
                account = self.account_id,
                bnet_account = self.battlenet_account_id(),
                "Failed to save account toy flags: {error}"
            );
        }
    }

    pub(crate) async fn save_account_heirlooms_like_cpp(&self) {
        let Some(login_db) = self.login_db().map(Arc::clone) else {
            return;
        };
        let save_rows = self.account_heirloom_save_rows_like_cpp();
        if save_rows.is_empty() {
            return;
        }

        let mut tx = SqlTransaction::new();
        for row in save_rows {
            let mut stmt = login_db.prepare(LoginStatements::REP_ACCOUNT_HEIRLOOMS);
            stmt.set_u32(0, row.bnet_account_id);
            stmt.set_u32(1, row.item_id);
            stmt.set_u32(2, row.flags);
            tx.append(stmt);
        }

        if let Err(error) = login_db.commit_transaction(tx).await {
            warn!(
                account = self.account_id,
                bnet_account = self.battlenet_account_id(),
                "Failed to save account heirloom flags: {error}"
            );
        }
    }

    pub(crate) async fn save_account_item_appearances_like_cpp(&mut self) {
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

    pub(crate) async fn save_account_transmog_illusions_like_cpp(&self) {
        let Some(login_db) = self.login_db().map(Arc::clone) else {
            return;
        };
        let plan = self.account_transmog_illusion_save_plan_like_cpp();
        if plan.is_empty() {
            return;
        }

        let bnet_account_id = self.battlenet_account_id();
        let mut tx = SqlTransaction::new();
        for (block_index, illusion_mask) in plan.illusion_blocks {
            let mut stmt = login_db.prepare(LoginStatements::INS_BNET_TRANSMOG_ILLUSIONS);
            stmt.set_u32(0, bnet_account_id);
            stmt.set_u32(1, block_index);
            stmt.set_u32(2, illusion_mask);
            tx.append(stmt);
        }

        if let Err(error) = login_db.commit_transaction(tx).await {
            warn!(
                account = self.account_id,
                bnet_account = bnet_account_id,
                "Failed to save account transmog illusions: {error}"
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
                "All ConnectTo retries exhausted for account {}, aborting login like C++",
                self.account_id
            );
            self.set_player_loading(None);
            self.release_character_login_claim_like_cpp();
            self.set_connect_to_key(None);
            self.set_connect_to_serial(None);
            self.send_packet(&CharacterLoginFailed {
                code: LoginFailureReasonLikeCpp::NoWorld,
            });
        }
    }

    pub(super) fn login_known_spells_after_account_collections_like_cpp(&self) -> Vec<i32> {
        // C++ `Player::HasSpell` includes inactive, non-disabled rows, while
        // `Player::SendKnownSpells` publishes only active rows. Prefer the
        // complete PlayerSpellMap when available so the internal mirror can
        // retain lower ranks without leaking them into the login packet.
        let mut spells = self
            .complete_represented_player_spell_rows_like_cpp()
            .map(|rows| {
                rows.values()
                    .filter(|spell| {
                        spell.state != crate::session::RepresentedPlayerSpellStateLikeCpp::Removed
                            && spell.active
                            && !spell.disabled
                    })
                    .map(|spell| spell.spell_id)
                    .collect()
            })
            .unwrap_or_else(|| self.known_spells_like_cpp().to_vec());
        for mount in self.account_mount_rows_like_cpp() {
            if !spells.contains(&mount.spell_id) {
                spells.push(mount.spell_id);
            }
        }
        spells
    }

    pub(super) async fn load_account_mounts_like_cpp(&mut self) -> bool {
        self.set_account_mounts_like_cpp(Vec::new());
        let Some(login_db) = self.login_db() else {
            return false;
        };

        let bnet_account_id = self.battlenet_account_id();
        if bnet_account_id == 0 {
            warn!(
                account = self.account_id,
                "Skipping account mount load because the game account is not linked to a Battle.net account"
            );
            return false;
        }

        let mut stmt = login_db.prepare(LoginStatements::SEL_ACCOUNT_MOUNTS);
        stmt.set_u32(0, bnet_account_id);

        let mut result = match login_db.query(&stmt).await {
            Ok(result) => result,
            Err(e) => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account mounts: {e}"
                );
                return false;
            }
        };

        if result.is_empty() {
            info!(
                account = self.account_id,
                bnet_account = bnet_account_id,
                "Loaded 0 account mounts from battlenet_account_mounts"
            );
            return true;
        }

        let mut mounts = Vec::new();
        let mut skipped_invalid_spell_id = 0usize;
        let mut skipped_missing_mount_db2 = 0usize;
        loop {
            let spell_id = result.try_read::<i32>(0).unwrap_or(0);
            let flags = result.try_read::<u8>(1).unwrap_or(0);
            if spell_id <= 0 {
                skipped_invalid_spell_id += 1;
                if !result.next_row() {
                    break;
                }
                continue;
            }

            let has_mount = spell_id > 0
                && self.mount_store().is_none_or(|store| {
                    store
                        .get_by_source_spell_id_like_cpp(spell_id as u32)
                        .is_some()
                });
            if has_mount {
                mounts.push(AccountMount { spell_id, flags });
            } else {
                skipped_missing_mount_db2 += 1;
            }

            if !result.next_row() {
                break;
            }
        }

        info!(
            account = self.account_id,
            bnet_account = bnet_account_id,
            loaded = mounts.len(),
            skipped_invalid_spell_id,
            skipped_missing_mount_db2,
            "Loaded represented account mounts like C++ CollectionMgr"
        );
        self.set_account_mounts_like_cpp(mounts.clone());
        true
    }

    pub(super) async fn load_account_toys_like_cpp(&mut self) {
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

    pub(super) async fn load_account_heirlooms_like_cpp(&mut self) {
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

    pub(super) async fn load_account_item_appearances_like_cpp(&mut self) {
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
                        let block_index = result.try_read::<u32>(0).unwrap_or(0);
                        let appearance_mask = result.try_read::<u32>(1).unwrap_or(0);
                        blocks.push((block_index, appearance_mask));
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
                        let item_modified_appearance_id = result.try_read::<u32>(0).unwrap_or(0);
                        favorites.push(item_modified_appearance_id);
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

    pub(super) async fn load_account_transmog_illusions_like_cpp(&mut self) {
        let Some(login_db) = self.login_db() else {
            self.load_represented_account_transmog_illusions_like_cpp([]);
            return;
        };

        let bnet_account_id = self.battlenet_account_id();
        let mut stmt = login_db.prepare(LoginStatements::SEL_BNET_TRANSMOG_ILLUSIONS);
        stmt.set_u32(0, bnet_account_id);
        let illusion_blocks = match login_db.query(&stmt).await {
            Ok(mut result) => {
                let mut blocks = Vec::new();
                if !result.is_empty() {
                    loop {
                        let block_index = result.try_read::<u32>(0).unwrap_or(0);
                        let illusion_mask = result.try_read::<u32>(1).unwrap_or(0);
                        blocks.push((block_index, illusion_mask));
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
                    "Failed to load account transmog illusions: {error}"
                );
                Vec::new()
            }
        };

        self.load_represented_account_transmog_illusions_like_cpp(illusion_blocks);
    }
}
