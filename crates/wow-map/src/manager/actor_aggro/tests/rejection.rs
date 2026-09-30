//! Lifetime/slot rejection preserves owned inputs and never replays the motor.
use super::*;
use crate::manager::MapTickCoordinationStateLikeCpp;

fn prefix(manager: &MapManager, guid: ObjectGuid) -> (u64, u64, u32, Position) {
    let actor = actor(manager, guid);
    (actor.runtime_elapsed_ms_like_cpp(), actor.runtime_motion_master_ticks_like_cpp(), actor.spline_id(), actor.position())
}

fn no_policy<R>(apply: impl FnOnce(&mut AggroPolicies<'_>) -> R) -> R {
    let mut hostility = |_: i32, _: AggroFactionTarget<'_>| panic!("rejected gate must precede hostility");
    let mut ai = |_: crate::map_manager::AggroAiFacts<'_>| panic!("rejected gate must precede AI selection");
    let mut can = |_| panic!("rejected gate must precede CanAttack");
    let mut distance = |_| panic!("rejected gate must precede acquisition");
    apply(&mut AggroPolicies { hostility: &mut hostility, select_ai: &mut ai,
        can_attack: &mut can, attack_distance: &mut distance })
}

#[test]
fn foreign_origin_and_epoch_prepare_return_candidates_before_any_policy() {
    let (mut manager, guid) = fixtures::manager_with_actor(812_001);
    configure(&mut manager, guid);
    let (tick, mut token) = fixtures::start(&mut manager, 77, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let (mut other, _) = fixtures::manager_with_actor(812_001);
    let (foreign_tick, mut foreign_token) = fixtures::start(&mut other, 99, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let before = prefix(&manager, guid);
    let candidates = vec![candidate(ObjectGuid::create_player(1, 812_003))];
    let pointer = candidates.as_ptr();
    let failure = no_policy(|policy| manager.prepare_aggro(&foreign_tick, &mut token, candidates, settings(), false, policy)).err().expect("prepare rejected");
    assert!(matches!(failure.error, ActorAggroError::Access(ActorTickAccessError::Tick(ObjectMapTickError::OriginMismatch { .. }))));
    assert_eq!(failure.candidates.as_ptr(), pointer);
    let failure = no_policy(|policy| manager.prepare_aggro(&tick, &mut foreign_token, failure.candidates, settings(), false, policy)).err().expect("prepare rejected");
    assert_eq!(failure.candidates.as_ptr(), pointer);
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(2);
    let failure = no_policy(|policy| manager.prepare_aggro(&tick, &mut token, failure.candidates, settings(), false, policy)).err().expect("prepare rejected");
    assert!(matches!(failure.error, ActorAggroError::Access(ActorTickAccessError::Tick(ObjectMapTickError::WrongEpoch { .. }))));
    assert_eq!(failure.candidates.as_ptr(), pointer);
    assert_eq!(prefix(&manager, guid), before);
    assert!(token.actor_operation.is_none());
}

#[test]
fn selected_record_rejects_entire_primary_pass_before_any_actor_mutation() {
    let (mut manager, guid) = fixtures::manager_with_actor(812_010);
    configure(&mut manager, guid);
    let other = fixtures::insert_actor(&mut manager, 812_011, Position::xyz(12.0, 20.0, 30.0), true);
    let (tick, mut token) = fixtures::start(&mut manager, 77, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let admitted = manager.selected_actor_guids(&tick, &mut token).unwrap();
    assert!(admitted.contains(&other));
    let removed = manager.find_map_mut(1, 0).unwrap().map_mut().remove_map_object(other).unwrap();
    let record = fixtures::new_actor(812_011, Position::xyz(12.0, 20.0, 30.0), true).creature;
    manager.find_map_mut(1, 0).unwrap().map_mut().insert_map_object_record(MapObjectRecord::new_creature(record).unwrap()).unwrap();
    let before = prefix(&manager, guid);
    assert!(no_policy(|policy| manager.prepare_aggro(&tick, &mut token, Vec::new(), settings(), false, policy)).is_err());
    assert_eq!(prefix(&manager, guid), before);
    assert!(token.actor_operation.is_none());
    drop(removed);
}

#[test]
fn foreign_and_wrong_epoch_resume_retain_original_partial_effects_and_response() {
    let (mut manager, tick, mut token, caller, _, _, request) = pending_fixture(812_020);
    let effects = request.partial().effects.as_ptr();
    let before = prefix(&manager, caller);
    let (mut other, _) = fixtures::manager_with_actor(812_020);
    let (other_tick, mut other_token) = fixtures::start(&mut other, 99, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let failure = match policies(|policy| other.resume_aggro_assistance_los(&other_tick, &mut other_token,
        request.into_continuation(), true, policy)) { Err(failure) => failure, _ => panic!("foreign token") };
    assert!(matches!(failure.error, ActorAggroError::Access(ActorTickAccessError::OperationMismatch { .. })));
    assert!(failure.response);
    assert_eq!(failure.continuation.partial().effects.as_ptr(), effects);
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(2);
    let failure = match policies(|policy| manager.resume_aggro_assistance_los(&tick, &mut token,
        failure.continuation, failure.response, policy)) { Err(failure) => failure, _ => panic!("wrong epoch") };
    assert!(matches!(failure.error, ActorAggroError::Access(ActorTickAccessError::Tick(ObjectMapTickError::WrongEpoch { .. }))));
    assert_eq!(failure.continuation.partial().effects.as_ptr(), effects);
    assert_eq!(prefix(&manager, caller), before);
    assert!(token.actor_operation.is_some());
    manager.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(1);
    let outcome = complete(policies(|policy| manager.resume_aggro_assistance_los(&tick, &mut token,
        failure.continuation, failure.response, policy)).unwrap());
    assert_eq!(outcome.assistance_scheduled, 1);
    assert!(token.actor_operation.is_none());
}

#[test]
fn secondary_same_guid_readmission_rejects_without_mutating_replacement() {
    let (mut manager, tick, mut token, caller, assistant, _, request) = pending_fixture(812_030);
    let removed = manager.find_map_mut(1, 0).unwrap().map_mut().remove_map_object(assistant).unwrap();
    assert_eq!(fixtures::insert_actor(&mut manager, 812_031, Position::xyz(500.0, 20.0, 30.0), false), assistant);
    let before = prefix(&manager, assistant);
    let failure = match policies(|policy| manager.resume_aggro_assistance_los(&tick, &mut token,
        request.into_continuation(), true, policy)) { Err(failure) => failure, _ => panic!("secondary ABA") };
    assert!(matches!(failure.error, ActorAggroError::Access(ActorTickAccessError::WitnessMismatch { guid }) if guid == assistant));
    assert!(failure.response);
    assert_eq!(prefix(&manager, assistant), before);
    assert!(token.actor_operation.is_some());
    assert_eq!(actor_mut(&mut manager, caller).take_due_assistance_like_cpp(), Vec::new());
    drop(failure); drop(removed);
    assert!(manager.begin_tick_like_cpp(1).is_busy());
}

#[test]
fn snapshot_keeps_secondary_witness_and_motor_then_original_resume_schedules() {
    let (mut manager, tick, mut token, _, assistant, _, request) = pending_fixture(812_040);
    let map = manager.find_map(1, 0).unwrap().map();
    let witness = map.creature_actor_witness(assistant).unwrap();
    let pointer = map.creature_actor(assistant).unwrap() as *const WorldCreature;
    let before = prefix(&manager, assistant);
    let mut snapshot = fixtures::new_actor(812_041, Position::xyz(500.0, 20.0, 30.0), false).creature;
    snapshot.set_faction(14);
    snapshot.set_react_state(wow_entities::ReactState::Aggressive);
    snapshot.unit_mut().set_health(17);
    manager.find_map_mut(1, 0).unwrap().map_mut().replace_creature_snapshot(MapObjectRecord::new_creature(snapshot).unwrap()).unwrap();
    assert!(witness.same_actor(&manager.find_map(1, 0).unwrap().map().creature_actor_witness(assistant).unwrap()));
    assert_eq!(actor(&manager, assistant) as *const WorldCreature, pointer);
    assert_eq!(prefix(&manager, assistant), before);
    let outcome = complete(policies(|policy| manager.resume_aggro_assistance_los(&tick, &mut token,
        request.into_continuation(), true, policy)).unwrap());
    assert_eq!(outcome.assistance_scheduled, 1);
    assert_eq!(actor(&manager, assistant).creature.current_health(), 17);
    assert_eq!(prefix(&manager, assistant), before);
}

#[test]
fn stale_map_and_second_prepare_keep_original_operation_busy() {
    let (mut manager, tick, mut token, caller, _, _, request) = pending_fixture(812_050);
    let rejected = policies(|policy| manager.prepare_aggro(&tick, &mut token, Vec::new(), settings(), false, policy)).err().expect("prepare rejected");
    assert!(matches!(rejected.error, ActorAggroError::Access(ActorTickAccessError::Tick(ObjectMapTickError::ActorOperationInFlight { .. }))));
    let old = manager.maps.remove(&crate::MapKey::new(1, 0)).unwrap();
    manager.create_world_map(1, 0);
    fixtures::insert_actor(&mut manager, 812_050, Position::xyz(10.0, 20.0, 30.0), true);
    let before = prefix(&manager, caller);
    let failure = match policies(|policy| manager.resume_aggro_assistance_los(&tick, &mut token,
        request.into_continuation(), false, policy)) { Err(failure) => failure, _ => panic!("stale map") };
    assert!(!failure.response);
    assert!(matches!(failure.error, ActorAggroError::Access(ActorTickAccessError::StaleParticipant { .. })));
    assert_eq!(prefix(&manager, caller), before);
    assert!(token.actor_operation.is_some());
    drop(failure); drop(old);
    assert!(manager.begin_tick_like_cpp(1).is_busy());
}

#[test]
fn player_readmission_rejects_old_los_before_hostility_or_scheduling() {
    let (mut manager, tick, mut token, caller, _, victim, request) = pending_fixture(812_060);
    let original = manager.current_player_admission_like_cpp(victim).unwrap();
    let retired = manager.retire_player_like_cpp(original.0).unwrap();
    assert_eq!(add_player(&mut manager, 812_062), victim);
    assert_ne!(manager.current_player_admission_like_cpp(victim).unwrap().0, original.0);
    let before = prefix(&manager, caller);
    let mut hostility = |_: i32, _: AggroFactionTarget<'_>| panic!("rejected Player identity cannot run hostility");
    let mut ai = |_: crate::map_manager::AggroAiFacts<'_>| panic!("prefix replay");
    let mut can = |_| panic!("prefix replay"); let mut distance = |_| panic!("prefix replay");
    let mut policy = AggroPolicies { hostility: &mut hostility, select_ai: &mut ai,
        can_attack: &mut can, attack_distance: &mut distance };
    let failure = match manager.resume_aggro_assistance_los(&tick, &mut token,
        request.into_continuation(), true, &mut policy) { Err(failure) => failure, _ => panic!("Player ABA") };
    assert_eq!(failure.error, ActorAggroError::PlayerIdentityMismatch { guid: victim });
    assert!(failure.response);
    assert_eq!(prefix(&manager, caller), before);
    assert!(token.actor_operation.is_some());
    drop(failure); drop(retired);
}
