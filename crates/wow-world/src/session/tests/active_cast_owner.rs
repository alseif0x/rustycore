//! Canonical Unit cast execution, including detached and stale Player lifetimes.

use super::*;

fn cast(guid: ObjectGuid) -> SpellCastState {
    SpellCastState {
        spell_id: 133,
        target_guid: guid,
        target_data: Default::default(),
        cast_id: ObjectGuid::new(6, 17),
        cast_start_time: Instant::now(),
        cast_time_ms: 30_000,
        spell_visual: Default::default(),
        metadata: Default::default(),
    }
}

#[test]
fn active_cast_and_timestamps_mutate_once_under_active_and_detached_unit_owner() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    let stamp = Instant::now();
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        assert_eq!(session.remaining_active_spell_cast_ms_like_cpp(), Some(0));
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            session.mutate_cast_execution_like_cpp(|execution| {
                calls.set(calls.get() + 1);
                assert!(matches!(
                    manager.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                ));
                execution.active = Some(cast(guid));
                execution.last_cast_time = Some(stamp);
                execution.last_cast_time_per_spell.insert(133, stamp);
            }),
            Some(())
        );
        assert_eq!(calls.get(), 1);
        assert!(manager.try_lock().is_ok());
        assert_eq!(
            session
                .active_spell_cast_snapshot_like_cpp()
                .unwrap()
                .spell_id,
            133
        );
        assert_eq!(session.last_spell_cast_time_like_cpp(), Some(Some(stamp)));
        assert_eq!(
            session.spell_last_cast_time_like_cpp(133),
            Some(Some(stamp))
        );
        assert!(
            session.active_spell_cast.is_none(),
            "do not mirror the canonical owner"
        );
        assert!(session.last_spell_cast_time.is_none());
        assert!(session.last_spell_cast_time_per_spell.is_empty());
        assert!(session.interrupt_non_melee_spell_cast_for_loot_like_cpp());
        assert!(!session.interrupt_non_melee_spell_cast_for_loot_like_cpp());
        assert_eq!(session.last_spell_cast_time_like_cpp(), Some(Some(stamp)));
        assert_eq!(
            session.spell_last_cast_time_like_cpp(133),
            Some(Some(stamp))
        );
    }
}

#[tokio::test]
async fn ready_canonical_cast_is_taken_once_before_execution_and_failure_publication() {
    let (mut session, _, send_rx) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let mut ready = cast(guid);
    ready.cast_time_ms = 0;
    assert!(session.set_active_spell_cast_like_cpp(Some(ready)));
    assert_eq!(session.last_spell_cast_time_like_cpp(), Some(None));
    session.tick_active_spell_cast().await;
    assert!(session.active_spell_cast_snapshot_like_cpp().is_none());
    assert!(session.last_spell_cast_time_like_cpp().flatten().is_some());
    // Missing SpellStore gives the existing deferred-execution failure response.
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::CastFailed]
    );
    let stamp = session.last_spell_cast_time_like_cpp();
    session.tick_active_spell_cast().await;
    assert!(send_rx.is_empty());
    assert_eq!(session.last_spell_cast_time_like_cpp(), stamp);
}

#[tokio::test]
async fn stale_or_missing_unit_owner_cannot_complete_cancel_or_replace_a_cast() {
    let (mut session, _, send_rx) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    let expected = cast(guid);
    replacement
        .unit_mut()
        .subsystems_mut()
        .spells
        .execution
        .active = Some(expected.clone());
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    assert_eq!(
        session.mutate_cast_execution_like_cpp(|_| panic!("stale owner")),
        None::<()>
    );
    assert!(!session.set_active_spell_cast_like_cpp(None));
    assert!(!session.interrupt_non_melee_spell_cast_for_loot_like_cpp());
    assert_eq!(session.remaining_active_spell_cast_ms_like_cpp(), None);
    assert_eq!(session.last_spell_cast_time_like_cpp(), None);
    assert_eq!(session.spell_last_cast_time_like_cpp(133), None);
    session.tick_active_spell_cast().await;
    assert!(send_rx.is_empty());
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, |player| {
                player.unit().subsystems().spells.execution.active.clone()
            }),
        Some(Some(expected))
    );
    session.canonical_map_manager = None;
    assert_eq!(
        session.mutate_cast_execution_like_cpp(|_| panic!("missing owner")),
        None::<()>
    );
}
