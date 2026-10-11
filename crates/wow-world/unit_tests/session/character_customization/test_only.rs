// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `character_customization` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    pub(crate) fn represented_learn_title_like_cpp(&mut self, title_id: u32) {
        let _canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.learn_title_like_cpp(title_id))
            .is_some();
        #[cfg(test)]
        if !_canonical && self.core.player_handle_like_cpp.is_none() {
            self.quest_state
                .fixture_set_represented_known_title_like_cpp(title_id, true);
        }
    }
    pub(crate) fn represented_has_title_like_cpp(&self, title_id: u32) -> bool {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.has_title_like_cpp(title_id));
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return self
                .quest_state
                .fixture_has_represented_known_title_like_cpp(title_id);
        }
        canonical.unwrap_or(false)
    }
    pub(crate) fn represented_set_chosen_title_like_cpp(&mut self, title_id: i32) {
        let _canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_chosen_title_like_cpp(title_id))
            .is_some();
        #[cfg(test)]
        if !_canonical && self.core.player_handle_like_cpp.is_none() {
            self.quest_state
                .fixture_set_represented_chosen_title_like_cpp(title_id);
        }
    }
}
