use super::*;

#[test]
fn canonical_player_skills_follow_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_563);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "SkillOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");
    let owned = HashMap::from([(
        333,
        RepresentedPlayerSkillLikeCpp {
            skill_id: 333,
            step: 1,
            value: 150,
            max: 225,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
        },
    )]);

    assert!(session.set_complete_player_skill_records_like_cpp(owned.clone(), 1));
    assert_eq!(
        session.resolved_player_skill_records_like_cpp(),
        Some(owned.clone())
    );
    assert_eq!(
        session.complete_player_skill_occupied_slots_like_cpp(),
        Some(1)
    );
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session.resolved_player_skill_records_like_cpp(),
        Some(owned.clone())
    );

    let replacement_record = wow_entities::PlayerSkillRecord {
        skill_line_id: 202,
        current_value: 300,
        max_value: 300,
        step: 1,
        profession_slot: 1,
        state: wow_entities::PlayerSkillLoadState::Unchanged,
    };
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.replace_skill_records_like_cpp(
        vec![replacement_record.clone()],
        true,
        true,
        Some(1),
        BTreeSet::new(),
    );
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.resolved_player_skill_records_like_cpp(), None);
    assert_eq!(session.resolved_player_skill_value_like_cpp(333), None);
    assert!(!session.set_complete_player_skill_records_like_cpp(owned, 1));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.skill_records_like_cpp().to_vec()
            }),
        Some(vec![replacement_record])
    );
}

#[test]
fn canonical_player_damage_control_follows_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_576);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "DamageControlOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    session.set_player_cheat_god_like_cpp(true);
    session.set_player_normal_damage_immune_like_cpp(true);
    session.set_player_environmental_damage_immune_like_cpp(true);
    assert_eq!(
        session.resolved_player_damage_control_like_cpp(),
        Some(wow_entities::PlayerDamageControlStateLikeCpp {
            cheat_god: true,
            normal_damage_immune: true,
            environmental_damage_immune: true,
        })
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert!(
        session
            .resolved_player_damage_control_like_cpp()
            .is_some_and(|state| state.cheat_god)
    );

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.set_normal_damage_immune_like_cpp(true);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.resolved_player_damage_control_like_cpp(), None);
    session.set_player_cheat_god_like_cpp(true);
    session.set_player_environmental_damage_immune_like_cpp(true);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| player
                .damage_control_like_cpp()),
        Some(wow_entities::PlayerDamageControlStateLikeCpp {
            cheat_god: false,
            normal_damage_immune: true,
            environmental_damage_immune: false,
        })
    );
}
