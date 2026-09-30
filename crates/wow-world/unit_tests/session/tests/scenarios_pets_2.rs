//! Session scenarios exercising the represented pets responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn battle_pet_modify_name_requires_lock_and_updates_name_state_like_cpp() {
    let (mut session, _, _) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x128);
    let new_pet_guid = ObjectGuid::new(0, 0x129);
    let unknown_guid = ObjectGuid::new(0, 0x12a);
    let declined = wow_packet::packets::misc::DeclinedNamesLikeCpp {
        names: ["Alpha", "Betas", "Gamma", "Delta", "Epsil"].map(str::to_string),
    };

    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        0x10,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );
    session.add_represented_battle_pet_like_cpp(
        new_pet_guid,
        0,
        RepresentedBattlePetSaveInfoLikeCpp::New,
    );

    assert!(!session.battle_pet_modify_name_like_cpp(pet_guid, "NoLock".to_string(), None, 10,));
    assert_eq!(
        session.represented_battle_pet_like_cpp(pet_guid),
        Some(RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
            0x10,
            RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        ))
    );

    session.send_battle_pet_journal_lock_status_like_cpp().await;
    assert!(!session.battle_pet_modify_name_like_cpp(unknown_guid, "Ghost".to_string(), None, 11,));
    assert!(session.battle_pet_modify_name_like_cpp(
        pet_guid,
        "Misha".to_string(),
        Some(declined.clone()),
        12_345,
    ));
    assert!(session.battle_pet_modify_name_like_cpp(
        new_pet_guid,
        "Newbie".to_string(),
        None,
        12_346,
    ));

    let changed_pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("changed pet");
    assert_eq!(changed_pet.name, "Misha");
    assert_eq!(changed_pet.name_timestamp, 12_345);
    assert_eq!(changed_pet.declined_names, Some(declined));
    assert_eq!(
        changed_pet.save_info,
        RepresentedBattlePetSaveInfoLikeCpp::Changed
    );

    let new_pet = session
        .represented_battle_pet_like_cpp(new_pet_guid)
        .expect("new pet");
    assert_eq!(new_pet.name, "Newbie");
    assert_eq!(new_pet.name_timestamp, 12_346);
    assert_eq!(new_pet.declined_names, None);
    assert_eq!(new_pet.save_info, RepresentedBattlePetSaveInfoLikeCpp::New);
}
#[test]
fn battle_pet_set_flags_applies_or_removes_known_pet_only_like_cpp() {
    let (mut session, _, _) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x12b);
    let new_pet_guid = ObjectGuid::new(0, 0x12c);
    let unknown_guid = ObjectGuid::new(0, 0x12d);

    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        0x10,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );
    session.add_represented_battle_pet_like_cpp(
        new_pet_guid,
        0x30,
        RepresentedBattlePetSaveInfoLikeCpp::New,
    );

    assert!(session.battle_pet_set_flags_like_cpp(
        pet_guid,
        0x02,
        BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP
    ));
    assert_eq!(
        session.represented_battle_pet_like_cpp(pet_guid),
        Some(RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
            0x12,
            RepresentedBattlePetSaveInfoLikeCpp::Changed,
        ))
    );

    assert!(session.battle_pet_set_flags_like_cpp(pet_guid, 0x10, 2));
    assert_eq!(
        session.represented_battle_pet_like_cpp(pet_guid),
        Some(RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
            0x02,
            RepresentedBattlePetSaveInfoLikeCpp::Changed,
        ))
    );

    assert!(session.battle_pet_set_flags_like_cpp(new_pet_guid, 0x10, 0));
    assert_eq!(
        session.represented_battle_pet_like_cpp(new_pet_guid),
        Some(RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
            0x20,
            RepresentedBattlePetSaveInfoLikeCpp::New,
        ))
    );
    assert!(!session.battle_pet_set_flags_like_cpp(
        unknown_guid,
        0x01,
        BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP
    ));
}
#[test]
fn battle_pet_set_battle_slot_assigns_known_pet_to_valid_slot_like_cpp() {
    let (mut session, _, _) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x12e);
    let other_guid = ObjectGuid::new(0, 0x12f);
    let unknown_guid = ObjectGuid::new(0, 0x130);

    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        0,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );
    session.add_represented_battle_pet_like_cpp(
        other_guid,
        0,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );

    assert!(session.battle_pet_set_battle_slot_like_cpp(pet_guid, 2));
    assert_eq!(
        session.represented_battle_pet_slot_like_cpp(2),
        Some(pet_guid)
    );

    assert!(!session.battle_pet_set_battle_slot_like_cpp(unknown_guid, 1));
    assert_eq!(session.represented_battle_pet_slot_like_cpp(1), None);

    assert!(!session.battle_pet_set_battle_slot_like_cpp(other_guid, 3));
    assert_eq!(
        session.represented_battle_pet_slot_like_cpp(2),
        Some(pet_guid)
    );
}
#[test]
fn battle_pet_unlock_slot_sends_slot_update_once_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x131);

    assert_eq!(
        session.represented_battle_pet_slot_locked_like_cpp(1),
        Some(true)
    );
    assert!(!session.battle_pet_unlock_slot_like_cpp(3));

    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        0,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );
    assert!(session.battle_pet_set_battle_slot_like_cpp(pet_guid, 1));
    assert!(session.battle_pet_unlock_slot_like_cpp(1));
    assert_eq!(
        session.represented_battle_pet_slot_locked_like_cpp(1),
        Some(false)
    );
    assert!(!session.battle_pet_unlock_slot_like_cpp(1));

    let bytes = send_rx.try_recv().expect("slot update packet");
    assert!(send_rx.try_recv().is_err(), "unlock sends once");
    let mut packet = wow_packet::WorldPacket::from_bytes(&bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::PetBattleSlotUpdates as u16
    );
    assert_eq!(packet.read_uint32().expect("slot count"), 1);
    assert!(packet.read_bit().expect("new slot"));
    assert!(!packet.read_bit().expect("auto slotted"));
    assert_eq!(packet.read_packed_guid().expect("pet guid"), pet_guid);
    assert_eq!(packet.read_uint32().expect("collar"), 0);
    assert_eq!(packet.read_uint8().expect("index"), 1);
    assert!(!packet.read_bit().expect("locked"));
    assert_eq!(packet.remaining(), 0);
}
#[test]
fn send_battle_pet_updates_emits_known_non_removed_rows_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x132);
    let removed_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x133);
    let unknown_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x134);

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            creature_id: 22,
            display_id: 33,
            breed: 44,
            level: 17,
            exp: 66,
            flags: 77,
            power: 88,
            health: 99,
            max_health: 111,
            speed: 222,
            quality: 3,
            owner_info: None,
            name: "Misha".to_string(),
            name_timestamp: 0,
            declined_names: None,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        },
    );
    session.add_represented_battle_pet_like_cpp(
        removed_guid,
        0,
        RepresentedBattlePetSaveInfoLikeCpp::Removed,
    );

    assert_eq!(
        session.send_battle_pet_updates_like_cpp(&[pet_guid, removed_guid, unknown_guid], true),
        1
    );

    let bytes = send_rx.try_recv().expect("battle pet updates packet");
    assert!(send_rx.try_recv().is_err(), "updates sends once");
    let mut packet = wow_packet::WorldPacket::from_bytes(&bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::BattlePetUpdates as u16
    );
    assert_eq!(packet.read_uint32().expect("pet count"), 1);
    assert!(packet.read_bit().expect("pet added"));
    assert_eq!(packet.read_packed_guid().expect("pet guid"), pet_guid);
    assert_eq!(packet.read_uint32().expect("species"), 11);
    assert_eq!(packet.read_uint32().expect("creature"), 22);
}
#[test]
fn battle_pet_send_error_emits_cpp_error_packet_like_cpp() {
    let (mut session, _, send_rx) = make_session();

    session.battle_pet_send_error_like_cpp(
        wow_packet::packets::misc::BattlePetErrorCodeLikeCpp::TooHighLevelToUncage,
        12_345,
    );

    let bytes = send_rx.try_recv().expect("battle pet error packet");
    assert!(send_rx.try_recv().is_err(), "error sends once");
    let mut packet = wow_packet::WorldPacket::from_bytes(&bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::BattlePetError as u16
    );
    assert_eq!(
        packet.read_bits(4).expect("result"),
        wow_packet::packets::misc::BattlePetErrorCodeLikeCpp::TooHighLevelToUncage as u32
    );
    assert_eq!(packet.read_int32().expect("creature id"), 12_345);
    assert_eq!(packet.remaining(), 0);
}
#[test]
fn battle_pet_max_pet_level_ignores_removed_rows_like_cpp() {
    let (mut session, _, _) = make_session();
    let low_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1b0);
    let high_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1b1);
    let removed_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1b2);

    assert_eq!(session.battle_pet_max_pet_level_like_cpp(), Some(0));

    session.add_represented_battle_pet_packet_info_like_cpp(
        low_guid,
        RepresentedBattlePetDataLikeCpp {
            level: 7,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.add_represented_battle_pet_packet_info_like_cpp(
        high_guid,
        RepresentedBattlePetDataLikeCpp {
            level: 19,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::New,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::New,
            )
        },
    );
    session.add_represented_battle_pet_packet_info_like_cpp(
        removed_guid,
        RepresentedBattlePetDataLikeCpp {
            level: 25,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Removed,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Removed,
            )
        },
    );

    assert_eq!(session.battle_pet_max_pet_level_like_cpp(), Some(19));
}
#[test]
fn battle_pet_has_max_pet_count_uses_cpp_default_species_limit() {
    let (mut session, _, _) = make_session();
    install_represented_battle_pet_species_flags_like_cpp(&mut session, 11, 0);

    for counter in 0x1b3..=0x1b5 {
        session.add_represented_battle_pet_packet_info_like_cpp(
            ObjectGuid::create_global(HighGuid::BattlePet, 0, counter),
            RepresentedBattlePetDataLikeCpp {
                species: 11,
                save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
                ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                    0,
                    RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
                )
            },
        );
    }
    session.add_represented_battle_pet_packet_info_like_cpp(
        ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1b6),
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Removed,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Removed,
            )
        },
    );
    session.add_represented_battle_pet_packet_info_like_cpp(
        ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1b7),
        RepresentedBattlePetDataLikeCpp {
            species: 12,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );

    assert_eq!(session.battle_pet_count_like_cpp(11, None), 3);
    assert_eq!(
        session.battle_pet_has_max_pet_count_like_cpp(11, None),
        Some(true)
    );
    assert_eq!(
        session.battle_pet_has_max_pet_count_like_cpp(12, None),
        Some(false)
    );
}
#[test]
fn battle_pet_has_max_pet_count_honors_legacy_account_unique_like_cpp() {
    let (mut session, _, _) = make_session();
    install_represented_battle_pet_species_flags_like_cpp(
        &mut session,
        11,
        wow_data::BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP,
    );

    session.add_represented_battle_pet_packet_info_like_cpp(
        ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1b8),
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::New,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::New,
            )
        },
    );

    assert_eq!(session.battle_pet_count_like_cpp(11, None), 1);
    assert_eq!(
        session.battle_pet_has_max_pet_count_like_cpp(11, None),
        Some(true)
    );
}
#[test]
fn battle_pet_count_filters_not_account_wide_owner_like_cpp() {
    let (mut session, _, _) = make_session();
    let owner_a = ObjectGuid::create_global(HighGuid::Player, 0, 0x20a);
    let owner_b = ObjectGuid::create_global(HighGuid::Player, 0, 0x20b);
    install_represented_battle_pet_species_flags_like_cpp(
        &mut session,
        11,
        wow_data::BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP,
    );

    for (counter, owner_info) in [
        (
            0x1b9,
            Some(wow_packet::packets::misc::BattlePetJournalPetOwnerInfo {
                guid: owner_a,
                player_virtual_realm: 1,
                player_native_realm: 1,
            }),
        ),
        (
            0x1ba,
            Some(wow_packet::packets::misc::BattlePetJournalPetOwnerInfo {
                guid: owner_b,
                player_virtual_realm: 1,
                player_native_realm: 1,
            }),
        ),
        (0x1bb, None),
    ] {
        session.add_represented_battle_pet_packet_info_like_cpp(
            ObjectGuid::create_global(HighGuid::BattlePet, 0, counter),
            RepresentedBattlePetDataLikeCpp {
                species: 11,
                owner_info,
                save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
                ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                    0,
                    RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
                )
            },
        );
    }

    assert_eq!(session.battle_pet_count_like_cpp(11, Some(owner_a)), 2);
    assert_eq!(session.battle_pet_count_like_cpp(11, Some(owner_b)), 2);
    assert_eq!(session.battle_pet_count_like_cpp(11, None), 3);
}
#[test]
fn battle_pet_heal_battle_pets_pct_heals_damaged_pets_and_sends_updates_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x185);
    let full_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x186);

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            creature_id: 22,
            display_id: 33,
            breed: 44,
            level: 17,
            exp: 0,
            flags: 0,
            power: 0,
            health: 40,
            max_health: 101,
            speed: 0,
            quality: 3,
            owner_info: None,
            name: String::new(),
            name_timestamp: 0,
            declined_names: None,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        },
    );
    session.add_represented_battle_pet_packet_info_like_cpp(
        full_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 12,
            health: 100,
            max_health: 100,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );

    assert_eq!(session.battle_pet_heal_battle_pets_pct_like_cpp(50), 1);

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("healed pet");
    assert_eq!(pet.health, 90, "CalculatePct(101, 50) truncates to 50");
    assert_eq!(pet.save_info, RepresentedBattlePetSaveInfoLikeCpp::Changed);
    assert_eq!(
        session
            .represented_battle_pet_like_cpp(full_guid)
            .expect("full pet")
            .save_info,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged
    );

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = wow_packet::WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::BattlePetUpdates as u16
    );
    assert_eq!(packet.read_uint32().expect("pet count"), 1);
    assert!(!packet.read_bit().expect("pet added"));
    assert_eq!(packet.read_packed_guid().expect("pet guid"), pet_guid);
    assert_eq!(packet.read_uint32().expect("species"), 11);
    assert_eq!(packet.read_uint32().expect("creature"), 22);
}
#[test]
fn battle_pet_heal_battle_pets_pct_preserves_new_and_removed_rows() {
    let (mut session, _, send_rx) = make_session();
    let new_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x187);
    let removed_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x188);

    session.add_represented_battle_pet_packet_info_like_cpp(
        new_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            health: 10,
            max_health: 100,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::New,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::New,
            )
        },
    );
    session.add_represented_battle_pet_packet_info_like_cpp(
        removed_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 12,
            health: 10,
            max_health: 100,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Removed,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Removed,
            )
        },
    );

    assert_eq!(session.battle_pet_heal_battle_pets_pct_like_cpp(50), 1);

    let new_pet = session
        .represented_battle_pet_like_cpp(new_guid)
        .expect("new pet");
    assert_eq!(new_pet.health, 60);
    assert_eq!(new_pet.save_info, RepresentedBattlePetSaveInfoLikeCpp::New);

    let removed_pet = session
        .represented_battle_pet_like_cpp(removed_guid)
        .expect("removed pet");
    assert_eq!(removed_pet.health, 10);
    assert_eq!(
        removed_pet.save_info,
        RepresentedBattlePetSaveInfoLikeCpp::Removed
    );

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = wow_packet::WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::BattlePetUpdates as u16
    );
    assert_eq!(packet.read_uint32().expect("pet count"), 1);
    assert!(!packet.read_bit().expect("pet added"));
    assert_eq!(packet.read_packed_guid().expect("pet guid"), new_guid);
}
#[tokio::test]
async fn battle_pet_change_quality_applies_cpp_gates_without_side_effects() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x189);
    let unknown_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x18a);

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            quality: 2,
            health: 50,
            max_health: 100,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );

    assert_eq!(
        session.battle_pet_change_battle_pet_quality_represented_like_cpp(pet_guid, 3),
        RepresentedBattlePetQualityOutcomeLikeCpp::NoJournalLock
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        session.battle_pet_change_battle_pet_quality_represented_like_cpp(unknown_guid, 3),
        RepresentedBattlePetQualityOutcomeLikeCpp::UnknownPet
    );
    assert_eq!(
        session.battle_pet_change_battle_pet_quality_represented_like_cpp(pet_guid, 4),
        RepresentedBattlePetQualityOutcomeLikeCpp::QualityAboveRare
    );
    install_represented_battle_pet_species_flags_like_cpp(
        &mut session,
        11,
        wow_data::BATTLE_PET_SPECIES_FLAG_CANT_BATTLE_LIKE_CPP,
    );
    assert_eq!(
        session.battle_pet_change_battle_pet_quality_represented_like_cpp(pet_guid, 3),
        RepresentedBattlePetQualityOutcomeLikeCpp::CantBattle
    );
    install_represented_battle_pet_species_flags_like_cpp(&mut session, 11, 0);
    assert_eq!(
        session.battle_pet_change_battle_pet_quality_represented_like_cpp(pet_guid, 2),
        RepresentedBattlePetQualityOutcomeLikeCpp::NotUpgrade
    );

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("unchanged pet");
    assert_eq!(pet.quality, 2);
    assert_eq!(pet.health, 50);
    assert_eq!(pet.max_health, 100);
    assert_eq!(
        pet.save_info,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged
    );
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
}
#[tokio::test]
async fn battle_pet_change_quality_applies_stats_heals_and_sends_update_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x18b);
    install_represented_battle_pet_stat_stores_like_cpp(&mut session);

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            creature_id: 22,
            display_id: 33,
            breed: 7,
            level: 2,
            exp: 5,
            flags: 6,
            power: 10,
            health: 50,
            max_health: 100,
            speed: 20,
            quality: 2,
            owner_info: None,
            name: String::new(),
            name_timestamp: 0,
            declined_names: None,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        },
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        session.battle_pet_change_battle_pet_quality_represented_like_cpp(pet_guid, 3),
        RepresentedBattlePetQualityOutcomeLikeCpp::Changed
    );

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("changed pet");
    assert_eq!(pet.quality, 3);
    assert_eq!(pet.health, 190);
    assert_eq!(pet.max_health, 190);
    assert_eq!(pet.power, 11);
    assert_eq!(pet.speed, 7);
    assert_eq!(pet.save_info, RepresentedBattlePetSaveInfoLikeCpp::Changed);

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = wow_packet::WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::BattlePetUpdates as u16
    );
    assert_eq!(packet.read_uint32().expect("pet count"), 1);
    assert!(!packet.read_bit().expect("pet added"));
    assert_eq!(packet.read_packed_guid().expect("pet guid"), pet_guid);
    assert_eq!(packet.read_uint32().expect("species"), 11);
    assert_eq!(packet.read_uint32().expect("creature"), 22);
    assert_eq!(packet.read_uint32().expect("display"), 33);
    assert_eq!(packet.read_uint16().expect("breed"), 7);
    assert_eq!(packet.read_uint16().expect("level"), 2);
    assert_eq!(packet.read_uint16().expect("exp"), 5);
    assert_eq!(packet.read_uint16().expect("flags"), 6);
    assert_eq!(packet.read_uint32().expect("power"), 11);
    assert_eq!(packet.read_uint32().expect("health"), 190);
    assert_eq!(packet.read_uint32().expect("max health"), 190);
    assert_eq!(packet.read_uint32().expect("speed"), 7);
    assert_eq!(packet.read_uint8().expect("quality"), 3);
}
#[tokio::test]
async fn battle_pet_change_quality_does_not_abort_when_calculate_stats_returns_early_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x18bb);
    install_represented_battle_pet_stat_stores_like_cpp(&mut session);

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            breed: 99,
            level: 2,
            power: 10,
            health: 50,
            max_health: 100,
            speed: 20,
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

    assert_eq!(
        session.battle_pet_change_battle_pet_quality_represented_like_cpp(pet_guid, 3),
        RepresentedBattlePetQualityOutcomeLikeCpp::Changed
    );

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("changed pet");
    assert_eq!(pet.quality, 3);
    assert_eq!(pet.max_health, 100);
    assert_eq!(pet.power, 10);
    assert_eq!(pet.speed, 20);
    assert_eq!(pet.health, 100);
    assert_eq!(pet.save_info, RepresentedBattlePetSaveInfoLikeCpp::Changed);
}
#[test]
fn battle_pet_calculate_stats_uses_cpp_db2_state_stores() {
    let (mut session, _, _) = make_session();
    assert_eq!(
        session.battle_pet_calculate_stats_like_cpp(7, 11, 3, 2),
        None
    );

    install_represented_battle_pet_stat_stores_like_cpp(&mut session);

    assert_eq!(
        session.battle_pet_calculate_stats_like_cpp(7, 11, 3, 2),
        Some(RepresentedBattlePetCalculatedStatsLikeCpp {
            max_health: 190,
            power: 11,
            speed: 7,
        })
    );
    assert_eq!(
        session.battle_pet_calculate_stats_like_cpp(8, 11, 3, 2),
        None
    );
}
#[tokio::test]
async fn battle_pet_grant_level_applies_cpp_gates_without_side_effects() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x18c);
    let max_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x18d);
    let unknown_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x18e);

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            level: 17,
            exp: 5,
            health: 50,
            max_health: 100,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.add_represented_battle_pet_packet_info_like_cpp(
        max_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 12,
            level: MAX_BATTLE_PET_LEVEL_LIKE_CPP,
            exp: 5,
            health: 50,
            max_health: 100,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );

    assert_eq!(
        session.battle_pet_grant_battle_pet_level_represented_like_cpp(pet_guid, 1),
        RepresentedBattlePetGrantLevelOutcomeLikeCpp::NoJournalLock
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        session.battle_pet_grant_battle_pet_level_represented_like_cpp(unknown_guid, 1),
        RepresentedBattlePetGrantLevelOutcomeLikeCpp::UnknownPet
    );
    install_represented_battle_pet_species_flags_like_cpp(
        &mut session,
        11,
        wow_data::BATTLE_PET_SPECIES_FLAG_CANT_BATTLE_LIKE_CPP,
    );
    assert_eq!(
        session.battle_pet_grant_battle_pet_level_represented_like_cpp(pet_guid, 1),
        RepresentedBattlePetGrantLevelOutcomeLikeCpp::CantBattle
    );
    install_represented_battle_pet_species_flags_like_cpp(&mut session, 11, 0);
    assert_eq!(
        session.battle_pet_grant_battle_pet_level_represented_like_cpp(max_guid, 1),
        RepresentedBattlePetGrantLevelOutcomeLikeCpp::AlreadyMaxLevel
    );
    assert_eq!(
        session.battle_pet_grant_battle_pet_level_represented_like_cpp(pet_guid, 0),
        RepresentedBattlePetGrantLevelOutcomeLikeCpp::NoGrantedLevels
    );

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("unchanged pet");
    assert_eq!(pet.level, 17);
    assert_eq!(pet.exp, 5);
    assert_eq!(pet.health, 50);
    assert_eq!(pet.max_health, 100);
    assert_eq!(
        pet.save_info,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged
    );
    assert!(
        session
            .represented_battle_pet_level_criteria_like_cpp()
            .is_empty()
    );
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
}
