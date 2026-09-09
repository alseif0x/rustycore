// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement packet handlers — CMSG_MOVE_*.
//!
//! All movement opcodes map to the same handler logic:
//!   1. Parse MovementInfo from the packet
//!   2. Sanitize movement flags like `Player::ValidateMovementInfo`
//!   3. Validate: GUID must match `Player::GetUnitBeingMoved()`, position must be finite
//!   4. Update server-side mover position when represented
//!   5. Broadcast SMSG_MOVE_UPDATE to nearby visible sessions
//!
//! Reference: C++ `WorldSession::HandleMovementOpcode`.

use tracing::{info, trace, warn};
use wow_packet::ClientPacket;

use wow_constants::ClientOpcodes;
use wow_constants::movement::MovementFlag;
use wow_constants::unit::UnitStandStateType;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ServerPacket;
use wow_packet::packets::movement::{
    ClientPlayerMovement, MoveApplyMovementForceAck, MoveInitActiveMoverComplete, MoveKnockBackAck,
    MoveRemoveMovementForceAck, MoveSetCollisionHeightAck, MoveSkipTime, MoveSplineDone,
    MoveTeleportAck, MoveTimeSkipped, MoveUpdate, MoveUpdateApplyMovementForce,
    MoveUpdateKnockBack, MoveUpdateModMovementForceMagnitude, MoveUpdateRemoveMovementForce,
    MovementAckMessage, MovementInfo, MovementSpeedAck, SetActiveMover,
};

use crate::map_manager::zone_and_area_for_position_like_cpp;
use crate::session::{
    AreaTriggerCatalogsLikeCpp, ProgressionCatalogsLikeCpp,
    SPELL_AURA_INTERRUPT_FLAG_LANDING_OR_FLIGHT_LIKE_CPP, SPELL_AURA_INTERRUPT_FLAG2_JUMP_LIKE_CPP,
    WorldSession,
};

mod ops_1;
mod state;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use state::*;

#[cfg(test)]
#[path = "movement/tests/mod.rs"]
mod tests;

// ── Handler registrations ─────────────────────────────────────────
// All CMSG_MOVE_* share the same handler (ThreadSafe in C#).

macro_rules! register_move {
    ($opcode:ident) => {
        inventory::submit! {
            PacketHandlerEntry {
                opcode: ClientOpcodes::$opcode,
                status: SessionStatus::LoggedIn,
                processing: PacketProcessing::ThreadSafe,
                handler_name: concat!("handle_movement_", stringify!($opcode)),
                handler: |session, catalogs, pkt| {
                    Box::pin(async move {
                        session
                            .handle_movement_with_catalogs_like_cpp(
                                catalogs.area_triggers.as_ref(),
                                catalogs.creature_spawns.as_ref(),
                                catalogs.progression.as_ref(),
                                &catalogs.player_grid_loader,
                                pkt,
                            )
                            .await
                    })
                },
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

// ── Handler registration (SetActiveMover) ────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetActiveMover,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_active_mover",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::SetActiveMover::read(&mut pkt) {
                    Ok(mover) => session.handle_set_active_mover(mover).await,
                    Err(e) => tracing::warn!("Failed to read SetActiveMover: {e}"),
                }
            })
        },
    }
}

// ── Handler registration (MoveInitActiveMoverComplete) ───────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveInitActiveMoverComplete,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_init_active_mover_complete",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveInitActiveMoverComplete::read(&mut pkt) {
                    Ok(init) => session.handle_move_init_active_mover_complete(init).await,
                    Err(e) => tracing::warn!("Failed to read MoveInitActiveMoverComplete: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetVehicleRecIdAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_set_vehicle_rec_id_ack",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move { let opcode = pkt.client_opcode().unwrap_or(ClientOpcodes::MoveSetVehicleRecIdAck); match wow_packet::packets::vehicle::MoveSetVehicleRecIdAck::read(&mut pkt) { Ok(ack) => session.handle_move_set_vehicle_rec_id_ack(opcode, ack).await, Err(e) => tracing::warn!("Failed to read MoveSetVehicleRecIdAck: {e}"), } })
        },
    }
}

macro_rules! register_movement_ack_message {
    ($opcode:ident) => {
        inventory::submit! {
            PacketHandlerEntry {
                opcode: ClientOpcodes::$opcode,
                status: SessionStatus::LoggedIn,
                processing: PacketProcessing::ThreadSafe,
                handler_name: "handle_movement_ack_message",
                handler: |session, _catalogs, mut pkt| {
                    Box::pin(async move { let opcode = pkt.client_opcode().unwrap_or(ClientOpcodes::$opcode); match wow_packet::packets::movement::MovementAckMessage::read(&mut pkt) { Ok(ack) => session.handle_movement_ack_message(opcode, ack).await, Err(e) => tracing::warn!("Failed to read MovementAckMessage: {e}"), } })
                },
            }
        }
    };
}

macro_rules! register_movement_speed_ack {
    ($opcode:ident) => {
        inventory::submit! {
            PacketHandlerEntry {
                opcode: ClientOpcodes::$opcode,
                status: SessionStatus::LoggedIn,
                processing: PacketProcessing::ThreadSafe,
                handler_name: "handle_movement_speed_ack",
                handler: |session, _catalogs, mut pkt| {
                    Box::pin(async move { let opcode = pkt.client_opcode().unwrap_or(ClientOpcodes::$opcode); match wow_packet::packets::movement::MovementSpeedAck::read(&mut pkt) { Ok(ack) => session.handle_movement_speed_ack(opcode, ack).await, Err(e) => tracing::warn!("Failed to read MovementSpeedAck: {e}"), } })
                },
            }
        }
    };
}

register_movement_ack_message!(MoveCollisionDisableAck);
register_movement_ack_message!(MoveCollisionEnableAck);
register_movement_ack_message!(MoveEnableDoubleJumpAck);
register_movement_ack_message!(MoveEnableSwimToFlyTransAck);
register_movement_ack_message!(MoveFeatherFallAck);
register_movement_ack_message!(MoveForceRootAck);
register_movement_ack_message!(MoveForceUnrootAck);
register_movement_ack_message!(MoveGravityDisableAck);
register_movement_ack_message!(MoveGravityEnableAck);
register_movement_ack_message!(MoveHoverAck);
register_movement_ack_message!(MoveInertiaDisableAck);
register_movement_ack_message!(MoveInertiaEnableAck);
register_movement_ack_message!(MoveSetCanFlyAck);
register_movement_ack_message!(MoveSetCanTurnWhileFallingAck);
register_movement_ack_message!(MoveSetIgnoreMovementForcesAck);
register_movement_ack_message!(MoveWaterWalkAck);

register_movement_speed_ack!(MoveForceWalkSpeedChangeAck);
register_movement_speed_ack!(MoveForceRunSpeedChangeAck);
register_movement_speed_ack!(MoveForceRunBackSpeedChangeAck);
register_movement_speed_ack!(MoveForceSwimSpeedChangeAck);
register_movement_speed_ack!(MoveForceSwimBackSpeedChangeAck);
register_movement_speed_ack!(MoveForceTurnRateChangeAck);
register_movement_speed_ack!(MoveForceFlightSpeedChangeAck);
register_movement_speed_ack!(MoveForceFlightBackSpeedChangeAck);
register_movement_speed_ack!(MoveForcePitchRateChangeAck);
register_movement_speed_ack!(MoveSetModMovementForceMagnitudeAck);

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveKnockBackAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_knock_back_ack",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveKnockBackAck::read(&mut pkt) {
                    Ok(ack) => session.handle_move_knock_back_ack(ack).await,
                    Err(e) => tracing::warn!("Failed to read MoveKnockBackAck: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetCollisionHeightAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_set_collision_height_ack",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveSetCollisionHeightAck::read(&mut pkt) {
                    Ok(ack) => session.handle_move_set_collision_height_ack(ack).await,
                    Err(e) => tracing::warn!("Failed to read MoveSetCollisionHeightAck: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveApplyMovementForceAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_apply_movement_force_ack",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveApplyMovementForceAck::read(&mut pkt) {
                    Ok(ack) => session.handle_move_apply_movement_force_ack(ack).await,
                    Err(e) => tracing::warn!("Failed to read MoveApplyMovementForceAck: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveRemoveMovementForceAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_remove_movement_force_ack",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveRemoveMovementForceAck::read(&mut pkt) {
                    Ok(ack) => session.handle_move_remove_movement_force_ack(ack).await,
                    Err(e) => tracing::warn!("Failed to read MoveRemoveMovementForceAck: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveTimeSkipped,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_move_time_skipped",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveTimeSkipped::read(&mut pkt) {
                    Ok(skipped) => session.handle_move_time_skipped(skipped).await,
                    Err(e) => tracing::warn!("Failed to read MoveTimeSkipped: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSplineDone,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_spline_done",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveSplineDone::read(&mut pkt) {
                    Ok(done) => session.handle_move_spline_done(done).await,
                    Err(e) => tracing::warn!("Failed to read MoveSplineDone: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveTeleportAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_teleport_ack",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveTeleportAck::read(&mut pkt) {
                    Ok(ack) => session.handle_move_teleport_ack(ack).await,
                    Err(e) => tracing::warn!("Failed to read MoveTeleportAck: {e}"),
                }
            })
        },
    }
}
