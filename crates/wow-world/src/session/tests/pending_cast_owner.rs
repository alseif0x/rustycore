//! Player-owned queued cast lifecycle, independent of Session fixture storage.

use super::*;

fn request(guid: ObjectGuid, id: i64) -> RepresentedPendingSpellCastRequestLikeCpp {
    RepresentedPendingSpellCastRequestLikeCpp {
        cast_id: ObjectGuid::new(6, id),
        spell_id: 133,
        casting_unit_guid: guid,
        target_guid: guid,
        target_data: Default::default(),
        spell_visual: Default::default(),
        metadata: Default::default(),
    }
}

fn cancelled(id: i64) -> Vec<u8> {
    wow_packet::packets::spell::CastFailed {
        cast_id: ObjectGuid::new(6, id),
        spell_id: 133,
        visual: Default::default(),
        reason: SPELL_FAILED_DONT_REPORT_LIKE_CPP,
        fail_arg1: 0,
        fail_arg2: 0,
    }
    .to_bytes()
}

#[test]
fn pending_cast_uses_active_and_detached_player_and_publishes_replacement_cancel_once() {
    let (mut session, _, send_rx) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            session.mutate_pending_spell_cast_like_cpp(|pending| {
                calls.set(calls.get() + 1);
                assert!(matches!(
                    manager.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                ));
                *pending = Some(request(guid, 1));
            }),
            Some(())
        );
        assert_eq!(calls.get(), 1);
        assert!(manager.try_lock().is_ok());
        session.request_represented_spell_cast_like_cpp(request(guid, 2));
        assert_eq!(
            session.pending_spell_cast_snapshot_like_cpp(),
            Some(Some(request(guid, 2)))
        );
        assert!(
            session
                .represented_pending_spell_cast_request_like_cpp
                .is_none()
        );
        assert_eq!(send_rx.try_recv().unwrap(), cancelled(1));
        assert!(send_rx.is_empty());
        assert!(session.cancel_pending_spell_cast_request_like_cpp());
        assert!(!session.cancel_pending_spell_cast_request_like_cpp());
        assert_eq!(session.pending_spell_cast_snapshot_like_cpp(), Some(None));
        assert_eq!(send_rx.try_recv().unwrap(), cancelled(2));
        assert!(send_rx.is_empty());
    }
}

#[test]
fn stale_pending_cast_cannot_cancel_replace_or_publish_for_new_incarnation() {
    let (mut session, _, send_rx) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.gameplay_state_mut().pending_spell_cast = Some(request(guid, 3));
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    assert_eq!(session.pending_spell_cast_snapshot_like_cpp(), None);
    assert_eq!(
        session.mutate_pending_spell_cast_like_cpp(|_| panic!("stale owner")),
        None::<()>
    );
    assert!(!session.cancel_pending_spell_cast_request_like_cpp());
    session.request_represented_spell_cast_like_cpp(request(guid, 4));
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, |player| {
                player.gameplay_state().pending_spell_cast.clone()
            }),
        Some(Some(request(guid, 3)))
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
    session.canonical_map_manager = None;
    assert_eq!(
        session.mutate_pending_spell_cast_like_cpp(|_| panic!("missing owner")),
        None::<()>
    );
}

#[tokio::test]
async fn pending_tick_cancels_unknown_spell_from_canonical_owner_once() {
    let (mut session, _, send_rx) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.request_represented_spell_cast_like_cpp(request(guid, 5));
    assert!(send_rx.is_empty());
    session.tick_pending_spell_cast_request_like_cpp().await;
    assert_eq!(send_rx.try_recv().unwrap(), cancelled(5));
    assert_eq!(session.pending_spell_cast_snapshot_like_cpp(), Some(None));
    session.tick_pending_spell_cast_request_like_cpp().await;
    assert!(send_rx.is_empty());
}
