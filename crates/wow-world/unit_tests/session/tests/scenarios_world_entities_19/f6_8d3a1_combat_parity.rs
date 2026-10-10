//! #1263 F6-8D3a-1 regressions: the admitted canonical executor reaches
//! combat-phase parity with the legacy bridge.
//!
//! One seeded fixture is built twice, identically: two creatures and two
//! players on map 0/0. The hostile creature is engaged with one player (threat
//! from a hostile reaction), the neutral creature is engaged by the other per
//! the R6-fix semantics (`EngageWithTarget`, accepted by `_IsTargetAcceptable`
//! only because it is engaged) and is that player's melee victim. World **L**
//! is driven through the legacy bridge in the legacy loop order (player melee,
//! the movement phase's `Unit::Update` clock step, aggro plus its canonical
//! start/stop commit and publication reduction, creature melee); world **C**
//! is driven through `run_admitted_creature_combat_phases_isolated_like_cpp`.
//! The comparison covers each phase's outcome (counters, `RuntimePlan` packet
//! bytes and recipients, command lists), the creature-owned RNG positions, the
//! swing timers and clocks, the threat tables and the health/revision tuples.
//!
//! The legacy store enumerates creatures in `HashMap` order and the executor in
//! admission order, so events and commands are compared per source (a stable
//! sort keeps each creature's own sequence). The two creatures fight different
//! players, so no cross-creature result depends on that order.
//!
//! The attack-table roll (`rolled_melee_outcome_like_cpp`) draws from the
//! thread RNG rather than a creature stream, so it cannot be byte-compared
//! across two worlds; the fixture runs the store-less always-hit branch of the
//! same body, and the one-body construction is what carries the table.

use super::*;

const D3A1_HOSTILE_SEED_LIKE_CPP: u64 = 0xD3A1_0001;
const D3A1_NEUTRAL_SEED_LIKE_CPP: u64 = 0xD3A1_0002;
const D3A1_DIFF_MS_LIKE_CPP: u32 = 2_000;

struct D3a1WorldLikeCpp {
    manager: crate::map_manager::SharedMapManager,
    canonical: SharedCanonicalMapManager,
    _session: WorldSession,
    hostile: ObjectGuid,
    neutral: ObjectGuid,
    striker: ObjectGuid,
    defender: ObjectGuid,
    attackers: Vec<crate::session::PlayerMeleeAttackerSnapshotLikeCpp>,
    candidates: Vec<LegacyCreatureAggroCandidateLikeCpp>,
}

/// Faction template 14 is hostile to the player template 1 (enemy faction
/// 930); template 15 has the shape of 3.4.3 `FactionTemplate` 7: neutral.
fn d3a1_config_like_cpp() -> LegacyCreatureAggroConfigLikeCpp {
    LegacyCreatureAggroConfigLikeCpp {
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
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let hostile = test_creature_guid(base);
    let neutral = test_creature_guid(base + 1);
    let striker = ObjectGuid::create_player(1, base + 2);
    let defender = ObjectGuid::create_player(1, base + 3);
    register_test_creature_mirrored_like_cpp(
        &mut session,
        manager.clone(),
        &canonical,
        hostile,
        100,
    );
    register_test_creature_mirrored_like_cpp(
        &mut session,
        manager.clone(),
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
            creature.creature.unit_mut().set_faction(14);
            creature.creature.engage_with_target_like_cpp(defender);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(defender, 5.0);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.seed_runtime_rng_like_cpp(D3A1_HOSTILE_SEED_LIKE_CPP);
        })
        .expect("the hostile creature is admitted");
    session
        .mutate_world_creature(neutral, |creature| {
            creature.creature.unit_mut().set_faction(15);
            creature.creature.engage_with_target_like_cpp(striker);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.seed_runtime_rng_like_cpp(D3A1_NEUTRAL_SEED_LIKE_CPP);
        })
        .expect("the neutral creature is admitted");
    manager
        .write()
        .unwrap()
        .set_tick_owner(crate::map_manager::RuntimeTickOwner::GlobalLegacy);
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
        legacy_aggro_candidate_like_cpp(striker, Position::new(11.0, 10.0, 0.0, 0.0)),
        legacy_aggro_candidate_like_cpp(defender, Position::new(11.0, 10.2, 0.0, 0.0)),
    ];
    D3a1WorldLikeCpp {
        manager,
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

/// Every phase outcome of one tick, plus the committed aggro commands.
#[derive(Debug)]
struct D3a1TickLikeCpp {
    player_melee: crate::session::LegacyPlayerMeleeTickOutcomeLikeCpp,
    aggro: LegacyCreatureAggroTickOutcomeLikeCpp,
    committed_starts: Vec<crate::session::mailbox::CreatureAttackStartLikeCppCommand>,
    committed_stops: Vec<crate::session::mailbox::CreatureAttackStopLikeCppCommand>,
    melee: crate::session::LegacyCreatureMeleeTickOutcomeLikeCpp,
}

/// World L: the legacy bridge, in the legacy loop order and with the
/// world-server aggro commit the bridge applies before delivery.
fn d3a1_legacy_tick_like_cpp(world: &D3a1WorldLikeCpp, diff_ms: u32) -> D3a1TickLikeCpp {
    let config = d3a1_config_like_cpp();
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let player_melee = run_legacy_player_melee_tick_once_like_cpp(
        &world.manager,
        Some(&world.canonical),
        &world.attackers,
        diff_ms,
        &mut phase_state,
        &config,
    );
    // The legacy movement phase opens with `Unit::Update`'s clock step
    // (`creature_movement_tick.rs`); the fixture creatures have no generator,
    // so that step is the whole of what the phase would do to them.
    for guid in [world.hostile, world.neutral] {
        world
            .manager
            .write()
            .unwrap()
            .find_creature_mut(0, 0, guid)
            .unwrap()
            .advance_runtime_clock_like_cpp(diff_ms);
    }
    let mut aggro = run_legacy_creature_aggro_tick_once_with_config_and_canonical_like_cpp(
        &world.manager,
        Some(&world.canonical),
        &world.candidates,
        config.clone(),
    );
    let (committed_starts, committed_stops) = {
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
        (
            crate::session::committed_creature_combat_commands_like_cpp(&aggro.commands, &starts),
            crate::session::committed_creature_combat_commands_like_cpp(
                &aggro.stop_commands,
                &stops,
            ),
        )
    };
    let melee = run_legacy_creature_melee_tick_once_like_cpp(
        &world.manager,
        Some(&world.canonical),
        &config,
    );
    D3a1TickLikeCpp {
        player_melee,
        aggro,
        committed_starts,
        committed_stops,
        melee,
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

/// World C: the admitted canonical executor over the same tick.
fn d3a1_executor_tick_like_cpp(
    world: &D3a1WorldLikeCpp,
    admission: &AdmittedCreatureExecutionLikeCpp,
) -> AdmittedCreatureCombatPhasesOutcomeLikeCpp {
    let config = d3a1_config_like_cpp();
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let lease = std::sync::Arc::new(std::sync::Mutex::new(CreatureExecutionLeaseLikeCpp::new()));
    run_admitted_creature_combat_phases_isolated_like_cpp(
        admission,
        &world.canonical,
        &lease,
        AdmittedCreatureCombatPhaseInputsLikeCpp {
            player_melee_attackers: &world.attackers,
            player_melee_phase_state: &mut phase_state,
            aggro_candidates: &world.candidates,
            terrain: None,
            config: &config,
        },
    )
}

/// Drive both worlds through one tick and return `(legacy, executor)`.
fn d3a1_drive_both_like_cpp(
    base: i64,
    perturb_executor_world: impl FnOnce(&D3a1WorldLikeCpp),
) -> (
    D3a1WorldLikeCpp,
    D3a1TickLikeCpp,
    D3a1WorldLikeCpp,
    AdmittedCreatureCombatPhasesOutcomeLikeCpp,
) {
    let legacy_world = d3a1_world_like_cpp(base);
    let executor_world = d3a1_world_like_cpp(base);
    let (_plan, admission) = d3a1_admit_like_cpp(&executor_world, D3A1_DIFF_MS_LIKE_CPP);
    assert_eq!(admission.objects.len(), 2, "both creatures are admitted");
    perturb_executor_world(&executor_world);
    let legacy = d3a1_legacy_tick_like_cpp(&legacy_world, admission.diff_ms);
    let executor = d3a1_executor_tick_like_cpp(&executor_world, &admission);
    assert!(executor.executed_like_cpp(), "{:?}", executor.admission);
    assert_eq!(
        executor.clock_advanced_ms,
        2 * u64::from(admission.diff_ms),
        "one clock step per admitted creature"
    );
    (legacy_world, legacy, executor_world, executor)
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
fn d3a1_creature_state_like_cpp(creature: &wow_entities::Creature) -> String {
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

fn d3a1_legacy_creature_like_cpp(world: &D3a1WorldLikeCpp, guid: ObjectGuid) -> String {
    let guard = world.manager.read().unwrap();
    d3a1_creature_state_like_cpp(&guard.find_creature(0, 0, guid).unwrap().creature)
}

fn d3a1_canonical_creature_like_cpp(world: &D3a1WorldLikeCpp, guid: ObjectGuid) -> String {
    let guard = world.canonical.lock().unwrap();
    d3a1_creature_state_like_cpp(
        guard
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_creature(guid)
            .unwrap(),
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

/// The parity check. `Err` names every observable that differs.
fn d3a1_parity_like_cpp(
    legacy_world: &D3a1WorldLikeCpp,
    legacy: &D3a1TickLikeCpp,
    executor_world: &D3a1WorldLikeCpp,
    executor: &AdmittedCreatureCombatPhasesOutcomeLikeCpp,
) -> Result<(), Vec<String>> {
    let mut mismatches = Vec::new();
    let mut check = |label: &str, left: String, right: String| {
        if left != right {
            mismatches.push(format!("{label}:\n  legacy:   {left}\n  executor: {right}"));
        }
    };
    check(
        "player melee counters",
        d3a1_counters_like_cpp(&legacy.player_melee),
        d3a1_counters_like_cpp(&executor.player_melee),
    );
    check(
        "player melee commands",
        format!("{:?}", legacy.player_melee.commands),
        format!("{:?}", executor.player_melee.commands),
    );
    check(
        "aggro counters",
        d3a1_counters_like_cpp(&legacy.aggro),
        d3a1_counters_like_cpp(&executor.aggro),
    );
    check(
        "aggro plan",
        format!("{:?}", d3a1_events_like_cpp(&legacy.aggro.plan.events)),
        format!("{:?}", d3a1_events_like_cpp(&executor.aggro.plan.events)),
    );
    check(
        "aggro committed starts",
        format!("{:?}", d3a1_sorted_debug_like_cpp(&legacy.committed_starts)),
        format!(
            "{:?}",
            d3a1_sorted_debug_like_cpp(&executor.aggro_committed_starts)
        ),
    );
    check(
        "aggro committed stops",
        format!("{:?}", d3a1_sorted_debug_like_cpp(&legacy.committed_stops)),
        format!(
            "{:?}",
            d3a1_sorted_debug_like_cpp(&executor.aggro_committed_stops)
        ),
    );
    check(
        "melee counters",
        d3a1_counters_like_cpp(&legacy.melee),
        d3a1_counters_like_cpp(&executor.melee),
    );
    check(
        "melee plan",
        format!("{:?}", d3a1_events_like_cpp(&legacy.melee.plan.events)),
        format!("{:?}", d3a1_events_like_cpp(&executor.melee.plan.events)),
    );
    check(
        "melee commands",
        format!("{:?}", d3a1_sorted_debug_like_cpp(&legacy.melee.commands)),
        format!("{:?}", d3a1_sorted_debug_like_cpp(&executor.melee.commands)),
    );
    for (label, guid) in [
        ("hostile", legacy_world.hostile),
        ("neutral", legacy_world.neutral),
    ] {
        check(
            label,
            d3a1_legacy_creature_like_cpp(legacy_world, guid),
            d3a1_canonical_creature_like_cpp(executor_world, guid),
        );
    }
    for (label, guid) in [
        ("striker", legacy_world.striker),
        ("defender", legacy_world.defender),
    ] {
        check(
            label,
            d3a1_player_like_cpp(legacy_world, guid),
            d3a1_player_like_cpp(executor_world, guid),
        );
    }
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(mismatches)
    }
}

fn d3a1_assert_parity_like_cpp(
    legacy_world: &D3a1WorldLikeCpp,
    legacy: &D3a1TickLikeCpp,
    executor_world: &D3a1WorldLikeCpp,
    executor: &AdmittedCreatureCombatPhasesOutcomeLikeCpp,
) {
    if let Err(mismatches) = d3a1_parity_like_cpp(legacy_world, legacy, executor_world, executor) {
        panic!("parity broken:\n{}", mismatches.join("\n"));
    }
}

#[test]
fn canonical_executor_matches_legacy_bridge_phase_by_phase_like_cpp() {
    let (legacy_world, legacy, executor_world, executor) = d3a1_drive_both_like_cpp(95_100, |_| {});
    d3a1_assert_parity_like_cpp(&legacy_world, &legacy, &executor_world, &executor);
}

#[test]
fn canonical_executor_matches_legacy_bridge_player_melee_phase_like_cpp() {
    let (legacy_world, legacy, executor_world, executor) = d3a1_drive_both_like_cpp(95_110, |_| {});
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
    assert_eq!(
        d3a1_counters_like_cpp(&legacy.player_melee),
        d3a1_counters_like_cpp(&executor.player_melee)
    );
    assert_eq!(
        format!("{:?}", legacy.player_melee.commands),
        format!("{:?}", executor.player_melee.commands),
        "same values update, health and presentation bytes"
    );
    assert_eq!(
        d3a1_legacy_creature_like_cpp(&legacy_world, legacy_world.neutral),
        d3a1_canonical_creature_like_cpp(&executor_world, executor_world.neutral),
        "the neutral victim's health, revision and threat match"
    );
}

#[test]
fn canonical_executor_matches_legacy_bridge_aggro_phase_like_cpp() {
    use wow_packet::ServerPacket;

    let (legacy_world, legacy, executor_world, executor) = d3a1_drive_both_like_cpp(95_120, |_| {});
    // R6-fix: both creatures switch to their engaged victim and publish
    // `SMSG_AI_REACTION` (hostile) then `SMSG_ATTACK_START`, with no evade.
    assert_eq!(executor.aggro.victim_switches, 2, "{:?}", executor.aggro);
    assert_eq!(executor.aggro.evades_started, 0);
    assert_eq!(executor.aggro_committed_starts.len(), 2);
    assert!(executor.aggro_committed_stops.is_empty());
    let neutral = executor_world.neutral;
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
                victim: executor_world.striker,
            }
            .to_bytes(),
        ]
    );
    assert_eq!(
        d3a1_counters_like_cpp(&legacy.aggro),
        d3a1_counters_like_cpp(&executor.aggro)
    );
    assert_eq!(
        d3a1_events_like_cpp(&legacy.aggro.plan.events),
        d3a1_events_like_cpp(&executor.aggro.plan.events)
    );
    assert_eq!(
        d3a1_sorted_debug_like_cpp(&legacy.committed_starts),
        d3a1_sorted_debug_like_cpp(&executor.aggro_committed_starts)
    );
    for guid in [legacy_world.hostile, legacy_world.neutral] {
        assert_eq!(
            d3a1_legacy_creature_like_cpp(&legacy_world, guid),
            d3a1_canonical_creature_like_cpp(&executor_world, guid),
            "threat tables and victims match"
        );
    }
}

#[test]
fn canonical_executor_matches_legacy_bridge_creature_melee_phase_like_cpp() {
    let (legacy_world, legacy, executor_world, executor) = d3a1_drive_both_like_cpp(95_130, |_| {});
    // Both creatures counter-attack their own victim with the real body.
    assert_eq!(executor.melee.swings_ready, 2, "{:?}", executor.melee);
    assert_eq!(executor.melee.canonical_hits, 2);
    assert_eq!(executor.melee.commands.len(), 2);
    assert_eq!(
        d3a1_counters_like_cpp(&legacy.melee),
        d3a1_counters_like_cpp(&executor.melee)
    );
    assert_eq!(
        d3a1_sorted_debug_like_cpp(&legacy.melee.commands),
        d3a1_sorted_debug_like_cpp(&executor.melee.commands),
        "same damage, hit info, victim state and health revisions"
    );
    for guid in [legacy_world.striker, legacy_world.defender] {
        assert_eq!(
            d3a1_player_like_cpp(&legacy_world, guid),
            d3a1_player_like_cpp(&executor_world, guid)
        );
    }
    for guid in [legacy_world.hostile, legacy_world.neutral] {
        assert_eq!(
            d3a1_legacy_creature_like_cpp(&legacy_world, guid),
            d3a1_canonical_creature_like_cpp(&executor_world, guid),
            "the swing consumed the same draws and rearmed the same timer"
        );
    }
}

/// Negative control: one extra creature-owned draw before the executor runs is
/// caught by the parity check, for exactly that reason.
#[test]
fn canonical_executor_parity_detects_a_perturbed_draw_like_cpp() {
    let (legacy_world, legacy, executor_world, executor) =
        d3a1_drive_both_like_cpp(95_140, |world| {
            let mut guard = world.canonical.lock().unwrap();
            let _ = guard
                .find_map_mut(0, 0)
                .unwrap()
                .map_mut()
                .with_creature_mut_like_cpp(world.hostile, wow_entities::Creature::roll_damage);
        });
    let mismatches = d3a1_parity_like_cpp(&legacy_world, &legacy, &executor_world, &executor)
        .expect_err("a perturbed RNG position must break parity");
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
