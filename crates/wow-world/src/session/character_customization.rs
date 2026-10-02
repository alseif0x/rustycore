// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character customization: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

pub(crate) use wow_world_core::session::{
    RepresentedAlterAppearanceLikeCpp, RepresentedConfirmBarbersChoiceLikeCpp,
    RepresentedConfirmRespecWipeLikeCpp, RepresentedTalentRespecVisualSpellCastLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_core::session::{
    RepresentedTalentResetScriptHookLikeCpp, RepresentedTalentRespecCriteriaEventLikeCpp,
};

/// Evidence for C++ `RemoveAtLoginFlag(flags, persist=true)`.
///
/// Non-persistent at-login removals intentionally mutate only the represented
/// in-memory flag field and do not push this boundary record.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedAtLoginFlagRemovalLikeCpp {
    pub flags: u16,
    pub persist: bool,
    pub db_statement_unrepresented: bool,
}

impl WorldSession {
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_confirm_respec_wipe_like_cpp(
        &mut self,
        request: RepresentedConfirmRespecWipeLikeCpp,
    ) {
        #[cfg(test)]
        self.fixtures
            .progression
            .represented_confirm_respec_wipe_requests_like_cpp
            .push(request);
    }

    #[cfg(test)]
    pub(crate) fn delayed_operations_processed_like_cpp(&self) -> u32 {
        self.fixtures.movement.delayed_operations_processed_like_cpp
    }

    pub(crate) fn represented_learn_title_like_cpp(&mut self, title_id: u32) {
        let _canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.learn_title_like_cpp(title_id))
            .is_some();
        #[cfg(test)]
        if !_canonical && self.core.player_handle_like_cpp.is_none() {
            self.quest_state
                .quest_test_fixture_like_cpp
                .represented_known_titles_like_cpp
                .insert(title_id);
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
                .quest_test_fixture_like_cpp
                .represented_known_titles_like_cpp
                .contains(&title_id);
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
                .quest_test_fixture_like_cpp
                .represented_chosen_title_like_cpp = title_id;
        }
    }

    #[cfg(test)]
    pub(crate) fn represented_chosen_title_like_cpp(&self) -> i32 {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.data().player_title);
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return self
                .quest_state
                .quest_test_fixture_like_cpp
                .represented_chosen_title_like_cpp;
        }
        canonical.expect("test Player title owner must resolve")
    }
}
