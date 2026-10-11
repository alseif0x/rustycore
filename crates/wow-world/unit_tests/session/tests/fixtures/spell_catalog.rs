//! Spell, skill, and pet catalog fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

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
