//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn legacy_creature_aggro_tick_once_enters_combat_and_returns_command_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_007);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.creature.unit_mut().set_level(25);
            creature
                .begin_move_spline_like_cpp(Position::new(20.0, 10.0, 0.0, 0.0))
                .expect("launch pre-aggro wander spline");
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_008);
    let candidates = vec![
        legacy_aggro_candidate_like_cpp(
            ObjectGuid::create_player(1, 91_009),
            Position::new(50.0, 50.0, 0.0, 0.0),
        ),
        legacy_aggro_candidate_like_cpp(player, Position::new(10.5, 10.5, 0.0, 0.0)),
    ];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(3.0),
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.candidates_seen, 2);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    let command = &outcome.commands[0];
    assert_eq!(command.attacker_guid, creature_guid);
    assert_eq!(command.victim_guid, player);
    assert_eq!(command.map_id, 0);
    assert_eq!(command.instance_id, 0);
    assert_eq!(
        outcome.movement_interrupts, 1,
        "C++ AttackStart installs chase and interrupts lower-priority wander in the same update"
    );
    assert_eq!(outcome.plan.events.len(), 2);
    let stop_event = &outcome.plan.events[0];
    assert_eq!(stop_event.source_guid, creature_guid);
    assert!(matches!(
        &stop_event.recipients,
        crate::map_manager::RecipientRule::NearbyVisible {
            source_guid,
            map_id: 0,
            instance_id: 0,
            required_3d: false,
            ..
        } if *source_guid == creature_guid
    ));
    let mut stop_packet = wow_packet::WorldPacket::from_bytes(&stop_event.packet_bytes);
    assert_eq!(
        stop_packet.read_uint16().expect("opcode"),
        ServerOpcodes::OnMonsterMove as u16
    );
    let attack_event = &outcome.plan.events[1];
    assert!(matches!(
        &attack_event.recipients,
        crate::map_manager::RecipientRule::NearbyVisibleDurable { .. }
    ));
    let mut attack_packet = wow_packet::WorldPacket::from_bytes(&attack_event.packet_bytes);
    assert_eq!(
        attack_packet.read_uint16().expect("opcode"),
        ServerOpcodes::AttackStart as u16
    );
    let (combat_target, runtime_kind, has_active_spline) = {
        let guard = manager.read().unwrap();
        let creature = guard.find_creature(0, 0, creature_guid).unwrap();
        assert_eq!(
            creature
                .creature
                .unit()
                .subsystems()
                .combat
                .threat_value(player),
            Some(0.0),
            "C++ EngageWithTarget creates a zero-threat reference"
        );
        (
            creature.creature.ai_ownership().combat_target,
            creature.runtime_motion_master_current_kind_like_cpp(),
            creature.active_move_spline_like_cpp().is_some(),
        )
    };
    assert_eq!(combat_target, Some(player));
    assert_eq!(
        runtime_kind,
        Some(wow_movement::MovementGeneratorType::Chase)
    );
    assert!(!has_active_spline);
}
#[test]
fn legacy_creature_threat_switches_only_above_cpp_melee_threshold() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_211);
    let tank = ObjectGuid::create_player(1, 91_212);
    let challenger = ObjectGuid::create_player(1, 91_213);
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(tank);
            let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
            combat.add_threat(tank, 100.0);
            combat.add_threat(challenger, 109.0);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let candidates = vec![
        legacy_aggro_candidate_like_cpp(tank, Position::new(10.5, 10.0, 0.0, 0.0)),
        legacy_aggro_candidate_like_cpp(challenger, Position::new(10.5, 10.0, 0.0, 0.0)),
    ];

    let held = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(3.0),
    );
    assert_eq!(held.victim_switches, 0);
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target,
        Some(tank),
        "109% melee threat must not pull from the current tank"
    );

    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(challenger, 2.0);
        })
        .unwrap();
    let switched = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(3.0),
    );
    assert_eq!(switched.victim_switches, 1);
    assert_eq!(switched.commands.len(), 1);
    assert!(
        switched.stop_commands.is_empty(),
        "C++ AttackStop during victim selection preserves threat/combat references; cleanup commands are evade-only"
    );
    assert_eq!(switched.commands[0].victim_guid, challenger);
    assert_eq!(switched.commands[0].previous_victim_guid, Some(tank));
    assert!(matches!(
        switched.plan.events[0].recipients,
        crate::map_manager::RecipientRule::NearbyVisibleDurable { .. }
    ));
    assert_eq!(switched.plan.events.len(), 1);
    let mut attack_start =
        wow_packet::WorldPacket::from_bytes(&switched.plan.events[0].packet_bytes);
    assert_eq!(
        attack_start.read_uint16().expect("opcode"),
        ServerOpcodes::AttackStart as u16,
        "C++ Unit::Attack changes the attacker relationship and broadcasts only SendMeleeAttackStart for the new target"
    );
}
#[test]
fn legacy_creature_suppresses_unattackable_threat_reference_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_217);
    let suppressed = ObjectGuid::create_player(1, 91_218);
    let available = ObjectGuid::create_player(1, 91_219);
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(suppressed);
            let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
            combat.add_threat(suppressed, 500.0);
            combat.add_threat(available, 10.0);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let mut suppressed_candidate =
        legacy_aggro_candidate_like_cpp(suppressed, Position::new(10.5, 10.0, 0.0, 0.0));
    suppressed_candidate.player_school_immunity_mask = 0x1;
    suppressed_candidate.player_has_confuse_aura = true;
    suppressed_candidate.player_has_breakable_stun_aura = true;
    let candidates = vec![
        suppressed_candidate,
        legacy_aggro_candidate_like_cpp(available, Position::new(10.5, 10.0, 0.0, 0.0)),
    ];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(3.0),
    );

    assert_eq!(outcome.victim_switches, 1);
    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    assert_eq!(
        creature.creature.ai_ownership().combat_target,
        Some(available),
        "C++ ranks an online target ahead of a higher-threat suppressed reference"
    );
    assert!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_ref(suppressed)
            .is_some_and(wow_entities::ThreatReferenceState::is_suppressed)
    );
    drop(guard);

    let candidates_without_suppression = vec![
        legacy_aggro_candidate_like_cpp(suppressed, Position::new(10.5, 10.0, 0.0, 0.0)),
        legacy_aggro_candidate_like_cpp(available, Position::new(10.5, 10.0, 0.0, 0.0)),
    ];
    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates_without_suppression,
        legacy_aggro_hostile_config_with_rate_like_cpp(3.0),
    );
    assert_eq!(
        outcome.victim_switches, 0,
        "C++ EvaluateSuppressed(false) does not reactivate a reference merely because its suppressing aura disappeared"
    );
    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    assert_eq!(
        creature.creature.ai_ownership().combat_target,
        Some(available)
    );
    assert!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_ref(suppressed)
            .is_some_and(wow_entities::ThreatReferenceState::is_suppressed),
        "only new threat from this target or TauntUpdate may expire C++ suppression"
    );
    drop(guard);

    manager
        .write()
        .unwrap()
        .find_creature_mut(0, 0, creature_guid)
        .unwrap()
        .creature
        .unit_mut()
        .subsystems_mut()
        .combat
        .add_threat(suppressed, 1.0);
    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates_without_suppression,
        legacy_aggro_hostile_config_with_rate_like_cpp(3.0),
    );
    assert_eq!(
        outcome.victim_switches, 1,
        "C++ AddThreat reactivates a formerly suppressed target once ShouldBeSuppressed clears"
    );
    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    assert_eq!(
        creature.creature.ai_ownership().combat_target,
        Some(suppressed)
    );
    assert!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_ref(suppressed)
            .is_some_and(wow_entities::ThreatReferenceState::is_online)
    );
}
#[test]
fn legacy_creature_never_suppresses_taunting_victim_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_220);
    let tank = ObjectGuid::create_player(1, 91_221);
    let taunter = ObjectGuid::create_player(1, 91_222);
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(tank);
            let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
            combat.add_threat(tank, 100.0);
            combat.add_threat(taunter, 1.0);
            creature
                .apply_taunt_aura_like_cpp(taunter, 91_220, 1, 5_000)
                .expect("apply represented taunt");
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let mut taunter_candidate =
        legacy_aggro_candidate_like_cpp(taunter, Position::new(10.5, 10.0, 0.0, 0.0));
    taunter_candidate.player_damage_immunity_mask = 0x1;
    taunter_candidate.player_has_confuse_aura = true;
    taunter_candidate.player_has_breakable_stun_aura = true;
    let candidates = vec![
        legacy_aggro_candidate_like_cpp(tank, Position::new(10.5, 10.0, 0.0, 0.0)),
        taunter_candidate,
    ];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(3.0),
    );

    assert_eq!(outcome.victim_switches, 1);
    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    assert_eq!(
        creature.creature.ai_ownership().combat_target,
        Some(taunter)
    );
    assert!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_ref(taunter)
            .is_some_and(wow_entities::ThreatReferenceState::is_online),
        "C++ ThreatReference::ShouldBeSuppressed exempts a taunting victim"
    );
}
#[test]
fn legacy_creature_without_online_threat_enters_evade_and_clears_combat_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_214);
    let vanished = ObjectGuid::create_player(1, 91_215);
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(vanished);
            creature.creature.unit_mut().set_health(40);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(vanished, 25.0);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(ObjectGuid::create_player(1, 91_216), 5.0);
            creature.creature.set_tapped_by_player(vanished, &[]);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &[],
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.evades_started, 1);
    assert_eq!(
        outcome.stop_commands.len(),
        2,
        "C++ CombatStop removes reciprocal combat references for every participant"
    );
    assert!(
        outcome
            .stop_commands
            .iter()
            .any(|command| command.victim_guid == vanished)
    );
    assert_eq!(outcome.plan.events.len(), 1);
    let mut stop_packet = wow_packet::WorldPacket::from_bytes(&outcome.plan.events[0].packet_bytes);
    assert_eq!(
        stop_packet.read_uint16().expect("opcode"),
        ServerOpcodes::AttackStop as u16,
        "C++ CombatStop broadcasts SMSG_ATTACK_STOP before returning home"
    );
    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    assert_eq!(creature.state(), wow_entities::CreatureAiState::Returning);
    assert_eq!(
        creature.creature.unit().data().health,
        40,
        "C++ does not restore spawn health until the home generator finalizes"
    );
    assert_eq!(creature.creature.ai_ownership().combat_target, None);
    assert!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .is_threat_list_empty(true)
    );
    assert!(!creature.creature.has_loot_recipient());
}
#[test]
fn legacy_creature_threat_keeps_represented_non_player_victim_online_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let attacker_guid = test_creature_guid(91_230);
    let victim_guid = test_creature_guid(91_231);
    register_test_creature(&mut session, manager.clone(), attacker_guid, 100);
    register_test_creature(&mut session, manager.clone(), victim_guid, 100);
    session
        .mutate_world_creature(attacker_guid, |attacker| {
            attacker.creature.set_faction(14);
            attacker.enter_combat(victim_guid);
            attacker
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(victim_guid, 25.0);
        })
        .unwrap();
    session
        .mutate_world_creature(victim_guid, |victim| {
            victim.creature.set_faction(1);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &[],
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.evades_started, 0);
    let guard = manager.read().unwrap();
    let attacker = guard.find_creature(0, 0, attacker_guid).unwrap();
    assert_eq!(
        attacker.creature.ai_ownership().combat_target,
        Some(victim_guid)
    );
    assert!(
        attacker
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_ref(victim_guid)
            .is_some_and(wow_entities::ThreatReferenceState::is_available)
    );
    drop(guard);

    session
        .mutate_world_creature(victim_guid, |victim| {
            victim.creature.set_faction(14);
        })
        .unwrap();
    let invalidated = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &[],
        legacy_aggro_hostile_config_like_cpp(),
    );
    assert_eq!(
        invalidated.evades_started, 1,
        "C++ SelectVictim rejects a threatened creature that became friendly"
    );
}
#[test]
fn legacy_creature_call_assistance_accepts_represented_creature_victim_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let caller_guid = test_creature_guid(91_232);
    let assistant_guid = test_creature_guid(91_233);
    let victim_guid = test_creature_guid(91_234);
    register_test_creature(&mut session, manager.clone(), caller_guid, 100);
    register_test_creature(&mut session, manager.clone(), assistant_guid, 100);
    register_test_creature(&mut session, manager.clone(), victim_guid, 100);
    session
        .mutate_world_creature(caller_guid, |caller| {
            caller.creature.set_faction(14);
            caller.enter_combat(victim_guid);
            caller
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(victim_guid, 25.0);
        })
        .unwrap();
    session
        .mutate_world_creature(assistant_guid, |assistant| {
            assistant.creature.set_faction(14);
            assistant.creature.ai_ownership_mut().aggro_radius = 0.0;
            assistant
                .creature
                .set_react_state(wow_entities::ReactState::Aggressive);
        })
        .unwrap();
    session
        .mutate_world_creature(victim_guid, |victim| {
            victim.creature.set_faction(1);
            victim.creature.ai_ownership_mut().aggro_radius = 0.0;
            victim
                .creature
                .set_react_state(wow_entities::ReactState::Passive);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let config = legacy_aggro_hostile_config_with_rate_like_cpp(3.0);

    let scheduled =
        run_legacy_creature_aggro_tick_once_with_config_like_cpp(&manager, &[], config.clone());
    assert_eq!(scheduled.assistance_scheduled, 1);

    manager
        .write()
        .unwrap()
        .find_creature_mut(0, 0, caller_guid)
        .unwrap()
        .backdate_runtime_clock_for_test(ASSISTANCE_DELAY_ELAPSED_LIKE_CPP);
    let assisted = run_legacy_creature_aggro_tick_once_with_config_like_cpp(&manager, &[], config);

    assert_eq!(assisted.assistance_starts, 1);
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, assistant_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target,
        Some(victim_guid)
    );
}
#[test]
fn legacy_creature_call_assistance_engages_same_faction_after_delay_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let caller_guid = test_creature_guid(91_216);
    let assistant_guid = test_creature_guid(91_217);
    let dead_assistant_guid = test_creature_guid(91_219);
    let controlled_assistant_guid = test_creature_guid(91_220);
    let passive_before_due_guid = test_creature_guid(91_221);
    let defensive_assistant_guid = test_creature_guid(91_222);
    let victim = ObjectGuid::create_player(1, 91_218);
    register_test_creature(&mut session, manager.clone(), caller_guid, 100);
    register_test_creature(&mut session, manager.clone(), assistant_guid, 100);
    register_test_creature(&mut session, manager.clone(), dead_assistant_guid, 100);
    register_test_creature(
        &mut session,
        manager.clone(),
        controlled_assistant_guid,
        100,
    );
    register_test_creature(&mut session, manager.clone(), passive_before_due_guid, 100);
    register_test_creature(&mut session, manager.clone(), defensive_assistant_guid, 100);
    session
        .mutate_world_creature(caller_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.set_faction(14);
        })
        .unwrap();
    session
        .mutate_world_creature(assistant_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 0.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.set_faction(14);
            creature
                .creature
                .set_react_state(wow_entities::ReactState::Aggressive);
        })
        .unwrap();
    session
        .mutate_world_creature(dead_assistant_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 0.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.set_faction(14);
        })
        .unwrap();
    session
        .mutate_world_creature(controlled_assistant_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 0.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.set_faction(14);
            creature
                .creature
                .unit_mut()
                .add_unit_state(wow_constants::UnitState::STUNNED.bits());
        })
        .unwrap();
    session
        .mutate_world_creature(passive_before_due_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 0.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.set_faction(14);
            creature
                .creature
                .set_react_state(wow_entities::ReactState::Aggressive);
        })
        .unwrap();
    session
        .mutate_world_creature(defensive_assistant_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 0.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.set_faction(14);
            creature
                .creature
                .set_react_state(wow_entities::ReactState::Defensive);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        victim,
        Position::new(10.5, 10.5, 0.0, 0.0),
    )];
    let config = legacy_aggro_hostile_config_with_rate_like_cpp(3.0);

    let scheduled = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        config.clone(),
    );
    assert_eq!(scheduled.aggro_starts, 1);
    assert_eq!(scheduled.assistance_scheduled, 3);
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, defensive_assistant_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target,
        None,
        "C++ Creature::CanAssistTo requires REACT_AGGRESSIVE"
    );
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, assistant_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target,
        None,
        "AssistDelayEvent must not engage the helper immediately"
    );

    manager
        .write()
        .unwrap()
        .find_creature_mut(0, 0, caller_guid)
        .unwrap()
        .backdate_runtime_clock_for_test(ASSISTANCE_DELAY_ELAPSED_LIKE_CPP);
    {
        let mut manager = manager.write().unwrap();
        let dead_assistant = manager
            .find_creature_mut(0, 0, dead_assistant_guid)
            .unwrap();
        dead_assistant.backdate_runtime_clock_for_test(ASSISTANCE_DELAY_ELAPSED_LIKE_CPP);
        dead_assistant
            .creature
            .unit_mut()
            .set_death_state(wow_constants::DeathState::JustDied);
        let passive = manager
            .find_creature_mut(0, 0, passive_before_due_guid)
            .unwrap();
        passive.backdate_runtime_clock_for_test(ASSISTANCE_DELAY_ELAPSED_LIKE_CPP);
        passive
            .creature
            .set_react_state(wow_entities::ReactState::Passive);
        let controlled = manager
            .find_creature_mut(0, 0, controlled_assistant_guid)
            .unwrap();
        controlled.backdate_runtime_clock_for_test(ASSISTANCE_DELAY_ELAPSED_LIKE_CPP);
        controlled
            .creature
            .unit_mut()
            .clear_unit_state(wow_constants::UnitState::STUNNED.bits());
    }
    let assisted =
        run_legacy_creature_aggro_tick_once_with_config_like_cpp(&manager, &candidates, config);
    assert_eq!(assisted.assistance_starts, 1);
    assert_eq!(
        assisted
            .plan
            .events
            .iter()
            .filter(|event| {
                wow_packet::WorldPacket::from_bytes(&event.packet_bytes).server_opcode()
                    == Some(ServerOpcodes::AttackStart)
            })
            .count(),
        1,
        "C++ SendMeleeAttackStart broadcasts each delayed assistance engagement"
    );
    assert!(
        assisted
            .commands
            .iter()
            .all(|command| command.packet_already_broadcast)
    );
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, assistant_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target,
        Some(victim)
    );
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, dead_assistant_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target,
        None,
        "AssistDelayEvent must not engage a helper that died before execution"
    );
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, passive_before_due_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target,
        None,
        "C++ AssistDelayEvent re-runs CanAssistTo when the delayed event executes"
    );
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, controlled_assistant_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target,
        None,
        "C++ CanAssistTo rejects a stunned helper during initial selection"
    );
}
