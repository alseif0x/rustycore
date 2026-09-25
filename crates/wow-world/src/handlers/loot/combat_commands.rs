// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session delivery of map-owned creature combat commands.

use super::*;

impl WorldSession {
    pub(crate) fn handle_creature_attack_start_like_cpp_command_like_cpp(
        &mut self,
        command: CreatureAttackStartLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn
            || self.player_guid() != Some(command.victim_guid)
            || self.resolved_player_is_alive_like_cpp() != Some(true)
            || self.player_map_id_like_cpp() != command.map_id
        {
            return;
        }
        let session_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if session_instance_id != command.instance_id {
            return;
        }
        let attacker_is_visible = self
            .client_visible_guids_like_cpp
            .contains(&command.attacker_guid);
        self.set_in_combat_like_cpp(true);
        if attacker_is_visible && !command.packet_already_broadcast {
            use wow_packet::packets::combat::AttackStart;
            self.send_packet(&AttackStart {
                attacker: command.attacker_guid,
                victim: command.victim_guid,
            });
        }
    }

    pub(crate) fn handle_creature_attack_stop_like_cpp_command_like_cpp(
        &mut self,
        command: CreatureAttackStopLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn
            || self.player_guid() != Some(command.victim_guid)
            || self.player_map_id_like_cpp() != command.map_id
        {
            return;
        }
        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return;
        };
        if map_key.instance_id != command.instance_id {
            return;
        }
        let Some(still_in_combat) = self.resolved_in_combat_like_cpp() else {
            return;
        };
        self.set_in_combat_like_cpp(still_in_combat);
    }

    pub(crate) fn handle_reconcile_pvp_combat_expiry_like_cpp(
        &mut self,
        command: ReconcilePvpCombatExpiryLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn
            || self.player_guid() != Some(command.player_guid)
            || self.player_map_id_like_cpp() != command.map_id
        {
            return;
        }
        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return;
        };
        if map_key.instance_id != command.instance_id {
            return;
        }
        let still_in_combat = self
            .canonical_map_manager
            .as_ref()
            .and_then(|manager| manager.lock().ok())
            .and_then(|manager| {
                manager
                    .find_map(map_key.map_id, map_key.instance_id)
                    .and_then(|managed| managed.map().get_typed_player(command.player_guid))
                    .map(|player| player.unit().subsystems().combat.has_combat())
            })
            .unwrap_or(false);
        self.set_in_combat_like_cpp(still_in_combat);
    }
}
