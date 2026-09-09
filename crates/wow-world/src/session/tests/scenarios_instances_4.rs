//! Session scenarios exercising the represented instances responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn far_sight_enable_rejects_cross_instance_target_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 4260);
    let target_guid = test_creature_guid(4261);
    let previous_seer = test_creature_guid(4262);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(player_guid));
    session.player_name = Some("FarSightCrossInstance".into());
    session.player_position = Some(Position::new(10.0, 10.0, 0.0, 0.0));
    session.current_map_id = 571;
    session.represented_seer_guid_like_cpp = Some(previous_seer);
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 11);
    set_canonical_player_farsight_object_on_map_like_cpp(
        &canonical,
        player_guid,
        target_guid,
        571,
        11,
    );
    add_canonical_test_creature_on_map(
        &canonical,
        target_guid,
        9003,
        Position::new(12.0, 10.0, 0.0, 0.0),
        0,
        571,
        12,
    );

    session.apply_far_sight_like_cpp(true);

    assert_eq!(
        session.represented_seer_guid_like_cpp(),
        Some(previous_seer)
    );
    assert!(
        !canonical
            .lock()
            .unwrap()
            .find_map(571, 11)
            .unwrap()
            .map()
            .contains_map_object_like_cpp(target_guid)
    );
}
#[test]
fn far_sight_enable_same_nonzero_instance_sets_seer_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 4263);
    let target_guid = test_creature_guid(4264);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(player_guid));
    session.player_name = Some("FarSightSameInstance".into());
    session.player_position = Some(Position::new(10.0, 10.0, 0.0, 0.0));
    session.current_map_id = 571;
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 13);
    add_canonical_test_creature_on_map(
        &canonical,
        target_guid,
        9004,
        Position::new(12.0, 10.0, 0.0, 0.0),
        0,
        571,
        13,
    );
    set_canonical_player_farsight_object_on_map_like_cpp(
        &canonical,
        player_guid,
        target_guid,
        571,
        13,
    );

    session.apply_far_sight_like_cpp(true);

    assert_eq!(session.represented_seer_guid_like_cpp(), Some(target_guid));
}
#[test]
fn canonical_player_logout_cleanup_preserves_other_map_objects_like_cpp() {
    run_canonical_player_owner_test(|| {
        let (mut session, _, _) = make_session();
        let (mut other_session, _, _) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 47);
        let other_player_guid = ObjectGuid::create_player(1, 48);
        let creature_guid = test_creature_guid(19_041);

        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_player_guid(Some(player_guid));
        session.player_name = Some("LogoutOnlySelf".into());
        session.player_position = Some(Position::new(1.0, 2.0, 3.0, 0.0));
        session.current_map_id = 571;

        other_session.set_player_guid(Some(other_player_guid));
        other_session.player_name = Some("OtherStays".into());
        other_session.player_position = Some(Position::new(4.0, 5.0, 6.0, 0.0));
        other_session.current_map_id = 571;

        insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
        assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
        insert_session_player_into_canonical_map_like_cpp(&other_session, &canonical, 571, 0);
        add_canonical_test_creature(
            &canonical,
            creature_guid,
            123,
            Position::new(7.0, 8.0, 9.0, 0.0),
            0,
        );

        session.cleanup_shared_runtime_state();

        let manager = canonical.lock().unwrap();
        let map = manager.find_map(571, 0).unwrap().map();
        assert!(map.get_typed_player(player_guid).is_none());
        assert!(map.get_typed_player(other_player_guid).is_some());
        assert!(
            map.with_creature_like_cpp(creature_guid, Clone::clone)
                .is_some()
        );
    });
}
#[test]
fn canonical_player_logout_cleanup_missing_map_is_noop_like_cpp() {
    run_canonical_player_owner_test(|| {
        let (mut session, _, _) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 49);

        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_player_guid(Some(player_guid));
        session.player_name = Some("NoMap".into());
        session.player_position = Some(Position::new(1.0, 2.0, 3.0, 0.0));
        session.current_map_id = 571;

        assert!(canonical.lock().unwrap().find_map(571, 0).is_none());
        session.cleanup_shared_runtime_state();
        assert!(canonical.lock().unwrap().find_map(571, 0).is_none());
    });
}
#[test]
fn canonical_player_logout_disconnect_cleanup_removes_player_from_map_like_cpp() {
    run_canonical_player_owner_test(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let (mut session, _, _) = make_session();
                let canonical = shared_canonical_map_manager();
                let player_guid = ObjectGuid::create_player(1, 50);

                session.set_canonical_map_manager(Arc::clone(&canonical));
                session.set_player_guid(Some(player_guid));
                session.player_name = Some("DisconnectMap".into());
                session.player_position = Some(Position::new(1.0, 2.0, 3.0, 0.0));
                session.current_map_id = 571;

                insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
                assert!(session.adopt_registered_canonical_player_fixture_like_cpp());

                session
                    .cleanup_shared_runtime_state_on_disconnect_like_cpp()
                    .await;

                assert!(
                    canonical
                        .lock()
                        .unwrap()
                        .find_map(571, 0)
                        .unwrap()
                        .map()
                        .get_typed_player(player_guid)
                        .is_none()
                );
            });
    });
}
#[test]
fn legacy_turret_ai_can_attack_uses_active_map_difficulty_range_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_306);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.set_spell(0, 7_004);
            creature.creature.unit_mut().set_combat_reach(0.0);
        })
        .unwrap();
    let mut active_misc = spell_misc_entry_like_cpp(8_303, 7_004, 72);
    active_misc.difficulty_id = 2;
    let config = LegacyCreatureAggroConfigLikeCpp {
        spell_misc_store: Some(Arc::new(wow_data::SpellMiscStore::from_entries([
            spell_misc_entry_like_cpp(8_302, 7_004, 71),
            active_misc,
        ]))),
        spell_range_store: Some(Arc::new(wow_data::SpellRangeStore::from_entries([
            spell_range_entry_like_cpp(71, 0.0, 2.0),
            spell_range_entry_like_cpp(72, 0.0, 10.0),
        ]))),
        difficulty_store: Some(Arc::new(DifficultyStore::from_entries([DifficultyEntry {
            id: 2,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        }]))),
        ..legacy_aggro_hostile_config_like_cpp()
    };
    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    let mut candidate = legacy_aggro_candidate_like_cpp(
        ObjectGuid::create_player(1, 91_307),
        Position::new(16.0, 10.0, 0.0, 0.0),
    );
    candidate.player_combat_reach = 0.0;
    candidate.map_difficulty_id = 2;

    assert_eq!(
        legacy_creature_ai_can_attack_decision_like_cpp(
            &CreatureAiKindLikeCpp::TurretAI,
            creature,
            &candidate,
            &config,
        ),
        LegacyCreatureAiCanAttackDecisionLikeCpp::Allowed,
        "active difficulty range 10 must win over base range 2"
    );
    candidate.map_difficulty_id = 0;
    assert_eq!(
        legacy_creature_ai_can_attack_decision_like_cpp(
            &CreatureAiKindLikeCpp::TurretAI,
            creature,
            &candidate,
            &config,
        ),
        LegacyCreatureAiCanAttackDecisionLikeCpp::Rejected
    );
}
