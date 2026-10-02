use std::collections::BTreeSet;

impl crate::session::HubRef<'_> {
    pub fn resolved_player_skill_non_durable_tombstones_like_cpp(
        &self,
    ) -> Option<BTreeSet<u16>> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player.non_durable_skill_tombstones_like_cpp().clone()
        });
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
