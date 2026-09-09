//! Chat handlers operations, part 2 of 2.
//!
//! The inherent `WorldSession` impl is divided by responsibility under
//! #654; every method keeps its original body.

use super::*;

impl WorldSession {
    pub(super) fn handle_chat_addon_message_params_like_cpp(
        &mut self,
        packet: ChatAddonMessage,
        target: &str,
        _channel_guid: ObjectGuid,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) {
        if packet.prefix.is_empty() || packet.prefix.len() > 16 {
            return;
        }

        if !chat_policy.addon_channel {
            return;
        }
        if !self.can_speak_like_cpp() {
            return;
        }
        self.update_speak_time_with_policy_like_cpp(
            ChatFloodThrottleIndexLikeCpp::Addon,
            chat_policy.flood,
        );

        if packet.text.len() > 255 {
            return;
        }

        let Some(msg_type) = chat_msg_from_i32_like_cpp(packet.msg_type) else {
            debug!(
                account = self.account_id,
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
                    account = self.account_id,
                    ty = ?msg_type,
                    prefix = %packet.prefix,
                    logged = packet.is_logged,
                    "Addon chat message ignored until guild/channel/targeted addon routing is ported"
                );
            }
            _ => {
                debug!(
                    account = self.account_id,
                    ty = ?msg_type,
                    "Unsupported addon chat message type ignored"
                );
            }
        }
    }
    /// CMSG_CHAT_ADDON_MESSAGE_WHISPER.
    ///
    /// C++ ref: `WorldSession::HandleChatAddonMessageWhisper`.
    pub(crate) async fn handle_chat_addon_message_whisper_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) {
        let packet = match ChatAddonMessageWhisper::read(&mut pkt) {
            Ok(packet) => packet,
            Err(e) => {
                tracing::warn!(account = self.account_id, "Bad addon whisper packet: {e}");
                return;
            }
        };

        if self.has_gm_silence_aura_like_cpp() {
            return;
        }

        if self.player_level_like_cpp() < chat_policy.level_requirements.whisper {
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
    pub(super) fn send_addon_whisper_like_cpp(
        &mut self,
        target_name: &str,
        prefix: String,
        message: String,
        is_logged: bool,
        notify_missing: bool,
    ) {
        let player_registry = self.player_registry().cloned();
        let target_info = player_registry
            .as_ref()
            .and_then(|registry| registry.social_recipient_by_name(target_name));

        let (Some(registry), Some(target)) = (player_registry, target_info) else {
            if notify_missing {
                self.send_packet(&ChatPlayerNotfound {
                    name: target_name.to_string(),
                });
            }
            return;
        };

        if player_team_for_race_cpp(self.player_race_like_cpp())
            != player_team_for_race_cpp(target.race)
        {
            if notify_missing {
                self.send_packet(&ChatPlayerNotfound {
                    name: target.player_name,
                });
            }
            return;
        }

        let (sender_guid, sender_name) = self.player_name_and_guid();
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
            virtual_realm: self.virtual_realm_address(),
        };

        let command =
            SessionCommand::SendAddonIfRegisteredLikeCpp(SendAddonIfRegisteredLikeCppCommand {
                prefix,
                packet_bytes: chat.to_bytes(),
            });
        let _ = registry.try_send_current_command(target.registration, command);
    }
    #[cfg(test)]
    pub async fn handle_chat_message(&mut self, pkt: wow_packet::WorldPacket, msg_type: ChatMsg) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        self.handle_chat_message_with_policy_like_cpp(pkt, msg_type, &chat_policy)
            .await;
    }
    #[cfg(test)]
    pub async fn handle_chat_whisper(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        self.handle_chat_whisper_with_policy_like_cpp(pkt, &chat_policy)
            .await;
    }
    #[cfg(test)]
    pub async fn handle_chat_channel_message(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        self.handle_chat_channel_message_with_policy_like_cpp(pkt, &chat_policy)
            .await;
    }
    #[cfg(test)]
    pub async fn handle_chat_afk(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        self.handle_chat_afk_with_policy_like_cpp(pkt, &chat_policy)
            .await;
    }
    #[cfg(test)]
    pub async fn handle_chat_dnd(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        self.handle_chat_dnd_with_policy_like_cpp(pkt, &chat_policy)
            .await;
    }
    #[cfg(test)]
    pub async fn handle_chat_emote(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        self.handle_chat_emote_with_policy_like_cpp(pkt, &chat_policy)
            .await;
    }
    #[cfg(test)]
    pub async fn handle_chat_addon_message(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        self.handle_chat_addon_message_with_policy_like_cpp(pkt, &chat_policy)
            .await;
    }
    #[cfg(test)]
    pub async fn handle_chat_addon_message_targeted(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        self.handle_chat_addon_message_targeted_with_policy_like_cpp(pkt, &chat_policy)
            .await;
    }
    #[cfg(test)]
    pub async fn handle_chat_addon_message_whisper(&mut self, pkt: wow_packet::WorldPacket) {
        let chat_policy = self.chat_policy_catalogs_for_test_like_cpp();
        self.handle_chat_addon_message_whisper_with_policy_like_cpp(pkt, &chat_policy)
            .await;
    }
    pub(super) fn player_name_and_guid(&self) -> (wow_core::ObjectGuid, String) {
        let guid = self.player_guid().unwrap_or(wow_core::ObjectGuid::EMPTY);
        let name = self.player_name_like_cpp().unwrap_or_default().to_string();
        (guid, name)
    }
    pub(super) fn has_gm_silence_aura_like_cpp(&self) -> bool {
        self.resolved_player_visible_auras_like_cpp()
            .is_some_and(|auras| {
                auras
                    .values()
                    .any(|aura| aura.spell_id == GM_SILENCE_AURA_LIKE_CPP)
            })
    }
    pub(super) fn send_gm_silence_notification_like_cpp(&self) {
        let (_, sender_name) = self.player_name_and_guid();
        self.send_packet(&PrintNotification {
            notify_text: format!("Silence is ON for {sender_name}"),
        });
    }
    pub(super) fn send_wait_before_speaking_notification_if_muted_like_cpp(&self) -> bool {
        let Some(remaining_secs) = self.mute_time_remaining_secs_like_cpp() else {
            return false;
        };
        self.send_packet(&PrintNotification {
            notify_text: format!(
                "You must wait {} before speaking again.",
                secs_to_full_time_string_like_cpp(remaining_secs)
            ),
        });
        true
    }
    /// Serialize `pkt` and broadcast its bytes to all players on the same map
    /// instance within `range` yards (excluding the sender).
    pub(super) fn broadcast_chat_packet(&self, pkt: &ChatPkt, range: f32) {
        self.broadcast_raw_packet(pkt.to_bytes(), range);
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
    pub(super) fn broadcast_group_addon_chat_like_cpp(
        &self,
        msg_type: ChatMsg,
        packet: ChatAddonMessage,
    ) {
        let (sender_guid, sender_name) = self.player_name_and_guid();
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
            virtual_realm: self.virtual_realm_address(),
        };
        self.broadcast_group_addon_packet_like_cpp(
            &group,
            subgroup_filter,
            sender_guid,
            packet.prefix,
            chat.to_bytes(),
        );
    }
    pub(super) fn current_chat_group_like_cpp(&self, sender_guid: ObjectGuid) -> Option<GroupInfo> {
        let registry = self.group_registry()?;
        if let Some(group_guid) = self.resolved_group_guid_like_cpp()
            && let Some(group) = registry.get(&group_guid)
            && group.members.contains(&sender_guid)
        {
            return Some(group.clone());
        }

        registry
            .snapshots()
            .into_iter()
            .find(|group| group.members.contains(&sender_guid))
    }
    pub(super) fn broadcast_group_chat_packet_like_cpp(
        &self,
        group: &GroupInfo,
        subgroup_filter: Option<u8>,
        bytes: Vec<u8>,
    ) {
        let Some(registry) = self.player_registry() else {
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
    pub(super) fn broadcast_group_addon_packet_like_cpp(
        &self,
        group: &GroupInfo,
        subgroup_filter: Option<u8>,
        sender_guid: ObjectGuid,
        prefix: String,
        bytes: Vec<u8>,
    ) {
        let Some(registry) = self.player_registry() else {
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
    /// Send pre-serialised packet `bytes` to all players in the same map
    /// instance within `range` yards, excluding this session's player.
    pub(crate) fn broadcast_raw_packet(&self, bytes: Vec<u8>, range: f32) {
        let registry = match self.player_registry() {
            Some(r) => r,
            None => return,
        };

        let sender_guid = self.player_guid().unwrap_or(ObjectGuid::EMPTY);
        let sender_pos = self.player_position_like_cpp();
        let sender_map = self.player_map_id_like_cpp();
        let sender_instance = registry
            .runtime_recipient(sender_guid)
            .map(|recipient| recipient.instance_id)
            .or_else(|| {
                self.current_canonical_player_map_key_like_cpp()
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
}
