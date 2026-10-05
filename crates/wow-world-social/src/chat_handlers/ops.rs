// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Chat handler packet bodies for [`ChatHandlerCxLikeCpp`](super::ChatHandlerCxLikeCpp).
//!
//! Split out of `chat_handlers.rs` to stay inside the physical file budget; the
//! borrowed context type, the channel/policy helpers, the host bridge and the
//! direct registrar stay in the parent module.

use super::*;

impl<'a> ChatHandlerCxLikeCpp<'a> {
    pub fn new(
        hub: HubMut<'a>,
        social: &'a mut SessionSocialLimits,
        policy: &'a ChatPolicyCatalogsLikeCpp,
    ) -> Self {
        Self {
            hub,
            social,
            policy,
        }
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    pub(super) fn broadcast_chat_packet(&self, pkt: &ChatPkt, range: f32) {
        self.broadcast_raw_packet(pkt.to_bytes(), range);
    }

    pub(super) fn broadcast_group_addon_chat_like_cpp(
        &self,
        msg_type: ChatMsg,
        packet: ChatAddonMessage,
    ) {
        let (sender_guid, sender_name) = player_name_and_guid_like_cpp(&self.hub.shared());
        let Some(group) = self.current_chat_group_like_cpp(sender_guid) else {
            return;
        };

        let subgroup_filter = if msg_type == ChatMsg::Party {
            Some(group.member_group_like_cpp(sender_guid))
        } else {
            None
        };

        let chat = ChatPkt {
            msg_type,
            language: if packet.is_logged {
                LANG_ADDON_LOGGED_LIKE_CPP
            } else {
                LANG_ADDON_LIKE_CPP
            },
            sender_guid,
            sender_name,
            target_guid: ObjectGuid::EMPTY,
            target_name: String::new(),
            prefix: packet.prefix.clone(),
            channel: String::new(),
            text: packet.text,
            virtual_realm: self.hub.shared().core.virtual_realm_address(),
        };
        self.broadcast_group_addon_packet_like_cpp(
            &group,
            subgroup_filter,
            sender_guid,
            packet.prefix,
            chat.to_bytes(),
        );
    }

    pub(super) fn broadcast_group_addon_packet_like_cpp(
        &self,
        group: &GroupInfo,
        subgroup_filter: Option<u8>,
        sender_guid: ObjectGuid,
        prefix: String,
        bytes: Vec<u8>,
    ) {
        let Some(registry) = self.hub.shared().core.player_registry() else {
            return;
        };

        for member_guid in &group.members {
            if *member_guid == sender_guid {
                continue;
            }
            if let Some(subgroup) = subgroup_filter
                && group.member_group_like_cpp(*member_guid) != subgroup
            {
                continue;
            }

            if let Some(member) = registry.group_presence(*member_guid) {
                let command = SessionCommand::SendAddonIfRegisteredLikeCpp(
                    SendAddonIfRegisteredLikeCppCommand {
                        prefix: prefix.clone(),
                        packet_bytes: bytes.clone(),
                    },
                );
                let _ = registry.try_send_current_command(member.registration, command);
            }
        }
    }

    pub(super) fn broadcast_group_chat_like_cpp(
        &self,
        requested_type: ChatMsg,
        language: u32,
        sender_guid: ObjectGuid,
        sender_name: String,
        text: String,
        virtual_realm: u32,
        party_raid_warnings: bool,
    ) {
        let Some(group) = self.current_chat_group_like_cpp(sender_guid) else {
            return;
        };

        let (chat_type, subgroup_filter) = match requested_type {
            ChatMsg::Party => {
                let chat_type = if group.is_leader_like_cpp(sender_guid) {
                    ChatMsg::PartyLeader
                } else {
                    ChatMsg::Party
                };
                (chat_type, Some(group.member_group_like_cpp(sender_guid)))
            }
            ChatMsg::Raid => {
                if !group.is_raid_group() {
                    return;
                }
                let chat_type = if group.is_leader_like_cpp(sender_guid) {
                    ChatMsg::RaidLeader
                } else {
                    ChatMsg::Raid
                };
                (chat_type, None)
            }
            ChatMsg::RaidWarning => {
                if !(group.is_raid_group() || party_raid_warnings)
                    || !(group.is_leader_like_cpp(sender_guid)
                        || group.is_assistant_like_cpp(sender_guid))
                {
                    return;
                }
                (ChatMsg::RaidWarning, None)
            }
            ChatMsg::InstanceChat => {
                let chat_type = if group.is_leader_like_cpp(sender_guid) {
                    ChatMsg::InstanceChatLeader
                } else {
                    ChatMsg::InstanceChat
                };
                (chat_type, None)
            }
            _ => return,
        };

        let chat = ChatPkt {
            msg_type: chat_type,
            language,
            sender_guid,
            sender_name,
            target_guid: ObjectGuid::EMPTY,
            target_name: String::new(),
            prefix: String::new(),
            channel: String::new(),
            text,
            virtual_realm,
        };
        self.broadcast_group_chat_packet_like_cpp(&group, subgroup_filter, chat.to_bytes());
    }

    pub(super) fn broadcast_group_chat_packet_like_cpp(
        &self,
        group: &GroupInfo,
        subgroup_filter: Option<u8>,
        bytes: Vec<u8>,
    ) {
        let Some(registry) = self.hub.shared().core.player_registry() else {
            return;
        };

        for member_guid in &group.members {
            if let Some(subgroup) = subgroup_filter
                && group.member_group_like_cpp(*member_guid) != subgroup
            {
                continue;
            }

            if let Some(member) = registry.group_presence(*member_guid) {
                let _ = registry.send_current_packet(member.registration, bytes.clone());
            }
        }
    }

    pub(crate) fn broadcast_raw_packet(&self, bytes: Vec<u8>, range: f32) {
        let registry = match self.hub.shared().core.player_registry() {
            Some(r) => r,
            None => return,
        };

        let sender_guid = self
            .hub
            .shared()
            .core
            .player_guid()
            .unwrap_or(ObjectGuid::EMPTY);
        let sender_pos = self.hub.shared().player_position_like_cpp();
        let sender_map = self.hub.shared().core.player_map_id_like_cpp();
        let sender_instance = registry
            .runtime_recipient(sender_guid)
            .map(|recipient| recipient.instance_id)
            .or_else(|| {
                self.hub
                    .shared()
                    .core
                    .current_canonical_player_map_key_like_cpp()
                    .map(|key| key.instance_id)
            })
            .unwrap_or(0);
        let range_sq = range * range;

        for recipient in registry.runtime_recipients() {
            // Skip self.
            if recipient.guid == sender_guid {
                continue;
            }

            // Must be an in-world player on the same map instance.
            if !recipient.is_in_world
                || recipient.map_id != sender_map
                || recipient.instance_id != sender_instance
            {
                continue;
            }

            // Distance check.
            if let Some(sp) = sender_pos {
                let dx = sp.x - recipient.position.x;
                let dy = sp.y - recipient.position.y;
                let dz = sp.z - recipient.position.z;
                if dx * dx + dy * dy + dz * dz > range_sq {
                    continue;
                }
            }

            let _ = registry.send_current_packet(recipient.registration, bytes.clone());
        }
    }

    pub(super) fn current_chat_group_like_cpp(&self, sender_guid: ObjectGuid) -> Option<GroupInfo> {
        let registry = self.hub.shared().core.group_registry()?;
        if let Some(group_guid) = {
            let owner = self.hub.shared().core.player_group_owner_access_like_cpp();
            self.social.resolved_group_guid_with_access_like_cpp(
                &owner,
                cfg!(any(test, feature = "test-fixtures")),
            )
        } && let Some(group) = registry.get(&group_guid)
            && group.members.contains(&sender_guid)
        {
            return Some(group.clone());
        }

        registry
            .snapshots()
            .into_iter()
            .find(|group| group.members.contains(&sender_guid))
    }

    pub fn handle_chat_addon_message_params_like_cpp(
        &mut self,
        packet: ChatAddonMessage,
        target: &str,
        _channel_guid: ObjectGuid,
    ) {
        if packet.prefix.is_empty() || packet.prefix.len() > 16 {
            return;
        }

        if !self.policy.addon_channel {
            return;
        }
        if !self.hub.shared().core.can_speak_like_cpp() {
            return;
        }
        self.social.update_speak_time_with_policy_like_cpp(
            &mut self.hub,
            ChatFloodThrottleIndexLikeCpp::Addon,
            self.policy.flood,
        );

        if packet.text.len() > 255 {
            return;
        }

        let Some(msg_type) = chat_msg_from_i32_like_cpp(packet.msg_type) else {
            debug!(
                account = self.hub.shared().core.account_id,
                ty = packet.msg_type,
                "Unknown addon chat message type ignored"
            );
            return;
        };

        match msg_type {
            ChatMsg::Whisper => {
                self.send_addon_whisper_like_cpp(
                    target,
                    packet.prefix,
                    packet.text,
                    packet.is_logged,
                    false,
                );
            }
            ChatMsg::Party | ChatMsg::Raid | ChatMsg::InstanceChat => {
                self.broadcast_group_addon_chat_like_cpp(msg_type, packet);
            }
            ChatMsg::Guild | ChatMsg::Officer | ChatMsg::Channel => {
                debug!(
                    account = self.hub.shared().core.account_id,
                    ty = ?msg_type,
                    prefix = %packet.prefix,
                    logged = packet.is_logged,
                    "Addon chat message ignored until guild/channel/targeted addon routing is ported"
                );
            }
            _ => {
                debug!(
                    account = self.hub.shared().core.account_id,
                    ty = ?msg_type,
                    "Unsupported addon chat message type ignored"
                );
            }
        }
    }

    pub async fn handle_chat_addon_message_targeted_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let packet = match ChatAddonMessageTargeted::read(&mut pkt) {
            Ok(packet) => packet,
            Err(e) => {
                tracing::warn!(
                    account = self.hub.shared().core.account_id,
                    "Bad targeted addon chat packet: {e}"
                );
                return;
            }
        };

        self.handle_chat_addon_message_params_like_cpp(
            packet.params,
            &packet.target,
            packet.channel_guid,
        );
    }

    pub async fn handle_chat_addon_message_whisper_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let packet = match ChatAddonMessageWhisper::read(&mut pkt) {
            Ok(packet) => packet,
            Err(e) => {
                tracing::warn!(
                    account = self.hub.shared().core.account_id,
                    "Bad addon whisper packet: {e}"
                );
                return;
            }
        };

        if self.has_gm_silence_aura_like_cpp() {
            return;
        }

        if self.hub.shared().player_level_like_cpp() < self.policy.level_requirements.whisper {
            return;
        }

        self.send_addon_whisper_like_cpp(
            &packet.target,
            packet.prefix,
            packet.message,
            false,
            true,
        );
    }

    pub async fn handle_chat_addon_message_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let packet = match ChatAddonMessage::read(&mut pkt) {
            Ok(packet) => packet,
            Err(e) => {
                tracing::warn!(
                    account = self.hub.shared().core.account_id,
                    "Bad addon chat packet: {e}"
                );
                return;
            }
        };

        self.handle_chat_addon_message_params_like_cpp(packet, "", ObjectGuid::EMPTY);
    }

    pub async fn handle_chat_afk_with_policy_like_cpp(&mut self, mut pkt: wow_packet::WorldPacket) {
        let mut msg = match ChatMessageAfk::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    account = self.hub.shared().core.account_id,
                    "Bad AFK chat packet: {e}"
                );
                return;
            }
        };

        if send_wait_before_speaking_notification_if_muted_like_cpp(&self.hub.shared()) {
            return;
        }
        self.social.update_speak_time_with_policy_like_cpp(
            &mut self.hub,
            ChatFloodThrottleIndexLikeCpp::Regular,
            self.policy.flood,
        );
        if msg.text.len() > 511 {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, self.policy.fake_message_preventing) {
            tracing::warn!(
                account = self.hub.shared().core.account_id,
                "AFK message rejected: invalid character/control sequence"
            );
            return;
        }
        if !self.validate_hyperlinks_and_maybe_kick_with_policy_like_cpp(&msg.text, "afk") {
            return;
        }
        if self.has_gm_silence_aura_like_cpp() {
            self.send_gm_silence_notification_like_cpp();
            return;
        }
        let _ = self.social.apply_chat_away_mode_like_cpp(
            &mut self.hub,
            PlayerAwayModeLikeCpp::Afk,
            msg.text,
        );
    }

    pub async fn handle_chat_channel_command(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ChannelCommand::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "ChannelCommand parse failed: {error}"
            );
        }

        // Channel lookup and command execution require ChannelMgr and are not represented
        // yet. Missing channel is silent like C++.
    }

    pub async fn handle_chat_channel_message_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let mut msg = match ChatMessageChannel::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    account = self.hub.shared().core.account_id,
                    "Bad channel chat packet: {e}"
                );
                return;
            }
        };

        if msg.language == LANG_UNIVERSAL_LIKE_CPP {
            tracing::warn!(
                account = self.hub.shared().core.account_id,
                "Channel chat rejected: client attempted LANG_UNIVERSAL"
            );
            return;
        }
        if !is_known_language_like_cpp(msg.language) {
            tracing::warn!(
                account = self.hub.shared().core.account_id,
                language = msg.language,
                "Channel chat rejected: unknown language"
            );
            return;
        }
        if send_wait_before_speaking_notification_if_muted_like_cpp(&self.hub.shared()) {
            return;
        }
        self.social.update_speak_time_with_policy_like_cpp(
            &mut self.hub,
            ChatFloodThrottleIndexLikeCpp::Regular,
            self.policy.flood,
        );
        if msg.text.len() > 511 || msg.text.is_empty() {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, self.policy.fake_message_preventing) {
            tracing::warn!(
                account = self.hub.shared().core.account_id,
                "Channel chat rejected: invalid character/control sequence"
            );
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !self.validate_hyperlinks_and_maybe_kick_with_policy_like_cpp(&msg.text, "channel_chat")
        {
            return;
        }
        if self.has_gm_silence_aura_like_cpp() {
            self.send_gm_silence_notification_like_cpp();
            return;
        }
        if !self.meets_chat_level_req_with_policy_like_cpp(ChatMsg::Channel) {
            if let Some(required_level) =
                Self::required_chat_level_with_policy_like_cpp(ChatMsg::Channel, self.policy)
            {
                self.send_chat_say_level_notification_like_cpp(required_level);
            }
            return;
        }

        debug!(
            account = self.hub.shared().core.account_id,
            target = %msg.target,
            channel_guid = ?msg.channel_guid,
            secure = ?msg.is_secure,
            "Channel chat ignored until ChannelMgr::Say is ported"
        );
    }

    pub async fn handle_chat_channel_password(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ChannelPassword::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
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

    pub async fn handle_chat_channel_player_command(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ChannelPlayerCommand::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
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

    pub async fn handle_chat_dnd_with_policy_like_cpp(&mut self, mut pkt: wow_packet::WorldPacket) {
        let mut msg = match ChatMessageDnd::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    account = self.hub.shared().core.account_id,
                    "Bad DND chat packet: {e}"
                );
                return;
            }
        };

        if send_wait_before_speaking_notification_if_muted_like_cpp(&self.hub.shared()) {
            return;
        }
        if msg.text.len() > 511 {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, self.policy.fake_message_preventing) {
            tracing::warn!(
                account = self.hub.shared().core.account_id,
                "DND message rejected: invalid character/control sequence"
            );
            return;
        }
        if !self.validate_hyperlinks_and_maybe_kick_with_policy_like_cpp(&msg.text, "dnd") {
            return;
        }
        if self.has_gm_silence_aura_like_cpp() {
            self.send_gm_silence_notification_like_cpp();
            return;
        }
        let _ = self.social.apply_chat_away_mode_like_cpp(
            &mut self.hub,
            PlayerAwayModeLikeCpp::Dnd,
            msg.text,
        );
    }

    pub async fn handle_chat_emote_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let mut msg = match ChatMessageEmote::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    account = self.hub.shared().core.account_id,
                    "Bad emote packet: {e}"
                );
                return;
            }
        };

        if send_wait_before_speaking_notification_if_muted_like_cpp(&self.hub.shared()) {
            return;
        }
        if msg.text.len() > 511 {
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, self.policy.fake_message_preventing) {
            tracing::warn!(
                account = self.hub.shared().core.account_id,
                "Text emote rejected: invalid character/control sequence"
            );
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !self.validate_hyperlinks_and_maybe_kick_with_policy_like_cpp(&msg.text, "text_emote") {
            return;
        }
        if self.has_gm_silence_aura_like_cpp() {
            self.send_gm_silence_notification_like_cpp();
            return;
        }
        if self.hub.shared().resolved_player_is_alive_like_cpp() != Some(true) {
            return;
        }
        if self.hub.shared().player_level_like_cpp() < self.policy.level_requirements.emote {
            self.send_chat_say_level_notification_like_cpp(self.policy.level_requirements.emote);
            return;
        }

        debug!(
            account = self.hub.shared().core.account_id,
            text = %msg.text,
            "Text emote"
        );

        let (sender_guid, sender_name) = player_name_and_guid_like_cpp(&self.hub.shared());
        let virtual_realm = self.hub.shared().core.virtual_realm_address();

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
        self.publication_like_cpp().send_packet(&chat);
        self.broadcast_chat_packet(&chat, self.policy.listen_ranges.text_emote);
    }

    pub async fn handle_chat_join_channel(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match JoinChannel::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "JoinChannel parse failed: {error}"
                );
                return;
            }
        };

        match join_channel_custom_precheck_like_cpp(&request) {
            JoinChannelPrecheckLikeCpp::Continue => {}
            JoinChannelPrecheckLikeCpp::InvalidName => {
                self.publication_like_cpp()
                    .send_packet(&ChannelNotify::invalid_name(request.channel_name));
                return;
            }
            JoinChannelPrecheckLikeCpp::PasswordTooLong => {
                warn!(
                    account = self.hub.shared().core.account_id,
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

    pub async fn handle_chat_leave_channel(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match LeaveChannel::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
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

    pub async fn handle_chat_message_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        msg_type: ChatMsg,
    ) {
        let mut msg = match ChatMessage::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    account = self.hub.shared().core.account_id,
                    "Bad chat packet: {e}"
                );
                return;
            }
        };

        if msg.language == LANG_UNIVERSAL_LIKE_CPP {
            tracing::warn!(
                account = self.hub.shared().core.account_id,
                ty = ?msg_type,
                "Chat message rejected: client attempted LANG_UNIVERSAL"
            );
            return;
        }
        if !is_known_language_like_cpp(msg.language) {
            tracing::warn!(
                account = self.hub.shared().core.account_id,
                ty = ?msg_type,
                language = msg.language,
                "Chat message rejected: unknown language"
            );
            return;
        }
        if send_wait_before_speaking_notification_if_muted_like_cpp(&self.hub.shared()) {
            return;
        }
        if !matches!(msg_type, ChatMsg::Afk | ChatMsg::Dnd) {
            self.social.update_speak_time_with_policy_like_cpp(
                &mut self.hub,
                ChatFloodThrottleIndexLikeCpp::Regular,
                self.policy.flood,
            );
        }
        if msg.text.len() > 511 {
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, self.policy.fake_message_preventing) {
            tracing::warn!(
                account = self.hub.shared().core.account_id,
                ty = ?msg_type,
                "Chat message rejected: invalid character/control sequence"
            );
            return;
        }
        if msg.text.is_empty() {
            return;
        }

        debug!(
            account = self.hub.shared().core.account_id,
            ty = ?msg_type,
            text = %msg.text,
            "Chat message"
        );

        if !self.validate_hyperlinks_and_maybe_kick_with_policy_like_cpp(&msg.text, "chat") {
            return;
        }
        if self.has_gm_silence_aura_like_cpp() {
            self.send_gm_silence_notification_like_cpp();
            return;
        }
        if matches!(msg_type, ChatMsg::Say | ChatMsg::Yell)
            && self.hub.shared().resolved_player_is_alive_like_cpp() != Some(true)
        {
            return;
        }
        if !self.meets_chat_level_req_with_policy_like_cpp(msg_type) {
            if let Some(required_level) =
                Self::required_chat_level_with_policy_like_cpp(msg_type, self.policy)
            {
                self.send_chat_say_level_notification_like_cpp(required_level);
            }
            return;
        }

        let (sender_guid, sender_name) = player_name_and_guid_like_cpp(&self.hub.shared());
        let virtual_realm = self.hub.shared().core.virtual_realm_address();

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
                self.policy.party_raid_warnings,
            );
            return;
        }

        if matches!(msg_type, ChatMsg::Guild | ChatMsg::Officer) {
            debug!(
                account = self.hub.shared().core.account_id,
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
        self.publication_like_cpp().send_packet(&chat);

        // Broadcast to nearby players on the same map.
        let listen_ranges = self.policy.listen_ranges;
        let range = if msg_type == ChatMsg::Yell {
            listen_ranges.yell
        } else {
            listen_ranges.say
        };
        self.broadcast_chat_packet(&chat, range);
    }

    pub async fn handle_chat_register_addon_prefixes(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match ChatRegisterAddonPrefixes::read(&mut pkt) {
            Ok(packet) => packet,
            Err(e) => {
                tracing::warn!(
                    account = self.hub.shared().core.account_id,
                    "Bad addon prefix packet: {e}"
                );
                return;
            }
        };

        let (prefixes, filter) = self.social.register_addon_prefixes_like_cpp(
            packet.prefixes,
            ChatRegisterAddonPrefixes::MAX_PREFIXES,
        );
        debug!(
            account = self.hub.shared().core.account_id,
            prefixes, filter, "Registered addon prefixes"
        );
    }

    pub async fn handle_chat_report_filtered(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(e) = ChatReportFiltered::read(&mut pkt) {
            tracing::warn!(
                account = self.hub.shared().core.account_id,
                "Bad chat report filtered packet: {e}"
            );
            return;
        }

        debug!(
            account = self.hub.shared().core.account_id,
            "ChatReportFiltered received; spam reporting is not represented yet"
        );
    }

    pub async fn handle_chat_report_ignored(&mut self, mut pkt: wow_packet::WorldPacket) {
        let report = match ChatReportIgnored::read(&mut pkt) {
            Ok(report) => report,
            Err(e) => {
                tracing::warn!(
                    account = self.hub.shared().core.account_id,
                    "Bad chat report ignored packet: {e}"
                );
                return;
            }
        };

        let (reporter_guid, reporter_name) = player_name_and_guid_like_cpp(&self.hub.shared());
        let virtual_realm = self.hub.shared().core.virtual_realm_address();

        let player_registry = self.hub.shared().core.player_registry().cloned();
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

    pub async fn handle_chat_unregister_all_addon_prefixes(
        &mut self,
        _pkt: wow_packet::WorldPacket,
    ) {
        self.social.clear_registered_addon_prefixes_like_cpp();
    }

    pub async fn handle_chat_whisper_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let mut msg = match ChatMessageWhisper::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    account = self.hub.shared().core.account_id,
                    "Bad whisper packet: {e}"
                );
                return;
            }
        };

        if msg.language == LANG_UNIVERSAL_LIKE_CPP {
            tracing::warn!(
                account = self.hub.shared().core.account_id,
                "Whisper rejected: client attempted LANG_UNIVERSAL"
            );
            return;
        }
        if !is_known_language_like_cpp(msg.language) {
            tracing::warn!(
                account = self.hub.shared().core.account_id,
                language = msg.language,
                "Whisper rejected: unknown language"
            );
            return;
        }
        if send_wait_before_speaking_notification_if_muted_like_cpp(&self.hub.shared()) {
            return;
        }
        self.social.update_speak_time_with_policy_like_cpp(
            &mut self.hub,
            ChatFloodThrottleIndexLikeCpp::Regular,
            self.policy.flood,
        );
        if msg.text.len() > 511 {
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !validate_message_like_cpp(&mut msg.text, self.policy.fake_message_preventing) {
            tracing::warn!(
                account = self.hub.shared().core.account_id,
                "Whisper rejected: invalid character/control sequence"
            );
            return;
        }
        if msg.text.is_empty() {
            return;
        }
        if !self.validate_hyperlinks_and_maybe_kick_with_policy_like_cpp(&msg.text, "whisper") {
            return;
        }

        debug!(
            account = self.hub.shared().core.account_id,
            target = %msg.target,
            text = %msg.text,
            "Whisper"
        );

        if !self.meets_whisper_level_req_with_policy_like_cpp() {
            self.send_chat_whisper_level_notification_like_cpp(
                self.policy.level_requirements.whisper,
            );
            return;
        }

        let (sender_guid, sender_name) = player_name_and_guid_like_cpp(&self.hub.shared());
        let virtual_realm = self.hub.shared().core.virtual_realm_address();
        let target_name = msg.target.clone();

        // Try to deliver to the target player via the registry.
        let player_registry = self.hub.shared().core.player_registry().cloned();
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
            self.publication_like_cpp().send_packet(&inform);

            if let Some(auto_reply) = registry.social_auto_reply(target.guid) {
                if target.is_afk {
                    self.send_whisper_away_reply_like_cpp(&target_name, &auto_reply, true);
                } else if target.is_dnd {
                    self.send_whisper_away_reply_like_cpp(&target_name, &auto_reply, false);
                }
            }
        } else {
            self.publication_like_cpp()
                .send_packet(&ChatPlayerNotfound { name: target_name });
        }
    }

    pub async fn handle_update_aadc_status(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(e) = UpdateAadcStatus::read(&mut pkt) {
            tracing::warn!(
                account = self.hub.shared().core.account_id,
                "Bad update AADC status packet: {e}"
            );
            return;
        }

        self.publication_like_cpp()
            .send_packet(&UpdateAadcStatusResponse {
                success: true,
                chat_disabled: false,
            });
    }

    pub(super) fn has_gm_silence_aura_like_cpp(&self) -> bool {
        self.hub
            .shared()
            .resolved_player_visible_auras_like_cpp()
            .is_some_and(|auras| {
                auras
                    .values()
                    .any(|aura| aura.spell_id == GM_SILENCE_AURA_LIKE_CPP)
            })
    }

    pub(super) fn meets_chat_level_req_with_policy_like_cpp(&self, msg_type: ChatMsg) -> bool {
        Self::required_chat_level_with_policy_like_cpp(msg_type, self.policy)
            .is_none_or(|required| self.hub.shared().player_level_like_cpp() >= required)
    }

    pub(super) fn meets_whisper_level_req_with_policy_like_cpp(&self) -> bool {
        self.hub.shared().player_is_game_master_like_cpp() == Some(true)
            || self.hub.shared().player_level_like_cpp() >= self.policy.level_requirements.whisper
    }

    pub(super) fn required_chat_level_with_policy_like_cpp(
        msg_type: ChatMsg,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) -> Option<u8> {
        let requirements = chat_policy.level_requirements;
        match msg_type {
            ChatMsg::Say => Some(requirements.say),
            ChatMsg::Yell => Some(requirements.yell),
            _ => None,
        }
    }

    pub(super) fn send_addon_whisper_like_cpp(
        &mut self,
        target_name: &str,
        prefix: String,
        message: String,
        is_logged: bool,
        notify_missing: bool,
    ) {
        let player_registry = self.hub.shared().core.player_registry().cloned();
        let target_info = player_registry
            .as_ref()
            .and_then(|registry| registry.social_recipient_by_name(target_name));

        let (Some(registry), Some(target)) = (player_registry, target_info) else {
            if notify_missing {
                self.publication_like_cpp()
                    .send_packet(&ChatPlayerNotfound {
                        name: target_name.to_string(),
                    });
            }
            return;
        };

        if player_team_for_race_cpp(self.hub.shared().player_race_like_cpp())
            != player_team_for_race_cpp(target.race)
        {
            if notify_missing {
                self.publication_like_cpp()
                    .send_packet(&ChatPlayerNotfound {
                        name: target.player_name,
                    });
            }
            return;
        }

        let (sender_guid, sender_name) = player_name_and_guid_like_cpp(&self.hub.shared());
        let chat = ChatPkt {
            msg_type: ChatMsg::Whisper,
            language: if is_logged {
                LANG_ADDON_LOGGED_LIKE_CPP
            } else {
                LANG_ADDON_LIKE_CPP
            },
            sender_guid,
            sender_name,
            target_guid: target.guid,
            target_name: target.player_name,
            prefix: prefix.clone(),
            channel: String::new(),
            text: message,
            virtual_realm: self.hub.shared().core.virtual_realm_address(),
        };

        let command =
            SessionCommand::SendAddonIfRegisteredLikeCpp(SendAddonIfRegisteredLikeCppCommand {
                prefix,
                packet_bytes: chat.to_bytes(),
            });
        let _ = registry.try_send_current_command(target.registration, command);
    }

    pub(super) fn send_chat_say_level_notification_like_cpp(&self, required_level: u8) {
        self.publication_like_cpp().send_packet(&PrintNotification {
            notify_text: format!(
                "You cannot say, yell or emote until you become level {required_level}."
            ),
        });
    }

    pub(super) fn send_chat_whisper_level_notification_like_cpp(&self, required_level: u8) {
        self.publication_like_cpp().send_packet(&PrintNotification {
            notify_text: format!("You cannot whisper until you become level {required_level}."),
        });
    }

    pub(super) fn send_gm_silence_notification_like_cpp(&self) {
        let (_, sender_name) = player_name_and_guid_like_cpp(&self.hub.shared());
        self.publication_like_cpp().send_packet(&PrintNotification {
            notify_text: format!("Silence is ON for {sender_name}"),
        });
    }

    pub(super) fn send_whisper_away_reply_like_cpp(
        &mut self,
        target_name: &str,
        auto_reply: &str,
        afk: bool,
    ) {
        let text = if afk {
            // C++ `LANG_PLAYER_AFK`: "%s is Away from Keyboard: %s".
            format!("{target_name} is Away from Keyboard: {auto_reply}")
        } else {
            // C++ `LANG_PLAYER_DND`: "%s wishes to not be disturbed and cannot receive whisper messages: %s".
            format!(
                "{target_name} wishes to not be disturbed and cannot receive whisper messages: {auto_reply}"
            )
        };
        let packet = ChatPkt {
            msg_type: ChatMsg::System,
            language: LANG_UNIVERSAL_LIKE_CPP as u32,
            sender_guid: ObjectGuid::EMPTY,
            sender_name: String::new(),
            target_guid: ObjectGuid::EMPTY,
            target_name: String::new(),
            prefix: String::new(),
            channel: String::new(),
            text,
            virtual_realm: self.hub.shared().core.virtual_realm_address(),
        };
        self.publication_like_cpp().send_packet(&packet);
    }

    pub(super) fn validate_hyperlinks_and_maybe_kick_with_policy_like_cpp(
        &mut self,
        text: &str,
        context: &str,
    ) -> bool {
        if check_all_links_shape_like_cpp(text) {
            return true;
        }

        tracing::warn!(
            account = self.hub.shared().core.account_id,
            context,
            "Chat message rejected: invalid hyperlink/control sequence"
        );

        if self.policy.strict_link_checking_kick {
            self.hub
                .core
                .kick("WorldSession::ValidateHyperlinksAndMaybeKick Invalid chat link");
        }

        false
    }
}
