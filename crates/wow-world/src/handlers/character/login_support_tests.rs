use super::*;
use wow_data::PlayerCreatePositionLikeCpp;

#[test]
fn invalid_homebind_repair_selects_cpp_create_mode_and_graveyard_order() {
    let normal = PlayerCreatePositionLikeCpp {
        map_id: 0,
        position: Position::new(-8_946.0, -246.0, 59.0, 0.0),
        transport_guid: None,
    };
    let npe = PlayerCreatePositionLikeCpp {
        map_id: 2_175,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        transport_guid: None,
    };
    let info = PlayerCreateInfoLikeCpp {
        create_position: normal,
        create_position_npe: Some(npe),
    };
    assert_eq!(
        first_login_creation_homebind_like_cpp(info, wow_data::PLAYER_CREATE_MODE_NORMAL_LIKE_CPP,),
        Some(CharacterLoginLocationLikeCpp {
            map_id: normal.map_id,
            bind_area_id: None,
            position: normal.position,
        })
    );
    assert_eq!(
        first_login_creation_homebind_like_cpp(info, wow_data::PLAYER_CREATE_MODE_NPE_LIKE_CPP,)
            .map(|homebind| homebind.position),
        Some(npe.position)
    );
    assert_eq!(
        first_login_creation_homebind_like_cpp(
            PlayerCreateInfoLikeCpp {
                create_position_npe: None,
                ..info
            },
            wow_data::PLAYER_CREATE_MODE_NPE_LIKE_CPP,
        )
        .map(|homebind| homebind.position),
        Some(normal.position),
        "C++ falls back to the normal class/race creation position when NPE data is invalid"
    );
    assert_eq!(
        first_login_creation_homebind_like_cpp(
            PlayerCreateInfoLikeCpp {
                create_position_npe: Some(PlayerCreatePositionLikeCpp {
                    transport_guid: Some(29),
                    ..npe
                }),
                ..info
            },
            wow_data::PLAYER_CREATE_MODE_NPE_LIKE_CPP,
        ),
        None,
        "C++ does not bind first-login transport offsets and falls through to graveyard"
    );

    assert_eq!(
        default_graveyard_safe_loc_ids_for_race_like_cpp(1),
        [Some(4), None]
    );
    assert_eq!(
        default_graveyard_safe_loc_ids_for_race_like_cpp(2),
        [Some(10), None]
    );
    assert_eq!(
        default_graveyard_safe_loc_ids_for_race_like_cpp(24),
        [Some(4), Some(3295)]
    );

    let area_store = wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 12,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: 0,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 13,
            continent_id: 0,
            parent_area_id: 12,
            area_bit: 0,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0x4000_0000,
        },
    ]);
    assert_eq!(
        zone_and_area_from_area_id_like_cpp(13, Some(&area_store)),
        (12, 13)
    );

    let scenario_garrison_store = wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 1_151,
        instance_type: wow_data::map::MAP_SCENARIO,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: wow_data::map::MAP_FLAG_GARRISON,
        flags2: 0,
    }]);
    assert!(!usable_character_homebind_like_cpp(
        CharacterLoginLocationLikeCpp {
            map_id: 1_151,
            bind_area_id: Some(12),
            position: Position::ZERO,
        },
        Some(&scenario_garrison_store),
        2,
    ));
}

#[test]
fn battleground_login_fallback_prefers_valid_entry_point_then_homebind_like_cpp() {
    let map_store = wow_data::MapStore::from_entries([
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
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ]);
    let entry_point = CharacterLoginLocationLikeCpp {
        map_id: 1,
        bind_area_id: None,
        position: Position::new(10.0, 20.0, 30.0, 1.0),
    };
    let homebind = CharacterLoginLocationLikeCpp {
        map_id: 0,
        bind_area_id: Some(12),
        position: Position::new(-1.0, -2.0, 3.0, 0.0),
    };
    let bg_data = CharacterBattlegroundLoginDataLikeCpp { entry_point };

    assert!(usable_character_homebind_like_cpp(
        homebind,
        Some(&map_store),
        2,
    ));
    assert!(!usable_character_homebind_like_cpp(
        entry_point,
        Some(&map_store),
        2,
    ));

    assert_eq!(
        login_location_zone_area_like_cpp(entry_point, |map_id, position| {
            assert_eq!(map_id, 1);
            assert_eq!(position, entry_point.position);
            Ok((34, 56))
        })
        .unwrap(),
        (34, 56)
    );
    assert_eq!(
        login_location_zone_area_like_cpp(homebind, |map_id, position| {
            assert_eq!(map_id, 0);
            assert_eq!(position, homebind.position);
            Ok((78, 90))
        })
        .unwrap(),
        (78, 90)
    );
    let bind_update = login_bind_point_update_like_cpp(homebind);
    assert_eq!(bind_update.x, homebind.position.x);
    assert_eq!(bind_update.y, homebind.position.y);
    assert_eq!(bind_update.z, homebind.position.z);
    assert_eq!(bind_update.map_id, homebind.map_id);
    assert_eq!(bind_update.area_id, 12);

    assert_eq!(
        battleground_login_fallback_location_like_cpp(
            Some(bg_data),
            Some(homebind),
            Some(&map_store),
        ),
        Some(entry_point)
    );
    assert_eq!(
        battleground_login_fallback_location_like_cpp(
            Some(CharacterBattlegroundLoginDataLikeCpp {
                entry_point: CharacterLoginLocationLikeCpp {
                    map_id: u32::from(u16::MAX),
                    bind_area_id: None,
                    position: Position::ZERO,
                },
                ..bg_data
            }),
            Some(homebind),
            Some(&map_store),
        ),
        Some(homebind)
    );
    assert_eq!(
        battleground_login_fallback_location_like_cpp(
            None,
            Some(CharacterLoginLocationLikeCpp {
                position: Position::new(f32::NAN, 0.0, 0.0, 0.0),
                ..homebind
            }),
            Some(&map_store),
        ),
        None
    );
}
