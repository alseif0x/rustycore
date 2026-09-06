//! Incarnation-safe retirement through the actual production Session cleanup.
use super::*;

async fn repeated_cleanup(replace_before_first: bool) {
    let (mut session, port, _output, _receiver) = hydrate(true, true, true).await;
    let guid = ObjectGuid::create_player(1, 42);
    session.kick("controlled old-session cleanup");
    if !replace_before_first {
        session.cleanup_shared_runtime_state();
    }
    let replacement = {
        let mut manager = port.manager.lock().unwrap();
        let mut player = Box::new(wow_entities::Player::new(Some(1), false));
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().set_level(73);
        let handle = manager.install_detached_player_like_cpp(player).unwrap();
        manager
            .attach_player_like_cpp(
                handle,
                wow_map::MapKey::new(0, 0),
                Position::new(0.0, 0.0, 0.0, 0.0),
            )
            .unwrap();
        handle
    };
    for attempt in 1..=2 {
        session.cleanup_shared_runtime_state();
        let manager = port.manager.lock().unwrap();
        assert_eq!(
            manager.with_player_like_cpp(replacement, |player| player.unit().data().level),
            Some(73),
            "old session must not erase replacement on cleanup attempt {attempt}"
        );
        assert!(
            manager
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_typed_player(guid)
                .is_some()
        );
    }
}

#[tokio::test]
async fn production_repeated_cleanup_preserves_replacement_of_stale_owner() {
    repeated_cleanup(true).await;
}

#[tokio::test]
async fn production_repeated_cleanup_preserves_replacement_after_successful_retirement() {
    repeated_cleanup(false).await;
}

#[tokio::test]
async fn production_failed_retirement_keeps_the_exact_token_for_a_later_attempt() {
    let (mut session, port, _output, _receiver) = hydrate(true, true, true).await;
    let guid = ObjectGuid::create_player(1, 42);
    session.kick("controlled missing backing Player during retirement");
    // Inject an inconsistent owner/backing-value boundary, retaining the actual
    // Box locally. This is failure injection, not a supported production transfer.
    let mut player = port
        .manager
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .remove_from_map_like_cpp(guid, false)
        .unwrap()
        .player
        .unwrap();
    session.cleanup_shared_runtime_state();
    player.unit_mut().world_mut().set_map(0, 0).unwrap();
    port.manager
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_boxed_player(player).unwrap(),
        )
        .unwrap();
    session.cleanup_shared_runtime_state();
    let mut manager = port.manager.lock().unwrap();
    assert!(
        manager
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(guid)
            .is_none()
    );
    // Retirement must remove the ownership index too, not just the map record.
    assert!(matches!(
        manager.adopt_active_player_like_cpp(guid),
        Err(wow_map::PlayerOwnerError::ActivePlayerMissing { .. })
    ));
}
