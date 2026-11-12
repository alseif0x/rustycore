// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler family: character creation (`HandleCharCreateOpcode`).

use std::sync::Arc;

use tracing::{info, warn};
use wow_core::{ObjectGuid, ObjectGuidGenerator};
use wow_data::PlayerStatSystemProjectionLikeCpp;
use wow_packet::packets::character::{CreateChar, CreateCharacter, response_codes};
use wow_persistence::CharacterAdministrationPersistencePortLikeCpp;
use wow_world_core::session::state::hub_support::RepresentedPlayerGearStatsLikeCpp;

use crate::character_creation::{
    default_character_power1_like_cpp, default_health_mana, max_health_u32_like_cpp, start_position,
};

use crate::character_handlers::{
    CharacterHandlerCxLikeCpp, CreateCharacterStepLikeCpp, MAX_CHARACTERS_PER_ACCOUNT,
    initial_character_rest_state_like_cpp,
};

impl CharacterHandlerCxLikeCpp<'_> {
    /// Handle CMSG_CREATE_CHARACTER — create a new character.
    ///
    /// C++ `HandleCharCreateOpcode` resolves the character-administration port,
    /// gates the requested name and the account character count, mints the new
    /// GUID, projects the level-1 health/mana and rest state, persists the
    /// character, refreshes the login-DB `realmcharacters` count and answers.
    /// The port, the GUID generator and the refresh belong to the World
    /// session's shell state, so the thunk lends them and runs the refresh
    /// where [`CreateCharacterStepLikeCpp`] names it.
    pub async fn handle_create_character_with_generator_like_cpp(
        &mut self,
        port: Option<Arc<dyn CharacterAdministrationPersistencePortLikeCpp>>,
        generator: &ObjectGuidGenerator,
        pkt: CreateCharacter,
    ) -> CreateCharacterStepLikeCpp {
        let port = match port {
            Some(port) => port,
            None => {
                self.hub.core.send_packet(&CreateChar {
                    code: response_codes::CHAR_CREATE_ERROR,
                    guid: ObjectGuid::EMPTY,
                });
                return CreateCharacterStepLikeCpp::Complete;
            }
        };

        // Validate name length
        if pkt.name.len() < 2 || pkt.name.len() > 12 {
            self.hub.core.send_packet(&CreateChar {
                code: response_codes::CHAR_CREATE_ERROR,
                guid: ObjectGuid::EMPTY,
            });
            return CreateCharacterStepLikeCpp::Complete;
        }

        // Validate name characters (alphanumeric only)
        if !pkt.name.chars().all(|c| c.is_ascii_alphabetic()) {
            self.hub.core.send_packet(&CreateChar {
                code: response_codes::CHAR_CREATE_ERROR,
                guid: ObjectGuid::EMPTY,
            });
            return CreateCharacterStepLikeCpp::Complete;
        }

        if matches!(
            port.find_character_name_like_cpp(&pkt.name).await,
            wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::Loaded(())
        ) {
            self.hub.core.send_packet(&CreateChar {
                code: response_codes::CHAR_CREATE_NAME_IN_USE,
                guid: ObjectGuid::EMPTY,
            });
            return CreateCharacterStepLikeCpp::Complete;
        }

        if matches!(
            port.load_account_character_count_like_cpp(self.hub.shared().core.account_id)
                .await,
            wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::Loaded(count)
                if count >= u64::from(MAX_CHARACTERS_PER_ACCOUNT)
        ) {
            self.hub.core.send_packet(&CreateChar {
                code: response_codes::CHAR_CREATE_ACCOUNT_LIMIT,
                guid: ObjectGuid::EMPTY,
            });
            return CreateCharacterStepLikeCpp::Complete;
        }

        // Generate new GUID
        let new_guid_counter = generator.generate();

        // Get start position
        let (map_id, x, y, z, o) = start_position(pkt.race);
        let sex = if pkt.sex < 0 { 0u8 } else { pkt.sex as u8 };

        // C++ `Player::Create` calls `InitStatsForLevel`, then
        // `UpdateMaxHealth`/`SetFullHealth` and `SetFullPower(POWER_MANA)`.
        // At this point create mana is the GtBaseMP value; the intellect
        // bonus is applied later by `UpdateAllStats`.
        let empty_gear = RepresentedPlayerGearStatsLikeCpp::default();
        let (health, mana) = self
            .player_stat_system_projection_like_cpp(pkt.race, pkt.class, 1, &empty_gear)
            .map(|projection| {
                (
                    max_health_u32_like_cpp(projection.max_health),
                    projection.base_mana.max(0) as u32,
                )
            })
            .unwrap_or_else(|| default_health_mana(pkt.class));
        let power1 = default_character_power1_like_cpp(pkt.class, mana);

        let create_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // C++ `Player::Create` equips the new-character loadout
        // (`ObjectMgr::LoadPlayerCreateInfo`: CharacterLoadout Purpose == 9)
        // and the enum reads the resulting visual block back through
        // `characters.equipmentCache`.
        let equipment_cache = self
            .hub
            .catalogs
            .character_creation
            .as_ref()
            .map(|creation| build_starting_equipment_cache_like_cpp(creation, pkt.race, pkt.class))
            .unwrap_or_default();

        let request = wow_persistence::CharacterCreatePersistenceRequestLikeCpp {
            guid: new_guid_counter as u64,
            account_id: self.hub.shared().core.account_id,
            name: pkt.name.clone(),
            race: pkt.race,
            class: pkt.class,
            sex,
            rest_state: initial_character_rest_state_like_cpp(
                self.hub.shared().core.is_a_recruiter_like_cpp(),
                self.hub.shared().core.recruiter_id_like_cpp(),
            ),
            map_id,
            position: [x, y, z, o],
            create_time,
            health,
            power1,
            last_login_build: self.hub.shared().core.build,
            customizations: pkt
                .customizations
                .iter()
                .map(
                    |choice| wow_persistence::CharacterCustomizationPersistenceLikeCpp {
                        option_id: choice.option_id,
                        choice_id: choice.choice_id,
                    },
                )
                .collect(),
            equipment_cache,
        };

        match port.create_character_like_cpp(request).await {
            wow_persistence::CharacterAdministrationMutationOutcomeLikeCpp::Applied => {
                let guid =
                    ObjectGuid::create_player(self.hub.shared().core.realm_id(), new_guid_counter);
                info!(
                    "Character '{}' created (guid={}, {} customizations) for account {}",
                    pkt.name,
                    new_guid_counter,
                    pkt.customizations.len(),
                    self.hub.shared().core.account_id
                );

                // Update realmcharacters count in login DB
                CreateCharacterStepLikeCpp::RefreshRealmCharacters { guid }
            }
            wow_persistence::CharacterAdministrationMutationOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to create character: {reason}");
                self.hub.core.send_packet(&CreateChar {
                    code: response_codes::CHAR_CREATE_ERROR,
                    guid: ObjectGuid::EMPTY,
                });
                CreateCharacterStepLikeCpp::Complete
            }
        }
    }

    /// C++ `HandleCharCreateOpcode` tail: the host refreshed the login-DB
    /// `realmcharacters` count, so the success result may be published.
    pub fn publish_create_character_success_like_cpp(&mut self, guid: ObjectGuid) {
        self.hub.core.send_packet(&CreateChar {
            code: response_codes::CHAR_CREATE_SUCCESS,
            guid,
        });
    }

    /// C++ `Player::Create` -> `InitStatsForLevel`/`UpdateMaxHealth`: the World
    /// session assembles the stat-system access from its catalogs, config and
    /// stat fixtures, which the handler context reaches through its hub.
    fn player_stat_system_projection_like_cpp(
        &mut self,
        race: u8,
        class: u8,
        level: u8,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) -> Option<PlayerStatSystemProjectionLikeCpp> {
        wow_world_inventory::stats_application_cx_from_hub_like_cpp(
            self.hub.reborrow_like_cpp(),
            self.inventory,
        )
        .player_stat_system_projection_like_cpp(race, class, level, gear)
    }
}

/// Equipment-slot resolution mirroring the C++ `InventoryType` auto-equip in
/// `Player::Create` (head/neck/.../robe/tabard fixed, finger/trinket first
/// free, weapons main-hand then off-hand).
fn equip_slot_for_inventory_type_like_cpp(inventory_type: i8, taken: &[bool; 34]) -> Option<u8> {
    let first_free = |candidates: &[u8]| candidates.iter().copied().find(|s| !taken[*s as usize]);
    match inventory_type {
        1 => Some(0),                          // head
        2 => Some(1),                          // neck
        3 => Some(2),                          // shoulders
        4 => Some(3),                          // body (shirt)
        5 | 21 => Some(4),                     // chest / robe
        6 => Some(5),                          // waist
        7 => Some(6),                          // legs
        8 => Some(7),                          // feet
        9 => Some(8),                          // wrist
        10 => Some(9),                         // hands
        11 => first_free(&[10, 11]),           // finger
        12 => first_free(&[12, 13]),           // trinket
        13 | 17 => first_free(&[15, 16]),      // one-hand / two-hand weapon
        14 | 23 | 26 => first_free(&[16, 15]), // shield / held / off-hand
        15 | 25 => Some(17),                   // ranged / thrown
        16 => Some(14),                        // cloak
        20 => Some(18),                        // tabard
        _ => None,
    }
}

/// Build the 34-slot `equipmentCache` string for the select screen from the
/// new-character CharacterLoadout chain; slots left empty write zeros.
fn build_starting_equipment_cache_like_cpp(
    creation: &wow_world_core::session::CharacterCreationCatalogsLikeCpp,
    race: u8,
    class: u8,
) -> String {
    let mut taken = [false; 34];
    // (inv_type, display_id, subclass) per equipment slot
    let mut visual = [(0u32, 0u32, 0u32); 34];

    for loadout in creation.loadouts.new_character_loadouts_for(race, class) {
        for item_id in creation.loadout_items.items_for_loadout(loadout.id) {
            let sparse = creation.item_stats.sparse_template(item_id);
            let inventory_type = sparse.map(|s| s.inventory_type).unwrap_or(0);
            let slot = match equip_slot_for_inventory_type_like_cpp(inventory_type, &taken) {
                Some(slot) => slot,
                None => continue,
            };
            let display_id = creation
                .modified_appearances
                .get_default_for_item(item_id)
                .and_then(|appearance| {
                    creation
                        .appearances
                        .item_display_info_id(appearance.item_appearance_id as u32)
                })
                .unwrap_or(0);
            taken[slot as usize] = true;
            visual[slot as usize] = (
                u32::from(inventory_type.unsigned_abs()),
                display_id,
                0, // subclass: not in the ItemSparse subset; the reader tolerates 0
            );
        }
    }

    let mut cache = String::new();
    for slot in 0..34 {
        if slot > 0 {
            cache.push(' ');
        }
        let (inv_type, display_id, subclass) = visual[slot];
        // invType displayId enchant subclass secondaryModifiedAppearance
        cache.push_str(&format!("{inv_type} {display_id} 0 {subclass} 0"));
    }
    cache
}
