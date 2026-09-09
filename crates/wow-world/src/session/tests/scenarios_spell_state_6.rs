//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn db_implicit_destination_or_db_invalid_row_zero_radius_keeps_caster_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 715_i32;
    let template_entry = 9021_u32;
    let player_guid = ObjectGuid::create_player(1, 7021);
    let player_position = Position::new(240.0, 340.0, 48.0, 0.75);
    let other_map_destination = Position::new(246.0, 344.0, 49.0, 2.875);
    let canonical = shared_canonical_map_manager();
    let mut summon_effect =
        summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap());
    summon_effect.implicit_target_1 =
        wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB;
    let spell_info =
        gameobject_summon_spell_info_like_cpp(spell_id, 0, vec![summon_effect.clone()]);

    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        spell_info.clone(),
    );
    let mut misc = summon_go_spell_misc_entry_like_cpp(spell_id as u32, 0);
    misc.range_index = 88;
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([misc])));
    session.set_spell_range_store(Arc::new(wow_data::SpellRangeStore::from_entries([
        wow_data::SpellRangeEntry {
            id: 88,
            display_name: String::new(),
            display_name_short: String::new(),
            flags: 0,
            range_min: [0.0, 0.0],
            range_max: [50.0, 50.0],
        },
    ])));
    let mut target_spell_store = wow_data::SpellStore::new();
    target_spell_store.insert(spell_id, spell_info);
    session.set_spell_target_position_store(Arc::new(
        wow_data::SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
            [wow_data::SpellTargetPositionRowLikeCpp {
                spell_id: spell_id as u32,
                effect_index: summon_effect.effect_index,
                target_map_id: 1,
                x: other_map_destination.x,
                y: other_map_destination.y,
                z: other_map_destination.z,
                orientation: Some(other_map_destination.orientation),
            }],
            &target_spell_store,
            |map_id| matches!(map_id, 1 | 571),
        ),
    ));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 715,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("OR_DB invalid DB row with zero radius should execute");

    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    let summoned_guid = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .filter(ObjectGuid::is_game_object)
        .find(|guid| {
            managed
                .map()
                .get_typed_game_object(*guid)
                .is_some_and(|go| go.world().object().entry() == template_entry)
        })
        .expect("OR_DB invalid-row fallback summon should be visible");
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    assert_eq!(
        summoned.world().position(),
        player_position,
        "C++ TARGET_DEST_NEARBY_ENTRY_OR_DB keeps SpellDestination(*caster) when the DB row is unusable and CalcRadius is zero"
    );
    assert_ne!(summoned.world().position(), other_map_destination);
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "OR_DB invalid-row fallback summon should trigger represented visibility create/update delivery"
    );
}
#[tokio::test]
async fn or_db_invalid_row_positive_radius_offsets_from_caster_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 718_i32;
    let template_entry = 9024_u32;
    let player_guid = ObjectGuid::create_player(1, 7027);
    let player_position = Position::new(270.0, 370.0, 54.0, 0.75);
    let other_map_destination = Position::new(276.0, 374.0, 55.0, 2.875);
    let radius = 12.0_f32;
    let canonical = shared_canonical_map_manager();
    let mut summon_effect =
        summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap());
    summon_effect.implicit_target_1 =
        wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB;
    summon_effect.effect_radius_index_1 = 31;
    let spell_info =
        gameobject_summon_spell_info_like_cpp(spell_id, 0, vec![summon_effect.clone()]);

    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        spell_info.clone(),
    );
    session.seed_represented_runtime_rng_like_cpp(0xD157);
    let mut misc = summon_go_spell_misc_entry_like_cpp(spell_id as u32, 0);
    misc.range_index = 88;
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([misc])));
    session.set_spell_range_store(Arc::new(wow_data::SpellRangeStore::from_entries([
        wow_data::SpellRangeEntry {
            id: 88,
            display_name: String::new(),
            display_name_short: String::new(),
            flags: 0,
            range_min: [0.0, 0.0],
            range_max: [50.0, 50.0],
        },
    ])));
    session.set_spell_radius_store(Arc::new(wow_data::SpellRadiusStore::from_entries([
        wow_data::SpellRadiusEntry {
            id: 31,
            radius: 0.0,
            radius_per_level: 0.0,
            radius_min: 0.0,
            radius_max: radius,
        },
    ])));
    let mut target_spell_store = wow_data::SpellStore::new();
    target_spell_store.insert(spell_id, spell_info);
    session.set_spell_target_position_store(Arc::new(
        wow_data::SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
            [wow_data::SpellTargetPositionRowLikeCpp {
                spell_id: spell_id as u32,
                effect_index: summon_effect.effect_index,
                target_map_id: 1,
                x: other_map_destination.x,
                y: other_map_destination.y,
                z: other_map_destination.z,
                orientation: Some(other_map_destination.orientation),
            }],
            &target_spell_store,
            |map_id| matches!(map_id, 1 | 571),
        ),
    ));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 718,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("OR_DB invalid DB row with positive radius should execute");

    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    let summoned_guid = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .filter(ObjectGuid::is_game_object)
        .find(|guid| {
            managed
                .map()
                .get_typed_game_object(*guid)
                .is_some_and(|go| go.world().object().entry() == template_entry)
        })
        .expect("OR_DB invalid-row positive-radius fallback summon should be visible");
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    let summoned_position = summoned.world().position();
    let distance_2d = summoned_position.distance_2d(&player_position);
    assert!(
        distance_2d <= radius + 0.001,
        "C++ MovePositionToFirstCollision keeps the caster fallback destination within CalcRadius"
    );
    assert!(
        distance_2d > 0.001,
        "seeded represented frand should move the positive-radius fallback away from the caster"
    );
    let expected_angle = player_position.orientation - std::f32::consts::FRAC_PI_4;
    assert!(
        ((summoned_position.x - player_position.x) - distance_2d * expected_angle.cos()).abs()
            < 0.001
    );
    assert!(
        ((summoned_position.y - player_position.y) - distance_2d * expected_angle.sin()).abs()
            < 0.001
    );
    assert_eq!(
        summoned_position.orientation, player_position.orientation,
        "C++ DB-row emergency branch preserves SpellDestination(*caster) orientation"
    );
    assert_ne!(summoned_position, other_map_destination);
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "OR_DB invalid-row positive-radius fallback summon should trigger represented visibility create/update delivery"
    );
}
#[tokio::test]
async fn db_implicit_destination_or_db_missing_row_zero_radius_keeps_caster_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 716_i32;
    let template_entry = 9022_u32;
    let player_guid = ObjectGuid::create_player(1, 7022);
    let player_position = Position::new(250.0, 350.0, 50.0, 0.875);
    let effect_position_facing = 1.625;
    let canonical = shared_canonical_map_manager();
    let mut summon_effect =
        summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap());
    summon_effect.implicit_target_1 =
        wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB;
    summon_effect.position_facing = effect_position_facing;
    let spell_info =
        gameobject_summon_spell_info_like_cpp(spell_id, 0, vec![summon_effect.clone()]);

    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        spell_info.clone(),
    );
    let mut misc = summon_go_spell_misc_entry_like_cpp(spell_id as u32, 0);
    misc.range_index = 88;
    misc.attributes[4] = wow_data::spell::attributes::SPELL_ATTR4_USE_FACING_FROM_SPELL as i32;
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([misc])));
    session.set_spell_range_store(Arc::new(wow_data::SpellRangeStore::from_entries([
        wow_data::SpellRangeEntry {
            id: 88,
            display_name: String::new(),
            display_name_short: String::new(),
            flags: 0,
            range_min: [0.0, 0.0],
            range_max: [50.0, 50.0],
        },
    ])));
    let mut target_spell_store = wow_data::SpellStore::new();
    target_spell_store.insert(spell_id, spell_info);
    session.set_spell_target_position_store(Arc::new(
        wow_data::SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
            Vec::<wow_data::SpellTargetPositionRowLikeCpp>::new(),
            &target_spell_store,
            |map_id| map_id == 571,
        ),
    ));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 716,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("OR_DB missing DB row with zero radius should execute");

    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    let summoned_guid = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .filter(ObjectGuid::is_game_object)
        .find(|guid| {
            managed
                .map()
                .get_typed_game_object(*guid)
                .is_some_and(|go| go.world().object().entry() == template_entry)
        })
        .expect("OR_DB missing-row fallback summon should be visible");
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    assert_eq!(
        summoned.world().position(),
        Position::new(
            player_position.x,
            player_position.y,
            player_position.z,
            effect_position_facing,
        ),
        "C++ TARGET_DEST_NEARBY_ENTRY_OR_DB without a DB row falls back to caster when SearchNearbyTarget finds no represented target and CalcRadius is zero, then applies SPELL_ATTR4_USE_FACING_FROM_SPELL"
    );
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "OR_DB missing-row fallback summon should trigger represented visibility create/update delivery"
    );
}
#[tokio::test]
async fn db_implicit_caster_destination_uses_spell_target_position_same_map_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 712_i32;
    let template_entry = 9017_u32;
    let player_guid = ObjectGuid::create_player(1, 7017);
    let player_position = Position::new(210.0, 310.0, 42.0, 0.0);
    let db_destination = Position::new(216.0, 314.0, 43.0, 1.875);
    let effect_position_facing = 0.625;
    let canonical = shared_canonical_map_manager();
    let mut summon_effect =
        summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap());
    summon_effect.implicit_target_1 = wow_data::spell::implicit_targets::TARGET_DEST_DB;
    summon_effect.position_facing = effect_position_facing;
    let spell_info =
        gameobject_summon_spell_info_like_cpp(spell_id, 0, vec![summon_effect.clone()]);

    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        spell_info.clone(),
    );
    let mut misc = summon_go_spell_misc_entry_like_cpp(spell_id as u32, 0);
    misc.attributes[4] = wow_data::spell::attributes::SPELL_ATTR4_USE_FACING_FROM_SPELL as i32;
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([misc])));
    let mut target_spell_store = wow_data::SpellStore::new();
    target_spell_store.insert(spell_id, spell_info);
    session.set_spell_target_position_store(Arc::new(
        wow_data::SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
            [wow_data::SpellTargetPositionRowLikeCpp {
                spell_id: spell_id as u32,
                effect_index: summon_effect.effect_index,
                target_map_id: 571,
                x: db_destination.x,
                y: db_destination.y,
                z: db_destination.z,
                orientation: Some(db_destination.orientation),
            }],
            &target_spell_store,
            |map_id| map_id == 571,
        ),
    ));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 712,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("TARGET_DEST_DB implicit caster destination should execute");

    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    let summoned_guid = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .filter(ObjectGuid::is_game_object)
        .find(|guid| {
            managed
                .map()
                .get_typed_game_object(*guid)
                .is_some_and(|go| go.world().object().entry() == template_entry)
        })
        .expect("DB caster-destination summon should be visible");
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    assert_eq!(
        summoned.world().position(),
        Position::new(
            db_destination.x,
            db_destination.y,
            db_destination.z,
            effect_position_facing,
        ),
        "C++ TARGET_DEST_DB caster destination uses spell_target_position without a range check, then applies SPELL_ATTR4_USE_FACING_FROM_SPELL"
    );
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "DB caster-destination summon should trigger represented visibility create/update delivery"
    );
}
#[tokio::test]
async fn db_implicit_caster_destination_ignores_other_map_for_non_teleport_bind_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 713_i32;
    let template_entry = 9018_u32;
    let player_guid = ObjectGuid::create_player(1, 7018);
    let player_position = Position::new(220.0, 320.0, 44.0, 0.25);
    let other_map_destination = Position::new(226.0, 324.0, 45.0, 2.125);
    let canonical = shared_canonical_map_manager();
    let mut summon_effect =
        summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap());
    summon_effect.implicit_target_1 = wow_data::spell::implicit_targets::TARGET_DEST_DB;
    let spell_info =
        gameobject_summon_spell_info_like_cpp(spell_id, 0, vec![summon_effect.clone()]);

    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        spell_info.clone(),
    );
    let mut target_spell_store = wow_data::SpellStore::new();
    target_spell_store.insert(spell_id, spell_info);
    session.set_spell_target_position_store(Arc::new(
        wow_data::SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
            [wow_data::SpellTargetPositionRowLikeCpp {
                spell_id: spell_id as u32,
                effect_index: summon_effect.effect_index,
                target_map_id: 1,
                x: other_map_destination.x,
                y: other_map_destination.y,
                z: other_map_destination.z,
                orientation: Some(other_map_destination.orientation),
            }],
            &target_spell_store,
            |map_id| matches!(map_id, 1 | 571),
        ),
    ));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 713,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("other-map TARGET_DEST_DB non-teleport summon should execute via fallback");

    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    let summoned_guid = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .filter(ObjectGuid::is_game_object)
        .find(|guid| {
            managed
                .map()
                .get_typed_game_object(*guid)
                .is_some_and(|go| go.world().object().entry() == template_entry)
        })
        .expect("fallback summon should be visible on caster map");
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    assert_eq!(
        summoned.world().position(),
        player_position,
        "C++ TARGET_DEST_DB keeps the default caster destination when spell_target_position is on another map and the spell is not TELEPORT_UNITS/BIND"
    );
    assert_ne!(summoned.world().position(), other_map_destination);
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "fallback summon should trigger represented visibility create/update delivery"
    );
}
#[tokio::test]
async fn implicit_destination_selection_uses_cpp_effect_order_before_spell_go() {
    let (mut session, _, send_rx) = make_session();
    session.set_map_store(crate::teleport_test_fixtures::world_maps([571, 1]));
    let spell_id = 86_901_i32;
    let player_guid = ObjectGuid::create_player(1, 7042);
    let player_position = Position::new(310.0, 410.0, 62.0, 0.5);
    let home_position = Position::new(40.0, 50.0, 60.0, 1.25);
    let db_destination = Position::new(70.0, 80.0, 90.0, 2.5);
    let canonical = shared_canonical_map_manager();
    let home_effect = wow_data::SpellEffectInfo {
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND,
        implicit_target_1: wow_data::spell::implicit_targets::TARGET_DEST_HOME,
        ..Default::default()
    };
    let db_effect = wow_data::SpellEffectInfo {
        effect_index: 1,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS,
        implicit_target_1: wow_data::spell::implicit_targets::TARGET_DEST_DB,
        ..Default::default()
    };
    let empty_home_effect = wow_data::SpellEffectInfo {
        effect_index: 2,
        effect: 0,
        implicit_target_1: wow_data::spell::implicit_targets::TARGET_DEST_HOME,
        ..Default::default()
    };
    let spell_info = teleport_units_spell_info_like_cpp(
        spell_id,
        vec![home_effect, db_effect.clone(), empty_home_effect],
    );

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "OrderedDestination".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    let _ = session.set_represented_homebind_like_cpp(RepresentedHomebindLikeCpp {
        map_id: 1,
        area_id: 1519,
        position: home_position,
    });
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, spell_info.clone());
    session.set_spell_store(Arc::new(spell_store));
    let mut target_spell_store = wow_data::SpellStore::new();
    target_spell_store.insert(spell_id, spell_info);
    session.set_spell_target_position_store(Arc::new(
        wow_data::SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
            [wow_data::SpellTargetPositionRowLikeCpp {
                spell_id: spell_id as u32,
                effect_index: db_effect.effect_index,
                target_map_id: 1,
                x: db_destination.x,
                y: db_destination.y,
                z: db_destination.z,
                orientation: Some(db_destination.orientation),
            }],
            &target_spell_store,
            |map_id| matches!(map_id, 1 | 571),
        ),
    ));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData::default(),
        )
        .await
        .expect("ordered HOME then DB teleport should execute");

    let bytes = send_rx.try_recv().expect("ordered destination SpellGo");
    let packet_target = decode_spell_go_target_data_like_cpp(&bytes, spell_id);
    assert_ne!(packet_target.flags & 0x0000_0040, 0);
    assert_eq!(
        packet_target
            .dst_location
            .expect("resolved final DB destination")
            .position,
        Position::new(db_destination.x, db_destination.y, db_destination.z, 0.0),
        "later TARGET_DEST_DB replaces earlier TARGET_DEST_HOME like C++ ModDst"
    );
    assert_eq!(packet_target.map_id, None);
    assert_eq!(
        session
            .represented_homebind_like_cpp()
            .expect("bind effect home snapshot")
            .position,
        home_position,
        "effect 0 BIND keeps its HOME snapshot while later TELEPORT uses DB"
    );
    assert_eq!(
        session.pending_teleport_like_cpp(),
        Some((1, db_destination))
    );
}
#[tokio::test]
async fn environmental_damage_spell_damages_player_and_logs_fire_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 721_i32;
    let player_guid = ObjectGuid::create_player(1, 7027);
    let player_position = Position::new(300.0, 400.0, 60.0, 1.0);
    let canonical = shared_canonical_map_manager();
    let environmental_effect = wow_data::SpellEffectInfo {
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE,
        effect_base_points: 73,
        ..Default::default()
    };
    let spell_info = environmental_damage_spell_info_like_cpp(spell_id, vec![environmental_effect]);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "EnvironmentalTarget".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(1_000, 1_000);
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, spell_info);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 721,
                script_visual_id: 0,
            },
            SpellTargetData {
                flags: 0x2,
                unit: player_guid,
                ..SpellTargetData::default()
            },
        )
        .await
        .expect("represented environmental damage spell should execute");

    assert_eq!(session.player_health_like_cpp(), 927);
    let packets = drain_server_packet_bytes(&send_rx);
    let opcodes: Vec<_> = packets
        .iter()
        .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(bytes).server_opcode())
        .collect();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::HealthUpdate,
            ServerOpcodes::EnvironmentalDamageLog,
            ServerOpcodes::CooldownEvent,
        ]
    );

    let mut damage_log = wow_packet::WorldPacket::from_bytes(&packets[2]);
    assert_eq!(
        damage_log.read_uint16().expect("opcode"),
        ServerOpcodes::EnvironmentalDamageLog as u16
    );
    assert_eq!(damage_log.read_packed_guid().expect("victim"), player_guid);
    assert_eq!(
        damage_log.read_uint8().expect("environmental type"),
        DAMAGE_FIRE_LIKE_CPP
    );
    assert_eq!(damage_log.read_int32().expect("amount"), 73);
    assert_eq!(damage_log.read_int32().expect("resisted"), 0);
    assert_eq!(damage_log.read_int32().expect("absorbed"), 0);
    assert!(!damage_log.has_bit().expect("has log data"));
}
#[tokio::test]
async fn primary_environmental_damage_spell_damages_player_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 729_i32;
    let player_guid = ObjectGuid::create_player(1, 7028);
    let player_position = Position::new(301.0, 401.0, 61.0, 1.0);
    let canonical = shared_canonical_map_manager();

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "PrimaryEnvironmentalTarget".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(1_000, 1_000);
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE,
            effect_base_points: 81,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 729,
                script_visual_id: 0,
            },
            SpellTargetData {
                flags: 0x2,
                unit: player_guid,
                ..SpellTargetData::default()
            },
        )
        .await
        .expect("represented primary environmental damage spell should execute");

    assert_eq!(session.player_health_like_cpp(), 919);
    let packets = drain_server_packet_bytes(&send_rx);
    let opcodes: Vec<_> = packets
        .iter()
        .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(bytes).server_opcode())
        .collect();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::HealthUpdate,
            ServerOpcodes::EnvironmentalDamageLog,
            ServerOpcodes::CooldownEvent,
        ],
        "C++ dispatches primary SPELL_EFFECT_ENVIRONMENTAL_DAMAGE through EffectEnvironmentalDMG"
    );

    let mut damage_log = wow_packet::WorldPacket::from_bytes(&packets[2]);
    assert_eq!(
        damage_log.read_uint16().expect("opcode"),
        ServerOpcodes::EnvironmentalDamageLog as u16
    );
    assert_eq!(damage_log.read_packed_guid().expect("victim"), player_guid);
    assert_eq!(
        damage_log.read_uint8().expect("environmental type"),
        DAMAGE_FIRE_LIKE_CPP
    );
    assert_eq!(damage_log.read_int32().expect("amount"), 81);
    assert_eq!(damage_log.read_int32().expect("resisted"), 0);
    assert_eq!(damage_log.read_int32().expect("absorbed"), 0);
    assert!(!damage_log.has_bit().expect("has log data"));
}
#[tokio::test]
async fn db_implicit_caster_destination_missing_row_uses_object_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 714_i32;
    let template_entry = 9019_u32;
    let player_guid = ObjectGuid::create_player(1, 7019);
    let target_guid = test_gameobject_guid(9020, 7020);
    let player_position = Position::new(230.0, 330.0, 46.0, 0.0);
    let target_position = Position::new(236.0, 334.0, 47.0, 2.625);
    let canonical = shared_canonical_map_manager();
    let mut summon_effect =
        summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap());
    summon_effect.implicit_target_1 = wow_data::spell::implicit_targets::TARGET_DEST_DB;
    let spell_info =
        gameobject_summon_spell_info_like_cpp(spell_id, 0, vec![summon_effect.clone()]);

    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        spell_info.clone(),
    );
    add_canonical_test_gameobject_on_map(&canonical, target_guid, 9_020, target_position, 571, 0);
    let mut target_spell_store = wow_data::SpellStore::new();
    target_spell_store.insert(spell_id, spell_info);
    session.set_spell_target_position_store(Arc::new(
        wow_data::SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
            Vec::<wow_data::SpellTargetPositionRowLikeCpp>::new(),
            &target_spell_store,
            |map_id| map_id == 571,
        ),
    ));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            target_guid,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 714,
                script_visual_id: 0,
            },
            SpellTargetData {
                unit: target_guid,
                ..Default::default()
            },
        )
        .await
        .expect("missing-row TARGET_DEST_DB should use explicit object target fallback");

    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    let summoned_guid = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .filter(ObjectGuid::is_game_object)
        .find(|guid| {
            managed
                .map()
                .get_typed_game_object(*guid)
                .is_some_and(|go| go.world().object().entry() == template_entry)
        })
        .expect("object-target fallback summon should be visible");
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    assert_eq!(
        summoned.world().position(),
        target_position,
        "C++ TARGET_DEST_DB falls back to m_targets.GetObjectTarget() when spell_target_position has no row"
    );
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "object-target fallback summon should trigger represented visibility create/update delivery"
    );
}
