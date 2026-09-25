use super::*;

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
