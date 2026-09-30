//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

mod equipment;







#[test]
fn spell_group_queries_return_empty_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.spell_spell_group_map_bounds_like_cpp(10).is_empty());
    assert!(
        session
            .spell_group_spell_map_bounds_like_cpp(1001)
            .is_empty()
    );
    assert!(!session.is_spell_member_of_spell_group_like_cpp(10, 1001));
    assert!(
        session
            .set_of_spells_in_spell_group_like_cpp(1001)
            .is_empty()
    );
}
#[test]
fn spell_group_queries_expand_nested_and_normalize_ranks_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_group_store(Arc::new(test_spell_group_store_like_cpp()));
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 25,
                supercedes_spell_id: 20,
            }],
            |_| true,
        ),
    ));

    assert_eq!(
        session.spell_group_spell_map_bounds_like_cpp(1001),
        &[10, -1002]
    );
    assert_eq!(
        session.set_of_spells_in_spell_group_like_cpp(1001),
        BTreeSet::from([10, 20])
    );
    assert_eq!(
        session.spell_spell_group_map_bounds_like_cpp(25),
        &[1001, 1002]
    );
    assert!(session.is_spell_member_of_spell_group_like_cpp(25, 1002));
    assert!(!session.is_spell_member_of_spell_group_like_cpp(10, 1002));
}
#[test]
fn spell_group_stack_rule_queries_default_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert_eq!(
        session.spell_group_stack_rule_like_cpp(1001),
        wow_data::SpellGroupStackRuleLikeCpp::Default
    );
    assert!(
        session
            .same_effect_stack_rule_aura_types_like_cpp(1002)
            .is_none()
    );
    assert_eq!(
        session.check_spell_group_stack_rules_like_cpp(10, 20),
        wow_data::SpellGroupStackRuleLikeCpp::Default
    );
}
#[test]
fn spell_group_stack_rule_queries_match_store_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_groups = Arc::new(test_spell_group_store_like_cpp());
    let stack_rules = Arc::new(test_spell_group_stack_rule_store_like_cpp(&spell_groups));
    session.set_spell_group_store(Arc::clone(&spell_groups));
    session.set_spell_group_stack_rule_store(stack_rules);

    assert_eq!(
        session.spell_group_stack_rule_like_cpp(1001),
        wow_data::SpellGroupStackRuleLikeCpp::ExclusiveHighest
    );
    assert_eq!(
        session.same_effect_stack_rule_aura_types_like_cpp(1002),
        Some(&BTreeSet::from([31]))
    );
    assert_eq!(
        session.check_spell_group_stack_rules_like_cpp(10, 20),
        wow_data::SpellGroupStackRuleLikeCpp::ExclusiveHighest
    );
}
#[test]
fn spell_linked_queries_return_empty_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(
        session
            .spell_linked_like_cpp(wow_data::SpellLinkedTypeLikeCpp::Cast, 10)
            .is_empty()
    );
}
#[test]
fn spell_linked_queries_preserve_signed_effect_order_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_linked_store(Arc::new(test_spell_linked_store_like_cpp()));

    assert_eq!(
        session.spell_linked_like_cpp(wow_data::SpellLinkedTypeLikeCpp::Cast, 10),
        &[20, -30]
    );
    assert_eq!(
        session.spell_linked_like_cpp(wow_data::SpellLinkedTypeLikeCpp::Remove, 40),
        &[50]
    );
    assert!(
        session
            .spell_linked_like_cpp(wow_data::SpellLinkedTypeLikeCpp::Hit, 10)
            .is_empty()
    );
}
#[test]
fn spell_totem_model_queries_return_zero_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert_eq!(session.model_for_totem_like_cpp(50, 2), 0);
}
#[test]
fn spell_totem_model_queries_match_spell_and_race_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_totem_model_store(Arc::new(test_spell_totem_model_store_like_cpp()));

    assert_eq!(
        session.model_for_totem_like_cpp(50, 2),
        2000,
        "C++ std::map assignment exposes the last valid duplicate row"
    );
    assert_eq!(session.model_for_totem_like_cpp(50, 8), 3000);
    assert_eq!(session.model_for_totem_like_cpp(50, 3), 0);
    assert_eq!(session.model_for_totem_like_cpp(51, 2), 0);
}
#[test]
fn spell_pet_aura_query_returns_none_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.pet_aura_like_cpp(77, 2).is_none());
}
#[test]
fn spell_pet_aura_query_matches_spell_effect_key_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_pet_aura_store(Arc::new(test_spell_pet_aura_store_like_cpp()));

    let aura = session.pet_aura_like_cpp(77, 2).expect("pet aura");
    assert!(aura.remove_on_change_pet);
    assert_eq!(aura.damage, 35);
    assert_eq!(aura.aura_for_pet_entry_like_cpp(501), 901);
    assert_eq!(aura.aura_for_pet_entry_like_cpp(502), 900);
    assert!(session.pet_aura_like_cpp(77, 3).is_none());
}
#[test]
fn spell_area_queries_return_empty_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.spell_area_map_bounds_like_cpp(100).is_empty());
    assert!(
        session
            .spell_area_for_area_map_bounds_like_cpp(10)
            .is_empty()
    );
    assert!(
        session
            .spell_area_for_quest_map_bounds_like_cpp(20)
            .is_empty()
    );
    assert!(
        session
            .spell_area_for_quest_end_map_bounds_like_cpp(30)
            .is_empty()
    );
    assert!(
        session
            .spell_area_for_aura_map_bounds_like_cpp(40)
            .is_empty()
    );
}
#[test]
fn spell_area_queries_match_primary_and_secondary_indices_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_area_store(Arc::new(test_spell_area_store_like_cpp()));

    assert_eq!(session.spell_area_map_bounds_like_cpp(100).len(), 1);
    assert_eq!(session.spell_area_for_area_map_bounds_like_cpp(10).len(), 1);
    assert_eq!(
        session.spell_area_for_quest_map_bounds_like_cpp(20).len(),
        1
    );
    assert_eq!(
        session.spell_area_for_quest_map_bounds_like_cpp(30).len(),
        2
    );
    assert_eq!(
        session
            .spell_area_for_quest_end_map_bounds_like_cpp(30)
            .len(),
        2
    );
    assert_eq!(session.spell_area_for_aura_map_bounds_like_cpp(40).len(), 1);
}
#[test]
fn spell_custom_attributes_return_zero_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert_eq!(
        session.spell_custom_attributes_for_difficulty_like_cpp(100, 0),
        0
    );
}
#[test]
fn spell_custom_attributes_lookup_exact_difficulty_like_cpp() {
    let (mut session, _, _) = make_session();
    session
        .set_spell_custom_attribute_store(Arc::new(test_spell_custom_attribute_store_like_cpp()));

    assert_eq!(
        session.spell_custom_attributes_for_difficulty_like_cpp(100, 0),
        wow_data::SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP
            | wow_data::SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP
    );
    assert_eq!(
        session.spell_custom_attributes_for_difficulty_like_cpp(100, 2),
        wow_data::SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP
            | wow_data::SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP
    );
    assert_eq!(
        session.spell_custom_attributes_for_difficulty_like_cpp(100, 1),
        0
    );
}
#[test]
fn serverside_spell_lookup_returns_none_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.serverside_spell_like_cpp(100, 0).is_none());
}
#[test]
fn serverside_spell_lookup_uses_exact_difficulty_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_serverside_spell_store(Arc::new(test_serverside_spell_store_like_cpp()));

    assert_eq!(
        session
            .serverside_spell_like_cpp(100, 0)
            .unwrap()
            .row
            .spell_name,
        "server spell"
    );
    assert!(session.serverside_spell_like_cpp(100, 1).is_none());
}
#[test]
fn spell_learn_skill_query_returns_none_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.spell_learn_skill_like_cpp(10).is_none());
    assert_eq!(
        session.spell_learn_skill_lookup_like_cpp(10),
        wow_data::SpellLearnSkillLookupLikeCpp::MissingCoverage
    );
}
#[test]
fn spell_learn_skill_query_matches_loaded_effects_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_learn_skill_store(Arc::new(test_spell_learn_skill_store_like_cpp()));

    assert_eq!(
        session.spell_learn_skill_like_cpp(10),
        Some(&wow_data::SpellLearnSkillNodeLikeCpp {
            skill: 755,
            step: 4,
            value: 0,
            maxvalue: 0,
        })
    );
    assert_eq!(
        session.spell_learn_skill_like_cpp(20),
        Some(&wow_data::SpellLearnSkillNodeLikeCpp {
            skill: wow_data::SKILL_DUAL_WIELD_LIKE_CPP,
            step: 1,
            value: 1,
            maxvalue: 1,
        })
    );
    assert!(session.spell_learn_skill_like_cpp(21).is_none());
    assert_eq!(
        session.spell_learn_skill_lookup_like_cpp(21),
        wow_data::SpellLearnSkillLookupLikeCpp::MissingCoverage
    );
}
#[test]
fn spell_learn_spell_queries_return_empty_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.spell_learn_spell_map_bounds_like_cpp(10).is_empty());
    assert!(!session.is_spell_learn_spell_like_cpp(10));
    assert!(!session.is_spell_learn_to_spell_like_cpp(10, 20));
}
#[test]
fn spell_learn_spell_queries_match_loaded_multimap_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_learn_spell_store(Arc::new(test_spell_learn_spell_store_like_cpp()));

    assert_eq!(
        session.spell_learn_spell_map_bounds_like_cpp(10),
        &[wow_data::SpellLearnSpellNodeLikeCpp {
            spell: 20,
            overrides_spell: 0,
            active: true,
            auto_learned: false,
        }]
    );
    assert!(session.is_spell_learn_spell_like_cpp(10));
    assert!(session.is_spell_learn_to_spell_like_cpp(10, 20));
    assert!(session.is_spell_learn_to_spell_like_cpp(30, 40));
    assert!(!session.is_spell_learn_to_spell_like_cpp(10, 21));
}
#[test]
fn pet_levelup_spell_list_returns_none_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.pet_levelup_spell_list_like_cpp(44).is_none());
}
#[test]
fn pet_levelup_spell_list_matches_family_multimap_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_pet_levelup_spell_store(Arc::new(test_pet_levelup_spell_store_like_cpp()));

    let spells = session
        .pet_levelup_spell_list_like_cpp(44)
        .expect("pet levelup spell list");
    assert_eq!(
        spells.iter().collect::<Vec<_>>(),
        vec![(10, 701), (20, 700)]
    );
    assert!(session.pet_levelup_spell_list_like_cpp(45).is_none());
}
#[test]
fn pet_default_spells_entry_returns_none_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.pet_default_spells_entry_like_cpp(500).is_none());
}
#[test]
fn pet_default_spells_entry_matches_template_spells_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_pet_default_spell_store(Arc::new(test_pet_default_spell_store_like_cpp()));

    let entry = session
        .pet_default_spells_entry_like_cpp(500)
        .expect("pet default spells entry");
    assert_eq!(entry.spellid, [10, 0, 11, 0]);
    assert!(session.pet_default_spells_entry_like_cpp(501).is_none());
}
#[test]
fn pet_family_spells_return_none_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.pet_family_spells_like_cpp(44).is_none());
}
#[test]
fn pet_family_spells_match_passive_family_store_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_pet_family_spell_store(Arc::new(test_pet_family_spell_store_like_cpp()));

    assert_eq!(session.pet_family_spells_like_cpp(44), Some(vec![800]));
    assert!(session.pet_family_spells_like_cpp(45).is_none());
}
