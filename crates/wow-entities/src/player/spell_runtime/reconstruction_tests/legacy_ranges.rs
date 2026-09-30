//! Original Session rule cases, now exercising canonical Player operations.

use super::*;
use super::support::*;

#[test]
fn remove_known_spell_downgrades_learned_skill_language_range_like_cpp() {
    let (mut session, _, _) = make_session();
    prepare_remove_spell_skill_range_fixture_like_cpp(
        &mut session,
        777,
        wow_data::SKILL_CATEGORY_LANGUAGES_LIKE_CPP,
        0,
        0,
        wow_data::SkillTiersStoreLikeCpp::default(),
        50,
        75,
    );

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&777),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_line_id: 777,
            step: 2,
            current_value: 300,
            max_value: 75,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Changed,
        }),
        "C++ GetSkillRangeType LANGUAGE forces value to 300 but only clamps existing max downward, so max is not raised when it was already below 300"
    );
}

#[test]
fn remove_known_spell_downgrades_learned_skill_level_range_like_cpp() {
    let (mut session, _, _) = make_session();
    prepare_remove_spell_skill_range_fixture_like_cpp(
        &mut session,
        778,
        9,
        0,
        0,
        wow_data::SkillTiersStoreLikeCpp::default(),
        80,
        100,
    );

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&778),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_line_id: 778,
            step: 2,
            current_value: 60,
            max_value: 60,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Changed,
        }),
        "C++ LEVEL range uses GetMaxSkillValueForLevel (level * 5) and then clamps current value/max"
    );
}

#[test]
fn remove_known_spell_downgrades_learned_skill_always_max_level_range_like_cpp() {
    let (mut session, _, _) = make_session();
    prepare_remove_spell_skill_range_fixture_like_cpp(
        &mut session,
        779,
        9,
        wow_data::SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP,
        0,
        wow_data::SkillTiersStoreLikeCpp::default(),
        10,
        100,
    );

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&779),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_line_id: 779,
            step: 2,
            current_value: 60,
            max_value: 60,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Changed,
        }),
        "C++ SKILL_FLAG_ALWAYS_MAX_VALUE sets value to the computed max before SetSkill"
    );
}

#[test]
fn remove_known_spell_downgrades_learned_skill_mono_range_like_cpp() {
    let (mut session, _, _) = make_session();
    prepare_remove_spell_skill_range_fixture_like_cpp(
        &mut session,
        780,
        wow_data::SKILL_CATEGORY_ARMOR_LIKE_CPP,
        0,
        0,
        wow_data::SkillTiersStoreLikeCpp::default(),
        10,
        100,
    );

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&780),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_line_id: 780,
            step: 2,
            current_value: 1,
            max_value: 1,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Changed,
        }),
        "C++ MONO range caps the downgraded learned skill to 1"
    );
}

#[test]
fn remove_known_spell_downgrades_learned_skill_rank_range_like_cpp() {
    let (mut session, _, _) = make_session();
    prepare_remove_spell_skill_range_fixture_like_cpp(
        &mut session,
        781,
        9,
        0,
        12,
        wow_data::SkillTiersStoreLikeCpp::from_rows_like_cpp([wow_data::SkillTiersRowLikeCpp {
            id: 12,
            value: [75, 150, 225, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        }]),
        200,
        225,
    );

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&781),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_line_id: 781,
            step: 2,
            current_value: 150,
            max_value: 150,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Changed,
        }),
        "C++ RANK range resolves SkillTiers[prevSkill.step - 1] when previous SpellLearnSkill maxvalue is 0"
    );
}

#[test]
fn remove_known_spell_downgrades_learned_skill_without_race_class_info_to_zero_like_cpp() {
    let (mut session, _, _) = make_session();
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
    session.set_spell_learn_skill_store(Arc::new(test_spell_learn_skill_rank_store_like_cpp(782)));
    session.set_skill_line_store(Arc::new(wow_data::SkillLineStore::from_entries([
        test_skill_line_entry_like_cpp(782, 9),
    ])));
    session.set_skill_store(Arc::new(
        wow_data::SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
            std::iter::empty::<wow_data::SkillLineAbilityRecord>(),
            std::iter::empty::<wow_data::SkillRaceClassInfoRecord>(),
        ),
    ));
    session.set_skill_tiers_store(Arc::new(wow_data::SkillTiersStoreLikeCpp::default()));
    session.set_player_skill_records_like_cpp(HashMap::from([(
        782,
        RepresentedPlayerSkillLikeCpp {
            skill_line_id: 782,
            step: 3,
            current_value: 80,
            max_value: 100,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
        },
    )]));
    session.set_known_spells_like_cpp(vec![20]);

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&782),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_line_id: 782,
            step: 0,
            current_value: 0,
            max_value: 0,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Deleted,
        }),
        "C++ leaves new_skill_max_value at 0 when SkillRaceClassInfo is missing, then clamps value/max to 0"
    );
}

