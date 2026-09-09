//! Skill tier and pet spell stores regression scenarios, part 2 of 2.
//!
//! Moved out of the skill.rs root under #638; every test is unchanged.

use super::*;

#[test]
fn default_death_knight_skill_uses_level_minus_one_value_like_cpp() {
    let store = SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
        std::iter::empty(),
        [SkillRaceClassInfoRecord {
            class_mask: 1 << (CLASS_DEATH_KNIGHT_LIKE_CPP - 1),
            ..race_class_info(1, 200, 0, 1, 1, 0)
        }],
    );
    let skill_lines = SkillLineStore::from_entries([skill_line(200, 0)]);

    assert_eq!(
        store.default_starting_skill_info_like_cpp(
            1,
            CLASS_DEATH_KNIGHT_LIKE_CPP,
            58,
            &skill_lines,
            &SkillTiersStoreLikeCpp::default(),
        )[0]
        .rank,
        285
    );
}

#[test]
fn loaded_skill_info_applies_cpp_fixed_ranges_and_steps() {
    let store = SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
        std::iter::empty(),
        [
            race_class_info(1, 300, 0, 0, 0, 0),
            race_class_info(2, 301, SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP, 0, 0, 0),
        ],
    );
    let skill_lines = SkillLineStore::from_entries([
        skill_line(300, SKILL_CATEGORY_LANGUAGES_LIKE_CPP),
        skill_line(301, SKILL_CATEGORY_SECONDARY_LIKE_CPP),
    ]);
    let tiers = SkillTiersStoreLikeCpp::default();

    assert_eq!(
        store.loaded_skill_info_like_cpp(300, 1, 1, 10, 12, 25, &skill_lines, &tiers),
        Some(SkillInfoEntry {
            skill_id: 300,
            step: 0,
            rank: 300,
            starting_rank: 1,
            max_rank: 300,
            temp_bonus: 0,
            perm_bonus: 0,
        })
    );
    assert_eq!(
        store.loaded_skill_info_like_cpp(301, 1, 1, 80, 12, 25, &skill_lines, &tiers),
        Some(SkillInfoEntry {
            skill_id: 301,
            step: 5,
            rank: 400,
            starting_rank: 1,
            max_rank: 400,
            temp_bonus: 0,
            perm_bonus: 0,
        })
    );
}

#[test]
fn loaded_profession_step_uses_cpp_max_div_75_even_for_nonstandard_tier() {
    let store = SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
        std::iter::empty(),
        [race_class_info(1, 301, 0, 0, 0, 12)],
    );
    let skill_lines =
        SkillLineStore::from_entries([skill_line(301, SKILL_CATEGORY_PROFESSION_LIKE_CPP)]);
    let tiers = SkillTiersStoreLikeCpp::from_rows_like_cpp([skill_tier_row(
        12,
        [73, 181, 400, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    )]);

    let loaded = store
        .loaded_skill_info_like_cpp(301, 1, 1, 80, 12, 400, &skill_lines, &tiers)
        .expect("the persisted profession is valid for this race/class");

    assert_eq!(
        loaded.step, 5,
        "pinned C++ _LoadSkills uses max / 75, not the matching custom tier index"
    );
}

#[test]
fn loaded_zero_value_skill_is_retained_for_cpp_default_reactivation() {
    let store = SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
        std::iter::empty(),
        [race_class_info(1, 301, 0, 0, 0, 0)],
    );
    let skill_lines =
        SkillLineStore::from_entries([skill_line(301, SKILL_CATEGORY_PROFESSION_LIKE_CPP)]);

    let loaded = store
        .loaded_skill_info_like_cpp(
            301,
            1,
            1,
            80,
            0,
            400,
            &skill_lines,
            &SkillTiersStoreLikeCpp::default(),
        )
        .expect("C++ _LoadSkills keeps the zero-valued status/update-field entry");

    assert_eq!(loaded.rank, 0);
    assert_eq!(loaded.max_rank, 400);
    assert_eq!(loaded.step, 5);
}

#[test]
fn skill_tier_value_clamps_large_index_like_cpp() {
    let tier = SkillTiersEntryLikeCpp {
        id: 1,
        value: [75, 150, 225, 300, 375, 450, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    };

    assert_eq!(tier.get_value_for_tier_index_like_cpp(99), 450);
}

#[test]
fn skill_line_ability_map_bounds_group_by_spell_like_cpp() {
    let store = SkillStore::from_skill_line_abilities_like_cpp([
        ability(1, 56, 585),
        ability(2, 56, 2050),
        ability(3, 78, 585),
    ]);

    let smite_bounds = store.get_skill_line_ability_map_bounds_like_cpp(585);
    assert_eq!(smite_bounds.len(), 2);
    assert_eq!(smite_bounds[0].id, 1);
    assert_eq!(smite_bounds[1].id, 3);
    assert_eq!(
        store
            .get_skill_line_ability_map_bounds_like_cpp(2050)
            .iter()
            .map(|ability| ability.id)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert!(
        store
            .get_skill_line_ability_map_bounds_like_cpp(999)
            .is_empty()
    );
}

#[test]
fn skill_line_ability_map_bounds_preserve_cpp_multimap_duplicates() {
    let store = SkillStore::from_skill_line_abilities_like_cpp([
        ability(10, 100, 777),
        ability(11, 100, 777),
    ]);

    assert_eq!(
        store
            .get_skill_line_ability_map_bounds_like_cpp(777)
            .iter()
            .map(|ability| ability.id)
            .collect::<Vec<_>>(),
        vec![10, 11],
        "C++ mSkillLineAbilityMap is a multimap and preserves every inserted row"
    );
}

#[test]
fn pet_levelup_spell_map_filters_like_cpp() {
    let skill_store = SkillStore::from_skill_line_abilities_like_cpp([
        pet_ability(
            1,
            10,
            1000,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
        pet_ability(2, 10, 1001, 1),
        pet_ability(
            3,
            10,
            1002,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
        pet_ability(
            4,
            10,
            1003,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
        pet_ability(
            5,
            20,
            2000,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
    ]);

    let store = PetLevelupSpellStoreLikeCpp::load_like_cpp(
        [creature_family(42, [10, 20]), creature_family(77, [0, 99])],
        &skill_store,
        |spell_id| match spell_id {
            1000 => Some(PetLevelupSpellInfoLikeCpp {
                id: 1000,
                spell_level: 4,
            }),
            1002 => None,
            1003 => Some(PetLevelupSpellInfoLikeCpp {
                id: 1003,
                spell_level: 0,
            }),
            2000 => Some(PetLevelupSpellInfoLikeCpp {
                id: 2000,
                spell_level: 7,
            }),
            _ => panic!("unexpected spell lookup {spell_id}"),
        },
    );

    let pet_family_42 = store
        .get_pet_levelup_spell_list_like_cpp(42)
        .expect("family should have levelup spells");
    assert_eq!(
        pet_family_42.iter().collect::<Vec<_>>(),
        vec![(4, 1000), (7, 2000)]
    );
    assert_eq!(store.count(), 2);
    assert_eq!(store.family_count(), 1);
    assert!(store.get_pet_levelup_spell_list_like_cpp(77).is_none());
}

#[test]
fn pet_levelup_spell_map_orders_like_cpp_multimap_by_spell_level() {
    let skill_store = SkillStore::from_skill_line_abilities_like_cpp([
        pet_ability(
            1,
            10,
            3000,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
        pet_ability(
            2,
            10,
            3001,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
        pet_ability(
            3,
            10,
            3002,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
    ]);

    let store = PetLevelupSpellStoreLikeCpp::load_like_cpp(
        [creature_family(42, [10, 0])],
        &skill_store,
        |spell_id| {
            let spell_level = match spell_id {
                3000 => 20,
                3001 => 10,
                3002 => 20,
                _ => unreachable!(),
            };
            Some(PetLevelupSpellInfoLikeCpp {
                id: spell_id as u32,
                spell_level,
            })
        },
    );

    assert_eq!(
        store
            .get_pet_levelup_spell_list_like_cpp(42)
            .expect("family should have levelup spells")
            .iter()
            .collect::<Vec<_>>(),
        vec![(10, 3001), (20, 3000), (20, 3002)],
        "C++ PetLevelupSpellSet is a multimap keyed by SpellLevel"
    );
}

#[test]
fn pet_default_spells_loads_summon_templates_like_cpp() {
    let levelup_spells = PetLevelupSpellStoreLikeCpp::default();
    let store = PetDefaultSpellStoreLikeCpp::load_like_cpp(
        [
            summon_spell(true, SPELL_EFFECT_SUMMON_LIKE_CPP, 500),
            summon_spell(true, SPELL_EFFECT_SUMMON_PET_LIKE_CPP, 501),
            summon_spell(false, SPELL_EFFECT_SUMMON_LIKE_CPP, 502),
            summon_spell(true, 2, 503),
            summon_spell(true, SPELL_EFFECT_SUMMON_LIKE_CPP, 999),
        ],
        [
            pet_default_template(500, 0, [10, 0, 11, 0]),
            pet_default_template(501, 0, [20, 21, 0, 0]),
            pet_default_template(502, 0, [30, 0, 0, 0]),
            pet_default_template(503, 0, [40, 0, 0, 0]),
        ],
        &levelup_spells,
    );

    assert_eq!(store.count(), 2);
    assert_eq!(
        store
            .get_pet_default_spells_entry_like_cpp(500)
            .expect("summon creature template should be loaded")
            .spellid,
        [10, 0, 11, 0]
    );
    assert_eq!(
        store
            .get_pet_default_spells_entry_like_cpp(501)
            .expect("summon pet creature template should be loaded")
            .spellid,
        [20, 21, 0, 0]
    );
    assert!(store.get_pet_default_spells_entry_like_cpp(502).is_none());
    assert!(store.get_pet_default_spells_entry_like_cpp(503).is_none());
    assert!(store.get_pet_default_spells_entry_like_cpp(999).is_none());
}

#[test]
fn pet_default_spells_removes_levelup_duplicates_like_cpp() {
    let skill_store = SkillStore::from_skill_line_abilities_like_cpp([
        pet_ability(
            1,
            10,
            100,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
        pet_ability(
            2,
            10,
            101,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
    ]);
    let levelup_spells = PetLevelupSpellStoreLikeCpp::load_like_cpp(
        [creature_family(7, [10, 0])],
        &skill_store,
        |spell_id| {
            Some(PetLevelupSpellInfoLikeCpp {
                id: spell_id as u32,
                spell_level: 1,
            })
        },
    );

    let store = PetDefaultSpellStoreLikeCpp::load_like_cpp(
        [summon_spell(true, SPELL_EFFECT_SUMMON_PET_LIKE_CPP, 500)],
        [pet_default_template(500, 7, [100, 999, 101, 0])],
        &levelup_spells,
    );

    assert_eq!(
        store
            .get_pet_default_spells_entry_like_cpp(500)
            .expect("non-levelup default spell keeps entry alive")
            .spellid,
        [0, 999, 0, 0]
    );
}

#[test]
fn pet_default_spells_skips_empty_after_levelup_duplicate_removal_like_cpp() {
    let skill_store = SkillStore::from_skill_line_abilities_like_cpp([pet_ability(
        1,
        10,
        100,
        SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
    )]);
    let levelup_spells = PetLevelupSpellStoreLikeCpp::load_like_cpp(
        [creature_family(7, [10, 0])],
        &skill_store,
        |_| {
            Some(PetLevelupSpellInfoLikeCpp {
                id: 100,
                spell_level: 1,
            })
        },
    );

    let store = PetDefaultSpellStoreLikeCpp::load_like_cpp(
        [summon_spell(true, SPELL_EFFECT_SUMMON_PET_LIKE_CPP, 500)],
        [pet_default_template(500, 7, [100, 0, 0, 0])],
        &levelup_spells,
    );

    assert_eq!(store.count(), 0);
    assert!(store.get_pet_default_spells_entry_like_cpp(500).is_none());
}

#[test]
fn pet_family_spells_store_filters_like_cpp() {
    let skill_store = SkillStore::from_skill_line_abilities_like_cpp([
        pet_ability(
            1,
            10,
            100,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
        pet_ability(
            2,
            20,
            101,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
        pet_ability(3, 10, 102, 1),
        pet_ability(
            4,
            10,
            103,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
        pet_ability(
            5,
            10,
            104,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
        pet_ability(
            6,
            99,
            105,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
    ]);

    let store = PetFamilySpellStoreLikeCpp::load_like_cpp(
        &skill_store,
        [creature_family(7, [10, 20]), creature_family(8, [30, 0])],
        [
            PetFamilySpellLevelLikeCpp {
                spell_id: 100,
                difficulty_id: 0,
                spell_level: 0,
            },
            PetFamilySpellLevelLikeCpp {
                spell_id: 101,
                difficulty_id: 1,
                spell_level: 80,
            },
            PetFamilySpellLevelLikeCpp {
                spell_id: 103,
                difficulty_id: 0,
                spell_level: 5,
            },
        ],
        |spell_id| match spell_id {
            100 | 101 | 103 | 105 => Some(PetFamilySpellInfoLikeCpp {
                id: spell_id as u32,
                is_passive: true,
            }),
            102 => Some(PetFamilySpellInfoLikeCpp {
                id: 102,
                is_passive: true,
            }),
            104 => Some(PetFamilySpellInfoLikeCpp {
                id: 104,
                is_passive: false,
            }),
            _ => None,
        },
    );

    assert_eq!(
        store.get_pet_family_spells_like_cpp(7),
        Some(vec![100, 101]),
        "difficulty-specific SpellLevels rows do not exclude the DIFFICULTY_NONE lookup"
    );
    assert_eq!(store.family_count(), 1);
    assert_eq!(store.spell_count(), 2);
    assert!(store.get_pet_family_spells_like_cpp(8).is_none());
}

#[test]
fn pet_family_spells_store_deduplicates_and_orders_like_cpp_set() {
    let skill_store = SkillStore::from_skill_line_abilities_like_cpp([
        pet_ability(
            1,
            10,
            300,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
        pet_ability(
            2,
            10,
            200,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
        pet_ability(
            3,
            10,
            300,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        ),
    ]);

    let store = PetFamilySpellStoreLikeCpp::load_like_cpp(
        &skill_store,
        [creature_family(7, [10, 0])],
        [],
        |spell_id| {
            Some(PetFamilySpellInfoLikeCpp {
                id: spell_id as u32,
                is_passive: true,
            })
        },
    );

    assert_eq!(
        store.get_pet_family_spells_like_cpp(7),
        Some(vec![200, 300]),
        "C++ PetFamilySpellsSet is std::set<uint32>"
    );
}

#[test]
fn test_load_skill_store() {
    let store = match load_store() {
        Some(s) => s,
        None => return,
    };
    assert!(
        store.ability_count() > 1000,
        "expected >1000 abilities, got {}",
        store.ability_count()
    );
    assert!(
        store.skill_count() > 100,
        "expected >100 skills, got {}",
        store.skill_count()
    );
    assert!(
        store.race_class_count() > 100,
        "expected >100 race/class entries, got {}",
        store.race_class_count()
    );
}

#[test]
fn live_hunter_starting_skill_rewards_match_cpp_capture() {
    let Some(store) = load_store() else {
        return;
    };
    let spell_names =
        crate::SpellNameStore::load(DATA_DIR, LOCALE).expect("failed to load SpellNameStore");
    let spell_levels =
        crate::SpellLevelsStore::load(DATA_DIR, LOCALE).expect("failed to load SpellLevelsStore");
    let mut learned = Vec::new();

    for (skill_id, skill_value) in [
        (45, 1),
        (51, 15),
        (95, 300),
        (109, 300),
        (137, 300),
        (162, 1),
        (163, 15),
        (172, 1),
        (173, 1),
        (183, 15),
        (414, 1),
        (415, 1),
        (756, 15),
        (777, 1),
    ] {
        learned.extend(
            store
                .skill_rewarded_spell_changes_like_cpp(
                    skill_id,
                    skill_value,
                    10,
                    3,
                    3,
                    |spell_id| {
                        let spell_id = u32::try_from(spell_id).ok()?;
                        spell_names.get(spell_id)?;
                        spell_levels
                            .entry_for_spell_difficulty_like_cpp(spell_id, 0)
                            .map(|entry| {
                                (
                                    u32::try_from(entry.base_level).unwrap_or(0),
                                    u32::try_from(entry.spell_level).unwrap_or(0),
                                )
                            })
                            .or(Some((0, 0)))
                    },
                    |_| false,
                )
                .learn,
        );
    }

    learned.sort_unstable();
    learned.dedup();
    assert_eq!(
        learned,
        vec![
            75, 81, 197, 203, 204, 264, 522, 669, 813, 822, 1180, 2382, 2973, 3050, 3365, 6233,
            6246, 6247, 6477, 6478, 6603, 7266, 7267, 7355, 8386, 9077, 9078, 9125, 13358, 21651,
            21652, 22027, 22810, 24949, 28730, 28877, 34082, 45927, 61437, 63644, 63645, 68398,
            349794,
        ],
        "C++ Player::LearnSkillRewardedSpells trace for TESTBOT1 guid 14"
    );
}

#[test]
fn test_matches_race() {
    assert!(matches_race(0, 1)); // mask=0 matches all
    assert!(matches_race(0, 5));
    assert!(matches_race(1, 1)); // bit 0 = race 1 (Human)
    assert!(!matches_race(1, 2)); // bit 0 only matches race 1
    assert!(matches_race(0b11, 2)); // bit 1 = race 2 (Orc)
}

#[test]
fn test_matches_class() {
    assert!(matches_class(0, 1)); // mask=0 matches all
    assert!(matches_class(0, 9));
    assert!(matches_class(1, 1)); // bit 0 = class 1 (Warrior)
    assert!(!matches_class(1, 2)); // bit 0 only matches class 1
}

#[test]
fn test_field_mapping_verified() {
    let dbc_dir = Path::new(DATA_DIR).join("dbc").join(LOCALE);
    let sla_path = dbc_dir.join("SkillLineAbility.db2");
    if !sla_path.exists() {
        eprintln!("Skipping: SkillLineAbility.db2 not found");
        return;
    }
    let sla = Wdc4Reader::open(&sla_path).unwrap();

    // Record 246: one-handed axes (spell=264) for skill line 45.
    // This DB2 has no external ID list: C++ reads its inline field[1].
    let idx = sla
        .get_record_index(246)
        .expect("inline SkillLineAbility ID 246 must be indexed");
    assert_eq!(
        sla.get_field_u32(idx, 1),
        246,
        "field[1] should be the inline record ID"
    );
    assert_eq!(
        sla.get_field_i32(idx, 2),
        45,
        "field[2] should be skill_line 45"
    );
    assert_eq!(
        sla.get_field_i32(idx, 3),
        264,
        "field[3] should be spell 264"
    );
}
