use super::*;

/// C++ `Spell::EffectApplyAura` applies to `unitTarget`: a debuff cast on a
/// creature lands on that creature (registered with its effect type, amount and
/// misc value), publishes `SMSG_AURA_UPDATE`, never touches the caster, and
/// expires when its represented duration elapses.
#[tokio::test]
async fn spell_apply_aura_on_creature_target_lands_on_the_creature_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 795_i32;
    let player_guid = ObjectGuid::create_player(1, 795);
    let creature_guid = test_creature_guid(18_795);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let manager = shared_map_manager();
    let canonical = Arc::clone(&manager);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Debuffer".to_string(),
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
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);

    let mut spell_store = wow_data::SpellStore::new();
    let mut debuff = threat_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        7,
    );
    debuff.effects[0].effect_aura =
        wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN;
    debuff.effects[0].effect_misc_value_1 = 0x01;
    let mut detect = debuff.clone();
    detect.spell_id = spell_id + 1;
    detect.effects[0].effect_aura = wow_data::spell::aura_types::SPELL_AURA_MOD_DETECT_RANGE;
    detect.effects[0].effect_misc_value_1 = 0;
    spell_store.insert(spell_id, debuff);
    spell_store.insert(spell_id + 1, detect);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("represented creature aura should apply");

    // The creature owns the aura with its represented effect data...
    let canonical_effects = session
        .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
            let auras = &creature.unit().subsystems().auras;
            let applied: Vec<_> = auras
                .applied_auras
                .iter()
                .filter(|aura| aura.spell_id == u32::try_from(spell_id).unwrap())
                .map(|aura| (aura.caster_guid, aura.effect_mask))
                .collect();
            let typed = auras.total_aura_modifier_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
            );
            (applied, typed)
        })
        .expect("canonical creature");
    assert_eq!(canonical_effects.0, vec![(player_guid, 1)]);
    assert_eq!(canonical_effects.1, 7);
    // ...the caster keeps none of it...
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .values()
                .any(|aura| aura.spell_id == spell_id)
        }),
        Some(false)
    );
    // ...and the client is told through `SMSG_AURA_UPDATE`.
    let packets = drain_server_packet_bytes(&send_rx);
    let opcodes: Vec<_> = packets
        .iter()
        .map(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes)
                .server_opcode()
                .expect("server opcode")
        })
        .collect();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::AuraUpdate,
            ServerOpcodes::CooldownEvent
        ]
    );
    // The `AuraUpdate` payload itself is covered by the packet writer's own
    // test; here the canonical registration and the sent opcode prove the
    // publication happened for the creature.

    // The registered effect data also feeds the other creature-aura consumers:
    // a detect-range aura changes the creature's own aggro modifier.
    session
        .execute_spell(spell_id + 1, creature_guid)
        .await
        .expect("represented detect-range aura should apply");
    let detect_range = session
        .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
            creature
                .unit()
                .subsystems()
                .auras
                .total_aura_modifier_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_DETECT_RANGE,
                )
        })
        .expect("canonical creature");
    assert_eq!(detect_range, 7);

    // C++ `Creature::Update` expires the aura when its duration elapses.
    let _ = drain_server_packet_bytes(&send_rx);
    session
        .world_entities
        .set_represented_creature_auras_applied_at_for_test_like_cpp(|| {
            std::time::Instant::now() - std::time::Duration::from_secs(60)
        });
    wow_world_spell::tick_auras(&mut session);
    let remaining = session
        .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
            creature
                .unit()
                .subsystems()
                .auras
                .applied_auras
                .iter()
                .filter(|aura| aura.caster_guid == player_guid)
                .count()
        })
        .expect("canonical creature");
    assert_eq!(remaining, 0, "the elapsed duration removes the auras");
    assert!(
        session
            .world_entities
            .represented_creature_auras_are_empty_like_cpp()
    );
    let after_expiry = drain_server_opcodes(&send_rx);
    let removals = after_expiry
        .iter()
        .filter(|opcode| **opcode == ServerOpcodes::AuraUpdate)
        .count();
    assert_eq!(
        removals, 2,
        "each expired aura publishes its removal: {after_expiry:?}"
    );
}

#[tokio::test]
async fn creature_aura_without_original_caster_keeps_caster_override_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 798_i32;
    let player_guid = ObjectGuid::create_player(1, 798);
    let creature_guid = test_creature_guid(18_798);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let canonical = shared_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "CreatureAuraOverride".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 0);
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
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
        .expect("canonical map manager");
    register_test_creature(&mut session, manager, creature_guid, 100);

    let mut aura = threat_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        7,
    );
    aura.effects[0].effect_aura = wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN;
    let mut spell_store =
        crate::selected_spell_fixtures::canonical_spell_store([(spell_id as u32, 0)]);
    spell_store.insert(spell_id, aura);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            creature_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData::default(),
            SpellCastMetadata {
                caster_guid_override: Some(creature_guid),
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("creature-caster aura should apply");

    let aura_caster = session
        .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
            creature
                .unit()
                .subsystems()
                .auras
                .applied_auras
                .iter()
                .find(|applied| applied.spell_id == spell_id as u32)
                .map(|applied| applied.caster_guid)
        })
        .flatten()
        .expect("creature aura is registered");
    assert_eq!(aura_caster, creature_guid);

    let bytes = send_rx.try_recv().expect("creature-cast SpellGo");
    let mut spell_go = wow_packet::WorldPacket::from_bytes(&bytes);
    assert_eq!(
        spell_go.read_uint16().expect("SpellGo opcode"),
        ServerOpcodes::SpellGo as u16
    );
    assert_eq!(
        spell_go.read_packed_guid().expect("visible caster"),
        creature_guid
    );
}

/// C++ `Aura::Create` builds one `AuraEffect` per applied slot, so a
/// creature-target aura with two effect slots answers `GetAuraEffectsByType`
/// with each slot's own amount and misc value instead of one shared amount.
#[tokio::test]
async fn spell_apply_aura_on_creature_keeps_each_effect_slot_amount_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 796_i32;
    let player_guid = ObjectGuid::create_player(1, 796);
    let creature_guid = test_creature_guid(18_796);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let manager = shared_map_manager();
    let canonical = Arc::clone(&manager);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Debuffer".to_string(),
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
    let mut debuff = threat_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        7,
    );
    debuff.effects[0].effect_aura =
        wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN;
    debuff.effects[0].effect_misc_value_1 = 0x01;
    debuff.effects.push(wow_data::SpellEffectInfo {
        effect_index: 1,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_base_points: -30,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_MELEE_HASTE,
        effect_misc_value_1: 0,
        ..Default::default()
    });
    spell_store.insert(spell_id, debuff);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("represented creature aura should apply");

    let registered = session
        .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
            let auras = &creature.unit().subsystems().auras;
            let masks: Vec<_> = auras
                .applied_auras
                .iter()
                .filter(|aura| aura.spell_id == u32::try_from(spell_id).unwrap())
                .map(|aura| aura.effect_mask)
                .collect();
            (
                masks,
                auras.total_aura_modifier_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
                ),
                auras.total_aura_modifier_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_MELEE_HASTE,
                ),
                auras
                    .visible_aura_applications_like_cpp
                    .values()
                    .flat_map(|application| application.effect_amounts.clone())
                    .map(|effect| (effect.effect_index, effect.amount))
                    .collect::<Vec<_>>(),
            )
        })
        .expect("canonical creature");
    assert_eq!(
        registered.0,
        vec![1, 2],
        "one per-slot AppliedAuraRef per AuraEffect, as the pet-load and threat paths key them"
    );
    assert_eq!(registered.1, 7, "the first slot keeps its own amount");
    assert_eq!(
        registered.2, -30,
        "the second slot keeps its own amount instead of the first slot's"
    );
    assert_eq!(
        registered.3,
        vec![(0, 7), (1, -30)],
        "the visible application carries both slot amounts"
    );

    // The elapsed duration removes every slot of the application, not only the
    // last registered one.
    let _ = drain_server_packet_bytes(&send_rx);
    session
        .world_entities
        .set_represented_creature_auras_applied_at_for_test_like_cpp(|| {
            std::time::Instant::now() - std::time::Duration::from_secs(60)
        });
    wow_world_spell::tick_auras(&mut session);
    let remaining = session
        .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
            creature
                .unit()
                .subsystems()
                .auras
                .applied_auras
                .iter()
                .filter(|aura| aura.caster_guid == player_guid)
                .count()
        })
        .expect("canonical creature");
    assert_eq!(remaining, 0, "the elapsed duration removes both slots");
}
