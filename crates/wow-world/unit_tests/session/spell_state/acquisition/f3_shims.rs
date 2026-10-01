// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_spell_acquisition_post_commit_actions_like_cpp(
        &self,
    ) -> &[crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp] {
        self.spell_state
            .represented_spell_acquisition_post_commit_actions_like_cpp()
    }
    pub(crate) fn record_spell_acquisition_post_commit_action_like_cpp(
        &mut self,
        action: crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp,
    ) {
        self.spell_state
            .record_spell_acquisition_post_commit_action_like_cpp(action)
    }
}
