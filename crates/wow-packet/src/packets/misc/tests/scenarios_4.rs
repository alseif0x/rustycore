//! Miscellaneous packet regressions, part 4 of 6.
//!
//! Moved out of the tests.rs root under #640; every test is unchanged.

use super::*;

#[test]
fn bind_point_update_preserves_full_uint32_area_like_cpp() {
    let bytes = BindPointUpdate {
        x: 1.0,
        y: 2.0,
        z: 3.0,
        map_id: 571,
        area_id: u32::MAX,
    }
    .to_bytes();
    assert_eq!(
        u32::from_le_bytes(bytes[18..22].try_into().unwrap()),
        u32::MAX
    );
}

#[test]
fn player_bound_serializes_packed_guid_and_area_like_cpp() {
    let binder_id = wow_core::ObjectGuid::new(0x0102_0304_0506_0708, 0x1112_1314_1516_1718);
    let pkt = PlayerBound {
        binder_id,
        area_id: 42,
    };

    let bytes = pkt.to_bytes();
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2ff8);

    let mut payload = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(payload.read_packed_guid().unwrap(), binder_id);
    assert_eq!(payload.read_uint32().unwrap(), 42);
    assert_eq!(payload.remaining(), 0);
}

#[test]
fn world_server_info_serializes() {
    let pkt = WorldServerInfo::default_open_world();
    let bytes = pkt.to_bytes();
    // opcode(2) + int32(4) + 5 bits flushed to 1 byte = 7
    assert_eq!(bytes.len(), 7);
}

#[test]
fn initial_setup_wotlk() {
    let pkt = InitialSetup::wotlk();
    let bytes = pkt.to_bytes();
    // opcode(2) + uint8(1) + uint8(1) = 4
    assert_eq!(bytes.len(), 4);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2580);
}

#[test]
fn time_sync_request_serializes() {
    let pkt = TimeSyncRequest { sequence_index: 0 };
    let bytes = pkt.to_bytes();
    // opcode(2) + u32(4) = 6
    assert_eq!(bytes.len(), 6);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2dd2);
}

#[test]
fn time_sync_response_reads_cpp_wire_order() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&7u32.to_le_bytes());
    bytes.extend_from_slice(&123_456u32.to_le_bytes());

    let mut pkt = WorldPacket::from_bytes(&bytes);
    let response = TimeSyncResponse::read(&mut pkt).expect("valid TimeSyncResponse");

    assert_eq!(response.sequence_index, 7);
    assert_eq!(response.client_time, 123_456);
}

#[test]
fn contact_list_empty() {
    let pkt = ContactList::all();
    let bytes = pkt.to_bytes();
    // opcode(2) + u32(4) + bits(8→1 byte) = 7
    assert_eq!(bytes.len(), 7);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x278c);
    // Flags = 7 (All)
    let flags = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
    assert_eq!(flags, 7);
}

#[test]
fn active_glyphs_empty() {
    let pkt = ActiveGlyphs {
        glyphs: Vec::new(),
        is_full_update: true,
    };
    let bytes = pkt.to_bytes();
    // opcode(2) + i32(4) + 1 bit flushed to 1 byte = 7
    assert_eq!(bytes.len(), 7);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2c51);
}

#[test]
fn active_glyphs_writes_bindings_like_cpp() {
    let pkt = ActiveGlyphs {
        glyphs: vec![GlyphBindingLikeCpp {
            spell_id: 12345,
            glyph_id: 678,
        }],
        is_full_update: false,
    };
    let bytes = pkt.to_bytes();

    assert_eq!(bytes.len(), 13);
    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2c51);
    assert_eq!(u32::from_le_bytes(bytes[2..6].try_into().unwrap()), 1);
    assert_eq!(u32::from_le_bytes(bytes[6..10].try_into().unwrap()), 12345);
    assert_eq!(u16::from_le_bytes(bytes[10..12].try_into().unwrap()), 678);
}

#[test]
fn load_equipment_set_empty() {
    let pkt = LoadEquipmentSet::default();
    let bytes = pkt.to_bytes();
    // opcode(2) + i32(4) = 6
    assert_eq!(bytes.len(), 6);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x270e);
}

#[test]
fn all_account_criteria_empty() {
    let pkt = AllAccountCriteria;
    let bytes = pkt.to_bytes();
    // opcode(2) + i32(4) = 6
    assert_eq!(bytes.len(), 6);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2571);
}

#[test]
fn all_achievement_data_empty() {
    let pkt = AllAchievementData;
    let bytes = pkt.to_bytes();
    // opcode(2) + i32(4) + i32(4) = 10
    assert_eq!(bytes.len(), 10);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2570);
}

#[test]
fn account_mount_update_empty() {
    let pkt = AccountMountUpdate::empty_full();
    let bytes = pkt.to_bytes();
    // opcode(2) + 1 bit(padded to 1 byte) + i32(4) = 7
    // wait: write_bit(true) → 1 bit buffered, then write_int32(0)
    // auto-flushes → 1 byte (bit), then 4 bytes (i32), then flush_bits (no-op) = 7
    assert_eq!(bytes.len(), 7);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x25ae);
}

#[test]
fn account_mount_update_writes_mount_entries_like_cpp() {
    let pkt = AccountMountUpdate::full(vec![
        AccountMount {
            spell_id: 100,
            flags: 0x01,
        },
        AccountMount {
            spell_id: 200,
            flags: 0x12,
        },
    ]);
    let bytes = pkt.to_bytes();

    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x25ae);
    assert_eq!(bytes[2], 0x80);
    assert_eq!(
        i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
        2
    );
    assert_eq!(
        i32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
        100
    );
    assert_eq!(bytes[11], 0x10);
    assert_eq!(
        i32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
        200
    );
    assert_eq!(bytes[16], 0x20);
}

#[test]
fn account_heirloom_update_writes_items_then_flags_like_cpp() {
    let pkt = AccountHeirloomUpdate::full(vec![
        AccountHeirloom {
            item_id: 44_000,
            flags: 0x01,
        },
        AccountHeirloom {
            item_id: 44_001,
            flags: 0x04,
        },
    ]);
    let bytes = pkt.to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::UpdateCapturePoint as u16
    );
    assert_eq!(bytes[2], 0x80);
    assert_eq!(
        i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
        0
    );
    assert_eq!(
        u32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
        2
    );
    assert_eq!(
        u32::from_le_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]),
        2
    );
    assert_eq!(
        i32::from_le_bytes([bytes[15], bytes[16], bytes[17], bytes[18]]),
        44_000
    );
    assert_eq!(
        i32::from_le_bytes([bytes[19], bytes[20], bytes[21], bytes[22]]),
        44_001
    );
    assert_eq!(
        u32::from_le_bytes([bytes[23], bytes[24], bytes[25], bytes[26]]),
        0x01
    );
    assert_eq!(
        u32::from_le_bytes([bytes[27], bytes[28], bytes[29], bytes[30]]),
        0x04
    );
}

#[test]
fn account_mount_update_partial_clears_full_update_bit_like_cpp() {
    let pkt = AccountMountUpdate::partial(vec![AccountMount {
        spell_id: 100,
        flags: 0x01,
    }]);
    let bytes = pkt.to_bytes();

    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x25ae);
    assert_eq!(bytes[2], 0x00);
    assert_eq!(
        i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
        1
    );
}

#[test]
fn mount_result_writes_result_int32_like_cpp() {
    let bytes = MountResult {
        result: MOUNT_RESULT_SHAPESHIFTED_LIKE_CPP,
    }
    .to_bytes();

    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::MountResult as u16).to_le_bytes()
    );
    assert_eq!(
        i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
        MOUNT_RESULT_SHAPESHIFTED_LIKE_CPP
    );
    assert_eq!(bytes.len(), 6);
}

#[test]
fn mount_set_favorite_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::MountSetFavorite as u16);
    pkt.write_uint32(1234);
    pkt.write_bit(true);
    pkt.flush_bits();

    let decoded = MountSetFavorite::read(&mut pkt).unwrap();
    assert_eq!(
        decoded,
        MountSetFavorite {
            mount_spell_id: 1234,
            is_favorite: true,
        }
    );
}

#[test]
fn mount_special_reads_count_sequence_and_visual_kits_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::MountSpecialAnim as u16);
    pkt.write_uint32(2);
    pkt.write_int32(-7);
    pkt.write_int32(111);
    pkt.write_int32(222);

    let decoded = MountSpecial::read(&mut pkt).unwrap();
    assert_eq!(
        decoded,
        MountSpecial {
            spell_visual_kit_ids: vec![111, 222],
            sequence_variation: -7,
        }
    );
}

#[test]
fn special_mount_anim_writes_guid_count_sequence_and_visual_kits_like_cpp() {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Player, 0, 1, 571, 0, 0, 42);
    let bytes = SpecialMountAnim {
        unit_guid: guid,
        spell_visual_kit_ids: vec![111, -222],
        sequence_variation: 3,
    }
    .to_bytes();

    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::SpecialMountAnim as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
    assert_eq!(&bytes[18..22], &2_u32.to_le_bytes());
    assert_eq!(&bytes[22..26], &3_i32.to_le_bytes());
    assert_eq!(&bytes[26..30], &111_i32.to_le_bytes());
    assert_eq!(&bytes[30..34], &(-222_i32).to_le_bytes());
    assert_eq!(bytes.len(), 34);
}

#[test]
fn account_toy_update_empty() {
    let pkt = AccountToyUpdate::full(Vec::new());
    let bytes = pkt.to_bytes();
    // opcode(2) + 1 bit(padded to 1 byte) + 3*i32(12) = 15
    assert_eq!(bytes.len(), 15);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x25b0);
}

#[test]
fn save_cuf_profiles_reads_cpp_shape() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::SaveCufProfiles as u16);
    pkt.write_uint32(1);
    pkt.write_bits(4, 7);
    for option in 0..CUF_BOOL_OPTIONS_COUNT_LIKE_CPP {
        pkt.write_bit(matches!(option, 0 | 5 | 26));
    }
    pkt.write_uint16(72);
    pkt.write_uint16(128);
    pkt.write_uint8(2);
    pkt.write_uint8(3);
    pkt.write_uint8(4);
    pkt.write_uint8(5);
    pkt.write_uint8(6);
    pkt.write_uint16(7);
    pkt.write_uint16(8);
    pkt.write_uint16(9);
    pkt.write_string("Raid");

    let mut packet = WorldPacket::from_bytes(pkt.data());
    let parsed = SaveCufProfiles::read(&mut packet).expect("valid SaveCUFProfiles");

    assert_eq!(parsed.profiles.len(), 1);
    let profile = &parsed.profiles[0];
    assert_eq!(profile.profile_name, "Raid");
    assert_eq!(profile.frame_height, 72);
    assert_eq!(profile.frame_width, 128);
    assert_eq!(profile.sort_by, 2);
    assert_eq!(profile.health_text, 3);
    assert_eq!(profile.top_point, 4);
    assert_eq!(profile.bottom_point, 5);
    assert_eq!(profile.left_point, 6);
    assert_eq!(profile.top_offset, 7);
    assert_eq!(profile.bottom_offset, 8);
    assert_eq!(profile.left_offset, 9);
    assert_eq!(profile.bool_options, (1 << 0) | (1 << 5) | (1 << 26));
}

#[test]
fn tutorial_set_flag_reads_update_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::Tutorial as u16);
    pkt.write_bits(TUTORIAL_ACTION_UPDATE_LIKE_CPP as u32, 2);
    pkt.write_uint32(37);

    let mut packet = WorldPacket::from_bytes(pkt.data());
    let parsed = TutorialSetFlag::read(&mut packet).expect("valid CMSG_TUTORIAL update");

    assert_eq!(parsed.action, TUTORIAL_ACTION_UPDATE_LIKE_CPP);
    assert_eq!(parsed.tutorial_bit, Some(37));
}

#[test]
fn tutorial_set_flag_reads_clear_without_bit_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::Tutorial as u16);
    pkt.write_bits(TUTORIAL_ACTION_CLEAR_LIKE_CPP as u32, 2);
    pkt.flush_bits();

    let mut packet = WorldPacket::from_bytes(pkt.data());
    let parsed = TutorialSetFlag::read(&mut packet).expect("valid CMSG_TUTORIAL clear");

    assert_eq!(parsed.action, TUTORIAL_ACTION_CLEAR_LIKE_CPP);
    assert_eq!(parsed.tutorial_bit, None);
}

#[test]
fn load_cuf_profiles_writes_count_and_fields_like_cpp() {
    let bytes = LoadCufProfiles {
        profiles: vec![CufProfile {
            profile_name: "Raid".to_string(),
            frame_height: 72,
            frame_width: 128,
            sort_by: 2,
            health_text: 3,
            top_point: 4,
            bottom_point: 5,
            left_point: 6,
            top_offset: 7,
            bottom_offset: 8,
            left_offset: 9,
            bool_options: (1 << 0) | (1 << 5) | (1 << 26),
        }],
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::LoadCufProfiles as u16
    );
    assert_eq!(
        u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
        1
    );
}

#[test]
fn account_toy_update_writes_ids_then_flag_bits_like_cpp() {
    let pkt = AccountToyUpdate::full(vec![
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
    ]);
    let bytes = pkt.to_bytes();

    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x25b0);
    assert_eq!(bytes[2], 0x80);
    assert_eq!(
        i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
        2
    );
    assert_eq!(
        i32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
        2
    );
    assert_eq!(
        i32::from_le_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]),
        2
    );
    assert_eq!(
        u32::from_le_bytes([bytes[15], bytes[16], bytes[17], bytes[18]]),
        30_000
    );
    assert_eq!(
        u32::from_le_bytes([bytes[19], bytes[20], bytes[21], bytes[22]]),
        30_001
    );
    assert_eq!(bytes[23], 0b1001_0000);
}

#[test]
fn add_toy_reads_cpp_guid_payload() {
    let guid = ObjectGuid::create_item(1, 99);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::AddToy as u16);
    pkt.write_packed_guid(&guid);

    let decoded = AddToy::read(&mut pkt).unwrap();
    assert_eq!(decoded.item_guid, guid);
}

#[test]
fn toy_clear_fanfare_reads_cpp_item_id_payload() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::ToyClearFanfare as u16);
    pkt.write_uint32(30_000);

    let decoded = ToyClearFanfare::read(&mut pkt).unwrap();
    assert_eq!(decoded.item_id, 30_000);
}

#[test]
fn use_toy_reads_spell_cast_request_like_cpp() {
    let cast_id = ObjectGuid::create_player(1, 123);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::UseToy as u16);
    write_minimal_toy_spell_cast(&mut pkt, cast_id, 30_000, 12_345);

    let decoded = UseToy::read(&mut pkt).unwrap();
    assert_eq!(decoded.cast.cast_id, cast_id);
    assert_eq!(decoded.cast.misc[0], 30_000);
    assert_eq!(decoded.cast.spell_id, 12_345);
}

#[test]
fn load_cuf_profiles_empty() {
    let pkt = LoadCufProfiles::empty();
    let bytes = pkt.to_bytes();
    // opcode(2) + i32(4) = 6
    assert_eq!(bytes.len(), 6);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x25bc);
}

#[test]
fn aura_update_empty() {
    let guid = ObjectGuid::create_player(1, 42);
    let pkt = AuraUpdate::empty_for(guid);
    let bytes = pkt.to_bytes();
    // opcode(2) + 10 bits(padded to 2 bytes) + packed_guid(variable)
    assert!(bytes.len() > 4);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2c1f);
    // Byte 2: UpdateAll=1(MSB) + first 7 bits of count(0) = 0x80
    assert_eq!(bytes[2], 0x80);
}

#[test]
fn aura_update_single_passive_matches_cpp_shape() {
    let unit_guid = ObjectGuid::create_player(1, 2);
    let cast_id =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Cast, 3, 1, 571, 0, 822, 1);
    let pkt = AuraUpdate {
        unit_guid,
        update_all: false,
        auras: vec![AuraInfoLikeCpp {
            slot: 0,
            aura_data: Some(AuraDataInfoLikeCpp {
                cast_id,
                spell_id: 822,
                flags: 0x0301,
                active_flags: 0x1,
                caster_guid: unit_guid,
                cast_level: 80,
                applications: 0,
                duration_ms: None,
                remaining_ms: None,
                points: Vec::new(),
            }),
        }],
    };
    let bytes = pkt.to_bytes();
    let expected = [
        0x1f, 0x2c, 0x00, 0x40, 0x00, 0x80, 0x01, 0xbb, 0x01, 0x83, 0xcd, 0x60, 0x47, 0x04, 0xbc,
        0x36, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x03, 0x01, 0x00, 0x00, 0x00, 0x50,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xa0, 0x02, 0x04, 0x08,
    ];
    assert_eq!(bytes, expected);
}

#[test]
fn battle_pet_journal_lock_acquired_empty() {
    let pkt = BattlePetJournalLockAcquired;
    let bytes = pkt.to_bytes();
    // opcode(2) + no payload = 2
    assert_eq!(bytes.len(), 2);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x25ed);
}

#[test]
fn battle_pet_journal_lock_denied_empty() {
    let pkt = BattlePetJournalLockDenied;
    let bytes = pkt.to_bytes();
    assert_eq!(bytes.len(), 2);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x25ee);
}

#[test]
fn battle_pet_deleted_writes_packed_guid_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4330);
    let bytes = BattlePetDeleted { pet_guid }.to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::BattlePetDeleted as u16
    );

    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_packed_guid().unwrap(), pet_guid);
    assert_eq!(body.remaining(), 0);
}

#[test]
fn battle_pet_error_writes_result_bits_then_creature_id_like_cpp() {
    let bytes =
        BattlePetError::new(BattlePetErrorCodeLikeCpp::TooHighLevelToUncage, 12_345).to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::BattlePetError as u16
    );

    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(
        body.read_bits(4).unwrap(),
        BattlePetErrorCodeLikeCpp::TooHighLevelToUncage as u32
    );
    assert_eq!(body.read_int32().unwrap(), 12_345);
    assert_eq!(body.remaining(), 0);
}

#[test]
fn battle_pet_request_journal_reads_empty_payload_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetRequestJournal as u16);

    assert_eq!(
        BattlePetRequestJournal::read(&mut pkt).unwrap(),
        BattlePetRequestJournal
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn battle_pet_request_journal_lock_reads_empty_payload_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetRequestJournalLock as u16);

    assert_eq!(
        BattlePetRequestJournalLock::read(&mut pkt).unwrap(),
        BattlePetRequestJournalLock
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn battle_pet_journal_writes_empty_default_slots_like_cpp() {
    let bytes = BattlePetJournal::empty_with_default_slots(true).to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::BattlePetJournal as u16
    );

    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_uint16().unwrap(), 0);
    assert_eq!(body.read_uint32().unwrap(), 3);
    assert_eq!(body.read_uint32().unwrap(), 0);
    assert!(body.read_bit().unwrap());

    for index in 0..3 {
        let slot_guid = body.read_packed_guid().unwrap();
        assert_eq!(slot_guid, empty_battle_pet_guid_like_cpp());
        assert_eq!(slot_guid.high_type(), HighGuid::BattlePet);
        assert_eq!(body.read_uint32().unwrap(), 0);
        assert_eq!(body.read_uint8().unwrap(), index);
        assert!(body.read_bit().unwrap());
    }
    assert_eq!(body.remaining(), 0);
}

#[test]
fn battle_pet_journal_writes_pet_rows_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4335);
    let owner_guid = ObjectGuid::create_player(1, 77);
    let bytes = BattlePetJournal {
        trap: 9,
        has_journal_lock: true,
        slots: Vec::new(),
        pets: vec![sample_battle_pet_journal_pet_like_cpp(pet_guid, owner_guid)],
    }
    .to_bytes();

    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_uint16().unwrap(), 9);
    assert_eq!(body.read_uint32().unwrap(), 0);
    assert_eq!(body.read_uint32().unwrap(), 1);
    assert!(body.read_bit().unwrap());
    assert_sample_battle_pet_journal_pet_like_cpp(&mut body, pet_guid, owner_guid);
    assert_eq!(body.remaining(), 0);
}

#[test]
fn battle_pet_updates_writes_count_flag_then_pet_rows_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4336);
    let owner_guid = ObjectGuid::create_player(1, 78);
    let bytes = BattlePetUpdates {
        pets: vec![sample_battle_pet_journal_pet_like_cpp(pet_guid, owner_guid)],
        pet_added: true,
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::BattlePetUpdates as u16
    );
    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_uint32().unwrap(), 1);
    assert!(body.read_bit().unwrap());
    assert_sample_battle_pet_journal_pet_like_cpp(&mut body, pet_guid, owner_guid);
    assert_eq!(body.remaining(), 0);
}

#[test]
fn pet_battle_slot_updates_writes_flags_then_slots_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4337);
    let bytes = PetBattleSlotUpdates {
        slots: vec![BattlePetJournalSlot {
            pet_guid,
            collar_id: 10,
            index: 2,
            locked: false,
        }],
        auto_slotted: false,
        new_slot: true,
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::PetBattleSlotUpdates as u16
    );
    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_uint32().unwrap(), 1);
    assert!(body.read_bit().unwrap());
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.read_packed_guid().unwrap(), pet_guid);
    assert_eq!(body.read_uint32().unwrap(), 10);
    assert_eq!(body.read_uint8().unwrap(), 2);
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.remaining(), 0);
}

#[test]
fn battle_pet_set_battle_slot_reads_cpp_shape() {
    let pet_guid = ObjectGuid::new(0, 0x4323);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetSetBattleSlot as u16);
    pkt.write_packed_guid(&pet_guid);
    pkt.write_uint8(2);

    let decoded = BattlePetSetBattleSlot::read(&mut pkt).unwrap();
    assert_eq!(decoded, BattlePetSetBattleSlot { pet_guid, slot: 2 });
}

#[test]
fn battle_pet_summon_reads_packed_guid_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4324);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetSummon as u16);
    pkt.write_packed_guid(&pet_guid);

    let decoded = BattlePetSummon::read(&mut pkt).unwrap();
    assert_eq!(decoded, BattlePetSummon { pet_guid });
}

#[test]
fn battle_pet_update_notify_reads_packed_guid_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4325);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetUpdateNotify as u16);
    pkt.write_packed_guid(&pet_guid);

    let decoded = BattlePetUpdateNotify::read(&mut pkt).unwrap();
    assert_eq!(decoded, BattlePetUpdateNotify { pet_guid });
}

#[test]
fn battle_pet_delete_pet_reads_placeholder_cpp_shape() {
    let pet_guid = ObjectGuid::new(0, 0x4331);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(0xBADD);
    pkt.write_packed_guid(&pet_guid);

    let decoded = BattlePetDeletePet::read_like_cpp(&mut pkt).unwrap();
    assert_eq!(decoded, BattlePetDeletePet { pet_guid });
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn cage_battle_pet_reads_placeholder_cpp_shape() {
    let pet_guid = ObjectGuid::new(0, 0x4334);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(0xBADD);
    pkt.write_packed_guid(&pet_guid);

    let decoded = CageBattlePet::read_like_cpp(&mut pkt).unwrap();
    assert_eq!(decoded, CageBattlePet { pet_guid });
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn battle_pet_modify_name_reads_without_declined_names_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4332);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(0xBADD);
    pkt.write_packed_guid(&pet_guid);
    pkt.write_bits(5, 7);
    pkt.write_bit(false);
    pkt.write_string("Misha");

    let decoded = BattlePetModifyName::read_like_cpp(&mut pkt).unwrap();
    assert_eq!(
        decoded,
        BattlePetModifyName {
            pet_guid,
            name: "Misha".to_string(),
            declined_names: None,
        }
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn battle_pet_modify_name_reads_declined_names_before_name_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4333);
    let declined = ["Mishy", "Mishys", "Mishyu", "Mishy2", "Mishy3"];
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(0xBADD);
    pkt.write_packed_guid(&pet_guid);
    pkt.write_bits(5, 7);
    pkt.write_bit(true);
    for name in declined {
        pkt.write_bits(name.len() as u32, 7);
    }
    for name in declined {
        pkt.write_string(name);
    }
    pkt.write_string("Misha");

    let decoded = BattlePetModifyName::read_like_cpp(&mut pkt).unwrap();
    assert_eq!(decoded.pet_guid, pet_guid);
    assert_eq!(decoded.name, "Misha");
    assert_eq!(
        decoded.declined_names.unwrap().names,
        declined.map(str::to_string)
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn query_battle_pet_name_reads_cpp_shape() {
    let battle_pet_id = ObjectGuid::new(0, 0x4326);
    let unit_guid = ObjectGuid::new(0, 0x4327);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::QueryBattlePetName as u16);
    pkt.write_packed_guid(&battle_pet_id);
    pkt.write_packed_guid(&unit_guid);

    let decoded = QueryBattlePetName::read(&mut pkt).unwrap();
    assert_eq!(
        decoded,
        QueryBattlePetName {
            battle_pet_id,
            unit_guid,
        }
    );
}
