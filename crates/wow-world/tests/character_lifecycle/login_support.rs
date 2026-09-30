// External application scenarios migrated with their original assertions.

use super::fixtures::make_session;
use super::*;
use std::sync::Arc;
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
fn default_homebind_reads_primary_then_neutral_pandaren_from_startup_store_like_cpp() {
    fn map(id: u32) -> wow_data::MapEntry {
        wow_data::MapEntry {
            id,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        }
    }
    let maps = wow_data::MapStore::from_entries([map(1), map(870)]);
    let primary = wow_data::WorldSafeLocRow {
        id: 4,
        map_id: 1,
        x: 1.0,
        y: 2.0,
        z: 3.0,
        facing_degrees: 90.0,
    };
    let fallback = wow_data::WorldSafeLocRow {
        id: 3295,
        map_id: 870,
        x: 4.0,
        y: 5.0,
        z: 6.0,
        facing_degrees: 180.0,
    };

    let (mut session, _send_rx) = make_session();
    let (store, report) =
        wow_data::WorldSafeLocStore::from_rows_like_cpp([fallback, primary], &maps);
    assert_eq!(report.loaded, 2);
    session.set_world_safe_loc_store_like_cpp(Arc::new(store));
    assert_eq!(
        session
            .character_load_default_graveyard_homebind_for_test(24)
            .expect("neutral Pandaren uses faction primary first"),
        CharacterLoginLocationLikeCpp {
            map_id: 1,
            bind_area_id: None,
            position: Position::new(1.0, 2.0, 3.0, 90_f32.to_radians()),
        }
    );

    let (fallback_only, report) =
        wow_data::WorldSafeLocStore::from_rows_like_cpp([fallback], &maps);
    assert_eq!(report.loaded, 1);
    session.set_world_safe_loc_store_like_cpp(Arc::new(fallback_only));
    assert_eq!(
        session
            .character_load_default_graveyard_homebind_for_test(24)
            .expect("neutral Pandaren keeps C++ 3295 fallback")
            .map_id,
        870
    );
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

#[test]
fn pvp_season_world_states_match_cpp_world_state_mgr() {
    // In-progress arena season 32 -> current(3191)=32, previous(3901)=31. Matches the
    // captured C++ INIT_WORLD_STATES (World.cpp:1363-1364). Existing ids stay untouched
    // and the 3191/3901 values are overridden in place (not duplicated).
    let mut states = vec![(3191, 0), (3901, 0), (1000, 5)];
    apply_pvp_season_world_states_like_cpp(&mut states, 32, true);
    assert_eq!(states, vec![(3191, 32), (3901, 31), (1000, 5)]);

    // Default (season not in progress): current=0, previous=season_id.
    let mut states = vec![(3191, 0), (3901, 0)];
    apply_pvp_season_world_states_like_cpp(&mut states, 32, false);
    assert_eq!(states, vec![(3191, 0), (3901, 32)]);

    // Absent ids are appended rather than dropped.
    let mut states: Vec<(i32, i32)> = Vec::new();
    apply_pvp_season_world_states_like_cpp(&mut states, 10, true);
    assert_eq!(states, vec![(3191, 10), (3901, 9)]);
}

#[test]
fn init_world_states_builder_orders_realm_then_map_and_filters_area_like_cpp() {
    let area_store = wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 4395,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 4613,
            continent_id: 571,
            parent_area_id: 4395,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ]);
    let templates = [
        LoginWorldStateTemplateLikeCpp {
            id: 10,
            default_value: 1,
            map_ids: BTreeSet::new(),
            area_ids: BTreeSet::new(),
        },
        LoginWorldStateTemplateLikeCpp {
            id: 20,
            default_value: 2,
            map_ids: BTreeSet::from([571]),
            area_ids: BTreeSet::new(),
        },
        LoginWorldStateTemplateLikeCpp {
            id: 30,
            default_value: 3,
            map_ids: BTreeSet::from([571]),
            area_ids: BTreeSet::from([4395]),
        },
        LoginWorldStateTemplateLikeCpp {
            id: 40,
            default_value: 4,
            map_ids: BTreeSet::from([571]),
            area_ids: BTreeSet::from([9999]),
        },
        LoginWorldStateTemplateLikeCpp {
            id: 50,
            default_value: 5,
            map_ids: BTreeSet::from([WORLDSTATE_ANY_MAP_LIKE_CPP]),
            area_ids: BTreeSet::new(),
        },
    ];

    assert_eq!(
        build_initial_world_states_like_cpp(
            templates,
            [(20, 22), (999, 999)],
            571,
            4613,
            Some(&area_store),
        ),
        vec![(10, 1), (50, 5), (20, 22), (30, 3)]
    );
}
