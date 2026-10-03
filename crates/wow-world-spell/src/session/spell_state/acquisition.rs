impl crate::SessionSpellState {
    pub fn record_spell_acquisition_post_commit_action_like_cpp(
        &mut self,
        action: wow_spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.represented_spell_acquisition_post_commit_actions_like_cpp
            .push(action);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = action;
    }
    pub fn begin_spell_acquisition_post_commit_action_batch_like_cpp(&mut self) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.represented_spell_acquisition_post_commit_actions_like_cpp
            .clear();
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_spell_acquisition_post_commit_actions_like_cpp(
        &self,
    ) -> &[wow_spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp] {
        &self.represented_spell_acquisition_post_commit_actions_like_cpp
    }
    pub fn grant_dual_wield_after_spell_acquisition_like_cpp(
        &mut self,
        hub: &mut wow_world_core::session::HubMut<'_>,
    ) -> bool {
        hub.core
            .owned_spell_acquisition_access_like_cpp()
            .grant_dual_wield_after_acquisition_like_cpp()
    }
    pub fn has_canonical_player_for_spell_acquisition_like_cpp(
        &self,
        hub: wow_world_core::session::HubRef<'_>,
    ) -> bool {
        hub.core
            .owned_spell_acquisition_access_like_cpp()
            .has_canonical_player_like_cpp()
    }
}
