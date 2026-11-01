// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler family: character-list enumeration (`HandleCharEnumOpcode`).

use std::sync::Arc;

use tracing::{debug, warn};
use wow_constants::character::{
    CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP, CHARACTER_FLAG_LOCKED_FOR_TRANSFER_LIKE_CPP,
};
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_packet::packets::character::{CharacterInfo, EnumCharactersResult, RaceUnlock};
use wow_persistence::{
    CharacterEnumerationLoadOutcomeLikeCpp, CharacterEnumerationPersistencePortLikeCpp,
    CharacterEnumerationRequestLikeCpp,
};
use wow_world_core::session::SupportFeaturePolicyLikeCpp;

use crate::character_enumeration::{
    enum_character_flags_like_cpp, enum_character_pet_data_like_cpp,
};

use crate::character_handlers::{CharacterHandlerCxLikeCpp, parse_equipment_cache};

impl CharacterHandlerCxLikeCpp<'_> {
    /// Handle CMSG_ENUM_CHARACTERS — list characters for this account.
    ///
    /// C++ `HandleCharEnumOpcode` resolves the enumeration port, projects each
    /// `CHAR_SEL_ENUM` row into `EnumCharactersResult::CharacterInfo`, records
    /// the unlocked GUIDs as this session's legitimate characters and answers
    /// with the race-unlock table. The port belongs to the World session's
    /// lifecycle state, so the thunk lends it.
    pub async fn handle_enum_characters_with_policy_like_cpp(
        &mut self,
        port: Option<Arc<dyn CharacterEnumerationPersistencePortLikeCpp>>,
        policy: &SupportFeaturePolicyLikeCpp,
    ) {
        let port = match port {
            Some(port) => port,
            None => {
                warn!(
                    "No character enumeration persistence port for account {}",
                    self.hub.shared().core.account_id
                );
                self.hub.core.send_packet(&EnumCharactersResult {
                    success: false,
                    characters: vec![],
                    race_unlock_data: vec![],
                });
                return;
            }
        };

        let request = CharacterEnumerationRequestLikeCpp {
            account_id: self.hub.shared().core.account_id,
            declined_names_used: policy.declined_names_used,
        };
        let (rows, cleanup_error) = match port.load_character_enumeration_like_cpp(request).await {
            CharacterEnumerationLoadOutcomeLikeCpp::Loaded {
                rows,
                expired_ban_cleanup_error,
            } => (rows, expired_ban_cleanup_error),
            CharacterEnumerationLoadOutcomeLikeCpp::Failed {
                reason,
                expired_ban_cleanup_error,
            } => {
                if let Some(error) = expired_ban_cleanup_error {
                    warn!(
                        "Failed to expire elapsed character bans before enum for account {}: {error}",
                        self.hub.shared().core.account_id
                    );
                }
                warn!(
                    "Failed to query characters for account {}: {reason}",
                    self.hub.shared().core.account_id
                );
                self.hub.core.send_packet(&EnumCharactersResult {
                    success: false,
                    characters: vec![],
                    race_unlock_data: vec![],
                });
                return;
            }
        };
        if let Some(error) = cleanup_error {
            warn!(
                "Failed to expire elapsed character bans before enum for account {}: {error}",
                self.hub.shared().core.account_id
            );
        }

        let mut characters = Vec::new();
        let mut legit_guids = Vec::new();

        for row in rows {
            let realm_id = self.hub.shared().core.realm_id();
            let guid = ObjectGuid::create_player(realm_id, row.guid_low as i64);

            let enum_flags = enum_character_flags_like_cpp(
                row.player_flags,
                row.at_login_flags,
                row.banned_guid,
                (!row.declined_genitive.is_empty()).then_some(row.declined_genitive.as_str()),
                policy.declined_names_used,
            );
            let (pet_display_id, pet_level, pet_family) = enum_character_pet_data_like_cpp(
                row.player_flags,
                row.at_login_flags,
                row.class,
                row.pet_entry,
                row.pet_display_id,
                row.pet_level,
                self.hub
                    .catalogs
                    .creature_template_lifecycle_store_like_cpp()
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
                name: row.name,
                list_position: row.list_slot,
                race_id: row.race,
                class_id: row.class,
                sex_id: row.gender,
                experience_level: row.level,
                zone_id: row.zone,
                map_id: row.map,
                position: Position::new(row.position_x, row.position_y, row.position_z, 0.0),
                guild_guid: if row.guild_id == 0 {
                    ObjectGuid::EMPTY
                } else {
                    ObjectGuid::create_guild(HighGuid::Guild, realm_id, row.guild_id as i64)
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
                equipment: parse_equipment_cache(&row.equipment_cache),
                last_played_time: row.last_played_time,
                spec_id: row.active_talent_group,
                last_login_version: row.last_login_build as i32,
                override_select_screen_file_data_id: 0,
            };

            characters.push(char_info);
        }

        self.hub.core.set_legit_characters(legit_guids);

        debug!(
            "Sending {} characters to account {}",
            characters.len(),
            self.hub.shared().core.account_id
        );

        // Build RaceUnlockData — from race_unlock_requirement table.
        // All WotLK races: expansion 0 (Classic) or 1 (TBC).
        // HasExpansion = true if account expansion >= required expansion.
        let account_exp = self.hub.shared().core.account_expansion;
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

        self.hub.core.send_packet(&EnumCharactersResult {
            success: true,
            characters,
            race_unlock_data,
        });
    }
}
