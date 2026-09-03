// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Session-only phase driver.
//!
//! This module owns one thing: the order in which a single Session pass does
//! its work, and when that pass decides the session is over. It coordinates
//! ingestion, mailbox drain, Session-owned ticks, timeouts and dispatch. It is
//! deliberately *not* the world, Map or gameplay tick owner — those clocks are
//! driven elsewhere and are unchanged by this module.
//!
//! The cadence around it (how often a pass runs, cancellation, the idle sleep)
//! stays in the composition root that spawns the Session task; what lives here
//! is the pass itself.
//!
//! `phases` freezes the ordered trace so tests assert on the real sequence
//! rather than a hand-written copy of it; `budget` holds the bound the two
//! ingestion phases share.

mod budget;
pub(crate) mod phases;

pub(crate) use budget::MAX_PACKETS_PER_UPDATE;
use phases::SessionDriverPhaseLikeCpp;

#[cfg(test)]
use std::sync::Arc;
use std::time::Instant;

use tracing::{debug, info};
use wow_packet::WorldPacket;

use super::{
    ClientOpcodes, RuntimeTickOwner, SessionHandlerCatalogsLikeCpp, SessionState, WorldSession,
};

impl WorldSession {
    /// Process queued packets (up to [`MAX_PACKETS_PER_UPDATE`] per call).
    ///
    /// Returns the number of packets processed.
    pub fn update_with_catalogs_like_cpp(
        &mut self,
        diff_ms: u32,
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) -> usize {
        let mut processed = 0;
        self.record_driver_phase_like_cpp(SessionDriverPhaseLikeCpp::DrainPrimaryPackets);

        // Drain the primary (instance) packet channel
        while processed < MAX_PACKETS_PER_UPDATE {
            let pkt = match self.packet_rx().try_recv() {
                Ok(p) => p,
                Err(flume::TryRecvError::Empty) => break,
                Err(flume::TryRecvError::Disconnected) => {
                    debug!(
                        "Packet channel disconnected for account {}",
                        self.account_id
                    );
                    self.state = SessionState::Disconnecting;
                    break;
                }
            };

            self.last_packet_time = Instant::now();
            self.reset_timeout_time_for_packet_like_cpp(pkt.opcode_raw());
            if !self.evaluate_packet_spoof_like_cpp(&pkt) {
                break;
            }
            if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some()
                && pkt.client_opcode() == Some(ClientOpcodes::RequestCemeteryList)
            {
                info!(
                    account = self.account_id,
                    state = ?self.state,
                    pending_before = self.pending_packets.len(),
                    "RUST_CEMETERY_TRACE queued primary packet"
                );
            }
            self.pending_packets.push(pkt);
            processed += 1;
        }

        // Also drain the realm socket channel (after ConnectTo, realm-type
        // packets like BattlenetRequest, Ping, etc. arrive here)
        self.record_driver_phase_like_cpp(SessionDriverPhaseLikeCpp::DrainRealmPackets);
        if let Some(realm_rx) = self.realm_packet_rx() {
            while processed < MAX_PACKETS_PER_UPDATE {
                match realm_rx.try_recv() {
                    Ok(pkt) => {
                        self.last_packet_time = Instant::now();
                        self.reset_timeout_time_for_packet_like_cpp(pkt.opcode_raw());
                        if !self.evaluate_packet_spoof_like_cpp(&pkt) {
                            break;
                        }
                        if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some()
                            && pkt.client_opcode() == Some(ClientOpcodes::RequestCemeteryList)
                        {
                            info!(
                                account = self.account_id,
                                state = ?self.state,
                                pending_before = self.pending_packets.len(),
                                "RUST_CEMETERY_TRACE queued realm packet"
                            );
                        }
                        self.pending_packets.push(pkt);
                        processed += 1;
                    }
                    Err(flume::TryRecvError::Empty) => break,
                    Err(flume::TryRecvError::Disconnected) => {
                        info!(
                            "Realm socket disconnected for account {} (instance still active)",
                            self.account_id
                        );
                        // Realm dropped — don't disconnect immediately, the
                        // instance socket may still be fine.
                        self.clear_realm_packet_rx();
                        break;
                    }
                }
            }
        }

        self.record_driver_phase_like_cpp(SessionDriverPhaseLikeCpp::ConnectionTimeout);
        if self.is_connection_idle_like_cpp() {
            debug!(
                "Session account {} timed out by SocketTimeOutTime-like deadline",
                self.account_id
            );
            self.state = SessionState::Disconnecting;
        }

        // ── Creature / player combat ticks ─────────────────────────
        // Creature AI is owned by the map runtime when GlobalLegacy is active.
        // Player auto-attack remains session-owned here: C++ Player::Update
        // calls DoMeleeAttackIfReady before Map::Update runs ObjectUpdater.
        if self.state == SessionState::LoggedIn {
            self.record_driver_phase_like_cpp(SessionDriverPhaseLikeCpp::SessionOwnedTicks);
            self.update_pvp_flag_like_cpp(wow_entities::game_time_secs_like_cpp());
            let _ = self.set_represented_can_delay_teleport_like_cpp(true);
            // Read the tick owner once; the lock is taken and released inside
            // runtime_tick_owner_like_cpp before any tick work begins.
            let owner = self.runtime_tick_owner_like_cpp();
            self.creature_tick = self.creature_tick.wrapping_add(1);
            if self.creature_tick % 4 == 0 && owner == RuntimeTickOwner::Session {
                self.tick_creatures_sync();
            }
            // Combat tick every 2 ticks (~100ms), and only when this session
            // owns the tick.
            //
            // Under `GlobalLegacy` the map owns this transition (#28), so the
            // swing resolves once from the loop's real map diff instead of once
            // per session on each session's own pass clock. Gated, not deleted:
            // `RustyCore.LegacyCreatureGlobalRuntime = 0` keeps the owner at
            // `Session`, and player auto-attack must keep working there.
            if self.creature_tick % 2 == 0 && owner == RuntimeTickOwner::Session {
                self.tick_combat_sync();
            }
            // Aura expiry tick every 4 ticks (~200ms) — always, regardless of owner.
            if self.creature_tick % 4 == 0 {
                self.tick_auras();
            }
            self.update_player_save_timer_like_cpp(diff_ms);
            self.revalidate_represented_tavern_resting_with_catalog_like_cpp(
                catalogs.area_triggers.db2.as_ref(),
            );
            self.tick_represented_online_xp_rest_bonus_like_cpp(
                Self::current_game_time_secs_like_cpp(),
            );
            let _ = self.set_represented_can_delay_teleport_like_cpp(false);
            self.process_represented_delayed_teleport_after_update_like_cpp();
        }

        // ── Periodic TimeSyncRequest ──────────────────────────────
        // C++ `WorldSession::Update` sends `SendTimeSync` every 10s after the
        // initial `Player::SendInitialPacketsBeforeAddToMap` sync.
        // The client MUST receive periodic TimeSyncRequests or its
        // internal clock sync state becomes inconsistent → crash.
        if self.state == SessionState::LoggedIn && self.time_sync_timer_ms > 0 {
            self.record_driver_phase_like_cpp(SessionDriverPhaseLikeCpp::TimeSync);
            if diff_ms >= self.time_sync_timer_ms {
                self.send_time_sync();
            } else {
                self.time_sync_timer_ms -= diff_ms;
            }
        }

        // ── Logout timer ────────────────────────────────────────────
        if let Some(logout_time) = self.logout_time {
            self.record_driver_phase_like_cpp(SessionDriverPhaseLikeCpp::LogoutTimer);
            if Instant::now() >= logout_time {
                self.logout_time = None;
                self.complete_logout();
            }
        }

        processed
    }

    #[cfg(test)]
    pub fn update(&mut self, diff_ms: u32) -> usize {
        let catalogs = self.session_handler_catalogs_for_test_like_cpp();
        self.update_with_catalogs_like_cpp(diff_ms, &catalogs)
    }

    /// Process pending packets asynchronously. Call after `update()`.
    pub async fn process_pending_with_catalogs_like_cpp(
        &mut self,
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) {
        self.record_driver_phase_like_cpp(SessionDriverPhaseLikeCpp::FlushPacketSpoofBan);
        self.flush_packet_spoof_ban_like_cpp().await;
        self.record_driver_phase_like_cpp(SessionDriverPhaseLikeCpp::SessionCommands);
        self.process_represented_session_commands_like_cpp().await;
        self.record_driver_phase_like_cpp(SessionDriverPhaseLikeCpp::CreatureKills);
        self.process_pending_creature_kills_like_cpp().await;

        // ── Spell casting tick ─────────────────────────────────────────
        // Check if an active spell cast has completed and execute it.
        if self.state == SessionState::LoggedIn {
            self.record_driver_phase_like_cpp(SessionDriverPhaseLikeCpp::LoggedInGameplayTicks);
            if let Some(player_guid) = self.player_guid() {
                self.close_retired_active_loot_windows_like_cpp(player_guid);
            }
            self.tick_represented_loot_rolls_like_cpp().await;
            self.tick_represented_gameobject_update_like_cpp();
            self.send_represented_gameobject_visibility_on_destroy_from_last_update_like_cpp();
            self.send_represented_capture_point_removed_from_last_update_like_cpp();
            self.send_represented_gameobject_visual_despawn_from_last_update_like_cpp();
            self.tick_active_spell_cast().await;
            self.tick_pending_spell_cast_request_like_cpp().await;
            self.sync_represented_farsight_clear_from_canonical_like_cpp();
            self.send_represented_dynamic_object_values_updates_from_last_map_send_object_updates_like_cpp();
        }

        // Check for instance link delivery (ConnectTo flow)
        self.record_driver_phase_like_cpp(SessionDriverPhaseLikeCpp::PollInstanceLink);
        self.poll_instance_link_with_module_registry_like_cpp(catalogs.modules.as_ref())
            .await;

        // Process pending creature/gameobject spawn (async DB query)
        self.record_driver_phase_like_cpp(SessionDriverPhaseLikeCpp::PendingCreatureSpawn);
        if let Some(spawn) = self.pending_creature_spawn.take() {
            self.send_nearby_creatures(spawn.map_id, &spawn.position, spawn.zone_id)
                .await;
            self.send_nearby_gameobjects(spawn.map_id, &spawn.position, spawn.zone_id)
                .await;
        }

        self.record_driver_phase_like_cpp(SessionDriverPhaseLikeCpp::DispatchQueuedPackets);
        let packets: Vec<WorldPacket> = self.pending_packets.drain(..).collect();
        for pkt in packets {
            if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some()
                && pkt.client_opcode() == Some(ClientOpcodes::RequestCemeteryList)
            {
                info!(
                    account = self.account_id,
                    state = ?self.state,
                    "RUST_CEMETERY_TRACE dispatching queued packet"
                );
            }
            self.dispatch_packet(catalogs, pkt).await;
        }

        self.record_driver_phase_like_cpp(SessionDriverPhaseLikeCpp::PeriodicPlayerSave);
        self.process_pending_periodic_player_save_like_cpp().await;
    }

    #[cfg(test)]
    pub async fn process_pending(&mut self) {
        let catalogs = self.session_handler_catalogs_for_test_like_cpp();
        self.process_pending_with_catalogs_like_cpp(&catalogs).await;
    }

    #[cfg(test)]
    fn session_handler_catalogs_for_test_like_cpp(&self) -> SessionHandlerCatalogsLikeCpp {
        let empty_catalogs = SessionHandlerCatalogsLikeCpp::default();
        let catalogs = self
            .object_mgr_catalogs_like_cpp
            .as_ref()
            .cloned()
            .unwrap_or_default();
        let catalogs = SessionHandlerCatalogsLikeCpp {
            object_mgr: catalogs,
            area_triggers: Arc::new(self.area_trigger_catalogs_for_test_like_cpp()),
            bank_bag_slot_prices: self
                .bank_bag_slot_prices_store
                .clone()
                .unwrap_or(empty_catalogs.bank_bag_slot_prices),
            adventure_map_pois: self
                .adventure_map_poi_store
                .clone()
                .unwrap_or(empty_catalogs.adventure_map_pois),
            battlemaster_lists: self
                .battlemaster_list_store
                .clone()
                .unwrap_or(empty_catalogs.battlemaster_lists),
            emotes: self.emotes_store.clone().unwrap_or(empty_catalogs.emotes),
            emotes_text: self
                .emotes_text_store
                .clone()
                .unwrap_or(empty_catalogs.emotes_text),
            graveyards: self
                .graveyard_store
                .clone()
                .unwrap_or(empty_catalogs.graveyards),
            lfg_dungeons: self
                .lfg_dungeon_store_like_cpp
                .clone()
                .unwrap_or(empty_catalogs.lfg_dungeons),
            tact_keys: self
                .tact_key_store
                .clone()
                .unwrap_or(empty_catalogs.tact_keys),
            modules: self
                .module_registry_like_cpp
                .clone()
                .unwrap_or(empty_catalogs.modules),
            id_generators: Arc::new(self.id_generators_for_test_like_cpp()),
        };
        catalogs
    }
}
