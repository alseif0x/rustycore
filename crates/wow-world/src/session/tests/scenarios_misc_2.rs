//! Session scenarios exercising the represented misc responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn or_db_missing_row_positive_radius_offsets_and_applies_facing_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 719_i32;
    let template_entry = 9025_u32;
    let player_guid = ObjectGuid::create_player(1, 7028);
    let player_position = Position::new(280.0, 380.0, 56.0, 1.125);
    let effect_position_facing = 2.5;
    let radius = 9.0_f32;
    let canonical = shared_canonical_map_manager();
    let mut summon_effect =
        summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap());
    summon_effect.implicit_target_1 =
        wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB;
    summon_effect.effect_radius_index_1 = 32;
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
    session.seed_represented_runtime_rng_like_cpp(0xD159);
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
    session.set_spell_radius_store(Arc::new(wow_data::SpellRadiusStore::from_entries([
        wow_data::SpellRadiusEntry {
            id: 32,
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
                spell_visual_id: 719,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("OR_DB missing DB row with positive radius should execute");

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
        .expect("OR_DB missing-row positive-radius fallback summon should be visible");
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    let summoned_position = summoned.world().position();
    let distance_2d = summoned_position.distance_2d(&player_position);
    assert!(
        distance_2d <= radius + 0.001,
        "C++ caster fallback randomization stays within CalcRadius"
    );
    assert!(
        distance_2d > 0.001,
        "seeded represented frand should move the positive-radius missing-row fallback away from the caster"
    );
    assert_eq!(
        summoned_position.orientation, effect_position_facing,
        "C++ common destination branch applies SPELL_ATTR4_USE_FACING_FROM_SPELL after random-radius fallback"
    );
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "OR_DB missing-row positive-radius fallback summon should trigger represented visibility create/update delivery"
    );
}
#[tokio::test]
async fn db_implicit_destination_or_db_missing_row_uses_nearest_represented_entry_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 717_i32;
    let template_entry = 9023_u32;
    let player_guid = ObjectGuid::create_player(1, 7023);
    let player_position = Position::new(260.0, 360.0, 52.0, 0.0);
    let near_target_guid = test_gameobject_guid(template_entry, 7024);
    let far_target_guid = test_gameobject_guid(template_entry, 7025);
    let wrong_entry_guid = test_gameobject_guid(template_entry + 1, 7026);
    let near_target_position = Position::new(264.0, 360.0, 52.0, 1.125);
    let far_target_position = Position::new(278.0, 360.0, 52.0, 2.25);
    let wrong_entry_position = Position::new(262.0, 360.0, 52.0, 3.0);
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
            Vec::<wow_data::SpellTargetPositionRowLikeCpp>::new(),
            &target_spell_store,
            |map_id| map_id == 571,
        ),
    ));
    add_canonical_test_gameobject_on_map(
        &canonical,
        far_target_guid,
        template_entry,
        far_target_position,
        571,
        0,
    );
    add_canonical_test_gameobject_on_map(
        &canonical,
        wrong_entry_guid,
        template_entry + 1,
        wrong_entry_position,
        571,
        0,
    );
    add_canonical_test_gameobject_on_map(
        &canonical,
        near_target_guid,
        template_entry,
        near_target_position,
        571,
        0,
    );

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 717,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("OR_DB missing DB row with represented nearby entry should execute");

    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    let summoned_guid = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .filter(ObjectGuid::is_game_object)
        .filter(|guid| {
            *guid != near_target_guid && *guid != far_target_guid && *guid != wrong_entry_guid
        })
        .find(|guid| {
            managed
                .map()
                .get_typed_game_object(*guid)
                .is_some_and(|go| go.world().object().entry() == template_entry)
        })
        .expect("OR_DB nearby-entry destination summon should be visible");
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    assert_eq!(
        summoned.world().position(),
        near_target_position,
        "C++ SearchNearbyTarget keeps the nearest matching entry as the destination before the caster fallback"
    );
    assert_ne!(summoned.world().position(), player_position);
    assert_ne!(summoned.world().position(), far_target_position);
    assert_ne!(summoned.world().position(), wrong_entry_position);
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "OR_DB nearby-entry destination summon should trigger represented visibility create/update delivery"
    );
}
#[tokio::test]
async fn or_db_missing_row_filters_represented_nearby_entry_conditions_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 720_i32;
    let template_entry = 9026_u32;
    let player_guid = ObjectGuid::create_player(1, 7030);
    let player_position = Position::new(300.0, 400.0, 60.0, 0.0);
    let rejected_guid = test_creature_guid(7031);
    let accepted_guid = test_creature_guid(7032);
    let rejected_position = Position::new(302.0, 400.0, 60.0, 0.5);
    let accepted_position = Position::new(310.0, 400.0, 60.0, 1.5);
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
    session.set_spell_target_position_store(Arc::new(
        wow_data::SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
            Vec::<wow_data::SpellTargetPositionRowLikeCpp>::new(),
            &{
                let mut store = wow_data::SpellStore::new();
                store.insert(spell_id, spell_info.clone());
                store
            },
            |map_id| map_id == 571,
        ),
    ));
    let condition_store = Arc::new(ConditionEntriesByTypeStore::from_conditions_like_cpp([
        Condition {
            source_type: ConditionSourceType::SpellImplicitTarget,
            source_group: 1 << summon_effect.effect_index,
            source_entry: spell_id,
            condition_type: ConditionType::ObjectEntryGuid,
            condition_value1: TypeId::Unit as u32,
            condition_value2: template_entry,
            ..Condition::default()
        },
        Condition {
            source_type: ConditionSourceType::SpellImplicitTarget,
            source_group: 1 << summon_effect.effect_index,
            source_entry: spell_id,
            condition_type: ConditionType::Level,
            condition_value1: 80,
            condition_value2: wow_constants::ComparisonType::Eq as u32,
            ..Condition::default()
        },
    ]));
    let mut conditioned_spell_store = wow_data::SpellStore::new();
    conditioned_spell_store.insert(spell_id, spell_info);
    conditioned_spell_store.attach_spell_implicit_target_conditions_like_cpp(&condition_store);
    session.set_condition_store(Arc::clone(&condition_store));
    session.set_spell_store(Arc::new(conditioned_spell_store));
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        rejected_guid,
        template_entry,
        rejected_position,
        571,
        0,
        70,
    );
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        accepted_guid,
        template_entry,
        accepted_position,
        571,
        0,
        80,
    );

    let implicit_conditions = session
        .implicit_target_conditions_like_cpp(spell_id, summon_effect.effect_index)
        .expect("implicit target conditions should be attached");
    assert_eq!(implicit_conditions.len(), 2);
    assert_eq!(
        session.represented_nearby_entry_destination_like_cpp(
            &summon_effect,
            &player_position,
            50.0,
            Some(implicit_conditions.as_slice()),
        ),
        Some(accepted_position),
        "represented SearchNearbyTarget seam should apply the implicit target conditions"
    );

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 720,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("OR_DB missing DB row with represented conditioned nearby entry should execute");

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
        .expect("OR_DB conditioned nearby-entry destination summon should be visible");
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    assert_eq!(
        summoned.world().position(),
        accepted_position,
        "C++ SearchNearbyTarget applies ImplicitTargetConditions before keeping the nearest candidate"
    );
    assert_ne!(summoned.world().position(), rejected_position);
    assert_ne!(summoned.world().position(), player_position);
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "OR_DB conditioned nearby-entry destination summon should trigger represented visibility create/update delivery"
    );
}
#[tokio::test]
async fn nearby_entry_destination_uses_represented_nearby_entry_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 721_i32;
    let template_entry = 9027_u32;
    let player_guid = ObjectGuid::create_player(1, 7033);
    let player_position = Position::new(320.0, 420.0, 62.0, 0.0);
    let near_target_guid = test_creature_guid(7034);
    let far_target_guid = test_creature_guid(7035);
    let wrong_entry_guid = test_creature_guid(7036);
    let near_target_position = Position::new(324.0, 420.0, 62.0, 0.75);
    let far_target_position = Position::new(336.0, 420.0, 62.0, 1.5);
    let wrong_entry_position = Position::new(322.0, 420.0, 62.0, 2.25);
    let canonical = shared_canonical_map_manager();
    let mut summon_effect =
        summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap());
    summon_effect.implicit_target_1 = wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY;
    let spell_info =
        gameobject_summon_spell_info_like_cpp(spell_id, 0, vec![summon_effect.clone()]);

    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        spell_info,
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
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        far_target_guid,
        template_entry,
        far_target_position,
        571,
        0,
        80,
    );
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        wrong_entry_guid,
        template_entry + 1,
        wrong_entry_position,
        571,
        0,
        80,
    );
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        near_target_guid,
        template_entry,
        near_target_position,
        571,
        0,
        80,
    );

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 721,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("nearby-entry implicit destination should execute");

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
        .expect("nearby-entry destination summon should be visible");
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    assert_eq!(
        summoned.world().position(),
        near_target_position,
        "C++ TARGET_DEST_NEARBY_ENTRY uses SearchNearbyTarget and SpellDestination(*target)"
    );
    assert_ne!(summoned.world().position(), player_position);
    assert_ne!(summoned.world().position(), far_target_position);
    assert_ne!(summoned.world().position(), wrong_entry_position);
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "nearby-entry destination summon should trigger represented visibility create/update delivery"
    );
}
#[tokio::test]
async fn nearby_entry_destination_without_target_fails_bad_implicit_targets_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 722_i32;
    let template_entry = 9028_u32;
    let player_guid = ObjectGuid::create_player(1, 7037);
    let player_position = Position::new(340.0, 440.0, 63.0, 0.0);
    let canonical = shared_canonical_map_manager();
    let mut summon_effect =
        summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap());
    summon_effect.implicit_target_1 = wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY;

    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        gameobject_summon_spell_info_like_cpp(spell_id, 0, vec![summon_effect]),
    );
    let mut misc = summon_go_spell_misc_entry_like_cpp(spell_id as u32, 0);
    misc.range_index = 89;
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([misc])));
    session.set_spell_range_store(Arc::new(wow_data::SpellRangeStore::from_entries([
        wow_data::SpellRangeEntry {
            id: 89,
            display_name: String::new(),
            display_name_short: String::new(),
            flags: 0,
            range_min: [0.0, 0.0],
            range_max: [30.0, 30.0],
        },
    ])));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 722,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("missing nearby-entry target should be reported as a represented cast failure");

    assert!(
        session
            .client_visible_guids_like_cpp
            .snapshot_like_cpp()
            .into_iter()
            .all(|guid| !guid.is_game_object()),
        "C++ TARGET_DEST_NEARBY_ENTRY must not fall back to a caster-based GameObject when no nearby target exists"
    );
    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    assert_eq!(
        managed.map().map_object_count(),
        1,
        "missing non-OR_DB nearby-entry destination must fail before fabricating a summoned GameObject"
    );
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    let expected_cast_failed = wow_packet::packets::spell::CastFailed {
        cast_id: ObjectGuid::EMPTY,
        spell_id,
        // C++ Spell::SendCastResult sets packet.Visual = m_SpellVisual (set in the
        // Spell ctor, before target selection), so an early BAD_IMPLICIT_TARGETS
        // failure still carries the spell's visual — not a default/empty one.
        visual: wow_packet::packets::spell::SpellCastVisual {
            spell_visual_id: 722,
            script_visual_id: 0,
        },
        reason: SpellCastResult::BadImplicitTargets as i32,
        fail_arg1: 0,
        fail_arg2: 0,
    }
    .to_bytes();
    assert_eq!(
        packets,
        vec![expected_cast_failed],
        "C++ Spell::SelectImplicitNearbyTargets sends SPELL_FAILED_BAD_IMPLICIT_TARGETS before SpellGo when target 46 finds no nearby object"
    );
    assert_eq!(update_object_packet_count_like_cpp(&packets), 0);
}
#[test]
fn bind_misc_area_preserves_cpp_uint32_conversion() {
    assert_eq!(WorldSession::bind_area_id_like_cpp(777, 34), 777);
    assert_eq!(WorldSession::bind_area_id_like_cpp(0, 34), 34);
    assert_eq!(
        WorldSession::bind_area_id_like_cpp(-1, 34),
        u32::MAX,
        "C++ assignment from int32 MiscValue to uint32 areaId preserves all 32 bits"
    );
}
#[tokio::test]
async fn bind_without_destination_uses_player_world_location_and_area_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 720_i32;
    let player_guid = ObjectGuid::create_player(1, 7026);
    let player_position = Position::new(290.0, 390.0, 58.0, 0.875);
    let canonical = shared_canonical_map_manager();
    let bind_effect = wow_data::SpellEffectInfo {
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND,
        ..Default::default()
    };
    let spell_info = bind_spell_info_like_cpp(spell_id, vec![bind_effect]);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "BindNoDst".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_zone_area_like_cpp(12, 34);
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
                spell_visual_id: 720,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("destination-less bind should execute");

    assert_eq!(
        session.represented_homebind_like_cpp(),
        Some(RepresentedHomebindLikeCpp {
            map_id: 571,
            area_id: 34,
            position: player_position,
        }),
        "C++ EffectBind falls back to player world location and current area when no dst/MiscValue exists"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(opcodes.contains(&ServerOpcodes::BindPointUpdate));
    assert!(opcodes.contains(&ServerOpcodes::PlayerBound));
}
#[tokio::test]
async fn primary_bind_without_destination_uses_player_location_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 731_i32;
    let player_guid = ObjectGuid::create_player(1, 7030);
    let player_position = Position::new(291.0, 391.0, 59.0, 0.875);
    let canonical = shared_canonical_map_manager();

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "PrimaryBindNoDst".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_zone_area_like_cpp(12, 34);
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND,
            effect_base_points: 0,
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
                spell_visual_id: 731,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("represented primary bind should execute");

    assert_eq!(
        session.represented_homebind_like_cpp(),
        Some(RepresentedHomebindLikeCpp {
            map_id: 571,
            area_id: 34,
            position: player_position,
        }),
        "C++ EffectBind falls back to player world location and current area when no destination/MiscValue exists"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(opcodes.contains(&ServerOpcodes::BindPointUpdate));
    assert!(opcodes.contains(&ServerOpcodes::PlayerBound));
}
#[test]
fn represented_mount_liquid_state_uses_cpp_liquid_bits_and_swimming_flag() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 21_861);
    let canonical = shared_canonical_map_manager();
    session.set_player_guid(Some(player_guid));
    session.player_position = Some(Position::ZERO);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, Position::ZERO, 0, 0);

    assert_eq!(
        session.represented_player_mount_liquid_state_like_cpp(),
        Some((false, false))
    );

    session.set_player_liquid_status_like_cpp(LIQUID_MAP_IN_WATER_LIKE_CPP);
    assert_eq!(
        session.represented_player_mount_liquid_state_like_cpp(),
        Some((false, true))
    );

    session.set_player_liquid_status_like_cpp(LIQUID_MAP_UNDER_WATER_LIKE_CPP);
    assert_eq!(
        session.represented_player_mount_liquid_state_like_cpp(),
        Some((true, true))
    );

    session.set_player_liquid_status_like_cpp(0);
    session.set_player_movement_flags_like_cpp(MovementFlag::SWIMMING);
    assert_eq!(
        session.represented_player_mount_liquid_state_like_cpp(),
        Some((true, false))
    );
}
#[test]
fn time_sync_response_sets_initial_clock_delta_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let _ = WorldSession::game_time_ms_like_cpp();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let sent_time = WorldSession::game_time_ms_like_cpp();
    session.time_sync_pending_requests.insert(7, sent_time);

    session.record_time_sync_response_like_cpp(7, sent_time.saturating_sub(1));

    assert!(session.time_sync_pending_requests.is_empty());
    assert_eq!(session.time_sync_clock_delta_queue.len(), 1);
    assert!(
        session.time_sync_clock_delta >= 1,
        "expected initial fallback delta from first sample"
    );
}
#[test]
fn send_time_sync_uses_cpp_timer_sequence() {
    let (mut session, _pkt_tx, _send_rx) = make_session();

    session.send_time_sync();
    assert_eq!(session.time_sync_next_counter, 1);
    assert_eq!(session.time_sync_timer_ms, 5_000);
    assert!(session.time_sync_pending_requests.contains_key(&0));

    session.send_time_sync();
    assert_eq!(session.time_sync_next_counter, 2);
    assert_eq!(session.time_sync_timer_ms, 10_000);
    assert!(session.time_sync_pending_requests.contains_key(&1));
}
#[tokio::test]
async fn dynamic_object_values_snapshot_not_in_world_player_no_send_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_516);
    let dynamic_guid = test_dynamic_object_guid(601_516, 50_517);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        player_guid,
        601_516,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 38.0);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 7)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(player_guid)
        .unwrap()
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    session.client_visible_guids_like_cpp.insert(dynamic_guid);

    assert_eq!(
        session.send_represented_dynamic_object_values_updates_from_last_map_send_object_updates_like_cpp(),
        0
    );

    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&dynamic_guid)
    );
}
