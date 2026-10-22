use super::SessionLifecycleState;

/// C++ `ShouldLogOut`: `currTime >= _logoutTime + 20`.
pub const LOGOUT_DELAY_SECS_LIKE_CPP: i64 = 20;

impl SessionLifecycleState {
    pub fn clear_logout_time(&mut self) {
        self.set_logout_start_time_like_cpp(0);
    }

    /// C++ `WorldSession::SetLogoutStartTime` (`WorldSession.h:1004`).
    pub fn set_logout_start_time_like_cpp(&mut self, request_time: i64) {
        self.logout_time = request_time;
    }

    /// C++ `WorldSession::isLogingOut` (`WorldSession.h:1001`).
    pub fn is_logging_out_like_cpp(&self) -> bool {
        self.logout_time != 0 || self.player_logout_like_cpp
    }

    /// C++ `WorldSession::ShouldLogOut` (`WorldSession.h:1010`): the timed
    /// logout is due 20 game-time seconds after it started.
    pub fn should_log_out_like_cpp(&self, current_time: i64) -> bool {
        self.logout_time > 0 && current_time >= self.logout_time + LOGOUT_DELAY_SECS_LIKE_CPP
    }

    pub fn set_player_logout_like_cpp(&mut self, player_logout: bool) {
        self.player_logout_like_cpp = player_logout;
        if player_logout {
            self.durable_loot_money_persistence_like_cpp
                .close_admission_permanently_like_cpp();
        }
    }

    pub fn player_logout_like_cpp(&self) -> bool {
        self.player_logout_like_cpp
    }
}
