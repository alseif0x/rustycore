// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `CMSG_SEND_TEXT_EMOTE` (#1363 slice 4a; moved from the World
//! `handlers/chat/ops_1.rs`). C++ ref: `WorldSession::HandleTextEmoteOpcode`.
//! The registration stays in `chat_handlers`; the aura interrupt is the one
//! World operation the body still calls.

use tracing::debug;
use wow_constants::UnitState;
use wow_core::ObjectGuid;
use wow_core::guid::HighGuid;
use wow_packet::packets::chat::{CTextEmote, EmoteMessage, STextEmote};
use wow_packet::{ClientPacket, ServerPacket};
use wow_world_core::session::{ChatPolicyCatalogsLikeCpp, SessionHubHostLikeCpp};

/// The World operation the moved text-emote body still calls.
pub trait TextEmoteHostLikeCpp: SessionHubHostLikeCpp {
    fn remove_auras_with_interrupt_flags_like_cpp(&mut self, flags: u32, flags2: u32) -> usize;
}

/// Emote ids and states used by the text-emote handler.
pub const EMOTE_ONESHOT_NONE_LIKE_CPP: i32 = 0;
pub const EMOTE_STATE_DANCE_LIKE_CPP: i32 = 10;
pub const EMOTE_STATE_SLEEP_LIKE_CPP: i32 = 12;
pub const EMOTE_STATE_SIT_LIKE_CPP: i32 = 13;
pub const EMOTE_STATE_KNEEL_LIKE_CPP: i32 = 68;
pub const EMOTE_STATE_READ_LIKE_CPP: i32 = 483;
pub const ANIM_MOUNT_SPECIAL_LIKE_CPP: i32 = 94;
pub const ANIM_MOUNT_SELF_SPECIAL_LIKE_CPP: i32 = 636;
pub const SPELL_AURA_INTERRUPT_FLAG_ANIM_LIKE_CPP: u32 = 0x0000_0020;

/// C++ ref: `WorldSession::ValidateHyperlinksAndMaybeKick`.
/// Handle say/yell/party/guild/raid/instance chat messages.
/// Handle whisper messages.
/// Handle CMSG_CHAT_MESSAGE_CHANNEL.
///
/// C++ routes this through `HandleChatMessage(CHAT_MSG_CHANNEL, ...)`.
/// The Rust parser and validation are represented, but the final
/// ChannelMgr lookup/fanout is intentionally parked until live channels
/// exist.
/// Handle CMSG_UPDATE_AADC_STATUS.
///
/// C++ ignores the requested state because disabling chat is unsupported,
/// then sends success with ChatDisabled=false so the client restores its cvar.
/// Handle CMSG_CHAT_MESSAGE_AFK.
/// Handle CMSG_CHAT_MESSAGE_DND.
/// Handle CMSG_CHAT_REPORT_IGNORED.
///
/// C++ ref: `WorldSession::HandleChatIgnoredOpcode`.
/// The receiver's client sends this after locally ignoring a chat message;
/// the server notifies the ignored player with `CHAT_MSG_IGNORED`.
/// Handle CMSG_CHAT_REPORT_FILTERED.
///
/// C++ ref: `WorldSession::HandleChatReportFiltered`.
/// TrinityCore currently reads an empty packet and only logs a TODO for the
/// unimplemented spam reporting system.
/// Handle emote text (/e).
/// Handle CMSG_SEND_TEXT_EMOTE — player performs a text emote (/wave, /dance…).
///
/// C++ ref: `WorldSession::HandleTextEmoteOpcode`.
pub async fn handle_text_emote_with_catalogs_like_cpp<H: TextEmoteHostLikeCpp>(
    host: &mut H,
    emotes_text: &wow_data::EmotesTextStore,
    emotes: &wow_data::EmotesStore,
    chat_policy: &ChatPolicyCatalogsLikeCpp,
    mut pkt: wow_packet::WorldPacket,
) {
    let msg = match CTextEmote::read(&mut pkt) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(
                account = host.core().account_id,
                "Bad CMSG_SEND_TEXT_EMOTE: {e}"
            );
            return;
        }
    };

    if host.hub_ref().resolved_player_is_alive_like_cpp() != Some(true) {
        return;
    }
    if crate::send_wait_before_speaking_notification_if_muted_like_cpp(&host.hub_ref()) {
        return;
    }

    let Some(text_emote_id) = u32::try_from(msg.emote_id).ok() else {
        return;
    };
    let Some(emote) = emotes_text
        .get(text_emote_id)
        .map(|entry| i32::from(entry.emote_id))
    else {
        return;
    };

    debug!(
        account = host.core().account_id,
        emote_id = msg.emote_id,
        sound_index = msg.sound_index,
        "CMSG_SEND_TEXT_EMOTE"
    );

    let (player_guid, _name) = crate::player_name_and_guid_like_cpp(&host.hub_ref());
    let account_guid =
        ObjectGuid::create_global(HighGuid::WowAccount, 0, host.core().account_id as i64);

    let text_emote = STextEmote {
        source_guid: player_guid,
        source_account_guid: account_guid,
        emote_id: msg.emote_id,
        sound_index: msg.sound_index,
        target_guid: msg.target,
    };

    let text_emote_range = chat_policy.listen_ranges.text_emote;
    let anim_emote = match emote {
        EMOTE_STATE_SLEEP_LIKE_CPP
        | EMOTE_STATE_SIT_LIKE_CPP
        | EMOTE_STATE_KNEEL_LIKE_CPP
        | EMOTE_ONESHOT_NONE_LIKE_CPP => None,
        // Local C++ source of truth:
        // `WorldSession::HandleTextEmoteOpcode` only routes DANCE and READ
        // through `SetEmoteState` in this branch.
        EMOTE_STATE_DANCE_LIKE_CPP | EMOTE_STATE_READ_LIKE_CPP => {
            publish_player_emote_state_like_cpp(host, emote as u32);
            None
        }
        _ if host.core().player_has_unit_state_like_cpp(UnitState::DIED) => None,
        _ => Some(EmoteMessage {
            guid: player_guid,
            emote_id: emote,
            spell_visual_kit_ids: spell_visual_kit_ids_for_emote_command_like_cpp(
                emotes,
                emote,
                &msg.spell_visual_kit_ids,
            ),
            sequence_variation: msg.sequence_variation,
        }),
    };

    if let Some(anim_emote) = anim_emote {
        host.send_packet(&anim_emote);
        host.hub_ref()
            .broadcast_to_movement_set_like_cpp(anim_emote.to_bytes(), false);
    }
    host.send_packet(&text_emote);
    host.hub_ref()
        .broadcast_to_movement_set_in_range_like_cpp(text_emote.to_bytes(), text_emote_range);
    // C++ then resolves `ObjectAccessor::GetUnit(*_player, packet.Target)` for
    // `CriteriaType::DoEmote` and `CreatureAI::ReceiveEmote`. Rust has no
    // live chat->criteria/CreatureAI bridge here yet; keep the C++ packet
    // order, target GUID, and fanout gates above while leaving those
    // post-broadcast side effects explicit.
    if emote != EMOTE_ONESHOT_NONE_LIKE_CPP {
        host.remove_auras_with_interrupt_flags_like_cpp(SPELL_AURA_INTERRUPT_FLAG_ANIM_LIKE_CPP, 0);
    }
}

pub fn spell_visual_kit_ids_for_emote_command_like_cpp(
    emotes: &wow_data::EmotesStore,
    emote: i32,
    client_spell_visual_kit_ids: &[i32],
) -> Vec<i32> {
    let Some(emote_id) = u32::try_from(emote).ok() else {
        return Vec::new();
    };
    let is_mount_special = emotes
        .get(emote_id)
        .map(|entry| {
            entry.anim_id == ANIM_MOUNT_SPECIAL_LIKE_CPP
                || entry.anim_id == ANIM_MOUNT_SELF_SPECIAL_LIKE_CPP
        })
        .unwrap_or(false);

    if is_mount_special {
        client_spell_visual_kit_ids.to_vec()
    } else {
        Vec::new()
    }
}

pub fn publish_player_emote_state_like_cpp<H: TextEmoteHostLikeCpp>(
    host: &mut H,
    emote_state: u32,
) {
    if let Some(update) = host.hub_mut().set_player_emote_state_like_cpp(emote_state) {
        host.send_packet(&update);
        host.hub_ref()
            .broadcast_to_movement_set_like_cpp(update.to_bytes(), false);
    }
}
