//! Spell packet regressions.
//!
//! Moved out of spell.rs under #683; every test is unchanged.

use super::*;

#[test]
fn cancel_cast_reads_cpp_cast_id_then_spell_id() {
    let cast_id = ObjectGuid::create_player(1, 77);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&cast_id);
    pkt.write_uint32(12_345);
    pkt.reset_read();

    let parsed = CancelCast::read(&mut pkt).unwrap();
    assert_eq!(parsed.cast_id, cast_id);
    assert_eq!(parsed.spell_id, 12_345);
    assert!(pkt.is_empty());
}

#[test]
fn cancel_aura_reads_cpp_spell_id_then_caster_guid() {
    let caster_guid = ObjectGuid::create_player(1, 77);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(12_345);
    pkt.write_packed_guid(&caster_guid);
    pkt.reset_read();

    let parsed = CancelAura::read(&mut pkt).unwrap();
    assert_eq!(parsed.spell_id, 12_345);
    assert_eq!(parsed.caster_guid, caster_guid);
    assert!(pkt.is_empty());
}

#[test]
fn cancel_channelling_reads_cpp_channel_spell_then_reason() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(12_345);
    pkt.write_int32(40);
    pkt.reset_read();

    let parsed = CancelChannelling::read(&mut pkt).unwrap();
    assert_eq!(parsed.channel_spell, 12_345);
    assert_eq!(parsed.reason, 40);
    assert!(pkt.is_empty());
}

#[test]
fn cancel_mod_speed_no_control_reads_cpp_target_guid() {
    let target_guid = ObjectGuid::create_player(1, 77);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&target_guid);
    pkt.reset_read();

    let parsed = CancelModSpeedNoControlAuras::read(&mut pkt).unwrap();
    assert_eq!(parsed.target_guid, target_guid);
    assert!(pkt.is_empty());
}

#[test]
fn cancel_empty_spell_packets_match_cpp_empty_reads() {
    assert_eq!(
        CancelAutoRepeatSpell::read(&mut WorldPacket::new_empty()).unwrap(),
        CancelAutoRepeatSpell
    );
    assert_eq!(
        CancelGrowthAura::read(&mut WorldPacket::new_empty()).unwrap(),
        CancelGrowthAura
    );
    assert_eq!(
        CancelMountAura::read(&mut WorldPacket::new_empty()).unwrap(),
        CancelMountAura
    );
    assert_eq!(
        CancelQueuedSpell::read(&mut WorldPacket::new_empty()).unwrap(),
        CancelQueuedSpell
    );
}

#[test]
fn set_action_button_reads_cpp_action_then_index() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(12_345 | (0x80 << 24));
    pkt.write_uint8(7);
    pkt.reset_read();

    let parsed = SetActionButton::read(&mut pkt).unwrap();
    assert_eq!(parsed.action, 12_345 | (0x80 << 24));
    assert_eq!(parsed.index, 7);
    assert!(pkt.is_empty());
}

#[test]
fn self_res_reads_cpp_spell_id() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(20_000);
    pkt.reset_read();

    let parsed = SelfRes::read(&mut pkt).unwrap();
    assert_eq!(parsed.spell_id, 20_000);
    assert!(pkt.is_empty());
}

#[test]
fn open_item_reads_cpp_slot_then_pack_slot() {
    let mut pkt = WorldPacket::from_bytes(&[0xC6, 0x32, 0xFF, 0x24]);
    pkt.skip_opcode();

    let open = OpenItem::read(&mut pkt).unwrap();
    assert_eq!(open.slot, 0xFF);
    assert_eq!(open.pack_slot, 0x24);
}

#[test]
fn spell_click_reads_cpp_guid_then_try_auto_dismount_bit() {
    let guid = ObjectGuid::new(0x0102_0304_0506_0708, 0x1112_1314_1516_1718);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::SpellClick as u16);
    pkt.write_packed_guid(&guid);
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();
    pkt.skip_opcode();

    let spell_click = SpellClick::read(&mut pkt).unwrap();
    assert_eq!(spell_click.unit_guid, guid);
    assert!(spell_click.try_auto_dismount);
}

#[test]
fn spell_target_data_roundtrips_optional_locations_orientation_map_name() {
    let unit = ObjectGuid::new(0x0102_0304_0506_0708, 0x1112_1314_1516_1718);
    let item = ObjectGuid::new(0x2122_2324_2526_2728, 0x3132_3334_3536_3738);
    let src_transport = ObjectGuid::new(0x4142_4344_4546_4748, 0x5152_5354_5556_5758);
    let dst_transport = ObjectGuid::new(0x6162_6364_6566_6768, 0x7172_7374_7576_7778);
    let target = SpellTargetData {
        flags: 0x0A_BC_DE_F0,
        unit,
        item,
        src_location: Some(TargetLocation {
            transport: src_transport,
            position: Position::xyz(1.25, -2.5, 3.75),
        }),
        dst_location: Some(TargetLocation {
            transport: dst_transport,
            position: Position::xyz(100.0, 200.5, -300.25),
        }),
        orientation: Some(4.125),
        map_id: Some(571),
        name: "FarsightTarget".to_string(),
    };

    let mut pkt = WorldPacket::new_empty();
    target.write(&mut pkt);
    pkt.reset_read();

    let parsed = SpellTargetData::read(&mut pkt).expect("target data must parse");
    assert_eq!(parsed.flags, target.flags);
    assert_eq!(parsed.unit, unit);
    assert_eq!(parsed.item, item);
    assert_eq!(parsed.src_location, target.src_location);
    assert_eq!(parsed.dst_location, target.dst_location);
    assert_eq!(parsed.orientation, target.orientation);
    assert_eq!(parsed.map_id, target.map_id);
    assert_eq!(parsed.name, target.name);
    assert!(pkt.is_empty());
}

#[test]
fn spell_target_data_default_minimal_has_no_optional_payload() {
    let target = SpellTargetData::default();
    let mut pkt = WorldPacket::new_empty();
    target.write(&mut pkt);

    // 28-bit flags + 4 presence bits + 7-bit name length flushed to 5 bytes,
    // then two empty packed GUIDs (2 bytes each), matching the previous minimal shape.
    assert_eq!(pkt.data().len(), 9);

    pkt.reset_read();
    let parsed = SpellTargetData::read(&mut pkt).expect("minimal target data must parse");
    assert_eq!(parsed.flags, 0);
    assert_eq!(parsed.unit, ObjectGuid::EMPTY);
    assert_eq!(parsed.item, ObjectGuid::EMPTY);
    assert_eq!(parsed.src_location, None);
    assert_eq!(parsed.dst_location, None);
    assert_eq!(parsed.orientation, None);
    assert_eq!(parsed.map_id, None);
    assert!(parsed.name.is_empty());
    assert!(pkt.is_empty());
}

#[test]
fn spell_cast_visual_serializes_one_int32_like_cpp() {
    let visual = SpellCastVisual {
        spell_visual_id: 0x1122_3344,
        script_visual_id: 0x5566_7788,
    };
    let mut pkt = WorldPacket::new_empty();

    visual.write(&mut pkt);

    assert_eq!(pkt.data(), &0x1122_3344u32.to_le_bytes());
    pkt.reset_read();
    let parsed = SpellCastVisual::read(&mut pkt).unwrap();
    assert_eq!(parsed.spell_visual_id, 0x1122_3344);
    assert_eq!(parsed.script_visual_id, 0);
    assert!(pkt.is_empty());
}

#[test]
fn cast_failed_writes_visual_between_spell_and_reason_like_cpp() {
    let cast_id = ObjectGuid::create_player(1, 99);
    let bytes = CastFailed {
        cast_id,
        spell_id: 12_345,
        visual: SpellCastVisual {
            spell_visual_id: 678,
            script_visual_id: 999,
        },
        reason: 2,
        fail_arg1: -1,
        fail_arg2: 7,
    }
    .to_bytes();

    assert_eq!(
        &bytes[0..2],
        &(ServerOpcodes::CastFailed as u16).to_le_bytes()
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_packed_guid().unwrap(), cast_id);
    assert_eq!(pkt.read_int32().unwrap(), 12_345);
    assert_eq!(
        SpellCastVisual::read(&mut pkt).unwrap().spell_visual_id,
        678
    );
    assert_eq!(pkt.read_int32().unwrap(), 2);
    assert_eq!(pkt.read_int32().unwrap(), -1);
    assert_eq!(pkt.read_int32().unwrap(), 7);
    assert!(pkt.is_empty());
}

#[test]
fn cast_spell_request_preserves_misc_like_cpp() {
    let cast_id = ObjectGuid::create_player(1, 77);
    let mut pkt = WorldPacket::new_empty();
    write_minimal_spell_cast_request(&mut pkt, cast_id, [30_000, 9], 12_345);

    let parsed = CastSpellRequest::read(&mut pkt).unwrap();
    assert_eq!(parsed.cast_id, cast_id);
    assert_eq!(parsed.misc, [30_000, 9]);
    assert_eq!(parsed.spell_id, 12_345);
    assert!(parsed.move_update.is_none());
}

#[test]
fn cast_spell_request_reads_optional_currency_cost_like_cpp() {
    let cast_id = ObjectGuid::create_player(1, 88);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&cast_id);
    pkt.write_int32(0);
    pkt.write_int32(0);
    pkt.write_int32(17_229);
    SpellCastVisual::default().write(&mut pkt);
    pkt.write_float(0.0);
    pkt.write_float(0.0);
    pkt.write_packed_guid(&ObjectGuid::EMPTY);
    pkt.write_uint32(1);
    pkt.write_uint32(0);
    pkt.write_uint32(0);
    pkt.write_int32(3_777);
    pkt.write_int32(25);
    pkt.write_bits(0, 5);
    pkt.write_bit(false);
    pkt.write_bits(0, 2);
    pkt.write_bit(false);
    pkt.flush_bits();
    SpellTargetData::default().write(&mut pkt);
    pkt.reset_read();

    let parsed = CastSpellRequest::read(&mut pkt).unwrap();
    assert_eq!(parsed.cast_id, cast_id);
    assert_eq!(parsed.spell_id, 17_229);
    assert_eq!(parsed.target, SpellTargetData::default());
}

#[test]
fn cast_spell_request_reads_move_update_before_weights_like_cpp() {
    let cast_id = ObjectGuid::create_player(1, 89);
    let mover_guid = ObjectGuid::create_player(1, 90);
    let movement = MovementInfo {
        guid: mover_guid,
        time: 123_456,
        position: Position::new(11.0, 22.0, 33.0, 1.25),
        ..MovementInfo::default()
    };

    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&cast_id);
    pkt.write_int32(0);
    pkt.write_int32(0);
    pkt.write_int32(17_229);
    SpellCastVisual::default().write(&mut pkt);
    pkt.write_float(0.0);
    pkt.write_float(0.0);
    pkt.write_packed_guid(&ObjectGuid::EMPTY);
    pkt.write_uint32(0);
    pkt.write_uint32(0);
    pkt.write_uint32(0);
    pkt.write_bits(0, 5);
    pkt.write_bit(true);
    pkt.write_bits(1, 2);
    pkt.write_bit(false);
    pkt.flush_bits();
    SpellTargetData::default().write(&mut pkt);
    movement.write(&mut pkt);
    pkt.write_bits(2, 2);
    pkt.flush_bits();
    pkt.write_int32(377);
    pkt.write_uint32(4);
    pkt.reset_read();

    let parsed = CastSpellRequest::read(&mut pkt).unwrap();
    let parsed_movement = parsed
        .move_update
        .expect("move update must be read before spell weights");
    assert_eq!(parsed.cast_id, cast_id);
    assert_eq!(parsed.spell_id, 17_229);
    assert_eq!(parsed_movement.guid, mover_guid);
    assert_eq!(parsed_movement.time, movement.time);
    assert_eq!(parsed_movement.position, movement.position);
    assert!(pkt.is_empty());
}

#[test]
fn spell_prepare_writes_client_and_server_cast_ids_like_cpp() {
    let client_cast_id = ObjectGuid::create_player(1, 77);
    let server_cast_id =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Cast, 0, 1, 571, 0, 12_345, 9);

    let bytes = SpellPreparePkt {
        client_cast_id,
        server_cast_id,
    }
    .to_bytes();

    assert_eq!(
        &bytes[0..2],
        &(ServerOpcodes::SpellPrepare as u16).to_le_bytes()
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_packed_guid().unwrap(), client_cast_id);
    assert_eq!(pkt.read_packed_guid().unwrap(), server_cast_id);
    assert!(pkt.is_empty());
}

#[test]
fn play_spell_visual_writes_cpp_field_order() {
    let guid = ObjectGuid::create_player(1, 77);
    let target_position = Position::new(1.25, -2.5, 3.75, 0.5);
    let bytes = PlaySpellVisual::self_target(guid, target_position, 222).to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes);

    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::PlaySpellVisual as u16
    );
    let mut source = [0u8; 16];
    for byte in &mut source {
        *byte = pkt.read_uint8().expect("source byte");
    }
    assert_eq!(ObjectGuid::from_raw_bytes(&source), guid);
    let mut target = [0u8; 16];
    for byte in &mut target {
        *byte = pkt.read_uint8().expect("target byte");
    }
    assert_eq!(ObjectGuid::from_raw_bytes(&target), guid);
    let mut transport = [0u8; 16];
    for byte in &mut transport {
        *byte = pkt.read_uint8().expect("transport byte");
    }
    assert_eq!(ObjectGuid::from_raw_bytes(&transport), ObjectGuid::EMPTY);
    assert_eq!(pkt.read_float().expect("target x"), 1.25);
    assert_eq!(pkt.read_float().expect("target y"), -2.5);
    assert_eq!(pkt.read_float().expect("target z"), 3.75);
    assert_eq!(pkt.read_uint32().expect("visual"), 222);
    assert_eq!(pkt.read_float().expect("travel speed"), 0.0);
    assert_eq!(pkt.read_uint16().expect("hit reason"), 0);
    assert_eq!(pkt.read_uint16().expect("miss reason"), 0);
    assert_eq!(pkt.read_uint16().expect("reflect status"), 0);
    assert_eq!(pkt.read_float().expect("launch delay"), 0.0);
    assert_eq!(pkt.read_float().expect("min duration"), 0.0);
    assert!(!pkt.read_bit().expect("speed as time"));
    assert!(pkt.is_empty());
}

#[test]
fn player_trainer_visual_matches_fresh_cpp_capture() {
    // cpp-trainer-fixed.pkt: SpellPackets.cpp PlaySpellVisualKit::Write,
    // player counter14, realm1, kit362/type1. Includes the opcode prefix.
    let bytes = PlaySpellVisualKit {
        unit: ObjectGuid::create_player(1, 14),
        kit_record_id: 362,
        kit_type: 1,
        duration: 0,
        mounted_visual: false,
    }
    .to_bytes();
    assert_eq!(
        bytes,
        [
            0x46, 0x2c, 0x01, 0xa0, 0x0e, 0x04, 0x08, 0x6a, 0x01, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0
        ]
    );
}

#[test]
fn clear_target_writes_guid_like_cpp() {
    let guid = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
    let bytes = ClearTarget { guid }.to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes);

    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::ClearTarget as u16
    );
    assert_eq!(pkt.read_packed_guid().expect("Guid"), guid);
    assert!(pkt.is_empty());
}

#[test]
fn spell_go_without_misses_keeps_zero_miss_vectors_on_wire() {
    let bytes = spell_go_bytes(Vec::new(), Vec::new());
    let (mut pkt, hit_count, miss_count, miss_status_count) = read_spell_go_through_target(&bytes);

    assert_eq!(hit_count, 0);
    assert_eq!(miss_count, 0);
    assert_eq!(miss_status_count, 0);
    assert!(!pkt.read_bit().expect("combat log data presence"));
    assert!(pkt.is_empty());
}

#[test]
fn spell_go_writes_miss_target_then_miss_reason_like_cpp() {
    let hit_target = ObjectGuid::new(0x0100, 0x0200);
    let miss_target = ObjectGuid::new(0x0300, 0x0400);
    let bytes = spell_go_bytes(
        vec![hit_target],
        vec![SpellMissTarget::new(miss_target, SpellMissReason::Miss)],
    );
    let (mut pkt, hit_count, miss_count, miss_status_count) = read_spell_go_through_target(&bytes);

    assert_eq!(hit_count, 1);
    assert_eq!(miss_count, 1);
    assert_eq!(miss_status_count, 1);
    assert_eq!(pkt.read_packed_guid().expect("hit target"), hit_target);
    assert_eq!(pkt.read_packed_guid().expect("miss target"), miss_target);
    assert_eq!(
        pkt.read_uint8().expect("miss reason"),
        SpellMissReason::Miss as u8
    );
    assert!(!pkt.read_bit().expect("combat log data presence"));
    assert!(pkt.is_empty());
}

#[test]
fn spell_go_writes_reflect_status_byte_after_reflect_reason() {
    let miss_target = ObjectGuid::new(0x0500, 0x0600);
    let bytes = spell_go_bytes(
        Vec::new(),
        vec![SpellMissTarget::reflected(
            miss_target,
            SpellMissReason::Resist,
        )],
    );
    let (mut pkt, hit_count, miss_count, miss_status_count) = read_spell_go_through_target(&bytes);

    assert_eq!(hit_count, 0);
    assert_eq!(miss_count, 1);
    assert_eq!(miss_status_count, 1);
    assert_eq!(pkt.read_packed_guid().expect("miss target"), miss_target);
    assert_eq!(
        pkt.read_uint8().expect("reflect reason"),
        SpellMissReason::Reflect as u8
    );
    assert_eq!(
        pkt.read_uint8().expect("reflected hit result"),
        SpellMissReason::Resist as u8
    );
    assert!(!pkt.read_bit().expect("combat log data presence"));
    assert!(pkt.is_empty());
}

#[test]
fn spell_go_preserves_original_cast_id_and_cast_flags_ex_like_cpp() {
    let caster = ObjectGuid::create_player(1, 77);
    let client_cast_id = ObjectGuid::create_player(1, 99);
    let server_cast_id =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Cast, 0, 1, 571, 0, 12_345, 9);

    let bytes = SpellGoPkt {
        caster,
        cast_id: server_cast_id,
        original_cast_id: client_cast_id,
        spell_id: 12_345,
        visual: SpellCastVisual::default(),
        cast_flags: 0x0004_0101,
        cast_flags_ex: 0x08000,
        cast_time_ms: 0x1234_5678,
        target: SpellTargetData::default(),
        cast_data: SpellCastData::default(),
        hit_targets: Vec::new(),
        miss_targets: Vec::new(),
    }
    .to_bytes();

    assert_eq!(&bytes[0..2], &(ServerOpcodes::SpellGo as u16).to_le_bytes());
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_packed_guid().unwrap(), caster);
    assert_eq!(pkt.read_packed_guid().unwrap(), caster);
    assert_eq!(pkt.read_packed_guid().unwrap(), server_cast_id);
    assert_eq!(pkt.read_packed_guid().unwrap(), client_cast_id);
    assert_eq!(pkt.read_int32().unwrap(), 12_345);
    let visual = SpellCastVisual::read(&mut pkt).unwrap();
    assert_eq!(visual.spell_visual_id, 0);
    assert_eq!(visual.script_visual_id, 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0x0004_0101);
    assert_eq!(pkt.read_uint32().unwrap(), 0x08000);
    assert_eq!(pkt.read_uint32().unwrap(), 0x1234_5678);
}

#[test]
fn spell_go_full_log_writes_cpp_stats_and_power_rows() {
    let go = SpellGoPkt {
        caster: ObjectGuid::EMPTY,
        cast_id: ObjectGuid::EMPTY,
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: 12_345,
        visual: SpellCastVisual::default(),
        cast_flags: 0,
        cast_flags_ex: 0,
        cast_time_ms: 0,
        target: SpellTargetData::default(),
        cast_data: SpellCastData::default(),
        hit_targets: Vec::new(),
        miss_targets: Vec::new(),
    };
    let bytes = go.to_full_log_bytes_like_cpp(&SpellCastLogData {
        health: 7_654,
        attack_power: 321,
        spell_power: -12,
        armor: 987,
        power_data: vec![
            SpellLogPowerData {
                power_type: 0,
                amount: 456,
                cost: 0,
            },
            SpellLogPowerData {
                power_type: 3,
                amount: 78,
                cost: 9,
            },
        ],
    });

    let (mut pkt, hit_count, miss_count, miss_status_count) = read_spell_go_through_target(&bytes);
    assert_eq!((hit_count, miss_count, miss_status_count), (0, 0, 0));
    assert!(pkt.read_bit().expect("combat log data presence"));
    assert_eq!(pkt.read_int64().expect("health"), 7_654);
    assert_eq!(pkt.read_int32().expect("attack power"), 321);
    assert_eq!(pkt.read_int32().expect("spell power"), -12);
    assert_eq!(pkt.read_int32().expect("armor"), 987);
    assert_eq!(pkt.read_bits(9).expect("power data count"), 2);
    for expected in [(0, 456, 0), (3, 78, 9)] {
        assert_eq!(pkt.read_int32().expect("power type"), expected.0);
        assert_eq!(pkt.read_int32().expect("power amount"), expected.1);
        assert_eq!(pkt.read_int32().expect("power cost"), expected.2);
    }
    assert!(pkt.is_empty());
}
