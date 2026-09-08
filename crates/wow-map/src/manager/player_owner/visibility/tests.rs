use super::*;
use crate::{
    DEFAULT_VISIBILITY_NOTIFY_PERIOD, GridStateKind, MIN_GRID_DELAY_MS, PeriodicTimer,
    compute_grid_coord,
};
use wow_core::Position;
use wow_entities::{MapObjectRecord, ObjectNotifyFlags, Player};

const KEY: MapKey = MapKey::new(1, 0);

#[test]
fn invalid_viewpoint_height_or_orientation_cannot_publish() {
    let (mut manager, handle) = installed();
    let intent = selected(&mut manager, handle);
    for invalid in [
        Position::new(10.0, 20.0, f32::NAN, 0.0),
        Position::new(10.0, 20.0, 30.0, f32::NAN),
    ] {
        manager
            .with_player_mut_like_cpp(handle, |player| {
                player.unit_mut().world_mut().relocate(invalid);
            })
            .unwrap();
        assert!(!manager.player_visibility_refresh_intent_is_current_like_cpp(intent));
    }
}

fn position() -> Position {
    Position::xyz(10.0, 20.0, 30.0)
}

fn player() -> Box<Player> {
    let mut player = Box::new(Player::new(Some(588_001), false));
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_player(1, 588_001));
    player.set_money(123);
    player
}

fn installed() -> (MapManager, PlayerHandle) {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(KEY.map_id, KEY.instance_id);
    let handle = manager.install_detached_player_like_cpp(player()).unwrap();
    manager
        .attach_player_like_cpp(handle, KEY, position())
        .unwrap();
    manager
        .with_player_mut_like_cpp(handle, |player| {
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .reset_all_notifies();
        })
        .unwrap();
    (manager, handle)
}

fn set_grid(manager: &mut MapManager, state: GridStateKind, remaining_ms: i64) {
    let grid = manager
        .find_map_mut(KEY.map_id, KEY.instance_id)
        .unwrap()
        .map_mut()
        .get_ngrid_mut(compute_grid_coord(position().x, position().y))
        .unwrap();
    grid.set_state(state);
    *grid.info_mut().relocation_timer_mut() =
        PeriodicTimer::new(DEFAULT_VISIBILITY_NOTIFY_PERIOD, remaining_ms);
}

fn notify(manager: &mut MapManager, handle: PlayerHandle) {
    manager
        .with_player_mut_like_cpp(handle, |player| {
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
        })
        .unwrap();
    set_grid(manager, GridStateKind::Active, 0);
}

fn selected(
    manager: &mut MapManager,
    handle: PlayerHandle,
) -> PlayerVisibilityRefreshIntentLikeCpp {
    notify(manager, handle);
    assert_eq!(manager.update(1), Some(1));
    let intents = manager.take_player_visibility_refresh_intents_like_cpp();
    assert_eq!(intents.len(), 1);
    intents[0]
}

#[test]
fn map_tail_exports_current_selection_once_after_notify_reset() {
    for updater_active in [false, true] {
        let (mut manager, handle) = installed();
        if updater_active {
            manager.map_updater_mut().activate(1);
        }
        let intent = selected(&mut manager, handle);
        assert_eq!(intent.handle(), handle);
        assert_eq!(intent.map_key(), KEY);
        assert_eq!(intent.residence_revision(), 1);
        assert_eq!(intent.viewpoint_guid(), handle.guid());
        assert!(manager.player_visibility_refresh_intent_is_current_like_cpp(intent));
        assert_eq!(
            manager.with_player_like_cpp(handle, |player| {
                player
                    .unit()
                    .world()
                    .object()
                    .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
            }),
            Some(false)
        );
        assert!(
            manager
                .take_player_visibility_refresh_intents_like_cpp()
                .is_empty()
        );
        if updater_active {
            assert_eq!(manager.map_updater().wait_calls(), 1);
        }
        manager.update(1);
        assert!(
            manager
                .take_player_visibility_refresh_intents_like_cpp()
                .is_empty()
        );
    }
}

#[test]
fn no_notify_inactive_and_nonexpired_grids_export_nothing() {
    for (marked, state, remaining) in [
        (false, GridStateKind::Active, 0),
        (true, GridStateKind::Idle, 0),
        (true, GridStateKind::Active, 100),
    ] {
        let (mut manager, handle) = installed();
        if marked {
            notify(&mut manager, handle);
        }
        set_grid(&mut manager, state, remaining);
        manager.update(1);
        assert!(
            manager
                .take_player_visibility_refresh_intents_like_cpp()
                .is_empty()
        );
        if marked {
            assert_eq!(
                manager.with_player_like_cpp(handle, |player| {
                    player
                        .unit()
                        .world()
                        .object()
                        .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
                }),
                Some(true)
            );
        }
    }
}

#[test]
fn unmarked_cells_do_not_create_publication_obligations() {
    let (mut manager, handle) = installed();
    notify(&mut manager, handle);
    let managed = manager.find_map_mut(KEY.map_id, KEY.instance_id).unwrap();
    managed.last_process_relocation_notifies_outcome_like_cpp =
        managed.map_mut().process_relocation_notifies_like_cpp(
            std::iter::empty(),
            1,
            DEFAULT_VISIBILITY_NOTIFY_PERIOD,
            std::iter::empty(),
        );
    manager.retain_selected_player_visibility_refreshes_like_cpp([KEY]);
    assert!(
        manager
            .take_player_visibility_refresh_intents_like_cpp()
            .is_empty()
    );
}

#[test]
fn interval_skip_does_not_replay_previous_map_summary() {
    let (mut manager, handle) = installed();
    let _ = selected(&mut manager, handle);
    manager.set_map_update_interval(100);
    assert_eq!(manager.update(1), None);
    assert!(
        manager
            .take_player_visibility_refresh_intents_like_cpp()
            .is_empty()
    );
}

#[test]
fn map_skipped_for_blocked_unload_does_not_replay_previous_selection() {
    let (mut manager, handle) = installed();
    let _ = selected(&mut manager, handle);
    manager
        .find_map_mut(KEY.map_id, KEY.instance_id)
        .unwrap()
        .set_can_unload(true);
    assert_eq!(manager.update(1), Some(1));
    assert!(manager.find_map(KEY.map_id, KEY.instance_id).is_some());
    assert!(
        manager
            .take_player_visibility_refresh_intents_like_cpp()
            .is_empty()
    );
}

#[test]
fn repeated_map_selections_coalesce_until_drained() {
    let (mut manager, handle) = installed();
    for _ in 0..3 {
        notify(&mut manager, handle);
        manager.update(1);
    }
    let intents = manager.take_player_visibility_refresh_intents_like_cpp();
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].handle(), handle);
    assert!(
        manager
            .take_player_visibility_refresh_intents_like_cpp()
            .is_empty()
    );
}

#[test]
fn detached_and_same_map_reattached_player_reject_old_residence() {
    let (mut manager, handle) = installed();
    let old = selected(&mut manager, handle);
    notify(&mut manager, handle);
    manager.update(1);
    manager.detach_player_like_cpp(handle).unwrap();
    assert!(!manager.player_visibility_refresh_intent_is_current_like_cpp(old));
    assert!(
        manager
            .take_player_visibility_refresh_intents_like_cpp()
            .is_empty()
    );
    manager
        .attach_player_like_cpp(handle, KEY, position())
        .unwrap();
    assert!(!manager.player_visibility_refresh_intent_is_current_like_cpp(old));
    let current = selected(&mut manager, handle);
    assert_eq!(current.handle(), old.handle());
    assert_eq!(current.map_key(), old.map_key());
    assert_eq!(current.residence_revision(), old.residence_revision() + 1);
}

#[test]
fn retired_and_replaced_incarnations_reject_old_intents() {
    let (mut manager, old_handle) = installed();
    let old = selected(&mut manager, old_handle);
    notify(&mut manager, old_handle);
    manager.update(1);
    let player = manager.retire_player_like_cpp(old_handle).unwrap();
    assert!(!manager.player_visibility_refresh_intent_is_current_like_cpp(old));
    assert!(
        manager
            .take_player_visibility_refresh_intents_like_cpp()
            .is_empty()
    );
    let new_handle = manager.install_detached_player_like_cpp(player).unwrap();
    manager
        .attach_player_like_cpp(new_handle, KEY, position())
        .unwrap();
    assert!(!manager.player_visibility_refresh_intent_is_current_like_cpp(old));
    let new = selected(&mut manager, new_handle);
    assert_ne!(old.handle(), new.handle());
    assert_eq!(old.residence_revision(), new.residence_revision());
}

#[test]
fn failed_attach_and_exhaustion_preserve_detached_owner() {
    let (mut manager, handle) = installed();
    manager.detach_player_like_cpp(handle).unwrap();
    let before = manager.player_owners_like_cpp[&handle.guid()];
    assert!(
        manager
            .attach_player_like_cpp(handle, MapKey::new(999, 0), position())
            .is_err()
    );
    assert_eq!(manager.player_owners_like_cpp[&handle.guid()], before);
    assert!(
        manager
            .attach_player_like_cpp(handle, KEY, Position::xyz(f32::NAN, 0.0, 0.0))
            .is_err()
    );
    assert_eq!(manager.player_owners_like_cpp[&handle.guid()], before);
    manager
        .player_owners_like_cpp
        .get_mut(&handle.guid())
        .unwrap()
        .residence_revision = u64::MAX;
    let exhausted = manager.player_owners_like_cpp[&handle.guid()];
    assert_eq!(
        manager.attach_player_like_cpp(handle, KEY, position()),
        Err(super::super::PlayerOwnerError::ResidenceRevisionExhausted)
    );
    assert_eq!(manager.player_owners_like_cpp[&handle.guid()], exhausted);
    assert_eq!(
        manager.checked_player_residence_like_cpp(handle),
        Ok(PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        manager.with_player_like_cpp(handle, Player::money),
        Some(123)
    );
    assert!(
        manager
            .find_map(KEY.map_id, KEY.instance_id)
            .unwrap()
            .map()
            .get_typed_player(handle.guid())
            .is_none()
    );
}

#[test]
fn rejected_runtime_attachment_restores_player_without_advancing_revision() {
    let (mut manager, handle) = installed();
    manager.detach_player_like_cpp(handle).unwrap();
    let before = manager.player_owners_like_cpp[&handle.guid()];
    let mut conflicting = *player();
    conflicting.set_money(456);
    conflicting
        .unit_mut()
        .world_mut()
        .set_map(KEY.map_id, KEY.instance_id)
        .unwrap();
    conflicting.unit_mut().world_mut().relocate(position());
    manager
        .find_map_mut(KEY.map_id, KEY.instance_id)
        .unwrap()
        .map_mut()
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_player(conflicting).unwrap())
        .unwrap();
    assert_eq!(
        manager.attach_player_like_cpp(handle, KEY, position()),
        Err(super::super::PlayerOwnerError::ActiveObjectAlreadyPresent {
            guid: handle.guid(),
            key: KEY,
        })
    );
    assert_eq!(manager.player_owners_like_cpp[&handle.guid()], before);
    assert_eq!(
        manager.with_player_like_cpp(handle, Player::money),
        Some(123)
    );
    assert_eq!(
        manager
            .find_map(KEY.map_id, KEY.instance_id)
            .unwrap()
            .map()
            .get_typed_player(handle.guid())
            .unwrap()
            .money(),
        456
    );
}

#[test]
fn changed_or_missing_viewpoint_rejects_exported_request() {
    let (mut manager, handle) = installed();
    let intent = selected(&mut manager, handle);
    manager
        .with_player_mut_like_cpp(handle, |player| {
            player.set_farsight_object_like_cpp(ObjectGuid::create_player(1, 588_002));
        })
        .unwrap();
    assert!(!manager.player_visibility_refresh_intent_is_current_like_cpp(intent));
}

#[test]
fn adoption_stamps_existing_owner_and_unowned_plans_cannot_publish() {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
    manager.create_world_map(KEY.map_id, KEY.instance_id);
    let mut player = *player();
    let guid = player.guid();
    player
        .unit_mut()
        .world_mut()
        .set_map(KEY.map_id, KEY.instance_id)
        .unwrap();
    player.unit_mut().world_mut().relocate(position());
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
    manager
        .find_map_mut(KEY.map_id, KEY.instance_id)
        .unwrap()
        .map_mut()
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    set_grid(&mut manager, GridStateKind::Active, 0);
    manager.update(1);
    assert!(
        manager
            .take_player_visibility_refresh_intents_like_cpp()
            .is_empty()
    );
    let handle = manager.adopt_active_player_like_cpp(guid).unwrap();
    let intent = selected(&mut manager, handle);
    assert_eq!(intent.residence_revision(), 1);
}

fn observer(manager: &mut MapManager, counter: i64, position: Position) -> PlayerHandle {
    let mut player = player();
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_player(1, counter));
    let handle = manager.install_detached_player_like_cpp(player).unwrap();
    manager
        .attach_player_like_cpp(handle, KEY, position)
        .unwrap();
    manager
        .with_player_mut_like_cpp(handle, |player| {
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .reset_all_notifies();
        })
        .unwrap();
    handle
}

#[test]
fn nearby_reciprocal_observers_are_fenced_and_coalesced_without_new_notifies() {
    for both_notified in [false, true] {
        let (mut manager, source) = installed();
        let nearby = observer(&mut manager, 588_003, Position::xyz(11.0, 20.0, 30.0));
        let far = observer(&mut manager, 588_004, Position::xyz(5_000.0, 20.0, 30.0));
        notify(&mut manager, source);
        if both_notified {
            notify(&mut manager, nearby);
        }
        manager.update(1);
        let intents = manager.take_player_visibility_refresh_intents_like_cpp();
        assert_eq!(intents.len(), 2);
        assert!(intents.iter().any(|intent| intent.handle() == source));
        let reciprocal = intents
            .iter()
            .find(|intent| intent.handle() == nearby)
            .unwrap();
        assert_eq!(reciprocal.viewpoint_guid(), nearby.guid());
        assert!(manager.player_visibility_refresh_intent_is_current_like_cpp(*reciprocal));
        assert!(!intents.iter().any(|intent| intent.handle() == far));
        assert_eq!(
            manager.with_player_like_cpp(nearby, |player| {
                player
                    .unit()
                    .world()
                    .object()
                    .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
            }),
            Some(false)
        );
        set_grid(&mut manager, GridStateKind::Active, 0);
        manager.update(1);
        assert!(
            manager
                .take_player_visibility_refresh_intents_like_cpp()
                .is_empty(),
            "publication must not start a notify loop"
        );
    }
}

#[test]
fn reciprocal_recipient_carries_its_own_current_viewpoint() {
    let (mut manager, source) = installed();
    let nearby = observer(&mut manager, 588_003, Position::xyz(11.0, 20.0, 30.0));
    let viewpoint = observer(&mut manager, 588_004, Position::xyz(12.0, 20.0, 30.0));
    manager
        .with_player_mut_like_cpp(nearby, |player| {
            player.set_farsight_object_like_cpp(viewpoint.guid());
        })
        .unwrap();
    notify(&mut manager, source);
    manager.update(1);
    let intents = manager.take_player_visibility_refresh_intents_like_cpp();
    let reciprocal = intents
        .iter()
        .find(|intent| intent.handle() == nearby)
        .unwrap();
    assert_eq!(reciprocal.viewpoint_guid(), viewpoint.guid());
    assert!(manager.player_visibility_refresh_intent_is_current_like_cpp(*reciprocal));
    manager.detach_player_like_cpp(nearby).unwrap();
    manager
        .attach_player_like_cpp(nearby, KEY, Position::xyz(11.0, 20.0, 30.0))
        .unwrap();
    assert!(!manager.player_visibility_refresh_intent_is_current_like_cpp(*reciprocal));
}
