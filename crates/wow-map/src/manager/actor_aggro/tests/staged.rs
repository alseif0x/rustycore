//! Delay/LOS lineage: World entities 20 assistance and 21 blocked/overlapping LOS.
use super::*;

#[test]
fn los_is_lazy_after_distance_and_before_hostility_with_secondary_outside_selection() {
    let (mut manager, tick, mut token, caller, assistant, _, request) = pending_fixture(811_001);
    assert_eq!(request.query().map_id, 1);
    assert_eq!(request.partial().assistance_scheduled, 0);
    let elapsed = actor(&manager, caller).runtime_elapsed_ms_like_cpp();
    let continuation = request.into_continuation();
    let mut calls = 0;
    let mut hostility = |_: i32, _: AggroFactionTarget<'_>| { calls += 1; Some(true) };
    let mut ai = |_: crate::map_manager::AggroAiFacts<'_>| panic!("resume never replays AI selection");
    let mut can = |_| panic!("resume never replays CanAIAttack");
    let mut distance = |_| panic!("resume never replays acquisition");
    let mut policy = AggroPolicies { hostility: &mut hostility, select_ai: &mut ai, can_attack: &mut can, attack_distance: &mut distance };
    let outcome = complete(manager.resume_aggro_assistance_los(&tick, &mut token, continuation, true, &mut policy).unwrap());
    assert_eq!(calls, 1);
    assert_eq!(outcome.assistance_scheduled, 1);
    assert_eq!(actor(&manager, caller).runtime_elapsed_ms_like_cpp(), elapsed);
    assert!(!actor(&manager, assistant).creature.is_in_combat());
    assert!(token.actor_operation.is_none());
    actor_mut(&mut manager, caller).advance_runtime_clock_like_cpp(100);
    assert_eq!(actor_mut(&mut manager, caller).take_due_assistance_like_cpp(), vec![(ObjectGuid::create_player(1, 811_003), vec![assistant])]);
}

#[test]
fn blocked_los_skips_hostility_and_does_not_schedule() {
    let (mut manager, tick, mut token, _, _, _, request) = pending_fixture(811_010);
    let mut hostility = |_: i32, _: AggroFactionTarget<'_>| panic!("blocked LOS short-circuits hostility");
    let mut ai = |_: crate::map_manager::AggroAiFacts<'_>| panic!("AI prefix replay");
    let mut can = |_| panic!("CanAttack replay");
    let mut distance = |_| panic!("range replay");
    let mut policy = AggroPolicies { hostility: &mut hostility, select_ai: &mut ai, can_attack: &mut can, attack_distance: &mut distance };
    let outcome = complete(manager.resume_aggro_assistance_los(&tick, &mut token, request.into_continuation(), false, &mut policy).unwrap());
    assert_eq!(outcome.assistance_scheduled, 0);
}

#[test]
fn terrain_none_uses_the_same_tail_without_a_query() {
    let (mut manager, caller) = fixtures::manager_with_actor(811_020);
    configure(&mut manager, caller);
    let assistant = fixtures::insert_actor(&mut manager, 811_021, Position::xyz(500.0, 20.0, 30.0), false);
    configure(&mut manager, assistant);
    let victim = add_player(&mut manager, 811_022);
    actor_mut(&mut manager, caller).enter_combat(victim);
    let (tick, mut token) = fixtures::start(&mut manager, 100, MapObjectUpdateSelectionLikeCpp::NearbyCells);
    let outcome = complete(policies(|policy| manager.prepare_aggro(&tick, &mut token, vec![candidate(victim)], settings(), false, policy)).unwrap());
    assert_eq!(outcome.assistance_scheduled, 1);
}

#[test]
fn dropping_query_keeps_slot_accounting_and_tail_busy() {
    let (mut manager, mut tick, token, caller, _, _, request) = pending_fixture(811_030);
    drop(request);
    assert_eq!(token.actor_operation.as_ref().unwrap().guid, caller);
    assert!(matches!(manager.prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::NearbyCells), Err(ObjectMapTickError::MapInFlight { .. })));
    let rejected = manager.try_finish_object_map::<fn(&mut crate::map::Map, crate::SpawnObjectType, crate::SpawnId) -> Option<crate::map::LoadedGridRespawnRecordsLikeCpp>>(&mut tick, token, None, None,
        crate::MapCreatureUpdateOwnerLikeCpp::ExternalRuntime).unwrap_err();
    assert!(matches!(rejected.0, ObjectMapTickError::ActorOperationInFlight { .. }));
    assert!(rejected.1.actor_operation.is_some());
}

#[test]
fn due_assistance_engages_present_creature_victim_outside_primary_selection() {
    let (mut manager, caller) = fixtures::manager_with_actor(811_040);
    configure(&mut manager, caller);
    let assistant = fixtures::insert_actor(&mut manager, 811_041, Position::xyz(500.0, 20.0, 30.0), false);
    let victim = fixtures::insert_actor(&mut manager, 811_042, Position::xyz(600.0, 20.0, 30.0), false);
    configure(&mut manager, assistant); configure(&mut manager, victim);
    actor_mut(&mut manager, caller).schedule_assistance_like_cpp(victim, vec![assistant], 0);
    let (tick, mut token) = fixtures::start(&mut manager, 100, MapObjectUpdateSelectionLikeCpp::NearbyCells);
    let mut config = settings(); config.family_assistance_radius = 0.0;
    let outcome = complete(policies(|policy| manager.prepare_aggro(&tick, &mut token, Vec::new(), config, false, policy)).unwrap());
    assert_eq!(outcome.assistance_starts, 1);
    assert_eq!(actor(&manager, assistant).creature.ai_ownership().combat_target, Some(victim));
    assert!(actor(&manager, victim).creature.unit().subsystems().combat.is_in_combat_with(assistant));
}

#[test]
fn multiple_los_replies_keep_one_slot_and_original_secondary_order() {
    let (mut manager, caller) = fixtures::manager_with_actor(811_050);
    configure(&mut manager, caller);
    let mut assistants = Vec::new();
    for (counter, x) in [(811_051, 500.0), (811_052, 600.0)] {
        let guid = fixtures::insert_actor(&mut manager, counter, Position::xyz(x, 20.0, 30.0), false);
        configure(&mut manager, guid); assistants.push(guid);
    }
    let victim = add_player(&mut manager, 811_053);
    actor_mut(&mut manager, caller).enter_combat(victim);
    let order: Vec<_> = manager.find_map(1, 0).unwrap().runtime.aggro_actor_witnesses()
        .into_iter().map(|(guid, _)| guid).filter(|guid| assistants.contains(guid)).collect();
    let (tick, mut token) = fixtures::start(&mut manager, 999, MapObjectUpdateSelectionLikeCpp::NearbyCells);
    let before = actor(&manager, caller).runtime_elapsed_ms_like_cpp();
    let mut progress = policies(|policy| manager.prepare_aggro(&tick, &mut token,
        vec![candidate(victim)], settings(), true, policy)).unwrap();
    let mut requests = 0;
    let outcome = loop {
        match progress {
            ActorAggroProgress::Complete(outcome) => break outcome,
            ActorAggroProgress::Pending(request) => {
                assert_eq!(token.actor_operation.as_ref().unwrap().guid, caller);
                requests += 1;
                progress = policies(|policy| manager.resume_aggro_assistance_los(&tick, &mut token,
                    request.into_continuation(), true, policy)).unwrap();
            }
        }
    };
    assert_eq!(requests, 2);
    assert_eq!(outcome.assistance_scheduled, 2);
    assert_eq!(actor(&manager, caller).runtime_elapsed_ms_like_cpp(), before);
    assert!(token.actor_operation.is_none());
    actor_mut(&mut manager, caller).advance_runtime_clock_like_cpp(100);
    assert_eq!(actor_mut(&mut manager, caller).take_due_assistance_like_cpp(), vec![(victim, order)]);
}
