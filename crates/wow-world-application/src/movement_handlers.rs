// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement handlers for the C++ `MovementHandler.cpp` family (#1263 F5).
//!
//! C++ source of truth: `src/server/game/Handlers/MovementHandler.cpp`
//! (`HandleMoveTimeSkippedOpcode` and `HandleSetActiveMoverOpcode`, the
//! handlers behind `CMSG_MOVE_TIME_SKIPPED` and `CMSG_SET_ACTIVE_MOVER`).
//! Both bodies keep their original parse, gates, order, log strings and
//! packets; the session only lends its hub, so the World side owns the host
//! thunks and the `MovementHandlerHostLikeCpp` implementation. The remaining
//! `CMSG_MOVE_*` family stays in `wow-world` for later slices.

use tracing::{info, trace, warn};
use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::movement::{MoveSkipTime, MoveTimeSkipped, SetActiveMover};
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
}

/// Builds a movement handler context from a host's hub.
pub trait MovementHandlerHostLikeCpp<C> {
    fn movement_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> MovementHandlerCxLikeCpp<'a>;
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
    Ok(())
}
