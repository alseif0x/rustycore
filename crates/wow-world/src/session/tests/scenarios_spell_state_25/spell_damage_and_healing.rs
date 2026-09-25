use super::*;

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
