// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Arena team packet handlers.
//!
//! C++ source of truth: `WorldSession::HandleArenaTeamRosterOpcode`,
//! `HandleArenaTeamAcceptOpcode`, `HandleArenaTeamDeclineOpcode`,
//! `HandleArenaTeamLeaveOpcode`, `HandleArenaTeamRemoveOpcode`,
//! `HandleArenaTeamDisbandOpcode`, `HandleArenaTeamLeaderOpcode` and
//! `HandleQueryArenaTeamOpcode` (`src/server/game/Handlers/ArenaTeamHandler.cpp`).
//! The live `ArenaTeamMgr` is not ported yet, so the family preserves the
//! C++ unknown-team branches instead of inventing roster packets; the World
//! session only builds the borrowed social state plus hub context (#1263 F5).

use tracing::{debug, warn};
use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;
use wow_packet::packets::misc::{
    ArenaTeamAccept, ArenaTeamDecline, ArenaTeamDisband, ArenaTeamLeader, ArenaTeamLeave,
    ArenaTeamRemove, ArenaTeamRoster, QueryArenaTeam,
};
use wow_world_core::session::HubMut;

use crate::SessionSocialLimits;

/// Borrowed inputs of one arena-team handler invocation.
pub struct ArenaTeamHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    social: &'a mut SessionSocialLimits,
}

impl<'a> ArenaTeamHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>, social: &'a mut SessionSocialLimits) -> Self {
        Self { hub, social }
    }

    pub async fn handle_arena_team_roster(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ArenaTeamRoster::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "ArenaTeamRoster parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sArenaTeamMgr has no arena team for TeamId.
        // The live arena-team manager is not ported here yet, so Rust preserves
        // that unknown-team branch instead of inventing an empty roster packet.
        debug!(
            account = self.hub.shared().core.account_id,
            team_id = request.team_id,
            "ArenaTeamRoster ignored without represented arena-team manager"
        );
    }

    pub async fn handle_arena_team_accept(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ArenaTeamAccept::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "ArenaTeamAccept parse failed: {error}"
            );
            return;
        }

        // C++ returns before clearing Player::m_ArenaTeamIdInvited when
        // sArenaTeamMgr has no team for the invited id. Rust has no live
        // ArenaTeamMgr in this represented seam, so preserve that no-op.
        debug!(
            account = self.hub.shared().core.account_id,
            "ArenaTeamAccept ignored without represented arena-team manager"
        );
    }

    pub async fn handle_arena_team_decline(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ArenaTeamDecline::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "ArenaTeamDecline parse failed: {error}"
            );
            return;
        }

        self.social
            .set_represented_arena_team_id_invited_like_cpp(&mut self.hub, 0);
    }

    pub async fn handle_arena_team_leave(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ArenaTeamLeave::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "ArenaTeamLeave parse failed: {error}"
            );
            return;
        }

        // C++ loops arena slots and only acts when sArenaTeamMgr resolves a
        // real team. No represented ArenaTeamMgr exists yet, so the bounded
        // no-team branch is intentionally silent.
        debug!(
            account = self.hub.shared().core.account_id,
            "ArenaTeamLeave ignored without represented arena-team manager"
        );
    }

    pub async fn handle_arena_team_remove(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ArenaTeamRemove::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "ArenaTeamRemove parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sArenaTeamMgr has no arena team for TeamId.
        debug!(
            account = self.hub.shared().core.account_id,
            team_id = request.team_id,
            target_name = %request.target_name,
            "ArenaTeamRemove ignored without represented arena-team manager"
        );
    }

    pub async fn handle_arena_team_disband(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ArenaTeamDisband::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "ArenaTeamDisband parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sArenaTeamMgr has no arena team for TeamId.
        debug!(
            account = self.hub.shared().core.account_id,
            team_id = request.team_id,
            "ArenaTeamDisband ignored without represented arena-team manager"
        );
    }

    pub async fn handle_arena_team_leader(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ArenaTeamLeader::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "ArenaTeamLeader parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sArenaTeamMgr has no arena team for TeamId.
        debug!(
            account = self.hub.shared().core.account_id,
            team_id = request.team_id,
            target_name = %request.target_name,
            "ArenaTeamLeader ignored without represented arena-team manager"
        );
    }

    pub async fn handle_query_arena_team(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match QueryArenaTeam::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "QueryArenaTeam parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sArenaTeamMgr has no arena team for TeamId.
        debug!(
            account = self.hub.shared().core.account_id,
            team_id = request.team_id,
            "QueryArenaTeam ignored without represented arena-team manager"
        );
    }
}

/// Builds an arena-team handler context from a host's social state and hub.
pub trait ArenaTeamHandlerHostLikeCpp<C> {
    fn arena_team_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> ArenaTeamHandlerCxLikeCpp<'a>;
}

fn handle_arena_team_roster_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ArenaTeamHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .arena_team_handler_cx_like_cpp(catalogs)
            .handle_arena_team_roster(pkt)
            .await;
    })
}

fn handle_arena_team_accept_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ArenaTeamHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .arena_team_handler_cx_like_cpp(catalogs)
            .handle_arena_team_accept(pkt)
            .await;
    })
}

fn handle_arena_team_decline_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ArenaTeamHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .arena_team_handler_cx_like_cpp(catalogs)
            .handle_arena_team_decline(pkt)
            .await;
    })
}

fn handle_arena_team_leave_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ArenaTeamHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .arena_team_handler_cx_like_cpp(catalogs)
            .handle_arena_team_leave(pkt)
            .await;
    })
}

fn handle_arena_team_remove_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ArenaTeamHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .arena_team_handler_cx_like_cpp(catalogs)
            .handle_arena_team_remove(pkt)
            .await;
    })
}

fn handle_arena_team_disband_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ArenaTeamHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .arena_team_handler_cx_like_cpp(catalogs)
            .handle_arena_team_disband(pkt)
            .await;
    })
}

fn handle_arena_team_leader_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ArenaTeamHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .arena_team_handler_cx_like_cpp(catalogs)
            .handle_arena_team_leader(pkt)
            .await;
    })
}

fn handle_query_arena_team_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ArenaTeamHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .arena_team_handler_cx_like_cpp(catalogs)
            .handle_query_arena_team(pkt)
            .await;
    })
}

/// Registers the arena-team handlers on the packet registry.
pub fn register_arena_team_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: ArenaTeamHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamRoster,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_roster",
        handler: handle_arena_team_roster_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamAccept,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_accept",
        handler: handle_arena_team_accept_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamDecline,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_decline",
        handler: handle_arena_team_decline_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamLeave,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_leave",
        handler: handle_arena_team_leave_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamRemove,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_remove",
        handler: handle_arena_team_remove_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamDisband,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_disband",
        handler: handle_arena_team_disband_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamLeader,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_leader",
        handler: handle_arena_team_leader_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryArenaTeam,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_arena_team",
        handler: handle_query_arena_team_thunk::<S, C>,
    })?;
    Ok(())
}
