// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Combat packet handlers.
//!
//! Handles CMSG_ATTACK_SWING, CMSG_ATTACK_STOP, CMSG_SET_SHEATHED.
//!
//! Reference: C++ `WorldSession::HandleAttack*Opcode`
//! (`src/server/game/Handlers/CombatHandler.cpp`).

use tracing::{debug, warn};

use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::combat::{AttackStart, AttackSwing, SAttackStop, SetSheathed};

use crate::session::{PlayerAttackStartLikeCppResult, WorldSession};

// ── Handler registrations ─────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AttackSwing,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_attack_swing",
        handler: |session, pkt| Box::pin(async move { session.handle_attack_swing(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AttackStop,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_attack_stop",
        handler: |session, pkt| Box::pin(async move { session.handle_attack_stop(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetSheathed,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_sheathed",
        handler: |session, pkt| Box::pin(async move { session.handle_set_sheathed(pkt) }),
    }
}

// ── Handler implementations ───────────────────────────────────────

impl WorldSession {
    /// Deliver one map-owned player auto-attack resolution to its attacker.
    ///
    /// #28 moved the transition itself to whoever owns the creature tick, so
    /// this is delivery only: it applies no damage and decides no swing. The
    /// order below is `run_combat_tick`'s tail verbatim, and the order is what
    /// keeps the bytes identical — `SMSG_ATTACKSWING_ERROR` before the swings,
    /// the values update gated on what this client already has, the kill queues
    /// before `SMSG_ATTACKSTOP`.
    pub(crate) fn handle_apply_player_melee_result_like_cpp_command_like_cpp(
        &mut self,
        command: crate::session::mailbox::ApplyPlayerMeleeResultLikeCppCommand,
    ) {
        use wow_packet::ServerPacket;
        use wow_packet::packets::combat::{
            AttackerStateUpdate, HIT_INFO_NORMAL_SWING, VICTIM_STATE_HIT,
        };
        use wow_packet::packets::movement::MonsterMoveStop;

        // Re-gate on arrival: a command resolved for one incarnation must not
        // land on a reconnect, another character, or another map.
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_guid() != Some(command.attacker_guid) {
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

        if let Some(swing_error) = command.swing_error_after {
            self.set_player_attack_swing_error_like_cpp(swing_error);
        }

        // The construction below is `run_combat_tick`'s tail verbatim; the
        // constants and field order are what make the bytes identical.
        if let Some(victim_guid) = command.victim_guid {
            for swing in &command.swings {
                self.send_raw_packet(
                    &AttackerStateUpdate {
                        attacker: command.attacker_guid,
                        victim: victim_guid,
                        hit_info: HIT_INFO_NORMAL_SWING,
                        damage: swing.damage as i32,
                        over_damage: swing.over_damage,
                        victim_state: VICTIM_STATE_HIT,
                        school_mask: 1,
                        target_level: command.target_level,
                        expansion: 2,
                    }
                    .to_bytes(),
                );
            }
        }

        if let (Some(values_update), Some(victim_guid)) =
            (command.victim_values_update.as_ref(), command.victim_guid)
            && self.client_visible_guids_like_cpp.contains(&victim_guid)
            && let Some(update) = self.represented_unit_values_update_to_update_object_like_cpp(
                victim_guid,
                command.map_id,
                values_update,
            )
        {
            self.send_raw_packet(&update.to_bytes());
        }

        if let Some(kill) = command.killed_creature.as_ref() {
            self.queue_pending_creature_kill_like_cpp(
                command.attacker_guid,
                kill.creature_guid,
                kill.creature_entry,
                kill.creature_level,
            );
            if let Some((current_pos, spline_id)) = kill.move_stop {
                self.send_raw_packet(
                    &MonsterMoveStop {
                        mover_guid: kill.creature_guid,
                        current_pos,
                        spline_id,
                    }
                    .to_bytes(),
                );
            }
            self.send_raw_packet(
                &SAttackStop {
                    attacker: command.attacker_guid,
                    victim: kill.creature_guid,
                    now_dead: true,
                }
                .to_bytes(),
            );
        }

        if let Some(combat_target) = command.combat_target_after {
            self.combat_target = combat_target;
        }
        if let Some(in_combat) = command.in_combat_after {
            self.set_in_combat_like_cpp(in_combat);
        }
    }

    /// CMSG_ATTACK_SWING — client requests to attack a target.
    ///
    /// The target must be a known creature in the current map.
    /// Sends SMSG_ATTACK_START if the target is valid.
    pub async fn handle_attack_swing(&mut self, mut pkt: wow_packet::WorldPacket) {
        let swing = match AttackSwing::read(&mut pkt) {
            Ok(s) => s,
            Err(e) => {
                warn!(account = self.account_id, "Failed to read AttackSwing: {e}");
                return;
            }
        };

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        debug!(
            account = self.account_id,
            target = ?swing.victim,
            "CMSG_ATTACK_SWING"
        );

        // Check the creature exists and is alive.
        let creature_alive = self
            .mutate_world_creature(swing.victim, |c| c.is_alive())
            .unwrap_or(false);

        if !creature_alive {
            // Send attack stop so client clears the attack state.
            let stop = SAttackStop {
                attacker: player_guid,
                victim: swing.victim,
                now_dead: true,
            };
            self.send_packet(&stop);
            return;
        }

        let attack_start = match self.start_player_attack_like_cpp(swing.victim) {
            PlayerAttackStartLikeCppResult::Rejected => {
                let stop = SAttackStop {
                    attacker: player_guid,
                    victim: swing.victim,
                    now_dead: false,
                };
                self.send_packet(&stop);
                return;
            }
            PlayerAttackStartLikeCppResult::Accepted { send_attack_start } => send_attack_start,
        };

        // Start combat with the canonical map-owned creature after C++-style
        // attack validation succeeds.
        let _ = self.mutate_world_creature(swing.victim, |creature| {
            creature.enter_combat(player_guid);
        });

        // Unit::Attack only emits a melee start packet for new targets or a
        // same-target ranged-to-melee switch; same-target no-op is accepted.
        if attack_start {
            let start = AttackStart {
                attacker: player_guid,
                victim: swing.victim,
            };
            self.send_packet(&start);
        }
    }

    /// CMSG_ATTACK_STOP — client stops attacking.
    pub async fn handle_attack_stop(&mut self, _pkt: wow_packet::WorldPacket) {
        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        debug!(account = self.account_id, "CMSG_ATTACK_STOP");

        if let Some(target) = self.stop_player_attack_like_cpp() {
            let stop = SAttackStop {
                attacker: player_guid,
                victim: target,
                now_dead: false,
            };
            self.send_packet(&stop);
        }
    }

    /// CMSG_SET_SHEATHED — client changes weapon sheathe state.
    ///
    /// We just ack silently; the client manages the visual state.
    pub fn handle_set_sheathed(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Ok(sheathed) = SetSheathed::read(&mut pkt) {
            debug!(
                account = self.account_id,
                state = sheathed.current_sheath_state,
                "SetSheathed"
            );
        }
    }
}
