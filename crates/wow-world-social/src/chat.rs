use crate::{PlayerAwayModeLikeCpp, SessionSocialLimits};
use wow_world_core::session::HubMut;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::HubRef;

impl SessionSocialLimits {
    pub fn apply_chat_away_mode_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        mode: PlayerAwayModeLikeCpp,
        text: String,
    ) -> bool {
        if hub.shared().resolved_in_combat_like_cpp() != Some(false) || text.len() > 511 {
            return false;
        }

        if hub.core.player_guid().is_none() {
            return false;
        }

        let default_text = match mode {
            PlayerAwayModeLikeCpp::Afk => "Away from Keyboard",
            PlayerAwayModeLikeCpp::Dnd => "Do not Disturb",
        };

        // C++ `WorldSession::HandleChatMessageAFKOpcode` (ChatHandler.cpp:594)
        // and its DND twin (:640) compose two Player transitions in this exact
        // order: assign the auto-reply message, clear the opposite mode, then
        // toggle this one. The session adapts the packet; the Player owns both
        // steps.
        //
        // Unimplemented participant: Classic then notifies the guild through
        // `Guild::SendEventAwayChanged`.
        hub.core
            .mutate_canonical_player_like_cpp(move |player| {
                let already_active = match mode {
                    PlayerAwayModeLikeCpp::Afk => player.is_afk_like_cpp(),
                    PlayerAwayModeLikeCpp::Dnd => player.is_dnd_like_cpp(),
                };
                if already_active {
                    if text.is_empty() {
                        match mode {
                            PlayerAwayModeLikeCpp::Afk => player.toggle_afk_like_cpp(),
                            PlayerAwayModeLikeCpp::Dnd => player.toggle_dnd_like_cpp(),
                        }
                    } else {
                        player.set_auto_reply_message_like_cpp(text);
                    }
                    return;
                }

                player.set_auto_reply_message_like_cpp(if text.is_empty() {
                    default_text.to_string()
                } else {
                    text
                });
                match mode {
                    PlayerAwayModeLikeCpp::Afk => {
                        if player.is_dnd_like_cpp() {
                            player.toggle_dnd_like_cpp();
                        }
                        player.toggle_afk_like_cpp();
                    }
                    PlayerAwayModeLikeCpp::Dnd => {
                        if player.is_afk_like_cpp() {
                            player.toggle_afk_like_cpp();
                        }
                        player.toggle_dnd_like_cpp();
                    }
                }
            })
            .is_some()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_emote_state_like_cpp(&self, hub: HubRef<'_>) -> u32 {
        hub.resolved_player_emote_state_like_cpp()
            .expect("test Player emote-state owner must resolve")
    }
}
