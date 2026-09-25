use super::*;

#[tokio::test]
async fn spell_heal_syncs_canonical_creature_health_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_003);
    session.player_guid = Some(ObjectGuid::create_player(1, 43));
    session.client_visible_guids_like_cpp.insert(guid);
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.take_damage(20);
            creature.creature.clear_data_changes();
        })
        .unwrap();

    session.apply_heal(None, guid, 7).await.unwrap();

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(world_creature.current_hp(), 27);
    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::UpdateObject as u16);
}

#[tokio::test]
async fn spell_heal_skips_dead_creature_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_004);
    session.player_guid = Some(ObjectGuid::create_player(1, 45));
    session.client_visible_guids_like_cpp.insert(guid);
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.take_damage(20);
            creature
                .creature
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Corpse);
            creature.creature.clear_data_changes();
        })
        .unwrap();

    session.apply_heal(None, guid, 7).await.unwrap();

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(world_creature.current_hp(), 20);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn spell_self_heal_adds_half_effective_heal_threat_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 144);
    let tank_guid = ObjectGuid::create_player(1, 145);
    let creature_guid = test_creature_guid(18_144);
    let heal_spell_id = 18_144;
    let position = Position::new(10.0, 20.0, 30.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "ThreatHealer".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(50, 100);
    // `register_world_creature` bootstraps the legacy facade in instance
    // zero. Configure that fixture before the canonical Player makes the
    // session resolve legacy mutations through the test instance below.
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 7);
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9_001,
        position,
        0,
        7,
        80,
    );
    {
        let mut legacy = manager.write().unwrap();
        let creature = legacy
            .remove_creature_any(0, 0, creature_guid)
            .expect("move the represented creature into the test instance");
        let (grid_x, grid_y) = crate::map_manager::world_to_grid_coords(position.x, position.y);
        legacy.add_creature(0, 7, grid_x, grid_y, creature);
    }
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(tank_guid);
            let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
            combat.add_threat(tank_guid, 100.0);
            combat.add_threat(player_guid, 10.0);
        })
        .unwrap();
    session.sync_represented_creature_threat_to_canonical_like_cpp(
        creature_guid,
        player_guid,
        10.0,
    );
    session.set_spell_threat_store(Arc::new(wow_data::SpellThreatStoreLikeCpp {
        entries_by_spell_id: HashMap::from([(
            heal_spell_id as u32,
            wow_data::SpellThreatEntryLikeCpp {
                flat_mod: 4,
                pct_mod: 2.0,
                ap_pct_mod: 0.0,
            },
        )]),
    }));
    let mut heal_spell_store = wow_data::SpellStore::new();
    heal_spell_store.insert(
        heal_spell_id,
        threat_spell_info_like_cpp(
            heal_spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL,
            20,
        ),
    );
    session.set_spell_store(Arc::new(heal_spell_store));

    session
        .execute_spell(heal_spell_id, player_guid)
        .await
        .unwrap();

    let legacy_threat = manager
        .read()
        .unwrap()
        .find_creature(0, 7, creature_guid)
        .unwrap()
        .creature
        .unit()
        .subsystems()
        .combat
        .threat_value(player_guid);
    assert_eq!(legacy_threat, Some(38.0));
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 7, creature_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target,
        Some(tank_guid),
        "C++ heal threat does not bypass the regular victim-selection thresholds"
    );
    assert_eq!(
        session.canonical_creature_threat_value_like_cpp(creature_guid, player_guid),
        Some(38.0)
    );

    let no_helpful_threat_spell_id = 18_145;
    let mut no_helpful_threat_store = wow_data::SpellStore::new();
    let mut attributes = [0; 15];
    attributes[4] = wow_data::spell::attributes::SPELL_ATTR4_NO_HELPFUL_THREAT;
    no_helpful_threat_store
        .insert_spell_misc_attributes_like_cpp(no_helpful_threat_spell_id, attributes);
    session.set_spell_store(Arc::new(no_helpful_threat_store));
    session
        .apply_heal(Some(no_helpful_threat_spell_id), player_guid, 10)
        .await
        .unwrap();
    assert_eq!(session.player_health_like_cpp(), 80);
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 7, creature_guid)
            .unwrap()
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_value(player_guid),
        Some(38.0),
        "C++ ForwardThreatForAssistingMe returns before forwarding SPELL_ATTR4_NO_HELPFUL_THREAT heals"
    );
}
