//! Login scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn continue_login_no_longer_names_the_core_character_statement() {
    let source = include_str!("../character/world_entry.rs");
    let (_, tail) = source
        .split_once("pub async fn handle_continue_player_login")
        .expect("continue-login handler starts");
    let (handler, _) = tail
        .split_once("pub(super) fn player_login_combat_stats_like_cpp")
        .expect("continue-login handler ends before packet helper");
    assert!(handler.contains("load_character_base_like_cpp"));
    assert!(handler.contains("PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(Some(row))"));
    assert!(handler.contains("PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(None)"));
    assert!(handler.contains("PlayerCharacterBaseLoadOutcomeLikeCpp::Failed { reason }"));
    assert!(!handler.contains("prepare(CharStatements::SEL_CHARACTER)"));
}
#[tokio::test]
async fn account_collection_empty_and_adapter_failure_clear_represented_rows_like_cpp() {
    let port = CollectionLoadPortLikeCpp::new([
        AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Toys(
            Vec::new(),
        )),
        AccountCollectionLoadOutcomeLikeCpp::Failed {
            reason: "heirloom read failed".to_owned(),
        },
    ]);
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_battlenet_account_id(77);
    session.load_represented_account_toys_like_cpp([(42, true, false)]);
    session.load_represented_account_heirlooms_like_cpp([(43, 2)]);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    session.load_account_toys_like_cpp().await;
    session.load_account_heirlooms_like_cpp().await;

    assert!(session.account_toy_rows_like_cpp().is_empty());
    assert!(session.account_heirloom_rows_like_cpp().is_empty());
    assert_eq!(
        port.requests(),
        vec![
            AccountCollectionLoadRequestLikeCpp::Toys {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::Heirlooms {
                bnet_account_id: 77
            },
        ]
    );
}
#[test]
fn void_storage_login_context_preserves_cpp_field_five_bug() {
    let selected_context_column = ItemContext::Timewalking as u8;

    assert_eq!(
        void_storage_login_context_like_cpp(29, selected_context_column),
        29
    );
    assert_ne!(29, selected_context_column);
}
#[tokio::test]
async fn handle_player_login_prelude_resends_account_state_and_orders_packets_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(32);
    let guid = ObjectGuid::create_player(1, 42);
    let tutorials = [10, 20, 30, 40, 50, 60, 70, 80];
    let mounts = [
        AccountMount {
            spell_id: 100,
            flags: 1,
        },
        AccountMount {
            spell_id: 200,
            flags: 2,
        },
    ];
    session.set_player_guid(Some(guid));
    session.load_tutorials_data_values_like_cpp(Some(tutorials));
    let generators = session.id_generators_for_test_like_cpp();
    let feature_policy = session.support_feature_policy_for_test_like_cpp();
    assert!(
        session
            .send_handle_player_login_packets_like_cpp(
                generators.item.as_ref(),
                &feature_policy,
                guid,
                &Position::new(1.0, 2.0, 3.0, 4.0),
                571,
                &mounts,
                "first@second",
            )
            .await
    );

    let packets = send_rx.try_iter().collect::<Vec<_>>();
    let opcodes = packets
        .iter()
        .filter_map(|bytes| WorldPacket::from_bytes(bytes).server_opcode())
        .collect::<Vec<_>>();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::AccountMountUpdate,
            ServerOpcodes::AccountMountUpdate,
            ServerOpcodes::AccountDataTimes,
            ServerOpcodes::TutorialFlags,
            ServerOpcodes::SetDungeonDifficulty,
            ServerOpcodes::LoginVerifyWorld,
            ServerOpcodes::AccountDataTimes,
            ServerOpcodes::FeatureSystemStatus,
            ServerOpcodes::ChatServerMessage,
            ServerOpcodes::ChatServerMessage,
            ServerOpcodes::SetTimeZoneInformation,
            ServerOpcodes::BattlePetJournalLockAcquired,
        ]
    );

    for (packet, expected_mount) in packets[..2].iter().zip(mounts) {
        let mut body = WorldPacket::from_bytes(&packet[2..]);
        assert!(!body.read_bit().unwrap());
        assert_eq!(body.read_int32().unwrap(), 1);
        assert_eq!(body.read_int32().unwrap(), expected_mount.spell_id);
        assert_eq!(body.read_bits(4).unwrap(), u32::from(expected_mount.flags));
        assert_eq!(body.remaining(), 0);
    }

    let mut global_account_data = WorldPacket::from_bytes(&packets[2][2..]);
    assert_eq!(
        global_account_data.read_packed_guid().unwrap(),
        ObjectGuid::EMPTY
    );
    let mut tutorial_packet = WorldPacket::from_bytes(&packets[3][2..]);
    for expected in tutorials {
        assert_eq!(tutorial_packet.read_uint32().unwrap(), expected);
    }
    assert_eq!(tutorial_packet.remaining(), 0);

    let mut character_account_data = WorldPacket::from_bytes(&packets[6][2..]);
    assert_eq!(character_account_data.read_packed_guid().unwrap(), guid);

    for (packet, expected_line) in packets[8..10].iter().zip(["first", "second"]) {
        let mut body = WorldPacket::from_bytes(&packet[2..]);
        assert_eq!(body.read_int32().unwrap(), 3);
        let string_len = body.read_bits(11).unwrap() as usize;
        assert_eq!(body.read_string(string_len).unwrap(), expected_line);
        assert_eq!(body.remaining(), 0);
    }

    assert!(session.has_represented_battle_pet_journal_lock_like_cpp());
}
#[test]
fn battleground_login_fallback_prefers_valid_entry_point_then_homebind_like_cpp() {
    let map_store = wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 1,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ]);
    let entry_point = CharacterLoginLocationLikeCpp {
        map_id: 1,
        bind_area_id: None,
        position: Position::new(10.0, 20.0, 30.0, 1.0),
    };
    let homebind = CharacterLoginLocationLikeCpp {
        map_id: 0,
        bind_area_id: Some(12),
        position: Position::new(-1.0, -2.0, 3.0, 0.0),
    };
    let bg_data = CharacterBattlegroundLoginDataLikeCpp { entry_point };

    assert!(usable_character_homebind_like_cpp(
        homebind,
        Some(&map_store),
        2,
    ));
    assert!(!usable_character_homebind_like_cpp(
        entry_point,
        Some(&map_store),
        2,
    ));

    assert_eq!(
        login_location_zone_area_like_cpp(entry_point, |map_id, position| {
            assert_eq!(map_id, 1);
            assert_eq!(position, entry_point.position);
            Ok((34, 56))
        })
        .unwrap(),
        (34, 56)
    );
    assert_eq!(
        login_location_zone_area_like_cpp(homebind, |map_id, position| {
            assert_eq!(map_id, 0);
            assert_eq!(position, homebind.position);
            Ok((78, 90))
        })
        .unwrap(),
        (78, 90)
    );
    let bind_update = login_bind_point_update_like_cpp(homebind);
    assert_eq!(bind_update.x, homebind.position.x);
    assert_eq!(bind_update.y, homebind.position.y);
    assert_eq!(bind_update.z, homebind.position.z);
    assert_eq!(bind_update.map_id, homebind.map_id);
    assert_eq!(bind_update.area_id, 12);

    assert_eq!(
        battleground_login_fallback_location_like_cpp(
            Some(bg_data),
            Some(homebind),
            Some(&map_store),
        ),
        Some(entry_point)
    );
    assert_eq!(
        battleground_login_fallback_location_like_cpp(
            Some(CharacterBattlegroundLoginDataLikeCpp {
                entry_point: CharacterLoginLocationLikeCpp {
                    map_id: u32::from(u16::MAX),
                    bind_area_id: None,
                    position: Position::ZERO,
                },
                ..bg_data
            }),
            Some(homebind),
            Some(&map_store),
        ),
        Some(homebind)
    );
    assert_eq!(
        battleground_login_fallback_location_like_cpp(
            None,
            Some(CharacterLoginLocationLikeCpp {
                position: Position::new(f32::NAN, 0.0, 0.0, 0.0),
                ..homebind
            }),
            Some(&map_store),
        ),
        None
    );
}
#[test]
fn rejected_instance_login_retries_valid_homebind_before_disconnect_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, _send_rx) = make_session_with_send_capacity(2);
        let guid = ObjectGuid::create_player(1, 45);
        let saved_position = Position::new(1.0, 2.0, 3.0, 0.0);
        let homebind_position = Position::new(10.0, 20.0, 30.0, 1.0);
        let canonical: crate::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 33,
                instance_type: wow_data::map::MAP_INSTANCE,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
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
            guid,
            "InstanceFallback".to_string(),
            saved_position,
            33,
            1,
            1,
            10,
            0,
        ));
        assert!(matches!(
            session.ensure_canonical_world_map_for_current_player_like_cpp(),
            Some(wow_map::CreateMapDecision::Reject { .. })
        ));
        assert!(
            session
                .current_canonical_player_map_key_like_cpp()
                .is_none()
        );

        let mut map_id = 33;
        let mut zone_id = 999;
        let mut position = saved_position;
        assert!(session.retry_login_at_homebind_like_cpp(
            &mut map_id,
            &mut zone_id,
            &mut position,
            CharacterLoginLocationLikeCpp {
                map_id: 1,
                bind_area_id: Some(12),
                position: homebind_position,
            },
        ));

        assert_eq!(map_id, 1);
        assert_eq!(zone_id, 12);
        assert_eq!(session.player_zone_area_like_cpp(), Some((12, 12)));
        assert_eq!(position, homebind_position);
        assert_eq!(
            session.current_canonical_player_map_key_like_cpp(),
            Some(wow_map::MapKey::new(1, 0))
        );
    });
}
#[test]
fn garrison_login_uses_create_map_world_branch_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let guid = ObjectGuid::create_player(1, 47);
        let canonical: crate::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 1_151,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 2,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: wow_data::map::MAP_FLAG_GARRISON,
                flags2: 0,
            },
        ])));
        assert!(session.ensure_login_player_controller_like_cpp(
            guid,
            "GarrisonLogin".to_string(),
            Position::ZERO,
            1_151,
            1,
            1,
            10,
            0,
        ));

        assert!(matches!(
            session.ensure_canonical_world_map_for_current_player_like_cpp(),
            Some(wow_map::CreateMapDecision::Create {
                key,
                kind: wow_map::ManagedMapKind::World,
                ..
            }) if key == wow_map::MapKey::new(1_151, 0)
        ));
        assert_eq!(
            session.current_canonical_player_map_key_like_cpp(),
            Some(wow_map::MapKey::new(1_151, 0))
        );
    });
}
#[test]
fn garrison_login_rejects_unsupported_expansion_and_retries_homebind_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let guid = ObjectGuid::create_player(1, 48);
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
            wow_data::MapEntry {
                id: 1_151,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 3,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: wow_data::map::MAP_FLAG_GARRISON,
                flags2: 0,
            },
        ])));
        assert!(session.ensure_login_player_controller_like_cpp(
            guid,
            "UnsupportedGarrisonLogin".to_string(),
            Position::ZERO,
            1_151,
            1,
            1,
            10,
            0,
        ));

        assert!(matches!(
            session.ensure_canonical_world_map_for_current_player_like_cpp(),
            Some(wow_map::CreateMapDecision::Reject { .. })
        ));
        assert!(
            session
                .current_canonical_player_map_key_like_cpp()
                .is_none()
        );
        assert!(canonical.lock().unwrap().find_map(1_151, 0).is_none());

        let mut map_id = 1_151;
        let mut zone_id = 999;
        let mut position = Position::ZERO;
        let homebind_position = Position::new(10.0, 20.0, 30.0, 1.0);
        assert!(session.retry_login_at_homebind_like_cpp(
            &mut map_id,
            &mut zone_id,
            &mut position,
            CharacterLoginLocationLikeCpp {
                map_id: 1,
                bind_area_id: Some(12),
                position: homebind_position,
            },
        ));
        assert_eq!(map_id, 1);
        assert_eq!(zone_id, 12);
        assert_eq!(session.player_zone_area_like_cpp(), Some((12, 12)));
        assert_eq!(position, homebind_position);
        assert_eq!(
            session.current_canonical_player_map_key_like_cpp(),
            Some(wow_map::MapKey::new(1, 0))
        );
    });
}
#[test]
fn unavailable_login_grid_cleans_partial_player_and_kicks_without_failure_packet_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        let guid = ObjectGuid::create_player(1, 42);
        let canonical: crate::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        let registry = Arc::new(crate::session::directory::PlayerRegistry::default());
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 33,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        session.set_player_registry(Arc::clone(&registry));
        assert!(session.ensure_login_player_controller_like_cpp(
            guid,
            "GridFailure".to_string(),
            Position::ZERO,
            33,
            1,
            1,
            10,
            0,
        ));
        let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
        session.register_in_player_registry();
        assert!(
            session
                .current_canonical_player_map_key_like_cpp()
                .is_some()
        );
        assert!(registry.runtime_recipient(guid).is_some());

        assert!(!session.continue_login_after_grid_load_like_cpp(
            guid,
            33,
            0,
            Some(crate::session::PlayerGridLoadOutcomeLikeCpp {
                map_unavailable: true,
                ..Default::default()
            }),
        ));

        assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
        assert!(session.player_guid().is_none());
        assert!(
            canonical
                .lock()
                .unwrap()
                .find_map(33, 0)
                .unwrap()
                .map()
                .get_typed_player(guid)
                .is_none()
        );
        assert!(registry.runtime_recipient(guid).is_none());
        assert!(send_rx.try_recv().is_err());
    });
}
#[test]
fn login_identity_hydrates_race_faction_into_registry_and_canonical_player_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let guid = ObjectGuid::create_player(1, 42_001);
    let canonical: crate::session::SharedCanonicalMapManager =
        Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    let registry = Arc::new(crate::session::directory::PlayerRegistry::default());
    let mut race_entry = chr_race_entry(1, 0);
    race_entry.faction_id = 1;

    session.set_chr_races_store(Arc::new(ChrRacesStore::from_entries([race_entry])));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_registry(Arc::clone(&registry));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));

    // Mirror the real LoadFromDB order: identity is loaded from the
    // character row before the controller/map/registry publication.
    session.set_player_guid(Some(guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
    assert!(session.ensure_login_player_controller_like_cpp(
        guid,
        "FactionLogin".to_string(),
        Position::ZERO,
        571,
        1,
        1,
        10,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.register_in_player_registry();

    assert_eq!(registry.legacy_aggro_candidates()[0].faction_template_id, 1);
    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .expect("login map")
        .map()
        .get_typed_player(guid)
        .expect("canonical login player");
    assert_eq!(player.unit().data().faction_template, 1);
}
#[test]
fn login_passive_parry_and_block_capabilities_feed_first_stat_projection_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 86);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_stats(Arc::new(PlayerStatsStore::from_entries([(
        (1, 1, 80),
        PlayerLevelStats {
            strength: 50,
            agility: 30,
            stamina: 40,
            intellect: 10,
            spirit: 20,
            base_mana: 0,
        },
    )])));
    session.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([chr_class_entry(
        1, 0,
    )])));
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 0, 0, 100, 220);
    let parry_spell_id = 90_087;
    let block_spell_id = 90_088;
    session.set_spell_store(Arc::new(passive_combat_capability_spell_store_like_cpp(
        parry_spell_id,
        block_spell_id,
    )));

    assert_eq!(
        session.canonical_player_parry_block_snapshot_like_cpp(),
        (false, false)
    );
    assert_eq!(
        session.apply_login_known_spell_combat_capabilities_like_cpp(&[
            parry_spell_id,
            block_spell_id,
        ]),
        2
    );
    assert_eq!(
        session.canonical_player_parry_block_snapshot_like_cpp(),
        (true, true)
    );
    let projection = session
        .player_stat_system_projection_like_cpp(
            1,
            1,
            80,
            &RepresentedPlayerGearStatsLikeCpp::default(),
        )
        .expect("warrior stat projection");
    assert_eq!(projection.parry_pct, 5.0);
    assert_eq!(projection.block_pct, 5.0);
}
