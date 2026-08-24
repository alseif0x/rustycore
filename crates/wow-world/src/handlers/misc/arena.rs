// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private arena capability handlers extracted from the legacy misc owner.

use tracing::{debug, warn};
use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    ArenaTeamAccept, ArenaTeamDecline, ArenaTeamDisband, ArenaTeamLeader, ArenaTeamLeave,
    ArenaTeamRemove, ArenaTeamRoster, QueryArenaTeam,
};

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamRoster,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_roster",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_arena_team_roster(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamAccept,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_accept",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_arena_team_accept(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamDecline,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_decline",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_arena_team_decline(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamLeave,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_leave",
        handler: |session, pkt| Box::pin(async move { session.handle_arena_team_leave(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamRemove,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_remove",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_arena_team_remove(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamDisband,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_disband",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_arena_team_disband(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamLeader,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_leader",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_arena_team_leader(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryArenaTeam,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_arena_team",
        handler: |session, pkt| Box::pin(async move { session.handle_query_arena_team(pkt).await }),
    }
}

impl crate::session::WorldSession {
    pub async fn handle_arena_team_roster(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ArenaTeamRoster::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ArenaTeamRoster parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sArenaTeamMgr has no arena team for TeamId.
        // The live arena-team manager is not ported here yet, so Rust preserves
        // that unknown-team branch instead of inventing an empty roster packet.
        debug!(
            account = self.account_id,
            team_id = request.team_id,
            "ArenaTeamRoster ignored without represented arena-team manager"
        );
    }

    pub async fn handle_arena_team_accept(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ArenaTeamAccept::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "ArenaTeamAccept parse failed: {error}"
            );
            return;
        }

        // C++ returns before clearing Player::m_ArenaTeamIdInvited when
        // sArenaTeamMgr has no team for the invited id. Rust has no live
        // ArenaTeamMgr in this represented seam, so preserve that no-op.
        debug!(
            account = self.account_id,
            "ArenaTeamAccept ignored without represented arena-team manager"
        );
    }

    pub async fn handle_arena_team_decline(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ArenaTeamDecline::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "ArenaTeamDecline parse failed: {error}"
            );
            return;
        }

        self.set_represented_arena_team_id_invited_like_cpp(0);
    }

    pub async fn handle_arena_team_leave(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ArenaTeamLeave::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "ArenaTeamLeave parse failed: {error}"
            );
            return;
        }

        // C++ loops arena slots and only acts when sArenaTeamMgr resolves a
        // real team. No represented ArenaTeamMgr exists yet, so the bounded
        // no-team branch is intentionally silent.
        debug!(
            account = self.account_id,
            "ArenaTeamLeave ignored without represented arena-team manager"
        );
    }

    pub async fn handle_arena_team_remove(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ArenaTeamRemove::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ArenaTeamRemove parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sArenaTeamMgr has no arena team for TeamId.
        debug!(
            account = self.account_id,
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
                    account = self.account_id,
                    "ArenaTeamDisband parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sArenaTeamMgr has no arena team for TeamId.
        debug!(
            account = self.account_id,
            team_id = request.team_id,
            "ArenaTeamDisband ignored without represented arena-team manager"
        );
    }

    pub async fn handle_arena_team_leader(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ArenaTeamLeader::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ArenaTeamLeader parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sArenaTeamMgr has no arena team for TeamId.
        debug!(
            account = self.account_id,
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
                    account = self.account_id,
                    "QueryArenaTeam parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sArenaTeamMgr has no arena team for TeamId.
        debug!(
            account = self.account_id,
            team_id = request.team_id,
            "QueryArenaTeam ignored without represented arena-team manager"
        );
    }
}
