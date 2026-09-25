//! Spell, skill, and pet catalog fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) fn difficulty_entry(
    id: u32,
    instance_type: u8,
    flags: DifficultyFlags,
) -> DifficultyEntry {
    DifficultyEntry {
        id,
        instance_type,
        flags: flags.bits(),
        fallback_difficulty_id: 0,
        toggle_difficulty_id: 0,
    }
}

pub(in crate::session::tests) fn test_spell_proc_entry_like_cpp(
    chance: f32,
) -> wow_data::SpellProcEntryLikeCpp {
    wow_data::SpellProcEntryLikeCpp {
        school_mask: 0,
        spell_family_name: 0,
        spell_family_mask: [0; 4],
        proc_flags: [1, 0],
        spell_type_mask: 0,
        spell_phase_mask: 0,
        hit_mask: 0,
        attributes_mask: 0,
        disable_effects_mask: 0,
        procs_per_minute: 0.0,
        chance,
        cooldown_ms: 0,
        charges: 0,
    }
}

pub(in crate::session::tests) fn test_spell_threat_entry_like_cpp(
    flat_mod: i32,
) -> wow_data::SpellThreatEntryLikeCpp {
    wow_data::SpellThreatEntryLikeCpp {
        flat_mod,
        pct_mod: 1.0,
        ap_pct_mod: 0.0,
    }
}

pub(in crate::session::tests) fn test_spell_required_store_like_cpp()
-> wow_data::SpellRequiredStoreLikeCpp {
    let outcome = wow_data::SpellRequiredStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::SpellRequiredRowLikeCpp {
                spell_id: 100,
                req_spell: 10,
            },
            wow_data::SpellRequiredRowLikeCpp {
                spell_id: 100,
                req_spell: 11,
            },
            wow_data::SpellRequiredRowLikeCpp {
                spell_id: 101,
                req_spell: 10,
            },
        ],
        |_| true,
        |_, _| false,
    );
    outcome.store
}

pub(in crate::session::tests) fn test_spell_group_store_like_cpp()
-> wow_data::SpellGroupStoreLikeCpp {
    let outcome = wow_data::SpellGroupStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::SpellGroupRowLikeCpp {
                group_id: 1001,
                spell_id: 10,
            },
            wow_data::SpellGroupRowLikeCpp {
                group_id: 1001,
                spell_id: -1002,
            },
            wow_data::SpellGroupRowLikeCpp {
                group_id: 1002,
                spell_id: 20,
            },
        ],
        |_| true,
        |_| 1,
    );
    outcome.store
}

pub(in crate::session::tests) fn test_spell_group_stack_rule_store_like_cpp(
    spell_groups: &wow_data::SpellGroupStoreLikeCpp,
) -> wow_data::SpellGroupStackRuleStoreLikeCpp {
    let outcome = wow_data::SpellGroupStackRuleStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::SpellGroupStackRuleRowLikeCpp {
                group_id: 1001,
                stack_rule: wow_data::SpellGroupStackRuleLikeCpp::ExclusiveHighest as u8,
            },
            wow_data::SpellGroupStackRuleRowLikeCpp {
                group_id: 1002,
                stack_rule: wow_data::SpellGroupStackRuleLikeCpp::ExclusiveSameEffect as u8,
            },
        ],
        spell_groups,
        |spell_id| {
            let mut spell = wow_data::SpellInfo {
                spell_id: spell_id as i32,
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
            };
            spell.effects.push(wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: 31,
                ..Default::default()
            });
            Some(spell)
        },
        |_| None,
    );
    outcome.store
}

pub(in crate::session::tests) fn test_spell_linked_store_like_cpp()
-> wow_data::SpellLinkedStoreLikeCpp {
    let outcome = wow_data::SpellLinkedStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::SpellLinkedRowLikeCpp {
                spell_trigger: 10,
                spell_effect: 20,
                link_type: 0,
            },
            wow_data::SpellLinkedRowLikeCpp {
                spell_trigger: 10,
                spell_effect: -30,
                link_type: 0,
            },
            wow_data::SpellLinkedRowLikeCpp {
                spell_trigger: -40,
                spell_effect: 50,
                link_type: 1,
            },
        ],
        |_| {
            Some(wow_data::SpellLinkedSpellInfoLikeCpp {
                effect_calc_values_by_index: Vec::new(),
            })
        },
    );
    outcome.store
}

pub(in crate::session::tests) fn test_spell_totem_model_store_like_cpp()
-> wow_data::SpellTotemModelStoreLikeCpp {
    let outcome = wow_data::SpellTotemModelStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::SpellTotemModelRowLikeCpp {
                spell_id: 50,
                race_id: 2,
                display_id: 1000,
            },
            wow_data::SpellTotemModelRowLikeCpp {
                spell_id: 50,
                race_id: 2,
                display_id: 2000,
            },
            wow_data::SpellTotemModelRowLikeCpp {
                spell_id: 50,
                race_id: 8,
                display_id: 3000,
            },
        ],
        |_| true,
        |_| true,
        |_| true,
    );
    outcome.store
}

pub(in crate::session::tests) fn test_spell_pet_aura_store_like_cpp()
-> wow_data::SpellPetAuraStoreLikeCpp {
    let outcome = wow_data::SpellPetAuraStoreLikeCpp::load_spell_pet_auras_like_cpp(
        [
            wow_data::SpellPetAuraRowLikeCpp {
                spell_id: 77,
                effect_index: 2,
                pet_entry: 0,
                aura_id: 900,
            },
            wow_data::SpellPetAuraRowLikeCpp {
                spell_id: 77,
                effect_index: 2,
                pet_entry: 501,
                aura_id: 901,
            },
        ],
        |_, _| {
            wow_data::SpellPetAuraSourceLookupLikeCpp::Found(
                wow_data::SpellPetAuraSourceEffectLikeCpp {
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUMMY,
                    apply_aura_name: 0,
                    target_a: wow_data::TARGET_UNIT_PET_LIKE_CPP,
                    calc_value: 35,
                },
            )
        },
        |_| true,
    );
    outcome.store
}

pub(in crate::session::tests) fn test_spell_area_store_like_cpp() -> wow_data::SpellAreaStoreLikeCpp
{
    let outcome = wow_data::SpellAreaStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::SpellAreaRowLikeCpp {
                spell_id: 100,
                area_id: 10,
                quest_start: 20,
                quest_start_status: 1,
                quest_end_status: 2,
                quest_end: 30,
                aura_spell: -40,
                race_mask: 1,
                gender: wow_data::GENDER_NONE_LIKE_CPP,
                flags: wow_data::SPELL_AREA_FLAG_AUTOREMOVE_LIKE_CPP,
            },
            wow_data::SpellAreaRowLikeCpp {
                spell_id: 101,
                area_id: 11,
                quest_start: 30,
                quest_start_status: 3,
                quest_end_status: 4,
                quest_end: 30,
                aura_spell: 0,
                race_mask: 0,
                gender: wow_data::GENDER_MALE_LIKE_CPP,
                flags: 0,
            },
        ],
        |_| true,
        |_| true,
        |_| true,
    );
    outcome.store
}

pub(in crate::session::tests) fn test_spell_custom_attribute_store_like_cpp()
-> wow_data::SpellCustomAttributeStoreLikeCpp {
    let outcome = wow_data::SpellCustomAttributeStoreLikeCpp::from_sql_rows_like_cpp(
        [
            wow_data::SpellCustomAttributeRowLikeCpp {
                spell_id: 100,
                attributes: wow_data::SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP,
            },
            wow_data::SpellCustomAttributeRowLikeCpp {
                spell_id: 100,
                attributes: wow_data::SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP,
            },
        ],
        |spell_id| {
            (spell_id == 100)
                .then(|| {
                    vec![
                        wow_data::SpellCustomAttributeSourceSpellInfoLikeCpp {
                            spell_id: 100,
                            difficulty: 0,
                            effects: vec![wow_data::SpellEffectInfo {
                                effect_index: 0,
                                effect:
                                    wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
                                ..Default::default()
                            }],
                        },
                        wow_data::SpellCustomAttributeSourceSpellInfoLikeCpp {
                            spell_id: 100,
                            difficulty: 2,
                            effects: vec![wow_data::SpellEffectInfo {
                                effect_index: 0,
                                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL,
                                ..Default::default()
                            }],
                        },
                    ]
                })
                .unwrap_or_default()
        },
    );
    outcome.store
}

pub(in crate::session::tests) fn test_serverside_spell_store_like_cpp()
-> wow_data::ServersideSpellStoreLikeCpp {
    let outcome = wow_data::ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [wow_data::ServersideSpellRowLikeCpp {
            spell_id: 100,
            difficulty_id: 0,
            category_id: 0,
            dispel: 0,
            mechanic: 0,
            attributes: 0,
            attributes_ex: [0; 14],
            stances: 0,
            stances_not: 0,
            targets: 0,
            target_creature_type: 0,
            requires_spell_focus: 0,
            facing_caster_flags: 0,
            caster_aura_state: 0,
            target_aura_state: 0,
            exclude_caster_aura_state: 0,
            exclude_target_aura_state: 0,
            caster_aura_spell: 0,
            target_aura_spell: 0,
            exclude_caster_aura_spell: 0,
            exclude_target_aura_spell: 0,
            caster_aura_type: 0,
            target_aura_type: 0,
            exclude_caster_aura_type: 0,
            exclude_target_aura_type: 0,
            casting_time_index: 0,
            recovery_time: 0,
            category_recovery_time: 0,
            start_recovery_category: 0,
            start_recovery_time: 0,
            interrupt_flags: 0,
            aura_interrupt_flags: [0; 2],
            channel_interrupt_flags: [0; 2],
            proc_flags: [0; 2],
            proc_chance: 0,
            proc_charges: 0,
            proc_cooldown: 0,
            proc_base_ppm: 0.0,
            max_level: 0,
            base_level: 0,
            spell_level: 0,
            duration_index: 0,
            range_index: 0,
            speed: 0.0,
            launch_delay: 0.0,
            stack_amount: 0,
            equipped_item_class: 0,
            equipped_item_sub_class_mask: 0,
            equipped_item_inventory_type_mask: 0,
            content_tuning_id: 0,
            spell_name: "server spell".to_string(),
            cone_angle: 0.0,
            cone_width: 0.0,
            max_target_level: 0,
            max_affected_targets: 0,
            spell_family_name: 0,
            spell_family_flags: [0; 4],
            dmg_class: 0,
            prevention_type: 0,
            area_group_id: 0,
            school_mask: 0,
            charge_category_id: 0,
        }],
        &wow_data::ServersideSpellEffectStoreLikeCpp::default(),
        |_| false,
    );
    outcome.store
}

pub(in crate::session::tests) fn test_spell_learn_skill_store_like_cpp()
-> wow_data::SpellLearnSkillStoreLikeCpp {
    let outcome = wow_data::SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([
        wow_data::SpellLearnSkillSourceSpellInfoLikeCpp {
            spell_id: 10,
            difficulty_none: true,
            effects: vec![wow_data::SpellLearnSkillEffectLikeCpp {
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SKILL,
                misc_value: 755,
                calc_value: 4,
            }],
        },
        wow_data::SpellLearnSkillSourceSpellInfoLikeCpp {
            spell_id: 20,
            difficulty_none: true,
            effects: vec![wow_data::SpellLearnSkillEffectLikeCpp {
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                misc_value: 0,
                calc_value: 0,
            }],
        },
    ]);
    outcome.store
}

pub(in crate::session::tests) fn test_spell_learn_skill_rank_store_like_cpp(
    skill_id: u16,
) -> wow_data::SpellLearnSkillStoreLikeCpp {
    wow_data::SpellLearnSkillStoreLikeCpp {
        skill_by_spell_id: BTreeMap::from([
            (
                10,
                wow_data::SpellLearnSkillNodeLikeCpp {
                    skill: skill_id,
                    step: 2,
                    value: 0,
                    maxvalue: 0,
                },
            ),
            (
                20,
                wow_data::SpellLearnSkillNodeLikeCpp {
                    skill: skill_id,
                    step: 3,
                    value: 0,
                    maxvalue: 0,
                },
            ),
        ]),
        ..Default::default()
    }
}

pub(in crate::session::tests) fn test_skill_line_entry_like_cpp(
    skill_id: u16,
    category_id: i8,
) -> wow_data::SkillLineEntry {
    wow_data::SkillLineEntry {
        id: u32::from(skill_id),
        display_name: String::new(),
        alternate_verb: String::new(),
        description: String::new(),
        horde_display_name: String::new(),
        override_source_info_display_name: String::new(),
        category_id,
        spell_icon_file_id: 0,
        can_link: 0,
        parent_skill_line_id: 0,
        parent_tier_index: 0,
        flags: 0,
        spell_book_spell_id: 0,
    }
}

pub(in crate::session::tests) fn test_skill_race_class_info_like_cpp(
    skill_id: u16,
    flags: u16,
    skill_tier_id: i16,
) -> wow_data::SkillRaceClassInfoRecord {
    wow_data::SkillRaceClassInfoRecord {
        id: u32::from(skill_id),
        race_mask: 0,
        skill_id,
        class_mask: 0,
        flags,
        availability: 0,
        min_level: 0,
        skill_tier_id,
    }
}

pub(in crate::session::tests) fn prepare_remove_spell_skill_range_fixture_like_cpp(
    session: &mut WorldSession,
    skill_id: u16,
    category_id: i8,
    race_class_flags: u16,
    skill_tier_id: i16,
    skill_tiers_store: wow_data::SkillTiersStoreLikeCpp,
    skill_value: u16,
    skill_max: u16,
) {
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 12, 0);
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 20,
                supercedes_spell_id: 10,
            }],
            |_| true,
        ),
    ));
    session.set_spell_learn_skill_store(Arc::new(test_spell_learn_skill_rank_store_like_cpp(
        skill_id,
    )));
    session.set_skill_line_store(Arc::new(wow_data::SkillLineStore::from_entries([
        test_skill_line_entry_like_cpp(skill_id, category_id),
    ])));
    session.set_skill_store(Arc::new(
        wow_data::SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
            std::iter::empty::<wow_data::SkillLineAbilityRecord>(),
            [test_skill_race_class_info_like_cpp(
                skill_id,
                race_class_flags,
                skill_tier_id,
            )],
        ),
    ));
    session.set_skill_tiers_store(Arc::new(skill_tiers_store));
    session.set_player_skill_records_like_cpp(HashMap::from([(
        skill_id,
        RepresentedPlayerSkillLikeCpp {
            skill_id,
            step: 3,
            value: skill_value,
            max: skill_max,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
        },
    )]));
    session.set_known_spells_like_cpp(vec![20]);
}

pub(in crate::session::tests) fn test_spell_learn_spell_store_like_cpp()
-> wow_data::SpellLearnSpellStoreLikeCpp {
    let outcome = wow_data::SpellLearnSpellStoreLikeCpp::from_sources_like_cpp(
        [wow_data::SpellLearnSpellSqlRowLikeCpp {
            entry: 10,
            spell_id: 20,
            active: true,
        }],
        [wow_data::SpellLearnSourceSpellInfoLikeCpp {
            spell_id: 30,
            difficulty_none: true,
            is_talent: false,
            is_passive: true,
            has_skill_step_effect: false,
            learn_spell_effects: vec![wow_data::SpellLearnSpellEffectLikeCpp {
                trigger_spell: 40,
                target_unit_pet: false,
            }],
        }],
        std::iter::empty::<wow_data::SpellLearnSpellEntry>(),
        |spell_id| {
            Some(wow_data::SpellLearnSourceSpellInfoLikeCpp {
                spell_id,
                difficulty_none: true,
                is_talent: false,
                is_passive: false,
                has_skill_step_effect: false,
                learn_spell_effects: Vec::new(),
            })
        },
        |_| true,
    );
    outcome.store
}

pub(in crate::session::tests) fn test_pet_levelup_spell_store_like_cpp()
-> wow_data::PetLevelupSpellStoreLikeCpp {
    let skill_store = wow_data::SkillStore::from_skill_line_abilities_like_cpp([
        wow_data::SkillLineAbilityRecord {
            id: 1,
            race_mask: 0,
            skill_line: 10,
            spell: 700,
            min_skill_line_rank: 0,
            class_mask: 0,
            supercedes_spell: 0,
            acquire_method: 2,
            trivial_rank_high: 0,
            trivial_rank_low: 0,
            flags: 0,
            num_skill_ups: 0,
            skillup_skill_line_id: 0,
        },
        wow_data::SkillLineAbilityRecord {
            id: 2,
            race_mask: 0,
            skill_line: 10,
            spell: 701,
            min_skill_line_rank: 0,
            class_mask: 0,
            supercedes_spell: 0,
            acquire_method: 2,
            trivial_rank_high: 0,
            trivial_rank_low: 0,
            flags: 0,
            num_skill_ups: 0,
            skillup_skill_line_id: 0,
        },
    ]);

    wow_data::PetLevelupSpellStoreLikeCpp::load_like_cpp(
        [wow_data::CreatureFamilyEntry {
            id: 44,
            name: String::new(),
            min_scale: 0.0,
            min_scale_level: 0,
            max_scale: 0.0,
            max_scale_level: 0,
            pet_food_mask: 0,
            pet_talent_type: 0,
            category_enum_id: 0,
            icon_file_id: 0,
            skill_line: [10, 0],
        }],
        &skill_store,
        |spell_id| match spell_id {
            700 => Some(wow_data::PetLevelupSpellInfoLikeCpp {
                id: 700,
                spell_level: 20,
            }),
            701 => Some(wow_data::PetLevelupSpellInfoLikeCpp {
                id: 701,
                spell_level: 10,
            }),
            _ => None,
        },
    )
}

pub(in crate::session::tests) fn test_pet_default_spell_store_like_cpp()
-> wow_data::PetDefaultSpellStoreLikeCpp {
    wow_data::PetDefaultSpellStoreLikeCpp::load_like_cpp(
        [wow_data::PetDefaultSpellInfoLikeCpp {
            difficulty_none: true,
            effects: vec![wow_data::PetDefaultSpellEffectLikeCpp {
                effect: 56,
                misc_value: 500,
            }],
        }],
        [wow_data::PetDefaultSpellCreatureTemplateLikeCpp {
            entry: 500,
            family: 0,
            spells: [10, 0, 11, 0],
        }],
        &wow_data::PetLevelupSpellStoreLikeCpp::default(),
    )
}

pub(in crate::session::tests) fn test_pet_family_spell_store_like_cpp()
-> wow_data::PetFamilySpellStoreLikeCpp {
    let skill_store = wow_data::SkillStore::from_skill_line_abilities_like_cpp([
        wow_data::SkillLineAbilityRecord {
            id: 1,
            race_mask: 0,
            skill_line: 10,
            spell: 800,
            min_skill_line_rank: 0,
            class_mask: 0,
            supercedes_spell: 0,
            acquire_method: 2,
            trivial_rank_high: 0,
            trivial_rank_low: 0,
            flags: 0,
            num_skill_ups: 0,
            skillup_skill_line_id: 0,
        },
        wow_data::SkillLineAbilityRecord {
            id: 2,
            race_mask: 0,
            skill_line: 10,
            spell: 801,
            min_skill_line_rank: 0,
            class_mask: 0,
            supercedes_spell: 0,
            acquire_method: 2,
            trivial_rank_high: 0,
            trivial_rank_low: 0,
            flags: 0,
            num_skill_ups: 0,
            skillup_skill_line_id: 0,
        },
    ]);

    wow_data::PetFamilySpellStoreLikeCpp::load_like_cpp(
        &skill_store,
        [wow_data::CreatureFamilyEntry {
            id: 44,
            name: String::new(),
            min_scale: 0.0,
            min_scale_level: 0,
            max_scale: 0.0,
            max_scale_level: 0,
            pet_food_mask: 0,
            pet_talent_type: 0,
            category_enum_id: 0,
            icon_file_id: 0,
            skill_line: [10, 0],
        }],
        [],
        |spell_id| match spell_id {
            800 => Some(wow_data::PetFamilySpellInfoLikeCpp {
                id: 800,
                is_passive: true,
            }),
            801 => Some(wow_data::PetFamilySpellInfoLikeCpp {
                id: 801,
                is_passive: false,
            }),
            _ => None,
        },
    )
}
