// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement handlers for the C++ `MovementHandler.cpp` family (#1263 F5).
//!
//! C++ source of truth: `src/server/game/Handlers/MovementHandler.cpp`
//! (`HandleMoveTimeSkippedOpcode`, `HandleSetActiveMoverOpcode`,
//! `HandleSetCollisionHeightAck`, `HandleMoveKnockBackAck`,
//! `HandleMoveApplyMovementForceAck` and `HandleMoveRemoveMovementForceAck`,
//! the handlers behind `CMSG_MOVE_TIME_SKIPPED`, `CMSG_SET_ACTIVE_MOVER`,
//! `CMSG_MOVE_SET_COLLISION_HEIGHT_ACK`, `CMSG_MOVE_KNOCK_BACK_ACK`,
//! `CMSG_MOVE_APPLY_MOVEMENT_FORCE_ACK` and
//! `CMSG_MOVE_REMOVE_MOVEMENT_FORCE_ACK`), plus
//! `src/server/game/Handlers/VehicleHandler.cpp` `HandleMoveSetVehicleRecAck`
//! for `CMSG_MOVE_SET_VEHICLE_REC_ID_ACK`.
//! Both bodies keep their original parse, gates, order, log strings and
//! packets; the session only lends its hub, so the World side owns the host
//! thunks and the `MovementHandlerHostLikeCpp` implementation. The remaining
//! `CMSG_MOVE_*` family stays in `wow-world` for later slices.

use tracing::{info, trace, warn};
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::movement::{
    MoveApplyMovementForceAck, MoveKnockBackAck, MoveRemoveMovementForceAck,
    MoveSetCollisionHeightAck, MoveSkipTime, MoveTimeSkipped, MoveUpdateApplyMovementForce,
    MoveUpdateKnockBack, MoveUpdateRemoveMovementForce, MovementAck, MovementForce, SetActiveMover,
};
use wow_packet::{ClientPacket, ServerPacket, WorldPacket};
use wow_world_core::session::HubMut;

/// Borrowed inputs of one movement handler invocation.
pub struct MovementHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
}

impl<'a> MovementHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>) -> Self {
        Self { hub }
    }

    /// Handle CMSG_SET_ACTIVE_MOVER — client sets which unit is currently being moved.
    ///
    /// The client sends this after login to establish the active mover GUID.
    /// The mover must match C++ `Player::GetUnitBeingMoved()`.
    pub async fn handle_set_active_mover(&mut self, pkt: SetActiveMover) {
        info!(
            account = self.hub.shared().core.account_id,
            mover = ?pkt.active_mover,
            expected = ?self.hub.shared().player_moved_unit_guid_like_cpp(),
            "RUST_LOGIN_TRACE SetActiveMover"
        );

        let Some(expected_mover) = self.hub.shared().player_moved_unit_guid_like_cpp() else {
            warn!(
                account = self.hub.shared().core.account_id,
                "SetActiveMover received without canonical active mover"
            );
            return;
        };
        if pkt.active_mover != expected_mover {
            warn!(
                account = self.hub.shared().core.account_id,
                "SetActiveMover GUID mismatch: expected {:?}, got {:?}",
                expected_mover,
                pkt.active_mover
            );
            // C++ only logs this mismatch.
        }
    }

    /// Handle C++ `HandleMoveTimeSkippedOpcode`.
    pub async fn handle_move_time_skipped(&mut self, pkt: MoveTimeSkipped) {
        trace!(
            account = self.hub.shared().core.account_id,
            mover = ?pkt.mover_guid,
            time_skipped = pkt.time_skipped,
            "MoveTimeSkipped"
        );
        if self
            .hub
            .apply_move_time_skipped_like_cpp(pkt.mover_guid, pkt.time_skipped)
            && let Some(source_position) = self.hub.shared().mover_position_like_cpp(pkt.mover_guid)
        {
            self.hub
                .shared()
                .broadcast_from_movement_source_set_like_cpp(
                    pkt.mover_guid,
                    source_position,
                    MoveSkipTime {
                        mover_guid: pkt.mover_guid,
                        time_skipped: pkt.time_skipped,
                    }
                    .to_bytes(),
                    wow_world_core::map_manager::VISIBILITY_RADIUS,
                );
        }
    }

    /// Handle C++ `HandleMoveSetVehicleRecAck` (`VehicleHandler.cpp:193`).
    pub async fn handle_move_set_vehicle_rec_id_ack(
        &mut self,
        opcode: ClientOpcodes,
        pkt: wow_packet::packets::vehicle::MoveSetVehicleRecIdAck,
    ) -> MoveSetVehicleRecIdAckStepLikeCpp {
        trace!(
            account = self.hub.shared().core.account_id,
            ?opcode,
            vehicle_rec_id = pkt.vehicle_rec_id,
            "MoveSetVehicleRecIdAck"
        );
        MoveSetVehicleRecIdAckStepLikeCpp::ApplyMoveSetVehicleRecIdAck { ack: pkt.data }
    }

    /// Handle C++ `HandleSetCollisionHeightAck` (`MovementHandler.cpp:576`).
    pub async fn handle_move_set_collision_height_ack(
        &mut self,
        pkt: MoveSetCollisionHeightAck,
    ) -> MoveSetCollisionHeightAckStepLikeCpp {
        trace!(
            account = self.hub.shared().core.account_id,
            height = pkt.height,
            mount_display_id = pkt.mount_display_id,
            reason = pkt.reason,
            "MoveSetCollisionHeightAck"
        );
        MoveSetCollisionHeightAckStepLikeCpp::RecordValidatedAck { ack: pkt.data }
    }

    /// Handle C++ `HandleMoveKnockBackAck` (`MovementHandler.cpp:548`).
    ///
    /// C++ order: the body logs the ACK, then `Player::ValidateMovementInfo`,
    /// the `_player->m_unitMovedByMe` gate and the `_player->m_movementInfo`
    /// write run inside `WorldSession::apply_knock_back_ack_like_cpp`, which
    /// lives on the World player shell that the handler context does not
    /// reach. The thunk applies that host step at this point and resumes in
    /// [`Self::finish_move_knock_back_ack`] with its accepted flag.
    pub async fn handle_move_knock_back_ack(
        &mut self,
        pkt: MoveKnockBackAck,
    ) -> MoveKnockBackAckStepLikeCpp {
        trace!(
            account = self.hub.shared().core.account_id,
            has_speeds = pkt.speeds.is_some(),
            "MoveKnockBackAck"
        );
        MoveKnockBackAckStepLikeCpp { ack: pkt.ack }
    }

    /// Resumes C++ `HandleMoveKnockBackAck` after the host applied the
    /// validated active-mover gate and the player movement-info write.
    pub async fn finish_move_knock_back_ack(
        &mut self,
        step: MoveKnockBackAckStepLikeCpp,
        accepted: bool,
    ) {
        if accepted {
            let mut status = step.ack.status.clone();
            let Some(adjusted_time) = self.hub.shared().resolved_player_movement_time_like_cpp()
            else {
                return;
            };
            status.time = adjusted_time;
            self.hub.shared().broadcast_to_movement_set_like_cpp(
                MoveUpdateKnockBack { status }.to_bytes(),
                false,
            );
        }
    }

    /// Handle C++ `HandleMoveApplyMovementForceAck` (`MovementHandler.cpp:581`).
    ///
    /// C++ order: the body logs the ACK, then `Player::ValidateMovementInfo`,
    /// the `mover->GetGUID()` gate and the ACK record run inside
    /// `WorldSession::record_apply_movement_force_ack_like_cpp`, which lives
    /// on the World player shell that the handler context does not reach. The
    /// thunk applies that host step at this point and resumes in
    /// [`Self::finish_move_apply_movement_force_ack`] with its accepted flag.
    pub async fn handle_move_apply_movement_force_ack(
        &mut self,
        pkt: MoveApplyMovementForceAck,
    ) -> MoveApplyMovementForceAckStepLikeCpp {
        trace!(
            account = self.hub.shared().core.account_id,
            force = ?pkt.force.id,
            "MoveApplyMovementForceAck"
        );
        MoveApplyMovementForceAckStepLikeCpp {
            ack: pkt.ack,
            force: pkt.force,
        }
    }

    /// Resumes C++ `HandleMoveApplyMovementForceAck` after the host applied
    /// the validated active-mover gate and recorded the ACK.
    pub async fn finish_move_apply_movement_force_ack(
        &mut self,
        step: MoveApplyMovementForceAckStepLikeCpp,
        accepted: bool,
    ) {
        if accepted
            && let Some(source_position) = self
                .hub
                .shared()
                .mover_position_like_cpp(step.ack.status.guid)
        {
            self.hub
                .shared()
                .broadcast_from_movement_source_set_like_cpp(
                    step.ack.status.guid,
                    source_position,
                    MoveUpdateApplyMovementForce {
                        status: step.ack.status,
                        force: step.force,
                    }
                    .to_bytes(),
                    wow_world_core::map_manager::VISIBILITY_RADIUS,
                );
        }
    }

    /// Handle C++ `HandleMoveRemoveMovementForceAck` (`MovementHandler.cpp:603`).
    ///
    /// C++ order: the body logs the ACK, then `Player::ValidateMovementInfo`,
    /// the `mover->GetGUID()` gate and the ACK record run inside
    /// `WorldSession::record_remove_movement_force_ack_like_cpp`, which lives
    /// on the World player shell that the handler context does not reach. The
    /// thunk applies that host step at this point and resumes in
    /// [`Self::finish_move_remove_movement_force_ack`] with its accepted flag.
    pub async fn handle_move_remove_movement_force_ack(
        &mut self,
        pkt: MoveRemoveMovementForceAck,
    ) -> MoveRemoveMovementForceAckStepLikeCpp {
        trace!(
            account = self.hub.shared().core.account_id,
            force = ?pkt.id,
            "MoveRemoveMovementForceAck"
        );
        MoveRemoveMovementForceAckStepLikeCpp {
            ack: pkt.ack,
            id: pkt.id,
        }
    }

    /// Resumes C++ `HandleMoveRemoveMovementForceAck` after the host applied
    /// the validated active-mover gate and recorded the ACK.
    pub async fn finish_move_remove_movement_force_ack(
        &mut self,
        step: MoveRemoveMovementForceAckStepLikeCpp,
        accepted: bool,
    ) {
        if accepted
            && let Some(source_position) = self
                .hub
                .shared()
                .mover_position_like_cpp(step.ack.status.guid)
        {
            self.hub
                .shared()
                .broadcast_from_movement_source_set_like_cpp(
                    step.ack.status.guid,
                    source_position,
                    MoveUpdateRemoveMovementForce {
                        status: step.ack.status,
                        trigger_guid: step.id,
                    }
                    .to_bytes(),
                    wow_world_core::map_manager::VISIBILITY_RADIUS,
                );
        }
    }
}

/// Host step that finishes one C++ `HandleMoveSetVehicleRecAck` call.
///
/// C++ runs `GetPlayer()->ValidateMovementInfo(&setVehicleRecIdAck.Data.Status)`
/// on the World player shell, which the handler context does not reach, so the
/// thunk applies it where this step says the C++ body runs it.
pub enum MoveSetVehicleRecIdAckStepLikeCpp {
    /// C++ order: after the owner logs the ACK, the host applies
    /// `WorldSession::apply_move_set_vehicle_rec_id_ack_like_cpp` to the ACK
    /// status.
    ApplyMoveSetVehicleRecIdAck { ack: MovementAck },
}

/// Host step that finishes one C++ `HandleSetCollisionHeightAck` call.
///
/// C++ runs `GetPlayer()->ValidateMovementInfo(&setCollisionHeightAck.Data.Status)`
/// through the session's validated-ACK path, which lives on the World shell
/// that the handler context does not reach, so the thunk applies it where this
/// step says the C++ body runs it.
pub enum MoveSetCollisionHeightAckStepLikeCpp {
    /// C++ order: after the owner logs the ACK, the host applies
    /// `WorldSession::record_validated_movement_ack_like_cpp` to the ACK.
    RecordValidatedAck { ack: MovementAck },
}

/// Host step that carries one logged C++ `HandleMoveKnockBackAck` ACK across
/// the World player-shell gate and into the accepted-mover broadcast.
pub struct MoveKnockBackAckStepLikeCpp {
    /// ACK the host validates in place before the broadcast decides.
    pub ack: MovementAck,
}

/// Host step that carries one logged C++ `HandleMoveApplyMovementForceAck`
/// ACK and force across the World player-shell gate and into the
/// accepted-mover broadcast.
pub struct MoveApplyMovementForceAckStepLikeCpp {
    /// ACK the host validates in place before the broadcast decides.
    pub ack: MovementAck,
    /// Movement force the C++ body echoes back to the movement set.
    pub force: MovementForce,
}

/// Host step that carries one logged C++ `HandleMoveRemoveMovementForceAck`
/// ACK and trigger GUID across the World player-shell gate and into the
/// accepted-mover broadcast.
pub struct MoveRemoveMovementForceAckStepLikeCpp {
    /// ACK the host validates in place before the broadcast decides.
    pub ack: MovementAck,
    /// Trigger GUID the C++ body echoes back to the movement set.
    pub id: ObjectGuid,
}

/// Builds a movement handler context from a host's hub.
pub trait MovementHandlerHostLikeCpp<C> {
    fn movement_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> MovementHandlerCxLikeCpp<'a>;

    /// C++ `HandleMoveSetVehicleRecAck` runs `Player::ValidateMovementInfo`
    /// on the ACK status, which lives on the World player shell that the
    /// handler context does not reach.
    fn apply_move_set_vehicle_rec_id_ack_like_cpp(&mut self, ack: &mut MovementAck);

    /// C++ `HandleSetCollisionHeightAck` validates the ACK status and records
    /// the validated movement ACK through
    /// `WorldSession::record_validated_movement_ack_like_cpp`, which lives on
    /// the World session shell that the handler context does not reach.
    fn record_validated_movement_ack_like_cpp(
        &mut self,
        opcode: ClientOpcodes,
        ack: &mut MovementAck,
        speed: Option<f32>,
    ) -> bool;

    /// C++ `HandleMoveKnockBackAck` (`MovementHandler.cpp:548`) runs
    /// `Player::ValidateMovementInfo`, the `m_unitMovedByMe` gate and the
    /// `m_movementInfo` write through
    /// `WorldSession::apply_knock_back_ack_like_cpp`, which lives on the World
    /// player shell that the handler context does not reach.
    fn apply_knock_back_ack_like_cpp(
        &mut self,
        opcode: ClientOpcodes,
        ack: &mut MovementAck,
    ) -> bool;

    /// C++ `HandleMoveApplyMovementForceAck` (`MovementHandler.cpp:581`) runs
    /// the same validated active-mover gate and records the ACK through
    /// `WorldSession::record_apply_movement_force_ack_like_cpp`, which lives
    /// on the World player shell that the handler context does not reach.
    fn record_apply_movement_force_ack_like_cpp(
        &mut self,
        ack: &mut MovementAck,
        force: &MovementForce,
    ) -> bool;

    /// C++ `HandleMoveRemoveMovementForceAck` (`MovementHandler.cpp:603`) runs
    /// the same validated active-mover gate and records the ACK through
    /// `WorldSession::record_remove_movement_force_ack_like_cpp`, which lives
    /// on the World player shell that the handler context does not reach.
    fn record_remove_movement_force_ack_like_cpp(
        &mut self,
        ack: &mut MovementAck,
        force_id: ObjectGuid,
    ) -> bool;
}

fn handle_set_active_mover_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: MovementHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match wow_packet::packets::movement::SetActiveMover::read(&mut pkt) {
            Ok(mover) => {
                session
                    .movement_handler_cx_like_cpp(catalogs)
                    .handle_set_active_mover(mover)
                    .await
            }
            Err(e) => tracing::warn!("Failed to read SetActiveMover: {e}"),
        }
    })
}

fn handle_move_time_skipped_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: MovementHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match wow_packet::packets::movement::MoveTimeSkipped::read(&mut pkt) {
            Ok(skipped) => {
                session
                    .movement_handler_cx_like_cpp(catalogs)
                    .handle_move_time_skipped(skipped)
                    .await
            }
            Err(e) => tracing::warn!("Failed to read MoveTimeSkipped: {e}"),
        }
    })
}

fn handle_move_set_vehicle_rec_id_ack_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: MovementHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let opcode = pkt
            .client_opcode()
            .unwrap_or(ClientOpcodes::MoveSetVehicleRecIdAck);
        match wow_packet::packets::vehicle::MoveSetVehicleRecIdAck::read(&mut pkt) {
            Ok(ack) => {
                let step = session
                    .movement_handler_cx_like_cpp(catalogs)
                    .handle_move_set_vehicle_rec_id_ack(opcode, ack)
                    .await;
                let MoveSetVehicleRecIdAckStepLikeCpp::ApplyMoveSetVehicleRecIdAck { mut ack } =
                    step;
                session.apply_move_set_vehicle_rec_id_ack_like_cpp(&mut ack);
            }
            Err(e) => tracing::warn!("Failed to read MoveSetVehicleRecIdAck: {e}"),
        }
    })
}

fn handle_move_set_collision_height_ack_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: MovementHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match wow_packet::packets::movement::MoveSetCollisionHeightAck::read(&mut pkt) {
            Ok(ack) => {
                let step = session
                    .movement_handler_cx_like_cpp(catalogs)
                    .handle_move_set_collision_height_ack(ack)
                    .await;
                let MoveSetCollisionHeightAckStepLikeCpp::RecordValidatedAck { mut ack } = step;
                session.record_validated_movement_ack_like_cpp(
                    ClientOpcodes::MoveSetCollisionHeightAck,
                    &mut ack,
                    None,
                );
            }
            Err(e) => tracing::warn!("Failed to read MoveSetCollisionHeightAck: {e}"),
        }
    })
}

fn handle_move_knock_back_ack_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: MovementHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match wow_packet::packets::movement::MoveKnockBackAck::read(&mut pkt) {
            Ok(ack) => {
                let mut step = session
                    .movement_handler_cx_like_cpp(catalogs)
                    .handle_move_knock_back_ack(ack)
                    .await;
                let accepted = session
                    .apply_knock_back_ack_like_cpp(ClientOpcodes::MoveKnockBackAck, &mut step.ack);
                session
                    .movement_handler_cx_like_cpp(catalogs)
                    .finish_move_knock_back_ack(step, accepted)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read MoveKnockBackAck: {e}"),
        }
    })
}

fn handle_move_apply_movement_force_ack_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: MovementHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match wow_packet::packets::movement::MoveApplyMovementForceAck::read(&mut pkt) {
            Ok(ack) => {
                let mut step = session
                    .movement_handler_cx_like_cpp(catalogs)
                    .handle_move_apply_movement_force_ack(ack)
                    .await;
                let accepted =
                    session.record_apply_movement_force_ack_like_cpp(&mut step.ack, &step.force);
                session
                    .movement_handler_cx_like_cpp(catalogs)
                    .finish_move_apply_movement_force_ack(step, accepted)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read MoveApplyMovementForceAck: {e}"),
        }
    })
}

fn handle_move_remove_movement_force_ack_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: MovementHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match wow_packet::packets::movement::MoveRemoveMovementForceAck::read(&mut pkt) {
            Ok(ack) => {
                let mut step = session
                    .movement_handler_cx_like_cpp(catalogs)
                    .handle_move_remove_movement_force_ack(ack)
                    .await;
                let accepted =
                    session.record_remove_movement_force_ack_like_cpp(&mut step.ack, step.id);
                session
                    .movement_handler_cx_like_cpp(catalogs)
                    .finish_move_remove_movement_force_ack(step, accepted)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read MoveRemoveMovementForceAck: {e}"),
        }
    })
}

pub fn register_movement_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: MovementHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetActiveMover,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_active_mover",
        handler: handle_set_active_mover_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveTimeSkipped,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_move_time_skipped",
        handler: handle_move_time_skipped_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetVehicleRecIdAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_set_vehicle_rec_id_ack",
        handler: handle_move_set_vehicle_rec_id_ack_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetCollisionHeightAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_set_collision_height_ack",
        handler: handle_move_set_collision_height_ack_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveKnockBackAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_knock_back_ack",
        handler: handle_move_knock_back_ack_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveApplyMovementForceAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_apply_movement_force_ack",
        handler: handle_move_apply_movement_force_ack_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MoveRemoveMovementForceAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_remove_movement_force_ack",
        handler: handle_move_remove_movement_force_ack_thunk::<S, C>,
    })?;
    Ok(())
}
