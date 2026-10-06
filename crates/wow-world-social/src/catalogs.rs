use crate::{ChatFloodThrottleIndexLikeCpp, SessionSocialLimits};
use wow_world_core::session::HubMut;
use wow_world_core::session::connection_identity::unix_now;
use wow_world_core::session_policy::ChatFloodConfigLikeCpp;

impl SessionSocialLimits {
    pub fn update_speak_time_with_policy_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        index: ChatFloodThrottleIndexLikeCpp,
        config: ChatFloodConfigLikeCpp,
    ) {
        // C++ skips chat spam checks for RBAC_PERM_SKIP_CHECK_CHAT_SPAM. RustyCore
        // has no RBAC store yet; represented GM state is the current session seam.
        if hub.shared().player_is_game_master_like_cpp() == Some(true) {
            return;
        }

        let (limit, delay_secs) = match index {
            ChatFloodThrottleIndexLikeCpp::Regular => {
                (config.message_count, config.message_delay_secs)
            }
            ChatFloodThrottleIndexLikeCpp::Addon => {
                (config.addon_message_count, config.addon_message_delay_secs)
            }
        };
        let current = unix_now();
        let data = &mut self.chat_flood_data_like_cpp[index as usize];

        if data.time > current {
            if limit == 0 {
                return;
            }

            data.count = data.count.saturating_add(1);
            if data.count >= limit {
                let new_mute = current.saturating_add(i64::from(config.mute_time_secs));
                if hub.core.account_state.mute_time_like_cpp < new_mute {
                    hub.core.account_state.mute_time_like_cpp = new_mute;
                }
                data.count = 0;
            }
        } else {
            data.count = 1;
        }

        data.time = current.saturating_add(i64::from(delay_secs));
    }
}
