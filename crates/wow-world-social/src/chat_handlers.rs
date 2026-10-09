// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Chat, channel and addon message packet handlers.
//!
//! C++ source of truth: `src/server/game/Handlers/ChatHandler.cpp` and the
//! `ChatHandler`/`Channel`/`ChannelMgr` runtime those commands reach. The family
//! owns the packet bodies, the chat policy gates, the channel commands and the
//! flood/link validation seams; the World session only builds the borrowed
//! context from its social state, the hub and the process chat policy (#1263 F5).

use tracing::{debug, warn};
use wow_chat::hyperlinks::check_all_links_shape_like_cpp;
use wow_chat::validation::validate_message_like_cpp;
use wow_constants::{ClientOpcodes, UnitState};
use wow_core::ObjectGuid;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::chat::{
    ChannelCommand, ChannelNotify, ChannelPassword, ChannelPlayerCommand, ChatAddonMessage,
    ChatAddonMessageTargeted, ChatAddonMessageWhisper, ChatMessage, ChatMessageAfk,
    ChatMessageChannel, ChatMessageDnd, ChatMessageEmote, ChatMessageWhisper, ChatMsg, ChatPkt,
    ChatPlayerNotfound, ChatRegisterAddonPrefixes, ChatReportFiltered, ChatReportIgnored,
    EmoteClient, JoinChannel, LeaveChannel, MAX_CHANNEL_NAME_STR_LIKE_CPP,
    MAX_CHANNEL_PASS_STR_LIKE_CPP, PrintNotification, UpdateAadcStatus, UpdateAadcStatusResponse,
};
use wow_packet::{ClientPacket, ServerPacket, WorldPacket};
use wow_social::group::GroupInfo;
use wow_world_core::session::mailbox::{SendAddonIfRegisteredLikeCppCommand, SessionCommand};
use wow_world_core::session::state::hub_support::player_team_for_race_cpp;
use wow_world_core::session::{
    ChatPolicyCatalogsLikeCpp, HubMut, HubRef, PacketPublicationAccessLikeCpp,
};

use crate::{ChatFloodThrottleIndexLikeCpp, PlayerAwayModeLikeCpp, SessionSocialLimits};

/// C++ `LanguageMgr::LoadLanguages` accepted language ids.
pub const KNOWN_LANGUAGES_LIKE_CPP: &[i32] = &[
    0, 1, 2, 3, 6, 7, 8, 9, 10, 11, 12, 13, 14, 33, 35, 36, 37, 38, 39, 40, 42, 43, 44, 168, 178,
    179, 180, 181, 182, 183, 184, 285, 287, 288, 290, 291, 292, 293, 294, 295, 296, 297, 298,
];

/// C++ `LANG_UNIVERSAL`, `LANG_ADDON` and `LANG_ADDON_LOGGED` chat languages.
pub const LANG_UNIVERSAL_LIKE_CPP: i32 = 0;
pub const LANG_ADDON_LIKE_CPP: u32 = 183;
pub const LANG_ADDON_LOGGED_LIKE_CPP: u32 = 184;
/// C++ `SPELL_GM_SILENCE` aura id used for the GM silence check.
pub const GM_SILENCE_AURA_LIKE_CPP: i32 = 1852;

/// C++ `EMOTE_ONESHOT_NONE` used to clear the represented emote state.
pub const EMOTE_ONESHOT_NONE_LIKE_CPP: i32 = 0;

/// C++ `WorldSession::GetPlayerName` plus the session guid for chat publication.
pub fn player_name_and_guid_like_cpp(hub: &HubRef<'_>) -> (wow_core::ObjectGuid, String) {
    let guid = hub
        .core
        .player_guid()
        .unwrap_or(wow_core::ObjectGuid::EMPTY);
    let name = hub.player_name_like_cpp().unwrap_or_default();
    (guid, name)
}

/// C++ muted-player notice sent before a chat message is dropped.
pub fn send_wait_before_speaking_notification_if_muted_like_cpp(hub: &HubRef<'_>) -> bool {
    let Some(remaining_secs) = hub.core.mute_time_remaining_secs_like_cpp() else {
        return false;
    };
    hub.core.packet_publication_access_like_cpp().send_packet(
        &wow_packet::packets::chat::PrintNotification {
            notify_text: format!(
                "You must wait {} before speaking again.",
                secs_to_full_time_string_like_cpp(remaining_secs)
            ),
        },
    );
    true
}

pub fn secs_to_full_time_string_like_cpp(time_in_secs: u64) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;

    let secs = time_in_secs % MINUTE;
    let minutes = time_in_secs % HOUR / MINUTE;
    let hours = time_in_secs % DAY / HOUR;
    let days = time_in_secs / DAY;

    let mut text = String::new();
    if days != 0 {
        text.push_str(&days.to_string());
        text.push_str(if days == 1 { " Day " } else { " Days " });
    }
    if hours != 0 {
        text.push_str(&hours.to_string());
        text.push_str(if hours <= 1 { " Hour " } else { " Hours " });
    }
    if minutes != 0 {
        text.push_str(&minutes.to_string());
        text.push_str(if minutes == 1 {
            " Minute "
        } else {
            " Minutes "
        });
    }
    if secs != 0 || (days == 0 && hours == 0 && minutes == 0) {
        text.push_str(&secs.to_string());
        text.push_str(if secs <= 1 { " Second." } else { " Seconds." });
    }
    text
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinChannelPrecheckLikeCpp {
    Continue,
    InvalidName,
    PasswordTooLong,
}

pub fn join_channel_custom_precheck_like_cpp(request: &JoinChannel) -> JoinChannelPrecheckLikeCpp {
    if request.chat_channel_id != 0 {
        return JoinChannelPrecheckLikeCpp::Continue;
    }

    if request
        .channel_name
        .chars()
        .next()
        .is_none_or(|first| first.is_ascii_digit())
    {
        return JoinChannelPrecheckLikeCpp::InvalidName;
    }

    if request.channel_name.chars().count() > MAX_CHANNEL_NAME_STR_LIKE_CPP {
        return JoinChannelPrecheckLikeCpp::InvalidName;
    }

    if request.password.len() > MAX_CHANNEL_PASS_STR_LIKE_CPP {
        return JoinChannelPrecheckLikeCpp::PasswordTooLong;
    }

    JoinChannelPrecheckLikeCpp::Continue
}

pub fn chat_msg_from_i32_like_cpp(value: i32) -> Option<ChatMsg> {
    if value == ChatMsg::Party as i32 {
        Some(ChatMsg::Party)
    } else if value == ChatMsg::Raid as i32 {
        Some(ChatMsg::Raid)
    } else if value == ChatMsg::Guild as i32 {
        Some(ChatMsg::Guild)
    } else if value == ChatMsg::Officer as i32 {
        Some(ChatMsg::Officer)
    } else if value == ChatMsg::Whisper as i32 {
        Some(ChatMsg::Whisper)
    } else if value == ChatMsg::Channel as i32 {
        Some(ChatMsg::Channel)
    } else if value == ChatMsg::InstanceChat as i32 {
        Some(ChatMsg::InstanceChat)
    } else {
        None
    }
}

pub fn is_known_language_like_cpp(language: i32) -> bool {
    KNOWN_LANGUAGES_LIKE_CPP.contains(&language)
}

/// Borrowed inputs of one chat handler invocation.
pub struct ChatHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    social: &'a mut SessionSocialLimits,
    policy: &'a ChatPolicyCatalogsLikeCpp,
}

/// Chat handler packet bodies.
mod ops;

/// Builds a chat handler context from a host's social state, hub and policy.
pub trait ChatHandlerHostLikeCpp<C> {
    fn chat_handler_cx_like_cpp<'a>(&'a mut self, catalogs: &'a C) -> ChatHandlerCxLikeCpp<'a>;

    /// C++ `ChatHandler.cpp` `WorldSession::HandleTextEmoteOpcode`.
    ///
    /// `#1263 F5 remaining families`: the legacy registration closure
    /// destructured the session catalog view (`emotes_text`, `emotes`,
    /// `chat_policy`), so the host receives that view here. The body still runs
    /// in the World shell.
    fn handle_text_emote_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;
}

fn handle_chat_message_say_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_message_with_policy_like_cpp(pkt, ChatMsg::Say)
            .await;
    })
}

fn handle_chat_message_yell_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_message_with_policy_like_cpp(pkt, ChatMsg::Yell)
            .await;
    })
}

fn handle_chat_message_party_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_message_with_policy_like_cpp(pkt, ChatMsg::Party)
            .await;
    })
}

fn handle_chat_message_guild_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_message_with_policy_like_cpp(pkt, ChatMsg::Guild)
            .await;
    })
}

fn handle_chat_message_officer_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_message_with_policy_like_cpp(pkt, ChatMsg::Officer)
            .await;
    })
}

fn handle_chat_message_raid_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_message_with_policy_like_cpp(pkt, ChatMsg::Raid)
            .await;
    })
}

fn handle_chat_message_raidwarning_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_message_with_policy_like_cpp(pkt, ChatMsg::RaidWarning)
            .await;
    })
}

fn handle_chat_message_instancechat_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_message_with_policy_like_cpp(pkt, ChatMsg::InstanceChat)
            .await;
    })
}

fn handle_chat_whisper_with_policy_like_cpp_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_whisper_with_policy_like_cpp(pkt)
            .await;
    })
}

fn handle_chat_channel_message_with_policy_like_cpp_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_channel_message_with_policy_like_cpp(pkt)
            .await;
    })
}

fn handle_chat_afk_with_policy_like_cpp_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_afk_with_policy_like_cpp(pkt)
            .await;
    })
}

fn handle_chat_dnd_with_policy_like_cpp_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_dnd_with_policy_like_cpp(pkt)
            .await;
    })
}

fn handle_update_aadc_status_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_update_aadc_status(pkt)
            .await;
    })
}

fn handle_chat_report_ignored_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_report_ignored(pkt)
            .await;
    })
}

fn handle_chat_report_filtered_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_report_filtered(pkt)
            .await;
    })
}

fn handle_chat_emote_with_policy_like_cpp_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_emote_with_policy_like_cpp(pkt)
            .await;
    })
}

fn handle_chat_register_addon_prefixes_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_register_addon_prefixes(pkt)
            .await;
    })
}

fn handle_chat_addon_message_with_policy_like_cpp_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_addon_message_with_policy_like_cpp(pkt)
            .await;
    })
}

fn handle_chat_addon_message_whisper_with_policy_like_cpp_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_addon_message_whisper_with_policy_like_cpp(pkt)
            .await;
    })
}

fn handle_chat_join_channel_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_join_channel(pkt)
            .await;
    })
}

fn handle_chat_leave_channel_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_leave_channel(pkt)
            .await;
    })
}

fn handle_chat_channel_password_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_channel_password(pkt)
            .await;
    })
}

fn handle_chat_unregister_all_addon_prefixes_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_unregister_all_addon_prefixes(pkt)
            .await;
    })
}

fn handle_chat_channel_command_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_channel_command(pkt)
            .await;
    })
}

fn handle_chat_channel_player_command_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_chat_channel_player_command(pkt)
            .await;
    })
}

fn handle_emote_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .chat_handler_cx_like_cpp(catalogs)
            .handle_emote_like_cpp(pkt)
            .await;
    })
}

/// `#1263 F5 remaining families`: the text-emote entry that lived in the World
/// shell's `handlers/chat/registrations.rs`. It carries the session catalog view
/// because the legacy closure destructured `emotes_text`, `emotes` and
/// `chat_policy`.
fn handle_text_emote_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .handle_text_emote_with_catalogs_like_cpp(catalogs, pkt)
            .await
    })
}

/// Register the chat packet entries through their social owner.
pub fn register_chat_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: ChatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageSay,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_say",
        handler: handle_chat_message_say_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageYell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_yell",
        handler: handle_chat_message_yell_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageParty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_party",
        handler: handle_chat_message_party_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageGuild,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_guild",
        handler: handle_chat_message_guild_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageOfficer,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_officer",
        handler: handle_chat_message_officer_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageRaid,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_raid",
        handler: handle_chat_message_raid_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageRaidWarning,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_raid_warning",
        handler: handle_chat_message_raidwarning_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageInstanceChat,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_instance",
        handler: handle_chat_message_instancechat_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageWhisper,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_whisper",
        handler: handle_chat_whisper_with_policy_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageChannel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_message",
        handler: handle_chat_channel_message_with_policy_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageAfk,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_afk",
        handler: handle_chat_afk_with_policy_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageDnd,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_dnd",
        handler: handle_chat_dnd_with_policy_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateAadcStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_update_aadc_status",
        handler: handle_update_aadc_status_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatReportIgnored,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_report_ignored",
        handler: handle_chat_report_ignored_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatReportFiltered,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_report_filtered",
        handler: handle_chat_report_filtered_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageEmote,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_emote",
        handler: handle_chat_emote_with_policy_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatRegisterAddonPrefixes,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_register_addon_prefixes",
        handler: handle_chat_register_addon_prefixes_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatAddonMessage,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_addon_message",
        handler: handle_chat_addon_message_with_policy_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatAddonMessageWhisper,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_addon_message_whisper",
        handler: handle_chat_addon_message_whisper_with_policy_like_cpp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatJoinChannel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_join_channel",
        handler: handle_chat_join_channel_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatLeaveChannel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_leave_channel",
        handler: handle_chat_leave_channel_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelPassword,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_password",
        handler: handle_chat_channel_password_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatUnregisterAllAddonPrefixes,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_unregister_all_addon_prefixes",
        handler: handle_chat_unregister_all_addon_prefixes_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelAnnouncements,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_command",
        handler: handle_chat_channel_command_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelDeclineInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_command",
        handler: handle_chat_channel_command_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelDisplayList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_command",
        handler: handle_chat_channel_command_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_command",
        handler: handle_chat_channel_command_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelOwner,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_command",
        handler: handle_chat_channel_command_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelBan,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_player_command",
        handler: handle_chat_channel_player_command_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_player_command",
        handler: handle_chat_channel_player_command_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelKick,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_player_command",
        handler: handle_chat_channel_player_command_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelModerator,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_player_command",
        handler: handle_chat_channel_player_command_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelSetOwner,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_player_command",
        handler: handle_chat_channel_player_command_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelSilenceAll,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_player_command",
        handler: handle_chat_channel_player_command_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelUnban,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_player_command",
        handler: handle_chat_channel_player_command_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelUnmoderator,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_player_command",
        handler: handle_chat_channel_player_command_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelUnsilenceAll,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_player_command",
        handler: handle_chat_channel_player_command_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::Emote,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_emote",
        handler: handle_emote_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SendTextEmote,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_text_emote",
        handler: handle_text_emote_thunk::<S, C>,
    })?;
    Ok(())
}
