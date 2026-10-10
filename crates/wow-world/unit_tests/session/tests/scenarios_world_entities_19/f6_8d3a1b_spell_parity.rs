//! #1263 F6-8D3a-1b regressions: the admitted canonical executor runs the
//! creature spell phase with parity to the legacy bridge.
//!
//! One seeded fixture is built twice, identically: a `CombatAI` creature that
//! knows the instant single-target school-damage spell 15691 (`AICOND_COMBAT`,
//! so the first tick initialises its `EventMap` schedule and a later tick casts
//! it), engaged with one player. `NO_MELEE` keeps creature melee out of the
//! measurement. World **L** is driven through the legacy bridge in the legacy
//! loop order (the movement phase — F6-8D3a-2; before it only its clock step —,
//! aggro plus its commit, spell, creature melee); world **C** through
//! `run_admitted_creature_combat_phases_isolated_like_cpp`.
//!
//! Every draw of the spell path comes from the creature-owned runtime RNG: the
//! `ScheduleEvent` initial and repeat delays
//! (`random_creature_spell_delay_like_cpp`) and the spell hit roll
//! (`random_creature_spell_hit_roll_like_cpp`). No part of it draws from the
//! thread RNG. One SMSG_SPELL_GO field is process-clock time, not RNG:
//! `SpellCastData::CastTime` is C++ `getMSTime()` (`Spell::SendSpellGo`),
//! represented by `game_time_ms_like_cpp()`; the two worlds sample it a few
//! microseconds apart, so the comparison masks exactly those four bytes and
//! compares every other byte of START and GO. Creature melee would reach
//! the thread-RNG attack table with a spell store configured; `NO_MELEE` rejects
//! it before that roll (the D3a-1 hit-table boundary is unchanged).
//!
//! In world L the caster's schedule, clock and RNG live on the legacy
//! representation and its spell cooldowns on the canonical mirror (where the
//! legacy validation reads and starts them); in world C all of them live on the
//! canonical incarnation. The comparison reads each from where its owner keeps
//! it.

use super::*;

const D3A1B_SPELL_ID_LIKE_CPP: i32 = 15_691;
const D3A1B_DIFF_MS_LIKE_CPP: u32 = 2_000;
/// Past the longest initial `ScheduleEvent` delay (2 x the 5 s minimum).
const D3A1B_DUE_ELAPSED_MS_LIKE_CPP: u64 = 20_000;

struct D3a1bWorldLikeCpp {
    manager: crate::map_manager::SharedMapManager,
    canonical: SharedCanonicalMapManager,
    _session: WorldSession,
    caster: ObjectGuid,
    victim: ObjectGuid,
    candidates: Vec<LegacyCreatureAggroCandidateLikeCpp>,
}

/// The first seed whose initial `ScheduleEvent` delay is followed by a HIT
/// hit-roll, the two creature-owned draws the phase consumes in that order.
fn d3a1b_hit_seed_like_cpp() -> u64 {
    let miss_threshold = creature_melee_spell_miss_threshold_3_3_5_like_cpp();
    (0_u64..10_000)
        .find(|seed| {
            let mut rng = StdRng::seed_from_u64(*seed);
            let _initial_delay = rng.gen_range(5_000_u64..=10_000_u64);
            rng.gen_range(0..=9_999_u32) >= miss_threshold
        })
        .expect("a HIT seed exists")
}

/// The spell configuration of the 15691 legacy fixture, with a real spell
/// cooldown so the cast starts one on its caster.
fn d3a1b_config_like_cpp() -> LegacyCreatureAggroConfigLikeCpp {
    let spell_id = D3A1B_SPELL_ID_LIKE_CPP;
    let mut spell = creature_ai_test_spell_info_like_cpp(spell_id, 6, 0);
    spell.recovery_time_ms = 0;
    spell.effect_base_points = 64;
    spell.effects[0].effect_base_points = 64;
    let mut config = creature_ai_spell_test_config_like_cpp(spell, false, 5.0);
    let mut attributes = [0_u32; 15];
    attributes[0] = 0x000d_0010;
    Arc::get_mut(config.spell_store.as_mut().unwrap())
        .unwrap()
        .insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    let mut misc = spell_misc_entry_like_cpp(8_320, spell_id as u32, 2);
    misc.attributes = attributes.map(|attribute| attribute as i32);
    config.spell_misc_store = Some(Arc::new(wow_data::SpellMiscStore::from_entries([misc])));
    let mut range = spell_range_entry_like_cpp(2, 0.0, 5.0);
    range.flags = 1;
    config.spell_range_store = Some(Arc::new(wow_data::SpellRangeStore::from_entries([range])));
    config.spell_cooldowns_store = Some(Arc::new(wow_data::SpellCooldownsStore::from_entries([
        wow_data::SpellCooldownsEntry {
            id: 8_321,
            difficulty_id: 0,
            category_recovery_time: 0,
            recovery_time: 6_000,
            start_recovery_time: 1_000,
            spell_id: spell_id as u32,
        },
    ])));
    config
}

/// Build the fixture. Every call yields the same state.
fn d3a1b_world_like_cpp(base: i64) -> D3a1bWorldLikeCpp {
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let caster = test_creature_guid(base);
    let victim = ObjectGuid::create_player(1, base + 1);
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, caster, victim);
    {
        let mut canonical = canonical.lock().unwrap();
        let map = canonical.find_map_mut(0, 0).unwrap().map_mut();
        map.get_typed_creature_mut(caster)
            .unwrap()
            .unit_mut()
            .set_level(64);
        let player = map.get_typed_player_mut(victim).unwrap();
        player.unit_mut().set_level(80);
        player
            .unit_mut()
            .world_mut()
            .relocate(Position::new(14.0, 10.0, 0.0, 0.0));
    }
    register_test_creature_mirrored_like_cpp(&mut session, manager.clone(), &canonical, caster, 25);
    let seed = d3a1b_hit_seed_like_cpp();
    session
        .mutate_world_creature(caster, |creature| {
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("CombatAI", String::new());
            creature
                .creature
                .set_spell(0, D3A1B_SPELL_ID_LIKE_CPP as u32);
            creature.enter_combat(victim);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(victim, 5.0);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            let mut static_flags = [0; 8];
            static_flags[0] = wow_constants::creature::CreatureStaticFlags::NO_MELEE_FLEE.bits();
            creature
                .creature
                .set_static_flags_runtime_like_cpp(static_flags);
            creature.seed_runtime_rng_like_cpp(seed);
        })
        .expect("the caster is admitted");
    manager
        .write()
        .unwrap()
        .set_tick_owner(crate::map_manager::RuntimeTickOwner::GlobalLegacy);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        victim,
        Position::new(14.0, 10.0, 0.0, 0.0),
    )];
    D3a1bWorldLikeCpp {
        manager,
        canonical,
        _session: session,
        caster,
        victim,
        candidates,
    }
}

/// The phases of one legacy tick this fixture observes.
#[derive(Debug)]
struct D3a1bTickLikeCpp {
    movement: crate::session::LegacyCreatureMovementTickOutcomeLikeCpp,
    aggro: LegacyCreatureAggroTickOutcomeLikeCpp,
    spell: crate::session::LegacyCreatureSpellTickOutcomeLikeCpp,
    melee: crate::session::LegacyCreatureMeleeTickOutcomeLikeCpp,
}

/// World L: the legacy bridge in the legacy loop order.
fn d3a1b_legacy_tick_like_cpp(world: &D3a1bWorldLikeCpp, diff_ms: u32) -> D3a1bTickLikeCpp {
    let config = d3a1b_config_like_cpp();
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let _ = run_legacy_player_melee_tick_once_like_cpp(
        &world.manager,
        Some(&world.canonical),
        &[],
        diff_ms,
        &mut phase_state,
        &config,
    );
    // #1263 F6-8D3a-2: the complete legacy movement phase.
    let chase_targets = super::f6_8d3a2_movement_lifecycle_parity::d3a2_chase_targets_like_cpp(
        &world.canonical,
        &[world.victim],
    );
    let movement = super::f6_8d3a2_movement_lifecycle_parity::d3a2_legacy_movement_tick_like_cpp(
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
    D3a1bTickLikeCpp {
        movement,
        aggro,
        spell,
        melee,
    }
}

/// World C: one admitted canonical tick through the executor. Returns the
/// outcome and the admitted diff (an abandoned earlier tick carries its time
/// into the next plan), which the legacy world then replays.
fn d3a1b_executor_tick_like_cpp(
    world: &D3a1bWorldLikeCpp,
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
        0x0D3A_1B00,
        plan.epoch_like_cpp(),
        plan.effective_diff_ms(),
        plan.updated_maps_like_cpp()
            .iter()
            .map(|participant| (participant.key, participant.incarnation)),
        [(key, world.caster)],
    )
    .expect("the canonical owner is readable");
    assert_eq!(admission.objects.len(), 1, "the caster is admitted");
    let config = d3a1b_config_like_cpp();
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let lease = std::sync::Arc::new(std::sync::Mutex::new(CreatureExecutionLeaseLikeCpp::new()));
    let chase_targets = super::f6_8d3a2_movement_lifecycle_parity::d3a2_chase_targets_like_cpp(
        &world.canonical,
        &[world.victim],
    );
    let mmap_config = super::f6_8d3a2_movement_lifecycle_parity::d3a2_mmap_config_like_cpp();
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
    let _ = world.canonical.lock().unwrap().abandon_tick_like_cpp(plan);
    (outcome, admission.diff_ms)
}

/// Move both creature clocks past the scheduled event, identically.
fn d3a1b_make_cast_due_like_cpp(
    legacy_world: &D3a1bWorldLikeCpp,
    executor_world: &D3a1bWorldLikeCpp,
) {
    legacy_world
        .manager
        .write()
        .unwrap()
        .find_creature_mut(0, 0, legacy_world.caster)
        .unwrap()
        .backdate_runtime_clock_for_test(Duration::from_millis(D3A1B_DUE_ELAPSED_MS_LIKE_CPP));
    executor_world
        .canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_creature_mut(executor_world.caster)
        .unwrap()
        .runtime_like_cpp_mut()
        .set_runtime_elapsed_ms_like_cpp(D3A1B_DUE_ELAPSED_MS_LIKE_CPP);
}

/// The caster's spell-phase observables: RNG position (next draws of a clone),
/// RNG authority, clock, swing timer, victim, cast state, `EventMap` schedule
/// and the spell-history cooldowns read from `cooldowns`.
fn d3a1b_caster_state_like_cpp(
    runtime: &wow_entities::Creature,
    cooldowns: &wow_entities::Creature,
) -> String {
    let mut probe = runtime.clone();
    let draws: Vec<_> = (0..4).map(|_| probe.roll_damage()).collect();
    let schedule = runtime.runtime_like_cpp();
    format!(
        "draws={draws:?} rng_complete={} elapsed={} last_swing={} swing_timer={} victim={:?} \
         casting={} schedule_initialized={} due={:?} epoch={} cooldowns={:?}",
        runtime.runtime_rng_authority_complete_like_cpp(),
        runtime.runtime_elapsed_ms_like_cpp(),
        runtime.ai_ownership().last_swing_ms,
        runtime.ai_ownership().swing_timer_ms,
        runtime.ai_ownership().combat_target,
        runtime.unit().has_unit_state(UnitState::CASTING.bits()),
        schedule.creature_spell_schedule_initialized_like_cpp,
        schedule.creature_spell_due_at_ms_like_cpp,
        runtime.creature_spell_engagement_epoch_like_cpp(),
        cooldowns.unit().subsystems().spells.history,
    )
}

fn d3a1b_legacy_caster_like_cpp(world: &D3a1bWorldLikeCpp) -> String {
    let legacy = world.manager.read().unwrap();
    let canonical = world.canonical.lock().unwrap();
    d3a1b_caster_state_like_cpp(
        &legacy.find_creature(0, 0, world.caster).unwrap().creature,
        canonical
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_creature(world.caster)
            .unwrap(),
    )
}

fn d3a1b_canonical_caster_like_cpp(world: &D3a1bWorldLikeCpp) -> String {
    let canonical = world.canonical.lock().unwrap();
    let caster = canonical
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_creature(world.caster)
        .unwrap();
    d3a1b_caster_state_like_cpp(caster, caster)
}

fn d3a1b_victim_like_cpp(world: &D3a1bWorldLikeCpp) -> String {
    let canonical = world.canonical.lock().unwrap();
    let unit = canonical
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(world.victim)
        .unwrap()
        .unit();
    format!(
        "health={} revision={} auras={:?} in_combat={}",
        unit.data().health,
        unit.health_state_revision_like_cpp(),
        unit.subsystems().auras.applied_auras,
        unit.subsystems().combat.has_combat(),
    )
}

/// Every counter of an outcome, with its plan and command lists removed.
fn d3a1b_counters_like_cpp<T: std::fmt::Debug>(outcome: &T) -> String {
    let rendered = format!("{outcome:?}");
    let cut = ["plan:", "commands:", "stop_commands:"]
        .iter()
        .filter_map(|field| rendered.find(field))
        .min()
        .unwrap_or(rendered.len());
    rendered[..cut].to_string()
}

/// The spell plan with the GO `CastTime` (`getMSTime()`) masked. The field
/// sits 16 bytes after the spell ID (spell ID, visual, flags, flags-ex).
fn d3a1b_spell_events_like_cpp(events: &[RuntimeEvent]) -> String {
    let spell_id = D3A1B_SPELL_ID_LIKE_CPP.to_le_bytes();
    let mask = |bytes: &mut Vec<u8>| {
        let at = bytes
            .windows(4)
            .position(|window| window == spell_id)
            .expect("GO carries the spell ID")
            + 16;
        bytes[at..at + 4].fill(0);
    };
    let mut events = events.to_vec();
    for event in &mut events {
        if let crate::map_manager::RecipientRule::NearbyVisibleDurableSpellCast {
            basic_go_packet_bytes,
            full_go_packet_bytes,
            ..
        } = &mut event.recipients
        {
            mask(basic_go_packet_bytes);
            mask(full_go_packet_bytes);
        }
    }
    format!("{events:?}")
}

/// The parity check over one tick. `Err` names every observable that differs.
fn d3a1b_parity_like_cpp(
    legacy_world: &D3a1bWorldLikeCpp,
    legacy: &D3a1bTickLikeCpp,
    executor_world: &D3a1bWorldLikeCpp,
    executor: &AdmittedCreatureCombatPhasesOutcomeLikeCpp,
) -> Result<(), Vec<String>> {
    let mut mismatches = Vec::new();
    let mut check = |label: &str, left: String, right: String| {
        if left != right {
            mismatches.push(format!("{label}:\n  legacy:   {left}\n  executor: {right}"));
        }
    };
    check(
        "spell phase executed",
        "true".to_string(),
        executor.spell_phase_executed.to_string(),
    );
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
        "aggro counters",
        d3a1b_counters_like_cpp(&legacy.aggro),
        d3a1b_counters_like_cpp(&executor.aggro),
    );
    check(
        "aggro plan",
        format!("{:?}", legacy.aggro.plan.events),
        format!("{:?}", executor.aggro.plan.events),
    );
    check(
        "spell counters",
        d3a1b_counters_like_cpp(&legacy.spell),
        d3a1b_counters_like_cpp(&executor.spell),
    );
    check(
        "spell plan",
        d3a1b_spell_events_like_cpp(&legacy.spell.plan.events),
        d3a1b_spell_events_like_cpp(&executor.spell.plan.events),
    );
    check(
        "melee counters",
        d3a1b_counters_like_cpp(&legacy.melee),
        d3a1b_counters_like_cpp(&executor.melee),
    );
    check(
        "melee commands",
        format!("{:?}", legacy.melee.commands),
        format!("{:?}", executor.melee.commands),
    );
    check(
        "caster",
        d3a1b_legacy_caster_like_cpp(legacy_world),
        d3a1b_canonical_caster_like_cpp(executor_world),
    );
    check(
        "victim",
        d3a1b_victim_like_cpp(legacy_world),
        d3a1b_victim_like_cpp(executor_world),
    );
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(mismatches)
    }
}

fn d3a1b_assert_parity_like_cpp(
    legacy_world: &D3a1bWorldLikeCpp,
    legacy: &D3a1bTickLikeCpp,
    executor_world: &D3a1bWorldLikeCpp,
    executor: &AdmittedCreatureCombatPhasesOutcomeLikeCpp,
) {
    if let Err(mismatches) = d3a1b_parity_like_cpp(legacy_world, legacy, executor_world, executor) {
        panic!("parity broken:\n{}", mismatches.join("\n"));
    }
}

/// Drive both worlds through the schedule tick and the cast tick, checking
/// parity after each. `perturb_executor_world` runs on world C right before
/// its cast tick.
fn d3a1b_drive_both_like_cpp(
    base: i64,
    perturb_executor_world: impl FnOnce(&D3a1bWorldLikeCpp),
) -> (
    D3a1bWorldLikeCpp,
    D3a1bTickLikeCpp,
    D3a1bWorldLikeCpp,
    AdmittedCreatureCombatPhasesOutcomeLikeCpp,
) {
    let legacy_world = d3a1b_world_like_cpp(base);
    let executor_world = d3a1b_world_like_cpp(base);

    // Tick 1: `UpdateVictim` keeps the engaged victim, and the first spell tick
    // initialises the `AICOND_COMBAT` schedule (one creature-owned delay draw).
    let (executor, diff_ms) = d3a1b_executor_tick_like_cpp(&executor_world, D3A1B_DIFF_MS_LIKE_CPP);
    let legacy = d3a1b_legacy_tick_like_cpp(&legacy_world, diff_ms);
    assert_eq!(
        executor.spell.schedules_initialized, 1,
        "{:?}",
        executor.spell
    );
    assert_eq!(executor.spell.casts_ready, 0);
    assert!(executor.spell.plan.events.is_empty());
    d3a1b_assert_parity_like_cpp(&legacy_world, &legacy, &executor_world, &executor);

    // Tick 2: the scheduled event is due and `DoCast` runs.
    d3a1b_make_cast_due_like_cpp(&legacy_world, &executor_world);
    perturb_executor_world(&executor_world);
    let (executor, diff_ms) = d3a1b_executor_tick_like_cpp(&executor_world, 100);
    let legacy = d3a1b_legacy_tick_like_cpp(&legacy_world, diff_ms);
    (legacy_world, legacy, executor_world, executor)
}

#[test]
fn canonical_executor_matches_legacy_bridge_spell_phase_like_cpp() {
    let (legacy_world, legacy, executor_world, executor) =
        d3a1b_drive_both_like_cpp(95_200, |_| {});

    // The phase really cast: one represented HIT, START+GO published.
    assert!(executor.spell_phase_executed);
    assert_eq!(executor.spell.casts_ready, 1, "{:?}", executor.spell);
    assert_eq!(executor.spell.canonical_cast_preconditions_passed, 1);
    assert_eq!(executor.spell.spell_hits, 1);
    // The HIT tombstones exact spell RNG authority, so the repeat
    // `ScheduleEvent` draw that follows `DoCast` is refused, as in the bridge.
    assert_eq!(executor.spell.runtime_rng_authority_rejections, 1);
    assert_eq!(executor.spell.plan.events.len(), 1);
    let (start, go) =
        decode_atomic_creature_spell_wire_pair_like_cpp(&executor.spell.plan.events[0]);
    assert_eq!(start.opcode, ServerOpcodes::SpellStart as u16);
    assert_eq!(go.opcode, ServerOpcodes::SpellGo as u16);
    assert_eq!(start.spell_id, D3A1B_SPELL_ID_LIKE_CPP);
    assert_eq!(go.hit_targets, vec![executor_world.victim]);
    assert!(go.miss_targets.is_empty());
    // The deferred publication carries the spell plan between aggro and melee.
    assert_eq!(
        executor.deferred_plan_events_like_cpp().len(),
        executor.aggro.plan.events.len() + 1 + executor.melee.plan.events.len()
    );
    // NO_MELEE: creature melee is rejected before any attack-table roll.
    assert_eq!(executor.melee.canonical_hits, 0);
    {
        let canonical = executor_world.canonical.lock().unwrap();
        let caster = canonical
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_creature(executor_world.caster)
            .unwrap();
        assert!(!caster.runtime_rng_authority_complete_like_cpp());
        assert!(
            caster.unit().subsystems().spells.history.has_cooldown(
                D3A1B_SPELL_ID_LIKE_CPP as u32,
                0,
                caster.runtime_elapsed_ms_like_cpp(),
            ),
            "the successful cast started its 6 s cooldown"
        );
    }

    d3a1b_assert_parity_like_cpp(&legacy_world, &legacy, &executor_world, &executor);
    // SMSG_SPELL_START carries no clock field: its bytes compare directly.
    assert_eq!(
        legacy.spell.plan.events[0].packet_bytes,
        executor.spell.plan.events[0].packet_bytes
    );
    let (legacy_start, legacy_go) =
        decode_atomic_creature_spell_wire_pair_like_cpp(&legacy.spell.plan.events[0]);
    assert_eq!(
        legacy_start.cast_id, start.cast_id,
        "same map cast GUID sequence"
    );
    assert_eq!(legacy_go.hit_targets, go.hit_targets);
}

/// Negative control: one extra creature-owned draw before the cast tick moves
/// the hit roll, and parity reports the caster's draw sequence.
#[test]
fn canonical_executor_spell_parity_detects_a_perturbed_draw_like_cpp() {
    let (legacy_world, legacy, executor_world, executor) =
        d3a1b_drive_both_like_cpp(95_210, |world| {
            let mut guard = world.canonical.lock().unwrap();
            let _ = guard
                .find_map_mut(0, 0)
                .unwrap()
                .map_mut()
                .with_creature_mut_like_cpp(
                    world.caster,
                    wow_entities::Creature::random_creature_spell_hit_roll_like_cpp,
                );
        });
    let mismatches = d3a1b_parity_like_cpp(&legacy_world, &legacy, &executor_world, &executor)
        .expect_err("a perturbed RNG position must break spell parity");
    assert!(
        mismatches
            .iter()
            .any(|mismatch| mismatch.starts_with("caster:") && mismatch.contains("draws=")),
        "the caster's draw sequence is a reported difference: {mismatches:?}"
    );
}
