use std::sync::Arc;

#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::GroupInvitePolicyLikeCpp;
use crate::session::state::SessionCore;
use wow_social::group::GroupRegistry;

impl crate::session::state::SessionWorldConfig {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn group_invite_policy_for_test_like_cpp(&self) -> GroupInvitePolicyLikeCpp {
        GroupInvitePolicyLikeCpp {
            allow_gm_group: self.allow_gm_group_like_cpp,
            allow_two_side_interaction: self.allow_two_side_interaction_group_like_cpp,
            minimum_level: self.party_level_req_like_cpp,
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_party_raid_warnings_like_cpp(&mut self, enabled: bool) {
        self.party_raid_warnings_like_cpp = enabled;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_allow_gm_group_like_cpp(&mut self, enabled: bool) {
        self.allow_gm_group_like_cpp = enabled;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_party_level_req_like_cpp(&mut self, level: u32) {
        self.party_level_req_like_cpp = level;
    }
}

impl SessionCore {
    /// Get a reference to the shared group registry.
    pub fn group_registry(&self) -> Option<&Arc<GroupRegistry>> {
        self.directory.group_registry.as_ref()
    }

    pub fn party_member_party_type_like_cpp(&self) -> [u8; 2] {
        let mut party_type = [wow_social::group::GROUP_TYPE_NONE_LIKE_CPP; 2];
        let (Some(group_registry), Some(player_guid)) =
            (&self.directory.group_registry, self.player_guid())
        else {
            return party_type;
        };

        for group in group_registry.snapshots() {
            let category = group.group_category_like_cpp();
            if category < wow_social::group::MAX_GROUP_CATEGORY_LIKE_CPP
                && group.members.contains(&player_guid)
            {
                party_type[usize::from(category)] = wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP;
            }
        }

        party_type
    }
}
