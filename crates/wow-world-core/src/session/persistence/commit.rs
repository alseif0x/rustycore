use std::collections::BTreeSet;

impl crate::session::HubRef<'_> {
    pub fn resolved_player_skill_non_durable_tombstones_like_cpp(&self) -> Option<BTreeSet<u16>> {
        let canonical = self
            .core
            .owned_spell_acquisition_access_like_cpp()
            .skill_non_durable_tombstones_snapshot_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                self.fixtures
                    .progression
                    .player_skill_test_fixture_like_cpp
                    .player_skill_non_durable_tombstones_like_cpp
                    .clone(),
            );
        }
        canonical
    }
}
