//! Session consumption of map-owned `Map::SendObjectUpdates` snapshots.

use std::hash::{Hash, Hasher};

use wow_core::ObjectGuid;
use wow_entities::{
    PlayerValuesUpdate, TYPEID_ACTIVE_PLAYER, TYPEID_OBJECT, TYPEID_PLAYER, TYPEID_UNIT,
    UnitValuesUpdate, UpdateFieldSectionKind, UpdateFieldVisibilityFlags, filter_disallowed_fields,
};
use wow_packet::ServerPacket;
use wow_world_core::entity_update_bridge::{
    player_values_update_to_update_object, unit_values_update_to_update_object,
};
use wow_world_core::session::HubMut;

use crate::VisibilityState;

pub fn represented_dynamic_object_values_update_delivery_fingerprint_like_cpp(
    guid: ObjectGuid,
    bytes: &[u8],
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    guid.hash(&mut hasher);
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn filter_unit_values_update_for_target_like_cpp(update: &mut UnitValuesUpdate, owner: bool) {
    let flags = owner
        .then_some(UpdateFieldVisibilityFlags::OWNER)
        .unwrap_or_else(UpdateFieldVisibilityFlags::empty);
    if let Some(unit_data) = update.unit_data.as_mut() {
        filter_disallowed_fields(UpdateFieldSectionKind::UnitData, &mut unit_data.mask, flags);
        if !unit_data.mask.is_any_set() {
            update.unit_data = None;
        }
    }
    update.changed_object_type_mask = update
        .object_data
        .as_ref()
        .map(|_| 1_u32 << TYPEID_OBJECT)
        .unwrap_or(0)
        | update
            .unit_data
            .as_ref()
            .map(|_| 1_u32 << TYPEID_UNIT)
            .unwrap_or(0);
}

fn filter_player_values_update_for_target_like_cpp(
    update: &mut PlayerValuesUpdate,
    owner: bool,
    party_member: bool,
) {
    let unit_flags = owner
        .then_some(UpdateFieldVisibilityFlags::OWNER)
        .unwrap_or_else(UpdateFieldVisibilityFlags::empty);
    let player_flags = unit_flags
        | party_member
            .then_some(UpdateFieldVisibilityFlags::PARTY_MEMBER)
            .unwrap_or_else(UpdateFieldVisibilityFlags::empty);
    if let Some(unit_data) = update.unit_data.as_mut() {
        filter_disallowed_fields(
            UpdateFieldSectionKind::UnitData,
            &mut unit_data.mask,
            unit_flags,
        );
        if !unit_data.mask.is_any_set() {
            update.unit_data = None;
        }
    }
    if let Some(player_data) = update.player_data.as_mut() {
        filter_disallowed_fields(
            UpdateFieldSectionKind::PlayerData,
            &mut player_data.mask,
            player_flags,
        );
        if !player_data.mask.is_any_set() {
            update.player_data = None;
        }
    }
    if !owner {
        update.active_player_data = None;
    }
    update.changed_object_type_mask = update
        .object_data
        .as_ref()
        .map(|_| 1_u32 << TYPEID_OBJECT)
        .unwrap_or(0)
        | update
            .unit_data
            .as_ref()
            .map(|_| 1_u32 << TYPEID_UNIT)
            .unwrap_or(0)
        | update
            .player_data
            .as_ref()
            .map(|_| 1_u32 << TYPEID_PLAYER)
            .unwrap_or(0)
        | update
            .active_player_data
            .as_ref()
            .map(|_| 1_u32 << TYPEID_ACTIVE_PLAYER)
            .unwrap_or(0);
}

fn source_is_in_current_group_like_cpp(
    active_group: Option<&(u64, Vec<ObjectGuid>)>,
    source_guid: ObjectGuid,
    source_group_guid: Option<u64>,
) -> bool {
    active_group.is_some_and(|(group_guid, members)| {
        source_group_guid == Some(*group_guid) && members.contains(&source_guid)
    })
}

impl VisibilityState {
    /// Deliver the Player and Unit VALUES snapshots captured by the canonical
    /// map's `Map::SendObjectUpdates` phase.
    ///
    /// C++ `WorldObject::BuildUpdate` (`Object.cpp:3722-3729`) builds one
    /// viewer-specific block for every nearby player that already has the
    /// source in `m_clientGUIDs`.  The map snapshot is immutable by the time
    /// this function runs, so the session performs the same final map,
    /// instance, incarnation, phase, distance and `HaveAtClient` checks before
    /// sending.  Player self updates retain ActivePlayerData; observer updates
    /// strip that owner-only section, matching `BuildValuesUpdate`'s target
    /// flags.  Creature/Pet values use the existing shared Unit update bridge;
    /// viewer-dependent loot projections remain owned by their dedicated
    /// producer paths.
    pub fn send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> usize {
        let Some(key) = hub.core.current_canonical_player_map_key_like_cpp() else {
            return 0;
        };
        let Ok(packet_map_id) = u16::try_from(key.map_id) else {
            return 0;
        };
        let Some(manager) = hub.core.canonical_map_manager.as_ref() else {
            return 0;
        };
        let Some(viewer_guid) = hub.core.player_guid() else {
            return 0;
        };
        let party_group = hub
            .core
            .player_group_owner_access_like_cpp()
            .party_group_for_values_update_like_cpp();

        let mut packets = Vec::new();
        let update_generation;
        {
            let Ok(manager) = manager.lock() else {
                return 0;
            };
            let Some(managed_map) = manager.find_map(key.map_id, key.instance_id) else {
                return 0;
            };
            let map = managed_map.map();
            let Some(viewer) = map.get_typed_player(viewer_guid) else {
                return 0;
            };
            if !viewer.unit().world().object().is_in_world() {
                return 0;
            }
            let viewer_world = viewer.unit().world();
            let viewer_phase = viewer_world.phase_shift().clone();
            let viewer_position = viewer_world.position();
            let visibility_range = map.visibility_range();
            update_generation = managed_map.update_calls().len() as u64;
            let summary = managed_map.last_send_object_updates_summary_like_cpp();

            for represented_update in summary.player_values_updates {
                let source_guid = represented_update.guid;
                let source_is_viewer = source_guid == viewer_guid;
                let (source_visible, source_group_guid) = if source_is_viewer {
                    (true, None)
                } else if hub
                    .core
                    .client_visible_guids_like_cpp
                    .contains(&source_guid)
                {
                    map.get_typed_player(source_guid)
                        .map_or((false, None), |source| {
                            let source_world = source.unit().world();
                            let visible = source_world.object().is_in_world()
                                && viewer_phase.can_see(source_world.phase_shift())
                                && source_world
                                    .position()
                                    .is_within_dist_2d(&viewer_position, visibility_range);
                            let group_guid = source
                                .gameplay_state()
                                .group
                                .as_ref()
                                .map(|group| group.group_guid.counter() as u64);
                            (visible, group_guid)
                        })
                } else {
                    (false, None)
                };
                if !source_visible {
                    continue;
                }

                let mut values_update = represented_update.values_update;
                // `BuildValuesUpdate` filters UnitData/PlayerData by the
                // receiver's flags and exposes ActivePlayerData only to the
                // owner. Current group members also receive PARTY_MEMBER
                // fields. The map snapshot is captured once with complete
                // masks, so apply the receiver-specific filter here.
                filter_player_values_update_for_target_like_cpp(
                    &mut values_update,
                    source_is_viewer,
                    source_is_viewer
                        || source_is_in_current_group_like_cpp(
                            party_group.as_ref(),
                            source_guid,
                            source_group_guid,
                        ),
                );
                let Some(update) = player_values_update_to_update_object(
                    source_guid,
                    packet_map_id,
                    &values_update,
                ) else {
                    continue;
                };
                packets.push((source_guid, update.to_bytes()));
            }

            for represented_update in summary.unit_values_updates {
                let source_guid = represented_update.guid;
                if !hub
                    .core
                    .client_visible_guids_like_cpp
                    .contains(&source_guid)
                {
                    continue;
                }
                let source_visibility = map
                    .with_creature_or_pet_like_cpp(source_guid, |source, owner| {
                        let source_world = source.unit().world();
                        (
                            source_world.object().is_in_world()
                                && viewer_phase.can_see(source_world.phase_shift())
                                && source_world
                                    .position()
                                    .is_within_dist_2d(&viewer_position, visibility_range),
                            owner == Some(viewer_guid),
                        )
                    })
                    .unwrap_or((false, false));
                if !source_visibility.0 {
                    continue;
                }
                let mut values_update = represented_update.values_update;
                filter_unit_values_update_for_target_like_cpp(
                    &mut values_update,
                    source_visibility.1,
                );
                let Some(update) =
                    unit_values_update_to_update_object(source_guid, packet_map_id, &values_update)
                else {
                    continue;
                };
                packets.push((source_guid, update.to_bytes()));
            }
        }

        // A session can change instance while the map lock is released.  The
        // snapshot is scoped to the admitted map key; fail closed before any
        // packet leaves the Session if that key is no longer current.
        if hub.core.current_canonical_player_map_key_like_cpp() != Some(key) {
            return 0;
        }

        let mut sent = 0;
        for (source_guid, bytes) in packets {
            // Recheck `HaveAtClient` after dropping the map lock.  A queued
            // visibility transition may have retired the source meanwhile.
            if source_guid != viewer_guid
                && !hub
                    .core
                    .client_visible_guids_like_cpp
                    .contains(&source_guid)
            {
                continue;
            }
            let fingerprint =
                represented_dynamic_object_values_update_delivery_fingerprint_like_cpp(
                    source_guid,
                    &bytes,
                );
            if !self
                .represented_player_unit_values_updates_delivered_like_cpp
                .insert((
                    key.map_id,
                    key.instance_id,
                    update_generation,
                    source_guid,
                    fingerprint,
                ))
            {
                continue;
            }
            hub.core.send_raw_packet(&bytes);
            sent += 1;
        }
        sent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_entities::{Player, PlayerEffectiveCombatStatsLikeCpp};

    #[test]
    fn derived_stat_fields_remain_owner_only_while_health_stays_public() {
        let mut player = Player::new(Some(7), false);
        player.unit_mut().set_max_health(100);
        player.clear_data_changes();
        player.unit_mut().set_health(50);
        player.replace_effective_combat_stats_for_values(PlayerEffectiveCombatStatsLikeCpp {
            stats: [1, 0, 0, 0, 0],
            mainhand_expertise: 1.0,
            ..Default::default()
        });
        let complete_update = player.values_update(true);

        let mut owner_update = complete_update.clone();
        filter_player_values_update_for_target_like_cpp(&mut owner_update, true, true);
        let owner_unit = owner_update.unit_data.as_ref().unwrap();
        assert!(owner_unit.mask.is_set(5), "Health is present for the owner");
        assert!(
            owner_unit.mask.is_set(175),
            "derived Stats are present for the owner"
        );
        assert!(owner_update.active_player_data.is_some());
        assert!(
            owner_update
                .active_player_data
                .as_ref()
                .unwrap()
                .mask
                .is_set(36)
        );

        let mut observer_update = complete_update;
        filter_player_values_update_for_target_like_cpp(&mut observer_update, false, false);
        let observer_unit = observer_update.unit_data.as_ref().unwrap();
        assert!(
            observer_unit.mask.is_set(5),
            "public Health remains visible"
        );
        assert!(
            !observer_unit.mask.is_set(175),
            "derived Stats are owner-only"
        );
        assert!(observer_update.active_player_data.is_none());
        assert_eq!(
            observer_update.changed_object_type_mask & (1 << TYPEID_ACTIVE_PLAYER),
            0
        );
    }

    #[test]
    fn quest_log_fields_are_visible_to_self_and_authoritative_current_group_only() {
        let mut player = Player::new(None, false);
        player.clear_data_changes();
        assert!(player.set_quest_slot_like_cpp(0, 1234));
        let complete_update = player.values_update(false);
        let player_data = complete_update.player_data.as_ref().unwrap();
        assert!(player_data.mask.is_set(35));
        assert!(player_data.mask.is_set(36));
        assert_eq!(player_data.quest_log[0].change_mask(), (1 << 29) - 1);

        let mut self_update = complete_update.clone();
        filter_player_values_update_for_target_like_cpp(&mut self_update, true, true);
        assert!(self_update.player_data.as_ref().unwrap().mask.is_set(36));

        let mut party_update = complete_update.clone();
        filter_player_values_update_for_target_like_cpp(&mut party_update, false, true);
        assert!(party_update.player_data.as_ref().unwrap().mask.is_set(36));
        assert!(party_update.active_player_data.is_none());

        let mut unrelated_update = complete_update;
        filter_player_values_update_for_target_like_cpp(&mut unrelated_update, false, false);
        assert!(unrelated_update.player_data.is_none());
    }

    #[test]
    fn current_group_visibility_requires_matching_source_snapshot_and_registry_member() {
        let active_group_guid = 77;
        let source = ObjectGuid::create_player(1, 7302);
        let members = vec![ObjectGuid::create_player(1, 7301), source];
        let active_group = (active_group_guid, members);

        assert!(source_is_in_current_group_like_cpp(
            Some(&active_group),
            source,
            Some(active_group_guid),
        ));
        assert!(!source_is_in_current_group_like_cpp(
            Some(&active_group),
            source,
            Some(active_group_guid + 1),
        ));
        assert!(!source_is_in_current_group_like_cpp(
            Some(&active_group),
            ObjectGuid::create_player(1, 7303),
            Some(active_group_guid),
        ));
    }
}
