// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Combat packet handlers.
//!
//! Handles CMSG_ATTACK_SWING, CMSG_ATTACK_STOP, CMSG_SET_SHEATHED.
//!
//! Reference: C# Game/Handlers/CombatHandler.cs

use tracing::{debug, warn};

use wow_constants::ClientOpcodes;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::combat::{AttackStart, AttackSwing, SAttackStop, SetSheathed};
use wow_packet::{ClientPacket, ServerPacket};

use crate::session::WorldSession;

// ── Handler registrations ─────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AttackSwing,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_attack_swing",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AttackStop,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_attack_stop",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetSheathed,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_sheathed",
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

        let player_guid = match &self.player_guid {
            Some(g) => g.clone(),
            None => return,
        };

        debug!(
            account = self.account_id,
            target = ?swing.victim,
            "CMSG_ATTACK_SWING"
        );

        // Check the creature exists and is alive.
        let creature_alive = self
            .creatures
            .get(&swing.victim)
            .map(|c| c.is_alive)
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

        // Start combat with the creature.
        if let Some(creature) = self.creatures.get_mut(&swing.victim) {
            creature.enter_combat(player_guid.clone());
        }

        // Set the player's current combat target.
        self.combat_target = Some(swing.victim);
        self.in_combat = true;

        // Notify client that combat started.
        let start = AttackStart {
            attacker: player_guid,
            victim: swing.victim,
        };
        self.send_packet(&start);
    }

    /// CMSG_ATTACK_STOP — client stops attacking.
    pub async fn handle_attack_stop(&mut self, _pkt: wow_packet::WorldPacket) {
        let player_guid = match &self.player_guid {
            Some(g) => g.clone(),
            None => return,
        };

        debug!(account = self.account_id, "CMSG_ATTACK_STOP");

        if let Some(target) = self.combat_target.take() {
            self.in_combat = false;

            // Reset creature combat if it was fighting us.
            if let Some(creature) = self.creatures.get_mut(&target) {
                if creature.state == wow_ai::CreatureState::InCombat {
                    creature.reset_combat();
                }
            }

            let stop = SAttackStop {
                attacker: player_guid.clone(),
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
