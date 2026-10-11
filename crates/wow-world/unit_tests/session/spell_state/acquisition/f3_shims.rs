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

impl WorldSession {
    /// Compatibility facade for callers that already hold represented rows.
    /// Translation, validation and canonical/fixture installation are shared
    /// with the application runtime provider.
    pub(crate) fn replace_complete_spell_acquisition_runtime_like_cpp(
        &mut self,
        spell_rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
        traits: impl IntoIterator<Item = (i32, i32)>,
        overrides: impl IntoIterator<Item = (i32, i32)>,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        occupied_skill_slots: u16,
        non_durable_skill_tombstones: BTreeSet<u16>,
    ) -> bool {
        let owner = self.core.player_acquisition_owner_access_like_cpp();
        let result = wow_world_application::install_represented_spell_acquisition_runtime_like_cpp(
            &owner,
            &mut self.spell_state,
            spell_rows,
            traits,
            overrides,
            skill_records,
            occupied_skill_slots,
            non_durable_skill_tombstones,
            cfg!(test),
            #[cfg(any(test, feature = "test-fixtures"))]
            (
                &mut self.fixtures.progression.player_skill_test_fixture_like_cpp,
                &mut self.fixtures.progression.represented_enchanting_skill,
            ),
        );
        if result.is_ok() {
            self.sync_player_registry_state_like_cpp();
            true
        } else {
            false
        }
    }
}
