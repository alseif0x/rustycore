// External application scenarios migrated with their original assertions.

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

// Login packet prelude and cross-socket ordering.

use super::*;
use std::sync::Arc;
use wow_constants::ServerOpcodes;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuidGenerator};
use wow_packet::WorldPacket;
use wow_world::test_fixtures::install_canonical_player_owner_for_test;

fn make_session_with_send_capacity(capacity: usize) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    let mut session = WorldSession::new_character_lifecycle_fixture(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    session.character_set_equipment_set_guid_generator_for_test(Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    (session, send_rx)
}

#[tokio::test]
async fn handle_player_login_prelude_resends_account_state_and_orders_packets_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(32);
    let guid = ObjectGuid::create_player(1, 42);
    let tutorials = [10, 20, 30, 40, 50, 60, 70, 80];
    let mounts = [
        AccountMount {
            spell_id: 100,
            flags: 1,
        },
        AccountMount {
            spell_id: 200,
            flags: 2,
        },
    ];
    session.set_player_guid(Some(guid));
    session.character_load_tutorials_data_values_for_test(Some(tutorials));
    let generators = session.character_id_generators_for_test_for_test();
    let feature_policy = session.character_support_feature_policy_for_test_for_test();
    assert!(
        session
            .character_send_handle_player_login_packets_for_test(
                generators.item.as_ref(),
                &feature_policy,
                guid,
                &Position::new(1.0, 2.0, 3.0, 4.0),
                571,
                &mounts,
                "first@second",
            )
            .await
    );

    let packets = send_rx.try_iter().collect::<Vec<_>>();
    let opcodes = packets
        .iter()
        .filter_map(|bytes| WorldPacket::from_bytes(bytes).server_opcode())
        .collect::<Vec<_>>();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::AccountMountUpdate,
            ServerOpcodes::AccountMountUpdate,
            ServerOpcodes::AccountDataTimes,
            ServerOpcodes::TutorialFlags,
            ServerOpcodes::SetDungeonDifficulty,
            ServerOpcodes::LoginVerifyWorld,
            ServerOpcodes::AccountDataTimes,
            ServerOpcodes::FeatureSystemStatus,
            ServerOpcodes::ChatServerMessage,
            ServerOpcodes::ChatServerMessage,
            ServerOpcodes::SetTimeZoneInformation,
            ServerOpcodes::BattlePetJournalLockAcquired,
        ]
    );

    for (packet, expected_mount) in packets[..2].iter().zip(mounts) {
        let mut body = WorldPacket::from_bytes(&packet[2..]);
        assert!(!body.read_bit().unwrap());
        assert_eq!(body.read_int32().unwrap(), 1);
        assert_eq!(body.read_int32().unwrap(), expected_mount.spell_id);
        assert_eq!(body.read_bits(4).unwrap(), u32::from(expected_mount.flags));
        assert_eq!(body.remaining(), 0);
    }

    let mut global_account_data = WorldPacket::from_bytes(&packets[2][2..]);
    assert_eq!(
        global_account_data.read_packed_guid().unwrap(),
        ObjectGuid::EMPTY
    );
    let mut tutorial_packet = WorldPacket::from_bytes(&packets[3][2..]);
    for expected in tutorials {
        assert_eq!(tutorial_packet.read_uint32().unwrap(), expected);
    }
    assert_eq!(tutorial_packet.remaining(), 0);

    let mut character_account_data = WorldPacket::from_bytes(&packets[6][2..]);
    assert_eq!(character_account_data.read_packed_guid().unwrap(), guid);

    for (packet, expected_line) in packets[8..10].iter().zip(["first", "second"]) {
        let mut body = WorldPacket::from_bytes(&packet[2..]);
        assert_eq!(body.read_int32().unwrap(), 3);
        let string_len = body.read_bits(11).unwrap() as usize;
        assert_eq!(body.read_string(string_len).unwrap(), expected_line);
        assert_eq!(body.remaining(), 0);
    }

    assert!(session.character_has_represented_battle_pet_journal_lock_for_test());
}

fn ensure_login_player_controller_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
    name: String,
    position: Position,
    map_id: u16,
    race: u8,
    class: u8,
    level: u8,
    gender: u8,
) -> bool {
    session.character_ensure_login_player_controller_for_test(
        guid, name, position, map_id, race, class, level, gender,
    )
}

#[tokio::test]
async fn unavailable_login_grid_aborts_before_success_login_packets_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let guid = ObjectGuid::create_player(1, 46);
    assert!(ensure_login_player_controller_for_test(
        &mut session,
        guid,
        "PreflightFailure".to_string(),
        Position::ZERO,
        1,
        1,
        1,
        10,
        0,
    ));
    let player_grid_loader: wow_world::session::PlayerGridLoadResolverLikeCpp =
        Arc::new(|_, _, _| wow_world::session::PlayerGridLoadOutcomeLikeCpp {
            map_unavailable: true,
            ..Default::default()
        });
    install_canonical_player_owner_for_test(&mut session, 1, 0);
    let generators = session.character_id_generators_for_test_for_test();
    let creature_spawn_catalogs = session.character_creature_spawn_catalogs_for_test_for_test();
    let feature_policy = session.character_support_feature_policy_for_test_for_test();

    assert!(
        !session
            .character_send_login_sequence_for_test(
                generators.item.as_ref(),
                &wow_data::trait_tree::TraitNodeEntryStore::from_entries([]),
                &creature_spawn_catalogs,
                &feature_policy,
                &player_grid_loader,
                guid,
                1,
                1,
                0,
                10,
                49,
                &Position::ZERO,
                1,
                0,
                CharacterLoginLocationLikeCpp {
                    map_id: 1,
                    bind_area_id: Some(0),
                    position: Position::ZERO,
                },
                None,
                [(0, 0, 0); 19],
                [ObjectGuid::EMPTY; 141],
                Vec::new(),
                PlayerCombatStats::default(),
                0,
                0,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                [0; 180],
                Vec::new(),
                Vec::new(),
            )
            .await
    );

    assert_eq!(
        session.state(),
        wow_world::session::SessionState::Disconnecting
    );
    assert!(session.player_guid().is_none());
    assert!(
        send_rx.try_recv().is_err(),
        "C++ LoadFromDB failure happens before DungeonDifficultySet/LoginVerifyWorld"
    );
}
