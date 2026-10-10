//! #1263 F6-8D3a-3 regressions: the admitted canonical executor runs the
//! creature lifecycle phase with parity to the legacy bridge.
//!
//! Each fixture is built twice, identically: one persistent-spawn creature on
//! map 0/0, killed before the first tick, and one observer player whose
//! session renders creature visibility. World **L** is driven through the
//! legacy bridge in the world-server loop order — player melee, the lifecycle
//! tick (`run_legacy_creature_lifecycle_tick_once_like_cpp`), the movement
//! phase, aggro with its commit, spell and creature melee — and world **C**
//! through `run_admitted_creature_combat_phases_isolated_like_cpp`, which runs
//! the same lifecycle body over the admitted canonical map and its own
//! respawn queue. After every tick the comparison covers the lifecycle
//! counters, the respawn DB statements and their order, the refresh map keys,
//! the respawn timers (persisted rows, queue length, the canonical map's
//! respawn store), the movement/aggro/melee counters and plans, the creature
//! (legacy representation against the executor's canonical incarnation, and
//! both canonical incarnations), and the bytes the observer session sends when
//! each world's deferred `RefreshVisibleWorldCreaturesLikeCpp` reaches it — the
//! DESTROY of the despawned corpse and the CREATE of the respawn.
//!
//! Masked fields. Respawn times are Unix seconds from the wall clock
//! (`unix_now()` in the lifecycle body, `game_time_secs_like_cpp()` at the
//! kill): each is compared with a one-second tolerance, because the two
//! worlds run a few milliseconds apart and may straddle a second boundary.
//! Every other part of a DB statement (kind, key, order) is exact. The
//! `Instant` due time of a queued respawn is not compared directly; its queue
//! length and persisted row are. The movement `canonical_syncs` count is the
//! legacy bridge's mirror and is masked, as in F6-8D3a-2. The runtime RNG
//! draws of a respawned incarnation are masked: the rebuild installs a fresh
//! runtime whose RNG is seeded from entropy on both sides.

use super::*;

const D3A3_SEED_LIKE_CPP: u64 = 0xD3A3_0001;
const D3A3_SPAWN_ID_LIKE_CPP: u64 = 77_301;
const D3A3_RUN_RATE_LIKE_CPP: f32 = 1.14286;
const D3A3_DIFF_MS_LIKE_CPP: u32 = 500;

struct D3a3WorldLikeCpp {
    manager: crate::map_manager::SharedMapManager,
    canonical: SharedCanonicalMapManager,
    _session: WorldSession,
    observer: WorldSession,
    observer_rx: flume::Receiver<Vec<u8>>,
    creature: ObjectGuid,
    map_store: wow_data::MapStore,
}

/// Build one fixture. Every call with the same arguments yields the same
/// state; `legacy_observer` gives the observer session the legacy map manager
/// as production does (world L), or the canonical map alone (world C).
async fn d3a3_world_like_cpp(base: i64, legacy_observer: bool) -> D3a3WorldLikeCpp {
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature = test_creature_guid(base);
    register_test_creature_mirrored_like_cpp(
        &mut session,
        manager.clone(),
        &canonical,
        creature,
        25,
    );
    session
        .mutate_world_creature(creature, |world_creature| {
            let unit = world_creature.creature.unit_mut();
            unit.set_speed_rate_like_cpp(wow_constants::UnitMoveType::Walk, 1.0);
            unit.set_speed_rate_like_cpp(wow_constants::UnitMoveType::Run, D3A3_RUN_RATE_LIKE_CPP);
            world_creature.seed_runtime_rng_like_cpp(D3A3_SEED_LIKE_CPP);
            world_creature.creature.set_spawn_id(D3A3_SPAWN_ID_LIKE_CPP);
            world_creature.creature.set_corpse_delay(1, false);
            world_creature.creature.set_respawn_delay(2);
            world_creature.creature.ai_ownership_mut().respawn_time_secs = 2;
            // `Unit::Kill` → `setDeathState(JUST_DIED)` → `CORPSE`.
            assert!(world_creature.take_damage(25), "the creature dies");
        })
        .expect("the creature is admitted");
    manager
        .write()
        .unwrap()
        .set_tick_owner(crate::map_manager::RuntimeTickOwner::GlobalLegacy);

    let (mut observer, _, observer_rx) = make_session();
    let observer_guid = ObjectGuid::create_player(1, base + 2);
    if legacy_observer {
        observer.set_map_manager(Arc::clone(&manager));
    }
    observer.set_canonical_map_manager(Arc::clone(&canonical));
    observer.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    observer.attach_player_controller_like_cpp(SessionPlayerController::new(
        observer_guid,
        "D3a3Observer".to_string(),
        Position::new(14.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    observer
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("the observer enters the canonical map");
    observer.apply_move_init_active_mover_complete_like_cpp(0);
    observer.set_state(SessionState::LoggedIn);
    observer.force_update_visibility_like_cpp().await;
    assert!(
        observer
            .core
            .client_visible_guids_like_cpp
            .contains(&creature),
        "the observer sees the corpse before the first tick"
    );
    let _ = drain_server_packet_bytes(&observer_rx);
    D3a3WorldLikeCpp {
        manager,
        canonical,
        _session: session,
        observer,
        observer_rx,
        creature,
        map_store: lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
    }
}

/// One tick's comparable output on either side.
struct D3a3TickLikeCpp {
    lifecycle: crate::session::LegacyCreatureLifecycleTickOutcomeLikeCpp,
    movement: crate::session::LegacyCreatureMovementTickOutcomeLikeCpp,
    aggro: LegacyCreatureAggroTickOutcomeLikeCpp,
    melee: crate::session::LegacyCreatureMeleeTickOutcomeLikeCpp,
    /// The packets the observer session sent for this tick's visibility
    /// refresh commands.
    visibility: Vec<Vec<u8>>,
}

/// Deliver the refresh commands of one tick to the observer, as the
/// world-server delivery does for every in-world session on the map.
async fn d3a3_deliver_refreshes_like_cpp(
    world: &mut D3a3WorldLikeCpp,
    refreshes: Vec<crate::session::mailbox::RefreshVisibleWorldCreaturesLikeCppCommand>,
) -> Vec<Vec<u8>> {
    let catalogs = world.observer.creature_spawn_catalogs_for_test_like_cpp();
    for command in refreshes {
        world
            .observer
            .handle_refresh_visible_world_creatures_with_catalogs_like_cpp_command_like_cpp(
                &catalogs, command,
            )
            .await;
    }
    drain_server_packet_bytes(&world.observer_rx)
}

/// World L: the legacy bridge in the world-server loop order.
async fn d3a3_legacy_tick_like_cpp(
    world: &mut D3a3WorldLikeCpp,
    diff_ms: u32,
    now: Instant,
) -> D3a3TickLikeCpp {
    let config = legacy_aggro_hostile_config_like_cpp();
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let _ = run_legacy_player_melee_tick_once_like_cpp(
        &world.manager,
        Some(&world.canonical),
        &[],
        diff_ms,
        &mut phase_state,
        &config,
    );
    let lifecycle = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &world.manager,
        Some(&world.canonical),
        &world.map_store,
        now,
    );
    let movement = super::f6_8d3a2_movement_lifecycle_parity::d3a2_legacy_movement_tick_like_cpp(
        &world.manager,
        &world.canonical,
        &HashMap::new(),
        diff_ms,
    );
    let mut aggro = run_legacy_creature_aggro_tick_once_with_config_and_canonical_like_cpp(
        &world.manager,
        Some(&world.canonical),
        &[],
        config.clone(),
    );
    {
        let mut guard = world.canonical.lock().unwrap();
        let starts = crate::session::apply_creature_attack_start_commands_on_manager_like_cpp(
            &mut guard,
            &aggro.commands,
        );
        let stops = crate::session::apply_creature_attack_stop_commands_on_manager_like_cpp(
            &mut guard,
            &aggro.stop_commands,
        );
        crate::session::retain_committed_creature_combat_events_like_cpp(
            &mut aggro.plan,
            &aggro.commands,
            &starts,
            &aggro.stop_commands,
            &stops,
        );
    }
    let _ = run_legacy_creature_spell_tick_once_like_cpp(
        &world.manager,
        Some(&world.canonical),
        &config,
    );
    let melee = run_legacy_creature_melee_tick_once_like_cpp(
        &world.manager,
        Some(&world.canonical),
        &config,
    );
    let refreshes = lifecycle
        .refresh_map_keys
        .iter()
        .map(|&(map_id, instance_id)| {
            crate::session::mailbox::RefreshVisibleWorldCreaturesLikeCppCommand {
                map_id,
                instance_id,
            }
        })
        .collect();
    let visibility = d3a3_deliver_refreshes_like_cpp(world, refreshes).await;
    D3a3TickLikeCpp {
        lifecycle,
        movement,
        aggro,
        melee,
        visibility,
    }
}

/// World C: one admitted canonical tick through the executor, then its
/// deferred visibility refresh. Returns the admitted diff the legacy world
/// then replays.
async fn d3a3_executor_tick_like_cpp(
    world: &mut D3a3WorldLikeCpp,
    diff_ms: u32,
    now: Instant,
) -> (
    D3a3TickLikeCpp,
    AdmittedCreatureCombatPhasesOutcomeLikeCpp,
    u32,
) {
    let plan = world
        .canonical
        .lock()
        .unwrap()
        .begin_tick_like_cpp(diff_ms)
        .into_started()
        .expect("the fixture map update timer has passed");
    let key = wow_map::MapKey::new(0, 0);
    let admission = capture_admitted_creature_execution_like_cpp(
        &world.canonical,
        0x0D3A_3000,
        plan.epoch_like_cpp(),
        plan.effective_diff_ms(),
        plan.updated_maps_like_cpp()
            .iter()
            .map(|participant| (participant.key, participant.incarnation)),
        [(key, world.creature)],
    )
    .expect("the canonical owner is readable");
    let config = legacy_aggro_hostile_config_like_cpp();
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let lease = std::sync::Arc::new(std::sync::Mutex::new(CreatureExecutionLeaseLikeCpp::new()));
    let mmap_config = super::f6_8d3a2_movement_lifecycle_parity::d3a2_mmap_config_like_cpp();
    let chase_targets = HashMap::new();
    let outcome = run_admitted_creature_combat_phases_isolated_like_cpp(
        &admission,
        &world.canonical,
        &lease,
        AdmittedCreatureCombatPhaseInputsLikeCpp {
            player_melee_attackers: &[],
            player_melee_phase_state: &mut phase_state,
            aggro_candidates: &[],
            terrain: None,
            mmap_config: &mmap_config,
            mmap_pathfinder: None,
            chase_targets: &chase_targets,
            config: &config,
            map_store: &world.map_store,
            now,
        },
    );
    assert!(outcome.executed_like_cpp(), "{:?}", outcome.admission);
    let _ = world.canonical.lock().unwrap().abandon_tick_like_cpp(plan);
    let visibility =
        d3a3_deliver_refreshes_like_cpp(world, outcome.deferred_visibility_refreshes_like_cpp())
            .await;
    let tick = D3a3TickLikeCpp {
        lifecycle: outcome.lifecycle.clone(),
        movement: outcome.movement.clone(),
        aggro: outcome.aggro.clone(),
        melee: outcome.melee.clone(),
        visibility,
    };
    (tick, outcome, admission.diff_ms)
}

/// The lifecycle counters, without the two lists compared on their own.
fn d3a3_lifecycle_counters_like_cpp(
    outcome: &crate::session::LegacyCreatureLifecycleTickOutcomeLikeCpp,
) -> String {
    format!(
        "skipped={} maps={} creatures={} despawned={} respawned={} canonical_removes={} \
         canonical_inserts={} respawn_adds={} respawn_removes={} refused={} deferred={} failed={}",
        outcome.skipped_owner_not_global,
        outcome.maps_seen,
        outcome.creatures_seen,
        outcome.corpses_despawned,
        outcome.respawns_processed,
        outcome.canonical_removes,
        outcome.canonical_inserts,
        outcome.canonical_respawn_adds,
        outcome.canonical_respawn_removes,
        outcome.respawn_publications_refused_like_cpp,
        outcome.respawn_publications_deferred_like_cpp,
        outcome.respawn_publications_failed_like_cpp,
    )
}

/// One respawn DB statement, split into its exact part and its wall-clock
/// respawn time.
fn d3a3_db_statement_like_cpp(
    mutation: &wow_persistence::RespawnPersistenceMutationLikeCpp,
) -> (String, Option<i64>) {
    match mutation {
        wow_persistence::RespawnPersistenceMutationLikeCpp::Save { key, respawn_time } => {
            (format!("Save {key:?}"), Some(*respawn_time))
        }
        wow_persistence::RespawnPersistenceMutationLikeCpp::Delete { key } => {
            (format!("Delete {key:?}"), None)
        }
    }
}

fn d3a3_times_agree_like_cpp(left: Option<i64>, right: Option<i64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.abs_diff(right) <= 1,
        (None, None) => true,
        _ => false,
    }
}

/// Every lifecycle-relevant creature-owned observable.
fn d3a3_creature_state_like_cpp(creature: Option<&wow_entities::Creature>) -> String {
    let Some(creature) = creature else {
        return "absent".to_string();
    };
    let unit = creature.unit();
    // The fixture seeds the killed incarnation's runtime RNG. A respawned
    // incarnation is a fresh `CreatureRuntimeLikeCpp` seeded from entropy on
    // both sides (as the legacy rebuild always was; C++ draws from the global
    // `urand`), so its draws are masked.
    let draws: Vec<_> = if unit.death_state() == wow_constants::DeathState::Alive {
        Vec::new()
    } else {
        let mut probe = creature.clone();
        (0..4).map(|_| probe.roll_damage()).collect()
    };
    let speed = unit.speed_rate();
    format!(
        "draws={draws:?} elapsed={} death={:?} ai={:?} position={:?} home={:?} health={}/{} \
         power={:?}/{:?} class={} display_power={} spawn={} speed_walk={} speed_run={} \
         flags={:?} generator={:?} victim={:?} aura_authority={:?}",
        creature.runtime_elapsed_ms_like_cpp(),
        unit.death_state(),
        creature.ai_ownership().state,
        creature.position(),
        creature.home_position(),
        unit.data().health,
        unit.data().max_health,
        unit.data().power,
        unit.data().max_power,
        unit.data().class_id,
        unit.data().display_power,
        creature.spawn_id(),
        speed[wow_constants::UnitMoveType::Walk as usize],
        speed[wow_constants::UnitMoveType::Run as usize],
        creature.movement_flags_like_cpp(),
        creature.runtime_motion_master_current_kind_like_cpp(),
        creature.ai_ownership().combat_target,
        creature
            .runtime_like_cpp()
            .respawn_aura_source_authority_like_cpp(),
    )
}

fn d3a3_legacy_creature_like_cpp(world: &D3a3WorldLikeCpp) -> String {
    let guard = world.manager.read().unwrap();
    d3a3_creature_state_like_cpp(
        guard
            .find_creature(0, 0, world.creature)
            .map(|creature| &creature.creature),
    )
}

fn d3a3_canonical_creature_like_cpp(world: &D3a3WorldLikeCpp) -> String {
    let guard = world.canonical.lock().unwrap();
    d3a3_creature_state_like_cpp(
        guard
            .find_map(0, 0)
            .and_then(|map| map.map().get_typed_creature(world.creature)),
    )
}

/// The respawn timers of one world: the persisted row and queue length of the
/// store that owns them, and the canonical map's respawn store.
fn d3a3_respawn_timers_like_cpp(
    world: &D3a3WorldLikeCpp,
    legacy_store: bool,
) -> (usize, Option<i64>, i64) {
    let creature_type = wow_map::SpawnObjectType::Creature;
    let guard = world.canonical.lock().unwrap();
    let map = guard.find_map(0, 0).unwrap().map();
    let canonical_store = map.get_respawn_time_like_cpp(creature_type, D3A3_SPAWN_ID_LIKE_CPP);
    if legacy_store {
        let legacy = world.manager.read().unwrap();
        (
            legacy.respawn_queue_len(0, 0),
            legacy.persisted_respawn_time_like_cpp(0, 0, creature_type, D3A3_SPAWN_ID_LIKE_CPP),
            canonical_store,
        )
    } else {
        let queue = map.creature_respawn_queue_like_cpp();
        (
            queue.respawn_queue_len(),
            queue.persisted_respawn_time_like_cpp(creature_type, D3A3_SPAWN_ID_LIKE_CPP),
            canonical_store,
        )
    }
}

/// The parity check over one tick. `Err` names every observable that differs.
fn d3a3_parity_like_cpp(
    legacy_world: &D3a3WorldLikeCpp,
    legacy: &D3a3TickLikeCpp,
    executor_world: &D3a3WorldLikeCpp,
    executor: &D3a3TickLikeCpp,
    executor_outcome: &AdmittedCreatureCombatPhasesOutcomeLikeCpp,
) -> Result<(), Vec<String>> {
    let mut mismatches = Vec::new();
    let mut check = |label: &str, left: String, right: String| {
        if left != right {
            mismatches.push(format!("{label}:\n  legacy:   {left}\n  executor: {right}"));
        }
    };
    check(
        "lifecycle counters",
        d3a3_lifecycle_counters_like_cpp(&legacy.lifecycle),
        d3a3_lifecycle_counters_like_cpp(&executor.lifecycle),
    );
    let legacy_db: Vec<_> = legacy
        .lifecycle
        .respawn_db_mutations
        .iter()
        .map(d3a3_db_statement_like_cpp)
        .collect();
    let executor_db: Vec<_> = executor_outcome
        .deferred_respawn_db_mutations_like_cpp()
        .iter()
        .map(d3a3_db_statement_like_cpp)
        .collect();
    check(
        "respawn DB statements",
        format!("{:?}", legacy_db.iter().map(|(s, _)| s).collect::<Vec<_>>()),
        format!(
            "{:?}",
            executor_db.iter().map(|(s, _)| s).collect::<Vec<_>>()
        ),
    );
    if legacy_db.len() == executor_db.len()
        && !legacy_db
            .iter()
            .zip(&executor_db)
            .all(|((_, left), (_, right))| d3a3_times_agree_like_cpp(*left, *right))
    {
        check(
            "respawn DB times",
            format!("{legacy_db:?}"),
            format!("{executor_db:?}"),
        );
    }
    check(
        "refresh map keys",
        format!("{:?}", legacy.lifecycle.refresh_map_keys),
        format!("{:?}", executor.lifecycle.refresh_map_keys),
    );
    let (legacy_len, legacy_row, legacy_store) = d3a3_respawn_timers_like_cpp(legacy_world, true);
    let (executor_len, executor_row, executor_store) =
        d3a3_respawn_timers_like_cpp(executor_world, false);
    if legacy_len != executor_len
        || !d3a3_times_agree_like_cpp(legacy_row, executor_row)
        || !d3a3_times_agree_like_cpp(Some(legacy_store), Some(executor_store))
    {
        check(
            "respawn timers",
            format!("{:?}", (legacy_len, legacy_row, legacy_store)),
            format!("{:?}", (executor_len, executor_row, executor_store)),
        );
    }
    check(
        "movement counters",
        super::f6_8d3a2_movement_lifecycle_parity::d3a2_movement_counters_like_cpp(
            &legacy.movement,
        ),
        super::f6_8d3a2_movement_lifecycle_parity::d3a2_movement_counters_like_cpp(
            &executor.movement,
        ),
    );
    check(
        "movement plan",
        format!("{:?}", legacy.movement.plan.events),
        format!("{:?}", executor.movement.plan.events),
    );
    check(
        "aggro plan",
        format!("{:?}", legacy.aggro.plan.events),
        format!("{:?}", executor.aggro.plan.events),
    );
    check(
        "melee plan",
        format!("{:?}", (&legacy.melee.plan.events, &legacy.melee.commands)),
        format!(
            "{:?}",
            (&executor.melee.plan.events, &executor.melee.commands)
        ),
    );
    check(
        "creature",
        d3a3_legacy_creature_like_cpp(legacy_world),
        d3a3_canonical_creature_like_cpp(executor_world),
    );
    check(
        "canonical creature",
        d3a3_canonical_creature_like_cpp(legacy_world),
        d3a3_canonical_creature_like_cpp(executor_world),
    );
    check(
        "visibility packets",
        format!("{:?}", legacy.visibility),
        format!("{:?}", executor.visibility),
    );
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(mismatches)
    }
}

/// The tick deadline of tick `tick`: the respawn tick drains with a deadline
/// past the respawn delay, every other tick with the current instant.
fn d3a3_now_like_cpp(tick: usize, respawn_tick: usize) -> Instant {
    if tick == respawn_tick {
        Instant::now() + Duration::from_secs(10)
    } else {
        Instant::now()
    }
}

const D3A3_RESPAWN_TICK_LIKE_CPP: usize = 3;
const D3A3_TICKS_LIKE_CPP: usize = 5;

#[tokio::test]
async fn canonical_executor_matches_legacy_bridge_death_corpse_respawn_like_cpp() {
    use wow_constants::ServerOpcodes;

    let mut legacy_world = d3a3_world_like_cpp(97_100, true).await;
    let mut executor_world = d3a3_world_like_cpp(97_100, false).await;
    let mut ticks = Vec::with_capacity(D3A3_TICKS_LIKE_CPP);
    let mut despawned_state = String::new();
    for tick in 0..D3A3_TICKS_LIKE_CPP {
        let now = d3a3_now_like_cpp(tick, D3A3_RESPAWN_TICK_LIKE_CPP);
        let (executor, outcome, diff_ms) =
            d3a3_executor_tick_like_cpp(&mut executor_world, D3A3_DIFF_MS_LIKE_CPP, now).await;
        let legacy = d3a3_legacy_tick_like_cpp(&mut legacy_world, diff_ms, now).await;
        if let Err(mismatches) =
            d3a3_parity_like_cpp(&legacy_world, &legacy, &executor_world, &executor, &outcome)
        {
            panic!("tick {tick}: parity broken:\n{}", mismatches.join("\n"));
        }
        if tick == 2 {
            despawned_state = d3a3_canonical_creature_like_cpp(&executor_world);
        }
        ticks.push((legacy, executor, outcome));
    }

    let opcode = |bytes: &[u8]| u16::from_le_bytes([bytes[0], bytes[1]]);
    let update_object = ServerOpcodes::UpdateObject as u16;
    // Tick 0: JUST_DIED's `SaveRespawnTime` — one REP_RESPAWN, no despawn yet.
    let (_, first, first_outcome) = &ticks[0];
    let first_db = first_outcome.deferred_respawn_db_mutations_like_cpp();
    assert_eq!(first_db.len(), 1, "{first_db:?}");
    assert!(matches!(
        first_db[0],
        wow_persistence::RespawnPersistenceMutationLikeCpp::Save { key, .. }
            if key.spawn_id == D3A3_SPAWN_ID_LIKE_CPP && key.map_id == 0 && key.instance_id == 0
    ));
    assert_eq!(first.lifecycle.corpses_despawned, 0);
    // Tick 2: the corpse timer expires — `RemoveCorpse`, the canonical respawn
    // info, and the observer's DESTROY.
    let (_, despawn, despawn_outcome) = &ticks[2];
    assert_eq!(
        despawn.lifecycle.corpses_despawned, 1,
        "{:?}",
        despawn.lifecycle
    );
    assert_eq!(despawn.lifecycle.canonical_respawn_adds, 1);
    assert_eq!(despawn.lifecycle.canonical_removes, 1);
    assert!(
        despawn_outcome
            .deferred_respawn_db_mutations_like_cpp()
            .is_empty()
    );
    assert_eq!(despawn.lifecycle.refresh_map_keys, vec![(0, 0)]);
    assert_eq!(despawn.visibility.len(), 1, "one DESTROY for the corpse");
    assert_eq!(opcode(&despawn.visibility[0]), update_object);
    assert_eq!(
        despawned_state, "absent",
        "the corpse left the canonical map"
    );
    // Tick 3: `ProcessRespawns` → `DoRespawn`: the rebuilt incarnation, the
    // DEL_RESPAWN, and the observer's CREATE.
    let (_, respawn, respawn_outcome) = &ticks[D3A3_RESPAWN_TICK_LIKE_CPP];
    assert_eq!(
        respawn.lifecycle.respawns_processed, 1,
        "{:?}",
        respawn.lifecycle
    );
    assert_eq!(respawn.lifecycle.canonical_inserts, 1);
    assert_eq!(respawn.lifecycle.canonical_respawn_removes, 1);
    let respawn_db = respawn_outcome.deferred_respawn_db_mutations_like_cpp();
    assert_eq!(respawn_db.len(), 1);
    assert!(matches!(
        respawn_db[0],
        wow_persistence::RespawnPersistenceMutationLikeCpp::Delete { key }
            if key.spawn_id == D3A3_SPAWN_ID_LIKE_CPP
    ));
    assert_eq!(respawn.visibility.len(), 1, "one CREATE for the respawn");
    assert_eq!(opcode(&respawn.visibility[0]), update_object);
    assert_eq!(
        respawn.movement.creatures_seen, 1,
        "the respawn joins the same tick's movement phase"
    );
    // The respawned incarnation: full health, at home, alive, with the rates
    // its CREATE projection caches.
    let canonical = d3a3_canonical_creature_like_cpp(&executor_world);
    assert!(canonical.contains("death=Alive"), "{canonical}");
    assert!(canonical.contains("health=25/25"), "{canonical}");
    assert!(
        canonical.contains("speed_walk=1 speed_run=1.14286"),
        "{canonical}"
    );
    assert!(
        canonical.contains("aura_authority=(false, false)"),
        "{canonical}"
    );
    {
        let guard = legacy_world.manager.read().unwrap();
        let projection = &guard
            .find_creature(0, 0, legacy_world.creature)
            .expect("the legacy bridge republished the respawn")
            .create_data;
        let unit_rates = {
            let canonical = legacy_world.canonical.lock().unwrap();
            let creature = canonical
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_typed_creature(legacy_world.creature)
                .unwrap()
                .clone();
            creature.unit().speed_rate()
        };
        assert_eq!(
            (projection.speed_walk_rate, projection.speed_run_rate),
            (
                unit_rates[wow_constants::UnitMoveType::Walk as usize],
                unit_rates[wow_constants::UnitMoveType::Run as usize]
            ),
            "`Unit::m_speed_rate` matches the cached CREATE rates after respawn"
        );
    }
    // The respawn queue and its persisted row are consumed on both sides.
    assert_eq!(d3a3_respawn_timers_like_cpp(&executor_world, false).0, 0);
    assert_eq!(d3a3_respawn_timers_like_cpp(&executor_world, false).1, None);
    // The DB statements of the whole run, in order.
    let order: Vec<_> = ticks
        .iter()
        .flat_map(|(_, _, outcome)| outcome.deferred_respawn_db_mutations_like_cpp())
        .map(|mutation| d3a3_db_statement_like_cpp(mutation).0)
        .collect();
    let legacy_order: Vec<_> = ticks
        .iter()
        .flat_map(|(legacy, _, _)| &legacy.lifecycle.respawn_db_mutations)
        .map(|mutation| d3a3_db_statement_like_cpp(mutation).0)
        .collect();
    assert_eq!(order, legacy_order);
    assert_eq!(order.len(), 2);
    assert!(order[0].starts_with("Save") && order[1].starts_with("Delete"));
}

/// Negative control: the executor's queued respawn entry is moved half a yard
/// off its home before the respawn tick, which the parity check catches in the
/// rebuilt creature and the observer's CREATE bytes.
#[tokio::test]
async fn canonical_executor_lifecycle_parity_detects_a_perturbed_respawn_like_cpp() {
    let mut legacy_world = d3a3_world_like_cpp(97_200, true).await;
    let mut executor_world = d3a3_world_like_cpp(97_200, false).await;
    for tick in 0..=D3A3_RESPAWN_TICK_LIKE_CPP {
        if tick == D3A3_RESPAWN_TICK_LIKE_CPP {
            let mut guard = executor_world.canonical.lock().unwrap();
            let queue = guard
                .find_map_mut(0, 0)
                .unwrap()
                .map_mut()
                .creature_respawn_queue_like_cpp_mut();
            assert_eq!(
                queue.respawn_queue.len(),
                1,
                "the corpse queued its respawn"
            );
            queue.respawn_queue[0].home_pos.x += 0.5;
        }
        let now = d3a3_now_like_cpp(tick, D3A3_RESPAWN_TICK_LIKE_CPP);
        let (executor, outcome, diff_ms) =
            d3a3_executor_tick_like_cpp(&mut executor_world, D3A3_DIFF_MS_LIKE_CPP, now).await;
        let legacy = d3a3_legacy_tick_like_cpp(&mut legacy_world, diff_ms, now).await;
        let parity =
            d3a3_parity_like_cpp(&legacy_world, &legacy, &executor_world, &executor, &outcome);
        if tick < D3A3_RESPAWN_TICK_LIKE_CPP {
            parity.unwrap_or_else(|mismatches| {
                panic!("tick {tick}: parity broken:\n{}", mismatches.join("\n"))
            });
            continue;
        }
        let mismatches = parity.expect_err("a perturbed respawn entry must break parity");
        assert!(
            mismatches
                .iter()
                .any(|mismatch| mismatch.starts_with("creature:")),
            "the rebuilt creature is a reported difference: {mismatches:?}"
        );
        assert!(
            mismatches
                .iter()
                .any(|mismatch| mismatch.starts_with("visibility packets:")),
            "the CREATE bytes are a reported difference: {mismatches:?}"
        );
    }
}

/// C++ `Creature::UpdateEntry` (`Creature.cpp:547-550`): the respawn rebuild
/// applies the template walk/run rates its CREATE projection carries to
/// `Unit::m_speed_rate`, and swim/flight at 1.0.
#[test]
fn respawn_rebuild_applies_template_speed_rates_to_the_unit_like_cpp() {
    use wow_constants::UnitMoveType;

    let guid = test_creature_guid(97_300);
    let queued = crate::map_manager::WorldCreature::new(
        guid,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        25,
        8,
        3,
        5,
        20.0,
        14,
        0,
        0,
        0,
    );
    let mut pending = crate::map_manager::pending_respawn_from_world_creature_like_cpp(
        &queued,
        Instant::now(),
        0,
    );
    pending.create_data.speed_walk_rate = 0.8;
    pending.create_data.speed_run_rate = 1.5;
    let creature = crate::map_manager::creature_from_pending_respawn_like_cpp(&pending, 0);
    let rates = creature.unit().speed_rate();
    assert_eq!(rates[UnitMoveType::Walk as usize], 0.8);
    assert_eq!(rates[UnitMoveType::Run as usize], 1.5);
    assert_eq!(rates[UnitMoveType::Swim as usize], 1.0);
    assert_eq!(rates[UnitMoveType::Flight as usize], 1.0);
    // The legacy bridge projection of the same rebuild caches the same rates.
    let world_creature =
        crate::map_manager::world_creature_from_pending_respawn_like_cpp(&pending, 0);
    assert_eq!(world_creature.create_data.speed_walk_rate, 0.8);
    assert_eq!(world_creature.create_data.speed_run_rate, 1.5);
    assert_eq!(
        world_creature.creature.unit().speed_rate()[UnitMoveType::Run as usize],
        1.5
    );
}
