//! F6-7 R7a regressions: one canonical application per admitted creature
//! mutation (reviewer signature §5.3.1 R7, repair order §5.3.2).
//!
//! R7a gates the single mutation root `SessionCore::mutate_world_creature`:
//!
//! * ownership is admitted against the *current canonical incarnation* before
//!   the mutation is invoked, so a stale snapshot, an ABA replay, a
//!   representation from another incarnation and a competing used loot
//!   allocation are all refused with the callback unexecuted;
//! * the mutated representation replaces the canonical entity state before the
//!   caller can observe success, so a successful mutation can no longer survive
//!   only in the legacy mirror;
//! * success and its events are exposed only after that canonical application,
//!   so a refused operation publishes nothing.
//!
//! These tests drive the production root and two of its direct production
//! consumers (the represented creature kill-hook completion and the looted
//! corpse lifecycle) rather than a test double.

use super::*;

/// The canonical creature for `guid`, if any.
pub(super) fn canonical_creature_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) -> Option<wow_entities::Creature> {
    canonical
        .lock()
        .ok()?
        .find_map(0, 0)?
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
}

/// The legacy representation for `guid`, if any.
pub(super) fn legacy_creature_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    guid: ObjectGuid,
) -> Option<wow_entities::Creature> {
    manager
        .read()
        .ok()?
        .find_creature(0, 0, guid)
        .map(|world_creature| world_creature.creature.clone())
}

/// A registered creature whose legacy representation and canonical incarnation
/// are one incarnation, which is what production registration produces.
pub(super) fn mirrored_session_like_cpp(
    guid: ObjectGuid,
    hp: u32,
) -> (
    WorldSession,
    crate::map_manager::SharedMapManager,
    SharedCanonicalMapManager,
) {
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    register_test_creature_mirrored_like_cpp(&mut session, manager.clone(), &canonical, guid, hp);
    (session, manager, canonical)
}

/// Advance the canonical incarnation's health timeline without touching the
/// legacy representation, which is what makes a representation stale.
pub(super) fn advance_canonical_max_health_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    max_health: u64,
) {
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .expect("canonical map instance")
        .map_mut()
        .with_creature_mut_like_cpp(guid, |creature| {
            creature.unit_mut().set_max_health(max_health)
        })
        .expect("canonical incarnation");
}

/// Every observable both representations must agree on. Comparing the whole
/// snapshot is what makes "a refused operation mutated nothing" checkable.
#[derive(Debug, PartialEq)]
pub(super) struct CreatureObservablesLikeCpp {
    health: u64,
    max_health: u64,
    death_state: wow_constants::unit::DeathState,
    level: u8,
    npc_flags: u32,
    loot_lifecycle_revision: u64,
    loot_stamp: wow_loot::OwnedLootAuthorityStamp,
    death_time_ms: Option<u64>,
    corpse_despawn_at_ms: Option<u64>,
    respawn_delay: u32,
    respawn_time: i64,
    save_respawn_requested: bool,
    combat_target: Option<ObjectGuid>,
    move_target: Option<Position>,
    attacking: Option<ObjectGuid>,
    is_dead: bool,
}

pub(super) fn observables_like_cpp(
    creature: &wow_entities::Creature,
) -> CreatureObservablesLikeCpp {
    CreatureObservablesLikeCpp {
        health: creature.unit().data().health,
        max_health: creature.unit().data().max_health,
        death_state: creature.unit().death_state(),
        level: creature.level(),
        npc_flags: creature.ai_ownership().npc_flags,
        loot_lifecycle_revision: creature.loot_lifecycle_revision_like_cpp(),
        loot_stamp: creature.loot_authority_like_cpp().stamp_like_cpp(),
        death_time_ms: creature.ai_ownership().death_time_ms,
        corpse_despawn_at_ms: creature.ai_ownership().corpse_despawn_at_ms,
        respawn_delay: creature.respawn_delay(),
        respawn_time: creature.respawn_time(),
        save_respawn_requested: creature.runtime_state().save_respawn_requested,
        combat_target: creature.ai_ownership().combat_target,
        move_target: creature.ai_ownership().move_target,
        attacking: creature.unit().attacking(),
        is_dead: creature.unit().is_dead(),
    }
}

/// One independently constructed admission candidate, matching what production
/// registration sets before admission.
fn admission_candidate_like_cpp(guid: ObjectGuid) -> wow_entities::Creature {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(9_400);
    let _ = creature.unit_mut().world_mut().set_map(0, 0);
    creature.unit_mut().world_mut().relocate(Position::ZERO);
    creature.unit_mut().set_level(1);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature
}

fn creature_loot_like_cpp(_guid: ObjectGuid, coins: u32, player: ObjectGuid) -> CreatureLoot {
    CreatureLoot {
        loot_guid: ObjectGuid::create_world_object(
            HighGuid::LootObject,
            0,
            0,
            0,
            0,
            0,
            i64::from(coins),
        ),
        coins,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![player],
        items: Vec::new(),
        looted_by_player: false,
    }
}

/// A used (non-pristine) allocation that has already owned a loot lifetime.
fn used_allocation_like_cpp(guid: ObjectGuid, player: ObjectGuid) -> wow_loot::OwnedLootAuthority {
    let authority = wow_loot::OwnedLootAuthority::new();
    let _ = authority.initialize_shared_like_cpp(creature_loot_like_cpp(guid, 11, player));
    authority
}

/// Install `authority` on the legacy representation, displacing only that
/// representation's own allocation.
fn install_legacy_authority_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    guid: ObjectGuid,
    authority: wow_loot::OwnedLootAuthority,
) {
    let mut guard = manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let creature = guard
        .find_creature_mut(0, 0, guid)
        .expect("legacy representation");
    assert!(
        creature.creature.rebind_loot_authority_like_cpp(authority),
        "the legacy representation adopts the competing allocation"
    );
}

/// The kill-hook events of one lethal completion, per named fact:
/// `[DeathStateJustDied, ZoneScriptUnitDeath, LootFlagsApplied,
/// CreatureOnHealthDepletedAi, CreatureJustDiedAi, ScriptMgrOnCreatureKill]`.
pub(super) fn lethal_event_counts_like_cpp(session: &WorldSession, guid: ObjectGuid) -> [usize; 6] {
    let mut counts = [0_usize; 6];
    for event in session.represented_creature_kill_events_like_cpp() {
        match event {
            RepresentedCreatureKillEventLikeCpp::DeathStateJustDied { victim_guid }
                if *victim_guid == guid =>
            {
                counts[0] += 1;
            }
            RepresentedCreatureKillEventLikeCpp::ZoneScriptUnitDeath { unit_guid }
                if *unit_guid == guid =>
            {
                counts[1] += 1;
            }
            RepresentedCreatureKillEventLikeCpp::LootFlagsApplied { creature_guid, .. }
                if *creature_guid == guid =>
            {
                counts[2] += 1;
            }
            RepresentedCreatureKillEventLikeCpp::CreatureOnHealthDepletedAi {
                creature_guid,
                ..
            } if *creature_guid == guid => {
                counts[3] += 1;
            }
            RepresentedCreatureKillEventLikeCpp::CreatureJustDiedAi { creature_guid, .. }
                if *creature_guid == guid =>
            {
                counts[4] += 1;
            }
            RepresentedCreatureKillEventLikeCpp::ScriptMgrOnCreatureKill {
                creature_guid, ..
            } if *creature_guid == guid => {
                counts[5] += 1;
            }
            _ => {}
        }
    }
    counts
}

#[test]
fn fresh_same_incarnation_non_health_mutation_is_applied_once_to_both_like_cpp() {
    let guid = test_creature_guid(91_600);
    let (mut session, manager, canonical) = mirrored_session_like_cpp(guid, 100);
    let legacy_before = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    let canonical_before = canonical_creature_like_cpp(&canonical, guid).expect("canonical");
    assert!(
        legacy_before
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(canonical_before.loot_authority_like_cpp()),
        "the registered pair is one incarnation with one authority"
    );

    let mut callback_runs = 0_usize;
    let applied = session
        .mutate_world_creature(guid, |creature| {
            callback_runs += 1;
            creature.creature.ai_ownership_mut().npc_flags = 0x0001_0042;
            creature.creature.unit_mut().set_level(41);
        })
        .is_some();

    assert!(applied, "a fresh same-incarnation mutation is admitted");
    assert_eq!(callback_runs, 1, "the mutation executes exactly once");
    let legacy_after = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    let canonical_after = canonical_creature_like_cpp(&canonical, guid).expect("canonical");
    assert_eq!(
        legacy_after.ai_ownership().npc_flags,
        0x0001_0042,
        "the legacy representation carries the mutation"
    );
    assert_eq!(
        canonical_after.ai_ownership().npc_flags,
        0x0001_0042,
        "the canonical incarnation carries the same mutation"
    );
    assert_eq!(legacy_after.level(), 41);
    assert_eq!(canonical_after.level(), 41);
    assert_eq!(
        observables_like_cpp(&legacy_after),
        observables_like_cpp(&canonical_after),
        "one incarnation, one observable state"
    );
    assert!(
        canonical_after
            .unit()
            .shares_health_state_revision_authority_like_cpp(
                &legacy_after
                    .unit()
                    .health_state_revision_authority_like_cpp()
            ),
        "the mutation preserves the incarnation's health timeline"
    );
    assert!(
        legacy_after
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(canonical_after.loot_authority_like_cpp()),
        "the mutation preserves the incarnation's loot authority"
    );
}

/// **Negative control (R7a).** Executed against the original bridge at
/// `40042c0cd` it fails: the stale non-health mutation is applied to the legacy
/// representation and reported as success while the canonical incarnation keeps
/// its earlier state. It passes once the gate admits ownership before invoking
/// the mutation.
#[test]
fn stale_same_incarnation_non_health_mutation_is_refused_before_invocation_like_cpp() {
    let guid = test_creature_guid(91_601);
    let (mut session, manager, canonical) = mirrored_session_like_cpp(guid, 100);
    let canonical_before = canonical_creature_like_cpp(&canonical, guid).expect("canonical");
    let legacy_before = observables_like_cpp(
        &legacy_creature_like_cpp(&manager, guid).expect("legacy representation"),
    );
    // The canonical incarnation advances without the representation: the
    // representation is now a stale snapshot of the same incarnation.
    advance_canonical_max_health_like_cpp(&canonical, guid, 140);
    let canonical_stale =
        observables_like_cpp(&canonical_creature_like_cpp(&canonical, guid).expect("canonical"));
    assert_ne!(
        canonical_stale.max_health, legacy_before.max_health,
        "the canonical incarnation is ahead of the representation"
    );

    let mut callback_runs = 0_usize;
    let refused = session
        .mutate_world_creature(guid, |creature| {
            callback_runs += 1;
            creature.creature.ai_ownership_mut().npc_flags = 0x0000_0099;
        })
        .is_none();

    assert!(
        refused,
        "a stale representation must not be reported as a successful mutation"
    );
    assert_eq!(
        callback_runs, 0,
        "ownership is refused before the mutation is invoked"
    );
    assert_eq!(
        observables_like_cpp(
            &legacy_creature_like_cpp(&manager, guid).expect("legacy representation")
        ),
        legacy_before,
        "a refused operation does not mutate the legacy representation"
    );
    assert_eq!(
        observables_like_cpp(&canonical_creature_like_cpp(&canonical, guid).expect("canonical")),
        canonical_stale,
        "a refused operation does not mutate the canonical incarnation"
    );
    assert!(
        canonical_before
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(
                canonical_creature_like_cpp(&canonical, guid)
                    .expect("canonical")
                    .loot_authority_like_cpp()
            ),
        "a refused operation does not displace the incarnation's authority"
    );
}

#[test]
fn aba_health_cycle_cannot_replay_a_stale_non_health_mutation_like_cpp() {
    let guid = test_creature_guid(91_602);
    let (mut session, manager, canonical) = mirrored_session_like_cpp(guid, 100);
    // Health leaves and returns to the same tuple while the revision advances:
    // the representation's tuple agrees but its revision is behind.
    {
        let mut guard = canonical.lock().unwrap();
        let map = guard.find_map_mut(0, 0).expect("canonical map instance");
        map.map_mut()
            .with_creature_mut_like_cpp(guid, |creature| {
                creature.unit_mut().set_health(60);
                creature.unit_mut().set_health(100);
            })
            .expect("canonical incarnation");
    }
    let canonical_before =
        observables_like_cpp(&canonical_creature_like_cpp(&canonical, guid).expect("canonical"));
    let legacy_before = observables_like_cpp(
        &legacy_creature_like_cpp(&manager, guid).expect("legacy representation"),
    );
    assert_eq!(canonical_before.health, legacy_before.health);
    assert_eq!(canonical_before.death_state, legacy_before.death_state);

    let mut callback_runs = 0_usize;
    assert!(
        session
            .mutate_world_creature(guid, |creature| {
                callback_runs += 1;
                creature.creature.ai_ownership_mut().npc_flags = 0x0000_00AB;
            })
            .is_none(),
        "an ABA replay is a stale snapshot, not a fresh one"
    );
    assert_eq!(callback_runs, 0);
    assert_eq!(
        observables_like_cpp(
            &legacy_creature_like_cpp(&manager, guid).expect("legacy representation")
        ),
        legacy_before
    );
    assert_eq!(
        observables_like_cpp(&canonical_creature_like_cpp(&canonical, guid).expect("canonical")),
        canonical_before
    );
}

#[test]
fn different_incarnation_replacement_refuses_the_legacy_representation_like_cpp() {
    let guid = test_creature_guid(91_603);
    let (mut session, manager, canonical) = mirrored_session_like_cpp(guid, 100);
    let destroyed = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    // The canonical object is destroyed and recreated under the same GUID: the
    // legacy representation now belongs to the destroyed incarnation.
    remove_canonical_creature_map_object_on_map_like_cpp(&canonical, 0, 0, guid);
    let replacement = insert_canonical_creature_map_object_on_map_like_cpp(
        &canonical,
        0,
        0,
        admission_candidate_like_cpp(guid),
    )
    .expect("the recreation is admitted");
    assert!(
        !replacement
            .loot_authority
            .shares_storage_like_cpp(destroyed.loot_authority_like_cpp()),
        "a destroyed/recreated creature gets a new allocation"
    );
    let canonical_before =
        observables_like_cpp(&canonical_creature_like_cpp(&canonical, guid).expect("replacement"));
    let legacy_before = observables_like_cpp(
        &legacy_creature_like_cpp(&manager, guid).expect("legacy representation"),
    );

    let mut callback_runs = 0_usize;
    assert!(
        session
            .mutate_world_creature(guid, |creature| {
                callback_runs += 1;
                creature.creature.ai_ownership_mut().npc_flags = 0x0000_00AC;
            })
            .is_none(),
        "a representation of a destroyed incarnation must not mutate the replacement"
    );
    assert_eq!(callback_runs, 0);
    assert_eq!(
        observables_like_cpp(
            &legacy_creature_like_cpp(&manager, guid).expect("legacy representation")
        ),
        legacy_before
    );
    assert_eq!(
        observables_like_cpp(&canonical_creature_like_cpp(&canonical, guid).expect("replacement")),
        canonical_before,
        "the replacement keeps its own state and lifetime"
    );
}

#[test]
fn competing_used_loot_allocation_refuses_the_representation_before_invocation_like_cpp() {
    let guid = test_creature_guid(91_604);
    let player = ObjectGuid::create_player(1, 91_605);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let (mut session, _, _) = make_session();
    // A legacy-only registration, then an independently constructed canonical
    // incarnation for the same GUID that shares nothing but the GUID.
    register_test_creature(&mut session, manager.clone(), guid, 100);
    let canonical_authority = used_allocation_like_cpp(guid, player);
    let mut candidate = admission_candidate_like_cpp(guid);
    candidate.rebind_loot_authority_like_cpp(canonical_authority.clone());
    let admitted =
        insert_canonical_creature_map_object_on_map_like_cpp(&canonical, 0, 0, candidate)
            .expect("the independent canonical incarnation is admitted");
    assert!(
        admitted
            .loot_authority
            .shares_storage_like_cpp(&canonical_authority)
    );
    // Give the representation the same health timeline but a competing used
    // allocation, so the only reason to refuse is the competing pool.
    {
        let canonical_creature = canonical_creature_like_cpp(&canonical, guid).expect("canonical");
        let mut guard = manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let legacy = guard
            .find_creature_mut(0, 0, guid)
            .expect("legacy representation");
        legacy
            .creature
            .unit_mut()
            .preserve_authoritative_health_state_for_snapshot_like_cpp(canonical_creature.unit());
    }
    let competing = used_allocation_like_cpp(guid, player);
    install_legacy_authority_like_cpp(&manager, guid, competing.clone());
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let legacy_before = observables_like_cpp(
        &legacy_creature_like_cpp(&manager, guid).expect("legacy representation"),
    );
    let canonical_before =
        observables_like_cpp(&canonical_creature_like_cpp(&canonical, guid).expect("canonical"));
    assert!(
        !legacy_creature_like_cpp(&manager, guid)
            .expect("legacy representation")
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(
                canonical_creature_like_cpp(&canonical, guid)
                    .expect("canonical")
                    .loot_authority_like_cpp()
            ),
        "the representation carries a competing used allocation, not the incarnation's"
    );

    let mut callback_runs = 0_usize;
    assert!(
        session
            .mutate_world_creature(guid, |creature| {
                callback_runs += 1;
                creature.creature.ai_ownership_mut().npc_flags = 0x0000_00AD;
            })
            .is_none(),
        "a competing used allocation must not be reconciled into the incarnation"
    );
    assert_eq!(callback_runs, 0);
    assert_eq!(
        observables_like_cpp(
            &legacy_creature_like_cpp(&manager, guid).expect("legacy representation")
        ),
        legacy_before,
        "the refused representation keeps its own competing allocation"
    );
    assert_eq!(
        observables_like_cpp(&canonical_creature_like_cpp(&canonical, guid).expect("canonical")),
        canonical_before,
        "the canonical incarnation is untouched"
    );
    assert_eq!(
        competing.lifecycle_like_cpp(),
        wow_loot::OwnedLootAuthorityLifecycle::Active,
        "the competing pool is neither quarantined nor retired"
    );
}

#[test]
fn missing_manager_map_or_object_refuses_before_invocation_like_cpp() {
    let guid = test_creature_guid(91_606);

    // No legacy manager at all.
    let (mut session, _, _) = make_session();
    let mut callback_runs = 0_usize;
    assert!(
        session
            .mutate_world_creature(guid, |_| {
                callback_runs += 1;
            })
            .is_none(),
        "a missing legacy manager has no representation to mutate"
    );
    assert_eq!(callback_runs, 0);

    // A bound canonical manager whose map instance does not exist.
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    register_test_creature(&mut session, manager.clone(), guid, 100);
    let legacy_before = observables_like_cpp(
        &legacy_creature_like_cpp(&manager, guid).expect("legacy representation"),
    );
    session.set_canonical_map_manager(Arc::clone(&canonical));
    assert!(
        session
            .mutate_world_creature(guid, |creature| {
                callback_runs += 1;
                creature.creature.ai_ownership_mut().npc_flags = 0x0000_00AE;
            })
            .is_none(),
        "a missing canonical map instance is unavailable ownership"
    );
    assert_eq!(callback_runs, 0);

    // A canonical map instance without the incarnation for this GUID.
    canonical.lock().unwrap().create_world_map(0, 0);
    assert!(
        session
            .mutate_world_creature(guid, |creature| {
                callback_runs += 1;
                creature.creature.ai_ownership_mut().npc_flags = 0x0000_00AF;
            })
            .is_none(),
        "a missing canonical object is unavailable ownership"
    );
    assert_eq!(callback_runs, 0);
    assert_eq!(
        observables_like_cpp(
            &legacy_creature_like_cpp(&manager, guid).expect("legacy representation")
        ),
        legacy_before,
        "every refusal left the representation untouched"
    );
}

/// Documented R7a boundary: a session with **no** canonical map manager owns no
/// canonical incarnation store to diverge from, which is the same compatibility
/// contract R1a's registration already keeps for that configuration. The
/// mutation stays legacy-only instead of being refused.
#[test]
fn session_without_a_canonical_store_keeps_the_legacy_only_path_like_cpp() {
    let guid = test_creature_guid(91_607);
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    register_test_creature(&mut session, manager.clone(), guid, 100);

    let mut callback_runs = 0_usize;
    let applied = session
        .mutate_world_creature(guid, |creature| {
            callback_runs += 1;
            creature.creature.ai_ownership_mut().npc_flags = 0x0000_00B0;
        })
        .is_some();

    assert!(applied);
    assert_eq!(callback_runs, 1);
    assert_eq!(
        legacy_creature_like_cpp(&manager, guid)
            .expect("legacy representation")
            .ai_ownership()
            .npc_flags,
        0x0000_00B0
    );
}
