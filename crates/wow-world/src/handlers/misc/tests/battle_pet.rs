// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! battle_pet capability handler tests.

use super::*;
use wow_core::GameTime;

#[tokio::test]
async fn battle_pet_request_journal_lock_sends_acquired_then_journal_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_battle_pet_request_journal_lock(battle_pet_request_journal_lock_packet())
        .await;

    assert!(session.has_represented_battle_pet_journal_lock_like_cpp());
    let bytes = send_rx
        .try_recv()
        .expect("battle pet journal lock acquired packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::BattlePetJournalLockAcquired as u16
    );
    assert_eq!(bytes.len(), 2);

    let journal_bytes = send_rx.try_recv().expect("battle pet journal packet");
    assert_eq!(
        u16::from_le_bytes([journal_bytes[0], journal_bytes[1]]),
        ServerOpcodes::BattlePetJournal as u16
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_request_journal_acquires_lock_then_sends_empty_journal_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_battle_pet_request_journal(battle_pet_request_journal_packet())
        .await;

    let lock_bytes = send_rx.try_recv().expect("journal lock acquired packet");
    assert_eq!(
        u16::from_le_bytes([lock_bytes[0], lock_bytes[1]]),
        ServerOpcodes::BattlePetJournalLockAcquired as u16
    );

    let journal_bytes = send_rx.try_recv().expect("battle pet journal packet");
    assert_eq!(
        u16::from_le_bytes([journal_bytes[0], journal_bytes[1]]),
        ServerOpcodes::BattlePetJournal as u16
    );
    let mut body = WorldPacket::from_bytes(&journal_bytes[2..]);
    assert_eq!(body.read_uint16().unwrap(), 0);
    assert_eq!(body.read_uint32().unwrap(), 3);
    assert_eq!(body.read_uint32().unwrap(), 0);
    assert!(body.read_bit().unwrap());
    for index in 0..3 {
        assert_eq!(
            body.read_packed_guid().unwrap(),
            empty_battle_pet_guid_like_cpp()
        );
        assert_eq!(body.read_uint32().unwrap(), 0);
        assert_eq!(body.read_uint8().unwrap(), index);
        assert!(body.read_bit().unwrap());
    }
    assert_eq!(body.remaining(), 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_request_journal_with_lock_sends_only_journal_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = send_rx.try_recv().expect("initial lock packet");

    session
        .handle_battle_pet_request_journal(battle_pet_request_journal_packet())
        .await;

    let journal_bytes = send_rx.try_recv().expect("battle pet journal packet");
    assert_eq!(
        u16::from_le_bytes([journal_bytes[0], journal_bytes[1]]),
        ServerOpcodes::BattlePetJournal as u16
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_request_journal_sends_represented_pet_rows_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x4338);
    session.set_player_guid(Some(player_guid));
    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        crate::session::RepresentedBattlePetDataLikeCpp {
            species: 11,
            creature_id: 22,
            display_id: 33,
            breed: 44,
            level: 55,
            exp: 66,
            flags: 77,
            power: 88,
            health: 99,
            max_health: 111,
            speed: 222,
            quality: 3,
            owner_info: Some(wow_packet::packets::misc::BattlePetJournalPetOwnerInfo {
                guid: player_guid,
                player_virtual_realm: 123,
                player_native_realm: 456,
            }),
            name: "Misha".to_string(),
            name_timestamp: 0,
            declined_names: None,
            save_info: crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        },
    );
    assert!(session.battle_pet_set_battle_slot_like_cpp(pet_guid, 0));

    session
        .handle_battle_pet_request_journal(battle_pet_request_journal_packet())
        .await;

    let _ = send_rx.try_recv().expect("journal lock acquired packet");
    let journal_bytes = send_rx.try_recv().expect("battle pet journal packet");
    let mut body = WorldPacket::from_bytes(&journal_bytes[2..]);
    assert_eq!(body.read_uint16().unwrap(), 0);
    assert_eq!(body.read_uint32().unwrap(), 3);
    assert_eq!(body.read_uint32().unwrap(), 1);
    assert!(body.read_bit().unwrap());
    assert_eq!(body.read_packed_guid().unwrap(), pet_guid);
    assert_eq!(body.read_uint32().unwrap(), 0);
    assert_eq!(body.read_uint8().unwrap(), 0);
    assert!(body.read_bit().unwrap());
    for index in 1..3 {
        assert_eq!(
            body.read_packed_guid().unwrap(),
            empty_battle_pet_guid_like_cpp()
        );
        assert_eq!(body.read_uint32().unwrap(), 0);
        assert_eq!(body.read_uint8().unwrap(), index);
        assert!(body.read_bit().unwrap());
    }
    assert_eq!(body.read_packed_guid().unwrap(), pet_guid);
    assert_eq!(body.read_uint32().unwrap(), 11);
    assert_eq!(body.read_uint32().unwrap(), 22);
    assert_eq!(body.read_uint32().unwrap(), 33);
    assert_eq!(body.read_uint16().unwrap(), 44);
    assert_eq!(body.read_uint16().unwrap(), 55);
    assert_eq!(body.read_uint16().unwrap(), 66);
    assert_eq!(body.read_uint16().unwrap(), 77);
    assert_eq!(body.read_uint32().unwrap(), 88);
    assert_eq!(body.read_uint32().unwrap(), 99);
    assert_eq!(body.read_uint32().unwrap(), 111);
    assert_eq!(body.read_uint32().unwrap(), 222);
    assert_eq!(body.read_uint8().unwrap(), 3);
    assert_eq!(body.read_bits(7).unwrap(), 5);
    assert!(body.read_bit().unwrap());
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.read_string(5).unwrap(), "Misha");
    assert_eq!(body.read_packed_guid().unwrap(), player_guid);
    assert_eq!(body.read_uint32().unwrap(), 123);
    assert_eq!(body.read_uint32().unwrap(), 456);
    assert_eq!(body.remaining(), 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_clear_fanfare_clears_known_pet_silently_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x223);
    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        crate::session::BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP | 0x20,
        crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );

    session
        .handle_battle_pet_clear_fanfare(battle_pet_clear_fanfare_packet(pet_guid))
        .await;

    assert_eq!(
        session.represented_battle_pet_like_cpp(pet_guid),
        Some(
            crate::session::RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0x20,
                crate::session::RepresentedBattlePetSaveInfoLikeCpp::Changed,
            )
        )
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_clear_fanfare_ignores_unknown_pet_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x224);

    session
        .handle_battle_pet_clear_fanfare(battle_pet_clear_fanfare_packet(pet_guid))
        .await;

    assert!(session.represented_battle_pet_like_cpp(pet_guid).is_none());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_delete_pet_requires_lock_and_marks_removed_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x2241);
    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        0x01,
        crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );

    session
        .handle_battle_pet_delete_pet_represented_like_cpp(battle_pet_delete_pet_packet(pet_guid))
        .await;
    assert_eq!(
        session.represented_battle_pet_like_cpp(pet_guid),
        Some(
            crate::session::RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0x01,
                crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        )
    );
    assert!(send_rx.try_recv().is_err());

    session
        .handle_battle_pet_request_journal_lock(battle_pet_request_journal_lock_packet())
        .await;
    let _ = send_rx.try_recv().expect("lock acquired packet");
    let _ = send_rx.try_recv().expect("battle pet journal packet");

    session
        .handle_battle_pet_delete_pet_represented_like_cpp(battle_pet_delete_pet_packet(pet_guid))
        .await;
    assert_eq!(
        session
            .represented_battle_pet_like_cpp(pet_guid)
            .expect("represented pet row remains until DB save")
            .save_info,
        crate::session::RepresentedBattlePetSaveInfoLikeCpp::Removed
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_delete_pet_ignores_unknown_pet_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x2242);

    session
        .handle_battle_pet_request_journal_lock(battle_pet_request_journal_lock_packet())
        .await;
    let _ = send_rx.try_recv().expect("lock acquired packet");
    let _ = send_rx.try_recv().expect("battle pet journal packet");

    session
        .handle_battle_pet_delete_pet_represented_like_cpp(battle_pet_delete_pet_packet(pet_guid))
        .await;

    assert!(session.represented_battle_pet_like_cpp(pet_guid).is_none());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn cage_battle_pet_requires_lock_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x2243);
    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        crate::session::RepresentedBattlePetDataLikeCpp {
            species: 11,
            creature_id: 22,
            display_id: 33,
            breed: 44,
            level: 17,
            exp: 0,
            flags: 0,
            power: 0,
            health: 100,
            max_health: 100,
            speed: 0,
            quality: 3,
            owner_info: None,
            name: String::new(),
            name_timestamp: 0,
            declined_names: None,
            save_info: crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        },
    );

    session
        .handle_cage_battle_pet_represented_like_cpp(cage_battle_pet_packet(pet_guid))
        .await;

    assert_eq!(
        session
            .represented_battle_pet_like_cpp(pet_guid)
            .expect("pet remains")
            .save_info,
        crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged
    );
    assert!(
        session
            .represented_battle_pet_cage_items_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn cage_battle_pet_handler_delegates_to_represented_manager_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x2244);
    let expected_item = crate::session::RepresentedBattlePetCageItemLikeCpp {
        item_id: crate::session::BATTLE_PET_CAGE_ITEM_ID_LIKE_CPP,
        species_id: 11,
        breed_data: 44 | (3 << 24),
        level: 17,
        display_id: 33,
    };

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        crate::session::RepresentedBattlePetDataLikeCpp {
            species: 11,
            creature_id: 22,
            display_id: 33,
            breed: 44,
            level: 17,
            exp: 0,
            flags: 0,
            power: 0,
            health: 100,
            max_health: 100,
            speed: 0,
            quality: 3,
            owner_info: None,
            name: String::new(),
            name_timestamp: 0,
            declined_names: None,
            save_info: crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        },
    );
    assert!(session.battle_pet_summon_toggle_like_cpp(pet_guid));

    session
        .handle_battle_pet_request_journal_lock(battle_pet_request_journal_lock_packet())
        .await;
    let _ = send_rx.try_recv().expect("lock acquired packet");
    let _ = send_rx.try_recv().expect("battle pet journal packet");

    session
        .handle_cage_battle_pet_represented_like_cpp(cage_battle_pet_packet(pet_guid))
        .await;

    assert_eq!(
        session.represented_battle_pet_cage_items_like_cpp(),
        &[expected_item]
    );
    assert_eq!(
        session
            .represented_battle_pet_like_cpp(pet_guid)
            .expect("removed pet row remains represented")
            .save_info,
        crate::session::RepresentedBattlePetSaveInfoLikeCpp::Removed
    );
    assert_eq!(
        session.represented_summoned_battle_pet_guid_like_cpp(),
        None
    );

    let packet_bytes = send_rx.try_recv().expect("battle pet deleted packet");
    let mut packet = wow_packet::WorldPacket::from_bytes(&packet_bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::BattlePetDeleted as u16
    );
    assert_eq!(packet.read_packed_guid().expect("pet guid"), pet_guid);
    assert_eq!(packet.remaining(), 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_modify_name_requires_lock_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x2245);
    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        0x01,
        crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );

    session
        .handle_battle_pet_modify_name_represented_like_cpp(battle_pet_modify_name_packet(
            pet_guid, "Misha", None,
        ))
        .await;

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("pet remains");
    assert_eq!(pet.name, "");
    assert_eq!(pet.name_timestamp, 0);
    assert_eq!(pet.declined_names, None);
    assert_eq!(
        pet.save_info,
        crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_modify_name_handler_delegates_to_manager_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x2246);
    let declined = ["Alpha", "Betas", "Gamma", "Delta", "Epsil"];
    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        0x01,
        crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );

    session
        .handle_battle_pet_request_journal_lock(battle_pet_request_journal_lock_packet())
        .await;
    let _ = send_rx.try_recv().expect("lock acquired packet");
    let _ = send_rx.try_recv().expect("battle pet journal packet");
    let before = i64::try_from(GameTime::now().as_secs()).unwrap_or(i64::MAX);

    session
        .handle_battle_pet_modify_name_represented_like_cpp(battle_pet_modify_name_packet(
            pet_guid,
            "Misha",
            Some(declined),
        ))
        .await;

    let after = i64::try_from(GameTime::now().as_secs()).unwrap_or(i64::MAX);
    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("pet renamed");
    assert_eq!(pet.name, "Misha");
    assert!((before..=after).contains(&pet.name_timestamp));
    assert_eq!(
        pet.declined_names.as_ref().expect("declined names").names,
        declined.map(str::to_string)
    );
    assert_eq!(
        pet.save_info,
        crate::session::RepresentedBattlePetSaveInfoLikeCpp::Changed
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_set_flags_applies_known_pet_silently_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x225);
    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        0x01,
        crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );

    session
        .handle_battle_pet_set_flags(battle_pet_set_flags_packet(
            pet_guid,
            0x04,
            crate::session::BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP,
        ))
        .await;
    assert_eq!(
        session.represented_battle_pet_like_cpp(pet_guid),
        Some(
            crate::session::RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0x01,
                crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        )
    );
    assert!(send_rx.try_recv().is_err());

    session
        .handle_battle_pet_request_journal_lock(battle_pet_request_journal_lock_packet())
        .await;
    let _ = send_rx.try_recv().expect("lock acquired packet");
    let _ = send_rx.try_recv().expect("battle pet journal packet");

    session
        .handle_battle_pet_set_flags(battle_pet_set_flags_packet(
            pet_guid,
            0x04,
            crate::session::BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP,
        ))
        .await;

    assert_eq!(
        session.represented_battle_pet_like_cpp(pet_guid),
        Some(
            crate::session::RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0x05,
                crate::session::RepresentedBattlePetSaveInfoLikeCpp::Changed,
            )
        )
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_set_battle_slot_assigns_known_pet_silently_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x226);
    let unknown_guid = ObjectGuid::new(0, 0x227);
    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        0,
        crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );

    session
        .handle_battle_pet_set_battle_slot(battle_pet_set_battle_slot_packet(pet_guid, 1))
        .await;
    assert_eq!(
        session.represented_battle_pet_slot_like_cpp(1),
        Some(pet_guid)
    );

    session
        .handle_battle_pet_set_battle_slot(battle_pet_set_battle_slot_packet(unknown_guid, 2))
        .await;
    assert_eq!(session.represented_battle_pet_slot_like_cpp(2), None);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_summon_toggles_known_pet_silently_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x228);
    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        0,
        crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );

    session
        .handle_battle_pet_summon(battle_pet_summon_packet(pet_guid))
        .await;
    assert_eq!(
        session.represented_summoned_battle_pet_guid_like_cpp(),
        Some(pet_guid)
    );
    assert!(send_rx.try_recv().is_err());

    session
        .handle_battle_pet_summon(battle_pet_summon_packet(pet_guid))
        .await;
    assert_eq!(
        session.represented_summoned_battle_pet_guid_like_cpp(),
        None
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_summon_ignores_unknown_pet_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x229);

    session
        .handle_battle_pet_summon(battle_pet_summon_packet(pet_guid))
        .await;

    assert_eq!(
        session.represented_summoned_battle_pet_guid_like_cpp(),
        None
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_update_notify_updates_known_active_pet_silently_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x22a);
    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        0,
        crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );
    assert!(session.battle_pet_summon_toggle_like_cpp(pet_guid));

    session
        .handle_battle_pet_update_notify(battle_pet_update_notify_packet(pet_guid))
        .await;

    assert_eq!(
        session.represented_battle_pet_data_updates_like_cpp(),
        &[pet_guid]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_update_notify_ignores_inactive_or_unknown_pet_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x22b);
    let unknown_guid = ObjectGuid::new(0, 0x22c);
    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        0,
        crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );

    session
        .handle_battle_pet_update_notify(battle_pet_update_notify_packet(pet_guid))
        .await;
    session
        .handle_battle_pet_update_notify(battle_pet_update_notify_packet(unknown_guid))
        .await;

    assert_eq!(session.represented_battle_pet_data_updates_like_cpp(), &[]);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn battle_pet_update_display_notify_is_explicit_noop_like_cpp() {
    let (mut session, send_rx) = make_session();
    session
        .handle_battle_pet_update_display_notify(battle_pet_update_display_notify_packet())
        .await;

    assert_eq!(session.represented_battle_pet_data_updates_like_cpp(), &[]);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn dismiss_critter_clears_active_critter_silently_like_cpp() {
    let (mut session, send_rx) = make_session();
    let critter_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 777, 42);
    session.set_represented_critter_guid_like_cpp(Some(critter_guid));

    session
        .handle_dismiss_critter(dismiss_critter_packet(critter_guid))
        .await;

    assert_eq!(session.represented_critter_guid_like_cpp(), None);
    assert_eq!(
        session.represented_dismissed_critter_guids_like_cpp(),
        &[critter_guid]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn dismiss_critter_ignores_non_active_critter_like_cpp() {
    let (mut session, send_rx) = make_session();
    let active_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 777, 43);
    let requested_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 777, 44);
    session.set_represented_critter_guid_like_cpp(Some(active_guid));

    session
        .handle_dismiss_critter(dismiss_critter_packet(requested_guid))
        .await;

    assert_eq!(
        session.represented_critter_guid_like_cpp(),
        Some(active_guid)
    );
    assert!(
        session
            .represented_dismissed_critter_guids_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn dismiss_critter_clears_matching_battle_pet_data_compat_like_cpp() {
    let (mut session, send_rx) = make_session();
    let critter_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 777, 45);
    let battle_pet_guid = ObjectGuid::new(0, 0x235);
    session.add_represented_battle_pet_like_cpp(
        battle_pet_guid,
        0,
        crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );
    assert!(session.battle_pet_summon_toggle_like_cpp(battle_pet_guid));
    session.set_represented_critter_guid_like_cpp(Some(critter_guid));
    session.set_represented_battle_pet_query_companion_like_cpp(
        critter_guid,
        crate::session::RepresentedBattlePetQueryCompanionLikeCpp {
            creature_id: 777,
            name_timestamp: 0,
            is_summon: true,
            owner_is_player: true,
            battle_pet_companion_guid: Some(battle_pet_guid),
        },
    );

    session
        .handle_dismiss_critter(dismiss_critter_packet(critter_guid))
        .await;

    assert_eq!(session.represented_critter_guid_like_cpp(), None);
    assert_eq!(
        session.represented_summoned_battle_pet_guid_like_cpp(),
        None
    );
    assert_eq!(
        session.represented_dismissed_critter_guids_like_cpp(),
        &[critter_guid]
    );
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn dismiss_critter_handler_metadata_like_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::DismissCritter)
        .expect("DismissCritter handler entry");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "handle_dismiss_critter");
}

#[test]
fn battle_pet_update_display_notify_handler_metadata_like_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::BattlePetUpdateDisplayNotify)
        .expect("BattlePetUpdateDisplayNotify handler entry");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(
        entry.handler_name,
        "handle_battle_pet_update_display_notify"
    );
}

#[tokio::test]
async fn query_battle_pet_name_sends_negative_response_like_cpp_until_runtime_exists() {
    let (mut session, send_rx) = make_session();
    let battle_pet_id = ObjectGuid::new(0, 0x22d);
    let unit_guid = ObjectGuid::new(0, 0x22e);

    session
        .handle_query_battle_pet_name(query_battle_pet_name_packet(battle_pet_id, unit_guid))
        .await;

    let bytes = send_rx.try_recv().expect("query battle pet name response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::QueryBattlePetNameResponse as u16
    );
    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_packed_guid().unwrap(), battle_pet_id);
    assert_eq!(body.read_int32().unwrap(), 0);
    assert_eq!(body.read_int64().unwrap(), 0);
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.remaining(), 0);
}

#[tokio::test]
async fn query_battle_pet_name_non_summon_keeps_zero_response_like_cpp() {
    let (mut session, send_rx) = make_session();
    let battle_pet_id = ObjectGuid::new(0, 0x22f);
    let unit_guid = ObjectGuid::new(0, 0x230);
    session.set_represented_battle_pet_query_companion_like_cpp(
        unit_guid,
        crate::session::RepresentedBattlePetQueryCompanionLikeCpp {
            creature_id: 777,
            name_timestamp: 1234,
            is_summon: false,
            owner_is_player: true,
            battle_pet_companion_guid: None,
        },
    );

    session
        .handle_query_battle_pet_name(query_battle_pet_name_packet(battle_pet_id, unit_guid))
        .await;

    let bytes = send_rx.try_recv().expect("query battle pet name response");
    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_packed_guid().unwrap(), battle_pet_id);
    assert_eq!(body.read_int32().unwrap(), 0);
    assert_eq!(body.read_int64().unwrap(), 0);
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.remaining(), 0);
}

#[tokio::test]
async fn query_battle_pet_name_preserves_summon_identity_until_allow_gate_like_cpp() {
    let (mut session, send_rx) = make_session();
    let battle_pet_id = ObjectGuid::new(0, 0x231);
    let unit_guid = ObjectGuid::new(0, 0x232);
    session.set_represented_battle_pet_query_companion_like_cpp(
        unit_guid,
        crate::session::RepresentedBattlePetQueryCompanionLikeCpp {
            creature_id: 777,
            name_timestamp: 1234,
            is_summon: true,
            owner_is_player: false,
            battle_pet_companion_guid: None,
        },
    );

    session
        .handle_query_battle_pet_name(query_battle_pet_name_packet(battle_pet_id, unit_guid))
        .await;

    let bytes = send_rx.try_recv().expect("query battle pet name response");
    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_packed_guid().unwrap(), battle_pet_id);
    assert_eq!(body.read_int32().unwrap(), 777);
    assert_eq!(body.read_int64().unwrap(), 1234);
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.remaining(), 0);
}

#[tokio::test]
async fn query_battle_pet_name_allows_known_named_player_pet_like_cpp() {
    let (mut session, send_rx) = make_session();
    let battle_pet_id = ObjectGuid::new(0, 0x233);
    let unit_guid = ObjectGuid::new(0, 0x234);
    let declined = wow_packet::packets::misc::DeclinedNamesLikeCpp {
        names: ["Alpha", "Betas", "Gamma", "Delta", "Epsil"].map(str::to_string),
    };
    session.set_represented_battle_pet_query_companion_like_cpp(
        unit_guid,
        crate::session::RepresentedBattlePetQueryCompanionLikeCpp {
            creature_id: 777,
            name_timestamp: 1234,
            is_summon: true,
            owner_is_player: true,
            battle_pet_companion_guid: None,
        },
    );
    session.add_represented_battle_pet_packet_info_like_cpp(
        battle_pet_id,
        crate::session::RepresentedBattlePetDataLikeCpp {
            species: 11,
            creature_id: 777,
            display_id: 33,
            breed: 44,
            level: 17,
            exp: 0,
            flags: 0,
            power: 0,
            health: 100,
            max_health: 100,
            speed: 0,
            quality: 3,
            owner_info: None,
            name: "Misha".to_string(),
            name_timestamp: 1234,
            declined_names: Some(declined.clone()),
            save_info: crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        },
    );

    session
        .handle_query_battle_pet_name(query_battle_pet_name_packet(battle_pet_id, unit_guid))
        .await;

    let bytes = send_rx.try_recv().expect("query battle pet name response");
    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_packed_guid().unwrap(), battle_pet_id);
    assert_eq!(body.read_int32().unwrap(), 777);
    assert_eq!(body.read_int64().unwrap(), 1234);
    assert!(body.read_bit().unwrap());
    assert_eq!(body.read_bits(8).unwrap(), 5);
    assert!(body.read_bit().unwrap());
    let mut declined_lengths = [0usize; 5];
    for length in &mut declined_lengths {
        *length = body.read_bits(7).unwrap() as usize;
    }
    let declined_names = declined_lengths
        .iter()
        .map(|length| body.read_string(*length).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(declined_names, declined.names);
    assert_eq!(body.read_string(5).unwrap(), "Misha");
    assert_eq!(body.remaining(), 0);
}
