use super::*;

#[tokio::test]
async fn wrapper_with_missing_live_aura_metadata_fails_before_charge_or_publication() {
    let mut fixture = trainer_wrapper_fixture();
    seed_unclassified_active_aura(&mut fixture.session, 2);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            WRAPPER_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.player_gold_like_cpp(), 100);
    assert!(
        !fixture
            .session
            .known_spells_like_cpp()
            .contains(&WRAPPER_LEARNED_SPELL)
    );
    assert_eq!(
        fixture.send_rx.try_recv().unwrap(),
        TrainerBuyFailed {
            trainer_guid: fixture.trainer,
            spell_id: WRAPPER_TRAINER_SPELL,
            reason: 0,
        }
        .to_bytes()
    );
    assert!(fixture.send_rx.try_recv().is_err());
}

#[tokio::test]
async fn wrapper_ignores_covered_non_immunity_aura_like_cpp() {
    let mut fixture = trainer_wrapper_fixture();
    install_wrapper_and_aura_catalog(&mut fixture.session, 999, 79, 0, 0, 0);
    seed_unclassified_active_aura(&mut fixture.session, 2);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            WRAPPER_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.player_gold_like_cpp(), 75);
    assert!(
        fixture
            .session
            .known_spells_like_cpp()
            .contains(&WRAPPER_LEARNED_SPELL)
    );
}

#[tokio::test]
async fn wrapper_ignores_effect_immunity_for_an_unrelated_effect_like_cpp() {
    let mut fixture = trainer_wrapper_fixture();
    install_wrapper_and_aura_catalog(
        &mut fixture.session,
        999,
        37, // SPELL_AURA_EFFECT_IMMUNITY
        10, // SPELL_EFFECT_HEAL, not the wrapper's learn effect
        0,
        0,
    );
    seed_unclassified_active_aura(&mut fixture.session, 2);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            WRAPPER_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.player_gold_like_cpp(), 75);
    assert!(
        fixture
            .session
            .known_spells_like_cpp()
            .contains(&WRAPPER_LEARNED_SPELL)
    );
}

#[tokio::test]
async fn wrapper_effect_immunity_removes_only_its_matching_effect_like_cpp() {
    let mut fixture = trainer_wrapper_fixture();
    let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
    let learned_id = WRAPPER_LEARNED_SPELL as u32;
    let aura_spell_id = 999;
    let learn_effect = player_learn_effect(1, wrapper_id, learned_id);
    let mut dual_wield_effect = player_learn_effect(2, wrapper_id, learned_id);
    dual_wield_effect.effect_index_raw = 1;
    dual_wield_effect.effect_type_raw = 40; // SPELL_EFFECT_DUAL_WIELD
    dual_wield_effect.effect_trigger_spell_raw = 0;
    fixture.session.set_spell_acquisition_catalog(Arc::new(
        SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            [wrapper_id, learned_id, aura_spell_id]
                .map(|spell_id| SpellAcquisitionCoverageSeedLikeCpp::covered(spell_id, 0)),
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![
                    learn_effect,
                    dual_wield_effect,
                    player_aura_effect(
                        3,
                        aura_spell_id,
                        37, // SPELL_AURA_EFFECT_IMMUNITY
                        40, // SPELL_EFFECT_DUAL_WIELD
                    ),
                ],
                ..Default::default()
            },
            SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        ),
    ));
    seed_unclassified_active_aura(&mut fixture.session, 2);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            WRAPPER_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.player_gold_like_cpp(), 75);
    assert!(
        fixture
            .session
            .known_spells_like_cpp()
            .contains(&WRAPPER_LEARNED_SPELL),
        "C++ preserves the non-immunized learn effect bit"
    );
    assert!(
        !fixture
            .session
            .mutate_canonical_player_like_cpp(|player| { player.unit().can_dual_wield_like_cpp() })
            .expect("canonical player"),
        "the immunized dual-wield effect bit must not execute"
    );
}

#[tokio::test]
async fn wrapper_matches_negative_aura_link_to_the_cast_spell_like_cpp() {
    let mut unrelated = trainer_wrapper_fixture();
    install_wrapper_and_aura_catalog(&mut unrelated.session, 999, 79, 0, 0, 0);
    install_aura_link(&mut unrelated.session, 999, -12_345);
    seed_unclassified_active_aura(&mut unrelated.session, 2);
    unrelated
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            unrelated.trainer,
            DEFAULT_TRAINER_ID as i32,
            WRAPPER_TRAINER_SPELL,
        ))
        .await;
    assert_eq!(unrelated.session.player_gold_like_cpp(), 75);

    let mut matching = trainer_wrapper_fixture();
    install_wrapper_and_aura_catalog(&mut matching.session, 999, 79, 0, 0, 0);
    install_aura_link(&mut matching.session, 999, -WRAPPER_TRAINER_SPELL);
    seed_unclassified_active_aura(&mut matching.session, 2);
    matching
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            matching.trainer,
            DEFAULT_TRAINER_ID as i32,
            WRAPPER_TRAINER_SPELL,
        ))
        .await;
    assert_eq!(matching.session.player_gold_like_cpp(), 75);
    assert!(
        !matching
            .session
            .known_spells_like_cpp()
            .contains(&WRAPPER_LEARNED_SPELL)
    );
}

#[tokio::test]
async fn wrapper_with_full_immunity_still_charges_and_publishes_visuals_like_cpp() {
    let mut fixture = trainer_wrapper_fixture();
    install_wrapper_and_aura_catalog(
        &mut fixture.session,
        999,
        37, // SPELL_AURA_EFFECT_IMMUNITY
        36, // SPELL_EFFECT_LEARN_SPELL
        0,
        0,
    );
    seed_unclassified_active_aura(&mut fixture.session, 2);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            WRAPPER_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.player_gold_like_cpp(), 75);
    assert!(
        !fixture
            .session
            .known_spells_like_cpp()
            .contains(&WRAPPER_LEARNED_SPELL)
    );
    assert_eq!(
        fixture.send_rx.try_recv().unwrap(),
        wow_packet::packets::update::UpdateObject::player_money_update(
            fixture.session.player_guid().unwrap(),
            fixture.session.player_map_id_like_cpp(),
            75,
            None,
        )
        .to_bytes()
    );
    assert_eq!(
        fixture.send_rx.try_recv().unwrap(),
        PlaySpellVisualKit {
            unit: fixture.trainer,
            kit_record_id: 179,
            kit_type: 0,
            duration: 0,
            mounted_visual: false,
        }
        .to_bytes()
    );
    assert_eq!(
        fixture.send_rx.try_recv().unwrap(),
        PlaySpellVisualKit {
            unit: fixture.session.player_guid().unwrap(),
            kit_record_id: 362,
            kit_type: 1,
            duration: 0,
            mounted_visual: false,
        }
        .to_bytes()
    );
    assert!(fixture.send_rx.try_recv().is_err());
}

#[tokio::test]
async fn wrapper_immunity_aura_preserves_its_creation_difficulty_like_cpp() {
    let store = trainer_store_from_rows(
        vec![trainer_row(DEFAULT_TRAINER_ID, 2, "Train")],
        vec![trainer_spell_row(
            DEFAULT_TRAINER_ID,
            WRAPPER_TRAINER_SPELL,
            25,
            1,
        )],
        Vec::new(),
        vec![CreatureTrainerRowLikeCpp {
            creature_id: CREATURE_ENTRY,
            trainer_id: DEFAULT_TRAINER_ID,
            menu_id: 0,
            option_id: 0,
        }],
    );
    let mut fixture = trainer_fixture_with_store_and_map_difficulty(store, 2);
    let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
    let learned_id = WRAPPER_LEARNED_SPELL as u32;
    let aura_spell_id = 999;
    let mut difficulty_immunity = player_aura_effect(
        3,
        aura_spell_id,
        37, // SPELL_AURA_EFFECT_IMMUNITY
        36, // SPELL_EFFECT_LEARN_SPELL
    );
    difficulty_immunity.difficulty_id_raw = 2;
    fixture.session.set_spell_acquisition_catalog(Arc::new(
        SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            [
                SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 0),
                SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 2),
                SpellAcquisitionCoverageSeedLikeCpp::covered(learned_id, 0),
                SpellAcquisitionCoverageSeedLikeCpp::covered(aura_spell_id, 0),
                SpellAcquisitionCoverageSeedLikeCpp::covered(aura_spell_id, 2),
            ],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![
                    player_learn_effect(1, wrapper_id, learned_id),
                    player_aura_effect(2, aura_spell_id, 79, 0),
                    difficulty_immunity,
                ],
                ..Default::default()
            },
            SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        ),
    ));
    let mut learn_skills = SpellLearnSkillStoreLikeCpp::default();
    learn_skills
        .covered_spell_ids
        .extend([wrapper_id, learned_id]);
    fixture
        .session
        .set_spell_learn_skill_store(Arc::new(learn_skills));
    fixture
        .session
        .set_spell_acquisition_static_authority_like_cpp([wrapper_id], []);
    fixture
        .session
        .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
    fixture.session.set_player_gold_like_cpp(100);
    fixture
        .session
        .set_loot_money_persistence_test_result_like_cpp(true);
    seed_unclassified_active_aura(&mut fixture.session, 2);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            WRAPPER_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.current_map_difficulty_id_like_cpp(), 2);
    assert_eq!(fixture.session.player_gold_like_cpp(), 75);
    assert!(
        fixture
            .session
            .known_spells_like_cpp()
            .contains(&WRAPPER_LEARNED_SPELL),
        "the retained difficulty-0 aura must not acquire its map-difficulty-2 immunity effect"
    );
}

#[tokio::test]
async fn wrapper_effect_no_immunity_attribute_bypasses_effect_immunity_like_cpp() {
    let mut fixture = trainer_wrapper_fixture();
    install_wrapper_and_aura_catalog(
        &mut fixture.session,
        999,
        37, // SPELL_AURA_EFFECT_IMMUNITY
        36, // SPELL_EFFECT_LEARN_SPELL
        1,  // SpellEffectAttributes::NoImmunity
        0,
    );
    seed_unclassified_active_aura(&mut fixture.session, 2);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            WRAPPER_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.player_gold_like_cpp(), 75);
    assert!(
        fixture
            .session
            .known_spells_like_cpp()
            .contains(&WRAPPER_LEARNED_SPELL)
    );
}

#[tokio::test]
async fn wrapper_spell_no_immunities_attribute_bypasses_effect_immunity_like_cpp() {
    let mut fixture = trainer_wrapper_fixture();
    install_wrapper_and_aura_catalog(
        &mut fixture.session,
        999,
        37, // SPELL_AURA_EFFECT_IMMUNITY
        36, // SPELL_EFFECT_LEARN_SPELL
        0,
        0x2000_0000, // SPELL_ATTR0_NO_IMMUNITIES
    );
    seed_unclassified_active_aura(&mut fixture.session, 2);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            WRAPPER_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.player_gold_like_cpp(), 75);
    assert!(
        fixture
            .session
            .known_spells_like_cpp()
            .contains(&WRAPPER_LEARNED_SPELL)
    );

    let mut linked_id_immunity = trainer_wrapper_fixture();
    install_wrapper_and_aura_catalog(&mut linked_id_immunity.session, 999, 37, 36, 0, 0x2000_0000);
    install_aura_link(&mut linked_id_immunity.session, 999, -WRAPPER_TRAINER_SPELL);
    seed_unclassified_active_aura(&mut linked_id_immunity.session, 2);
    linked_id_immunity
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            linked_id_immunity.trainer,
            DEFAULT_TRAINER_ID as i32,
            WRAPPER_TRAINER_SPELL,
        ))
        .await;
    assert_eq!(linked_id_immunity.session.player_gold_like_cpp(), 75);
    assert!(
        !linked_id_immunity
            .session
            .known_spells_like_cpp()
            .contains(&WRAPPER_LEARNED_SPELL)
    );
}
