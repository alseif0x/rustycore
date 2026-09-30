//! The compatibility producer and staged canonical producer share one motor.
use super::*;

#[test]
fn synchronous_and_staged_assistance_preserve_order_deadlines_and_rng() {
    let (mut canonical, tick, mut token, caller, assistant, victim, request) = pending_fixture(813_001);
    let mut legacy = crate::map_manager::MapManager::new();
    for (counter, position, active) in [(813_001, Position::xyz(10.0, 20.0, 30.0), true),
        (813_002, Position::xyz(500.0, 20.0, 30.0), false)] {
        let mut incoming = fixtures::new_actor(counter, position, active);
        incoming.creature.set_faction(14);
        incoming.creature.set_react_state(wow_entities::ReactState::Aggressive);
        incoming.creature.unit_mut().set_level(25);
        incoming.creature.ai_ownership_mut().aggro_radius = 20.0;
        incoming.seed_runtime_rng_like_cpp(0x5757);
        if incoming.guid() == caller { incoming.enter_combat(victim); }
        else { incoming.creature.unit_mut().add_unit_state(UnitState::SIGHTLESS.bits()); }
        assert!(legacy.add_creature(1, 0, 32, 32, incoming));
    }
    // TerrainNone is the legacy bypass; the staged reply supplies the same
    // visible result without recomputing pre-LOS distance/flags or the prefix.
    let sync = policies(|policy| legacy.run_aggro_map(1, 0, vec![candidate(victim)], settings(), None, policy));
    let staged = complete(policies(|policy| canonical.resume_aggro_assistance_los(&tick, &mut token,
        request.into_continuation(), true, policy)).unwrap());
    assert_eq!(sync.assistance_scheduled, staged.assistance_scheduled);
    assert_eq!(sync.aggro_starts, staged.aggro_starts);
    assert_eq!(sync.commands.len(), staged.commands.len());
    for (expected, actual) in sync.commands.iter().zip(&staged.commands) {
        assert_eq!((expected.attacker_guid, expected.victim_guid, expected.previous_victim_guid),
            (actual.attacker_guid, actual.victim_guid, actual.previous_victim_guid));
    }
    for guid in [caller, assistant] {
        let sync_actor = legacy.find_creature_mut(1, 0, guid).unwrap();
        let staged_actor = actor_mut(&mut canonical, guid);
        assert_eq!(sync_actor.runtime_elapsed_ms_like_cpp(), staged_actor.runtime_elapsed_ms_like_cpp());
        assert_eq!(sync_actor.runtime_motion_master_ticks_like_cpp(), staged_actor.runtime_motion_master_ticks_like_cpp());
        assert_eq!(sync_actor.creature.ai_ownership().combat_target, staged_actor.creature.ai_ownership().combat_target);
        assert_eq!(sync_actor.runtime_spell_schedule_rng_next_like_cpp(), staged_actor.runtime_spell_schedule_rng_next_like_cpp());
        sync_actor.advance_runtime_clock_like_cpp(100);
        staged_actor.advance_runtime_clock_like_cpp(100);
        assert_eq!(sync_actor.take_due_assistance_like_cpp(), staged_actor.take_due_assistance_like_cpp());
    }
}

#[test]
fn nan_assistance_radius_retains_original_positive_only_gate() {
    let (mut manager, caller) = fixtures::manager_with_actor(813_010);
    configure(&mut manager, caller);
    let victim = add_player(&mut manager, 813_012);
    let assistant = fixtures::insert_actor(&mut manager, 813_011, Position::xyz(500.0, 20.0, 30.0), false);
    configure(&mut manager, assistant);
    actor_mut(&mut manager, caller).enter_combat(victim);
    let (tick, mut token) = fixtures::start(&mut manager, 100, MapObjectUpdateSelectionLikeCpp::NearbyCells);
    let mut config = settings(); config.family_assistance_radius = f32::NAN;
    let outcome = complete(policies(|policy| manager.prepare_aggro(&tick, &mut token,
        vec![candidate(victim)], config, true, policy)).unwrap());
    assert_eq!(outcome.assistance_scheduled, 0);
    assert!(token.actor_operation.is_none());
}
