use super::SessionLifecycleState;
use std::time::Instant;

impl SessionLifecycleState {
    pub fn total_played_time_like_cpp(&self) -> u32 {
        self.total_played_time
    }

    pub fn set_total_played_time_like_cpp(&mut self, total_played_time: u32) {
        self.total_played_time = total_played_time;
    }

    pub fn level_played_time_like_cpp(&self) -> u32 {
        self.level_played_time
    }

    pub fn set_level_played_time_like_cpp(&mut self, level_played_time: u32) {
        self.level_played_time = level_played_time;
    }

    pub fn login_time_like_cpp(&self) -> Option<Instant> {
        self.login_time
    }

    pub fn set_login_time_like_cpp(&mut self, login_time: Option<Instant>) {
        self.login_time = login_time;
    }
}
