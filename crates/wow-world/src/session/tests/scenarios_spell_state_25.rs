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
