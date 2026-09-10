//! Misc scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn init_self_orders_transport_attached_player_and_fellow_passenger_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let passenger_guid = ObjectGuid::create_player(1, 43);
    let transport_guid = ObjectGuid::create_transport(HighGuid::Transport, 7_001);
    let mut player_update = UpdateObject::create_player(
        player_guid,
        1,
        8,
        0,
        80,
        49,
        &Position::ZERO,
        571,
        0,
        true,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    player_update.set_player_movement_transport_like_cpp(TransportInfo {
        guid: transport_guid,
        x: 1.0,
        y: 2.0,
        z: 3.0,
        o: 0.5,
        seat: -1,
        time: 0,
        prev_time: None,
        vehicle_id: None,
    });
    let transport_block = UpdateObject::create_transport_block(
        GameObjectCreateData {
            guid: transport_guid,
            entry: 1,
            dynamic_flags: 0,
            display_id: 2,
            go_type: GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_LIKE_CPP,
            position: Position::ZERO,
            rotation: [0.0, 0.0, 0.0, 1.0],
            anim_progress: 255,
            state: wow_entities::GoState::Ready as i8,
            art_kit: 0,
            created_by: ObjectGuid::EMPTY,
            faction_template: 0,
            gameobject_flags: 0x0010_0028,
            world_effect_id: 0,
            scale: 1.0,
            level: 1_000,
            parent_rotation: [0.0, 0.0, 0.0, 1.0],
        },
        0,
    );
    let mut passenger_update = UpdateObject::create_player(
        passenger_guid,
        1,
        8,
        0,
        80,
        49,
        &Position::ZERO,
        571,
        0,
        false,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    let passenger_block = passenger_update
        .blocks
        .pop()
        .expect("fellow passenger CREATE");

    assert_eq!(
        compose_init_self_create_blocks_like_cpp(
            &mut player_update,
            Vec::new(),
            Some((transport_guid, transport_block)),
            vec![passenger_block],
        ),
        Some(transport_guid)
    );
    assert_eq!(player_update.num_updates, 3);
    assert!(matches!(
        player_update.blocks.first(),
        Some(UpdateBlock::CreateTransport { guid, .. }) if *guid == transport_guid
    ));
    let Some(UpdateBlock::CreateObject {
        guid,
        movement: Some(movement),
        is_self: true,
        ..
    }) = player_update.blocks.get(1)
    else {
        panic!("expected attached self player after its transport");
    };
    assert_eq!(*guid, player_guid);
    assert_eq!(
        movement.transport.as_ref().map(|transport| transport.guid),
        Some(transport_guid)
    );
    assert!(matches!(
        player_update.blocks.get(2),
        Some(UpdateBlock::CreateObject {
            guid,
            is_self: false,
            ..
        }) if *guid == passenger_guid
    ));
}
#[test]
fn pvp_season_world_states_match_cpp_world_state_mgr() {
    // In-progress arena season 32 -> current(3191)=32, previous(3901)=31. Matches the
    // captured C++ INIT_WORLD_STATES (World.cpp:1363-1364). Existing ids stay untouched
    // and the 3191/3901 values are overridden in place (not duplicated).
    let mut states = vec![(3191, 0), (3901, 0), (1000, 5)];
    apply_pvp_season_world_states_like_cpp(&mut states, 32, true);
    assert_eq!(states, vec![(3191, 32), (3901, 31), (1000, 5)]);

    // Default (season not in progress): current=0, previous=season_id.
    let mut states = vec![(3191, 0), (3901, 0)];
    apply_pvp_season_world_states_like_cpp(&mut states, 32, false);
    assert_eq!(states, vec![(3191, 0), (3901, 32)]);

    // Absent ids are appended rather than dropped.
    let mut states: Vec<(i32, i32)> = Vec::new();
    apply_pvp_season_world_states_like_cpp(&mut states, 10, true);
    assert_eq!(states, vec![(3191, 10), (3901, 9)]);
}
#[tokio::test]
async fn initial_world_state_port_preserves_independent_read_failures_like_cpp() {
    let port = CollectionLoadPortLikeCpp::for_initial_world_states([
        PlayerInitialWorldStatesLoadOutcomeLikeCpp {
            templates: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![
                PlayerInitialWorldStateTemplateRowLikeCpp {
                    id: 10,
                    default_value: 1,
                    map_ids_csv: String::new(),
                    area_ids_csv: String::new(),
                },
            ]),
            saved_values: PlayerInitialWorldStateRowsLikeCpp::Failed {
                reason: "character read failed".to_owned(),
            },
        },
        PlayerInitialWorldStatesLoadOutcomeLikeCpp {
            templates: PlayerInitialWorldStateRowsLikeCpp::Failed {
                reason: "world read failed".to_owned(),
            },
            saved_values: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![
                PlayerInitialWorldStateValueRowLikeCpp { id: 10, value: 22 },
            ]),
        },
    ]);
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_player_lifecycle_port_like_cpp(port);

    let values_failed = session
        .test_load_initial_world_states_for_login_like_cpp(571, 0)
        .await;
    let templates_failed = session
        .test_load_initial_world_states_for_login_like_cpp(571, 0)
        .await;

    assert!(values_failed.contains(&(10, 1)));
    assert!(!templates_failed.iter().any(|(id, _)| *id == 10));
}
#[test]
fn motd_split_preserves_cpp_empty_and_trailing_lines() {
    assert_eq!(
        motd_lines_like_cpp("first@@third@"),
        vec!["first", "", "third", ""]
    );
}
#[test]
fn invalid_homebind_repair_selects_cpp_create_mode_and_graveyard_order() {
    let normal = PlayerCreatePositionLikeCpp {
        map_id: 0,
        position: Position::new(-8_946.0, -246.0, 59.0, 0.0),
        transport_guid: None,
    };
    let npe = PlayerCreatePositionLikeCpp {
        map_id: 2_175,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        transport_guid: None,
    };
    let info = PlayerCreateInfoLikeCpp {
        create_position: normal,
        create_position_npe: Some(npe),
    };
    assert_eq!(
        first_login_creation_homebind_like_cpp(info, wow_data::PLAYER_CREATE_MODE_NORMAL_LIKE_CPP,),
        Some(CharacterLoginLocationLikeCpp {
            map_id: normal.map_id,
            bind_area_id: None,
            position: normal.position,
        })
    );
    assert_eq!(
        first_login_creation_homebind_like_cpp(info, wow_data::PLAYER_CREATE_MODE_NPE_LIKE_CPP,)
            .map(|homebind| homebind.position),
        Some(npe.position)
    );
    assert_eq!(
        first_login_creation_homebind_like_cpp(
            PlayerCreateInfoLikeCpp {
                create_position_npe: None,
                ..info
            },
            wow_data::PLAYER_CREATE_MODE_NPE_LIKE_CPP,
        )
        .map(|homebind| homebind.position),
        Some(normal.position),
        "C++ falls back to the normal class/race creation position when NPE data is invalid"
    );
    assert_eq!(
        first_login_creation_homebind_like_cpp(
            PlayerCreateInfoLikeCpp {
                create_position_npe: Some(PlayerCreatePositionLikeCpp {
                    transport_guid: Some(29),
                    ..npe
                }),
                ..info
            },
            wow_data::PLAYER_CREATE_MODE_NPE_LIKE_CPP,
        ),
        None,
        "C++ does not bind first-login transport offsets and falls through to graveyard"
    );

    assert_eq!(
        default_graveyard_safe_loc_ids_for_race_like_cpp(1),
        [Some(4), None]
    );
    assert_eq!(
        default_graveyard_safe_loc_ids_for_race_like_cpp(2),
        [Some(10), None]
    );
    assert_eq!(
        default_graveyard_safe_loc_ids_for_race_like_cpp(24),
        [Some(4), Some(3295)]
    );

    let area_store = wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 12,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: 0,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 13,
            continent_id: 0,
            parent_area_id: 12,
            area_bit: 0,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0x4000_0000,
        },
    ]);
    assert_eq!(
        zone_and_area_from_area_id_like_cpp(13, Some(&area_store)),
        (12, 13)
    );

    let scenario_garrison_store = wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 1_151,
        instance_type: wow_data::map::MAP_SCENARIO,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: wow_data::map::MAP_FLAG_GARRISON,
        flags2: 0,
    }]);
    assert!(!usable_character_homebind_like_cpp(
        CharacterLoginLocationLikeCpp {
            map_id: 1_151,
            bind_area_id: Some(12),
            position: Position::ZERO,
        },
        Some(&scenario_garrison_store),
        2,
    ));
}
#[test]
fn default_homebind_reads_primary_then_neutral_pandaren_from_startup_store_like_cpp() {
    fn map(id: u32) -> wow_data::MapEntry {
        wow_data::MapEntry {
            id,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        }
    }
    let maps = wow_data::MapStore::from_entries([map(1), map(870)]);
    let primary = wow_data::WorldSafeLocRow {
        id: 4,
        map_id: 1,
        x: 1.0,
        y: 2.0,
        z: 3.0,
        facing_degrees: 90.0,
    };
    let fallback = wow_data::WorldSafeLocRow {
        id: 3295,
        map_id: 870,
        x: 4.0,
        y: 5.0,
        z: 6.0,
        facing_degrees: 180.0,
    };

    let (mut session, _, _) = make_bank_slot_session(4);
    let (store, report) =
        wow_data::WorldSafeLocStore::from_rows_like_cpp([fallback, primary], &maps);
    assert_eq!(report.loaded, 2);
    session.set_world_safe_loc_store_like_cpp(Arc::new(store));
    assert_eq!(
        session
            .load_default_graveyard_homebind_like_cpp(24)
            .expect("neutral Pandaren uses faction primary first"),
        CharacterLoginLocationLikeCpp {
            map_id: 1,
            bind_area_id: None,
            position: Position::new(1.0, 2.0, 3.0, 90_f32.to_radians()),
        }
    );

    let (fallback_only, report) =
        wow_data::WorldSafeLocStore::from_rows_like_cpp([fallback], &maps);
    assert_eq!(report.loaded, 1);
    session.set_world_safe_loc_store_like_cpp(Arc::new(fallback_only));
    assert_eq!(
        session
            .load_default_graveyard_homebind_like_cpp(24)
            .expect("neutral Pandaren keeps C++ 3295 fallback")
            .map_id,
        870
    );
}
#[test]
fn stat_update_preserves_current_mana_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 77);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana(&mut session, player_guid, 777, 1320);

    let (_, changes) = session
        .player_stat_changes_like_cpp()
        .expect("stat changes");

    assert_eq!(changes.power0, 777);
    assert_eq!(changes.max_power0, 1320);
    assert_eq!(changes.base_mana, 1000);
    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Mana),
        Some((777, 1320))
    );
}
#[test]
fn stat_update_clamps_current_mana_to_new_max_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 78);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana(&mut session, player_guid, 2_000, 2_500);

    let (_, changes) = session
        .player_stat_changes_like_cpp()
        .expect("stat changes");

    assert_eq!(changes.power0, 1320);
    assert_eq!(changes.max_power0, 1320);
    assert_eq!(changes.base_mana, 1000);
    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Mana),
        Some((1320, 1320))
    );
}
#[test]
fn stat_update_preserves_current_health_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 79);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1320, 7, 500);

    let (_, changes) = session
        .player_stat_changes_like_cpp()
        .expect("stat changes");

    assert_eq!(changes.health, 7);
    assert_eq!(changes.max_health, 10);
    assert_eq!(session.player_health_like_cpp(), 7);
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((7, 10))
    );
}
#[test]
fn stat_update_clamps_current_health_to_new_max_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 80);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1320, 500, 500);

    let (_, changes) = session
        .player_stat_changes_like_cpp()
        .expect("stat changes");

    assert_eq!(changes.health, 10);
    assert_eq!(changes.max_health, 10);
    assert_eq!(session.player_health_like_cpp(), 10);
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((10, 10))
    );
}
#[test]
fn total_stat_percentage_non_ability_keeps_current_health_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 83);
    let spell_id = 90_083;
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1_000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1_320, 5, 10);
    session.set_player_health_like_cpp(5, 10);
    session.set_spell_store(Arc::new(total_stat_percentage_spell_store_like_cpp(
        spell_id, false,
    )));
    session.set_state(crate::session::SessionState::LoggedIn);

    session
        .apply_aura(spell_id, player_guid, 30_000, 1)
        .expect("apply non-ability stamina aura");

    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((5, 20)),
        "C++ only preserves health percentage for SPELL_ATTR0_IS_ABILITY"
    );
}
#[test]
fn create_character_seeds_valid_cpp_rest_state_for_raf_roles() {
    assert_eq!(
        initial_character_rest_state_like_cpp(false, 0),
        REST_STATE_NORMAL_LIKE_CPP
    );
    assert_eq!(
        initial_character_rest_state_like_cpp(false, 7),
        REST_STATE_RAF_LINKED_LIKE_CPP
    );
    assert_eq!(
        initial_character_rest_state_like_cpp(true, 0),
        REST_STATE_RAF_LINKED_LIKE_CPP
    );
}
#[test]
fn default_character_power1_seeds_energy_classes_like_cpp() {
    assert_eq!(
        default_character_power1_like_cpp(4, 0),
        100,
        "C++ level-1 rogues enter with full base Energy, not zeroed mana"
    );
    assert_eq!(default_character_power1_like_cpp(5, 160), 160);
    assert_eq!(default_character_power1_like_cpp(1, 0), 0);
}
#[test]
fn character_rename_name_validation_matches_represented_cpp_gates() {
    assert_eq!(
        crate::handlers::character_rules::represented_character_rename_name_result_like_cpp(""),
        CHAR_NAME_NO_NAME_LIKE_CPP
    );
    assert_eq!(
        crate::handlers::character_rules::represented_character_rename_name_result_like_cpp("A"),
        CHAR_NAME_TOO_SHORT_LIKE_CPP
    );
    assert_eq!(
        crate::handlers::character_rules::represented_character_rename_name_result_like_cpp(
            "VeryLongNameX"
        ),
        CHAR_NAME_TOO_LONG_LIKE_CPP
    );
    assert_eq!(
        crate::handlers::character_rules::represented_character_rename_name_result_like_cpp("Bad1"),
        CHAR_NAME_INVALID_CHARACTER_LIKE_CPP
    );
    assert_eq!(
        crate::handlers::character_rules::represented_character_rename_name_result_like_cpp(
            "Newname"
        ),
        RESPONSE_SUCCESS_LIKE_CPP
    );
}
#[tokio::test]
async fn character_rename_invalid_name_sends_cpp_result_without_guid() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let guid = ObjectGuid::create_player(1, 42);
    session.set_legit_characters(vec![guid]);

    session
        .handle_character_rename_request(CharacterRenameRequest {
            guid,
            new_name: String::new(),
        })
        .await;

    let sent = send_rx.try_recv().expect("rename result");
    let mut pkt = WorldPacket::from_bytes(&sent);
    assert_eq!(
        pkt.server_opcode(),
        Some(ServerOpcodes::CharacterRenameResult)
    );
    pkt.skip_opcode();
    assert_eq!(pkt.read_uint8().unwrap(), CHAR_NAME_NO_NAME_LIKE_CPP);
    assert!(!pkt.read_bit().unwrap());
    assert_eq!(pkt.read_bits(6).unwrap(), 0);
    assert_eq!(pkt.remaining(), 0);
}
#[tokio::test]
async fn character_rename_non_owned_guid_kicks_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_character_rename_request(CharacterRenameRequest {
            guid: ObjectGuid::create_player(1, 42),
            new_name: "Newname".to_string(),
        })
        .await;

    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn char_customize_without_character_db_sends_cpp_failure() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let guid = ObjectGuid::create_player(1, 42);
    session.set_legit_characters(vec![guid]);

    session
        .handle_char_customize(CharCustomize {
            guid,
            sex_id: 1,
            customizations: vec![],
            name: "Newname".to_string(),
        })
        .await;

    let sent = send_rx.try_recv().expect("customize failure");
    let mut pkt = WorldPacket::from_bytes(&sent);
    assert_eq!(
        pkt.server_opcode(),
        Some(ServerOpcodes::CharCustomizeFailure)
    );
    pkt.skip_opcode();
    assert_eq!(pkt.read_uint8().unwrap(), CHAR_CREATE_ERROR_LIKE_CPP);
    assert_eq!(pkt.read_guid().unwrap(), guid);
    assert_eq!(pkt.remaining(), 0);
}
#[tokio::test]
async fn char_customize_non_owned_guid_kicks_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_char_customize(CharCustomize {
            guid: ObjectGuid::create_player(1, 42),
            sex_id: 1,
            customizations: vec![],
            name: "Newname".to_string(),
        })
        .await;

    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn alter_appearance_without_barber_chair_sends_not_on_chair_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);

    session
        .handle_alter_appearance(alter_appearance_packet(1, 1, 0, &[(20, 200)]))
        .await;

    assert_eq!(
        read_barber_shop_result(send_rx.try_recv().unwrap()),
        BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP
    );
    assert!(
        session
            .represented_alter_appearance_requests_like_cpp()
            .is_empty()
    );
}
#[tokio::test]
async fn set_player_declined_names_without_runtime_sends_error_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let player = ObjectGuid::create_player(1, 42);

    session
        .handle_set_player_declined_names(declined_names_packet(
            player,
            ["Gen", "Dat", "Acc", "Inst", "Prep"],
        ))
        .await;

    assert_eq!(
        read_declined_names_result(send_rx.try_recv().unwrap()),
        (DECLINED_NAMES_RESULT_ERROR_LIKE_CPP, player)
    );
}
#[tokio::test]
async fn set_player_declined_names_short_packet_does_not_send_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_set_player_declined_names(WorldPacket::from_bytes(&[0x2a, 0x00]))
        .await;

    assert!(send_rx.try_recv().is_err());
}
#[test]
fn child_redirect_validates_both_steps_without_mutating_runtime_like_upstream_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let entry_id = 121;
    session.set_player_guid(Some(player_guid));
    install_equippable_item_fixture(&mut session, entry_id, InventoryType::Weapon, None);

    let source_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        entry_id,
        71,
        InventoryType::Weapon,
    );
    let parent_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
        entry_id,
        72,
        InventoryType::Weapon,
    );
    let child_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        entry_id,
        73,
        InventoryType::Weapon,
    );
    session.update_inventory_item_object_like_cpp(child_guid, |child| {
        child.set_item_flag(ItemFieldFlags::CHILD);
        child.set_creator(parent_guid);
    });

    let source = wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let destination = wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND);
    let redirect = session
        .plan_inventory_swap_preflight_like_cpp(source, destination)
        .expect("player inventory preflight");
    let SwapItemPreflightResult::ChildRedirect {
        first_src,
        first_dst,
        second_src,
        second_dst,
    } = redirect.result
    else {
        panic!("equipped child destination must redirect through its parent");
    };

    assert_eq!(
        session
            .plan_inventory_child_redirect_like_cpp(
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_MAINHAND,
                first_src,
                first_dst,
                second_src,
                second_dst,
            )
            .expect("both redirected moves are legal"),
        CHILD_EQUIPMENT_SLOT_START
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .map(|item| item.guid),
        Some(child_guid),
        "the validation overlay must restore the visible child slot"
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(source_guid)
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, wow_entities::EQUIPMENT_SLOT_OFFHAND,)
            .map(|item| item.guid),
        Some(parent_guid)
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, CHILD_EQUIPMENT_SLOT_START)
            .is_none()
    );
}
#[test]
fn rejected_child_redirect_restores_validation_overlay_before_error_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let entry_id = 122;
    session.set_player_guid(Some(player_guid));
    session.set_player_alive_like_cpp(false);
    install_equippable_item_fixture(&mut session, entry_id, InventoryType::Weapon, None);

    let source_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        entry_id,
        74,
        InventoryType::Weapon,
    );
    let parent_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
        entry_id,
        75,
        InventoryType::Weapon,
    );
    let child_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        entry_id,
        76,
        InventoryType::Weapon,
    );
    session.update_inventory_item_object_like_cpp(child_guid, |child| {
        child.set_item_flag(ItemFieldFlags::CHILD);
        child.set_creator(parent_guid);
    });

    let source = wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let destination = wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND);
    let redirect = session
        .plan_inventory_swap_preflight_like_cpp(source, destination)
        .expect("player inventory preflight");
    let SwapItemPreflightResult::ChildRedirect {
        first_src,
        first_dst,
        second_src,
        second_dst,
    } = redirect.result
    else {
        panic!("C++ examines child redirects before the player-dead gate");
    };

    assert_eq!(
        session.plan_inventory_child_redirect_like_cpp(
            INVENTORY_SLOT_BAG_0,
            EQUIPMENT_SLOT_MAINHAND,
            first_src,
            first_dst,
            second_src,
            second_dst,
        ),
        Err(InventoryResult::PlayerDead)
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .map(|item| item.guid),
        Some(child_guid),
        "a rejected redirected move must not persist the child in a hidden slot"
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(source_guid)
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, wow_entities::EQUIPMENT_SLOT_OFFHAND,)
            .map(|item| item.guid),
        Some(parent_guid)
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, CHILD_EQUIPMENT_SLOT_START)
            .is_none()
    );
}
#[test]
fn recursive_destroy_descendants_are_deepest_first_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let parent_guid = ObjectGuid::create_item(1, 90);
    let child_bag_guid = ObjectGuid::create_item(1, 91);
    let leaf_guid = ObjectGuid::create_item(1, 92);
    session.set_player_guid(Some(player_guid));

    let parent = session.make_inventory_item_object(
        parent_guid,
        600,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    session.insert_inventory_item_object(parent);
    let mut child_bag = session.make_inventory_item_object(
        child_bag_guid,
        601,
        player_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    child_bag.set_container_guid_and_slot(parent_guid, INVENTORY_SLOT_ITEM_START);
    session.insert_inventory_item_object(child_bag);
    let mut leaf =
        session.make_inventory_item_object(leaf_guid, 700, player_guid, 1, 0, ItemContext::None, 0);
    leaf.set_container_guid_and_slot(child_bag_guid, 0);
    session.insert_inventory_item_object(leaf);

    let descendants = session
        .represented_inventory_descendants_postorder_like_cpp(parent_guid)
        .expect("fixture canonical inventory owner");
    assert_eq!(
        descendants
            .iter()
            .map(|(_, _, item)| item.guid)
            .collect::<Vec<_>>(),
        vec![leaf_guid, child_bag_guid]
    );
}
#[tokio::test]
async fn spirit_healer_activate_without_interactable_healer_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let healer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9, 1);
    let mut request = WorldPacket::new_empty();
    request.write_packed_guid(&healer);

    session.handle_spirit_healer_activate(request).await;

    assert!(send_rx.try_recv().is_err());
}
