// External application scenarios migrated with their original assertions.

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

// Persisted login-map recovery and partial-login cleanup.

use super::*;
use std::sync::Arc;
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

fn run_login_grid_cleanup_test(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .name("login-grid-cleanup".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn rejected_instance_login_retries_valid_homebind_before_disconnect_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, _send_rx) = make_session_with_send_capacity(2);
        let guid = ObjectGuid::create_player(1, 45);
        let saved_position = Position::new(1.0, 2.0, 3.0, 0.0);
        let homebind_position = Position::new(10.0, 20.0, 30.0, 1.0);
        let canonical: wow_world::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 33,
                instance_type: wow_data::map::MAP_INSTANCE,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
            wow_data::MapEntry {
                id: 1,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        assert!(ensure_login_player_controller_for_test(
            &mut session,
            guid,
            "InstanceFallback".to_string(),
            saved_position,
            33,
            1,
            1,
            10,
            0,
        ));
        assert!(matches!(
            session.character_ensure_canonical_world_map_for_current_player_for_test(),
            Some(wow_map::CreateMapDecision::Reject { .. })
        ));
        assert!(
            session
                .character_current_canonical_player_map_key_for_test()
                .is_none()
        );

        let mut map_id = 33;
        let mut zone_id = 999;
        let mut position = saved_position;
        assert!(session.character_retry_login_at_homebind_for_test(
            &mut map_id,
            &mut zone_id,
            &mut position,
            CharacterLoginLocationLikeCpp {
                map_id: 1,
                bind_area_id: Some(12),
                position: homebind_position,
            },
        ));

        assert_eq!(map_id, 1);
        assert_eq!(zone_id, 12);
        assert_eq!(
            session.character_player_zone_area_for_test(),
            Some((12, 12))
        );
        assert_eq!(position, homebind_position);
        assert_eq!(
            session.character_current_canonical_player_map_key_for_test(),
            Some(wow_map::MapKey::new(1, 0))
        );
    });
}
#[test]
fn garrison_login_uses_create_map_world_branch_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let guid = ObjectGuid::create_player(1, 47);
        let canonical: wow_world::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 1_151,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 2,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: wow_data::map::MAP_FLAG_GARRISON,
                flags2: 0,
            },
        ])));
        assert!(ensure_login_player_controller_for_test(
            &mut session,
            guid,
            "GarrisonLogin".to_string(),
            Position::ZERO,
            1_151,
            1,
            1,
            10,
            0,
        ));

        assert!(matches!(
            session.character_ensure_canonical_world_map_for_current_player_for_test(),
            Some(wow_map::CreateMapDecision::Create {
                key,
                kind: wow_map::ManagedMapKind::World,
                ..
            }) if key == wow_map::MapKey::new(1_151, 0)
        ));
        assert_eq!(
            session.character_current_canonical_player_map_key_for_test(),
            Some(wow_map::MapKey::new(1_151, 0))
        );
    });
}
#[test]
fn garrison_login_rejects_unsupported_expansion_and_retries_homebind_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let guid = ObjectGuid::create_player(1, 48);
        let canonical: wow_world::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 1,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
            wow_data::MapEntry {
                id: 1_151,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 3,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: wow_data::map::MAP_FLAG_GARRISON,
                flags2: 0,
            },
        ])));
        assert!(ensure_login_player_controller_for_test(
            &mut session,
            guid,
            "UnsupportedGarrisonLogin".to_string(),
            Position::ZERO,
            1_151,
            1,
            1,
            10,
            0,
        ));

        assert!(matches!(
            session.character_ensure_canonical_world_map_for_current_player_for_test(),
            Some(wow_map::CreateMapDecision::Reject { .. })
        ));
        assert!(
            session
                .character_current_canonical_player_map_key_for_test()
                .is_none()
        );
        assert!(canonical.lock().unwrap().find_map(1_151, 0).is_none());

        let mut map_id = 1_151;
        let mut zone_id = 999;
        let mut position = Position::ZERO;
        let homebind_position = Position::new(10.0, 20.0, 30.0, 1.0);
        assert!(session.character_retry_login_at_homebind_for_test(
            &mut map_id,
            &mut zone_id,
            &mut position,
            CharacterLoginLocationLikeCpp {
                map_id: 1,
                bind_area_id: Some(12),
                position: homebind_position,
            },
        ));
        assert_eq!(map_id, 1);
        assert_eq!(zone_id, 12);
        assert_eq!(
            session.character_player_zone_area_for_test(),
            Some((12, 12))
        );
        assert_eq!(position, homebind_position);
        assert_eq!(
            session.character_current_canonical_player_map_key_for_test(),
            Some(wow_map::MapKey::new(1, 0))
        );
    });
}
#[test]
fn unavailable_login_grid_cleans_partial_player_and_kicks_without_failure_packet_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        let guid = ObjectGuid::create_player(1, 42);
        let canonical: wow_world::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        let registry = Arc::new(wow_world::session::directory::PlayerRegistry::default());
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 33,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        session.set_player_registry(Arc::clone(&registry));
        assert!(ensure_login_player_controller_for_test(
            &mut session,
            guid,
            "GridFailure".to_string(),
            Position::ZERO,
            33,
            1,
            1,
            10,
            0,
        ));
        let _ = session.character_ensure_canonical_world_map_for_current_player_for_test();
        session.character_register_in_player_registry_for_test();
        assert!(
            session
                .character_current_canonical_player_map_key_for_test()
                .is_some()
        );
        assert!(registry.runtime_recipient(guid).is_some());

        assert!(!session.character_continue_login_after_grid_load_for_test(
            guid,
            33,
            0,
            Some(wow_world::session::PlayerGridLoadOutcomeLikeCpp {
                map_unavailable: true,
                ..Default::default()
            }),
        ));

        assert_eq!(
            session.state(),
            wow_world::session::SessionState::Disconnecting
        );
        assert!(session.player_guid().is_none());
        assert!(
            canonical
                .lock()
                .unwrap()
                .find_map(33, 0)
                .unwrap()
                .map()
                .get_typed_player(guid)
                .is_none()
        );
        assert!(registry.runtime_recipient(guid).is_none());
        assert!(send_rx.try_recv().is_err());
    });
}

#[test]
fn late_login_sequence_failure_releases_claim_and_partial_player_like_cpp() {
    let guid = ObjectGuid::create_player(1, 9_001_701);
    let (mut failed, _failed_rx) = make_session_with_send_capacity(1);
    assert!(failed.character_try_claim_character_login_for_test(guid));
    assert!(ensure_login_player_controller_for_test(
        &mut failed,
        guid,
        "LateFenceFailure".to_string(),
        Position::ZERO,
        1,
        1,
        1,
        10,
        0,
    ));

    install_canonical_player_owner_for_test(&mut failed, 1, 0);
    failed.character_abort_partial_login_sequence_for_test();

    assert_eq!(
        failed.state(),
        wow_world::session::SessionState::Disconnecting
    );
    assert!(failed.player_guid().is_none());
    let (mut retry, _retry_rx) = make_session_with_send_capacity(1);
    assert!(
        retry.character_try_claim_character_login_for_test(guid),
        "the failed login must not retain the only process-wide character claim"
    );
    retry.character_release_character_login_claim_for_test();
}

#[test]
fn login_without_grid_resolver_fails_closed_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let guid = ObjectGuid::create_player(1, 43);
    assert!(ensure_login_player_controller_for_test(
        &mut session,
        guid,
        "MissingResolver".to_string(),
        Position::ZERO,
        1,
        1,
        1,
        10,
        0,
    ));

    install_canonical_player_owner_for_test(&mut session, 1, 0);
    assert!(!session.character_continue_login_after_grid_load_for_test(guid, 1, 0, None));

    assert_eq!(
        session.state(),
        wow_world::session::SessionState::Disconnecting
    );
    assert!(session.player_guid().is_none());
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn homebind_retry_refreshes_zone_when_saved_coordinates_already_match_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let guid = ObjectGuid::create_player(1, 46);
        let homebind_position = Position::new(10.0, 20.0, 30.0, 1.0);
        let canonical: wow_world::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 1,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        assert!(ensure_login_player_controller_for_test(
            &mut session,
            guid,
            "MatchingHomebind".to_string(),
            homebind_position,
            1,
            1,
            1,
            10,
            0,
        ));

        let mut map_id = 1;
        let mut zone_id = 999;
        let mut position = homebind_position;
        assert!(session.character_retry_login_at_homebind_for_test(
            &mut map_id,
            &mut zone_id,
            &mut position,
            CharacterLoginLocationLikeCpp {
                map_id: 1,
                bind_area_id: Some(12),
                position: homebind_position,
            },
        ));

        assert_eq!(map_id, 1);
        assert_eq!(zone_id, 12);
        assert_eq!(position, homebind_position);
        assert_eq!(
            session.character_player_zone_area_for_test(),
            Some((12, 12))
        );
        assert_eq!(
            session.character_current_canonical_player_map_key_for_test(),
            Some(wow_map::MapKey::new(1, 0))
        );
    });
}
