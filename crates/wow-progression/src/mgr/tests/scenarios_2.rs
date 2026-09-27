//! Reputation manager regression scenarios, part 2 of 2.
//!
//! Moved out of the mgr.rs root under #662; every test is unchanged.

use super::*;

#[test]
fn set_reputation_like_cpp_db_template_respects_rank_cap_and_no_spillover() {
    let mut source = FactionEntry::for_test_like_cpp(102, 26);
    source.reputation_race_mask[0] = 1;
    source.reputation_max[0] = 42_000;
    let mut spill = FactionEntry::for_test_like_cpp(103, 27);
    spill.reputation_race_mask[0] = 1;
    spill.reputation_base[0] = 21_000;
    spill.reputation_max[0] = 42_000;
    let faction_store = FactionStore::from_entries([source.clone(), spill.clone()]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, None, 1, 1);
    let mut template = RepSpilloverTemplateLikeCpp::empty_like_cpp();
    template.faction[0] = 103;
    template.faction_rate[0] = 1.0;
    template.faction_rank[0] = ReputationRankLikeCpp::Honored.as_u8();

    let outcome = mgr.set_reputation_like_cpp(
        &source,
        1_000,
        set_reputation_options_for_test_like_cpp(false),
        &faction_store,
        Some(&template),
        None,
        None,
        None,
    );

    assert!(outcome.applied);
    assert!(outcome.spillover_mutations.is_empty());
    assert_eq!(mgr.get_state(27).expect("spill state").standing, 0);

    let mut no_spillover_options = set_reputation_options_for_test_like_cpp(false);
    no_spillover_options.no_spillover = true;
    let no_spillover = mgr.set_reputation_like_cpp(
        &source,
        2_000,
        no_spillover_options,
        &faction_store,
        Some(&template),
        None,
        None,
        None,
    );

    assert!(no_spillover.spillover_mutations.is_empty());
    assert_eq!(mgr.get_state(27).expect("spill state").standing, 0);
    assert_eq!(mgr.get_state(26).expect("source state").standing, 2_000);
}

#[test]
fn set_reputation_like_cpp_redirects_positive_exalted_gain_to_paragon_faction() {
    let mut source = FactionEntry::for_test_like_cpp(104, 28);
    source.reputation_race_mask[0] = 1;
    source.reputation_base[0] = 42_000;
    source.reputation_max[0] = 42_000;
    source.paragon_faction_id = 105;
    let mut paragon = FactionEntry::for_test_like_cpp(105, 29);
    paragon.reputation_race_mask[0] = 1;
    paragon.reputation_max[0] = 42_000;
    let faction_store = FactionStore::from_entries([source.clone(), paragon.clone()]);
    let paragon_store = ParagonReputationStore::from_entries([ParagonReputationEntry {
        id: 4,
        faction_id: 105,
        level_threshold: 10_000,
        quest_id: 800,
    }]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, Some(&paragon_store), 1, 1);

    let outcome = mgr.set_reputation_like_cpp(
        &source,
        1_000,
        set_reputation_options_for_test_like_cpp(true),
        &faction_store,
        None,
        None,
        Some(&paragon_store),
        None,
    );

    assert!(outcome.applied);
    assert_eq!(
        outcome.primary_mutation.as_ref().map(|entry| entry.0),
        Some(105)
    );
    assert_eq!(outcome.send_state_rep_list_id, Some(29));
    assert_eq!(mgr.get_state(28).expect("source state").standing, 0);
    assert_eq!(mgr.get_state(29).expect("paragon state").standing, 1_000);
}

#[test]
fn set_reputation_like_cpp_applies_dbc_sub_faction_spillover_with_cap() {
    let mut source = FactionEntry::for_test_like_cpp(110, 30);
    source.reputation_race_mask[0] = 1;
    source.reputation_max[0] = 42_000;
    let mut child = FactionEntry::for_test_like_cpp(111, 31);
    child.reputation_race_mask[0] = 1;
    child.parent_faction_id = 110;
    child.parent_faction_mod[0] = 0.5;
    child.parent_faction_cap[0] = ReputationRankLikeCpp::Exalted.as_u8();
    child.reputation_max[0] = 42_000;
    let mut capped = FactionEntry::for_test_like_cpp(112, 32);
    capped.reputation_race_mask[0] = 1;
    capped.parent_faction_id = 110;
    capped.parent_faction_mod[0] = 1.0;
    capped.parent_faction_cap[0] = ReputationRankLikeCpp::Honored.as_u8();
    capped.reputation_base[0] = 21_000;
    capped.reputation_max[0] = 42_000;
    let faction_store = FactionStore::from_entries([source.clone(), child.clone(), capped]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, None, 1, 1);

    let outcome = mgr.set_reputation_like_cpp(
        &source,
        1_000,
        set_reputation_options_for_test_like_cpp(false),
        &faction_store,
        None,
        None,
        None,
        None,
    );

    assert!(outcome.applied);
    assert_eq!(outcome.spillover_mutations.len(), 1);
    assert_eq!(outcome.spillover_mutations[0].0, 111);
    assert_eq!(outcome.spillover_mutations[0].1.reputation_change, 500);
    assert_eq!(mgr.get_state(31).expect("child state").standing, 500);
    assert_eq!(mgr.get_state(32).expect("capped state").standing, 0);
}

#[test]
fn set_reputation_like_cpp_applies_dbc_sister_spillover_when_parent_has_no_bar() {
    let mut parent = FactionEntry::for_test_like_cpp(120, -1);
    parent.reputation_race_mask[0] = 1;
    let mut source = FactionEntry::for_test_like_cpp(121, 33);
    source.reputation_race_mask[0] = 1;
    source.parent_faction_id = 120;
    source.parent_faction_mod[1] = 0.5;
    source.reputation_max[0] = 42_000;
    let mut sister = FactionEntry::for_test_like_cpp(122, 34);
    sister.reputation_race_mask[0] = 1;
    sister.parent_faction_id = 120;
    sister.parent_faction_mod[0] = 0.25;
    sister.parent_faction_cap[0] = ReputationRankLikeCpp::Exalted.as_u8();
    sister.reputation_max[0] = 42_000;
    let faction_store = FactionStore::from_entries([parent, source.clone(), sister]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, None, 1, 1);

    let outcome = mgr.set_reputation_like_cpp(
        &source,
        1_000,
        set_reputation_options_for_test_like_cpp(false),
        &faction_store,
        None,
        None,
        None,
        None,
    );

    assert!(outcome.applied);
    assert_eq!(outcome.spillover_mutations.len(), 1);
    assert_eq!(outcome.spillover_mutations[0].0, 122);
    assert_eq!(outcome.spillover_mutations[0].1.reputation_change, 125);
    assert_eq!(mgr.get_state(34).expect("sister state").standing, 125);
}

#[test]
fn set_reputation_like_cpp_spills_to_parent_when_parent_header_shows_bar() {
    let mut parent = FactionEntry::for_test_like_cpp(130, 35);
    parent.reputation_race_mask[0] = 1;
    parent.reputation_flags[0] = ReputationFlagsLikeCpp::HEADER_SHOWS_BAR.bits();
    parent.reputation_max[0] = 42_000;
    let mut source = FactionEntry::for_test_like_cpp(131, 36);
    source.reputation_race_mask[0] = 1;
    source.parent_faction_id = 130;
    source.parent_faction_mod[1] = 0.5;
    source.reputation_max[0] = 42_000;
    let mut sister = FactionEntry::for_test_like_cpp(132, 37);
    sister.reputation_race_mask[0] = 1;
    sister.parent_faction_id = 130;
    sister.parent_faction_mod[0] = 1.0;
    sister.parent_faction_cap[0] = ReputationRankLikeCpp::Exalted.as_u8();
    sister.reputation_max[0] = 42_000;
    let faction_store = FactionStore::from_entries([parent, source.clone(), sister]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, None, 1, 1);

    let outcome = mgr.set_reputation_like_cpp(
        &source,
        1_000,
        set_reputation_options_for_test_like_cpp(false),
        &faction_store,
        None,
        None,
        None,
        None,
    );

    assert!(outcome.applied);
    assert_eq!(outcome.spillover_mutations.len(), 1);
    assert_eq!(outcome.spillover_mutations[0].0, 130);
    assert_eq!(outcome.spillover_mutations[0].1.reputation_change, 500);
    assert_eq!(mgr.get_state(35).expect("parent state").standing, 500);
    assert_eq!(mgr.get_state(37).expect("sister state").standing, 0);
}
