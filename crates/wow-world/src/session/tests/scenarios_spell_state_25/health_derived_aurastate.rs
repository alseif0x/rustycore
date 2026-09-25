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
