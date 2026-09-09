//! Chat handlers operations, part 1 of 2.
//!
//! The inherent `WorldSession` impl is divided by responsibility under
//! #654; every method keeps its original body.

use super::*;

impl WorldSession {
    /// C++ ref: `WorldSession::ValidateHyperlinksAndMaybeKick`.
    pub(super) fn validate_hyperlinks_and_maybe_kick_with_policy_like_cpp(
        &mut self,
        text: &str,
        context: &str,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) -> bool {
        if check_all_links_shape_like_cpp(text) {
            return true;
        }

        tracing::warn!(
            account = self.account_id,
            context,
            "Chat message rejected: invalid hyperlink/control sequence"
        );

        if chat_policy.strict_link_checking_kick {
            self.kick("WorldSession::ValidateHyperlinksAndMaybeKick Invalid chat link");
        }

        false
    }
    /// Handle say/yell/party/guild/raid/instance chat messages.
    pub(crate) async fn handle_chat_message_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        msg_type: ChatMsg,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) {
        let mut msg = match ChatMessage::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad chat packet: {e}");
                return;
            }
        };

        if msg.language == LANG_UNIVERSAL_LIKE_CPP {
            tracing::warn!(
                account = self.account_id,
                ty = ?msg_type,
                "Chat message rejected: client attempted LANG_UNIVERSAL"
            );
            return;
        }
        if !is_known_language_like_cpp(msg.language) {
            tracing::warn!(
                account = self.account_id,
                ty = ?msg_type,
                language = msg.language,
                "Chat message rejected: unknown language"
            );
            return;
        }
        if self.send_wait_before_speaking_notification_if_muted_like_cpp() {
            return;
        }
        if !matches!(msg_type, ChatMsg::Afk | ChatMsg::Dnd) {
            self.update_speak_time_with_policy_like_cpp(
                ChatFloodThrottleIndexLikeCpp::Regular,
                chat_policy.flood,
            );
        }
        if msg.text.len() > 511 {
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, chat_policy.fake_message_preventing) {
            tracing::warn!(
                account = self.account_id,
                ty = ?msg_type,
                "Chat message rejected: invalid character/control sequence"
            );
            return;
        }
        if msg.text.is_empty() {
            return;
        }

        debug!(
            account = self.account_id,
            ty = ?msg_type,
            text = %msg.text,
            "Chat message"
        );

        if !self.validate_hyperlinks_and_maybe_kick_with_policy_like_cpp(
            &msg.text,
            "chat",
            chat_policy,
        ) {
            return;
        }
        if self.has_gm_silence_aura_like_cpp() {
            self.send_gm_silence_notification_like_cpp();
            return;
        }
        if matches!(msg_type, ChatMsg::Say | ChatMsg::Yell)
            && self.resolved_player_is_alive_like_cpp() != Some(true)
        {
            return;
        }
        if !self.meets_chat_level_req_with_policy_like_cpp(msg_type, chat_policy) {
            if let Some(required_level) =
                Self::required_chat_level_with_policy_like_cpp(msg_type, chat_policy)
            {
                self.send_chat_say_level_notification_like_cpp(required_level);
            }
            return;
        }

        let (sender_guid, sender_name) = self.player_name_and_guid();
        let virtual_realm = self.virtual_realm_address();

        if matches!(
            msg_type,
            ChatMsg::Party | ChatMsg::Raid | ChatMsg::RaidWarning | ChatMsg::InstanceChat
        ) {
            self.broadcast_group_chat_like_cpp(
                msg_type,
                msg.language as u32,
                sender_guid,
                sender_name,
                msg.text,
                virtual_realm,
                chat_policy.party_raid_warnings,
            );
            return;
        }

        if matches!(msg_type, ChatMsg::Guild | ChatMsg::Officer) {
            debug!(
                account = self.account_id,
                ty = ?msg_type,
                "Guild chat ignored until GuildRegistry/BroadcastToGuild is ported"
            );
            return;
        }

        let chat = ChatPkt {
            msg_type,
            language: msg.language as u32,
            sender_guid,
            sender_name,
            target_guid: wow_core::ObjectGuid::EMPTY,
            target_name: String::new(),
            prefix: String::new(),
            channel: String::new(),
            text: msg.text,
            virtual_realm,
        };

        // Echo back to the sender (they need to see their own message).
        self.send_packet(&chat);

        // Broadcast to nearby players on the same map.
        let listen_ranges = chat_policy.listen_ranges;
        let range = if msg_type == ChatMsg::Yell {
            listen_ranges.yell
        } else {
            listen_ranges.say
        };
        self.broadcast_chat_packet(&chat, range);
    }
    /// Handle whisper messages.
    pub(crate) async fn handle_chat_whisper_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) {
        let mut msg = match ChatMessageWhisper::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad whisper packet: {e}");
                return;
            }
        };

        if msg.language == LANG_UNIVERSAL_LIKE_CPP {
            tracing::warn!(
                account = self.account_id,
                "Whisper rejected: client attempted LANG_UNIVERSAL"
            );
            return;
        }
        if !is_known_language_like_cpp(msg.language) {
            tracing::warn!(
                account = self.account_id,
                language = msg.language,
                "Whisper rejected: unknown language"
            );
            return;
        }
        if self.send_wait_before_speaking_notification_if_muted_like_cpp() {
            return;
        }
        self.update_speak_time_with_policy_like_cpp(
            ChatFloodThrottleIndexLikeCpp::Regular,
            chat_policy.flood,
        );
        if msg.text.len() > 511 {
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, chat_policy.fake_message_preventing) {
            tracing::warn!(
                account = self.account_id,
                "Whisper rejected: invalid character/control sequence"
            );
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !self.validate_hyperlinks_and_maybe_kick_with_policy_like_cpp(
            &msg.text,
            "whisper",
            chat_policy,
        ) {
            return;
        }

        debug!(
            account = self.account_id,
            target = %msg.target,
            text = %msg.text,
            "Whisper"
        );

        if !self.meets_whisper_level_req_with_policy_like_cpp(chat_policy) {
            self.send_chat_whisper_level_notification_like_cpp(
                chat_policy.level_requirements.whisper,
            );
            return;
        }

        let (sender_guid, sender_name) = self.player_name_and_guid();
        let virtual_realm = self.virtual_realm_address();
        let target_name = msg.target.clone();

        // Try to deliver to the target player via the registry.
        let player_registry = self.player_registry().cloned();
        let target_info = player_registry
            .as_ref()
            .and_then(|registry| registry.social_recipient_by_name(&target_name));

        if let (Some(registry), Some(target)) = (player_registry, target_info) {
            if self.has_gm_silence_aura_like_cpp() && !target.is_game_master {
                self.send_gm_silence_notification_like_cpp();
                return;
            }

            // Forward the whisper to the target as a normal Say-whisper.
            let to_target = ChatPkt {
                msg_type: ChatMsg::Whisper,
                language: msg.language as u32,
                sender_guid,
                sender_name: sender_name.clone(),
                target_guid: ObjectGuid::EMPTY,
                target_name: target_name.clone(),
                prefix: String::new(),
                channel: String::new(),
                text: msg.text.clone(),
                virtual_realm,
            };
            let _ = registry.send_current_packet(target.registration, to_target.to_bytes());

            // Inform sender their whisper was delivered.
            let inform = ChatPkt {
                msg_type: ChatMsg::WhisperInform,
                language: msg.language as u32,
                sender_guid,
                sender_name: sender_name.clone(),
                target_guid: ObjectGuid::EMPTY,
                target_name: target_name.clone(),
                prefix: String::new(),
                channel: String::new(),
                text: msg.text,
                virtual_realm,
            };
            self.send_packet(&inform);

            if let Some(auto_reply) = registry.social_auto_reply(target.guid) {
                if target.is_afk {
                    self.send_whisper_away_reply_like_cpp(&target_name, &auto_reply, true);
                } else if target.is_dnd {
                    self.send_whisper_away_reply_like_cpp(&target_name, &auto_reply, false);
                }
            }
        } else {
            self.send_packet(&ChatPlayerNotfound { name: target_name });
        }
    }
    /// Handle CMSG_CHAT_MESSAGE_CHANNEL.
    ///
    /// C++ routes this through `HandleChatMessage(CHAT_MSG_CHANNEL, ...)`.
    /// The Rust parser and validation are represented, but the final
    /// ChannelMgr lookup/fanout is intentionally parked until live channels
    /// exist.
    pub(crate) async fn handle_chat_channel_message_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) {
        let mut msg = match ChatMessageChannel::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad channel chat packet: {e}");
                return;
            }
        };

        if msg.language == LANG_UNIVERSAL_LIKE_CPP {
            tracing::warn!(
                account = self.account_id,
                "Channel chat rejected: client attempted LANG_UNIVERSAL"
            );
            return;
        }
        if !is_known_language_like_cpp(msg.language) {
            tracing::warn!(
                account = self.account_id,
                language = msg.language,
                "Channel chat rejected: unknown language"
            );
            return;
        }
        if self.send_wait_before_speaking_notification_if_muted_like_cpp() {
            return;
        }
        self.update_speak_time_with_policy_like_cpp(
            ChatFloodThrottleIndexLikeCpp::Regular,
            chat_policy.flood,
        );
        if msg.text.len() > 511 || msg.text.is_empty() {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, chat_policy.fake_message_preventing) {
            tracing::warn!(
                account = self.account_id,
                "Channel chat rejected: invalid character/control sequence"
            );
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !self.validate_hyperlinks_and_maybe_kick_with_policy_like_cpp(
            &msg.text,
            "channel_chat",
            chat_policy,
        ) {
            return;
        }
        if self.has_gm_silence_aura_like_cpp() {
            self.send_gm_silence_notification_like_cpp();
            return;
        }
        if !self.meets_chat_level_req_with_policy_like_cpp(ChatMsg::Channel, chat_policy) {
            if let Some(required_level) =
                Self::required_chat_level_with_policy_like_cpp(ChatMsg::Channel, chat_policy)
            {
                self.send_chat_say_level_notification_like_cpp(required_level);
            }
            return;
        }

        debug!(
            account = self.account_id,
            target = %msg.target,
            channel_guid = ?msg.channel_guid,
            secure = ?msg.is_secure,
            "Channel chat ignored until ChannelMgr::Say is ported"
        );
    }
    /// Handle CMSG_UPDATE_AADC_STATUS.
    ///
    /// C++ ignores the requested state because disabling chat is unsupported,
    /// then sends success with ChatDisabled=false so the client restores its cvar.
    pub async fn handle_update_aadc_status(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(e) = UpdateAadcStatus::read(&mut pkt) {
            tracing::warn!(
                account = self.account_id,
                "Bad update AADC status packet: {e}"
            );
            return;
        }

        self.send_packet(&UpdateAadcStatusResponse {
            success: true,
            chat_disabled: false,
        });
    }
    /// Handle CMSG_CHAT_MESSAGE_AFK.
    pub(crate) async fn handle_chat_afk_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) {
        let mut msg = match ChatMessageAfk::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad AFK chat packet: {e}");
                return;
            }
        };

        if self.send_wait_before_speaking_notification_if_muted_like_cpp() {
            return;
        }
        self.update_speak_time_with_policy_like_cpp(
            ChatFloodThrottleIndexLikeCpp::Regular,
            chat_policy.flood,
        );
        if msg.text.len() > 511 {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, chat_policy.fake_message_preventing) {
            tracing::warn!(
                account = self.account_id,
                "AFK message rejected: invalid character/control sequence"
            );
            return;
        }
        if !self.validate_hyperlinks_and_maybe_kick_with_policy_like_cpp(
            &msg.text,
            "afk",
            chat_policy,
        ) {
            return;
        }
        if self.has_gm_silence_aura_like_cpp() {
            self.send_gm_silence_notification_like_cpp();
            return;
        }
        let _ = self.apply_chat_away_mode_like_cpp(PlayerAwayModeLikeCpp::Afk, msg.text);
    }
    /// Handle CMSG_CHAT_MESSAGE_DND.
    pub(crate) async fn handle_chat_dnd_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) {
        let mut msg = match ChatMessageDnd::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad DND chat packet: {e}");
                return;
            }
        };

        if self.send_wait_before_speaking_notification_if_muted_like_cpp() {
            return;
        }
        if msg.text.len() > 511 {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, chat_policy.fake_message_preventing) {
            tracing::warn!(
                account = self.account_id,
                "DND message rejected: invalid character/control sequence"
            );
            return;
        }
        if !self.validate_hyperlinks_and_maybe_kick_with_policy_like_cpp(
            &msg.text,
            "dnd",
            chat_policy,
        ) {
            return;
        }
        if self.has_gm_silence_aura_like_cpp() {
            self.send_gm_silence_notification_like_cpp();
            return;
        }
        let _ = self.apply_chat_away_mode_like_cpp(PlayerAwayModeLikeCpp::Dnd, msg.text);
    }
    /// Handle CMSG_CHAT_REPORT_IGNORED.
    ///
    /// C++ ref: `WorldSession::HandleChatIgnoredOpcode`.
    /// The receiver's client sends this after locally ignoring a chat message;
    /// the server notifies the ignored player with `CHAT_MSG_IGNORED`.
    pub async fn handle_chat_report_ignored(&mut self, mut pkt: wow_packet::WorldPacket) {
        let report = match ChatReportIgnored::read(&mut pkt) {
            Ok(report) => report,
            Err(e) => {
                tracing::warn!(
                    account = self.account_id,
                    "Bad chat report ignored packet: {e}"
                );
                return;
            }
        };

        let (reporter_guid, reporter_name) = self.player_name_and_guid();
        let virtual_realm = self.virtual_realm_address();

        let player_registry = self.player_registry().cloned();
        let ignored = player_registry
            .as_ref()
            .and_then(|registry| registry.social_recipient(report.ignored_guid));

        if let (Some(registry), Some(ignored_recipient)) = (player_registry, ignored) {
            let ignored = ChatPkt {
                msg_type: ChatMsg::Ignored,
                language: 0,
                sender_guid: reporter_guid,
                sender_name: reporter_name.clone(),
                target_guid: reporter_guid,
                target_name: reporter_name.clone(),
                prefix: String::new(),
                channel: String::new(),
                text: reporter_name,
                virtual_realm,
            };
            let _ =
                registry.send_current_packet(ignored_recipient.registration, ignored.to_bytes());
        }
    }
    /// Handle CMSG_CHAT_REPORT_FILTERED.
    ///
    /// C++ ref: `WorldSession::HandleChatReportFiltered`.
    /// TrinityCore currently reads an empty packet and only logs a TODO for the
    /// unimplemented spam reporting system.
    pub async fn handle_chat_report_filtered(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(e) = ChatReportFiltered::read(&mut pkt) {
            tracing::warn!(
                account = self.account_id,
                "Bad chat report filtered packet: {e}"
            );
            return;
        }

        debug!(
            account = self.account_id,
            "ChatReportFiltered received; spam reporting is not represented yet"
        );
    }
    /// Handle emote text (/e).
    pub(crate) async fn handle_chat_emote_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) {
        let mut msg = match ChatMessageEmote::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad emote packet: {e}");
                return;
            }
        };

        if self.send_wait_before_speaking_notification_if_muted_like_cpp() {
            return;
        }
        if msg.text.len() > 511 {
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, chat_policy.fake_message_preventing) {
            tracing::warn!(
                account = self.account_id,
                "Text emote rejected: invalid character/control sequence"
            );
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !self.validate_hyperlinks_and_maybe_kick_with_policy_like_cpp(
            &msg.text,
            "text_emote",
            chat_policy,
        ) {
            return;
        }
        if self.has_gm_silence_aura_like_cpp() {
            self.send_gm_silence_notification_like_cpp();
            return;
        }
        if self.resolved_player_is_alive_like_cpp() != Some(true) {
            return;
        }
        if self.player_level_like_cpp() < chat_policy.level_requirements.emote {
            self.send_chat_say_level_notification_like_cpp(chat_policy.level_requirements.emote);
            return;
        }

        debug!(
            account = self.account_id,
            text = %msg.text,
            "Text emote"
        );

        let (sender_guid, sender_name) = self.player_name_and_guid();
        let virtual_realm = self.virtual_realm_address();

        let chat = ChatPkt {
            msg_type: ChatMsg::Emote,
            language: 0,
            sender_guid,
            sender_name,
            target_guid: wow_core::ObjectGuid::EMPTY,
            target_name: String::new(),
            prefix: String::new(),
            channel: String::new(),
            text: msg.text,
            virtual_realm,
        };
        self.send_packet(&chat);
        self.broadcast_chat_packet(&chat, chat_policy.listen_ranges.text_emote);
    }
    /// Handle CMSG_EMOTE — client notifies us it cleared its emote state.
    ///
    /// C++ ref: `WorldSession::HandleEmoteOpcode`.
    pub async fn handle_emote(&mut self, mut pkt: wow_packet::WorldPacket) {
        // EmoteClient has no body — read returns Ok(()) immediately.
        let _ = EmoteClient::read(&mut pkt);
        if self.resolved_player_is_alive_like_cpp() != Some(true)
            || self.player_has_unit_state_like_cpp(UnitState::DIED)
        {
            return;
        }

        self.publish_player_emote_state_like_cpp(EMOTE_ONESHOT_NONE_LIKE_CPP as u32);
        debug!(account = self.account_id, "CMSG_EMOTE: clear emote state");
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
                tracing::warn!(account = self.account_id, "Bad CMSG_SEND_TEXT_EMOTE: {e}");
                return;
            }
        };

        if self.resolved_player_is_alive_like_cpp() != Some(true) {
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
            account = self.account_id,
            emote_id = msg.emote_id,
            sound_index = msg.sound_index,
            "CMSG_SEND_TEXT_EMOTE"
        );

        let (player_guid, _name) = self.player_name_and_guid();
        let account_guid =
            ObjectGuid::create_global(HighGuid::WowAccount, 0, self.account_id as i64);

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
            _ if self.player_has_unit_state_like_cpp(UnitState::DIED) => None,
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
            self.broadcast_to_movement_set_like_cpp(anim_emote.to_bytes(), false);
        }
        self.send_packet(&text_emote);
        self.broadcast_to_movement_set_in_range_like_cpp(text_emote.to_bytes(), text_emote_range);
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
        if let Some(update) = self.set_player_emote_state_like_cpp(emote_state) {
            self.send_packet(&update);
            self.broadcast_to_movement_set_like_cpp(update.to_bytes(), false);
        }
    }
    /// CMSG_CHAT_REGISTER_ADDON_PREFIXES.
    ///
    /// C++ ref: `WorldSession::HandleAddonRegisteredPrefixesOpcode`.
    pub async fn handle_chat_register_addon_prefixes(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match ChatRegisterAddonPrefixes::read(&mut pkt) {
            Ok(packet) => packet,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad addon prefix packet: {e}");
                return;
            }
        };

        self.registered_addon_prefixes.extend(packet.prefixes);
        self.filter_addon_messages =
            self.registered_addon_prefixes.len() <= ChatRegisterAddonPrefixes::MAX_PREFIXES;
        debug!(
            account = self.account_id,
            prefixes = self.registered_addon_prefixes.len(),
            filter = self.filter_addon_messages,
            "Registered addon prefixes"
        );
    }
    /// CMSG_CHAT_ADDON_MESSAGE.
    ///
    /// C++ ref: `WorldSession::HandleChatAddonMessageOpcode`.
    /// Until guild/channel addon routing is ported, parse and validate the C++
    /// packet shape then drop unsupported traffic. This matches disabled addon
    /// channel behavior and prevents unknown-opcode noise during login.
    pub(crate) async fn handle_chat_addon_message_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) {
        let packet = match ChatAddonMessage::read(&mut pkt) {
            Ok(packet) => packet,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad addon chat packet: {e}");
                return;
            }
        };

        self.handle_chat_addon_message_params_like_cpp(packet, "", ObjectGuid::EMPTY, chat_policy);
    }
    /// CMSG_CHAT_ADDON_MESSAGE_TARGETED.
    ///
    /// C++ ref: `WorldSession::HandleChatAddonMessageTargetedOpcode`.
    ///
    /// Not registered in the runtime opcode table yet: the inspected C++ 3.4.3
    /// source marks this packet as `0xBADD`, which is a duplicated unresolved
    /// placeholder in the Rust opcode enum. Keep the parser and handler covered
    /// by direct tests until the real opcode mapping is known.
    pub(crate) async fn handle_chat_addon_message_targeted_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) {
        let packet = match ChatAddonMessageTargeted::read(&mut pkt) {
            Ok(packet) => packet,
            Err(e) => {
                tracing::warn!(
                    account = self.account_id,
                    "Bad targeted addon chat packet: {e}"
                );
                return;
            }
        };

        self.handle_chat_addon_message_params_like_cpp(
            packet.params,
            &packet.target,
            packet.channel_guid,
            chat_policy,
        );
    }
}
