// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! corpse capability handler tests.

use super::*;
use crate::session::WorldSession;

mod release;
#[path = "corpse/selected_recovery.rs"]
mod selected_recovery;

async fn handle_resurrect_response(session: &mut WorldSession, pkt: WorldPacket) {
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    wow_world_entities::handle_resurrect_response(
        session,
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creature_spawns,
        },
        pkt,
    )
    .await;
}

async fn continue_resurrection_after_valid_teleport(session: &mut WorldSession) -> bool {
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    assert!(wow_world_entities::mark_player_resurrection_teleport_completed(session));
    wow_world_entities::continue_player_resurrection_after_teleport(
        session,
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creature_spawns,
        },
    )
    .await
}

/// CMSG_REPOP_REQUEST with the test session's catalogs (#43).
/// CMSG_RECLAIM_CORPSE with the test session's catalogs (#43 slice 2).
async fn handle_reclaim_corpse(session: &mut WorldSession, pkt: WorldPacket) {
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();
    wow_world_entities::handle_reclaim_corpse(
        session,
        wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creature_spawns,
            graveyards: &graveyards,
        },
        pkt,
    )
    .await;
}

async fn handle_repop_request(session: &mut WorldSession, pkt: WorldPacket) {
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();
    wow_world_entities::handle_repop_request(
        session,
        wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creature_spawns,
            graveyards: &graveyards,
        },
        pkt,
    )
    .await;
}

#[tokio::test]
async fn request_cemetery_list_without_links_sends_no_response_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    crate::session::hub_mut(&mut session).set_player_zone_area_like_cpp(1234, 5678);
    session.set_graveyard_store(Arc::new(GraveyardStore::default()));

    session
        .handle_request_cemetery_list(request_cemetery_list_packet(None))
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns without SMSG_REQUEST_CEMETERY_LIST_RESPONSE when no graveyards match"
    );
}

#[tokio::test]
async fn request_cemetery_list_ignores_payload_and_sends_zone_ids_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    crate::session::hub_mut(&mut session).set_player_zone_area_like_cpp(4321, 8765);
    let (graveyards, condition_store) = graveyard_store_with_links(4321, [11, 12], []);
    session.set_graveyard_store(graveyards);
    session.set_condition_store(condition_store);

    session
        .handle_request_cemetery_list(request_cemetery_list_packet(Some(1)))
        .await;

    let (is_gossip_triggered, cemetery_ids) =
        read_cemetery_list_response(&send_rx.try_recv().unwrap());
    assert!(
        !is_gossip_triggered,
        "C++ request has no gossip bool payload; response always sets false here"
    );
    assert_eq!(cemetery_ids, vec![11, 12]);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn request_cemetery_list_filters_conditions_and_caps_at_sixteen_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    crate::session::hub_mut(&mut session).set_player_zone_area_like_cpp(2222, 3333);
    let conditions = [
        graveyard_team_condition(2222, 100, wow_data::TEAM_HORDE_LIKE_CPP),
        graveyard_team_condition(2222, 101, wow_data::TEAM_ALLIANCE_LIKE_CPP),
    ];
    let (graveyards, condition_store) = graveyard_store_with_links(2222, 100..120, conditions);
    session.set_graveyard_store(graveyards);
    session.set_condition_store(condition_store);

    session
        .handle_request_cemetery_list(request_cemetery_list_packet(None))
        .await;

    let (_, cemetery_ids) = read_cemetery_list_response(&send_rx.try_recv().unwrap());
    assert_eq!(
        cemetery_ids,
        (101..117).collect::<Vec<_>>(),
        "C++ skips failed condition rows and continues until 16 accepted IDs"
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn resurrect_response_accepts_matching_request_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_map_store(crate::teleport_test_fixtures::world_maps([571]));
    let resurrecter = ObjectGuid::create_player(1, 77);
    let target_position = Position::new(11.0, 22.0, 33.0, 1.5);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.0));
    session.set_player_health_like_cpp(0, 1_000);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .set_power_index(wow_constants::PowerType::Mana, Some(0));
            player
                .unit_mut()
                .set_power_index(wow_constants::PowerType::Rage, Some(1));
            player
                .unit_mut()
                .set_power_index(wow_constants::PowerType::Energy, Some(2));
            player
                .unit_mut()
                .set_power_index(wow_constants::PowerType::Focus, Some(3));
            player
                .unit_mut()
                .set_max_power(wow_constants::PowerType::Mana, 2_000);
            player
                .unit_mut()
                .set_max_power(wow_constants::PowerType::Rage, 100);
            player
                .unit_mut()
                .set_power(wow_constants::PowerType::Rage, 25);
            player
                .unit_mut()
                .set_max_power(wow_constants::PowerType::Energy, 100);
            player
                .unit_mut()
                .set_power(wow_constants::PowerType::Energy, 20);
            player
                .unit_mut()
                .set_max_power(wow_constants::PowerType::Focus, 80);
            player
                .unit_mut()
                .set_power(wow_constants::PowerType::Focus, 13);
        })
        .unwrap();
    session.set_represented_resurrection_request_like_cpp(
        wow_entities::PlayerResurrectionRequestLikeCpp {
            resurrecter,
            map_id: 571,
            position: target_position,
            health: 450,
            mana: 120,
            aura: 0,
        },
    );

    handle_resurrect_response(&mut session, resurrect_response_packet(resurrecter, 0)).await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(
        session
            .represented_resurrection_request_like_cpp()
            .is_none()
    );
    assert!(
        session
            .represented_delayed_resurrection_after_teleport_like_cpp()
            .is_some()
    );
    assert_eq!(
        std::iter::from_fn(|| send_rx.try_recv().ok())
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>(),
        vec![
            ServerOpcodes::CancelCombat as u16,
            ServerOpcodes::MoveTeleport as u16,
        ]
    );

    let action = wow_world_entities::handle_move_teleport_ack_like_cpp(
        &mut session,
        ObjectGuid::create_player(1, 42),
        0,
        0,
    );
    assert_eq!(
        action,
        crate::session::MoveTeleportAckActionLikeCpp::Accepted
    );

    assert!(continue_resurrection_after_valid_teleport(&mut session).await);

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 450);
    session
        .core
        .with_owned_player_like_cpp(|player| {
            assert_eq!(player.unit().get_power(wow_constants::PowerType::Mana), 120);
            assert_eq!(player.unit().get_power(wow_constants::PowerType::Rage), 0);
            assert_eq!(
                player.unit().get_power(wow_constants::PowerType::Energy),
                100
            );
            assert_eq!(player.unit().get_power(wow_constants::PowerType::Focus), 80);
        })
        .unwrap();
    assert_eq!(session.player_position_like_cpp(), Some(target_position));
    assert!(
        session
            .represented_delayed_resurrection_after_teleport_like_cpp()
            .is_none()
    );
}

#[tokio::test]
async fn resurrect_response_decline_clears_request_like_cpp() {
    let (mut session, send_rx) = make_session();
    let resurrecter = ObjectGuid::create_player(1, 78);
    session.set_player_health_like_cpp(0, 1_000);
    session.set_represented_resurrection_request_like_cpp(
        wow_entities::PlayerResurrectionRequestLikeCpp {
            resurrecter,
            map_id: 571,
            position: Position::new(11.0, 22.0, 33.0, 1.5),
            health: 450,
            mana: 120,
            aura: 0,
        },
    );

    handle_resurrect_response(&mut session, resurrect_response_packet(resurrecter, 1)).await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(
        session
            .represented_resurrection_request_like_cpp()
            .is_none()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn resurrect_response_ignores_mismatched_resurrecter_like_cpp() {
    let (mut session, send_rx) = make_session();
    let resurrecter = ObjectGuid::create_player(1, 79);
    session.set_player_health_like_cpp(0, 1_000);
    session.set_represented_resurrection_request_like_cpp(
        wow_entities::PlayerResurrectionRequestLikeCpp {
            resurrecter,
            map_id: 571,
            position: Position::new(11.0, 22.0, 33.0, 1.5),
            health: 450,
            mana: 120,
            aura: 0,
        },
    );

    handle_resurrect_response(
        &mut session,
        resurrect_response_packet(ObjectGuid::create_player(1, 80), 0),
    )
    .await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(
        session
            .represented_resurrection_request_like_cpp()
            .is_some()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn resurrect_response_ignores_alive_player_like_cpp() {
    let (mut session, send_rx) = make_session();
    let resurrecter = ObjectGuid::create_player(1, 81);
    session.set_player_health_like_cpp(777, 1_000);
    session.set_represented_resurrection_request_like_cpp(
        wow_entities::PlayerResurrectionRequestLikeCpp {
            resurrecter,
            map_id: 571,
            position: Position::new(11.0, 22.0, 33.0, 1.5),
            health: 450,
            mana: 120,
            aura: 0,
        },
    );

    handle_resurrect_response(&mut session, resurrect_response_packet(resurrecter, 0)).await;

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 777);
    assert!(
        session
            .represented_resurrection_request_like_cpp()
            .is_some()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn repop_request_dead_non_ghost_sets_ghost_and_repop_count_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let (realm_tx, realm_rx) = flume::bounded(16);
    session.install_realm_send_channel_for_test(realm_tx);
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    wow_world_entities::set_player_alive_like_cpp(&mut session, false);

    handle_repop_request(&mut session, repop_request_packet(true)).await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        1
    );
    // C++ `BuildPlayerRepop` always starts with SMSG_PRE_RESSURECT; without a
    // race faction the corpse cannot be created and the release stops there.
    // SMSG_PRE_RESSURECT is a realm-connection opcode.
    assert_eq!(
        sent_opcodes(&realm_rx).first().copied(),
        Some(ServerOpcodes::PreRessurect as u16)
    );
}

#[tokio::test]
async fn repop_request_alive_or_already_ghost_returns_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 43);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );

    wow_world_entities::set_player_alive_like_cpp(&mut session, true);
    handle_repop_request(&mut session, repop_request_packet(false)).await;
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());

    wow_world_entities::set_player_alive_like_cpp(&mut session, false);
    wow_world_entities::set_player_ghost_flag_like_cpp(&mut session, true);
    handle_repop_request(&mut session, repop_request_packet(false)).await;
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );
    assert!(crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
}

#[tokio::test]
async fn client_port_graveyard_dead_ghost_repops_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 4301);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    wow_world_entities::set_player_alive_like_cpp(&mut session, false);
    wow_world_entities::set_player_ghost_flag_like_cpp(&mut session, true);

    let handled = session
        .try_handle_client_port_graveyard_like_cpp(port_graveyard_packet())
        .await;

    assert!(handled);
    assert!(!session.player_is_alive_like_cpp());
    assert!(crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        1
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn client_port_graveyard_alive_or_not_ghost_returns_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 4302);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );

    wow_world_entities::set_player_alive_like_cpp(&mut session, true);
    assert!(
        session
            .try_handle_client_port_graveyard_like_cpp(port_graveyard_packet())
            .await
    );
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );
    assert!(session.player_is_alive_like_cpp());

    wow_world_entities::set_player_alive_like_cpp(&mut session, false);
    wow_world_entities::set_player_ghost_flag_like_cpp(&mut session, false);
    assert!(
        session
            .try_handle_client_port_graveyard_like_cpp(port_graveyard_packet())
            .await
    );
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());

    let mut non_empty = WorldPacket::new_empty();
    non_empty.write_uint8(1);
    non_empty.reset_read();
    assert!(
        !session
            .try_handle_client_port_graveyard_like_cpp(non_empty)
            .await
    );
}

#[tokio::test]
async fn reclaim_corpse_without_corpse_returns_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 44);
    let corpse_guid = ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 571, 0, 0, 99);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    wow_world_entities::set_player_alive_like_cpp(&mut session, false);
    wow_world_entities::set_player_ghost_flag_like_cpp(&mut session, true);
    let _ = crate::session::hub_mut(&mut session).sync_canonical_player_health_like_cpp(0, 100);

    handle_reclaim_corpse(&mut session, reclaim_corpse_packet(corpse_guid)).await;

    // C++ `HandleReclaimCorpse` returns when `GetCorpse()` is null.
    assert!(!session.player_is_alive_like_cpp());
    assert!(crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn reclaim_corpse_alive_or_not_ghost_returns_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 45);
    let corpse_guid = ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 571, 0, 0, 100);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );

    wow_world_entities::set_player_alive_like_cpp(&mut session, true);
    handle_reclaim_corpse(&mut session, reclaim_corpse_packet(corpse_guid)).await;
    assert!(session.player_is_alive_like_cpp());
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());

    wow_world_entities::set_player_alive_like_cpp(&mut session, false);
    handle_reclaim_corpse(&mut session, reclaim_corpse_packet(corpse_guid)).await;
    assert!(!session.player_is_alive_like_cpp());
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
}

#[tokio::test]
async fn instance_lock_response_decline_repops_and_clears_pending_bind_like_cpp() {
    let (mut session, _send_rx) = make_session();
    session
        .instances
        .set_represented_pending_bind_for_test_like_cpp(Some(
            crate::session::RepresentedPendingBind {
                map_id: 0,
                instance_id: 77,
                completed_mask: 0xA5,
                time_until_lock_ms: 60_000,
            },
        ));

    session
        .handle_instance_lock_response(WorldPacket::from_bytes(&[0x00]))
        .await;

    assert!(
        session
            .instances
            .represented_pending_bind_for_test_like_cpp()
            .is_none()
    );
    assert!(
        session
            .instances
            .represented_confirmed_pending_binds_for_test_like_cpp()
            .is_empty()
    );
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        1
    );
}

fn sent_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<u16> {
    std::iter::from_fn(|| send_rx.try_recv().ok())
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .collect()
}

#[test]
fn water_walking_publication_is_idempotent_and_uses_the_unit_counter() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 4_306);
    session.set_player_guid(Some(player_guid));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);

    for expected in 0..3 {
        assert_eq!(
            crate::session::hub_mut(&mut session).next_movement_counter_like_cpp(),
            Some(expected)
        );
    }
    assert!(crate::session::hub_mut(&mut session).set_player_water_walking(true));
    let water_walk = send_rx.try_recv().expect("water-walk status packet");
    assert_eq!(
        u16::from_le_bytes([water_walk[0], water_walk[1]]),
        ServerOpcodes::MoveSetWaterWalk as u16
    );
    assert_eq!(&water_walk[water_walk.len() - 4..], &3u32.to_le_bytes());
    assert!(
        crate::session::hub_ref(&session)
            .player_movement_flags_like_cpp()
            .contains(wow_constants::MovementFlag::WATER_WALK)
    );

    assert!(!crate::session::hub_mut(&mut session).set_player_water_walking(true));
    assert_eq!(
        crate::session::hub_ref(&session).movement_counter_like_cpp(),
        Some(4),
        "an idempotent call does not consume a movement sequence"
    );
    assert!(send_rx.try_recv().is_err());

    assert!(crate::session::hub_mut(&mut session).set_player_water_walking(false));
    let land_walk = send_rx.try_recv().expect("land-walk status packet");
    assert_eq!(
        u16::from_le_bytes([land_walk[0], land_walk[1]]),
        ServerOpcodes::MoveSetLandWalk as u16
    );
    assert_eq!(&land_walk[land_walk.len() - 4..], &4u32.to_le_bytes());
    assert!(
        !crate::session::hub_ref(&session)
            .player_movement_flags_like_cpp()
            .contains(wow_constants::MovementFlag::WATER_WALK)
    );
    assert_eq!(
        crate::session::hub_ref(&session).movement_counter_like_cpp(),
        Some(5)
    );
}

#[derive(Default)]
pub(super) struct RecordingCorpsePortLikeCpp {
    saves: std::sync::Mutex<Vec<wow_persistence::CorpseSaveRequestLikeCpp>>,
    deletes: std::sync::Mutex<Vec<u64>>,
    fail_next_save: std::sync::atomic::AtomicBool,
    fail_next_delete: std::sync::atomic::AtomicBool,
}

impl RecordingCorpsePortLikeCpp {
    pub(super) fn delete_requests(&self) -> Vec<u64> {
        self.deletes.lock().unwrap().clone()
    }

    fn fail_next_delete(&self) {
        self.fail_next_delete
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub(super) fn fail_next_save(&self) {
        self.fail_next_save
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

impl wow_persistence::MapCorpsePersistencePortLikeCpp for RecordingCorpsePortLikeCpp {
    fn load_map_corpses_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::MapCorpseLoadRequestLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<'a, wow_persistence::MapCorpseLoadOutcomeLikeCpp>
    {
        Box::pin(async {
            wow_persistence::MapCorpseLoadOutcomeLikeCpp::Failed {
                reason: "not used".into(),
            }
        })
    }

    fn save_corpse_like_cpp<'a>(
        &'a self,
        request: wow_persistence::CorpseSaveRequestLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp,
    > {
        self.saves.lock().unwrap().push(request);
        let fail = self
            .fail_next_save
            .swap(false, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async move {
            if fail {
                wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp::Failed {
                    reason: "controlled unconfirmed corpse save".into(),
                }
            } else {
                wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp::Committed
            }
        })
    }

    fn delete_corpse_like_cpp<'a>(
        &'a self,
        owner_guid: u64,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp,
    > {
        self.deletes.lock().unwrap().push(owner_guid);
        let fail = self
            .fail_next_delete
            .swap(false, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async move {
            if fail {
                wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp::Failed {
                    reason: "controlled unconfirmed corpse delete".into(),
                }
            } else {
                wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp::Committed
            }
        })
    }
}

pub(super) fn human_race_entry_with_faction(
    faction_id: i16,
) -> wow_data::character_progression::ChrRacesEntry {
    wow_data::character_progression::ChrRacesEntry {
        id: 1,
        client_prefix: String::new(),
        client_file_string: String::new(),
        name: String::new(),
        flags: 0,
        male_display_id: 0,
        female_display_id: 0,
        high_res_male_display_id: 0,
        high_res_female_display_id: 0,
        res_sickness_spell_id: 15007,
        splash_sound_id: 0,
        create_screen_file_data_id: 0,
        select_screen_file_data_id: 0,
        low_res_screen_file_data_id: 0,
        altered_form_start_visual_kit_id: [0; 3],
        altered_form_finish_visual_kit_id: [0; 3],
        heritage_armor_achievement_id: 0,
        starting_level: 1,
        ui_display_order: 0,
        playable_race_bit: 0,
        female_skeleton_file_data_id: 0,
        male_skeleton_file_data_id: 0,
        helmet_anim_scaling_race_id: 0,
        transmogrify_disabled_slot_mask: 0,
        faction_id,
        cinematic_sequence_id: 0,
        base_language: 0,
        creature_type: 0,
        alliance: 0,
        race_related: 0,
        unaltered_visual_race_id: 0,
        default_class_id: 0,
        neutral_race_id: 0,
    }
}

/// A dead (not yet released) human on map 571 with the Alliance default
/// graveyard (WorldSafeLocs 4) on the same map.
fn dead_player_session_with_graveyard_like_cpp(
    guid_counter: i64,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
    ObjectGuid,
    Arc<RecordingCorpsePortLikeCpp>,
) {
    dead_player_session_on_map_like_cpp(
        guid_counter,
        crate::teleport_test_fixtures::world_maps([571]),
    )
}

fn dead_player_session_on_map_like_cpp(
    guid_counter: i64,
    maps: Arc<wow_data::MapStore>,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
    ObjectGuid,
    Arc<RecordingCorpsePortLikeCpp>,
) {
    let (session, send_rx, realm_rx, player_guid, port) =
        dead_player_session_on_map_with_optional_port(guid_counter, maps, true);
    (
        session,
        send_rx,
        realm_rx,
        player_guid,
        port.expect("fixture requested corpse persistence"),
    )
}

fn dead_player_session_without_corpse_port(
    guid_counter: i64,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
    ObjectGuid,
) {
    let (session, send_rx, realm_rx, player_guid, _) =
        dead_player_session_on_map_with_optional_port(
            guid_counter,
            crate::teleport_test_fixtures::world_maps([571]),
            false,
        );
    (session, send_rx, realm_rx, player_guid)
}

/// Supplies the selected DB2 rows required by the production release casts.
fn install_release_cast_spell_fixture(session: &mut WorldSession) {
    let mut spells = wow_data::SpellStore::new();
    let mut misc_rows = Vec::new();
    for (spell_id, aura_types, base_points, misc_value, attributes_1) in [
        (
            wow_world_entities::SPELL_GHOST_LIKE_CPP,
            [
                wow_data::spell::aura_types::SPELL_AURA_GHOST,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
            ],
            [-20, 49, 49],
            [0, 0, 0],
            0x0000_0400,
        ),
        (
            wow_world_entities::SPELL_WISP_SPIRIT_AURA_LIKE_CPP,
            [
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
                wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
            ],
            [74, 74, -1],
            [0, 0, 12_861],
            0x1000_0400,
        ),
    ] {
        let effects = aura_types
            .into_iter()
            .zip(base_points)
            .zip(misc_value)
            .enumerate()
            .map(
                |(effect_index, ((effect_aura, effect_base_points), effect_misc_value_1))| {
                    wow_data::SpellEffectInfo {
                        effect_index: effect_index as u32,
                        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                        effect_aura,
                        effect_base_points,
                        effect_die_sides: 1,
                        effect_misc_value_1,
                        ..Default::default()
                    }
                },
            )
            .collect::<Vec<_>>();
        let mut attributes = [0; 15];
        attributes[0] = 0x2D80_0100;
        attributes[1] = attributes_1;
        attributes[3] = 0x0010_0000;
        misc_rows.push(wow_data::SpellMiscEntry {
            id: spell_id as u32,
            spell_id: spell_id as u32,
            attributes: attributes.map(|value| value as i32),
            duration_index: 21,
            ..Default::default()
        });
        spells.insert_spell_misc_attributes_like_cpp(
            spell_id,
            attributes.map(|value| value as u32),
        );
        spells.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: effects[0].effect_base_points,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(effects[0].effect_aura),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects,
            },
        );
    }
    session.set_spell_store(Arc::new(spells));
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries(misc_rows)));
    session
        .set_spell_aura_options_store(Arc::new(wow_data::SpellAuraOptionsStore::from_entries([])));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 21,
            duration: -1,
            duration_per_level: 0,
            max_duration: -1,
        },
    ])));
    session.set_creature_template_lifecycle_store_like_cpp(Arc::new(
        wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates([
            wow_data::CreatureTemplateLifecycleRecordLikeCpp {
                entry: 12_861,
                name: "Wisp Spirit".into(),
                scale: 1.0,
                models: vec![wow_data::CreatureTemplateLifecycleModelLikeCpp {
                    creature_display_id: 1_825,
                    display_scale: 1.0,
                    probability: 1.0,
                }],
                ..Default::default()
            },
        ]),
    ));
    session.set_creature_model_info_store(Arc::new(
        wow_data::CreatureModelInfoStoreLikeCpp::from_entries([
            wow_data::CreatureModelInfoLikeCpp {
                display_id: 1_825,
                bounding_radius: 0.5,
                combat_reach: 1.5,
                display_id_other_gender: 0,
                is_trigger: false,
            },
        ]),
    ));
    session.set_creature_aggro_config_like_cpp(crate::session::CreatureAggroConfigLikeCpp {
        spell_x_spell_visual_store: Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries(
            [wow_data::SpellXSpellVisualEntry {
                id: 241_720,
                difficulty_id: 0,
                spell_visual_id: 886,
                probability: 1.0,
                flags: 0,
                priority: 0,
                spell_icon_file_id: 0,
                active_icon_file_id: 0,
                viewer_unit_condition_id: 0,
                viewer_player_condition_id: 0,
                caster_unit_condition_id: 0,
                caster_player_condition_id: 0,
                spell_id: wow_world_entities::SPELL_GHOST_LIKE_CPP as u32,
            }],
        ))),
        ..Default::default()
    });
}

fn dead_player_session_on_map_with_optional_port(
    guid_counter: i64,
    maps: Arc<wow_data::MapStore>,
    install_port: bool,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
    ObjectGuid,
    Option<Arc<RecordingCorpsePortLikeCpp>>,
) {
    let (mut session, send_rx) = make_session();
    let (realm_tx, realm_rx) = flume::bounded(64);
    session.install_realm_send_channel_for_test(realm_tx);
    let map_id = maps.entries().next().expect("one map").id;
    session.set_map_store(Arc::clone(&maps));
    let player_guid = ObjectGuid::create_player(1, guid_counter);
    session.set_player_guid(Some(player_guid));
    crate::canonical_player_access::install_canonical_player_owner_for_test(
        &mut session,
        map_id,
        0,
    );
    install_release_cast_spell_fixture(&mut session);
    session.set_loaded_player_identity_like_cpp(map_id as u16, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.0));
    session.set_player_health_like_cpp(0, 1_000);
    wow_world_entities::set_player_alive_like_cpp(&mut session, false);
    session.set_chr_races_store(Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([
            human_race_entry_with_faction(1),
        ]),
    ));
    let (safe_locs, _) = wow_data::WorldSafeLocStore::from_rows_like_cpp(
        [wow_data::WorldSafeLocRow {
            id: wow_data::ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP,
            map_id,
            x: 50.0,
            y: 60.0,
            z: 70.0,
            facing_degrees: 0.0,
        }],
        &maps,
    );
    session.set_world_safe_loc_store_like_cpp(Arc::new(safe_locs));
    let port = install_port.then(|| Arc::new(RecordingCorpsePortLikeCpp::default()));
    if let Some(port) = &port {
        session.set_map_corpse_persistence_port_like_cpp(port.clone());
    }
    while send_rx.try_recv().is_ok() {}
    while realm_rx.try_recv().is_ok() {}
    (session, send_rx, realm_rx, player_guid, port)
}

pub(super) fn current_corpse_guid_like_cpp(
    session: &WorldSession,
    owner: ObjectGuid,
) -> Option<ObjectGuid> {
    let manager = crate::session::hub_ref(session)
        .core
        .canonical_map_manager
        .clone()?;
    let manager = manager.lock().unwrap();
    manager
        .find_map(571, 0)?
        .map()
        .corpse_guid_by_player_like_cpp(owner)
}

#[tokio::test]
async fn repop_request_creates_saved_corpse_and_releases_to_graveyard_like_cpp() {
    let (mut session, send_rx, realm_rx, player_guid, port) =
        dead_player_session_with_graveyard_like_cpp(4_301);
    set_death_timer_for_driver(&mut session, 12_345);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| player.set_player_flag(0x0000_4000))
        .expect("canonical Player for OOB setup");

    handle_repop_request(&mut session, repop_request_packet(true)).await;

    // BuildPlayerRepop: ghost at 1 health, corpse on the map at the death spot.
    assert!(!session.player_is_alive_like_cpp());
    assert!(crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    let player_flags = session
        .core
        .with_owned_player_like_cpp(|player| player.data().player_flags)
        .expect("canonical Player flags after repop");
    assert_eq!(player_flags & 0x0000_4000, 0, "RepopAtGraveyard clears OOB");
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((1, 1_000))
    );
    assert!(current_corpse_guid_like_cpp(&session, player_guid).is_some());
    assert_eq!(
        wow_world_entities::player_corpse_location_like_cpp(&session),
        Some((571, Position::new(1.0, 2.0, 3.0, 0.0)))
    );
    assert_eq!(death_timer_from_driver(&session), 0);

    // Corpse::SaveToDB on a non-instanceable map; no prior corpse to delete.
    let saves = port.saves.lock().unwrap().clone();
    assert_eq!(saves.len(), 1);
    assert_eq!(saves[0].owner_guid, 4_301);
    assert_eq!(saves[0].map_id, 571);
    assert_eq!(
        (saves[0].pos_x, saves[0].pos_y, saves[0].pos_z),
        (1.0, 2.0, 3.0)
    );
    assert_eq!(saves[0].race, 1);
    assert_eq!(saves[0].corpse_type, 1);
    assert_eq!(
        saves[0].item_cache.split(' ').count(),
        wow_entities::CORPSE_ITEMS + 1
    );
    assert!(port.deletes.lock().unwrap().is_empty());

    // RepopAtGraveyard: near teleport to WorldSafeLocs 4, then DeathReleaseLoc.
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        1
    );
    // SMSG_PRE_RESSURECT and SMSG_DEATH_RELEASE_LOC go on the realm
    // connection (Opcodes.cpp:1915, :1333); the reclaim delay and the
    // teleport stay on the instance connection.
    assert_eq!(
        sent_opcodes(&realm_rx),
        vec![
            ServerOpcodes::PreRessurect as u16,
            ServerOpcodes::DeathReleaseLoc as u16,
        ]
    );
    let opcodes = sent_opcodes(&send_rx);
    assert!(!opcodes.contains(&(ServerOpcodes::PreRessurect as u16)));
    assert!(!opcodes.contains(&(ServerOpcodes::DeathReleaseLoc as u16)));
    let reclaim_delay = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::CorpseReclaimDelay as u16)
        .expect("BuildPlayerRepop sends the reclaim delay");
    let teleport = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::MoveTeleport as u16)
        .expect("RepopAtGraveyard teleports to the graveyard");
    assert!(reclaim_delay < teleport);
}

#[tokio::test]
async fn spawn_corpse_bones_deletes_rows_and_leaves_bones_like_cpp() {
    let (mut session, _send_rx, _realm_rx, player_guid, port) =
        dead_player_session_with_graveyard_like_cpp(4_302);
    handle_repop_request(&mut session, repop_request_packet(true)).await;
    let corpse_guid = current_corpse_guid_like_cpp(&session, player_guid).unwrap();
    let generators = session.id_generators_for_test_like_cpp();

    assert!(
        wow_world_entities::spawn_corpse_bones_like_cpp(
            &mut session,
            generators.item.as_ref(),
            false,
        )
        .await
    );

    assert!(current_corpse_guid_like_cpp(&session, player_guid).is_none());
    assert_eq!(
        wow_world_entities::player_corpse_location_like_cpp(&session),
        None
    );
    assert_eq!(*port.deletes.lock().unwrap(), vec![4_302]);
    let manager = crate::session::hub_ref(&session)
        .core
        .canonical_map_manager
        .clone()
        .unwrap();
    let manager = manager.lock().unwrap();
    let bones = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_corpse(corpse_guid)
        .expect("bones keep the corpse counter on the same map");
    assert_eq!(bones.corpse_type(), wow_entities::CorpseType::Bones);
}

#[tokio::test]
async fn failed_bones_delete_disconnects_and_fences_immediate_resurrection() {
    let (mut session, _send_rx, _realm_rx, player_guid, port) =
        dead_player_session_with_graveyard_like_cpp(4_311);
    handle_repop_request(&mut session, repop_request_packet(true)).await;
    assert_eq!(
        wow_world_entities::handle_move_teleport_ack_like_cpp(&mut session, player_guid, 0, 0),
        crate::session::MoveTeleportAckActionLikeCpp::Accepted
    );
    assert!(current_corpse_guid_like_cpp(&session, player_guid).is_some());

    let resurrecter = ObjectGuid::create_player(1, 4_312);
    port.fail_next_delete();
    assert!(session.set_represented_resurrection_request_like_cpp(
        wow_entities::PlayerResurrectionRequestLikeCpp {
            resurrecter,
            // TeleportTo rejects this map. C++ continues ResurrectUsingRequestData
            // at the current location when no teleport remains pending.
            map_id: 9_999,
            position: Position::new(11.0, 22.0, 33.0, 1.5),
            health: 450,
            mana: 120,
            aura: 0,
        }
    ));

    handle_resurrect_response(&mut session, resurrect_response_packet(resurrecter, 0)).await;

    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 450);
    assert_eq!(port.delete_requests(), vec![4_311]);
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| player
            .resurrection_state_like_cpp()
            .delayed_after_teleport
            .map(|continuation| continuation.phase)),
        Some(Some(wow_entities::PlayerResurrectionPhase::BonesInFlight)),
        "an unconfirmed delete retains the continuation"
    );
    assert!(
        session.core.player_handle_like_cpp.is_some(),
        "owner is retained"
    );
    assert_eq!(
        session.save_current_player_to_db_like_cpp().await,
        crate::session::PlayerSaveOutcomeLikeCpp::Deferred,
        "the canonical save fence blocks a full save before corpse deletion is confirmed"
    );

    // A duplicate client response cannot rerun corpse deletion or release the
    // retained owner after the partial native resurrection has made it alive.
    handle_resurrect_response(&mut session, resurrect_response_packet(resurrecter, 0)).await;
    assert_eq!(port.delete_requests(), vec![4_311]);
    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert!(session.core.player_handle_like_cpp.is_some());
}

#[tokio::test]
async fn failed_bones_delete_disconnects_from_near_teleport_ack() {
    let (mut session, _send_rx, _realm_rx, player_guid, port) =
        dead_player_session_with_graveyard_like_cpp(4_313);
    session.set_map_store(crate::teleport_test_fixtures::world_maps([571]));
    handle_repop_request(&mut session, repop_request_packet(true)).await;
    assert_eq!(
        wow_world_entities::handle_move_teleport_ack_like_cpp(&mut session, player_guid, 0, 0),
        crate::session::MoveTeleportAckActionLikeCpp::Accepted
    );

    let resurrecter = ObjectGuid::create_player(1, 4_314);
    port.fail_next_delete();
    assert!(session.set_represented_resurrection_request_like_cpp(
        wow_entities::PlayerResurrectionRequestLikeCpp {
            resurrecter,
            map_id: 571,
            position: Position::new(11.0, 22.0, 33.0, 1.5),
            health: 450,
            mana: 120,
            aura: 0,
        }
    ));
    handle_resurrect_response(&mut session, resurrect_response_packet(resurrecter, 0)).await;
    assert!(
        crate::session::hub_ref(&session)
            .player_teleport_state_snapshot_like_cpp()
            .is_some_and(|state| state.near_pending)
    );

    let catalogs = session.session_handler_catalogs_for_test_like_cpp();
    let registration = crate::session::registry::get_handler(ClientOpcodes::MoveTeleportAck)
        .expect("registered MoveTeleportAck handler");
    let mut ack = WorldPacket::new_empty();
    ack.write_packed_guid(&player_guid);
    ack.write_int32(0);
    ack.write_int32(0);
    (registration.handler)(&mut session, &catalogs, ack).await;

    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert_eq!(port.delete_requests(), vec![4_313]);
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| player
            .resurrection_state_like_cpp()
            .delayed_after_teleport
            .map(|continuation| continuation.phase)),
        Some(Some(wow_entities::PlayerResurrectionPhase::BonesInFlight))
    );
    assert_eq!(
        session.save_current_player_to_db_like_cpp().await,
        crate::session::PlayerSaveOutcomeLikeCpp::Deferred
    );
    assert!(session.core.player_handle_like_cpp.is_some());
}

#[tokio::test]
async fn second_repop_after_release_is_ignored_like_cpp() {
    let (mut session, _send_rx, _realm_rx, _player_guid, port) =
        dead_player_session_with_graveyard_like_cpp(4_303);
    handle_repop_request(&mut session, repop_request_packet(true)).await;
    handle_repop_request(&mut session, repop_request_packet(true)).await;

    assert_eq!(port.saves.lock().unwrap().len(), 1);
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        1
    );
}

#[test]
fn corpse_reclaim_delay_follows_cpp_death_expire_steps_like_cpp() {
    use wow_world_core::session::DeathPolicyLikeCpp;
    use wow_world_entities::{
        corpse_reclaim_delay_secs_like_cpp, updated_death_expire_time_like_cpp,
    };
    let policy = DeathPolicyLikeCpp::default();
    let now = 1_000_000;

    // First death: expire = now + 5 min, delay 30 s.
    let first = updated_death_expire_time_like_cpp(false, 0, now, policy);
    assert_eq!(first, now + 300);
    assert_eq!(
        corpse_reclaim_delay_secs_like_cpp(false, first, now, policy),
        30
    );
    // A second death 100 s later: one partial period left, so C++ extends to
    // two full steps from now (count = 200/300 + 1 = 1) and the delay is 60 s.
    let later = now + 100;
    let second = updated_death_expire_time_like_cpp(false, first, later, policy);
    assert_eq!(second, later + 600);
    assert_eq!(
        corpse_reclaim_delay_secs_like_cpp(false, second, later, policy),
        60
    );
    // Dying again at once: count = 600/300 + 1 = 3 reaches MAX_DEATH_COUNT,
    // so the expire time caps at three steps and the delay at 120 s.
    let third = updated_death_expire_time_like_cpp(false, second, later, policy);
    assert_eq!(third, later + 900);
    assert_eq!(
        corpse_reclaim_delay_secs_like_cpp(false, third, later, policy),
        120
    );
    assert_eq!(
        updated_death_expire_time_like_cpp(false, third, later, policy),
        later + 900
    );
    // Once the window has passed the delay drops back to the first step.
    assert_eq!(
        corpse_reclaim_delay_secs_like_cpp(false, third, later + 900, policy),
        30
    );

    // PvE delay disabled: always 0 and the expire time does not move.
    let no_pve = DeathPolicyLikeCpp {
        corpse_reclaim_delay_pve: false,
        ..policy
    };
    assert_eq!(
        corpse_reclaim_delay_secs_like_cpp(false, third, now, no_pve),
        0
    );
    assert_eq!(updated_death_expire_time_like_cpp(false, 7, now, no_pve), 7);
    // PvP delay disabled: always the first step.
    let no_pvp = DeathPolicyLikeCpp {
        corpse_reclaim_delay_pvp: false,
        ..policy
    };
    assert_eq!(
        corpse_reclaim_delay_secs_like_cpp(true, third, now, no_pvp),
        30
    );
}

#[test]
fn corpse_flags_and_item_cache_match_cpp_like_cpp() {
    assert_eq!(
        wow_world_entities::corpse_flags_like_cpp(0x01, false, false),
        0x04
    );
    assert_eq!(
        wow_world_entities::corpse_flags_like_cpp(0x04, true, false),
        0x60
    );
    assert_eq!(wow_world_entities::corpse_flags_like_cpp(0, true, true), 0);
    assert_eq!(
        wow_world_entities::corpse_item_cache_like_cpp(&[1, 0, 33]),
        "1 0 33 "
    );
}

#[tokio::test]
async fn repop_in_dungeon_skips_corpse_save_and_revives_at_teleport_like_cpp() {
    let maps = Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 33,
        instance_type: wow_data::map::MAP_INSTANCE,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));
    let (mut session, _send_rx, realm_rx, _player_guid, port) =
        dead_player_session_on_map_like_cpp(4_304, maps);

    handle_repop_request(&mut session, repop_request_packet(true)).await;

    // CreateCorpse: "we do not need to save corpses for instances".
    assert!(port.saves.lock().unwrap().is_empty());
    // RepopAtGraveyard: a non-raid dungeon converts the corpse to bones and
    // teleports with TELE_REVIVE_AT_TELEPORT, reviving at 50 %.
    assert_eq!(*port.deletes.lock().unwrap(), vec![4_304]);
    assert_eq!(
        wow_world_entities::player_corpse_location_like_cpp(&session),
        None
    );
    assert!(session.player_is_alive_like_cpp());
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((500, 1_000))
    );
    // Alive after the teleport: no DeathReleaseLoc.
    assert_eq!(
        sent_opcodes(&realm_rx),
        vec![ServerOpcodes::PreRessurect as u16]
    );
}

#[tokio::test]
async fn repop_of_just_died_player_runs_kill_player_first_like_cpp() {
    let (mut session, send_rx, _realm_rx, _player_guid, _port) =
        dead_player_session_with_graveyard_like_cpp(4_305);
    let _ = crate::session::hub_ref(&session)
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::JustDied);
        });

    handle_repop_request(&mut session, repop_request_packet(true)).await;

    // KillPlayer and BuildPlayerRepop each send SMSG_CORPSE_RECLAIM_DELAY.
    let opcodes = sent_opcodes(&send_rx);
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::CorpseReclaimDelay as u16)
            .count(),
        2
    );
    let root = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::MoveRoot as u16)
        .expect("KillPlayer roots before changing JUST_DIED to CORPSE");
    let water_walk = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::MoveSetWaterWalk as u16)
        .expect("BuildPlayerRepop enables water walking after health becomes one");
    let unroot = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::MoveUnroot as u16)
        .expect("BuildPlayerRepop unroots when logout and stun guards allow it");
    assert!(root < water_walk && water_walk < unroot);
    assert!(
        crate::session::hub_ref(&session)
            .player_movement_flags_like_cpp()
            .contains(wow_constants::MovementFlag::WATER_WALK)
    );
    assert!(crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
}

#[tokio::test]
async fn repop_does_not_unroot_while_logging_out_or_stunned() {
    for (index, player_logout, timed_logout, stunned) in [
        (0i64, true, false, false),
        (1i64, false, true, false),
        (2i64, false, false, true),
    ] {
        let (mut session, send_rx, _realm_rx, _player_guid, _port) =
            dead_player_session_with_graveyard_like_cpp(4_307 + index);
        session.core.with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::JustDied);
            if stunned {
                player
                    .unit_mut()
                    .add_unit_state(wow_constants::UnitState::STUNNED.bits());
            }
        });
        if player_logout {
            session.set_player_logout_like_cpp(true);
        }
        if timed_logout {
            session.lifecycle.set_logout_start_time_like_cpp(1);
        }

        handle_repop_request(&mut session, repop_request_packet(true)).await;

        let movement_flags = crate::session::hub_ref(&session).player_movement_flags_like_cpp();
        assert!(movement_flags.contains(wow_constants::MovementFlag::ROOT));
        assert!(movement_flags.contains(wow_constants::MovementFlag::WATER_WALK));
        assert!(!sent_opcodes(&send_rx).contains(&(ServerOpcodes::MoveUnroot as u16)));
    }
}

fn set_death_timer_for_driver(session: &mut WorldSession, remaining_ms: u32) {
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .resurrection_state_mut_like_cpp()
                .death_timer_remaining_ms = remaining_ms;
        })
        .expect("canonical Player owner");
}

fn death_timer_from_driver(session: &WorldSession) -> u32 {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .resurrection_state_like_cpp()
                .death_timer_remaining_ms
        })
        .expect("canonical Player owner")
}

fn install_prevent_resurrection_aura_for_driver(session: &mut WorldSession) {
    let spell_id = 94_300;
    let aura_type = wow_data::spell::aura_types::SPELL_AURA_PREVENT_RESURRECTION;
    let mut spells = wow_data::SpellStore::new();
    spells.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(aura_type),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: aura_type,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spells));

    let aura = wow_entities::AuraApplicationLikeCpp {
        spell_id,
        difficulty_id: 0,
        caster_guid: session.player_guid().expect("player GUID"),
        slot: 1,
        duration_total: 10_000,
        duration_remaining: 10_000,
        stack_count: 1,
        aura_flags: 0,
        effect_mask: 1,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: std::time::Instant::now(),
    };
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .insert_runtime_application_like_cpp(aura);
        })
        .expect("canonical Player owner");
}

fn keep_driver_input_open(session: &mut WorldSession) -> flume::Sender<wow_packet::WorldPacket> {
    let (packet_tx, packet_rx) = flume::bounded(8);
    session.set_packet_rx(packet_rx);
    packet_tx
}

#[tokio::test]
async fn driver_kill_player_runs_once_after_regeneration_and_waits_for_timer() {
    let (mut session, send_rx, _realm_rx, _player_guid, _port) =
        dead_player_session_with_graveyard_like_cpp(4_310);
    let _packet_tx = keep_driver_input_open(&mut session);
    session.core.state = crate::session::SessionState::LoggedIn;
    session.core.with_owned_player_mut_like_cpp(|player| {
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::JustDied);
    });

    session.update(0).await;

    assert_eq!(
        session
            .core
            .with_owned_player_like_cpp(|player| player.unit().death_state()),
        Some(wow_constants::DeathState::Corpse)
    );
    assert_eq!(death_timer_from_driver(&session), 360_000);
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    assert!(
        current_corpse_guid_like_cpp(&session, session.player_guid().expect("player GUID"))
            .is_none()
    );

    assert_ne!(
        crate::session::hub_ref(&session).active_player_local_flags_like_cpp() & 0x8,
        0,
        "KillPlayer sets PLAYER_LOCAL_FLAG_RELEASE_TIMER on a world map"
    );

    session.update(25).await;
    assert_eq!(death_timer_from_driver(&session), 359_975);
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());

    session.update(50).await;

    assert_eq!(death_timer_from_driver(&session), 359_925);
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    let opcodes = sent_opcodes(&send_rx);
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::CorpseReclaimDelay as u16)
            .count(),
        1,
        "the driver must consume JUST_DIED once and must not restart its timer"
    );
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );
}

#[tokio::test]
async fn driver_death_timer_expires_at_the_exact_six_minute_boundary() {
    let (mut session, send_rx, realm_rx, player_guid, port) =
        dead_player_session_with_graveyard_like_cpp(4_311);
    let _packet_tx = keep_driver_input_open(&mut session);
    session.core.state = crate::session::SessionState::LoggedIn;
    let (graveyards, conditions) =
        graveyard_store_with_links(0, [wow_data::ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP], []);
    session.set_graveyard_store(graveyards);
    session.set_condition_store(conditions);
    set_death_timer_for_driver(&mut session, 360_000);

    session.update(359_999).await;

    assert_eq!(death_timer_from_driver(&session), 1);
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    assert!(current_corpse_guid_like_cpp(&session, player_guid).is_none());
    assert!(port.saves.lock().unwrap().is_empty());

    session.update(1).await;

    assert_eq!(death_timer_from_driver(&session), 0);
    assert!(crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    assert!(current_corpse_guid_like_cpp(&session, player_guid).is_some());
    assert_eq!(port.saves.lock().unwrap().len(), 1);
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        1
    );
    assert!(crate::session::hub_ref(&session).near_teleport_pending_like_cpp());
    let teleport = crate::session::hub_ref(&session)
        .player_teleport_state_snapshot_like_cpp()
        .expect("canonical teleport state");
    assert!(!teleport.can_delay);
    assert!(!teleport.has_delayed);
    assert_eq!(teleport.delayed, None);
    assert_eq!(
        teleport.near_destination,
        Some((571, Position::new(50.0, 60.0, 70.0, 0.0)))
    );
    let opcodes = sent_opcodes(&send_rx);
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::MoveTeleport as u16)
            .count(),
        1,
        "death-timer graveyard teleport is emitted during expiry, with delayed teleport disabled"
    );
    assert_eq!(
        sent_opcodes(&realm_rx),
        vec![
            ServerOpcodes::PreRessurect as u16,
            ServerOpcodes::DeathReleaseLoc as u16,
        ]
    );
}

#[tokio::test]
async fn driver_death_timer_freezes_in_instance_maps_and_with_prevent_resurrection() {
    let instance_maps = Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 33,
        instance_type: wow_data::map::MAP_INSTANCE,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));
    let (mut instance_session, ..) = dead_player_session_on_map_like_cpp(4_312, instance_maps);
    let _instance_packet_tx = keep_driver_input_open(&mut instance_session);
    instance_session.core.state = crate::session::SessionState::LoggedIn;
    instance_session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::JustDied);
        });
    instance_session.update(500).await;
    assert_eq!(death_timer_from_driver(&instance_session), 360_000);
    assert_eq!(
        crate::session::hub_ref(&instance_session).active_player_local_flags_like_cpp() & 0x8,
        0,
        "KillPlayer clears PLAYER_LOCAL_FLAG_RELEASE_TIMER in instances"
    );

    let (mut aura_session, ..) = dead_player_session_with_graveyard_like_cpp(4_313);
    let _aura_packet_tx = keep_driver_input_open(&mut aura_session);
    aura_session.core.state = crate::session::SessionState::LoggedIn;
    install_prevent_resurrection_aura_for_driver(&mut aura_session);
    aura_session.core.with_owned_player_mut_like_cpp(|player| {
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::JustDied);
    });
    aura_session.update(500).await;
    assert_eq!(death_timer_from_driver(&aura_session), 360_000);
    assert!(!crate::session::hub_ref(&aura_session).player_has_ghost_flag_like_cpp());
    assert_eq!(
        crate::session::hub_ref(&aura_session).active_player_local_flags_like_cpp() & 0x8,
        0,
        "KillPlayer clears PLAYER_LOCAL_FLAG_RELEASE_TIMER with PreventResurrection"
    );
}

#[tokio::test]
async fn driver_does_not_kill_or_advance_death_timer_while_player_is_detached() {
    let (mut session, send_rx, _realm_rx, _player_guid, _port) =
        dead_player_session_with_graveyard_like_cpp(4_314);
    let _packet_tx = keep_driver_input_open(&mut session);
    session.core.state = crate::session::SessionState::LoggedIn;
    set_death_timer_for_driver(&mut session, 12_345);
    session.core.with_owned_player_mut_like_cpp(|player| {
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::JustDied);
    });
    assert!(
        crate::session::hub_mut(&mut session)
            .remove_current_player_from_canonical_current_map_like_cpp()
    );
    assert!(!session.core.player_is_strictly_in_world_like_cpp());

    session.update(500).await;

    assert_eq!(
        session
            .core
            .with_owned_player_like_cpp(|player| player.unit().death_state()),
        Some(wow_constants::DeathState::JustDied)
    );
    assert_eq!(death_timer_from_driver(&session), 12_345);
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    let opcodes = sent_opcodes(&send_rx);
    for forbidden in [
        ServerOpcodes::CorpseReclaimDelay as u16,
        ServerOpcodes::MoveRoot as u16,
        ServerOpcodes::MoveSetWaterWalk as u16,
    ] {
        assert!(!opcodes.contains(&forbidden));
    }
}

/// A released ghost (corpse at (1, 2, 3) on 571) with the PvE reclaim delay
/// disabled so the reclaim is admitted at once.
pub(super) async fn released_ghost_without_reclaim_delay_like_cpp(
    guid_counter: i64,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
    Arc<RecordingCorpsePortLikeCpp>,
) {
    let (mut session, send_rx, realm_rx, _player_guid, port) =
        dead_player_session_with_graveyard_like_cpp(guid_counter);
    session.set_death_policy_like_cpp(wow_world_core::session::DeathPolicyLikeCpp {
        corpse_reclaim_delay_pve: false,
        ..Default::default()
    });
    handle_repop_request(&mut session, repop_request_packet(true)).await;
    assert!(crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    while send_rx.try_recv().is_ok() {}
    while realm_rx.try_recv().is_ok() {}
    (session, send_rx, realm_rx, port)
}

#[tokio::test]
async fn reclaim_corpse_in_range_resurrects_half_and_spawns_bones_like_cpp() {
    let (mut session, send_rx, realm_rx, port) =
        released_ghost_without_reclaim_delay_like_cpp(4_401).await;
    let player_guid = session.player_guid().unwrap();
    let corpse_guid = current_corpse_guid_like_cpp(&session, player_guid).unwrap();
    // The ghost walks back to its corpse.
    session.set_player_position_like_cpp(Position::new(10.0, 2.0, 3.0, 0.0));
    // C++ ResurrectPlayer forces the water-walking bit on before
    // SetWaterWalking(false), so land-walk must publish even with no bit set.
    session.core.with_owned_player_mut_like_cpp(|player| {
        let mut flags = player.unit().movement_flags_like_cpp();
        flags.remove(wow_constants::MovementFlag::WATER_WALK);
        player.unit_mut().set_movement_flags_like_cpp(flags);
    });
    set_death_timer_for_driver(&mut session, 12_345);

    handle_reclaim_corpse(&mut session, reclaim_corpse_packet(corpse_guid)).await;

    // ResurrectPlayer(0.5f): alive, ghost cleared, half health.
    assert!(session.player_is_alive_like_cpp());
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((500, 1_000))
    );
    assert_eq!(death_timer_from_driver(&session), 0);
    // SMSG_DEATH_RELEASE_LOC(-1) on the realm connection (Opcodes.cpp:1333).
    assert_eq!(
        sent_opcodes(&realm_rx),
        vec![ServerOpcodes::DeathReleaseLoc as u16]
    );
    let opcodes = sent_opcodes(&send_rx);
    let land_walk = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::MoveSetLandWalk as u16)
        .expect("ResurrectPlayer forces the land-walk status packet");
    let unroot = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::MoveUnroot as u16)
        .expect("ResurrectPlayer unroots after changing to land-walk");
    assert!(land_walk < unroot);
    assert!(
        !crate::session::hub_ref(&session)
            .player_movement_flags_like_cpp()
            .contains(wow_constants::MovementFlag::WATER_WALK)
    );
    // SpawnCorpseBones: corpse rows deleted, bones keep the corpse counter.
    assert_eq!(*port.deletes.lock().unwrap(), vec![4_401]);
    assert!(current_corpse_guid_like_cpp(&session, player_guid).is_none());
    assert_eq!(
        wow_world_entities::player_corpse_location_like_cpp(&session),
        None
    );
}

#[tokio::test]
async fn reclaim_corpse_out_of_range_is_ignored_like_cpp() {
    let (mut session, send_rx, realm_rx, port) =
        released_ghost_without_reclaim_delay_like_cpp(4_402).await;
    let player_guid = session.player_guid().unwrap();
    let corpse_guid = current_corpse_guid_like_cpp(&session, player_guid).unwrap();
    // CORPSE_RECLAIM_RADIUS 39 plus the combat reaches: 60 yards is too far.
    session.set_player_position_like_cpp(Position::new(61.0, 2.0, 3.0, 0.0));

    handle_reclaim_corpse(&mut session, reclaim_corpse_packet(corpse_guid)).await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    assert!(port.deletes.lock().unwrap().is_empty());
    assert!(sent_opcodes(&send_rx).is_empty());
    assert!(sent_opcodes(&realm_rx).is_empty());
}

#[tokio::test]
async fn reclaim_corpse_before_reclaim_delay_is_ignored_like_cpp() {
    let (mut session, _send_rx, _realm_rx, _player_guid, port) =
        dead_player_session_with_graveyard_like_cpp(4_403);
    // Default policy: Death.CorpseReclaimDelay.PvE = true, so the first death
    // waits 30 s after the release.
    handle_repop_request(&mut session, repop_request_packet(true)).await;
    let player_guid = session.player_guid().unwrap();
    let corpse_guid = current_corpse_guid_like_cpp(&session, player_guid).unwrap();
    session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.0));

    handle_reclaim_corpse(&mut session, reclaim_corpse_packet(corpse_guid)).await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(port.deletes.lock().unwrap().is_empty());
}
