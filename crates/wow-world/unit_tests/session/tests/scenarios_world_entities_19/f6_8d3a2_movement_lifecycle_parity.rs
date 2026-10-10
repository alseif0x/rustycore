//! #1263 F6-8D3a-2 regressions: the admitted canonical executor runs the
//! creature movement phase with parity to the legacy bridge.
//!
//! Each fixture is built twice, identically: one creature and one player on
//! map 0/0. World **L** is driven through the legacy bridge in the legacy loop
//! order — the complete movement phase
//! (`run_legacy_creature_movement_tick_once_like_cpp`), aggro plus its
//! canonical start/stop commit, spell, creature melee — and world **C**
//! through `run_admitted_creature_combat_phases_isolated_like_cpp`, which now
//! runs the movement phase where the legacy loop runs it, on the canonical
//! incarnation in place. The comparison covers each phase's counters (the
//! movement `canonical_syncs` attempt count is the legacy bridge's mirror and
//! is masked: the executor has no mirror), the `RuntimePlan` packet bytes and
//! recipients, the commands, and the creature-owned observables: RNG position
//! (next draws of a clone), clock, position, home, AI and unit state, health,
//! the active `MoveSpline`, the spline id, the selected `MotionMaster`
//! generator and the movement flags.
//!
//! Nothing on the movement path draws from the thread RNG or reads a process
//! clock: random and waypoint rolls come from the creature-owned runtime RNG,
//! and every spline deadline is the creature-local `Unit::Update` clock. No
//! field is masked.
//!
//! The legacy bridge launches splines from its cached `CreatureCreateData`
//! speed rates and the canonical incarnation from `Unit::m_speed_rate`. The
//! fixtures set the unit rates to the template defaults the projection carries
//! (walk 1.0, run 1.14286) so both read the same C++ `GetSpeedRate`.
//!
//! Lifecycle (corpse despawn, respawn and its visibility refresh) is the
//! declared F6-8D3a-2 boundary and is not part of the executor yet.

use super::*;

const D3A2_SEED_LIKE_CPP: u64 = 0xD3A2_0001;
const D3A2_RUN_RATE_LIKE_CPP: f32 = 1.14286;

/// The player chase snapshots the legacy bridge takes from the registry
/// (`collect_legacy_chase_target_snapshots_like_cpp`: position, combat reach,
/// in world, dry), read from the canonical players.
pub(super) fn d3a2_chase_targets_like_cpp(
    canonical: &SharedCanonicalMapManager,
    players: &[ObjectGuid],
) -> HashMap<(u16, u32, ObjectGuid), crate::map_manager::ChaseTargetSnapshotLikeCpp> {
    let guard = canonical.lock().unwrap();
    let map = guard.find_map(0, 0).unwrap().map();
    players
        .iter()
        .filter_map(|guid| {
            let player = map.get_typed_player(*guid)?;
            Some((
                (0, 0, *guid),
                crate::map_manager::ChaseTargetSnapshotLikeCpp {
                    guid: *guid,
                    position: player.unit().world().position(),
                    combat_reach: player.unit().data().combat_reach,
                    in_world: true,
                    in_water: Some(false),
                },
            ))
        })
        .collect()
}

/// The movement phase's pathfinding seam with no navmesh: every path is the
/// direct `PATHFIND_NOT_USING_PATH` shortcut, as on an unmeshed map.
pub(super) fn d3a2_mmap_config_like_cpp() -> MMapRuntimeConfigLikeCpp {
    MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    }
}

/// The complete legacy movement phase, as the world-server bridge runs it.
pub(super) fn d3a2_legacy_movement_tick_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    canonical: &SharedCanonicalMapManager,
    chase_targets: &HashMap<(u16, u32, ObjectGuid), crate::map_manager::ChaseTargetSnapshotLikeCpp>,
    diff_ms: u32,
) -> crate::session::LegacyCreatureMovementTickOutcomeLikeCpp {
    run_legacy_creature_movement_tick_once_like_cpp(
        manager,
        Some(canonical),
        &d3a2_mmap_config_like_cpp(),
        None,
        chase_targets,
        diff_ms,
    )
}

/// The movement counters without the legacy mirror's attempt count.
pub(super) fn d3a2_movement_counters_like_cpp(
    outcome: &crate::session::LegacyCreatureMovementTickOutcomeLikeCpp,
) -> String {
    format!(
        "skipped={} maps={} creatures={} packets={}",
        outcome.skipped_owner_not_global,
        outcome.maps_seen,
        outcome.creatures_seen,
        outcome.movement_packets,
    )
}

struct D3a2WorldLikeCpp {
    manager: crate::map_manager::SharedMapManager,
    canonical: SharedCanonicalMapManager,
    _session: WorldSession,
    creature: ObjectGuid,
    player: ObjectGuid,
    candidates: Vec<LegacyCreatureAggroCandidateLikeCpp>,
}

/// Build one fixture: the creature at (10, 10) and the player at `player_at`.
/// Every call with the same arguments yields the same state.
fn d3a2_world_like_cpp(
    base: i64,
    player_at: Position,
    configure: impl FnOnce(&mut crate::map_manager::WorldCreature, ObjectGuid),
    candidate: impl FnOnce(ObjectGuid) -> Option<LegacyCreatureAggroCandidateLikeCpp>,
) -> D3a2WorldLikeCpp {
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature = test_creature_guid(base);
    let player = ObjectGuid::create_player(1, base + 1);
    register_test_creature_mirrored_like_cpp(
        &mut session,
        manager.clone(),
        &canonical,
        creature,
        25,
    );
    add_canonical_test_player_on_map(&canonical, player, player_at, 0, 0);
    {
        let mut guard = canonical.lock().unwrap();
        let unit = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player)
            .unwrap()
            .unit_mut();
        unit.set_faction(1);
        unit.set_level(80);
        unit.set_max_health(1_000);
        unit.set_health(1_000);
        unit.set_death_state(wow_constants::DeathState::Alive);
    }
    session
        .mutate_world_creature(creature, |world_creature| {
            let unit = world_creature.creature.unit_mut();
            unit.set_speed_rate_like_cpp(wow_constants::UnitMoveType::Walk, 1.0);
            unit.set_speed_rate_like_cpp(wow_constants::UnitMoveType::Run, D3A2_RUN_RATE_LIKE_CPP);
            world_creature.seed_runtime_rng_like_cpp(D3A2_SEED_LIKE_CPP);
            configure(world_creature, player);
        })
        .expect("the creature is admitted");
    manager
        .write()
        .unwrap()
        .set_tick_owner(crate::map_manager::RuntimeTickOwner::GlobalLegacy);
    let candidates = candidate(player).into_iter().collect();
    D3a2WorldLikeCpp {
        manager,
        canonical,
        _session: session,
        creature,
        player,
        candidates,
    }
}

#[derive(Debug)]
struct D3a2TickLikeCpp {
    movement: crate::session::LegacyCreatureMovementTickOutcomeLikeCpp,
    aggro: LegacyCreatureAggroTickOutcomeLikeCpp,
    spell: crate::session::LegacyCreatureSpellTickOutcomeLikeCpp,
    melee: crate::session::LegacyCreatureMeleeTickOutcomeLikeCpp,
}

/// World L: the legacy bridge in the legacy loop order.
fn d3a2_legacy_tick_like_cpp(world: &D3a2WorldLikeCpp, diff_ms: u32) -> D3a2TickLikeCpp {
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
    let chase_targets = d3a2_chase_targets_like_cpp(&world.canonical, &[world.player]);
    let movement = d3a2_legacy_movement_tick_like_cpp(
        &world.manager,
        &world.canonical,
        &chase_targets,
        diff_ms,
    );
    let mut aggro = run_legacy_creature_aggro_tick_once_with_config_and_canonical_like_cpp(
        &world.manager,
        Some(&world.canonical),
        &world.candidates,
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
    let spell = run_legacy_creature_spell_tick_once_like_cpp(
        &world.manager,
        Some(&world.canonical),
        &config,
    );
    let melee = run_legacy_creature_melee_tick_once_like_cpp(
        &world.manager,
        Some(&world.canonical),
        &config,
    );
    D3a2TickLikeCpp {
        movement,
        aggro,
        spell,
        melee,
    }
}

/// World C: one admitted canonical tick through the executor. Returns the
/// outcome and the admitted diff, which the legacy world then replays.
fn d3a2_executor_tick_like_cpp(
    world: &D3a2WorldLikeCpp,
    diff_ms: u32,
) -> (AdmittedCreatureCombatPhasesOutcomeLikeCpp, u32) {
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
        0x0D3A_2000,
        plan.epoch_like_cpp(),
        plan.effective_diff_ms(),
        plan.updated_maps_like_cpp()
            .iter()
            .map(|participant| (participant.key, participant.incarnation)),
        [(key, world.creature)],
    )
    .expect("the canonical owner is readable");
    assert_eq!(admission.objects.len(), 1, "the creature is admitted");
    let config = legacy_aggro_hostile_config_like_cpp();
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let lease = std::sync::Arc::new(std::sync::Mutex::new(CreatureExecutionLeaseLikeCpp::new()));
    let chase_targets = d3a2_chase_targets_like_cpp(&world.canonical, &[world.player]);
    let mmap_config = d3a2_mmap_config_like_cpp();
    let outcome = run_admitted_creature_combat_phases_isolated_like_cpp(
        &admission,
        &world.canonical,
        &lease,
        AdmittedCreatureCombatPhaseInputsLikeCpp {
            player_melee_attackers: &[],
            player_melee_phase_state: &mut phase_state,
            aggro_candidates: &world.candidates,
            terrain: None,
            mmap_config: &mmap_config,
            mmap_pathfinder: None,
            chase_targets: &chase_targets,
            config: &config,
        },
    );
    assert!(outcome.executed_like_cpp(), "{:?}", outcome.admission);
    assert_eq!(
        outcome.clock_advanced_ms,
        u64::from(admission.diff_ms),
        "one movement-phase clock step for the admitted creature"
    );
    let _ = world.canonical.lock().unwrap().abandon_tick_like_cpp(plan);
    (outcome, admission.diff_ms)
}

/// Every movement-relevant creature-owned observable.
fn d3a2_creature_state_like_cpp(creature: &wow_entities::Creature) -> String {
    let mut probe = creature.clone();
    let draws: Vec<_> = (0..4).map(|_| probe.roll_damage()).collect();
    format!(
        "draws={draws:?} elapsed={} position={:?} home={:?} ai={:?} unit_state={:#x} \
         health={}/{} revision={} victim={:?} spline_id={} generator={:?} flags={:?} \
         spline={:?}",
        creature.runtime_elapsed_ms_like_cpp(),
        creature.position(),
        creature.home_position(),
        creature.ai_ownership().state,
        creature.unit().unit_state(),
        creature.unit().data().health,
        creature.unit().data().max_health,
        creature.unit().health_state_revision_like_cpp(),
        creature.ai_ownership().combat_target,
        creature.spline_id(),
        creature.runtime_motion_master_current_kind_like_cpp(),
        creature.movement_flags_like_cpp(),
        creature.active_move_spline_like_cpp(),
    )
}

fn d3a2_legacy_creature_like_cpp(world: &D3a2WorldLikeCpp) -> String {
    let guard = world.manager.read().unwrap();
    d3a2_creature_state_like_cpp(&guard.find_creature(0, 0, world.creature).unwrap().creature)
}

fn d3a2_canonical_creature_like_cpp(world: &D3a2WorldLikeCpp) -> String {
    let guard = world.canonical.lock().unwrap();
    d3a2_creature_state_like_cpp(
        guard
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_creature(world.creature)
            .unwrap(),
    )
}

/// Every counter of an outcome, with its order-dependent lists removed.
fn d3a2_counters_like_cpp<T: std::fmt::Debug>(outcome: &T) -> String {
    let rendered = format!("{outcome:?}");
    let cut = ["plan:", "commands:", "stop_commands:"]
        .iter()
        .filter_map(|field| rendered.find(field))
        .min()
        .unwrap_or(rendered.len());
    rendered[..cut].to_string()
}

/// The parity check over one tick. `Err` names every observable that differs.
fn d3a2_parity_like_cpp(
    legacy_world: &D3a2WorldLikeCpp,
    legacy: &D3a2TickLikeCpp,
    executor_world: &D3a2WorldLikeCpp,
    executor: &AdmittedCreatureCombatPhasesOutcomeLikeCpp,
) -> Result<(), Vec<String>> {
    let mut mismatches = Vec::new();
    let mut check = |label: &str, left: String, right: String| {
        if left != right {
            mismatches.push(format!("{label}:\n  legacy:   {left}\n  executor: {right}"));
        }
    };
    check(
        "movement counters",
        d3a2_movement_counters_like_cpp(&legacy.movement),
        d3a2_movement_counters_like_cpp(&executor.movement),
    );
    check(
        "movement plan",
        format!("{:?}", legacy.movement.plan.events),
        format!("{:?}", executor.movement.plan.events),
    );
    check(
        "aggro counters",
        d3a2_counters_like_cpp(&legacy.aggro),
        d3a2_counters_like_cpp(&executor.aggro),
    );
    check(
        "aggro plan",
        format!("{:?}", legacy.aggro.plan.events),
        format!("{:?}", executor.aggro.plan.events),
    );
    check(
        "aggro commands",
        format!(
            "{:?}",
            (&legacy.aggro.commands, &legacy.aggro.stop_commands)
        ),
        format!(
            "{:?}",
            (&executor.aggro.commands, &executor.aggro.stop_commands)
        ),
    );
    check(
        "spell counters",
        d3a2_counters_like_cpp(&legacy.spell),
        d3a2_counters_like_cpp(&executor.spell),
    );
    check(
        "melee counters",
        d3a2_counters_like_cpp(&legacy.melee),
        d3a2_counters_like_cpp(&executor.melee),
    );
    check(
        "melee plan",
        format!("{:?}", legacy.melee.plan.events),
        format!("{:?}", executor.melee.plan.events),
    );
    check(
        "melee commands",
        format!("{:?}", legacy.melee.commands),
        format!("{:?}", executor.melee.commands),
    );
    check(
        "creature",
        d3a2_legacy_creature_like_cpp(legacy_world),
        d3a2_canonical_creature_like_cpp(executor_world),
    );
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(mismatches)
    }
}

/// Drive both worlds through `ticks` ticks of `diff_ms`, checking parity after
/// each, and return every tick's pair of outcomes.
fn d3a2_drive_both_like_cpp(
    legacy_world: &D3a2WorldLikeCpp,
    executor_world: &D3a2WorldLikeCpp,
    ticks: usize,
    diff_ms: u32,
) -> Vec<(D3a2TickLikeCpp, AdmittedCreatureCombatPhasesOutcomeLikeCpp)> {
    let mut outcomes = Vec::with_capacity(ticks);
    for tick in 0..ticks {
        let (executor, admitted_diff_ms) = d3a2_executor_tick_like_cpp(executor_world, diff_ms);
        let legacy = d3a2_legacy_tick_like_cpp(legacy_world, admitted_diff_ms);
        if let Err(mismatches) =
            d3a2_parity_like_cpp(legacy_world, &legacy, executor_world, &executor)
        {
            panic!("tick {tick}: parity broken:\n{}", mismatches.join("\n"));
        }
        outcomes.push((legacy, executor));
    }
    outcomes
}

fn d3a2_opcode_like_cpp(packet_bytes: &[u8]) -> u16 {
    u16::from_le_bytes([packet_bytes[0], packet_bytes[1]])
}

/// The chase fixture: the creature is engaged with a player standing out of
/// melee range, so `ChaseMovementGenerator` launches a `MonsterMove`.
fn d3a2_chase_world_like_cpp(base: i64) -> D3a2WorldLikeCpp {
    let player_at = Position::new(18.0, 10.0, 0.0, 0.0);
    d3a2_world_like_cpp(
        base,
        player_at,
        |creature, player| {
            creature.enter_combat(player);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(player, 5.0);
        },
        |player| Some(legacy_aggro_candidate_like_cpp(player, player_at)),
    )
}

#[test]
fn canonical_executor_matches_legacy_bridge_chase_movement_like_cpp() {
    let legacy_world = d3a2_chase_world_like_cpp(96_100);
    let executor_world = d3a2_chase_world_like_cpp(96_100);
    let outcomes = d3a2_drive_both_like_cpp(&legacy_world, &executor_world, 3, 200);
    let (legacy, executor) = &outcomes[0];
    // The phase acted: one chase spline toward the player, published to the
    // creature's nearby observers.
    assert_eq!(executor.movement.creatures_seen, 1);
    assert_eq!(
        executor.movement.movement_packets, 1,
        "{:?}",
        executor.movement
    );
    assert_eq!(
        executor.movement.canonical_syncs, 0,
        "no mirror on the executor"
    );
    assert_eq!(
        legacy.movement.canonical_syncs, 1,
        "the bridge mirrors its frame"
    );
    let event = &executor.movement.plan.events[0];
    assert_eq!(event.source_guid, executor_world.creature);
    assert_eq!(
        d3a2_opcode_like_cpp(&event.packet_bytes),
        wow_constants::ServerOpcodes::OnMonsterMove as u16
    );
    assert!(matches!(
        event.recipients,
        crate::map_manager::RecipientRule::NearbyVisible { .. }
    ));
    assert_eq!(
        legacy.movement.plan.events[0].packet_bytes, event.packet_bytes,
        "the same MonsterMove bytes"
    );
    let canonical = d3a2_canonical_creature_like_cpp(&executor_world);
    assert!(canonical.contains("generator=Some(Chase)"), "{canonical}");
    // The canonical incarnation itself walked the spline toward the player.
    let position = executor_world
        .canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_creature(executor_world.creature)
        .unwrap()
        .position();
    assert!(position.x > 10.0, "{canonical}");
}

#[test]
fn canonical_executor_matches_legacy_bridge_home_return_and_restore_like_cpp() {
    let build = || {
        d3a2_world_like_cpp(
            96_200,
            Position::new(40.0, 40.0, 0.0, 0.0),
            |creature, _| {
                creature
                    .creature
                    .set_ai_position(Position::new(16.0, 10.0, 0.0, 0.0));
                creature.creature.unit_mut().set_health(10);
                creature
                    .creature
                    .set_ai_state(wow_entities::CreatureAiState::Returning);
            },
            |_| None,
        )
    };
    let legacy_world = build();
    let executor_world = build();
    let outcomes = d3a2_drive_both_like_cpp(&legacy_world, &executor_world, 6, 500);

    // Tick 0: `HomeMovementGenerator` launches the return to the spawn point.
    let (_, first) = &outcomes[0];
    assert_eq!(first.movement.movement_packets, 1, "{:?}", first.movement);
    assert_eq!(
        d3a2_opcode_like_cpp(&first.movement.plan.events[0].packet_bytes),
        wow_constants::ServerOpcodes::OnMonsterMove as u16
    );
    // A later tick reaches home: `DoFinalize`'s `SetSpawnHealth()` restores the
    // creature and the movement phase publishes the values update once.
    let restores: Vec<_> = outcomes
        .iter()
        .flat_map(|(_, executor)| &executor.movement.plan.events)
        .filter(|event| {
            d3a2_opcode_like_cpp(&event.packet_bytes)
                == wow_constants::ServerOpcodes::UpdateObject as u16
        })
        .collect();
    assert_eq!(restores.len(), 1, "one reached-home values update");
    assert!(matches!(
        restores[0].recipients,
        crate::map_manager::RecipientRule::NearbyVisibleDurable { .. }
    ));
    let canonical = d3a2_canonical_creature_like_cpp(&executor_world);
    assert!(canonical.contains("health=25/25"), "{canonical}");
    assert!(canonical.contains("ai=Idle"), "{canonical}");
}

#[test]
fn canonical_executor_matches_legacy_bridge_alert_move_distract_like_cpp() {
    use wow_packet::ServerPacket;

    let build = || {
        let player_at = Position::new(17.0, 10.0, 0.0, 0.0);
        d3a2_world_like_cpp(
            96_300,
            player_at,
            |creature, _| {
                creature.creature.ai_ownership_mut().aggro_radius = 9.0;
                creature.creature.unit_mut().set_level(80);
            },
            |player| {
                let mut candidate = legacy_aggro_candidate_like_cpp(player, player_at);
                let mut stealthed = Unit::new(true);
                stealthed.set_stealth_like_cpp(0, 408);
                candidate.player_visibility_detection =
                    stealthed.visibility_detection_like_cpp().clone();
                Some(candidate)
            },
        )
    };
    let legacy_world = build();
    let executor_world = build();
    let outcomes = d3a2_drive_both_like_cpp(&legacy_world, &executor_world, 1, 200);
    let (legacy, executor) = &outcomes[0];
    // `CreatureAI::TriggerAlert`: the alert reaction is published because the
    // canonical store now starts `MoveDistract` itself.
    assert_eq!(executor.aggro.alert_triggers, 1, "{:?}", executor.aggro);
    assert_eq!(legacy.aggro.alert_triggers, 1);
    assert_eq!(executor.aggro.aggro_starts, 0);
    assert_eq!(
        executor.aggro.plan.events[0].packet_bytes,
        wow_packet::packets::combat::AIReaction {
            unit_guid: executor_world.creature,
            reaction: wow_constants::creature::AiReaction::Alert,
        }
        .to_bytes()
    );
    let distracted = executor_world
        .canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_creature(executor_world.creature)
        .unwrap()
        .unit()
        .subsystems()
        .motion
        .active_generators
        .iter()
        .any(|generator| generator.kind == wow_entities::MovementGeneratorKind::Distract);
    assert!(distracted, "the distract generator holds the creature");
    let canonical = d3a2_canonical_creature_like_cpp(&executor_world);
    assert!(
        canonical.contains("spline=Some("),
        "MoveDistract launched its facing spline on the canonical incarnation: {canonical}"
    );
}

/// Negative control: the executor's creature stands half a yard off before the
/// tick, which the parity check catches in the movement packet and the
/// creature state.
#[test]
fn canonical_executor_movement_parity_detects_a_perturbed_position_like_cpp() {
    let legacy_world = d3a2_chase_world_like_cpp(96_400);
    let executor_world = d3a2_chase_world_like_cpp(96_400);
    {
        let mut guard = executor_world.canonical.lock().unwrap();
        let creature = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(executor_world.creature)
            .unwrap();
        creature.set_ai_position(Position::new(10.5, 10.0, 0.0, 0.0));
    }
    let (executor, admitted_diff_ms) = d3a2_executor_tick_like_cpp(&executor_world, 200);
    let legacy = d3a2_legacy_tick_like_cpp(&legacy_world, admitted_diff_ms);
    let mismatches = d3a2_parity_like_cpp(&legacy_world, &legacy, &executor_world, &executor)
        .expect_err("a perturbed position must break parity");
    assert!(
        mismatches
            .iter()
            .any(|mismatch| mismatch.starts_with("movement plan:")),
        "the MonsterMove bytes are a reported difference: {mismatches:?}"
    );
    assert!(
        mismatches
            .iter()
            .any(|mismatch| mismatch.starts_with("creature:")),
        "the creature state is a reported difference: {mismatches:?}"
    );
}
