// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement packet handlers — CMSG_MOVE_*.
//!
//! All movement opcodes map to the same handler logic:
//!   1. Parse MovementInfo from the packet
//!   2. Validate: GUID must match the player, position must be finite
//!   3. Update server-side player position
//!   4. Broadcast SMSG_MOVE_UPDATE to nearby sessions (TODO: multi-session map)
//!
//! Reference: C# Game/Handlers/MovementHandler.cs

use tracing::{trace, warn};

use wow_constants::ClientOpcodes;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::movement::{ClientPlayerMovement, MoveUpdate, SetActiveMover, MoveInitActiveMoverComplete};
use wow_packet::ServerPacket;

use crate::session::WorldSession;

// ── Handler registrations ─────────────────────────────────────────
// All CMSG_MOVE_* share the same handler (ThreadSafe in C#).

macro_rules! register_move {
    ($opcode:ident) => {
        inventory::submit! {
            PacketHandlerEntry {
                opcode: ClientOpcodes::$opcode,
                status: SessionStatus::LoggedIn,
                processing: PacketProcessing::ThreadUnsafe,
                handler_name: concat!("handle_movement_", stringify!($opcode)),
            }
        }
    };
}

register_move!(MoveStartForward);
register_move!(MoveStartBackward);
register_move!(MoveStop);
register_move!(MoveStartStrafeLeft);
register_move!(MoveStartStrafeRight);
register_move!(MoveStopStrafe);
register_move!(MoveStartTurnLeft);
register_move!(MoveStartTurnRight);
register_move!(MoveStopTurn);
register_move!(MoveStartPitchUp);
register_move!(MoveStartPitchDown);
register_move!(MoveStopPitch);
register_move!(MoveSetRunMode);
register_move!(MoveSetWalkMode);
register_move!(MoveHeartbeat);
register_move!(MoveFallLand);
register_move!(MoveFallReset);
register_move!(MoveJump);
register_move!(MoveSetFacing);
register_move!(MoveSetFacingHeartbeat);
register_move!(MoveSetPitch);
register_move!(MoveSetFly);
register_move!(MoveStartAscend);
register_move!(MoveStopAscend);
register_move!(MoveStartDescend);
register_move!(MoveStartSwim);
register_move!(MoveStopSwim);
register_move!(MoveUpdateFallSpeed);

// ── Handler implementation ─────────────────────────────────────────

impl WorldSession {
    /// Handle any CMSG_MOVE_* packet.
    ///
    /// Parses MovementInfo, validates it, updates player position,
    /// and queues a broadcast to nearby players.
    pub async fn handle_movement(&mut self, mut pkt: wow_packet::WorldPacket) {
        let info = match ClientPlayerMovement::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                warn!(account = self.account_id, "Failed to parse movement packet: {e}");
                return;
            }
        };

        // Validate: GUID must match our player character.
        if let Some(ref player_guid) = self.player_guid {
            if info.info.guid != *player_guid && !info.info.guid.is_empty() {
                warn!(
                    account = self.account_id,
                    "Movement GUID mismatch: expected {:?}, got {:?}",
                    player_guid,
                    info.info.guid
                );
                return;
            }
        }

        // Validate: position must be finite (anti-cheat sanity check).
        let pos = &info.info.position;
        if !pos.x.is_finite() || !pos.y.is_finite() || !pos.z.is_finite() {
            warn!(account = self.account_id, "Invalid movement position: {pos:?}");
            return;
        }

        // Update server-side player position.
        self.player_position = Some(info.info.position);
        // Keep the broadcast registry in sync so chat range checks are accurate.
        self.update_registry_position();
        trace!(
            account = self.account_id,
            x = pos.x, y = pos.y, z = pos.z,
            "Player moved"
        );

        // Dynamic visibility update: send new creatures/GOs that came into
        // range and remove those that left. Internally throttled to 50 yards.
        self.update_visibility().await;

        // Check area triggers at the new position
        self.check_area_triggers().await;

        // TODO: aggro proximity check re-enable once combat system is stable
        // self.check_creature_aggro().await;

        // Broadcast movement to other players on the same map.
        if let (Some(guid), Some(registry)) = (self.player_guid, self.player_registry()) {
            use wow_core::ObjectGuid;
            use wow_network::PlayerBroadcastInfo;

            let move_update = MoveUpdate { info: info.info };
            let bytes = move_update.to_bytes();

            for entry in registry.iter() {
                let (other_guid, other_info): (&ObjectGuid, &PlayerBroadcastInfo) = entry.pair();
                if *other_guid == guid { continue; }
                if other_info.map_id != self.current_map_id { continue; }
                let _ = other_info.send_tx.send(bytes.clone());
            }
        }
    }

    /// Handle CMSG_SET_ACTIVE_MOVER — client sets which unit is currently being moved.
    ///
    /// The client sends this after login to establish the active mover GUID.
    /// In single‑player sessions, the mover must be the player's own GUID.
    pub async fn handle_set_active_mover(&mut self, pkt: SetActiveMover) {
        trace!(
            account = self.account_id,
            mover = ?pkt.active_mover,
            "SetActiveMover"
        );

        // Validate: in a single‑player session, the active mover must be
        // the player's own GUID (or empty, meaning "no unit moving").
        if let Some(ref player_guid) = self.player_guid {
            if !pkt.active_mover.is_empty() && pkt.active_mover != *player_guid {
                warn!(
                    account = self.account_id,
                    "SetActiveMover GUID mismatch: expected {:?}, got {:?}",
                    player_guid,
                    pkt.active_mover
                );
                // Not fatal; the client can be wrong, we just ignore.
            }
        }
    }

    /// Handle CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE — client acknowledges active mover ready.
    ///
    /// In C# this updates transport timing flags and triggers visibility update.
    /// For now we just log receipt; transport timing is not yet implemented.
    pub async fn handle_move_init_active_mover_complete(&mut self, pkt: MoveInitActiveMoverComplete) {
        trace!(
            account = self.account_id,
            ticks = pkt.ticks,
            "MoveInitActiveMoverComplete"
        );
        // TODO: Set player local flags and transport server time when transport system exists.
        // For now, do nothing — the client expects no response.
    }
}

// ── Handler registration (SetActiveMover) ────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetActiveMover,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_active_mover",
    }
}

// ── Handler registration (MoveInitActiveMoverComplete) ───────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveInitActiveMoverComplete,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_move_init_active_mover_complete",
    }
}
