//! Represented friend and ignore lists.
//!
//! Moved out of the Session root under #615. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_friendship_rep_reaction_store(&mut self, store: Arc<FriendshipRepReactionStore>) {
        self.catalogs.friendship_rep_reaction_store = Some(store);
    }
    pub(in crate::session) fn represented_recruit_a_friend_xp_rest_state_applies_like_cpp(
        &self,
    ) -> bool {
        self.gets_recruit_a_friend_xp_bonus_like_cpp()
    }
    pub(in crate::session) fn gets_recruit_a_friend_xp_bonus_like_cpp(&self) -> bool {
        self.gets_recruit_a_friend_bonus_like_cpp(true)
    }
    pub(in crate::session) fn gets_recruit_a_friend_bonus_like_cpp(&self, for_xp: bool) -> bool {
        wow_world_application::QuestRewardCx::gets_recruit_a_friend_bonus_like_cpp(
            self.core
                .xp_gain_access_like_cpp(&self.catalogs, &self.config),
            &self.social,
            &self.config,
            for_xp,
            cfg!(test),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.movement.player_position,
        )
    }
    pub fn set_recruit_a_friend_xp_config_like_cpp(
        &mut self,
        max_bonus_level: u32,
        max_level_difference: u32,
    ) {
        self.social.set_recruit_a_friend_xp_limits_like_cpp(max_bonus_level, max_level_difference);
    }
}
