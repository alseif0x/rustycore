//! Preserved direct-damage scenarios for caster aura terms.

use super::*;


/// The direct-damage guard in `Unit::SpellDamageBonusTaken` runs before the
/// caster school-mask term (`Unit.cpp:6775-6777` and `6815-6822`).
#[tokio::test]
async fn spell_power_drain_ignores_caster_school_damage_taken_aura_for_direct_damage_like_cpp() {
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
        100,
        "direct power drain ignores the caster school-mask taken aura"
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
            .unwrap(),
        75,
        "the caster share is `int32(100 * 0.5) = 50` added to the initial 25"
    );
}

/// The direct-damage guard in `Unit::SpellDamageBonusTaken` runs before the
/// caster spell-family term (`Unit.cpp:6775-6777` and `6823-6828`).
#[tokio::test]
async fn spell_power_drain_ignores_caster_spell_damage_taken_aura_for_direct_damage_like_cpp() {
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
        100,
        "direct power drain ignores the caster spell-family taken aura"
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
            .unwrap(),
        75,
        "the caster share is `int32(100 * 0.5) = 50` added to the initial 25"
    );
}

/// The direct-damage guard in `Unit::SpellDamageBonusTaken` runs before the
/// caster label term (`Unit.cpp:6775-6777` and `6830-6835`).
#[tokio::test]
async fn spell_power_drain_ignores_caster_label_damage_taken_aura_for_direct_damage_like_cpp() {
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
        100,
        "direct power drain ignores the caster label taken aura"
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
            .unwrap(),
        75,
        "the caster share is `int32(100 * 0.5) = 50` added to the initial 25"
    );
}
