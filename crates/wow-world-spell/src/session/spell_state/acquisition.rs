#[cfg(any(test, feature = "test-fixtures"))]
use crate::records::{
    RepresentedPlayerSpellRuntimeLikeCpp, canonical_player_spell_runtime_like_cpp,
    represented_player_spell_runtime_like_cpp,
};

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

    /// Read the existing spell fixture in the same represented shape used by
    /// the World runtime snapshot path.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_spell_runtime_fixture_like_cpp(
        &self,
    ) -> crate::RepresentedPlayerSpellRuntimeLikeCpp {
        let fixture = &self.player_spell_test_fixture_like_cpp;
        crate::RepresentedPlayerSpellRuntimeLikeCpp {
            known_spells: fixture.known_spells.clone(),
            rows: fixture.represented_player_spell_rows_like_cpp.clone(),
            rows_loaded: fixture.represented_player_spell_rows_loaded_like_cpp,
            rows_complete: fixture.represented_player_spell_rows_complete_like_cpp,
            fallback_rows: fixture.represented_fallback_player_spell_rows_like_cpp.clone(),
            dependent_known_spells: fixture
                .represented_dependent_known_spells_like_cpp
                .clone(),
            removed_known_spells: fixture.represented_removed_known_spells_like_cpp.clone(),
            favorite_known_spells: fixture.represented_favorite_known_spells_like_cpp.clone(),
            trait_definition_ids: fixture
                .represented_spell_trait_definition_ids_like_cpp
                .clone(),
            trait_definition_ids_complete: fixture
                .represented_spell_trait_definition_ids_complete_like_cpp,
            trait_config_rows: fixture.represented_trait_config_rows_like_cpp.clone(),
            trait_config_rows_complete: fixture.represented_trait_config_rows_complete_like_cpp,
            trait_entry_rows_complete: fixture.represented_trait_entry_rows_complete_like_cpp,
            trait_entry_rows_empty: fixture.represented_trait_entry_rows_empty_like_cpp,
            override_spells: self.represented_override_spells_like_cpp.clone(),
            override_spells_complete: self.represented_override_spells_complete_like_cpp,
        }
    }

    /// Store one represented spell runtime in the existing fixture owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn store_represented_spell_runtime_fixture_like_cpp(
        &mut self,
        runtime: crate::RepresentedPlayerSpellRuntimeLikeCpp,
    ) {
        self.player_spell_test_fixture_like_cpp.known_spells = runtime.known_spells;
        self.player_spell_test_fixture_like_cpp.represented_player_spell_rows_like_cpp =
            runtime.rows;
        self.player_spell_test_fixture_like_cpp.represented_player_spell_rows_loaded_like_cpp =
            runtime.rows_loaded;
        self.player_spell_test_fixture_like_cpp.represented_player_spell_rows_complete_like_cpp =
            runtime.rows_complete;
        self.player_spell_test_fixture_like_cpp.represented_fallback_player_spell_rows_like_cpp =
            runtime.fallback_rows;
        self.player_spell_test_fixture_like_cpp.represented_dependent_known_spells_like_cpp =
            runtime.dependent_known_spells;
        self.player_spell_test_fixture_like_cpp.represented_removed_known_spells_like_cpp =
            runtime.removed_known_spells;
        self.player_spell_test_fixture_like_cpp.represented_favorite_known_spells_like_cpp =
            runtime.favorite_known_spells;
        self.player_spell_test_fixture_like_cpp.represented_spell_trait_definition_ids_like_cpp =
            runtime.trait_definition_ids;
        self.player_spell_test_fixture_like_cpp
            .represented_spell_trait_definition_ids_complete_like_cpp =
            runtime.trait_definition_ids_complete;
        self.player_spell_test_fixture_like_cpp.represented_trait_config_rows_like_cpp =
            runtime.trait_config_rows;
        self.player_spell_test_fixture_like_cpp.represented_trait_config_rows_complete_like_cpp =
            runtime.trait_config_rows_complete;
        self.player_spell_test_fixture_like_cpp.represented_trait_entry_rows_complete_like_cpp =
            runtime.trait_entry_rows_complete;
        self.player_spell_test_fixture_like_cpp.represented_trait_entry_rows_empty_like_cpp =
            runtime.trait_entry_rows_empty;
        self.replace_represented_override_spell_fixture_like_cpp(
            runtime.override_spells,
            runtime.override_spells_complete,
        );
    }

    /// Apply the acquisition image through the same represented/canonical
    /// conversions used by fixture runtime mutations, retaining current
    /// fallback rows and trait-config evidence from `runtime`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn install_complete_spell_acquisition_fixture_like_cpp(
        &mut self,
        runtime: crate::RepresentedPlayerSpellRuntimeLikeCpp,
        snapshot: wow_entities::PlayerSpellAcquisitionSnapshotLikeCpp,
    ) {
        let mut runtime = canonical_player_spell_runtime_like_cpp(runtime);
        runtime.install_acquisition_snapshot_like_cpp(snapshot);
        self.store_represented_spell_runtime_fixture_like_cpp(
            represented_player_spell_runtime_like_cpp(&runtime),
        );
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
