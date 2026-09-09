//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn cancel_mount_aura_removes_represented_mounted_aura_like_cpp() {
    let (mut session, _, _) = make_session();
    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 77,
        effect_misc_value_1: 0,
        ..Default::default()
    };

    session
        .apply_represented_mounted_aura_like_cpp(100, ObjectGuid::EMPTY, &effect)
        .unwrap();

    assert!(session.player_mounted_like_cpp);
    assert!(
        session
            .player_unit_flags_like_cpp
            .contains(UnitFlags::MOUNT)
    );
    assert!(
        session
            .visible_auras
            .values()
            .any(|aura| { aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::Mounted) })
    );

    assert_eq!(
        session.remove_represented_mount_auras_cancelable_like_cpp(),
        1
    );

    assert!(!session.player_mounted_like_cpp);
    assert!(
        !session
            .player_unit_flags_like_cpp
            .contains(UnitFlags::MOUNT)
    );
    assert!(
        !session
            .visible_auras
            .values()
            .any(|aura| { aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::Mounted) })
    );
    assert_eq!(
        session.remove_represented_mount_auras_cancelable_like_cpp(),
        0
    );
}
#[test]
fn represented_mount_source_spell_usable_matches_cpp_mount_condition_filter() {
    let (mut session, _, _) = make_session();
    session.player_class = 1;
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 1,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 100,
            player_condition_id: 42,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
        wow_data::MountEntry {
            id: 2,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 101,
            player_condition_id: 43,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            class_mask: 1,
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ])));

    assert!(session.represented_mount_source_spell_usable_like_cpp(100));
    assert!(!session.represented_mount_source_spell_usable_like_cpp(101));
    assert!(!session.represented_mount_source_spell_usable_like_cpp(999));
}
#[test]
fn account_mount_load_learns_mount_spells_before_use_condition_like_cpp() {
    let (mut session, _, _) = make_session();
    session.player_class = 1;
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 1,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 100,
            player_condition_id: 42,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
        wow_data::MountEntry {
            id: 2,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 101,
            player_condition_id: 43,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            class_mask: 1,
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ])));

    session.set_account_mounts_like_cpp(vec![
        wow_packet::packets::misc::AccountMount {
            spell_id: 100,
            flags: 0,
        },
        wow_packet::packets::misc::AccountMount {
            spell_id: 101,
            flags: 0,
        },
    ]);
    assert_eq!(
        session.account_mount_login_partial_rows_like_cpp(),
        vec![wow_packet::packets::misc::AccountMount {
            spell_id: 100,
            flags: 0,
        }],
        "C++ CollectionMgr::AddMount stores and learns both rows, but sends the partial AccountMountUpdate only after PlayerCondition succeeds"
    );
    assert!(session.known_spells_like_cpp().contains(&100));
    assert!(
        session.known_spells_like_cpp().contains(&101),
        "C++ CollectionMgr::AddMount stores/learns mounts before PlayerCondition; that condition applies to using the mount"
    );
    assert!(
        session
            .represented_dependent_known_spells_like_cpp()
            .contains(&100)
    );
    assert!(
        session
            .represented_dependent_known_spells_like_cpp()
            .contains(&101),
        "C++ CollectionMgr::AddMount calls Player::LearnSpell(spellId, true), which creates dependent spells skipped by _SaveSpells"
    );

    session.set_known_spells_like_cpp(vec![635]);
    assert!(session.known_spells_like_cpp().contains(&635));
    assert!(session.known_spells_like_cpp().contains(&100));
    assert!(session.known_spells_like_cpp().contains(&101));
    assert!(
        !session
            .represented_dependent_known_spells_like_cpp()
            .contains(&635)
    );
    assert!(
        session
            .represented_dependent_known_spells_like_cpp()
            .contains(&100)
    );
    assert!(
        session
            .represented_dependent_known_spells_like_cpp()
            .contains(&101)
    );
}
#[test]
fn loaded_character_mount_spells_promote_to_account_collection_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 1,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 300,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
        wow_data::MountEntry {
            id: 2,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 100,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));
    session.set_account_mounts_like_cpp(vec![wow_packet::packets::misc::AccountMount {
        spell_id: 300,
        flags: 1,
    }]);

    let added = session.promote_loaded_character_mount_spells_like_cpp(&[100, 200, 300]);

    assert_eq!(
        added, 1,
        "C++ Player::_LoadSpells calls AddSpell; AddSpell promotes DB-backed mount spells into CollectionMgr while preserving existing account mount rows"
    );
    assert_eq!(
        session.account_mount_rows_like_cpp(),
        vec![
            wow_packet::packets::misc::AccountMount {
                spell_id: 100,
                flags: 0
            },
            wow_packet::packets::misc::AccountMount {
                spell_id: 300,
                flags: 1
            },
        ],
        "C++ CollectionMgr stores mounts in std::map, so the full AccountMountUpdate is sorted by source spell id"
    );
}
#[test]
fn loaded_character_mount_spell_promotes_faction_counterpart_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 1,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 100,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
        wow_data::MountEntry {
            id: 2,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 101,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));
    session.set_mount_definition_store_like_cpp(Arc::new(
        wow_data::MountDefinitionStoreLikeCpp::from_entries([(100, 101)]),
    ));

    let added = session.promote_loaded_character_mount_spells_like_cpp(&[100]);

    assert_eq!(added, 1);
    assert_eq!(
        session.account_mount_rows_like_cpp(),
        vec![
            wow_packet::packets::misc::AccountMount {
                spell_id: 100,
                flags: 0
            },
            wow_packet::packets::misc::AccountMount {
                spell_id: 101,
                flags: 0
            },
        ],
        "C++ Player::_LoadSpells -> AddSpell -> CollectionMgr::AddMount includes faction-specific counterpart mounts"
    );
}
#[test]
fn mount_store_injection_relearns_account_mount_spells_with_controller_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "MountedPlayer".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([])));
    session.set_account_mounts_like_cpp(vec![wow_packet::packets::misc::AccountMount {
        spell_id: 100,
        flags: 0,
    }]);
    assert!(
        !session.known_spells_like_cpp().contains(&100),
        "an unavailable Mount.db2 source spell must not become castable"
    );

    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 1,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 100,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));

    assert!(
        session.known_spells_like_cpp().contains(&100),
        "C++ CollectionMgr account mounts must be castable through Player::HasSpell after the mount store is available"
    );
}
#[tokio::test]
async fn dynamic_object_values_snapshot_dynamic_object_seer_wrong_caster_no_send_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_522);
    let other_player_guid = ObjectGuid::create_player(1, 50_523);
    let dynamic_guid = test_dynamic_object_guid(601_522, 50_524);
    let seer_guid = test_dynamic_object_guid(601_523, 50_525);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    let visibility_range = canonical
        .lock()
        .unwrap()
        .find_map(571, 7)
        .unwrap()
        .map()
        .visibility_range();
    let updated_position = Position::new(10.0 + visibility_range + 25.0, 20.0, 30.0, 0.0);
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        player_guid,
        601_522,
        updated_position,
        571,
        7,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        seer_guid,
        other_player_guid,
        601_523,
        Position::new(
            updated_position.x + 1.0,
            updated_position.y,
            updated_position.z,
            0.0,
        ),
        571,
        7,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 40.5);
    session.represented_seer_guid_like_cpp = Some(seer_guid);
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
#[tokio::test]
async fn add_farsight_live_spell_sets_session_seer_to_created_dynamic_object_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 4940);
    let spell_id = 49_400;
    let destination = Position::new(100.0, 200.0, 30.0, 1.5);

    configure_add_farsight_live_session_like_cpp(&mut session, &canonical, player_guid, spell_id);
    assert_eq!(session.represented_seer_guid_like_cpp(), Some(player_guid));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 678,
                script_visual_id: 0,
            },
            add_farsight_target_data(Some(destination)),
        )
        .await
        .expect("live AddFarsight spell should execute");

    let seer_guid = session
        .represented_seer_guid_like_cpp()
        .expect("C++ SetSeer(target) evidence should update represented session seer");
    assert_ne!(seer_guid, player_guid);
    let guard = canonical.lock().unwrap();
    let dynamic_object = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_dynamic_object(seer_guid)
        .expect("represented session seer should be the created map-owned DynamicObject");
    assert_eq!(dynamic_object.world().position(), destination);
    assert_eq!(dynamic_object.caster_guid(), player_guid);
    assert_eq!(dynamic_object.spell_id(), spell_id);
    assert_eq!(dynamic_object.data().spell_visual_id, 678);
    drop(guard);

    let expected_farsight_update = expected_active_player_farsight_object_values_update_like_cpp(
        player_guid,
        session.player_map_id_like_cpp(),
        seer_guid,
    );
    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        packets
            .iter()
            .any(|bytes| bytes == &expected_farsight_update),
        "live AddFarsight SetViewpoint success should send the C++ ActivePlayerData::FarsightObject VALUES update with mask bits 0+26"
    );
}
#[tokio::test]
async fn add_farsight_live_spell_without_destination_keeps_session_seer_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 4941);
    let spell_id = 49_401;

    configure_add_farsight_live_session_like_cpp(&mut session, &canonical, player_guid, spell_id);

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 679,
                script_visual_id: 0,
            },
            add_farsight_target_data(None),
        )
        .await
        .expect("missing AddFarsight destination is a no-op effect, not a spell failure");

    assert_eq!(session.represented_seer_guid_like_cpp(), Some(player_guid));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(571, 0)
            .unwrap()
            .map()
            .map_object_count(),
        1,
        "only the canonical player should remain map-owned"
    );
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(
        update_object_packet_count_like_cpp(&packets),
        0,
        "missing AddFarsight destination must not emit represented ActivePlayerData::FarsightObject VALUES update"
    );
}
#[tokio::test]
async fn add_farsight_live_spell_already_has_viewpoint_keeps_session_seer_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 4942);
    let spell_id = 49_402;
    let existing_viewpoint =
        ObjectGuid::create_world_object(HighGuid::DynamicObject, 0, 1, 571, 0, 12_000, 494_200);

    configure_add_farsight_live_session_like_cpp(&mut session, &canonical, player_guid, spell_id);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_farsight_object_like_cpp(existing_viewpoint);
        })
        .expect("canonical player should exist");

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 680,
                script_visual_id: 0,
            },
            add_farsight_target_data(Some(Position::new(150.0, 250.0, 35.0, 1.0))),
        )
        .await
        .expect("AlreadyHasViewpoint is represented as no-op SetSeer consumption");

    assert_eq!(session.represented_seer_guid_like_cpp(), Some(player_guid));
    let guard = canonical.lock().unwrap();
    let player = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(player_guid)
        .unwrap();
    assert_eq!(player.active_data().farsight_object, existing_viewpoint);
    drop(guard);
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(
        update_object_packet_count_like_cpp(&packets),
        0,
        "AlreadyHasViewpoint must not emit represented ActivePlayerData::FarsightObject VALUES update"
    );
}
#[test]
fn represented_spellclick_accepts_pet_guid_through_get_pet_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let pet_guid = test_pet_guid(232);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 9007,
            spell_id: 913,
            cast_flags: NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP
                | NPC_CLICK_CAST_ORIG_CASTER_OWNER_LIKE_CPP,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 9007,
        |spell| spell == 913,
    )));
    add_canonical_test_pet(
        &canonical,
        pet_guid,
        player_guid,
        9007,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );

    assert!(pet_guid.is_pet());
    assert_eq!(
        session.represented_can_see_spell_click_on_creature_like_cpp(pet_guid),
        RepresentedCanSeeSpellClickOutcomeLikeCpp::Visible,
        "C++ ObjectAccessor::GetCreatureOrPetOrVehicle resolves pet GUIDs through GetPet"
    );
    let plan = session.represented_handle_spell_click_plan_like_cpp(pet_guid);
    assert_eq!(plan.casts.len(), 1);
    assert_eq!(plan.casts[0].spell_id, 913);
    assert_eq!(
        plan.casts[0].original_caster,
        RepresentedSpellClickUnitRefLikeCpp::Owner,
        "Rust keeps the canonical Pet owner GUID so owner-original-caster rows are not collapsed into the clicker during planning"
    );
}
#[test]
fn represented_can_see_spellclick_requires_viewer_flag_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(121);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 701,
            spell_id: 901,
            cast_flags: 0,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 701,
        |spell| spell == 901,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        701,
        Position::new(12.0, 0.0, 0.0, 0.0),
        0,
    );

    assert_eq!(
        session.represented_can_see_spell_click_on_creature_like_cpp(creature_guid),
        RepresentedCanSeeSpellClickOutcomeLikeCpp::Hidden
    );
}
#[test]
fn represented_can_see_spellclick_party_raid_require_exact_context_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(122);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 702,
            spell_id: 902,
            cast_flags: 0,
            user_type: SPELL_CLICK_USER_PARTY_LIKE_CPP,
        }],
        |entry| entry == 702,
        |spell| spell == 902,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        702,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );

    assert_eq!(
        session.represented_can_see_spell_click_on_creature_like_cpp(creature_guid),
        RepresentedCanSeeSpellClickOutcomeLikeCpp::ExactContextUnrepresented
    );
}
#[test]
fn represented_viewer_dependent_npc_flags_clears_hidden_spellclick_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(123);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [],
        |entry| entry == 703,
        |_| false,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        703,
        Position::new(12.0, 0.0, 0.0, 0.0),
        (UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32) | wow_constants::unit::NPCFlags1::GOSSIP.bits(),
    );

    let filtered = session.represented_viewer_dependent_creature_npc_flags_like_cpp(
        creature_guid,
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP
            | u64::from(wow_constants::unit::NPCFlags1::GOSSIP.bits()),
    );

    assert_eq!(
        filtered,
        u64::from(wow_constants::unit::NPCFlags1::GOSSIP.bits())
    );
}
#[test]
fn represented_viewer_dependent_npc_flags_keeps_unrepresented_spellclick_context_visible() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(124);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 704,
            spell_id: 904,
            cast_flags: 0,
            user_type: SPELL_CLICK_USER_RAID_LIKE_CPP,
        }],
        |entry| entry == 704,
        |spell| spell == 904,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        704,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );

    assert_eq!(
        session.represented_can_see_spell_click_on_creature_like_cpp(creature_guid),
        RepresentedCanSeeSpellClickOutcomeLikeCpp::ExactContextUnrepresented
    );
    assert_eq!(
        session.represented_viewer_dependent_creature_npc_flags_like_cpp(
            creature_guid,
            UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP,
        ),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP
    );
}
#[test]
fn represented_handle_spellclick_plans_cast_flags_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(224);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 804,
            spell_id: 904,
            cast_flags: NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP
                | NPC_CLICK_CAST_TARGET_CLICKER_LIKE_CPP
                | NPC_CLICK_CAST_ORIG_CASTER_OWNER_LIKE_CPP,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 804,
        |spell| spell == 904,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        804,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );

    let plan = session.represented_handle_spell_click_plan_like_cpp(creature_guid);
    assert_eq!(plan.casts.len(), 1);
    assert_eq!(plan.casts[0].spell_id, 904);
    assert_eq!(
        plan.casts[0].caster,
        RepresentedSpellClickUnitRefLikeCpp::Clicker
    );
    assert_eq!(
        plan.casts[0].target,
        RepresentedSpellClickUnitRefLikeCpp::Clicker
    );
    assert_eq!(
        plan.casts[0].original_caster,
        RepresentedSpellClickUnitRefLikeCpp::Owner
    );
    assert!(plan.ai_on_spell_click_unrepresented);
}
