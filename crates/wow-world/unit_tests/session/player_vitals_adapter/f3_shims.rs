// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn player_is_alive_like_cpp(&self) -> bool {
        crate::session::hub_ref(self).player_is_alive_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn set_player_game_master_like_cpp(&mut self, is_game_master: bool) {
        crate::session::hub_mut(self).set_player_game_master_like_cpp(is_game_master)
    }
    #[cfg(test)]
    pub(crate) fn set_player_mounted_like_cpp(&mut self, mounted: bool) {
        crate::session::hub_mut(self).set_player_mounted_like_cpp(mounted)
    }
    #[cfg(test)]
    pub(crate) fn player_mounted_like_cpp(&self) -> bool {
        crate::session::hub_ref(self).player_mounted_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn set_player_cheat_god_like_cpp(&mut self, enabled: bool) {
        crate::session::hub_mut(self).set_player_cheat_god_like_cpp(enabled)
    }
}
