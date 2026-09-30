use super::*;

/// C++ `Spell::EffectPowerDrain` logs the drained power even when the pool is
/// empty and still calls `EnergizeBySpell` for a non-self caster
/// (`SpellEffects.cpp:1090-1101`), so both logs carry zeros rather than being
/// skipped.
#[tokio::test]
async fn spell_power_drain_on_an_empty_creature_pool_logs_zero_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 90_304_i32;
    let player_guid = ObjectGuid::create_player(1, 907);
    let creature_guid = test_creature_guid(19_304);
    let position = Position::new(10.0, 20.0, 30.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Drainer".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
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
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_power_index(PowerType::Mana, Some(0));
            player.unit_mut().set_max_power(PowerType::Mana, 200);
            player.unit_mut().set_power(PowerType::Mana, 25);
            player.clear_data_changes();
        })
        .unwrap();
    session
        .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
            let unit = creature.unit_mut();
            unit.set_power_index(PowerType::Mana, Some(0));
            unit.set_max_power(PowerType::Mana, 100);
            unit.set_power(PowerType::Mana, 0);
            unit.set_display_power(PowerType::Mana);
            creature.clear_data_changes();
        })
        .unwrap();

    let mut spell = power_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
        15,
        PowerType::Mana,
    );
    spell.effects[0].effect_amplitude = 0.5;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, spell);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("an empty represented creature pool is still a valid drain target");

    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
            .unwrap(),
        25,
        "a zero drain restores nothing"
    );
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(
        packets
            .iter()
            .map(|bytes| {
                wow_packet::WorldPacket::from_bytes(bytes)
                    .server_opcode()
                    .expect("server opcode")
            })
            .collect::<Vec<_>>(),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellEnergizeLog,
            ServerOpcodes::SpellExecuteLog,
            ServerOpcodes::CooldownEvent
        ],
        "C++ still publishes the zeroed energize log and the take-power row"
    );
    let log_bytes = packets
        .iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::SpellEnergizeLog)
        })
        .expect("energize log packet");
    let mut energize = wow_packet::WorldPacket::from_bytes(log_bytes);
    energize.read_uint16().expect("opcode");
    energize.read_packed_guid().expect("target");
    energize.read_packed_guid().expect("caster");
    energize.read_int32().expect("spell id");
    energize.read_int32().expect("power type");
    assert_eq!(
        energize.read_int32().expect("amount"),
        0,
        "C++ `EnergizeBySpell` publishes the zero gain"
    );
    assert_eq!(energize.read_int32().expect("over energize"), 0);
}

/// C++ `Spell::EffectPowerDrain` pre-scales the effect amount with
/// `Unit::SpellDamageBonusDone(..., SPELL_DIRECT_DAMAGE, ...)`
/// (`SpellEffects.cpp:1082-1088`) before draining, so the spell-power
/// coefficient grows the drained pool and the caster's share.
#[tokio::test]
async fn spell_power_drain_pre_scales_with_spell_damage_bonus_done_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 90_305_i32;
    let player_guid = ObjectGuid::create_player(1, 908);
    let creature_guid = test_creature_guid(19_305);
    let position = Position::new(10.0, 20.0, 30.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Drainer".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
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
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_power_index(PowerType::Mana, Some(0));
            player.unit_mut().set_max_power(PowerType::Mana, 200);
            player.unit_mut().set_power(PowerType::Mana, 25);
            // `GetBaseSpellPowerBonus()` 100 for the coefficient term.
            player.replace_effective_combat_stats_like_cpp(
                wow_entities::PlayerEffectiveCombatStatsLikeCpp {
                    spell_power: 100,
                    ..Default::default()
                },
            );
            player.clear_data_changes();
        })
        .unwrap();
    session
        .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
            let unit = creature.unit_mut();
            unit.set_power_index(PowerType::Mana, Some(0));
            unit.set_max_power(PowerType::Mana, 200);
            unit.set_power(PowerType::Mana, 200);
            unit.set_display_power(PowerType::Mana);
            creature.clear_data_changes();
        })
        .unwrap();
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        wow_data::SpellMiscEntry {
            id: spell_id as u32,
            spell_id: spell_id as u32,
            // Holy.
            school_mask: 1 << 1,
            ..Default::default()
        },
    ])));

    let mut spell = power_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
        100,
        PowerType::Mana,
    );
    // `SpellEffectInfo::BonusCoefficient` and the drain value multiplier.
    spell.effect_bonus_coefficient = 0.5;
    spell.effects[0].effect_amplitude = 0.5;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, spell);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("represented creature EffectPowerDrain should execute");

    assert_eq!(
        session
            .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| creature
                .unit()
                .get_power(PowerType::Mana))
            .unwrap(),
        50,
        "C++ drains `int32(100 + int32(100 * 0.5)) = 150` from the pool"
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
            .unwrap(),
        100,
        "the caster share is `int32(150 * 0.5) = 75`"
    );
}
