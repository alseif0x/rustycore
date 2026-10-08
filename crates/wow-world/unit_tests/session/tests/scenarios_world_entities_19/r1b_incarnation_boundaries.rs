//! F6-7 R1b regressions for the synchronization and admission boundaries.
//!
//! R1b seals the boundaries that select an incarnation's authority: a coexisting
//! representation may only synchronize an incarnation it belongs to and may only
//! carry that incarnation's allocation or an unused pristine candidate.
//! Synchronization across incarnations, or from a competing used allocation, is
//! refused *before* any authority is selected, so the canonical object is never
//! mutated, reconciled or quarantined by such a snapshot. The reconciliation
//! helper itself is unchanged and stays the compatibility repair for R3.

use super::*;

/// The minimum an independently constructed admission candidate needs, matching
/// what the production registration sets before admission.
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

fn candidate_loot_like_cpp(guid: ObjectGuid, coins: u32, player: ObjectGuid) -> CreatureLoot {
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

fn canonical_creature_like_cpp(
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

#[test]
fn canonical_creature_sync_refuses_a_stale_incarnation_snapshot_like_cpp() {
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let guid = test_creature_guid(91_400);

    let first = admission_candidate_like_cpp(guid);
    let first_authority = first.loot_authority_like_cpp().clone();
    let first_timeline = first.unit().health_state_revision_authority_like_cpp();
    // A delayed transport snapshot of the incarnation that is about to be
    // destroyed, taken exactly the way the legacy consumers take it.
    let stale_snapshot = first.clone();
    let admitted = insert_canonical_creature_map_object_on_map_like_cpp(&canonical, 0, 0, first)
        .expect("fresh admission installs the incarnation");
    assert!(
        admitted
            .loot_authority
            .shares_storage_like_cpp(&first_authority)
    );

    // The object is destroyed and recreated under the same GUID.
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
            .shares_storage_like_cpp(&first_authority),
        "a destroyed/recreated creature gets a new allocation"
    );
    let replacement_timeline = canonical_creature_like_cpp(&canonical, guid)
        .expect("replacement")
        .unit()
        .health_state_revision_authority_like_cpp();
    assert!(
        !replacement_timeline.shares_storage_like_cpp(&first_timeline),
        "a destroyed/recreated creature gets a new health timeline"
    );
    let replacement_stamp = replacement.loot_authority.stamp_like_cpp();

    // A stale snapshot of the destroyed incarnation cannot alter the
    // replacement: it belongs to another timeline and carries a competing used
    // allocation, so it is refused before any authority is selected.
    assert!(
        sync_canonical_creature_entity_on_map_like_cpp(&canonical, 0, 0, stale_snapshot).is_none(),
        "a stale incarnation snapshot must be refused"
    );

    let stored = canonical_creature_like_cpp(&canonical, guid).expect("replacement survives");
    assert!(
        stored
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&replacement.loot_authority)
    );
    assert_eq!(
        stored.loot_authority_like_cpp().stamp_like_cpp(),
        replacement_stamp,
        "the replacement's lifetime is untouched by the stale publication attempt"
    );
    assert!(
        stored
            .unit()
            .shares_health_state_revision_authority_like_cpp(&replacement_timeline),
        "the replacement keeps its own health timeline"
    );
}

#[test]
fn canonical_admission_refuses_a_competing_retired_candidate_like_cpp() {
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let guid = test_creature_guid(91_401);
    let player = ObjectGuid::create_player(1, 91_401);

    let admitted = insert_canonical_creature_map_object_on_map_like_cpp(
        &canonical,
        0,
        0,
        admission_candidate_like_cpp(guid),
    )
    .expect("fresh admission installs the incarnation");
    admitted
        .loot_authority
        .initialize_shared_like_cpp(candidate_loot_like_cpp(guid, 33, player));
    let canonical_stamp_before = admitted.loot_authority.stamp_like_cpp();

    // A second, independently allocated pool that has already owned a lifetime:
    // C++ `ClearLoot` retired it, so it is a used allocation, not a pristine
    // candidate.
    let mut candidate = admission_candidate_like_cpp(guid);
    candidate.initialize_shared_loot_authority_like_cpp(candidate_loot_like_cpp(guid, 9, player));
    candidate.clear_loot_like_cpp();
    let candidate_authority = candidate.loot_authority_like_cpp().clone();
    assert_eq!(
        candidate_authority.lifecycle_like_cpp(),
        wow_loot::OwnedLootAuthorityLifecycle::Retired
    );

    let refused = insert_canonical_creature_map_object_on_map_like_cpp(&canonical, 0, 0, candidate);
    assert!(
        refused.is_none(),
        "a competing retired pool must be refused, never reconciled into the canonical object"
    );

    let stored = canonical_creature_like_cpp(&canonical, guid).expect("canonical object");
    assert!(
        stored
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&admitted.loot_authority)
    );
    assert_eq!(
        stored.loot_authority_like_cpp().stamp_like_cpp(),
        canonical_stamp_before,
        "a refused candidate must not mutate the canonical lifetime"
    );
    assert_eq!(
        stored.loot_authority_like_cpp().lifecycle_like_cpp(),
        wow_loot::OwnedLootAuthorityLifecycle::Active
    );
    assert_eq!(
        stored
            .loot_authority_like_cpp()
            .shared_snapshot_like_cpp()
            .expect("the canonical pool stays openable")
            .loot
            .coins,
        33
    );
    assert_eq!(
        candidate_authority.lifecycle_like_cpp(),
        wow_loot::OwnedLootAuthorityLifecycle::Retired,
        "the refused candidate keeps its own untouched allocation"
    );
}

/// F6-7 R7b-1 (reviewer ruling): the *reachable* admission gap, repaired at the
/// admission boundary instead of pinned.
///
/// Registration and the ready-respawn path used to publish a legacy
/// representation eagerly while a canonical map instance is created lazily.
/// With a canonical manager configured but no instance for the key there is no
/// canonical incarnation to admit that representation, so the R7 gate refused
/// *every* owner mutation on it for its whole life. The reviewer's ruling is
/// that such a candidate has **no admitted owner** and its publication must be
/// deferred until legitimate admission, not published into a life of refused
/// mutations.
///
/// This test pins the repaired contract through the production registration
/// entry point and through the shared admission decision the ready-respawn path
/// calls: nothing is published while the canonical instance is missing, the
/// built candidate waits in the map's own spawn queue, the gate is therefore
/// never handed an un-admitted representation, and the deferred creature
/// becomes admissible and mutable once the canonical instance legitimately
/// exists. The ready-respawn half of the same contract is covered by
/// `r1b_incarnation_lifecycle::lifecycle_respawn_without_canonical_incarnation_defers_publication_like_cpp`.
#[test]
fn missing_canonical_instance_defers_the_representation_instead_of_publishing_it_like_cpp() {
    let guid = test_creature_guid(91_608);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    session.set_map_manager(manager.clone());
    session.set_player_map_position_like_cpp(0, Position::new(10.0, 10.0, 0.0, 0.0));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    assert!(
        canonical.lock().unwrap().find_map(0, 0).is_none(),
        "the fixture starts without a canonical map instance"
    );

    // Production registration, at exactly the key the legacy store publishes to.
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
    assert!(
        manager.read().unwrap().find_creature(0, 0, guid).is_none(),
        "a configured canonical manager with no instance must not publish a representation it cannot admit"
    );
    assert!(
        canonical_creature_like_cpp(&canonical, guid).is_none(),
        "the missing instance means no canonical incarnation was admitted"
    );
    assert_eq!(
        manager.read().unwrap().respawn_queue_len(0, 0),
        1,
        "the built candidate waits for legitimate admission in the map spawn queue"
    );

    // The gate is therefore never handed an un-admitted representation: there is
    // no legacy representation for this GUID at all, and the callback does not
    // run.
    let mut callback_runs = 0_usize;
    assert!(
        session
            .mutate_world_creature(guid, |creature| {
                callback_runs += 1;
                creature.creature.ai_ownership_mut().npc_flags = 0x0000_00B1;
            })
            .is_none(),
        "a deferred creature has no admitted incarnation to mutate"
    );
    assert_eq!(callback_runs, 0);

    // Eventual admission: once the canonical instance legitimately exists, the
    // production global lifecycle tick publishes the deferred creature with the
    // one admission decision, and the one mutation root admits it.
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(crate::map_manager::RuntimeTickOwner::GlobalLegacy);
    let _ = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        Instant::now(),
    );
    assert_eq!(
        manager.read().unwrap().respawn_queue_len(0, 0),
        0,
        "the deferred publication is no longer waiting"
    );
    let canonical_creature = canonical_creature_like_cpp(&canonical, guid)
        .expect("the incarnation is admitted once the instance exists");
    let legacy_creature = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .map(|world_creature| world_creature.creature.clone())
        .expect("the deferred creature is published once it can be admitted");
    assert!(
        legacy_creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(canonical_creature.loot_authority_like_cpp()),
        "the published representation aliases the admitted incarnation's one allocation"
    );
    assert!(
        session
            .mutate_world_creature(guid, |creature| {
                callback_runs += 1;
                creature.creature.ai_ownership_mut().npc_flags = 0x0000_00B1;
            })
            .is_some(),
        "a creature deferred at registration becomes mutable once it is admitted"
    );
    assert_eq!(callback_runs, 1);

    // The ready-respawn admission makes the same decision for a key the canonical
    // manager has never created, from the one shared admission function.
    let respawn_guid = test_creature_guid(91_609);
    let mut respawn_candidate = admission_candidate_like_cpp(respawn_guid);
    let _ = respawn_candidate.unit_mut().world_mut().set_map(0, 3);
    assert!(
        canonical.lock().unwrap().find_map(0, 3).is_none(),
        "the respawn key has no canonical instance yet"
    );
    assert!(
        insert_canonical_creature_map_object_on_map_like_cpp(&canonical, 0, 3, respawn_candidate)
            .is_none(),
        "the ready-respawn admission declines the same way"
    );
    assert!(
        canonical.lock().unwrap().find_map(0, 3).is_none(),
        "declining publishes no canonical incarnation"
    );
}
