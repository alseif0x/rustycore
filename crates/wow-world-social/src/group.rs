//! Represented group operations owned by the Social boundary.

use crate::{GROUP_XP_DISTANCE_LIKE_CPP, SessionSocialLimits};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedSilencePartyTalkerLikeCpp;
use wow_core::{ObjectGuid, Position};
use wow_world_core::session::{HubMut, HubRef};

impl SessionSocialLimits {
    pub fn current_player_is_in_group_guid_like_cpp(
        &self,
        hub: HubRef<'_>,
        current_group_guid: Option<u64>,
        group_owner: ObjectGuid,
    ) -> bool {
        let Some(group_guid) = current_group_guid else {
            return false;
        };
        if ObjectGuid::create_group(group_guid) != group_owner {
            return false;
        }
        let Some(group_registry) = hub.core.directory.group_registry.as_ref() else {
            return false;
        };
        let Some(player_guid) = hub.core.player_guid() else {
            return false;
        };
        group_registry
            .get(&group_guid)
            .is_some_and(|group| group.members.contains(&player_guid))
    }

    pub fn current_player_is_group_visible_for_owner_like_cpp(
        &self,
        hub: HubRef<'_>,
        current_group_guid: Option<u64>,
        owner_guid: ObjectGuid,
    ) -> bool {
        let (Some(group_guid), Some(group_registry), Some(player_guid)) = (
            current_group_guid,
            hub.core.directory.group_registry.as_ref(),
            hub.core.player_guid(),
        ) else {
            return false;
        };
        group_registry.get(&group_guid).is_some_and(|group| {
            group.members.contains(&player_guid) && group.members.contains(&owner_guid)
        })
    }

    pub fn send_player_party_type_update_like_cpp(
        &self,
        hub: HubRef<'_>,
        category: u8,
        party_type: u8,
    ) {
        let Some(guid) = hub.core.player_guid() else {
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
        hub.core.send_packet(
            &wow_packet::packets::update::UpdateObject::full_player_values_update(
                guid,
                hub.core.player_map_id_like_cpp(),
                data,
            ),
        );
    }

    pub fn resolved_group_subgroup_like_cpp(&self, hub: HubRef<'_>) -> Option<u8> {
        let canonical = hub.core.with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .group
                .as_ref()
                .map(|group| group.subgroup)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return self.represented_subgroup_like_cpp;
        }
        canonical.flatten()
    }

    /// C++ `Player::SetGroup`: replace the Player-owned `GroupReference`
    /// snapshot. Group membership itself remains authoritative in GroupRegistry.
    pub fn set_owned_player_group_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        membership: Option<(u64, u8)>,
    ) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            self.group_guid = membership.map(|(group_guid, _)| group_guid);
            self.represented_subgroup_like_cpp = membership.map(|(_, subgroup)| subgroup);
            return true;
        }

        let state = match membership {
            None => None,
            Some((group_guid, subgroup)) => {
                let Some(player_guid) = hub.core.player_guid() else {
                    return false;
                };
                let Some(group_registry) = hub.core.directory.group_registry.as_ref() else {
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

        hub.core
            .with_owned_player_mut_like_cpp(move |player| {
                player.set_group_like_cpp(state);
            })
            .is_some()
    }

    pub fn apply_group_join_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        group_guid: u64,
        subgroup: u8,
    ) {
        if self.set_owned_player_group_like_cpp(hub, Some((group_guid, subgroup))) {
            self.sync_player_registry_party_member_party_type_like_cpp(hub.shared());
        }
    }

    pub fn sync_player_registry_party_member_party_type_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) {
        hub.core
            .sync_player_registry_party_member_party_type_like_cpp();
    }

    /// Record that this session must reconcile its owned group snapshot.
    ///
    /// Used by the group command handlers when an admission phase or an
    /// ordering guard prevents applying a delivered state change.
    pub fn defer_group_state_reconciliation_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) {
        let (Some(player_guid), Some(player_registry)) =
            (hub.core.player_guid(), hub.core.player_registry.as_ref())
        else {
            return;
        };
        player_registry.mark_group_state_reconciliation_like_cpp(player_guid);
    }

    pub fn clear_represented_group_subgroup_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) {
        let _ = self.set_owned_player_group_like_cpp(hub, None);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_subgroup_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<u8> {
        self.resolved_group_subgroup_like_cpp(hub)
    }

    /// C++ `Player::NextGroupUpdateSequenceNumber` returns the current
    /// per-player category sequence and then increments it.
    pub fn next_group_update_sequence_number_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        category: u8,
    ) -> Option<i32> {
        if category >= wow_social::group::MAX_GROUP_CATEGORY_LIKE_CPP {
            return None;
        }

        let canonical = hub.core.with_owned_player_mut_like_cpp(|player| {
            player.next_group_update_sequence_number_like_cpp(usize::from(category))
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
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
    pub fn send_destroyed_group_party_update_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        group_guid: u64,
        category: u8,
    ) {
        let Some(sequence_num) = self.next_group_update_sequence_number_like_cpp(hub, category)
        else {
            return;
        };
        hub.core
            .send_packet_realm(&wow_packet::packets::party::PartyUpdate {
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

    pub fn represented_player_group_reward_state_like_cpp(
        &self,
        hub: HubRef<'_>,
        player_guid: ObjectGuid,
    ) -> Option<(u8, u16, Position, bool)> {
        if hub.core.player_guid() == Some(player_guid) {
            return Some((
                hub.player_level_like_cpp(),
                hub.core.player_map_id_like_cpp(),
                hub.player_position_like_cpp()?,
                hub.resolved_player_is_alive_like_cpp()?,
            ));
        }
        hub.core.player_registry.as_ref().and_then(|registry| {
            registry
                .group_reward_snapshot(player_guid)
                .map(|entry| (entry.level, entry.map_id, entry.position, entry.is_alive))
        })
    }

    pub fn represented_player_at_group_reward_distance_like_cpp(
        &self,
        hub: HubRef<'_>,
        player_guid: ObjectGuid,
        reward_map_id: u16,
        reward_position: Position,
    ) -> bool {
        let player_state = if hub.core.player_guid() == Some(player_guid) {
            hub.player_position_like_cpp()
                .map(|position| (hub.core.player_map_id_like_cpp(), position))
        } else {
            hub.core.player_registry.as_ref().and_then(|registry| {
                registry
                    .loot_presence(player_guid)
                    .map(|entry| (entry.map_id, entry.position))
            })
        };
        let Some((player_map_id, player_position)) = player_state else {
            return hub.core.player_guid() == Some(player_guid);
        };
        if player_map_id != reward_map_id {
            return false;
        }
        if hub
            .catalogs
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

    pub fn record_represented_silence_party_talker_like_cpp(
        &mut self,
        target: ObjectGuid,
        silent: bool,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.represented_silence_party_talker_like_cpp
            .push(RepresentedSilencePartyTalkerLikeCpp { target, silent });
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = (target, silent);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_silence_party_talker_like_cpp(
        &self,
    ) -> &[RepresentedSilencePartyTalkerLikeCpp] {
        &self.represented_silence_party_talker_like_cpp
    }
}
