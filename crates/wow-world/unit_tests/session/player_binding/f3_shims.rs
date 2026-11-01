// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn set_player_character_points_like_cpp(&mut self, points: i32) -> bool {
        crate::session::hub_mut(self).set_player_character_points_like_cpp(points)
    }
    #[cfg(test)]
    pub(crate) fn represented_can_swim_to_fly_transition_like_cpp(&self) -> bool {
        crate::session::hub_ref(self).represented_can_swim_to_fly_transition_like_cpp()
    }
    pub(in crate::session) fn resolved_can_swim_to_fly_transition_like_cpp(&self) -> Option<bool> {
        crate::session::hub_ref(self).resolved_can_swim_to_fly_transition_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn set_player_faction_template_like_cpp(&mut self, faction_template: u32) {
        crate::session::hub_mut(self).set_player_faction_template_like_cpp(faction_template)
    }
    pub(in crate::session) fn resolved_player_scale_duration_like_cpp(&self) -> Option<i32> {
        crate::session::hub_ref(self).resolved_player_scale_duration_like_cpp()
    }
    pub(in crate::session) fn player_world_local_state_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerWorldLocalState> {
        crate::session::hub_ref(self).player_world_local_state_like_cpp()
    }
    pub(crate) fn set_player_create_mode_like_cpp(&mut self, create_mode: u8) -> bool {
        crate::session::hub_mut(self).set_player_create_mode_like_cpp(create_mode)
    }
}

impl WorldSession {
    pub(crate) fn set_player_liquid_status_like_cpp(&mut self, status: u32) {
        crate::canonical_player_sync::sync_player_liquid_status_like_cpp(self, status);
    }
    pub(crate) fn set_player_level_like_cpp(&mut self, level: u8) {
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
        wow_world_quest::QuestRewardCx::set_player_level_and_refresh_talent_points_like_cpp(
            &mut player,
            level,
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
        wow_world_quest::QuestRewardCx::set_player_level_and_refresh_talent_points_like_cpp(
            &mut player,
            level,
            catalogs,
            world_test_consumer,
        );
    }
}
