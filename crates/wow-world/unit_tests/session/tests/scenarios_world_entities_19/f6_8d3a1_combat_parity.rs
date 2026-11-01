//! #1263 F6-8D3a-1 regressions: the admitted canonical executor's combat
//! phases, against the recorded legacy-bridge expectations.
//!
//! One seeded fixture: two creatures and two players on map 0/0. The hostile
//! creature is engaged with one player (threat from a hostile reaction), the
//! neutral creature is engaged by the other per the R6-fix semantics
//! (`EngageWithTarget`, accepted by `_IsTargetAcceptable` only because it is
//! engaged) and is that player's melee victim. The fixture is driven through
//! `run_admitted_creature_combat_phases_isolated_like_cpp` (player melee,
//! movement, aggro plus its start/stop commit, spell, creature melee).
//!
//! Until #1263 F6-8E the same fixture was also driven through the legacy
//! bridge and the two compared (F6-8D3a-1 parity). F6-8E deleted the bridge;
//! its observables are recorded in `f6_8d3a_recorded_expectations.tsv` (see
//! `f6_8d3a_recorded_expectations`) and each one is checked against the
//! executor: each phase's outcome (counters, `RuntimePlan` packet bytes and
//! recipients, command lists), the creature-owned RNG positions, the swing
//! timers and clocks, the threat tables and the health/revision tuples.
//! Events and commands are compared per source (a stable sort keeps each
//! creature's own sequence), as the parity check did.
//!
//! The attack-table roll (`rolled_melee_outcome_like_cpp`) draws from the
//! thread RNG rather than a creature stream, so it cannot be recorded; the
//! fixture runs the store-less always-hit branch of the same body.

use super::f6_8d3a_recorded_expectations::{RecordedExpectationsLikeCpp, assert_recorded_like_cpp};
use super::*;

const D3A1_HOSTILE_SEED_LIKE_CPP: u64 = 0xD3A1_0001;
const D3A1_NEUTRAL_SEED_LIKE_CPP: u64 = 0xD3A1_0002;
const D3A1_DIFF_MS_LIKE_CPP: u32 = 2_000;

struct D3a1WorldLikeCpp {
    canonical: SharedCanonicalMapManager,
    _session: WorldSession,
    hostile: ObjectGuid,
    neutral: ObjectGuid,
    striker: ObjectGuid,
    defender: ObjectGuid,
    attackers: Vec<crate::session::PlayerMeleeAttackerSnapshotLikeCpp>,
    candidates: Vec<CreatureAggroCandidateLikeCpp>,
}

/// Faction template 14 is hostile to the player template 1 (enemy faction
/// 930); template 15 has the shape of 3.4.3 `FactionTemplate` 7: neutral.
fn d3a1_config_like_cpp() -> CreatureAggroConfigLikeCpp {
    CreatureAggroConfigLikeCpp {
        faction_template_store: Some(Arc::new(
            wow_data::progression_rewards::FactionTemplateStore::from_entries([
                faction_template_entry(14, 72, 0, 0, 930),
                faction_template_entry(15, 7, 0, 0, 0),
                faction_template_entry(1, 930, 0, 0, 0),
            ]),
        )),
        faction_store: Some(Arc::new(FactionStore::from_entries([
            FactionEntry::for_test_like_cpp(72, 1),
            FactionEntry::for_test_like_cpp(7, -1),
        ]))),
        ..Default::default()
    }
}

fn d3a1_add_player_like_cpp(canonical: &SharedCanonicalMapManager, guid: ObjectGuid, at: Position) {
    add_canonical_test_player_on_map(canonical, guid, at, 0, 0);
    let mut guard = canonical.lock().unwrap();
    let player = guard
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(guid)
        .unwrap();
    let unit = player.unit_mut();
    unit.set_faction(1);
    unit.set_level(80);
    unit.set_max_health(1_000);
    unit.set_health(1_000);
    unit.set_death_state(wow_constants::DeathState::Alive);
    unit.subsystems_mut()
        .auras
        .set_spell_hit_aura_authority_inert_like_cpp(true);
}

/// Build the fixture. Every call yields the same state: same GUIDs, seeds,
/// positions, timers and engagements.
fn d3a1_world_like_cpp(base: i64) -> D3a1WorldLikeCpp {
    let canonical = shared_map_manager();
    let (mut session, _, _) = make_session();
    let hostile = test_creature_guid(base);
    let neutral = test_creature_guid(base + 1);
    let striker = ObjectGuid::create_player(1, base + 2);
    let defender = ObjectGuid::create_player(1, base + 3);
    register_test_creature_mirrored_like_cpp(
        &mut session,
        canonical.clone(),
        &canonical,
        hostile,
        100,
    );
    register_test_creature_mirrored_like_cpp(
        &mut session,
        canonical.clone(),
        &canonical,
        neutral,
        100,
    );
    // The striker faces the creatures (west); the defender stands in the
    // hostile creature's melee range.
    d3a1_add_player_like_cpp(
        &canonical,
        striker,
        Position::new(11.0, 10.0, 0.0, std::f32::consts::PI),
    );
    d3a1_add_player_like_cpp(&canonical, defender, Position::new(11.0, 10.2, 0.0, 0.0));
    {
        let mut guard = canonical.lock().unwrap();
        let player = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(striker)
            .unwrap();
        let unit = player.unit_mut();
        unit.set_attacking(Some(neutral));
        unit.set_target(neutral);
        unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
        unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
        unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        unit.reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
    }
    session
        .mutate_world_creature(hostile, |creature| {
            creature.unit_mut().set_faction(14);
            creature.engage_with_target_like_cpp(defender);
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(defender, 5.0);
            creature.ai_ownership_mut().last_swing_ms = 0;
            creature.ai_ownership_mut().swing_timer_ms = 0;
            crate::creature_projection::test_support::seed_runtime_rng_like_cpp(
                creature,
                D3A1_HOSTILE_SEED_LIKE_CPP,
            );
        })
        .expect("the hostile creature is admitted");
    session
        .mutate_world_creature(neutral, |creature| {
            creature.unit_mut().set_faction(15);
            creature.engage_with_target_like_cpp(striker);
            creature.ai_ownership_mut().last_swing_ms = 0;
            creature.ai_ownership_mut().swing_timer_ms = 0;
            crate::creature_projection::test_support::seed_runtime_rng_like_cpp(
                creature,
                D3A1_NEUTRAL_SEED_LIKE_CPP,
            );
        })
        .expect("the neutral creature is admitted");
    let (send_tx, _rx) = flume::bounded::<Vec<u8>>(8);
    let (command_tx, _crx) = flume::bounded::<SessionCommand>(8);
    let registration = PlayerRegistry::new().register_or_replace(
        striker,
        broadcast_info_with_command(striker, send_tx, command_tx),
        Default::default(),
    );
    let attackers = vec![crate::session::PlayerMeleeAttackerSnapshotLikeCpp {
        registration,
        player_guid: striker,
        map_id: 0,
        instance_id: 0,
        in_combat_mirror: false,
        tap_group_guids: Vec::new(),
    }];
    let candidates = vec![
        aggro_candidate_like_cpp(striker, Position::new(11.0, 10.0, 0.0, 0.0)),
        aggro_candidate_like_cpp(defender, Position::new(11.0, 10.2, 0.0, 0.0)),
    ];
    D3a1WorldLikeCpp {
        canonical,
        _session: session,
        hostile,
        neutral,
        striker,
        defender,
        attackers,
        candidates,
    }
}

/// Admit one canonical tick of `diff_ms` over both creatures.
fn d3a1_admit_like_cpp(
    world: &D3a1WorldLikeCpp,
    diff_ms: u32,
) -> (
    wow_map::MapTickPlanLikeCpp,
    AdmittedCreatureExecutionLikeCpp,
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
        0x0D3A_1000,
        plan.epoch_like_cpp(),
        plan.effective_diff_ms(),
        plan.updated_maps_like_cpp()
            .iter()
            .map(|participant| (participant.key, participant.incarnation)),
        [(key, world.hostile), (key, world.neutral)],
    )
    .expect("the canonical owner is readable");
    (plan, admission)
}

/// One tick of the admitted canonical executor.
fn d3a1_executor_tick_like_cpp(
    world: &D3a1WorldLikeCpp,
    admission: &AdmittedCreatureExecutionLikeCpp,
) -> AdmittedCreatureCombatPhasesOutcomeLikeCpp {
    let config = d3a1_config_like_cpp();
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let lease = std::sync::Arc::new(std::sync::Mutex::new(CreatureExecutionLeaseLikeCpp::new()));
    let chase_targets = super::f6_8d3a2_movement_lifecycle_parity::d3a2_chase_targets_like_cpp(
        &world.canonical,
        &[world.striker, world.defender],
    );
    let mmap_config = super::f6_8d3a2_movement_lifecycle_parity::d3a2_mmap_config_like_cpp();
    run_admitted_creature_combat_phases_isolated_like_cpp(
        admission,
        &world.canonical,
        &lease,
        AdmittedCreatureCombatPhaseInputsLikeCpp {
            player_melee_attackers: &world.attackers,
            player_melee_phase_state: &mut phase_state,
            aggro_candidates: &world.candidates,
            terrain: None,
            mmap_config: &mmap_config,
            mmap_pathfinder: None,
            chase_targets: &chase_targets,
            config: &config,
            map_store: &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
            now: Instant::now(),
            record_creature_module_events: false,
        },
    )
}

/// Drive the fixture through one tick (`perturb` runs right before it) and
/// check it against the recorded expectations of `test`. Returns the world,
/// the outcome, the expectations (for a further pass) and the check result.
fn d3a1_drive_like_cpp(
    test: &'static str,
    base: i64,
    perturb: impl FnOnce(&D3a1WorldLikeCpp),
) -> (
    D3a1WorldLikeCpp,
    AdmittedCreatureCombatPhasesOutcomeLikeCpp,
    RecordedExpectationsLikeCpp,
    Result<(), Vec<String>>,
) {
    let world = d3a1_world_like_cpp(base);
    let (_plan, admission) = d3a1_admit_like_cpp(&world, D3A1_DIFF_MS_LIKE_CPP);
    assert_eq!(admission.objects.len(), 2, "both creatures are admitted");
    perturb(&world);
    let executor = d3a1_executor_tick_like_cpp(&world, &admission);
    assert!(executor.executed_like_cpp(), "{:?}", executor.admission);
    assert_eq!(
        executor.clock_advanced_ms,
        2 * u64::from(admission.diff_ms),
        "one clock step per admitted creature"
    );
    let mut expectations = RecordedExpectationsLikeCpp::for_test_like_cpp(test);
    let result = d3a1_check_like_cpp(&mut expectations, &world, &executor);
    (world, executor, expectations, result)
}

/// The plan events in per-source order.
fn d3a1_events_like_cpp(events: &[RuntimeEvent]) -> Vec<String> {
    let mut events: Vec<_> = events
        .iter()
        .map(|event| (format!("{:?}", event.source_guid), format!("{event:?}")))
        .collect();
    // Stable: each source keeps its own sequence.
    events.sort_by(|a, b| a.0.cmp(&b.0));
    events.into_iter().map(|(_, event)| event).collect()
}

fn d3a1_sorted_debug_like_cpp<T: std::fmt::Debug>(items: &[T]) -> Vec<String> {
    let mut items: Vec<_> = items.iter().map(|item| format!("{item:?}")).collect();
    items.sort();
    items
}

/// Every counter of an outcome, with its order-dependent lists removed.
fn d3a1_counters_like_cpp<T: std::fmt::Debug>(outcome: &T) -> String {
    let rendered = format!("{outcome:?}");
    let cut = ["plan:", "commands:", "stop_commands:"]
        .iter()
        .filter_map(|field| rendered.find(field))
        .min()
        .unwrap_or(rendered.len());
    rendered[..cut].to_string()
}

/// The creature-owned observables: RNG position (next draws of a clone, which
/// leaves the stream untouched), clock, swing timer, victim, threat table,
/// health and health-state revision.
fn d3a1_creature_like_cpp(world: &D3a1WorldLikeCpp, guid: ObjectGuid) -> String {
    let guard = world.canonical.lock().unwrap();
    let creature = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_creature(guid)
        .unwrap();
    let mut probe = creature.clone();
    let draws: Vec<_> = (0..4).map(|_| probe.roll_damage()).collect();
    let combat = &creature.unit().subsystems().combat;
    let threat: Vec<_> = combat
        .sorted_threat_guids()
        .into_iter()
        .map(|guid| {
            let reference = combat.threat_ref(guid);
            (
                guid,
                reference.map(|reference| reference.threat()),
                reference.is_some_and(wow_entities::ThreatReferenceState::is_online),
            )
        })
        .collect();
    format!(
        "draws={draws:?} elapsed={} last_swing={} swing_timer={} victim={:?} threat={threat:?} health={} revision={}",
        creature.runtime_elapsed_ms_like_cpp(),
        creature.ai_ownership().last_swing_ms,
        creature.ai_ownership().swing_timer_ms,
        creature.ai_ownership().combat_target,
        creature.unit().data().health,
        creature.unit().health_state_revision_like_cpp(),
    )
}

fn d3a1_player_like_cpp(world: &D3a1WorldLikeCpp, guid: ObjectGuid) -> String {
    let guard = world.canonical.lock().unwrap();
    let player = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(guid)
        .unwrap();
    let unit = player.unit();
    format!(
        "health={} revision={} attacking={:?} in_combat={}",
        unit.data().health,
        unit.health_state_revision_like_cpp(),
        unit.attacking(),
        unit.subsystems().combat.has_combat(),
    )
}

/// One pass over every recorded observable. `Err` names every observable
/// that differs.
fn d3a1_check_like_cpp(
    expectations: &mut RecordedExpectationsLikeCpp,
    world: &D3a1WorldLikeCpp,
    executor: &AdmittedCreatureCombatPhasesOutcomeLikeCpp,
) -> Result<(), Vec<String>> {
    let mut pass = expectations.pass_like_cpp();
    pass.check_like_cpp(
        "player melee counters",
        d3a1_counters_like_cpp(&executor.player_melee),
    );
    pass.check_like_cpp(
        "player melee commands",
        d3a1_player_melee_commands_like_cpp(&executor.player_melee.commands),
    );
    pass.check_like_cpp(
        "movement counters",
        super::f6_8d3a2_movement_lifecycle_parity::d3a2_movement_counters_like_cpp(
            &executor.movement,
        ),
    );
    pass.check_like_cpp(
        "movement plan",
        format!("{:?}", d3a1_events_like_cpp(&executor.movement.plan.events)),
    );
    pass.check_like_cpp("aggro counters", d3a1_counters_like_cpp(&executor.aggro));
    pass.check_like_cpp(
        "aggro plan",
        format!("{:?}", d3a1_events_like_cpp(&executor.aggro.plan.events)),
    );
    pass.check_like_cpp(
        "aggro committed starts",
        format!(
            "{:?}",
            d3a1_sorted_debug_like_cpp(&executor.aggro_committed_starts)
        ),
    );
    pass.check_like_cpp(
        "aggro committed stops",
        format!(
            "{:?}",
            d3a1_sorted_debug_like_cpp(&executor.aggro_committed_stops)
        ),
    );
    pass.check_like_cpp(
        "spell phase executed",
        executor.spell_phase_executed.to_string(),
    );
    pass.check_like_cpp("spell counters", d3a1_counters_like_cpp(&executor.spell));
    pass.check_like_cpp(
        "spell plan",
        format!("{:?}", d3a1_events_like_cpp(&executor.spell.plan.events)),
    );
    pass.check_like_cpp("melee counters", d3a1_counters_like_cpp(&executor.melee));
    pass.check_like_cpp(
        "melee plan",
        format!("{:?}", d3a1_events_like_cpp(&executor.melee.plan.events)),
    );
    pass.check_like_cpp(
        "melee commands",
        format!("{:?}", d3a1_sorted_debug_like_cpp(&executor.melee.commands)),
    );
    for (label, guid) in [("hostile", world.hostile), ("neutral", world.neutral)] {
        pass.check_like_cpp(label, d3a1_creature_like_cpp(world, guid));
    }
    for (label, guid) in [("striker", world.striker), ("defender", world.defender)] {
        pass.check_like_cpp(label, d3a1_player_like_cpp(world, guid));
    }
    pass.finish_like_cpp()
}

/// The player melee commands, without the victim VALUES copy.
///
/// #1263 F6-8D3b-3: the legacy bridge's victim was a legacy copy, so its
/// command carried the victim's VALUES to the attacker and the recording
/// leaves them out; the executor's victim is the canonical map record, whose
/// `Map::SendObjectUpdates` publishes them (C++ `DealDamage` only marks the
/// field), so its command carries none.
fn d3a1_player_melee_commands_like_cpp(
    commands: &[crate::session::mailbox::ApplyPlayerMeleeResultLikeCppCommand],
) -> String {
    let commands: Vec<_> = commands
        .iter()
        .cloned()
        .map(|mut command| {
            command.victim_values_update = None;
            command
        })
        .collect();
    format!("{commands:?}")
}

#[test]
fn canonical_executor_matches_recorded_phase_by_phase_like_cpp() {
    let (world, executor, mut expectations, result) = d3a1_drive_like_cpp(
        "canonical_executor_matches_recorded_phase_by_phase_like_cpp",
        95_100,
        |_| {},
    );
    assert_recorded_like_cpp("tick 0", result);
    // A second pass over the same state reads the same values.
    assert_recorded_like_cpp(
        "tick 0, second pass",
        d3a1_check_like_cpp(&mut expectations, &world, &executor),
    );
}

#[test]
fn canonical_executor_matches_recorded_player_melee_phase_like_cpp() {
    let (_world, executor, _, result) = d3a1_drive_like_cpp(
        "canonical_executor_matches_recorded_player_melee_phase_like_cpp",
        95_110,
        |_| {},
    );
    assert_recorded_like_cpp("tick 0", result);
    // Phase 0 really ran: the striker hit the neutral creature once.
    assert_eq!(executor.player_melee.swings_ready, 1);
    assert_eq!(
        executor.player_melee.creature_hits, 1,
        "{:?}",
        executor.player_melee
    );
    assert_eq!(executor.player_melee.commands.len(), 1);
    let swings = &executor.player_melee.commands[0].swings;
    assert_eq!(swings.len(), 1);
    assert_eq!(swings[0].damage, 7);
    assert!(
        executor.player_melee.commands[0]
            .victim_values_update
            .is_none(),
        "the canonical victim's map publishes its VALUES"
    );
}

#[test]
fn canonical_executor_matches_recorded_aggro_phase_like_cpp() {
    use wow_packet::ServerPacket;

    let (world, executor, _, result) = d3a1_drive_like_cpp(
        "canonical_executor_matches_recorded_aggro_phase_like_cpp",
        95_120,
        |_| {},
    );
    assert_recorded_like_cpp("tick 0", result);
    // R6-fix: both creatures switch to their engaged victim and publish
    // `SMSG_AI_REACTION` (hostile) then `SMSG_ATTACK_START`, with no evade.
    assert_eq!(executor.aggro.victim_switches, 2, "{:?}", executor.aggro);
    assert_eq!(executor.aggro.evades_started, 0);
    assert_eq!(executor.aggro_committed_starts.len(), 2);
    assert!(executor.aggro_committed_stops.is_empty());
    let neutral = world.neutral;
    let neutral_packets: Vec<_> = executor
        .aggro
        .plan
        .events
        .iter()
        .filter(|event| event.source_guid == neutral)
        .map(|event| event.packet_bytes.clone())
        .collect();
    assert_eq!(
        neutral_packets,
        vec![
            wow_packet::packets::combat::AIReaction {
                unit_guid: neutral,
                reaction: wow_constants::creature::AiReaction::Hostile,
            }
            .to_bytes(),
            wow_packet::packets::combat::AttackStart {
                attacker: neutral,
                victim: world.striker,
            }
            .to_bytes(),
        ]
    );
}

#[test]
fn canonical_executor_matches_recorded_creature_melee_phase_like_cpp() {
    let (_world, executor, _, result) = d3a1_drive_like_cpp(
        "canonical_executor_matches_recorded_creature_melee_phase_like_cpp",
        95_130,
        |_| {},
    );
    assert_recorded_like_cpp("tick 0", result);
    // Both creatures counter-attack their own victim with the real body.
    assert_eq!(executor.melee.swings_ready, 2, "{:?}", executor.melee);
    assert_eq!(executor.melee.canonical_hits, 2);
    assert_eq!(executor.melee.commands.len(), 2);
}

/// Negative control: one extra creature-owned draw before the executor runs is
/// caught against the recorded expectations, for exactly that reason.
#[test]
fn canonical_executor_regression_detects_a_perturbed_draw_like_cpp() {
    let (_world, _executor, _, result) = d3a1_drive_like_cpp(
        "canonical_executor_regression_detects_a_perturbed_draw_like_cpp",
        95_140,
        |world| {
            let mut guard = world.canonical.lock().unwrap();
            let _ = guard
                .find_map_mut(0, 0)
                .unwrap()
                .map_mut()
                .with_creature_mut_like_cpp(world.hostile, wow_entities::Creature::roll_damage);
        },
    );
    let mismatches = result.expect_err("a perturbed RNG position must be caught");
    assert!(
        mismatches
            .iter()
            .any(|mismatch| mismatch.starts_with("hostile:") && mismatch.contains("draws=")),
        "the hostile creature's draw sequence is the reported difference: {mismatches:?}"
    );
}

#[test]
fn canonical_executor_does_not_republish_unchanged_values_like_cpp() {
    let world = d3a1_world_like_cpp(95_150);
    let neutral = world.neutral;
    let drain = |world: &D3a1WorldLikeCpp| -> usize {
        let mut guard = world.canonical.lock().unwrap();
        guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .send_object_updates_like_cpp()
            .unit_values_updates
            .iter()
            .filter(|update| update.guid == neutral)
            .count()
    };
    // The fixture's own setup writes are not part of the measurement.
    let _ = drain(&world);
    let health_before = world
        .canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_creature(neutral)
        .unwrap()
        .unit()
        .data()
        .health;

    let mut values_updates = 0;
    // Tick 1: the striker's swing changes the neutral creature's health once.
    let (plan, admission) = d3a1_admit_like_cpp(&world, D3A1_DIFF_MS_LIKE_CPP);
    let first = d3a1_executor_tick_like_cpp(&world, &admission);
    assert_eq!(
        first.player_melee.creature_hits, 1,
        "{:?}",
        first.player_melee
    );
    values_updates += drain(&world);
    let _ = world.canonical.lock().unwrap().abandon_tick_like_cpp(plan);
    // Tick 2: the striker has stopped swinging (`AttackStop`), so nothing about
    // the creature changes and nothing may be republished.
    {
        let mut guard = world.canonical.lock().unwrap();
        let striker = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(world.striker)
            .unwrap();
        striker.unit_mut().set_attacking(None);
    }
    let (plan, admission) = d3a1_admit_like_cpp(&world, 100);
    let second = d3a1_executor_tick_like_cpp(&world, &admission);
    assert!(second.executed_like_cpp());
    assert_eq!(second.player_melee.creature_hits, 0);
    values_updates += drain(&world);
    let _ = world.canonical.lock().unwrap().abandon_tick_like_cpp(plan);

    let health_after = world
        .canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_creature(neutral)
        .unwrap()
        .unit()
        .data()
        .health;
    assert_eq!(health_after, health_before - 7, "exactly one health change");
    assert_eq!(
        values_updates, 1,
        "one health change across two ticks publishes exactly one values update"
    );
}
