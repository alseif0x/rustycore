// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature canonical adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{HashSet, ObjectGuid, OwnedLootAuthority, OwnedLootAuthorityLifecycle};
use super::{OwnedLootAuthorityStamp, Position, SharedCanonicalMapManager, info};

pub(crate) fn relocate_canonical_creature_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
    position: Position,
) {
    let Ok(mut manager) = manager.lock() else {
        return;
    };
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return;
    };
    let _ = map.map_mut().relocate_map_object_like_cpp(guid, position);
}

pub(crate) fn sync_canonical_creature_entity_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    mut creature: wow_entities::Creature,
) -> Option<OwnedLootAuthority> {
    let guid = creature.unit().world().object().guid();
    let Ok(mut manager) = manager.lock() else {
        return None;
    };
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return None;
    };
    if map
        .map()
        .creature_transform_vitals_snapshot_like_cpp(guid)
        .is_none()
    {
        return None;
    }
    let accept_incoming_entity_state = map.map().with_creature_like_cpp(guid, |current| {
        let incoming_unit = creature.unit();
        let current_unit = current.unit();
        let shares_health_timeline = incoming_unit.shares_health_state_revision_authority_like_cpp(
            &current_unit.health_state_revision_authority_like_cpp(),
        );
        let incoming_revision = incoming_unit.health_state_revision_like_cpp();
        let current_revision = current_unit.health_state_revision_like_cpp();
        let health_tuple_matches = incoming_unit.data().health == current_unit.data().health
            && incoming_unit.data().max_health == current_unit.data().max_health
            && incoming_unit.death_state() == current_unit.death_state();

        // Whole-entity replacement is safe only inside the same incarnation
        // timeline. A lower revision is a stale snapshot even when health has
        // completed an ABA cycle; an equal revision is valid only when its full
        // represented health tuple agrees. Reject the entire snapshot instead
        // of copying only health, because death/respawn hooks also mutate AI,
        // combat, loot, aura, timer, flag, and runtime-plan state.
        shares_health_timeline
            && (incoming_revision > current_revision
                || (incoming_revision == current_revision && health_tuple_matches))
    })?;

    let current_authority = map
        .map()
        .with_creature_like_cpp(guid, |current| current.loot_authority_like_cpp().clone())?;
    let incoming_authority = creature.loot_authority_like_cpp().clone();
    let current_stamp = current_authority.stamp_like_cpp();
    let incoming_stamp = incoming_authority.stamp_like_cpp();
    let authority = reconcile_creature_loot_authority_mirrors_like_cpp(
        &current_authority,
        current_stamp,
        &incoming_authority,
        incoming_stamp,
    );
    map.map_mut().with_creature_mut_like_cpp(guid, |current| {
        current.rebind_loot_authority_if_current_like_cpp(
            &current_authority,
            current_stamp,
            authority.clone(),
        )
    })??;
    if !accept_incoming_entity_state {
        // The actual legacy owner performs its own expected-stamp CAS with the
        // returned authority. Its rejected transport clone must not replace any
        // canonical lifecycle fields.
        return Some(authority);
    }
    // `creature` is a cloned transport snapshot whose old authority is still
    // owned by the live legacy entity. Do not detach it here; the caller
    // performs the expected-stamp CAS on that actual entity.
    creature.adopt_loot_authority_for_snapshot_like_cpp(authority);
    let old_threat_guids = map
        .map()
        .with_creature_like_cpp(guid, |current| {
            current.unit().subsystems().combat.sorted_threat_guids()
        })
        .unwrap_or_default();
    let incoming_threat_guids: HashSet<_> = creature
        .unit()
        .subsystems()
        .combat
        .sorted_threat_guids()
        .into_iter()
        .collect();
    let removed_threat_guids: Vec<_> = old_threat_guids
        .into_iter()
        .filter(|threat_guid| !incoming_threat_guids.contains(threat_guid))
        .collect();
    let mirrored_threat_guids: Vec<_> = incoming_threat_guids.iter().copied().collect();

    creature.unit_mut().world_mut().object_mut().add_to_world();
    let Ok(record) = wow_entities::MapObjectRecord::new_creature(creature) else {
        return None;
    };
    let authority = record
        .creature()
        .map(|creature| creature.loot_authority_like_cpp().clone())?;
    map.map_mut().insert_map_object_record(record).ok()?;
    for added_guid in mirrored_threat_guids {
        let threat_ref = map
            .map()
            .with_creature_like_cpp(guid, |creature| {
                creature
                    .unit()
                    .subsystems()
                    .combat
                    .threat_ref(added_guid)
                    .copied()
            })
            .flatten();
        let Some(threat_ref) = threat_ref else {
            continue;
        };
        if let Some(player) = map.map_mut().get_typed_player_mut(added_guid) {
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .put_threatened_by_me_ref(guid, threat_ref);
        } else if let Some(creature) = map.map_mut().get_typed_creature_mut(added_guid) {
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .put_threatened_by_me_ref(guid, threat_ref);
        }
    }
    for removed_guid in removed_threat_guids {
        if let Some(player) = map.map_mut().get_typed_player_mut(removed_guid) {
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_threatened_by_me_ref(guid);
        } else if let Some(creature) = map.map_mut().get_typed_creature_mut(removed_guid) {
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_threatened_by_me_ref(guid);
        }
    }
    Some(authority)
}

/// Selects one backing authority for two mirrors without ever merging two
/// independently claimable active states. Distinct non-pristine authorities
/// are quarantined as one retired canonical tombstone.
pub(crate) fn reconcile_creature_loot_authority_mirrors_like_cpp(
    canonical: &OwnedLootAuthority,
    canonical_stamp: OwnedLootAuthorityStamp,
    incoming: &OwnedLootAuthority,
    incoming_stamp: OwnedLootAuthorityStamp,
) -> OwnedLootAuthority {
    if canonical.shares_storage_like_cpp(incoming) {
        return canonical.clone();
    }

    use OwnedLootAuthorityLifecycle::{Active, Detached, Pristine, Quarantined, Retired};

    match (canonical_stamp.lifecycle, incoming_stamp.lifecycle) {
        // Once divergent live pools were observed, keep the attached terminal
        // tombstone until object destruction. It must not be reopened merely
        // because another stale mirror still looks active.
        (Quarantined, _) => canonical.clone(),
        (_, Quarantined) => incoming.clone(),
        // Two independently claimable live pools, or a live pool conflicting
        // with an attached destruction tombstone, are ambiguous without a
        // shared incarnation id. Converge on a terminal fail-closed authority.
        (Active, Active) | (Active, Retired) | (Retired, Active) => {
            return OwnedLootAuthority::new_retired_tombstone_like_cpp();
        }
        // A live authority can safely fill a never-used placeholder. A
        // detached allocation has already lost entity ownership.
        (Active, Pristine | Detached) => canonical.clone(),
        (Pristine | Detached, Active) => incoming.clone(),
        // A still-attached retired authority is the lifetime tombstone shared
        // across respawn/restock. A displaced authority is classified as
        // `Detached`, so it cannot win this branch or be resurrected.
        (Retired, Pristine) => canonical.clone(),
        (Pristine, Retired) => incoming.clone(),
        (Pristine, Pristine) | (Retired, Retired) => canonical.clone(),
        (Detached, Detached) => OwnedLootAuthority::new_retired_tombstone_like_cpp(),
        (Detached, _) => incoming.clone(),
        (_, Detached) => canonical.clone(),
    }
}

pub(crate) fn remove_canonical_creature_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
) {
    let Ok(mut manager) = manager.lock() else {
        return;
    };
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return;
    };
    let _ = map.map_mut().remove_from_map_like_cpp(guid, true);
}

pub(crate) fn add_canonical_creature_respawn_info_and_remove_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
    info: wow_map::RespawnInfoLikeCpp,
) -> (bool, bool) {
    let Ok(mut manager) = manager.lock() else {
        return (false, false);
    };
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return (false, false);
    };

    let respawn_added = matches!(
        map.map_mut().add_respawn_info_like_cpp(info),
        wow_map::AddRespawnInfoOutcomeLikeCpp::Inserted
            | wow_map::AddRespawnInfoOutcomeLikeCpp::ReplacedExisting
    );
    let object_removed = map.map_mut().remove_from_map_like_cpp(guid, true).is_ok();
    (respawn_added, object_removed)
}

pub(crate) fn remove_canonical_respawn_time_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
) -> bool {
    let Ok(mut manager) = manager.lock() else {
        return false;
    };
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return false;
    };
    map.map_mut()
        .remove_respawn_time_like_cpp(object_type, spawn_id)
        .is_some()
}
