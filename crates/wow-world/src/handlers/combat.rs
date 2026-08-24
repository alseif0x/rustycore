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
