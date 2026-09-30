//! Real selected tokens and operation reservations; the production entry is dormant.
use super::*;
use crate::manager::actor_tick_access::fixtures::*;
use crate::manager::{MapObjectUpdateSelectionLikeCpp, ObjectMapTickError};
use wow_constants::{DeathState, UnitDynFlags};
use wow_core::Position;

fn assert_live_untouched(manager: &MapManager, guid: ObjectGuid) {
    let actor = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor(guid)
        .unwrap();
    assert_eq!(actor.current_hp(), 75);
    assert_eq!(actor.creature.unit().death_state(), DeathState::Alive);
    assert!(
        !actor
            .creature
            .unit()
            .world()
            .object()
            .has_dynamic_flag(UnitDynFlags::Lootable as u32)
    );
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 0);
}

#[test]
fn selected_finalization_uses_same_owner_and_retains_successful_pending_slot() {
    let (mut manager, guid) = manager_with_actor(921);
    let (tick, mut token) = start(
        &mut manager,
        431,
        MapObjectUpdateSelectionLikeCpp::NearbyCells,
    );
    let witness = manager
        .begin_actor_operation(&tick, &mut token, guid, None)
        .unwrap();
    manager
        .with_selected_actor(&tick, &mut token, guid, Some(&witness), |actor| {
            let mut damage = actor.begin_represented_damage().unwrap().apply(
                75,
                ObjectGuid::create_player(1, 1),
                None,
                false,
                false,
                1.0,
                1.0,
            );
            assert!(damage.died());
            let _ = damage.stop_after_kill();
            let _ = damage.finish();
        })
        .unwrap();
    let _ = manager
        .finalize_actor_kill(&tick, &mut token, guid, &witness, true, true)
        .unwrap();
    assert_eq!(token.effective_diff_ms(), 431);
    let actor = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor(guid)
        .unwrap();
    assert_eq!(actor.current_hp(), 0);
    assert_eq!(actor.creature.unit().death_state(), DeathState::Corpse);
    assert!(
        actor
            .creature
            .unit()
            .world()
            .object()
            .has_dynamic_flag(UnitDynFlags::Lootable as u32)
    );
    assert!(
        actor
            .creature
            .unit()
            .world()
            .object()
            .has_dynamic_flag(UnitDynFlags::CanSkin as u32)
    );
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 0);
    let deadline = actor.creature.ai_ownership().corpse_despawn_at_ms;
    let _ = manager
        .finalize_actor_kill(&tick, &mut token, guid, &witness, true, true)
        .unwrap();
    assert_eq!(
        manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .creature_actor(guid)
            .unwrap()
            .creature
            .ai_ownership()
            .corpse_despawn_at_ms,
        deadline
    );
    assert_eq!(
        manager
            .begin_actor_operation(&tick, &mut token, guid, Some(&witness))
            .unwrap_err(),
        ActorTickAccessError::Tick(ObjectMapTickError::ActorOperationInFlight { guid })
    );
    manager
        .complete_actor_operation(&tick, &mut token, guid, &witness)
        .unwrap();
    assert!(token.actor_operation.is_none());
}

#[test]
fn missing_slot_rejects_before_corpse_flag_mutation() {
    let (mut manager, guid) = manager_with_actor(922);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let witness = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor_witness(guid)
        .unwrap();
    assert_eq!(
        manager
            .finalize_actor_kill(&tick, &mut token, guid, &witness, true, true)
            .unwrap_err(),
        ActorTickAccessError::NoActorOperation
    );
    assert!(token.actor_operation.is_none());
    assert_live_untouched(&manager, guid);
}

#[test]
fn foreign_guid_and_witness_do_not_release_the_reserved_operation() {
    let (mut manager, guid) = manager_with_actor(923);
    let other = insert_actor(&mut manager, 924, Position::xyz(11.0, 20.0, 30.0), false);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let witness = manager
        .begin_actor_operation(&tick, &mut token, guid, None)
        .unwrap();
    let other_witness = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor_witness(other)
        .unwrap();
    assert_eq!(
        manager
            .finalize_actor_kill(&tick, &mut token, other, &other_witness, true, true)
            .unwrap_err(),
        ActorTickAccessError::OperationMismatch { guid: other }
    );
    assert_eq!(
        manager
            .finalize_actor_kill(&tick, &mut token, guid, &other_witness, true, true)
            .unwrap_err(),
        ActorTickAccessError::OperationMismatch { guid }
    );
    assert_live_untouched(&manager, guid);
    assert_live_untouched(&manager, other);
    manager
        .resume_actor_operation(&tick, &mut token, guid, &witness)
        .unwrap();
    manager
        .complete_actor_operation(&tick, &mut token, guid, &witness)
        .unwrap();
}

#[test]
fn same_guid_readmission_rejects_without_touching_replacement_and_allows_explicit_disposal() {
    let (mut manager, guid) = manager_with_actor(925);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::NearbyCells,
    );
    let witness = manager
        .begin_actor_operation(&tick, &mut token, guid, None)
        .unwrap();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    let removed = map.remove_map_object(guid).unwrap();
    map.admit_creature_actor(new_actor(925, Position::xyz(10.0, 20.0, 30.0), true))
        .unwrap();
    assert_eq!(
        manager
            .finalize_actor_kill(&tick, &mut token, guid, &witness, true, true)
            .unwrap_err(),
        ActorTickAccessError::WitnessMismatch { guid }
    );
    assert_live_untouched(&manager, guid);
    assert!(token.actor_operation.is_some());
    manager
        .complete_actor_operation(&tick, &mut token, guid, &witness)
        .unwrap();
    assert!(token.actor_operation.is_none());
    drop(removed);
}

#[test]
fn stale_map_rejects_without_touching_replacement_or_clearing_pending_slot() {
    let (mut manager, guid) = manager_with_actor(926);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::NearbyCells,
    );
    let witness = manager
        .begin_actor_operation(&tick, &mut token, guid, None)
        .unwrap();
    let admitted_incarnation = token.incarnation();
    assert!(manager.destroy_map(1, 0));
    manager.create_world_map(1, 0);
    assert_eq!(
        insert_actor(&mut manager, 926, Position::xyz(10.0, 20.0, 30.0), true),
        guid
    );
    let current_incarnation = manager.map_incarnation_like_cpp(token.key());
    assert_eq!(
        manager
            .finalize_actor_kill(&tick, &mut token, guid, &witness, true, true)
            .unwrap_err(),
        ActorTickAccessError::StaleParticipant {
            key: token.key(),
            admitted_incarnation,
            current_incarnation
        }
    );
    assert_live_untouched(&manager, guid);
    assert!(token.actor_operation.is_some());
    manager
        .complete_actor_operation(&tick, &mut token, guid, &witness)
        .unwrap();
    assert!(token.actor_operation.is_none());
}

#[test]
fn foreign_token_rejects_before_operation_or_actor_mutation() {
    let (mut first, guid) = manager_with_actor(927);
    let (mut second, _) = manager_with_actor(927);
    let (tick, mut token) = start(
        &mut first,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let (_second_tick, second_token) = start(
        &mut second,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let witness = first
        .begin_actor_operation(&tick, &mut token, guid, None)
        .unwrap();
    assert!(matches!(
        second.finalize_actor_kill(&tick, &mut token, guid, &witness, true, true),
        Err(ActorTickAccessError::Tick(
            ObjectMapTickError::OriginMismatch { .. }
        ))
    ));
    assert_live_untouched(&first, guid);
    assert_live_untouched(&second, guid);
    assert!(second_token.actor_operation.is_none());
    first
        .resume_actor_operation(&tick, &mut token, guid, &witness)
        .unwrap();
    assert!(token.actor_operation.is_some());
}

#[test]
fn unselected_and_removed_actors_fail_closed_with_original_error_priority() {
    let (mut manager, guid) = manager_with_actor(928);
    let far = insert_actor(
        &mut manager,
        929,
        Position::xyz(4000.0, 4000.0, 30.0),
        false,
    );
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::NearbyCells,
    );
    assert_eq!(
        manager
            .begin_actor_operation(&tick, &mut token, far, None)
            .unwrap_err(),
        ActorTickAccessError::OutsideSelection { guid: far }
    );
    let witness = manager
        .begin_actor_operation(&tick, &mut token, guid, None)
        .unwrap();
    let far_witness = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor_witness(far)
        .unwrap();
    assert_eq!(
        manager
            .finalize_actor_kill(&tick, &mut token, far, &far_witness, true, true)
            .unwrap_err(),
        ActorTickAccessError::OperationMismatch { guid: far }
    );
    assert_live_untouched(&manager, far);
    let removed = manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .remove_map_object(guid)
        .unwrap();
    assert_eq!(
        manager
            .finalize_actor_kill(&tick, &mut token, guid, &witness, true, true)
            .unwrap_err(),
        ActorTickAccessError::ActorUnavailable { guid }
    );
    assert!(token.actor_operation.is_some());
    manager
        .complete_actor_operation(&tick, &mut token, guid, &witness)
        .unwrap();
    drop(removed);
}
