//! Session scenarios exercising the represented login responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn check_account_heirloom_upgrades_promotes_static_owned_variant_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 80);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    let owned_upgrade_guid = ObjectGuid::create_item(1, 90_002);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.set_heirloom_store(Arc::new(HeirloomStore::from_entries([
        HeirloomEntry {
            id: 1,
            source_text: "base".to_string(),
            item_id: 44_000,
            legacy_upgraded_item_id: 0,
            static_upgraded_item_id: 44_001,
            source_type_enum: 0,
            flags: 0,
            legacy_item_id: 0,
            upgrade_item_id: [0; 6],
            upgrade_item_bonus_list_id: [0; 6],
        },
        HeirloomEntry {
            id: 2,
            source_text: "heroic".to_string(),
            item_id: 44_001,
            legacy_upgraded_item_id: 0,
            static_upgraded_item_id: 44_002,
            source_type_enum: 0,
            flags: 0,
            legacy_item_id: 0,
            upgrade_item_id: [0; 6],
            upgrade_item_bonus_list_id: [0; 6],
        },
        HeirloomEntry {
            id: 3,
            source_text: "mythic".to_string(),
            item_id: 44_002,
            legacy_upgraded_item_id: 0,
            static_upgraded_item_id: 0,
            source_type_enum: 0,
            flags: 0,
            legacy_item_id: 0,
            upgrade_item_id: [0; 6],
            upgrade_item_bonus_list_id: [0; 6],
        },
    ])));
    session.load_represented_account_heirlooms_like_cpp([(44_000, 0x03)]);
    session
        .add_player_heirloom_dynamic_fields_like_cpp(44_000, 0x03)
        .unwrap();
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());
    insert_open_item_top_level(
        &mut session,
        player_guid,
        23,
        owned_upgrade_guid,
        44_002,
        true,
    );

    let update = session
        .check_account_heirloom_upgrades_like_cpp(44_000)
        .expect("owned static upgrade should promote the account heirloom row");

    assert_eq!(session.account_heirloom_rows_like_cpp(), vec![(44_002, 0)]);
    assert_eq!(session.account_heirloom_bonus_like_cpp(44_000), 0);
    assert_eq!(session.account_heirloom_bonus_like_cpp(44_002), 0);
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                (
                    player.heirlooms_like_cpp().to_vec(),
                    player.heirloom_flags_like_cpp().to_vec(),
                )
            })
            .unwrap(),
        (vec![44_002], vec![0])
    );
    let active_update = update.active_player_data.as_ref().unwrap();
    assert!(
        active_update
            .mask
            .is_set(wow_entities::ACTIVE_PLAYER_DATA_HEIRLOOMS_BIT)
    );
    assert!(
        active_update
            .mask
            .is_set(wow_entities::ACTIVE_PLAYER_DATA_HEIRLOOM_FLAGS_BIT)
    );
}
#[test]
fn account_toy_rows_preserve_cpp_flags_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_battlenet_account_id(77);

    session.load_represented_account_toys_like_cpp([(30_001, false, true), (30_000, true, false)]);

    assert_eq!(
        session.account_toy_rows_like_cpp(),
        vec![(30_000, true, false), (30_001, false, true)]
    );
    assert_eq!(
        session.account_toy_save_rows_like_cpp(),
        Some(vec![
            AccountToySaveRowLikeCpp {
                bnet_account_id: 77,
                item_id: 30_000,
                is_favorite: true,
                has_fanfare: false,
            },
            AccountToySaveRowLikeCpp {
                bnet_account_id: 77,
                item_id: 30_001,
                is_favorite: false,
                has_fanfare: true,
            },
        ]),
        "C++ CollectionMgr::SaveAccountToys appends the battlenet account id to the LoginDatabase transaction"
    );
    assert_eq!(
        session.account_toy_packet_rows_like_cpp(),
        vec![
            AccountToy {
                item_id: 30_000,
                is_favorite: true,
                has_fanfare: false,
            },
            AccountToy {
                item_id: 30_001,
                is_favorite: false,
                has_fanfare: true,
            },
        ]
    );
    assert_eq!(
        session.account_toy_active_player_rows_like_cpp(),
        vec![30_000, 30_001]
    );
}
#[test]
fn add_account_toy_inserts_once_like_cpp() {
    let (mut session, _, _) = make_session();

    assert!(session.add_account_toy_like_cpp(30_000, false, false));
    assert!(!session.add_account_toy_like_cpp(30_000, true, true));

    assert_eq!(
        session.account_toy_rows_like_cpp(),
        vec![(30_000, false, false)]
    );
}
#[test]
fn canonical_player_logout_retires_detached_handle_like_cpp() {
    run_canonical_player_owner_test(|| {
        let (mut session, _, _) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 56);

        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            player_guid,
            "LogoutDetached".to_string(),
            Position::new(3700.0, 1500.0, 120.0, 0.0),
            571,
            1,
            1,
            80,
            0,
        ));
        session
            .ensure_canonical_world_map_for_current_player_like_cpp()
            .expect("initial world map");
        let handle = session.player_handle_like_cpp.expect("canonical handle");
        assert!(session.remove_current_player_from_canonical_current_map_like_cpp());

        session.cleanup_shared_runtime_state();

        assert_eq!(session.player_handle_like_cpp, None);
        assert_eq!(
            canonical.lock().unwrap().player_residence_like_cpp(handle),
            None
        );
    });
}
#[tokio::test]
async fn disconnect_cleanup_releases_active_loot_views_like_cpp_logout_player() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 19_040);
    session.set_player_guid(Some(player_guid));
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
        .cleanup_shared_runtime_state_on_disconnect_like_cpp()
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(
        !session.loot_table.contains_key(&loot_guid),
        "full disconnect release retires the session packet-cache copy like C++"
    );
}
