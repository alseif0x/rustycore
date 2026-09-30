//! Real token-slot contracts for the dormant reserved loot continuation.
use super::*;
use crate::manager::actor_tick_access::fixtures::*;
use crate::manager::{
    MapCreatureUpdateOwnerLikeCpp, MapObjectFinishOutcome, MapObjectUpdateSelectionLikeCpp,
    ObjectMapTickError,
};
use crate::map::{LoadedGridRespawnRecordsLikeCpp, Map};
use crate::spawn::{SpawnId, SpawnObjectType};
use wow_constants::DeathState;
use wow_core::Position;

type LoadRecord = fn(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>;

fn begin(
    manager: &mut MapManager,
    tick: &MapObjectTickContinuation,
    token: &mut ObjectMapUpdateToken,
    guid: ObjectGuid,
) -> (CreatureLootActorHandle, CreatureLootObservation) {
    match manager.begin_actor_loot_operation(tick, token, guid) {
        CreatureLootAccess::Ready(observation) => observation.into_parts(),
        other => panic!("fixture must reserve its admitted Actor: {other:?}"),
    }
}

fn finish(
    manager: &mut MapManager,
    mut tick: MapObjectTickContinuation,
    token: ObjectMapUpdateToken,
) {
    match manager.try_finish_object_map::<LoadRecord>(
        &mut tick,
        token,
        None,
        None,
        MapCreatureUpdateOwnerLikeCpp::ExternalRuntime,
    ) {
        Ok(outcome) => assert_eq!(outcome, ObjectMapFinishOutcome::Completed),
        Err((error, _)) => panic!("disposed fixture must finish: {error:?}"),
    }
    manager.finalize_object_tick(tick).unwrap();
}

fn empty_pool(guid: ObjectGuid) -> CreatureLoot {
    CreatureLoot {
        loot_guid: guid,
        coins: 0,
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
    }
}

#[test]
fn reserved_projection_and_handle_drop_leave_the_actual_token_slot_owned() {
    let (mut manager, guid) = manager_with_actor(93001);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let (handle, source) = begin(&mut manager, &tick, &mut token, guid);
    assert_eq!(handle.guid(), guid);
    assert!(source.is_alive());
    assert_eq!(source.position(), Position::xyz(10.0, 20.0, 30.0));
    let witness = token.actor_operation.as_ref().unwrap().witness.clone();
    drop(source);
    assert!(token.actor_operation.is_some());
    drop(handle);
    assert!(token.actor_operation.is_some());
    assert!(
        matches!(manager.begin_actor_loot_operation(&tick, &mut token, guid),
        CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(
            ActorTickAccessError::Tick(ObjectMapTickError::ActorOperationInFlight { guid: pending })))
            if pending == guid)
    );
    assert_eq!(
        manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .creature_actor(guid)
            .unwrap()
            .runtime_elapsed_ms_like_cpp(),
        0
    );
    // Test disposal uses the same private primitive and retained actual witness;
    // dropping either public owned value did not manufacture this cleanup.
    manager
        .complete_actor_operation(&tick, &mut token, guid, &witness)
        .unwrap();
    assert!(token.actor_operation.is_none());
}

#[test]
fn own_reserved_slot_installs_forces_and_releases_on_the_same_actor() {
    let (mut manager, guid) = manager_with_actor(93002);
    let (authority, lifetime, pointer, health_revision) = {
        let actor = manager
            .find_map_mut(1, 0)
            .unwrap()
            .map_mut()
            .creature_actor_mut(guid)
            .unwrap();
        actor.creature.unit_mut().set_health(0);
        actor
            .creature
            .unit_mut()
            .set_death_state(DeathState::Corpse);
        actor.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
        (
            actor.creature.loot_authority_like_cpp().clone(),
            actor.creature.loot_lifecycle_revision_like_cpp(),
            &actor.creature as *const _,
            actor.creature.unit().health_state_revision_like_cpp(),
        )
    };
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let (handle, source) = begin(&mut manager, &tick, &mut token, guid);
    assert_eq!(source.loot_lifecycle_revision(), lifetime);
    assert!(matches!(
        manager.observe_tick_creature_loot(&tick, &mut token, guid),
        CreatureLootAccess::Rejected(CreatureLootAccessError::PendingActorOperation)
    ));
    assert!(matches!(
        manager.force_tick_creature_loot_flags(&tick, &mut token, &handle),
        CreatureLootAccess::Rejected(CreatureLootAccessError::PendingActorOperation)
    ));
    assert!(matches!(
        manager.install_reserved_actor_kill_loot(
            &tick,
            &mut token,
            &handle,
            &authority,
            0,
            lifetime,
            Some(empty_pool(guid)),
            HashMap::new()
        ),
        CreatureLootAccess::Ready(true)
    ));
    assert!(matches!(
        manager.force_reserved_actor_loot_flags(&tick, &mut token, &handle),
        CreatureLootAccess::Ready(_)
    ));
    let observed = authority
        .fully_looted_unviewed_lifecycle_observation_like_cpp()
        .unwrap();
    assert!(matches!(
        manager.release_reserved_actor_loot(
            &tick,
            &mut token,
            &handle,
            &authority,
            observed.object_generation,
            observed.lifecycle_revision,
            false,
            1.0,
            CreatureLootReleasePhase::Normal
        ),
        CreatureLootAccess::Ready(Some(_))
    ));
    let actor = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor(guid)
        .unwrap();
    assert!(!actor.has_lootable_dynamic_flag_like_cpp());
    assert_eq!(&actor.creature as *const _, pointer);
    assert_eq!(
        actor.creature.unit().health_state_revision_like_cpp(),
        health_revision
    );
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 0);
    assert!(token.actor_operation.is_some());
    manager
        .complete_reserved_actor_loot_operation(&tick, &mut token, handle)
        .unwrap();
    assert!(token.actor_operation.is_none());
}

#[test]
fn rejected_install_keeps_reservation_until_explicit_disposal() {
    let (mut manager, guid) = manager_with_actor(93003);
    let authority = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor(guid)
        .unwrap()
        .creature
        .loot_authority_like_cpp()
        .clone();
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let (handle, source) = begin(&mut manager, &tick, &mut token, guid);
    assert!(matches!(
        manager.install_reserved_actor_kill_loot(
            &tick,
            &mut token,
            &handle,
            &authority,
            0,
            source.loot_lifecycle_revision(),
            Some(empty_pool(guid)),
            HashMap::new()
        ),
        CreatureLootAccess::Ready(false)
    ));
    assert!(authority.is_retired_like_cpp());
    assert!(token.actor_operation.is_some());
    manager
        .complete_reserved_actor_loot_operation(&tick, &mut token, handle)
        .unwrap();
    assert!(token.actor_operation.is_none());
}

#[test]
fn foreign_guid_cannot_resume_or_clear_the_reserved_actor() {
    let (mut manager, guid) = manager_with_actor(93004);
    let other = insert_actor(&mut manager, 93005, Position::xyz(11.0, 20.0, 30.0), false);
    let mut foreign = match manager.idle_creature_loot_handle(MapKey::new(1, 0), other) {
        CreatureLootAccess::Ready(handle) => handle,
        value => panic!("{value:?}"),
    };
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let (handle, _) = begin(&mut manager, &tick, &mut token, guid);
    // Corrupt only opaque test provenance; production callers cannot build it.
    foreign.reserved_epoch = handle.reserved_epoch;
    assert!(
        matches!(manager.force_reserved_actor_loot_flags(&tick, &mut token, &foreign),
        CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(
            ActorTickAccessError::OperationMismatch { guid: rejected })) if rejected == other)
    );
    let (error, _returned) = manager
        .complete_reserved_actor_loot_operation(&tick, &mut token, foreign)
        .unwrap_err();
    assert_eq!(
        error,
        CreatureLootAccessError::Tick(ActorTickAccessError::OperationMismatch { guid: other })
    );
    assert_eq!(token.actor_operation.as_ref().unwrap().guid, guid);
    manager
        .complete_reserved_actor_loot_operation(&tick, &mut token, handle)
        .unwrap();
}

#[test]
fn foreign_witness_failure_returns_handle_and_never_clears_slot() {
    let (mut manager, guid) = manager_with_actor(93006);
    let other = insert_actor(&mut manager, 93007, Position::xyz(11.0, 20.0, 30.0), false);
    let foreign = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor_witness(other)
        .unwrap();
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let (mut handle, _) = begin(&mut manager, &tick, &mut token, guid);
    let original = std::mem::replace(&mut handle.witness, foreign);
    assert!(
        matches!(manager.force_reserved_actor_loot_flags(&tick, &mut token, &handle),
        CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(
            ActorTickAccessError::OperationMismatch { guid: rejected })) if rejected == guid)
    );
    let (error, mut returned) = manager
        .complete_reserved_actor_loot_operation(&tick, &mut token, handle)
        .unwrap_err();
    assert_eq!(
        error,
        CreatureLootAccessError::Tick(ActorTickAccessError::OperationMismatch { guid })
    );
    assert!(token.actor_operation.is_some());
    returned.witness = original;
    manager
        .complete_reserved_actor_loot_operation(&tick, &mut token, returned)
        .unwrap();
    assert!(token.actor_operation.is_none());
}

#[test]
fn foreign_real_token_and_manager_cannot_resume_or_dispose() {
    let (mut manager, guid) = manager_with_actor(93008);
    let (mut other, _) = manager_with_actor(93008);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let (handle, _) = begin(&mut manager, &tick, &mut token, guid);
    let (foreign_tick, mut foreign_token) = start(
        &mut other,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    assert!(matches!(
        other.force_reserved_actor_loot_flags(&foreign_tick, &mut foreign_token, &handle),
        CreatureLootAccess::Rejected(CreatureLootAccessError::WrongOrigin)
    ));
    assert!(matches!(
        manager.force_reserved_actor_loot_flags(&foreign_tick, &mut foreign_token, &handle),
        CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(ActorTickAccessError::Tick(
            ObjectMapTickError::OriginMismatch { .. }
        )))
    ));
    let (_, returned) = manager
        .complete_reserved_actor_loot_operation(&foreign_tick, &mut foreign_token, handle)
        .unwrap_err();
    assert!(token.actor_operation.is_some());
    assert!(foreign_token.actor_operation.is_none());
    manager
        .complete_reserved_actor_loot_operation(&tick, &mut token, returned)
        .unwrap();
}

#[test]
fn earlier_real_epoch_cannot_clear_a_new_reservation_for_the_same_actor() {
    let (mut manager, guid) = manager_with_actor(93009);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let (old, _) = begin(&mut manager, &tick, &mut token, guid);
    // Retain an obsolete handle across explicit private disposal to exercise
    // the epoch fence. Public successful completion consumes that handle.
    manager
        .complete_actor_operation(&tick, &mut token, guid, &old.witness)
        .unwrap();
    finish(&mut manager, tick, token);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let (current, _) = begin(&mut manager, &tick, &mut token, guid);
    assert_ne!(old.reserved_epoch, current.reserved_epoch);
    assert!(
        matches!(manager.force_reserved_actor_loot_flags(&tick, &mut token, &old),
        CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(
            ActorTickAccessError::OperationMismatch { guid: rejected })) if rejected == guid)
    );
    let (error, _returned) = manager
        .complete_reserved_actor_loot_operation(&tick, &mut token, old)
        .unwrap_err();
    assert_eq!(
        error,
        CreatureLootAccessError::Tick(ActorTickAccessError::OperationMismatch { guid })
    );
    assert!(token.actor_operation.is_some());
    manager
        .complete_reserved_actor_loot_operation(&tick, &mut token, current)
        .unwrap();
}

#[test]
fn actor_aba_rejects_resume_but_allows_disposal_without_writing_replacement() {
    let (mut manager, guid) = manager_with_actor(93010);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let (handle, _) = begin(&mut manager, &tick, &mut token, guid);
    manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .remove_map_object(guid)
        .unwrap();
    insert_actor(&mut manager, 93010, Position::xyz(10.0, 20.0, 30.0), true);
    let before = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor(guid)
        .unwrap()
        .creature
        .unit()
        .values_update();
    assert!(
        matches!(manager.force_reserved_actor_loot_flags(&tick, &mut token, &handle),
        CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(
            ActorTickAccessError::WitnessMismatch { guid: rejected })) if rejected == guid)
    );
    assert!(token.actor_operation.is_some());
    manager
        .complete_reserved_actor_loot_operation(&tick, &mut token, handle)
        .unwrap();
    assert!(token.actor_operation.is_none());
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
fn stale_map_rejects_resume_but_disposal_uses_original_token_identity() {
    let (mut manager, guid) = manager_with_actor(93011);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let (handle, _) = begin(&mut manager, &tick, &mut token, guid);
    let before = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor(guid)
        .unwrap()
        .creature
        .unit()
        .values_update();
    manager.map_incarnations_like_cpp.remove(&handle.key());
    assert!(matches!(
        manager.force_reserved_actor_loot_flags(&tick, &mut token, &handle),
        CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(
            ActorTickAccessError::StaleParticipant { .. }
        ))
    ));
    assert!(token.actor_operation.is_some());
    manager
        .complete_reserved_actor_loot_operation(&tick, &mut token, handle)
        .unwrap();
    assert!(token.actor_operation.is_none());
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
fn unreserved_handle_cannot_use_another_operation_even_on_same_actor() {
    let (mut manager, guid) = manager_with_actor(93012);
    let ordinary = match manager.idle_creature_loot_handle(MapKey::new(1, 0), guid) {
        CreatureLootAccess::Ready(handle) => handle,
        value => panic!("{value:?}"),
    };
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let (reserved, _) = begin(&mut manager, &tick, &mut token, guid);
    assert!(
        matches!(manager.force_reserved_actor_loot_flags(&tick, &mut token, &ordinary),
        CreatureLootAccess::Rejected(CreatureLootAccessError::Tick(
            ActorTickAccessError::OperationMismatch { guid: rejected })) if rejected == guid)
    );
    assert!(
        manager
            .complete_reserved_actor_loot_operation(&tick, &mut token, ordinary)
            .is_err()
    );
    assert!(token.actor_operation.is_some());
    manager
        .complete_reserved_actor_loot_operation(&tick, &mut token, reserved)
        .unwrap();
}
