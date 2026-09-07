//! Finalization and failed-login retirement through character consumers.

use super::*;

#[path = "transfer_routing.rs"]
mod transfer_routing;

#[test]
fn late_login_sequence_failure_releases_claim_and_partial_player_like_cpp() {
    let guid = ObjectGuid::create_player(1, 9_001_701);
    let (mut failed, _failed_rx) = make_session_with_send_capacity(1);
    assert!(failed.try_claim_character_login_like_cpp(guid));
    assert!(failed.ensure_login_player_controller_like_cpp(
        guid,
        "LateFenceFailure".to_string(),
        Position::ZERO,
        1,
        1,
        1,
        10,
        0,
    ));

    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut failed, 1, 0);
    failed.abort_partial_login_sequence_like_cpp();

    assert_eq!(failed.state(), crate::session::SessionState::Disconnecting);
    assert!(failed.player_guid().is_none());
    let (mut retry, _retry_rx) = make_session_with_send_capacity(1);
    assert!(
        retry.try_claim_character_login_like_cpp(guid),
        "the failed login must not retain the only process-wide character claim"
    );
    retry.release_character_login_claim_like_cpp();
}

#[tokio::test]
async fn unavailable_login_grid_aborts_before_success_login_packets_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let guid = ObjectGuid::create_player(1, 46);
    assert!(session.ensure_login_player_controller_like_cpp(
        guid,
        "PreflightFailure".to_string(),
        Position::ZERO,
        1,
        1,
        1,
        10,
        0,
    ));
    let player_grid_loader: crate::session::PlayerGridLoadResolverLikeCpp =
        Arc::new(|_, _, _| crate::session::PlayerGridLoadOutcomeLikeCpp {
            map_unavailable: true,
            ..Default::default()
        });
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 1, 0);
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawn_catalogs = session.creature_spawn_catalogs_for_test_like_cpp();
    let feature_policy = session.support_feature_policy_for_test_like_cpp();

    assert!(
        !session
            .send_login_sequence(
                generators.item.as_ref(),
                &wow_data::trait_tree::TraitNodeEntryStore::from_entries([]),
                &creature_spawn_catalogs,
                &feature_policy,
                &player_grid_loader,
                guid,
                1,
                1,
                0,
                10,
                49,
                &Position::ZERO,
                1,
                0,
                CharacterLoginLocationLikeCpp {
                    map_id: 1,
                    bind_area_id: Some(0),
                    position: Position::ZERO,
                },
                None,
                [(0, 0, 0); 19],
                [ObjectGuid::EMPTY; 141],
                Vec::new(),
                PlayerCombatStats::default(),
                0,
                0,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                [0; 180],
                Vec::new(),
                Vec::new(),
            )
            .await
    );

    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert!(session.player_guid().is_none());
    assert!(
        send_rx.try_recv().is_err(),
        "C++ LoadFromDB failure happens before DungeonDifficultySet/LoginVerifyWorld"
    );
}

#[test]
fn login_without_grid_resolver_fails_closed_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let guid = ObjectGuid::create_player(1, 43);
    assert!(session.ensure_login_player_controller_like_cpp(
        guid,
        "MissingResolver".to_string(),
        Position::ZERO,
        1,
        1,
        1,
        10,
        0,
    ));

    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 1, 0);
    assert!(!session.continue_login_after_grid_load_like_cpp(guid, 1, 0, None));

    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert!(session.player_guid().is_none());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn logout_releases_active_loot_views_like_cpp_remove_from_world() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_player_lifecycle_port_like_cpp(CollectionLoadPortLikeCpp::new([]));
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 19_030);
    let canonical: crate::session::SharedCanonicalMapManager =
        Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 1,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    assert!(session.ensure_login_player_controller_like_cpp(
        player_guid,
        "LogoutOwner".to_string(),
        Position::ZERO,
        1,
        1,
        1,
        10,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    assert_eq!(
        session.current_canonical_player_map_key_like_cpp(),
        Some(wow_map::MapKey::new(1, 0))
    );
    assert!(session.try_claim_character_login_like_cpp(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_logout_request(LogoutRequest { idle_logout: false })
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LogoutResponse as u16
    );

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);

    assert!(
        send_rx.try_recv().is_err(),
        "failed persistence must not publish LogoutComplete"
    );
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(
        !session.loot_table.contains_key(&loot_guid),
        "loot release retires the packet-cache copy before persistence"
    );
    assert_eq!(session.player_guid(), Some(player_guid));
    assert_eq!(
        session.finalization_report_like_cpp().unwrap().disposition,
        crate::FinalizationDisposition::RetainAndEscalate
    );
    assert!(
        canonical
            .lock()
            .unwrap()
            .find_map(1, 0)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .is_some(),
        "failed persistence retains the canonical Player"
    );
    let (mut replacement, _) = make_session_with_send_capacity(1);
    assert!(
        !replacement.try_claim_character_login_like_cpp(player_guid),
        "failed finalization retains its claim"
    );
    replacement.release_character_login_claim_like_cpp();
}
