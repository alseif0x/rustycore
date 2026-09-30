//! Concrete canonical gates; no transport or runtime activation.
use super::*;
use crate::manager::actor_tick_access::fixtures::*;
use crate::manager::{MapObjectUpdateSelectionLikeCpp, ObjectMapTickError};
use wow_core::Position;

fn handle(manager: &MapManager, guid: ObjectGuid) -> CreatureLootActorHandle {
    match manager.idle_creature_loot_handle(MapKey::new(1, 0), guid) {
        CreatureLootAccess::Ready(handle) => handle,
        other => panic!("expected existing idle Actor: {other:?}"),
    }
}

#[test]
fn no_actor_allows_compatibility_but_observed_busy_actor_is_rejected() {
    let (mut manager, guid) = manager_with_actor(92001);
    let missing = ObjectGuid::create_player(1, 92);
    assert!(matches!(
        manager.idle_creature_loot_handle(MapKey::new(1, 0), missing),
        CreatureLootAccess::NoActor
    ));
    let _tick = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    assert!(matches!(
        manager.observe_idle_creature_loot(MapKey::new(1, 0), guid),
        CreatureLootAccess::Rejected(CreatureLootAccessError::Busy)
    ));
    assert!(matches!(
        manager.idle_creature_loot_handle(MapKey::new(1, 0), missing),
        CreatureLootAccess::NoActor
    ));
}

#[test]
fn foreign_manager_and_stale_incarnation_reject_existing_handles() {
    let (mut manager, guid) = manager_with_actor(92002);
    let handle = handle(&manager, guid);
    let (mut other, _) = manager_with_actor(92002);
    assert!(matches!(
        other.force_idle_creature_loot_flags(&handle),
        CreatureLootAccess::Rejected(CreatureLootAccessError::WrongOrigin)
    ));
    manager.map_incarnations_like_cpp.remove(&handle.key());
    assert!(matches!(
        manager.force_idle_creature_loot_flags(&handle),
        CreatureLootAccess::Rejected(CreatureLootAccessError::StaleIncarnation)
    ));
}

#[test]
fn same_guid_readmission_cannot_use_old_witness() {
    let (mut manager, guid) = manager_with_actor(92003);
    let handle = handle(&manager, guid);
    manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .remove_map_object(guid)
        .unwrap();
    insert_actor(&mut manager, 92003, Position::xyz(10.0, 20.0, 30.0), true);
    let before = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor(guid)
        .unwrap()
        .creature
        .unit()
        .values_update();
    assert!(matches!(
        manager.force_idle_creature_loot_flags(&handle),
        CreatureLootAccess::Rejected(CreatureLootAccessError::WitnessMismatch)
    ));
    assert!(matches!(
        manager.observe_idle_creature_loot_source(&handle),
        CreatureLootAccess::Rejected(CreatureLootAccessError::WitnessMismatch)
    ));
    assert_eq!(
        manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .creature_actor(guid)
            .unwrap()
            .creature
            .unit()
            .values_update(),
        before
    );
}

#[test]
fn pending_actor_slot_rejects_reads_and_writes_without_clearing_it() {
    let (mut manager, guid) = manager_with_actor(92004);
    let handle = handle(&manager, guid);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let witness = manager
        .begin_actor_operation(&tick, &mut token, guid, None)
        .unwrap();
    assert!(matches!(
        manager.observe_tick_creature_loot(&tick, &mut token, guid),
        CreatureLootAccess::Rejected(CreatureLootAccessError::PendingActorOperation)
    ));
    assert!(matches!(
        manager.force_tick_creature_loot_flags(&tick, &mut token, &handle),
        CreatureLootAccess::Rejected(CreatureLootAccessError::PendingActorOperation)
    ));
    assert!(token.actor_operation.is_some());
    manager
        .complete_actor_operation(&tick, &mut token, guid, &witness)
        .unwrap();
    assert!(matches!(
        manager.force_tick_creature_loot_flags(&tick, &mut token, &handle),
        CreatureLootAccess::Ready(_)
    ));
}

#[test]
fn wrong_epoch_rejects_before_actor_access() {
    let (mut manager, guid) = manager_with_actor(92005);
    let handle = handle(&manager, guid);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    token.epoch += 1;
    assert!(matches!(
        manager.force_tick_creature_loot_flags(&tick, &mut token, &handle),
        CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(ActorTickAccessError::Tick(
            ObjectMapTickError::WrongEpoch { .. }
        )))
    ));
    assert_eq!(
        manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .creature_actor(guid)
            .unwrap()
            .creature
            .current_health(),
        75
    );
}

#[test]
fn tick_observation_and_install_mutate_same_actor_without_advancing_runtime() {
    let (mut manager, guid) = manager_with_actor(92006);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let (authority, lifetime, entity) = manager
        .with_selected_actor(&tick, &mut token, guid, None, |actor| {
            actor.creature.unit_mut().set_health(0);
            (
                actor.creature.loot_authority_like_cpp().clone(),
                actor.creature.loot_lifecycle_revision_like_cpp(),
                &actor.creature as *const _,
            )
        })
        .unwrap()
        .0;
    let observation = match manager.observe_tick_creature_loot(&tick, &mut token, guid) {
        CreatureLootAccess::Ready(observation) => observation,
        other => panic!("expected selected Actor: {other:?}"),
    };
    let (handle, source) = observation.into_parts();
    assert!(!source.is_alive());
    assert!(matches!(
        manager.install_tick_creature_kill_loot(
            &tick,
            &mut token,
            &handle,
            &authority,
            0,
            lifetime,
            None,
            HashMap::new()
        ),
        CreatureLootAccess::Ready(true)
    ));
    let actor = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor(guid)
        .unwrap();
    assert_eq!(&actor.creature as *const _, entity);
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 0);
    assert!(
        actor
            .creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&authority)
    );
}

#[test]
fn dormant_candidate_reader_obeys_phase_without_replacing_general_enumeration() {
    let (mut manager, guid) = manager_with_actor(92007);
    match manager.idle_creature_loot_candidates(MapKey::new(1, 0)) {
        CreatureLootAccess::Ready(guids) => assert_eq!(guids, vec![guid]),
        other => panic!("expected idle candidates: {other:?}"),
    }
    let _tick = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    assert!(matches!(
        manager.idle_creature_loot_candidates(MapKey::new(1, 0)),
        CreatureLootAccess::Rejected(CreatureLootAccessError::Busy)
    ));
}

#[test]
fn removed_admitted_actor_is_rejected_instead_of_becoming_no_actor() {
    let (mut manager, guid) = manager_with_actor(92008);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    assert!(matches!(
        manager.observe_tick_creature_loot(&tick, &mut token, guid),
        CreatureLootAccess::Ready(_)
    ));
    manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .remove_map_object(guid)
        .unwrap();
    assert!(
        matches!(manager.observe_tick_creature_loot(&tick, &mut token, guid),
        CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(
            ActorTickAccessError::ActorUnavailable { guid: rejected })) if rejected == guid)
    );
    assert_eq!(
        manager.selected_actor_guids(&tick, &mut token).unwrap(),
        vec![guid]
    );
    assert!(token.actor_operation.is_none());
}

#[test]
fn never_admitted_absent_actor_returns_no_actor_and_freezes_workset() {
    let (mut manager, original) = manager_with_actor(92009);
    let absent = new_actor(92010, Position::xyz(10.0, 20.0, 30.0), false).guid();
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    assert!(matches!(
        manager.observe_tick_creature_loot(&tick, &mut token, absent),
        CreatureLootAccess::NoActor
    ));
    assert_eq!(
        manager.selected_actor_guids(&tick, &mut token).unwrap(),
        vec![original]
    );
    assert!(token.actor_operation.is_none());
}

#[test]
fn late_actor_after_absent_observation_stays_outside_frozen_workset() {
    let (mut manager, original) = manager_with_actor(92011);
    let late = new_actor(92012, Position::xyz(11.0, 20.0, 30.0), false).guid();
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    assert!(matches!(
        manager.observe_tick_creature_loot(&tick, &mut token, late),
        CreatureLootAccess::NoActor
    ));
    assert_eq!(
        insert_actor(&mut manager, 92012, Position::xyz(11.0, 20.0, 30.0), false),
        late
    );
    assert!(
        matches!(manager.observe_tick_creature_loot(&tick, &mut token, late),
        CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(
            ActorTickAccessError::OutsideSelection { guid: rejected })) if rejected == late)
    );
    assert_eq!(
        manager.selected_actor_guids(&tick, &mut token).unwrap(),
        vec![original]
    );
    assert!(token.actor_operation.is_none());
    assert_eq!(
        manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .creature_actor(late)
            .unwrap()
            .runtime_elapsed_ms_like_cpp(),
        0
    );
}

#[test]
fn creature_record_loot_read_preserves_pool_semantics_and_update_fields() {
    use wow_entities::MapObjectRecord;
    for coins in [0, 1] {
        let mut manager = MapManager::new(crate::MIN_GRID_DELAY_MS, 200);
        manager.create_world_map(1, 0);
        let mut record = new_actor(
            92013 + i64::from(coins),
            Position::xyz(10.0, 20.0, 30.0),
            false,
        )
        .creature;
        let guid = record.guid();
        record.replace_loot_authority_like_cpp(
            Some(CreatureLoot {
                loot_guid: guid,
                coins,
                unlooted_count: 0,
                loot_type: 1,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            }),
            HashMap::new(),
        );
        let before = record.unit().values_update();
        manager
            .find_map_mut(1, 0)
            .unwrap()
            .map_mut()
            .insert_map_object_record(MapObjectRecord::new_creature(record).unwrap())
            .unwrap();
        assert!(
            matches!(manager.creature_record_loot_fully_consumed(MapKey::new(1, 0), guid),
            CreatureLootAccess::Ready(Some(value)) if value == (coins == 0))
        );
        assert_eq!(
            manager
                .find_map(1, 0)
                .unwrap()
                .map()
                .with_creature_like_cpp(guid, |creature| creature.unit().values_update()),
            Some(before)
        );
    }
}

#[test]
fn creature_record_loot_read_rejects_actor_admitted_after_no_actor_observation() {
    let (mut manager, _) = manager_with_actor(92015);
    let guid = new_actor(92016, Position::xyz(11.0, 20.0, 30.0), false).guid();
    assert!(matches!(
        manager.idle_creature_loot_handle(MapKey::new(1, 0), guid),
        CreatureLootAccess::NoActor
    ));
    insert_actor(&mut manager, 92016, Position::xyz(11.0, 20.0, 30.0), false);
    let before = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor(guid)
        .unwrap()
        .creature
        .unit()
        .values_update();
    let _tick = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    assert!(matches!(
        manager.creature_record_loot_fully_consumed(MapKey::new(1, 0), guid),
        CreatureLootAccess::Rejected(CreatureLootAccessError::ActorPresent)
    ));
    assert_eq!(
        manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .creature_actor(guid)
            .unwrap()
            .creature
            .unit()
            .values_update(),
        before
    );
}
