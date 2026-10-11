// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Cross-crate session fixtures (#1263 F6-8D3b-2).
//!
//! The world-server integration tests boot the production canonical map loop
//! and need a **real** `WorldSession` on the other end of its delivery: one
//! that drains its own mailbox, runs the creature-kill phase of the session
//! update and dispatches client packets through the registered production
//! handlers. These entry points expose exactly those steps, in the order the
//! session driver runs them, and nothing else. Built only with `test` or the
//! dev-dependency feature `test-fixtures`; never in a production build.

use std::sync::Arc;

use wow_constants::ClientOpcodes;
use wow_core::{ObjectGuid, Position};
use wow_packet::WorldPacket;

use crate::session::mailbox::SessionCommand;
use crate::session::{
    PlayerRegistry, SessionHandlerCatalogsLikeCpp, SessionPlayerController, SessionState,
    SharedCanonicalMapManager, WorldSession,
};

impl WorldSession {
    /// Put this session's character in the world on its canonical map and
    /// register it in `registry`, the way the login path leaves a session:
    /// controller attached, canonical player on the map, active mover
    /// initialized, `LoggedIn`, registered, and one visibility pass sent.
    /// Returns whether the canonical map admitted the player.
    #[allow(clippy::too_many_arguments)]
    pub async fn enter_canonical_world_for_test_like_cpp(
        &mut self,
        canonical: SharedCanonicalMapManager,
        map_store: Arc<wow_data::MapStore>,
        registry: Arc<PlayerRegistry>,
        player_guid: ObjectGuid,
        name: &str,
        position: Position,
        level: u8,
    ) -> bool {
        self.set_canonical_map_manager(canonical);
        self.set_map_store(map_store);
        self.attach_player_controller_like_cpp(SessionPlayerController::new(
            player_guid,
            name.to_string(),
            position,
            0,
            1,
            1,
            level,
            0,
        ));
        if self
            .ensure_canonical_world_map_for_current_player_like_cpp()
            .is_none()
        {
            return false;
        }
        self.apply_move_init_active_mover_complete_like_cpp(0);
        self.set_state(SessionState::LoggedIn);
        self.set_player_registry(registry);
        self.register_in_player_registry();
        let catalogs = SessionHandlerCatalogsLikeCpp::default();
        self.force_update_visibility_with_catalogs_like_cpp(catalogs.creature_spawns.as_ref())
            .await;
        true
    }

    /// One session update over the committed commands, as the driver runs it:
    /// drain both mailbox rails, apply each command (after `observe` sees it),
    /// flush the pending visibility refresh, then the creature-kill phase
    /// (`process_pending_creature_kills`: kill loot, rewards and the
    /// `setDeathState(JUST_DIED)` completion through the mutation root).
    pub async fn run_session_commands_and_kills_for_test_like_cpp(
        &mut self,
        mut observe: impl FnMut(&SessionCommand),
    ) {
        let catalogs = SessionHandlerCatalogsLikeCpp::default();
        // The map pass the canonical tick requested on the phase rail, as the
        // session task answers it.
        let phase_rx = self.session_phase_receiver_like_cpp();
        while let Ok(request) = phase_rx.try_recv() {
            let _ = self
                .run_requested_session_phase_like_cpp(request, &catalogs)
                .await;
        }
        for command in self.core.drain_session_commands() {
            observe(&command);
            self.apply_session_command_with_catalogs_like_cpp(&catalogs, command)
                .await;
        }
        self.flush_pending_visibility_refresh_with_catalogs_like_cpp(
            catalogs.creature_spawns.as_ref(),
        )
        .await;
        self.process_pending_creature_kills_with_generator_like_cpp(
            catalogs.id_generators.item.as_ref(),
        )
        .await;
    }

    /// Dispatch one client packet through its registered production handler.
    pub async fn dispatch_client_packet_for_test_like_cpp(
        &mut self,
        opcode: ClientOpcodes,
        packet: WorldPacket,
    ) {
        let catalogs = SessionHandlerCatalogsLikeCpp::default();
        let entry = crate::session::registry::registered_handler_entries_like_cpp()
            .find(|entry| entry.opcode == opcode)
            .expect("the opcode has a registered production handler");
        (entry.handler)(self, &catalogs, packet).await;
    }
}

impl WorldSession {
    /// The logged-in gameplay step that sends the Unit/Player VALUES of the
    /// map's last `SendObjectUpdates` to this client (once per map update
    /// generation). Returns how many updates it sent.
    pub fn send_map_unit_values_updates_for_test_like_cpp(&mut self) -> usize {
        self.send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
        )
    }

    /// Whether this session's client currently has `guid` (C++
    /// `Player::HaveAtClient`).
    pub fn client_sees_guid_for_test_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.core.client_visible_guids_like_cpp.contains(&guid)
    }
}
