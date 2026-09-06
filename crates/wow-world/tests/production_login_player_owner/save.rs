//! Production wow-world (without cfg(test)): admitted save, real map access during
//! delayed I/O, late incarnation completion and saturated delivery. The port is
//! controlled, not MariaDB; partial login hydration is not full login/relogin QA.
use super::*;
use std::sync::Mutex;

pub(super) async fn assert_terminal_source_save(session: &mut WorldSession, port: &LoginPort) {
    let probe = Arc::new(SaveProbe {
        requests: Mutex::new(vec![]),
        released: AtomicBool::new(true),
        outcome: PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    });
    *port.save_probe.lock().unwrap() = Some(Arc::clone(&probe));
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    session
        .save_disconnect_player_to_db_with_generator_like_cpp(&generator)
        .await;
    let requests = probe.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    // The login fixture installs its canonical Player at the origin, not the rejected
    // destination (7,8,9) or homebind (100,200,300).
    assert_eq!(requests[0].character.position.map_id, 0);
    assert_eq!(requests[0].character.position.x, 0.0);
    assert_eq!(requests[0].character.position.y, 0.0);
    assert_eq!(requests[0].character.position.z, 0.0);
}

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
        // Change only the canonical owner after login hydrated Session's level.
        p.unit_mut().set_level(73);
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
    assert_eq!(request.character.level, 73);
    assert!(matches!(
        request.spells,
        Some(PlayerSpellSaveGroupLikeCpp::Complete { .. })
    ));
    let replacement = {
        let mut owner = manager
            .try_lock()
            .expect("pending DB must not retain the owner");
        assert!(
            !owner.destroy_map(0, 0),
            "a pending save must not lose its still-active Player through map destruction"
        );
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
async fn production_disconnect_finishes_retained_worldport_before_save_with_full_output() {
    let (mut session, port, output, receiver) = hydrate(true, true, true).await;
    let guid = ObjectGuid::create_player(1, 42);
    {
        let mut manager = port.manager.lock().unwrap();
        let player = manager
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(guid)
            .unwrap();
        let position = player.unit().world().position();
        player.teleport_state_mut_like_cpp().post_add =
            Some(wow_entities::PlayerWorldportPostAddLikeCpp {
                map_id: 0,
                position,
                phase: wow_entities::PlayerWorldportPostAddPhaseLikeCpp::ZoneApplied,
            });
        player.gameplay_state_mut().using_pvp_item_levels = true;
        player.unit_mut().set_max_health(1000);
        player.unit_mut().set_health(500);
        player
            .resurrection_state_mut_like_cpp()
            .delayed_after_teleport = Some(wow_entities::PlayerResurrectionRequestLikeCpp {
            resurrecter: guid,
            map_id: 0,
            position,
            health: 123,
            mana: 0,
            aura: 0,
        });
        // A represented equipped item makes the normal scaling path publish stats.
        // Finalization must complete native work even when that output is saturated.
        let item_guid = ObjectGuid::create_item(1, 99);
        let mut item = wow_entities::Item::new(0);
        item.object_mut().create(item_guid);
        item.object_mut().set_entry(6948);
        player
            .inventory_runtime_mut_like_cpp()
            .item_objects_mut()
            .insert(item_guid, item);
    }
    let probe = Arc::new(SaveProbe {
        requests: Mutex::new(vec![]),
        released: AtomicBool::new(true),
        outcome: PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    });
    *port.save_probe.lock().unwrap() = Some(Arc::clone(&probe));
    for _ in 0..8 {
        output.try_send(vec![0]).unwrap();
    }
    assert!(output.is_full());
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    session
        .save_disconnect_player_to_db_with_generator_like_cpp(&generator)
        .await;
    assert_eq!(
        receiver.len(),
        8,
        "native finalization does not publish packets"
    );
    let requests = probe.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].character.health, 123);
    let manager = port.manager.lock().unwrap();
    let player = manager
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(guid)
        .unwrap();
    assert!(player.teleport_state_like_cpp().post_add.is_none());
    assert!(
        player
            .resurrection_state_like_cpp()
            .delayed_after_teleport
            .is_none()
    );
    assert!(!player.gameplay_state().using_pvp_item_levels);
}

#[tokio::test]
async fn production_disconnect_completes_pending_far_transfer_before_save() {
    pending_far_disconnect(DisconnectDestination::Requested).await;
}

#[tokio::test]
async fn production_disconnect_recovers_pending_far_transfer_to_homebind_without_packets() {
    pending_far_disconnect(DisconnectDestination::Homebind).await;
}

#[tokio::test]
async fn production_disconnect_rejected_far_and_homebind_preserves_terminal_source_save() {
    pending_far_disconnect(DisconnectDestination::Rejected).await;
}

#[derive(Clone, Copy, PartialEq)]
enum DisconnectDestination {
    Requested,
    Homebind,
    Rejected,
}

async fn pending_far_disconnect(outcome: DisconnectDestination) {
    let (mut session, port, output, receiver) = hydrate(true, true, true).await;
    let guid = ObjectGuid::create_player(1, 42);
    let destination = Position::new(7.0, 8.0, 9.0, 0.5);
    let home = Position::new(100.0, 200.0, 300.0, 0.0);
    session.set_map_store(Arc::new(MapStore::from_entries([0, 1].map(|id| {
        MapEntry {
            id,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        }
    }))));
    {
        let mut manager = port.manager.lock().unwrap();
        let player = manager
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(guid)
            .unwrap();
        player.unit_mut().set_max_health(1000);
        player.unit_mut().set_health(500);
        player
            .gameplay_state_mut()
            .homebind
            .as_mut()
            .unwrap()
            .position = home;
        player
            .resurrection_state_mut_like_cpp()
            .delayed_after_teleport = Some(wow_entities::PlayerResurrectionRequestLikeCpp {
            resurrecter: guid,
            map_id: 1,
            position: destination,
            health: 123,
            mana: 0,
            aura: 0,
        });
    }
    session.set_state(SessionState::LoggedIn);
    session.teleport_to(1, destination).await;
    if outcome != DisconnectDestination::Requested {
        session.set_map_store(Arc::new(MapStore::from_entries(
            (outcome == DisconnectDestination::Homebind).then_some(MapEntry {
                id: 0,
                instance_type: 0,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            }),
        )));
    }
    while receiver.try_recv().is_ok() {}
    for _ in 0..8 {
        output.try_send(vec![0]).unwrap();
    }
    assert!(output.is_full());
    session.kick("controlled disconnect before suspend/worldport ACK");
    let probe = Arc::new(SaveProbe {
        requests: Mutex::new(vec![]),
        released: AtomicBool::new(true),
        outcome: PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    });
    *port.save_probe.lock().unwrap() = Some(Arc::clone(&probe));
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    session
        .save_disconnect_player_to_db_with_generator_like_cpp(&generator)
        .await;
    assert_eq!(
        receiver.len(),
        8,
        "disconnect admission/recovery cannot publish into full output"
    );
    let requests = probe.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    let manager = port.manager.lock().unwrap();
    if outcome == DisconnectDestination::Rejected {
        assert_eq!(requests[0].character.position.map_id, 0);
        assert_eq!(requests[0].character.position.x, 0.0);
        assert_eq!(requests[0].character.position.y, 0.0);
        assert_eq!(requests[0].character.position.z, 0.0);
        assert_eq!(requests[0].character.health, 500);
        assert!(manager.find_map(1, 0).is_none());
        assert_eq!(session.state(), SessionState::Disconnecting);
        return;
    }
    let (expected_map, expected_position) = if outcome == DisconnectDestination::Requested {
        (1, destination)
    } else {
        (0, home)
    };
    let player = manager
        .find_map(expected_map, 0)
        .and_then(|map| map.map().get_typed_player(guid))
        .expect(
            "disconnect must finish native entry before save, not only save proposed coordinates",
        );
    assert_eq!(player.unit().world().position(), expected_position);
    assert!(!player.teleport_state_like_cpp().far_pending);
    assert!(player.teleport_state_like_cpp().far_destination.is_none());
    assert!(player.teleport_state_like_cpp().post_add.is_none());
    assert_eq!(requests[0].character.health, 123);
    assert_eq!(requests[0].character.position.map_id, expected_map as u16);
    assert_eq!(requests[0].character.position.x, expected_position.x);
    assert_eq!(session.state(), SessionState::Disconnecting);
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
