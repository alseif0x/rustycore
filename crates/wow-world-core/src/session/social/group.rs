use std::sync::Arc;

use crate::session::state::SessionCore;
use wow_social::group::GroupRegistry;

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
