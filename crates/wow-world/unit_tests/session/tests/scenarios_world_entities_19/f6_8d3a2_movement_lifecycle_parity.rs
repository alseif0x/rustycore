//! #1263 F6-8D3a-2 regressions: the admitted canonical executor's creature
//! movement phase, against the recorded legacy-bridge expectations.
//!
//! Each fixture: one creature and one player on map 0/0, driven through
//! `run_admitted_creature_combat_phases_isolated_like_cpp`, which runs the
//! movement phase on the canonical incarnation in place.
//!
//! Until #1263 F6-8E each fixture was also driven through the legacy bridge
//! and the two compared after every tick (F6-8D3a-2 parity). F6-8E deleted
//! the bridge; its observables are recorded in
//! `f6_8d3a_recorded_expectations.tsv` (see `f6_8d3a_recorded_expectations`)
//! and checked against the executor after every tick: each phase's counters,
//! the `RuntimePlan` packet bytes and recipients, the commands, and the
//! creature-owned observables: RNG position (next draws of a clone), clock,
//! position, home, AI and unit state, health, the active `MoveSpline`, the
//! spline id, the selected `MotionMaster` generator and the movement flags.
//!
//! Nothing on the movement path draws from the thread RNG or reads a process
//! clock: random and waypoint rolls come from the creature-owned runtime RNG,
//! and every spline deadline is the creature-local `Unit::Update` clock. No
//! field is masked. The fixtures set the unit speed rates to the template
//! defaults (walk 1.0, run 1.14286) the recording ran with.

use super::f6_8d3a_recorded_expectations::{RecordedExpectationsLikeCpp, assert_recorded_like_cpp};
use super::*;

const D3A2_SEED_LIKE_CPP: u64 = 0xD3A2_0001;
const D3A2_RUN_RATE_LIKE_CPP: f32 = 1.14286;

/// The player chase snapshots the map loop takes from the registry
/// (`collect_chase_target_snapshots_like_cpp`: position, combat reach,
/// in world, dry), read from the canonical players.
pub(super) fn d3a2_chase_targets_like_cpp(
    canonical: &SharedCanonicalMapManager,
    players: &[ObjectGuid],
) -> HashMap<(u16, u32, ObjectGuid), crate::creature_movement::ChaseTargetSnapshotLikeCpp> {
    let guard = canonical.lock().unwrap();
    let map = guard.find_map(0, 0).unwrap().map();
    players
        .iter()
        .filter_map(|guid| {
            let player = map.get_typed_player(*guid)?;
            Some((
                (0, 0, *guid),
                crate::creature_movement::ChaseTargetSnapshotLikeCpp {
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

/// The movement counters, as recorded (without the deleted legacy-owner skip
/// flag and mirror attempt count).
pub(super) fn d3a2_movement_counters_like_cpp(
    outcome: &crate::session::CreatureMovementTickOutcomeLikeCpp,
) -> String {
    format!(
        "maps={} creatures={} packets={}",
        outcome.maps_seen, outcome.creatures_seen, outcome.movement_packets,
    )
}

struct D3a2WorldLikeCpp {
    canonical: SharedCanonicalMapManager,
    _session: WorldSession,
    creature: ObjectGuid,
    player: ObjectGuid,
    candidates: Vec<CreatureAggroCandidateLikeCpp>,
}

/// Build one fixture: the creature at (10, 10) and the player at `player_at`.
/// Every call with the same arguments yields the same state.
fn d3a2_world_like_cpp(
    base: i64,
    player_at: Position,
    configure: impl FnOnce(&mut wow_entities::Creature, ObjectGuid),
    candidate: impl FnOnce(ObjectGuid) -> Option<CreatureAggroCandidateLikeCpp>,
) -> D3a2WorldLikeCpp {
    let canonical = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature = test_creature_guid(base);
    let player = ObjectGuid::create_player(1, base + 1);
    register_test_creature_mirrored_like_cpp(
        &mut session,
        canonical.clone(),
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
            let unit = world_creature.unit_mut();
            unit.set_speed_rate_like_cpp(wow_constants::UnitMoveType::Walk, 1.0);
            unit.set_speed_rate_like_cpp(wow_constants::UnitMoveType::Run, D3A2_RUN_RATE_LIKE_CPP);
            crate::creature_projection::test_support::seed_runtime_rng_like_cpp(
                world_creature,
                D3A2_SEED_LIKE_CPP,
            );
            configure(world_creature, player);
        })
        .expect("the creature is admitted");
    let candidates = candidate(player).into_iter().collect();
    D3a2WorldLikeCpp {
        canonical,
        _session: session,
        creature,
        player,
        candidates,
    }
}

/// One admitted canonical tick through the executor.
fn d3a2_executor_tick_like_cpp(
    world: &D3a2WorldLikeCpp,
    diff_ms: u32,
) -> AdmittedCreatureCombatPhasesOutcomeLikeCpp {
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
    let config = aggro_hostile_config_like_cpp();
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
            map_store: &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
            now: Instant::now(),
        },
    );
    assert!(outcome.executed_like_cpp(), "{:?}", outcome.admission);
    assert_eq!(
        outcome.clock_advanced_ms,
        u64::from(admission.diff_ms),
        "one movement-phase clock step for the admitted creature"
    );
    let _ = world.canonical.lock().unwrap().abandon_tick_like_cpp(plan);
    outcome
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

/// One pass over every recorded observable of one tick. `Err` names every
/// observable that differs.
fn d3a2_check_like_cpp(
    expectations: &mut RecordedExpectationsLikeCpp,
    world: &D3a2WorldLikeCpp,
    executor: &AdmittedCreatureCombatPhasesOutcomeLikeCpp,
) -> Result<(), Vec<String>> {
    let mut pass = expectations.pass_like_cpp();
    pass.check_like_cpp(
        "movement counters",
        d3a2_movement_counters_like_cpp(&executor.movement),
    );
    pass.check_like_cpp(
        "movement plan",
        format!("{:?}", executor.movement.plan.events),
    );
    pass.check_like_cpp("aggro counters", d3a2_counters_like_cpp(&executor.aggro));
    pass.check_like_cpp("aggro plan", format!("{:?}", executor.aggro.plan.events));
    pass.check_like_cpp(
        "aggro commands",
        format!(
            "{:?}",
            (&executor.aggro.commands, &executor.aggro.stop_commands)
        ),
    );
    pass.check_like_cpp("spell counters", d3a2_counters_like_cpp(&executor.spell));
    pass.check_like_cpp("melee counters", d3a2_counters_like_cpp(&executor.melee));
    pass.check_like_cpp("melee plan", format!("{:?}", executor.melee.plan.events));
    pass.check_like_cpp("melee commands", format!("{:?}", executor.melee.commands));
    pass.check_like_cpp("creature", d3a2_canonical_creature_like_cpp(world));
    pass.finish_like_cpp()
}

/// Drive the fixture through `ticks` ticks of `diff_ms`, checking each against
/// the recorded expectations of `test`, and return every tick's outcome.
fn d3a2_drive_like_cpp(
    test: &'static str,
    world: &D3a2WorldLikeCpp,
    ticks: usize,
    diff_ms: u32,
) -> Vec<AdmittedCreatureCombatPhasesOutcomeLikeCpp> {
    let mut expectations = RecordedExpectationsLikeCpp::for_test_like_cpp(test);
    (0..ticks)
        .map(|tick| {
            let executor = d3a2_executor_tick_like_cpp(world, diff_ms);
            assert_recorded_like_cpp(
                &format!("tick {tick}"),
                d3a2_check_like_cpp(&mut expectations, world, &executor),
            );
            executor
        })
        .collect()
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
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(player, 5.0);
        },
        |player| Some(aggro_candidate_like_cpp(player, player_at)),
    )
}

#[test]
fn canonical_executor_matches_recorded_chase_movement_like_cpp() {
    let executor_world = d3a2_chase_world_like_cpp(96_100);
    let outcomes = d3a2_drive_like_cpp(
        "canonical_executor_matches_recorded_chase_movement_like_cpp",
        &executor_world,
        3,
        200,
    );
    let executor = &outcomes[0];
    // The phase acted: one chase spline toward the player, published to the
    // creature's nearby observers.
    assert_eq!(executor.movement.creatures_seen, 1);
    assert_eq!(
        executor.movement.movement_packets, 1,
        "{:?}",
        executor.movement
    );
    let event = &executor.movement.plan.events[0];
    assert_eq!(event.source_guid, executor_world.creature);
    assert_eq!(
        d3a2_opcode_like_cpp(&event.packet_bytes),
        wow_constants::ServerOpcodes::OnMonsterMove as u16
    );
    assert!(matches!(
        event.recipients,
        crate::runtime_plan::RecipientRule::NearbyVisible { .. }
    ));
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
fn canonical_executor_matches_recorded_home_return_and_restore_like_cpp() {
    let build = || {
        d3a2_world_like_cpp(
            96_200,
            Position::new(40.0, 40.0, 0.0, 0.0),
            |creature, _| {
                creature.set_ai_position(Position::new(16.0, 10.0, 0.0, 0.0));
                creature.unit_mut().set_health(10);
                creature.set_ai_state(wow_entities::CreatureAiState::Returning);
            },
            |_| None,
        )
    };
    let executor_world = build();
    let outcomes = d3a2_drive_like_cpp(
        "canonical_executor_matches_recorded_home_return_and_restore_like_cpp",
        &executor_world,
        6,
        500,
    );

    // Tick 0: `HomeMovementGenerator` launches the return to the spawn point.
    let first = &outcomes[0];
    assert_eq!(first.movement.movement_packets, 1, "{:?}", first.movement);
    assert_eq!(
        d3a2_opcode_like_cpp(&first.movement.plan.events[0].packet_bytes),
        wow_constants::ServerOpcodes::OnMonsterMove as u16
    );
    // A later tick reaches home: `DoFinalize`'s `SetSpawnHealth()` restores the
    // creature and the movement phase publishes the values update once.
    let restores: Vec<_> = outcomes
        .iter()
        .flat_map(|executor| &executor.movement.plan.events)
        .filter(|event| {
            d3a2_opcode_like_cpp(&event.packet_bytes)
                == wow_constants::ServerOpcodes::UpdateObject as u16
        })
        .collect();
    assert_eq!(restores.len(), 1, "one reached-home values update");
    assert!(matches!(
        restores[0].recipients,
        crate::runtime_plan::RecipientRule::NearbyVisibleDurable { .. }
    ));
    let canonical = d3a2_canonical_creature_like_cpp(&executor_world);
    assert!(canonical.contains("health=25/25"), "{canonical}");
    assert!(canonical.contains("ai=Idle"), "{canonical}");
}

#[test]
fn canonical_executor_matches_recorded_alert_move_distract_like_cpp() {
    use wow_packet::ServerPacket;

    let build = || {
        let player_at = Position::new(17.0, 10.0, 0.0, 0.0);
        d3a2_world_like_cpp(
            96_300,
            player_at,
            |creature, _| {
                creature.ai_ownership_mut().aggro_radius = 9.0;
                creature.unit_mut().set_level(80);
            },
            |player| {
                let mut candidate = aggro_candidate_like_cpp(player, player_at);
                let mut stealthed = Unit::new(true);
                stealthed.set_stealth_like_cpp(0, 408);
                candidate.player_visibility_detection =
                    stealthed.visibility_detection_like_cpp().clone();
                Some(candidate)
            },
        )
    };
    let executor_world = build();
    let outcomes = d3a2_drive_like_cpp(
        "canonical_executor_matches_recorded_alert_move_distract_like_cpp",
        &executor_world,
        1,
        200,
    );
    let executor = &outcomes[0];
    // `CreatureAI::TriggerAlert`: the alert reaction is published because the
    // canonical store now starts `MoveDistract` itself.
    assert_eq!(executor.aggro.alert_triggers, 1, "{:?}", executor.aggro);
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

/// Negative control: the creature stands half a yard off before the tick,
/// which the check against the recorded expectations catches in the movement
/// packet and the creature state.
#[test]
fn canonical_executor_movement_regression_detects_a_perturbed_position_like_cpp() {
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
    let executor = d3a2_executor_tick_like_cpp(&executor_world, 200);
    let mut expectations = RecordedExpectationsLikeCpp::for_test_like_cpp(
        "canonical_executor_movement_regression_detects_a_perturbed_position_like_cpp",
    );
    let mismatches = d3a2_check_like_cpp(&mut expectations, &executor_world, &executor)
        .expect_err("a perturbed position must be caught");
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
