//! Original loot application cases with explicit operation-local fixture permission.
use super::interaction_support::*;

#[tokio::test]
async fn loot_release_accepts_secondary_active_owner_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let primary_guid = test_creature_guid(19_029);
    let secondary_guid = test_creature_guid(19_030);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, primary_guid);
    add_active_loot_view_owner_for_test(&mut session, secondary_guid);
    set_loot_for_test(&mut session, 
        secondary_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(secondary_guid),
            coins: 5,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    release_local_loot_for_test(&mut session, secondary_guid, player_guid)
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), secondary_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(is_active_loot_guid_for_test(&session, primary_guid));
    assert!(active_loot_view_owners_for_test(&session).contains(&primary_guid));
    assert!(!active_loot_view_owners_for_test(&session).contains(&secondary_guid));
    assert!(has_loot_for_test(&session, secondary_guid));
}

#[tokio::test]
async fn loot_release_fishing_gameobjects_follow_cpp_state_branches() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let fishing_node = test_gameobject_guid(19_033);
    let fishing_hole = test_gameobject_guid(19_034);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    set_active_loot_guid_for_test(&mut session, fishing_node);
    add_active_loot_view_owner_for_test(&mut session, fishing_hole);
    for (guid, go_type, loot_type) in [
        (
            fishing_node,
            GAMEOBJECT_TYPE_FISHING_NODE as u8,
            LOOT_TYPE_FISHING_LIKE_CPP,
        ),
        (
            fishing_hole,
            GAMEOBJECT_TYPE_FISHING_HOLE as u8,
            LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
        ),
    ] {
        record_represented_gameobject_runtime_state_for_test(&mut session, 
            0,
            guid,
            guid.entry(),
            Position::ZERO,
            go_type,
        );
        set_loot_for_test(&mut session, 
            guid,
            CreatureLoot {
                loot_guid: guid,
                coins: 0,
                unlooted_count: 1,
                loot_type,
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
    }

    release_local_loot_for_test(&mut session, fishing_node, player_guid)
        .await;
    release_local_loot_for_test(&mut session, fishing_hole, player_guid)
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(send_rx.try_recv().is_ok());
    assert_eq!(
        gameobject_loot_release_snapshot_for_test(&session, fishing_node)
            .unwrap()
            .loot_state,
        Some(LootState::JustDeactivated)
    );
    let hole_state = gameobject_loot_release_snapshot_for_test(&session, fishing_hole)
        .unwrap();
    assert_eq!(hole_state.loot_state, Some(LootState::Ready));
    assert_eq!(hole_state.personal_loot_uses, 1);
}

#[tokio::test]
async fn loot_release_personal_chest_records_per_player_despawn_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let restocked_chest = test_gameobject_guid(19_039);
    let fallback_chest = test_gameobject_guid(19_040);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    set_active_loot_guid_for_test(&mut session, restocked_chest);
    add_active_loot_view_owner_for_test(&mut session, fallback_chest);
    wow_world::test_fixtures::insert_client_visible_guid_for_test(&mut session, restocked_chest);
    wow_world::test_fixtures::insert_client_visible_guid_for_test(&mut session, fallback_chest);

    for (guid, restock_time) in [(restocked_chest, 45), (fallback_chest, 0)] {
        record_represented_gameobject_runtime_state_for_test(&mut session, 
            0,
            guid,
            guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        record_gameobject_chest_release_metadata_for_loot_test(&mut session, 
            guid,
            GameObjectLootSource {
                personal_loot_id: 7_001,
                chest_restock_time_secs: restock_time,
                chest_consumable: false,
                ..Default::default()
            },
        );
        set_loot_for_test(&mut session, 
            guid,
            CreatureLoot {
                loot_guid: guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            },
        );
    }

    release_local_loot_for_test(&mut session, restocked_chest, player_guid)
        .await;
    release_local_loot_for_test(&mut session, fallback_chest, player_guid)
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(send_rx.try_recv().is_ok());
    assert!(send_rx.try_recv().is_ok());
    assert!(send_rx.try_recv().is_ok());
    assert!(
        !loot_committed_visibility_for_test(&session).contains(&restocked_chest)
    );
    assert!(
        !loot_committed_visibility_for_test(&session).contains(&fallback_chest)
    );
    assert_eq!(
        gameobject_loot_release_snapshot_for_test(&session, restocked_chest)
            .unwrap()
            .per_player_despawn_secs,
        Some(45)
    );
    assert_eq!(
        gameobject_loot_release_snapshot_for_test(&session, fallback_chest)
            .unwrap()
            .per_player_despawn_secs,
        Some(wow_entities::DEFAULT_GAMEOBJECT_RESPAWN_DELAY_SECS)
    );
    assert!(
        gameobject_loot_release_snapshot_for_test(&session, restocked_chest)
            .unwrap()
            .per_player_despawn_until
            .is_some()
    );
    assert!(is_per_player_gameobject_despawned_for_loot_test(&mut session, restocked_chest));
}

#[tokio::test]
async fn loot_release_personal_chest_without_have_at_client_sends_no_out_of_range_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let chest_guid = test_gameobject_guid(19_137);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    set_active_loot_guid_for_test(&mut session, chest_guid);
    record_represented_gameobject_runtime_state_for_test(&mut session, 
        0,
        chest_guid,
        chest_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    record_gameobject_chest_release_metadata_for_loot_test(&mut session, 
        chest_guid,
        GameObjectLootSource {
            personal_loot_id: 7_001,
            chest_restock_time_secs: 45,
            chest_consumable: false,
            ..Default::default()
        },
    );
    set_loot_for_test(&mut session, 
        chest_guid,
        CreatureLoot {
            loot_guid: chest_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    release_local_loot_for_test(&mut session, chest_guid, player_guid)
        .await;

    let release_bytes = send_rx.try_recv().unwrap();
    let mut release = WorldPacket::from_bytes(&release_bytes);
    assert_eq!(
        release.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert!(send_rx.try_recv().is_err());
    let state = gameobject_loot_release_snapshot_for_test(&session, chest_guid)
        .unwrap();
    assert_eq!(state.per_player_despawn_secs, Some(45));
    assert!(state.per_player_despawn_until.is_some());
    assert_eq!(state.per_player_state_player_guid, Some(player_guid));
}

#[tokio::test]
async fn loot_release_shared_chest_restock_starts_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let partial_chest = test_gameobject_guid(19_041);
    let full_chest = test_gameobject_guid(19_042);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, partial_chest);
    add_active_loot_view_owner_for_test(&mut session, full_chest);

    for guid in [partial_chest, full_chest] {
        record_represented_gameobject_runtime_state_for_test(&mut session, 
            0,
            guid,
            guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        record_gameobject_chest_release_metadata_for_loot_test(&mut session, 
            guid,
            GameObjectLootSource {
                loot_id: 7_001,
                chest_restock_time_secs: 45,
                chest_consumable: false,
                ..Default::default()
            },
        );
    }
    set_loot_for_test(&mut session, 
        partial_chest,
        CreatureLoot {
            loot_guid: partial_chest,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: Vec::new(),
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );
    set_loot_for_test(&mut session, 
        full_chest,
        CreatureLoot {
            loot_guid: full_chest,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    release_local_loot_for_test(&mut session, partial_chest, player_guid)
        .await;
    release_local_loot_for_test(&mut session, full_chest, player_guid)
        .await;

    let partial_state = gameobject_loot_release_snapshot_for_test(&session, partial_chest)
        .unwrap();
    assert_eq!(partial_state.loot_state, Some(LootState::Activated));
    assert!(partial_state.chest_restock_until.is_some());
    assert!(has_loot_for_test(&session, partial_chest));

    let full_state = gameobject_loot_release_snapshot_for_test(&session, full_chest)
        .unwrap();
    assert_eq!(full_state.loot_state, Some(LootState::NotReady));
    assert!(full_state.chest_restock_until.is_some());
    assert!(!has_loot_for_test(&session, full_chest));
}

#[tokio::test]
async fn loot_release_fishing_hole_just_deactivates_at_max_opens_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let fishing_hole = test_gameobject_guid(19_037);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    set_active_loot_guid_for_test(&mut session, fishing_hole);
    record_represented_gameobject_runtime_state_for_test(&mut session, 
        0,
        fishing_hole,
        fishing_hole.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    record_fishing_hole_max_opens_for_loot_test(&mut session, fishing_hole, 1);
    set_loot_for_test(&mut session, 
        fishing_hole,
        CreatureLoot {
            loot_guid: fishing_hole,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
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

    release_local_loot_for_test(&mut session, fishing_hole, player_guid)
        .await;

    assert!(send_rx.try_recv().is_ok());
    let hole_state = gameobject_loot_release_snapshot_for_test(&session, fishing_hole)
        .unwrap();
    assert_eq!(hole_state.personal_loot_uses, 1);
    assert_eq!(hole_state.loot_state, Some(LootState::JustDeactivated));
}

#[tokio::test]
async fn loot_release_gathering_node_sets_local_active_state_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let gathering_node = test_gameobject_guid(19_038);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    let game_object = make_canonical_gameobject_for_session(
        &session,
        gathering_node,
        GAMEOBJECT_TYPE_GATHERING_NODE as u8,
    );
    attach_canonical_gameobject(&mut session, game_object);
    set_active_loot_guid_for_test(&mut session, gathering_node);
    wow_world::test_fixtures::insert_client_visible_guid_for_test(&mut session, gathering_node);
    record_represented_gameobject_runtime_state_for_test(&mut session, 
        0,
        gathering_node,
        gathering_node.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_GATHERING_NODE as u8,
    );
    set_loot_for_test(&mut session, 
        gathering_node,
        CreatureLoot {
            loot_guid: gathering_node,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_GATHERING_NODE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    // Prepare the same fresh canonical authority formerly installed by cfg(test).
    refresh_loot_summary_for_test(&mut session, gathering_node, player_guid);
    let authority = loot_recovery_authority_for_test(&mut session, gathering_node).unwrap();
    let generation = authority.snapshot_for_player_like_cpp(player_guid).unwrap().generation;
    bind_loot_view_for_test(&mut session, gathering_node, generation, &authority);

    release_local_loot_for_test(&mut session, gathering_node, player_guid)
        .await;

    let release_bytes = send_rx.try_recv().unwrap();
    let mut release = WorldPacket::from_bytes(&release_bytes);
    assert_eq!(
        release.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    let state = gameobject_loot_release_snapshot_for_test(&session, gathering_node)
        .unwrap();
    assert_eq!(state.go_state, Some(GoState::Active));
    assert_eq!(state.loot_state, None);
    let expected = wow_packet::packets::update::UpdateObject::game_object_values_update(
        gathering_node,
        571,
        wow_packet::packets::update::GameObjectDataValuesUpdate {
            changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
            object_data: Some(wow_packet::packets::update::ObjectDataValuesUpdate {
                changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
                object_data_mask: 0x05,
                entry_id: 0,
                dynamic_flags: wow_entities::GO_DYNFLAG_LO_DEPLETED,
                scale: 0.0,
            }),
            game_object_data_mask: 0,
            state_world_effect_ids: Vec::new(),
            enable_doodad_sets: Vec::new(),
            enable_doodad_sets_update_mask: None,
            world_effects: Vec::new(),
            world_effects_update_mask: None,
            display_id: 0,
            spell_visual_id: 0,
            state_spell_visual_id: 0,
            spawn_tracking_state_anim_id: 0,
            spawn_tracking_state_anim_kit_id: 0,
            created_by: ObjectGuid::EMPTY,
            guild_guid: ObjectGuid::EMPTY,
            flags: 0,
            parent_rotation: [0.0; 4],
            faction_template: 0,
            level: 0,
            state: 0,
            type_id: 0,
            percent_health: 0,
            art_kit: 0,
            custom_param: 0,
        },
    )
    .to_bytes();
    assert_eq!(send_rx.try_recv().unwrap(), expected);
}

