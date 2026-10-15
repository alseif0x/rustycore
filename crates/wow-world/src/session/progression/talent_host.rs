//! The World session as the host of the talent bodies moved to
//! `wow_world_spell` (#1363 slice 9b). Each operation names the inherent
//! `WorldSession` method or the moved free function (slice 12a) by path.

use super::*;

impl wow_world_spell::TalentHostLikeCpp for WorldSession {
    fn refresh_represented_talent_points_like_cpp(&mut self) {
        wow_world_quest::refresh_represented_talent_points_like_cpp(self)
    }
    fn resolved_represented_at_login_flags_like_cpp(&self) -> Option<u16> {
        WorldSession::resolved_represented_at_login_flags_like_cpp(self)
    }
    fn remove_represented_at_login_flag_like_cpp(&mut self, flags: u16, persist: bool) -> bool {
        WorldSession::remove_represented_at_login_flag_like_cpp(self, flags, persist)
    }
    fn represented_talents_loaded_like_cpp(&self) -> bool {
        WorldSession::represented_talents_loaded_like_cpp(self)
    }
    fn trinity_string_like_cpp(&self, entry: u32) -> &str {
        WorldSession::trinity_string_like_cpp(self, entry)
    }
}
