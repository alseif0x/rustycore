//! Production wow-world (without cfg(test)): admitted save, real map access during
//! delayed I/O, late incarnation completion and saturated delivery. The port is
//! controlled, not MariaDB; partial login hydration is not full login/relogin QA.
use super::*;
use std::sync::Mutex;

pub(super) struct SaveProbe {
    requests: Mutex<Vec<PlayerCharacterSaveRequestLikeCpp>>,
    released: AtomicBool,
    outcome: PersistenceOutcomeLikeCpp,
}

pub(super) fn save<'a>(
    port: &'a LoginPort,
    request: PlayerCharacterSaveRequestLikeCpp,
) -> PersistenceFutureLikeCpp<'a, PlayerCharacterSaveResultLikeCpp> {
    let probe = port
        .save_probe
        .lock()
        .unwrap()
        .clone()
        .expect("unexpected save");
    let committed = request.committed_groups_like_cpp();
    probe.requests.lock().unwrap().push(request);
    Box::pin(async move {
        // Tests poll explicitly before and after release; no timer/sleep ordering.
        std::future::poll_fn(|_| {
            if probe.released.load(Ordering::SeqCst) {
                std::task::Poll::Ready(())
            } else {
                std::task::Poll::Pending
            }
        })
        .await;
        PlayerCharacterSaveResultLikeCpp {
            outcome: probe.outcome.clone(),
            committed,
        }
    })
}

fn spell(id: i32) -> wow_entities::PlayerKnownSpellRecord {
    wow_entities::PlayerKnownSpellRecord {
        spell_id: id,
        state: wow_entities::PlayerSpellLoadState::New,
        active: true,
        disabled: false,
        favorite: false,
        dependent: false,
    }
}

async fn exercise(replace: bool, cancel: bool, outcome: PersistenceOutcomeLikeCpp) {
    let (mut session, port, output, receiver) = hydrate(true, true, true).await;
    let manager = Arc::clone(&port.manager);
    let guid = ObjectGuid::create_player(1, 42);
    {
        let mut owner = manager.lock().unwrap();
        let p = owner
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(guid)
            .unwrap();
        let spells = &mut p.gameplay_state_mut().spells;
        spells.rows_loaded = true;
        spells.rows_complete = true;
        spells.rows.insert(10, spell(10));
        owner.create_world_map(1, 0);
    }
    let probe = Arc::new(SaveProbe {
        requests: Mutex::new(vec![]),
        released: AtomicBool::new(false),
        outcome: outcome.clone(),
    });
    *port.save_probe.lock().unwrap() = Some(Arc::clone(&probe));
    for _ in 0..8 {
        output.try_send(vec![0]).unwrap();
    }
    assert!(output.is_full());
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let mut future =
        Box::pin(session.save_disconnect_player_to_db_with_generator_like_cpp(&generator));
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(future.as_mut().poll(cx).is_pending()))
            .await
    );
    let request = probe.requests.lock().unwrap()[0].clone();
    assert_eq!(request.player_guid, 42);
    assert!(matches!(
        request.spells,
        Some(PlayerSpellSaveGroupLikeCpp::Complete { .. })
    ));
    let replacement = {
        let mut owner = manager
            .try_lock()
            .expect("pending DB must not retain the owner");
        owner.update(10); // Both real map updates can progress while the save is pending.
        if replace {
            let mut p = Box::new(wow_entities::Player::new(Some(1), false));
            p.unit_mut().world_mut().object_mut().create(guid);
            p.gameplay_state_mut().spells.rows_loaded = true;
            p.gameplay_state_mut().spells.rows_complete = true;
            p.gameplay_state_mut().spells.rows.insert(10, spell(10));
            Some(owner.install_detached_player_like_cpp(p).unwrap())
        } else {
            owner
                .find_map_mut(0, 0)
                .unwrap()
                .map_mut()
                .get_typed_player_mut(guid)
                .unwrap()
                .gameplay_state_mut()
                .spells
                .rows
                .insert(20, spell(20));
            None
        }
    };
    if !cancel {
        probe.released.store(true, Ordering::SeqCst);
        future.await;
    } else {
        drop(future);
    }
    let owner = manager
        .try_lock()
        .expect("completion releases its owner before publication");
    let inspect = |p: &wow_entities::Player| {
        let spells = &p.gameplay_state().spells;
        let clean =
            !replace && !cancel && matches!(outcome, PersistenceOutcomeLikeCpp::Applied { .. });
        assert_eq!(
            spells.rows[&10].state,
            if clean {
                wow_entities::PlayerSpellLoadState::Unchanged
            } else {
                wow_entities::PlayerSpellLoadState::New
            }
        );
        if !replace {
            assert_eq!(
                spells.rows[&20].state,
                wow_entities::PlayerSpellLoadState::New
            );
        }
    };
    if let Some(handle) = replacement {
        owner.with_player_like_cpp(handle, inspect).unwrap();
    } else {
        inspect(
            owner
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_typed_player(guid)
                .unwrap(),
        );
    }
    drop(owner);
    assert_eq!(
        receiver.len(),
        8,
        "save/ACK is not packet delivery under backpressure"
    );
    assert_eq!(probe.requests.lock().unwrap().len(), 1);
    if cancel || matches!(outcome, PersistenceOutcomeLikeCpp::Unknown { .. }) {
        session
            .save_disconnect_player_to_db_with_generator_like_cpp(&generator)
            .await;
        assert_eq!(
            probe.requests.lock().unwrap().len(),
            1,
            "unknown/cancelled transaction cannot be retried over an uncertain durable base"
        );
    }
}

#[tokio::test]
async fn production_save_retains_later_spell_and_allows_two_maps_with_full_output() {
    exercise(false, false, PersistenceOutcomeLikeCpp::Applied { rows: 1 }).await;
}
#[tokio::test]
async fn production_save_old_completion_does_not_clean_replacement_incarnation() {
    exercise(true, false, PersistenceOutcomeLikeCpp::Applied { rows: 1 }).await;
}
#[tokio::test]
async fn production_save_rollback_unknown_and_cancellation_keep_native_dirty_state() {
    exercise(
        false,
        false,
        PersistenceOutcomeLikeCpp::Failed {
            reason: "rollback".into(),
        },
    )
    .await;
    exercise(
        false,
        false,
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "lost reply".into(),
        },
    )
    .await;
    exercise(false, true, PersistenceOutcomeLikeCpp::Applied { rows: 1 }).await;
}
