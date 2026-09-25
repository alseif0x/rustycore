// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Login, world entry, logout and the client-state handshake.

use super::*;

mod initial_packets;
mod login;
mod login_recovery;

pub(super) fn is_represented_bag_slot(slot: u8) -> bool {
    (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot)
        || (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&slot)
        || (REAGENT_BAG_SLOT_START..REAGENT_BAG_SLOT_END).contains(&slot)
}

impl WorldSession {
    /// Fallback: skip ConnectTo and trigger direct login on the realm socket.
    ///
    /// Used when no session manager is configured or all ConnectTo retries fail.
    /// Sets a flag so that `process_pending` will call `handle_continue_player_login`.
    pub(super) fn fallback_direct_login(&mut self) {
        // player_loading is already set — create a dummy oneshot that fires immediately
        let (tx, rx) = tokio::sync::oneshot::channel();
        let link = wow_network::session_mgr::InstanceLink {
            send_tx: self.send_tx().clone(),
            send_write_fence_like_cpp: None,
            pkt_rx: None, // None = keep using realm socket's packet_rx
        };
        let _ = tx.send(link);
        self.set_instance_link_rx(Some(rx));
        info!(
            "Fallback: direct login scheduled for account {}",
            self.account_id
        );
    }

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
    /// C++ MiscHandler.cpp:238 validates combat/falling/duel and selects instant
    /// or timed logout. Rust's existing instant-only admission remains incomplete;
    /// the persistence completion below does not implement those missing rules.
    pub async fn handle_logout_request_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        req: LogoutRequest,
    ) {
        if self.state() == crate::session::SessionState::Disconnecting {
            return;
        }
        info!(
            "LogoutRequest (idle={}) from account {}",
            req.idle_logout, self.account_id
        );

        if !self.active_loot_guid.is_empty() {
            self.send_packet(&LootReleaseAll);
        }

        // Always allow instant logout for now (no combat/duel checks)
        self.send_packet(&LogoutResponse::instant_ok());

        let report = self
            .finalize_session_with_generator_like_cpp(
                crate::FinalizationMode::CharacterSelection,
                item_guid_generator,
            )
            .await;
        if report.disposition != crate::FinalizationDisposition::Complete {
            return;
        }

        info!("Player logged out for account {}", self.account_id);
    }

    #[cfg(test)]
    pub async fn handle_logout_request(&mut self, req: LogoutRequest) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_logout_request_with_generator_like_cpp(generators.item.as_ref(), req)
            .await;
    }

    /// Handle CMSG_LOGOUT_CANCEL — player cancels a pending logout.
    pub async fn handle_logout_cancel(&mut self) {
        info!("LogoutCancel from account {}", self.account_id);
        self.logout_time = None;
        self.send_packet(&LogoutCancelAck);
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
        let gear = self.represented_player_gear_stats_like_cpp(true)?;
        let projection = self.player_stat_system_projection_like_cpp(race, class, level, &gear)?;
        self.publish_player_effective_combat_stats_like_cpp(level, projection, &gear);
        let weapon_damage = wow_data::player::effective_weapon_damage_ranges_like_cpp(
            projection,
            gear.weapon_damage,
            gear.base_attack_time,
            self.represented_shapeshift_combat_round_time_like_cpp(),
        );
        let min_damage = weapon_damage[0][0];
        let max_damage = weapon_damage[0][1];
        let min_ranged_damage = weapon_damage[2][0];
        let max_ranged_damage = weapon_damage[2][1];
        let combat = PlayerCombatStats {
            health: restored_saved_health_like_cpp(saved_health, projection.max_health),
            max_health: projection.max_health,
            stats: projection.stats,
            stat_pos_buff: projection.stat_pos_buff,
            stat_neg_buff: projection.stat_neg_buff,
            base_armor: projection.armor,
            school_resistances: self.represented_school_resistances_like_cpp(&gear),
            base_mana: projection.base_mana,
            max_mana: projection.max_mana,
            attack_power: projection.attack_power,
            attack_power_mod_pos: projection.attack_power_mod_pos,
            attack_power_multiplier: projection.attack_power_multiplier,
            ranged_attack_power: projection.ranged_attack_power,
            ranged_attack_power_mod_pos: projection.ranged_attack_power_mod_pos,
            ranged_attack_power_multiplier: projection.ranged_attack_power_multiplier,
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
            mod_damage_done_pos: projection.mod_damage_done_pos,
            mod_damage_done_neg: projection.mod_damage_done_neg,
            mod_healing_done_pos: projection.mod_healing_done_pos,
            mod_damage_done_percent: projection.mod_damage_done_percent,
            mod_healing_done_pct: projection.mod_healing_done_percent,
            mod_target_resistance: projection.mod_target_resistance,
            mod_target_physical_resistance: projection.mod_target_physical_resistance,
            versatility_bonus: projection.versatility_bonus,
            override_spell_power_by_ap_percent: projection.override_spell_power_by_ap_percent,
            override_ap_by_spell_power_percent: projection.override_ap_by_spell_power_percent,
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
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        feature_policy: &SupportFeaturePolicyLikeCpp,
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

        let Some(dungeon_difficulty) = self.represented_dungeon_difficulty_packet_like_cpp() else {
            return false;
        };
        self.send_packet_realm(&dungeon_difficulty);
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
        self.send_packet_realm(&self.feature_system_status_with_policy_like_cpp(feature_policy));

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
        self.recover_battle_pet_trainer_purchases_with_generator_like_cpp(item_guid_generator)
            .await;
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
