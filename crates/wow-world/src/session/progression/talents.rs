//! Represented talents and glyph slots, and their published state.
//!
//! Moved out of the Session root under #611. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_talent_store(&mut self, store: Arc<TalentStore>) {
        self.catalogs.talent_store = Some(store);
    }
    pub fn set_num_talents_at_level_store(&mut self, store: Arc<NumTalentsAtLevelStore>) {
        self.catalogs.num_talents_at_level_store = Some(store);
        self.refresh_represented_talent_points_like_cpp();
    }

    pub(crate) fn refresh_represented_talent_points_like_cpp(&mut self) {
        let catalogs = &self.catalogs;
        #[cfg(any(test, feature = "test-fixtures"))]
        let quest_state = &self.quest_state;
        let world_test_consumer = cfg!(test);
        #[cfg(any(test, feature = "test-fixtures"))]
        let mut player = self.core.quest_reward_player_access_like_cpp(
            &self.fixtures.identity.player_race,
            &self.fixtures.identity.player_class,
        );
        #[cfg(any(test, feature = "test-fixtures"))]
        wow_world_quest::QuestRewardCx::refresh_represented_talent_points_like_cpp(
            &mut player,
            catalogs,
            quest_state,
            world_test_consumer,
            &mut self.fixtures.identity.player_level,
            &self
                .fixtures
                .progression
                .represented_gray_level_script_overrides_like_cpp,
            &self.fixtures.progression.represented_talents_like_cpp,
            &self
                .fixtures
                .progression
                .represented_active_talent_group_like_cpp,
            &mut self.fixtures.progression.player_character_points_like_cpp,
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let mut player = self.core.quest_reward_player_access_like_cpp();
        #[cfg(not(any(test, feature = "test-fixtures")))]
        wow_world_quest::QuestRewardCx::refresh_represented_talent_points_like_cpp(
            &mut player,
            catalogs,
            world_test_consumer,
        );
    }
}
