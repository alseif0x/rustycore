//! Session scenarios exercising the represented unit aura state and the spell
//! damage percentage terms that read it (`Unit::HasAuraState`,
//! `Unit::SpellDamagePctDone`).
//!
//! Split out of scenarios_spell_state_11.rs when that file reached the
//! physical test-file budget; assertions and registrations are unchanged and
//! the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn spell_school_damage_applies_damage_done_versus_health_aurastate_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let spell_id = 734_i32;
    let guid = test_creature_guid(18_021);
    let player_guid = ObjectGuid::create_player(1, 65);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    register_test_creature(&mut session, manager.clone(), guid, 2_000);

    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        wow_data::SpellMiscEntry {
            id: spell_id as u32,
            spell_id: spell_id as u32,
            school_mask: 1 << 1,
            ..Default::default()
        },
    ])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    spell_store.insert(
        90_988,
        wow_data::SpellInfo {
            spell_id: 90_988,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 100,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(
                wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE,
            ),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura:
                    wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE,
                effect_misc_value_1: i32::from(wow_entities::AURA_STATE_WOUNDED_20_PERCENT),
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session
        .apply_aura(90_988, player_guid, 30_000, 1)
        .expect("apply wounded-aurastate damage aura");

    // At full health `Unit::Update` keeps no wounded bit, so the aura misses.
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("school damage against a healthy target");
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        1_900
    );

    // C++ `Unit::Update` sets `AURA_STATE_WOUNDED_20_PERCENT` while alive and
    // below 20% health; the creature's aura subsystem owns no such bit.
    session
        .mutate_world_creature(guid, |creature| {
            creature.creature.unit_mut().set_health(300);
        })
        .expect("wounded creature");
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("school damage against a wounded target");
    // `100 * (1 + 100/100)`.
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        100
    );
}

#[test]
fn represented_aura_state_unions_health_and_aura_bits_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 66);
    session.player_guid = Some(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);

    // 50% health keeps every `Unit::Update` wounded/healthy bit clear.
    session.set_player_health_like_cpp(50, 100);
    assert_eq!(
        session.represented_player_aura_state_mask_like_cpp(),
        Some(0)
    );
    assert!(!session.represented_has_aura_state_like_cpp(u32::from(
        wow_entities::AURA_STATE_WOUNDED_20_PERCENT
    )));

    session.set_player_health_like_cpp(10, 100);
    let mask = session
        .represented_player_aura_state_mask_like_cpp()
        .expect("canonical player aura state mask");
    assert!(
        mask & (1 << (wow_entities::AURA_STATE_WOUNDED_20_PERCENT - 1)) != 0,
        "10% health must set AURA_STATE_WOUNDED_20_PERCENT, got {mask:#x}"
    );
    assert!(session.represented_has_aura_state_like_cpp(u32::from(
        wow_entities::AURA_STATE_WOUNDED_20_PERCENT
    )));
    // AURA_STATE_HEALTHY_75_PERCENT must not survive the drop below 20%.
    assert!(!session.represented_has_aura_state_like_cpp(23));
}

#[test]
fn login_passive_cast_gate_uses_health_derived_caster_aura_state_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 67);
    session.player_guid = Some(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    let spell_id = 60_004_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, [0; 15]);
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_aura_restrictions_store(Arc::new(
        wow_data::SpellAuraRestrictionsStore::from_entries([
            wow_data::SpellAuraRestrictionsEntry {
                id: 1,
                difficulty_id: 0,
                caster_aura_state: wow_entities::AURA_STATE_WOUNDED_20_PERCENT,
                target_aura_state: 0,
                exclude_caster_aura_state: 0,
                exclude_target_aura_state: 0,
                caster_aura_spell: 0,
                target_aura_spell: 0,
                exclude_caster_aura_spell: 0,
                exclude_target_aura_spell: 0,
                spell_id: spell_id as u32,
            },
        ]),
    ));

    // C++ `Spell::CheckCast` reads `Unit::m_unitData->AuraState`, which is the
    // union of the aura-driven bits and the `Unit::Update` health bits.
    session.set_player_health_like_cpp(50, 100);
    assert!(!session.represented_login_passive_spell_cast_gate_like_cpp(spell_id));
    session.set_player_health_like_cpp(10, 100);
    assert!(session.represented_login_passive_spell_cast_gate_like_cpp(spell_id));
}

fn represented_direct_damage_spell_like_cpp(
    spell_id: i32,
    base_damage: i32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
            effect_base_points: base_damage,
            ..Default::default()
        }],
    }
}

fn represented_aura_spell_like_cpp(
    spell_id: i32,
    aura_type: i32,
    misc_value: i32,
    amount: i32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_base_points: amount,
        effect_bonus_coefficient: 0.0,
        aura_type: Some(aura_type),
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_type,
            effect_misc_value_1: misc_value,
            effect_base_points: amount,
            ..Default::default()
        }],
    }
}

fn represented_frost_spell_misc_like_cpp(spell_ids: &[i32]) -> wow_data::SpellMiscStore {
    wow_data::SpellMiscStore::from_entries(spell_ids.iter().map(|spell_id| {
        wow_data::SpellMiscEntry {
            id: *spell_id as u32,
            spell_id: *spell_id as u32,
            school_mask: 1 << 1,
            ..Default::default()
        }
    }))
}

#[tokio::test]
async fn spell_school_damage_respects_ignore_caster_damage_modifiers_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let plain_spell_id = 735_i32;
    let gated_spell_id = 736_i32;
    let caster_modifier_gated_spell_id = 737_i32;
    let guid = test_creature_guid(18_022);
    let player_guid = ObjectGuid::create_player(1, 68);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    register_test_creature(&mut session, manager.clone(), guid, 1_000);
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .modify_aura_state(wow_entities::AURA_STATE_DEFENSIVE, true);
        })
        .expect("creature with a defensive aura state");

    session.set_spell_misc_store(Arc::new(represented_frost_spell_misc_like_cpp(&[
        plain_spell_id,
        gated_spell_id,
        caster_modifier_gated_spell_id,
    ])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        plain_spell_id,
        represented_direct_damage_spell_like_cpp(plain_spell_id, 100),
    );
    spell_store.insert(
        gated_spell_id,
        represented_direct_damage_spell_like_cpp(gated_spell_id, 100),
    );
    spell_store.insert(
        caster_modifier_gated_spell_id,
        represented_direct_damage_spell_like_cpp(caster_modifier_gated_spell_id, 100),
    );
    let mut attributes = [0_u32; 15];
    attributes[6] = wow_data::spell::attributes::SPELL_ATTR6_IGNORE_CASTER_DAMAGE_MODIFIERS;
    spell_store.insert_spell_misc_attributes_for_difficulty_like_cpp(gated_spell_id, 0, attributes);
    let mut attributes = [0_u32; 15];
    attributes[3] = wow_data::spell::attributes::SPELL_ATTR3_IGNORE_CASTER_MODIFIERS;
    spell_store.insert_spell_misc_attributes_for_difficulty_like_cpp(
        caster_modifier_gated_spell_id,
        0,
        attributes,
    );
    spell_store.insert(
        90_989,
        represented_aura_spell_like_cpp(
            90_989,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE,
            i32::from(wow_entities::AURA_STATE_DEFENSIVE),
            100,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));
    session
        .apply_aura(90_989, player_guid, 30_000, 1)
        .expect("apply versus-aurastate damage aura");

    session
        .execute_spell(plain_spell_id, guid)
        .await
        .expect("school damage without the ignore-modifiers attribute");
    // `100 * (1 + 100/100)`.
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        800
    );

    session
        .execute_spell(gated_spell_id, guid)
        .await
        .expect("school damage with the ignore-damage-modifiers attribute");
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        700
    );

    session
        .execute_spell(caster_modifier_gated_spell_id, guid)
        .await
        .expect("school damage with the ignore-caster-modifiers attribute");
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        600
    );
}

#[tokio::test]
async fn spell_school_damage_applies_ice_lance_frozen_triple_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let spell_id = 228_598_i32;
    let guid = test_creature_guid(18_023);
    let player_guid = ObjectGuid::create_player(1, 69);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    register_test_creature(&mut session, manager.clone(), guid, 1_000);

    session.set_spell_misc_store(Arc::new(represented_frost_spell_misc_like_cpp(&[spell_id])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        represented_direct_damage_spell_like_cpp(spell_id, 100),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, guid)
        .await
        .expect("Ice Lance against a target that is not frozen");
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        900
    );

    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .modify_aura_state(wow_entities::AURA_STATE_FROZEN, true);
        })
        .expect("frozen creature");
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("Ice Lance against a frozen target");
    // `100 * 3`.
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        600
    );
}

#[tokio::test]
async fn spell_school_damage_applies_drain_soul_wounded_double_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let spell_id = 198_590_i32;
    let guid = test_creature_guid(18_024);
    let player_guid = ObjectGuid::create_player(1, 70);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    register_test_creature(&mut session, manager.clone(), guid, 1_000);

    session.set_spell_misc_store(Arc::new(represented_frost_spell_misc_like_cpp(&[spell_id])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        represented_direct_damage_spell_like_cpp(spell_id, 100),
    );
    session.set_spell_store(Arc::new(spell_store));

    // The scripted term reads the *caster's* wounded state, not the victim's.
    session.set_player_health_like_cpp(50, 100);
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("Drain Soul from a healthy caster");
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        900
    );

    session.set_player_health_like_cpp(10, 100);
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("Drain Soul from a wounded caster");
    // `100 * 2`.
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        700
    );
}

fn represented_direct_heal_spell_like_cpp(
    spell_id: i32,
    base_heal: i32,
    coefficient: f32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: coefficient,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL,
            effect_base_points: base_heal,
            ..Default::default()
        }],
    }
}

#[tokio::test]
async fn spell_direct_heal_applies_damage_done_versus_aurastate_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let spell_id = 738_i32;
    let guid = test_creature_guid(18_025);
    let player_guid = ObjectGuid::create_player(1, 71);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    register_test_creature(&mut session, manager.clone(), guid, 1_000);
    session
        .mutate_world_creature(guid, |creature| {
            creature.creature.unit_mut().set_health(100);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .modify_aura_state(wow_entities::AURA_STATE_DEFENSIVE, true);
        })
        .expect("wounded creature with a defensive aura state");

    session.set_spell_misc_store(Arc::new(represented_frost_spell_misc_like_cpp(&[spell_id])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        represented_direct_heal_spell_like_cpp(spell_id, 100, 0.0),
    );
    spell_store.insert(
        90_990,
        represented_aura_spell_like_cpp(
            90_990,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE,
            i32::from(wow_entities::AURA_STATE_DEFENSIVE),
            100,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));
    session
        .apply_aura(90_990, player_guid, 30_000, 1)
        .expect("apply versus-aurastate healing aura");

    session
        .execute_spell(spell_id, guid)
        .await
        .expect("direct heal against a defensive-aurastate target");
    // `100 * (1 + 100/100)`.
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        300
    );
}

#[tokio::test]
async fn spell_direct_heal_respects_ignore_healing_modifiers_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let plain_spell_id = 739_i32;
    let gated_spell_id = 740_i32;
    let guid = test_creature_guid(18_026);
    let player_guid = ObjectGuid::create_player(1, 72);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    register_test_creature(&mut session, manager.clone(), guid, 1_000);
    session
        .mutate_world_creature(guid, |creature| {
            creature.creature.unit_mut().set_health(100);
        })
        .expect("wounded creature");
    // `SpellHealingPctDone`: `ModHealingDonePercent = 1.5`.
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.replace_effective_combat_stats_like_cpp(
                wow_entities::PlayerEffectiveCombatStatsLikeCpp {
                    spell_power: 100,
                    mod_healing_done_percent: 1.5,
                    ..Default::default()
                },
            );
        })
        .expect("canonical player owner");

    session.set_spell_misc_store(Arc::new(represented_frost_spell_misc_like_cpp(&[
        plain_spell_id,
        gated_spell_id,
    ])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        plain_spell_id,
        represented_direct_heal_spell_like_cpp(plain_spell_id, 100, 0.5),
    );
    spell_store.insert(
        gated_spell_id,
        represented_direct_heal_spell_like_cpp(gated_spell_id, 100, 0.5),
    );
    let mut attributes = [0_u32; 15];
    attributes[6] = wow_data::spell::attributes::SPELL_ATTR6_IGNORE_HEALING_MODIFIERS;
    spell_store.insert_spell_misc_attributes_for_difficulty_like_cpp(gated_spell_id, 0, attributes);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(plain_spell_id, guid)
        .await
        .expect("direct heal without the ignore-healing-modifiers attribute");
    // `(100 + 100 * 0.5) * 1.5`.
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        325
    );

    session
        .execute_spell(gated_spell_id, guid)
        .await
        .expect("direct heal with the ignore-healing-modifiers attribute");
    // The early-out only gates the percentage chain: the flat spell-power
    // benefit still applies, so `(100 + 100 * 0.5) * 1.0`.
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        475
    );
}

fn represented_ap_scaled_damage_spell_like_cpp(
    spell_id: i32,
    base_damage: i32,
    coefficient_from_ap: f32,
) -> wow_data::SpellInfo {
    let mut spell = represented_direct_damage_spell_like_cpp(spell_id, base_damage);
    if let Some(effect) = spell.effects.first_mut() {
        effect.effect_bonus_coefficient_from_ap = coefficient_from_ap;
    }
    spell
}

#[tokio::test]
async fn spell_school_damage_applies_bonus_coefficient_from_ap_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let plain_spell_id = 741_i32;
    let ap_spell_id = 742_i32;
    let guid = test_creature_guid(18_027);
    let player_guid = ObjectGuid::create_player(1, 73);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    register_test_creature(&mut session, manager.clone(), guid, 1_000);
    // `Unit::GetTotalAttackPowerValue(BASE_ATTACK)` = 100.
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.replace_effective_combat_stats_like_cpp(
                wow_entities::PlayerEffectiveCombatStatsLikeCpp {
                    attack_power: 100,
                    ..Default::default()
                },
            );
        })
        .expect("canonical player owner");

    session.set_spell_misc_store(Arc::new(represented_frost_spell_misc_like_cpp(&[
        plain_spell_id,
        ap_spell_id,
    ])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        plain_spell_id,
        represented_ap_scaled_damage_spell_like_cpp(plain_spell_id, 100, 0.0),
    );
    spell_store.insert(
        ap_spell_id,
        represented_ap_scaled_damage_spell_like_cpp(ap_spell_id, 100, 3.0),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(plain_spell_id, guid)
        .await
        .expect("school damage without an AP coefficient");
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        900
    );

    session
        .execute_spell(ap_spell_id, guid)
        .await
        .expect("school damage with a BonusCoefficientFromAP");
    // `100 + int32(3.0 * 100)`.
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        500
    );
}

#[tokio::test]
async fn spell_direct_heal_applies_bonus_coefficient_from_ap_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let spell_id = 743_i32;
    let guid = test_creature_guid(18_028);
    let player_guid = ObjectGuid::create_player(1, 74);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    register_test_creature(&mut session, manager.clone(), guid, 1_000);
    session
        .mutate_world_creature(guid, |creature| {
            creature.creature.unit_mut().set_health(100);
        })
        .expect("wounded creature");
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.replace_effective_combat_stats_like_cpp(
                wow_entities::PlayerEffectiveCombatStatsLikeCpp {
                    attack_power: 100,
                    ..Default::default()
                },
            );
        })
        .expect("canonical player owner");

    session.set_spell_misc_store(Arc::new(represented_frost_spell_misc_like_cpp(&[spell_id])));
    let mut spell_store = wow_data::SpellStore::new();
    let mut heal = represented_direct_heal_spell_like_cpp(spell_id, 100, 0.0);
    if let Some(effect) = heal.effects.first_mut() {
        effect.effect_bonus_coefficient_from_ap = 2.0;
    }
    spell_store.insert(spell_id, heal);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, guid)
        .await
        .expect("direct heal with a BonusCoefficientFromAP");
    // `100 + int32(2.0 * 100)`.
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        400
    );
}

#[tokio::test]
async fn spell_school_damage_ignores_the_flat_benefit_of_ignore_caster_modifiers_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let plain_spell_id = 744_i32;
    let gated_spell_id = 745_i32;
    let guid = test_creature_guid(18_029);
    let player_guid = ObjectGuid::create_player(1, 75);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    register_test_creature(&mut session, manager.clone(), guid, 1_000);
    // `SpellBaseDamageBonusDone` = 100 spell power.
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.replace_effective_combat_stats_like_cpp(
                wow_entities::PlayerEffectiveCombatStatsLikeCpp {
                    spell_power: 100,
                    ..Default::default()
                },
            );
        })
        .expect("canonical player owner");

    session.set_spell_misc_store(Arc::new(represented_frost_spell_misc_like_cpp(&[
        plain_spell_id,
        gated_spell_id,
    ])));
    let mut spell_store = wow_data::SpellStore::new();
    for spell_id in [plain_spell_id, gated_spell_id] {
        let mut spell = represented_direct_damage_spell_like_cpp(spell_id, 100);
        spell.effect_bonus_coefficient = 0.5;
        spell_store.insert(spell_id, spell);
    }
    let mut attributes = [0_u32; 15];
    attributes[3] = wow_data::spell::attributes::SPELL_ATTR3_IGNORE_CASTER_MODIFIERS;
    spell_store.insert_spell_misc_attributes_for_difficulty_like_cpp(gated_spell_id, 0, attributes);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(plain_spell_id, guid)
        .await
        .expect("school damage without the ignore-caster-modifiers attribute");
    // `100 + int32(100 * 0.5)`.
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        850
    );

    session
        .execute_spell(gated_spell_id, guid)
        .await
        .expect("school damage with the ignore-caster-modifiers attribute");
    // C++ returns before the flat advertised benefit as well.
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        750
    );
}

#[tokio::test]
async fn represented_haste_aura_scales_attack_time_multiplier_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 76);
    session.player_guid = Some(player_guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    let mut spell_store = wow_data::SpellStore::new();
    // `SPELL_AURA_MOD_MELEE_HASTE` and `SPELL_AURA_MOD_SPEED_SLOW_ALL` with
    // opposite signs exercise both branches of `ApplyAttackTimePercentMod`.
    spell_store.insert(
        90_991,
        represented_aura_spell_like_cpp(
            90_991,
            wow_data::spell::aura_types::SPELL_AURA_MOD_MELEE_HASTE,
            0,
            30,
        ),
    );
    spell_store.insert(
        90_992,
        represented_aura_spell_like_cpp(
            90_992,
            wow_data::spell::aura_types::SPELL_AURA_MOD_SPEED_SLOW_ALL,
            0,
            -30,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    assert_eq!(
        session
            .canonical_player_snapshot_like_cpp(|player| player.unit().mod_attack_speed_pct())
            .expect("canonical player"),
        [1.0, 1.0, 1.0]
    );

    session
        .apply_aura(90_991, player_guid, 30_000, 1)
        .expect("apply melee haste aura");
    // `100 / (100 + 30)` for main hand and off hand, ranged untouched. The
    // consumer is the swing timer reset, which reads the same multiplier.
    let hasted = session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .reset_attack_timer_like_cpp(wow_constants::WeaponAttackType::BaseAttack);
            (
                player.unit().mod_attack_speed_pct(),
                player
                    .unit()
                    .attack_timer(wow_constants::WeaponAttackType::BaseAttack),
                player.unit().base_attack_speed()[0],
            )
        })
        .expect("canonical player");
    assert!((hasted.0[0] - 100.0 / 130.0).abs() < 1e-6, "{:?}", hasted.0);
    assert!((hasted.0[1] - 100.0 / 130.0).abs() < 1e-6, "{:?}", hasted.0);
    assert_eq!(hasted.0[2], 1.0);
    assert_eq!(hasted.1, (hasted.2 as f32 * 100.0 / 130.0) as u32);

    session
        .apply_aura(90_992, player_guid, 30_000, 1)
        .expect("apply combat slow aura");
    // `100 / 130 * (100 + 30) / 100` for every attack.
    let slowed = session
        .canonical_player_snapshot_like_cpp(|player| player.unit().mod_attack_speed_pct())
        .expect("canonical player");
    assert!((slowed[0] - 1.0).abs() < 1e-6, "{slowed:?}");
    assert!((slowed[1] - 1.0).abs() < 1e-6, "{slowed:?}");
    assert!((slowed[2] - 1.3).abs() < 1e-6, "{slowed:?}");

    session.remove_aura(0).expect("remove melee haste aura");
    let restored = session
        .canonical_player_snapshot_like_cpp(|player| player.unit().mod_attack_speed_pct())
        .expect("canonical player");
    assert!((restored[0] - 1.3).abs() < 1e-6, "{restored:?}");
    assert!((restored[1] - 1.3).abs() < 1e-6, "{restored:?}");
    assert!((restored[2] - 1.3).abs() < 1e-6, "{restored:?}");

    session.remove_aura(1).expect("remove combat slow aura");
    assert_eq!(
        session
            .canonical_player_snapshot_like_cpp(|player| player.unit().mod_attack_speed_pct())
            .expect("canonical player"),
        [1.0, 1.0, 1.0]
    );
}

#[tokio::test]
async fn represented_shapeshift_combat_round_time_sets_form_attack_time_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    session.player_guid = Some(player_guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    let form_id = 8_u32;
    let spell_id = 90_993_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        represented_aura_spell_like_cpp(
            spell_id,
            wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT,
            form_id as i32,
            0,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_shapeshift_form_store(Arc::new(
        wow_data::SpellShapeshiftFormStore::from_entries([wow_data::SpellShapeshiftFormEntry {
            id: form_id,
            name: "Dire Bear Form".to_string(),
            creature_type: 0,
            flags: 0,
            attack_icon_file_id: 0,
            bonus_action_bar: 0,
            combat_round_time: 1_000,
            damage_variance: 0.0,
            mount_type_id: 0,
            creature_display_id: [0; 4],
            preset_spell_id: [0; wow_data::MAX_SHAPESHIFT_SPELLS],
        }]),
    ));

    assert_eq!(
        session.represented_shapeshift_combat_round_time_like_cpp(),
        None
    );

    session
        .apply_aura(spell_id, player_guid, 30_000, 1)
        .expect("apply shapeshift aura");
    // C++ `Player::GetShapeshiftForm` now resolves the form and
    // `InitDataForForm` writes its `CombatRoundTime` to both melee attacks.
    assert_eq!(
        session.represented_shapeshift_combat_round_time_like_cpp(),
        Some(1_000.0)
    );
    let formed = session
        .canonical_player_snapshot_like_cpp(|player| player.unit().base_attack_speed())
        .expect("canonical player");
    assert_eq!(formed[0], 1_000);
    assert_eq!(formed[1], 1_000);
    assert_eq!(formed[2], 2_000);

    session.remove_aura(0).expect("remove shapeshift aura");
    assert_eq!(
        session.represented_shapeshift_combat_round_time_like_cpp(),
        None
    );
    let restored = session
        .canonical_player_snapshot_like_cpp(|player| player.unit().base_attack_speed())
        .expect("canonical player");
    // C++ `Player::SetRegularAttackTime` only writes an attack whose equipped
    // weapon declares a delay, so this unarmed fixture keeps the form's melee
    // time while the ranged arm stays at `BASE_ATTACK_TIME`.
    assert_eq!(restored, [1_000, 1_000, 2_000]);
}
