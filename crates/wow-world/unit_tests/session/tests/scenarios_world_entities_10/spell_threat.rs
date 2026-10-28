use super::*;

#[tokio::test]
async fn spell_threat_effect_adds_creature_threat_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 787_i32;
    let player_guid = ObjectGuid::create_player(1, 787);
    let creature_guid = test_creature_guid(18_787);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let manager = shared_map_manager();
    let canonical = Arc::clone(&manager);
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
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
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
        .lock()
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
    let canonical = Arc::clone(&manager);
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
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
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
        .lock()
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
    for (duration, expected_duration_bits) in [(3_000, 3_000_u32), (-1, u32::MAX)] {
        let (mut session, _, send_rx) = make_session();
        let spell_id = 793_i32;
        let player_guid = ObjectGuid::create_player(1, 793);
        let other_player_guid = ObjectGuid::create_player(1, 1793);
        let creature_guid = test_creature_guid(18_793);
        let position = Position::new(10.0, 10.0, 0.0, 0.0);
        let manager = shared_map_manager();
        let canonical = Arc::clone(&manager);
        let registry = Arc::new(PlayerRegistry::default());
        let (observer_tx, _observer_rx) = flume::bounded(4);
        let (observer_command_tx, observer_command_rx) = flume::bounded(4);
        let observer_guid = ObjectGuid::create_player(1, 2793);
        let mut observer =
            broadcast_info_with_command(observer_guid, observer_tx, observer_command_tx);
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
        let manager = session
            .core
            .canonical_map_manager
            .clone()
            .unwrap_or_else(|| manager.clone());
        register_test_creature(&mut session, manager.clone(), creature_guid, 100);
        session
            .mutate_world_creature(creature_guid, |creature| {
                let combat = &mut creature.unit_mut().subsystems_mut().combat;
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
                duration,
                duration_per_level: 0,
                max_duration: duration,
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
                        .find_map(|(slot, aura)| {
                            (aura.spell_id == spell_id as u32).then_some(*slot)
                        })
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

        let manager_guard = manager.lock().unwrap();
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
        let packets = drain_server_packet_bytes(&send_rx);
        let opcodes = packets
            .iter()
            .filter_map(|bytes| WorldPacket::from_bytes(bytes).server_opcode())
            .collect::<Vec<_>>();
        assert_eq!(
            opcodes,
            vec![
                ServerOpcodes::SpellGo,
                ServerOpcodes::AuraUpdate,
                ServerOpcodes::CooldownEvent
            ]
        );
        let aura_update = packets
            .iter()
            .find(|bytes| {
                WorldPacket::from_bytes(bytes).server_opcode() == Some(ServerOpcodes::AuraUpdate)
            })
            .expect("Taunt AuraUpdate frame");
        let mut frame = WorldPacket::from_bytes(aura_update);
        frame.skip_opcode();
        assert!(!frame.read_bit().expect("Taunt update-all bit"));
        assert_eq!(frame.read_bits(9).expect("AuraInfo count"), 1);
        let _slot = frame.read_uint8().expect("Taunt AuraInfo slot");
        assert!(frame.read_bit().expect("Taunt AuraData bit"));
        frame.reset_bits();
        let _cast_id = frame.read_packed_guid().expect("Taunt CastID");
        assert_eq!(frame.read_int32().expect("Taunt spell ID"), spell_id);
        let _spell_visual_id = frame.read_int32().expect("Taunt spell visual ID");
        let _flags = frame.read_uint16().expect("Taunt aura flags");
        let _active_flags = frame.read_uint32().expect("Taunt active flags");
        let _cast_level = frame.read_uint16().expect("Taunt cast level");
        let _applications = frame.read_uint8().expect("Taunt applications");
        let _content_tuning_id = frame.read_int32().expect("Taunt content tuning ID");
        let has_caster = frame.read_bit().expect("Taunt caster bit");
        let has_duration = frame.read_bit().expect("Taunt duration bit");
        let has_remaining = frame.read_bit().expect("Taunt remaining bit");
        let _has_time_mod = frame.read_bit().expect("Taunt time-mod bit");
        let _point_count = frame.read_bits(6).expect("Taunt points count");
        let _estimated_point_count = frame.read_bits(6).expect("Taunt estimated points count");
        let _has_content_tuning = frame.read_bit().expect("Taunt content-tuning bit");
        frame.reset_bits();
        if has_caster {
            let _caster = frame.read_packed_guid().expect("Taunt caster GUID");
        }
        assert!(has_duration, "legacy Taunt AuraInfo includes Duration");
        assert!(has_remaining, "legacy Taunt AuraInfo includes Remaining");
        assert_eq!(
            frame.read_uint32().expect("Taunt Duration bits"),
            expected_duration_bits
        );
        assert_eq!(
            frame.read_uint32().expect("Taunt Remaining bits"),
            expected_duration_bits
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
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    let mut attributes = [0; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_ALLOW_CAST_WHILE_DEAD as i32;
    let misc = wow_data::spell_db2::SpellMiscEntry {
        id: spell_id as u32,
        spell_id: spell_id as u32,
        attributes,
        ..Default::default()
    };
    let mut spell_store =
        crate::selected_spell_fixtures::canonical_spell_store([(spell_id as u32, 0)])
            .hydrate_effective_core_db2_like_cpp(
                wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
                    wow_data::spell_db2::SpellCategoriesStore::from_entries([]),
                    wow_data::spell_db2::SpellMiscStore::from_entries([misc]),
                    wow_data::spell_db2::SpellEffectDb2Store::from_entries([]),
                    wow_data::spell_db2::SpellShapeshiftStore::from_entries([]),
                    wow_data::spell_db2::SpellInterruptsStore::from_entries([]),
                    wow_data::spell_db2::SpellCastTimesStore::from_entries([]),
                    wow_data::spell_db2::SpellCooldownsStore::from_entries([]),
                    wow_data::spell_db2::SpellCastingRequirementsStore::from_entries([]),
                    wow_data::spell_db2::SpellPowerStore::from_entries([]),
                    wow_data::spell_db2::SpellPowerDifficultyStore::from_entries([]),
                ),
            );
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
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            creature_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            Default::default(),
            wow_entities::SpellCastMetadata {
                actual_spell_difficulty_id_like_cpp: Some(0),
                ..Default::default()
            },
        )
        .await
        .expect("dead caster EffectThreat should execute as C++ no-op");

    let legacy_threat = manager
        .lock()
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
