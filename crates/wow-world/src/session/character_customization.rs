// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character customization: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

#[cfg(test)]
pub(crate) use wow_world_lifecycle::RepresentedAtLoginFlagRemovalLikeCpp;

#[cfg(test)]
pub(crate) use wow_world_core::session::{
    RepresentedAlterAppearanceLikeCpp, RepresentedConfirmBarbersChoiceLikeCpp,
    RepresentedTalentResetScriptHookLikeCpp, RepresentedTalentRespecCriteriaEventLikeCpp,
};
pub(crate) use wow_world_core::session::{
    RepresentedConfirmRespecWipeLikeCpp, RepresentedTalentRespecVisualSpellCastLikeCpp,
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

#[cfg(test)]
#[path = "../../unit_tests/session/character_customization/test_only.rs"]
mod test_only;
