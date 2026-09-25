//! Spell damage-done modifier scenarios.

use super::*;

#[tokio::test]
async fn spell_school_damage_applies_damage_done_versus_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let spell_id = 728_i32;
    let guid = test_creature_guid(18_015);
    let player_guid = ObjectGuid::create_player(1, 59);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    register_test_creature(&mut session, manager.clone(), guid, 1_000);
    // `register_test_creature` uses entry 9001; give it creature type 7.
    session.set_creature_template_lifecycle_store_like_cpp(Arc::new(
        wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates([
            wow_data::CreatureTemplateLifecycleRecordLikeCpp {
                entry: 9001,
                creature_type: 7,
                ..Default::default()
            },
        ]),
    ));

    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        wow_data::SpellMiscEntry {
            id: spell_id as u32,
            spell_id: spell_id as u32,
            // Holy.
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
    // `SPELL_AURA_MOD_DAMAGE_DONE_VERSUS` for the matching creature type (7 ->
    // bit 6) and for an unrelated one (2 -> bit 1).
    for (aura_spell_id, misc_value) in [(90_960_i32, 1 << 6), (90_961_i32, 1 << 1)] {
        spell_store.insert(
            aura_spell_id,
            wow_data::SpellInfo {
                spell_id: aura_spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 100,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS,
                    effect_misc_value_1: misc_value,
                    effect_base_points: 100,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));

    // A non-matching creature type leaves the damage unchanged.
    session
        .apply_aura(90_961, player_guid, 30_000, 1)
        .expect("apply unrelated versus aura");
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("school damage with an unrelated versus aura");
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
        .apply_aura(90_960, player_guid, 30_000, 1)
        .expect("apply matching versus aura");
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("school damage with the matching versus aura");
    // `100 * (1 + 100/100)`.
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

#[tokio::test]
async fn spell_school_damage_applies_damage_done_versus_aurastate_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let spell_id = 730_i32;
    let guid = test_creature_guid(18_017);
    let player_guid = ObjectGuid::create_player(1, 61);
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
        .expect("creature with an aura state");

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
    // `SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE` for the defensive state and
    // for an unrelated one.
    for (aura_spell_id, misc_value) in [(90_980_i32, 1), (90_981_i32, 2)] {
        spell_store.insert(
            aura_spell_id,
            wow_data::SpellInfo {
                spell_id: aura_spell_id,
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
                    effect_misc_value_1: misc_value,
                    effect_base_points: 100,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));

    // The victim does not carry state 2, so that aura does not apply.
    session
        .apply_aura(90_981, player_guid, 30_000, 1)
        .expect("apply unrelated aurastate aura");
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("school damage with an unrelated aurastate aura");
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
        .apply_aura(90_980, player_guid, 30_000, 1)
        .expect("apply matching aurastate aura");
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("school damage with the matching aurastate aura");
    // `100 * (1 + 100/100)`.
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

#[tokio::test]
async fn spell_school_damage_applies_damage_done_for_mechanic_like_cpp() {
    const MECHANIC_STUN_LIKE_CPP: i32 = 12;
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let spell_id = 732_i32;
    let guid = test_creature_guid(18_018);
    let player_guid = ObjectGuid::create_player(1, 63);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    register_test_creature(&mut session, manager.clone(), guid, 1_000);

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
    // The cast effect carries mechanic 12; only the matching
    // `SPELL_AURA_MOD_DAMAGE_DONE_FOR_MECHANIC` aura may scale the damage.
    spell_store.insert_spell_hit_metadata_for_difficulty_like_cpp(
        spell_id,
        0,
        wow_data::SpellHitMetadataLikeCpp {
            effect_mechanics: BTreeMap::from([(0, MECHANIC_STUN_LIKE_CPP)]),
            ..Default::default()
        },
    );
    for (aura_spell_id, misc_value) in [(90_983_i32, MECHANIC_STUN_LIKE_CPP), (90_984_i32, 13)] {
        spell_store.insert(
            aura_spell_id,
            wow_data::SpellInfo {
                spell_id: aura_spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 100,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_FOR_MECHANIC,
                ),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura:
                        wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_FOR_MECHANIC,
                    effect_misc_value_1: misc_value,
                    effect_base_points: 100,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));

    // A mechanic mismatch keeps the base damage.
    session
        .apply_aura(90_984, player_guid, 30_000, 1)
        .expect("apply non-matching mechanic aura");
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("school damage with a non-matching mechanic aura");
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
        .apply_aura(90_983, player_guid, 30_000, 1)
        .expect("apply matching mechanic aura");
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("school damage with the matching mechanic aura");
    // `100 * (1 + 100/100)`.
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

#[tokio::test]
async fn spell_school_damage_applies_damage_percent_done_by_target_aura_mechanic_like_cpp() {
    const MECHANIC_STUN_LIKE_CPP: i32 = 12;
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let spell_id = 733_i32;
    let guid = test_creature_guid(18_019);
    let player_guid = ObjectGuid::create_player(1, 64);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    register_test_creature(&mut session, manager.clone(), guid, 1_000);

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
    // The caster's `SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC`
    // only matches a victim aura whose `SpellInfo::Mechanic` is 12.
    spell_store.insert(
        90_985,
        wow_data::SpellInfo {
            spell_id: 90_985,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 100,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(
                wow_data::spell::aura_types::
                    SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC,
            ),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::
                    SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC,
                effect_misc_value_1: MECHANIC_STUN_LIKE_CPP,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    for (victim_aura_spell_id, mechanic) in [(90_986_i32, 13_i8), (90_987_i32, 12_i8)] {
        spell_store.insert(
            victim_aura_spell_id,
            wow_data::SpellInfo {
                spell_id: victim_aura_spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_DUMMY),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_DUMMY,
                    ..Default::default()
                }],
            },
        );
        spell_store.insert_spell_hit_metadata_for_difficulty_like_cpp(
            victim_aura_spell_id,
            0,
            wow_data::SpellHitMetadataLikeCpp {
                spell_mechanic: mechanic,
                ..Default::default()
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session
        .apply_aura(90_985, player_guid, 30_000, 1)
        .expect("apply target-aura-mechanic damage aura");

    // The victim only carries an unrelated mechanic, so the aura does not apply.
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(90_986, player_guid, 0, 1));
        })
        .expect("creature with an unrelated mechanic aura");
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("school damage against an unrelated victim mechanic");
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
                .add_applied(wow_entities::AppliedAuraRef::new(90_987, player_guid, 1, 1));
        })
        .expect("creature with the matching mechanic aura");
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("school damage against the matching victim mechanic");
    // `100 * (1 + 100/100)`.
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
