//! Loot scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn loot_release_accepts_secondary_active_owner_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let primary_guid = test_creature_guid(19_029);
    let secondary_guid = test_creature_guid(19_030);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(primary_guid);
    session.add_active_loot_view_owner_like_cpp(secondary_guid);
    session.loot_table.insert(
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

    session
        .handle_loot_release(loot_release_packet(secondary_guid))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), secondary_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(session.is_active_loot_guid(primary_guid));
    assert!(session.active_loot_view_owners.contains(&primary_guid));
    assert!(!session.active_loot_view_owners.contains(&secondary_guid));
    assert!(session.loot_table.contains_key(&secondary_guid));
}
#[test]
fn looted_corpse_decay_uses_cpp_rate_and_ignore_flag() {
    assert_eq!(
        looted_corpse_decay_secs_like_cpp(false, 120, false, 0.5),
        60
    );
    assert_eq!(
        looted_corpse_decay_secs_like_cpp(false, 120, true, 0.5),
        120
    );
    assert_eq!(
        looted_corpse_decay_secs_like_cpp(false, 120, false, -1.0),
        0
    );
    assert_eq!(looted_corpse_decay_secs_like_cpp(true, 120, false, 0.5), 0);
}
#[tokio::test]
async fn player_corpse_loot_release_removes_corpse_lootable_dynflag_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let corpse_guid = test_corpse_guid(19_117);
    let corpse = make_canonical_corpse_for_session(&session, corpse_guid);
    attach_canonical_corpse(&mut session, corpse);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(corpse_guid);
    session.loot_table.insert(
        corpse_guid,
        CreatureLoot {
            loot_guid: corpse_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_INSIGNIA_LIKE_CPP,
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
    assert_eq!(
        canonical_corpse_snapshot(&session, corpse_guid)
            .unwrap()
            .data()
            .dynamic_flags
            & CORPSE_DYNFLAG_LOOTABLE,
        CORPSE_DYNFLAG_LOOTABLE
    );

    session
        .handle_loot_release(loot_release_packet(corpse_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let corpse = canonical_corpse_snapshot(&session, corpse_guid).unwrap();
    assert_eq!(
        corpse.data().dynamic_flags & CORPSE_DYNFLAG_LOOTABLE,
        0,
        "C++ DoLootRelease removes CORPSE_DYNFLAG_LOOTABLE from fully looted player corpses"
    );
    assert!(
        corpse
            .corpse_data_changes_mask()
            .is_set(wow_entities::CORPSE_DATA_DYNAMIC_FLAGS_BIT)
    );
}
#[tokio::test]
async fn loot_release_fishing_hole_just_deactivates_at_max_opens_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let fishing_hole = test_gameobject_guid(19_037);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(fishing_hole);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        fishing_hole,
        fishing_hole.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    session.record_represented_fishing_hole_max_opens_like_cpp(fishing_hole, 1);
    session.loot_table.insert(
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

    session
        .handle_loot_release(loot_release_packet(fishing_hole))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let hole_state = session
        .represented_gameobject_use_states
        .get(&fishing_hole)
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
    session.set_active_loot_guid(gathering_node);
    session.client_visible_guids_like_cpp.insert(gathering_node);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        gathering_node,
        gathering_node.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_GATHERING_NODE as u8,
    );
    session.loot_table.insert(
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

    session
        .handle_loot_release(loot_release_packet(gathering_node))
        .await;

    let release_bytes = send_rx.try_recv().unwrap();
    let mut release = WorldPacket::from_bytes(&release_bytes);
    assert_eq!(
        release.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    let state = session
        .represented_gameobject_use_states
        .get(&gathering_node)
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
#[test]
fn partial_gathering_node_release_does_not_run_on_loot_release_state_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_900);
    let gathering_node = test_gameobject_guid(61_901);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        gathering_node,
        gathering_node.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_GATHERING_NODE as u8,
    );

    session.apply_represented_gameobject_loot_release_like_cpp(
        gathering_node,
        player_guid,
        false,
        false,
        None,
    );

    let state = session
        .represented_gameobject_use_states
        .get(&gathering_node)
        .unwrap();
    assert_ne!(state.go_state, Some(GoState::Active));
    assert_eq!(state.loot_state, Some(LootState::Activated));
}
