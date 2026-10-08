//! F6-7 R1b regressions: one incarnation per coexisting creature representation.
//!
//! Reviewer signature §5.3.1 R1 and repair order §5.3.2. R1a admitted one loot
//! authority per creature incarnation at registration; R1b seals the remaining
//! construction, publication and rebinding paths:
//!
//! * every coexisting representation must adopt the admitted incarnation's
//!   authority **and** health timeline before it can become claimable, so the
//!   respawn rail consumes R1a's authority/health/provenance result together;
//! * synchronization and rebinding across incarnations, or from a competing used
//!   allocation, is rejected before any authority is selected;
//! * a refusal publishes nothing, so no competing alias can become claimable;
//! * R7b-1: a canonical manager without a map instance for the key provides no
//!   admitted owner, so a ready respawn is **deferred** rather than published
//!   into a life of refused mutations, and is published with the same admission
//!   decision once the instance legitimately exists.
//!
//! These tests drive the production consumers — the global lifecycle tick
//! (`run_legacy_creature_lifecycle_tick_once_like_cpp`), the global creature
//! melee tick, the global movement tick and the map-owned player melee tick —
//! rather than a test double.

use super::*;

fn test_position_like_cpp() -> Position {
    Position::new(10.0, 10.0, 0.0, 0.0)
}

fn corpse_loot_like_cpp(guid: ObjectGuid, coins: u32, player: ObjectGuid) -> CreatureLoot {
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

/// The canonical creature for `guid`, if any.
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

/// The legacy creature for `guid`, if any.
fn legacy_creature_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    guid: ObjectGuid,
) -> Option<wow_entities::Creature> {
    manager
        .read()
        .ok()?
        .find_creature(0, 0, guid)
        .map(|world_creature| world_creature.creature.clone())
}

/// Queue one ready respawn for `guid` and run the production global lifecycle
/// tick once.
fn run_respawn_tick_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    canonical: Option<&SharedCanonicalMapManager>,
    guid: ObjectGuid,
    hp: u32,
    now: Instant,
) -> crate::session::LegacyCreatureLifecycleTickOutcomeLikeCpp {
    use crate::map_manager::{RuntimeTickOwner, pending_respawn_from_world_creature_like_cpp};

    let queued = crate::map_manager::WorldCreature::new(
        guid,
        9001,
        test_position_like_cpp(),
        hp,
        8,
        9,
        13,
        20.0,
        105,
        14,
        0,
        0,
    );
    let pending = pending_respawn_from_world_creature_like_cpp(&queued, now, 0);
    {
        let mut guard = manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.set_tick_owner(RuntimeTickOwner::GlobalLegacy);
        guard.push_respawn(0, 0, pending);
    }
    run_legacy_creature_lifecycle_tick_once_like_cpp(
        manager,
        canonical,
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now + Duration::from_secs(1),
    )
}

/// The pristine-duplicate respawn: a canonical incarnation for the GUID already
/// exists while the legacy store has none. This is the shape the reviewer named
/// as the likeliest silent break — authority aliases agree but health timelines
/// differ, so later valid synchronization is rejected.
fn setup_pristine_duplicate_respawn_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    canonical_health: u64,
    respawn_health: u32,
    now: Instant,
) -> crate::session::LegacyCreatureLifecycleTickOutcomeLikeCpp {
    add_canonical_test_creature_on_map(canonical, guid, 9001, test_position_like_cpp(), 0, 0, 0);
    {
        let mut guard = canonical.lock().unwrap();
        let typed = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(guid)
            .expect("the canonical incarnation pre-exists");
        typed.unit_mut().set_level(80);
        typed.unit_mut().set_max_health(canonical_health);
        typed.unit_mut().set_health(canonical_health);
    }
    run_respawn_tick_like_cpp(manager, Some(canonical), guid, respawn_health, now)
}

#[test]
fn lifecycle_respawn_pristine_duplicate_adopts_the_admitted_incarnation_like_cpp() {
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let now = Instant::now();
    let guid = test_creature_guid(91_300);
    let player = ObjectGuid::create_player(1, 91_300);

    let outcome = setup_pristine_duplicate_respawn_like_cpp(
        &manager,
        &canonical,
        guid,
        137,
        105,
        now - Duration::from_secs(1),
    );
    assert_eq!(outcome.respawns_processed, 1);
    assert_eq!(outcome.canonical_inserts, 1);

    let canonical_creature = canonical_creature_like_cpp(&canonical, guid).expect("canonical");
    let legacy_creature =
        legacy_creature_like_cpp(&manager, guid).expect("the respawn is published in legacy");

    assert!(
        legacy_creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(canonical_creature.loot_authority_like_cpp()),
        "a pristine duplicate respawn must alias the admitted authority, not allocate a second pool"
    );
    assert!(
        legacy_creature
            .unit()
            .shares_health_state_revision_authority_like_cpp(
                &canonical_creature
                    .unit()
                    .health_state_revision_authority_like_cpp(),
            ),
        "the respawn must adopt the admitted incarnation's health timeline, not only its authority"
    );
    assert_eq!(
        legacy_creature.unit().data().health,
        137,
        "the published representation carries the admitted incarnation's health tuple"
    );
    assert_eq!(legacy_creature.unit().data().max_health, 137);

    // One claimable pool: loot opened through the canonical allocation is the
    // pool the legacy representation serves.
    {
        let mut guard = canonical.lock().unwrap();
        guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(guid)
            .unwrap()
            .initialize_shared_loot_authority_like_cpp(corpse_loot_like_cpp(guid, 44, player));
    }
    let legacy_creature = legacy_creature_like_cpp(&manager, guid).unwrap();
    assert_eq!(
        legacy_creature
            .loot_authority_like_cpp()
            .shared_snapshot_like_cpp()
            .expect("the admitted pool is openable through the legacy representation")
            .loot
            .coins,
        44
    );
}

#[test]
fn lifecycle_respawn_fresh_incarnation_allocates_one_incarnation_like_cpp() {
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let now = Instant::now();
    let guid = test_creature_guid(91_301);
    let player = ObjectGuid::create_player(1, 91_301);

    let outcome = run_respawn_tick_like_cpp(
        &manager,
        Some(&canonical),
        guid,
        105,
        now - Duration::from_secs(1),
    );
    assert_eq!(outcome.respawns_processed, 1);
    assert_eq!(outcome.canonical_inserts, 1);

    let canonical_creature = canonical_creature_like_cpp(&canonical, guid).expect("canonical");
    let legacy_creature = legacy_creature_like_cpp(&manager, guid).expect("legacy");
    assert!(
        legacy_creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(canonical_creature.loot_authority_like_cpp()),
        "a new incarnation may allocate, and both representations must alias that one allocation"
    );
    assert!(
        legacy_creature
            .unit()
            .shares_health_state_revision_authority_like_cpp(
                &canonical_creature
                    .unit()
                    .health_state_revision_authority_like_cpp(),
            ),
        "the clone published by the respawn shares one health timeline with the canonical object"
    );
    assert_eq!(legacy_creature.unit().data().health, 105);

    let canonical_authority = canonical_creature.loot_authority_like_cpp().clone();
    canonical_authority.initialize_shared_like_cpp(corpse_loot_like_cpp(guid, 17, player));
    assert_eq!(
        canonical_authority
            .shared_snapshot_like_cpp()
            .expect("the fresh incarnation owns the pool")
            .loot
            .coins,
        17
    );
    assert_eq!(
        legacy_creature
            .loot_authority_like_cpp()
            .shared_snapshot_like_cpp()
            .expect("the alias reads the same pool")
            .loot
            .coins,
        17
    );
}

#[test]
fn lifecycle_respawn_without_canonical_incarnation_defers_publication_like_cpp() {
    // F6-7 R7b-1 contract. A canonical manager without a canonical map instance
    // for this key provides no admitted owner, so the ready respawn must not
    // publish a legacy representation the R7 gate would refuse for its whole
    // life; the built candidate is held until legitimate admission.
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let now = Instant::now();
    let guid = test_creature_guid(91_302);

    let outcome = run_respawn_tick_like_cpp(
        &manager,
        Some(&canonical),
        guid,
        105,
        now - Duration::from_secs(1),
    );
    assert_eq!(
        outcome.respawns_processed, 0,
        "a respawn with no admitted canonical owner is not published"
    );
    assert_eq!(outcome.canonical_inserts, 0);
    assert!(
        legacy_creature_like_cpp(&manager, guid).is_none(),
        "the deferred respawn publishes no legacy representation"
    );
    assert!(
        canonical_creature_like_cpp(&canonical, guid).is_none(),
        "no canonical incarnation was admitted"
    );
    assert_eq!(
        manager.read().unwrap().respawn_queue_len(0, 0),
        1,
        "the built respawn candidate waits for legitimate admission in the map spawn queue"
    );

    // Eventual admission: once the canonical instance legitimately exists, the
    // next global lifecycle tick publishes the deferred respawn with the same
    // admission decision a ready respawn consumes — one allocation and one health
    // timeline across both representations.
    canonical.lock().unwrap().create_world_map(0, 0);
    let second = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now + Duration::from_secs(2),
    );
    assert_eq!(
        manager.read().unwrap().respawn_queue_len(0, 0),
        0,
        "the deferred respawn is no longer waiting"
    );
    assert_eq!(
        second.respawns_processed, 1,
        "the deferred respawn is published by the rail once it can be admitted"
    );
    let canonical_creature =
        canonical_creature_like_cpp(&canonical, guid).expect("the incarnation is admitted");
    let legacy_creature = legacy_creature_like_cpp(&manager, guid)
        .expect("the deferred respawn is published once it can be admitted");
    assert!(
        legacy_creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(canonical_creature.loot_authority_like_cpp()),
        "the published respawn aliases the admitted incarnation's one allocation"
    );
    assert!(
        legacy_creature
            .unit()
            .shares_health_state_revision_authority_like_cpp(
                &canonical_creature
                    .unit()
                    .health_state_revision_authority_like_cpp(),
            ),
        "the published respawn adopts the admitted incarnation's health timeline"
    );
    assert_eq!(legacy_creature.unit().data().health, 105);
    assert_eq!(
        second.refresh_map_keys,
        vec![(0, 0)],
        "the map key is signalled so sessions recompute visibility"
    );
}

#[test]
fn respawn_adopted_incarnation_accepts_creature_melee_damage_sync_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let now = Instant::now();
    let victim = test_creature_guid(91_303);
    let outcome = setup_pristine_duplicate_respawn_like_cpp(
        &manager,
        &canonical,
        victim,
        137,
        105,
        now - Duration::from_secs(1),
    );
    assert_eq!(outcome.respawns_processed, 1);

    let attacker = test_creature_guid(91_304);
    add_canonical_test_creature_on_map(
        &canonical,
        attacker,
        9002,
        test_position_like_cpp(),
        0,
        0,
        0,
    );
    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    register_test_creature(&mut session, manager.clone(), attacker, 25);
    session
        .mutate_world_creature(attacker, |creature| {
            creature.enter_combat(victim);
            let ai = creature.creature.ai_ownership_mut();
            ai.last_swing_ms = 0;
            ai.swing_timer_ms = 0;
            ai.min_damage = 30;
            ai.max_damage = 30;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let melee = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );
    assert_eq!(melee.swings_ready, 1, "the attacker swings once");
    assert_eq!(melee.canonical_creature_hits, 1);
    assert_eq!(
        melee.legacy_creature_victim_syncs, 1,
        "the damage commit must mirror into the respawned incarnation"
    );
    assert_eq!(
        melee.legacy_creature_victim_sync_cas_rejections, 0,
        "an authority-only respawn adoption would leave another health timeline and reject this sync"
    );

    let canonical_health = canonical_creature_like_cpp(&canonical, victim)
        .unwrap()
        .unit()
        .data()
        .health;
    assert_eq!(canonical_health, 107);
    assert_eq!(
        legacy_creature_like_cpp(&manager, victim)
            .unwrap()
            .unit()
            .data()
            .health,
        canonical_health,
        "the mirrored victim carries the committed canonical damage"
    );
}

#[test]
fn respawn_adopted_incarnation_syncs_movement_to_canonical_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let now = Instant::now();
    let guid = test_creature_guid(91_305);
    let outcome = setup_pristine_duplicate_respawn_like_cpp(
        &manager,
        &canonical,
        guid,
        137,
        105,
        now - Duration::from_secs(1),
    );
    assert_eq!(outcome.respawns_processed, 1);

    {
        let mut guard = manager.write().unwrap();
        guard.set_tick_owner(RuntimeTickOwner::GlobalLegacy);
        let creature = guard.find_creature_mut(0, 0, guid).unwrap();
        creature
            .creature
            .set_default_movement_type_runtime_like_cpp(
                wow_entities::MovementGeneratorType::Random,
            );
        let ai = creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 3.0;
        creature.seed_runtime_rng_like_cpp(0x9130);
        creature.backdate_runtime_clock_for_test(Duration::from_millis(10));
    }

    let mmap_config = MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };
    let movement = run_legacy_creature_movement_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &mmap_config,
        None,
        &HashMap::new(),
        10,
    );
    assert_eq!(movement.canonical_syncs, 1);
    assert_eq!(
        canonical_creature_like_cpp(&canonical, guid)
            .expect("canonical creature")
            .ai_state(),
        wow_entities::CreatureAiState::WalkingRandom,
        "a respawn sharing the admitted health timeline publishes its movement state"
    );
}

#[tokio::test]
async fn respawn_adopted_incarnation_accepts_player_melee_mirror_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let now = Instant::now();
    let victim = test_creature_guid(91_306);
    let outcome = setup_pristine_duplicate_respawn_like_cpp(
        &manager,
        &canonical,
        victim,
        100_000,
        105,
        now - Duration::from_secs(1),
    );
    assert_eq!(outcome.respawns_processed, 1);

    let player_guid = ObjectGuid::create_player(1, 91_306);
    let map_store = Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 0,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::clone(&map_store));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "R1bSolo".to_string(),
        test_position_like_cpp(),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(victim));
            unit.set_target(victim);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 1_000.0, 1_000.0);
            unit.reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
        })
        .unwrap();
    let (send_tx, _rx) = flume::bounded::<Vec<u8>>(8);
    let (command_tx, _crx) = flume::bounded::<SessionCommand>(8);
    let registration = PlayerRegistry::new().register_or_replace(
        player_guid,
        broadcast_info_with_command(player_guid, send_tx, command_tx),
        Default::default(),
    );
    let attackers = vec![crate::session::PlayerMeleeAttackerSnapshotLikeCpp {
        registration,
        player_guid,
        map_id: 0,
        instance_id: 0,
        in_combat_mirror: true,
        tap_group_guids: Vec::new(),
    }];
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();

    let player_melee = crate::session::run_legacy_player_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &attackers,
        2_000,
        &mut phase_state,
        &Default::default(),
    );
    assert_eq!(player_melee.creature_hits, 1, "the player swing resolves");
    assert_eq!(
        player_melee.canonical_mirror_rejections, 0,
        "the respawned victim must share the admitted incarnation, so the mirror is accepted"
    );
}

#[tokio::test]
async fn in_place_respawn_retains_allocation_and_invalidates_old_leases_like_cpp() {
    let manager = shared_map_manager();
    let canonical: SharedCanonicalMapManager =
        Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    canonical.lock().unwrap().create_world_map(0, 0);
    let guid = test_creature_guid(91_307);
    let player = ObjectGuid::create_player(1, 91_307);
    register_test_creature(&mut session, manager.clone(), guid, 25);
    let canonical_authority = canonical_creature_like_cpp(&canonical, guid)
        .expect("fresh admission")
        .loot_authority_like_cpp()
        .clone();
    canonical_authority.initialize_shared_like_cpp(corpse_loot_like_cpp(guid, 60, player));
    let lease = canonical_authority
        .reserve_money_like_cpp(player)
        .await
        .expect("the admitted pool is claimable");
    assert!(lease.shares_authority_like_cpp(&canonical_authority));

    // The object respawns in place: C++ `ClearLoot` retires the lifetime and the
    // next generation installs onto the same object-owned allocation.
    {
        let mut guard = canonical.lock().unwrap();
        let creature = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(guid)
            .expect("canonical creature");
        creature.clear_loot_like_cpp();
        assert_ne!(
            creature.replace_loot_authority_like_cpp(
                Some(corpse_loot_like_cpp(guid, 12, player)),
                HashMap::new(),
            ),
            0,
            "the in-place respawn installs a new lifetime"
        );
    }

    assert!(
        canonical_authority.shares_storage_like_cpp(
            canonical_creature_like_cpp(&canonical, guid)
                .unwrap()
                .loot_authority_like_cpp()
        ),
        "an in-place respawn retains the incarnation's allocation"
    );
    assert_eq!(
        lease.commit_with_snapshot_like_cpp().unwrap_err(),
        wow_loot::LootClaimCommitError::StaleGeneration,
        "a lease taken before the in-place respawn cannot commit into the new lifetime"
    );
    assert_eq!(
        canonical_authority
            .shared_snapshot_like_cpp()
            .expect("the new lifetime is openable")
            .loot
            .coins,
        12
    );
    assert!(
        legacy_creature_like_cpp(&manager, guid)
            .unwrap()
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&canonical_authority),
        "the legacy representation stays an alias of that one allocation across the respawn"
    );
}
