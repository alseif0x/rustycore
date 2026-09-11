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
        let Some(group_guid) = current_group_guid else {
            return false;
        };
        if ObjectGuid::create_group(group_guid) != group_owner {
            return false;
        }
        let Some(group_registry) = self.group_registry.as_ref() else {
            return false;
        };
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        group_registry
            .get(&group_guid)
            .is_some_and(|group| group.members.contains(&player_guid))
    }
    pub(in crate::session) fn current_player_is_group_visible_for_owner_like_cpp(
        &self,
        current_group_guid: Option<u64>,
        owner_guid: ObjectGuid,
    ) -> bool {
        let (Some(group_guid), Some(group_registry), Some(player_guid)) = (
            current_group_guid,
            self.group_registry.as_ref(),
            self.player_guid(),
        ) else {
            return false;
        };
        group_registry.get(&group_guid).is_some_and(|group| {
            group.members.contains(&player_guid) && group.members.contains(&owner_guid)
        })
    }
    pub(in crate::session) fn current_player_is_in_raid_group_like_cpp(&self) -> bool {
        let (Some(group_guid), Some(group_registry), Some(player_guid)) = (
            self.resolved_group_guid_like_cpp(),
            self.group_registry.as_ref(),
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
            self.group_registry.as_ref(),
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
            self.group_registry.as_ref(),
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
    pub(crate) fn group_invite_policy_for_test_like_cpp(&self) -> GroupInvitePolicyLikeCpp {
        GroupInvitePolicyLikeCpp {
            allow_gm_group: self.allow_gm_group_like_cpp,
            allow_two_side_interaction: self.allow_two_side_interaction_group_like_cpp,
            minimum_level: self.party_level_req_like_cpp,
        }
    }
    #[cfg(test)]
    pub fn set_party_raid_warnings_like_cpp(&mut self, enabled: bool) {
        self.party_raid_warnings_like_cpp = enabled;
    }
    #[cfg(test)]
    pub fn set_allow_gm_group_like_cpp(&mut self, enabled: bool) {
        self.allow_gm_group_like_cpp = enabled;
    }
    #[cfg(test)]
    pub fn set_allow_two_side_interaction_group_like_cpp(&mut self, enabled: bool) {
        self.allow_two_side_interaction_group_like_cpp = enabled;
    }
    #[cfg(test)]
    pub fn set_party_level_req_like_cpp(&mut self, level: u32) {
        self.party_level_req_like_cpp = level;
    }
    #[cfg(test)]
    pub(crate) fn party_raid_warnings_like_cpp(&self) -> bool {
        self.party_raid_warnings_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn allow_gm_group_like_cpp(&self) -> bool {
        self.allow_gm_group_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn allow_two_side_interaction_group_like_cpp(&self) -> bool {
        self.allow_two_side_interaction_group_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn party_level_req_like_cpp(&self) -> u32 {
        self.party_level_req_like_cpp
    }
    pub(crate) fn canonical_player_party_power_snapshot_like_cpp(&self) -> Option<(u8, u16, u16)> {
        self.canonical_player_snapshot_like_cpp(|player| {
            let power_type = player.unit().data().display_power;
            let power = party_member_power_kind_from_u8_like_cpp(power_type);
            (
                power_type,
                party_member_power_to_u16_like_cpp(player.get_power(power)),
                party_member_power_to_u16_like_cpp(player.get_max_power(power)),
            )
        })
    }
    pub(crate) fn send_player_party_type_update_like_cpp(&self, category: u8, party_type: u8) {
        let Some(guid) = self.player_guid() else {
            return;
        };
        if category >= wow_social::group::MAX_GROUP_CATEGORY_LIKE_CPP {
            return;
        }

        let mut data = wow_packet::packets::update::PlayerDataValuesDeltaUpdate::default();
        let category_index = usize::from(category);
        data.player_data_mask[wow_entities::PLAYER_DATA_PARTY_TYPE_PARENT_BIT / 32] |=
            1 << (wow_entities::PLAYER_DATA_PARTY_TYPE_PARENT_BIT % 32);
        data.player_data_mask
            [(wow_entities::PLAYER_DATA_PARTY_TYPE_FIRST_BIT + category_index) / 32] |=
            1 << ((wow_entities::PLAYER_DATA_PARTY_TYPE_FIRST_BIT + category_index) % 32);
        data.party_type[category_index] = party_type;
        self.send_packet(
            &wow_packet::packets::update::UpdateObject::full_player_values_update(
                guid,
                self.player_map_id_like_cpp(),
                data,
            ),
        );
    }
    /// Resolve C++ `Player::m_group` through this session incarnation's
    /// generation-checked canonical Player handle. An unresolved owner never
    /// falls back in production.
    pub(crate) fn resolved_group_guid_like_cpp(&self) -> Option<u64> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .group
                .as_ref()
                .map(|group| group.group_guid.counter() as u64)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self.group_guid;
        }
        canonical.flatten()
    }
    fn resolved_group_subgroup_like_cpp(&self) -> Option<u8> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .group
                .as_ref()
                .map(|group| group.subgroup)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self.represented_subgroup_like_cpp;
        }
        canonical.flatten()
    }
    /// C++ `Player::SetGroup`: replace the Player-owned `GroupReference`
    /// snapshot. Group membership itself remains authoritative in GroupRegistry.
    pub(crate) fn set_owned_player_group_like_cpp(
        &mut self,
        membership: Option<(u64, u8)>,
    ) -> bool {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.group_guid = membership.map(|(group_guid, _)| group_guid);
            self.represented_subgroup_like_cpp = membership.map(|(_, subgroup)| subgroup);
            return true;
        }

        let state = match membership {
            None => None,
            Some((group_guid, subgroup)) => {
                let Some(player_guid) = self.player_guid() else {
                    return false;
                };
                let Some(group_registry) = self.group_registry.as_ref() else {
                    return false;
                };
                let Some(group) = group_registry.get(&group_guid) else {
                    return false;
                };
                let Some(slot) = group.member_slot_like_cpp(player_guid) else {
                    return false;
                };
                Some(wow_entities::PlayerGroupState {
                    group_guid: ObjectGuid::create_group(group_guid),
                    leader_guid: group.leader_guid,
                    role_mask: slot.roles,
                    subgroup,
                })
            }
        };

        self.with_owned_player_mut_like_cpp(move |player| {
            player.set_group_like_cpp(state);
        })
        .is_some()
    }
    pub(crate) fn apply_group_subgroup_like_cpp(&mut self, group_guid: u64, subgroup: u8) {
        if self.resolved_group_guid_like_cpp() == Some(group_guid)
            && self.set_owned_player_group_like_cpp(Some((group_guid, subgroup)))
        {
            self.sync_player_registry_state_like_cpp();
        }
    }
    pub(crate) fn apply_group_join_like_cpp(&mut self, group_guid: u64, subgroup: u8) {
        if self.set_owned_player_group_like_cpp(Some((group_guid, subgroup))) {
            self.sync_player_registry_party_member_party_type_like_cpp();
        }
    }
    pub(crate) fn sync_player_registry_party_member_party_type_like_cpp(&self) {
        let (Some(guid), Some(registry)) = (self.player_guid(), &self.player_registry) else {
            return;
        };
        let party_type = self.party_member_party_type_like_cpp();
        registry.publish_party_type_for_control_channel(guid, &self.session_command_tx, party_type);
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
            self.player_registry.as_ref().map(Arc::clone),
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
        let Some(group_registry) = self.group_registry.as_ref().map(Arc::clone) else {
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

    /// Record that this session must reconcile its owned group snapshot.
    ///
    /// Used by the group command handlers when an admission phase or an
    /// ordering guard prevents applying a delivered state change.
    pub(crate) fn defer_group_state_reconciliation_like_cpp(&self) {
        let (Some(player_guid), Some(player_registry)) =
            (self.player_guid(), self.player_registry.as_ref())
        else {
            return;
        };
        player_registry.mark_group_state_reconciliation_like_cpp(player_guid);
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
        let current_subgroup = self.resolved_group_subgroup_like_cpp();

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
            self.group_registry.as_ref(),
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
        let _ = self.set_owned_player_group_like_cpp(None);
    }
    #[cfg(test)]
    pub(crate) fn represented_subgroup_like_cpp(&self) -> Option<u8> {
        self.resolved_group_subgroup_like_cpp()
    }
    /// C++ `Player::ResetGroupUpdateSequenceIfNeeded` resets the per-player
    /// sequence for a group category only when the loaded group guid changed.
    pub(crate) fn reset_group_update_sequence_if_needed_like_cpp(&mut self) -> bool {
        let (Some(group_guid), Some(group_registry)) = (
            self.resolved_group_guid_like_cpp(),
            self.group_registry.as_ref(),
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

        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.reset_group_update_sequence_if_needed_like_cpp(usize::from(category), group_guid)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            let sequence =
                &mut self.represented_group_update_sequences_like_cpp[usize::from(category)];
            if sequence.group_guid == Some(group_guid) {
                return false;
            }
            sequence.group_guid = Some(group_guid);
            sequence.update_sequence_number = 1;
            return true;
        }
        canonical.unwrap_or(false)
    }
    /// C++ `Player::NextGroupUpdateSequenceNumber` returns the current
    /// per-player category sequence and then increments it.
    pub(crate) fn next_group_update_sequence_number_like_cpp(
        &mut self,
        category: u8,
    ) -> Option<i32> {
        if category >= wow_social::group::MAX_GROUP_CATEGORY_LIKE_CPP {
            return None;
        }

        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.next_group_update_sequence_number_like_cpp(usize::from(category))
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            let sequence =
                &mut self.represented_group_update_sequences_like_cpp[usize::from(category)];
            let current = sequence.update_sequence_number;
            sequence.update_sequence_number = sequence.update_sequence_number.saturating_add(1);
            return Some(current);
        }
        canonical
    }
    /// C++ `Group::SendUpdateDestroyGroupToPlayer` (`Group.cpp:917-926`): the
    /// removed member tears down its party frames from a destroyed
    /// `PartyUpdate` that carries no members. `Group::RemoveMember` sends it
    /// after the kick when the group survives (`Group.cpp:654-655`) and
    /// `Group::Disband` sends it to every member (`Group.cpp:746`).
    pub(crate) fn send_destroyed_group_party_update_like_cpp(
        &mut self,
        group_guid: u64,
        category: u8,
    ) {
        let Some(sequence_num) = self.next_group_update_sequence_number_like_cpp(category) else {
            return;
        };
        self.send_packet_realm(&wow_packet::packets::party::PartyUpdate {
            party_flags: wow_social::group::GROUP_FLAG_DESTROYED_LIKE_CPP,
            party_index: category,
            party_type: wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
            my_index: -1,
            party_guid: group_guid,
            sequence_num,
            leader_guid: ObjectGuid::EMPTY,
            leader_faction_group: 0,
            player_list: Vec::new(),
            loot_settings: None,
            difficulty_settings: None,
        });
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
                self.group_registry
                    .as_ref()
                    .and_then(|registry| registry.get(&group_guid).map(|group| group.leader_guid))
            })
            .is_some_and(|leader_guid| leader_guid == player_guid);

        let updated = self
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
        self.phase_group_store = Some(store);
    }
    pub(in crate::session) fn represented_player_group_reward_state_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Option<(u8, u16, Position, bool)> {
        if self.player_guid() == Some(player_guid) {
            return Some((
                self.player_level_like_cpp(),
                self.player_map_id_like_cpp(),
                self.player_position_like_cpp()?,
                self.resolved_player_is_alive_like_cpp()?,
            ));
        }
        self.player_registry.as_ref().and_then(|registry| {
            registry
                .group_reward_snapshot(player_guid)
                .map(|entry| (entry.level, entry.map_id, entry.position, entry.is_alive))
        })
    }
    /// Set the shared group registry and pending invites.
    pub fn set_group_registry(&mut self, reg: Arc<GroupRegistry>, invites: Arc<PendingInvites>) {
        self.group_registry = Some(reg);
        self.pending_invites = Some(invites);
    }
    /// Get a reference to the shared group registry.
    pub fn group_registry(&self) -> Option<&Arc<GroupRegistry>> {
        self.group_registry.as_ref()
    }
    pub(crate) fn party_member_party_type_like_cpp(&self) -> [u8; 2] {
        let mut party_type = [wow_social::group::GROUP_TYPE_NONE_LIKE_CPP; 2];
        let (Some(group_registry), Some(player_guid)) = (&self.group_registry, self.player_guid())
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
    pub(in crate::session) fn represented_player_at_group_reward_distance_like_cpp(
        &self,
        player_guid: ObjectGuid,
        reward_map_id: u16,
        reward_position: Position,
    ) -> bool {
        let player_state = if self.player_guid() == Some(player_guid) {
            self.player_position_like_cpp()
                .map(|position| (self.player_map_id_like_cpp(), position))
        } else {
            self.player_registry.as_ref().and_then(|registry| {
                registry
                    .loot_presence(player_guid)
                    .map(|entry| (entry.map_id, entry.position))
            })
        };
        let Some((player_map_id, player_position)) = player_state else {
            return self.player_guid() == Some(player_guid);
        };
        if player_map_id != reward_map_id {
            return false;
        }
        if self
            .maps
            .store
            .as_ref()
            .and_then(|store| store.get(u32::from(player_map_id)))
            .is_some_and(|entry| entry.is_dungeon())
        {
            return true;
        }
        player_position.distance(&reward_position) <= GROUP_XP_DISTANCE_LIKE_CPP
    }
    pub(crate) fn record_represented_silence_party_talker_like_cpp(
        &mut self,
        target: ObjectGuid,
        silent: bool,
    ) {
        #[cfg(test)]
        self.represented_silence_party_talker_like_cpp
            .push(RepresentedSilencePartyTalkerLikeCpp { target, silent });
        #[cfg(not(test))]
        let _ = (target, silent);
    }
    #[cfg(test)]
    pub(crate) fn represented_silence_party_talker_like_cpp(
        &self,
    ) -> &[RepresentedSilencePartyTalkerLikeCpp] {
        &self.represented_silence_party_talker_like_cpp
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
