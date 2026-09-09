// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Chat packet handlers — CMSG_CHAT_MESSAGE_*.
//!
//! Say / Yell / Emote messages are broadcast to nearby players on the same map
//! via the shared PlayerRegistry. Whispers are forwarded to the named target if
//! they are online; otherwise echoed back as a "not found" message.
//!
//! Broadcast ranges use C++ `ListenRange.Say`, `ListenRange.TextEmote`, and
//! `ListenRange.Yell` from `World.cpp`.
//!
//! Reference: C++ `Handlers/ChatHandler.cpp`, `Entities/Player/Player.cpp`,
//! and `Server/Packets/ChatPackets.cpp`.

use tracing::debug;

use crate::session::mailbox::{SendAddonIfRegisteredLikeCppCommand, SessionCommand};
use wow_chat::hyperlinks::check_all_links_shape_like_cpp;
use wow_chat::validation::validate_message_like_cpp;
use wow_constants::{ClientOpcodes, UnitState};
use wow_core::ObjectGuid;
use wow_core::guid::HighGuid;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::packets::chat::{
    CTextEmote, ChatAddonMessage, ChatAddonMessageTargeted, ChatAddonMessageWhisper, ChatMessage,
    ChatMessageAfk, ChatMessageChannel, ChatMessageDnd, ChatMessageEmote, ChatMessageWhisper,
    ChatMsg, ChatPkt, ChatPlayerNotfound, ChatRegisterAddonPrefixes, ChatReportFiltered,
    ChatReportIgnored, EmoteClient, EmoteMessage, PrintNotification, STextEmote, UpdateAadcStatus,
    UpdateAadcStatusResponse,
};
use wow_packet::{ClientPacket, ServerPacket};
use wow_social::group::GroupInfo;

use crate::session::{
    ChatFloodThrottleIndexLikeCpp, ChatPolicyCatalogsLikeCpp, PlayerAwayModeLikeCpp,
    SPELL_AURA_INTERRUPT_FLAG_ANIM_LIKE_CPP, WorldSession, player_team_for_race_cpp,
};

mod ops_1;
mod ops_2;
mod state;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use ops_2::*;
#[allow(unused_imports)]
pub use state::*;

#[cfg(test)]
#[path = "chat/tests/mod.rs"]
mod tests;

// ── Handler registrations ─────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageSay,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_say",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::Say, catalogs.chat_policy.as_ref()).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageYell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_yell",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::Yell, catalogs.chat_policy.as_ref()).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageParty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_party",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::Party, catalogs.chat_policy.as_ref()).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageGuild,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_guild",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::Guild, catalogs.chat_policy.as_ref()).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageOfficer,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_officer",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::Officer, catalogs.chat_policy.as_ref()).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageRaid,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_raid",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::Raid, catalogs.chat_policy.as_ref()).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageRaidWarning,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_raid_warning",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::RaidWarning, catalogs.chat_policy.as_ref()).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageInstanceChat,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_instance",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::InstanceChat, catalogs.chat_policy.as_ref()).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageWhisper,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_whisper",
        handler: |session, catalogs, pkt| Box::pin(async move { session.handle_chat_whisper_with_policy_like_cpp(pkt, catalogs.chat_policy.as_ref()).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageChannel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_message",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_channel_message_with_policy_like_cpp(pkt, catalogs.chat_policy.as_ref()).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageAfk,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_afk",
        handler: |session, catalogs, pkt| Box::pin(async move { session.handle_chat_afk_with_policy_like_cpp(pkt, catalogs.chat_policy.as_ref()).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageDnd,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_dnd",
        handler: |session, catalogs, pkt| Box::pin(async move { session.handle_chat_dnd_with_policy_like_cpp(pkt, catalogs.chat_policy.as_ref()).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateAadcStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_update_aadc_status",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_update_aadc_status(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatReportIgnored,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_report_ignored",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_chat_report_ignored(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatReportFiltered,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_report_filtered",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_chat_report_filtered(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageEmote,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_emote",
        handler: |session, catalogs, pkt| Box::pin(async move { session.handle_chat_emote_with_policy_like_cpp(pkt, catalogs.chat_policy.as_ref()).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::Emote,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_emote",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_emote(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SendTextEmote,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_text_emote",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_text_emote_with_catalogs_like_cpp(
                        catalogs.emotes_text.as_ref(),
                        catalogs.emotes.as_ref(),
                        catalogs.chat_policy.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatRegisterAddonPrefixes,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_register_addon_prefixes",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_chat_register_addon_prefixes(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatAddonMessage,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_addon_message",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_addon_message_with_policy_like_cpp(pkt, catalogs.chat_policy.as_ref()).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatAddonMessageWhisper,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_addon_message_whisper",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_addon_message_whisper_with_policy_like_cpp(pkt, catalogs.chat_policy.as_ref()).await })
        },
    }
}

// ── Handler implementations ───────────────────────────────────────
