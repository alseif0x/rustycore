// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character customization: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

#[cfg(test)]
pub(crate) use wow_world_lifecycle::RepresentedAtLoginFlagRemovalLikeCpp;

pub(crate) use wow_world_core::session::{
    RepresentedAlterAppearanceLikeCpp, RepresentedConfirmBarbersChoiceLikeCpp,
    RepresentedConfirmRespecWipeLikeCpp, RepresentedTalentRespecVisualSpellCastLikeCpp,
};
#[cfg(test)]
pub(crate) use wow_world_core::session::{
    RepresentedTalentResetScriptHookLikeCpp, RepresentedTalentRespecCriteriaEventLikeCpp,
};

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

    #[cfg(test)]
    pub(crate) fn represented_chosen_title_like_cpp(&self) -> i32 {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.data().player_title);
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return self.quest_state.fixture_represented_chosen_title_like_cpp();
        }
        canonical.expect("test Player title owner must resolve")
    }
}
