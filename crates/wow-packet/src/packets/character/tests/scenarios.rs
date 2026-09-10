//! Character packet regressions.
//!
//! Moved out of character.rs under #685; every test is unchanged.

use super::*;

#[test]
fn set_title_reads_cpp_int32_title_id() {
    for title_id in [-1, 0, 42] {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(title_id);
        pkt.reset_read();

        assert_eq!(SetTitle::read(&mut pkt).unwrap().title_id, title_id);
    }
}

#[test]
fn enum_characters_result_empty_list() {
    let pkt_data = EnumCharactersResult {
        success: true,
        characters: vec![],
        race_unlock_data: vec![],
    };
    let bytes = pkt_data.to_bytes();
    // Should have at least opcode + bits + counts
    assert!(bytes.len() > 2);
}

#[test]
fn enum_characters_result_preserves_pet_data_field_order_like_cpp() {
    let character = CharacterInfo {
        name: "Petowner".to_string(),
        pet_display_id: 0x1122_3344,
        pet_level: 0x5566_7788,
        pet_family: 0x99aa_bbcc,
        ..CharacterInfo::default()
    };
    let bytes = EnumCharactersResult {
        success: true,
        characters: vec![character],
        race_unlock_data: vec![],
    }
    .to_bytes();

    let pet_sequence = [
        0x44, 0x33, 0x22, 0x11, // PetCreatureDisplayID
        0x88, 0x77, 0x66, 0x55, // PetExperienceLevel
        0xcc, 0xbb, 0xaa, 0x99, // PetCreatureFamilyID
    ];
    assert_eq!(
        bytes
            .windows(pet_sequence.len())
            .filter(|window| *window == pet_sequence)
            .count(),
        1
    );
}

#[test]
fn create_char_response_roundtrip() {
    let resp = CreateChar {
        code: response_codes::CHAR_CREATE_SUCCESS,
        guid: ObjectGuid::create_player(1, 100),
    };
    let bytes = resp.to_bytes();
    assert!(bytes.len() > 2);
}

#[test]
fn delete_char_response_roundtrip() {
    let resp = DeleteChar {
        code: response_codes::CHAR_DELETE_SUCCESS,
    };
    let bytes = resp.to_bytes();
    assert_eq!(bytes.len(), 3); // opcode(2) + code(1)
}

#[test]
fn login_verify_world_serialization() {
    let pkt = LoginVerifyWorld {
        map_id: 0,
        position: Position::new(-8949.95, -132.493, 83.5312, 0.0),
        reason: 0,
    };
    let bytes = pkt.to_bytes();
    // opcode(2) + map_id(4) + x(4) + y(4) + z(4) + o(4) + reason(4) = 26
    assert_eq!(bytes.len(), 26);
}

#[test]
fn enum_characters_empty_read() {
    let mut pkt = WorldPacket::from_bytes(&[0x00, 0x00]);
    pkt.skip_opcode();
    let result = EnumCharacters::read(&mut pkt);
    assert!(result.is_ok());
}

#[test]
fn char_delete_read_roundtrip() {
    let guid = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);

    pkt.reset_read();
    let result = CharDelete::read(&mut pkt).unwrap();
    assert_eq!(result.guid, guid);
}

#[test]
fn player_login_read_roundtrip() {
    let guid = ObjectGuid::create_player(1, 99);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    pkt.write_float(1000.0);

    pkt.reset_read();
    let result = PlayerLogin::read(&mut pkt).unwrap();
    assert_eq!(result.guid, guid);
    assert!((result.far_clip - 1000.0).abs() < 0.001);
}

#[test]
fn character_rename_request_reads_cpp_full_guid_then_name_bits() {
    let guid = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&guid);
    pkt.write_bits(7, 6);
    pkt.write_string("Newname");
    pkt.reset_read();

    let result = CharacterRenameRequest::read(&mut pkt).unwrap();

    assert_eq!(result.guid, guid);
    assert_eq!(result.new_name, "Newname");
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn character_rename_result_writes_cpp_result_guid_bit_name_len_and_payload() {
    let guid = ObjectGuid::create_player(1, 42);
    let bytes = CharacterRenameResult {
        result: 0,
        name: "Newname".to_string(),
        guid: Some(guid),
    }
    .to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes);

    assert_eq!(
        pkt.server_opcode(),
        Some(ServerOpcodes::CharacterRenameResult)
    );
    pkt.skip_opcode();
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert!(pkt.read_bit().unwrap());
    assert_eq!(pkt.read_bits(6).unwrap(), 7);
    assert_eq!(pkt.read_guid().unwrap(), guid);
    assert_eq!(pkt.read_string(7).unwrap(), "Newname");
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn char_customize_reads_cpp_guid_sex_customizations_then_name_bits() {
    let guid = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&guid);
    pkt.write_uint8(1);
    pkt.write_uint32(2);
    pkt.write_int32(20);
    pkt.write_int32(200);
    pkt.write_int32(10);
    pkt.write_int32(100);
    pkt.write_bits(7, 6);
    pkt.write_string("Newname");
    pkt.reset_read();

    let result = CharCustomize::read(&mut pkt).unwrap();

    assert_eq!(result.guid, guid);
    assert_eq!(result.sex_id, 1);
    assert_eq!(result.name, "Newname");
    assert_eq!(
        result.customizations,
        vec![
            ChrCustomizationChoice {
                option_id: 10,
                choice_id: 100,
            },
            ChrCustomizationChoice {
                option_id: 20,
                choice_id: 200,
            },
        ]
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn char_customize_success_writes_cpp_guid_sex_customizations_name() {
    let guid = ObjectGuid::create_player(1, 42);
    let bytes = CharCustomizeSuccess {
        guid,
        sex_id: 1,
        customizations: vec![ChrCustomizationChoice {
            option_id: 10,
            choice_id: 100,
        }],
        name: "Newname".to_string(),
    }
    .to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes);

    assert_eq!(
        pkt.server_opcode(),
        Some(ServerOpcodes::CharCustomizeSuccess)
    );
    pkt.skip_opcode();
    assert_eq!(pkt.read_guid().unwrap(), guid);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint32().unwrap(), 1);
    assert_eq!(pkt.read_int32().unwrap(), 10);
    assert_eq!(pkt.read_int32().unwrap(), 100);
    assert_eq!(pkt.read_bits(6).unwrap(), 7);
    assert_eq!(pkt.read_string(7).unwrap(), "Newname");
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn char_customize_failure_writes_cpp_result_then_guid() {
    let guid = ObjectGuid::create_player(1, 42);
    let bytes = CharCustomizeFailure { result: 25, guid }.to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes);

    assert_eq!(
        pkt.server_opcode(),
        Some(ServerOpcodes::CharCustomizeFailure)
    );
    pkt.skip_opcode();
    assert_eq!(pkt.read_uint8().unwrap(), 25);
    assert_eq!(pkt.read_guid().unwrap(), guid);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn create_character_read() {
    let mut pkt = WorldPacket::new_empty();
    // Name "Test" = 4 chars
    pkt.write_bits(4, 6);
    pkt.write_bit(false); // has_template_set
    pkt.write_bit(false); // is_trial_boost
    pkt.write_bit(false); // use_npe

    pkt.write_uint8(1); // race: Human
    pkt.write_uint8(1); // class: Warrior
    pkt.write_int8(0); // sex: Male
    pkt.write_uint32(0); // customization_count

    pkt.write_string("Test");

    pkt.reset_read();
    let result = CreateCharacter::read(&mut pkt).unwrap();
    assert_eq!(result.name, "Test");
    assert_eq!(result.race, 1);
    assert_eq!(result.class, 1);
    assert_eq!(result.sex, 0);
    assert!(result.template_set.is_none());
}

#[test]
fn create_character_sorts_customizations_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(4, 6);
    pkt.write_bit(false); // has_template_set
    pkt.write_bit(false); // is_trial_boost
    pkt.write_bit(false); // use_npe

    pkt.write_uint8(1);
    pkt.write_uint8(1);
    pkt.write_int8(0);
    pkt.write_uint32(3);
    pkt.write_string("Test");
    pkt.write_int32(20);
    pkt.write_int32(200);
    pkt.write_int32(10);
    pkt.write_int32(100);
    pkt.write_int32(20);
    pkt.write_int32(201);

    pkt.reset_read();
    let result = CreateCharacter::read(&mut pkt).unwrap();
    let choices: Vec<(i32, i32)> = result
        .customizations
        .iter()
        .map(|choice| (choice.option_id, choice.choice_id))
        .collect();
    assert_eq!(choices, vec![(10, 100), (20, 200), (20, 201)]);
}

#[test]
fn alter_appearance_reads_cpp_count_sex_race_model_and_sorted_customizations() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(3);
    pkt.write_uint8(1);
    pkt.write_int32(7);
    pkt.write_int32(11);
    pkt.write_int32(20);
    pkt.write_int32(200);
    pkt.write_int32(10);
    pkt.write_int32(100);
    pkt.write_int32(20);
    pkt.write_int32(201);
    pkt.reset_read();

    let parsed = AlterAppearance::read(&mut pkt).unwrap();

    assert_eq!(parsed.new_sex, 1);
    assert_eq!(parsed.customized_race, 7);
    assert_eq!(parsed.customized_chr_model_id, 11);
    let choices: Vec<(i32, i32)> = parsed
        .customizations
        .iter()
        .map(|choice| (choice.option_id, choice.choice_id))
        .collect();
    assert_eq!(choices, vec![(10, 100), (20, 200), (20, 201)]);
}

#[test]
fn confirm_barbers_choice_reads_cpp_count_and_uint32_rows_without_sorting() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(2);
    pkt.write_uint32(20);
    pkt.write_uint32(200);
    pkt.write_uint32(10);
    pkt.write_uint32(100);
    pkt.reset_read();

    let parsed = ConfirmBarbersChoice::read(&mut pkt).unwrap();

    let choices: Vec<(i32, i32)> = parsed
        .customizations
        .iter()
        .map(|choice| (choice.option_id, choice.choice_id))
        .collect();
    assert_eq!(choices, vec![(20, 200), (10, 100)]);
}

#[test]
fn barber_shop_result_writes_cpp_int32_result() {
    let bytes = BarberShopResult {
        result: BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP,
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::BarberShopResult as u16
    );
    let mut payload = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(
        payload.read_int32().unwrap(),
        BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP
    );
    assert_eq!(payload.remaining(), 0);
}

#[test]
fn set_player_declined_names_reads_cpp_guid_lengths_then_strings() {
    let guid = ObjectGuid::create_player(1, 42);
    let names = ["Gen", "Dat", "Acc", "Inst", "Prep"];
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&guid);
    for name in names {
        pkt.write_bits(name.len() as u32, 7);
    }
    for name in names {
        pkt.write_string(name);
    }
    pkt.reset_read();

    let parsed = SetPlayerDeclinedNames::read(&mut pkt).unwrap();

    assert_eq!(parsed.player, guid);
    assert_eq!(parsed.declined_names.names, names.map(str::to_string));
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn set_player_declined_names_result_writes_cpp_result_then_guid() {
    let guid = ObjectGuid::create_player(1, 42);
    let bytes = SetPlayerDeclinedNamesResult {
        player: guid,
        result_code: DECLINED_NAMES_RESULT_ERROR_LIKE_CPP,
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::SetPlayerDeclinedNamesResult as u16
    );
    let mut payload = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(
        payload.read_int32().unwrap(),
        DECLINED_NAMES_RESULT_ERROR_LIKE_CPP
    );
    assert_eq!(payload.read_guid().unwrap(), guid);
    assert_eq!(payload.remaining(), 0);
}

#[test]
fn enum_characters_with_one_character() {
    let char_info = CharacterInfo {
        guid: ObjectGuid::create_player(1, 42),
        name: "TestChar".into(),
        list_position: 0,
        race_id: 1,
        class_id: 1,
        sex_id: 0,
        experience_level: 1,
        zone_id: 12,
        map_id: 0,
        position: Position::new(-8949.95, -132.493, 83.5312, 0.0),
        guild_guid: ObjectGuid::EMPTY,
        first_login: true,
        ..CharacterInfo::default()
    };

    let pkt = EnumCharactersResult {
        success: true,
        characters: vec![char_info],
        race_unlock_data: vec![RaceUnlock {
            race_id: 1,
            has_expansion: true,
            has_achievement: false,
            has_heritage_armor: false,
            is_locked: false,
        }],
    };
    let bytes = pkt.to_bytes();
    // Should be a reasonably sized packet
    assert!(bytes.len() > 50);
}
