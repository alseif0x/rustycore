//! F6-7 R7a regressions: one canonical application per admitted creature
//! mutation (reviewer signature §5.3.1 R7, repair order §5.3.2).
//!
//! R7a gated the single mutation root `SessionCore::mutate_world_creature`
//! by admitting the legacy representation against the current canonical
//! incarnation. #1263 F6-8D3b-2 inverted the root to canonical-first and
//! F6-8E deleted the legacy copy, so the former refusals (stale snapshot,
//! competing used allocation) have no subject. The surviving regressions prove
//! the canonical contract: the callback runs once on the GUID's current
//! incarnation (also after an ABA health cycle and a recreation), its state and
//! authority are kept, and a missing owner refuses before invocation.
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

/// The creature for `guid` as the session's store holds it (#1263 F6-8E: the
/// canonical incarnation; the name is kept for the sibling fixtures).
pub(super) fn legacy_creature_like_cpp(
    manager: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) -> Option<wow_entities::Creature> {
    manager
        .lock()
        .ok()?
        .find_creature(0, 0, guid)
        .map(|world_creature| world_creature.creature.clone())
}

/// A creature registered through the session's canonical registration path.
pub(super) fn mirrored_session_like_cpp(
    guid: ObjectGuid,
    hp: u32,
) -> (
    WorldSession,
    SharedCanonicalMapManager,
    SharedCanonicalMapManager,
) {
    let manager = shared_map_manager();
    let canonical = Arc::clone(&manager);
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
fn fresh_non_health_mutation_is_applied_once_to_the_incarnation_like_cpp() {
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

#[test]
fn aba_health_cycle_mutates_the_incarnation_once_like_cpp() {
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
            .is_some(),
        "the canonical-first root mutates the current incarnation"
    );
    assert_eq!(callback_runs, 1);
    let canonical_after = canonical_creature_like_cpp(&canonical, guid).expect("canonical");
    assert_eq!(canonical_after.ai_ownership().npc_flags, 0x0000_00AB);
    let mut expected = canonical_before;
    expected.npc_flags = 0x0000_00AB;
    assert_eq!(observables_like_cpp(&canonical_after), expected);
    assert_eq!(
        observables_like_cpp(&legacy_creature_like_cpp(&manager, guid).expect("legacy projection")),
        expected
    );
    let _ = legacy_before;
}

#[test]
fn destroyed_incarnation_copy_cannot_reach_the_replacement_like_cpp() {
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
            .is_some(),
        "the canonical-first root mutates the current incarnation"
    );
    assert_eq!(callback_runs, 1);
    let canonical_after = canonical_creature_like_cpp(&canonical, guid).expect("canonical");
    assert_eq!(canonical_after.ai_ownership().npc_flags, 0x0000_00AC);
    let mut expected = canonical_before;
    expected.npc_flags = 0x0000_00AC;
    assert_eq!(
        observables_like_cpp(&canonical_after),
        expected,
        "the mutation reaches the GUID's current incarnation, the replacement"
    );
    assert!(
        canonical_after
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&replacement.loot_authority),
        "the replacement keeps its own allocation"
    );
    assert!(
        legacy_creature_like_cpp(&manager, guid)
            .expect("legacy projection")
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&replacement.loot_authority),
        "the destroyed incarnation's copy is replaced by the replacement's projection"
    );
    let _ = legacy_before;
}

#[test]
fn missing_manager_map_or_object_refuses_before_invocation_like_cpp() {
    let guid = test_creature_guid(91_606);

    // No canonical manager at all.
    let (mut session, _, _) = make_session();
    let mut callback_runs = 0_usize;
    assert!(
        session
            .mutate_world_creature(guid, |_| {
                callback_runs += 1;
            })
            .is_none(),
        "a session without a creature store has nothing to mutate"
    );
    assert_eq!(callback_runs, 0);

    // A bound canonical manager whose map instance does not exist.
    let canonical = shared_map_manager();
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
}

/// #1263 F6-8E: a session with **no** canonical map manager has no creature
/// store at all. Registration admits nothing and the root refuses; there is no
/// legacy-only path any more.
#[test]
fn session_without_a_canonical_store_publishes_and_mutates_nothing_like_cpp() {
    let guid = test_creature_guid(91_607);
    let (mut session, _, _) = make_session();
    session.set_player_map_position_like_cpp(0, Position::new(10.0, 10.0, 0.0, 0.0));
    session.register_world_creature(
        0,
        Position::new(10.0, 10.0, 0.0, 0.0),
        test_creature_create_data(guid, 9_001, 100),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );

    let mut callback_runs = 0_usize;
    assert!(
        session
            .mutate_world_creature(guid, |creature| {
                callback_runs += 1;
                creature.creature.ai_ownership_mut().npc_flags = 0x0000_00B0;
            })
            .is_none(),
        "nothing was admitted, so nothing can be mutated"
    );
    assert_eq!(callback_runs, 0);
    assert!(
        session
            .core
            .read_world_creature_like_cpp(guid, |_| ())
            .is_none()
    );
}
