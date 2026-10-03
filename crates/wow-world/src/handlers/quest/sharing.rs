// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest sharing between party members.

use super::*;

impl WorldSession {
    pub(super) fn represented_can_share_quest_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        quest.flags & QUEST_FLAGS_SHARABLE_LIKE_CPP != 0
            && self
                .represented_player_quest_status_like_cpp(quest.id)
                .is_some_and(|status| status.is_some())
    }

    pub(super) fn send_push_quest_result_to_sender_if_available_like_cpp(
        &self,
        sender_guid: Option<ObjectGuid>,
        result: u8,
    ) {
        if let Some(sender_guid) = sender_guid {
            self.send_packet(&QuestPushResultResponse {
                sender_guid,
                result,
                quest_title: String::new(),
            });
        }
    }

    pub(super) fn send_push_quest_result_to_sender_with_title_if_available_like_cpp(
        &self,
        sender_guid: ObjectGuid,
        result: u8,
        quest_title: String,
    ) {
        self.send_packet(&QuestPushResultResponse {
            sender_guid,
            result,
            quest_title,
        });
    }
}
