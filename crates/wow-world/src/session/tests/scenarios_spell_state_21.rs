//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn spell_effect_uncage_battle_pet_rejects_too_high_level_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 230);
    let existing_pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1bc);
    let spell_id = 77_290;

    session.set_player_guid(Some(player_guid));
    install_represented_battle_pet_species_like_cpp(&mut session, 11, 9001, 0);
    session.add_represented_battle_pet_packet_info_like_cpp(
        existing_pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 12,
            level: 10,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_UNCAGE_BATTLEPET,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData {
                flags: 0x2,
                unit: player_guid,
                ..SpellTargetData::default()
            },
            SpellCastMetadata {
                cast_item_battle_pet_modifiers: Some(SpellCastBattlePetItemModifiersLikeCpp {
                    source_item_guid: ObjectGuid::create_item(1, 901),
                    species_id: 11,
                    breed_data: 7 | (3 << 24),
                    level: 11,
                    display_id: 33,
                }),
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("represented uncage failure should send packets");

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 4);
    assert_eq!(
        packets
            .iter()
            .map(|bytes| {
                let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
                packet.read_uint16().expect("opcode")
            })
            .collect::<Vec<_>>(),
        vec![
            ServerOpcodes::SpellGo as u16,
            ServerOpcodes::BattlePetError as u16,
            ServerOpcodes::CastFailed as u16,
            ServerOpcodes::CooldownEvent as u16,
        ]
    );

    let mut error = wow_packet::WorldPacket::from_bytes(&packets[1]);
    let _ = error.read_uint16().expect("opcode");
    assert_eq!(
        error.read_bits(4).expect("result"),
        wow_packet::packets::misc::BattlePetErrorCodeLikeCpp::TooHighLevelToUncage as u32
    );
    assert_eq!(error.read_int32().expect("creature id"), 9001);

    let mut failed = wow_packet::WorldPacket::from_bytes(&packets[2]);
    let _ = failed.read_uint16().expect("opcode");
    let _ = failed.read_packed_guid().expect("cast id");
    assert_eq!(failed.read_int32().expect("spell id"), spell_id);
    let _ = wow_packet::packets::spell::SpellCastVisual::read(&mut failed).expect("visual");
    assert_eq!(
        failed.read_int32().expect("reason"),
        SpellCastResult::CantAddBattlePet as i32
    );
}
#[tokio::test]
async fn spell_effect_uncage_battle_pet_rejects_max_species_count_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 231);
    let spell_id = 77_291;

    session.set_player_guid(Some(player_guid));
    install_represented_battle_pet_species_like_cpp(&mut session, 11, 9002, 0);
    for counter in 0x1bd..=0x1bf {
        session.add_represented_battle_pet_packet_info_like_cpp(
            ObjectGuid::create_global(HighGuid::BattlePet, 0, counter),
            RepresentedBattlePetDataLikeCpp {
                species: 11,
                level: 25,
                save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
                ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                    0,
                    RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
                )
            },
        );
    }

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_UNCAGE_BATTLEPET,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData {
                flags: 0x2,
                unit: player_guid,
                ..SpellTargetData::default()
            },
            SpellCastMetadata {
                cast_item_battle_pet_modifiers: Some(SpellCastBattlePetItemModifiersLikeCpp {
                    source_item_guid: ObjectGuid::create_item(1, 902),
                    species_id: 11,
                    breed_data: 7 | (3 << 24),
                    level: 1,
                    display_id: 33,
                }),
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("represented uncage max-count failure should send packets");

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 4);
    let mut error = wow_packet::WorldPacket::from_bytes(&packets[1]);
    let _ = error.read_uint16().expect("opcode");
    assert_eq!(
        error.read_bits(4).expect("result"),
        wow_packet::packets::misc::BattlePetErrorCodeLikeCpp::CantHaveMorePetsOfType as u32
    );
    assert_eq!(error.read_int32().expect("creature id"), 9002);

    let mut failed = wow_packet::WorldPacket::from_bytes(&packets[2]);
    let _ = failed.read_uint16().expect("opcode");
    let _ = failed.read_packed_guid().expect("cast id");
    assert_eq!(failed.read_int32().expect("spell id"), spell_id);
    let _ = wow_packet::packets::spell::SpellCastVisual::read(&mut failed).expect("visual");
    assert_eq!(
        failed.read_int32().expect("reason"),
        SpellCastResult::CantAddBattlePet as i32
    );
}
#[tokio::test]
async fn spell_effect_uncage_battle_pet_adds_pet_and_updates_criteria_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 232);
    let existing_pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1c0);
    let spell_id = 77_292;

    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    install_represented_battle_pet_stat_stores_like_cpp(&mut session);
    install_represented_battle_pet_species_like_cpp(
        &mut session,
        11,
        9003,
        wow_data::BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP
            | wow_data::BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP,
    );
    session.add_represented_battle_pet_packet_info_like_cpp(
        existing_pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 12,
            level: 25,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_UNCAGE_BATTLEPET,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData {
                flags: 0x2,
                unit: player_guid,
                ..SpellTargetData::default()
            },
            SpellCastMetadata {
                cast_item_battle_pet_modifiers: Some(SpellCastBattlePetItemModifiersLikeCpp {
                    source_item_guid: ObjectGuid::create_item(1, 903),
                    species_id: 11,
                    breed_data: 7 | (3 << 24),
                    level: 3,
                    display_id: 33,
                }),
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("represented uncage success should add a battle pet");

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(
        packets
            .iter()
            .map(|bytes| {
                let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
                packet.read_uint16().expect("opcode")
            })
            .collect::<Vec<_>>(),
        vec![
            ServerOpcodes::SpellGo as u16,
            ServerOpcodes::BattlePetUpdates as u16,
            ServerOpcodes::PlaySpellVisual as u16,
            ServerOpcodes::CooldownEvent as u16,
        ]
    );

    let mut update = wow_packet::WorldPacket::from_bytes(&packets[1]);
    let _ = update.read_uint16().expect("opcode");
    assert_eq!(update.read_uint32().expect("pet count"), 1);
    assert!(update.read_bit().expect("pet added"));
    let pet_guid = update.read_packed_guid().expect("pet guid");
    assert_eq!(update.read_uint32().expect("species"), 11);
    assert_eq!(update.read_uint32().expect("creature"), 9003);
    assert_eq!(update.read_uint32().expect("display"), 33);
    assert_eq!(update.read_uint16().expect("breed"), 7);
    assert_eq!(update.read_uint16().expect("level"), 3);
    assert_eq!(update.read_uint16().expect("exp"), 0);
    assert_eq!(update.read_uint16().expect("flags"), 0);
    assert_eq!(update.read_uint32().expect("power"), 16);
    assert_eq!(update.read_uint32().expect("health"), 235);
    assert_eq!(update.read_uint32().expect("max health"), 235);
    assert_eq!(update.read_uint32().expect("speed"), 10);
    assert_eq!(update.read_uint8().expect("quality"), 3);
    assert_eq!(update.read_bits(7).expect("name length"), 0);
    assert!(update.read_bit().expect("has owner info"));
    assert!(!update.read_bit().expect("no rename"));
    update.flush_bits();
    assert_eq!(update.read_string(0).expect("empty name"), "");
    assert_eq!(update.read_packed_guid().expect("owner guid"), player_guid);
    assert_eq!(update.read_uint32().expect("virtual realm"), 1);
    assert_eq!(update.read_uint32().expect("native realm"), 1);
    assert_eq!(update.remaining(), 0);

    let mut visual = wow_packet::WorldPacket::from_bytes(&packets[2]);
    let _ = visual.read_uint16().expect("opcode");
    let mut source = [0u8; 16];
    for byte in &mut source {
        *byte = visual.read_uint8().expect("visual source byte");
    }
    assert_eq!(ObjectGuid::from_raw_bytes(&source), player_guid);
    let mut target = [0u8; 16];
    for byte in &mut target {
        *byte = visual.read_uint8().expect("visual target byte");
    }
    assert_eq!(ObjectGuid::from_raw_bytes(&target), player_guid);
    let mut transport = [0u8; 16];
    for byte in &mut transport {
        *byte = visual.read_uint8().expect("visual transport byte");
    }
    assert_eq!(ObjectGuid::from_raw_bytes(&transport), ObjectGuid::EMPTY);
    assert_eq!(visual.read_float().expect("target x"), 0.0);
    assert_eq!(visual.read_float().expect("target y"), 0.0);
    assert_eq!(visual.read_float().expect("target z"), 0.0);
    assert_eq!(
        visual.read_uint32().expect("spell visual id"),
        BATTLE_PET_SPELL_VISUAL_UNCAGE_PET_LIKE_CPP
    );
    assert_eq!(visual.read_float().expect("travel speed"), 0.0);
    assert_eq!(visual.read_uint16().expect("hit reason"), 0);
    assert_eq!(visual.read_uint16().expect("miss reason"), 0);
    assert_eq!(visual.read_uint16().expect("reflect status"), 0);
    assert_eq!(visual.read_float().expect("launch delay"), 0.0);
    assert_eq!(visual.read_float().expect("min duration"), 0.0);
    assert!(!visual.read_bit().expect("speed as time"));
    assert_eq!(visual.remaining(), 0);

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("added represented pet");
    assert_eq!(pet.save_info, RepresentedBattlePetSaveInfoLikeCpp::New);
    assert_eq!(
        session.represented_battle_pet_unique_owned_criteria_like_cpp(),
        1
    );
    assert_eq!(
        session.represented_battle_pet_learned_new_pet_criteria_like_cpp(),
        &[11]
    );
}
#[tokio::test]
async fn battle_pet_spell_check_cast_uses_cpp_error_order_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 222);
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1A2);
    let other_pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1A3);
    let creature_guid = ObjectGuid::create_global(HighGuid::Creature, 0, 0xCB00);
    let spell_id = 77_288;

    session.set_player_guid(Some(player_guid));
    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            level: 23,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    assert!(session.battle_pet_summon_toggle_like_cpp(pet_guid));

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_LEVEL,
                effect_base_points: 1,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            creature_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData {
                flags: 0x2,
                unit: creature_guid,
                ..SpellTargetData::default()
            },
            SpellCastMetadata {
                unit_target_battle_pet_companion_guid: Some(other_pet_guid),
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("represented checkcast failure should be sent");

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = wow_packet::WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::CastFailed as u16
    );
    let _cast_id = packet.read_packed_guid().expect("cast id");
    assert_eq!(packet.read_int32().expect("spell id"), spell_id);
    let _ = wow_packet::packets::spell::SpellCastVisual::read(&mut packet).expect("visual");
    assert_eq!(
        packet.read_int32().expect("reason"),
        SpellCastResult::CantDoThatRightNow as i32
    );

    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);
    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            creature_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData {
                flags: 0x2,
                unit: creature_guid,
                ..SpellTargetData::default()
            },
            SpellCastMetadata {
                unit_target_battle_pet_companion_guid: Some(other_pet_guid),
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("represented checkcast failure should be sent");

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = wow_packet::WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::CastFailed as u16
    );
    let _cast_id = packet.read_packed_guid().expect("cast id");
    assert_eq!(packet.read_int32().expect("spell id"), spell_id);
    let _ = wow_packet::packets::spell::SpellCastVisual::read(&mut packet).expect("visual");
    assert_eq!(
        packet.read_int32().expect("reason"),
        SpellCastResult::BadTargets as i32
    );
}
#[tokio::test]
async fn battle_pet_spell_check_cast_species_type_and_level_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 223);
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1A4);
    let creature_guid = ObjectGuid::create_global(HighGuid::Creature, 0, 0xCB01);
    let spell_id = 77_289;

    session.set_player_guid(Some(player_guid));
    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            level: MAX_BATTLE_PET_LEVEL_LIKE_CPP,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.set_battle_pet_species_store(Arc::new(wow_data::BattlePetSpeciesStore::from_entries(
        [wow_data::BattlePetSpeciesEntry {
            id: 11,
            description: String::new(),
            source_text: String::new(),
            creature_id: 0,
            summon_spell_id: 0,
            icon_file_data_id: 0,
            pet_type_enum: 2,
            flags: 0,
            source_type_enum: 0,
            card_ui_model_scene_id: 0,
            loadout_ui_model_scene_id: 0,
        }],
    )));
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);
    assert!(session.battle_pet_summon_toggle_like_cpp(pet_guid));

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect:
                    wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE,
                effect_base_points: 1,
                effect_misc_value_1: 1 << 1,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            creature_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData {
                flags: 0x2,
                unit: creature_guid,
                ..SpellTargetData::default()
            },
            SpellCastMetadata {
                unit_target_battle_pet_companion_guid: Some(pet_guid),
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("represented checkcast failure should be sent");

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = wow_packet::WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::CastFailed as u16
    );
    let _cast_id = packet.read_packed_guid().expect("cast id");
    assert_eq!(packet.read_int32().expect("spell id"), spell_id);
    let _ = wow_packet::packets::spell::SpellCastVisual::read(&mut packet).expect("visual");
    assert_eq!(
        packet.read_int32().expect("reason"),
        SpellCastResult::WrongBattlePetType as i32
    );

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect:
                    wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE,
                effect_base_points: 1,
                effect_misc_value_1: 1 << 2,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            creature_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData {
                flags: 0x2,
                unit: creature_guid,
                ..SpellTargetData::default()
            },
            SpellCastMetadata {
                unit_target_battle_pet_companion_guid: Some(pet_guid),
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("represented checkcast failure should be sent");

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = wow_packet::WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::CastFailed as u16
    );
    let _cast_id = packet.read_packed_guid().expect("cast id");
    assert_eq!(packet.read_int32().expect("spell id"), spell_id);
    let _ = wow_packet::packets::spell::SpellCastVisual::read(&mut packet).expect("visual");
    assert_eq!(
        packet.read_int32().expect("reason"),
        SpellCastResult::GrantPetLevelFail as i32
    );
}
#[tokio::test]
async fn spell_effect_grant_battle_pet_level_uses_unit_companion_guid_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 224);
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1A5);
    let creature_guid = ObjectGuid::create_global(HighGuid::Creature, 0, 0xCB02);
    let spell_id = 77_290;

    session.set_player_guid(Some(player_guid));
    install_represented_battle_pet_stat_stores_like_cpp(&mut session);
    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            creature_id: 22,
            display_id: 33,
            breed: 7,
            level: 23,
            exp: 5,
            flags: 6,
            power: 10,
            health: 50,
            max_health: 100,
            speed: 20,
            quality: 3,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);
    assert!(session.battle_pet_summon_toggle_like_cpp(pet_guid));

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_LEVEL,
                effect_base_points: 2,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            creature_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData {
                flags: 0x2,
                unit: creature_guid,
                ..SpellTargetData::default()
            },
            SpellCastMetadata {
                unit_target_battle_pet_companion_guid: Some(pet_guid),
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("represented battle-pet level spell effect should execute");

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("leveled pet");
    assert_eq!(pet.level, MAX_BATTLE_PET_LEVEL_LIKE_CPP);
    assert_eq!(pet.exp, 0);
    assert_eq!(pet.health, 1225);
    assert_eq!(pet.max_health, 1225);
    assert_eq!(
        session.represented_battle_pet_level_criteria_like_cpp(),
        &[
            RepresentedBattlePetLevelCriteriaLikeCpp {
                species: 11,
                level: 24
            },
            RepresentedBattlePetLevelCriteriaLikeCpp {
                species: 11,
                level: 25
            }
        ]
    );
    assert!(
        session
            .represented_battle_pet_active_level_criteria_like_cpp()
            .is_empty()
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::BattlePetUpdates,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_effect_change_battle_pet_quality_is_empty_handler_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 225);
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1A6);
    let creature_guid = ObjectGuid::create_global(HighGuid::Creature, 0, 0xCB03);
    let spell_id = 77_291;

    session.set_player_guid(Some(player_guid));
    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            level: 23,
            quality: 2,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);
    assert!(session.battle_pet_summon_toggle_like_cpp(pet_guid));

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_CHANGE_BATTLEPET_QUALITY,
                effect_base_points: 3,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            creature_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData {
                flags: 0x2,
                unit: creature_guid,
                ..SpellTargetData::default()
            },
            SpellCastMetadata {
                unit_target_battle_pet_companion_guid: Some(pet_guid),
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("represented empty battle-pet quality spell handler should execute");

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("unchanged pet");
    assert_eq!(pet.quality, 2);
    assert_eq!(
        pet.save_info,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
