//! Spell handler mount scenarios.
//!
//! Split out of the inline test module under #624; assertions unchanged.

use super::*;

#[tokio::test]
async fn cancel_aura_removes_matching_represented_mount_aura_like_cpp() {
    let (mut session, send_rx) = make_session();
    let caster_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(caster_guid));
    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 77,
        effect_misc_value_1: 0,
        ..Default::default()
    };

    session
        .apply_represented_mounted_aura_for_test_like_cpp(12_345, caster_guid, &effect)
        .unwrap();
    session.set_spell_store(mounted_spell_store(12_345, 0));
    assert!(session.player_mounted_like_cpp());

    session
        .handle_cancel_aura(cancel_aura_packet(12_345, caster_guid))
        .await;

    assert!(!session.player_mounted_like_cpp());
    assert!(!send_rx.is_empty());
}
#[tokio::test]
async fn cancel_aura_no_aura_cancel_spell_preserves_represented_mount_like_cpp() {
    let (mut session, send_rx) = make_session();
    let caster_guid = ObjectGuid::create_player(1, 42);
    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 77,
        effect_misc_value_1: 0,
        ..Default::default()
    };

    session
        .apply_represented_mounted_aura_for_test_like_cpp(12_345, caster_guid, &effect)
        .unwrap();
    session.set_spell_store(mounted_spell_store_with_no_aura_cancel(12_345, 0));
    let _ = drain_server_opcodes(&send_rx);

    session
        .handle_cancel_aura(cancel_aura_packet(12_345, caster_guid))
        .await;

    assert!(session.player_mounted_like_cpp());
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn cancel_aura_preserves_represented_mount_from_other_caster_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let caster_guid = ObjectGuid::create_player(1, 42);
    let other_caster_guid = ObjectGuid::create_player(1, 43);
    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 77,
        effect_misc_value_1: 0,
        ..Default::default()
    };

    session
        .apply_represented_mounted_aura_for_test_like_cpp(12_345, caster_guid, &effect)
        .unwrap();
    session.set_spell_store(mounted_spell_store(12_345, 0));

    session
        .handle_cancel_aura(cancel_aura_packet(12_345, other_caster_guid))
        .await;

    assert!(session.player_mounted_like_cpp());
}
#[tokio::test]
async fn cancel_aura_empty_caster_matches_represented_mount_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let caster_guid = ObjectGuid::create_player(1, 42);
    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 77,
        effect_misc_value_1: 0,
        ..Default::default()
    };

    session
        .apply_represented_mounted_aura_for_test_like_cpp(12_345, caster_guid, &effect)
        .unwrap();
    session.set_spell_store(mounted_spell_store(12_345, 0));

    session
        .handle_cancel_aura(cancel_aura_packet(12_345, ObjectGuid::EMPTY))
        .await;

    assert!(!session.player_mounted_like_cpp());
}
#[tokio::test]
async fn cancel_mount_aura_no_aura_cancel_spell_preserves_mount_like_cpp() {
    let (mut session, send_rx) = make_session();
    let caster_guid = ObjectGuid::create_player(1, 42);
    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 77,
        effect_misc_value_1: 0,
        ..Default::default()
    };

    session
        .apply_represented_mounted_aura_for_test_like_cpp(12_345, caster_guid, &effect)
        .unwrap();
    session.set_spell_store(mounted_spell_store_with_no_aura_cancel(12_345, 0));
    let _ = drain_server_opcodes(&send_rx);

    session
        .handle_cancel_mount_aura(WorldPacket::new_empty())
        .await;

    assert!(session.player_mounted_like_cpp());
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn cast_known_account_mount_spell_applies_mounted_aura_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let spell_id = 12345;

    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_known_spells_like_cpp(vec![spell_id]);
    session.set_spell_store(mounted_spell_store(spell_id, 0));
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 7,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: spell_id,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));
    session.set_mount_x_display_store(Arc::new(wow_data::MountXDisplayStore::from_entries([
        wow_data::MountXDisplayEntry {
            id: 1,
            creature_display_info_id: 4321,
            player_condition_id: 0,
            mount_id: 7,
        },
    ])));

    session
        .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
        .await;

    assert!(session.player_mounted_like_cpp());
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(opcodes.contains(&ServerOpcodes::SpellGo));
    assert!(opcodes.contains(&ServerOpcodes::AuraUpdate));
    assert!(opcodes.contains(&ServerOpcodes::UpdateObject));

    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(player_guid)
        .unwrap();
    assert_eq!(player.unit().data().mount_display_id, 4321);
}
#[tokio::test]
async fn cast_mount_spell_fails_not_here_without_mount_capability_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 43);
    let spell_id = 12_346;

    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_known_spells_like_cpp(vec![spell_id]);
    session.set_spell_store(mounted_spell_store(spell_id, 1234));
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 8,
            mount_type_id: 7,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: spell_id,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));

    session
        .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
        .await;

    assert!(!session.player_mounted_like_cpp());
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 2);
    assert_eq!(
        &packets[0][..2],
        &(ServerOpcodes::SpellPrepare as u16).to_le_bytes()
    );
    assert_eq!(
        cast_failed_reason_like_cpp(&packets[1]),
        SpellCastResult::NotHere as i32
    );
}
#[tokio::test]
async fn cast_flying_mount_spell_fails_only_abovewater_in_water_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 44);
    let spell_id = 12_347;

    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_known_spells_like_cpp(vec![spell_id]);
    session.set_player_liquid_status_like_cpp(crate::session::LIQUID_MAP_IN_WATER_LIKE_CPP);
    session.set_spell_store(mounted_flying_spell_store(spell_id, 1234));
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 9,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: spell_id,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));

    session
        .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
        .await;

    assert!(!session.player_mounted_like_cpp());
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 2);
    assert_eq!(
        &packets[0][..2],
        &(ServerOpcodes::SpellPrepare as u16).to_le_bytes()
    );
    assert_eq!(
        cast_failed_reason_like_cpp(&packets[1]),
        SpellCastResult::OnlyAbovewater as i32
    );
}
#[tokio::test]
async fn cast_shapeshift_mount_form_fails_not_here_without_capability_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 45);
    let spell_id = 12_348;
    let form_id = 55;

    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_known_spells_like_cpp(vec![spell_id]);
    session.set_spell_store(shapeshift_spell_store(spell_id, form_id));
    session.set_spell_shapeshift_form_store(Arc::new(
        wow_data::SpellShapeshiftFormStore::from_entries([wow_data::SpellShapeshiftFormEntry {
            id: form_id as u32,
            name: "Mounted Form".to_string(),
            creature_type: 0,
            flags: 0,
            attack_icon_file_id: 0,
            bonus_action_bar: 0,
            combat_round_time: 0,
            damage_variance: 0.0,
            mount_type_id: 7,
            creature_display_id: [0; 4],
            preset_spell_id: [0; wow_data::MAX_SHAPESHIFT_SPELLS],
        }]),
    ));

    session
        .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
        .await;

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 2);
    assert_eq!(
        &packets[0][..2],
        &(ServerOpcodes::SpellPrepare as u16).to_le_bytes()
    );
    assert_eq!(
        cast_failed_reason_like_cpp(&packets[1]),
        SpellCastResult::NotHere as i32
    );
}
#[tokio::test]
async fn cast_mount_spell_in_disallowed_shapeshift_form_sends_mount_result_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 46);
    let mount_spell_id = 12_349;
    let shapeshift_spell_id = 22_349;
    let form_id = 56;

    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_known_spells_like_cpp(vec![mount_spell_id]);
    session.set_spell_store(mounted_spell_store_with_active_shapeshift_aura(
        mount_spell_id,
        shapeshift_spell_id,
        form_id,
    ));
    session.set_spell_shapeshift_form_store(Arc::new(
        wow_data::SpellShapeshiftFormStore::from_entries([wow_data::SpellShapeshiftFormEntry {
            id: form_id as u32,
            name: "Non Stance Form".to_string(),
            creature_type: 0,
            flags: 0,
            attack_icon_file_id: 0,
            bonus_action_bar: 0,
            combat_round_time: 0,
            damage_variance: 0.0,
            mount_type_id: 0,
            creature_display_id: [0; 4],
            preset_spell_id: [0; wow_data::MAX_SHAPESHIFT_SPELLS],
        }]),
    ));
    assert!(
        session.insert_player_visible_aura_like_cpp(active_shapeshift_aura_for_test(
            shapeshift_spell_id,
            player_guid
        ),)
    );

    session
        .handle_cast_spell(cast_spell_packet(mount_spell_id, player_guid))
        .await;

    assert!(!session.player_mounted_like_cpp());
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 2);
    assert_eq!(
        &packets[0][..2],
        &(ServerOpcodes::SpellPrepare as u16).to_le_bytes()
    );
    assert_eq!(mount_result_like_cpp(&packets[1]), 8);
}
#[tokio::test]
async fn cast_mount_spell_in_stance_shapeshift_form_is_allowed_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 47);
    let mount_spell_id = 12_350;
    let shapeshift_spell_id = 22_350;
    let form_id = 57;

    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_known_spells_like_cpp(vec![mount_spell_id]);
    session.set_spell_store(mounted_spell_store_with_active_shapeshift_aura(
        mount_spell_id,
        shapeshift_spell_id,
        form_id,
    ));
    session.set_spell_shapeshift_form_store(Arc::new(
        wow_data::SpellShapeshiftFormStore::from_entries([wow_data::SpellShapeshiftFormEntry {
            id: form_id as u32,
            name: "Stance Form".to_string(),
            creature_type: 0,
            flags: 0x0000_0001,
            attack_icon_file_id: 0,
            bonus_action_bar: 0,
            combat_round_time: 0,
            damage_variance: 0.0,
            mount_type_id: 0,
            creature_display_id: [0; 4],
            preset_spell_id: [0; wow_data::MAX_SHAPESHIFT_SPELLS],
        }]),
    ));
    assert!(
        session.insert_player_visible_aura_like_cpp(active_shapeshift_aura_for_test(
            shapeshift_spell_id,
            player_guid
        ),)
    );

    session
        .handle_cast_spell(cast_spell_packet(mount_spell_id, player_guid))
        .await;

    assert!(session.player_mounted_like_cpp());
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(!opcodes.contains(&ServerOpcodes::MountResult));
    assert!(!opcodes.contains(&ServerOpcodes::CastFailed));
    assert!(opcodes.contains(&ServerOpcodes::SpellGo));
}
#[tokio::test]
async fn cast_mount_spell_in_disallowed_transformed_display_sends_mount_result_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 48);
    let mount_spell_id = 12_351;
    let transformed_display_id = 88_001;

    install_canonical_player(&mut session, &canonical, player_guid);
    set_canonical_player_display_for_test(&canonical, player_guid, transformed_display_id, false);
    session.set_known_spells_like_cpp(vec![mount_spell_id]);
    session.set_spell_store(mounted_spell_store(mount_spell_id, 0));
    set_transformed_display_mount_check_stores_for_test(&mut session, transformed_display_id, 0, 0);

    session
        .handle_cast_spell(cast_spell_packet(mount_spell_id, player_guid))
        .await;

    assert!(!session.player_mounted_like_cpp());
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 2);
    assert_eq!(
        &packets[0][..2],
        &(ServerOpcodes::SpellPrepare as u16).to_le_bytes()
    );
    assert_eq!(mount_result_like_cpp(&packets[1]), 8);
}
#[tokio::test]
async fn cast_mount_spell_with_mountable_transform_spell_is_allowed_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 50);
    let mount_spell_id = 12_353;
    let transform_spell_id = 22_353;
    let transformed_display_id = 88_003;

    install_canonical_player(&mut session, &canonical, player_guid);
    set_canonical_player_display_for_test(&canonical, player_guid, transformed_display_id, false);
    session.set_known_spells_like_cpp(vec![mount_spell_id]);
    session.set_spell_store(mounted_spell_store_with_transform_spell(
        mount_spell_id,
        transform_spell_id,
        true,
    ));
    set_transformed_display_mount_check_stores_for_test(&mut session, transformed_display_id, 0, 0);
    assert!(
        session.insert_player_visible_aura_like_cpp(active_shapeshift_aura_for_test(
            transform_spell_id,
            player_guid
        ),)
    );

    session
        .handle_cast_spell(cast_spell_packet(mount_spell_id, player_guid))
        .await;

    assert!(session.player_mounted_like_cpp());
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(!opcodes.contains(&ServerOpcodes::MountResult));
    assert!(!opcodes.contains(&ServerOpcodes::CastFailed));
    assert!(opcodes.contains(&ServerOpcodes::SpellGo));
}
#[tokio::test]
async fn cast_mount_spell_in_mountable_transformed_model_is_allowed_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 49);
    let mount_spell_id = 12_352;
    let transformed_display_id = 88_002;

    install_canonical_player(&mut session, &canonical, player_guid);
    set_canonical_player_display_for_test(&canonical, player_guid, transformed_display_id, false);
    session.set_known_spells_like_cpp(vec![mount_spell_id]);
    session.set_spell_store(mounted_spell_store(mount_spell_id, 0));
    set_transformed_display_mount_check_stores_for_test(
        &mut session,
        transformed_display_id,
        0x0000_0080,
        0,
    );

    session
        .handle_cast_spell(cast_spell_packet(mount_spell_id, player_guid))
        .await;

    assert!(session.player_mounted_like_cpp());
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(!opcodes.contains(&ServerOpcodes::MountResult));
    assert!(!opcodes.contains(&ServerOpcodes::CastFailed));
    assert!(opcodes.contains(&ServerOpcodes::SpellGo));
}
