//! Immediate bones visibility and its separate durable completion fence.

use super::*;
use wow_packet::packets::update::UpdateObject;

fn install_peer(
    session: &mut WorldSession,
    counter: i64,
) -> (WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid) {
    let registry = Arc::new(wow_world_core::player_directory::PlayerRegistry::new());
    session.set_player_registry(Arc::clone(&registry));
    session.core.state = crate::session::SessionState::LoggedIn;
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().world_mut().set_name("BonesOwner")
        })
        .unwrap();
    session.register_in_player_registry();
    let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    let (mut peer, output) = make_session();
    let guid = ObjectGuid::create_player(1, counter);
    peer.set_player_guid(Some(guid));
    peer.set_canonical_map_manager(Arc::clone(&manager));
    peer.set_player_registry(registry);
    peer.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    let handle = {
        let mut manager = manager.lock().unwrap();
        let mut player = Box::new(wow_entities::Player::new(Some(1), false));
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_name("BonesPeer");
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(100);
        let handle = manager.install_detached_player_like_cpp(player).unwrap();
        manager
            .attach_player_like_cpp(
                handle,
                wow_map::MapKey::new(571, 0),
                Position::new(11.0, 2.0, 3.0, 0.0),
            )
            .unwrap();
        handle
    };
    peer.core.player_handle_like_cpp = Some(handle);
    peer.core.player_identity_bootstrap_like_cpp = None;
    peer.core.state = crate::session::SessionState::LoggedIn;
    peer.register_in_player_registry();
    (peer, output, guid)
}

#[tokio::test]
async fn reclaim_bones_destroy_then_birth_precedes_receipt_actor_values_and_rediscovery() {
    let (mut session, output, _realm, port) =
        released_ghost_without_reclaim_delay_like_cpp(9_320).await;
    let owner = session.player_guid().unwrap();
    let corpse_guid = current_corpse_guid_like_cpp(&session, owner).unwrap();
    session.set_player_position_like_cpp(Position::new(10.0, 2.0, 3.0, 0.0));
    let (mut peer, peer_output, peer_guid) = install_peer(&mut session, 9_321);
    session
        .core
        .client_visible_guids_like_cpp
        .insert(corpse_guid);
    session.core.client_visible_guids_like_cpp.insert(peer_guid);
    peer.core.client_visible_guids_like_cpp.insert(corpse_guid);
    peer.core.client_visible_guids_like_cpp.insert(owner);
    let own_known = session.core.client_visible_guids_like_cpp.clone();
    let peer_known = peer.core.client_visible_guids_like_cpp.clone();
    let handle = session.core.player_handle_like_cpp.unwrap();
    let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    let mut expected_data = {
        let manager = manager.lock().unwrap();
        wow_world_core::entity_update_bridge::corpse_create_data_from_entity_like_cpp(
            manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .get_typed_corpse(corpse_guid)
                .unwrap(),
        )
    };
    expected_data.scale = 0.0;
    expected_data.flags |= wow_map::CORPSE_FLAG_BONES_LIKE_CPP;
    let expected_destroy = UpdateObject::out_of_range_objects(vec![corpse_guid], 571).to_bytes();
    let expected_birth = UpdateObject::create_world_objects(
        vec![UpdateObject::create_corpse_birth_block(
            expected_data.clone(),
        )],
        571,
    )
    .to_bytes();
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    port.hold_delete();
    let mut reclaim = Box::pin(handle_reclaim_corpse(
        &mut session,
        reclaim_corpse_packet(corpse_guid),
    ));
    tokio::select! {
        result = tokio::time::timeout(std::time::Duration::from_secs(5), port.wait_for_delete()) => assert!(result.is_ok()),
        _ = &mut reclaim => panic!("delete must remain gated after native publication"),
    }
    let before_receipt = output.try_iter().collect::<Vec<_>>();
    let destroy = before_receipt
        .iter()
        .position(|bytes| bytes == &expected_destroy)
        .expect("old corpse out of range");
    assert_eq!(before_receipt.get(destroy + 1), Some(&expected_birth));
    assert_eq!(
        peer_output.try_iter().collect::<Vec<_>>(),
        vec![expected_destroy.clone(), expected_birth.clone()]
    );
    assert!(own_known.contains(&corpse_guid) && own_known.contains(&peer_guid));
    assert!(peer_known.contains(&corpse_guid) && peer_known.contains(&owner));
    assert_eq!(
        save_port.character_save_count(),
        0,
        "birth is not a persistence receipt"
    );
    {
        let mut manager = manager.lock().unwrap();
        assert_eq!(
            manager.with_player_like_cpp(handle, |player| player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .unwrap()
                .phase),
            Some(wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight)
        );
        let map = manager.find_map_mut(571, 0).unwrap().map_mut();
        let bones = map.get_typed_corpse(corpse_guid).unwrap();
        assert_eq!(bones.corpse_type(), wow_entities::CorpseType::Bones);
        assert_eq!(bones.world().object().scale(), 0.0);
        assert!(!bones.world().object().is_new_object());
        assert!(
            map.take_object_visibility_destroy_for_guid_like_cpp(corpse_guid)
                .is_none(),
            "no deferred old destroy can erase the new discovery"
        );
    }
    port.release_delete();
    reclaim.await;
    assert_eq!(save_port.character_save_count(), 1);
    assert!(own_known.contains(&corpse_guid) && peer_known.contains(&corpse_guid));
    let _ = manager.lock().unwrap().update(1);
    assert!(
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            )
            > 0
    );
    assert!(
        output
            .try_iter()
            .any(|bytes| u16::from_le_bytes([bytes[0], bytes[1]])
                == ServerOpcodes::UpdateObject as u16),
        "actor VALUES follow native birth at Map tail"
    );
    peer.process_represented_session_commands_like_cpp().await;
    let _ = peer_output.try_iter().count();
    peer.core.client_visible_guids_like_cpp.remove(&corpse_guid);
    let creatures = peer.creature_spawn_catalogs_for_test_like_cpp();
    wow_world_entities::force_update_visibility_with_catalogs_like_cpp(&mut peer, &creatures).await;
    let rediscovery = UpdateObject::create_world_objects(
        vec![UpdateObject::create_corpse_block(expected_data)],
        571,
    )
    .to_bytes();
    assert_eq!(
        peer_output.try_iter().collect::<Vec<_>>(),
        vec![rediscovery]
    );
    let generators = session.id_generators_for_test_like_cpp();
    assert!(
        wow_world_entities::spawn_corpse_bones_like_cpp(
            &mut session,
            generators.item.as_ref(),
            false
        )
        .await
    );
    assert_eq!(
        port.delete_requests(),
        vec![9_320],
        "conversion is consumed once"
    );
}

#[tokio::test]
async fn bones_publication_backpressure_retains_continuation_but_still_attempts_delete() {
    let (mut session, output, _realm, port) =
        released_ghost_without_reclaim_delay_like_cpp(9_322).await;
    let owner = session.player_guid().unwrap();
    let guid = current_corpse_guid_like_cpp(&session, owner).unwrap();
    session.set_player_position_like_cpp(Position::new(10.0, 2.0, 3.0, 0.0));
    session.core.client_visible_guids_like_cpp.remove(&guid);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            assert!(player.begin_resurrection_operation(
                wow_entities::PlayerResurrectionOperation::ReclaimCorpse {
                    restore_percent: 0.5
                },
                wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight
            ));
        })
        .unwrap();
    while session.core.send_tx().try_send(vec![0xFF]).is_ok() {}
    let generators = session.id_generators_for_test_like_cpp();
    assert!(
        !wow_world_entities::spawn_corpse_bones_like_cpp(
            &mut session,
            generators.item.as_ref(),
            true
        )
        .await
    );
    assert_eq!(port.delete_requests(), vec![9_322]);
    assert!(!session.core.client_visible_guids_like_cpp.contains(&guid));
    assert!(output.try_iter().all(|bytes| bytes == [0xFF]));
    session
        .core
        .with_owned_player_like_cpp(|player| {
            assert_eq!(
                player
                    .resurrection_state_like_cpp()
                    .delayed_after_teleport
                    .unwrap()
                    .phase,
                wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight
            );
        })
        .unwrap();
}

#[tokio::test]
async fn far_owner_does_not_suppress_nearby_bones_birth_or_clear_unrelated_discovery() {
    let (mut session, output, _realm, port) =
        released_ghost_without_reclaim_delay_like_cpp(9_324).await;
    let owner = session.player_guid().unwrap();
    let guid = current_corpse_guid_like_cpp(&session, owner).unwrap();
    let (peer, peer_output, _) = install_peer(&mut session, 9_325);
    peer.core.client_visible_guids_like_cpp.insert(guid);
    let unrelated = ObjectGuid::create_player(1, 9_326);
    peer.core.client_visible_guids_like_cpp.insert(unrelated);
    session.set_player_position_like_cpp(Position::new(5_000.0, 5_000.0, 3.0, 0.0));
    session.core.client_visible_guids_like_cpp.remove(&guid);
    let generators = session.id_generators_for_test_like_cpp();
    assert!(
        wow_world_entities::spawn_corpse_bones_like_cpp(
            &mut session,
            generators.item.as_ref(),
            false
        )
        .await
    );
    assert!(output.is_empty(), "far owner need not discover bones");
    let packets = peer_output.try_iter().collect::<Vec<_>>();
    assert_eq!(packets.len(), 2);
    assert_eq!(
        packets[0],
        UpdateObject::out_of_range_objects(vec![guid], 571).to_bytes()
    );
    assert!(peer.core.client_visible_guids_like_cpp.contains(&guid));
    assert!(peer.core.client_visible_guids_like_cpp.contains(&unrelated));
    assert_eq!(port.delete_requests(), vec![9_324]);
}

#[tokio::test]
async fn disabled_bones_still_publish_the_old_out_of_range_before_delete_completion() {
    let (mut session, output, _realm, port) =
        released_ghost_without_reclaim_delay_like_cpp(9_327).await;
    let owner = session.player_guid().unwrap();
    let guid = current_corpse_guid_like_cpp(&session, owner).unwrap();
    session.set_player_position_like_cpp(Position::new(10.0, 2.0, 3.0, 0.0));
    session.core.client_visible_guids_like_cpp.insert(guid);
    session.set_death_policy_like_cpp(wow_world_core::session::DeathPolicyLikeCpp {
        bones_world: false,
        ..Default::default()
    });
    let generators = session.id_generators_for_test_like_cpp();
    assert!(
        wow_world_entities::spawn_corpse_bones_like_cpp(
            &mut session,
            generators.item.as_ref(),
            false
        )
        .await
    );
    assert_eq!(
        output.try_iter().collect::<Vec<_>>(),
        vec![UpdateObject::out_of_range_objects(vec![guid], 571).to_bytes()]
    );
    assert!(!session.core.client_visible_guids_like_cpp.contains(&guid));
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .unwrap()
        .lock()
        .unwrap();
    assert!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_corpse(guid)
            .is_none()
    );
    assert_eq!(port.delete_requests(), vec![9_327]);
}

#[tokio::test]
async fn committed_bones_delete_cannot_complete_an_earlier_residence() {
    let (mut session, output, _realm, port) =
        released_ghost_without_reclaim_delay_like_cpp(9_328).await;
    let owner = session.player_guid().unwrap();
    let guid = current_corpse_guid_like_cpp(&session, owner).unwrap();
    let position = Position::new(10.0, 2.0, 3.0, 0.0);
    session.set_player_position_like_cpp(position);
    session.core.client_visible_guids_like_cpp.insert(guid);
    let handle = session.core.player_handle_like_cpp.unwrap();
    let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    port.hold_delete();
    let mut reclaim = Box::pin(handle_reclaim_corpse(
        &mut session,
        reclaim_corpse_packet(guid),
    ));
    tokio::select! {
        result = tokio::time::timeout(std::time::Duration::from_secs(5), port.wait_for_delete()) => assert!(result.is_ok()),
        _ = &mut reclaim => panic!("delete must remain gated"),
    }
    assert!(
        output
            .try_iter()
            .any(|bytes| bytes == UpdateObject::out_of_range_objects(vec![guid], 571).to_bytes())
    );
    {
        let mut manager = manager.lock().unwrap();
        manager.detach_player_like_cpp(handle).unwrap();
        manager
            .attach_player_like_cpp(handle, wow_map::MapKey::new(571, 0), position)
            .unwrap();
    }
    port.release_delete();
    reclaim.await;
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(session.core.player_handle_like_cpp, Some(handle));
    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    session
        .core
        .with_owned_player_like_cpp(|player| {
            assert_eq!(
                player
                    .resurrection_state_like_cpp()
                    .delayed_after_teleport
                    .unwrap()
                    .phase,
                wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight
            );
        })
        .unwrap();
}
