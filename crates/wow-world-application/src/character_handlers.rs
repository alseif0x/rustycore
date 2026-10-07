// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler family.
//!
//! C++ source of truth: `CharacterHandler.cpp` (`HandleSetPlayerDeclinedNames`,
//! `HandleAlterAppearance`, `HandleConfirmBarbersChoice`,
//! `HandleGetUndeleteCooldownStatus`, `HandleCharDeleteOpcode`,
//! `HandleCharRenameOpcode`, `HandleOpeningCinematic`, `HandleCharEnumOpcode`,
//! `HandleCharCreateOpcode`, `HandleCharCustomizeOpcode`). The family owns the
//! packet bodies, the barber-chair gate, the represented barber request records,
//! the opening-cinematic selection, the character-list projection and the
//! create/customize persistence flow; the World session lends the hub, inventory
//! and world-entity state and keeps the shell-only capabilities the host trait
//! exposes (#1263 F5). Bodies are moved unchanged from the World shell. Login
//! stays in the World shell while it needs its realm and connection
//! orchestration.

use std::sync::Arc;

use tracing::{debug, info, warn};
use wow_constants::ClientOpcodes;
use wow_constants::character::{
    AT_LOGIN_CUSTOMIZE_LIKE_CPP, CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP,
    CHARACTER_FLAG_LOCKED_FOR_TRANSFER_LIKE_CPP, RESPONSE_SUCCESS_LIKE_CPP,
};
use wow_constants::rest::{REST_STATE_NORMAL_LIKE_CPP, REST_STATE_RAF_LINKED_LIKE_CPP};
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, ObjectGuidGenerator, Position};
use wow_data::PlayerStatSystemProjectionLikeCpp;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::character::{
    AlterAppearance, BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP, BARBER_SHOP_RESULT_SUCCESS_LIKE_CPP,
    BarberShopResult, CharCustomize, CharCustomizeFailure, CharCustomizeSuccess, CharDelete,
    CharacterInfo, CharacterRenameRequest, CharacterRenameResult, ConfirmBarbersChoice, CreateChar,
    CreateCharacter, DECLINED_NAMES_RESULT_ERROR_LIKE_CPP, DeleteChar, EnumCharactersResult,
    RaceUnlock, SetPlayerDeclinedNames, SetPlayerDeclinedNamesResult, VisualItemInfo,
    response_codes,
};
use wow_packet::packets::misc::TriggerCinematic;
use wow_packet::{ClientPacket, WorldPacket};
use wow_persistence::{
    CharacterAdministrationPersistencePortLikeCpp, CharacterEnumerationLoadOutcomeLikeCpp,
    CharacterEnumerationPersistencePortLikeCpp, CharacterEnumerationRequestLikeCpp,
};
use wow_world_core::session::state::hub_support::RepresentedPlayerGearStatsLikeCpp;
use wow_world_core::session::{
    HubMut, PacketPublicationAccessLikeCpp, RepresentedAlterAppearanceLikeCpp,
    RepresentedConfirmBarbersChoiceLikeCpp, SupportFeaturePolicyLikeCpp,
};
use wow_world_entities::{RepresentedGameObjectUseEffect, WorldEntitiesState};
use wow_world_inventory::InventoryState;

use crate::character_creation::{
    default_character_power1_like_cpp, default_health_mana, max_health_u32_like_cpp, start_position,
};
use crate::character_enumeration::{
    enum_character_flags_like_cpp, enum_character_pet_data_like_cpp,
};

/// C++ `SharedDefines.h` `CHAR_CREATE_ERROR`, the failure result
/// `HandleCharRenameOpcode` publishes for a missing port or a refused submit.
const CHAR_CREATE_ERROR_LIKE_CPP: u8 = 25;

/// C++ `SharedDefines.h` `CHAR_CREATE_NAME_IN_USE`, the failure result
/// `HandleCharCustomizeOpcode` publishes for a name another character holds.
const CHAR_CREATE_NAME_IN_USE_LIKE_CPP: u8 = 27;

/// Maximum characters per account.
const MAX_CHARACTERS_PER_ACCOUNT: u32 = 10;

/// C++ `Player::Create` seeds the represented RAF rest state from the account's
/// recruiter role.
pub fn initial_character_rest_state_like_cpp(is_a_recruiter: bool, recruiter_id: u32) -> u8 {
    if is_a_recruiter || recruiter_id != 0 {
        REST_STATE_RAF_LINKED_LIKE_CPP
    } else {
        REST_STATE_NORMAL_LIKE_CPP
    }
}

/// Parse a space-separated equipment cache string into VisualItemInfo array.
///
/// C++ `EnumCharactersResult::CharacterInfo` parses `equipmentCache` as five
/// fields per slot: InvType, DisplayID, DisplayEnchantID, Subclass, and
/// SecondaryItemModifiedAppearanceID.
pub fn parse_equipment_cache(cache: &str) -> [VisualItemInfo; 34] {
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

/// Host step that finishes one C++ `HandleCharDeleteOpcode` call.
///
/// The character-administration port and the login-DB `realmcharacters` refresh
/// live on the World session's lifecycle state, which the handler context does
/// not reach, so the thunk lends the port and runs the refresh where this step
/// says the C++ body runs it.
pub enum CharDeleteStepLikeCpp {
    /// Every packet of the C++ body is already published.
    Complete,
    /// C++ order: the host refreshes `realmcharacters`, then the owner
    /// publishes `CHAR_DELETE_SUCCESS`.
    RefreshRealmCharacters,
}

/// Host step that finishes one C++ `HandleCharRenameOpcode` call.
///
/// The rename read is submitted on the World session's rename callback rail,
/// which the handler context does not reach.
pub enum CharRenameStepLikeCpp {
    /// Every packet of the C++ body is already published.
    Complete,
    /// C++ order: the host submits the read; when it is refused, the owner
    /// publishes the `CHAR_CREATE_ERROR` result.
    SubmitCharacterRename {
        port: Arc<dyn CharacterAdministrationPersistencePortLikeCpp>,
        guid: ObjectGuid,
        new_name: String,
    },
}

/// Host step that finishes one C++ `HandleCharCreateOpcode` call.
///
/// The login-DB `realmcharacters` refresh runs on the World session's
/// lifecycle state, which the handler context does not reach, so the thunk
/// runs it where this step says the C++ body runs it.
pub enum CreateCharacterStepLikeCpp {
    /// Every packet of the C++ body is already published.
    Complete,
    /// C++ order: the host refreshes `realmcharacters`, then the owner
    /// publishes `CHAR_CREATE_SUCCESS` for the created character.
    RefreshRealmCharacters { guid: ObjectGuid },
}

/// Borrowed inputs of one character handler invocation.
pub struct CharacterHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    inventory: &'a mut InventoryState,
    world_entities: &'a WorldEntitiesState,
}

impl<'a> CharacterHandlerCxLikeCpp<'a> {
    pub fn new(
        hub: HubMut<'a>,
        inventory: &'a mut InventoryState,
        world_entities: &'a WorldEntitiesState,
    ) -> Self {
        Self {
            hub,
            inventory,
            world_entities,
        }
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    /// C++ `Player::IsSittingOnBarberChair` over the represented chair-use
    /// effects: the latest chair use by this Player at its current stand state.
    fn represented_is_on_barber_chair_like_cpp(&self) -> bool {
        let Some(player_guid) = self.hub.shared().core.player_guid() else {
            return false;
        };
        // `UnitStandStateType` is a fieldless `#[repr(u8)]` enum, so the
        // discriminant cast equals the former `ToPrimitive::to_u32` result.
        let Some(current_stand_state) = self
            .hub
            .shared()
            .resolved_player_stand_state_like_cpp()
            .map(|state| state as u32)
        else {
            return false;
        };

        self.world_entities
            .represented_gameobject_use_effects_since_like_cpp(0)
            .iter()
            .rev()
            .any(|effect| {
                matches!(
                    effect,
                    RepresentedGameObjectUseEffect::BarberChairUsed {
                        player_guid: effect_player_guid,
                        stand_state,
                        ..
                    } if *effect_player_guid == player_guid && *stand_state == current_stand_state
                )
            })
    }

    /// Handle CMSG_GET_UNDELETE_CHARACTER_COOLDOWN_STATUS.
    ///
    /// The client sends this when it wants to know if character undelete is
    /// available. We always respond with "no cooldown" (undelete available).
    pub async fn handle_get_undelete_cooldown_status(&mut self) {
        self.publication_like_cpp()
            .send_packet(&wow_packet::packets::misc::UndeleteCooldownStatusResponse::no_cooldown());
    }

    /// Handle CMSG_ALTER_APPEARANCE.
    ///
    /// C++ `HandleAlterAppearance` validates customization DB2 requirements,
    /// requires the player to be sitting on a nearby barber chair, checks
    /// `GetBarberShopCost`, sends `SMSG_BARBER_SHOP_RESULT`, then mutates
    /// player gender/customizations and criteria.
    ///
    /// Rust currently represents barber-chair use and stand-state, but does
    /// not yet own the full ChrCustomization/BarberShop cost/runtime mutation.
    /// This seam preserves packet/dispatch, the C++ not-on-chair result, and
    /// records accepted requests without fabricating the full appearance change.
    pub async fn handle_alter_appearance(&mut self, mut pkt: WorldPacket) {
        let request = match AlterAppearance::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad AlterAppearance: {error}");
                return;
            }
        };

        if !self.represented_is_on_barber_chair_like_cpp() {
            self.publication_like_cpp().send_packet(&BarberShopResult {
                result: BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP,
            });
            return;
        }

        let cost = 0;
        self.publication_like_cpp().send_packet(&BarberShopResult {
            result: BARBER_SHOP_RESULT_SUCCESS_LIKE_CPP,
        });
        self.inventory.record_represented_alter_appearance_like_cpp(
            &mut self.hub,
            RepresentedAlterAppearanceLikeCpp {
                new_sex: request.new_sex,
                customizations: request.customizations,
                customized_race: request.customized_race,
                customized_chr_model_id: request.customized_chr_model_id,
                cost,
            },
        );
    }

    /// Handle CMSG_CONFIRM_BARBERS_CHOICE.
    ///
    /// C++ `HandleConfirmBarbersChoice` converts the barber rows into
    /// `ChrCustomizationChoice`, checks `GetBarberShopCost`, sends only the
    /// no-money failure, and otherwise mutates money/customizations/criteria
    /// without a success packet. Rust records the accepted request until the
    /// Player customization/cost/criteria runtime is canonical.
    pub async fn handle_confirm_barbers_choice(&mut self, mut pkt: WorldPacket) {
        let request = match ConfirmBarbersChoice::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad ConfirmBarbersChoice: {error}");
                return;
            }
        };

        let cost = 0;
        self.hub.record_represented_confirm_barbers_choice_like_cpp(
            RepresentedConfirmBarbersChoiceLikeCpp {
                customizations: request.customizations,
                cost,
            },
        );
    }

    /// Handle CMSG_SET_PLAYER_DECLINED_NAMES.
    ///
    /// C++ resolves the target character through `sCharacterCache`, requires a
    /// Cyrillic base name, normalizes all five declined forms, validates them
    /// with `ObjectMgr::CheckDeclinedNames`, then replaces the
    /// `character_declinedname` row and returns success. Rust does not yet
    /// carry that character-cache / locale-validation runtime through this
    /// session path, so this bounded seam preserves the parse/dispatch and the
    /// C++ error-result branch instead of fabricating persisted declined names.
    pub async fn handle_set_player_declined_names(&mut self, mut pkt: WorldPacket) {
        let request = match SetPlayerDeclinedNames::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad SetPlayerDeclinedNames: {error}");
                return;
            }
        };

        self.publication_like_cpp()
            .send_packet(&SetPlayerDeclinedNamesResult {
                player: request.player,
                result_code: DECLINED_NAMES_RESULT_ERROR_LIKE_CPP,
            });
    }

    /// Handle CMSG_CHAR_DELETE — delete a character.
    ///
    /// C++ `HandleCharDeleteOpcode` resolves the character-administration port,
    /// refuses a character the account does not own, deletes the row, refreshes
    /// the login-DB `realmcharacters` count and answers. The port and the
    /// refresh belong to the World session's lifecycle state, so the thunk
    /// lends the port and runs the refresh where [`CharDeleteStepLikeCpp`]
    /// names it.
    pub async fn handle_char_delete(
        &mut self,
        port: Option<Arc<dyn CharacterAdministrationPersistencePortLikeCpp>>,
        pkt: CharDelete,
    ) -> CharDeleteStepLikeCpp {
        let port = match port {
            Some(port) => port,
            None => {
                self.hub.core.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_FAILED,
                });
                return CharDeleteStepLikeCpp::Complete;
            }
        };

        // Verify the character belongs to this account
        if !self.hub.shared().core.is_legit_character(&pkt.guid) {
            warn!(
                "Account {} tried to delete non-owned character {:?}",
                self.hub.shared().core.account_id,
                pkt.guid
            );
            self.hub.core.send_packet(&DeleteChar {
                code: response_codes::CHAR_DELETE_FAILED,
            });
            return CharDeleteStepLikeCpp::Complete;
        }

        let account_id = self.hub.shared().core.account_id;
        match port
            .delete_owned_character_like_cpp(pkt.guid.counter() as u64, account_id)
            .await
        {
            wow_persistence::CharacterAdministrationMutationOutcomeLikeCpp::Applied => {
                info!(
                    "Character {:?} deleted for account {}",
                    pkt.guid, account_id
                );
                self.hub.remove_legit_character(&pkt.guid);

                // Update realmcharacters count in login DB
                CharDeleteStepLikeCpp::RefreshRealmCharacters
            }
            wow_persistence::CharacterAdministrationMutationOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to delete character: {reason}");
                self.hub.core.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_FAILED,
                });
                CharDeleteStepLikeCpp::Complete
            }
        }
    }

    /// C++ `HandleCharDeleteOpcode` tail: the host refreshed the login-DB
    /// `realmcharacters` count, so the success result may be published.
    pub fn publish_char_delete_success_like_cpp(&mut self) {
        self.hub.core.send_packet(&DeleteChar {
            code: response_codes::CHAR_DELETE_SUCCESS,
        });
    }

    fn send_character_rename_like_cpp(
        &self,
        result: u8,
        guid: ObjectGuid,
        new_name: impl Into<String>,
    ) {
        let name = new_name.into();
        self.hub.core.send_packet(&CharacterRenameResult {
            result,
            name,
            guid: (result == RESPONSE_SUCCESS_LIKE_CPP).then_some(guid),
        });
    }

    /// Handle CMSG_CHARACTER_RENAME_REQUEST.
    ///
    /// C++ `HandleCharRenameOpcode` refuses a character the account does not
    /// own, validates the new name, resolves the character-administration port
    /// and submits the read for a commit admitted only in its ready callback.
    /// The port and the callback rail belong to the World session's lifecycle
    /// state, so the thunk lends the port and submits where
    /// [`CharRenameStepLikeCpp`] names it.
    pub async fn handle_character_rename_request(
        &mut self,
        port: Option<Arc<dyn CharacterAdministrationPersistencePortLikeCpp>>,
        pkt: CharacterRenameRequest,
    ) -> CharRenameStepLikeCpp {
        if !self.hub.shared().core.is_legit_character(&pkt.guid) {
            warn!(
                "Account {} tried to rename non-owned character {:?}",
                self.hub.shared().core.account_id,
                pkt.guid
            );
            self.hub.core.kick(
                "WorldSession::HandleCharRenameOpcode rename character from a different account",
            );
            return CharRenameStepLikeCpp::Complete;
        }

        let name_result =
            wow_entities::represented_character_rename_name_result_like_cpp(&pkt.new_name);
        if name_result != RESPONSE_SUCCESS_LIKE_CPP {
            self.send_character_rename_like_cpp(name_result, pkt.guid, pkt.new_name);
            return CharRenameStepLikeCpp::Complete;
        }

        let Some(port) = port else {
            self.send_character_rename_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, pkt.guid, pkt.new_name);
            return CharRenameStepLikeCpp::Complete;
        };

        CharRenameStepLikeCpp::SubmitCharacterRename {
            port,
            guid: pkt.guid,
            new_name: pkt.new_name,
        }
    }

    /// C++ `HandleCharRenameOpcode` tail: the session's rename callback rail
    /// refused the read, so the `CHAR_CREATE_ERROR` result is published.
    pub fn publish_character_rename_refusal_like_cpp(
        &mut self,
        guid: ObjectGuid,
        new_name: String,
    ) {
        self.send_character_rename_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, guid, new_name);
    }

    /// Handle CMSG_OPENING_CINEMATIC.
    pub async fn handle_opening_cinematic(&mut self, _pkt: WorldPacket) {
        let _ = self.opening_cinematic_like_cpp();
    }

    /// C++ `HandleOpeningCinematic`: a character that has never gained
    /// experience plays its class cinematic, or its race cinematic when the
    /// class has none.
    fn opening_cinematic_like_cpp(&mut self) -> Option<u32> {
        if self.hub.shared().resolved_player_xp_like_cpp()? != 0 {
            return None;
        }

        let class_store = self.hub.catalogs.chr.classes_store.as_ref()?;
        let class_entry = class_store.get(u32::from(self.hub.shared().player_class_like_cpp()))?;
        let cinematic_id = if class_entry.cinematic_sequence_id != 0 {
            u32::from(class_entry.cinematic_sequence_id)
        } else {
            let race_store = self.hub.catalogs.chr.races_store.as_ref()?;
            race_store
                .get(u32::from(self.hub.shared().player_race_like_cpp()))
                .map(|race_entry| race_entry.cinematic_sequence_id as u32)?
        };

        send_represented_cinematic_start_like_cpp(&mut self.hub, cinematic_id);
        Some(cinematic_id)
    }

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

    fn send_char_customize_failure_like_cpp(&self, result: u8, guid: ObjectGuid) {
        self.hub
            .core
            .send_packet(&CharCustomizeFailure { result, guid });
    }

    fn send_char_customize_success_like_cpp(&self, request: &CharCustomize) {
        self.hub.core.send_packet(&CharCustomizeSuccess {
            guid: request.guid,
            sex_id: request.sex_id,
            customizations: request.customizations.clone(),
            name: request.name.clone(),
        });
    }

    /// Handle CMSG_CHAR_CUSTOMIZE.
    ///
    /// C++ `HandleCharCustomizeOpcode` refuses a character the account does not
    /// own, loads the at-login candidate, requires `AT_LOGIN_CUSTOMIZE`,
    /// validates the new name, clears the flag and commits the customization.
    /// The character-administration port belongs to the World session's
    /// lifecycle state, so the thunk lends it.
    pub async fn handle_char_customize(
        &mut self,
        port: Option<Arc<dyn CharacterAdministrationPersistencePortLikeCpp>>,
        request: CharCustomize,
    ) {
        if !self.hub.shared().core.is_legit_character(&request.guid) {
            warn!(
                "Account {} tried to customize non-owned character {:?}",
                self.hub.shared().core.account_id,
                request.guid
            );
            self.hub.core.kick(
                "WorldSession::HandleCharCustomize Trying to customise character of another account",
            );
            return;
        }

        let port = match port {
            Some(port) => port,
            None => {
                self.send_char_customize_failure_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, request.guid);
                return;
            }
        };

        let candidate = match port
            .load_customize_candidate_like_cpp(request.guid.counter() as u64)
            .await
        {
            wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::Loaded(candidate) => {
                candidate
            }
            wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::NotFound => {
                self.send_char_customize_failure_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, request.guid);
                return;
            }
            wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Character customize info query failed: {reason}");
                self.send_char_customize_failure_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, request.guid);
                return;
            }
        };

        let old_name = candidate.old_name;
        let mut at_login_flags = candidate.at_login_flags;
        if (at_login_flags & AT_LOGIN_CUSTOMIZE_LIKE_CPP) == 0 {
            self.send_char_customize_failure_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, request.guid);
            return;
        }

        let name_result =
            wow_entities::represented_character_rename_name_result_like_cpp(&request.name);
        if name_result != RESPONSE_SUCCESS_LIKE_CPP {
            self.send_char_customize_failure_like_cpp(name_result, request.guid);
            return;
        }

        if request.name != old_name {
            match port.find_character_name_like_cpp(&request.name).await {
                wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::Loaded(()) => {
                    self.send_char_customize_failure_like_cpp(
                        CHAR_CREATE_NAME_IN_USE_LIKE_CPP,
                        request.guid,
                    );
                    return;
                }
                wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!("Character customize name query failed: {reason}");
                    self.send_char_customize_failure_like_cpp(
                        CHAR_CREATE_ERROR_LIKE_CPP,
                        request.guid,
                    );
                    return;
                }
                wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::NotFound => {}
            }
        }

        at_login_flags &= !AT_LOGIN_CUSTOMIZE_LIKE_CPP;

        let customizations = request
            .customizations
            .iter()
            .map(
                |choice| wow_persistence::CharacterCustomizationPersistenceLikeCpp {
                    option_id: choice.option_id,
                    choice_id: choice.choice_id,
                },
            )
            .collect();
        if let wow_persistence::CharacterAdministrationMutationOutcomeLikeCpp::Failed { reason } =
            port.commit_customize_like_cpp(
                request.guid.counter() as u64,
                &request.name,
                at_login_flags,
                customizations,
            )
            .await
        {
            warn!("Character customize transaction failed: {reason}");
            self.send_char_customize_failure_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, request.guid);
            return;
        }

        info!(
            "Account {} customized character {:?} from {} to {}",
            self.hub.shared().core.account_id,
            request.guid,
            old_name,
            request.name
        );
        self.send_char_customize_success_like_cpp(&request);
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
        crate::stats::stats_application_cx_from_hub_like_cpp(
            self.hub.reborrow_like_cpp(),
            self.inventory,
        )
        .player_stat_system_projection_like_cpp(race, class, level, gear)
    }
}

/// C++ `WorldSession::SendCinematicStart` over the represented cinematic state:
/// publish `SMSG_TRIGGER_CINEMATIC` and begin the sequence when it is known.
///
/// The application owner keeps the body (#1263 F5); the World session delegates
/// its remaining gameobject-use caller to this helper.
pub fn send_represented_cinematic_start_like_cpp(hub: &mut HubMut<'_>, cinematic_id: u32) {
    if hub
        .shared()
        .player_cinematic_state_snapshot_like_cpp()
        .is_none()
    {
        return;
    }
    hub.core.send_packet(&TriggerCinematic {
        cinematic_id,
        conversation_guid: ObjectGuid::EMPTY,
    });
    if let Some(sequence) = hub
        .catalogs
        .cinematic_sequences_store
        .as_ref()
        .and_then(|store| store.get(cinematic_id))
    {
        let camera_ids = sequence.camera;
        let _ = hub.with_player_cinematic_state_like_cpp(|state| {
            state.begin_cinematic_like_cpp(cinematic_id, camera_ids);
        });
    }
}

/// Builds a character handler context from a host's state.
pub trait CharacterHandlerHostLikeCpp<C> {
    fn character_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> CharacterHandlerCxLikeCpp<'a>;

    /// C++ `HandleCharDeleteOpcode`/`HandleCharRenameOpcode` resolve the
    /// character-administration port from the session's lifecycle state, which
    /// the handler context cannot reach.
    fn character_administration_persistence_port_like_cpp(
        &mut self,
    ) -> Option<Arc<dyn CharacterAdministrationPersistencePortLikeCpp>>;

    /// C++ `HandleCharDeleteOpcode` refreshes the login-DB `realmcharacters`
    /// count between the delete commit and `CHAR_DELETE_SUCCESS`; the refresh
    /// needs the session's player-lifecycle port.
    fn update_realm_characters_like_cpp(&mut self) -> HandlerFuture<'_, ()>;

    /// C++ `HandleCharRenameOpcode` submits the read on the session's rename
    /// callback rail, which admits the commit only in its ready callback.
    fn submit_character_rename_like_cpp(
        &mut self,
        port: Arc<dyn CharacterAdministrationPersistencePortLikeCpp>,
        guid: ObjectGuid,
        name: String,
    ) -> bool;

    /// C++ `HandleCharEnumOpcode` resolves the character-enumeration port from
    /// the session's lifecycle state, which the handler context cannot reach.
    fn character_enumeration_persistence_port_like_cpp(
        &mut self,
    ) -> Option<Arc<dyn CharacterEnumerationPersistencePortLikeCpp>>;

    /// C++ `HandleCharEnumOpcode` reads `sWorld->getBoolConfig(
    /// CONFIG_DECLINED_NAMES_USED)`; the support-feature policy lives on the
    /// shell catalogs, which the generic thunk cannot name.
    fn support_feature_policy_like_cpp(catalogs: &C) -> &SupportFeaturePolicyLikeCpp;

    /// C++ `sObjectMgr->GenerateCharacterGuid()`: the player GUID generator
    /// lives on the shell catalogs, which the generic thunk cannot name.
    fn character_guid_generator_like_cpp(catalogs: &C) -> &ObjectGuidGenerator;
}

fn handle_get_undelete_cooldown_status_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    _pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .character_handler_cx_like_cpp(catalogs)
            .handle_get_undelete_cooldown_status()
            .await;
    })
}

fn handle_alter_appearance_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .character_handler_cx_like_cpp(catalogs)
            .handle_alter_appearance(pkt)
            .await;
    })
}

fn handle_confirm_barbers_choice_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .character_handler_cx_like_cpp(catalogs)
            .handle_confirm_barbers_choice(pkt)
            .await;
    })
}

fn handle_set_player_declined_names_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .character_handler_cx_like_cpp(catalogs)
            .handle_set_player_declined_names(pkt)
            .await;
    })
}

fn handle_char_delete_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CharDelete::read(&mut pkt) {
            Ok(del) => {
                let port = session.character_administration_persistence_port_like_cpp();
                let step = session
                    .character_handler_cx_like_cpp(catalogs)
                    .handle_char_delete(port, del)
                    .await;
                if matches!(step, CharDeleteStepLikeCpp::RefreshRealmCharacters) {
                    session.update_realm_characters_like_cpp().await;
                    session
                        .character_handler_cx_like_cpp(catalogs)
                        .publish_char_delete_success_like_cpp();
                }
            }
            Err(e) => tracing::warn!("Failed to read CharDelete: {e}"),
        }
    })
}

fn handle_character_rename_request_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CharacterRenameRequest::read(&mut pkt) {
            Ok(rename) => {
                let port = session.character_administration_persistence_port_like_cpp();
                let step = session
                    .character_handler_cx_like_cpp(catalogs)
                    .handle_character_rename_request(port, rename)
                    .await;
                if let CharRenameStepLikeCpp::SubmitCharacterRename {
                    port,
                    guid,
                    new_name,
                } = step
                {
                    if !session.submit_character_rename_like_cpp(port, guid, new_name.clone()) {
                        session
                            .character_handler_cx_like_cpp(catalogs)
                            .publish_character_rename_refusal_like_cpp(guid, new_name);
                    }
                }
            }
            Err(e) => tracing::warn!("Failed to read CharacterRenameRequest: {e}"),
        }
    })
}

fn handle_opening_cinematic_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .character_handler_cx_like_cpp(catalogs)
            .handle_opening_cinematic(pkt)
            .await;
    })
}

fn handle_enum_characters_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    _pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let port = session.character_enumeration_persistence_port_like_cpp();
        session
            .character_handler_cx_like_cpp(catalogs)
            .handle_enum_characters_with_policy_like_cpp(
                port,
                S::support_feature_policy_like_cpp(catalogs),
            )
            .await;
    })
}

fn handle_create_character_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CreateCharacter::read(&mut pkt) {
            Ok(create) => {
                let port = session.character_administration_persistence_port_like_cpp();
                let generator = S::character_guid_generator_like_cpp(catalogs);
                let step = session
                    .character_handler_cx_like_cpp(catalogs)
                    .handle_create_character_with_generator_like_cpp(port, generator, create)
                    .await;
                if let CreateCharacterStepLikeCpp::RefreshRealmCharacters { guid } = step {
                    session.update_realm_characters_like_cpp().await;
                    session
                        .character_handler_cx_like_cpp(catalogs)
                        .publish_create_character_success_like_cpp(guid);
                }
            }
            Err(e) => tracing::warn!("Failed to read CreateCharacter: {e}"),
        }
    })
}

fn handle_char_customize_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CharCustomize::read(&mut pkt) {
            Ok(customize) => {
                let port = session.character_administration_persistence_port_like_cpp();
                session
                    .character_handler_cx_like_cpp(catalogs)
                    .handle_char_customize(port, customize)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read CharCustomize: {e}"),
        }
    })
}

/// Registers the character handlers on the packet registry.
pub fn register_character_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: CharacterHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GetUndeleteCharacterCooldownStatus,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_get_undelete_cooldown_status",
        handler: handle_get_undelete_cooldown_status_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AlterAppearance,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_alter_appearance",
        handler: handle_alter_appearance_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ConfirmBarbersChoice,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_confirm_barbers_choice",
        handler: handle_confirm_barbers_choice_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetPlayerDeclinedNames,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_player_declined_names",
        handler: handle_set_player_declined_names_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CharDelete,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_char_delete",
        handler: handle_char_delete_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CharacterRenameRequest,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_character_rename_request",
        handler: handle_character_rename_request_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::OpeningCinematic,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_opening_cinematic",
        handler: handle_opening_cinematic_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::EnumCharacters,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_enum_characters",
        handler: handle_enum_characters_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CreateCharacter,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_create_character",
        handler: handle_create_character_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CharCustomize,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_char_customize",
        handler: handle_char_customize_thunk::<S, C>,
    })?;
    Ok(())
}
