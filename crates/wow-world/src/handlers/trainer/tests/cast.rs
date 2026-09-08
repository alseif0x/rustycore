use super::*;

#[tokio::test]
async fn active_difficulty_channeled_wrapper_fails_closed_before_charge() {
    let mut fixture = trainer_wrapper_fixture();
    let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
    let learned_id = WRAPPER_LEARNED_SPELL as u32;
    fixture.session.set_spell_acquisition_catalog(Arc::new(
        SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            [wrapper_id, learned_id]
                .map(|spell_id| SpellAcquisitionCoverageSeedLikeCpp::covered(spell_id, 0)),
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![player_learn_effect(1, wrapper_id, learned_id)],
                spell_misc: vec![SpellAcquisitionMiscLikeCpp {
                    record_id: 2,
                    spell_id_raw: i64::from(wrapper_id),
                    difficulty_id_raw: 0,
                    // SPELL_ATTR1_IS_CHANNELLED
                    attributes_raw: [0, 0x0000_0004],
                    show_future_spell_player_condition_id_raw: 0,
                }],
                ..Default::default()
            },
            SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        ),
    ));

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
async fn trainer_wrapper_consumes_active_difficulty_effect_rows_like_cpp() {
    const HEROIC_LEARNED_SPELL: u32 = 54_326;
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
    let normal_learned_id = WRAPPER_LEARNED_SPELL as u32;
    let mut heroic_effect = player_learn_effect(2, wrapper_id, HEROIC_LEARNED_SPELL);
    heroic_effect.difficulty_id_raw = 2;
    fixture.session.set_spell_acquisition_catalog(Arc::new(
        SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            [
                SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 0),
                SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 2),
                SpellAcquisitionCoverageSeedLikeCpp::covered(normal_learned_id, 0),
                SpellAcquisitionCoverageSeedLikeCpp::covered(HEROIC_LEARNED_SPELL, 0),
            ],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![
                    player_learn_effect(1, wrapper_id, normal_learned_id),
                    heroic_effect,
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
        .extend([wrapper_id, normal_learned_id, HEROIC_LEARNED_SPELL]);
    fixture
        .session
        .set_spell_learn_skill_store(Arc::new(learn_skills));
    fixture
        .session
        .set_spell_acquisition_static_authority_like_cpp([wrapper_id], []);
    fixture
        .session
        .set_spell_target_restrictions_store(Arc::new(
            wow_data::SpellTargetRestrictionsStore::from_entries([
                spell_target_restriction_row(1, wrapper_id, 0, 1 << (3 - 1)),
                spell_target_restriction_row(2, wrapper_id, 2, 1 << (7 - 1)),
            ]),
        ));
    fixture
        .session
        .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
    fixture.session.set_player_gold_like_cpp(100);
    fixture
        .session
        .set_loot_money_persistence_test_result_like_cpp(true);

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
            .contains(&(HEROIC_LEARNED_SPELL as i32))
    );
    assert!(
        !fixture
            .session
            .known_spells_like_cpp()
            .contains(&WRAPPER_LEARNED_SPELL)
    );
}

#[tokio::test]
async fn active_difficulty_pet_aura_hook_fails_closed_before_charge() {
    const HEROIC_LEARNED_SPELL: u32 = 54_327;
    let mut fixture = trainer_wrapper_fixture_with_map_difficulty(2);
    let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
    let learned_id = WRAPPER_LEARNED_SPELL as u32;
    let mut heroic_effect = player_learn_effect(2, wrapper_id, HEROIC_LEARNED_SPELL);
    heroic_effect.difficulty_id_raw = 2;
    heroic_effect.effect_index_raw = 1;
    fixture.session.set_spell_acquisition_catalog(Arc::new(
        SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            [
                SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 0),
                SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 2),
                SpellAcquisitionCoverageSeedLikeCpp::covered(learned_id, 0),
                SpellAcquisitionCoverageSeedLikeCpp::covered(HEROIC_LEARNED_SPELL, 0),
            ],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![
                    player_learn_effect(1, wrapper_id, learned_id),
                    heroic_effect,
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
        .extend([wrapper_id, learned_id, HEROIC_LEARNED_SPELL]);
    fixture
        .session
        .set_spell_learn_skill_store(Arc::new(learn_skills));
    let pet_auras = wow_data::SpellPetAuraStoreLikeCpp::load_spell_pet_auras_like_cpp(
        [wow_data::SpellPetAuraRowLikeCpp {
            spell_id: wrapper_id,
            effect_index: 1,
            pet_entry: 0,
            aura_id: 90_001,
        }],
        |_, _| {
            wow_data::SpellPetAuraSourceLookupLikeCpp::Found(
                wow_data::SpellPetAuraSourceEffectLikeCpp {
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUMMY,
                    apply_aura_name: 0,
                    target_a: wow_data::TARGET_UNIT_PET_LIKE_CPP,
                    calc_value: 0,
                },
            )
        },
        |_| true,
    );
    assert_eq!(pet_auras.loaded_row_count, 1);
    fixture
        .session
        .set_spell_pet_aura_store(Arc::new(pet_auras.store));

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
            .contains(&(HEROIC_LEARNED_SPELL as i32))
    );
    while fixture.send_rx.try_recv().is_ok() {}

    let (disable_mgr, report) = wow_data::DisableMgrLikeCpp::from_rows_like_cpp(
        [wow_data::DisableDbRowLikeCpp {
            source_type: wow_data::DISABLE_TYPE_SPELL,
            entry: wrapper_id,
            flags: wow_data::disable_mgr::SPELL_DISABLE_PLAYER,
            params_0: String::new(),
            params_1: String::new(),
        }],
        wow_data::DisableMgrRefsLikeCpp {
            spell_store: fixture.session.spell_store().map(AsRef::as_ref),
            ..Default::default()
        },
    );
    assert_eq!(report.loaded_count, 1);
    fixture.session.set_disable_mgr(Arc::new(disable_mgr));
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
            .contains(&(HEROIC_LEARNED_SPELL as i32))
    );
    assert_trainer_charge_and_visuals_like_cpp(&mut fixture);
}

#[tokio::test]
async fn player_disable_stops_cast_effects_after_charge_and_visuals_like_cpp() {
    let mut fixture = trainer_wrapper_fixture();
    let (disable_mgr, report) = wow_data::DisableMgrLikeCpp::from_rows_like_cpp(
        [wow_data::DisableDbRowLikeCpp {
            source_type: wow_data::DISABLE_TYPE_SPELL,
            entry: WRAPPER_TRAINER_SPELL as u32,
            flags: wow_data::disable_mgr::SPELL_DISABLE_PLAYER,
            params_0: String::new(),
            params_1: String::new(),
        }],
        wow_data::DisableMgrRefsLikeCpp {
            spell_store: fixture.session.spell_store().map(AsRef::as_ref),
            ..Default::default()
        },
    );
    assert_eq!(report.loaded_count, 1);
    fixture.session.set_disable_mgr(Arc::new(disable_mgr));

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
async fn active_difficulty_aura_restriction_fails_after_charge_like_cpp() {
    let mut fixture = trainer_wrapper_fixture_with_map_difficulty(2);
    let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
    let learned_id = WRAPPER_LEARNED_SPELL as u32;
    let mut active_effect = player_learn_effect(2, wrapper_id, learned_id);
    active_effect.difficulty_id_raw = 2;
    fixture.session.set_spell_acquisition_catalog(Arc::new(
        SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            [
                SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 0),
                SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 2),
                SpellAcquisitionCoverageSeedLikeCpp::covered(learned_id, 0),
            ],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![
                    player_learn_effect(1, wrapper_id, learned_id),
                    active_effect,
                ],
                ..Default::default()
            },
            SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        ),
    ));
    fixture
        .session
        .set_spell_acquisition_static_authority_like_cpp([wrapper_id], []);
    fixture.session.set_spell_aura_restrictions_store(Arc::new(
        wow_data::SpellAuraRestrictionsStore::from_entries([
            wow_data::SpellAuraRestrictionsEntry {
                id: 1,
                difficulty_id: 2,
                caster_aura_state: 0,
                target_aura_state: 0,
                exclude_caster_aura_state: 0,
                exclude_target_aura_state: 0,
                caster_aura_spell: 999,
                target_aura_spell: 0,
                exclude_caster_aura_spell: 0,
                exclude_target_aura_spell: 0,
                spell_id: wrapper_id,
            },
        ]),
    ));
    assert_eq!(
        fixture
            .session
            .resolved_player_aura_authority_complete_like_cpp(),
        Some(true),
        "trainer fixture must accredit the canonical Player aura load"
    );
    assert!(
        fixture
            .session
            .resolved_player_visible_auras_like_cpp()
            .is_some(),
        "trainer fixture must resolve the canonical Player aura owner"
    );
    fixture
        .session
        .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
    fixture.session.set_player_gold_like_cpp(100);
    fixture
        .session
        .set_loot_money_persistence_test_result_like_cpp(true);

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
    assert_trainer_charge_and_visuals_like_cpp(&mut fixture);
}

#[tokio::test]
async fn incompatible_wrapper_self_target_fails_after_charge_like_cpp() {
    let mut fixture = trainer_wrapper_fixture();
    fixture
        .session
        .set_spell_target_restrictions_store(Arc::new(
            wow_data::SpellTargetRestrictionsStore::from_entries([spell_target_restriction_row(
                1,
                WRAPPER_TRAINER_SPELL as u32,
                0,
                1 << (3 - 1), // CREATURE_TYPEMASK_BEAST, not player/humanoid
            )]),
        ));

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
    assert_trainer_charge_and_visuals_like_cpp(&mut fixture);
}

#[tokio::test]
async fn definite_target_failure_precedes_unsupported_pet_aura_hook_like_cpp() {
    let mut fixture = trainer_wrapper_fixture();
    let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
    let pet_auras = wow_data::SpellPetAuraStoreLikeCpp::load_spell_pet_auras_like_cpp(
        [wow_data::SpellPetAuraRowLikeCpp {
            spell_id: wrapper_id,
            effect_index: 0,
            pet_entry: 0,
            aura_id: 90_002,
        }],
        |_, _| {
            wow_data::SpellPetAuraSourceLookupLikeCpp::Found(
                wow_data::SpellPetAuraSourceEffectLikeCpp {
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUMMY,
                    apply_aura_name: 0,
                    target_a: wow_data::TARGET_UNIT_PET_LIKE_CPP,
                    calc_value: 0,
                },
            )
        },
        |_| true,
    );
    assert_eq!(pet_auras.loaded_row_count, 1);
    fixture
        .session
        .set_spell_pet_aura_store(Arc::new(pet_auras.store));
    fixture
        .session
        .set_spell_target_restrictions_store(Arc::new(
            wow_data::SpellTargetRestrictionsStore::from_entries([spell_target_restriction_row(
                1,
                wrapper_id,
                0,
                1 << (3 - 1), // CREATURE_TYPEMASK_BEAST, not player/humanoid
            )]),
        ));

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
    assert_trainer_charge_and_visuals_like_cpp(&mut fixture);
}

#[tokio::test]
async fn incomplete_persisted_aura_authority_stops_before_charge_like_cpp() {
    let mut fixture = trainer_wrapper_fixture();
    fixture
        .session
        .set_player_aura_authority_complete_like_cpp(false);

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
}

#[tokio::test]
async fn creature_disable_does_not_block_the_player_cast_like_cpp() {
    let mut fixture = trainer_wrapper_fixture();
    let (disable_mgr, report) = wow_data::DisableMgrLikeCpp::from_rows_like_cpp(
        [wow_data::DisableDbRowLikeCpp {
            source_type: wow_data::DISABLE_TYPE_SPELL,
            entry: WRAPPER_TRAINER_SPELL as u32,
            flags: wow_data::disable_mgr::SPELL_DISABLE_CREATURE,
            params_0: String::new(),
            params_1: String::new(),
        }],
        wow_data::DisableMgrRefsLikeCpp {
            spell_store: fixture.session.spell_store().map(AsRef::as_ref),
            ..Default::default()
        },
    );
    assert_eq!(report.loaded_count, 1);
    fixture.session.set_disable_mgr(Arc::new(disable_mgr));

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
async fn map_scoped_player_disable_only_blocks_matching_trainer_map_like_cpp() {
    for (disabled_map, expected_known) in [(0_u32, false), (1_u32, true)] {
        let mut fixture = trainer_wrapper_fixture();
        let (disable_mgr, report) = wow_data::DisableMgrLikeCpp::from_rows_like_cpp(
            [wow_data::DisableDbRowLikeCpp {
                source_type: wow_data::DISABLE_TYPE_SPELL,
                entry: WRAPPER_TRAINER_SPELL as u32,
                flags: wow_data::disable_mgr::SPELL_DISABLE_PLAYER
                    | wow_data::disable_mgr::SPELL_DISABLE_MAP,
                params_0: disabled_map.to_string(),
                params_1: String::new(),
            }],
            wow_data::DisableMgrRefsLikeCpp {
                spell_store: fixture.session.spell_store().map(AsRef::as_ref),
                ..Default::default()
            },
        );
        assert_eq!(report.loaded_count, 1);
        fixture.session.set_disable_mgr(Arc::new(disable_mgr));

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert_eq!(
            fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL),
            expected_known
        );
    }
}

#[tokio::test]
async fn trainer_player_cast_applies_caster_owned_skill_like_cpp() {
    let mut fixture = trainer_wrapper_fixture();
    let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
    let mut skill_effect = player_learn_effect(1, wrapper_id, 0);
    skill_effect.effect_type_raw =
        i64::from(wow_data::spell::spell_effect_types::SPELL_EFFECT_SKILL);
    skill_effect.effect_index_raw = 1;
    skill_effect.effect_trigger_spell_raw = 0;
    skill_effect.effect_misc_value_raw[0] = 164;
    skill_effect.effect_base_points_raw = 1;
    skill_effect.implicit_target_raw = [0, 0];
    fixture.session.set_spell_acquisition_catalog(Arc::new(
        SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            [
                SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 0),
                SpellAcquisitionCoverageSeedLikeCpp::covered(WRAPPER_LEARNED_SPELL as u32, 0),
            ],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![
                    player_learn_effect(2, wrapper_id, WRAPPER_LEARNED_SPELL as u32),
                    skill_effect,
                ],
                ..Default::default()
            },
            SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        ),
    ));
    fixture.session.set_skill_store(Arc::new(
        SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
            [],
            [SkillRaceClassInfoRecord {
                id: 1,
                race_mask: 0,
                skill_id: 164,
                class_mask: 0,
                flags: 0,
                availability: 1,
                min_level: 1,
                skill_tier_id: 1,
            }],
        ),
    ));
    fixture
        .session
        .set_skill_line_store(Arc::new(SkillLineStore::from_entries([SkillLineEntry {
            id: 164,
            display_name: "Blacksmithing".to_string(),
            alternate_verb: String::new(),
            description: String::new(),
            horde_display_name: String::new(),
            override_source_info_display_name: String::new(),
            category_id: wow_data::skill::SKILL_CATEGORY_SECONDARY_LIKE_CPP,
            spell_icon_file_id: 0,
            can_link: 0,
            parent_skill_line_id: 0,
            parent_tier_index: 0,
            flags: 0,
            spell_book_spell_id: 0,
        }])));
    let mut tier_values = [0; wow_data::MAX_SKILL_STEP_LIKE_CPP];
    tier_values[0] = 75;
    fixture
        .session
        .set_skill_tiers_store(Arc::new(SkillTiersStoreLikeCpp::from_rows_like_cpp([
            SkillTiersRowLikeCpp {
                id: 1,
                value: tier_values,
            },
        ])));

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            WRAPPER_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.player_gold_like_cpp(), 75);
    let skill = &fixture.session.player_skill_records_like_cpp()[&164];
    assert_eq!((skill.step, skill.value, skill.max), (1, 1, 75));
    assert!(
        fixture
            .session
            .known_spells_like_cpp()
            .contains(&WRAPPER_LEARNED_SPELL)
    );
}
