// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private chat capability handlers extracted from the legacy misc owner.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::chat::{
    ChannelCommand, ChannelNotify, ChannelPassword, ChannelPlayerCommand, JoinChannel,
    LeaveChannel, MAX_CHANNEL_NAME_STR_LIKE_CPP, MAX_CHANNEL_PASS_STR_LIKE_CPP,
};

use super::{JoinChannelPrecheckLikeCpp, join_channel_custom_precheck_like_cpp};

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatJoinChannel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_join_channel",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_chat_join_channel(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatLeaveChannel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_leave_channel",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_chat_leave_channel(pkt).await })
        },
    }
}

macro_rules! register_chat_channel_command_handler {
    ($opcode:ident) => {
        inventory::submit! {
            PacketHandlerEntry {
                opcode: ClientOpcodes::$opcode,
                status: SessionStatus::LoggedIn,
                processing: PacketProcessing::ThreadUnsafe,
                handler_name: "handle_chat_channel_command",
                handler: |session, _catalogs, pkt| {
                    Box::pin(async move { session.handle_chat_channel_command(pkt).await })
                },
            }
        }
    };
}

register_chat_channel_command_handler!(ChatChannelAnnouncements);
register_chat_channel_command_handler!(ChatChannelDeclineInvite);
register_chat_channel_command_handler!(ChatChannelDisplayList);
register_chat_channel_command_handler!(ChatChannelList);
register_chat_channel_command_handler!(ChatChannelOwner);

macro_rules! register_chat_channel_player_command_handler {
    ($opcode:ident) => {
        inventory::submit! {
            PacketHandlerEntry {
                opcode: ClientOpcodes::$opcode,
                status: SessionStatus::LoggedIn,
                processing: PacketProcessing::ThreadUnsafe,
                handler_name: "handle_chat_channel_player_command",
                handler: |session, _catalogs, pkt| {
                    Box::pin(async move { session.handle_chat_channel_player_command(pkt).await })
                },
            }
        }
    };
}

register_chat_channel_player_command_handler!(ChatChannelBan);
register_chat_channel_player_command_handler!(ChatChannelInvite);
register_chat_channel_player_command_handler!(ChatChannelKick);
register_chat_channel_player_command_handler!(ChatChannelModerator);
register_chat_channel_player_command_handler!(ChatChannelSetOwner);
register_chat_channel_player_command_handler!(ChatChannelSilenceAll);
register_chat_channel_player_command_handler!(ChatChannelUnban);
register_chat_channel_player_command_handler!(ChatChannelUnmoderator);
register_chat_channel_player_command_handler!(ChatChannelUnsilenceAll);

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelPassword,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_password",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_chat_channel_password(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatUnregisterAllAddonPrefixes,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_unregister_all_addon_prefixes",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_chat_unregister_all_addon_prefixes(pkt).await })
        },
    }
}

impl crate::session::WorldSession {
    /// CMSG_CHAT_JOIN_CHANNEL — player joins a chat channel.
    /// C++ ref: `WorldSession::HandleJoinChannel`.
    pub async fn handle_chat_join_channel(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match JoinChannel::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "JoinChannel parse failed: {error}"
                );
                return;
            }
        };

        match join_channel_custom_precheck_like_cpp(&request) {
            JoinChannelPrecheckLikeCpp::Continue => {}
            JoinChannelPrecheckLikeCpp::InvalidName => {
                self.send_packet(&ChannelNotify::invalid_name(request.channel_name));
                return;
            }
            JoinChannelPrecheckLikeCpp::PasswordTooLong => {
                warn!(
                    account = self.account_id,
                    password_len = request.password.len(),
                    max_password_len = MAX_CHANNEL_PASS_STR_LIKE_CPP,
                    "JoinChannel password too long"
                );
                return;
            }
        }

        // ChannelMgr, system-zone channel validation, custom channel creation,
        // password handling, hyperlink kick checks, and system channel validation
        // are not represented yet.
    }

    /// CMSG_CHAT_LEAVE_CHANNEL.
    /// C++ ref: `WorldSession::HandleLeaveChannel`.

    pub async fn handle_chat_leave_channel(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match LeaveChannel::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "LeaveChannel parse failed: {error}"
                );
                return;
            }
        };

        if request.channel_name.is_empty() && request.zone_channel_id == 0 {
            return;
        }

        // ChannelMgr/system-channel zone validation and LeaveChannel fanout are not
        // represented yet. With no resolved channel this is silent like C++.
    }

    /// CMSG_CHAT_CHANNEL_{ANNOUNCEMENTS,DECLINE_INVITE,DISPLAY_LIST,LIST,OWNER}.
    /// C++ ref: `WorldSession::HandleChannelCommand`.

    pub async fn handle_chat_channel_command(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ChannelCommand::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "ChannelCommand parse failed: {error}"
            );
        }

        // Channel lookup and command execution require ChannelMgr and are not represented
        // yet. Missing channel is silent like C++.
    }

    /// CMSG_CHAT_CHANNEL_* player-targeted commands.
    /// C++ ref: `WorldSession::HandleChannelPlayerCommand`.

    pub async fn handle_chat_channel_player_command(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ChannelPlayerCommand::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ChannelPlayerCommand parse failed: {error}"
                );
                return;
            }
        };

        if request.name.len() >= MAX_CHANNEL_NAME_STR_LIKE_CPP {
            return;
        }

        // normalizePlayerName, ChannelMgr lookup, and the concrete channel action are not
        // represented yet. Missing/invalid channel remains silent like C++.
    }

    /// CMSG_CHAT_CHANNEL_PASSWORD.
    /// C++ ref: `WorldSession::HandleChannelPassword`.

    pub async fn handle_chat_channel_password(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ChannelPassword::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ChannelPassword parse failed: {error}"
                );
                return;
            }
        };

        if request.password.len() > MAX_CHANNEL_PASS_STR_LIKE_CPP {
            return;
        }

        // ChannelMgr lookup and Password() mutation are not represented yet. Missing
        // channel is silent like C++.
    }

    pub async fn handle_chat_unregister_all_addon_prefixes(
        &mut self,
        _pkt: wow_packet::WorldPacket,
    ) {
        self.registered_addon_prefixes.clear();
    }
}
