//! Object update-block regressions, part 1 of 5.
//!
//! Moved out of the update_tests.rs root under #640; every test is unchanged.

use super::*;

#[test]
fn movement_create_block_flushes_cpp_eight_subbits_before_speeds_for_54261() {
    let mv = MovementBlock {
        position: Position::ZERO,
        movement_flags: 0,
        movement_flags2: 0,
        movement_flags3: 0,
        transport: None,
        create_object_spline: None,
        walk_speed: 1.0,
        run_speed: 2.0,
        run_back_speed: 3.0,
        swim_speed: 4.0,
        swim_back_speed: 5.0,
        fly_speed: 6.0,
        fly_back_speed: 7.0,
        turn_rate: 8.0,
        pitch_rate: 9.0,
    };

    let mut buf = WorldPacket::new_empty();
    write_movement_update(&mut buf, &ObjectGuid::EMPTY, &mv);
    let bytes = buf.data();

    // EMPTY packed GUID = 2 bytes, then movement flags/flags2/extra2,
    // time, position, pitch, step elevation, remove-forces count, move index.
    const HEADER_BEFORE_SUBBITS: usize = 2 + 12 + 4 + 16 + 4 + 4 + 4 + 4;
    const SPEEDS_OFFSET: usize = HEADER_BEFORE_SUBBITS + 1;

    assert_eq!(
        &bytes[HEADER_BEFORE_SUBBITS..SPEEDS_OFFSET],
        &[0x00],
        "C++ 3.4.3 movement create writes eight false sub-bits before speeds"
    );

    let read_f32 = |off: usize| f32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
    for (index, expected) in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            read_f32(SPEEDS_OFFSET + index * 4),
            expected,
            "movement speed {index} is not aligned after the 54261 sub-bit block"
        );
    }

    assert_ne!(
        read_f32(HEADER_BEFORE_SUBBITS + 2),
        1.0,
        "speed block shifted to the old Rust-only nine-sub-bit boundary"
    );
}

#[test]
fn movement_create_block_serializes_nested_transport_like_cpp() {
    let transport_guid = ObjectGuid::create_transport(wow_core::guid::HighGuid::Transport, 7_001);
    let mv = MovementBlock {
        position: Position::new(100.0, 200.0, 300.0, 1.5),
        transport: Some(Box::new(TransportInfo {
            guid: transport_guid,
            x: 1.25,
            y: -2.5,
            z: 3.75,
            o: 0.5,
            seat: -1,
            time: 42,
            prev_time: None,
            vehicle_id: None,
        })),
        ..Default::default()
    };

    let mut bytes = WorldPacket::new_empty();
    write_movement_update(&mut bytes, &ObjectGuid::EMPTY, &mv);
    let mut reader = WorldPacket::from_bytes(bytes.data());
    let decoded = crate::packets::movement::MovementInfo::read(&mut reader)
        .expect("C++ nested MovementInfo::TransportInfo");
    let transport = decoded.transport.expect("HasTransport");

    assert_eq!(transport.guid, transport_guid);
    assert_eq!(
        (transport.x, transport.y, transport.z, transport.o),
        (1.25, -2.5, 3.75, 0.5)
    );
    assert_eq!(transport.seat, -1);
    assert_eq!(transport.time, 42);
    assert_eq!(transport.prev_time, None);
    assert_eq!(transport.vehicle_id, None);
}

#[test]
fn movement_create_block_serializes_creature_hover_flag_like_cpp() {
    let mv = MovementBlock {
        position: Position::ZERO,
        movement_flags: wow_constants::movement::MovementFlag::HOVER.bits(),
        movement_flags2: 0,
        movement_flags3: 0,
        transport: None,
        create_object_spline: None,
        walk_speed: 1.0,
        run_speed: 2.0,
        run_back_speed: 3.0,
        swim_speed: 4.0,
        swim_back_speed: 5.0,
        fly_speed: 6.0,
        fly_back_speed: 7.0,
        turn_rate: 8.0,
        pitch_rate: 9.0,
    };

    let mut buf = WorldPacket::new_empty();
    write_movement_update(&mut buf, &ObjectGuid::EMPTY, &mv);
    let bytes = buf.data();

    // EMPTY packed GUID is 2 bytes; C++ then writes MovementFlags,
    // MovementFlags2 and ExtraMovementFlags2 as three u32 values.
    assert_eq!(
        &bytes[2..6],
        &wow_constants::movement::MovementFlag::HOVER
            .bits()
            .to_le_bytes(),
        "C++ Creature::LoadCreaturesAddon preserves MOVEMENTFLAG_HOVER in BuildMovementUpdate"
    );
    assert_eq!(&bytes[6..10], &[0, 0, 0, 0]);
    assert_eq!(&bytes[10..14], &[0, 0, 0, 0]);
}

#[test]
fn gameobject_create_values_serializes_created_by_guid_like_cpp() {
    let base = GameObjectCreateData {
        guid: ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::GameObject,
            0,
            1,
            0,
            0,
            123,
            456,
        ),
        entry: 123,
        dynamic_flags: 0,
        display_id: 456,
        go_type: 3,
        position: Position::ZERO,
        rotation: [0.0, 0.0, 0.0, 1.0],
        anim_progress: 255,
        state: 1,
        art_kit: 0,
        created_by: ObjectGuid::EMPTY,
        faction_template: 0,
        gameobject_flags: 0,
        world_effect_id: 0,
        scale: 1.0,
        level: 0,
        parent_rotation: [0.0, 0.0, 0.0, 1.0],
    };

    let mut empty_owner_packet = WorldPacket::new_empty();
    base.write_values_create(&mut empty_owner_packet);

    let mut owned = base;
    owned.created_by = ObjectGuid::create_player(1, 42);
    let mut owned_packet = WorldPacket::new_empty();
    owned.write_values_create(&mut owned_packet);

    assert!(owned_packet.data().len() > empty_owner_packet.data().len());
    assert_ne!(owned_packet.data(), empty_owner_packet.data());
}

#[test]
fn gameobject_create_values_serializes_level_period_for_transport_like_cpp() {
    // Regression for the world-entry ERROR #132 client crash: a MO_TRANSPORT must carry
    // its path period in GameObjectData::Level (C++ Transport::Create -> SetPeriod ->
    // GameObjectData::Level; Transport.h:89). Level=0 made the 3.4.3 client divide
    // PathProgress by a zero period -> 0xFFFF path-node index -> render-worker NULL deref.
    let mut data = GameObjectCreateData {
        guid: ObjectGuid::create_transport(wow_core::guid::HighGuid::Transport, 7),
        entry: 181688,
        dynamic_flags: 0,
        display_id: 3015,
        go_type: 15, // MO_TRANSPORT
        position: Position::ZERO,
        rotation: [0.0, 0.0, 0.0, 1.0],
        anim_progress: 255,
        state: 1,
        art_kit: 0,
        created_by: ObjectGuid::EMPTY,
        faction_template: 0,
        gameobject_flags: 0,
        world_effect_id: 0,
        scale: 1.0,
        level: 0x0011_2233, // distinctive period
        parent_rotation: [0.0, 0.0, 0.0, 1.0],
    };
    let mut pkt = WorldPacket::new_empty();
    data.write_values_create(&mut pkt);
    let bytes = pkt.into_data();
    assert!(
        bytes.windows(4).any(|w| w == 0x0011_2233u32.to_le_bytes()),
        "MO_TRANSPORT Level (path period) must be serialized in GameObjectData"
    );
    // A zero period must NOT silently survive: changing level changes the wire.
    data.level = 0;
    let mut pkt0 = WorldPacket::new_empty();
    data.write_values_create(&mut pkt0);
    assert_ne!(bytes, pkt0.into_data());
}

#[test]
fn gameobject_create_values_serializes_parent_rotation_like_cpp() {
    // C++ GameObjectData::ParentRotation is sourced from per-spawn gameobject_addon
    // (GameObject::Create, GameObject.cpp:1003-1008) — distinct from the local rotation.
    // #NEXT.R8.ENTITIES.1216: a non-identity parent rotation must reach the wire instead
    // of being hardcoded to identity.
    let mut data = GameObjectCreateData {
        guid: ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::GameObject,
            0,
            1,
            571,
            1,
            195821,
            42,
        ),
        entry: 195821,
        dynamic_flags: 0,
        display_id: 8112,
        go_type: 10, // GENERIC (uses write_values_create, not the transport block)
        position: Position::ZERO,
        rotation: [0.0, 0.0, 0.0, 1.0],
        anim_progress: 255,
        state: 1,
        art_kit: 0,
        created_by: ObjectGuid::EMPTY,
        faction_template: 0,
        gameobject_flags: 0,
        world_effect_id: 0,
        scale: 1.0,
        level: 0,
        parent_rotation: [0.25, 0.5, 0.75, 0.125],
    };
    let mut pkt = WorldPacket::new_empty();
    data.write_values_create(&mut pkt);
    let bytes = pkt.into_data();
    for component in data.parent_rotation {
        assert!(
            bytes.windows(4).any(|w| w == component.to_le_bytes()),
            "GameObjectData::ParentRotation component {component} must be serialized"
        );
    }
    // Identity must produce a different wire — the field is not a no-op.
    data.parent_rotation = [0.0, 0.0, 0.0, 1.0];
    let mut identity_pkt = WorldPacket::new_empty();
    data.write_values_create(&mut identity_pkt);
    assert_ne!(bytes, identity_pkt.into_data());
}

#[test]
fn gameobject_create_values_serializes_flags_and_faction_template_like_cpp() {
    let create = GameObjectCreateData {
        guid: ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::GameObject,
            0,
            1,
            0,
            0,
            123,
            456,
        ),
        entry: 123,
        dynamic_flags: 0x44,
        display_id: 456,
        go_type: 3,
        position: Position::ZERO,
        rotation: [0.0, 0.0, 0.0, 1.0],
        anim_progress: 255,
        state: 1,
        art_kit: 0x5566_7788,
        created_by: ObjectGuid::EMPTY,
        faction_template: 1735,
        gameobject_flags: 0x20,
        world_effect_id: 0,
        scale: 1.0,
        level: 0,
        parent_rotation: [0.0, 0.0, 0.0, 1.0],
    };

    let mut packet = WorldPacket::new_empty();
    create.write_values_create(&mut packet);
    let data = packet.data();
    assert!(
        data.windows(4)
            .any(|window| window == 1735i32.to_le_bytes())
    );
    assert!(
        data.windows(4)
            .any(|window| window == 0x20u32.to_le_bytes())
    );
    assert!(
        data.windows(4)
            .any(|window| window == 0x44u32.to_le_bytes())
    );
    assert!(
        data.windows(4)
            .any(|window| window == 0x5566_7788u32.to_le_bytes()),
        "C++ GameObjectData::ArtKit must be serialized in CREATE values"
    );
}

#[test]
fn gameobject_create_omits_gameobject_payload_without_world_effect_like_cpp() {
    let guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::GameObject,
        0,
        1,
        571,
        0,
        179976,
        0x184,
    );
    let create = GameObjectCreateData {
        guid,
        entry: 179976,
        dynamic_flags: 0,
        display_id: 123,
        go_type: 5,
        position: Position::ZERO,
        rotation: [0.0, 0.0, 0.0, 1.0],
        anim_progress: 255,
        state: 1,
        art_kit: 0,
        created_by: ObjectGuid::EMPTY,
        faction_template: 0,
        gameobject_flags: 0,
        world_effect_id: 0,
        scale: 1.0,
        level: 0,
        parent_rotation: [0.0, 0.0, 0.0, 1.0],
    };

    let mut block = WorldPacket::new_empty();
    write_gameobject_create_block(&mut block, UpdateType::CreateObject, &guid, &create);
    let block_bytes = block.data().len();
    let values_bytes = debug_gameobject_create_values_len_like_cpp(&create);
    let movement_bytes = block_bytes
        - debug_create_header_len_like_cpp(UpdateType::CreateObject, &guid, TypeId::GameObject)
        - values_bytes;

    assert_eq!(
        movement_bytes, 31,
        "C++ GameObject constructor only sets Stationary+Rotation by default"
    );

    let mut with_world_effect = create.clone();
    with_world_effect.world_effect_id = 77;
    let mut block = WorldPacket::new_empty();
    write_gameobject_create_block(
        &mut block,
        UpdateType::CreateObject,
        &guid,
        &with_world_effect,
    );
    let block_bytes = block.data().len();
    let values_bytes = debug_gameobject_create_values_len_like_cpp(&with_world_effect);
    let movement_bytes = block_bytes
        - debug_create_header_len_like_cpp(UpdateType::CreateObject, &guid, TypeId::GameObject)
        - values_bytes;

    assert_eq!(
        movement_bytes, 36,
        "C++ writes WorldEffectID plus one false bit only when CreateObjectBits::GameObject is set"
    );
}

#[test]
fn dynamic_object_create_block_serializes_stationary_create_values_like_cpp() {
    let guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::DynamicObject,
        0,
        1,
        571,
        0,
        7001,
        9001,
    );
    let caster = ObjectGuid::create_player(1, 42);
    let position = Position::new(11.0, 22.0, 33.0, 1.5);
    let pkt = UpdateObject::create_world_objects(
        vec![UpdateObject::create_dynamic_object_block(
            DynamicObjectCreateData {
                guid,
                entry_id: 7001,
                dynamic_flags: 0,
                scale: 1.0,
                position,
                caster,
                dynamic_object_type: 2,
                spell_visual_id: 456,
                spell_id: 777,
                radius: 12.5,
                cast_time_ms: 12345,
            },
        )],
        571,
    );

    let bytes = pkt.to_bytes();
    assert_eq!(
        u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
        1
    );
    assert!(
        bytes
            .windows(1)
            .any(|window| window == [UpdateType::CreateObject2 as u8])
    );
    assert!(
        bytes
            .windows(1)
            .any(|window| window == [TypeId::DynamicObject as u8])
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == position.x.to_le_bytes())
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == position.y.to_le_bytes())
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == position.z.to_le_bytes())
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == position.orientation.to_le_bytes())
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == 7001i32.to_le_bytes())
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == 456i32.to_le_bytes())
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == 777i32.to_le_bytes())
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == 12.5f32.to_le_bytes())
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == 12345u32.to_le_bytes())
    );
    assert!(!bytes.windows(1).all(|window| window == [0]));
}

#[test]
fn corpse_create_block_matches_cpp_stationary_and_values_create_shape() {
    let mut items = [0; 19];
    items[0] = 0xAABB_CCDD;
    let create_data = CorpseCreateData {
        guid: ObjectGuid::EMPTY,
        entry_id: 44,
        object_dynamic_flags: 7,
        scale: 1.25,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        corpse_dynamic_flags: 9,
        owner: ObjectGuid::EMPTY,
        party_guid: ObjectGuid::EMPTY,
        guild_guid: ObjectGuid::EMPTY,
        display_id: 123,
        items,
        race_id: 1,
        sex: 0,
        class: 2,
        customizations: vec![ChrCustomizationChoiceValuesUpdate {
            option_id: 11,
            choice_id: 22,
        }],
        flags: 0x55,
        faction_template: 35,
    };

    let mut block = WorldPacket::new_empty();
    write_corpse_create_block(&mut block, &ObjectGuid::EMPTY, &create_data);
    let bytes = block.into_data();

    assert_eq!(bytes[0], UpdateType::CreateObject as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(bytes[3], TypeId::Corpse as u8);
    assert_eq!(u32::from_le_bytes(bytes[27..31].try_into().unwrap()), 126);
    assert_eq!(bytes[31], 0);
    assert_eq!(i32::from_le_bytes(bytes[32..36].try_into().unwrap()), 44);
    assert_eq!(u32::from_le_bytes(bytes[44..48].try_into().unwrap()), 9);
    assert!(
        bytes
            .windows(4)
            .any(|window| window == 0xAABB_CCDDu32.to_le_bytes())
    );
    assert_eq!(bytes.len(), 157);
}

#[test]
fn scene_object_create_block_matches_cpp_scene_movement_and_values_shape() {
    let create_data = SceneObjectCreateData {
        guid: ObjectGuid::EMPTY,
        entry_id: 77,
        dynamic_flags: 5,
        scale: 1.0,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        script_package_id: 99,
        rnd_seed_val: 1234,
        created_by: ObjectGuid::EMPTY,
        scene_type: 1,
    };

    let mut block = WorldPacket::new_empty();
    write_scene_object_create_block(&mut block, &ObjectGuid::EMPTY, &create_data);
    let bytes = block.into_data();

    assert_eq!(bytes[0], UpdateType::CreateObject as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(bytes[3], TypeId::SceneObject as u8);
    assert_eq!(bytes[27], 0); // two false SceneObject extension bits
    assert_eq!(u32::from_le_bytes(bytes[28..32].try_into().unwrap()), 27);
    assert_eq!(i32::from_le_bytes(bytes[45..49].try_into().unwrap()), 99);
    assert_eq!(bytes.len(), 59);
}

#[test]
fn conversation_create_block_matches_cpp_texture_lines_and_actors_shape() {
    let create_data = ConversationCreateData {
        guid: ObjectGuid::EMPTY,
        entry_id: 88,
        dynamic_flags: 6,
        scale: 1.0,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        texture_kit_id: 321,
        lines: vec![ConversationLineValuesUpdate {
            conversation_line_id: 7,
            start_time: 100,
            ui_camera_id: -3,
            actor_index: 2,
            flags: 0x80,
        }],
        actors: vec![ConversationActorValuesUpdate {
            actor_type: 1,
            id: 55,
            creature_id: 12_345,
            creature_display_info_id: 54_321,
            actor_guid: ObjectGuid::EMPTY,
        }],
        last_line_end_time: 777,
    };

    let mut block = WorldPacket::new_empty();
    write_conversation_create_block(&mut block, &ObjectGuid::EMPTY, &create_data);
    let bytes = block.into_data();

    assert_eq!(bytes[0], UpdateType::CreateObject as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(bytes[3], TypeId::Conversation as u8);
    assert_eq!(bytes[27], 0x80); // HasTextureKit
    assert_eq!(u32::from_le_bytes(bytes[28..32].try_into().unwrap()), 321);
    assert_eq!(u32::from_le_bytes(bytes[32..36].try_into().unwrap()), 52);
    assert_eq!(i32::from_le_bytes(bytes[57..61].try_into().unwrap()), 7);
    assert!(
        bytes
            .windows(4)
            .any(|window| window == 54_321u32.to_le_bytes())
    );
    assert_eq!(bytes.len(), 88);
}

#[test]
fn update_object_create_player_serializes() {
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(-8949.95, -132.493, 83.5312, 0.0);

    let pkt = UpdateObject::create_player(
        guid,
        1,
        1,
        0,
        1,
        49,
        &pos,
        0,
        12,
        true,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    let bytes = pkt.to_bytes();
    // Should be a substantial packet (many KB with ActivePlayerData)
    assert!(
        bytes.len() > 1000,
        "Packet too small: {} bytes",
        bytes.len()
    );
    let UpdateBlock::CreateObject { update_type, .. } = pkt.blocks[0] else {
        panic!("player login self must be a create block");
    };
    assert_eq!(
        update_type,
        UpdateType::CreateObject,
        "C++ SendInitSelf writes UPDATETYPE_CREATE_OBJECT for the existing player object"
    );
}

#[test]
fn update_object_out_of_range() {
    let pkt = UpdateObject {
        map_id: 0,
        num_updates: 0,
        destroy_guids: Vec::new(),
        out_of_range_guids: vec![
            ObjectGuid::create_player(1, 1),
            ObjectGuid::create_player(1, 2),
        ],
        blocks: Vec::new(),
    };
    let bytes = pkt.to_bytes();
    assert!(bytes.len() > 10);
}

#[test]
fn socketed_gem_create_uses_cpp_create_order_not_update_order() {
    let gem = SocketedGemValuesUpdate {
        socketed_gem_mask: 0x000F_FFFF,
        item_id: 40_111,
        context: 3,
        bonus_list_ids: std::array::from_fn(|index| (index as u16) + 70),
    };
    let mut packet = WorldPacket::new_empty();

    write_socketed_gem_create_like_cpp(&mut packet, &gem);

    let mut expected = Vec::new();
    expected.extend_from_slice(&gem.item_id.to_le_bytes());
    for bonus in gem.bonus_list_ids {
        expected.extend_from_slice(&bonus.to_le_bytes());
    }
    expected.push(gem.context);
    assert_eq!(packet.into_data(), expected);
}

#[test]
fn item_create_serializes_random_properties_context_and_socketed_gems() {
    let item_guid = ObjectGuid::create_item(1, 900);
    let owner_guid = ObjectGuid::create_player(1, 42);
    let pkt = UpdateObject::create_items(
        vec![ItemCreateData {
            item_guid,
            entry_id: 700,
            owner_guid,
            contained_in: owner_guid,
            stack_count: 7,
            dynamic_flags: 0,
            durability: 12,
            max_durability: 20,
            random_properties_seed: 456,
            random_properties_id: -77,
            enchantments: {
                let mut enchantments = [ItemEnchantmentValuesUpdate::default(); 13];
                enchantments[0] = ItemEnchantmentValuesUpdate {
                    id: 2673,
                    duration: 0,
                    charges: 0,
                    ..Default::default()
                };
                enchantments
            },
            gems: vec![SocketedGemValuesUpdate {
                socketed_gem_mask: 0x000F_FFFF,
                item_id: 40111,
                context: 3,
                bonus_list_ids: {
                    let mut bonuses = [0; 16];
                    bonuses[0] = 77;
                    bonuses
                },
            }],
            context: 2,
            container_slots: 0,
            container_item_guids: [ObjectGuid::EMPTY; 36],
        }],
        0,
    );

    let bytes = pkt.to_bytes();
    let UpdateBlock::CreateItem { update_type, .. } = pkt.blocks[0] else {
        panic!("item packet must contain an item create block");
    };
    assert_eq!(
        update_type,
        UpdateType::CreateObject2,
        "newly created item objects keep C++ m_isNewObject/CreateObject2 semantics"
    );

    assert!(bytes.windows(4).any(|window| window == 7i32.to_le_bytes()));
    assert!(bytes.windows(4).any(|window| window == 12i32.to_le_bytes()));
    assert!(bytes.windows(4).any(|window| window == 20i32.to_le_bytes()));
    assert!(
        bytes
            .windows(4)
            .any(|window| window == 456i32.to_le_bytes())
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == (-77i32).to_le_bytes())
    );
    assert!(bytes.windows(4).any(|window| window == 2i32.to_le_bytes()));
    assert!(
        bytes
            .windows(4)
            .any(|window| window == 2673i32.to_le_bytes())
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == 40111i32.to_le_bytes())
    );
    assert!(bytes.windows(2).any(|window| window == 77u16.to_le_bytes()));
}

#[test]
fn stored_item_create_uses_cpp_non_map_create_type() {
    let item_guid = ObjectGuid::create_item(1, 900);
    let owner_guid = ObjectGuid::create_player(1, 42);
    let packet = UpdateObject::create_stored_items(
        vec![ItemCreateData {
            item_guid,
            entry_id: 700,
            owner_guid,
            contained_in: owner_guid,
            stack_count: 1,
            dynamic_flags: 0,
            durability: 0,
            max_durability: 0,
            random_properties_seed: 0,
            random_properties_id: 0,
            enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
            gems: Vec::new(),
            context: 0,
            container_slots: 0,
            container_item_guids: [ObjectGuid::EMPTY; 36],
        }],
        0,
    );

    let UpdateBlock::CreateItem { update_type, .. } = packet.blocks[0] else {
        panic!("stored item packet must contain an item create block");
    };
    assert_eq!(update_type, UpdateType::CreateObject);
}

#[test]
fn container_create_serializes_cpp_container_data_after_item_data() {
    let item_guid = ObjectGuid::create_item(1, 900);
    let owner_guid = ObjectGuid::create_player(1, 42);
    let contained_item = ObjectGuid::create_item(1, 901);
    let mut container_item_guids = [ObjectGuid::EMPTY; 36];
    container_item_guids[3] = contained_item;

    let pkt = UpdateObject::create_items(
        vec![ItemCreateData {
            item_guid,
            entry_id: 700,
            owner_guid,
            contained_in: owner_guid,
            stack_count: 1,
            dynamic_flags: 0,
            durability: 0,
            max_durability: 0,
            random_properties_seed: 0,
            random_properties_id: 0,
            enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
            gems: Vec::new(),
            context: 0,
            container_slots: 16,
            container_item_guids,
        }],
        0,
    );

    let bytes = pkt.to_bytes();
    assert!(
        bytes
            .windows(1)
            .any(|window| window == [TypeId::Container as u8]),
        "C++ Bag CREATE uses TYPEID_CONTAINER"
    );
    assert!(
        bytes.windows(4).any(|window| window == 16u32.to_le_bytes()),
        "C++ ContainerData::WriteCreate writes NumSlots after 36 slot GUIDs"
    );
}

#[test]
fn item_stack_count_update_serializes_item_values_delta() {
    let item_guid = ObjectGuid::create_item(1, 900);
    let pkt = UpdateObject::item_stack_count_update(item_guid, 0, 19);

    let bytes = pkt.to_bytes();

    assert!(bytes.len() > 20);
    assert!(bytes.windows(4).any(|window| window == 19i32.to_le_bytes()));
}

#[test]
fn bound_existing_stack_serializes_count_and_flags_in_one_values_update() {
    let item_guid = ObjectGuid::create_item(1, 901);
    let dynamic_flags = 0x0000_0001;
    let pkt = UpdateObject::item_stack_count_and_flags_update(item_guid, 0, 19, dynamic_flags);

    assert_eq!(pkt.num_updates, 1);
    assert_eq!(pkt.blocks.len(), 1);
    assert!(matches!(
        pkt.blocks.as_slice(),
        [UpdateBlock::ItemValuesUpdate {
            guid,
            stack_count: 19,
            dynamic_flags: Some(flags),
        }] if *guid == item_guid && *flags == dynamic_flags
    ));

    let bytes = pkt.to_bytes();
    assert!(bytes.windows(4).any(|window| window == 19i32.to_le_bytes()));
    assert!(
        bytes
            .windows(4)
            .any(|window| window == dynamic_flags.to_le_bytes())
    );
}

#[test]
fn movement_block_default_speeds() {
    let mv = MovementBlock::default();
    assert_eq!(mv.walk_speed, 2.5);
    assert_eq!(mv.run_speed, 7.0);
    assert_eq!(mv.run_back_speed, 4.5);
    assert_eq!(mv.swim_speed, 4.72222);
    assert_eq!(mv.swim_back_speed, 2.5);
    assert_eq!(mv.fly_speed, 7.0);
    assert_eq!(mv.fly_back_speed, 4.5);
    assert_eq!(mv.turn_rate, 3.141594);
    assert_eq!(mv.pitch_rate, 3.14);
}

#[test]
fn player_create_data_faction() {
    assert_eq!(PlayerCreateData::faction_for_race(1), 1); // Human
    assert_eq!(PlayerCreateData::faction_for_race(2), 2); // Orc
    assert_eq!(PlayerCreateData::faction_for_race(10), 1610); // BloodElf
    assert_eq!(PlayerCreateData::faction_for_race(11), 1629); // Draenei
}
