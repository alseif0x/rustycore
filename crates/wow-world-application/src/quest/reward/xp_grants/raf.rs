// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Recruit-A-Friend query shared by XP and the other selected App operations.

use wow_world_core::session::{CoreXPGainAccessLikeCpp, SessionWorldConfig};
use wow_world_social::SessionSocialLimits;

#[cfg(any(test, feature = "test-fixtures"))]
use super::QuestXpGainFixtureRefsLikeCpp;
use super::super::QuestRewardCx;

impl QuestRewardCx<'_> {
    pub(super) fn gets_recruit_a_friend_xp_bonus_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &QuestXpGainFixtureRefsLikeCpp<'_>,
    ) -> bool {
        Self::gets_recruit_a_friend_bonus_like_cpp(
            self.xp_gain_access_like_cpp(),
            self.social,
            self.config,
            true,
            self.world_test_consumer,
            #[cfg(any(test, feature = "test-fixtures"))]
            &*self.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures.core.player_position_fixture_like_cpp(),
        )
    }

    /// Shared RAF query used by XP, reputation and rest-state transitions.
    /// Each caller selects its original policy mode and supplies only the
    /// detached position/level fallbacks when that owner is test-enabled.
    pub fn gets_recruit_a_friend_bonus_like_cpp(
        player: CoreXPGainAccessLikeCpp<'_>,
        social: &SessionSocialLimits,
        config: &SessionWorldConfig,
        for_xp: bool,
        world_test_consumer: bool,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_position: &Option<wow_core::Position>,
    ) -> bool {
        Self::gets_recruit_a_friend_bonus_from_access_like_cpp(
            &player,
            social,
            config,
            for_xp,
            world_test_consumer,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_position,
        )
    }

    pub(super) fn gets_recruit_a_friend_bonus_from_access_like_cpp(
        player: &CoreXPGainAccessLikeCpp<'_>,
        social: &SessionSocialLimits,
        config: &SessionWorldConfig,
        for_xp: bool,
        world_test_consumer: bool,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_position: &Option<wow_core::Position>,
    ) -> bool {
        if !player.session_is_logged_in_like_cpp() {
            return false;
        }
        let player_level = u32::from(player.player_level_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_level,
        ));
        if for_xp && player_level > social.recruit_a_friend_xp_limits_like_cpp().0 {
            return false;
        }

        let (Some(player_guid), Some(group_guid)) = (
            player.player_guid_like_cpp(),
            {
                let owner = player.player_group_owner_access_like_cpp();
                social.resolved_group_guid_with_access_like_cpp(&owner, world_test_consumer)
            },
        ) else {
            return false;
        };
        let Some(group_members) =
            player.group_members_for_player_like_cpp(group_guid, player_guid)
        else {
            return false;
        };
        let Some(player_position) = player.player_position_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_position,
        ) else {
            return false;
        };
        let player_map_id = player.player_map_id_like_cpp();
        let player_instance_id = player.player_instance_id_like_cpp().unwrap_or(0);
        let max_distance = config
            .reputation_rates_like_cpp()
            .recruit_a_friend_distance;

        for member_guid in group_members {
            if member_guid == player_guid {
                continue;
            }
            let Some(member) = player.group_member_presence_like_cpp(member_guid) else {
                continue;
            };
            if member.map_id != player_map_id {
                continue;
            }
            if member.instance_id != player_instance_id {
                continue;
            }
            if !member.is_in_world {
                continue;
            }
            if !member.is_alive {
                continue;
            }
            if member.position.distance(&player_position) > max_distance {
                continue;
            }
            if for_xp {
                let member_level = u32::from(member.level);
                let (max_player_level, max_level_difference) =
                    social.recruit_a_friend_xp_limits_like_cpp();
                if member_level > max_player_level {
                    continue;
                }
                if member_level < player_level
                    && player_level - member_level > max_level_difference
                {
                    continue;
                }
            }

            let member_recruited_self = member.recruiter_id == player.account_id_like_cpp();
            let self_recruited_member = player.recruiter_id_like_cpp() == member.account_id;
            if member_recruited_self || self_recruited_member {
                return true;
            }
        }

        false
    }
}
