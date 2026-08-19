//! Behaviour tests for [`super`].
//!
//! Extracted verbatim from `update.rs`, which was 13,439 lines of which
//! 3,704 — 28% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant.

#![cfg(test)]

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

#[test]
fn update_object_envelope_format() {
    // Verify the top-level format: opcode + NumObjUpdates + MapID + Data
    let guid = ObjectGuid::create_player(1, 1);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
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

    // opcode (2 bytes)
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, ServerOpcodes::UpdateObject as u16);

    // NumObjUpdates (u32 at offset 2)
    let num_updates = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
    assert_eq!(num_updates, 1);

    // MapID (u16 at offset 6)
    let map_id = u16::from_le_bytes([bytes[6], bytes[7]]);
    assert_eq!(map_id, 0);
}

#[test]
fn update_object_destroy_and_oor() {
    let pkt = UpdateObject {
        map_id: 0,
        num_updates: 0,
        destroy_guids: vec![ObjectGuid::create_player(1, 10)],
        out_of_range_guids: vec![ObjectGuid::create_player(1, 20)],
        blocks: Vec::new(),
    };
    let bytes = pkt.to_bytes();
    // Should contain destroy + oor data
    assert!(bytes.len() > 20);
}

#[test]
fn update_object_destroy_sets_dedupe_like_cpp() {
    let guid1 = ObjectGuid::create_player(1, 1);
    let guid2 = ObjectGuid::create_player(1, 2);
    let guid3 = ObjectGuid::create_player(1, 3);
    let pkt = UpdateObject {
        map_id: 0,
        num_updates: 0,
        destroy_guids: vec![guid2, guid1, guid1],
        out_of_range_guids: vec![guid3, guid3],
        blocks: Vec::new(),
    };
    let bytes = pkt.to_bytes();

    // opcode(2) + NumObjUpdates(4) + MapID(2) + HasDestroy bit byte(1)
    let destroy_count = u16::from_le_bytes([bytes[9], bytes[10]]);
    let total_count = u32::from_le_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]);
    assert_eq!(destroy_count, 2);
    assert_eq!(total_count, 3);
}

#[test]
fn object_values_update_block_matches_cpp_objectdata_delta_shape() {
    let mut block = WorldPacket::new_empty();
    write_object_values_update_block(
        &mut block,
        &ObjectGuid::EMPTY,
        ObjectDataValuesUpdate {
            changed_object_type_mask: 1,
            object_data_mask: 0b1011,
            entry_id: 42,
            dynamic_flags: 0x80,
            scale: 2.0,
        },
    );

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 13);
    assert_eq!(u32::from_le_bytes(bytes[7..11].try_into().unwrap()), 1);
    assert_eq!(bytes[11], 0b1011_0000);
    assert_eq!(i32::from_le_bytes(bytes[12..16].try_into().unwrap()), 42);
    assert_eq!(f32::from_le_bytes(bytes[16..20].try_into().unwrap()), 2.0);
    assert_eq!(bytes.len(), 20);
}

#[test]
fn dynamic_object_values_update_block_matches_cpp_dynamicobjectdata_delta_shape() {
    let mut block = WorldPacket::new_empty();
    write_dynamic_object_values_update_block(
        &mut block,
        &ObjectGuid::EMPTY,
        DynamicObjectDataValuesUpdate {
            changed_object_type_mask: VALUES_TYPE_DYNAMIC_OBJECT,
            object_data: None,
            dynamic_object_data_mask: 0b111_1111,
            caster: ObjectGuid::EMPTY,
            dynamic_object_type: 1,
            spell_visual_id: 42,
            spell_id: 1337,
            radius: 8.5,
            cast_time_ms: 123_456,
        },
    );

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 24);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_DYNAMIC_OBJECT
    );
    assert_eq!(bytes[11], 0b1111_1110);
    assert_eq!(&bytes[12..14], &[0, 0]);
    assert_eq!(bytes[14], 1);
    assert_eq!(i32::from_le_bytes(bytes[15..19].try_into().unwrap()), 42);
    assert_eq!(i32::from_le_bytes(bytes[19..23].try_into().unwrap()), 1337);
    assert_eq!(f32::from_le_bytes(bytes[23..27].try_into().unwrap()), 8.5);
    assert_eq!(
        u32::from_le_bytes(bytes[27..31].try_into().unwrap()),
        123_456
    );
    assert_eq!(bytes.len(), 31);
}

#[test]
fn area_trigger_create_block_writes_cpp_shape_and_create_values() {
    fn scale_curve(override_active: bool) -> ScaleCurveValuesUpdate {
        ScaleCurveValuesUpdate {
            scale_curve_mask: 0,
            override_active,
            start_time_offset: 7,
            parameter_curve: 1.0f32.to_bits() | 1,
            points: [(1.0, 2.0), (3.0, 4.0)],
        }
    }

    let create_data = AreaTriggerCreateData {
        guid: ObjectGuid::EMPTY,
        entry_id: 9003,
        dynamic_flags: 0x80,
        scale: 1.0,
        position: Position::new(1.0, 2.0, 3.0, 0.5),
        time_since_created_ms: 123,
        roll_pitch_yaw: Position::new(0.1, 0.2, 0.3, 0.0),
        target_roll_pitch_yaw: Position::ZERO,
        create_properties_flags: 0,
        scale_curve_id: 0,
        morph_curve_id: 0,
        facing_curve_id: 0,
        move_curve_id: 0,
        shape: AreaTriggerShapeCreateData {
            shape_type: 0,
            data: [4.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            polygon_vertices: Vec::new(),
            polygon_vertices_target: Vec::new(),
        },
        spline_points: Vec::new(),
        orbit: None,
        override_scale_curve: scale_curve(true),
        extra_scale_curve: scale_curve(false),
        override_move_curve_x: scale_curve(false),
        override_move_curve_y: scale_curve(false),
        override_move_curve_z: scale_curve(false),
        caster: ObjectGuid::EMPTY,
        duration: 0,
        time_to_target: 0,
        time_to_target_scale: 0,
        time_to_target_extra_scale: 0,
        time_to_target_pos: 0,
        spell_id: 0,
        spell_for_visuals: 0,
        spell_visual_id: 4321,
        bounds_radius_2d: 7.0,
        decal_properties_id: 24,
        creating_effect_guid: ObjectGuid::EMPTY,
        orbit_path_target: ObjectGuid::EMPTY,
        visual_anim: VisualAnimValuesUpdate {
            visual_anim_mask: 0,
            field_c: true,
            animation_data_id: 11,
            anim_kit_id: 22,
            anim_progress: 0,
        },
    };

    let mut block = WorldPacket::new_empty();
    write_area_trigger_create_block(&mut block, &ObjectGuid::EMPTY, &create_data);
    let bytes = block.into_data();

    assert_eq!(bytes[0], UpdateType::CreateObject as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(bytes[3], TypeId::AreaTrigger as u8);
    assert!(bytes.len() > 120);
    assert!(
        bytes
            .windows(4)
            .any(|window| window == 4.0f32.to_le_bytes()),
        "sphere radius must be written in the AreaTrigger movement payload"
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == 4321i32.to_le_bytes()),
        "SpellXSpellVisualID must be written in AreaTriggerData::WriteCreate order"
    );
}

#[test]
fn scene_object_values_update_block_matches_cpp_sceneobjectdata_delta_shape() {
    let mut block = WorldPacket::new_empty();
    write_scene_object_values_update_block(
        &mut block,
        &ObjectGuid::EMPTY,
        SceneObjectDataValuesUpdate {
            changed_object_type_mask: VALUES_TYPE_SCENE_OBJECT,
            object_data: None,
            scene_object_data_mask: 0b1_1111,
            script_package_id: 77,
            rnd_seed_val: 0xAABB_CCDD,
            created_by: ObjectGuid::EMPTY,
            scene_type: 1,
        },
    );

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 19);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_SCENE_OBJECT
    );
    assert_eq!(bytes[11], 0b1111_1000);
    assert_eq!(i32::from_le_bytes(bytes[12..16].try_into().unwrap()), 77);
    assert_eq!(
        u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
        0xAABB_CCDD
    );
    assert_eq!(&bytes[20..22], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[22..26].try_into().unwrap()), 1);
    assert_eq!(bytes.len(), 26);
}

#[test]
fn conversation_values_update_block_matches_cpp_last_line_delta_shape() {
    let mut block = WorldPacket::new_empty();
    write_conversation_values_update_block(
        &mut block,
        &ObjectGuid::EMPTY,
        &ConversationDataValuesUpdate {
            changed_object_type_mask: VALUES_TYPE_CONVERSATION,
            object_data: None,
            conversation_data_mask: 0b1001,
            lines: Vec::new(),
            actors: Vec::new(),
            actor_update_mask: None,
            last_line_end_time: 12_345,
        },
    );

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 9);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_CONVERSATION
    );
    assert_eq!(bytes[11], 0b1001_0000);
    assert_eq!(
        i32::from_le_bytes(bytes[12..16].try_into().unwrap()),
        12_345
    );
    assert_eq!(bytes.len(), 16);
}

#[test]
fn conversation_values_update_block_matches_cpp_lines_actors_delta_shape() {
    let mut block = WorldPacket::new_empty();
    write_conversation_values_update_block(
        &mut block,
        &ObjectGuid::EMPTY,
        &ConversationDataValuesUpdate {
            changed_object_type_mask: VALUES_TYPE_CONVERSATION,
            object_data: None,
            conversation_data_mask: 0b1111,
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
            actor_update_mask: None,
            last_line_end_time: 777,
        },
    );

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 45);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_CONVERSATION
    );
    assert_eq!(&bytes[11..16], &[0xF0, 0x00, 0x00, 0x00, 0x10]);
    assert_eq!(i32::from_le_bytes(bytes[16..20].try_into().unwrap()), 7);
    assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 100);
    assert_eq!(i32::from_le_bytes(bytes[24..28].try_into().unwrap()), -3);
    assert_eq!(&bytes[28..30], &[2, 0x80]);
    assert_eq!(&bytes[30..35], &[0x00, 0x00, 0x00, 0x01, 0x80]);
    assert_eq!(bytes[35], 0x80);
    assert_eq!(i32::from_le_bytes(bytes[36..40].try_into().unwrap()), 55);
    assert_eq!(
        u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
        12_345
    );
    assert_eq!(
        u32::from_le_bytes(bytes[44..48].try_into().unwrap()),
        54_321
    );
    assert_eq!(i32::from_le_bytes(bytes[48..52].try_into().unwrap()), 777);
    assert_eq!(bytes.len(), 52);
}

#[test]
fn game_object_values_update_block_matches_cpp_gameobjectdata_delta_shape() {
    let mut block = WorldPacket::new_empty();
    write_game_object_values_update_block(
        &mut block,
        &ObjectGuid::EMPTY,
        &GameObjectDataValuesUpdate {
            changed_object_type_mask: VALUES_TYPE_GAME_OBJECT,
            object_data: None,
            game_object_data_mask: 0x0003_8011,
            state_world_effect_ids: Vec::new(),
            enable_doodad_sets: Vec::new(),
            enable_doodad_sets_update_mask: None,
            world_effects: Vec::new(),
            world_effects_update_mask: None,
            display_id: 123,
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
            state: -1,
            type_id: 5,
            percent_health: 90,
            art_kit: 0,
            custom_param: 0,
        },
    );

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 14);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_GAME_OBJECT
    );
    assert_eq!(&bytes[11..14], &[0x38, 0x01, 0x10]);
    assert_eq!(i32::from_le_bytes(bytes[14..18].try_into().unwrap()), 123);
    assert_eq!(&bytes[18..21], &[0xFF, 5, 90]);
    assert_eq!(bytes.len(), 21);
}

#[test]
fn corpse_values_update_block_matches_cpp_corpse_data_delta_shape() {
    let mut items = [0u32; 19];
    items[0] = 0xAABB_CCDD;

    let mut block = WorldPacket::new_empty();
    write_corpse_values_update_block(
        &mut block,
        &ObjectGuid::EMPTY,
        &CorpseDataValuesUpdate {
            changed_object_type_mask: VALUES_TYPE_CORPSE,
            object_data: None,
            corpse_data_mask: 0x0000_3007,
            customizations: vec![ChrCustomizationChoiceValuesUpdate {
                option_id: 11,
                choice_id: 22,
            }],
            customizations_update_mask: None,
            dynamic_flags: 0x44,
            owner: ObjectGuid::EMPTY,
            party_guid: ObjectGuid::EMPTY,
            guild_guid: ObjectGuid::EMPTY,
            display_id: 0,
            race_id: 0,
            sex: 0,
            class: 0,
            flags: 0,
            faction_template: 0,
            items,
        },
    );

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 29);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_CORPSE
    );
    assert_eq!(&bytes[11..15], &[0x00, 0x00, 0x30, 0x07]);
    assert_eq!(&bytes[15..20], &[0x00, 0x00, 0x00, 0x01, 0x80]);
    assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 11);
    assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 22);
    assert_eq!(u32::from_le_bytes(bytes[28..32].try_into().unwrap()), 0x44);
    assert_eq!(
        u32::from_le_bytes(bytes[32..36].try_into().unwrap()),
        0xAABB_CCDD
    );
    assert_eq!(bytes.len(), 36);
}

#[test]
fn area_trigger_values_update_block_matches_cpp_areatriggerdata_delta_shape() {
    let empty_curve = ScaleCurveValuesUpdate {
        scale_curve_mask: 0,
        override_active: false,
        start_time_offset: 0,
        parameter_curve: 0,
        points: [(0.0, 0.0); 2],
    };

    let mut block = WorldPacket::new_empty();
    write_area_trigger_values_update_block(
        &mut block,
        &ObjectGuid::EMPTY,
        &AreaTriggerDataValuesUpdate {
            changed_object_type_mask: VALUES_TYPE_AREA_TRIGGER,
            object_data: None,
            area_trigger_data_mask: 0x0008_1081,
            override_scale_curve: empty_curve,
            extra_scale_curve: empty_curve,
            override_move_curve_x: empty_curve,
            override_move_curve_y: empty_curve,
            override_move_curve_z: empty_curve,
            caster: ObjectGuid::EMPTY,
            duration: 12_000,
            time_to_target: 0,
            time_to_target_scale: 0,
            time_to_target_extra_scale: 0,
            time_to_target_pos: 0,
            spell_id: 99,
            spell_for_visuals: 0,
            spell_visual_id: 0,
            bounds_radius_2d: 0.0,
            decal_properties_id: 0,
            creating_effect_guid: ObjectGuid::EMPTY,
            orbit_path_target: ObjectGuid::EMPTY,
            visual_anim: VisualAnimValuesUpdate {
                visual_anim_mask: 0b0_0111,
                field_c: true,
                animation_data_id: 77,
                anim_kit_id: 0,
                anim_progress: 0,
            },
        },
    );

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 20);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_AREA_TRIGGER
    );
    assert_eq!(&bytes[11..14], &[0x81, 0x08, 0x10]);
    assert_eq!(
        u32::from_le_bytes(bytes[14..18].try_into().unwrap()),
        12_000
    );
    assert_eq!(i32::from_le_bytes(bytes[18..22].try_into().unwrap()), 99);
    assert_eq!(bytes[22], 0x3C);
    assert_eq!(u32::from_le_bytes(bytes[23..27].try_into().unwrap()), 77);
    assert_eq!(bytes.len(), 27);
}

fn test_item_data(mask: u64) -> ItemDataValuesDeltaUpdate {
    ItemDataValuesDeltaUpdate {
        changed_object_type_mask: VALUES_TYPE_ITEM,
        object_data: None,
        item_data_mask: mask,
        artifact_powers: Vec::new(),
        artifact_powers_update_mask: None,
        gems: Vec::new(),
        gems_update_mask: None,
        owner: ObjectGuid::EMPTY,
        contained_in: ObjectGuid::EMPTY,
        creator: ObjectGuid::EMPTY,
        gift_creator: ObjectGuid::EMPTY,
        stack_count: 5,
        expiration: 0,
        dynamic_flags: 0,
        property_seed: 0,
        random_properties_id: 0,
        durability: 0,
        max_durability: 0,
        create_played_time: 0,
        context: 0,
        create_time: 0,
        artifact_xp: 0,
        item_appearance_mod_id: 0,
        modifiers: ItemModListValuesUpdate {
            item_mod_list_mask: 0,
            values: Vec::new(),
            values_update_mask: None,
        },
        dynamic_flags2: 0,
        item_bonus_key: ItemBonusKeyValuesUpdate::default(),
        debug_item_level: 0,
        spell_charges: [0; 5],
        enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
    }
}

#[test]
fn full_item_values_update_block_matches_cpp_itemdata_stack_delta_shape() {
    let mut block = WorldPacket::new_empty();
    write_full_item_values_update_block(
        &mut block,
        &ObjectGuid::EMPTY,
        &test_item_data((1 << 0) | (1 << 7)),
    );

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 13);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_ITEM
    );
    assert_eq!(&bytes[11..16], &[0x40, 0x00, 0x00, 0x20, 0x40]);
    assert_eq!(u32::from_le_bytes(bytes[16..20].try_into().unwrap()), 5);
    assert_eq!(bytes.len(), 20);
}

#[test]
fn container_values_update_block_matches_cpp_containerdata_slot_delta_shape() {
    let mut slots = [ObjectGuid::EMPTY; 36];
    slots[0] = ObjectGuid::EMPTY;

    let mut block = WorldPacket::new_empty();
    write_container_values_update_block(
        &mut block,
        &ObjectGuid::EMPTY,
        &ContainerDataValuesUpdate {
            changed_object_type_mask: VALUES_TYPE_CONTAINER,
            object_data: None,
            item_data: None,
            container_data_mask: 0x0F,
            num_slots: 16,
            slots,
        },
    );

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 15);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_CONTAINER
    );
    assert_eq!(&bytes[11..16], &[0x40, 0x00, 0x00, 0x03, 0xC0]);
    assert_eq!(u32::from_le_bytes(bytes[16..20].try_into().unwrap()), 16);
    assert_eq!(&bytes[20..22], &[0, 0]);
    assert_eq!(bytes.len(), 22);
}

#[test]
fn full_unit_values_update_block_matches_cpp_unitdata_health_delta_shape() {
    let mut data = UnitDataValuesDeltaUpdate {
        health: 77,
        max_health: 99,
        ..Default::default()
    };
    data.unit_data_mask[0] = (1 << 0) | (1 << 5) | (1 << 6);

    let mut block = WorldPacket::new_empty();
    write_full_unit_values_update_block(&mut block, &ObjectGuid::EMPTY, &data);

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 25);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_UNIT
    );
    assert_eq!(&bytes[11..16], &[0x01, 0x00, 0x00, 0x00, 0x61]);
    assert_eq!(i64::from_le_bytes(bytes[16..24].try_into().unwrap()), 77);
    assert_eq!(i64::from_le_bytes(bytes[24..32].try_into().unwrap()), 99);
    assert_eq!(bytes.len(), 32);
}

#[test]
fn full_unit_values_update_block_matches_cpp_stand_state_delta_shape() {
    let mut data = UnitDataValuesDeltaUpdate {
        stand_state: 1,
        ..Default::default()
    };
    // C++ generated `UpdateField<uint8, 32, 56> StandState`.
    data.unit_data_mask[1] = (1 << 0) | (1 << 24);

    let mut block = WorldPacket::new_empty();
    write_full_unit_values_update_block(&mut block, &ObjectGuid::EMPTY, &data);

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 10);

    let mut values = WorldPacket::from_bytes(&bytes[7..]);
    assert_eq!(values.read_uint32().unwrap(), VALUES_TYPE_UNIT);
    assert_eq!(values.read_bits(8).unwrap(), 1 << 1);
    assert_eq!(values.read_bits(32).unwrap(), (1 << 0) | (1 << 24));
    values.reset_bits();
    assert_eq!(values.read_uint8().unwrap(), 1);
    assert_eq!(values.remaining(), 0);
    assert_eq!(bytes.len(), 17);
}

#[test]
fn full_unit_values_update_block_matches_cpp_unitdata_virtual_item_delta_shape() {
    let mut data = UnitDataValuesDeltaUpdate::default();
    data.unit_data_mask[5] = (1 << 7) | (1 << 8);
    data.virtual_items[0] = VisibleItemValuesUpdate {
        visible_item_mask: 0x0F,
        item_id: 19019,
        appearance_mod_id: 2,
        item_visual: 3,
    };

    let mut block = WorldPacket::new_empty();
    write_full_unit_values_update_block(&mut block, &ObjectGuid::EMPTY, &data);

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 18);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_UNIT
    );
    assert_eq!(&bytes[11..16], &[0x20, 0x00, 0x00, 0x01, 0x80]);
    assert_eq!(bytes[16], 0xF0);
    assert_eq!(i32::from_le_bytes(bytes[17..21].try_into().unwrap()), 19019);
    assert_eq!(u16::from_le_bytes(bytes[21..23].try_into().unwrap()), 2);
    assert_eq!(u16::from_le_bytes(bytes[23..25].try_into().unwrap()), 3);
    assert_eq!(bytes.len(), 25);
}

#[test]
fn full_unit_values_update_block_matches_cpp_unitdata_npc_flags_delta_shape() {
    let mut data = UnitDataValuesDeltaUpdate::default();
    data.unit_data_mask[3] = (1 << 17) | (1 << 18) | (1 << 19);
    data.npc_flags = [0x40, 0x1];

    let mut block = WorldPacket::new_empty();
    write_full_unit_values_update_block(&mut block, &ObjectGuid::EMPTY, &data);

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 17);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_UNIT
    );
    assert_eq!(u32::from_le_bytes(bytes[16..20].try_into().unwrap()), 0x40);
    assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 0x1);
    assert_eq!(bytes.len(), 24);
}

#[test]
fn full_player_values_update_block_matches_cpp_playerdata_visible_item_delta_shape() {
    let mut data = PlayerDataValuesDeltaUpdate::default();
    data.player_data_mask[1] = (1 << 29) | (1 << 30);
    data.visible_items[0] = VisibleItemValuesUpdate {
        visible_item_mask: 0x0F,
        item_id: 19019,
        appearance_mod_id: 2,
        item_visual: 3,
    };

    let mut block = WorldPacket::new_empty();
    write_full_player_values_update_block(&mut block, &ObjectGuid::EMPTY, &data);

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 18);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_PLAYER
    );
    assert_eq!(&bytes[11..16], &[0x26, 0x00, 0x00, 0x00, 0x00]);
    assert_eq!(bytes[16], 0xF0);
    assert_eq!(i32::from_le_bytes(bytes[17..21].try_into().unwrap()), 19019);
    assert_eq!(u16::from_le_bytes(bytes[21..23].try_into().unwrap()), 2);
    assert_eq!(u16::from_le_bytes(bytes[23..25].try_into().unwrap()), 3);
    assert_eq!(bytes.len(), 25);
}

#[test]
fn full_player_values_update_block_can_append_active_player_section_like_cpp() {
    let mut active_data = ActivePlayerDataValuesUpdate {
        coinage: 1234,
        ..Default::default()
    };
    set_active_player_bit(&mut active_data, 0);
    set_active_player_bit(&mut active_data, 28);

    let mut data = PlayerDataValuesDeltaUpdate {
        changed_object_type_mask: VALUES_TYPE_PLAYER | VALUES_TYPE_ACTIVE_PLAYER,
        active_player_data: Some(active_data),
        ..Default::default()
    };
    data.player_data_mask[1] = (1 << 29) | (1 << 30);
    data.visible_items[0] = VisibleItemValuesUpdate {
        visible_item_mask: 0x0F,
        item_id: 19019,
        appearance_mod_id: 2,
        item_visual: 3,
    };

    let mut block = WorldPacket::new_empty();
    write_full_player_values_update_block(&mut block, &ObjectGuid::EMPTY, &data);

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_PLAYER | VALUES_TYPE_ACTIVE_PLAYER
    );
    assert_eq!(&bytes[11..16], &[0x26, 0x00, 0x00, 0x00, 0x00]);
    assert_eq!(bytes[16], 0xF0);
    assert_eq!(i32::from_le_bytes(bytes[17..21].try_into().unwrap()), 19019);
    assert_eq!(&bytes[25..29], &[0x01, 0x00, 0x00, 0x00]);
    assert_eq!(&bytes[29..31], &[0x00, 0x00]);
    assert_eq!(&bytes[31..35], &[0x10, 0x00, 0x00, 0x01]);
    assert_eq!(u64::from_le_bytes(bytes[35..43].try_into().unwrap()), 1234);
    assert_eq!(bytes.len(), 43);
}

#[test]
fn creature_health_values_update_has_no_create_flags_byte_like_cpp() {
    let mut block = WorldPacket::new_empty();
    write_creature_health_update_block(&mut block, &ObjectGuid::EMPTY, 7, 11);

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 25);
    assert_eq!(u32::from_le_bytes(bytes[7..11].try_into().unwrap()), 1 << 5);
    assert_eq!(bytes[11], 0x01);
    assert_eq!(&bytes[12..16], &[0, 0, 0, 0x61]);
    assert_eq!(i64::from_le_bytes(bytes[16..24].try_into().unwrap()), 7);
    assert_eq!(i64::from_le_bytes(bytes[24..32].try_into().unwrap()), 11);
    assert_eq!(bytes.len(), 32);
}

#[test]
fn buyback_values_update_interleaves_price_and_timestamp_like_cpp() {
    let mut values = WorldPacket::new_empty();
    write_active_player_data_values_update(
        &mut values,
        &[],
        &[(94, 123, 456), (95, 789, 101112)],
        None,
        None,
    );

    let bytes = values.into_data();
    let tail = &bytes[bytes.len() - 24..];
    assert_eq!(u32::from_le_bytes(tail[0..4].try_into().unwrap()), 123);
    assert_eq!(i64::from_le_bytes(tail[4..12].try_into().unwrap()), 456);
    assert_eq!(u32::from_le_bytes(tail[12..16].try_into().unwrap()), 789);
    assert_eq!(i64::from_le_bytes(tail[16..24].try_into().unwrap()), 101112);
}

#[test]
fn active_player_coinage_values_update_matches_cpp_mask_shape() {
    let mut values = WorldPacket::new_empty();
    write_active_player_data_values_update(&mut values, &[], &[], None, Some(1234));

    let bytes = values.into_data();
    assert_eq!(&bytes[0..4], &[0x01, 0x00, 0x00, 0x00]); // group 0: block 0
    assert_eq!(&bytes[4..6], &[0x00, 0x00]); // group 1: no blocks 32..47
    assert_eq!(&bytes[6..10], &[0x10, 0x00, 0x00, 0x01]); // block 0: bits 0 and 28
    assert_eq!(u64::from_le_bytes(bytes[10..18].try_into().unwrap()), 1234);
    assert_eq!(bytes.len(), 18);
}

#[test]
fn active_player_scaling_delta_values_update_matches_cpp_mask_and_value() {
    let mut data = ActivePlayerDataValuesUpdate::default();
    data.active_player_data_mask[0] = 1;
    data.active_player_data_mask[2] = (1 << (70 - 64)) | (1 << (94 - 64));
    data.scaling_player_level_delta = -1;

    let mut values = WorldPacket::new_empty();
    write_active_player_data_values_update_section(&mut values, &data);

    assert_eq!(
        values.into_data(),
        vec![
            0x05, 0x00, 0x00, 0x00, // group 0: blocks 0 and 2
            0x00, 0x00, // group 1: no blocks 32..47
            0x00, 0x00, 0x00, 0x01, // block 0: root bit 0
            0x40, 0x00, 0x00, 0x40, // block 2: parent 70 and field 94
            0xFF, 0xFF, 0xFF, 0xFF, // ScalingPlayerLevelDelta = -1
        ]
    );
}

#[test]
fn active_player_stats_values_update_matches_cpp_common_runtime_masks() {
    let mut combat_ratings = [0; 32];
    combat_ratings[0] = 11;
    combat_ratings[31] = 99;

    let stats = PlayerStatChanges {
        health: 0,
        max_health: 0,
        min_damage: 0.0,
        max_damage: 0.0,
        base_mana: 0,
        base_health: 0,
        attack_power: 0,
        attack_power_mod_pos: 0,
        attack_power_mod_neg: 0,
        attack_power_multiplier: 0.0,
        ranged_attack_power: 0,
        ranged_attack_power_mod_pos: 0,
        ranged_attack_power_mod_neg: 0,
        ranged_attack_power_multiplier: 0.0,
        min_ranged_damage: 0.0,
        max_ranged_damage: 0.0,
        power0: 0,
        max_power0: 0,
        stats: [0; 5],
        stat_pos_buff: [0; 5],
        stat_neg_buff: [0; 5],
        armor: 0,
        combat_ratings,
        spell_power: 123,
        block_pct: 1.0,
        dodge_pct: 2.0,
        parry_pct: 3.0,
        crit_pct: 4.0,
        ranged_crit_pct: 5.0,
        spell_crit_pct: [6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
        mana_regen: 0.0,
        mana_regen_combat: 0.0,
        mana_regen_mp5: 0.0,
        mainhand_expertise: 13.0,
        offhand_expertise: 14.0,
        ranged_expertise: 15.0,
        combat_rating_expertise: 16.0,
        dodge_from_attr: 17.0,
        parry_from_attr: 18.0,
        offhand_crit_pct: 19.0,
        shield_block: 20,
        shield_block_crit_pct: 21.0,
        mod_healing_pct: 1.0,
        mod_healing_done_pct: 1.0,
        mod_periodic_healing_pct: 1.0,
        mod_spell_power_pct: 1.0,
    };

    let mut values = WorldPacket::new_empty();
    write_active_player_data_values_update(&mut values, &[], &[], Some(&stats), None);

    let bytes = values.into_data();
    assert_eq!(&bytes[0..4], &[0x07, 0x01, 0x06, 0x00]); // blocks 0,1,2,8,17,18
    assert_eq!(&bytes[4..6], &[0x00, 0x00]);
    assert_eq!(&bytes[6..10], &[0x00, 0x00, 0x00, 0x01]);
    // block 1 = 0xFFFBFFF0: bits 4,5 + bits 6..31 EXCEPT bit 18 (field 50,
    // ShieldBlockCritPercentage, reserved in the 54261 client grammar).
    assert_eq!(&bytes[10..14], &[0xFF, 0xFB, 0xFF, 0xF0]);
    assert_eq!(&bytes[14..18], &[0x00, 0x00, 0x00, 0x3F]);
    assert_eq!(&bytes[18..22], &[0x0F, 0xFF, 0xE0, 0x00]);
    assert_eq!(&bytes[22..26], &[0xC0, 0x00, 0x00, 0x00]);
    assert_eq!(&bytes[26..30], &[0x7F, 0xFF, 0xFF, 0xFF]);

    let expertise = 13.0f32.to_le_bytes();
    let values_start = bytes
        .windows(4)
        .position(|window| window == expertise)
        .expect("mainhand expertise value must be present after ActivePlayerData masks");
    let mut offset = values_start;
    assert_eq!(
        f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()),
        13.0
    );
    offset += 4;
    assert_eq!(
        f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()),
        14.0
    );
    offset += 4;
    // Parent-38 section is 30 fields (bits 39-49, 51-69); field bit 50 is
    // reserved and not emitted, so skip 30 floats (not 31) to reach SpellCrit.
    offset += 30 * 4;

    for expected in [6.0f32, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0] {
        assert_eq!(
            f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()),
            expected
        );
        offset += 4;
        offset += 4; // ModDamageDonePos for the same school.
    }

    assert_eq!(
        i32::from_le_bytes(
            bytes[bytes.len() - 128..bytes.len() - 124]
                .try_into()
                .unwrap()
        ),
        11
    );
    assert_eq!(
        i32::from_le_bytes(bytes[bytes.len() - 4..].try_into().unwrap()),
        99
    );
}

/// Regression for the real 3.4.3.54261 client world-entry crash: the
/// ActivePlayerData stats VALUES update must NOT mask or write field bit 50
/// (ShieldBlockCritPercentage). That field is reserved in the 54261 client
/// grammar (oracle `hp_ObjectUpdateBuilder.cs:567,841` skip it); emitting it
/// shifted every following field by +4 bytes and desynced the client.
#[test]
fn active_player_stats_values_update_omits_reserved_bit_50() {
    let mut stats = zeroed_stat_changes();
    stats.shield_block = 20; // field bit 49
    stats.shield_block_crit_pct = 21.0; // field bit 50 (must be dropped)

    let mut values = WorldPacket::new_empty();
    write_active_player_data_values_update(&mut values, &[], &[], Some(&stats), None);
    let bytes = values.into_data();

    // block 1 mask is serialized big-endian at bytes[10..14]. Field bit 50 =
    // block-1 bit 18 MUST be clear, while bit 49 (ShieldBlock) and bit 51
    // (Mastery) MUST be set — proving only the reserved hole is skipped.
    let block1 = u32::from_be_bytes(bytes[10..14].try_into().unwrap());
    assert_eq!((block1 >> 18) & 1, 0, "field bit 50 must NOT be masked");
    assert_eq!(
        (block1 >> 17) & 1,
        1,
        "field bit 49 (ShieldBlock) must be masked"
    );
    assert_eq!(
        (block1 >> 19) & 1,
        1,
        "field bit 51 (Mastery) must be masked"
    );
}

/// A fully-zeroed `PlayerStatChanges` for focused serialization tests.
fn zeroed_stat_changes() -> PlayerStatChanges {
    PlayerStatChanges {
        health: 0,
        max_health: 0,
        min_damage: 0.0,
        max_damage: 0.0,
        base_mana: 0,
        base_health: 0,
        attack_power: 0,
        attack_power_mod_pos: 0,
        attack_power_mod_neg: 0,
        attack_power_multiplier: 0.0,
        ranged_attack_power: 0,
        ranged_attack_power_mod_pos: 0,
        ranged_attack_power_mod_neg: 0,
        ranged_attack_power_multiplier: 0.0,
        min_ranged_damage: 0.0,
        max_ranged_damage: 0.0,
        power0: 0,
        max_power0: 0,
        stats: [0; 5],
        stat_pos_buff: [0; 5],
        stat_neg_buff: [0; 5],
        armor: 0,
        combat_ratings: [0; 32],
        spell_power: 0,
        block_pct: 0.0,
        dodge_pct: 0.0,
        parry_pct: 0.0,
        crit_pct: 0.0,
        ranged_crit_pct: 0.0,
        spell_crit_pct: [0.0; 7],
        mana_regen: 0.0,
        mana_regen_combat: 0.0,
        mana_regen_mp5: 0.0,
        mainhand_expertise: 0.0,
        offhand_expertise: 0.0,
        ranged_expertise: 0.0,
        combat_rating_expertise: 0.0,
        dodge_from_attr: 0.0,
        parry_from_attr: 0.0,
        offhand_crit_pct: 0.0,
        shield_block: 0,
        shield_block_crit_pct: 0.0,
        mod_healing_pct: 0.0,
        mod_healing_done_pct: 0.0,
        mod_periodic_healing_pct: 0.0,
        mod_spell_power_pct: 0.0,
    }
}

#[test]
fn unit_stats_values_update_writes_cpp_ap_modifiers_and_negative_stat_buffs() {
    let mut stats = zeroed_stat_changes();
    stats.health = 1;
    stats.max_health = 2;
    stats.min_damage = 3.0;
    stats.max_damage = 4.0;
    stats.base_mana = 5;
    stats.base_health = 6;
    stats.attack_power = 7;
    stats.attack_power_mod_pos = 8;
    stats.attack_power_mod_neg = 9;
    stats.attack_power_multiplier = 10.0;
    stats.ranged_attack_power = 11;
    stats.ranged_attack_power_mod_pos = 12;
    stats.ranged_attack_power_mod_neg = 13;
    stats.ranged_attack_power_multiplier = 14.0;
    stats.min_ranged_damage = 15.0;
    stats.max_ranged_damage = 16.0;
    stats.mana_regen = 17.0;
    stats.mana_regen_combat = 18.0;
    stats.power0 = 19;
    stats.max_power0 = 20;
    stats.mana_regen_mp5 = 21.0;
    stats.stats = [22, 25, 28, 31, 34];
    stats.stat_pos_buff = [23, 26, 29, 32, 35];
    stats.stat_neg_buff = [24, 27, 30, 33, 36];
    stats.armor = 37;

    let mut values = WorldPacket::new_empty();
    write_unit_data_values_update(&mut values, &[], Some(&stats));
    let mut values = WorldPacket::from_bytes(&values.into_data());

    assert_eq!(values.read_bits(8).unwrap(), 0x3F);
    assert_eq!(
        values.read_bits(32).unwrap(),
        (1 << 0) | (1 << 5) | (1 << 6)
    );
    assert_eq!(
        values.read_bits(32).unwrap(),
        (1 << 0) | (1 << 20) | (1 << 21)
    );
    assert_eq!(
        values.read_bits(32).unwrap(),
        (1 << 0)
            | (1 << 11)
            | (1 << 12)
            | (1 << 17)
            | (1 << 18)
            | (1 << 19)
            | (1 << 20)
            | (1 << 21)
            | (1 << 22)
            | (1 << 23)
            | (1 << 24)
            | (1 << 27)
            | (1 << 28)
    );
    assert_eq!(
        values.read_bits(32).unwrap(),
        (1 << 20) | (1 << 21) | (1 << 31)
    );
    assert_eq!(
        values.read_bits(32).unwrap(),
        (1 << 9) | (1 << 19) | (1 << 29)
    );
    assert_eq!(values.read_bits(32).unwrap(), u32::MAX << 14);
    values.reset_bits();

    assert_eq!(values.read_int64().unwrap(), 1);
    assert_eq!(values.read_int64().unwrap(), 2);
    assert_eq!(values.read_float().unwrap(), 3.0);
    assert_eq!(values.read_float().unwrap(), 4.0);
    for expected in [5, 6, 7, 8, 9] {
        assert_eq!(values.read_int32().unwrap(), expected);
    }
    assert_eq!(values.read_float().unwrap(), 10.0);
    for expected in [11, 12, 13] {
        assert_eq!(values.read_int32().unwrap(), expected);
    }
    assert_eq!(values.read_float().unwrap(), 14.0);
    for expected in [15.0, 16.0, 17.0, 18.0] {
        assert_eq!(values.read_float().unwrap(), expected);
    }
    assert_eq!(values.read_int32().unwrap(), 19);
    assert_eq!(values.read_int32().unwrap(), 20);
    assert_eq!(values.read_float().unwrap(), 21.0);
    for expected in 22..=36 {
        assert_eq!(values.read_int32().unwrap(), expected);
    }
    assert_eq!(values.read_int32().unwrap(), 37);
    assert_eq!(values.remaining(), 0);
}

fn set_active_player_bit(data: &mut ActivePlayerDataValuesUpdate, bit: usize) {
    data.active_player_data_mask[bit / 32] |= 1 << (bit % 32);
}

#[test]
fn full_active_player_values_update_matches_cpp_coinage_shape() {
    let mut data = ActivePlayerDataValuesUpdate {
        coinage: 1234,
        ..Default::default()
    };
    set_active_player_bit(&mut data, 0);
    set_active_player_bit(&mut data, 28);

    let mut values = WorldPacket::new_empty();
    write_active_player_data_values_update_section(&mut values, &data);

    let bytes = values.into_data();
    assert_eq!(&bytes[0..4], &[0x01, 0x00, 0x00, 0x00]); // group 0: block 0
    assert_eq!(&bytes[4..6], &[0x00, 0x00]); // group 1: no blocks 32..47
    assert_eq!(&bytes[6..10], &[0x10, 0x00, 0x00, 0x01]); // block 0: bits 0 and 28
    assert_eq!(u64::from_le_bytes(bytes[10..18].try_into().unwrap()), 1234);
    assert_eq!(bytes.len(), 18);
}

#[test]
fn full_active_player_values_update_block_uses_active_player_type_mask() {
    let mut data = ActivePlayerDataValuesUpdate {
        coinage: 1234,
        ..Default::default()
    };
    set_active_player_bit(&mut data, 0);
    set_active_player_bit(&mut data, 28);

    let mut block = WorldPacket::new_empty();
    write_full_active_player_values_update_block(&mut block, &ObjectGuid::EMPTY, &data);

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_ACTIVE_PLAYER
    );
    assert_eq!(u64::from_le_bytes(bytes[21..29].try_into().unwrap()), 1234);
}

#[test]
fn full_active_player_values_update_matches_cpp_late_array_order() {
    let mut data = ActivePlayerDataValuesUpdate::default();
    set_active_player_bit(&mut data, 636);
    set_active_player_bit(&mut data, 637);
    set_active_player_bit(&mut data, 1512);
    set_active_player_bit(&mut data, 1513);
    set_active_player_bit(&mut data, 1519);
    set_active_player_bit(&mut data, 607);
    set_active_player_bit(&mut data, 608);
    data.quest_completed[0] = 0x0102_0304_0506_0708;
    data.glyph_slots[0] = 55;
    data.glyphs[0] = 66;
    data.pvp_info[0] = PvpInfoValuesUpdate {
        pvp_info_mask: 0x0D,
        bracket: 7,
        pvp_rating_id: 99,
        ..Default::default()
    };

    let mut values = WorldPacket::new_empty();
    write_active_player_data_values_update_section(&mut values, &data);

    let bytes = values.into_data();
    let quest_pos = bytes
        .windows(8)
        .position(|window| window == 0x0102_0304_0506_0708u64.to_le_bytes())
        .expect("QuestCompleted value must be present");
    let glyph_slot_pos = bytes
        .windows(4)
        .position(|window| window == 55u32.to_le_bytes())
        .expect("GlyphSlots value must be present");
    let glyph_pos = bytes
        .windows(4)
        .position(|window| window == 66u32.to_le_bytes())
        .expect("Glyphs value must be present");
    let pvp_pos = bytes
        .windows(4)
        .position(|window| window == 99i32.to_le_bytes())
        .expect("PVP rating value must be present");

    assert!(quest_pos < glyph_slot_pos);
    assert!(glyph_slot_pos < glyph_pos);
    assert!(glyph_pos < pvp_pos);
}

#[test]
fn skill_info_values_update_matches_cpp_mask_and_value_order() {
    let mut data = SkillInfoValuesUpdate::default();
    data.skill_info_mask[0] = (1 << 0) | (1 << 1);
    data.skill_info_mask[16] = 1 << 1; // global bit 513: SkillRank[0]
    data.skill_line_id[0] = 164;
    data.skill_rank[0] = 75;

    let mut values = WorldPacket::new_empty();
    write_skill_info_values_update(&mut values, &data);

    let bytes = values.into_data();
    assert_eq!(&bytes[0..4], &[0x01, 0x00, 0x01, 0x00]); // blocks 0 and 16
    assert_eq!(
        u16::from_le_bytes(bytes[bytes.len() - 4..bytes.len() - 2].try_into().unwrap()),
        164
    );
    assert_eq!(
        u16::from_le_bytes(bytes[bytes.len() - 2..].try_into().unwrap()),
        75
    );
}

#[test]
fn active_player_nested_simple_values_update_match_cpp_order() {
    let mut research = WorldPacket::new_empty();
    write_research_values_update(
        &mut research,
        ResearchValuesUpdate {
            research_project_id: -123,
        },
    );
    assert_eq!(
        i16::from_le_bytes(research.into_data().try_into().unwrap()),
        -123
    );

    let mut rest = WorldPacket::new_empty();
    write_rest_info_values_update(
        &mut rest,
        RestInfoValuesUpdate {
            rest_info_mask: 0x07,
            threshold: 10_000,
            state_id: 3,
        },
    );
    let rest_bytes = rest.into_data();
    assert_eq!(rest_bytes[0] & 0xE0, 0xE0); // 3-bit mask 0b111
    assert_eq!(
        u32::from_le_bytes(rest_bytes[1..5].try_into().unwrap()),
        10_000
    );
    assert_eq!(rest_bytes[5], 3);

    let mut pvp = WorldPacket::new_empty();
    write_pvp_info_values_update(
        &mut pvp,
        PvpInfoValuesUpdate {
            pvp_info_mask: 0x0F,
            disqualified: true,
            bracket: -1,
            pvp_rating_id: 42,
            ..Default::default()
        },
    );
    let pvp_bytes = pvp.into_data();
    assert_eq!(pvp_bytes[pvp_bytes.len() - 5], 0xFFu8); // Bracket i8
    assert_eq!(
        i32::from_le_bytes(pvp_bytes[pvp_bytes.len() - 4..].try_into().unwrap()),
        42
    );
}

#[test]
fn active_player_dynamic_entry_values_update_match_cpp_order() {
    let mut restriction = WorldPacket::new_empty();
    write_character_restriction_values_update(
        &mut restriction,
        CharacterRestrictionValuesUpdate {
            field_0: 1,
            field_4: 2,
            field_8: 3,
            restriction_type: 17,
        },
    );
    let restriction_bytes = restriction.into_data();
    assert_eq!(
        i32::from_le_bytes(restriction_bytes[0..4].try_into().unwrap()),
        1
    );
    assert_eq!(
        i32::from_le_bytes(restriction_bytes[4..8].try_into().unwrap()),
        2
    );
    assert_eq!(
        i32::from_le_bytes(restriction_bytes[8..12].try_into().unwrap()),
        3
    );
    assert_eq!(restriction_bytes[12] & 0xF8, 0x88); // 5-bit type 17.

    let mut pct = WorldPacket::new_empty();
    write_spell_pct_mod_by_label_values_update(
        &mut pct,
        SpellPctModByLabelValuesUpdate {
            mod_index: 4,
            modifier_value: 1.5,
            label_id: 6,
        },
    );
    let pct_bytes = pct.into_data();
    assert_eq!(i32::from_le_bytes(pct_bytes[0..4].try_into().unwrap()), 4);
    assert_eq!(f32::from_le_bytes(pct_bytes[4..8].try_into().unwrap()), 1.5);
    assert_eq!(i32::from_le_bytes(pct_bytes[8..12].try_into().unwrap()), 6);

    let mut flat = WorldPacket::new_empty();
    write_spell_flat_mod_by_label_values_update(
        &mut flat,
        SpellFlatModByLabelValuesUpdate {
            mod_index: 7,
            modifier_value: 8,
            label_id: 9,
        },
    );
    assert_eq!(flat.into_data(), [7, 0, 0, 0, 8, 0, 0, 0, 9, 0, 0, 0]);

    let mut cooldown = WorldPacket::new_empty();
    write_category_cooldown_mod_values_update(
        &mut cooldown,
        CategoryCooldownModValuesUpdate {
            spell_category_id: 10,
            mod_cooldown: 11,
        },
    );
    assert_eq!(cooldown.into_data(), [10, 0, 0, 0, 11, 0, 0, 0]);

    let mut weekly = WorldPacket::new_empty();
    write_weekly_spell_use_values_update(
        &mut weekly,
        WeeklySpellUseValuesUpdate {
            spell_category_id: 12,
            uses: 13,
        },
    );
    assert_eq!(weekly.into_data(), [12, 0, 0, 0, 13]);
}

#[test]
fn active_player_dynamic_nested_values_update_match_cpp_order() {
    let mut research_history = WorldPacket::new_empty();
    write_research_history_values_update(
        &mut research_history,
        &ResearchHistoryValuesUpdate {
            research_history_mask: 0x03,
            completed_projects: vec![CompletedProjectValuesUpdate {
                completed_project_mask: 0x0F,
                project_id: 101,
                first_completed: 202,
                completion_count: 3,
            }],
            completed_projects_update_mask: None,
        },
    );
    let rh = research_history.into_data();
    assert_eq!(
        u32::from_le_bytes(rh[rh.len() - 16..rh.len() - 12].try_into().unwrap()),
        101
    );
    assert_eq!(
        i64::from_le_bytes(rh[rh.len() - 12..rh.len() - 4].try_into().unwrap()),
        202
    );
    assert_eq!(
        u32::from_le_bytes(rh[rh.len() - 4..].try_into().unwrap()),
        3
    );

    let mut trait_config = WorldPacket::new_empty();
    write_trait_config_values_update(
        &mut trait_config,
        &TraitConfigValuesUpdate {
            trait_config_mask: 0x07F,
            entries: vec![TraitEntryValuesUpdate {
                trait_node_id: 1,
                trait_node_entry_id: 2,
                rank: 3,
                granted_ranks: 4,
            }],
            entries_update_mask: None,
            id: 55,
            name: "Spec".to_string(),
            config_type: 2,
            skill_line_id: 777,
            ..Default::default()
        },
    );
    let tc = trait_config.into_data();
    assert!(tc.windows(4).any(|window| window == [1, 0, 0, 0]));
    assert!(tc.windows(4).any(|window| window == [55, 0, 0, 0]));
    assert!(tc.windows(4).any(|window| window == 777u32.to_le_bytes()));
    assert!(tc.windows(4).any(|window| window == b"Spec"));

    let mut stable = WorldPacket::new_empty();
    write_stable_info_values_update(
        &mut stable,
        &StableInfoValuesUpdate {
            stable_info_mask: 0x07,
            pets: vec![StablePetInfoValuesUpdate {
                stable_pet_mask: 0xFF,
                pet_slot: 1,
                pet_number: 2,
                creature_id: 3,
                display_id: 4,
                experience_level: 5,
                name: "Pet".to_string(),
                pet_flags: 6,
            }],
            pets_update_mask: None,
            stable_master: ObjectGuid::EMPTY,
        },
    );
    let stable_bytes = stable.into_data();
    assert!(stable_bytes.windows(4).any(|window| window == [1, 0, 0, 0]));
    assert!(stable_bytes.windows(4).any(|window| window == [5, 0, 0, 0]));
    assert!(stable_bytes.windows(3).any(|window| window == b"Pet"));
}

fn test_player_create_data_with_farsight(farsight_object: ObjectGuid) -> PlayerCreateData {
    PlayerCreateData {
        guid: ObjectGuid::create_player(1, 42),
        wow_account: ObjectGuid::EMPTY,
        bnet_account: ObjectGuid::EMPTY,
        race: 1,
        class: 1,
        sex: 0,
        level: 1,
        display_id: 49,
        native_display_id: 49,
        health: 100,
        max_health: 100,
        faction_template: PlayerCreateData::faction_for_race(1),
        current_area_id: 12,
        player_flags: 0,
        player_flags_ex: 0,
        stats: [0; 5],
        stat_pos_buff: [0; 5],
        stat_neg_buff: [0; 5],
        base_armor: 0,
        base_mana: 0,
        max_mana: 0,
        current_power0: 1000,
        attack_power: 0,
        attack_power_mod_pos: 0,
        ranged_attack_power: 0,
        ranged_attack_power_mod_pos: 0,
        min_damage: 1.0,
        max_damage: 2.0,
        min_ranged_damage: 0.0,
        max_ranged_damage: 0.0,
        block_pct: 0.0,
        dodge_pct: 0.0,
        dodge_from_attr: 0.0,
        parry_pct: 0.0,
        parry_from_attr: 0.0,
        crit_pct: 5.0,
        ranged_crit_pct: 5.0,
        offhand_crit_pct: 5.0,
        spell_crit_pct: [5.0; 7],
        combat_ratings: [0; 32],
        spell_power: 0,
        visible_items: [(0, 0, 0); 19],
        customizations: Vec::new(),
        inv_slots: [ObjectGuid::EMPTY; 141],
        farsight_object,
        action_buttons: [0; MAX_ACTION_BUTTONS],
        skill_info: Vec::new(),
        quest_log: Vec::new(),
        party_type: [0; 2],
        coinage: 0,
        xp: 0,
        next_level_xp: 400,
        max_level: 80,
        scaling_player_level_delta: 0,
        rest_info: [
            RestInfoValuesUpdate {
                rest_info_mask: 0x07,
                threshold: 0,
                state_id: 2,
            },
            RestInfoValuesUpdate {
                rest_info_mask: 0x07,
                threshold: 0,
                state_id: 2,
            },
        ],
        watched_faction_index: -1,
        heirlooms: Vec::new(),
        heirloom_flags: Vec::new(),
        toys: Vec::new(),
        transmog: Vec::new(),
        trait_configs: Vec::new(),
    }
}

#[test]
fn player_create_writes_party_type_like_cpp() {
    let mut create = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    create.party_type = [17, 23];

    let mut packet = WorldPacket::new_empty();
    create.write_player_data(&mut packet, 0x03);

    assert!(
        packet
            .data()
            .windows(4)
            .any(|window| window == [17, 23, 0, create.sex]),
        "PlayerData::PartyType[2] must be serialized before NumBankSlots/NativeSex"
    );
}

#[test]
fn player_create_writes_loaded_player_flags_like_cpp() {
    let mut create = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    create.player_flags = 0x22;
    create.player_flags_ex = 0x04;

    let mut packet = WorldPacket::new_empty();
    create.write_player_data(&mut packet, 0x03);

    assert!(
        packet
            .data()
            .windows(8)
            .any(|window| window == [0x22, 0, 0, 0, 0x04, 0, 0, 0]),
        "C++ PlayerData::WriteCreate serializes loaded PlayerFlags/PlayerFlagsEx"
    );
}

#[test]
fn player_create_playerdata_self_layout_ends_with_dungeon_score_like_cpp() {
    let create = test_player_create_data_with_farsight(ObjectGuid::EMPTY);

    let mut packet = WorldPacket::new_empty();
    create.write_player_data(&mut packet, 0x03);
    let bytes = packet.data();

    // C++ `UF::PlayerData::WriteCreate` with empty dynamic arrays and
    // self-view PartyMember fields:
    // - fixed header/account/flags/customization/party fields: 50 bytes
    // - QuestLog[25]: 25 * (i64 + i32 + u32 + 24*u16) = 1600 bytes
    // - VisibleItems[19]: 19 * (i32 + u16 + u16) = 152 bytes
    // - fixed tail through Field_3120[19]: 147 bytes
    // - DungeonScoreSummary: f32 + f32 + u32 = 12 bytes
    const EMPTY_SELF_PLAYER_DATA_LEN: usize = 50 + 1600 + 152 + 147 + 12;
    assert_eq!(bytes.len(), EMPTY_SELF_PLAYER_DATA_LEN);

    let dungeon_score = &bytes[bytes.len() - 12..];
    assert_eq!(
        dungeon_score, &[0; 12],
        "empty C++ DungeonScoreSummary is two zero f32 values plus zero Runs count"
    );
}

#[test]
fn player_create_unitdata_owner_layout_matches_cpp_field_count() {
    let mut create = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    create.visible_items[15] = (0x0102_0304, 0x0506, 0x0708);

    let mut packet = WorldPacket::new_empty();
    create.write_unit_data(&mut packet, 0x01);
    let bytes = packet.data();

    // C++ `UF::UnitData::WriteCreate` with `UpdateFieldFlag::Owner`
    // and empty dynamic arrays/packed GUIDs serializes 823 bytes.
    // This covers the shared player/creature UnitData create layout.
    const OWNER_UNIT_DATA_LEN: usize = 823;
    assert_eq!(bytes.len(), OWNER_UNIT_DATA_LEN);

    assert!(
        bytes
            .windows(8)
            .any(|window| window == [0x04, 0x03, 0x02, 0x01, 0x06, 0x05, 0x08, 0x07]),
        "C++ VisibleItem/VirtualItems order is ItemID(i32), AppearanceMod(u16), ItemVisual(u16)"
    );
}

#[test]
fn player_create_writes_customizations_like_cpp() {
    let without_customizations = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    let mut with_customizations = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    with_customizations.customizations = vec![
        ChrCustomizationChoiceValuesUpdate {
            option_id: 110,
            choice_id: 17913,
        },
        ChrCustomizationChoiceValuesUpdate {
            option_id: 111,
            choice_id: 17929,
        },
    ];

    let mut base_packet = WorldPacket::new_empty();
    without_customizations.write_player_data(&mut base_packet, 0x03);
    let base_len = base_packet.data().len();

    let mut packet = WorldPacket::new_empty();
    with_customizations.write_player_data(&mut packet, 0x03);
    let bytes = packet.data();

    assert_eq!(bytes.len(), base_len + 16);
    assert!(
        bytes
            .windows(8)
            .any(|window| window == [110, 0, 0, 0, 249, 69, 0, 0]),
        "PlayerData::Customizations must serialize option/choice uint32 pairs"
    );
    assert!(
        bytes
            .windows(8)
            .any(|window| window == [111, 0, 0, 0, 9, 70, 0, 0]),
        "PlayerData::Customizations must preserve DB order"
    );
}

#[test]
fn player_create_writes_account_guids_like_cpp() {
    let without_account_guids = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    let mut with_account_guids = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    with_account_guids.wow_account =
        ObjectGuid::create_global(wow_core::guid::HighGuid::WowAccount, 0, 1);
    with_account_guids.bnet_account =
        ObjectGuid::create_global(wow_core::guid::HighGuid::BNetAccount, 0, 1);

    let mut base_packet = WorldPacket::new_empty();
    without_account_guids.write_player_data(&mut base_packet, 0x03);
    let base_len = base_packet.data().len();

    let mut packet = WorldPacket::new_empty();
    with_account_guids.write_player_data(&mut packet, 0x03);

    assert_eq!(packet.data().len(), base_len + 4);
}

#[test]
fn active_player_create_writes_farsight_after_inventory_slots() {
    let farsight_object = ObjectGuid::new(0x0102_0304_0506_0708, 0x1112_1314_1516_1718);
    let create = test_player_create_data_with_farsight(farsight_object);
    let mut packet = WorldPacket::new_empty();
    create.write_active_player_data(&mut packet);
    let data = packet.data();

    let mut expected_guid = WorldPacket::new_empty();
    expected_guid.write_packed_guid(&farsight_object);
    let expected_guid = expected_guid.into_data();
    let farsight_offset = 141 * 2;
    let summoned_battle_pet_offset = farsight_offset + expected_guid.len();

    assert_ne!(expected_guid, [0, 0]);
    assert_eq!(
        &data[farsight_offset..summoned_battle_pet_offset],
        expected_guid.as_slice()
    );
    assert_eq!(
        &data[summoned_battle_pet_offset..summoned_battle_pet_offset + 2],
        [0, 0]
    );
}

#[test]
fn active_player_create_empty_layout_matches_cpp_trace_offsets() {
    let create = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    let mut packet = WorldPacket::new_empty();
    create.write_active_player_data(&mut packet);
    let data = packet.data();

    // These offsets mirror the trace labels in C++
    // `UF::ActivePlayerData::WriteCreate` for an empty/default player.
    const INV_SLOTS_END: usize = 141 * 2;
    const FARSIGHT_BATTLEPET_END: usize = INV_SLOTS_END + 2 + 2;
    const SKILL_END: usize = FARSIGHT_BATTLEPET_END + 4 + 8 + 4 + 4 + 4 + (256 * 14);
    const EXPLORED_ZONES_END: usize = 6034;
    const BUYBACK_END: usize = 6268;
    const COMBAT_RATINGS_END: usize = 6444;
    const QUEST_COMPLETED_END: usize = 13561;
    const DYNAMIC_SIZES_END: usize = 13721;
    const PVP_INFO_END: usize = 14183;
    const RESEARCH_HISTORY_END: usize = 14188;
    const FROZEN_PERKS_END: usize = 14229;

    assert_eq!(data.len(), FROZEN_PERKS_END);
    assert_eq!(FARSIGHT_BATTLEPET_END, 286);
    assert_eq!(SKILL_END, 3894);
    assert_eq!(&data[INV_SLOTS_END..FARSIGHT_BATTLEPET_END], &[0, 0, 0, 0]);
    assert_eq!(&data[SKILL_END - 14..SKILL_END], &[0; 14]);
    assert_eq!(&data[EXPLORED_ZONES_END - 8..EXPLORED_ZONES_END], &[0; 8]);
    assert_eq!(&data[BUYBACK_END - 8..BUYBACK_END], &[0; 8]);
    assert_eq!(&data[COMBAT_RATINGS_END - 4..COMBAT_RATINGS_END], &[0; 4]);
    assert_eq!(&data[QUEST_COMPLETED_END - 8..QUEST_COMPLETED_END], &[0; 8]);
    assert_eq!(&data[DYNAMIC_SIZES_END - 1..DYNAMIC_SIZES_END], &[0]);
    assert_eq!(&data[PVP_INFO_END - 1..PVP_INFO_END], &[0]);
    assert_eq!(
        &data[RESEARCH_HISTORY_END - 4..RESEARCH_HISTORY_END],
        &[0; 4]
    );
    assert_eq!(&data[FROZEN_PERKS_END - 9..FROZEN_PERKS_END], &[0; 9]);
}

#[test]
fn active_player_create_writes_collection_dynamic_fields_like_cpp() {
    let mut create = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    create.heirlooms = vec![44_000, 44_001];
    create.heirloom_flags = vec![0x03, 0x04];
    create.toys = vec![30_000];
    create.transmog = vec![0x2000_0000, 0, 0x01];

    let mut packet = WorldPacket::new_empty();
    create.write_active_player_data(&mut packet);
    let data = packet.data();

    assert!(data.windows(16).any(|window| {
        window
            == [
                2, 0, 0, 0, // Heirlooms.Size
                2, 0, 0, 0, // HeirloomFlags.Size
                1, 0, 0, 0, // Toys.Size
                3, 0, 0, 0, // Transmog.Size
            ]
    }));
    assert!(
        data.windows(12)
            .any(|window| window == [224, 171, 0, 0, 225, 171, 0, 0, 3, 0, 0, 0])
    );
    assert!(
        data.windows(8)
            .any(|window| window == [4, 0, 0, 0, 48, 117, 0, 0])
    );
    assert!(
        data.windows(12)
            .any(|window| window == [0, 0, 0, 32, 0, 0, 0, 0, 1, 0, 0, 0])
    );
}

#[test]
fn active_player_create_writes_cpp_transmog_and_trait_config_dynamic_payloads() {
    let mut baseline = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    let mut packet = WorldPacket::new_empty();
    baseline.write_active_player_data(&mut packet);
    let baseline_len = packet.data().len();

    baseline.transmog = vec![0; 5528];
    baseline.trait_configs = vec![
        TraitConfigCreateData {
            id: 1,
            config_type: 1,
            skill_line_id: 0,
            chr_specialization_id: 256,
            combat_config_flags: 1,
            local_identifier: 1,
            trait_system_id: 0,
            name: String::new(),
            entries: Vec::new(),
        },
        TraitConfigCreateData {
            id: 2,
            config_type: 1,
            skill_line_id: 0,
            chr_specialization_id: 257,
            combat_config_flags: 1,
            local_identifier: 1,
            trait_system_id: 0,
            name: String::new(),
            entries: Vec::new(),
        },
        TraitConfigCreateData {
            id: 3,
            config_type: 1,
            skill_line_id: 0,
            chr_specialization_id: 258,
            combat_config_flags: 1,
            local_identifier: 1,
            trait_system_id: 0,
            name: String::new(),
            entries: Vec::new(),
        },
    ];

    let mut packet = WorldPacket::new_empty();
    baseline.write_active_player_data(&mut packet);
    let data = packet.data();

    assert_eq!(data.len() - baseline_len, 5528 * 4 + 3 * 26);
}

#[test]
fn create_player_defaults_farsight_object_empty() {
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let packet = UpdateObject::create_player(
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

    let UpdateBlock::CreateObject { create_data, .. } = &packet.blocks[0] else {
        panic!("create_player should emit one CreateObject block");
    };
    assert_eq!(create_data.farsight_object, ObjectGuid::EMPTY);
}

#[test]
fn create_player_self_current_power_can_use_saved_db_value_like_cpp() {
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let mut combat = PlayerCombatStats::default();
    combat.max_mana = 1000;
    combat.base_mana = 155;
    let mut packet = UpdateObject::create_player(
        guid,
        1,
        5,
        0,
        1,
        49,
        &pos,
        0,
        12,
        true,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        combat,
        Vec::new(),
        0,
        Vec::new(),
    );

    packet.set_player_current_power0_like_cpp(321);

    let UpdateBlock::CreateObject { create_data, .. } = &packet.blocks[0] else {
        panic!("create_player should emit one CreateObject block");
    };
    assert_eq!(
        create_data.current_power_for_slot0(),
        321,
        "C++ login self UpdateObject serializes current UnitData::Power[0] from characters.power1"
    );
    assert_eq!(
        create_data.max_power_for_slot0(),
        1000,
        "current power must not overwrite UnitData::MaxPower[0]"
    );
    assert_eq!(
        create_data.base_mana_for_create_like_cpp(),
        155,
        "C++ BaseMana keeps GtBaseMP separate from intellect-inflated MaxPower"
    );
}

#[test]
fn create_player_self_xp_can_use_saved_db_value_like_cpp() {
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let mut packet = UpdateObject::create_player(
        guid,
        1,
        1,
        0,
        10,
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

    packet.set_player_xp_like_cpp(1_234);
    packet.set_player_max_level_like_cpp(70);
    packet.set_player_scaling_level_delta_like_cpp(-1);

    let UpdateBlock::CreateObject { create_data, .. } = &packet.blocks[0] else {
        panic!("create_player should emit one CreateObject block");
    };
    assert_eq!(create_data.xp, 1_234);
    assert_eq!(create_data.max_level, 70);
    assert_eq!(create_data.scaling_player_level_delta, -1);

    let mut active = WorldPacket::new_empty();
    create_data.write_active_player_data(&mut active);
    let active = active.into_data();
    assert_eq!(
        i32::from_le_bytes(active[298..302].try_into().unwrap()),
        1_234
    );
    assert_eq!(
        i32::from_le_bytes(active[6_444..6_448].try_into().unwrap()),
        70
    );
    assert_eq!(
        i32::from_le_bytes(active[6_448..6_452].try_into().unwrap()),
        -1
    );
}

#[test]
fn create_player_non_self() {
    // Non-self player should be smaller (no ActivePlayerData)
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let self_pkt = UpdateObject::create_player(
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
    let other_pkt = UpdateObject::create_player(
        guid,
        1,
        1,
        0,
        1,
        49,
        &pos,
        0,
        12,
        false,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    let self_bytes = self_pkt.to_bytes();
    let other_bytes = other_pkt.to_bytes();
    // Self packet should be much larger due to ActivePlayerData
    assert!(
        self_bytes.len() > other_bytes.len() + 1000,
        "Self ({}) should be much larger than other ({})",
        self_bytes.len(),
        other_bytes.len()
    );
}

#[test]
fn power_type_mapping() {
    assert_eq!(power_type_for_class(1), 1); // Warrior → Rage
    assert_eq!(power_type_for_class(2), 0); // Paladin → Mana
    assert_eq!(power_type_for_class(4), 3); // Rogue → Energy
    // DeathKnight DisplayPower = POWER_RUNIC_POWER (6), NOT POWER_RUNES (5) — C++
    // CalculateDisplayPowerType / ChrClasses (SharedDefines.h:287). #NEXT.R8.ENTITIES.1213.
    assert_eq!(power_type_for_class(6), 6); // DK → Runic Power
}

#[test]
fn player_unit_data_health_aura_state_matches_cpp_modify_aura_state() {
    // #NEXT.R8.ENTITIES.1212 — C++ Unit::Update/ModifyAuraState seeds health-based aura
    // states on EVERY alive unit incl. the player (Unit.cpp:469-476). Full HP => 0x00D00000.
    assert_eq!(health_aura_state_like_cpp(100, 100, true), 0x00D0_0000);
    assert_eq!(health_aura_state_like_cpp(0, 100, false), 0); // dead
    assert_eq!(health_aura_state_like_cpp(50, 0, true), 0); // no max
    // Low HP (<=20%): WOUND_HEALTH_20_80 (0x100000) set, HEALTHY_75 clear.
    let low = health_aura_state_like_cpp(10, 100, true);
    assert_ne!(low & 0x0010_0000, 0);
    assert_eq!(low & 0x0040_0000, 0);
}

#[test]
fn creature_create_serializes() {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 1234, 5678);
    let pos = Position::new(-8949.0, -132.0, 83.0, 0.0);
    let data = CreatureCreateData {
        guid,
        entry: 1234,
        display_id: 856,
        native_display_id: 856,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 500,
        max_health: 500,
        level: 5,
        faction_template: 14,
        npc_flags: 0,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        aura_state: 0x00D0_0000,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        scale: 1.0,
        unit_class: 1,
        display_power: 1,
        power: [0; 10],
        max_power: [0; 10],
        base_mana: 0,
        virtual_items: [(0, 0, 0); 3],
        base_attack_time: 2000,
        ranged_attack_time: 0,
        movement_flags: 0,
        vehicle_id: 0,
        play_hover_anim: false,
        hover_height: 1.0,
        mount_display_id: 0,
        stand_state: 0,
        vis_flags: 0,
        anim_tier: 0,
        emote_state: 0,
        sheathe_state: wow_constants::unit::SheathState::Melee as u8,
        pvp_flags: 0,
        current_area_id: 12,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    };
    let mut values = WorldPacket::new_empty();
    data.write_values_create(&mut values);
    let values = values.into_data();
    assert_eq!(
        values.len(),
        536,
        "C++ Unit::BuildValuesCreate writes size prefix plus 532 bytes for base creature ObjectData+UnitData"
    );
    // Regression for the world-entry ERROR #132 render-worker NULL deref: every creature
    // CREATE must carry StateAnimID = 1772 (C++ Creature::UpdateEntry seeds
    // DB2Manager::GetEmptyAnimStateID(); DB2Stores.cpp:1765 hardcodes 1772 because the
    // Classic client expects the retail AnimationData storage size). StateSpellVisualID
    // and StateAnimKitID stay 0. Shipping StateAnimID=0 crashed the 3.4.3 client ~4s in-world.
    assert!(
        values
            .windows(12)
            .any(|w| w == [0, 0, 0, 0, 0xEC, 0x06, 0, 0, 0, 0, 0, 0]),
        "creature CREATE must serialize StateSpellVisualID=0, StateAnimID=1772, StateAnimKitID=0"
    );
    assert_eq!(&values[0..4], &532u32.to_le_bytes());
    assert_eq!(values[4], 0, "creature create uses no owner/party flags");

    let block = UpdateObject::create_creature_block(data.clone(), &pos);
    let pkt = UpdateObject::create_creatures(vec![block], 0);
    let bytes = pkt.to_bytes();
    // Creature packet should be much smaller than player (no PlayerData/ActivePlayerData)
    assert!(
        bytes.len() > 100,
        "Creature packet too small: {} bytes",
        bytes.len()
    );
    assert!(
        bytes.len() < 2000,
        "Creature packet too large: {} bytes",
        bytes.len()
    );

    let mut vehicle_data = data.clone();
    vehicle_data.vehicle_id = 686;
    let normal_block = UpdateObject::create_creature_block(data, &pos);
    let vehicle_block = UpdateObject::create_creature_block(vehicle_data, &pos);
    let normal = UpdateObject::create_creatures(vec![normal_block], 0).to_bytes();
    let vehicle = UpdateObject::create_creatures(vec![vehicle_block], 0).to_bytes();
    let mut expected_vehicle_payload = Vec::new();
    expected_vehicle_payload.extend_from_slice(&686u32.to_le_bytes());
    expected_vehicle_payload.extend_from_slice(&pos.orientation.to_le_bytes());
    assert_eq!(
        vehicle.len(),
        normal.len() + 8,
        "C++ CreateObjectBits::Vehicle writes VehicleRecID plus InitialRawFacing"
    );
    assert!(
        vehicle
            .windows(expected_vehicle_payload.len())
            .any(|window| window == expected_vehicle_payload),
        "vehicle create block must include the C++ VehicleRecID/orientation payload"
    );
}

#[test]
fn creature_create_serializes_cpp_anim_kit_block_when_any_anim_kit_is_set() {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 1234, 5678);
    let pos = Position::new(-8949.0, -132.0, 83.0, 0.0);
    let data = CreatureCreateData {
        guid,
        entry: 1234,
        display_id: 856,
        native_display_id: 856,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 500,
        max_health: 500,
        level: 5,
        faction_template: 14,
        npc_flags: 0,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        aura_state: 0x00D0_0000,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        scale: 1.0,
        unit_class: 1,
        display_power: 1,
        power: [0; 10],
        max_power: [0; 10],
        base_mana: 0,
        virtual_items: [(0, 0, 0); 3],
        base_attack_time: 2000,
        ranged_attack_time: 0,
        movement_flags: 0,
        vehicle_id: 0,
        play_hover_anim: false,
        hover_height: 1.0,
        mount_display_id: 0,
        stand_state: 0,
        vis_flags: 0,
        anim_tier: 0,
        emote_state: 0,
        sheathe_state: wow_constants::unit::SheathState::Melee as u8,
        pvp_flags: 0,
        current_area_id: 12,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 11,
        movement_anim_kit_id: 22,
        melee_anim_kit_id: 33,
    };
    let block = UpdateObject::create_creature_block(data, &pos);
    let pkt = UpdateObject::create_creatures(vec![block], 0);
    let bytes = pkt.to_bytes();

    assert!(
        bytes
            .windows(6)
            .any(|window| window == [11, 0, 22, 0, 33, 0]),
        "C++ CreateObjectBits::AnimKit writes AiID, MovementID, MeleeID as u16 payload"
    );
}

#[test]
fn creature_create_serializes_cpp_addon_unit_fields() {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 1234, 1);
    let pos = Position::new(1.0, 2.0, 3.0, 4.0);
    let data = CreatureCreateData {
        guid,
        entry: 1234,
        display_id: 856,
        native_display_id: 856,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 500,
        max_health: 500,
        level: 5,
        faction_template: 14,
        npc_flags: 0,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        aura_state: 0x00D0_0000,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        scale: 1.0,
        unit_class: 1,
        display_power: 1,
        power: [0; 10],
        max_power: [0; 10],
        base_mana: 0,
        virtual_items: [(0, 0, 0); 3],
        base_attack_time: 2000,
        ranged_attack_time: 0,
        movement_flags: 0,
        vehicle_id: 0,
        play_hover_anim: false,
        hover_height: 1.25,
        mount_display_id: 0x0102_0304,
        stand_state: 2,
        vis_flags: 0x12,
        anim_tier: 3,
        emote_state: 0x1122_3344,
        sheathe_state: 1,
        pvp_flags: 5,
        current_area_id: 12,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    };
    let bytes =
        UpdateObject::create_creatures(vec![UpdateObject::create_creature_block(data, &pos)], 0)
            .to_bytes();

    assert!(
        bytes.windows(4).any(|window| window == [4, 3, 2, 1]),
        "C++ UnitData::WriteCreate writes MountDisplayID after native display scale"
    );
    assert!(
        bytes
            .windows(8)
            .any(|window| window == [2, 0, 0x12, 3, 0, 0, 0, 0]),
        "C++ UnitData::WriteCreate writes StandState/PetTalentPoints/VisFlags/AnimTier followed by PetNumber"
    );
    assert!(
        !bytes
            .windows(5)
            .any(|window| window == [2, 0, 0x12, 0x12, 3]),
        "C++ UnitData::WriteCreate writes VisFlags once, not twice"
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == [0x44, 0x33, 0x22, 0x11]),
        "C++ UnitData::WriteCreate writes EmoteState"
    );
    let mut mod_time_rate_sequence = Vec::new();
    for _ in 0..6 {
        mod_time_rate_sequence.extend_from_slice(&1.0f32.to_le_bytes());
    }
    mod_time_rate_sequence.extend_from_slice(&0i32.to_le_bytes());
    mod_time_rate_sequence.extend_from_slice(&0x1122_3344i32.to_le_bytes());
    assert!(
        bytes
            .windows(mod_time_rate_sequence.len())
            .any(|window| window == mod_time_rate_sequence),
        "C++ UnitData::WriteCreate writes six speed/haste/time-rate floats before CreatedBySpell and EmoteState"
    );
    assert!(
        bytes.windows(4).any(|window| window == [1, 5, 0, 0]),
        "C++ UnitData::WriteCreate writes SheatheState/PvpFlags/PetFlags/ShapeshiftForm"
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == 1.25f32.to_le_bytes()),
        "C++ UnitData::WriteCreate writes UnitData::HoverHeight from CreatureModelData"
    );
}

#[test]
fn creature_create_serializes_cpp_power_and_max_power_arrays() {
    let mut power = [0; 10];
    let mut max_power = [0; 10];
    power[0] = 77;
    max_power[0] = 123;
    let data = CreatureCreateData {
        guid: ObjectGuid::EMPTY,
        entry: 1234,
        display_id: 856,
        native_display_id: 856,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 500,
        max_health: 500,
        level: 5,
        faction_template: 14,
        npc_flags: 0,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        aura_state: 0x00D0_0000,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        scale: 1.0,
        unit_class: 1,
        display_power: 0,
        power,
        max_power,
        base_mana: 123,
        virtual_items: [(0, 0, 0); 3],
        base_attack_time: 2000,
        ranged_attack_time: 0,
        movement_flags: 0,
        vehicle_id: 0,
        play_hover_anim: false,
        hover_height: 1.0,
        mount_display_id: 0,
        stand_state: 0,
        vis_flags: 0,
        anim_tier: 0,
        emote_state: 0,
        sheathe_state: wow_constants::unit::SheathState::Melee as u8,
        pvp_flags: 0,
        current_area_id: 0,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    };
    let bytes = UpdateObject::create_creatures(
        vec![UpdateObject::create_creature_block(data, &Position::ZERO)],
        0,
    )
    .to_bytes();
    let mut expected = Vec::new();
    expected.extend_from_slice(&77i32.to_le_bytes());
    expected.extend_from_slice(&123i32.to_le_bytes());
    expected.extend_from_slice(&0.0f32.to_le_bytes());
    assert!(
        bytes
            .windows(expected.len())
            .any(|window| window == expected),
        "C++ UnitData::WriteCreate writes Power[0], MaxPower[0], ModPowerRegen[0]"
    );
}

#[test]
fn creature_create_serializes_play_hover_anim_bit_like_cpp() {
    let mut data = CreatureCreateData {
        guid: ObjectGuid::EMPTY,
        entry: 1234,
        display_id: 856,
        native_display_id: 856,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 500,
        max_health: 500,
        level: 5,
        faction_template: 14,
        npc_flags: 0,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        aura_state: 0x00D0_0000,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        scale: 1.0,
        unit_class: 1,
        display_power: 1,
        power: [0; 10],
        max_power: [0; 10],
        base_mana: 0,
        virtual_items: [(0, 0, 0); 3],
        base_attack_time: 2000,
        ranged_attack_time: 0,
        movement_flags: wow_constants::movement::MovementFlag::HOVER.bits(),
        vehicle_id: 0,
        play_hover_anim: false,
        hover_height: 1.25,
        mount_display_id: 0,
        stand_state: 0,
        vis_flags: 0,
        anim_tier: 0,
        emote_state: 0,
        sheathe_state: wow_constants::unit::SheathState::Melee as u8,
        pvp_flags: 0,
        current_area_id: 12,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    };
    let movement = MovementBlock {
        position: Position::ZERO,
        movement_flags: data.movement_flags,
        ..Default::default()
    };
    let mut without_hover = WorldPacket::new_empty();
    write_creature_create_block(&mut without_hover, &ObjectGuid::EMPTY, &movement, &data);
    data.play_hover_anim = true;
    let mut with_hover = WorldPacket::new_empty();
    write_creature_create_block(&mut with_hover, &ObjectGuid::EMPTY, &movement, &data);

    let diffs = without_hover
        .data()
        .iter()
        .zip(with_hover.data())
        .filter_map(|(left, right)| {
            let diff = left ^ right;
            (diff != 0).then_some(diff)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        diffs,
        vec![1 << 5],
        "the third C++ CreateObjectBits WriteBit toggles PlayHoverAnim"
    );
}

#[test]
fn creature_smaller_than_player() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 100, 1);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);

    let player_pkt = UpdateObject::create_player(
        player_guid,
        1,
        1,
        0,
        1,
        49,
        &pos,
        0,
        12,
        false,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );

    let creature_data = CreatureCreateData {
        guid: creature_guid,
        entry: 100,
        display_id: 856,
        native_display_id: 856,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 100,
        max_health: 100,
        level: 1,
        faction_template: 14,
        npc_flags: 0,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        aura_state: 0x00D0_0000,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        scale: 1.0,
        unit_class: 1,
        display_power: 1,
        power: [0; 10],
        max_power: [0; 10],
        base_mana: 0,
        virtual_items: [(0, 0, 0); 3],
        base_attack_time: 2000,
        ranged_attack_time: 0,
        movement_flags: 0,
        vehicle_id: 0,
        play_hover_anim: false,
        hover_height: 1.0,
        mount_display_id: 0,
        stand_state: 0,
        vis_flags: 0,
        anim_tier: 0,
        emote_state: 0,
        sheathe_state: wow_constants::unit::SheathState::Melee as u8,
        pvp_flags: 0,
        current_area_id: 12,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    };
    let block = UpdateObject::create_creature_block(creature_data, &pos);
    let creature_pkt = UpdateObject::create_creatures(vec![block], 0);

    let player_bytes = player_pkt.to_bytes();
    let creature_bytes = creature_pkt.to_bytes();

    // Creature has no PlayerData, so it should be smaller than even a non-self player
    assert!(
        creature_bytes.len() < player_bytes.len(),
        "Creature ({}) should be smaller than non-self player ({})",
        creature_bytes.len(),
        player_bytes.len()
    );
}

#[test]
fn creature_batched_multiple() {
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let mut blocks = Vec::new();
    for i in 0..5 {
        let guid =
            ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 100, i);
        let data = CreatureCreateData {
            guid,
            entry: 100,
            display_id: 856,
            native_display_id: 856,
            display_scale: 1.0,
            native_x_display_scale: 1.0,
            bounding_radius: 0.389,
            combat_reach: 1.5,
            health: 100,
            max_health: 100,
            level: 1,
            faction_template: 14,
            npc_flags: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            aura_state: 0x00D0_0000,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            scale: 1.0,
            unit_class: 1,
            display_power: 1,
            power: [0; 10],
            max_power: [0; 10],
            base_mana: 0,
            virtual_items: [(0, 0, 0); 3],
            base_attack_time: 2000,
            ranged_attack_time: 0,
            movement_flags: 0,
            vehicle_id: 0,
            play_hover_anim: false,
            hover_height: 1.0,
            mount_display_id: 0,
            stand_state: 0,
            vis_flags: 0,
            anim_tier: 0,
            emote_state: 0,
            sheathe_state: wow_constants::unit::SheathState::Melee as u8,
            pvp_flags: 0,
            current_area_id: 12,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
        };
        blocks.push(UpdateObject::create_creature_block(data, &pos));
    }
    let pkt = UpdateObject::create_creatures(blocks, 0);
    let bytes = pkt.to_bytes();

    // 5 creatures should be 5x the single creature data
    assert!(
        bytes.len() > 500,
        "Batched packet too small: {} bytes",
        bytes.len()
    );

    // Check num_updates = 5
    let num_updates = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
    assert_eq!(num_updates, 5);
}

#[test]
fn creature_npc_flags_written_correctly() {
    // Verify that NpcFlags value appears in the creature's values block.
    // NpcFlags=1 (Gossip) should be written as 0x01000000 in the packet.
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 3296, 1);
    let pos = Position::new(1600.0, -4400.0, 10.0, 0.0);
    let data = CreatureCreateData {
        guid,
        entry: 3296,
        display_id: 4500,
        native_display_id: 4500,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 500,
        max_health: 500,
        level: 55,
        faction_template: 85,
        npc_flags: 0x1_0000_0001, // Gossip flag plus NPCFlags2 bit 0
        unit_flags: 32768,
        unit_flags2: 2048,
        unit_flags3: 0,
        aura_state: 0x00D0_0000,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        scale: 1.0,
        unit_class: 1,
        display_power: 1,
        power: [0; 10],
        max_power: [0; 10],
        base_mana: 0,
        virtual_items: [(0, 0, 0); 3],
        base_attack_time: 2000,
        ranged_attack_time: 0,
        movement_flags: 0,
        vehicle_id: 0,
        play_hover_anim: false,
        hover_height: 1.0,
        mount_display_id: 0,
        stand_state: 0,
        vis_flags: 0,
        anim_tier: 0,
        emote_state: 0,
        sheathe_state: wow_constants::unit::SheathState::Melee as u8,
        pvp_flags: 0,
        current_area_id: 1637,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    };
    let block = UpdateObject::create_creature_block(data, &pos);
    let pkt = UpdateObject::create_creatures(vec![block], 1);
    let bytes = pkt.to_bytes();

    // Find NpcFlags=1 in the packet bytes.
    // The values block contains:
    //   [u8 flags=0x00]
    //   [i32 EntryId] [u32 DynamicFlags] [f32 Scale]  (ObjectData: 4+4+4=12 bytes)
    //   [i64 Health] [i64 MaxHealth] [i32 DisplayId]   (UnitData: 8+8+4=20 bytes)
    //   [u32 NpcFlags[0]] [u32 NpcFlags[1]]            (UnitData: 4+4=8 bytes)
    // So NpcFlags[0] starts at offset 1+12+20 = 33 from values block start.
    // The value 1 in little-endian is [0x01, 0x00, 0x00, 0x00].
    // Search for this pattern preceded by DisplayId (4500 = 0x94110000 LE).
    let display_le = 4500u32.to_le_bytes();
    let npc_le = 1u32.to_le_bytes();
    let npc2_le = 1u32.to_le_bytes();
    let mut found = false;
    for i in 0..bytes.len().saturating_sub(8) {
        if bytes[i..i + 4] == display_le && bytes[i + 4..i + 8] == npc_le {
            found = true;
            // Also check NpcFlags[1] = 1
            assert_eq!(bytes[i + 8..i + 12], npc2_le, "NpcFlags[1] should be 1");
            break;
        }
    }
    assert!(
        found,
        "NpcFlags=1 not found after DisplayId={} in packet ({} bytes). \
        This means NpcFlags are not being written correctly!",
        4500,
        bytes.len()
    );
}

#[test]
fn active_player_movement_block_adds_721_bytes() {
    // Self-view packets include a 721-byte ActivePlayer block in
    // BuildMovementUpdate: 1 byte (3 bits + flush) + 180 action buttons (720 bytes).
    // Non-self packets don't have this block.
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let self_pkt = UpdateObject::create_player(
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
    let other_pkt = UpdateObject::create_player(
        guid,
        1,
        1,
        0,
        1,
        49,
        &pos,
        0,
        12,
        false,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    let self_bytes = self_pkt.to_bytes();
    let other_bytes = other_pkt.to_bytes();

    // The difference between self and non-self should include:
    // - 721 bytes from ActivePlayer movement block
    // - plus the ActivePlayerData values block difference
    // The ActivePlayer movement block alone is 721 bytes.
    let diff = self_bytes.len() - other_bytes.len();
    assert!(
        diff > 721,
        "Self/non-self difference ({}) should be > 721 (ActivePlayer block)",
        diff
    );
}

#[test]
fn active_player_movement_block_writes_loaded_action_buttons_like_cpp() {
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let mut pkt = UpdateObject::create_player(
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
    let sentinel = 0xA1B2_C3D4u32;
    let mut action_buttons = [0; MAX_ACTION_BUTTONS];
    action_buttons[17] = sentinel;
    pkt.set_player_action_buttons_like_cpp(action_buttons);

    let bytes = pkt.to_bytes();
    let sentinel_bytes = sentinel.to_le_bytes();
    assert!(
        bytes.windows(4).any(|window| window == sentinel_bytes),
        "C++ ActivePlayer movement block writes Player::m_actionButtons into self CREATE"
    );
}
