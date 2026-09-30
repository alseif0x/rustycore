//! Source-only regressions for the existing prefix and ExternalRuntime boundary.
use super::*;
use super::fixtures::*;
use crate::manager::{
    ActorRespawnProgress, MapObjectUpdateSelectionLikeCpp, MapObjectTickContinuation,
    ObjectMapUpdateToken, MapCreatureUpdateOwnerLikeCpp,
};
use crate::map::{
    Map, LoadedGridRespawnRecordsLikeCpp,
};
use crate::spawn::{SpawnObjectType, SpawnId};
use std::cell::Cell;
use std::time::Instant;
use wow_core::Position;
use wow_constants::DeathState;

type LoadRecord = fn(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>;

fn finish(manager: &mut MapManager, tick: &mut MapObjectTickContinuation, token: ObjectMapUpdateToken)
    -> ObjectMapFinishOutcome
{
    match manager.try_finish_object_map::<LoadRecord>(
        tick, token, None, None, MapCreatureUpdateOwnerLikeCpp::ExternalRuntime,
    ) {
        Ok(outcome) => outcome,
        Err((error, _token)) => panic!("existing external-runtime finish failed: {error:?}"),
    }
}

#[test]
fn canonical_death_prefix_runs_before_nearby_selection_without_advancing_actor_clock() {
    let (mut manager, selected) = manager_with_actor(801);
    let unselected = insert_actor(&mut manager, 802, Position::xyz(4000.0, 4000.0, 30.0), false);
    {
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        let actor = map.creature_actor_mut(unselected).unwrap();
        actor.creature.set_spawn_id(802);
        actor.creature.unit_mut().set_health(0);
        actor.creature.unit_mut().set_death_state(DeathState::JustDied);
        actor.creature.runtime_state_mut().save_respawn_requested = true;
        actor.advance_runtime_clock_like_cpp(37);
    }
    let now = Instant::now();
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let prefix = manager.begin_actor_respawn_map(
        &plan, plan.updated_maps_like_cpp()[0], now, now, 100, true,
    ).unwrap();
    let ActorRespawnProgress::Complete(outcome) = prefix else {
        panic!("death-save-only prefix must not request a factory");
    };
    assert_eq!(outcome.creatures_seen, 2);
    assert_eq!(outcome.respawn_db_mutations.len(), 1);
    assert_eq!(outcome.corpses_removed, 0);
    let actor = manager.find_map(1, 0).unwrap().map().creature_actor(unselected).unwrap();
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 37);
    assert!(!actor.creature.runtime_state().save_respawn_requested);
    let mut tick = manager.begin_object_tick(plan).unwrap();
    let mut token = manager.prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::NearbyCells)
        .unwrap().unwrap();
    assert_eq!(manager.selected_actor_guids(&tick, &mut token).unwrap(), vec![selected]);
    assert_eq!(finish(&mut manager, &mut tick, token), ObjectMapFinishOutcome::Completed);
    let actor = manager.find_map(1, 0).unwrap().map().creature_actor(unselected).unwrap();
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 37);
    assert!(!actor.creature.runtime_state().appeared_notified);
    assert_eq!(manager.find_map(1, 0).unwrap().last_creatures_update_summary().visited, 0);
    manager.finalize_object_tick(tick).unwrap();
}

#[test]
fn canonical_corpse_prefix_queues_once_and_external_finish_does_not_run_a_second_plan() {
    let (mut manager, live) = manager_with_actor(803);
    let corpse = insert_actor(&mut manager, 804, Position::xyz(4000.0, 4000.0, 30.0), false);
    {
        let actor = manager.find_map_mut(1, 0).unwrap().map_mut().creature_actor_mut(corpse).unwrap();
        actor.creature.set_spawn_id(804);
        actor.creature.unit_mut().set_health(0);
        actor.creature.unit_mut().set_death_state(DeathState::Corpse);
        actor.creature.set_ai_corpse_despawn_at(Some(0));
    }
    let now = Instant::now();
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let ActorRespawnProgress::Complete(outcome) = manager.begin_actor_respawn_map(
        &plan, plan.updated_maps_like_cpp()[0], now, now, 100, true,
    ).unwrap() else { panic!("new corpse queue must not be ready yet"); };
    assert_eq!(outcome.corpses_removed, 1);
    assert_eq!(outcome.respawn_db_mutations.len(), 1);
    assert!(manager.find_map(1, 0).unwrap().map().creature_actor(corpse).is_none());
    assert_eq!(manager.find_map(1, 0).unwrap().map().respawn_store_like_cpp().actor_queue_len(), 1);
    let before = {
        let actor = manager.find_map(1, 0).unwrap().map().creature_actor(live).unwrap();
        (actor.runtime_elapsed_ms_like_cpp(), actor.creature.regen_timer(), actor.current_hp(),
            actor.creature.runtime_state().appeared_notified)
    };
    let mut tick = manager.begin_object_tick(plan).unwrap();
    let mut token = manager.prepare_next_object_map(&mut tick, MapObjectUpdateSelectionLikeCpp::NearbyCells)
        .unwrap().unwrap();
    assert_eq!(manager.selected_actor_guids(&tick, &mut token).unwrap(), vec![live]);
    assert_eq!(finish(&mut manager, &mut tick, token), ObjectMapFinishOutcome::Completed);
    let actor = manager.find_map(1, 0).unwrap().map().creature_actor(live).unwrap();
    assert_eq!((actor.runtime_elapsed_ms_like_cpp(), actor.creature.regen_timer(), actor.current_hp(),
        actor.creature.runtime_state().appeared_notified), before);
    assert_eq!(manager.find_map(1, 0).unwrap().map().respawn_store_like_cpp().actor_queue_len(), 1);
    assert_eq!(manager.find_map(1, 0).unwrap().last_creatures_update_summary().visited, 0);
    manager.finalize_object_tick(tick).unwrap();
}

#[test]
fn selected_actor_read_preserves_diff_clock_and_rejects_unselected_before_callback() {
    let (mut manager, selected) = manager_with_actor(805);
    let far = insert_actor(&mut manager, 806, Position::xyz(4000.0, 4000.0, 30.0), false);
    let (mut tick, mut token) = start(&mut manager, 431, MapObjectUpdateSelectionLikeCpp::NearbyCells);
    assert_eq!(token.effective_diff_ms(), 431);
    let calls = Cell::new(0);
    assert_eq!(manager.with_selected_actor(&tick, &mut token, far, None, |_| {
        calls.set(calls.get() + 1);
    }).unwrap_err(), ActorTickAccessError::OutsideSelection { guid: far });
    let (clock, _) = manager.with_selected_actor(&tick, &mut token, selected, None, |actor| {
        calls.set(calls.get() + 1);
        actor.runtime_elapsed_ms_like_cpp()
    }).unwrap();
    assert_eq!(clock, 0);
    assert_eq!(calls.get(), 1);
    assert_eq!(finish(&mut manager, &mut tick, token), ObjectMapFinishOutcome::Completed);
    assert_eq!(manager.find_map(1, 0).unwrap().map().creature_actor(selected).unwrap()
        .runtime_elapsed_ms_like_cpp(), clock);
}

#[test]
fn selected_lifecycle_access_rejects_same_guid_aba_without_mutating_replacement() {
    let (mut manager, guid) = manager_with_actor(807);
    let (mut tick, mut token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::NearbyCells);
    assert_eq!(manager.selected_actor_guids(&tick, &mut token).unwrap(), vec![guid]);
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    let removed = map.remove_map_object(guid).unwrap();
    map.admit_creature_actor(new_actor(807, Position::xyz(10.0, 20.0, 30.0), true)).unwrap();
    let calls = Cell::new(0);
    assert_eq!(manager.with_selected_actor(&tick, &mut token, guid, None, |actor| {
        calls.set(calls.get() + 1);
        actor.advance_runtime_clock_like_cpp(200);
    }).unwrap_err(), ActorTickAccessError::WitnessMismatch { guid });
    assert_eq!(calls.get(), 0);
    assert_eq!(manager.find_map(1, 0).unwrap().map().creature_actor(guid).unwrap()
        .runtime_elapsed_ms_like_cpp(), 0);
    drop(removed);
    assert_eq!(finish(&mut manager, &mut tick, token), ObjectMapFinishOutcome::Completed);
}

#[test]
fn stale_lifecycle_token_rejects_callback_and_does_not_update_replacement_map() {
    let (mut manager, guid) = manager_with_actor(808);
    let (mut tick, mut token) = start(&mut manager, 200, MapObjectUpdateSelectionLikeCpp::NearbyCells);
    let admitted_incarnation = token.incarnation();
    assert!(manager.destroy_map(1, 0));
    manager.create_world_map(1, 0);
    let replacement = insert_actor(&mut manager, 808, Position::xyz(10.0, 20.0, 30.0), true);
    assert_eq!(replacement, guid);
    let current_incarnation = manager.map_incarnation_like_cpp(token.key());
    let calls = Cell::new(0);
    assert_eq!(manager.with_selected_actor(&tick, &mut token, guid, None, |_| {
        calls.set(calls.get() + 1);
    }).unwrap_err(), ActorTickAccessError::StaleParticipant {
        key: token.key(), admitted_incarnation, current_incarnation,
    });
    assert_eq!(calls.get(), 0);
    assert_eq!(finish(&mut manager, &mut tick, token), ObjectMapFinishOutcome::StaleParticipant {
        key: crate::MapKey::new(1, 0), admitted_incarnation, current_incarnation,
    });
    let map = manager.find_map(1, 0).unwrap();
    assert_eq!(map.map().creature_actor(guid).unwrap().runtime_elapsed_ms_like_cpp(), 0);
    assert_eq!(map.last_creatures_update_summary().visited, 0);
    assert!(!map.last_map_update_tail_summary_like_cpp().script_hook.invoked);
}
