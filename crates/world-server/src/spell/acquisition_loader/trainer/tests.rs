    use super::*;

    fn acquisition_effect(
        record_id: u32,
        effect_index: i64,
        effect_type: u32,
        misc_value: i64,
        trigger_spell: i64,
    ) -> SpellAcquisitionEffectLikeCpp {
        SpellAcquisitionEffectLikeCpp {
            record_id,
            spell_id_raw: 100,
            difficulty_id_raw: 0,
            effect_index_raw: effect_index,
            effect_type_raw: i64::from(effect_type),
            effect_aura_raw: 0,
            effect_mechanic_raw: 0,
            effect_attributes_raw: 0,
            effect_base_points_raw: 0,
            effect_die_sides_raw: 0,
            effect_chain_targets_raw: 0,
            effect_points_per_resource_bits: 0.0f32.to_bits(),
            effect_real_points_per_level_bits: 0.0f32.to_bits(),
            effect_coefficient_bits: 0.0f32.to_bits(),
            effect_variance_bits: 0.0f32.to_bits(),
            effect_trigger_spell_raw: trigger_spell,
            effect_item_type_raw: 0,
            effect_misc_value_raw: [misc_value, 0],
            implicit_target_raw: [0, 0],
        }
    }

    fn reagent_row(id: u32, spell_id: i32, reagents: [i32; 8]) -> SpellReagentsEntry {
        SpellReagentsEntry {
            id,
            spell_id,
            reagent: reagents,
            reagent_count: [0; 8],
        }
    }

    #[test]
    fn trainer_craft_reagents_use_db2_official_custom_then_final_removal_order() {
        let effective = compose_effective_spell_reagents_like_cpp(
            [
                reagent_row(7, 12_716, [1, 0, 0, 0, 0, 0, 0, 0]),
                reagent_row(8, 13_240, [4, 0, 0, 0, 0, 0, 0, 0]),
            ],
            [reagent_row(7, 12_716, [2, 0, 0, 0, 0, 0, 0, 0])],
            [reagent_row(7, 12_716, [3, 0, 0, 0, 0, 0, 0, 0])],
            [8],
        );

        assert_eq!(
            effective,
            BTreeMap::from([(12_716, [3, 0, 0, 0, 0, 0, 0, 0])])
        );
    }

    #[test]
    fn trainer_craft_authority_uses_effective_outputs_reagents_and_loot_zero_branch() {
        let mut create_item = acquisition_effect(1, 0, SPELL_EFFECT_CREATE_ITEM_LIKE_CPP, 0, 0);
        create_item.spell_id_raw = 700;
        create_item.effect_item_type_raw = 10_577;
        let mut missing_output = create_item.clone();
        missing_output.record_id = 2;
        missing_output.spell_id_raw = 701;
        missing_output.effect_item_type_raw = 17_771;
        let mut loot_zero = acquisition_effect(3, 0, SPELL_EFFECT_CREATE_LOOT_LIKE_CPP, 0, 0);
        loot_zero.spell_id_raw = 702;
        let mut create_zero = acquisition_effect(4, 0, SPELL_EFFECT_CREATE_ITEM_LIKE_CPP, 0, 0);
        create_zero.spell_id_raw = 703;
        let mut missing_reagent = create_item.clone();
        missing_reagent.record_id = 5;
        missing_reagent.spell_id_raw = 704;

        let effects = BTreeMap::from([
            (700, vec![create_item]),
            (701, vec![missing_output]),
            (702, vec![loot_zero]),
            (703, vec![create_zero]),
            (704, vec![missing_reagent]),
        ]);
        let reagents = BTreeMap::from([
            (700, [100, -1, 200, 0, 0, 0, 0, 0]),
            (704, [300, 0, 0, 0, 0, 0, 0, 0]),
        ]);
        let existing_items = BTreeSet::from([10_577, 100, 200]);

        assert_eq!(
            derive_valid_craft_spell_ids_like_cpp(&effects, &reagents, |item_id| {
                existing_items.contains(&item_id)
            }),
            BTreeSet::from([700, 702])
        );
    }

    #[test]
    fn trainer_craft_authority_recursively_rejects_invalid_learned_spell_and_cycles() {
        let mut parent_create = acquisition_effect(1, 0, SPELL_EFFECT_CREATE_ITEM_LIKE_CPP, 0, 0);
        parent_create.spell_id_raw = 800;
        parent_create.effect_item_type_raw = 10_577;
        let mut parent_learn = acquisition_effect(2, 1, SPELL_EFFECT_LEARN_SPELL, 0, 801);
        parent_learn.spell_id_raw = 800;
        let mut child_create = acquisition_effect(3, 0, SPELL_EFFECT_CREATE_ITEM_LIKE_CPP, 0, 0);
        child_create.spell_id_raw = 801;
        child_create.effect_item_type_raw = 17_771;

        let effects = BTreeMap::from([
            (800, vec![parent_create, parent_learn]),
            (801, vec![child_create]),
        ]);
        assert!(
            derive_valid_craft_spell_ids_like_cpp(&effects, &BTreeMap::new(), |item_id| {
                item_id == 10_577
            })
            .is_empty()
        );

        let mut cycle_create = acquisition_effect(4, 0, SPELL_EFFECT_CREATE_ITEM_LIKE_CPP, 0, 0);
        cycle_create.spell_id_raw = 900;
        cycle_create.effect_item_type_raw = 10_577;
        let mut cycle_learn = acquisition_effect(5, 1, SPELL_EFFECT_LEARN_SPELL, 0, 900);
        cycle_learn.spell_id_raw = 900;
        assert!(
            derive_valid_craft_spell_ids_like_cpp(
                &BTreeMap::from([(900, vec![cycle_create, cycle_learn])]),
                &BTreeMap::new(),
                |item_id| item_id == 10_577,
            )
            .is_empty()
        );
    }

    #[test]
    fn trainer_cast_static_world_hook_audit_fails_closed_for_every_dynamic_hook() {
        assert!(trainer_cast_world_hooks_are_static_safe_like_cpp(
            TrainerCastWorldHookAuditLikeCpp::default()
        ));

        for audit in [
            TrainerCastWorldHookAuditLikeCpp {
                script_binding: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                legacy_script: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                condition: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                aura_restriction: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                equipped_item_restriction: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                channeled: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                spell_focus_requirement: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                required_area_requirement: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                spell_area_requirement: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                linked_spell: true,
                ..Default::default()
            },
        ] {
            assert!(!trainer_cast_world_hooks_are_static_safe_like_cpp(audit));
        }
    }

    #[test]
    fn trainer_cast_requirement_audit_uses_effective_casting_requirements() {
        let mut requirements = SpellCastingRequirementsEntry {
            id: 1,
            spell_id: 100,
            facing_caster_flags: 0,
            min_faction_id: 0,
            min_reputation: 0,
            required_areas_id: 0,
            required_aura_vision: 0,
            requires_spell_focus: 7,
        };
        assert_eq!(
            trainer_cast_effective_casting_requirement_audit_like_cpp(true, Some(&requirements)),
            (true, false)
        );

        requirements.requires_spell_focus = 0;
        requirements.required_areas_id = 9;
        assert_eq!(
            trainer_cast_effective_casting_requirement_audit_like_cpp(true, Some(&requirements)),
            (false, true)
        );
        assert_eq!(
            trainer_cast_effective_casting_requirement_audit_like_cpp(false, None),
            (true, false),
            "missing SpellInfo remains outside static authority"
        );
    }

    #[test]
    fn trainer_script_bindings_preserve_cpp_signed_rank_semantics() {
        let bindings = TrainerSpellScriptBindingsLikeCpp {
            exact_spell_ids: BTreeSet::from([200]),
            all_rank_root_spell_ids: BTreeSet::from([100]),
        };

        assert!(bindings.contains_like_cpp(200, 200));
        assert!(!bindings.contains_like_cpp(201, 200));
        assert!(bindings.contains_like_cpp(101, 100));
        assert!(!bindings.contains_like_cpp(301, 300));
    }

    #[test]
    fn trainer_cast_static_restriction_audit_ignores_cpp_neutral_db2_rows() {
        let neutral_aura = SpellAuraRestrictionsEntry {
            id: 1,
            difficulty_id: 0,
            caster_aura_state: 0,
            target_aura_state: 0,
            exclude_caster_aura_state: 0,
            exclude_target_aura_state: 0,
            caster_aura_spell: 0,
            target_aura_spell: 0,
            exclude_caster_aura_spell: 0,
            exclude_target_aura_spell: 0,
            spell_id: 100,
        };
        assert!(
            !trainer_cast_has_unsupported_effective_aura_state_restriction_like_cpp(&neutral_aura)
        );

        let mut aura_spell_only = neutral_aura.clone();
        aura_spell_only.target_aura_spell = 200;
        assert!(
            !trainer_cast_has_unsupported_effective_aura_state_restriction_like_cpp(
                &aura_spell_only
            ),
            "aura-spell gates are resolved from complete runtime player aura authority"
        );

        let mut effective_aura = aura_spell_only;
        effective_aura.target_aura_state = 7;
        assert!(
            trainer_cast_has_unsupported_effective_aura_state_restriction_like_cpp(&effective_aura)
        );

        let mut raid_restriction = effective_aura.clone();
        raid_restriction.id = 2;
        raid_restriction.difficulty_id = 16;
        let restrictions = SpellAuraRestrictionsStore::from_entries([
            neutral_aura.clone(),
            raid_restriction.clone(),
        ]);
        let raid_only = SpellAuraRestrictionsStore::from_entries([raid_restriction.clone()]);
        assert!(
            !trainer_cast_has_unsupported_difficulty_none_aura_state_restriction_like_cpp(
                &raid_only, 100,
            ),
            "a nonzero-difficulty-only row does not apply to DIFFICULTY_NONE"
        );
        assert!(
            !trainer_cast_has_unsupported_difficulty_none_aura_state_restriction_like_cpp(
                &restrictions,
                100,
            ),
            "a restriction on another difficulty must not contaminate DIFFICULTY_NONE"
        );

        let mut wildcard_restriction = raid_restriction;
        wildcard_restriction.id = 3;
        wildcard_restriction.difficulty_id = u8::MAX;
        let wildcard_only =
            SpellAuraRestrictionsStore::from_entries([wildcard_restriction.clone()]);
        assert!(
            trainer_cast_has_unsupported_difficulty_none_aura_state_restriction_like_cpp(
                &wildcard_only,
                100,
            ),
            "the represented wildcard applies when no exact difficulty row exists"
        );
        let exact_over_wildcard =
            SpellAuraRestrictionsStore::from_entries([neutral_aura, wildcard_restriction]);
        assert!(
            !trainer_cast_has_unsupported_difficulty_none_aura_state_restriction_like_cpp(
                &exact_over_wildcard,
                100,
            ),
            "an exact DIFFICULTY_NONE row takes precedence over the wildcard"
        );

        let mut obsolete_restrictive = effective_aura.clone();
        obsolete_restrictive.id = 4;
        obsolete_restrictive.difficulty_id = 0;
        let mut effective_neutral = obsolete_restrictive.clone();
        effective_neutral.id = 5;
        effective_neutral.target_aura_state = 0;
        effective_neutral.target_aura_spell = 0;
        let duplicate_exact =
            SpellAuraRestrictionsStore::from_entries([obsolete_restrictive, effective_neutral]);
        assert!(
            !trainer_cast_has_unsupported_difficulty_none_aura_state_restriction_like_cpp(
                &duplicate_exact,
                100,
            ),
            "C++ later-record assignment makes only the highest-ID duplicate effective"
        );

        assert!(
            !trainer_cast_has_effective_equipped_item_restriction_like_cpp(
                &SpellEquippedItemsEntry {
                    id: 2,
                    spell_id: 100,
                    equipped_item_class: -1,
                    equipped_item_inv_types: i32::MAX,
                    equipped_item_subclass: i32::MAX,
                }
            )
        );
        assert!(
            trainer_cast_has_effective_equipped_item_restriction_like_cpp(
                &SpellEquippedItemsEntry {
                    id: 3,
                    spell_id: 100,
                    equipped_item_class: 2,
                    equipped_item_inv_types: 0,
                    equipped_item_subclass: 0,
                }
            )
        );
        let duplicate_equipped_rows = SpellEquippedItemsStore::from_entries([
            SpellEquippedItemsEntry {
                id: 3,
                spell_id: 100,
                equipped_item_class: -1,
                equipped_item_inv_types: 0,
                equipped_item_subclass: 0,
            },
            SpellEquippedItemsEntry {
                id: 9,
                spell_id: 100,
                equipped_item_class: 2,
                equipped_item_inv_types: 0,
                equipped_item_subclass: 0,
            },
        ]);
        assert!(
            duplicate_equipped_rows
                .entry_for_spell_id_like_cpp(100)
                .is_some_and(trainer_cast_has_effective_equipped_item_restriction_like_cpp),
            "the loader audit must consume C++'s highest-record-ID assignment"
        );
    }

    #[test]
    fn trainer_cast_static_effect_audit_accepts_only_player_acquisition_closure() {
        let learn = acquisition_effect(1, 0, SPELL_EFFECT_LEARN_SPELL, 0, 200);
        let skill = acquisition_effect(2, 1, SPELL_EFFECT_SKILL, 164, 0);
        let noop = acquisition_effect(3, 2, 0, 0, 0);

        assert!(trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[learn.clone(), skill, noop.clone()],
            |_| false,
        ));
        assert!(!trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[noop],
            |_| false,
        ));
    }

    #[test]
    fn trainer_cast_static_effect_audit_rejects_target_pet_aura_and_invalid_rows() {
        let mut pet_target = acquisition_effect(1, 0, SPELL_EFFECT_LEARN_SPELL, 0, 200);
        pet_target.implicit_target_raw = [5, 0];
        assert!(!trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[pet_target],
            |_| false,
        ));

        let learn = acquisition_effect(2, 1, SPELL_EFFECT_LEARN_SPELL, 0, 200);
        assert!(!trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[learn],
            |effect_index| effect_index == 1,
        ));

        let mut invalid_index = acquisition_effect(3, 256, SPELL_EFFECT_LEARN_SPELL, 0, 200);
        invalid_index.implicit_target_raw = [1, 0];
        assert!(!trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[invalid_index],
            |_| false,
        ));

        let mut mechanic = acquisition_effect(4, 0, SPELL_EFFECT_LEARN_SPELL, 0, 200);
        mechanic.effect_mechanic_raw = 3;
        assert!(!trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[mechanic],
            |_| false,
        ));

        let mut aura = acquisition_effect(5, 0, SPELL_EFFECT_LEARN_SPELL, 0, 200);
        aura.effect_aura_raw = 79;
        assert!(!trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[aura],
            |_| false,
        ));
    }

    #[test]
    fn trainer_cast_static_effect_audit_pins_only_the_audited_riding_dummy() {
        let learn = acquisition_effect(1, 0, SPELL_EFFECT_LEARN_SPELL, 0, 200);
        let dummy = acquisition_effect(2, 1, 3, 0, 0);

        assert!(trainer_cast_effects_are_static_safe_like_cpp(
            33_388,
            &[learn.clone(), dummy.clone()],
            |_| false,
        ));
        assert!(!trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[learn, dummy],
            |_| false,
        ));
    }

