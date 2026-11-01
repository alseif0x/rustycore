// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The mechanical registration tail of the C++ `MovementHandler.cpp` family
//! (#1263 F5 tail).
//!
//! C++ source of truth: `src/server/game/Handlers/MovementHandler.cpp:305`
//! `WorldSession::HandleMovementOpcodes`, `:310` `HandleMovementOpcode`, `:468`
//! `HandleForceSpeedChangeAck`, `:563` `HandleMovementAckMessage` and `:667`
//! `HandleMoveSplineDoneOpcode`. The application crate owns this registration
//! tail because it is the area that coordinates active-mover validation,
//! movement application and taxi continuation for those opcodes; the movement
//! implementation itself stays in `wow-world`'s `handlers/movement` tree.
//!
//! The three statement macros expand to exactly one
//! `builder.register(PacketHandlerEntry { .. })?` each and are invoked only
//! from [`register_movement_tail_handlers_like_cpp`], so every effective entry
//! originates from this area registrar instead of the legacy inventory. The
//! registered tuples (opcode, handler name, session status, packet processing)
//! are the ones the World registry submitted before this move.

use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;
use wow_packet::packets::movement::{MoveSplineDone, MovementAckMessage, MovementSpeedAck};

use super::MovementHandlerHostLikeCpp;

/// One `CMSG_MOVE_*` opcode of the C++ `HandleMovementOpcode` family.
macro_rules! register_move {
    ($builder:ident, $opcode:ident) => {
        $builder.register(PacketHandlerEntry {
            opcode: ClientOpcodes::$opcode,
            status: SessionStatus::LoggedIn,
            processing: PacketProcessing::ThreadSafe,
            handler_name: concat!("handle_movement_", stringify!($opcode)),
            handler: handle_movement_tail_move_thunk::<S, C>,
        })?
    };
}

/// One `CMSG_MOVE_*_ACK` opcode served by C++ `HandleMovementAckMessage`.
macro_rules! register_movement_ack_message {
    ($builder:ident, $opcode:ident) => {
        $builder.register(PacketHandlerEntry {
            opcode: ClientOpcodes::$opcode,
            status: SessionStatus::LoggedIn,
            processing: PacketProcessing::ThreadSafe,
            handler_name: "handle_movement_ack_message",
            handler: |session, catalogs, pkt| {
                handle_movement_tail_ack_message_thunk::<S, C>(
                    session,
                    catalogs,
                    ClientOpcodes::$opcode,
                    pkt,
                )
            },
        })?
    };
}

/// One speed-change acknowledgement of C++ `HandleForceSpeedChangeAck`.
macro_rules! register_movement_speed_ack {
    ($builder:ident, $opcode:ident) => {
        $builder.register(PacketHandlerEntry {
            opcode: ClientOpcodes::$opcode,
            status: SessionStatus::LoggedIn,
            processing: PacketProcessing::ThreadSafe,
            handler_name: "handle_movement_speed_ack",
            handler: |session, catalogs, pkt| {
                handle_movement_tail_speed_ack_thunk::<S, C>(
                    session,
                    catalogs,
                    ClientOpcodes::$opcode,
                    pkt,
                )
            },
        })?
    };
}

fn handle_movement_tail_move_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: MovementHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move { session.handle_movement_opcode_like_cpp(catalogs, pkt).await })
}

fn handle_movement_tail_ack_message_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    declared_opcode: ClientOpcodes,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: MovementHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let opcode = pkt.client_opcode().unwrap_or(declared_opcode);
        match MovementAckMessage::read(&mut pkt) {
            Ok(ack) => {
                session
                    .handle_movement_ack_message_like_cpp(opcode, ack)
                    .await
            }
            Err(e) => tracing::warn!("Failed to read MovementAckMessage: {e}"),
        }
    })
}

fn handle_movement_tail_speed_ack_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    declared_opcode: ClientOpcodes,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: MovementHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let opcode = pkt.client_opcode().unwrap_or(declared_opcode);
        match MovementSpeedAck::read(&mut pkt) {
            Ok(ack) => {
                session
                    .handle_movement_speed_ack_like_cpp(opcode, ack)
                    .await
            }
            Err(e) => tracing::warn!("Failed to read MovementSpeedAck: {e}"),
        }
    })
}

fn handle_move_spline_done_tail_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: MovementHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match MoveSplineDone::read(&mut pkt) {
            Ok(done) => session.handle_move_spline_done_like_cpp(done).await,
            Err(e) => tracing::warn!("Failed to read MoveSplineDone: {e}"),
        }
    })
}

/// Register the mechanical `CMSG_MOVE_*` tail (53 macro entries and the direct
/// `CMSG_MOVE_SPLINE_DONE` entry).
pub fn register_movement_tail_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: MovementHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    register_move!(builder, MoveStartForward);
    register_move!(builder, MoveStartBackward);
    register_move!(builder, MoveStop);
    register_move!(builder, MoveStartStrafeLeft);
    register_move!(builder, MoveStartStrafeRight);
    register_move!(builder, MoveStopStrafe);
    register_move!(builder, MoveStartTurnLeft);
    register_move!(builder, MoveStartTurnRight);
    register_move!(builder, MoveStopTurn);
    register_move!(builder, MoveStartPitchUp);
    register_move!(builder, MoveStartPitchDown);
    register_move!(builder, MoveStopPitch);
    register_move!(builder, MoveSetRunMode);
    register_move!(builder, MoveSetWalkMode);
    register_move!(builder, MoveHeartbeat);
    register_move!(builder, MoveFallLand);
    register_move!(builder, MoveFallReset);
    register_move!(builder, MoveJump);
    register_move!(builder, MoveSetFacing);
    register_move!(builder, MoveSetFacingHeartbeat);
    register_move!(builder, MoveSetPitch);
    register_move!(builder, MoveSetFly);
    register_move!(builder, MoveStartAscend);
    register_move!(builder, MoveStopAscend);
    register_move!(builder, MoveStartDescend);
    register_move!(builder, MoveStartSwim);
    register_move!(builder, MoveStopSwim);
    register_move!(builder, MoveUpdateFallSpeed);

    register_movement_ack_message!(builder, MoveCollisionDisableAck);
    register_movement_ack_message!(builder, MoveCollisionEnableAck);
    register_movement_ack_message!(builder, MoveEnableDoubleJumpAck);
    register_movement_ack_message!(builder, MoveEnableSwimToFlyTransAck);
    register_movement_ack_message!(builder, MoveFeatherFallAck);
    register_movement_ack_message!(builder, MoveForceRootAck);
    register_movement_ack_message!(builder, MoveForceUnrootAck);
    register_movement_ack_message!(builder, MoveGravityDisableAck);
    register_movement_ack_message!(builder, MoveGravityEnableAck);
    register_movement_ack_message!(builder, MoveHoverAck);
    register_movement_ack_message!(builder, MoveInertiaDisableAck);
    register_movement_ack_message!(builder, MoveInertiaEnableAck);
    register_movement_ack_message!(builder, MoveSetCanFlyAck);
    register_movement_ack_message!(builder, MoveSetCanTurnWhileFallingAck);
    register_movement_ack_message!(builder, MoveSetIgnoreMovementForcesAck);
    register_movement_ack_message!(builder, MoveWaterWalkAck);

    register_movement_speed_ack!(builder, MoveForceWalkSpeedChangeAck);
    register_movement_speed_ack!(builder, MoveForceRunSpeedChangeAck);
    register_movement_speed_ack!(builder, MoveForceRunBackSpeedChangeAck);
    register_movement_speed_ack!(builder, MoveForceSwimSpeedChangeAck);
    register_movement_speed_ack!(builder, MoveForceSwimBackSpeedChangeAck);
    register_movement_speed_ack!(builder, MoveForceTurnRateChangeAck);
    register_movement_speed_ack!(builder, MoveForceFlightSpeedChangeAck);
    register_movement_speed_ack!(builder, MoveForceFlightBackSpeedChangeAck);
    register_movement_speed_ack!(builder, MoveForcePitchRateChangeAck);

    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSplineDone,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_spline_done",
        handler: handle_move_spline_done_tail_thunk::<S, C>,
    })?;
    Ok(())
}
