// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player group membership queries for difficulty and visibility.

use crate::session::state::SessionCore;
use wow_core::ObjectGuid;
use wow_social::group::GroupRegistry;

pub struct PlayerGroupOwnerAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    pub fn player_group_owner_access_like_cpp(&self) -> PlayerGroupOwnerAccessLikeCpp<'_> {
        PlayerGroupOwnerAccessLikeCpp { core: self }
    }
}

impl PlayerGroupOwnerAccessLikeCpp<'_> {
    /// `None` means the canonical Player did not resolve; `Some(None)` means
    /// it resolved and has no group membership.
    pub fn canonical_group_guid_like_cpp(&self) -> Option<Option<u64>> {
        self.core.with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .group
                .as_ref()
                .map(|group| group.group_guid.counter() as u64)
        })
    }

    /// Return the current Player group identity and authoritative registry
    /// members for update-field visibility. The Player-owned guid is only a
    /// lookup candidate; registry membership decides whether it is current.
    pub fn party_group_for_values_update_like_cpp(&self) -> Option<(u64, Vec<ObjectGuid>)> {
        let player_guid = self.core.player_guid()?;
        let group_guid = self.canonical_group_guid_like_cpp().flatten()?;
        let registry = self.core.group_registry()?;
        authoritative_current_group_members_like_cpp(registry, group_guid, player_guid)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }
}

fn authoritative_current_group_members_like_cpp(
    registry: &GroupRegistry,
    group_guid: u64,
    player_guid: ObjectGuid,
) -> Option<(u64, Vec<ObjectGuid>)> {
    let group = registry.get(&group_guid)?;
    (group.group_guid == group_guid && group.members.contains(&player_guid))
        .then_some((group_guid, group.members))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_social::group::GroupInfo;

    #[test]
    fn group_membership_requires_current_registry_authority() {
        let registry = GroupRegistry::new();
        let viewer = ObjectGuid::create_player(1, 7101);
        let party_member = ObjectGuid::create_player(1, 7102);
        let unrelated = ObjectGuid::create_player(1, 7103);
        let mut group = GroupInfo::new(viewer);
        group.members.push(party_member);
        let group_guid = group.group_guid;
        registry.register_group_like_cpp(group_guid, group);

        let (active_group_guid, members) =
            authoritative_current_group_members_like_cpp(&registry, group_guid, viewer)
                .expect("current group membership");
        assert_eq!(active_group_guid, group_guid);
        assert!(members.contains(&party_member));
        assert!(!members.contains(&unrelated));

        let stale_viewer = ObjectGuid::create_player(1, 7104);
        assert!(
            authoritative_current_group_members_like_cpp(&registry, group_guid, stale_viewer)
                .is_none()
        );
    }

    #[test]
    fn current_instance_group_is_eligible_for_party_visibility() {
        let registry = GroupRegistry::new();
        let viewer = ObjectGuid::create_player(1, 7201);
        let group = GroupInfo::new(viewer);
        let group_guid = group.group_guid;
        registry.register_group_like_cpp(group_guid, group);
        let mut snapshot = registry.get(&group_guid).unwrap();
        snapshot.group_category = wow_social::group::GROUP_CATEGORY_INSTANCE_LIKE_CPP;
        registry.register_group_like_cpp(group_guid, snapshot);

        let (active_group_guid, members) =
            authoritative_current_group_members_like_cpp(&registry, group_guid, viewer)
                .expect("current instance group membership remains visible");
        assert_eq!(active_group_guid, group_guid);
        assert_eq!(members, vec![viewer]);
    }
}
