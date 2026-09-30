//! APP keeps the original work on preflight error; finalized None is settled.
use super::*;
use wow_core::ObjectGuid;
use wow_map::{Map, MapManager, MapTickCoordinationStateLikeCpp, ObjectMapTickError};

fn setup(maps: &[u32]) -> (MapManager, CanonicalObjectWork) {
    let mut manager = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    for id in maps {
        manager.create_world_map(*id, 0);
    }
    manager.updater.activate(1);
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let object_tick = manager.begin_object_tick(plan).unwrap();
    (
        manager,
        CanonicalObjectWork {
            object_tick,
            respawn_summary: Default::default(),
        },
    )
}

fn no_record(
    _: &mut Map,
    _: wow_map::SpawnObjectType,
    _: wow_map::SpawnId,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    None
}

fn drain(manager: &mut MapManager, work: &mut CanonicalObjectWork) {
    while let Some(token) = work.prepare_next(manager).unwrap() {
        if let Err((error, _token)) = work.try_finish_map(manager, token, None, &mut no_record) {
            panic!("actual map token must finish: {error:?}");
        }
    }
}

#[test]
fn incomplete_complete_returns_original_summary_allocation_then_delivers_once() {
    let (mut manager, mut work) = setup(&[1, 2]);
    let owner = ObjectGuid::EMPTY;
    work.respawn_summary
        .expired_pvp_combat_refs
        .push((1, 0, owner, owner));
    work.respawn_summary.respawn_db_delete_failed = 23;
    let original_vector = work.respawn_summary.expired_pvp_combat_refs.as_ptr();
    let maps = wow_data::MapStore::from_entries([]);
    work = match work.try_complete(&mut manager, &maps) {
        Err((
            ObjectMapTickError::Incomplete {
                processed_participants: 0,
                total_participants: 2,
            },
            work,
        )) => work,
        _ => panic!("preflight error must retain original summary"),
    };
    assert_eq!(
        work.respawn_summary.expired_pvp_combat_refs.as_ptr(),
        original_vector
    );
    assert_eq!(
        work.respawn_summary.expired_pvp_combat_refs,
        [(1, 0, owner, owner)]
    );
    assert_eq!(work.respawn_summary.respawn_db_delete_failed, 23);
    assert_eq!(manager.updater.wait_calls(), 0);
    for id in [1, 2] {
        assert!(
            manager
                .find_map(id, 0)
                .unwrap()
                .delayed_update_calls()
                .is_empty()
        );
    }
    drain(&mut manager, &mut work);
    let summary = match work.try_complete(&mut manager, &maps) {
        Ok(Some(summary)) => summary,
        _ => panic!("original nonempty summary must be delivered"),
    };
    assert_eq!(summary.expired_pvp_combat_refs.as_ptr(), original_vector);
    assert_eq!(summary.expired_pvp_combat_refs, [(1, 0, owner, owner)]);
    assert_eq!(summary.respawn_db_delete_failed, 23);
    assert_eq!(manager.updater.wait_calls(), 1);
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Idle
    );
    for id in [1, 2] {
        assert_eq!(
            manager.find_map(id, 0).unwrap().delayed_update_calls(),
            [200]
        );
    }
}

#[test]
fn inflight_complete_returns_work_until_its_actual_token_finishes() {
    let (mut manager, mut work) = setup(&[1]);
    let token = work.prepare_next(&mut manager).unwrap().unwrap();
    let incarnation = token.incarnation();
    let maps = wow_data::MapStore::from_entries([]);
    work = match work.try_complete(&mut manager, &maps) {
        Err((ObjectMapTickError::MapInFlight { participant }, work)) => {
            assert_eq!(participant.incarnation, incarnation);
            work
        }
        _ => panic!("in-flight rejection must return original work"),
    };
    assert_eq!(manager.updater.wait_calls(), 0);
    if let Err((error, _token)) = work.try_finish_map(&mut manager, token, None, &mut no_record) {
        panic!("original token retry rejected: {error:?}");
    }
    drain(&mut manager, &mut work);
    assert!(matches!(work.try_complete(&mut manager, &maps), Ok(None)));
    assert_eq!(manager.updater.wait_calls(), 1);
    assert_eq!(
        manager.find_map(1, 0).unwrap().delayed_update_calls(),
        [200]
    );
}

#[test]
fn foreign_complete_returns_original_work_and_summary_to_its_actual_manager() {
    let (mut owner, mut work) = setup(&[1]);
    let (mut foreign, _foreign_work) = setup(&[1]);
    work.respawn_summary.maps_evaluated = 19;
    drain(&mut owner, &mut work);
    let maps = wow_data::MapStore::from_entries([]);
    let work = match work.try_complete(&mut foreign, &maps) {
        Err((ObjectMapTickError::OriginMismatch { .. }, work)) => work,
        _ => panic!("foreign manager must return the original APP work"),
    };
    assert_eq!(work.respawn_summary.maps_evaluated, 19);
    assert_eq!(foreign.updater.wait_calls(), 0);
    assert!(
        foreign
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .is_empty()
    );
    let summary = match work.try_complete(&mut owner, &maps) {
        Ok(Some(summary)) => summary,
        _ => panic!("actual owner must finalize once"),
    };
    assert_eq!(summary.maps_evaluated, 19);
    assert_eq!(owner.updater.wait_calls(), 1);
    assert!(foreign.begin_tick_like_cpp(1).is_busy());
}

#[test]
fn finalized_empty_tail_is_settled_idle_and_next_tick_does_not_replay_delay() {
    let (mut manager, mut work) = setup(&[1]);
    drain(&mut manager, &mut work);
    let maps = wow_data::MapStore::from_entries([]);
    assert!(matches!(work.try_complete(&mut manager, &maps), Ok(None)));
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Idle
    );
    assert_eq!(manager.updater.wait_calls(), 1);
    assert_eq!(
        manager.find_map(1, 0).unwrap().delayed_update_calls(),
        [200]
    );
    assert!(
        manager
            .take_player_visibility_refresh_intents_like_cpp()
            .is_empty()
    );
    assert!(
        manager
            .find_map_mut(1, 0)
            .unwrap()
            .map_mut()
            .take_object_visibility_destroy_recipients_like_cpp()
            .is_empty()
    );
    assert!(manager.begin_tick_like_cpp(199).into_started().is_none());
    assert_eq!(
        manager.find_map(1, 0).unwrap().delayed_update_calls(),
        [200]
    );
    assert_eq!(manager.updater.wait_calls(), 1);
    assert_eq!(
        manager
            .begin_tick_like_cpp(1)
            .into_started()
            .unwrap()
            .effective_diff_ms(),
        200
    );
}

#[test]
fn old_complete_still_discards_incomplete_work_and_leaves_manager_busy() {
    let (mut manager, work) = setup(&[1]);
    let maps = wow_data::MapStore::from_entries([]);
    assert!(work.complete(&mut manager, &maps).is_none());
    assert!(manager.begin_tick_like_cpp(1).is_busy());
    assert_eq!(manager.updater.wait_calls(), 0);
    assert!(
        manager
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .is_empty()
    );
}
