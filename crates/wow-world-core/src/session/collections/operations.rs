use std::collections::HashSet;

impl crate::session::HubMut<'_> {
    pub fn replace_completed_achievement_ids_like_cpp(
        &mut self,
        achievement_ids: impl IntoIterator<Item = u32>,
    ) -> bool {
        let achievement_ids: HashSet<_> = achievement_ids
            .into_iter()
            .filter(|achievement_id| *achievement_id != 0)
            .collect();
        let achievements = achievement_ids
            .iter()
            .copied()
            .map(|achievement_id| wow_entities::PlayerAchievementRecord {
                achievement_id,
                completed_at: None,
            })
            .collect::<Vec<_>>();
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.hydrate_completed_achievements_like_cpp(achievements);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .collections
                .represented_completed_achievements_like_cpp = achievement_ids;
            return true;
        }
        canonical
    }
}

impl crate::session::HubRef<'_> {
    pub fn replace_owned_player_mails_like_cpp(
        &self,
        mails: Vec<wow_entities::PlayerMailRecord>,
    ) -> bool {
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.hydrate_mails_like_cpp(mails);
            })
            .is_some()
    }

    pub fn owned_player_mails_like_cpp(
        &self,
    ) -> Option<Vec<wow_entities::PlayerMailRecord>> {
        self.core
            .with_owned_player_like_cpp(|player| player.gameplay_state().mails.clone())
    }
}

impl crate::session::HubRef<'_> {
    pub fn completed_achievement_ids_snapshot_like_cpp(
        &self,
    ) -> Option<HashSet<u32>> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .achievements
                .iter()
                .map(|achievement| achievement.achievement_id)
                .collect()
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                self.fixtures
                    .collections
                    .represented_completed_achievements_like_cpp
                    .clone(),
            );
        }
        canonical
    }
}
