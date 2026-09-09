//! Chat handlers state definitions, part 1 of 1.
//!
//! Separated from the chat.rs root under #654. Behaviour is preserved.

use super::*;

pub(super) const LANG_UNIVERSAL_LIKE_CPP: i32 = 0;

pub(super) const LANG_ADDON_LIKE_CPP: u32 = 183;

pub(super) const LANG_ADDON_LOGGED_LIKE_CPP: u32 = 184;

pub(super) const GM_SILENCE_AURA_LIKE_CPP: i32 = 1852;

pub(super) const EMOTE_ONESHOT_NONE_LIKE_CPP: i32 = 0;

pub(super) const EMOTE_STATE_DANCE_LIKE_CPP: i32 = 10;

pub(super) const EMOTE_STATE_SLEEP_LIKE_CPP: i32 = 12;

pub(super) const EMOTE_STATE_SIT_LIKE_CPP: i32 = 13;

pub(super) const EMOTE_STATE_KNEEL_LIKE_CPP: i32 = 68;

pub(super) const EMOTE_STATE_READ_LIKE_CPP: i32 = 483;

pub(super) const ANIM_MOUNT_SPECIAL_LIKE_CPP: i32 = 94;

pub(super) const ANIM_MOUNT_SELF_SPECIAL_LIKE_CPP: i32 = 636;

pub(super) const KNOWN_LANGUAGES_LIKE_CPP: &[i32] = &[
    // C++ `LanguageMgr::LoadLanguages` accepts Languages.db2 entries plus the
    // code-only languages registered in `SharedDefines.h`.
    0, 1, 2, 3, 6, 7, 8, 9, 10, 11, 12, 13, 14, 33, 35, 36, 37, 38, 39, 40, 42, 43, 44, 168, 178,
    179, 180, 181, 182, 183, 184, 285, 287, 288, 290, 291, 292, 293, 294, 295, 296, 297, 298,
];

pub(super) fn chat_msg_from_i32_like_cpp(value: i32) -> Option<ChatMsg> {
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

pub(super) fn is_known_language_like_cpp(language: i32) -> bool {
    KNOWN_LANGUAGES_LIKE_CPP.contains(&language)
}

impl WorldSession {
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

    pub(super) fn meets_chat_level_req_with_policy_like_cpp(
        &self,
        msg_type: ChatMsg,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) -> bool {
        Self::required_chat_level_with_policy_like_cpp(msg_type, chat_policy)
            .is_none_or(|required| self.player_level_like_cpp() >= required)
    }

    pub(super) fn meets_whisper_level_req_with_policy_like_cpp(
        &self,
        chat_policy: &ChatPolicyCatalogsLikeCpp,
    ) -> bool {
        self.player_is_game_master_like_cpp() == Some(true)
            || self.player_level_like_cpp() >= chat_policy.level_requirements.whisper
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
            virtual_realm: self.virtual_realm_address(),
        };
        self.send_packet(&packet);
    }

    pub(super) fn send_chat_say_level_notification_like_cpp(&self, required_level: u8) {
        self.send_packet(&PrintNotification {
            notify_text: format!(
                "You cannot say, yell or emote until you become level {required_level}."
            ),
        });
    }

    pub(super) fn send_chat_whisper_level_notification_like_cpp(&self, required_level: u8) {
        self.send_packet(&PrintNotification {
            notify_text: format!("You cannot whisper until you become level {required_level}."),
        });
    }
}

pub(super) fn secs_to_full_time_string_like_cpp(time_in_secs: u64) -> String {
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
