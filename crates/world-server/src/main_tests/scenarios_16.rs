//! Scenarios for [`super`], part 16.
//!
//! #1263 F6-8D3b-1: the `RuntimeTickOwner::CanonicalMap` creature tick owner.
//!
//! The integration regression boots the real canonical map update loop
//! (`spawn_canonical_map_update_loop`) with the canonical creature owner on a
//! fixture with one hostile creature and one player, drives it for real ticks,
//! and reads what reached the player's session rails. The legacy creature loop
//! is spawned **enabled** beside it, as a misconfiguration would, to prove the
//! single owner value keeps it inert: the legacy representation is never
//! ticked, and the canonical creature's clock equals the executor's own count.
//! The same fixture under `GlobalLegacy` is the VALUES-count comparison.

use std::sync::atomic::Ordering;

use super::*;
use crate::runtime::canonical_creature_runtime::{
    CanonicalCreatureRuntimeLikeCpp, CanonicalCreatureRuntimeProbeLikeCpp,
    creature_tick_owner_loops_like_cpp,
};
use wow_world::map_manager::RuntimeTickOwner;

const D3B1_TICK_MS_LIKE_CPP: u32 = 20;
const D3B1_CREATURE_HP_LIKE_CPP: u32 = 20;
const D3B1_PLAYER_HP_LIKE_CPP: u64 = 1_000;
const D3B1_SPAWN_ID_LIKE_CPP: u64 = 96_301;

fn d3b1_aggro_config_like_cpp() -> wow_world::session::LegacyCreatureAggroConfigLikeCpp {
    let template = |id, faction, enemies| wow_data::progression_rewards::FactionTemplateEntry {
        id,
        faction,
        flags: 0,
        faction_group: 0,
        friend_group: 0,
        enemy_group: 0,
        enemies,
        friend: [0; 8],
    };
    wow_world::session::LegacyCreatureAggroConfigLikeCpp {
        faction_template_store: Some(Arc::new(
            wow_data::progression_rewards::FactionTemplateStore::from_entries([
                template(14, 72, [930, 0, 0, 0, 0, 0, 0, 0]),
                template(1, 930, [0; 8]),
            ]),
        )),
        faction_store: Some(Arc::new(
            wow_data::progression_rewards::FactionStore::from_entries([
                wow_data::progression_rewards::FactionEntry::for_test_like_cpp(72, 1),
            ]),
        )),
        ..Default::default()
    }
}

/// One hostile persistent-spawn creature, admitted on both stores as
/// production registers it, and one player that swings at it.
struct D3b1WorldLikeCpp {
    legacy: wow_world::SharedMapManager,
    canonical: wow_world::SharedCanonicalMapManager,
    registry: Arc<PlayerRegistry>,
    player: ObjectGuid,
    creature: ObjectGuid,
    command_rx: flume::Receiver<SessionCommand>,
    writer: RespawnDbWriterSenderLikeCpp,
}

fn d3b1_world_like_cpp(base: i64, owner: RuntimeTickOwner) -> D3b1WorldLikeCpp {
    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);
    let creature = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, base);
    let player = ObjectGuid::create_player(1, base + 1);
    let creature_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let player_position = Position::new(11.0, 10.0, 0.0, std::f32::consts::PI);

    let mut world_creature = wow_world::map_manager::WorldCreature::new(
        creature,
        9001,
        creature_position,
        D3B1_CREATURE_HP_LIKE_CPP,
        2,
        3,
        5,
        20.0,
        100,
        14,
        0,
        0,
    );
    world_creature.creature.set_spawn_id(D3B1_SPAWN_ID_LIKE_CPP);
    world_creature.creature.set_corpse_delay(1, false);
    world_creature.creature.set_respawn_delay(1);
    world_creature.creature.ai_ownership_mut().respawn_time_secs = 1;
    world_creature.creature.ai_ownership_mut().swing_timer_ms = 0;
    world_creature.seed_runtime_rng_like_cpp(0xD3B1);
    let mut canonical_creature = world_creature.creature.clone();
    canonical_creature
        .unit_mut()
        .world_mut()
        .set_map(0, 0)
        .unwrap();
    // `Map::AddToMap` puts the creature in its grid cell, so the `NearbyCells`
    // selection sees it, as production admission does.
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_creature(canonical_creature).unwrap(),
        )
        .unwrap();
    {
        let mut manager = legacy.write().unwrap();
        manager.add_creature(
            0,
            0,
            wow_world::map_manager::world_to_grid_x(creature_position.x),
            wow_world::map_manager::world_to_grid_y(creature_position.y),
            world_creature,
        );
        manager.set_tick_owner(owner);
    }

    {
        let mut canonical_player = Player::new(Some(1), false);
        let unit = canonical_player.unit_mut();
        unit.world_mut().object_mut().create(player);
        unit.world_mut().set_name("D3b1Player");
        unit.world_mut().set_map(0, 0).unwrap();
        unit.world_mut().relocate(player_position);
        unit.set_level(80);
        unit.set_faction(1);
        unit.set_max_health(D3B1_PLAYER_HP_LIKE_CPP);
        unit.set_health(D3B1_PLAYER_HP_LIKE_CPP);
        canonical
            .lock()
            .unwrap()
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .add_map_object_record_to_map_like_cpp(
                MapObjectRecord::new_player(canonical_player).unwrap(),
            )
            .unwrap();
    }
    {
        let mut guard = canonical.lock().unwrap();
        let map = guard.find_map_mut(0, 0).unwrap().map_mut();
        let unit = map.get_typed_player_mut(player).unwrap().unit_mut();
        unit.set_death_state(wow_constants::DeathState::Alive);
        unit.subsystems_mut()
            .auras
            .set_spell_hit_aura_authority_inert_like_cpp(true);
        unit.set_attacking(Some(creature));
        unit.set_target(creature);
        unit.add_unit_state(wow_constants::UnitState::MELEE_ATTACKING.bits());
        unit.set_base_attack_time_like_cpp(wow_constants::WeaponAttackType::BaseAttack, 300);
        unit.set_weapon_damage(wow_constants::WeaponAttackType::BaseAttack, 10.0, 10.0);
        unit.reset_attack_timer_like_cpp(wow_constants::WeaponAttackType::BaseAttack);
    }

    let registry = Arc::new(PlayerRegistry::default());
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    let (send_tx, _send_rx) = flume::bounded(1024);
    let (command_tx, command_rx) = flume::bounded(1024);
    let mut info = player_registration_fixture_like_cpp(send_tx, command_tx, "D3b1Player");
    info.placement.map_id = 0;
    info.placement.instance_id = 0;
    info.placement.position = player_position;
    info.placement.is_in_world = true;
    info.client_visible_guids_like_cpp.insert(creature);
    registry.register_or_replace(player, info, Default::default());

    D3b1WorldLikeCpp {
        legacy,
        canonical,
        registry,
        player,
        creature,
        command_rx,
        writer: RespawnDbWriterSenderLikeCpp::new_like_cpp(),
    }
}

/// What reached the player's rails and the respawn writer over one run.
#[derive(Debug, Default)]
struct D3b1ObservedLikeCpp {
    attack_starts: usize,
    creature_melee: usize,
    player_melee_results: usize,
    kills: usize,
    refreshes: usize,
    creature_values_updates: usize,
    db_mutations: Vec<RespawnPersistenceMutationLikeCpp>,
    respawned: bool,
}

impl D3b1ObservedLikeCpp {
    fn record_like_cpp(&mut self, creature: ObjectGuid, command: SessionCommand) {
        match command {
            SessionCommand::CreatureAttackStartLikeCpp(_) => self.attack_starts += 1,
            SessionCommand::ApplyCreatureMeleeDamageLikeCpp(_) => self.creature_melee += 1,
            SessionCommand::ApplyPlayerMeleeResultLikeCpp(result) => {
                self.player_melee_results += 1;
                self.kills += usize::from(result.killed_creature.is_some());
            }
            SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(_) => self.refreshes += 1,
            SessionCommand::SendVisibleObjectValuesUpdate(update)
                if update.object_guid == creature =>
            {
                self.creature_values_updates += 1;
            }
            _ => {}
        }
    }
}

/// Boot the canonical map loop (with the canonical creature owner when
/// `owner` selects it) and the legacy creature loop, drive them until the
/// creature has died and respawned or `deadline` passes, and return what the
/// player and the writer observed, plus the executor probe.
async fn d3b1_run_like_cpp(
    world: &D3b1WorldLikeCpp,
    owner: RuntimeTickOwner,
    legacy_loop_enabled: bool,
    deadline: Duration,
    mut on_first_creature_melee: impl FnMut(&D3b1WorldLikeCpp, &CanonicalCreatureRuntimeProbeLikeCpp),
) -> (
    D3b1ObservedLikeCpp,
    Arc<CanonicalCreatureRuntimeProbeLikeCpp>,
) {
    let loops = creature_tick_owner_loops_like_cpp(owner);
    let runtime = loops.canonical_creature_runtime.then(|| {
        CanonicalCreatureRuntimeLikeCpp::new_like_cpp(
            wow_world::MMapRuntimeConfigLikeCpp {
                enabled: false,
                ..Default::default()
            },
            None,
            d3b1_aggro_config_like_cpp(),
            None,
        )
    });
    let probe = runtime.as_ref().map_or_else(
        || Arc::new(CanonicalCreatureRuntimeProbeLikeCpp::default()),
        |runtime| Arc::clone(&runtime.probe),
    );
    let respawn_db_mutation_order = Arc::new(Mutex::new(()));
    let producer_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let map_store = Arc::new(legacy_runtime_world_map_store_like_cpp());
    let canonical_handle = crate::runtime::spawn_canonical_map_update_loop(
        Arc::clone(&world.canonical),
        Arc::clone(&world.legacy),
        D3B1_TICK_MS_LIKE_CPP,
        60_000,
        Arc::new(Mutex::new(
            spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
                SpawnStore::new(),
                BTreeMap::new(),
            ),
        )),
        Arc::new(wow_data::ConditionEntriesByTypeStore::default()),
        Arc::clone(&map_store),
        Arc::new(FakeGameEventPersistencePortLikeCpp::default()),
        world.writer.clone(),
        Arc::clone(&respawn_db_mutation_order),
        Arc::clone(&producer_stop),
        empty_loaded_grid_creature_respawn_caches_like_cpp(),
        Arc::new(wow_data::AreaTriggerTemplateStore::default()),
        CanonicalGameEventSchedulerLikeCpp::start_system(3_600_000),
        Arc::clone(&world.registry),
        Arc::new(ActiveWorldSessionRegistryLikeCpp::new()),
        Arc::new(wow_data::BattlemasterListStore::from_entries([])),
        Arc::new(Mutex::new(Default::default())),
        runtime,
    );
    let legacy_handle = spawn_legacy_creature_runtime_update_loop_like_cpp(
        legacy_loop_enabled,
        Arc::clone(&world.legacy),
        Arc::clone(&world.canonical),
        map_store,
        wow_world::MMapRuntimeConfigLikeCpp {
            enabled: false,
            ..Default::default()
        },
        None,
        d3b1_aggro_config_like_cpp(),
        D3B1_TICK_MS_LIKE_CPP,
        Some(world.writer.clone()),
        respawn_db_mutation_order,
        producer_stop,
        None,
        Arc::clone(&world.registry),
    );

    let mut observed = D3b1ObservedLikeCpp::default();
    let mut seen_creature_melee = false;
    let mut death_completed = false;
    let started = Instant::now();
    while started.elapsed() < deadline && !observed.respawned {
        tokio::time::sleep(Duration::from_millis(u64::from(D3B1_TICK_MS_LIKE_CPP))).await;
        // A dropped map-pass request is revoked by its coordinator, so the
        // tick continues without a session pass.
        for command in world.command_rx.try_iter() {
            observed.record_like_cpp(world.creature, command);
        }
        for command in
            drain_durable_creature_runtime_commands_like_cpp(&world.registry, world.player)
        {
            observed.record_like_cpp(world.creature, command);
        }
        if observed.creature_melee > 0 && !seen_creature_melee {
            seen_creature_melee = true;
            on_first_creature_melee(world, &probe);
        }
        if observed.kills > 0 && !death_completed {
            death_completed = true;
            // The kill hooks and `setDeathState(JUST_DIED)` are the attacker
            // session's (`process_pending_creature_kills_*` →
            // `complete_represented_creature_death_state_after_kill_hooks_like_cpp`),
            // through the legacy-first mutation root. Under `CanonicalMap` that
            // root refuses the stale legacy representation (F6-8D3b-2 inverts
            // it), so the test completes the death on the canonical
            // incarnation, as the canonical-first root will.
            let mut guard = world.canonical.lock().unwrap();
            guard
                .find_map_mut(0, 0)
                .unwrap()
                .map_mut()
                .with_creature_mut_like_cpp(world.creature, |creature| {
                    creature.complete_death_state_after_kill_hooks_like_cpp();
                })
                .expect("the killed creature is on the canonical map");
        }
        // The test is the writer: it takes every due statement in order.
        loop {
            let attempt = world
                .writer
                .mailbox
                .state
                .lock()
                .unwrap()
                .queue
                .take_due(Instant::now() + Duration::from_secs(60));
            let Some(attempt) = attempt else {
                break;
            };
            observed.db_mutations.push(attempt.pending.mutation);
        }
        observed.respawned = observed.kills > 0
            && observed.db_mutations.iter().any(|mutation| {
                matches!(mutation, RespawnPersistenceMutationLikeCpp::Delete { .. })
            })
            && world
                .canonical
                .lock()
                .unwrap()
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_typed_creature(world.creature)
                .is_some_and(|creature| creature.is_alive());
    }
    canonical_handle.abort();
    legacy_handle.abort();
    let _ = canonical_handle.await;
    let _ = legacy_handle.await;
    (observed, probe)
}

/// The legacy representation's runtime clock: it moves only when a legacy
/// phase body ticks it.
fn d3b1_legacy_clock_like_cpp(world: &D3b1WorldLikeCpp) -> Option<u64> {
    world
        .legacy
        .read()
        .unwrap()
        .find_creature(0, 0, world.creature)
        .map(|creature| creature.creature.runtime_elapsed_ms_like_cpp())
}

#[tokio::test]
async fn canonical_map_owner_runs_the_creature_lifecycle_alone_and_delivers_to_the_session_like_cpp()
 {
    let world = d3b1_world_like_cpp(96_300, RuntimeTickOwner::CanonicalMap);
    let mut clock_check = None;
    let (observed, probe) = d3b1_run_like_cpp(
        &world,
        RuntimeTickOwner::CanonicalMap,
        // Spawned enabled on purpose: the owner value alone keeps it inert.
        true,
        Duration::from_secs(8),
        |world, probe| {
            // Read under the canonical guard, which the executor holds while it
            // advances the clock and the probe: both are this tick's values.
            let guard = world.canonical.lock().unwrap();
            let creature_clock = guard
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_typed_creature(world.creature)
                .map(|creature| creature.runtime_elapsed_ms_like_cpp());
            clock_check = Some((
                creature_clock,
                probe.clock_advanced_ms.load(Ordering::Relaxed),
                d3b1_legacy_clock_like_cpp(world),
            ));
            drop(guard);
        },
    )
    .await;

    assert!(
        observed.respawned,
        "the creature died and respawned: {observed:?}"
    );
    assert!(
        observed.attack_starts >= 1,
        "aggro reached the victim: {observed:?}"
    );
    assert!(
        observed.creature_melee >= 1,
        "creature melee reached the victim: {observed:?}"
    );
    assert!(
        observed.kills >= 1,
        "the kill reached the attacker: {observed:?}"
    );
    assert!(
        observed.refreshes >= 2,
        "despawn and respawn refreshed the map: {observed:?}"
    );
    let kinds: Vec<_> = observed
        .db_mutations
        .iter()
        .map(|mutation| match mutation {
            RespawnPersistenceMutationLikeCpp::Save { key, .. } => ("save", key.spawn_id),
            RespawnPersistenceMutationLikeCpp::Delete { key } => ("delete", key.spawn_id),
        })
        .collect();
    assert_eq!(
        kinds,
        [
            ("save", D3B1_SPAWN_ID_LIKE_CPP),
            ("delete", D3B1_SPAWN_ID_LIKE_CPP)
        ],
        "JUST_DIED save, then the respawn delete, through the shared writer"
    );

    let executed = probe.executed_ticks.load(Ordering::Relaxed);
    assert!(
        executed >= 10,
        "the executor ran every admitted tick: {executed}"
    );
    assert_eq!(probe.owner_skipped_ticks.load(Ordering::Relaxed), 0);

    // No double tick: the creature's own clock is exactly the executor's
    // advancement, and the legacy representation was never ticked although
    // the legacy loop was running.
    let (creature_clock, probe_clock, legacy_clock) =
        clock_check.expect("creature melee was observed");
    assert_eq!(creature_clock, Some(probe_clock));
    assert_eq!(
        legacy_clock,
        Some(0),
        "the legacy loop never ticked the creature"
    );
    assert_eq!(d3b1_legacy_clock_like_cpp(&world), Some(0));
}

/// Drive `ticks` admitted canonical ticks by hand, with the creature owner
/// `owner` selects, and count the creature's Unit VALUES snapshots each
/// `Map::SendObjectUpdates` captured (the session-side P3.10 adapter turns each
/// into one `SMSG_UPDATE_OBJECT` per viewer).
fn d3b1_values_per_tick_like_cpp(owner: RuntimeTickOwner, ticks: usize) -> Vec<usize> {
    let world = d3b1_world_like_cpp(96_320, owner);
    d3b1_drive_ticks_like_cpp(&world, owner, ticks).0
}

/// Drive `ticks` admitted canonical ticks by hand on `world`; returns the
/// creature's VALUES snapshots per tick and the executor probe.
fn d3b1_drive_ticks_like_cpp(
    world: &D3b1WorldLikeCpp,
    owner: RuntimeTickOwner,
    ticks: usize,
) -> (Vec<usize>, Arc<CanonicalCreatureRuntimeProbeLikeCpp>) {
    let mut runtime = creature_tick_owner_loops_like_cpp(owner)
        .canonical_creature_runtime
        .then(|| {
            CanonicalCreatureRuntimeLikeCpp::new_like_cpp(
                wow_world::MMapRuntimeConfigLikeCpp {
                    enabled: false,
                    ..Default::default()
                },
                None,
                d3b1_aggro_config_like_cpp(),
                None,
            )
        });
    let map_store = legacy_runtime_world_map_store_like_cpp();
    let metadata =
        spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new());
    let conditions = wow_data::ConditionEntriesByTypeStore::default();
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(60_000);
    let player_melee_state = Arc::new(Mutex::new(Default::default()));
    let mut per_tick = Vec::new();
    for _ in 0..ticks {
        let now = Instant::now();
        if owner == RuntimeTickOwner::GlobalLegacy {
            let _ = run_legacy_creature_runtime_tick_and_deliver_once_like_cpp(
                &world.legacy,
                Some(&world.canonical),
                &map_store,
                &wow_world::MMapRuntimeConfigLikeCpp {
                    enabled: false,
                    ..Default::default()
                },
                None,
                d3b1_aggro_config_like_cpp(),
                100,
                now,
                &world.registry,
                None,
                None,
                None,
                &player_melee_state,
            );
        }
        let inputs = runtime.as_ref().and_then(|runtime| {
            runtime.collect_inputs_like_cpp(&world.legacy, &world.canonical, &world.registry)
        });
        let mut manager = world.canonical.lock().unwrap();
        let plan = crate::runtime::map_tick::canonical_map_tick_begin_like_cpp(&mut manager, 100)
            .expect("the fixture map is due every 100 ms");
        let mut creature_phase =
            |manager: &mut wow_map::MapManager, plan: &wow_map::MapTickPlanLikeCpp| {
                if let (Some(runtime), Some(inputs)) = (runtime.as_mut(), inputs.as_ref()) {
                    let _ = runtime.run_on_locked_manager_like_cpp(
                        manager,
                        plan,
                        0x0D3B_1000,
                        inputs,
                        &world.legacy,
                        &map_store,
                        now,
                    );
                }
            };
        let _ = crate::runtime::map_tick::canonical_map_tick_resume_with_creature_owner_like_cpp(
            &mut manager,
            Some(&world.legacy),
            plan.plan,
            &mut scheduler,
            &metadata,
            &conditions,
            &map_store,
            &caches,
            Some(&mut creature_phase),
        );
        per_tick.push(
            manager
                .find_map(0, 0)
                .unwrap()
                .last_send_object_updates_summary_like_cpp()
                .unit_values_updates
                .iter()
                .filter(|update| update.guid == world.creature)
                .count(),
        );
    }
    let probe = runtime.as_ref().map_or_else(
        || Arc::new(CanonicalCreatureRuntimeProbeLikeCpp::default()),
        |runtime| Arc::clone(&runtime.probe),
    );
    (per_tick, probe)
}

#[test]
fn canonical_map_owner_publishes_no_more_values_than_the_legacy_owner_like_cpp() {
    // 30 ticks of 100 ms: aggro, the creature's swings, the player's two
    // swings and the kill, then a corpse whose values never change again.
    let canonical = d3b1_values_per_tick_like_cpp(RuntimeTickOwner::CanonicalMap, 30);
    let legacy = d3b1_values_per_tick_like_cpp(RuntimeTickOwner::GlobalLegacy, 30);
    let canonical_total: usize = canonical.iter().sum();
    let legacy_total: usize = legacy.iter().sum();
    assert!(
        canonical_total > 0,
        "the fight changed the creature: {canonical:?}"
    );
    // R6 flood: unchanged values are never republished, so most ticks carry
    // no snapshot and the tail after the kill carries none at all.
    assert!(
        canonical.iter().filter(|&&count| count > 0).count() * 3 < canonical.len(),
        "values are published on change only: {canonical:?}"
    );
    assert!(
        canonical[20..].iter().all(|&count| count == 0),
        "{canonical:?}"
    );
    assert!(
        canonical_total <= legacy_total,
        "the canonical owner publishes no more VALUES than the legacy owner: canonical {canonical:?}, legacy {legacy:?}"
    );
}

#[test]
fn creature_tick_owner_stays_global_legacy_unless_canonical_map_is_selected_like_cpp() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    let owner_for = |config: &str| {
        wow_config::load_config_from_str(config).expect("config should load");
        crate::creature_tick_owner_from_config_like_cpp()
    };

    assert_eq!(RuntimeTickOwner::default(), RuntimeTickOwner::Session);
    assert_eq!(owner_for(""), RuntimeTickOwner::GlobalLegacy);
    assert_eq!(
        owner_for("RustyCore.CanonicalCreatureRuntime = 0\n"),
        RuntimeTickOwner::GlobalLegacy
    );
    assert_eq!(
        owner_for("RustyCore.CanonicalCreatureRuntime = 1\n"),
        RuntimeTickOwner::CanonicalMap
    );
    assert_eq!(
        owner_for(
            "RustyCore.CanonicalCreatureRuntime = 1\nRustyCore.LegacyCreatureGlobalRuntime = 1\n"
        ),
        RuntimeTickOwner::CanonicalMap
    );
    assert_eq!(
        owner_for("RustyCore.LegacyCreatureGlobalRuntime = 0\n"),
        RuntimeTickOwner::Session
    );
    wow_config::load_config_from_str("").expect("config should load");

    // One owner value, one live creature loop.
    for (owner, legacy_loop, canonical_runtime) in [
        (RuntimeTickOwner::Session, false, false),
        (RuntimeTickOwner::GlobalLegacy, true, false),
        (RuntimeTickOwner::CanonicalMap, false, true),
    ] {
        let loops = creature_tick_owner_loops_like_cpp(owner);
        assert_eq!(loops.legacy_creature_loop, legacy_loop, "{owner:?}");
        assert_eq!(
            loops.canonical_creature_runtime, canonical_runtime,
            "{owner:?}"
        );
    }
}

#[test]
fn canonical_map_owner_adopts_r7b1_deferred_registrations_from_the_legacy_queue_like_cpp() {
    // R7b-1: a registration deferred while the canonical manager had no
    // instance for its key waits in the legacy map queue, which no legacy
    // lifecycle drains under `CanonicalMap`. The admitted map adopts it.
    let world = d3b1_world_like_cpp(96_340, RuntimeTickOwner::CanonicalMap);
    let deferred_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9002, 96_349);
    let deferred = wow_world::map_manager::WorldCreature::new(
        deferred_guid,
        9002,
        Position::new(30.0, 30.0, 0.0, 0.0),
        D3B1_CREATURE_HP_LIKE_CPP,
        2,
        3,
        5,
        20.0,
        100,
        14,
        0,
        0,
    );
    world.legacy.write().unwrap().push_respawn(
        0,
        0,
        wow_world::map_manager::pending_respawn_from_world_creature_like_cpp(
            &deferred,
            Instant::now(),
            0,
        ),
    );

    let (_, probe) = d3b1_drive_ticks_like_cpp(&world, RuntimeTickOwner::CanonicalMap, 3);

    assert_eq!(probe.adopted_legacy_respawns.load(Ordering::Relaxed), 1);
    assert_eq!(world.legacy.read().unwrap().respawn_queue_len(0, 0), 0);
    let canonical = world.canonical.lock().unwrap();
    let map = canonical.find_map(0, 0).unwrap().map();
    assert!(
        map.typed_combat_unit_guids_like_cpp()
            .into_iter()
            .any(|guid| {
                map.get_typed_creature(guid)
                    .is_some_and(|creature| creature.entry() == 9002 && creature.is_alive())
            }),
        "the deferred creature was admitted on the canonical map"
    );
    assert_eq!(map.creature_respawn_queue_like_cpp().respawn_queue_len(), 0);
}

#[test]
fn respawn_entry_carries_the_template_speed_rates_and_scales_like_cpp() {
    // C++ `Creature::InitEntry` (`Creature.cpp:547-553`) re-applies the
    // template walk/run rates and the native object scale on every respawn.
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9003, 96_360);
    let mut world_creature = wow_world::map_manager::WorldCreature::new(
        guid,
        9003,
        Position::new(10.0, 10.0, 0.0, 0.0),
        D3B1_CREATURE_HP_LIKE_CPP,
        2,
        3,
        5,
        20.0,
        100,
        14,
        0,
        0,
    );
    let mut projection =
        wow_entities::creature_create::CreatureRespawnCreateProjectionLikeCpp::from_create_data_like_cpp(
            &world_creature.create_data,
        );
    projection.speed_walk_rate = 0.8;
    projection.speed_run_rate = 1.5;
    projection.display_scale = 1.25;
    projection.native_x_display_scale = 1.1;
    projection.scale = 2.0;
    world_creature
        .creature
        .runtime_like_cpp_mut()
        .set_respawn_create_projection_like_cpp(projection);

    let pending = wow_world::map_manager::pending_respawn_from_world_creature_like_cpp(
        &world_creature,
        Instant::now(),
        0,
    );
    assert_eq!(pending.create_data.speed_walk_rate, 0.8);
    assert_eq!(pending.create_data.speed_run_rate, 1.5);
    assert_eq!(pending.create_data.display_scale, 1.25);
    assert_eq!(pending.create_data.native_x_display_scale, 1.1);
    assert_eq!(pending.create_data.scale, 2.0);
    let rebuilt = wow_world::map_manager::creature_from_pending_respawn_like_cpp(&pending, 0);
    assert_eq!(rebuilt.unit().world().object().scale(), 2.0);
    let rates = rebuilt.unit().speed_rate();
    assert_eq!(rates[wow_constants::UnitMoveType::Run as usize], 1.5);
}

#[test]
fn session_owned_creature_lifecycle_is_unreachable_under_canonical_map_like_cpp() {
    // The session-owned corpse/respawn body (`run_creatures_tick`) has one
    // caller, `tick_creatures_sync`, whose one production caller is the session
    // driver, gated on the `Session` owner.
    let ticks = include_str!("../../../wow-world/src/session/spell_effects/ticks.rs");
    assert_eq!(ticks.matches("self.run_creatures_tick()").count(), 1);
    assert!(ticks.contains(
        "fn tick_creatures_sync(&mut self) {\n        let out = self.run_creatures_tick();"
    ));
    let driver = include_str!("../../../wow-world/src/session/driver/mod.rs");
    assert_eq!(driver.matches("tick_creatures_sync()").count(), 1);
    let call = driver.find("self.tick_creatures_sync()").unwrap();
    assert!(
        driver[call.saturating_sub(160)..call].contains("&& owner == RuntimeTickOwner::Session")
    );
}
