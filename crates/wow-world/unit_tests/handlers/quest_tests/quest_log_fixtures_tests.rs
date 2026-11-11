use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};
use wow_map::{MapKey, MapManager, PlayerHandle, PlayerResidenceLikeCpp};

type Manager = Arc<Mutex<MapManager>>;

fn owned_session() -> (WorldSession, Manager, PlayerHandle, usize) {
    let (mut session, _send_rx) = make_session();
    install_native_quest_log_player_like_cpp(&mut session);
    let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    let handle = session.core.player_handle_like_cpp.unwrap();
    let registry = session
        .core
        .player_registry
        .get_or_insert_with(|| Arc::new(PlayerRegistry::new()));
    assert!(registry.bind_canonical_map_manager(Arc::clone(&manager)));
    let allocation = {
        let mut guard = manager.lock().unwrap();
        assert_eq!(
            guard.checked_player_residence_like_cpp(handle),
            Ok(PlayerResidenceLikeCpp::Active(MapKey::new(571, 0)))
        );
        guard
            .with_player_mut_like_cpp(handle, |player| {
                player.set_money(123_456);
                player.gameplay_state_mut().quests.insert_status_like_cpp(
                    64_000,
                    wow_entities::PlayerQuestStatusRecord {
                        quest_id: 64_000,
                        status: 3,
                        explored: true,
                        accept_time_secs: 12,
                        end_time_secs: 34,
                        objective_counts: vec![7, 8],
                        slot: 5,
                    },
                );
                let mut item = wow_entities::Item::default();
                item.object_mut().create(ObjectGuid::create_item(1, 64_001));
                item.object_mut().set_entry(990);
                item.set_count(7);
                assert!(player.trade_store_item_like_cpp(
                    item,
                    64_001,
                    None,
                    wow_entities::INVENTORY_SLOT_BAG_0,
                    wow_entities::INVENTORY_SLOT_ITEM_START,
                ));
                player as *const Player as usize
            })
            .unwrap()
    };
    (session, manager, handle, allocation)
}

fn assert_player_data(player: &Player, allocation: usize, position: Position) {
    assert_eq!(player as *const Player as usize, allocation);
    assert_eq!(player.unit().world().position(), position);
    assert_eq!(player.money(), 123_456);
    let status = player
        .gameplay_state()
        .quests
        .status_like_cpp(64_000)
        .unwrap();
    assert_eq!(
        (status.quest_id, status.status, status.slot),
        (64_000, 3, 5)
    );
    assert_eq!(
        (
            status.accept_time_secs,
            status.end_time_secs,
            status.explored
        ),
        (12, 34, true)
    );
    assert_eq!(status.objective_counts, vec![7, 8]);
    let item = player
        .trade_item_at_like_cpp(
            wow_entities::INVENTORY_SLOT_BAG_0,
            wow_entities::INVENTORY_SLOT_ITEM_START,
        )
        .unwrap();
    assert_eq!(item.object().guid(), ObjectGuid::create_item(1, 64_001));
    assert_eq!((item.object().entry(), item.count()), (990, 7));
    assert_eq!(item.owner_guid(), player.guid());
}

fn assert_same_owner(
    session: &WorldSession,
    manager: &Manager,
    handle: PlayerHandle,
    allocation: usize,
    position: Position,
) {
    assert_eq!(session.core.player_handle_like_cpp, Some(handle));
    assert!(Arc::ptr_eq(
        session.core.canonical_map_manager.as_ref().unwrap(),
        manager,
    ));
    let registered = session
        .core
        .player_registry
        .as_ref()
        .unwrap()
        .canonical_map_manager_like_cpp()
        .unwrap();
    assert!(Arc::ptr_eq(&registered, manager));
    let guard = manager.lock().unwrap();
    assert_eq!(
        guard.checked_player_residence_like_cpp(handle),
        Ok(PlayerResidenceLikeCpp::Active(MapKey::new(571, 0)))
    );
    assert_eq!(
        guard
            .current_player_admission_like_cpp(handle.guid())
            .unwrap()
            .0,
        handle
    );
    guard
        .with_player_like_cpp(handle, |player| {
            assert_player_data(player, allocation, position)
        })
        .unwrap();
}

#[test]
fn quest_log_fixture_already_resident_is_a_noop_for_the_same_owner() {
    let (mut session, manager, handle, allocation) = owned_session();
    let before = manager
        .lock()
        .unwrap()
        .player_active_residence_revision_like_cpp(handle);
    let position = manager
        .lock()
        .unwrap()
        .with_player_like_cpp(handle, |player| player.unit().world().position())
        .unwrap();

    ensure_quest_log_map_player_like_cpp(&mut session);

    assert_eq!(
        manager
            .lock()
            .unwrap()
            .player_active_residence_revision_like_cpp(handle),
        before
    );
    assert_same_owner(&session, &manager, handle, allocation, position);
}

#[test]
fn quest_log_fixture_attaches_the_same_detached_player_at_its_real_position() {
    let (mut session, manager, handle, allocation) = owned_session();
    let position = Position::new(31.0, 42.0, 7.0, 0.75);
    {
        let mut guard = manager.lock().unwrap();
        guard.detach_player_like_cpp(handle).unwrap();
        guard
            .with_player_mut_like_cpp(handle, |player| {
                player.unit_mut().world_mut().relocate(position)
            })
            .unwrap();
        assert_eq!(
            guard.checked_player_residence_like_cpp(handle),
            Ok(PlayerResidenceLikeCpp::Detached)
        );
        assert!(
            guard
                .find_map(571, 0)
                .unwrap()
                .map()
                .get_typed_player(handle.guid())
                .is_none()
        );
    }

    ensure_quest_log_map_player_like_cpp(&mut session);

    assert_same_owner(&session, &manager, handle, allocation, position);
}

#[test]
fn quest_log_fixture_moves_an_active_owner_without_leaving_a_second_player() {
    for old in [MapKey::new(0, 7), MapKey::new(571, 9)] {
        let (mut session, manager, handle, allocation) = owned_session();
        let position = Position::new(51.0, 62.0, 3.0, 1.25);
        {
            let mut guard = manager.lock().unwrap();
            guard.create_world_map(old.map_id, old.instance_id);
            guard.detach_player_like_cpp(handle).unwrap();
            guard.attach_player_like_cpp(handle, old, position).unwrap();
        }

        ensure_quest_log_map_player_like_cpp(&mut session);

        assert_same_owner(&session, &manager, handle, allocation, position);
        assert!(
            manager
                .lock()
                .unwrap()
                .find_map(old.map_id, old.instance_id)
                .unwrap()
                .map()
                .get_typed_player(handle.guid())
                .is_none()
        );
    }
}

#[test]
fn quest_log_fixture_rejects_missing_manager_or_handle_without_replacing_them() {
    let (mut session, manager, handle, _) = owned_session();
    session.core.canonical_map_manager = None;
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            ensure_quest_log_map_player_like_cpp(&mut session)
        }))
        .is_err()
    );
    assert!(session.core.canonical_map_manager.is_none());
    assert_eq!(session.core.player_handle_like_cpp, Some(handle));
    assert!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, Player::guid)
            .is_some()
    );

    let (mut session, manager, handle, _) = owned_session();
    session.core.player_handle_like_cpp = None;
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            ensure_quest_log_map_player_like_cpp(&mut session)
        }))
        .is_err()
    );
    assert!(session.core.player_handle_like_cpp.is_none());
    assert!(Arc::ptr_eq(
        session.core.canonical_map_manager.as_ref().unwrap(),
        &manager
    ));
    assert!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, Player::guid)
            .is_some()
    );
}

#[test]
fn quest_log_fixture_rejects_a_retired_owner_without_fabricating_a_player() {
    let (mut session, manager, handle, allocation) = owned_session();
    let retired = manager
        .lock()
        .unwrap()
        .retire_player_like_cpp(handle)
        .unwrap();
    assert_eq!(retired.as_ref() as *const Player as usize, allocation);
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            ensure_quest_log_map_player_like_cpp(&mut session)
        }))
        .is_err()
    );
    assert_eq!(session.core.player_handle_like_cpp, Some(handle));
    assert!(Arc::ptr_eq(
        session.core.canonical_map_manager.as_ref().unwrap(),
        &manager
    ));
    let guard = manager.lock().unwrap_or_else(|poison| poison.into_inner());
    assert!(
        guard
            .current_player_admission_like_cpp(handle.guid())
            .is_none()
    );
    assert!(
        guard
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(handle.guid())
            .is_none()
    );
    assert_eq!(retired.money(), 123_456);
}

#[test]
fn quest_log_fixture_rejects_a_stale_handle_and_keeps_the_new_incarnation_detached() {
    let (mut session, manager, old, allocation) = owned_session();
    let current = {
        let mut guard = manager.lock().unwrap();
        let player = guard.retire_player_like_cpp(old).unwrap();
        guard.install_detached_player_like_cpp(player).unwrap()
    };
    assert_ne!(old.generation(), current.generation());
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            ensure_quest_log_map_player_like_cpp(&mut session)
        }))
        .is_err()
    );
    assert_eq!(session.core.player_handle_like_cpp, Some(old));
    assert!(Arc::ptr_eq(
        session.core.canonical_map_manager.as_ref().unwrap(),
        &manager
    ));
    let guard = manager.lock().unwrap_or_else(|poison| poison.into_inner());
    assert_eq!(
        guard.checked_player_residence_like_cpp(current),
        Ok(PlayerResidenceLikeCpp::Detached)
    );
    assert!(guard.with_player_like_cpp(old, Player::guid).is_none());
    guard
        .with_player_like_cpp(current, |player| {
            assert_eq!(player as *const Player as usize, allocation);
            assert_eq!(player.money(), 123_456);
        })
        .unwrap();
    assert!(
        guard
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(old.guid())
            .is_none()
    );
}
