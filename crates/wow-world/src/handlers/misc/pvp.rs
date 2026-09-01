// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private pvp capability handlers extracted from the legacy misc owner.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::gossip::Hello;
use wow_packet::packets::misc::{
    AcceptWargameInvite, BattlefieldLeave, BattlefieldListRequest, BattlefieldPort,
    BattlemasterJoin, BattlemasterJoinArena, BattlemasterJoinSkirmish, RatedPvpInfo,
    RequestBattlefieldStatus, SetPvp, TogglePvp,
};

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestBattlefieldStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_battlefield_status",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_request_battlefield_status(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlemasterHello,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlemaster_hello",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_battlemaster_hello(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlefieldList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlefield_list",
        handler: |session, pkt| Box::pin(async move { session.handle_battlefield_list(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlemasterJoin,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlemaster_join",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_battlemaster_join(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlemasterJoinArena,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlemaster_join_arena",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_battlemaster_join_arena(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlemasterJoinSkirmish,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlemaster_join_skirmish",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_battlemaster_join_skirmish(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlefieldPort,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlefield_port",
        handler: |session, pkt| Box::pin(async move { session.handle_battlefield_port(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestRatedPvpInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_rated_pvp_info",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_request_rated_pvp_info(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlefieldLeave,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlefield_leave",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_battlefield_leave(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AcceptWargameInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_accept_wargame_invite",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_accept_wargame_invite(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestPvpRewards,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_pvp_rewards",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_request_pvp_rewards(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TogglePvp,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_toggle_pvp",
        handler: |session, pkt| Box::pin(async move { session.handle_toggle_pvp(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetPvp,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_pvp",
        handler: |session, pkt| Box::pin(async move { session.handle_set_pvp(pkt).await }),
    }
}

impl crate::session::WorldSession {
    pub async fn handle_request_battlefield_status(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = RequestBattlefieldStatus::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "RequestBattlefieldStatus parse failed: {error}"
            );
            return;
        }

        // C++ iterates PLAYER_MAX_BATTLEGROUND_QUEUES and sends active,
        // confirmation, or queued status only for non-empty queue slots.
        // Rust has no represented battleground queue state in this handler yet,
        // so the no-queue branch is silent.
    }

    /// CMSG_BATTLEMASTER_HELLO — player asks a battlemaster NPC for its queue list.
    /// C++ ref: `WorldSession::HandleBattlemasterHelloOpcode`.

    pub async fn handle_battlemaster_hello(&mut self, mut pkt: wow_packet::WorldPacket) {
        let hello = match Hello::read(&mut pkt) {
            Ok(hello) => hello,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlemasterHello parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when the target cannot be interacted with as a
        // battlemaster. The accepted branch records the list intent until
        // BattlegroundMgr::SendBattlegroundList is live in Rust.
        let _accepted = self.battlemaster_hello_like_cpp(hello.unit);
    }

    /// CMSG_BATTLEFIELD_LIST — player asks for the queue list of a battleground type.
    /// C++ ref: `WorldSession::HandleBattlefieldListOpcode`.

    pub async fn handle_battlefield_list(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match BattlefieldListRequest::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlefieldList parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sBattlemasterListStore has no ListID row.
        // The accepted branch records the SendBattlegroundList intent until
        // BattlegroundMgr owns live queue/list packets in Rust.
        let _accepted = self.battlefield_list_like_cpp(request.list_id);
    }

    /// CMSG_BATTLEMASTER_JOIN — player asks to join a battleground queue.
    /// C++ ref: `WorldSession::HandleBattlemasterJoinOpcode`.

    pub async fn handle_battlemaster_join(&mut self, mut pkt: wow_packet::WorldPacket) {
        let join = match BattlemasterJoin::read(&mut pkt) {
            Ok(join) => join,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlemasterJoin parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently for missing/invalid queues and early queue gates.
        // The accepted branch records the queue intent until BattlegroundQueue
        // and BattlegroundMgr queue-status packets are live in Rust.
        let _accepted =
            self.battlemaster_join_like_cpp(&join.queue_ids, join.roles, join.blacklist_map);
    }

    /// CMSG_BATTLEMASTER_JOIN_ARENA — player asks to join a rated arena queue.
    /// C++ ref: `WorldSession::HandleBattlemasterJoinArena`.

    pub async fn handle_battlemaster_join_arena(&mut self, mut pkt: wow_packet::WorldPacket) {
        let join = match BattlemasterJoinArena::read(&mut pkt) {
            Ok(join) => join,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlemasterJoinArena parse failed: {error}"
                );
                return;
            }
        };

        // C++ gates on already-in-BG, the all-arenas template, disabled arena,
        // group and leader before entering ArenaTeamMgr/queue code. Rust records
        // the bounded queue intent after those representable gates until the
        // live rated-arena manager is ported.
        let _accepted = self.battlemaster_join_arena_like_cpp(join.team_size_index, join.roles);
    }

    /// CMSG_BATTLEMASTER_JOIN_SKIRMISH — player asks to join an arena skirmish queue.
    /// C++ ref: `WorldSession::HandleBattlemasterJoinSkirmish`.

    pub async fn handle_battlemaster_join_skirmish(&mut self, mut pkt: wow_packet::WorldPacket) {
        let join = match BattlemasterJoinSkirmish::read(&mut pkt) {
            Ok(join) => join,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlemasterJoinSkirmish parse failed: {error}"
                );
                return;
            }
        };

        // C++ ignores IsRated here, derives 2v2/3v3/5v5 from BgTypeId/BracketId,
        // and only applies group/leader gates when AsGroup is set. Queue add and
        // status fanout remain represented until live BattlegroundQueue is ported.
        let _accepted = self.battlemaster_join_skirmish_like_cpp(
            join.bg_type_id,
            join.bracket_id,
            join.as_group,
            join.is_rated,
        );
    }

    /// CMSG_BATTLEFIELD_PORT — player accepts an invite or leaves a BG queue slot.
    /// C++ ref: `WorldSession::HandleBattleFieldPortOpcode`.

    pub async fn handle_battlefield_port(&mut self, mut pkt: wow_packet::WorldPacket) {
        let port = match BattlefieldPort::read(&mut pkt) {
            Ok(port) => port,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlefieldPort parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently for not-in-queue, invalid queue slot, and
        // AcceptedInvite without an invitation. The accepted/leave branch is
        // represented only until live BattlegroundQueue/BattlegroundMgr exists.
        let _accepted = self.battlefield_port_like_cpp(port.ticket, port.accepted_invite);
    }

    /// CMSG_BATTLEFIELD_LEAVE — player asks to leave the current battleground.
    /// C++ ref: `WorldSession::HandleBattlefieldLeaveOpcode`.

    pub async fn handle_battlefield_leave(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = BattlefieldLeave::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "BattlefieldLeave parse failed: {error}"
            );
            return;
        }

        if self.resolved_in_combat_like_cpp() != Some(false)
            && self.player_in_represented_battleground_like_cpp()
            && !self.represented_battleground_status_is_wait_leave_like_cpp()
        {
            return;
        }

        self.request_represented_battleground_leave_like_cpp();
    }

    pub async fn handle_accept_wargame_invite(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match AcceptWargameInvite::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "AcceptWargameInvite parse failed: {error}"
                );
                return;
            }
        };

        self.accept_represented_wargame_invite_like_cpp(&packet.inviter_name);
    }

    pub async fn handle_request_rated_pvp_info(&mut self, _pkt: wow_packet::WorldPacket) {
        self.send_packet_realm(&RatedPvpInfo::default());
    }

    pub async fn handle_request_pvp_rewards(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ dispatches to Player::SendPvpRewards(), but that method's
        // SMSG_REQUEST_PVP_REWARDS_RESPONSE send is commented out in the
        // canonical source, so the observable behavior is silence.
    }

    pub async fn handle_toggle_pvp(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = TogglePvp::read(&mut pkt) {
            warn!(account = self.account_id, "TogglePvP parse failed: {error}");
            return;
        }

        self.apply_toggle_pvp_like_cpp();
    }

    pub async fn handle_set_pvp(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetPvp::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(account = self.account_id, "SetPvP parse failed: {error}");
                return;
            }
        };

        self.apply_set_pvp_like_cpp(packet.enable_pvp);
    }
}
