// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session-local publication gates for visibility-sensitive loot commands.
//!
//! These handlers own only the final per-session admission and packet delivery.
//! Visibility selection and authoritative state remain with their existing map
//! and session owners.

use super::*;

impl WorldSession {
    /// Recompute this session's map-owned creature visibility.
    ///
    /// This is the session-local side of future global creature CREATE/DESTROY
    /// work. C++ performs creature create/out-of-range decisions in
    /// `Player::UpdateVisibilityOf`; this command reuses Rust's represented
    /// `update_visibility` pass instead of sending raw bytes that cannot update
    /// `client_visible_guids_like_cpp`.
    pub(crate) async fn handle_refresh_visible_world_creatures_with_catalogs_like_cpp_command_like_cpp(
        &mut self,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        command: RefreshVisibleWorldCreaturesLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_map_id_like_cpp() != command.map_id {
            return;
        }
        let session_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|k| k.instance_id)
            .unwrap_or(0);
        if session_instance_id != command.instance_id {
            return;
        }
        self.clear_pending_visibility_refresh_like_cpp();
        self.force_update_visibility_with_catalogs_like_cpp(creature_spawn_catalogs)
            .await;
    }

    pub(crate) fn handle_send_visible_object_values_update_command_like_cpp(
        &mut self,
        command: crate::session::mailbox::SendVisibleObjectValuesUpdateCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_map_id_like_cpp() != command.map_id {
            return;
        }
        let client_has_object = if command.object_guid.is_mo_transport() {
            self.client_visible_transports_like_cpp
                .contains(&command.object_guid)
        } else {
            self.client_visible_guids_like_cpp
                .contains(&command.object_guid)
        };
        if !client_has_object {
            return;
        }

        if let Some(unit_values_update) = command.unit_values_update {
            let update = self.represented_unit_packet_update_to_update_object_like_cpp(
                command.object_guid,
                command.map_id,
                unit_values_update,
            );
            self.send_packet(&update);
        } else {
            self.send_raw_packet(&command.packet_bytes);
        }
    }

    pub(crate) fn handle_send_if_visible_like_cpp_command_like_cpp(
        &mut self,
        command: SendIfVisibleLikeCppCommand,
        realm_connection: bool,
        allow_legacy_creature_source: bool,
    ) {
        if !self.send_if_visible_like_cpp_gate_passes_like_cpp(
            command.queued_at,
            command.source_guid,
            command.map_id,
            command.instance_id,
            &command.packet_bytes,
            allow_legacy_creature_source,
        ) {
            return;
        }
        // All gates passed — deliver the already-serialised packet as-is.
        if command
            .packet_bytes
            .get(0..2)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u16::from_le_bytes)
            == Some(wow_constants::ServerOpcodes::OnMonsterMove as u16)
        {
            tracing::info!(
                account = self.account_id,
                source_guid = ?command.source_guid,
                "RUST_MONSTER_MOVE_DELIVERY sent"
            );
        }
        if realm_connection {
            self.send_raw_packet_realm(&command.packet_bytes);
        } else {
            self.send_raw_packet(&command.packet_bytes);
        }
    }

    /// Deliver one map-owned creature START+GO pair after one visibility gate.
    ///
    /// C++ `WorldObject::SendCombatLogMessage` selects the committed full GO
    /// frame for advanced-combat-log viewers and the basic frame otherwise.
    /// Both viewers receive START and their selected GO consecutively with no
    /// command drain or visibility revalidation between them. The two
    /// frame-oriented socket sends are not transactional against other cloned
    /// producers or a receiver closing after START; absolute writer adjacency
    /// needs a future batch-aware socket envelope.
    pub(crate) fn handle_send_creature_spell_cast_if_visible_like_cpp_command_like_cpp(
        &mut self,
        command: SendCreatureSpellCastIfVisibleLikeCppCommand,
    ) {
        let opcode = |packet_bytes: &[u8]| {
            packet_bytes
                .get(0..2)
                .and_then(|bytes| bytes.try_into().ok())
                .map(u16::from_le_bytes)
        };
        if opcode(&command.start_packet_bytes)
            != Some(wow_constants::ServerOpcodes::SpellStart as u16)
            || opcode(&command.go_packet_bytes)
                != Some(wow_constants::ServerOpcodes::SpellGo as u16)
        {
            return;
        }
        // Recipient selection already happened where C++ performs it: inside the
        // synchronous `SendSpellGo` fan-out, against this session's
        // `HaveAtClient` set. Re-deriving it here from the drain-time set would
        // drop a correctly committed pair after a visibility exit and deliver a
        // stale cast to a viewer that only became visible afterwards. Validate
        // that the command belongs to this session incarnation and that the
        // session is still on the map it was committed for, then honor it.
        if !self
            .client_visible_guids_like_cpp
            .shares_storage_like_cpp(&command.committed_visibility_like_cpp)
        {
            return;
        }
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_map_id_like_cpp() != command.map_id {
            return;
        }
        let session_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if session_instance_id != command.instance_id {
            return;
        }

        // The basic/full combat-log representation was already chosen for this
        // recipient when the cast resolved, so a preference the client toggled
        // since then cannot retroactively change an earlier cast's frame.
        self.send_raw_packet(&command.start_packet_bytes);
        self.send_raw_packet(&command.go_packet_bytes);
    }

    /// Per-session gate for addon chat delivery.
    ///
    /// Mirrors C++ `WorldSession::IsAddonRegistered(prefix)`: when
    /// `_filterAddonMessages` is false, all prefixes are accepted; otherwise
    /// the prefix must be in the session-local registered list.
    pub(crate) fn handle_send_addon_if_registered_like_cpp_command_like_cpp(
        &mut self,
        command: SendAddonIfRegisteredLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.is_addon_registered_like_cpp(&command.prefix) {
            self.send_raw_packet(&command.packet_bytes);
        }
    }
}
