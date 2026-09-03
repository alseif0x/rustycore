// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character create/delete/rename/customise, corpse and resurrection.

use super::*;

impl WorldSession {
    /// Handle CMSG_CREATE_CHARACTER — create a new character.
    pub async fn handle_create_character_with_generator_like_cpp(
        &mut self,
        generator: &wow_core::ObjectGuidGenerator,
        pkt: CreateCharacter,
    ) {
        let port = match self.character_administration_persistence_port_like_cpp() {
            Some(port) => port,
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

        if matches!(
            port.find_character_name_like_cpp(&pkt.name).await,
            wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::Loaded(())
        ) {
            self.send_packet(&CreateChar {
                code: response_codes::CHAR_CREATE_NAME_IN_USE,
                guid: ObjectGuid::EMPTY,
            });
            return;
        }

        if matches!(
            port.load_account_character_count_like_cpp(self.account_id)
                .await,
            wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::Loaded(count)
                if count >= u64::from(MAX_CHARACTERS_PER_ACCOUNT)
        ) {
            self.send_packet(&CreateChar {
                code: response_codes::CHAR_CREATE_ACCOUNT_LIMIT,
                guid: ObjectGuid::EMPTY,
            });
            return;
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
            account_id: self.account_id,
            name: pkt.name.clone(),
            race: pkt.race,
            class: pkt.class,
            sex,
            rest_state: initial_character_rest_state_like_cpp(
                self.is_a_recruiter_like_cpp(),
                self.recruiter_id_like_cpp(),
            ),
            map_id,
            position: [x, y, z, o],
            create_time,
            health,
            power1,
            last_login_build: self.build,
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
                let guid = ObjectGuid::create_player(self.realm_id(), new_guid_counter);
                info!(
                    "Character '{}' created (guid={}, {} customizations) for account {}",
                    pkt.name,
                    new_guid_counter,
                    pkt.customizations.len(),
                    self.account_id
                );

                // Update realmcharacters count in login DB
                self.update_realm_characters().await;

                self.send_packet(&CreateChar {
                    code: response_codes::CHAR_CREATE_SUCCESS,
                    guid,
                });
            }
            wow_persistence::CharacterAdministrationMutationOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to create character: {reason}");
                self.send_packet(&CreateChar {
                    code: response_codes::CHAR_CREATE_ERROR,
                    guid: ObjectGuid::EMPTY,
                });
            }
        }
    }

    #[cfg(test)]
    pub async fn handle_create_character(&mut self, pkt: CreateCharacter) {
        let generator = self.guid_generator().cloned();
        let Some(generator) = generator else {
            self.send_packet(&CreateChar {
                code: response_codes::CHAR_CREATE_ERROR,
                guid: ObjectGuid::EMPTY,
            });
            return;
        };
        self.handle_create_character_with_generator_like_cpp(generator.as_ref(), pkt)
            .await;
    }

    /// Handle CMSG_CHAR_DELETE — delete a character.
    pub async fn handle_char_delete(&mut self, pkt: CharDelete) {
        let port = match self.character_administration_persistence_port_like_cpp() {
            Some(port) => port,
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

        match port
            .delete_owned_character_like_cpp(pkt.guid.counter() as u64, self.account_id)
            .await
        {
            wow_persistence::CharacterAdministrationMutationOutcomeLikeCpp::Applied => {
                info!(
                    "Character {:?} deleted for account {}",
                    pkt.guid, self.account_id
                );
                self.remove_legit_character(&pkt.guid);

                // Update realmcharacters count in login DB
                self.update_realm_characters().await;

                self.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_SUCCESS,
                });
            }
            wow_persistence::CharacterAdministrationMutationOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to delete character: {reason}");
                self.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_FAILED,
                });
            }
        }
    }

    pub(super) fn represented_character_rename_name_result_like_cpp(name: &str) -> u8 {
        if name.is_empty() {
            return CHAR_NAME_NO_NAME_LIKE_CPP;
        }
        if name.len() < 2 {
            return CHAR_NAME_TOO_SHORT_LIKE_CPP;
        }
        if name.len() > 12 {
            return CHAR_NAME_TOO_LONG_LIKE_CPP;
        }
        if !name.chars().all(|c| c.is_ascii_alphabetic()) {
            return CHAR_NAME_INVALID_CHARACTER_LIKE_CPP;
        }

        RESPONSE_SUCCESS_LIKE_CPP
    }

    fn send_character_rename_like_cpp(
        &self,
        result: u8,
        guid: ObjectGuid,
        new_name: impl Into<String>,
    ) {
        let name = new_name.into();
        self.send_packet(&CharacterRenameResult {
            result,
            name,
            guid: (result == RESPONSE_SUCCESS_LIKE_CPP).then_some(guid),
        });
    }

    /// Handle CMSG_CHARACTER_RENAME_REQUEST.
    pub async fn handle_character_rename_request(&mut self, pkt: CharacterRenameRequest) {
        if !self.is_legit_character(&pkt.guid) {
            warn!(
                "Account {} tried to rename non-owned character {:?}",
                self.account_id, pkt.guid
            );
            self.kick(
                "WorldSession::HandleCharRenameOpcode rename character from a different account",
            );
            return;
        }

        let name_result = Self::represented_character_rename_name_result_like_cpp(&pkt.new_name);
        if name_result != RESPONSE_SUCCESS_LIKE_CPP {
            self.send_character_rename_like_cpp(name_result, pkt.guid, pkt.new_name);
            return;
        }

        let port = match self.character_administration_persistence_port_like_cpp() {
            Some(port) => port,
            None => {
                self.send_character_rename_like_cpp(
                    CHAR_CREATE_ERROR_LIKE_CPP,
                    pkt.guid,
                    pkt.new_name,
                );
                return;
            }
        };

        let candidate = match port
            .load_rename_candidate_like_cpp(pkt.guid.counter() as u64, &pkt.new_name)
            .await
        {
            wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::Loaded(candidate) => {
                candidate
            }
            wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::NotFound => {
                self.send_character_rename_like_cpp(
                    CHAR_CREATE_ERROR_LIKE_CPP,
                    pkt.guid,
                    pkt.new_name,
                );
                return;
            }
            wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Character rename free-name query failed: {reason}");
                self.send_character_rename_like_cpp(
                    CHAR_CREATE_ERROR_LIKE_CPP,
                    pkt.guid,
                    pkt.new_name,
                );
                return;
            }
        };

        let old_name = candidate.old_name;
        let mut at_login_flags = candidate.at_login_flags;
        if (at_login_flags & AT_LOGIN_RENAME_LIKE_CPP) == 0 {
            self.send_character_rename_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, pkt.guid, pkt.new_name);
            return;
        }

        at_login_flags &= !AT_LOGIN_RENAME_LIKE_CPP;

        if let wow_persistence::CharacterAdministrationMutationOutcomeLikeCpp::Failed { reason } =
            port.commit_rename_like_cpp(pkt.guid.counter() as u64, &pkt.new_name, at_login_flags)
                .await
        {
            warn!("Character rename transaction failed: {reason}");
            self.send_character_rename_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, pkt.guid, pkt.new_name);
            return;
        }

        info!(
            "Account {} renamed character {:?} from {} to {}",
            self.account_id, pkt.guid, old_name, pkt.new_name
        );
        self.send_character_rename_like_cpp(RESPONSE_SUCCESS_LIKE_CPP, pkt.guid, pkt.new_name);
    }

    fn send_char_customize_failure_like_cpp(&self, result: u8, guid: ObjectGuid) {
        self.send_packet(&CharCustomizeFailure { result, guid });
    }

    fn send_char_customize_success_like_cpp(&self, request: &CharCustomize) {
        self.send_packet(&CharCustomizeSuccess {
            guid: request.guid,
            sex_id: request.sex_id,
            customizations: request.customizations.clone(),
            name: request.name.clone(),
        });
    }

    /// Handle CMSG_CHAR_CUSTOMIZE.
    pub async fn handle_char_customize(&mut self, request: CharCustomize) {
        if !self.is_legit_character(&request.guid) {
            warn!(
                "Account {} tried to customize non-owned character {:?}",
                self.account_id, request.guid
            );
            self.kick("WorldSession::HandleCharCustomize Trying to customise character of another account");
            return;
        }

        let port = match self.character_administration_persistence_port_like_cpp() {
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

        let name_result = Self::represented_character_rename_name_result_like_cpp(&request.name);
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
            self.account_id, request.guid, old_name, request.name
        );
        self.send_char_customize_success_like_cpp(&request);
    }

    /// Handle CMSG_GET_UNDELETE_CHARACTER_COOLDOWN_STATUS.
    ///
    /// The client sends this when it wants to know if character undelete is
    /// available. We always respond with "no cooldown" (undelete available).
    pub async fn handle_get_undelete_cooldown_status(&mut self) {
        self.send_packet(&wow_packet::packets::misc::UndeleteCooldownStatusResponse::no_cooldown());
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
            self.send_packet(&BarberShopResult {
                result: BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP,
            });
            return;
        }

        let cost = 0;
        self.send_packet(&BarberShopResult {
            result: BARBER_SHOP_RESULT_SUCCESS_LIKE_CPP,
        });
        self.record_represented_alter_appearance_like_cpp(RepresentedAlterAppearanceLikeCpp {
            new_sex: request.new_sex,
            customizations: request.customizations,
            customized_race: request.customized_race,
            customized_chr_model_id: request.customized_chr_model_id,
            cost,
        });
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
        self.record_represented_confirm_barbers_choice_like_cpp(
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

        self.send_packet(&SetPlayerDeclinedNamesResult {
            player: request.player,
            result_code: DECLINED_NAMES_RESULT_ERROR_LIKE_CPP,
        });
    }

    pub async fn handle_query_corpse_location(&mut self, query: QueryCorpseLocationFromClient) {
        // C++ sends an invalid CorpseLocation when the queried player is missing,
        // has no corpse, or is not in the querying player's raid. Rust does not
        // yet have the live corpse/raid lookup needed for the valid branch.
        self.send_packet(&CorpseLocation::not_found_like_cpp(query.player));
    }

    pub async fn handle_query_corpse_transport(&mut self, query: QueryCorpseTransport) {
        // C++ always sends CorpseTransportQuery. Position/facing remain default
        // unless the queried player is in raid and has a corpse on this transport.
        self.send_packet(&CorpseTransportQuery::not_found_like_cpp(query.player));
    }

    /// CMSG_HEARTH_AND_RESURRECT — battlefield hearth/resurrection escape.
    /// C++ ref: `WorldSession::HandleHearthAndResurrect`.
    pub async fn handle_hearth_and_resurrect(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = HearthAndResurrect::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "HearthAndResurrect parse failed: {error}"
            );
            return;
        }

        if self.resolved_is_in_taxi_flight_like_cpp() != Some(false) {
            return;
        }

        let Some((_, area_id)) = self.player_zone_area_like_cpp() else {
            return;
        };
        let Some(area_table_store) = self.area_table_store() else {
            debug!(
                account = self.account_id,
                area_id, "HearthAndResurrect ignored without represented AreaTableStore"
            );
            return;
        };
        let Some(area_entry) = area_table_store.get(area_id) else {
            return;
        };
        if !area_entry.allow_hearth_and_resurrect_from_area_like_cpp() {
            return;
        }

        // C++ first lets Battlefield own the leave flow when one exists. Rust
        // has no battlefield manager attached to WorldSession yet, so this
        // represented branch covers the AreaTable/homebind path only.
        self.apply_represented_resurrection_percent_like_cpp(1.0);
        if let Some(homebind) = self.represented_homebind_like_cpp() {
            self.teleport_to(homebind.map_id, homebind.position).await;
        }
    }

    /// C++ `Player::LoadFromDB`: `CHAR_SEL_CHARACTER_CUSTOMIZATIONS`.
    ///
    /// The rows are copied into `PlayerData::Customizations` before
    /// `PlayerData::WriteCreate`, and each element serializes as
    /// `(ChrCustomizationOptionID, ChrCustomizationChoiceID)` uint32s.
    pub(crate) async fn load_player_customizations_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Vec<ChrCustomizationChoiceValuesUpdate> {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return Vec::new();
        };

        let rows = match port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Customizations {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Customizations(rows),
            ) => rows,
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    player_guid = guid.counter(),
                    "Failed to load character customizations: {reason}"
                );
                return Vec::new();
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    player_guid = guid.counter(),
                    "Player lifecycle port returned the wrong auxiliary login data for customizations"
                );
                return Vec::new();
            }
        };
        let customizations = rows
            .into_iter()
            .map(|row| ChrCustomizationChoiceValuesUpdate {
                option_id: row.option_id,
                choice_id: row.choice_id,
            })
            .collect::<Vec<_>>();

        info!(
            player_guid = guid.counter(),
            customizations = customizations.len(),
            "Loaded character customizations like C++"
        );
        customizations
    }

    pub(super) fn load_default_graveyard_homebind_like_cpp(
        &self,
        race: u8,
    ) -> Option<CharacterLoginLocationLikeCpp> {
        let [primary_safe_loc_id, neutral_pandaren_safe_loc_id] =
            default_graveyard_safe_loc_ids_for_race_like_cpp(race);
        let primary_safe_loc_id = primary_safe_loc_id?;
        let store = self.world_safe_loc_store_like_cpp()?;
        store
            .get(primary_safe_loc_id)
            .or_else(|| neutral_pandaren_safe_loc_id.and_then(|id| store.get(id)))
            .map(|safe_loc| CharacterLoginLocationLikeCpp {
                map_id: safe_loc.map_id,
                bind_area_id: None,
                position: safe_loc.position,
            })
    }

    /// Load the map's persisted corpses once, including the two auxiliary
    /// tables consumed by C++ `Map::LoadCorpseData` before `AddCorpse`.
    pub(super) async fn load_map_corpse_data_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
    ) -> MapCorpseLoadOutcomeLikeCpp {
        let Some(manager) = self.canonical_map_manager.as_ref().map(Arc::clone) else {
            return MapCorpseLoadOutcomeLikeCpp::default();
        };
        {
            let Ok(manager) = manager.lock() else {
                warn!(
                    map_id,
                    instance_id, "Cannot inspect canonical map corpse-load state: lock poisoned"
                );
                return MapCorpseLoadOutcomeLikeCpp::default();
            };
            let Some(map) = manager.find_map(u32::from(map_id), instance_id) else {
                warn!(
                    map_id,
                    instance_id, "Cannot load C++ map corpses: canonical map is unavailable"
                );
                return MapCorpseLoadOutcomeLikeCpp::default();
            };
            if map.map().corpse_data_loaded_like_cpp() {
                return MapCorpseLoadOutcomeLikeCpp {
                    already_loaded: true,
                    ..Default::default()
                };
            }
        }

        let Some(port) = self.map_corpse_persistence_port_like_cpp().map(Arc::clone) else {
            return MapCorpseLoadOutcomeLikeCpp::default();
        };
        let (corpse_rows, phase_rows, customization_rows) = match port
            .load_map_corpses_like_cpp(wow_persistence::MapCorpseLoadRequestLikeCpp {
                map_id: u32::from(map_id),
                instance_id,
            })
            .await
        {
            wow_persistence::MapCorpseLoadOutcomeLikeCpp::Loaded {
                corpses,
                phases,
                customizations,
            } => (corpses, phases, customizations),
            wow_persistence::MapCorpseLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    map_id,
                    instance_id, "C++ Map::LoadCorpseData base query failed: {reason}"
                );
                return MapCorpseLoadOutcomeLikeCpp::default();
            }
        };

        let mut rows = Vec::with_capacity(corpse_rows.len());
        let mut invalid_type_rows = 0u32;
        for row in corpse_rows {
            let corpse_type = match row.corpse_type {
                1 => Some(CorpseType::ResurrectablePve),
                2 => Some(CorpseType::ResurrectablePvp),
                // C++ rejects bones and values >= MAX_CORPSE_TYPE.
                _ => None,
            };
            if let Some(corpse_type) = corpse_type {
                rows.push(LoadedMapCorpseRowLikeCpp {
                    position: Position::new(row.pos_x, row.pos_y, row.pos_z, row.orientation),
                    map_id: row.map_id,
                    display_id: row.display_id,
                    items: parse_corpse_items_like_cpp(&row.item_cache),
                    race: row.race,
                    class: row.class,
                    sex: row.sex,
                    flags: u32::from(row.flags),
                    dynamic_flags: u32::from(row.dynamic_flags),
                    ghost_time: i64::from(row.ghost_time),
                    corpse_type,
                    instance_id: row.instance_id,
                    owner_db_guid: row.owner_guid,
                });
            } else {
                invalid_type_rows = invalid_type_rows.saturating_add(1);
            }
        }

        let mut phases = HashMap::<u64, BTreeSet<u32>>::new();
        let mut customizations = HashMap::<u64, Vec<CorpseCustomizationChoice>>::new();
        match phase_rows {
            wow_persistence::MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(phase_rows) => {
                for row in phase_rows {
                    phases
                        .entry(row.owner_guid)
                        .or_default()
                        .insert(row.phase_id);
                }
            }
            wow_persistence::MapCorpseAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    map_id,
                    instance_id,
                    "C++ Map::LoadCorpseData phase query failed; continuing without phases: {reason}"
                );
            }
        }
        match customization_rows {
            wow_persistence::MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(customization_rows) => {
                for row in customization_rows {
                    customizations.entry(row.owner_guid).or_default().push(
                        CorpseCustomizationChoice {
                            option_id: row.option_id,
                            choice_id: row.choice_id,
                        },
                    );
                }
            }
            wow_persistence::MapCorpseAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    map_id,
                    instance_id,
                    "C++ Map::LoadCorpseData customization query failed; continuing without customizations: {reason}"
                );
            }
        }

        let faction_templates_by_race = rows
            .iter()
            .filter_map(|row| {
                self.faction_template_for_race_like_cpp(row.race)
                    .map(|faction| (row.race, faction))
            })
            .collect::<HashMap<_, _>>();
        let Ok(mut manager) = manager.lock() else {
            warn!(
                map_id,
                instance_id, "Cannot materialize canonical map corpses: lock poisoned"
            );
            return MapCorpseLoadOutcomeLikeCpp {
                invalid_type_rows,
                ..Default::default()
            };
        };
        let Some(map) = manager.find_map_mut(u32::from(map_id), instance_id) else {
            warn!(
                map_id,
                instance_id, "Cannot materialize C++ map corpses: canonical map disappeared"
            );
            return MapCorpseLoadOutcomeLikeCpp {
                invalid_type_rows,
                ..Default::default()
            };
        };
        let mut outcome = materialize_loaded_map_corpses_like_cpp(
            map.map_mut(),
            self.realm_id(),
            rows,
            &phases,
            &customizations,
            &faction_templates_by_race,
        );
        outcome.invalid_type_rows = outcome.invalid_type_rows.saturating_add(invalid_type_rows);
        outcome
    }
}
