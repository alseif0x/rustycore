use super::*;
use crate::manager::{MIN_GRID_DELAY_MS, ManagedMapKind, PlayerHandle};
use wow_core::Position;
use wow_entities::Player;

fn context(
    expiry_time: u64,
) -> (
    CreateMapInstanceLockContext,
    InstanceMapExpiryContextLikeCpp,
) {
    let entries = wow_instances::MapDb2Entries {
        map_id: 33,
        difficulty_id: 1,
        lock_id: 7,
        reset_interval: wow_instances::MapDifficultyResetInterval::Daily,
        max_players: 10,
        is_flex_locking: false,
        is_using_encounter_locks: false,
    };
    (
        CreateMapInstanceLockContext {
            instance_id: 7,
            difficulty_id: 1,
            token: 0x1234,
            owner_guid_counter: 42,
        },
        InstanceMapExpiryContextLikeCpp {
            current_expiry_time: expiry_time,
            entries,
            reset_schedule: wow_instances::ResetSchedule::default(),
        },
    )
}

fn empty_instance_map(expiry_time: u64) -> (MapManager, MapKey) {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    let key = MapKey::new(33, 7);
    let managed = manager.create_map_entry(
        key.map_id,
        key.instance_id,
        1,
        ManagedMapKind::Dungeon {
            has_reset_schedule: true,
        },
    );
    let (lock_context, expiry_context) = context(expiry_time);
    managed.set_instance_lock_context(Some(lock_context));
    managed.set_instance_expiry_context_like_cpp(Some(expiry_context));
    (manager, key)
}

fn attach_player(
    manager: &mut MapManager,
    key: MapKey,
    counter: i64,
) -> (ObjectGuid, PlayerHandle) {
    let guid = ObjectGuid::create_player(1, counter);
    let mut player = Player::new(None, false);
    player.unit_mut().world_mut().object_mut().create(guid);
    let handle = manager
        .install_detached_player_like_cpp(Box::new(player))
        .unwrap();
    manager
        .attach_player_like_cpp(handle, key, Position::xyz(10.0, 20.0, 30.0))
        .unwrap();
    (guid, handle)
}

#[test]
fn expiry_keeps_constructor_time_strict_zero_and_extended_refresh_semantics_like_cpp() {
    let (mut manager, key) = empty_instance_map(100);
    let map = manager.find_map_mut(key.map_id, key.instance_id).unwrap();
    assert_eq!(map.instance_expire_event_time, Some(100));

    let (lock_context, mut extended_context) = context(200);
    extended_context.current_expiry_time = 200;
    map.set_instance_lock_context(Some(lock_context));
    map.set_instance_expiry_context_like_cpp(Some(extended_context));
    assert_eq!(map.instance_expire_event_time, Some(100));
    assert!(
        map.expire_instance_after_map_update_at_like_cpp(100)
            .is_none()
    );
    assert!(
        map.expire_instance_after_map_update_at_like_cpp(101)
            .is_some()
    );

    let (mut manager, key) = empty_instance_map(0);
    let map = manager.find_map_mut(key.map_id, key.instance_id).unwrap();
    assert!(
        map.expire_instance_after_map_update_at_like_cpp(u64::MAX)
            .is_none()
    );
}

#[test]
fn map_update_owns_due_expiry_and_reset_rearms_once_for_the_next_period_like_cpp() {
    let (mut manager, key) = empty_instance_map(1);
    let (occupant, _) = attach_player(&mut manager, key, 45_001);

    // A direct reset operation is not the scheduled expiry owner.
    assert!(matches!(
        manager.reset_instance_like_cpp(key.map_id, key.instance_id, None),
        InstanceResetResultLikeCpp::NotEmpty { .. }
    ));
    assert!(
        manager
            .take_last_instance_expiry_updates_like_cpp()
            .is_empty()
    );

    assert_eq!(manager.update(1), Some(1));
    let updates = manager.take_last_instance_expiry_updates_like_cpp();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].key, key);
    assert!(matches!(
        &updates[0].update.reset_result,
        InstanceResetResultLikeCpp::NotEmpty { occupant_guids } if occupant_guids == &[occupant]
    ));
    let next_event = manager
        .find_map(key.map_id, key.instance_id)
        .unwrap()
        .instance_expire_event_time;
    assert!(next_event.is_some_and(|time| time > 1));

    assert_eq!(manager.update(1), Some(1));
    assert!(
        manager
            .take_last_instance_expiry_updates_like_cpp()
            .is_empty()
    );
}

#[test]
fn empty_expiry_succeeds_and_arms_next_update_unload_like_cpp() {
    let (mut manager, key) = empty_instance_map(100);
    let event = manager
        .find_map_mut(key.map_id, key.instance_id)
        .unwrap()
        .expire_instance_after_map_update_at_like_cpp(101)
        .expect("strictly expired empty instance");
    assert_eq!(event.reset_result, InstanceResetResultLikeCpp::Success);
    assert!(
        manager
            .find_map(key.map_id, key.instance_id)
            .unwrap()
            .can_unload(1)
    );
    assert!(
        manager
            .find_map(key.map_id, key.instance_id)
            .unwrap()
            .instance_expire_event_time
            .is_some_and(|next| next > 101)
    );
}

#[test]
fn pending_login_is_filtered_without_suppressing_admitted_occupants_like_cpp() {
    let (mut manager, key) = empty_instance_map(1);
    let (pending, _) = attach_player(&mut manager, key, 45_002);
    let (admitted, _) = attach_player(&mut manager, key, 45_003);
    manager.mark_instance_player_entry_pending_like_cpp(key, pending);

    assert_eq!(manager.update(1), Some(1));
    let updates = manager.take_last_instance_expiry_updates_like_cpp();
    assert_eq!(updates.len(), 1);
    assert!(matches!(
        &updates[0].update.reset_result,
        InstanceResetResultLikeCpp::NotEmpty { occupant_guids } if occupant_guids == &[admitted]
    ));
    assert_eq!(updates[0].occupants.len(), 1);
    assert_eq!(updates[0].occupants[0].guid, admitted);
}

#[test]
fn sole_preattached_login_defers_due_event_until_registration_phase_completes_like_cpp() {
    let (mut manager, key) = empty_instance_map(1);
    let (guid, _) = attach_player(&mut manager, key, 45_004);
    manager.mark_instance_player_entry_pending_like_cpp(key, guid);

    assert_eq!(manager.update(1), Some(1));
    assert!(
        manager
            .take_last_instance_expiry_updates_like_cpp()
            .is_empty()
    );
    assert_eq!(
        manager
            .find_map(key.map_id, key.instance_id)
            .unwrap()
            .instance_expire_event_time,
        Some(1),
        "the original due event remains pending until the login is registered"
    );

    manager.complete_instance_player_entry_like_cpp(key, guid);
    assert_eq!(manager.update(1), Some(1));
    assert_eq!(
        manager.take_last_instance_expiry_updates_like_cpp().len(),
        1
    );
}

#[test]
fn last_leave_intent_requires_current_incarnation_token_and_empty_map_like_cpp() {
    let (mut manager, key) = empty_instance_map(1);
    let (_, handle) = attach_player(&mut manager, key, 45_005);
    let first_incarnation = manager.map_incarnation_like_cpp(key).unwrap();
    manager.detach_player_like_cpp(handle).unwrap();
    let snapshot = manager
        .take_pending_instance_map_last_leave_checks_like_cpp()
        .pop()
        .unwrap();
    assert_eq!(snapshot.incarnation, first_incarnation);
    assert!(manager.arm_expired_instance_unload_like_cpp(snapshot));
    assert!(
        manager
            .find_map(key.map_id, key.instance_id)
            .unwrap()
            .can_unload(1)
    );

    let (_, next_handle) = attach_player(&mut manager, key, 45_006);
    assert!(
        !manager
            .find_map(key.map_id, key.instance_id)
            .unwrap()
            .can_unload(1)
    );
    assert!(
        !manager.arm_expired_instance_unload_like_cpp(snapshot),
        "admission makes the old last-leave intent inapplicable"
    );
    manager.detach_player_like_cpp(next_handle).unwrap();
    let stale = manager
        .take_pending_instance_map_last_leave_checks_like_cpp()
        .pop()
        .unwrap();
    let mut renewed_context = stale.instance_lock_context;
    renewed_context.token += 1;
    manager
        .find_map_mut(key.map_id, key.instance_id)
        .unwrap()
        .set_instance_lock_context(Some(renewed_context));
    assert!(!manager.arm_expired_instance_unload_like_cpp(stale));
}

#[test]
fn last_leave_snapshot_cannot_arm_a_recreated_map_incarnation_like_cpp() {
    let (mut manager, key) = empty_instance_map(1);
    let (_, handle) = attach_player(&mut manager, key, 45_007);
    let original_incarnation = manager.map_incarnation_like_cpp(key).unwrap();
    manager.detach_player_like_cpp(handle).unwrap();
    let stale_snapshot = manager
        .take_pending_instance_map_last_leave_checks_like_cpp()
        .pop()
        .unwrap();
    manager
        .find_map_mut(key.map_id, key.instance_id)
        .unwrap()
        .set_can_unload(true);
    assert_eq!(manager.update(1), Some(1));
    assert!(manager.find_map(key.map_id, key.instance_id).is_none());

    let map = manager.create_map_entry(
        key.map_id,
        key.instance_id,
        1,
        ManagedMapKind::Dungeon {
            has_reset_schedule: true,
        },
    );
    let (lock_context, expiry_context) = context(1);
    map.set_instance_lock_context(Some(lock_context));
    map.set_instance_expiry_context_like_cpp(Some(expiry_context));
    assert_ne!(
        manager.map_incarnation_like_cpp(key),
        Some(original_incarnation)
    );
    assert!(!manager.arm_expired_instance_unload_like_cpp(stale_snapshot));
}
