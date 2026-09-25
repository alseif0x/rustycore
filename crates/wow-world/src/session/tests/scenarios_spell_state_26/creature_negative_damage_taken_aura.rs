use super::*;

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
