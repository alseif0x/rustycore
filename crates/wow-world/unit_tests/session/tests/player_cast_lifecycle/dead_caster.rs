use super::*;

const DEAD_CAST_ID: i32 = 133;

fn install_dead_gate_spell(
    session: &mut WorldSession,
    spell: wow_data::SpellInfo,
    exact_difficulties: &[u8],
    misc_rows: &[(u8, u32)],
    misc_loaded: bool,
) {
    let spell_id = spell.spell_id;
    let exact_keys = exact_difficulties
        .iter()
        .map(|difficulty| (spell_id as u32, *difficulty))
        .collect::<Vec<_>>();
    let mut spell_store = crate::selected_spell_fixtures::canonical_spell_store(exact_keys);
    if misc_loaded {
        let rows = misc_rows
            .iter()
            .enumerate()
            .map(|(index, (difficulty_id, attributes0))| {
                let mut attributes = [0; 15];
                attributes[0] = *attributes0 as i32;
                wow_data::spell_db2::SpellMiscEntry {
                    id: spell_id as u32 + index as u32,
                    spell_id: spell_id as u32,
                    difficulty_id: *difficulty_id,
                    attributes,
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();
        spell_store = spell_store.hydrate_effective_core_db2_like_cpp(
            wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
                wow_data::spell_db2::SpellCategoriesStore::from_entries([]),
                wow_data::spell_db2::SpellMiscStore::from_entries(rows),
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
    }
    spell_store.insert(spell_id, spell);
    session.set_spell_store(Arc::new(spell_store));
}

fn spell_for_dead_gate(cast_time_ms: u32) -> wow_data::SpellInfo {
    let mut spell = minimal_spell();
    spell.spell_id = DEAD_CAST_ID;
    spell.cast_time_ms = cast_time_ms;
    spell
}

fn configure_dead_player(session: &mut WorldSession, state: wow_constants::DeathState) {
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .set_max_power(wow_constants::PowerType::Mana, 100);
            player
                .unit_mut()
                .set_power(wow_constants::PowerType::Mana, 100);
            player.unit_mut().set_health(0);
            player.unit_mut().set_death_state(state);
        })
        .expect("dead-caster tests retain the canonical Player owner");
}

fn configure_cast_visuals(session: &mut WorldSession) {
    session.set_creature_aggro_config_like_cpp(CreatureAggroConfigLikeCpp {
        spell_x_spell_visual_store: Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries(
            [],
        ))),
        ..Default::default()
    });
}

fn read_cast_failed_reason(bytes: &[u8]) -> i32 {
    let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::CastFailed as u16
    );
    packet.read_packed_guid().expect("cast ID");
    packet.read_int32().expect("spell ID");
    wow_packet::packets::spell::SpellCastVisual::read(&mut packet).expect("visual");
    packet.read_int32().expect("failure reason")
}

fn drain_frames(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<Vec<u8>> {
    std::iter::from_fn(|| send_rx.try_recv().ok()).collect()
}

#[tokio::test]
async fn world_dead_caster_client_and_server_dead_or_ghost_casts_fail_before_start_go_or_effects() {
    for (death_state, suffix) in [
        (wow_constants::DeathState::Dead, 0_i64),
        (wow_constants::DeathState::Corpse, 1_i64),
    ] {
        let (mut session, _, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 59_100 + suffix);
        install_canonical_player(
            &mut session,
            &canonical,
            player_guid,
            571,
            0,
            Position::ZERO,
        );
        configure_dead_player(&mut session, death_state);
        configure_cast_visuals(&mut session);
        session.set_known_spells_like_cpp(vec![DEAD_CAST_ID]);
        let mut spell = spell_for_dead_gate(0);
        spell.power_costs = vec![wow_data::SpellPowerCostInfoLikeCpp {
            order_index: 0,
            power_type: wow_constants::PowerType::Mana as i8,
            mana_cost: 20,
            mana_cost_per_level: 0,
            mana_per_second: 0,
            power_cost_pct: 0.0,
            power_cost_max_pct: 0.0,
            power_pct_per_second: 0.0,
            required_aura_spell_id: 0,
            optional_cost: 0,
        }];
        spell.effects = vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SELF_RESURRECT,
            effect_base_points: -25,
            ..Default::default()
        }];
        install_dead_gate_spell(&mut session, spell, &[0], &[], true);

        let mut pending = request(player_guid, 59_200 + suffix);
        pending.spell_id = DEAD_CAST_ID;
        assert!(crate::player_cast::request(&mut session, pending));
        session.tick_pending_spell_cast_request_like_cpp().await;

        let frames = drain_frames(&send_rx);
        assert_eq!(frames.len(), 2, "client mapping then one CheckCast result");
        assert_eq!(
            u16::from_le_bytes([frames[0][0], frames[0][1]]),
            ServerOpcodes::SpellPrepare as u16
        );
        assert_eq!(
            read_cast_failed_reason(&frames[1]),
            wow_constants::SpellCastResult::CasterDead as i32
        );
        assert!(session.active_spell_cast_snapshot_like_cpp().is_none());
        assert_eq!(
            session.with_owned_player_like_cpp(|player| {
                (
                    player.unit().death_state(),
                    player.unit().data().health,
                    player.get_power(wow_constants::PowerType::Mana),
                )
            }),
            Some((death_state, 0, 100)),
            "the rejected SelfRes effect does not resurrect, charge power or mutate health"
        );
        assert_eq!(
            drain_server_opcodes(&send_rx),
            Vec::<ServerOpcodes>::new(),
            "no Start, Go or effect publication follows CasterDead"
        );

        let (mut server_session, _, server_rx) = make_session();
        let server_map = shared_canonical_map_manager();
        let server_guid = ObjectGuid::create_player(1, 59_300 + suffix);
        install_canonical_player(
            &mut server_session,
            &server_map,
            server_guid,
            571,
            0,
            Position::ZERO,
        );
        configure_dead_player(&mut server_session, death_state);
        configure_cast_visuals(&mut server_session);
        install_dead_gate_spell(&mut server_session, spell_for_dead_gate(0), &[0], &[], true);
        let generators = server_session.id_generators_for_test_like_cpp();
        let creature_spawns = server_session.creature_spawn_catalogs_for_test_like_cpp();
        assert_eq!(
            wow_world_spell::try_cast_server_normal_player_spell_with_generator_like_cpp(
                &mut server_session,
                generators.item.as_ref(),
                &creature_spawns,
                DEAD_CAST_ID,
                server_guid,
                Default::default(),
            )
            .await,
            Ok(false)
        );
        let frames = drain_frames(&server_rx);
        assert_eq!(frames.len(), 1);
        assert_eq!(
            read_cast_failed_reason(&frames[0]),
            wow_constants::SpellCastResult::CasterDead as i32
        );
        assert!(
            server_session
                .active_spell_cast_snapshot_like_cpp()
                .is_none()
        );
        assert_eq!(
            server_session.with_owned_player_like_cpp(|player| {
                (player.unit().death_state(), player.unit().data().health)
            }),
            Some((death_state, 0))
        );
    }
}

#[tokio::test]
async fn world_dead_caster_allow_dead_self_res_uses_actual_map_spellinfo_key() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 59_400);
    session.set_player_guid(Some(player_guid));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map_with_difficulty(
        &canonical,
        player_guid,
        Position::ZERO,
        571,
        0,
        3,
    );
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    configure_dead_player(&mut session, wow_constants::DeathState::Dead);
    configure_cast_visuals(&mut session);
    session.set_known_spells_like_cpp(vec![DEAD_CAST_ID]);
    let mut spell = spell_for_dead_gate(1_500);
    spell.effects = vec![wow_data::SpellEffectInfo {
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SELF_RESURRECT,
        effect_base_points: -25,
        ..Default::default()
    }];
    install_dead_gate_spell(
        &mut session,
        spell,
        &[0, 3],
        &[(
            3,
            wow_data::spell::attributes::SPELL_ATTR0_ALLOW_CAST_WHILE_DEAD,
        )],
        true,
    );

    let mut pending = request(player_guid, 59_401);
    pending.spell_id = DEAD_CAST_ID;
    assert!(crate::player_cast::request(&mut session, pending));
    session.tick_pending_spell_cast_request_like_cpp().await;

    let active = session
        .active_spell_cast_snapshot_like_cpp()
        .expect("AllowWhileDead admits the timed normal SelfRes cast");
    assert_eq!(active.metadata.actual_spell_difficulty_id_like_cpp, Some(3));
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellPrepare, ServerOpcodes::SpellStart]
    );
    assert!(
        session
            .with_owned_player_like_cpp(|player| !player.unit().is_alive())
            .unwrap()
    );
}

#[tokio::test]
async fn world_dead_caster_late_checkcast_rejects_direct_spell_before_go() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 59_450);
    install_canonical_player(
        &mut session,
        &canonical,
        player_guid,
        571,
        0,
        Position::ZERO,
    );
    configure_dead_player(&mut session, wow_constants::DeathState::Dead);
    install_dead_gate_spell(&mut session, spell_for_dead_gate(0), &[0], &[], true);

    session
        .execute_spell(DEAD_CAST_ID, player_guid)
        .await
        .expect("late CheckCast publishes an ordinary dead-caster rejection");

    let frames = drain_frames(&send_rx);
    assert_eq!(frames.len(), 1);
    assert_eq!(
        read_cast_failed_reason(&frames[0]),
        wow_constants::SpellCastResult::CasterDead as i32
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        Vec::<ServerOpcodes>::new(),
        "late CheckCast runs before SpellGo"
    );
    assert_eq!(
        session.with_owned_player_like_cpp(|player| {
            (player.unit().death_state(), player.unit().data().health)
        }),
        Some((wow_constants::DeathState::Dead, 0))
    );
}

#[test]
fn dead_caster_gate_uses_source_words_full_mask_and_actual_typed_caster() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 59_500);
    install_canonical_player(
        &mut session,
        &canonical,
        player_guid,
        571,
        0,
        Position::ZERO,
    );
    configure_dead_player(&mut session, wow_constants::DeathState::Dead);
    let spell = spell_for_dead_gate(0);
    let cast_id = ObjectGuid::new(6, 59_501);
    let visual = wow_packet::packets::spell::SpellCastVisual::default();

    install_dead_gate_spell(&mut session, spell.clone(), &[0], &[], true);
    assert_eq!(
        wow_world_spell::check_represented_caster_death_like_cpp(
            &mut session,
            &spell,
            cast_id,
            &visual,
            &mut Default::default(),
        ),
        Ok(false),
        "a loaded Misc catalog with known absence supplies Source zero words"
    );
    let frames = drain_frames(&send_rx);
    assert_eq!(frames.len(), 1);
    assert_eq!(
        read_cast_failed_reason(&frames[0]),
        wow_constants::SpellCastResult::CasterDead as i32
    );

    install_dead_gate_spell(&mut session, spell.clone(), &[0], &[], false);
    assert!(
        wow_world_spell::check_represented_caster_death_like_cpp(
            &mut session,
            &spell,
            cast_id,
            &visual,
            &mut Default::default(),
        )
        .is_err()
    );
    assert!(
        send_rx.is_empty(),
        "unavailable Misc authority is not CasterDead"
    );

    let mut missing_creature = wow_entities::SpellCastMetadata {
        caster_guid_override: Some(test_creature_guid(59_509)),
        ..Default::default()
    };
    assert!(
        wow_world_spell::check_represented_caster_death_like_cpp(
            &mut session,
            &spell,
            cast_id,
            &visual,
            &mut missing_creature,
        )
        .is_err()
    );
    assert!(
        send_rx.is_empty(),
        "missing typed caster authority is not CasterDead"
    );

    for allowed_attributes in [
        wow_data::spell::attributes::SPELL_ATTR0_PASSIVE,
        wow_data::spell::attributes::SPELL_ATTR0_ALLOW_CAST_WHILE_DEAD,
    ] {
        install_dead_gate_spell(
            &mut session,
            spell.clone(),
            &[0],
            &[(0, allowed_attributes)],
            true,
        );
        assert_eq!(
            wow_world_spell::check_represented_caster_death_like_cpp(
                &mut session,
                &spell,
                cast_id,
                &visual,
                &mut Default::default(),
            ),
            Ok(true)
        );
    }
    install_dead_gate_spell(&mut session, spell.clone(), &[0], &[], false);
    let mut triggered = wow_entities::SpellCastMetadata {
        trigger_cast_flags_like_cpp: wow_entities::TRIGGERED_FULL_MASK_LIKE_CPP,
        ..Default::default()
    };
    assert_eq!(
        wow_world_spell::check_represented_caster_death_like_cpp(
            &mut session,
            &spell,
            cast_id,
            &visual,
            &mut triggered,
        ),
        Ok(true),
        "a direct full-mask trigger bypasses unavailable attributes without aura-origin state"
    );
    install_dead_gate_spell(&mut session, spell.clone(), &[0], &[], true);
    let mut arbitrary = wow_entities::SpellCastMetadata {
        trigger_cast_flags_like_cpp: 1 << 19,
        ..Default::default()
    };
    assert_eq!(
        wow_world_spell::check_represented_caster_death_like_cpp(
            &mut session,
            &spell,
            cast_id,
            &visual,
            &mut arbitrary,
        ),
        Ok(false),
        "bits outside TriggerCastFlags::FULL do not bypass the dead check"
    );
    let frames = drain_frames(&send_rx);
    assert_eq!(frames.len(), 1);
    assert_eq!(
        read_cast_failed_reason(&frames[0]),
        wow_constants::SpellCastResult::CasterDead as i32
    );

    let creature_guid = test_creature_guid(59_510);
    add_canonical_test_creature_on_map(
        &canonical,
        creature_guid,
        59_511,
        Position::ZERO,
        0,
        571,
        0,
    );
    let mut actual_creature = wow_entities::SpellCastMetadata {
        caster_guid_override: Some(creature_guid),
        ..Default::default()
    };
    // The session Player is dead, but this actual Creature caster is alive.
    assert_eq!(
        wow_world_spell::check_represented_caster_death_like_cpp(
            &mut session,
            &spell,
            cast_id,
            &visual,
            &mut actual_creature,
        ),
        Ok(true)
    );

    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .get_typed_creature_mut(creature_guid)
        .unwrap()
        .unit_mut()
        .set_death_state(wow_constants::DeathState::Dead);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_health(100);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Alive);
        })
        .unwrap();
    install_dead_gate_spell(&mut session, spell.clone(), &[0], &[], true);
    assert_eq!(
        wow_world_spell::check_represented_caster_death_like_cpp(
            &mut session,
            &spell,
            cast_id,
            &visual,
            &mut actual_creature,
        ),
        Ok(false),
        "a dead actual Creature rejects independently of the living session Player"
    );
    let frames = drain_frames(&send_rx);
    assert_eq!(frames.len(), 1);
    assert_eq!(
        read_cast_failed_reason(&frames[0]),
        wow_constants::SpellCastResult::CasterDead as i32
    );

    let vehicle_guid = ObjectGuid::create_vehicle_like_cpp(1, 571, 59_520, 59_521);
    add_canonical_test_creature_on_map(&canonical, vehicle_guid, 59_520, Position::ZERO, 0, 571, 0);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_health(0);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Dead);
        })
        .unwrap();
    let mut actual_vehicle = wow_entities::SpellCastMetadata {
        caster_guid_override: Some(vehicle_guid),
        ..Default::default()
    };
    assert_eq!(
        wow_world_spell::check_represented_caster_death_like_cpp(
            &mut session,
            &spell,
            cast_id,
            &visual,
            &mut actual_vehicle,
        ),
        Ok(true),
        "a live actual Vehicle caster is a Unit even while the session Player is dead"
    );
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .get_typed_creature_mut(vehicle_guid)
        .unwrap()
        .unit_mut()
        .set_death_state(wow_constants::DeathState::Dead);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_health(100);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Alive);
        })
        .unwrap();
    assert_eq!(
        wow_world_spell::check_represented_caster_death_like_cpp(
            &mut session,
            &spell,
            cast_id,
            &visual,
            &mut actual_vehicle,
        ),
        Ok(false),
        "a dead actual Vehicle caster is rejected independently of the living Player"
    );
    let vehicle_failed = drain_frames(&send_rx);
    assert_eq!(vehicle_failed.len(), 1);
    assert_eq!(
        read_cast_failed_reason(&vehicle_failed[0]),
        wow_constants::SpellCastResult::CasterDead as i32
    );

    let non_unit = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::GameObject,
        0,
        1,
        571,
        0,
        59_512,
        59_513,
    );
    let mut non_unit_caster = wow_entities::SpellCastMetadata {
        caster_guid_override: Some(non_unit),
        ..Default::default()
    };
    assert_eq!(
        wow_world_spell::check_represented_caster_death_like_cpp(
            &mut session,
            &spell,
            cast_id,
            &visual,
            &mut non_unit_caster,
        ),
        Ok(true),
        "a known non-Unit caster is outside this Source condition"
    );

    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_health(0);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Alive);
        })
        .unwrap();
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
    let mut no_attributes_needed = wow_entities::SpellCastMetadata::default();
    assert_eq!(
        wow_world_spell::check_represented_caster_death_like_cpp(
            &mut session,
            &spell,
            cast_id,
            &visual,
            &mut no_attributes_needed,
        ),
        Ok(true),
        "native Alive with health zero short-circuits before SpellMisc authority"
    );
}
