//! Spell-state regressions, part 26.
//!
//! Split out of `scenarios_spell_state_12.rs` when that module reached the
//! terminal file limit; the tests are unchanged.

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

/// C++ `Unit::SpellDamageBonusTaken` (`Unit.cpp:6775-6820`) multiplies the
/// pre-scaled amount by the victim's
/// `SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN` for the spell's school, unless the
/// spell carries `SPELL_ATTR4_IGNORE_DAMAGE_TAKEN_MODIFIERS`.
#[tokio::test]
async fn spell_power_drain_applies_the_victim_school_damage_taken_aura_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 90_306_i32;
    let aura_spell_id = 90_307_i32;
    let player_guid = ObjectGuid::create_player(1, 909);
    let creature_guid = test_creature_guid(19_306);
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
            unit.set_max_power(PowerType::Mana, 200);
            unit.set_power(PowerType::Mana, 200);
            unit.set_display_power(PowerType::Mana);
            creature.clear_data_changes();
        })
        .unwrap();

    // A +50% damage-taken aura for the normal school (misc bit 0), applied the
    // way the round-105 creature-aura path does.
    let mut aura_spell = power_spell_info_like_cpp(
        aura_spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        50,
        PowerType::Mana,
    );
    aura_spell.effects[0].effect_aura =
        wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN;
    aura_spell.effects[0].effect_misc_value_1 = 0x01;
    let mut spell = power_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
        100,
        PowerType::Mana,
    );
    spell.effects[0].effect_amplitude = 0.5;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(aura_spell_id, aura_spell);
    spell_store.insert(spell_id, spell);
    session.set_spell_store(Arc::new(spell_store));

    session
        .apply_creature_aura_like_cpp(aura_spell_id, player_guid, creature_guid, 1, 30_000)
        .expect("represented creature damage-taken aura should apply");

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
        "C++ drains `int32(100 * 1.5) = 150` with a +50% damage-taken aura"
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
            .unwrap(),
        100,
        "the caster share is `int32(150 * 0.5) = 75`"
    );
}

/// C++ `Unit::SpellDamageBonusTaken` (`Unit.cpp:6775-6840`) accumulates the
/// school, mechanic and caster terms multiplicatively into one `TakenTotalMod`
/// that scales the amount once, so two +50% terms yield `225` from a base of
/// `100`, not `200`.
#[tokio::test]
async fn spell_power_drain_stacks_damage_taken_terms_multiplicatively_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 90_315_i32;
    let school_aura_id = 90_316_i32;
    let mechanic_aura_id = 90_317_i32;
    let player_guid = ObjectGuid::create_player(1, 914);
    let creature_guid = test_creature_guid(19_311);
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
            unit.set_max_power(PowerType::Mana, 400);
            unit.set_power(PowerType::Mana, 400);
            unit.set_display_power(PowerType::Mana);
            creature.clear_data_changes();
        })
        .unwrap();

    let mut school_aura = power_spell_info_like_cpp(
        school_aura_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        50,
        PowerType::Mana,
    );
    school_aura.effects[0].effect_aura =
        wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN;
    school_aura.effects[0].effect_misc_value_1 = 0x01;
    let mut mechanic_aura = power_spell_info_like_cpp(
        mechanic_aura_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        50,
        PowerType::Mana,
    );
    mechanic_aura.effects[0].effect_aura =
        wow_data::spell::aura_types::SPELL_AURA_MOD_MECHANIC_DAMAGE_TAKEN_PERCENT;
    mechanic_aura.effects[0].effect_misc_value_1 = 1 << 5;
    let mut spell = power_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
        100,
        PowerType::Mana,
    );
    spell.effects[0].effect_amplitude = 0.5;
    spell.effects[0].effect_mechanic = 5;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(school_aura_id, school_aura);
    spell_store.insert(mechanic_aura_id, mechanic_aura);
    spell_store.insert(spell_id, spell);
    session.set_spell_store(Arc::new(spell_store));

    session
        .apply_creature_aura_like_cpp(school_aura_id, player_guid, creature_guid, 1, 30_000)
        .expect("represented creature school aura should apply");
    session
        .apply_creature_aura_like_cpp(mechanic_aura_id, player_guid, creature_guid, 1, 30_000)
        .expect("represented creature mechanic aura should apply");

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
        175,
        "C++ multiplies the terms: `int32(100 * 1.5 * 1.5) = 225`"
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
            .unwrap(),
        137,
        "the caster share is `int32(225 * 0.5) = 112`"
    );
}

/// C++ cheat-death term of `Unit::SpellDamageBonusTaken` (`Unit.cpp:6793-6795`):
/// aura spell `45182` adds its amount as a percentage when its misc value
/// intersects `SPELL_SCHOOL_MASK_NORMAL`.
#[tokio::test]
async fn spell_power_drain_applies_the_cheat_death_taken_term_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 90_308_i32;
    let cheat_death_spell_id = 45_182_i32;
    let player_guid = ObjectGuid::create_player(1, 910);
    let creature_guid = test_creature_guid(19_307);
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
            unit.set_max_power(PowerType::Mana, 200);
            unit.set_power(PowerType::Mana, 200);
            unit.set_display_power(PowerType::Mana);
            creature.clear_data_changes();
        })
        .unwrap();

    // `SPELL_AURA_MOD_DAMAGE_DONE` so the school damage-taken term stays 1.0.
    let mut cheat_death = power_spell_info_like_cpp(
        cheat_death_spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        20,
        PowerType::Mana,
    );
    cheat_death.effects[0].effect_aura = wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE;
    cheat_death.effects[0].effect_misc_value_1 = 0x01;
    let mut spell = power_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
        100,
        PowerType::Mana,
    );
    spell.effects[0].effect_amplitude = 0.5;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(cheat_death_spell_id, cheat_death);
    spell_store.insert(spell_id, spell);
    session.set_spell_store(Arc::new(spell_store));

    session
        .apply_creature_aura_like_cpp(cheat_death_spell_id, player_guid, creature_guid, 1, 30_000)
        .expect("represented creature cheat-death aura should apply");

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
        80,
        "C++ drains `int32(100 * 1.2) = 120` with the cheat-death term"
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
            .unwrap(),
        85,
        "the caster share is `int32(120 * 0.5) = 60`"
    );
}

/// C++ `Unit::SpellDamageBonusTaken` (`Unit.cpp:6815-6822`): the damage taken is
/// also multiplied by the caster's
/// `SPELL_AURA_MOD_SCHOOL_MASK_DAMAGE_FROM_CASTER` auras whose misc value
/// intersects the spell's school.
#[tokio::test]
async fn spell_power_drain_applies_the_caster_school_damage_taken_aura_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 90_309_i32;
    let aura_spell_id = 90_310_i32;
    let player_guid = ObjectGuid::create_player(1, 911);
    let creature_guid = test_creature_guid(19_308);
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
            unit.set_max_power(PowerType::Mana, 200);
            unit.set_power(PowerType::Mana, 200);
            unit.set_display_power(PowerType::Mana);
            creature.clear_data_changes();
        })
        .unwrap();

    let mut aura_spell = power_spell_info_like_cpp(
        aura_spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        50,
        PowerType::Mana,
    );
    aura_spell.effects[0].effect_aura =
        wow_data::spell::aura_types::SPELL_AURA_MOD_SCHOOL_MASK_DAMAGE_FROM_CASTER;
    // Misc bit 0 is the normal school the drain spell defaults to.
    aura_spell.effects[0].effect_misc_value_1 = 0x01;
    let mut spell = power_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
        100,
        PowerType::Mana,
    );
    spell.effects[0].effect_amplitude = 0.5;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(aura_spell_id, aura_spell);
    spell_store.insert(spell_id, spell);
    session.set_spell_store(Arc::new(spell_store));

    session
        .apply_creature_aura_like_cpp(aura_spell_id, player_guid, creature_guid, 1, 30_000)
        .expect("represented caster school aura should apply");

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
        "C++ drains `int32(100 * 1.5) = 150` with the caster school-mask aura"
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
            .unwrap(),
        100,
        "the caster share is `int32(150 * 0.5) = 75`"
    );
}

/// C++ `Unit::SpellDamageBonusTaken` (`Unit.cpp:6783-6791`): the damage taken is
/// multiplied by the victim's
/// `SPELL_AURA_MOD_MECHANIC_DAMAGE_TAKEN_PERCENT` auras whose misc value
/// intersects `SpellInfo::GetAllEffectsMechanicMask`.
#[tokio::test]
async fn spell_power_drain_applies_the_mechanic_damage_taken_aura_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 90_311_i32;
    let aura_spell_id = 90_312_i32;
    let player_guid = ObjectGuid::create_player(1, 912);
    let creature_guid = test_creature_guid(19_309);
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
            unit.set_max_power(PowerType::Mana, 200);
            unit.set_power(PowerType::Mana, 200);
            unit.set_display_power(PowerType::Mana);
            creature.clear_data_changes();
        })
        .unwrap();

    // Aura type 255 with misc bit 5, matching the drain spell's mechanic 5.
    let mut aura_spell = power_spell_info_like_cpp(
        aura_spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        50,
        PowerType::Mana,
    );
    aura_spell.effects[0].effect_aura =
        wow_data::spell::aura_types::SPELL_AURA_MOD_MECHANIC_DAMAGE_TAKEN_PERCENT;
    aura_spell.effects[0].effect_misc_value_1 = 1 << 5;
    let mut spell = power_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
        100,
        PowerType::Mana,
    );
    spell.effects[0].effect_amplitude = 0.5;
    spell.effects[0].effect_mechanic = 5;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(aura_spell_id, aura_spell);
    spell_store.insert(spell_id, spell);
    session.set_spell_store(Arc::new(spell_store));

    session
        .apply_creature_aura_like_cpp(aura_spell_id, player_guid, creature_guid, 1, 30_000)
        .expect("represented creature mechanic aura should apply");

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
        "C++ drains `int32(100 * 1.5) = 150` with the mechanic damage-taken aura"
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
            .unwrap(),
        100,
        "the caster share is `int32(150 * 0.5) = 75`"
    );
}

/// C++ `Unit::SpellDamageBonusTaken` (`Unit.cpp:6823-6828`): the damage taken is
/// also multiplied by the caster's
/// `SPELL_AURA_MOD_SPELL_DAMAGE_FROM_CASTER` auras whose spell affects the
/// damaging spell (`SpellFamilyName` equal, `SpellFamilyFlags` intersecting).
#[tokio::test]
async fn spell_power_drain_applies_the_caster_spell_damage_taken_aura_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 90_313_i32;
    let aura_spell_id = 90_314_i32;
    let player_guid = ObjectGuid::create_player(1, 913);
    let creature_guid = test_creature_guid(19_310);
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
            unit.set_max_power(PowerType::Mana, 200);
            unit.set_power(PowerType::Mana, 200);
            unit.set_display_power(PowerType::Mana);
            creature.clear_data_changes();
        })
        .unwrap();

    let mut aura_spell = power_spell_info_like_cpp(
        aura_spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        50,
        PowerType::Mana,
    );
    aura_spell.effects[0].effect_aura =
        wow_data::spell::aura_types::SPELL_AURA_MOD_SPELL_DAMAGE_FROM_CASTER;
    aura_spell.effects[0].effect_misc_value_1 = 0;
    let mut spell = power_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
        100,
        PowerType::Mana,
    );
    spell.effects[0].effect_amplitude = 0.5;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(aura_spell_id, aura_spell);
    spell_store.insert(spell_id, spell);
    session.set_spell_store(Arc::new(spell_store));
    // Both spells share family 3 and the same family flags, so the aura affects
    // the damaging spell (`AuraEffect::IsAffectingSpell`).
    session.set_spell_class_options_store(Arc::new(
        wow_data::SpellClassOptionsStore::from_entries([
            wow_data::SpellClassOptionsEntry {
                id: aura_spell_id as u32,
                spell_id: aura_spell_id,
                modal_next_spell: 0,
                spell_class_set: 3,
                spell_class_mask: [0x1, 0, 0, 0],
            },
            wow_data::SpellClassOptionsEntry {
                id: spell_id as u32,
                spell_id,
                modal_next_spell: 0,
                spell_class_set: 3,
                spell_class_mask: [0x1, 0, 0, 0],
            },
        ]),
    ));

    session
        .apply_creature_aura_like_cpp(aura_spell_id, player_guid, creature_guid, 1, 30_000)
        .expect("represented caster spell aura should apply");

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
        "C++ drains `int32(100 * 1.5) = 150` with the caster spell-mask aura"
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
            .unwrap(),
        100,
        "the caster share is `int32(150 * 0.5) = 75`"
    );
}

/// C++ `Unit::SpellDamageBonusTaken` (`Unit.cpp:6830-6835`): the damage taken is
/// also multiplied by the caster's
/// `SPELL_AURA_MOD_DAMAGE_TAKEN_FROM_CASTER_BY_LABEL` auras whose misc value is a
/// label the damaging spell carries (`SpellInfo::HasLabel`).
#[tokio::test]
async fn spell_power_drain_applies_the_caster_label_damage_taken_aura_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 90_318_i32;
    let aura_spell_id = 90_319_i32;
    let player_guid = ObjectGuid::create_player(1, 915);
    let creature_guid = test_creature_guid(19_312);
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
            unit.set_max_power(PowerType::Mana, 200);
            unit.set_power(PowerType::Mana, 200);
            unit.set_display_power(PowerType::Mana);
            creature.clear_data_changes();
        })
        .unwrap();

    let mut aura_spell = power_spell_info_like_cpp(
        aura_spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        50,
        PowerType::Mana,
    );
    aura_spell.effects[0].effect_aura =
        wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_TAKEN_FROM_CASTER_BY_LABEL;
    // Misc value 7 is the label the damaging spell carries.
    aura_spell.effects[0].effect_misc_value_1 = 7;
    let mut spell = power_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
        100,
        PowerType::Mana,
    );
    spell.effects[0].effect_amplitude = 0.5;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(aura_spell_id, aura_spell);
    spell_store.insert(spell_id, spell);
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_label_store(Arc::new(wow_data::SpellLabelStore::from_entries([
        wow_data::SpellLabelEntry {
            id: 1,
            label_id: 7,
            spell_id: spell_id as u32,
        },
    ])));

    session
        .apply_creature_aura_like_cpp(aura_spell_id, player_guid, creature_guid, 1, 30_000)
        .expect("represented caster label aura should apply");

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
        "C++ drains `int32(100 * 1.5) = 150` with the caster label aura"
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
            .unwrap(),
        100,
        "the caster share is `int32(150 * 0.5) = 75`"
    );
}

/// Regression for the amount path the drain pre-scaling reads: a negative-base
/// `SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN` registered through the creature-aura
/// path must feed `total_aura_multiplier_by_misc_mask_like_cpp` as a reduction.
#[tokio::test]
async fn creature_negative_damage_taken_aura_registers_a_reduction_like_cpp() {
    let (mut session, _, _) = make_session();
    let aura_spell_id = 90_323_i32;
    let player_guid = ObjectGuid::create_player(1, 917);
    let creature_guid = test_creature_guid(19_314);
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

    let mut aura_spell = power_spell_info_like_cpp(
        aura_spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        -50,
        PowerType::Mana,
    );
    aura_spell.effects[0].effect_aura =
        wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN;
    aura_spell.effects[0].effect_misc_value_1 = 0x01;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(aura_spell_id, aura_spell);
    session.set_spell_store(Arc::new(spell_store));

    session
        .apply_creature_aura_like_cpp(aura_spell_id, player_guid, creature_guid, 1, 30_000)
        .expect("represented creature reduction aura should apply");

    let multiplier = session
        .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
            creature
                .unit()
                .subsystems()
                .auras
                .total_aura_multiplier_by_misc_mask_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
                    0x01,
                )
        })
        .expect("canonical creature");
    assert_eq!(multiplier, 0.5, "a -50 amount must fold to `1 + (-50)/100`");
}
