// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature canonical adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use std::collections::HashSet;

use crate::session::SharedCanonicalMapManager;
use wow_core::{ObjectGuid, Position};
use wow_loot::{OwnedLootAuthority, OwnedLootAuthorityLifecycle, OwnedLootAuthorityStamp};

pub fn relocate_canonical_creature_map_object_on_map_like_cpp(
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

/// What applying one coexisting creature representation did to the canonical
/// incarnation that currently owns its GUID.
#[derive(Debug)]
pub enum CanonicalCreatureEntityApplicationLikeCpp {
    /// The snapshot replaced the canonical entity state; the returned authority
    /// is the incarnation's authority.
    Applied(OwnedLootAuthority),
    /// The incarnation admitted the representation but refused this exact
    /// snapshot under the health-revision/tuple guard. The canonical entity
    /// state is unchanged, so the caller must not publish the mutation.
    Rejected(OwnedLootAuthority),
    /// The incarnation, its authority or the canonical object refused the
    /// representation outright. Nothing was selected and nothing was written.
    Refused,
}

/// Is `incoming` a representation of the incarnation `current` belongs to?
///
/// This is the read-only admission predicate evaluated *before* an owner mutates
/// an existing legacy representation: the same-incarnation health timeline, a
/// snapshot at or ahead of the canonical revision, and either the incarnation's
/// own loot allocation or an unused pristine candidate. It is the same guard
/// [`apply_canonical_creature_entity_on_map_like_cpp`] applies, so an admitted
/// representation cannot be refused because of its incarnation.
pub fn creature_representation_is_admitted_like_cpp(
    current: &wow_entities::Creature,
    incoming: &wow_entities::Creature,
) -> bool {
    let current_unit = current.unit();
    let incoming_unit = incoming.unit();
    if !incoming_unit.shares_health_state_revision_authority_like_cpp(
        &current_unit.health_state_revision_authority_like_cpp(),
    ) {
        return false;
    }
    let incoming_revision = incoming_unit.health_state_revision_like_cpp();
    let current_revision = current_unit.health_state_revision_like_cpp();
    let health_tuple_matches = incoming_unit.data().health == current_unit.data().health
        && incoming_unit.data().max_health == current_unit.data().max_health
        && incoming_unit.death_state() == current_unit.death_state();
    if !(incoming_revision > current_revision
        || (incoming_revision == current_revision && health_tuple_matches))
    {
        return false;
    }
    let incoming_authority = incoming.loot_authority_like_cpp();
    incoming_authority.shares_storage_like_cpp(current.loot_authority_like_cpp())
        || incoming_authority.is_pristine_like_cpp()
}

/// Apply one transported creature snapshot to the canonical incarnation that
/// currently owns its GUID, with the canonical map already locked.
///
/// This is the in-guard half of [`sync_canonical_creature_entity_on_map_like_cpp`]:
/// the caller owns the canonical execution lock, so this performs no manager
/// lookup, no manager re-entry, no I/O, no await and no delivery.
pub fn apply_canonical_creature_entity_on_map_like_cpp(
    map: &mut wow_map::ManagedMapInnerLikeCpp,
    mut creature: wow_entities::Creature,
) -> CanonicalCreatureEntityApplicationLikeCpp {
    let guid = creature.unit().world().object().guid();
    if map
        .creature_transform_vitals_snapshot_like_cpp(guid)
        .is_none()
    {
        return CanonicalCreatureEntityApplicationLikeCpp::Refused;
    }
    let Some((shares_health_timeline, accept_incoming_entity_state, current_authority)) = map
        .with_creature_like_cpp(guid, |current| {
            let incoming_unit = creature.unit();
            let current_unit = current.unit();
            let shares_health_timeline = incoming_unit
                .shares_health_state_revision_authority_like_cpp(
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
            (
                shares_health_timeline,
                shares_health_timeline
                    && (incoming_revision > current_revision
                        || (incoming_revision == current_revision && health_tuple_matches)),
                current.loot_authority_like_cpp().clone(),
            )
        })
    else {
        return CanonicalCreatureEntityApplicationLikeCpp::Refused;
    };

    let incoming_authority = creature.loot_authority_like_cpp().clone();
    // R1b guard, before any authority is selected: a coexisting representation
    // may only synchronize an incarnation it belongs to and may only carry the
    // incarnation's own allocation or an unused pristine candidate. A snapshot
    // from another health timeline is a different incarnation, and a second
    // independently used allocation is a competing claimable pool, so neither
    // may be reconciled into this object or republished from it. Refusing here
    // leaves the canonical object and its authority untouched; the legacy owner
    // keeps whatever it had instead of adopting a competing pool. The
    // reconciliation below stays the compatibility repair for the remaining
    // same-incarnation cases.
    if !shares_health_timeline
        || (!incoming_authority.shares_storage_like_cpp(&current_authority)
            && !incoming_authority.is_pristine_like_cpp())
    {
        return CanonicalCreatureEntityApplicationLikeCpp::Refused;
    }
    let current_stamp = current_authority.stamp_like_cpp();
    let incoming_stamp = incoming_authority.stamp_like_cpp();
    let authority = reconcile_creature_loot_authority_mirrors_like_cpp(
        &current_authority,
        current_stamp,
        &incoming_authority,
        incoming_stamp,
    );
    // The original `??` contract: a missing canonical object or a failed
    // expected-stamp compare/exchange refuses the snapshot. `Some(false)` is the
    // successful "already this authority" case, not a refusal.
    if map
        .with_creature_mut_like_cpp(guid, |current| {
            current.rebind_loot_authority_if_current_like_cpp(
                &current_authority,
                current_stamp,
                authority.clone(),
            )
        })
        .flatten()
        .is_none()
    {
        return CanonicalCreatureEntityApplicationLikeCpp::Refused;
    }
    if !accept_incoming_entity_state {
        // The actual legacy owner performs its own expected-stamp CAS with the
        // returned authority. Its rejected transport clone must not replace any
        // canonical lifecycle fields.
        return CanonicalCreatureEntityApplicationLikeCpp::Rejected(authority);
    }
    // `creature` is a cloned transport snapshot whose old authority is still
    // owned by the live legacy entity. Do not detach it here; the caller
    // performs the expected-stamp CAS on that actual entity.
    creature.adopt_loot_authority_for_snapshot_like_cpp(authority);
    let old_threat_guids = map
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
        return CanonicalCreatureEntityApplicationLikeCpp::Refused;
    };
    let Some(authority) = record
        .creature()
        .map(|creature| creature.loot_authority_like_cpp().clone())
    else {
        return CanonicalCreatureEntityApplicationLikeCpp::Refused;
    };
    if map.insert_map_object_record(record).is_err() {
        return CanonicalCreatureEntityApplicationLikeCpp::Refused;
    }
    for added_guid in mirrored_threat_guids {
        let threat_ref = map
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
        if let Some(player) = map.get_typed_player_mut(added_guid) {
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .put_threatened_by_me_ref(guid, threat_ref);
        } else if let Some(creature) = map.get_typed_creature_mut(added_guid) {
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .put_threatened_by_me_ref(guid, threat_ref);
        }
    }
    for removed_guid in removed_threat_guids {
        if let Some(player) = map.get_typed_player_mut(removed_guid) {
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_threatened_by_me_ref(guid);
        } else if let Some(creature) = map.get_typed_creature_mut(removed_guid) {
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_threatened_by_me_ref(guid);
        }
    }
    CanonicalCreatureEntityApplicationLikeCpp::Applied(authority)
}

/// Synchronize one transported creature snapshot through the shared canonical
/// map manager.
///
/// This is the locking compatibility wrapper around
/// [`apply_canonical_creature_entity_on_map_like_cpp`]: it keeps the historical
/// signature and the historical return contract (the selected authority for an
/// applied *or* rejected snapshot, `None` only for a refused one). Owners that
/// mutate an existing representation go through
/// [`crate::session::SessionCore::mutate_world_creature`], which holds the
/// canonical execution lock across the mutation and refuses to expose a result
/// the canonical incarnation did not apply.
pub fn sync_canonical_creature_entity_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    creature: wow_entities::Creature,
) -> Option<OwnedLootAuthority> {
    let Ok(mut manager) = manager.lock() else {
        return None;
    };
    let map = manager.find_map_mut(map_id, instance_id)?;
    match apply_canonical_creature_entity_on_map_like_cpp(map.map_mut(), creature) {
        CanonicalCreatureEntityApplicationLikeCpp::Applied(authority)
        | CanonicalCreatureEntityApplicationLikeCpp::Rejected(authority) => Some(authority),
        CanonicalCreatureEntityApplicationLikeCpp::Refused => None,
    }
}

/// Selects one backing authority for two mirrors without ever merging two
/// independently claimable active states. Distinct non-pristine authorities
/// are quarantined as one retired canonical tombstone.
pub fn reconcile_creature_loot_authority_mirrors_like_cpp(
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

pub fn remove_canonical_creature_map_object_on_map_like_cpp(
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

pub fn add_canonical_creature_respawn_info_and_remove_map_object_on_map_like_cpp(
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

pub fn remove_canonical_respawn_time_on_map_like_cpp(
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
