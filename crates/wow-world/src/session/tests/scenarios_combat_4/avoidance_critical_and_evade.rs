use super::*;

#[test]
fn white_swing_gates_avoidance_on_the_controlled_state_like_cpp() {
    use wow_packet::packets::combat::{HIT_INFO_AFFECTS_VICTIM, VICTIM_STATE_DODGE};

    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_041);
    let player = ObjectGuid::create_player(1, 99);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Controlled".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 0);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            // A +100% `SPELL_AURA_MOD_DODGE_PERCENT` pushes the dodge band past
            // the roll, so the swing is guaranteed to be dodged.
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(91_152, player, 0, 1));
        })
        .unwrap();
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        91_152,
        wow_data::SpellInfo {
            spell_id: 91_152,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 100,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_DODGE_PERCENT),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DODGE_PERCENT,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    let swing = |session: &mut WorldSession| {
        let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
        let armor_mitigation = session.represented_melee_armor_mitigation_like_cpp();
        let outcome_facts = session.represented_melee_outcome_facts_like_cpp();
        let damage_taken = session.represented_melee_damage_taken_like_cpp();
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_attack_timer(WeaponAttackType::BaseAttack, 0);
                take_canonical_player_attack_swings_like_cpp(
                    player,
                    0,
                    true,
                    true,
                    true,
                    melee_damage_bonus,
                    armor_mitigation,
                    outcome_facts,
                    damage_taken,
                )
            })
            .flatten()
            .map(|(swings, _)| swings)
    };
    // The +100% dodge aura makes the band absolute while the victim is free.
    assert_eq!(
        swing(&mut session).map(|swings| (swings[0].damage, swings[0].victim_state)),
        Some((0, VICTIM_STATE_DODGE))
    );
    // C++ clears both avoidance gates for a `UNIT_STATE_CONTROLLED` victim.
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .unit_mut()
                .add_unit_state(UnitState::CONTROLLED.bits());
        })
        .unwrap();
    let facts = session.represented_melee_outcome_facts_like_cpp();
    assert!(facts.1.is_controlled);
    assert_eq!(
        swing(&mut session).map(|swings| (swings[0].damage, swings[0].hit_info)),
        Some((7, HIT_INFO_AFFECTS_VICTIM))
    );
}

#[test]
fn white_swing_applies_victim_critical_chance_auras_like_cpp() {
    use wow_packet::packets::combat::{
        HIT_INFO_AFFECTS_VICTIM, HIT_INFO_CRITICAL_HIT, VICTIM_STATE_HIT,
    };

    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_039);
    let player = ObjectGuid::create_player(1, 96);
    let foreign_caster = ObjectGuid::create_player(1, 97);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Crit".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 0);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    // `SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER` only applies when the attacker
    // cast it; the fixture's foreign caster must not add critical chance.
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        91_160,
        wow_data::SpellInfo {
            spell_id: 91_160,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 100,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    spell_store.insert(
        91_161,
        wow_data::SpellInfo {
            spell_id: 91_161,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 100,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(
                wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH,
            ),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura:
                    wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH,
                effect_misc_value_1: 0,
                // `MiscValueB` is the health threshold: `!HealthBelowPct(100)`
                // is always true for a living victim.
                effect_misc_value_2: 100,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    spell_store.insert(
        91_162,
        wow_data::SpellInfo {
            spell_id: 91_162,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 100,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_DAMAGE_BONUS),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_DAMAGE_BONUS,
                effect_misc_value_1: 0x01,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    let apply = |session: &mut WorldSession, spell_id: u32, caster: ObjectGuid| {
        session
            .mutate_world_creature(guid, |creature| {
                creature
                    .creature
                    .unit_mut()
                    .subsystems_mut()
                    .auras
                    .add_applied(wow_entities::AppliedAuraRef::new(spell_id, caster, 0, 1));
            })
            .expect("victim aura");
    };
    let swing = |session: &mut WorldSession| {
        let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
        let armor_mitigation = session.represented_melee_armor_mitigation_like_cpp();
        let outcome_facts = session.represented_melee_outcome_facts_like_cpp();
        let damage_taken = session.represented_melee_damage_taken_like_cpp();
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_attack_timer(WeaponAttackType::BaseAttack, 0);
                take_canonical_player_attack_swings_like_cpp(
                    player,
                    0,
                    true,
                    true,
                    true,
                    melee_damage_bonus,
                    armor_mitigation,
                    outcome_facts,
                    damage_taken,
                )
            })
            .flatten()
            .map(|(swings, _)| swings)
    };

    // A foreign caster's `MOD_CRIT_CHANCE_FOR_CASTER` is ignored.
    apply(&mut session, 91_160, foreign_caster);
    let facts = session.represented_melee_outcome_facts_like_cpp();
    assert_eq!(facts.1.crit_chance_for_caster_pct, 0.0);
    let swings = swing(&mut session).expect("white swing");
    assert_eq!(swings[0].damage, 7);
    assert_eq!(swings[0].hit_info, HIT_INFO_AFFECTS_VICTIM);

    // The attacker's own aura applies.
    apply(&mut session, 91_160, player);
    let facts = session.represented_melee_outcome_facts_like_cpp();
    assert_eq!(facts.1.crit_chance_for_caster_pct, 100.0);

    // The target-health aura is over the whole band, so the swing crits
    // deterministically.
    apply(&mut session, 91_161, player);
    let facts = session.represented_melee_outcome_facts_like_cpp();
    assert_eq!(facts.1.crit_chance_vs_target_health_pct, 100.0);
    let swings = swing(&mut session).expect("white swing");
    assert_eq!(swings[0].damage, 14);
    // C++ assigns `OriginalDamage` after the doubling, so the critical swing
    // publishes the doubled value as its original too (`Unit.cpp:1362-1375`).
    assert_eq!(swings[0].original_damage, 14);
    assert_eq!(
        swings[0].hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_CRITICAL_HIT
    );
    assert_eq!(swings[0].victim_state, VICTIM_STATE_HIT);

    // C++ `SPELL_AURA_MOD_CRIT_DAMAGE_BONUS` scales the doubled damage.
    session
        .apply_aura(91_162, player, 30_000, 1)
        .expect("apply crit-damage-bonus aura");
    let swings = swing(&mut session).expect("white swing");
    assert_eq!(swings[0].damage, 28);
    assert_eq!(swings[0].original_damage, 28);
    assert_eq!(
        swings[0].hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_CRITICAL_HIT
    );
}

#[test]
fn melee_attack_table_reads_the_dual_wield_penalty_aura_like_cpp() {
    use crate::session_rules::melee_outcome_inputs_like_cpp;

    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_040);
    let player = ObjectGuid::create_player(1, 98);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "DualWield".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
        })
        .unwrap();
    register_test_creature(&mut session, manager.clone(), guid, 40);
    // The fixture trains the attacker past the dual-wield penalty; this scenario
    // needs the plain `7.5` `m_modMeleeHitChance`.
    session
        .mutate_canonical_player_like_cpp(|player| {
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.melee_hit_chance_pct = 7.5;
            player.replace_effective_combat_stats_like_cpp(stats);
        })
        .unwrap();
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        91_170,
        wow_data::SpellInfo {
            spell_id: 91_170,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_IGNORE_DUAL_WIELD_HIT_PENALTY),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_IGNORE_DUAL_WIELD_HIT_PENALTY,
                effect_base_points: 0,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    let bare = session.represented_melee_outcome_facts_like_cpp();
    assert!(!bare.0.ignores_dual_wield_hit_penalty);
    // A dual-wielding attacker without the aura carries `5 + 19` miss.
    let dual = crate::session_rules::RepresentedMeleeAttackerFactsLikeCpp {
        dual_wielding: true,
        ..bare.0
    };
    let inputs = melee_outcome_inputs_like_cpp(&dual, &bare.1);
    // `5 + 19 - 7.5`.
    assert_eq!(inputs[0].miss_chance_pct, 16.5);

    // With the aura the penalty disappears.
    session
        .apply_aura(91_170, player, 30_000, 1)
        .expect("apply ignore-dual-wield aura");
    let facts = session.represented_melee_outcome_facts_like_cpp();
    assert!(facts.0.ignores_dual_wield_hit_penalty);
    let dual = crate::session_rules::RepresentedMeleeAttackerFactsLikeCpp {
        dual_wielding: true,
        ..facts.0
    };
    let inputs = melee_outcome_inputs_like_cpp(&dual, &facts.1);
    assert_eq!(inputs[0].miss_chance_pct, 0.0);
}

#[test]
fn white_swing_publishes_an_evade_like_cpp() {
    use wow_packet::packets::combat::{
        HIT_INFO_MISS, HIT_INFO_SWING_NO_HIT_SOUND, VICTIM_STATE_EVADES,
    };

    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_042);
    let player = ObjectGuid::create_player(1, 100);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Evade".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 0);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    let swing = |session: &mut WorldSession| {
        let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
        let armor_mitigation = session.represented_melee_armor_mitigation_like_cpp();
        let outcome_facts = session.represented_melee_outcome_facts_like_cpp();
        let damage_taken = session.represented_melee_damage_taken_like_cpp();
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_attack_timer(WeaponAttackType::BaseAttack, 0);
                take_canonical_player_attack_swings_like_cpp(
                    player,
                    0,
                    true,
                    true,
                    true,
                    melee_damage_bonus,
                    armor_mitigation,
                    outcome_facts,
                    damage_taken,
                )
            })
            .flatten()
            .map(|(swings, _)| swings)
    };

    // A free victim lands a normal hit.
    assert_eq!(swing(&mut session).map(|s| s[0].damage), Some(7));

    // C++ `IsEvadingAttacks()` returns `MELEE_HIT_EVADE` before any band.
    session
        .mutate_world_creature(guid, |creature| {
            creature.creature.set_in_evade_mode_like_cpp(true);
        })
        .unwrap();
    let facts = session.represented_melee_outcome_facts_like_cpp();
    assert!(facts.1.is_evading_attacks);
    let swings = swing(&mut session).expect("white swing resolves");
    assert_eq!(swings[0].damage, 0);
    assert_eq!(
        swings[0].hit_info,
        HIT_INFO_MISS | HIT_INFO_SWING_NO_HIT_SOUND
    );
    assert_eq!(swings[0].victim_state, VICTIM_STATE_EVADES);
}
