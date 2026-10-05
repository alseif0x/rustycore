//! Chat handlers operations, part 1 of 2.
//!
//! The inherent `WorldSession` impl is divided by responsibility under
//! #654; every method keeps its original body.

use super::*;

impl WorldSession {
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
    /// Handle CMSG_EMOTE — client notifies us it cleared its emote state.
    ///
    /// C++ ref: `WorldSession::HandleEmoteOpcode`.
    pub async fn handle_emote(&mut self, mut pkt: wow_packet::WorldPacket) {
        // EmoteClient has no body — read returns Ok(()) immediately.
        let _ = EmoteClient::read(&mut pkt);
        if crate::session::hub_ref(self).resolved_player_is_alive_like_cpp() != Some(true)
            || self.core.player_has_unit_state_like_cpp(UnitState::DIED)
        {
            return;
        }

        self.publish_player_emote_state_like_cpp(EMOTE_ONESHOT_NONE_LIKE_CPP as u32);
        debug!(
            account = self.core.account_id,
            "CMSG_EMOTE: clear emote state"
        );
    }
    /// Handle CMSG_SEND_TEXT_EMOTE — player performs a text emote (/wave, /dance…).
    ///
    /// C++ ref: `WorldSession::HandleTextEmoteOpcode`.
    pub(crate) async fn handle_text_emote_with_catalogs_like_cpp(
        &mut self,
        emotes_text: &wow_data::EmotesTextStore,
        emotes: &wow_data::EmotesStore,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let msg = match CTextEmote::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    account = self.core.account_id,
                    "Bad CMSG_SEND_TEXT_EMOTE: {e}"
                );
                return;
            }
        };

        if crate::session::hub_ref(self).resolved_player_is_alive_like_cpp() != Some(true) {
            return;
        }
        if self.send_wait_before_speaking_notification_if_muted_like_cpp() {
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
            account = self.core.account_id,
            emote_id = msg.emote_id,
            sound_index = msg.sound_index,
            "CMSG_SEND_TEXT_EMOTE"
        );

        let (player_guid, _name) = self.player_name_and_guid();
        let account_guid =
            ObjectGuid::create_global(HighGuid::WowAccount, 0, self.core.account_id as i64);

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
                self.publish_player_emote_state_like_cpp(emote as u32);
                None
            }
            _ if self.core.player_has_unit_state_like_cpp(UnitState::DIED) => None,
            _ => Some(EmoteMessage {
                guid: player_guid,
                emote_id: emote,
                spell_visual_kit_ids: self.spell_visual_kit_ids_for_emote_command_like_cpp(
                    emotes,
                    emote,
                    &msg.spell_visual_kit_ids,
                ),
                sequence_variation: msg.sequence_variation,
            }),
        };

        if let Some(anim_emote) = anim_emote {
            self.send_packet(&anim_emote);
            crate::session::hub_ref(self)
                .broadcast_to_movement_set_like_cpp(anim_emote.to_bytes(), false);
        }
        self.send_packet(&text_emote);
        crate::session::hub_ref(self)
            .broadcast_to_movement_set_in_range_like_cpp(text_emote.to_bytes(), text_emote_range);
        // C++ then resolves `ObjectAccessor::GetUnit(*_player, packet.Target)` for
        // `CriteriaType::DoEmote` and `CreatureAI::ReceiveEmote`. Rust has no
        // live chat->criteria/CreatureAI bridge here yet; keep the C++ packet
        // order, target GUID, and fanout gates above while leaving those
        // post-broadcast side effects explicit.
        if emote != EMOTE_ONESHOT_NONE_LIKE_CPP {
            self.remove_auras_with_interrupt_flags_like_cpp(
                SPELL_AURA_INTERRUPT_FLAG_ANIM_LIKE_CPP,
                0,
            );
        }
    }
    pub(super) fn spell_visual_kit_ids_for_emote_command_like_cpp(
        &self,
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
    #[cfg(test)]
    pub async fn handle_text_emote(&mut self, pkt: wow_packet::WorldPacket) {
        let emotes_text = self
            .emotes_text_store_for_test_like_cpp()
            .cloned()
            .unwrap_or_else(|| std::sync::Arc::new(wow_data::EmotesTextStore::from_entries([])));
        let emotes = self
            .emotes_store_for_test_like_cpp()
            .cloned()
            .unwrap_or_else(|| std::sync::Arc::new(wow_data::EmotesStore::from_entries([])));
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        self.handle_text_emote_with_catalogs_like_cpp(
            emotes_text.as_ref(),
            emotes.as_ref(),
            &chat_policy,
            pkt,
        )
        .await;
    }
    pub(super) fn publish_player_emote_state_like_cpp(&mut self, emote_state: u32) {
        if let Some(update) =
            crate::session::hub_mut(self).set_player_emote_state_like_cpp(emote_state)
        {
            self.send_packet(&update);
            crate::session::hub_ref(self)
                .broadcast_to_movement_set_like_cpp(update.to_bytes(), false);
        }
    }
}
