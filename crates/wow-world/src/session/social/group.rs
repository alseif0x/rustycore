//! Represented group and party membership at the Session boundary.
//!
//! Moved out of the Session root under #615. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn current_player_is_in_group_guid_like_cpp(
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
    pub(in crate::session) fn represented_player_is_same_raid_with_like_cpp(
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
    #[cfg(test)]
    pub fn set_allow_two_side_interaction_group_like_cpp(&mut self, enabled: bool) {
        self.config.allow_two_side_interaction_group_like_cpp = enabled;
    }
    #[cfg(test)]
    pub(crate) fn party_raid_warnings_like_cpp(&self) -> bool {
        self.config.party_raid_warnings_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn allow_gm_group_like_cpp(&self) -> bool {
        self.config.allow_gm_group_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn allow_two_side_interaction_group_like_cpp(&self) -> bool {
        self.config.allow_two_side_interaction_group_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn party_level_req_like_cpp(&self) -> u32 {
        self.config.party_level_req_like_cpp
    }
    pub(crate) fn send_player_party_type_update_like_cpp(&self, category: u8, party_type: u8) {
        let (state, hub) = crate::session::split_social_ref(self);
        state.send_player_party_type_update_like_cpp(hub, category, party_type)
    }
    /// Resolve C++ `Player::m_group` through this session incarnation's
    /// generation-checked canonical Player handle. An unresolved owner never
    /// falls back in production.
    pub(crate) fn resolved_group_guid_like_cpp(&self) -> Option<u64> {
        let owner = self.core.player_group_owner_access_like_cpp();
        self.social
            .resolved_group_guid_with_access_like_cpp(&owner, cfg!(test))
    }
    pub(crate) fn set_owned_player_group_like_cpp(
        &mut self,
        membership: Option<(u64, u8)>,
    ) -> bool {
        let (state, mut hub) = crate::session::split_social_mut(self);
        state.set_owned_player_group_like_cpp(&mut hub, membership)
    }
    pub(crate) fn apply_group_subgroup_like_cpp(&mut self, group_guid: u64, subgroup: u8) {
        if self.resolved_group_guid_like_cpp() == Some(group_guid)
            && self.set_owned_player_group_like_cpp(Some((group_guid, subgroup)))
        {
            self.sync_player_registry_state_like_cpp();
        }
    }
    pub(crate) fn sync_player_registry_party_member_party_type_like_cpp(&self) {
        let (state, hub) = crate::session::split_social_ref(self);
        state.sync_player_registry_party_member_party_type_like_cpp(hub)
    }
    /// #743: converge the owned group snapshot on `GroupRegistry`.
    ///
    /// C++ never needs this: `Group::RemoveMember` (`Group.cpp:550`),
    /// `Group::Disband` (`Group.cpp:713`) and `Group::ChangeMembersGroup`
    /// reach every connected member's `Player` directly, so `Player::SetGroup`
    /// (`Player.cpp:23440`) runs inside the same operation. RustyCore applies
    /// the same transitions on the member's own session to keep its admission
    /// phase, canonical Player guard and publication order; this reconciliation
    /// is the fence that makes the queue hop lossless.
    ///
    /// It runs only for a session the group authority marked, so an untouched
    /// session never pays for a canonical read. A command that does arrive
    /// after the reconciliation finds the snapshot already converged and its
    /// own group guard rejects it, so nothing is published twice.
    pub(crate) fn reconcile_group_state_like_cpp(&mut self) -> bool {
        let (Some(player_guid), Some(player_registry)) = (
            self.player_guid(),
            self.core.player_registry.as_ref().map(Arc::clone),
        ) else {
            return false;
        };
        if !player_registry.take_group_state_reconciliation_like_cpp(player_guid) {
            return false;
        }
        if self.state() != crate::session::SessionState::LoggedIn {
            // The represented Player is not admitted yet. C++ resolves group
            // membership from the authority at `Player::_LoadGroup`, so the
            // mark is kept rather than discarded at an ineligible phase.
            player_registry.mark_group_state_reconciliation_like_cpp(player_guid);
            return false;
        }
        let Some(group_registry) = self.core.directory.group_registry.as_ref().map(Arc::clone)
        else {
            return false;
        };
        let applied = self.apply_authoritative_group_state_like_cpp(player_guid, &group_registry);
        if applied == GroupReconciliationOutcomeLikeCpp::Unreachable {
            // The canonical Player could not be resolved this pass (transfer,
            // detached residence). Keep the obligation instead of losing it.
            player_registry.mark_group_state_reconciliation_like_cpp(player_guid);
            return false;
        }
        applied == GroupReconciliationOutcomeLikeCpp::Applied
    }

    pub(crate) fn defer_group_state_reconciliation_like_cpp(&self) {
        let (state, hub) = crate::session::split_social_ref(self);
        state.defer_group_state_reconciliation_like_cpp(hub)
    }

    /// Apply the authority's membership to the owned snapshot and publish the
    /// difference, in the order the original operation would have published it.
    fn apply_authoritative_group_state_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        group_registry: &GroupRegistry,
    ) -> GroupReconciliationOutcomeLikeCpp {
        let authority = group_registry.member_group_state_like_cpp(player_guid);
        let current_group_guid = self.resolved_group_guid_like_cpp();
        let current_subgroup = {
            let (s, h) = crate::session::split_social_ref(self);
            s.resolved_group_subgroup_like_cpp(h)
        };

        let Some(authority) = authority else {
            let Some(previous_group_guid) = current_group_guid else {
                return GroupReconciliationOutcomeLikeCpp::AlreadyConverged;
            };
            // `Group::Disband` retires the group itself; `Group::RemoveMember`
            // keeps it. That difference selects which teardown packet the
            // member missed.
            let surviving_category = group_registry.group_category_like_cpp(previous_group_guid);
            let category =
                surviving_category.unwrap_or(wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP);
            if !self.set_owned_player_group_like_cpp(None) {
                return GroupReconciliationOutcomeLikeCpp::Unreachable;
            }
            self.send_player_party_type_update_like_cpp(
                category,
                wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
            );
            self.sync_player_registry_state_like_cpp();
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
            if surviving_category.is_some() {
                self.send_packet_realm(&wow_packet::packets::party::GroupUninvite);
            } else {
                self.send_packet_realm(&wow_packet::packets::party::GroupDestroyed);
            }
            self.send_destroyed_group_party_update_like_cpp(previous_group_guid, category);
            return GroupReconciliationOutcomeLikeCpp::Applied;
        };

        let membership_converged = current_group_guid == Some(authority.group_guid)
            && current_subgroup == Some(authority.subgroup);
        let mut applied = false;
        if !membership_converged {
            let joined_new_group = current_group_guid != Some(authority.group_guid);
            if !self
                .set_owned_player_group_like_cpp(Some((authority.group_guid, authority.subgroup)))
            {
                return GroupReconciliationOutcomeLikeCpp::Unreachable;
            }
            if joined_new_group {
                self.send_player_party_type_update_like_cpp(
                    authority.group_category,
                    wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP,
                );
                self.sync_player_registry_party_member_party_type_like_cpp();
                let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
            } else {
                self.sync_player_registry_state_like_cpp();
            }
            applied = true;
        }
        // The group also owns the member's three difficulty preferences; a lost
        // `ApplyGroupDifficultyLikeCpp` diverges them without changing membership.
        if let Some(group) = group_registry.get(&authority.group_guid)
            && self.reconcile_group_difficulty_like_cpp(
                group.dungeon_difficulty_id,
                group.raid_difficulty_id,
                group.legacy_raid_difficulty_id,
            )
        {
            applied = true;
        }
        if applied {
            GroupReconciliationOutcomeLikeCpp::Applied
        } else {
            GroupReconciliationOutcomeLikeCpp::AlreadyConverged
        }
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

    pub(crate) fn clear_represented_group_subgroup_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_social_mut(self);
        state.clear_represented_group_subgroup_like_cpp(&mut hub)
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
    pub(crate) fn next_group_update_sequence_number_like_cpp(
        &mut self,
        category: u8,
    ) -> Option<i32> {
        let (state, mut hub) = crate::session::split_social_mut(self);
        state.next_group_update_sequence_number_like_cpp(&mut hub, category)
    }
    pub(crate) fn send_destroyed_group_party_update_like_cpp(
        &mut self,
        group_guid: u64,
        category: u8,
    ) {
        let (state, mut hub) = crate::session::split_social_mut(self);
        state.send_destroyed_group_party_update_like_cpp(&mut hub, group_guid, category)
    }
    /// C++ `Player::_LoadGroup` sets `PLAYER_FLAGS_GROUP_LEADER` when the
    /// loaded group leader matches the player, and removes it otherwise.
    pub(crate) fn apply_represented_group_leader_flag_like_cpp(&mut self) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };

        let is_group_leader = self
            .resolved_group_guid_like_cpp()
            .and_then(|group_guid| {
                self.core
                    .directory
                    .group_registry
                    .as_ref()
                    .and_then(|registry| registry.get(&group_guid).map(|group| group.leader_guid))
            })
            .is_some_and(|leader_guid| leader_guid == player_guid);

        let updated = self
            .core
            .mutate_canonical_player_like_cpp(|player| {
                if is_group_leader {
                    player.set_player_flag(PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP);
                } else {
                    player.remove_player_flag(PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP);
                }
            })
            .is_some();

        if updated {
            self.sync_player_registry_state_like_cpp();
        }

        updated
    }
    pub fn set_phase_group_store(&mut self, store: Arc<PhaseGroupStore>) {
        self.catalogs.phase_group_store = Some(store);
    }
    /// Set the shared group registry and pending invites.
    pub fn set_group_registry(&mut self, reg: Arc<GroupRegistry>, invites: Arc<PendingInvites>) {
        self.core.directory.group_registry = Some(reg);
        self.core.directory.pending_invites = Some(invites);
    }
    pub fn group_registry(&self) -> Option<&Arc<GroupRegistry>> {
        self.core.group_registry()
    }
    pub(in crate::session) fn represented_player_at_group_reward_distance_like_cpp(
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

/// Result of one reconciliation attempt against the group authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupReconciliationOutcomeLikeCpp {
    /// The owned snapshot already matched the authority.
    AlreadyConverged,
    /// The snapshot was corrected and the missed difference published.
    Applied,
    /// The canonical Player could not be resolved; the mark is retained.
    Unreachable,
}

#[cfg(test)]
#[path = "../../../unit_tests/session/social/group/f3_shims.rs"]
mod f3_shims;
