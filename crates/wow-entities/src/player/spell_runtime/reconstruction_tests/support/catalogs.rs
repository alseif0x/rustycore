//! Original immutable DB2 fixtures; range classification remains in the real stores.

use super::*;

pub(in crate::player::spell_runtime::reconstruction_tests) fn test_spell_learn_skill_store_like_cpp()
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

pub(in crate::player::spell_runtime::reconstruction_tests) fn test_spell_learn_skill_rank_store_like_cpp(
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

pub(in crate::player::spell_runtime::reconstruction_tests) fn test_skill_line_entry_like_cpp(
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

pub(in crate::player::spell_runtime::reconstruction_tests) fn test_skill_race_class_info_like_cpp(
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

pub(in crate::player::spell_runtime::reconstruction_tests) fn prepare_remove_spell_skill_range_fixture_like_cpp(
    session: &mut TestPlayer,
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
            skill_line_id: u32::from(skill_id),
            step: 3,
            current_value: skill_value,
            max_value: skill_max,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
        },
    )]));
    session.set_known_spells_like_cpp(vec![20]);
}
