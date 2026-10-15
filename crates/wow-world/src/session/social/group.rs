//! Represented group and party membership at the Session boundary.
//!
//! Moved out of the Session root under #615. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn current_player_is_in_group_guid_like_cpp(
        &self,
        current_group_guid: Option<u64>,
        group_owner: ObjectGuid,
    ) -> bool {
        let (state, hub) = crate::session::split_social_ref(self);
        state.current_player_is_in_group_guid_like_cpp(hub, current_group_guid, group_owner)
    }
    pub(in crate::session) fn current_player_is_in_raid_group_like_cpp(&self) -> bool {
        let (Some(group_guid), Some(group_registry), Some(player_guid)) = (
            self.resolved_group_guid_like_cpp(),
            self.core.directory.group_registry.as_ref(),
            self.player_guid(),
        ) else {
            return false;
        };

        group_registry
            .get(&group_guid)
            .is_some_and(|group| group.is_raid_group() && group.members.contains(&player_guid))
    }
    pub(crate) fn represented_player_is_same_raid_with_like_cpp(
        &self,
        player_guid: ObjectGuid,
        owner_guid: ObjectGuid,
    ) -> bool {
        let (Some(group_guid), Some(group_registry)) = (
            self.resolved_group_guid_like_cpp(),
            self.core.directory.group_registry.as_ref(),
        ) else {
            return false;
        };
        group_registry.get(&group_guid).is_some_and(|group| {
            group.members.contains(&player_guid) && group.members.contains(&owner_guid)
        })
    }
    pub(in crate::session) fn current_group_member_guids_for_tap_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        // #743: tap rights follow C++ `Player::GetGroup()`, which the group
        // owner clears on removal. Resolve them through the authority so a
        // removal notification still in flight cannot keep granting them.
        let (Some(group_guid), Some(group_registry)) = (
            self.authoritative_group_membership_like_cpp(),
            self.core.directory.group_registry.as_ref(),
        ) else {
            return Vec::new();
        };
        group_registry
            .get(&group_guid)
            .map(|group| {
                group
                    .members
                    .iter()
                    .copied()
                    .filter(|member| *member != player_guid)
                    .collect()
            })
            .unwrap_or_default()
    }
    /// Resolve C++ `Player::m_group` through this session incarnation's
    /// generation-checked canonical Player handle. An unresolved owner never
    /// falls back in production.
    pub(crate) fn resolved_group_guid_like_cpp(&self) -> Option<u64> {
        let (state, hub) = crate::session::split_social_ref(self);
        wow_world_instances::resolved_group_guid_like_cpp(hub, state, cfg!(test))
    }
    pub(crate) fn set_owned_player_group_like_cpp(
        &mut self,
        membership: Option<(u64, u8)>,
    ) -> bool {
        let (state, mut hub) = crate::session::split_social_mut(self);
        state.set_owned_player_group_like_cpp(&mut hub, membership)
    }
    pub(crate) fn sync_player_registry_party_member_party_type_like_cpp(&self) {
        let (state, hub) = crate::session::split_social_ref(self);
        state.sync_player_registry_party_member_party_type_like_cpp(hub)
    }
    /// Whether this session currently belongs to `group_guid` by the authority.
    ///
    /// C++ readers dereference `Player::m_group`, which the group owner
    /// clears in the same operation. A membership-sensitive Rust reader must
    /// not grant rights from the owned snapshot alone while a notification can
    /// still be in flight.
    pub(crate) fn authoritative_group_membership_like_cpp(&self) -> Option<u64> {
        let (Some(group_guid), Some(group_registry), Some(player_guid)) = (
            self.resolved_group_guid_like_cpp(),
            self.core.directory.group_registry.as_ref(),
            self.player_guid(),
        ) else {
            return None;
        };
        group_registry
            .get(&group_guid)
            .filter(|group| group.members.contains(&player_guid))
            .map(|group| group.group_guid)
    }

    /// C++ `Player::ResetGroupUpdateSequenceIfNeeded` resets the per-player
    /// sequence for a group category only when the loaded group guid changed.
    pub(crate) fn reset_group_update_sequence_if_needed_like_cpp(&mut self) -> bool {
        let (Some(group_guid), Some(group_registry)) = (
            self.resolved_group_guid_like_cpp(),
            self.core.directory.group_registry.as_ref(),
        ) else {
            return false;
        };

        let Some(group) = group_registry.get(&group_guid) else {
            return false;
        };
        let category = group.group_category_like_cpp();
        if category >= wow_social::group::MAX_GROUP_CATEGORY_LIKE_CPP {
            return false;
        }

        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            player.reset_group_update_sequence_if_needed_like_cpp(usize::from(category), group_guid)
        });
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            let sequence =
                &mut self.social.represented_group_update_sequences_like_cpp[usize::from(category)];
            if sequence.group_guid == Some(group_guid) {
                return false;
            }
            sequence.group_guid = Some(group_guid);
            sequence.update_sequence_number = 1;
            return true;
        }
        canonical.unwrap_or(false)
    }
    pub fn set_phase_group_store(&mut self, store: Arc<PhaseGroupStore>) {
        self.catalogs.phase_group_store = Some(store);
    }
    /// Set the shared group registry and pending invites.
    pub fn set_group_registry(&mut self, reg: Arc<GroupRegistry>, invites: Arc<PendingInvites>) {
        self.core.directory.group_registry = Some(reg);
        self.core.directory.pending_invites = Some(invites);
    }
    pub(crate) fn group_registry(&self) -> Option<&Arc<GroupRegistry>> {
        self.core.group_registry()
    }
    pub(crate) fn represented_player_at_group_reward_distance_like_cpp(
        &self,
        player_guid: ObjectGuid,
        reward_map_id: u16,
        reward_position: Position,
    ) -> bool {
        let (state, hub) = crate::session::split_social_ref(self);
        state.represented_player_at_group_reward_distance_like_cpp(
            hub,
            player_guid,
            reward_map_id,
            reward_position,
        )
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/social/group/f3_shims.rs"]
mod f3_shims;
