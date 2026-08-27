// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Login, world entry, logout and the client-state handshake.

use super::*;

impl WorldSession {
    /// Handle CMSG_PLAYER_LOGIN — initiate ConnectTo flow.
    ///
    /// Instead of sending the login sequence directly, we send SMSG_CONNECT_TO
    /// to redirect the client to the instance port. The login sequence is sent
    /// after the client reconnects via `handle_continue_player_login`.
    pub async fn handle_player_login(&mut self, pkt: PlayerLogin) {
        if self.player_loading().is_some() || self.player_guid().is_some() {
            warn!(
                account = self.account_id,
                "Player tried to login while another character is loading or active"
            );
            self.kick("WorldSession::HandlePlayerLoginOpcode Another client logging in");
            return;
        }

        // Verify character ownership
        if !self.is_legit_character(&pkt.guid) {
            warn!(
                "Account {} tried to login with non-owned character {:?}",
                self.account_id, pkt.guid
            );
            return;
        }

        // C++ exposes one live `Player*` per character GUID through
        // ObjectAccessor. Claim that ownership before ConnectTo/DB loading so
        // two sessions cannot become independent save authorities.
        if !self.try_claim_character_login_like_cpp(pkt.guid) {
            warn!(
                account = self.account_id,
                guid = ?pkt.guid,
                "Rejecting duplicate live-character login"
            );
            self.send_packet(&CharacterLoginFailed {
                code: LoginFailureReasonLikeCpp::DuplicateCharacter,
            });
            return;
        }

        // Store the loading character GUID
        self.set_player_loading(Some(pkt.guid));

        // Build ConnectTo and register with SessionManager
        self.send_connect_to(ConnectToSerial::WorldAttempt1);
    }

    pub async fn handle_opening_cinematic(&mut self, _pkt: WorldPacket) {
        let _ = self.opening_cinematic_like_cpp();
    }

    /// Handle CMSG_SERVER_TIME_OFFSET_REQUEST — respond with current realm time.
    pub async fn handle_server_time_offset_request(&mut self) {
        self.send_packet(&ServerTimeOffset::now());
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

        if let Some(player_guid) = self.player_guid() {
            self.wait_for_active_loot_persistence_like_cpp().await;
            self.do_loot_release_all_like_cpp(player_guid).await;
        }

        // Trinity clears buyback slots before SaveToDB; persisted buyback items must not survive logout.
        self.clear_buyback_on_logout().await;
        self.save_current_player_to_db_like_cpp().await;
        self.save_account_mounts_like_cpp().await;
        self.save_account_toys_like_cpp().await;
        self.save_account_heirlooms_like_cpp().await;
        self.save_account_item_appearances_like_cpp().await;
        self.save_account_transmog_illusions_like_cpp().await;

        // Mark character offline in DB
        self.mark_character_offline().await;

        // Queue the full visibility diff while the old canonical snapshot can
        // still identify its map, then retire every shared owner of that
        // Player before releasing the sole-login claim below.
        self.unregister_from_player_registry();
        self.notify_other_players_visibility_changed_like_cpp();
        self.unregister_canonical_player_from_map_like_cpp();
        self.unregister_from_object_accessor();

        // Send LogoutComplete → client returns to character select
        self.set_state(crate::session::SessionState::Authed);
        self.send_packet(&LogoutComplete);
        self.mark_character_account_offline_like_cpp().await;
        self.set_player_guid(None);
        // Keep the sole character authority until the account-wide offline
        // write and old Player identity teardown are complete. Otherwise a
        // new login can publish online=true before this logout's broader
        // online=false update reaches the database.
        self.release_character_login_claim_like_cpp();

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

    /// Continue the player login after the instance socket is connected.
    ///
    /// Called when the `instance_link_rx` oneshot delivers the new channels.
    /// Sends ResumeComms and the full login sequence after the instance socket is connected.
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

        // Send ResumeComms only when using ConnectTo flow.
        // In direct login (no session_mgr), the client didn't go through ConnectTo
        // and doesn't expect ResumeComms — sending it causes a disconnect.
        if self.session_mgr().is_some() {
            self.send_packet(&ResumeComms);
        }

        let Some(player_lifecycle_port) = self.player_lifecycle_port_like_cpp().map(Arc::clone)
        else {
            warn!("No player lifecycle persistence port for continue login");
            self.release_character_login_claim_like_cpp();
            return;
        };
        let base_row = match player_lifecycle_port
            .load_character_base_like_cpp(wow_persistence::PlayerCharacterBaseLoadRequestLikeCpp {
                player_guid: guid.counter() as u64,
            })
            .await
        {
            wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(Some(row)) => row,
            wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(None) => {
                warn!("Character {:?} not found in database", guid);
                self.release_character_login_claim_like_cpp();
                return;
            }
            wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load character {:?}: {reason}", guid);
                self.release_character_login_claim_like_cpp();
                return;
            }
        };

        let name = base_row.name.clone();
        // Store character name for chat messages.
        self.set_loaded_player_name_like_cpp(name.clone());
        let race = base_row.race;
        let class = base_row.class;
        let gender = base_row.gender;
        let level = base_row.level;
        // C++ CHAR_SEL_CHARACTER column order:
        // 7=xp, 8=money, 14..18=position/map/orientation, 21=createMode, 23..24=played time,
        // 28=resettalents_cost, 29=resettalents_time, 39=at_login, 40=zone.
        let mut zone: i32 = base_row.zone_id.unwrap_or(0) as i32; // smallint unsigned
        let at_login_flags = base_row.at_login_flags.unwrap_or(0);
        let create_mode = base_row.create_mode.unwrap_or(0);
        let mut map_id: i32 = base_row.map_id.unwrap_or(0) as i32; // smallint unsigned
        let saved_map_id_for_transport = map_id as u16;
        let saved_transport_guid_low = base_row.transport_guid_low.unwrap_or(0);
        let saved_transport_position = Position::new(
            base_row.transport_x.unwrap_or(0.0),
            base_row.transport_y.unwrap_or(0.0),
            base_row.transport_z.unwrap_or(0.0),
            base_row.transport_orientation.unwrap_or(0.0),
        );
        let pos_x = base_row.position_x.unwrap_or(0.0);
        let pos_y = base_row.position_y.unwrap_or(0.0);
        let pos_z = base_row.position_z.unwrap_or(0.0);
        let orientation = base_row.orientation.unwrap_or(0.0);

        let mut position = Position::new(pos_x, pos_y, pos_z, orientation);
        let display_id = default_display_id(race, gender);
        let saved_character_map_is_battleground = self
            .map_store()
            .and_then(|store| store.get(map_id as u32))
            .is_some_and(|entry| entry.is_battleground_or_arena());
        let battleground_login_data = if saved_character_map_is_battleground {
            match player_lifecycle_port
                .load_login_admission_like_cpp(
                    wow_persistence::PlayerLoginAdmissionLoadRequestLikeCpp::BattlegroundLocation {
                        player_guid: guid.counter() as u64,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAdmissionLoadedLikeCpp::BattlegroundLocation(row),
                ) => row.map(|row| CharacterBattlegroundLoginDataLikeCpp {
                    entry_point: CharacterLoginLocationLikeCpp {
                        map_id: u32::from(row.map_id.unwrap_or(u16::MAX)),
                        bind_area_id: None,
                        position: Position::new(
                            row.x.unwrap_or(f32::NAN),
                            row.y.unwrap_or(f32::NAN),
                            row.z.unwrap_or(f32::NAN),
                            row.orientation.unwrap_or(f32::NAN),
                        ),
                    },
                }),
                wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!(
                        player_guid = guid.counter(),
                        %reason,
                        "failed to load character_battleground_data like C++ Player::_LoadBGData"
                    );
                    None
                }
                _ => {
                    warn!(
                        player_guid = guid.counter(),
                        "unexpected battleground-location lifecycle result"
                    );
                    None
                }
            }
        } else {
            None
        };
        let loaded_login_homebind = match player_lifecycle_port
            .load_login_admission_like_cpp(
                wow_persistence::PlayerLoginAdmissionLoadRequestLikeCpp::HomebindLocation {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAdmissionLoadedLikeCpp::HomebindLocation(row),
            ) => row.map(|row| CharacterLoginLocationLikeCpp {
                map_id: u32::from(row.map_id.unwrap_or(u16::MAX)),
                bind_area_id: Some(u32::from(row.area_id.unwrap_or(0))),
                position: Position::new(
                    row.x.unwrap_or(f32::NAN),
                    row.y.unwrap_or(f32::NAN),
                    row.z.unwrap_or(f32::NAN),
                    row.orientation.unwrap_or(f32::NAN),
                ),
            }),
            wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    player_guid = guid.counter(),
                    %reason,
                    "failed to load character homebind like C++ Player::_LoadHomeBind"
                );
                self.kick("WorldSession::HandlePlayerLogin Player::_LoadHomeBind query failed");
                return;
            }
            _ => {
                warn!(
                    player_guid = guid.counter(),
                    "unexpected homebind-location lifecycle result"
                );
                self.kick("WorldSession::HandlePlayerLogin Player::_LoadHomeBind query failed");
                return;
            }
        };
        let loaded_guild_id_like_cpp = match player_lifecycle_port
            .load_login_admission_like_cpp(
                wow_persistence::PlayerLoginAdmissionLoadRequestLikeCpp::GuildMembership {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAdmissionLoadedLikeCpp::GuildMembership(rows),
            ) if rows.is_empty() => Some(0),
            wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAdmissionLoadedLikeCpp::GuildMembership(rows),
            ) if rows.len() == 1 => {
                if rows[0].guild_id.is_none() {
                    warn!(
                        player_guid = guid.counter(),
                        "Keeping guild membership authority incomplete: malformed row"
                    );
                }
                rows[0].guild_id
            }
            wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAdmissionLoadedLikeCpp::GuildMembership(_),
            ) => {
                warn!(
                    player_guid = guid.counter(),
                    "Keeping guild membership authority incomplete: duplicate rows"
                );
                None
            }
            wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    player_guid = guid.counter(),
                    %reason,
                    "Failed to load guild membership for player login"
                );
                None
            }
            _ => {
                warn!(
                    player_guid = guid.counter(),
                    "unexpected guild-membership lifecycle result"
                );
                None
            }
        };
        let valid_login_homebind = loaded_login_homebind.filter(|homebind| {
            usable_character_homebind_like_cpp(
                *homebind,
                self.map_store().map(Arc::as_ref),
                self.expansion,
            )
        });
        let first_login = at_login_flags & 0x020 != 0;
        let Some(player_create_info) = self
            .player_create_info_store_like_cpp()
            .and_then(|store| store.get(race, class))
            .copied()
        else {
            warn!(
                player_guid = guid.counter(),
                race,
                class,
                "C++ Player::_LoadHomeBind rejected missing/invalid playercreateinfo; aborting login"
            );
            self.kick("WorldSession::HandlePlayerLogin Player::_LoadHomeBind player info failed");
            return;
        };
        if loaded_login_homebind.is_some() && valid_login_homebind.is_none() {
            warn!(
                player_guid = guid.counter(),
                "repairing invalid, instanceable, or expansion-inaccessible character homebind like C++ Player::_LoadHomeBind"
            );
            self.delete_invalid_character_homebind_like_cpp(guid).await;
        }
        let repaired_or_valid_homebind = if let Some(homebind) = valid_login_homebind {
            Some(homebind)
        } else {
            self.repair_character_homebind_like_cpp(
                guid,
                race,
                player_create_info,
                create_mode,
                first_login,
            )
            .await
        };
        let Some(login_homebind) = repaired_or_valid_homebind else {
            warn!(
                player_guid = guid.counter(),
                race,
                class,
                create_mode,
                "C++ Player::_LoadHomeBind could not establish a valid homebind; aborting login"
            );
            self.kick("WorldSession::HandlePlayerLogin Player::_LoadHomeBind failed");
            return;
        };

        // Load played time + money/xp from DB using C++ CHAR_SEL_CHARACTER order.
        self.total_played_time = base_row.total_played_time.unwrap_or(0);
        self.level_played_time = base_row.level_played_time.unwrap_or(0);
        self.set_player_gold_like_cpp(base_row.money.unwrap_or(0));
        self.set_player_inventory_slot_count_like_cpp(
            loaded_inventory_slot_count_with_legacy_rust_compat(
                base_row.inventory_slots.unwrap_or(INVENTORY_DEFAULT_SIZE),
            ),
        );
        self.set_player_bank_bag_slot_count_like_cpp(base_row.bank_slots.unwrap_or(0));
        self.set_player_xp_like_cpp(base_row.xp.unwrap_or(0));
        self.set_represented_talent_reset_state_like_cpp(
            base_row.talent_reset_cost.unwrap_or(0),
            base_row.talent_reset_time_secs.unwrap_or(0),
        );
        self.set_represented_active_talent_group_like_cpp(
            base_row.active_talent_group.unwrap_or(0),
        );
        self.set_represented_bonus_talent_groups_like_cpp(
            base_row.bonus_talent_groups.unwrap_or(0),
        );
        self.set_player_create_mode_like_cpp(create_mode);
        self.set_represented_at_login_flags_like_cpp(at_login_flags);
        let saved_rest_state = base_row.rest_state.unwrap_or(REST_STATE_NORMAL_LIKE_CPP);
        let saved_rest_bonus = base_row.rest_bonus.unwrap_or(0.0);
        let saved_logout_time_secs = base_row.logout_time_secs.unwrap_or(0);
        let saved_logout_was_resting = base_row.logout_was_resting.unwrap_or(0) != 0;
        self.load_represented_explored_zones_like_cpp(&base_row.explored_zones);
        self.set_player_guid(Some(guid));
        self.set_loaded_player_flags_like_cpp(base_row.player_flags.unwrap_or(0));
        self.set_loaded_player_flags_ex_like_cpp(base_row.player_flags_ex.unwrap_or(0));
        self.set_loaded_player_identity_like_cpp(map_id as u16, race, class, level, gender);
        // C++ recalculates zone/area from terrain after AddToMap
        // (`Player::SendInitialPacketsAfterAddToMap`). Seed from DB until
        // that post-add terrain pass runs.
        self.set_player_zone_area_like_cpp(zone as u32, zone as u32);
        self.set_represented_homebind_like_cpp(RepresentedHomebindLikeCpp {
            map_id: login_homebind.map_id,
            area_id: login_homebind
                .bind_area_id
                .expect("validated character homebind must have an area ID"),
            position: login_homebind.position,
        });
        if let Some(guild_id) = loaded_guild_id_like_cpp {
            self.set_represented_guild_id_like_cpp(guild_id);
        }
        self.load_represented_player_difficulties_like_cpp(
            base_row.dungeon_difficulty.unwrap_or(0),
            base_row.raid_difficulty.unwrap_or(0),
            base_row.legacy_raid_difficulty.unwrap_or(0),
        );
        let summoned_pet_number = base_row.summoned_pet_number.unwrap_or(0);
        const AT_LOGIN_RESET_PET_TALENTS_LIKE_CPP: u16 = 0x010;
        if (self.represented_at_login_flags_like_cpp() & AT_LOGIN_RESET_PET_TALENTS_LIKE_CPP) != 0 {
            let outcome = player_lifecycle_port
                .reset_login_pet_talents_like_cpp(guid.counter() as u64)
                .await;
            if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason: error }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason: error } =
                outcome.spell_delete
            {
                warn!(
                    player_guid = guid.counter(),
                    %error,
                    "failed to apply represented AT_LOGIN_RESET_PET_TALENTS pet_spell delete like C++"
                );
            }
            if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason: error }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason: error } =
                outcome.specialization_reset
            {
                warn!(
                    player_guid = guid.counter(),
                    %error,
                    "failed to apply represented AT_LOGIN_RESET_PET_TALENTS pet specialization reset like C++"
                );
            }
        }
        self.begin_represented_character_pet_authority_load_like_cpp();
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::PetStable {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::PetStable(rows),
            ) => {
                let rows = rows.into_iter().map(|row| CharacterPetStableRowLikeCpp {
                    pet_number: row.pet_number,
                    creature_id: row.creature_id,
                    display_id: row.display_id,
                    level: row.level,
                    experience: row.experience,
                    react_state: row.react_state,
                    slot: row.slot,
                    name: row.name,
                    was_renamed: row.was_renamed,
                    health: row.health,
                    mana: row.mana,
                    action_bar: row.action_bar,
                    last_save_time: row.last_save_time,
                    created_by_spell_id: row.created_by_spell_id,
                    pet_type: row.pet_type,
                    specialization_id: row.specialization_id,
                });
                let loaded =
                    self.load_represented_pet_stable_rows_like_cpp(summoned_pet_number, rows);
                trace!(
                    player_guid = guid.counter(),
                    summoned_pet_number,
                    loaded,
                    "loaded represented character_pet stable rows like C++"
                );
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => warn!(
                player_guid = guid.counter(),
                error = %reason,
                "failed to load represented character_pet rows"
            ),
            _ => unreachable!("pet stable request returned a different row family"),
        }
        if summoned_pet_number != 0 {
            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuras {
                        pet_number: summoned_pet_number,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::PetAuras(rows),
                ) => {
                    let rows = rows.into_iter().map(|row| CharacterPetAuraRowLikeCpp {
                        caster_guid: object_guid_from_db_binary_like_cpp(row.caster_guid_binary),
                        spell_id: row.spell_id,
                        effect_mask: row.effect_mask,
                        recalculate_mask: row.recalculate_mask,
                        difficulty: row.difficulty,
                        stack_count: row.stack_count,
                        max_duration_ms: row.max_duration_ms,
                        remain_time_ms: row.remain_time_ms,
                        remain_charges: row.remain_charges,
                    });
                    let loaded =
                        self.load_represented_pet_aura_rows_like_cpp(summoned_pet_number, rows);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number, loaded, "loaded represented pet_aura rows like C++"
                    );
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!(player_guid = guid.counter(), summoned_pet_number, error = %reason, "failed to load represented pet_aura rows")
                }
                _ => unreachable!("pet aura request returned a different row family"),
            }

            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuraEffects {
                        pet_number: summoned_pet_number,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::PetAuraEffects(rows),
                ) => {
                    let rows = rows
                        .into_iter()
                        .map(|row| CharacterPetAuraEffectRowLikeCpp {
                            caster_guid: object_guid_from_db_binary_like_cpp(
                                row.caster_guid_binary,
                            ),
                            spell_id: row.spell_id,
                            effect_mask: row.effect_mask,
                            effect_index: row.effect_index,
                            amount: row.amount,
                            base_amount: row.base_amount,
                        });
                    let loaded = self
                        .load_represented_pet_aura_effect_rows_like_cpp(summoned_pet_number, rows);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        loaded,
                        "loaded represented pet_aura_effect rows like C++"
                    );
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!(player_guid = guid.counter(), summoned_pet_number, error = %reason, "failed to load represented pet_aura_effect rows")
                }
                _ => unreachable!("pet aura-effect request returned a different row family"),
            }

            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpells {
                        pet_number: summoned_pet_number,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::PetSpells(rows),
                ) => {
                    let rows = rows.into_iter().map(|row| CharacterPetSpellRowLikeCpp {
                        spell_id: row.spell_id,
                        active: row.active,
                    });
                    let loaded =
                        self.load_represented_pet_spell_rows_like_cpp(summoned_pet_number, rows);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number, loaded, "loaded represented pet_spell rows like C++"
                    );
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!(player_guid = guid.counter(), summoned_pet_number, error = %reason, "failed to load represented pet_spell rows")
                }
                _ => unreachable!("pet spell request returned a different row family"),
            }

            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCooldowns {
                        pet_number: summoned_pet_number,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::PetSpellCooldowns(rows),
                ) => {
                    let rows = rows
                        .into_iter()
                        .map(|row| CharacterPetSpellCooldownRowLikeCpp {
                            spell_id: row.spell_id,
                            cooldown_end_unix_secs: row.cooldown_end_unix_secs,
                            category_id: row.category_id,
                            category_end_unix_secs: row.category_end_unix_secs,
                        });
                    let loaded = self.load_represented_pet_spell_cooldown_rows_like_cpp(
                        summoned_pet_number,
                        rows,
                    );
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        loaded,
                        "loaded represented pet_spell_cooldown rows like C++"
                    );
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!(player_guid = guid.counter(), summoned_pet_number, error = %reason, "failed to load represented pet_spell_cooldown rows")
                }
                _ => unreachable!("pet cooldown request returned a different row family"),
            }

            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCharges {
                        pet_number: summoned_pet_number,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::PetSpellCharges(rows),
                ) => {
                    let rows = rows
                        .into_iter()
                        .map(|row| CharacterPetSpellChargeRowLikeCpp {
                            category_id: row.category_id,
                            recharge_start_unix_secs: row.recharge_start_unix_secs,
                            recharge_end_unix_secs: row.recharge_end_unix_secs,
                        });
                    let loaded = self
                        .load_represented_pet_spell_charge_rows_like_cpp(summoned_pet_number, rows);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        loaded,
                        "loaded represented pet_spell_charges rows like C++"
                    );
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!(player_guid = guid.counter(), summoned_pet_number, error = %reason, "failed to load represented pet_spell_charges rows")
                }
                _ => unreachable!("pet charge request returned a different row family"),
            }

            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::PetDeclinedNames {
                        player_guid: guid.counter() as u64,
                        pet_number: summoned_pet_number,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::PetDeclinedNames(rows),
                ) => {
                    let row = rows
                        .into_iter()
                        .next()
                        .map(|row| CharacterPetDeclinedNamesRowLikeCpp { names: row.names });
                    let loaded =
                        self.load_represented_pet_declined_names_like_cpp(summoned_pet_number, row);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        loaded,
                        "loaded represented character_pet_declinedname row like C++"
                    );
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!(player_guid = guid.counter(), summoned_pet_number, error = %reason, "failed to load represented character_pet_declinedname row")
                }
                _ => unreachable!("pet declined-name request returned a different row family"),
            }
        }
        if (self.represented_at_login_flags_like_cpp() & AT_LOGIN_RESET_PET_TALENTS_LIKE_CPP) != 0 {
            self.apply_represented_login_pet_talent_reset_like_cpp();
        }
        self.group_guid = None;
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::GroupMembership {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::GroupMembership(rows),
            ) => {
                if let Some(db_store_id) = rows.into_iter().next() {
                    let _ = self.load_represented_group_by_db_store_id_like_cpp(db_store_id);
                    let _ = self.reset_group_update_sequence_if_needed_like_cpp();
                }
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => warn!(
                player_guid = guid.counter(),
                error = %reason,
                "failed to load represented group membership"
            ),
            _ => unreachable!("group-membership request returned a different row family"),
        }
        self.refresh_next_level_xp();
        self.clamp_loaded_player_xp_to_next_level_like_cpp();
        let attached_controller = self.ensure_login_player_controller_like_cpp(
            guid,
            name.clone(),
            position,
            map_id as u16,
            race,
            class,
            level,
            gender,
        );
        if saved_character_map_is_battleground {
            // Rust does not yet have a live BattlegroundMgr roster/status
            // authority, so it cannot prove C++'s `currentBg &&
            // IsPlayerInBattleground && status != WAIT_LEAVE` resume branch.
            // Follow C++'s BG-unavailable branch instead of fabricating or
            // joining a canonical map from stale DB data.
            let fallback = battleground_login_fallback_location_like_cpp(
                battleground_login_data,
                Some(login_homebind),
                self.map_store().map(Arc::as_ref),
            );
            if let Some(fallback) = fallback {
                let fallback_map_id = u16::try_from(fallback.map_id)
                    .expect("validated battleground login fallback map ID");
                map_id = i32::from(fallback_map_id);
                position = fallback.position;
                self.seed_login_location_zone_area_like_cpp(&mut zone, fallback);
                self.set_player_map_position_like_cpp(fallback_map_id, fallback.position);
                let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
                info!(
                    player_guid = guid.counter(),
                    map_id,
                    "battleground runtime unavailable; relocated to entry point/homebind like C++ Player::LoadFromDB"
                );
            } else {
                warn!(
                    player_guid = guid.counter(),
                    saved_map_id = map_id,
                    "battleground unavailable and no valid entry point/homebind was loaded"
                );
            }
        } else if attached_controller {
            let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
        }
        if self.retry_login_at_homebind_like_cpp(
            &mut map_id,
            &mut zone,
            &mut position,
            login_homebind,
        ) {
            info!(
                player_guid = guid.counter(),
                map_id,
                "initial canonical map selection failed; relocated to homebind like C++ Player::LoadFromDB"
            );
        }
        if attached_controller {
            self.apply_loaded_player_flags_to_canonical_like_cpp();
            let _ = self.apply_represented_group_leader_flag_like_cpp();
        }
        self.load_represented_xp_rest_bonus_like_cpp(saved_rest_state, saved_rest_bonus);
        let applied_rest_bonus = self.apply_offline_xp_rest_bonus_like_cpp(
            saved_logout_time_secs,
            Self::current_game_time_secs_like_cpp(),
            saved_logout_was_resting,
        );
        if std::env::var_os("RUSTYCORE_REST_TRACE").is_some() {
            info!(
                player_guid = guid.counter(),
                saved_rest_state,
                saved_rest_bonus,
                saved_logout_time_secs,
                saved_logout_was_resting,
                applied_rest_bonus,
                rest_bonus = self.represented_xp_rest_bonus_like_cpp(),
                rest_state = self.represented_xp_rest_state_like_cpp(),
                "RUST_PLAYER_REST_LOAD"
            );
        }
        self.load_represented_character_titles_like_cpp(
            &base_row.known_titles.clone().unwrap_or_default(),
            base_row.chosen_title.unwrap_or(0),
        );

        self.load_account_toys_like_cpp().await;
        self.load_account_heirlooms_like_cpp().await;
        self.load_account_item_appearances_like_cpp().await;
        self.load_account_transmog_illusions_like_cpp().await;
        let account_mount_rows_complete_like_cpp = self.load_account_mounts_like_cpp().await;

        // Load equipped items for visible display + inventory objects
        let mut visible_items = [(0i32, 0u16, 0u16); 19];
        let mut inv_slots = [ObjectGuid::EMPTY; 141];
        let mut item_creates: Vec<wow_packet::packets::update::ItemCreateData> = Vec::new();
        let mut login_bag_create_index_by_slot: HashMap<u8, usize> = HashMap::new();
        let mut loaded_inventory_item_guids: Vec<ObjectGuid> = Vec::new();
        let mut loaded_equipped_item_guids: Vec<ObjectGuid> = Vec::new();
        let realm_id = self.realm_id();
        self.clear_inventory_items_and_objects_like_cpp();
        self.clear_player_currencies_like_cpp();
        {
            self.begin_player_equipment_inventory_authority_load_like_cpp();
            let mut refund_cleanup_actions = Vec::new();
            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentInventory {
                        player_guid: guid.counter() as u64,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::EquipmentInventory(rows),
                ) => {
                    let equipment_inventory_source_is_proven_empty = rows.is_empty();
                    for row in rows {
                        let slot = row.slot;
                        let item = row.item;
                        let item_entry = item.item_entry;
                        let item_db_guid = item.item_db_guid;
                        let item_count = item.count;
                        let item_durability = item.durability;
                        let item_context =
                            <ItemContext as num_traits::FromPrimitive>::from_u8(item.context)
                                .unwrap_or(ItemContext::None);
                        let item_flags = item.flags;
                        let item_played_time = item.played_time;
                        let item_expiration = item.expiration;
                        let item_spell_charges = item.spell_charges;
                        let item_enchantments = item.enchantments;
                        let item_enchantment_values =
                            loaded_item_enchantments_like_cpp(&item_enchantments);
                        let random_properties = loaded_item_random_properties_like_cpp(
                            item.random_properties_id,
                            item.random_properties_seed,
                            self.item_random_properties_store()
                                .map(|store| store.as_ref()),
                            self.item_random_suffix_store().map(|store| store.as_ref()),
                        );
                        let random_properties_id =
                            random_properties.map(|random| random.id).unwrap_or(0);
                        let random_properties_seed =
                            random_properties.map(|random| random.seed).unwrap_or(0);
                        let socketed_gems = loaded_socketed_gems_like_cpp(item.gems);
                        let socketed_gem_create_updates =
                            loaded_socketed_gem_create_updates_like_cpp(&socketed_gems);
                        let item_create_enchantments = loaded_item_effective_enchantments_like_cpp(
                            item_enchantment_values.as_ref(),
                            random_properties_id,
                            self.item_random_properties_store()
                                .map(|store| store.as_ref()),
                            self.item_random_suffix_store().map(|store| store.as_ref()),
                        );
                        let refund_decision = loaded_item_refund_decision(
                            item_flags,
                            item_played_time,
                            item.paid_money,
                            item.paid_extended_cost,
                        );
                        if item_entry > 0 && (slot as usize) < 141 {
                            let item_max_durability = self
                                .item_template_max_durability(item_entry)
                                .max(item_durability);
                            let item_guid = ObjectGuid::create_item(realm_id, item_db_guid as i64);
                            let stored_flags = match refund_decision {
                                LoadedItemRefundDecision::Clear { new_flags } => {
                                    refund_cleanup_actions.push(
                                        wow_persistence::PlayerLoginItemRepairActionLikeCpp::ClearRefundable {
                                            item_guid: item_db_guid,
                                            new_flags,
                                        },
                                    );
                                    new_flags
                                }
                                LoadedItemRefundDecision::None
                                | LoadedItemRefundDecision::Valid { .. } => item_flags,
                            };
                            inv_slots[slot as usize] = item_guid;
                            let storage_template = self.item_storage_template(item_entry);
                            let inventory_type = storage_template
                                .as_ref()
                                .map(|template| template.inventory_type as u8)
                                .filter(|&inventory_type| {
                                    inventory_type != InventoryType::NonEquip as u8
                                })
                                .or_else(|| {
                                    if slot < 19 {
                                        slot_to_inventory_type(slot)
                                    } else {
                                        None
                                    }
                                });
                            let is_bag_container = inventory_type == Some(InventoryType::Bag as u8);
                            let container_slots = if is_bag_container {
                                storage_template
                                    .as_ref()
                                    .map(|template| u32::from(template.container_slots))
                                    .unwrap_or(0)
                                    .min(36)
                            } else {
                                0
                            };
                            let create_index = item_creates.len();
                            item_creates.push(wow_packet::packets::update::ItemCreateData {
                                item_guid,
                                entry_id: item_entry as i32,
                                owner_guid: guid,
                                contained_in: guid,
                                stack_count: item_count,
                                dynamic_flags: stored_flags,
                                durability: item_durability,
                                max_durability: item_max_durability,
                                random_properties_seed,
                                random_properties_id,
                                enchantments: item_create_enchantments,
                                gems: socketed_gem_create_updates,
                                context: item_context as u8,
                                container_slots,
                                container_item_guids: [ObjectGuid::EMPTY; 36],
                            });
                            if container_slots > 0 {
                                login_bag_create_index_by_slot.insert(slot, create_index);
                            }
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
                            let template_expiration = self
                                .item_stats_store()
                                .and_then(|store| store.duration_in_inventory(item_entry))
                                .unwrap_or(0);
                            let effect_count = self.item_effect_count_like_cpp(item_entry);
                            let expiration_needs_save =
                                apply_loaded_item_storage_mutable_fields_like_cpp(
                                    &mut item_object,
                                    item_expiration,
                                    template_expiration,
                                    &item_spell_charges,
                                    effect_count,
                                );
                            apply_loaded_item_instance_fields_like_cpp(
                                &mut item_object,
                                &item_create_enchantments,
                                random_properties,
                            );
                            item_object.set_gems(socketed_gems);
                            item_object.replace_all_item_flags(ItemFieldFlags::from_bits_retain(
                                stored_flags,
                            ));
                            if expiration_needs_save {
                                refund_cleanup_actions.push(
                                    wow_persistence::PlayerLoginItemRepairActionLikeCpp::NormalizeOnLoad {
                                        item_guid: item_db_guid,
                                        expiration: item_object.data().expiration,
                                        flags: item_object.item_flags_bits(),
                                        durability: item_object.data().durability,
                                    },
                                );
                            }
                            if let LoadedItemRefundDecision::Valid {
                                paid_money,
                                paid_extended_cost,
                            } = refund_decision
                            {
                                item_object.set_refund_recipient(guid);
                                item_object.set_paid_money(paid_money);
                                item_object.set_paid_extended_cost(u32::from(paid_extended_cost));
                            }
                            self.apply_loaded_inventory_item_collection_hooks_like_cpp(
                                &item_object,
                            );
                            item_object.set_state(ItemUpdateState::Unchanged);
                            let visible_item_fields = ((slot as usize) < 19).then(|| {
                                self.loaded_inventory_item_visible_fields_like_cpp(&item_object)
                            });
                            self.insert_inventory_item_object(item_object);
                            loaded_inventory_item_guids.push(item_guid);
                            if loaded_item_slot_applies_equipped_enchantments_like_cpp(slot) {
                                loaded_equipped_item_guids.push(item_guid);
                            }
                            // Slots 0-18 also populate VisibleItems for character model
                            if let Some(fields) = visible_item_fields {
                                visible_items[slot as usize] = fields;
                            }
                        }
                    }
                    if equipment_inventory_source_is_proven_empty {
                        self.complete_player_equipment_inventory_authority_load_like_cpp();
                    }
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!("Failed to load equipment for {:?}: {}", guid, reason);
                }
                _ => unreachable!("equipment request returned a different row family"),
            }
            if !refund_cleanup_actions.is_empty() {
                let outcome = player_lifecycle_port
                    .persist_login_item_repairs_like_cpp(
                        wow_persistence::PlayerLoginItemRepairRequestLikeCpp {
                            actions: refund_cleanup_actions,
                        },
                    )
                    .await;
                if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
                | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } = outcome
                {
                    warn!(
                        "Failed to clean expired/missing item refund metadata for {:?}: {}",
                        guid, reason
                    );
                }
            }

            // ── Load represented bag contents (nested items) ──
            // C++ `Player::_LoadInventory` loads child rows after their top-level
            // bag rows. `character_inventory.bag` stores the bag item GUID, so the
            // query joins back to the represented bag row and returns its top-level slot.
            {
                let mut bag_load_fix_actions = Vec::new();
                match player_lifecycle_port
                    .load_login_auxiliary_like_cpp(
                        wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::BagInventory {
                            player_guid: guid.counter() as u64,
                        },
                    )
                    .await
                {
                    wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                        wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::BagInventory(rows),
                    ) => {
                        for row in rows {
                            let bag_slot = row.bag_slot;
                            let inner_slot = row.inner_slot;
                            let item = row.item;
                            let item_entry = item.item_entry;
                            let item_db_guid = item.item_db_guid;
                            let item_count = item.count;
                            let item_durability = item.durability;
                            let item_context =
                                <ItemContext as num_traits::FromPrimitive>::from_u8(item.context)
                                    .unwrap_or(ItemContext::None);
                            let item_flags = item.flags;
                            let item_played_time = item.played_time;
                            let item_expiration = item.expiration;
                            let item_spell_charges = item.spell_charges;
                            let item_enchantments = item.enchantments;
                            let item_enchantment_values =
                                loaded_item_enchantments_like_cpp(&item_enchantments);
                            let random_properties = loaded_item_random_properties_like_cpp(
                                item.random_properties_id,
                                item.random_properties_seed,
                                self.item_random_properties_store()
                                    .map(|store| store.as_ref()),
                                self.item_random_suffix_store().map(|store| store.as_ref()),
                            );
                            let random_properties_id =
                                random_properties.map(|random| random.id).unwrap_or(0);
                            let random_properties_seed =
                                random_properties.map(|random| random.seed).unwrap_or(0);
                            let socketed_gems = loaded_socketed_gems_like_cpp(item.gems);
                            let socketed_gem_create_updates =
                                loaded_socketed_gem_create_updates_like_cpp(&socketed_gems);
                            let item_create_enchantments =
                                loaded_item_effective_enchantments_like_cpp(
                                    item_enchantment_values.as_ref(),
                                    random_properties_id,
                                    self.item_random_properties_store()
                                        .map(|store| store.as_ref()),
                                    self.item_random_suffix_store().map(|store| store.as_ref()),
                                );
                            if item_entry > 0 && is_represented_bag_slot(bag_slot) {
                                if let Some(bag_item_guid) = self
                                    .inventory_items_like_cpp()
                                    .get(&bag_slot)
                                    .map(|bag_item| bag_item.guid)
                                {
                                    let item_guid =
                                        ObjectGuid::create_item(realm_id, item_db_guid as i64);
                                    let item_max_durability = self
                                        .item_template_max_durability(item_entry)
                                        .max(item_durability);
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
                                    let template_expiration = self
                                        .item_stats_store()
                                        .and_then(|store| store.duration_in_inventory(item_entry))
                                        .unwrap_or(0);
                                    let effect_count = self.item_effect_count_like_cpp(item_entry);
                                    let expiration_needs_save =
                                        apply_loaded_item_storage_mutable_fields_like_cpp(
                                            &mut item_object,
                                            item_expiration,
                                            template_expiration,
                                            &item_spell_charges,
                                            effect_count,
                                        );
                                    apply_loaded_item_instance_fields_like_cpp(
                                        &mut item_object,
                                        &item_create_enchantments,
                                        random_properties,
                                    );
                                    item_object.set_gems(socketed_gems);
                                    item_object.replace_all_item_flags(
                                        ItemFieldFlags::from_bits_retain(item_flags),
                                    );
                                    if expiration_needs_save {
                                        bag_load_fix_actions.push(
                                            wow_persistence::PlayerLoginItemRepairActionLikeCpp::NormalizeOnLoad {
                                                item_guid: item_db_guid,
                                                expiration: item_object.data().expiration,
                                                flags: item_object.item_flags_bits(),
                                                durability: item_object.data().durability,
                                            },
                                        );
                                    }
                                    item_object
                                        .set_container_guid_and_slot(bag_item_guid, bag_slot);
                                    self.apply_loaded_inventory_item_collection_hooks_like_cpp(
                                        &item_object,
                                    );
                                    item_object.set_state(ItemUpdateState::Unchanged);
                                    if let Some(&create_index) =
                                        login_bag_create_index_by_slot.get(&bag_slot)
                                    {
                                        if (inner_slot as usize) < 36 {
                                            item_creates[create_index].container_item_guids
                                                [inner_slot as usize] = item_guid;
                                        }
                                    }
                                    item_creates.push(
                                        wow_packet::packets::update::ItemCreateData {
                                            item_guid,
                                            entry_id: item_entry as i32,
                                            owner_guid: guid,
                                            contained_in: bag_item_guid,
                                            stack_count: item_count,
                                            dynamic_flags: item_flags,
                                            durability: item_durability,
                                            max_durability: item_max_durability,
                                            random_properties_seed,
                                            random_properties_id,
                                            enchantments: item_create_enchantments,
                                            gems: socketed_gem_create_updates,
                                            context: item_context as u8,
                                            container_slots: 0,
                                            container_item_guids: [ObjectGuid::EMPTY; 36],
                                        },
                                    );
                                    self.insert_inventory_item_object(item_object);
                                    loaded_inventory_item_guids.push(item_guid);
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
                        }
                    }
                    wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                        warn!("Failed to load bag contents for {:?}: {}", guid, reason);
                    }
                    _ => unreachable!("bag inventory request returned a different row family"),
                }
                if !bag_load_fix_actions.is_empty() {
                    let outcome = player_lifecycle_port
                        .persist_login_item_repairs_like_cpp(
                            wow_persistence::PlayerLoginItemRepairRequestLikeCpp {
                                actions: bag_load_fix_actions,
                            },
                        )
                        .await;
                    if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
                    | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } = outcome
                    {
                        warn!(
                            "Failed to normalize loaded bag item state for {:?}: {}",
                            guid, reason
                        );
                    }
                }
            }

            // inventory_type is now loaded from the canonical ItemTemplate bridge.
            // No SQL cache needed.
        }
        self.sync_player_inventory_like_cpp();
        let (loaded_item_time_updates, loaded_non_equipped_enchantment_updates) = self
            .register_loaded_inventory_item_duration_refs_like_cpp(
                &loaded_inventory_item_guids,
                &loaded_equipped_item_guids,
            );

        // ── Load void storage ──
        // C++ `Player::LoadFromDB` calls `_LoadVoidStorage` only when the
        // already-loaded player flags say the vault is unlocked. A locked
        // character starts with coherent empty storage even if stale rows
        // exist in CharacterDB.
        if self.prepare_represented_void_storage_login_load_like_cpp() {
            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::VoidStorage {
                        player_guid: guid.counter() as u64,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::VoidStorage(rows),
                ) => {
                    for row in rows {
                        let context = void_storage_login_context_like_cpp(
                            row.random_properties_id,
                            row.context,
                        );
                        let creator_guid = if row.creator_guid == 0 {
                            ObjectGuid::EMPTY
                        } else {
                            ObjectGuid::create_player(realm_id, row.creator_guid as i64)
                        };
                        let loaded = self.load_represented_void_storage_row_like_cpp(
                            row.slot,
                            RepresentedVoidStorageItemLikeCpp {
                                item_id: row.item_id,
                                item_entry: row.item_entry,
                                creator_guid,
                                fixed_scaling_level: row.fixed_scaling_level,
                                random_properties_id: row.random_properties_id,
                                random_properties_seed: row.random_properties_seed,
                                context,
                            },
                        );
                        if !loaded {
                            warn!(
                                player_guid = guid.counter(),
                                item_id = row.item_id,
                                item_entry = row.item_entry,
                                slot = row.slot,
                                "Player::_LoadVoidStorage skipped an invalid row like C++"
                            );
                        }
                    }
                    self.mark_represented_void_storage_loaded_like_cpp();
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!("Failed to load void storage for {:?}: {}", guid, reason);
                }
                _ => unreachable!("void-storage request returned a different row family"),
            }
        }

        // ── Load equipment sets / transmog outfits ──
        // C++ `Player::_LoadEquipmentSets` and `_LoadTransmogOutfits` rebuild
        // one shared `_equipmentSets` container before `SendEquipmentSetList`.
        self.clear_represented_equipment_sets_like_cpp();
        let mut equipment_sets_loaded = true;
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentSets {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::EquipmentSets(rows),
            ) => {
                for row in rows {
                    let mut pieces = [ObjectGuid::EMPTY;
                        wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
                    for (piece, item_low_guid) in
                        pieces.iter_mut().zip(row.item_low_guids.into_iter())
                    {
                        if item_low_guid != 0 {
                            *piece = ObjectGuid::create_item(realm_id, item_low_guid as i64);
                        }
                    }
                    self.load_represented_equipment_set_row_like_cpp(
                        row.set_guid,
                        u32::from(row.set_id),
                        row.name,
                        row.icon,
                        row.ignore_mask,
                        row.assigned_spec_index,
                        pieces,
                    );
                }
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                equipment_sets_loaded = false;
                warn!("Failed to load equipment sets for {:?}: {}", guid, reason);
            }
            _ => unreachable!("equipment-set request returned a different row family"),
        }
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::TransmogOutfits {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::TransmogOutfits(rows),
            ) => {
                for row in rows {
                    let mut appearances =
                        [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
                    for (appearance, loaded) in
                        appearances.iter_mut().zip(row.appearances.into_iter())
                    {
                        *appearance = loaded;
                    }
                    self.load_represented_transmog_outfit_row_like_cpp(
                        row.set_guid,
                        u32::from(row.set_id),
                        row.name,
                        row.icon,
                        row.ignore_mask,
                        appearances,
                        row.enchants,
                    );
                }
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                equipment_sets_loaded = false;
                warn!("Failed to load transmog outfits for {:?}: {}", guid, reason);
            }
            _ => unreachable!("transmog-outfit request returned a different row family"),
        }
        if equipment_sets_loaded {
            self.mark_represented_equipment_sets_loaded_like_cpp();
        }

        // ── Load compact unit-frame profiles ──
        // C++ `Player::_LoadCUFProfiles` fills `_CUFProfiles[id]`, then
        // `WorldSession::SendLoadCUFProfiles` sends only occupied slots. The
        // legacy fork checks `id > MAX_CUF_PROFILES`, but the backing array has
        // length MAX_CUF_PROFILES; Rust rejects `id >= MAX` to avoid the OOB
        // bug while preserving valid row semantics.
        self.clear_represented_cuf_profiles_like_cpp();
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::CufProfiles {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::CufProfiles(rows),
            ) => {
                for row in rows {
                    let id = row.id;
                    let profile = wow_packet::packets::misc::CufProfile {
                        profile_name: row.name,
                        frame_height: row.frame_height,
                        frame_width: row.frame_width,
                        sort_by: row.sort_by,
                        health_text: row.health_text,
                        bool_options: row.bool_options,
                        top_point: row.top_point,
                        bottom_point: row.bottom_point,
                        left_point: row.left_point,
                        top_offset: row.top_offset,
                        bottom_offset: row.bottom_offset,
                        left_offset: row.left_offset,
                    };
                    if !self.load_represented_cuf_profile_like_cpp(id, profile) {
                        warn!(
                            player_guid = guid.counter(),
                            id,
                            max_profiles = wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP,
                            "Skipping invalid CUF profile id"
                        );
                    }
                }
                self.mark_represented_cuf_profiles_loaded_like_cpp();
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load CUF profiles for {:?}: {}", guid, reason);
            }
            _ => unreachable!("CUF-profile request returned a different row family"),
        }

        // ── Load character currencies from character_currency ──
        // C++ `Player::_LoadCurrency` skips rows not found in sCurrencyTypesStore.
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Currencies {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Currencies(rows),
            ) => {
                for row in rows {
                    let currency_id = u32::from(row.currency_id);
                    let known_currency = self
                        .currency_types_store()
                        .is_some_and(|store| store.has_record(currency_id));
                    if known_currency {
                        let mut currencies = self.player_currencies_like_cpp().clone();
                        currencies.entry(currency_id).or_insert_with(|| {
                            crate::session::PlayerCurrency {
                                state: crate::session::PlayerCurrencyState::Unchanged,
                                quantity: row.quantity,
                                weekly_quantity: row.weekly_quantity,
                                tracked_quantity: row.tracked_quantity,
                                increased_cap_quantity: row.increased_cap_quantity,
                                earned_quantity: row.earned_quantity,
                                flags: row.flags,
                            }
                        });
                        self.set_player_currencies_like_cpp(currencies);
                    }
                }
                info!(
                    "Loaded {} currencies for {:?}",
                    self.player_currencies_like_cpp().len(),
                    guid
                );
                self.sync_player_currencies_like_cpp();
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load currencies for {:?}: {}", guid, reason);
            }
            _ => unreachable!("currency request returned a different row family"),
        }

        // ── Load known spells from character_spell ──
        // Column types: spell=int unsigned, active=tinyint unsigned, disabled=tinyint unsigned
        let mut known_spells: Vec<i32> = Vec::new();
        let mut loaded_spell_side_effect_spells: Vec<i32> = Vec::new();
        let mut favorite_spell_rows: HashSet<i32> = HashSet::new();
        let mut loaded_player_spell_rows = Vec::new();
        let mut skill_rewarded_dependent_spells = HashSet::new();
        let mut skill_rewarded_removed_spells = HashSet::new();
        let mut loaded_player_spell_rows_complete_like_cpp = false;
        let mut favorite_spell_rows_complete_like_cpp = false;
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Spells {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Spells(rows),
            ) => {
                for row in rows {
                    if let Ok(spell_id) = i32::try_from(row.spell_id)
                        && spell_id > 0
                    {
                        loaded_player_spell_rows.push(
                            crate::session::RepresentedPlayerSpellLikeCpp {
                                spell_id,
                                active: row.active != 0,
                                disabled: row.disabled != 0,
                                dependent: false,
                                favorite: false,
                                state:
                                    crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged,
                            },
                        );
                    }
                    if let Some(spell_id_i32) =
                        loaded_spell_for_add_spell_side_effects_like_cpp(row.spell_id, row.disabled)
                    {
                        loaded_spell_side_effect_spells.push(spell_id_i32);
                    }
                    if let Some(spell_id) =
                        active_known_spell_for_send_like_cpp(row.spell_id, row.active, row.disabled)
                    {
                        known_spells.push(spell_id);
                    }
                }
                loaded_player_spell_rows_complete_like_cpp = true;
                info!("Loaded {} DB spells for {:?}", known_spells.len(), guid);
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load spells for {:?}: {}", guid, reason);
            }
            _ => unreachable!("spell request returned a different row family"),
        }

        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellFavorites {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::SpellFavorites(rows),
            ) => {
                for spell_id in rows {
                    if let Ok(spell_id) = i32::try_from(spell_id) {
                        favorite_spell_rows.insert(spell_id);
                    }
                }
                favorite_spell_rows_complete_like_cpp = true;
                info!(
                    "Loaded {} DB favorite spells for {:?}",
                    favorite_spell_rows.len(),
                    guid
                );
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load favorite spells for {:?}: {}", guid, reason);
            }
            _ => unreachable!("favorite-spell request returned a different row family"),
        }

        // ── C++ Player::_LoadSkills ──
        let mut skill_records =
            std::collections::HashMap::<u16, crate::session::RepresentedPlayerSkillLikeCpp>::new();
        let mut skill_info_by_id = BTreeMap::<u16, wow_data::SkillInfoEntry>::new();
        let mut loaded_skill_records_like_cpp = false;
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Skills {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Skills(rows),
            ) => {
                loaded_skill_records_like_cpp = true;
                for row in rows {
                    if row.skill_id > 0 {
                        skill_records.insert(
                            row.skill_id,
                            crate::session::RepresentedPlayerSkillLikeCpp {
                                skill_id: row.skill_id,
                                step: 0,
                                value: row.value,
                                max: row.max,
                                profession_slot: row.profession_slot,
                                state:
                                    crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged,
                            },
                        );
                    }
                }
                info!(
                    "Loaded {} persisted skill rows for {:?}",
                    skill_records.len(),
                    guid
                );
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load character_skills for {:?}: {}", guid, reason);
            }
            _ => unreachable!("skill request returned a different row family"),
        }

        // C++ `_LoadSkills` rejects forbidden race/class rows, fixes language,
        // mono and level ranges, then `UpdateSkillsForLevel` applies
        // ALWAYS_MAX_VALUE before learning the skill-rewarded spells.
        if let (Some(skill_store), Some(skill_line_store), Some(skill_tiers_store)) = (
            self.skill_store().cloned(),
            self.skill_line_store().cloned(),
            self.skill_tiers_store().cloned(),
        ) {
            let mut normalized_records = HashMap::new();
            let mut persisted_records: Vec<_> = skill_records.into_values().collect();
            persisted_records.sort_by_key(|skill| skill.skill_id);
            for mut skill_record in persisted_records {
                let Some(entry) = skill_store.loaded_skill_info_like_cpp(
                    skill_record.skill_id,
                    race,
                    class,
                    level,
                    skill_record.value,
                    skill_record.max,
                    skill_line_store.as_ref(),
                    skill_tiers_store.as_ref(),
                ) else {
                    warn!(
                        player_guid = guid.counter(),
                        race,
                        class,
                        skill_id = skill_record.skill_id,
                        "Skipping forbidden persisted skill like C++ Player::_LoadSkills"
                    );
                    continue;
                };
                skill_record.step = entry.step;
                skill_record.value = entry.rank;
                skill_record.max = entry.max_rank;
                // Pinned 3.4.3 C++ `_LoadSkills` also inserts a status and
                // initial update-field slot when the persisted value is zero.
                // `HasSkill` then remains false, allowing the later
                // `LearnDefaultSkills` pass to reactivate a default skill.
                normalized_records.insert(skill_record.skill_id, skill_record);
                skill_info_by_id.insert(entry.skill_id, entry);
            }
            skill_records = normalized_records;
            sync_loaded_fist_weapons_with_unarmed_like_cpp(
                &mut skill_records,
                &mut skill_info_by_id,
                level,
            );
        }

        if loaded_skill_records_like_cpp {
            self.replace_player_skill_records_like_cpp(skill_records.clone(), true, false);
        }
        for entry in skill_info_by_id.values() {
            let mut changes = self.skill_rewarded_spell_changes_for_login_like_cpp(
                entry.skill_id,
                entry.rank,
                race,
                class,
                level,
            );
            // C++ `_LoadSkills` runs before `_LoadSpells`, so its RemoveSpell
            // branch cannot remove a character_spell row that has not been
            // loaded yet. The later LearnDefaultSkills pass below runs after
            // `_LoadSpells` and does apply removals.
            changes.remove.clear();
            apply_skill_rewarded_spell_changes_to_login_like_cpp(
                &mut known_spells,
                &mut loaded_spell_side_effect_spells,
                &mut skill_rewarded_dependent_spells,
                &mut skill_rewarded_removed_spells,
                changes,
            );
        }

        // ── Load talents from character_talent ──
        // C++ `Player::LoadFromDB` calls `_LoadTalents` before `_LoadSpells`;
        // `_LoadTalents -> AddTalent` learns the active talent group's spell
        // immediately, so passive talent auras must be present before the
        // login AuraUpdate is built.
        self.reset_represented_talents_like_cpp();
        let mut talent_rows_complete_like_cpp = false;
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Talents {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Talents(rows),
            ) => {
                let mut loaded = 0usize;
                let mut skipped = 0usize;
                for row in rows {
                    if self.load_represented_talent_row_with_spell_side_effects_like_cpp(
                        row.talent_id,
                        row.rank,
                        row.talent_group,
                        &mut known_spells,
                        &mut skill_rewarded_dependent_spells,
                    ) {
                        loaded += 1;
                    } else {
                        skipped += 1;
                    }
                }
                self.mark_represented_talents_loaded_like_cpp();
                talent_rows_complete_like_cpp = true;
                info!(
                    loaded,
                    skipped,
                    player_guid = guid.counter(),
                    "Loaded represented character talents like C++ Player::_LoadTalents"
                );
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    "Failed to load character talents for {:?}: {}",
                    guid, reason
                );
            }
            _ => unreachable!("talent request returned a different row family"),
        }

        let custom_spell_count =
            self.apply_represented_start_all_spells_like_cpp(&mut known_spells);
        if custom_spell_count > 0 {
            info!(
                player_guid = guid.counter(),
                custom_spell_count,
                "Applied represented C++ Player::LearnCustomSpells / CONFIG_START_ALL_SPELLS"
            );
        }
        let mut loaded_dependency_roots = loaded_spell_side_effect_spells.clone();
        loaded_dependency_roots.extend(known_spells.iter().copied());
        loaded_dependency_roots.sort_unstable();
        loaded_dependency_roots.dedup();
        let dependent_spell_count = self.apply_loaded_spell_dependencies_from_roots_like_cpp(
            &loaded_dependency_roots,
            &mut known_spells,
        );
        if dependent_spell_count > 0 {
            info!(
                player_guid = guid.counter(),
                dependent_spell_count,
                "Applied represented C++ Player::_LoadSpells/AddSpell spell_learn_spell dependencies"
            );
        }
        for &spell_id in &known_spells {
            if !loaded_spell_side_effect_spells.contains(&spell_id) {
                loaded_spell_side_effect_spells.push(spell_id);
            }
        }
        let login_proficiencies =
            self.apply_login_known_spell_proficiencies_like_cpp(&loaded_spell_side_effect_spells);
        if login_proficiencies > 0 {
            info!(
                player_guid = guid.counter(),
                login_proficiencies,
                "Applied represented login spell proficiencies like C++ Player::_LoadSpells/AddSpell"
            );
        }
        let login_combat_capabilities = self
            .apply_login_known_spell_combat_capabilities_like_cpp(&loaded_spell_side_effect_spells);
        if login_combat_capabilities > 0 {
            info!(
                player_guid = guid.counter(),
                login_combat_capabilities,
                "Applied represented login parry/block capabilities like C++ Player::_LoadSpells/AddSpell"
            );
        }
        let inactive_lower_rank_count =
            self.deactivate_lower_rank_known_spells_for_send_like_cpp(&mut known_spells);
        if inactive_lower_rank_count > 0 {
            info!(
                player_guid = guid.counter(),
                inactive_lower_rank_count,
                "Deactivated represented lower-rank known spells like C++ Player::AddSpell"
            );
        }

        // Store final known_spells in session for later use (ShowTradeSkill, etc.)
        self.set_known_spells_like_cpp(known_spells.clone());
        self.set_represented_favorite_known_spells_like_cpp(favorite_spell_rows.clone());
        let login_passive_auras = self.apply_login_passive_known_spell_auras_like_cpp();
        if login_passive_auras > 0 {
            info!(
                player_guid = guid.counter(),
                login_passive_auras,
                "Applied represented login passive spell auras like C++ Player::_LoadSpells/AddSpell"
            );
        }
        let prev_rank_passive_auras =
            self.apply_loaded_known_spell_previous_rank_passive_auras_like_cpp(&known_spells);
        if prev_rank_passive_auras > 0 {
            info!(
                player_guid = guid.counter(),
                prev_rank_passive_auras,
                "Applied represented C++ Player::_LoadSpells/AddSpell previous-rank passive auras"
            );
        }
        let promoted_character_mounts =
            self.promote_loaded_character_mount_spells_like_cpp(&known_spells);
        if promoted_character_mounts > 0 {
            info!(
                player_guid = guid.counter(),
                promoted_character_mounts,
                "Promoted loaded character mount spells into the represented account mount collection like C++ Player::_LoadSpells -> AddMount"
            );
        }

        // ── Load glyphs from character_glyphs ──
        // C++ `Player::_LoadGlyphs`: skip invalid talent group/slot and glyph ids
        // missing from GlyphProperties.db2.
        self.reset_represented_glyphs_like_cpp();
        let mut reputation_rows_complete_like_cpp = false;
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Glyphs {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Glyphs(rows),
            ) => {
                let mut loaded = 0usize;
                let mut skipped = 0usize;
                for row in rows {
                    if self.load_represented_glyph_row_like_cpp(
                        row.talent_group,
                        row.glyph_slot,
                        row.glyph_id,
                    ) {
                        loaded += 1;
                    } else {
                        skipped += 1;
                    }
                }
                self.mark_represented_glyphs_loaded_like_cpp();
                info!(
                    loaded,
                    skipped,
                    player_guid = guid.counter(),
                    "Loaded represented character glyphs like C++ Player::_LoadGlyphs"
                );
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load character glyphs for {:?}: {}", guid, reason);
            }
            _ => unreachable!("glyph request returned a different row family"),
        }

        // ── Load action buttons from character_action ──
        // Column types: button=tinyint unsigned, action=int unsigned, type=tinyint unsigned
        let mut action_buttons = [0i64; 180];
        let mut action_count = 0u32;
        self.reset_represented_action_buttons_like_cpp();
        // C++ loads the action-button map for GetActiveTalentGroup(), not always spec 0.
        let (active_spec, trait_config_id) = self.represented_action_button_db_context_like_cpp();
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::ActionButtons {
                    player_guid: guid.counter() as u64,
                    active_spec,
                    trait_config_id,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::ActionButtons(rows),
            ) => {
                for row in rows {
                    if (row.button as usize) < 180 && row.action > 0 {
                        self.record_loaded_action_button_like_cpp(
                            row.button,
                            row.action,
                            row.button_type,
                        );
                        action_buttons[row.button as usize] =
                            wow_packet::packets::misc::UpdateActionButtons::pack_button(
                                row.action as i32,
                                row.button_type,
                            );
                        action_count += 1;
                    }
                }
                self.mark_represented_action_buttons_loaded_like_cpp();
                info!("Loaded {} action buttons for {:?}", action_count, guid);
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load action buttons for {:?}: {}", guid, reason);
            }
            _ => unreachable!("action-button request returned a different row family"),
        }

        // Store current map and character info for VALUES updates + stat recalculation
        self.set_loaded_player_identity_like_cpp(map_id as u16, race, class, level, gender);
        let validated_persisted_transport_login = if saved_transport_guid_low != 0
            && !saved_character_map_is_battleground
        {
            if let Some(transport) = self
                .resolve_persisted_transport_login_like_cpp(
                    saved_transport_guid_low,
                    saved_map_id_for_transport,
                    saved_transport_position,
                )
                .await
            {
                map_id = i32::from(transport.map_id);
                position = transport.world_position;
                self.seed_login_location_zone_area_like_cpp(
                    &mut zone,
                    CharacterLoginLocationLikeCpp {
                        map_id: u32::from(transport.map_id),
                        bind_area_id: None,
                        position: transport.world_position,
                    },
                );
                self.set_player_map_position_like_cpp(transport.map_id, transport.world_position);
                self.set_player_transport_guid_like_cpp(Some(transport.guid));
                self.set_player_transport_position_like_cpp(Some(transport.offset));
                if attached_controller {
                    let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
                }
                Some(transport)
            } else {
                warn!(
                    player_guid = guid.counter(),
                    transport_guid_low = saved_transport_guid_low,
                    offset_x = saved_transport_position.x,
                    offset_y = saved_transport_position.y,
                    offset_z = saved_transport_position.z,
                    offset_o = saved_transport_position.orientation,
                    "invalid persisted transport login state; relocated to homebind like C++ Player::LoadFromDB"
                );
                let homebind_map_id = u16::try_from(login_homebind.map_id)
                    .expect("validated character login homebind map ID");
                map_id = i32::from(homebind_map_id);
                position = login_homebind.position;
                self.seed_login_location_zone_area_like_cpp(&mut zone, login_homebind);
                self.set_player_map_position_like_cpp(homebind_map_id, login_homebind.position);
                self.set_player_transport_guid_like_cpp(None);
                self.set_player_transport_position_like_cpp(None);
                if attached_controller {
                    let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
                }
                None
            }
        } else {
            self.set_player_transport_guid_like_cpp(None);
            self.set_player_transport_position_like_cpp(None);
            None
        };
        self.refresh_next_level_xp();
        // NOTE: known_spells is stored below after DBC merge (see "Merge DBC auto-learned spells")

        // C++ login query set includes CHAR_SEL_CHARACTER_REPUTATION and
        // ReputationMgr::LoadFromDB reinitializes from Faction.db2 before merging rows.
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Reputation {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Reputation(rows),
            ) => {
                let rows: Vec<_> = rows
                    .into_iter()
                    .map(|row| CharacterReputationRowLikeCpp {
                        faction_id: row.faction_id,
                        standing: row.standing,
                        flags: row.flags,
                    })
                    .collect();
                if self.load_character_reputation_rows_like_cpp(rows) {
                    reputation_rows_complete_like_cpp = true;
                    info!("Loaded character reputation rows for {:?}", guid);
                } else {
                    warn!(
                        "Skipped character reputation load for {:?}: missing Faction.db2 store",
                        guid
                    );
                }
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    "Failed to load character reputation for {:?}: {}",
                    guid, reason
                );
            }
            _ => unreachable!("reputation request returned a different row family"),
        }

        // C++ `Player::LoadFromDB` restores `fields.health` after `UpdateAllStats`,
        // clamping it to the recalculated max and preserving zero as corpse state.
        let saved_health = base_row.health;
        let loaded_powers = std::array::from_fn(|index| {
            base_row.powers[index].unwrap_or(0).min(i32::MAX as u32) as i32
        });
        self.set_loaded_player_powers_like_cpp(loaded_powers);
        let saved_power0 = loaded_powers[0];

        // Load active quests from characters DB
        self.load_player_quests().await;

        // C++ calls `LearnDefaultSkills` after `_LoadSkills`, `_LoadSpells`
        // and quest-status loading. Only Availability == 1 rows at or below
        // the player's level are candidates; `LearnDefaultSkill` computes the
        // range-specific value/max and `SetSkill` immediately runs
        // `LearnSkillRewardedSpells` with that real value.
        let persisted_skill_count = skill_info_by_id.len();
        let mut default_skill_entries = Vec::new();
        if let (Some(skill_store), Some(skill_line_store), Some(skill_tiers_store)) = (
            self.skill_store().cloned(),
            self.skill_line_store().cloned(),
            self.skill_tiers_store().cloned(),
        ) {
            for entry in skill_store.default_starting_skill_info_like_cpp(
                race,
                class,
                level,
                skill_line_store.as_ref(),
                skill_tiers_store.as_ref(),
            ) {
                if skill_records
                    .get(&entry.skill_id)
                    .is_some_and(|skill| skill.value > 0)
                {
                    continue;
                }
                if skill_info_by_id.len() >= 256 {
                    break;
                }

                let profession_slot = skill_records
                    .get(&entry.skill_id)
                    .map(|skill| skill.profession_slot)
                    .unwrap_or(-1);
                let state = skill_records
                    .get(&entry.skill_id)
                    .map(|skill| {
                        if skill.state
                            == crate::session::RepresentedPlayerSkillStateLikeCpp::Deleted
                        {
                            crate::session::RepresentedPlayerSkillStateLikeCpp::Changed
                        } else {
                            crate::session::RepresentedPlayerSkillStateLikeCpp::New
                        }
                    })
                    .unwrap_or(crate::session::RepresentedPlayerSkillStateLikeCpp::New);
                skill_records.insert(
                    entry.skill_id,
                    crate::session::RepresentedPlayerSkillLikeCpp {
                        skill_id: entry.skill_id,
                        step: entry.step,
                        value: entry.rank,
                        max: entry.max_rank,
                        profession_slot,
                        state,
                    },
                );
                skill_info_by_id.insert(entry.skill_id, entry);
                default_skill_entries.push(entry);
            }
            self.replace_player_skill_records_like_cpp(skill_records.clone(), true, false);
        }

        for entry in &default_skill_entries {
            let changes = self.skill_rewarded_spell_changes_for_login_like_cpp(
                entry.skill_id,
                entry.rank,
                race,
                class,
                level,
            );
            apply_skill_rewarded_spell_changes_to_login_like_cpp(
                &mut known_spells,
                &mut loaded_spell_side_effect_spells,
                &mut skill_rewarded_dependent_spells,
                &mut skill_rewarded_removed_spells,
                changes,
            );
        }

        // Default skill spells run through C++ AddSpell just like DB-loaded
        // spells. Re-run the idempotent represented side effects so newly
        // learned dependencies, proficiencies, capabilities and passives are
        // present before the initial player CreateObject.
        let (default_dependent_spell_count, loaded_spell_skills_complete_like_cpp) = self
            .apply_loaded_spell_dependency_skills_like_cpp(
                &mut known_spells,
                &mut loaded_spell_side_effect_spells,
            );
        if loaded_skill_records_like_cpp && loaded_spell_skills_complete_like_cpp {
            skill_records = self.player_skill_records_like_cpp().clone();
            let occupied_slots = u16::try_from(skill_records.len()).unwrap_or(u16::MAX);
            if !self
                .set_complete_player_skill_records_like_cpp(skill_records.clone(), occupied_slots)
            {
                warn!(
                    player_guid = guid.counter(),
                    occupied_slots, "Could not authorize represented post-login player skill slots"
                );
            }
        }
        let default_inactive_lower_rank_count =
            self.deactivate_lower_rank_known_spells_for_send_like_cpp(&mut known_spells);
        self.set_known_spells_like_cpp(known_spells.clone());
        self.apply_login_known_spell_proficiencies_like_cpp(&loaded_spell_side_effect_spells);
        self.apply_login_known_spell_combat_capabilities_like_cpp(&loaded_spell_side_effect_spells);
        self.apply_login_passive_known_spell_auras_like_cpp();
        self.apply_loaded_known_spell_previous_rank_passive_auras_like_cpp(&known_spells);
        self.promote_loaded_character_mount_spells_like_cpp(&known_spells);

        // Retain the raw inactive/disabled DB rows and merge spells introduced by
        // represented AddSpell work. Trainer/acquisition decisions need the full
        // logical PlayerSpellMap, not the active-only client projection.
        let canonical_known_spells = self.known_spells_like_cpp().to_vec();
        let canonical_known_spell_ids = canonical_known_spells
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        let mut final_player_spell_rows = loaded_player_spell_rows
            .into_iter()
            .map(|mut row| {
                let dependent = self
                    .represented_dependent_known_spells_like_cpp()
                    .contains(&row.spell_id)
                    || skill_rewarded_dependent_spells.contains(&row.spell_id);
                row.favorite = !dependent && favorite_spell_rows.contains(&row.spell_id);
                row.dependent |= dependent;
                if skill_rewarded_removed_spells.contains(&row.spell_id) {
                    row.active = false;
                    row.disabled = false;
                    row.dependent = false;
                    row.favorite = false;
                    row.state = crate::session::RepresentedPlayerSpellStateLikeCpp::Removed;
                    return (row.spell_id, row);
                }
                if !row.disabled {
                    row.active = canonical_known_spell_ids.contains(&row.spell_id);
                }
                (row.spell_id, row)
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        for spell_id in canonical_known_spells {
            final_player_spell_rows
                .entry(spell_id)
                .and_modify(|row| {
                    row.disabled = false;
                    row.favorite = favorite_spell_rows.contains(&spell_id);
                })
                .or_insert(crate::session::RepresentedPlayerSpellLikeCpp {
                    spell_id,
                    active: true,
                    disabled: false,
                    dependent: self
                        .represented_dependent_known_spells_like_cpp()
                        .contains(&spell_id)
                        || skill_rewarded_dependent_spells.contains(&spell_id),
                    favorite: favorite_spell_rows.contains(&spell_id),
                    state: crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged,
                });
        }
        if loaded_player_spell_rows_complete_like_cpp
            && favorite_spell_rows_complete_like_cpp
            && talent_rows_complete_like_cpp
            && account_mount_rows_complete_like_cpp
            && reputation_rows_complete_like_cpp
        {
            let complete_spell_rows = self.set_complete_represented_player_spell_rows_like_cpp(
                final_player_spell_rows.into_values(),
            );
            if complete_spell_rows {
                self.mark_represented_spell_acquisition_snapshot_complete_like_cpp();
            } else {
                warn!(
                    player_guid = guid.counter(),
                    "Could not authorize represented post-login PlayerSpellMap"
                );
            }
        } else {
            warn!(
                player_guid = guid.counter(),
                "Keeping represented PlayerSpellMap incomplete after incomplete login authority"
            );
        }

        info!(
            player_guid = guid.counter(),
            loaded_skill_count = persisted_skill_count,
            default_skill_count = default_skill_entries.len(),
            default_dependent_spell_count,
            default_inactive_lower_rank_count,
            total_spell_count = known_spells.len(),
            "Applied C++ LearnDefaultSkills and LearnSkillRewardedSpells"
        );

        let skill_info_tuples: Vec<(u16, u16, u16, u16, u16, i16, u16)> = skill_info_by_id
            .values()
            .map(|entry| {
                (
                    entry.skill_id,
                    entry.step,
                    entry.rank,
                    entry.starting_rank,
                    entry.max_rank,
                    entry.temp_bonus,
                    entry.perm_bonus,
                )
            })
            .collect();

        self.load_completed_achievements_like_cpp().await;
        self.load_instance_time_restrictions_like_cpp().await;
        self.load_player_account_data_like_cpp(guid).await;
        {
            self.set_player_aura_authority_complete_like_cpp(false);
            let mut aura_rows = Vec::new();
            let mut aura_rows_complete = false;
            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuras {
                        player_guid: guid.counter() as u64,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::CharacterAuras(rows),
                ) => {
                    aura_rows_complete = true;
                    aura_rows.extend(rows.into_iter().map(|row| {
                        crate::session::CharacterAuraRowLikeCpp {
                            caster_guid: object_guid_from_db_binary_like_cpp(
                                row.caster_guid_binary,
                            ),
                            spell_id: row.spell_id,
                            effect_mask: row.effect_mask,
                            recalculate_mask: row.recalculate_mask,
                            difficulty: row.difficulty,
                            stack_count: row.stack_count,
                            max_duration_ms: row.max_duration_ms,
                            remain_time_ms: row.remain_time_ms,
                            remain_charges: row.remain_charges,
                        }
                    }));
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!("Failed to load character auras for {:?}: {}", guid, reason)
                }
                _ => unreachable!("character-aura request returned a different row family"),
            }

            let mut aura_effect_rows = Vec::new();
            let mut aura_effect_rows_complete = false;
            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuraEffects {
                        player_guid: guid.counter() as u64,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::CharacterAuraEffects(rows),
                ) => {
                    aura_effect_rows_complete = true;
                    aura_effect_rows.extend(rows.into_iter().map(|row| {
                        crate::session::CharacterAuraEffectRowLikeCpp {
                            caster_guid: object_guid_from_db_binary_like_cpp(
                                row.caster_guid_binary,
                            ),
                            spell_id: row.spell_id,
                            effect_mask: row.effect_mask,
                            effect_index: row.effect_index,
                            amount: row.amount,
                            base_amount: row.base_amount,
                        }
                    }));
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!(
                        "Failed to load character aura effects for {:?}: {}",
                        guid, reason
                    )
                }
                _ => unreachable!("character-aura-effect request returned a different row family"),
            }
            let loaded_character_auras =
                self.load_represented_character_auras_like_cpp(aura_rows, aura_effect_rows, 0);
            self.set_player_aura_authority_complete_like_cpp(
                aura_rows_complete && aura_effect_rows_complete,
            );
            info!(
                loaded_character_auras,
                player_guid = guid.counter(),
                "Loaded represented character auras like C++ Player::_LoadAuras"
            );
        }
        // C++ `Player::LoadFromDB` runs `_LoadAuras` before `_LoadInventory`,
        // whose final `_ApplyAllItemMods` pass applies, for each equipment slot,
        // the item-set effect, regular equip spell, and enchantments before
        // advancing to the next item. Keep the replay here so loaded and
        // item-provided auras receive the same slot order.
        let initial_item_mods =
            self.apply_initial_loaded_item_mods_like_cpp(&loaded_equipped_item_guids);
        if initial_item_mods.item_set_auras > 0 {
            info!(
                player_guid = guid.counter(),
                initial_item_set_auras = initial_item_mods.item_set_auras,
                "Applied represented initial item-set auras like C++ Player::_ApplyAllItemMods"
            );
        }
        if initial_item_mods.item_equip_auras > 0 {
            info!(
                player_guid = guid.counter(),
                initial_item_equip_auras = initial_item_mods.item_equip_auras,
                "Applied represented initial item equip auras like C++ Player::_ApplyAllItemMods"
            );
        }
        let loaded_enchantment_updates = initial_item_mods.enchantments;

        // C++ defers `UpdateAllStats` and the saved-health clamp until after
        // `_LoadAuras` and `_LoadInventory` have applied every aura and item
        // modifier. Build the initial self snapshot at the same boundary.
        let (combat, base_mana_like_cpp, current_power0) = if let Some(combat) =
            self.player_login_combat_stats_like_cpp(race, class, level, saved_health, saved_power0)
        {
            combat
        } else {
            warn!(
                "Missing C++ player stats or ChrClasses coefficients for race={race} class={class} level={level}; using fallback"
            );
            let (h, m) = default_health_mana(class);
            let combat = PlayerCombatStats {
                health: restored_saved_health_like_cpp(saved_health, h as i64),
                max_health: h as i64,
                base_mana: m as i32,
                max_mana: m as i64,
                ..PlayerCombatStats::default()
            };
            let max_power0 = primary_max_power_for_class_like_cpp(class, combat.max_mana);
            (combat, m as i32, saved_power0.clamp(0, max_power0.max(0)))
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

        let login_known_spells = self.login_known_spells_after_account_collections_like_cpp();
        let login_favorite_spells =
            favorite_known_spells_for_send_like_cpp(&login_known_spells, &favorite_spell_rows);
        let (spell_history_entries, spell_charge_entries) = self
            .load_character_spell_history_packets_like_cpp(guid)
            .await;
        // Persist the login snapshot so the before-add init helper can re-send spell
        // history/charges on far teleport without a DB round trip. #NEXT.R8.ENTITIES.1229.
        self.record_login_spell_history_packets_like_cpp(
            spell_history_entries.clone(),
            spell_charge_entries.clone(),
        );

        if !self
            .send_login_sequence(
                guid,
                race,
                class,
                gender,
                level,
                display_id,
                &position,
                map_id,
                zone,
                login_homebind,
                validated_persisted_transport_login,
                visible_items,
                inv_slots,
                item_creates,
                combat,
                current_power0,
                base_mana_like_cpp,
                login_known_spells,
                login_favorite_spells,
                spell_history_entries,
                spell_charge_entries,
                action_buttons,
                skill_info_tuples,
                self.account_mount_rows_like_cpp(),
            )
            .await
        {
            self.abort_partial_login_sequence_like_cpp();
            return;
        }
        self.send_item_time_update_plans(&loaded_item_time_updates);
        self.send_item_enchant_time_update_plans(guid, &loaded_non_equipped_enchantment_updates);
        self.send_loaded_equipped_item_enchantment_updates_like_cpp(&loaded_enchantment_updates);
        self.apply_represented_login_spell_reset_if_needed_like_cpp();
        self.apply_represented_login_talent_reset_if_needed_like_cpp();
        let applied_first_login_like_cpp =
            self.apply_represented_first_login_flag_if_needed_like_cpp();
        if applied_first_login_like_cpp {
            self.apply_represented_first_login_cast_spells_like_cpp()
                .await;
            self.apply_represented_first_login_explored_zones_like_cpp();
            self.apply_represented_first_login_reputation_like_cpp();
        }

        // C++ processes reset-at-login and first-login casts after the initial
        // map packet sequence. Publish only after those normal mutations. The
        // first-login cast closure is not represented losslessly, so that
        // Player remains fail-closed for this entire session.
        if applied_first_login_like_cpp {
            self.tombstone_player_spell_hit_aura_authority_like_cpp();
        } else {
            let _ = self.sync_player_spell_hit_aura_authority_to_canonical_like_cpp();
        }

        // Mark online in DB. This remains best-effort at the existing Rust
        // sequencing point; #432 changes ownership, not login timing.
        let _ = player_lifecycle_port
            .mark_player_online_like_cpp(wow_persistence::PlayerOnlineMarkRequestLikeCpp {
                player_guid: guid.counter() as u32,
            })
            .await;

        // C++ `sScriptMgr->OnPlayerLogin(pCurrChar, firstLogin)`
        // (`CharacterHandler.cpp:1452`), after the completed login and after
        // the login criteria update. Trusted linked modules observe here.
        self.dispatch_module_player_login_like_cpp(first_login);
    }

    /// Build the self CreateObject combat snapshot after C++ login has loaded
    /// persisted auras and applied all equipped-item modifiers.
    pub(super) fn player_login_combat_stats_like_cpp(
        &self,
        race: u8,
        class: u8,
        level: u8,
        saved_health: Option<u32>,
        saved_power0: i32,
    ) -> Option<(PlayerCombatStats, i32, i32)> {
        let gear = self.represented_player_gear_stats_like_cpp(true);
        let projection = self.player_stat_system_projection_like_cpp(race, class, level, &gear)?;
        let ap_f = projection.total_attack_power as f32;
        let base_dmg = ap_f / 14.0 * 2.0;
        let min_damage = (base_dmg + 1.0).max(1.0);
        let max_damage = min_damage + 1.0;
        let ranged_ap_f = projection.total_ranged_attack_power as f32;
        let (min_ranged_damage, max_ranged_damage) = if ranged_ap_f > 0.0 {
            let damage = ranged_ap_f / 14.0 * 2.8;
            ((damage + 1.0).max(1.0), damage + 3.0)
        } else {
            (0.0, 0.0)
        };

        let combat = PlayerCombatStats {
            health: restored_saved_health_like_cpp(saved_health, projection.max_health),
            max_health: projection.max_health,
            stats: projection.stats,
            stat_pos_buff: projection.stat_pos_buff,
            stat_neg_buff: projection.stat_neg_buff,
            base_armor: projection.armor,
            base_mana: projection.base_mana,
            max_mana: projection.max_mana,
            attack_power: projection.attack_power,
            attack_power_mod_pos: projection.attack_power_mod_pos,
            ranged_attack_power: projection.ranged_attack_power,
            ranged_attack_power_mod_pos: projection.ranged_attack_power_mod_pos,
            min_damage,
            max_damage,
            min_ranged_damage,
            max_ranged_damage,
            block_pct: projection.block_pct,
            dodge_pct: projection.dodge_pct,
            dodge_from_attr: projection.dodge_from_attr,
            parry_pct: projection.parry_pct,
            parry_from_attr: projection.parry_from_attr,
            crit_pct: projection.crit_pct,
            ranged_crit_pct: projection.ranged_crit_pct,
            offhand_crit_pct: projection.offhand_crit_pct,
            spell_crit_pct: projection.spell_crit_pct,
            combat_ratings: gear.combat_ratings,
            spell_power: gear.spell_power,
        };
        let max_power0 = primary_max_power_for_class_like_cpp(class, combat.max_mana);
        Some((
            combat,
            projection.base_mana,
            saved_power0.clamp(0, max_power0.max(0)),
        ))
    }

    /// C++ `WorldSession::HandlePlayerLogin` packet prelude through
    /// `BattlePetMgr::SendJournalLockStatus`.
    pub(super) async fn send_handle_player_login_packets_like_cpp(
        &mut self,
        guid: ObjectGuid,
        position: &Position,
        map_id: i32,
        account_mounts: &[AccountMount],
        motd: &str,
    ) -> bool {
        // C++ `Player::LoadFromDB -> CollectionMgr::LoadMounts -> AddMount`
        // publishes one partial update for every usable account mount before
        // `HandlePlayerLogin` begins its explicit packet burst.
        for mount in account_mounts {
            self.send_packet(&AccountMountUpdate::partial(vec![*mount]));
        }
        // `LoadFromDB` may already have published proficiency/aura packets on
        // the instance writer even when this account has no partial mounts.
        // Drain that complete prefix before starting the realm-routed login
        // burst so the two physical sockets retain C++ call order.
        if !self
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
        {
            return false;
        }

        // C++ resends both account-scoped datasets here even though they were
        // already sent by `InitializeSessionCallback` on the glue screen.
        self.send_packet_realm(
            &self.account_data_times_like_cpp(ObjectGuid::EMPTY, GLOBAL_CACHE_MASK_LIKE_CPP),
        );
        self.send_packet_realm(&self.tutorial_flags_packet_like_cpp());

        self.send_packet_realm(&self.represented_dungeon_difficulty_packet_like_cpp());
        if !self
            .wait_for_realm_send_before_instance_update_like_cpp()
            .await
        {
            return false;
        }
        self.send_packet(&LoginVerifyWorld {
            map_id,
            position: *position,
            reason: 0,
        });
        if !self
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
        {
            return false;
        }
        self.send_packet_realm(
            &self.account_data_times_like_cpp(guid, ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP),
        );
        self.send_packet_realm(&self.feature_system_status_like_cpp());

        for motd_line in motd_lines_like_cpp(motd) {
            self.send_packet_realm(&ChatServerMessage {
                message_id: 3,
                string_param: motd_line,
            });
        }

        self.send_packet_realm(&SetTimeZoneInformation::utc());

        // Issue #161: converge interrupted battle-pet trainer purchases
        // before the journal lock and before the client can interact; any
        // recovery publication lands inside this login burst. The recovery
        // writes instance-socket packets between Realm-socket neighbours, so
        // it is bracketed by the same cross-socket ordering fences the rest
        // of the burst already uses.
        if !self
            .wait_for_realm_send_before_instance_update_like_cpp()
            .await
        {
            return false;
        }
        self.recover_battle_pet_trainer_purchases_like_cpp().await;
        if !self
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
        {
            return false;
        }

        // C++ sends the journal lock before
        // `Player::SendInitialPacketsBeforeAddToMap`.
        self.send_battle_pet_journal_lock_status_like_cpp().await;
        self.wait_for_realm_send_before_instance_update_like_cpp()
            .await
    }
}
