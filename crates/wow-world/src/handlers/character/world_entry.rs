// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Login, world entry, logout and the client-state handshake.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use wow_database::{CharStatements, SqlTransaction};

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

        // Load character from DB and send login sequence
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => {
                warn!("No character database for continue login");
                self.release_character_login_claim_like_cpp();
                return;
            }
        };

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
            let mut bg_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_BGDATA);
            bg_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&bg_stmt).await {
                Ok(bg_result) if !bg_result.is_empty() => {
                    Some(CharacterBattlegroundLoginDataLikeCpp {
                        entry_point: CharacterLoginLocationLikeCpp {
                            map_id: u32::from(bg_result.try_read::<u16>(6).unwrap_or(u16::MAX)),
                            bind_area_id: None,
                            position: Position::new(
                                bg_result.try_read::<f32>(2).unwrap_or(f32::NAN),
                                bg_result.try_read::<f32>(3).unwrap_or(f32::NAN),
                                bg_result.try_read::<f32>(4).unwrap_or(f32::NAN),
                                bg_result.try_read::<f32>(5).unwrap_or(f32::NAN),
                            ),
                        },
                    })
                }
                Ok(_) => None,
                Err(error) => {
                    warn!(
                        player_guid = guid.counter(),
                        %error,
                        "failed to load character_battleground_data like C++ Player::_LoadBGData"
                    );
                    None
                }
            }
        } else {
            None
        };
        let loaded_login_homebind = {
            let mut homebind_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_HOMEBIND);
            homebind_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&homebind_stmt).await {
                Ok(homebind_result) if !homebind_result.is_empty() => {
                    Some(CharacterLoginLocationLikeCpp {
                        map_id: u32::from(homebind_result.try_read::<u16>(0).unwrap_or(u16::MAX)),
                        bind_area_id: Some(u32::from(
                            homebind_result.try_read::<u16>(1).unwrap_or(0),
                        )),
                        position: Position::new(
                            homebind_result.try_read::<f32>(2).unwrap_or(f32::NAN),
                            homebind_result.try_read::<f32>(3).unwrap_or(f32::NAN),
                            homebind_result.try_read::<f32>(4).unwrap_or(f32::NAN),
                            homebind_result.try_read::<f32>(5).unwrap_or(f32::NAN),
                        ),
                    })
                }
                Ok(_) => None,
                Err(error) => {
                    warn!(
                        player_guid = guid.counter(),
                        %error,
                        "failed to load character homebind like C++ Player::_LoadHomeBind"
                    );
                    self.kick("WorldSession::HandlePlayerLogin Player::_LoadHomeBind query failed");
                    return;
                }
            }
        };
        let loaded_guild_id_like_cpp = {
            let mut guild_stmt = char_db.prepare(CharStatements::SEL_GUILD_MEMBER);
            guild_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&guild_stmt).await {
                Ok(guild_result) if guild_result.is_empty() => Some(0),
                Ok(mut guild_result) => {
                    let guild_id = guild_result.try_read::<u64>(0);
                    if guild_result.next_row() {
                        warn!(
                            player_guid = guid.counter(),
                            "Keeping guild membership authority incomplete: duplicate rows"
                        );
                        None
                    } else if guild_id.is_none() {
                        warn!(
                            player_guid = guid.counter(),
                            "Keeping guild membership authority incomplete: malformed row"
                        );
                        None
                    } else {
                        guild_id
                    }
                }
                Err(error) => {
                    warn!(
                        player_guid = guid.counter(),
                        %error,
                        "Failed to load guild membership for player login"
                    );
                    None
                }
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
            let mut delete_pet_spells =
                char_db.prepare(CharStatements::DEL_ALL_PET_SPELLS_BY_OWNER);
            delete_pet_spells.set_u64(0, guid.counter() as u64);
            if let Err(error) = char_db.execute(&delete_pet_spells).await {
                warn!(
                    player_guid = guid.counter(),
                    %error,
                    "failed to apply represented AT_LOGIN_RESET_PET_TALENTS pet_spell delete like C++"
                );
            }

            let mut reset_pet_specs = char_db.prepare(CharStatements::UPD_PET_SPECS_BY_OWNER);
            reset_pet_specs.set_u64(0, guid.counter() as u64);
            if let Err(error) = char_db.execute(&reset_pet_specs).await {
                warn!(
                    player_guid = guid.counter(),
                    %error,
                    "failed to apply represented AT_LOGIN_RESET_PET_TALENTS pet specialization reset like C++"
                );
            }
        }
        self.begin_represented_character_pet_authority_load_like_cpp();
        {
            let mut pets_stmt = char_db.prepare(CharStatements::SEL_CHAR_PETS);
            pets_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&pets_stmt).await {
                Ok(mut pets_result) => {
                    let mut rows = Vec::new();
                    if !pets_result.is_empty() {
                        loop {
                            rows.push(CharacterPetStableRowLikeCpp {
                                pet_number: pets_result.try_read::<u32>(0).unwrap_or(0),
                                creature_id: pets_result.try_read::<u32>(1).unwrap_or(0),
                                display_id: pets_result.try_read::<u32>(2).unwrap_or(0),
                                level: pets_result.try_read::<u8>(3).unwrap_or(1),
                                experience: pets_result.try_read::<u32>(4).unwrap_or(0),
                                react_state: pets_result.try_read::<u8>(5).unwrap_or(0),
                                slot: pets_result.try_read::<i16>(6).unwrap_or(-1),
                                name: pets_result.read_string(7),
                                was_renamed: pets_result.try_read::<bool>(8).unwrap_or(false),
                                health: pets_result.try_read::<u32>(9).unwrap_or(1),
                                mana: pets_result.try_read::<u32>(10).unwrap_or(0),
                                action_bar: pets_result.try_read::<String>(11).unwrap_or_default(),
                                last_save_time: pets_result.try_read::<u32>(12).unwrap_or(0),
                                created_by_spell_id: pets_result.try_read::<u32>(13).unwrap_or(0),
                                pet_type: pets_result.try_read::<u8>(14).unwrap_or(0),
                                specialization_id: pets_result.try_read::<u16>(15).unwrap_or(0),
                            });
                            if !pets_result.next_row() {
                                break;
                            }
                        }
                    }
                    let loaded =
                        self.load_represented_pet_stable_rows_like_cpp(summoned_pet_number, rows);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        loaded,
                        "loaded represented character_pet stable rows like C++"
                    );
                }
                Err(error) => {
                    warn!(
                        player_guid = guid.counter(),
                        %error,
                        "failed to load represented character_pet rows"
                    );
                }
            }
        }
        if summoned_pet_number != 0 {
            let mut pet_aura_stmt = char_db.prepare(CharStatements::SEL_PET_AURA);
            pet_aura_stmt.set_u32(0, summoned_pet_number);
            match char_db.query(&pet_aura_stmt).await {
                Ok(mut aura_result) => {
                    let mut rows = Vec::new();
                    if !aura_result.is_empty() {
                        loop {
                            rows.push(CharacterPetAuraRowLikeCpp {
                                caster_guid: object_guid_from_db_binary_like_cpp(
                                    aura_result.try_read::<Vec<u8>>(0).unwrap_or_default(),
                                ),
                                spell_id: aura_result.try_read::<u32>(1).unwrap_or(0),
                                effect_mask: aura_result.try_read::<u32>(2).unwrap_or(0),
                                recalculate_mask: aura_result.try_read::<u32>(3).unwrap_or(0),
                                difficulty: aura_result.try_read::<u8>(4).unwrap_or(0),
                                stack_count: aura_result.try_read::<u8>(5).unwrap_or(0),
                                max_duration_ms: aura_result.try_read::<i32>(6).unwrap_or(0),
                                remain_time_ms: aura_result.try_read::<i32>(7).unwrap_or(0),
                                remain_charges: aura_result.try_read::<u8>(8).unwrap_or(0),
                            });
                            if !aura_result.next_row() {
                                break;
                            }
                        }
                    }
                    let loaded =
                        self.load_represented_pet_aura_rows_like_cpp(summoned_pet_number, rows);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number, loaded, "loaded represented pet_aura rows like C++"
                    );
                }
                Err(error) => {
                    warn!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        %error,
                        "failed to load represented pet_aura rows"
                    );
                }
            }

            let mut pet_aura_effect_stmt = char_db.prepare(CharStatements::SEL_PET_AURA_EFFECT);
            pet_aura_effect_stmt.set_u32(0, summoned_pet_number);
            match char_db.query(&pet_aura_effect_stmt).await {
                Ok(mut aura_effect_result) => {
                    let mut rows = Vec::new();
                    if !aura_effect_result.is_empty() {
                        loop {
                            rows.push(CharacterPetAuraEffectRowLikeCpp {
                                caster_guid: object_guid_from_db_binary_like_cpp(
                                    aura_effect_result
                                        .try_read::<Vec<u8>>(0)
                                        .unwrap_or_default(),
                                ),
                                spell_id: aura_effect_result.try_read::<u32>(1).unwrap_or(0),
                                effect_mask: aura_effect_result.try_read::<u32>(2).unwrap_or(0),
                                effect_index: aura_effect_result.try_read::<u8>(3).unwrap_or(0),
                                amount: aura_effect_result.try_read::<i32>(4).unwrap_or(0),
                                base_amount: aura_effect_result.try_read::<i32>(5).unwrap_or(0),
                            });
                            if !aura_effect_result.next_row() {
                                break;
                            }
                        }
                    }
                    let loaded = self
                        .load_represented_pet_aura_effect_rows_like_cpp(summoned_pet_number, rows);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        loaded,
                        "loaded represented pet_aura_effect rows like C++"
                    );
                }
                Err(error) => {
                    warn!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        %error,
                        "failed to load represented pet_aura_effect rows"
                    );
                }
            }

            let mut pet_spell_stmt = char_db.prepare(CharStatements::SEL_PET_SPELL);
            pet_spell_stmt.set_u32(0, summoned_pet_number);
            match char_db.query(&pet_spell_stmt).await {
                Ok(mut spells_result) => {
                    let mut rows = Vec::new();
                    if !spells_result.is_empty() {
                        loop {
                            rows.push(CharacterPetSpellRowLikeCpp {
                                spell_id: spells_result.try_read::<u32>(0).unwrap_or(0),
                                active: spells_result.try_read::<u8>(1).unwrap_or(0),
                            });
                            if !spells_result.next_row() {
                                break;
                            }
                        }
                    }
                    let loaded =
                        self.load_represented_pet_spell_rows_like_cpp(summoned_pet_number, rows);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number, loaded, "loaded represented pet_spell rows like C++"
                    );
                }
                Err(error) => {
                    warn!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        %error,
                        "failed to load represented pet_spell rows"
                    );
                }
            }

            let mut pet_cooldown_stmt = char_db.prepare(CharStatements::SEL_PET_SPELL_COOLDOWN);
            pet_cooldown_stmt.set_u32(0, summoned_pet_number);
            match char_db.query(&pet_cooldown_stmt).await {
                Ok(mut cooldowns_result) => {
                    let mut rows = Vec::new();
                    if !cooldowns_result.is_empty() {
                        loop {
                            rows.push(CharacterPetSpellCooldownRowLikeCpp {
                                spell_id: cooldowns_result.try_read::<u32>(0).unwrap_or(0),
                                cooldown_end_unix_secs: cooldowns_result
                                    .try_read::<i64>(1)
                                    .unwrap_or(0),
                                category_id: cooldowns_result.try_read::<u32>(2).unwrap_or(0),
                                category_end_unix_secs: cooldowns_result
                                    .try_read::<i64>(3)
                                    .unwrap_or(0),
                            });
                            if !cooldowns_result.next_row() {
                                break;
                            }
                        }
                    }
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
                Err(error) => {
                    warn!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        %error,
                        "failed to load represented pet_spell_cooldown rows"
                    );
                }
            }

            let mut pet_charges_stmt = char_db.prepare(CharStatements::SEL_PET_SPELL_CHARGES);
            pet_charges_stmt.set_u32(0, summoned_pet_number);
            match char_db.query(&pet_charges_stmt).await {
                Ok(mut charges_result) => {
                    let mut rows = Vec::new();
                    if !charges_result.is_empty() {
                        loop {
                            rows.push(CharacterPetSpellChargeRowLikeCpp {
                                category_id: charges_result.try_read::<u32>(0).unwrap_or(0),
                                recharge_start_unix_secs: charges_result
                                    .try_read::<i64>(1)
                                    .unwrap_or(0),
                                recharge_end_unix_secs: charges_result
                                    .try_read::<i64>(2)
                                    .unwrap_or(0),
                            });
                            if !charges_result.next_row() {
                                break;
                            }
                        }
                    }
                    let loaded = self
                        .load_represented_pet_spell_charge_rows_like_cpp(summoned_pet_number, rows);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        loaded,
                        "loaded represented pet_spell_charges rows like C++"
                    );
                }
                Err(error) => {
                    warn!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        %error,
                        "failed to load represented pet_spell_charges rows"
                    );
                }
            }

            let mut pet_declined_stmt = char_db.prepare(CharStatements::SEL_PET_DECLINED_NAME);
            pet_declined_stmt.set_u64(0, guid.counter() as u64);
            pet_declined_stmt.set_u32(1, summoned_pet_number);
            match char_db.query(&pet_declined_stmt).await {
                Ok(declined_result) => {
                    let row = if declined_result.is_empty() {
                        None
                    } else {
                        Some(CharacterPetDeclinedNamesRowLikeCpp {
                            names: [
                                declined_result.read_string(0),
                                declined_result.read_string(1),
                                declined_result.read_string(2),
                                declined_result.read_string(3),
                                declined_result.read_string(4),
                            ],
                        })
                    };
                    let loaded =
                        self.load_represented_pet_declined_names_like_cpp(summoned_pet_number, row);
                    trace!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        loaded,
                        "loaded represented character_pet_declinedname row like C++"
                    );
                }
                Err(error) => {
                    warn!(
                        player_guid = guid.counter(),
                        summoned_pet_number,
                        %error,
                        "failed to load represented character_pet_declinedname row"
                    );
                }
            }
        }
        if (self.represented_at_login_flags_like_cpp() & AT_LOGIN_RESET_PET_TALENTS_LIKE_CPP) != 0 {
            self.apply_represented_login_pet_talent_reset_like_cpp();
        }
        self.group_guid = None;
        {
            let mut group_stmt = char_db.prepare(CharStatements::SEL_GROUP_MEMBER);
            group_stmt.set_u32(0, guid.counter() as u32);
            match char_db.query(&group_stmt).await {
                Ok(group_result) => {
                    if !group_result.is_empty() {
                        let db_store_id: u32 = group_result.read(0);
                        let _ = self.load_represented_group_by_db_store_id_like_cpp(db_store_id);
                        let _ = self.reset_group_update_sequence_if_needed_like_cpp();
                    }
                }
                Err(error) => {
                    warn!(
                        player_guid = guid.counter(),
                        %error,
                        "failed to load represented group membership"
                    );
                }
            }
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
            let mut eq_stmt = char_db.prepare(CharStatements::SEL_CHAR_EQUIPMENT);
            eq_stmt.set_u64(0, guid.counter() as u64);
            let mut refund_cleanup_tx = SqlTransaction::new();
            match char_db.query(&eq_stmt).await {
                Ok(mut eq_result) => {
                    let equipment_inventory_source_is_proven_empty = eq_result.is_empty();
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
                            let item_expiration = eq_result.try_read::<u32>(22).unwrap_or(0);
                            let item_spell_charges =
                                eq_result.try_read::<String>(23).unwrap_or_default();
                            let item_enchantments =
                                eq_result.try_read::<String>(8).unwrap_or_default();
                            let item_enchantment_values =
                                loaded_item_enchantments_like_cpp(&item_enchantments);
                            let random_properties = loaded_item_random_properties_like_cpp(
                                eq_result.try_read::<i32>(9).unwrap_or(0),
                                eq_result.try_read::<i32>(10).unwrap_or(0),
                                self.item_random_properties_store()
                                    .map(|store| store.as_ref()),
                                self.item_random_suffix_store().map(|store| store.as_ref()),
                            );
                            let random_properties_id =
                                random_properties.map(|random| random.id).unwrap_or(0);
                            let random_properties_seed =
                                random_properties.map(|random| random.seed).unwrap_or(0);
                            let socketed_gems = loaded_socketed_gems_like_cpp([
                                (
                                    eq_result.try_read::<i32>(11).unwrap_or(0),
                                    eq_result.try_read::<String>(12).unwrap_or_default(),
                                    eq_result.try_read::<u8>(13).unwrap_or(0),
                                ),
                                (
                                    eq_result.try_read::<i32>(14).unwrap_or(0),
                                    eq_result.try_read::<String>(15).unwrap_or_default(),
                                    eq_result.try_read::<u8>(16).unwrap_or(0),
                                ),
                                (
                                    eq_result.try_read::<i32>(17).unwrap_or(0),
                                    eq_result.try_read::<String>(18).unwrap_or_default(),
                                    eq_result.try_read::<u8>(19).unwrap_or(0),
                                ),
                            ]);
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
                            let refund_decision = loaded_item_refund_decision(
                                item_flags,
                                item_played_time,
                                eq_result.try_read::<u64>(20),
                                eq_result.try_read::<u16>(21),
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
                                let is_bag_container =
                                    inventory_type == Some(InventoryType::Bag as u8);
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
                                item_object.replace_all_item_flags(
                                    ItemFieldFlags::from_bits_retain(stored_flags),
                                );
                                if expiration_needs_save {
                                    let mut update_item_on_load =
                                        char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_ON_LOAD);
                                    update_item_on_load.set_u32(0, item_object.data().expiration);
                                    update_item_on_load.set_u32(1, item_object.item_flags_bits());
                                    update_item_on_load.set_u32(2, item_object.data().durability);
                                    update_item_on_load.set_u64(3, item_db_guid);
                                    refund_cleanup_tx.append(update_item_on_load);
                                }
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
                            if !eq_result.next_row() {
                                break;
                            }
                        }
                    }
                    if equipment_inventory_source_is_proven_empty {
                        self.complete_player_equipment_inventory_authority_load_like_cpp();
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
                let mut bag_load_fix_tx = SqlTransaction::new();
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
                                let item_expiration = bag_result.try_read::<u32>(23).unwrap_or(0);
                                let item_spell_charges =
                                    bag_result.try_read::<String>(24).unwrap_or_default();
                                let item_enchantments =
                                    bag_result.try_read::<String>(9).unwrap_or_default();
                                let item_enchantment_values =
                                    loaded_item_enchantments_like_cpp(&item_enchantments);
                                let random_properties = loaded_item_random_properties_like_cpp(
                                    bag_result.try_read::<i32>(10).unwrap_or(0),
                                    bag_result.try_read::<i32>(11).unwrap_or(0),
                                    self.item_random_properties_store()
                                        .map(|store| store.as_ref()),
                                    self.item_random_suffix_store().map(|store| store.as_ref()),
                                );
                                let random_properties_id =
                                    random_properties.map(|random| random.id).unwrap_or(0);
                                let random_properties_seed =
                                    random_properties.map(|random| random.seed).unwrap_or(0);
                                let socketed_gems = loaded_socketed_gems_like_cpp([
                                    (
                                        bag_result.try_read::<i32>(12).unwrap_or(0),
                                        bag_result.try_read::<String>(13).unwrap_or_default(),
                                        bag_result.try_read::<u8>(14).unwrap_or(0),
                                    ),
                                    (
                                        bag_result.try_read::<i32>(15).unwrap_or(0),
                                        bag_result.try_read::<String>(16).unwrap_or_default(),
                                        bag_result.try_read::<u8>(17).unwrap_or(0),
                                    ),
                                    (
                                        bag_result.try_read::<i32>(18).unwrap_or(0),
                                        bag_result.try_read::<String>(19).unwrap_or_default(),
                                        bag_result.try_read::<u8>(20).unwrap_or(0),
                                    ),
                                ]);
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
                                            .and_then(|store| {
                                                store.duration_in_inventory(item_entry)
                                            })
                                            .unwrap_or(0);
                                        let effect_count =
                                            self.item_effect_count_like_cpp(item_entry);
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
                                            let mut update_item_on_load = char_db
                                                .prepare(CharStatements::UPD_ITEM_INSTANCE_ON_LOAD);
                                            update_item_on_load
                                                .set_u32(0, item_object.data().expiration);
                                            update_item_on_load
                                                .set_u32(1, item_object.item_flags_bits());
                                            update_item_on_load
                                                .set_u32(2, item_object.data().durability);
                                            update_item_on_load.set_u64(3, item_db_guid);
                                            bag_load_fix_tx.append(update_item_on_load);
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
                if !bag_load_fix_tx.is_empty()
                    && let Err(e) = char_db.commit_transaction(bag_load_fix_tx).await
                {
                    warn!(
                        "Failed to normalize loaded bag item state for {:?}: {}",
                        guid, e
                    );
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
            let mut void_stmt = char_db.prepare(CharStatements::SEL_CHAR_VOID_STORAGE);
            void_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&void_stmt).await {
                Ok(mut void_result) => {
                    if !void_result.is_empty() {
                        loop {
                            let item_id: u64 = void_result.try_read(0).unwrap_or(0);
                            let item_entry: u32 = void_result.try_read(1).unwrap_or(0);
                            let slot: u8 = void_result.try_read(2).unwrap_or(u8::MAX);
                            let creator_counter: u64 = void_result.try_read(3).unwrap_or(0);
                            let fixed_scaling_level: u32 = void_result.try_read(4).unwrap_or(0);
                            let random_properties_id: i32 = void_result.try_read(5).unwrap_or(0);
                            let random_properties_seed: i32 = void_result.try_read(6).unwrap_or(0);
                            let selected_context_column: u8 = void_result.try_read(7).unwrap_or(0);
                            let context = void_storage_login_context_like_cpp(
                                random_properties_id,
                                selected_context_column,
                            );
                            let creator_guid = if creator_counter == 0 {
                                ObjectGuid::EMPTY
                            } else {
                                ObjectGuid::create_player(realm_id, creator_counter as i64)
                            };
                            let loaded = self.load_represented_void_storage_row_like_cpp(
                                slot,
                                RepresentedVoidStorageItemLikeCpp {
                                    item_id,
                                    item_entry,
                                    creator_guid,
                                    fixed_scaling_level,
                                    random_properties_id,
                                    random_properties_seed,
                                    context,
                                },
                            );
                            if !loaded {
                                warn!(
                                    player_guid = guid.counter(),
                                    item_id,
                                    item_entry,
                                    slot,
                                    "Player::_LoadVoidStorage skipped an invalid row like C++"
                                );
                            }
                            if !void_result.next_row() {
                                break;
                            }
                        }
                    }
                    self.mark_represented_void_storage_loaded_like_cpp();
                }
                Err(e) => {
                    warn!("Failed to load void storage for {:?}: {}", guid, e);
                }
            }
        }

        // ── Load equipment sets / transmog outfits ──
        // C++ `Player::_LoadEquipmentSets` and `_LoadTransmogOutfits` rebuild
        // one shared `_equipmentSets` container before `SendEquipmentSetList`.
        self.clear_represented_equipment_sets_like_cpp();
        let mut equipment_sets_loaded = true;
        {
            let mut equipment_set_stmt =
                char_db.prepare(CharStatements::SEL_CHARACTER_EQUIPMENTSETS);
            equipment_set_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&equipment_set_stmt).await {
                Ok(mut equipment_set_result) => {
                    if !equipment_set_result.is_empty() {
                        loop {
                            let set_guid: u64 = equipment_set_result.try_read(0).unwrap_or(0);
                            let set_id: u32 =
                                u32::from(equipment_set_result.try_read::<u8>(1).unwrap_or(0));
                            let set_name: String =
                                equipment_set_result.try_read(2).unwrap_or_default();
                            let set_icon: String =
                                equipment_set_result.try_read(3).unwrap_or_default();
                            let ignore_mask: u32 = equipment_set_result.try_read(4).unwrap_or(0);
                            let assigned_spec_index: i32 =
                                equipment_set_result.try_read(5).unwrap_or(-1);
                            let mut pieces = [ObjectGuid::EMPTY;
                                wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
                            for (slot, piece) in pieces.iter_mut().enumerate() {
                                let item_low_guid: u64 =
                                    equipment_set_result.try_read(6 + slot).unwrap_or(0);
                                if item_low_guid != 0 {
                                    *piece =
                                        ObjectGuid::create_item(realm_id, item_low_guid as i64);
                                }
                            }
                            self.load_represented_equipment_set_row_like_cpp(
                                set_guid,
                                set_id,
                                set_name,
                                set_icon,
                                ignore_mask,
                                assigned_spec_index,
                                pieces,
                            );
                            if !equipment_set_result.next_row() {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    equipment_sets_loaded = false;
                    warn!("Failed to load equipment sets for {:?}: {}", guid, e);
                }
            }
        }
        {
            let mut transmog_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_TRANSMOG_OUTFITS);
            transmog_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&transmog_stmt).await {
                Ok(mut transmog_result) => {
                    if !transmog_result.is_empty() {
                        loop {
                            // The canonical CharacterDB schema keeps transmog
                            // `setguid`/`ignore_mask` signed, unlike the
                            // equipment-set table. C++ Field::GetUInt* accepts
                            // their nonnegative values; mirror that conversion
                            // instead of silently defaulting signed rows to 0.
                            let set_guid = transmog_result
                                .try_read::<i64>(0)
                                .and_then(nonnegative_i64_to_u64_like_cpp)
                                .or_else(|| transmog_result.try_read::<u64>(0))
                                .unwrap_or(0);
                            let set_id: u32 =
                                u32::from(transmog_result.try_read::<u8>(1).unwrap_or(0));
                            let set_name: String = transmog_result.try_read(2).unwrap_or_default();
                            let set_icon: String = transmog_result.try_read(3).unwrap_or_default();
                            let ignore_mask = transmog_result
                                .try_read::<i32>(4)
                                .and_then(nonnegative_i32_to_u32_like_cpp)
                                .or_else(|| transmog_result.try_read::<u32>(4))
                                .unwrap_or(0);
                            let mut appearances =
                                [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
                            for (slot, appearance) in appearances.iter_mut().enumerate() {
                                *appearance = transmog_result.try_read(5 + slot).unwrap_or(0);
                            }
                            let enchants = [
                                transmog_result.try_read(24).unwrap_or(0),
                                transmog_result.try_read(25).unwrap_or(0),
                            ];
                            self.load_represented_transmog_outfit_row_like_cpp(
                                set_guid,
                                set_id,
                                set_name,
                                set_icon,
                                ignore_mask,
                                appearances,
                                enchants,
                            );
                            if !transmog_result.next_row() {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    equipment_sets_loaded = false;
                    warn!("Failed to load transmog outfits for {:?}: {}", guid, e);
                }
            }
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
        {
            let mut cuf_stmt = char_db.prepare(CharStatements::SEL_CHAR_CUF_PROFILES);
            cuf_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&cuf_stmt).await {
                Ok(mut cuf_result) => {
                    if !cuf_result.is_empty() {
                        loop {
                            let id: u8 = cuf_result.try_read(0).unwrap_or(0);
                            let profile = wow_packet::packets::misc::CufProfile {
                                profile_name: cuf_result.try_read(1).unwrap_or_default(),
                                frame_height: cuf_result.try_read(2).unwrap_or(0),
                                frame_width: cuf_result.try_read(3).unwrap_or(0),
                                sort_by: cuf_result.try_read(4).unwrap_or(0),
                                health_text: cuf_result.try_read(5).unwrap_or(0),
                                bool_options: cuf_result.try_read(6).unwrap_or(0),
                                top_point: cuf_result.try_read(7).unwrap_or(0),
                                bottom_point: cuf_result.try_read(8).unwrap_or(0),
                                left_point: cuf_result.try_read(9).unwrap_or(0),
                                top_offset: cuf_result.try_read(10).unwrap_or(0),
                                bottom_offset: cuf_result.try_read(11).unwrap_or(0),
                                left_offset: cuf_result.try_read(12).unwrap_or(0),
                            };
                            if !self.load_represented_cuf_profile_like_cpp(id, profile) {
                                warn!(
                                    player_guid = guid.counter(),
                                    id,
                                    max_profiles =
                                        wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP,
                                    "Skipping invalid CUF profile id"
                                );
                            }
                            if !cuf_result.next_row() {
                                break;
                            }
                        }
                    }
                    self.mark_represented_cuf_profiles_loaded_like_cpp();
                }
                Err(e) => {
                    warn!("Failed to load CUF profiles for {:?}: {}", guid, e);
                }
            }
        }

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
        let mut loaded_spell_side_effect_spells: Vec<i32> = Vec::new();
        let mut favorite_spell_rows: HashSet<i32> = HashSet::new();
        let mut loaded_player_spell_rows = Vec::new();
        let mut skill_rewarded_dependent_spells = HashSet::new();
        let mut skill_rewarded_removed_spells = HashSet::new();
        let mut loaded_player_spell_rows_complete_like_cpp = false;
        let mut favorite_spell_rows_complete_like_cpp = false;
        {
            let mut spell_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_SPELL);
            spell_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&spell_stmt).await {
                Ok(mut spell_result) => {
                    if !spell_result.is_empty() {
                        loop {
                            let spell_id: u32 = spell_result.try_read(0).unwrap_or(0);
                            let active: u8 = spell_result.try_read(1).unwrap_or(1);
                            let disabled: u8 = spell_result.try_read(2).unwrap_or(0);
                            if let Ok(spell_id) = i32::try_from(spell_id)
                                && spell_id > 0
                            {
                                loaded_player_spell_rows.push(
                                    crate::session::RepresentedPlayerSpellLikeCpp {
                                        spell_id,
                                        active: active != 0,
                                        disabled: disabled != 0,
                                        dependent: false,
                                        favorite: false,
                                        state: crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged,
                                    },
                                );
                            }
                            if let Some(spell_id_i32) =
                                loaded_spell_for_add_spell_side_effects_like_cpp(spell_id, disabled)
                            {
                                loaded_spell_side_effect_spells.push(spell_id_i32);
                            }
                            if let Some(spell_id) =
                                active_known_spell_for_send_like_cpp(spell_id, active, disabled)
                            {
                                known_spells.push(spell_id);
                            }
                            if !spell_result.next_row() {
                                break;
                            }
                        }
                    }
                    loaded_player_spell_rows_complete_like_cpp = true;
                    info!("Loaded {} DB spells for {:?}", known_spells.len(), guid);
                }
                Err(e) => {
                    warn!("Failed to load spells for {:?}: {}", guid, e);
                }
            }

            let mut favorite_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_SPELL_FAVORITES);
            favorite_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&favorite_stmt).await {
                Ok(mut favorite_result) => {
                    if !favorite_result.is_empty() {
                        loop {
                            let spell_id: u32 = favorite_result.try_read(0).unwrap_or(0);
                            if let Ok(spell_id) = i32::try_from(spell_id) {
                                favorite_spell_rows.insert(spell_id);
                            }
                            if !favorite_result.next_row() {
                                break;
                            }
                        }
                    }
                    favorite_spell_rows_complete_like_cpp = true;
                    info!(
                        "Loaded {} DB favorite spells for {:?}",
                        favorite_spell_rows.len(),
                        guid
                    );
                }
                Err(e) => {
                    warn!("Failed to load favorite spells for {:?}: {}", guid, e);
                }
            }
        }

        // ── C++ Player::_LoadSkills ──
        let mut skill_records =
            std::collections::HashMap::<u16, crate::session::RepresentedPlayerSkillLikeCpp>::new();
        let mut skill_info_by_id = BTreeMap::<u16, wow_data::SkillInfoEntry>::new();
        let mut loaded_skill_records_like_cpp = false;
        {
            let mut skill_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_SKILLS);
            skill_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&skill_stmt).await {
                Ok(mut skill_result) => {
                    loaded_skill_records_like_cpp = true;
                    if !skill_result.is_empty() {
                        loop {
                            let skill_id: u16 = skill_result.try_read(0).unwrap_or(0);
                            let skill_value: u16 = skill_result.try_read(1).unwrap_or(0);
                            let skill_max: u16 = skill_result.try_read(2).unwrap_or(skill_value);
                            let profession_slot: i8 = skill_result.try_read(3).unwrap_or(-1);
                            if skill_id > 0 {
                                skill_records.insert(
                                    skill_id,
                                    crate::session::RepresentedPlayerSkillLikeCpp {
                                        skill_id,
                                        step: 0,
                                        value: skill_value,
                                        max: skill_max,
                                        profession_slot,
                                        state: crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged,
                                    },
                                );
                            }
                            if !skill_result.next_row() {
                                break;
                            }
                        }
                    }
                    info!(
                        "Loaded {} persisted skill rows for {:?}",
                        skill_records.len(),
                        guid
                    );
                }
                Err(e) => {
                    warn!("Failed to load character_skills for {:?}: {}", guid, e);
                }
            }
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
        {
            let mut talent_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_TALENTS);
            talent_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&talent_stmt).await {
                Ok(mut talent_result) => {
                    let mut loaded = 0usize;
                    let mut skipped = 0usize;
                    if !talent_result.is_empty() {
                        loop {
                            let talent_id: u32 = talent_result.try_read(0).unwrap_or(0);
                            let rank: u8 = talent_result.try_read(1).unwrap_or(0);
                            let talent_group: u8 = talent_result.try_read(2).unwrap_or(0);
                            if self.load_represented_talent_row_with_spell_side_effects_like_cpp(
                                talent_id,
                                rank,
                                talent_group,
                                &mut known_spells,
                                &mut skill_rewarded_dependent_spells,
                            ) {
                                loaded += 1;
                            } else {
                                skipped += 1;
                            }
                            if !talent_result.next_row() {
                                break;
                            }
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
                Err(e) => {
                    warn!("Failed to load character talents for {:?}: {}", guid, e);
                }
            }
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
        {
            let mut glyph_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_GLYPHS);
            glyph_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&glyph_stmt).await {
                Ok(mut glyph_result) => {
                    let mut loaded = 0usize;
                    let mut skipped = 0usize;
                    if !glyph_result.is_empty() {
                        loop {
                            let talent_group: u8 = glyph_result.try_read(0).unwrap_or(0);
                            let glyph_slot: u8 = glyph_result.try_read(1).unwrap_or(0);
                            let glyph_id: u16 = glyph_result.try_read(2).unwrap_or(0);
                            if self.load_represented_glyph_row_like_cpp(
                                talent_group,
                                glyph_slot,
                                glyph_id,
                            ) {
                                loaded += 1;
                            } else {
                                skipped += 1;
                            }
                            if !glyph_result.next_row() {
                                break;
                            }
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
                Err(e) => {
                    warn!("Failed to load character glyphs for {:?}: {}", guid, e);
                }
            }
        }

        // ── Load action buttons from character_action ──
        // Column types: button=tinyint unsigned, action=int unsigned, type=tinyint unsigned
        let mut action_buttons = [0i64; 180];
        let mut action_count = 0u32;
        self.reset_represented_action_buttons_like_cpp();
        {
            let mut action_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_ACTIONS_SPEC);
            action_stmt.set_u64(0, guid.counter() as u64);
            // C++ loads the action-button map for GetActiveTalentGroup(), not always spec 0.
            let (active_spec, trait_config_id) =
                self.represented_action_button_db_context_like_cpp();
            action_stmt.set_u8(1, active_spec);
            action_stmt.set_i32(2, trait_config_id);
            match char_db.query(&action_stmt).await {
                Ok(mut action_result) => {
                    if !action_result.is_empty() {
                        loop {
                            let button: u8 = action_result.read(0);
                            let action: u32 = action_result.try_read(1).unwrap_or(0);
                            let btn_type: u8 = action_result.try_read(2).unwrap_or(0);
                            if (button as usize) < 180 && action > 0 {
                                self.record_loaded_action_button_like_cpp(button, action, btn_type);
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
                    self.mark_represented_action_buttons_loaded_like_cpp();
                    info!("Loaded {} action buttons for {:?}", action_count, guid);
                }
                Err(e) => {
                    warn!("Failed to load action buttons for {:?}: {}", guid, e);
                }
            }
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
                        reputation_rows_complete_like_cpp = true;
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
            let mut aura_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_AURAS);
            aura_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&aura_stmt).await {
                Ok(mut aura_result) => {
                    aura_rows_complete = true;
                    if !aura_result.is_empty() {
                        loop {
                            aura_rows.push(crate::session::CharacterAuraRowLikeCpp {
                                caster_guid: object_guid_from_db_binary_like_cpp(
                                    aura_result.try_read::<Vec<u8>>(0).unwrap_or_default(),
                                ),
                                spell_id: aura_result.try_read(2).unwrap_or(0),
                                effect_mask: aura_result.try_read(3).unwrap_or(0),
                                recalculate_mask: aura_result.try_read(4).unwrap_or(0),
                                difficulty: aura_result.try_read(5).unwrap_or(0),
                                stack_count: aura_result.try_read(6).unwrap_or(1),
                                max_duration_ms: aura_result.try_read(7).unwrap_or(0),
                                remain_time_ms: aura_result.try_read(8).unwrap_or(0),
                                remain_charges: aura_result.try_read(9).unwrap_or(0),
                            });
                            if !aura_result.next_row() {
                                break;
                            }
                        }
                    }
                }
                Err(e) => warn!("Failed to load character auras for {:?}: {}", guid, e),
            }

            let mut aura_effect_rows = Vec::new();
            let mut aura_effect_rows_complete = false;
            let mut aura_effect_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_AURA_EFFECTS);
            aura_effect_stmt.set_u64(0, guid.counter() as u64);
            match char_db.query(&aura_effect_stmt).await {
                Ok(mut aura_effect_result) => {
                    aura_effect_rows_complete = true;
                    if !aura_effect_result.is_empty() {
                        loop {
                            aura_effect_rows.push(crate::session::CharacterAuraEffectRowLikeCpp {
                                caster_guid: object_guid_from_db_binary_like_cpp(
                                    aura_effect_result
                                        .try_read::<Vec<u8>>(0)
                                        .unwrap_or_default(),
                                ),
                                spell_id: aura_effect_result.try_read(2).unwrap_or(0),
                                effect_mask: aura_effect_result.try_read(3).unwrap_or(0),
                                effect_index: aura_effect_result.try_read(4).unwrap_or(0),
                                amount: aura_effect_result.try_read(5).unwrap_or(0),
                                base_amount: aura_effect_result.try_read(6).unwrap_or(0),
                            });
                            if !aura_effect_result.next_row() {
                                break;
                            }
                        }
                    }
                }
                Err(e) => warn!(
                    "Failed to load character aura effects for {:?}: {}",
                    guid, e
                ),
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

        // Mark online in DB
        let mut online_stmt = char_db.prepare(CharStatements::UPD_CHAR_ONLINE);
        online_stmt.set_u32(0, guid.counter() as u32);
        let _ = char_db.execute(&online_stmt).await;

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
