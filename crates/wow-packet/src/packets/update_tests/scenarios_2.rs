//! Object update-block regressions, part 2 of 5.
//!
//! Moved out of the update_tests.rs root under #640; every test is unchanged.

use super::*;

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
