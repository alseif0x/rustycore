use super::*;

#[tokio::test]
async fn spell_threat_effect_adds_creature_threat_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 787_i32;
    let player_guid = ObjectGuid::create_player(1, 787);
    let creature_guid = test_creature_guid(18_787);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "ThreatCaster".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 0);
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9001,
        position,
        0,
        0,
        80,
    );
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        threat_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_THREAT,
            35,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("represented EffectThreat should execute");

    let legacy_threat = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .creature
        .unit()
        .subsystems()
        .combat
        .threat_value(player_guid);
    assert_eq!(legacy_threat, Some(35.0));
    assert_eq!(
        session.canonical_creature_threat_value_like_cpp(creature_guid, player_guid),
        Some(35.0)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}

#[tokio::test]
async fn spell_modify_threat_percent_scales_existing_creature_threat_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 788_i32;
    let player_guid = ObjectGuid::create_player(1, 788);
    let creature_guid = test_creature_guid(18_788);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "ThreatScaler".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 0);
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9001,
        position,
        0,
        0,
        80,
    );
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(player_guid, 120.0);
        })
        .unwrap();
    session.sync_represented_creature_threat_to_canonical_like_cpp(
        creature_guid,
        player_guid,
        120.0,
    );
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        threat_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_THREAT_PERCENT,
            -50,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("represented EffectModifyThreatPercent should execute");

    let legacy_threat = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .creature
        .unit()
        .subsystems()
        .combat
        .threat_value(player_guid);
    assert_eq!(legacy_threat, Some(60.0));
    assert_eq!(
        session.canonical_creature_threat_value_like_cpp(creature_guid, player_guid),
        Some(60.0)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}

#[tokio::test]
async fn spell_taunt_effect_matches_caster_threat_to_highest_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 793_i32;
    let player_guid = ObjectGuid::create_player(1, 793);
    let other_player_guid = ObjectGuid::create_player(1, 1793);
    let creature_guid = test_creature_guid(18_793);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let registry = Arc::new(PlayerRegistry::default());
    let (observer_tx, _observer_rx) = flume::bounded(4);
    let (observer_command_tx, observer_command_rx) = flume::bounded(4);
    let observer_guid = ObjectGuid::create_player(1, 2793);
    let mut observer = broadcast_info_with_command(observer_guid, observer_tx, observer_command_tx);
    observer.placement.position = position;
    registry.register_or_replace(observer_guid, observer, Default::default());
    session.set_player_registry(registry);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TauntCaster".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 0);
    add_canonical_test_player_on_map(&canonical, observer_guid, position, 0, 0);
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9001,
        position,
        0,
        0,
        80,
    );
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    session
        .mutate_world_creature(creature_guid, |creature| {
            let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
            combat.add_threat(other_player_guid, 120.0);
            combat.add_threat(player_guid, 5.0);
        })
        .unwrap();
    let mut spell_store = wow_data::SpellStore::new();
    let mut taunt_spell = threat_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        0,
    );
    taunt_spell.effects[0].effect_aura = wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT;
    spell_store.insert(spell_id, taunt_spell);
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        wow_data::SpellMiscEntry {
            id: spell_id as u32,
            duration_index: 7,
            spell_id: spell_id as u32,
            ..Default::default()
        },
    ])));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 7,
            duration: 3_000,
            duration_per_level: 0,
            max_duration: 3_000,
        },
    ])));

    let cast_id = {
        let counter = canonical
            .lock()
            .unwrap()
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .generate_low_guid_like_cpp(HighGuid::Cast)
            .expect("the represented player cast must allocate its Map-owned CastID");
        represented_spell_cast_guid_for_map_like_cpp(session.realm_id(), 0, spell_id, counter)
    };
    session
        .execute_spell_with_visual(
            spell_id,
            creature_guid,
            cast_id,
            wow_packet::packets::spell::SpellCastVisual::default(),
        )
        .await
        .expect("represented EffectTaunt should execute");

    let (taunt_provenance, next_cast_counter) = {
        let mut manager = canonical.lock().unwrap();
        let map = manager.find_map_mut(0, 0).unwrap().map_mut();
        let provenance = map
            .with_creature_like_cpp(creature_guid, |creature| {
                let auras = &creature.unit().subsystems().auras;
                let slot = auras
                    .visible_auras
                    .iter()
                    .find_map(|(slot, aura)| (aura.spell_id == spell_id as u32).then_some(*slot))
                    .expect("taunt aura slot");
                auras.aura_cast_provenance_like_cpp(slot)
            })
            .expect("canonical creature");
        let next = map
            .get_max_low_guid_like_cpp(HighGuid::Cast)
            .expect("Cast sequence");
        (provenance, next)
    };
    assert_eq!(taunt_provenance.cast_id.high_type(), HighGuid::Cast);
    assert_eq!(taunt_provenance.cast_id.entry(), spell_id as u32);
    assert_eq!(taunt_provenance.spell_visual_id, 0);
    assert_eq!(
        next_cast_counter, 2,
        "the taunt Aura retains its parent Spell Cast ID without allocating a second ID"
    );

    let manager_guard = manager.read().unwrap();
    let combat = &manager_guard
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .creature
        .unit()
        .subsystems()
        .combat;
    let legacy_threat = combat.threat_value(player_guid);
    assert_eq!(legacy_threat, Some(120.0));
    assert!(
        combat
            .threat_ref(player_guid)
            .is_some_and(wow_entities::ThreatReferenceState::is_taunting),
        "C++ MOD_TAUNT forces the caster above ordinary threat for the aura duration"
    );
    drop(manager_guard);
    assert_eq!(
        session.canonical_creature_threat_value_like_cpp(creature_guid, player_guid),
        Some(120.0)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::AuraUpdate,
            ServerOpcodes::CooldownEvent
        ]
    );
    let observer_command = observer_command_rx
        .try_recv()
        .expect("nearby sessions receive the taunt AuraUpdate command");
    let SessionCommand::SendIfVisibleLikeCpp(observer_command) = observer_command else {
        panic!("expected visibility-gated taunt fanout");
    };
    assert_eq!(
        u16::from_le_bytes([
            observer_command.packet_bytes[0],
            observer_command.packet_bytes[1],
        ]),
        ServerOpcodes::AuraUpdate as u16
    );
}

#[tokio::test]
async fn spell_threat_effect_skips_dead_caster_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 789_i32;
    let player_guid = ObjectGuid::create_player(1, 789);
    let creature_guid = test_creature_guid(18_789);
    let manager = shared_map_manager();
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(0, 100);
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        threat_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_THREAT,
            35,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("dead caster EffectThreat should execute as C++ no-op");

    let legacy_threat = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .creature
        .unit()
        .subsystems()
        .combat
        .threat_value(player_guid);
    assert_eq!(legacy_threat, None);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
