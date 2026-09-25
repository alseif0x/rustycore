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

mod channels;
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

mod registrations;

// ── Handler implementations ───────────────────────────────────────
