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

#[cfg(test)]
use tracing::debug;

#[cfg(test)]
use crate::session::mailbox::{SendAddonIfRegisteredLikeCppCommand, SessionCommand};
#[cfg(test)]
use wow_chat::hyperlinks::check_all_links_shape_like_cpp;
#[cfg(test)]
use wow_chat::validation::validate_message_like_cpp;
#[cfg(test)]
use wow_constants::UnitState;
#[cfg(test)]
use wow_core::ObjectGuid;
#[cfg(test)]
use wow_core::guid::HighGuid;

#[cfg(test)]
use wow_packet::packets::chat::{
    CTextEmote, ChatAddonMessage, ChatAddonMessageTargeted, ChatAddonMessageWhisper, ChatMessage,
    ChatMessageAfk, ChatMessageChannel, ChatMessageDnd, ChatMessageEmote, ChatMessageWhisper,
    ChatMsg, ChatPkt, ChatPlayerNotfound, ChatRegisterAddonPrefixes, ChatReportFiltered,
    ChatReportIgnored, EmoteClient, EmoteMessage, PrintNotification, STextEmote, UpdateAadcStatus,
    UpdateAadcStatusResponse,
};
#[cfg(test)]
use wow_packet::{ClientPacket, ServerPacket};
#[cfg(test)]
use wow_social::group::GroupInfo;

#[cfg(test)]
use crate::session::{
    ChatFloodThrottleIndexLikeCpp, ChatPolicyCatalogsLikeCpp, PlayerAwayModeLikeCpp, WorldSession,
    player_team_for_race_cpp,
};

/// Chat constants consumed by the moved handlers and their scenario module.
#[cfg(test)]
pub(crate) use wow_world_social::{
    GM_SILENCE_AURA_LIKE_CPP, LANG_ADDON_LIKE_CPP, LANG_ADDON_LOGGED_LIKE_CPP,
    LANG_UNIVERSAL_LIKE_CPP, secs_to_full_time_string_like_cpp,
};

/// Emote ids used by the chat scenarios; the text-emote body moved to
/// `wow_world_social::text_emote` (#1363 slice 4a).
#[cfg(test)]
pub(crate) use wow_world_social::{
    ANIM_MOUNT_SPECIAL_LIKE_CPP, EMOTE_STATE_DANCE_LIKE_CPP, EMOTE_STATE_SIT_LIKE_CPP,
};

mod ops_2;
#[allow(unused_imports)]
pub use ops_2::*;

/// Test-only entry points for the moved chat handlers.
#[cfg(test)]
mod test_shims;

#[cfg(test)]
#[path = "../../unit_tests/handlers/chat/tests/mod.rs"]
mod tests;

// ── Handler implementations ───────────────────────────────────────
