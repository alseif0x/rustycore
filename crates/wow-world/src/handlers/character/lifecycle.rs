// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character create/delete/rename/customise, corpse and resurrection.

use super::*;

impl WorldSession {
    /// Update the realmcharacters count in the login database.
    ///
    /// Counts how many characters this account has on the character DB, then
    /// upserts the count into `realmcharacters` in the login DB.
    pub(crate) async fn update_realm_characters(&self) {
        let port = match self
            .lifecycle
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
        {
            Some(port) => port,
            None => return,
        };
        let request = wow_persistence::PlayerRealmCharacterCountRefreshRequestLikeCpp {
            account_id: self.core.account_id,
            realm_id: self.realm_id() as u32,
        };
        match port.refresh_realm_character_count_like_cpp(request).await {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {
                debug!(
                    "Updated realmcharacters: account={} realm={}",
                    self.core.account_id,
                    self.realm_id()
                );
            }
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!("Failed to update realmcharacters: {reason}");
            }
        }
    }

    /// Handle CMSG_CREATE_CHARACTER — create a new character.
    pub async fn handle_create_character_with_generator_like_cpp(
        &mut self,
        generator: &wow_core::ObjectGuidGenerator,
        pkt: CreateCharacter,
    ) {
        let port = match self
            .lifecycle
            .character_administration_persistence_port_like_cpp()
        {
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
            port.load_account_character_count_like_cpp(self.core.account_id)
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
            account_id: self.core.account_id,
            name: pkt.name.clone(),
            race: pkt.race,
            class: pkt.class,
            sex,
            rest_state: initial_character_rest_state_like_cpp(
                self.core.is_a_recruiter_like_cpp(),
                self.core.recruiter_id_like_cpp(),
            ),
            map_id,
            position: [x, y, z, o],
            create_time,
            health,
            power1,
            last_login_build: self.core.build,
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
                    self.core.account_id
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
        let port = match self
            .lifecycle
            .character_administration_persistence_port_like_cpp()
        {
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
                self.core.account_id, pkt.guid
            );
            self.send_packet(&DeleteChar {
                code: response_codes::CHAR_DELETE_FAILED,
            });
            return;
        }

        match port
            .delete_owned_character_like_cpp(pkt.guid.counter() as u64, self.core.account_id)
            .await
        {
            wow_persistence::CharacterAdministrationMutationOutcomeLikeCpp::Applied => {
                info!(
                    "Character {:?} deleted for account {}",
                    pkt.guid, self.core.account_id
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
                self.core.account_id, pkt.guid
            );
            self.kick(
                "WorldSession::HandleCharRenameOpcode rename character from a different account",
            );
            return;
        }

        let name_result =
            wow_entities::represented_character_rename_name_result_like_cpp(&pkt.new_name);
        if name_result != RESPONSE_SUCCESS_LIKE_CPP {
            self.send_character_rename_like_cpp(name_result, pkt.guid, pkt.new_name);
            return;
        }

        let port = match self
            .lifecycle
            .character_administration_persistence_port_like_cpp()
        {
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

        if !self
            .lifecycle
            .submit_character_rename_like_cpp(port, pkt.guid, pkt.new_name.clone())
        {
            self.send_character_rename_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, pkt.guid, pkt.new_name);
        }
    }

    /// Apply only on the owning Session, never from a database worker.
    pub(crate) fn enqueue_character_rename_like_cpp(
        &self,
        guid: ObjectGuid,
        outcome: &crate::character_administration::RenameOutcome,
    ) -> flume::r#async::SendFut<'static, Vec<u8>> {
        use crate::character_administration::RenameFailure;
        let result = if outcome.result.is_ok() {
            RESPONSE_SUCCESS_LIKE_CPP
        } else {
            CHAR_CREATE_ERROR_LIKE_CPP
        };
        let delivery = self
            .send_tx()
            .clone()
            .into_send_async(wow_packet::ServerPacket::to_bytes(&CharacterRenameResult {
                result,
                name: outcome.new_name.clone(),
                guid: (result == RESPONSE_SUCCESS_LIKE_CPP).then_some(guid),
            }));
        // Confirmed DB outcome is logged once, not on each poll of a pending send.
        match &outcome.result {
            Ok(old_name) => {
                info!(
                    "Account {} renamed character {:?} from {} to {}",
                    self.core.account_id, guid, old_name, outcome.new_name
                );
            }
            Err(failure) => match failure {
                RenameFailure::QueryFailed(reason) => {
                    warn!("Character rename free-name query failed: {reason}");
                }
                RenameFailure::CommitFailed(reason) => {
                    warn!("Character rename transaction failed: {reason}");
                }
                RenameFailure::NotFound | RenameFailure::NotEligible => {}
            },
        };
        delivery
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
                self.core.account_id, request.guid
            );
            self.kick("WorldSession::HandleCharCustomize Trying to customise character of another account");
            return;
        }

        let port = match self
            .lifecycle
            .character_administration_persistence_port_like_cpp()
        {
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
            self.core.account_id, request.guid, old_name, request.name
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
        {
            let (s, mut h) = crate::session::split_inventory_mut(self);
            s.record_represented_alter_appearance_like_cpp(
                &mut h,
                RepresentedAlterAppearanceLikeCpp {
                    new_sex: request.new_sex,
                    customizations: request.customizations,
                    customized_race: request.customized_race,
                    customized_chr_model_id: request.customized_chr_model_id,
                    cost,
                },
            )
        };
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
        crate::session::hub_mut(self).record_represented_confirm_barbers_choice_like_cpp(
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

    /// CMSG_HEARTH_AND_RESURRECT — battlefield hearth/resurrection escape.
    /// C++ ref: `WorldSession::HandleHearthAndResurrect`.
    pub async fn handle_hearth_and_resurrect(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = HearthAndResurrect::read(&mut pkt) {
            warn!(
                account = self.core.account_id,
                "HearthAndResurrect parse failed: {error}"
            );
            return;
        }

        if crate::session::hub_ref(self).resolved_is_in_taxi_flight_like_cpp() != Some(false) {
            return;
        }

        let Some((_, area_id)) = crate::session::hub_ref(self).player_zone_area_like_cpp() else {
            return;
        };
        let Some(area_table_store) = self.catalogs.area_table_store() else {
            debug!(
                account = self.core.account_id,
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
        crate::session::hub_mut(self).apply_represented_resurrection_percent_like_cpp(1.0);
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
        let Some(port) = self
            .lifecycle
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
        else {
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
        let store = self.catalogs.world_safe_loc_store_like_cpp()?;
        store
            .get(primary_safe_loc_id)
            .or_else(|| neutral_pandaren_safe_loc_id.and_then(|id| store.get(id)))
            .map(|safe_loc| CharacterLoginLocationLikeCpp {
                map_id: safe_loc.map_id,
                bind_area_id: None,
                position: safe_loc.position,
            })
    }

    pub(super) async fn load_map_corpse_data_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
    ) -> MapCorpseLoadOutcomeLikeCpp {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state
            .load_map_corpse_data_like_cpp(hub, map_id, instance_id)
            .await
    }
}
