//! Creature AI state machines regression scenarios, part 2 of 2.
//!
//! Moved out of the lib.rs root under #658; every test is unchanged.

use super::*;

#[test]
fn default_target_selector_matches_cpp_filters() {
    let mut target = target_candidate(110, 0, 12.0);
    assert!(!default_target_selector_accepts_like_cpp(
        &target,
        DefaultTargetSelectorLikeCpp {
            dist: 10.0,
            ..DefaultTargetSelectorLikeCpp::default()
        }
    ));
    assert!(!default_target_selector_accepts_like_cpp(
        &target,
        DefaultTargetSelectorLikeCpp {
            dist: -20.0,
            ..DefaultTargetSelectorLikeCpp::default()
        }
    ));

    target.is_player = false;
    assert!(!default_target_selector_accepts_like_cpp(
        &target,
        DefaultTargetSelectorLikeCpp {
            player_only: true,
            ..DefaultTargetSelectorLikeCpp::default()
        }
    ));

    target.is_player = true;
    target.is_last_victim = true;
    assert!(!default_target_selector_accepts_like_cpp(
        &target,
        DefaultTargetSelectorLikeCpp {
            with_tank: false,
            ..DefaultTargetSelectorLikeCpp::default()
        }
    ));

    target.is_last_victim = false;
    target.has_aura = false;
    assert!(!default_target_selector_accepts_like_cpp(
        &target,
        DefaultTargetSelectorLikeCpp {
            aura: 123,
            ..DefaultTargetSelectorLikeCpp::default()
        }
    ));

    target.has_aura = true;
    assert!(!default_target_selector_accepts_like_cpp(
        &target,
        DefaultTargetSelectorLikeCpp {
            aura: -123,
            ..DefaultTargetSelectorLikeCpp::default()
        }
    ));
}

#[test]
fn plain_creature_has_no_boss_ai_view_like_cpp_failed_dynamic_cast() {
    let creature = creature_with_boss_id(None);

    assert!(creature.boss_ai_like_cpp().is_none());
}

#[test]
fn boss_creature_exposes_script_boss_id_like_cpp_boss_ai() {
    let creature = creature_with_boss_id(Some(7));

    assert_eq!(creature.boss_ai_like_cpp().unwrap().boss_id(), 7);
}

#[test]
fn summon_list_preserves_order_and_removes_all_matching_guids_like_cpp() {
    let mut summons = SummonListLikeCpp::new();
    let a = guid(1);
    let b = guid(2);

    summons.summon_like_cpp(a);
    summons.summon_like_cpp(b);
    summons.summon_like_cpp(a);

    assert_eq!(summons.iter_like_cpp().collect::<Vec<_>>(), vec![a, b, a]);
    assert_eq!(summons.size_like_cpp(), 3);

    summons.despawn_like_cpp(a);

    assert_eq!(summons.iter_like_cpp().collect::<Vec<_>>(), vec![b]);
}

#[test]
fn summon_list_despawn_all_drains_fifo_and_ignores_missing_like_cpp() {
    let mut summons = SummonListLikeCpp::new();
    let a = guid(10);
    let missing = guid(11);
    let b = guid(12);
    let creatures = HashMap::from([
        (a, creature_view(a, 100, true)),
        (b, creature_view(b, 200, true)),
    ]);

    summons.summon_like_cpp(a);
    summons.summon_like_cpp(missing);
    summons.summon_like_cpp(b);

    let plan = summons.despawn_all_like_cpp(|guid| resolve_from(&creatures, guid));

    assert_eq!(plan, vec![a, b]);
    assert!(summons.empty_like_cpp());
}

#[test]
fn summon_list_despawn_entry_erases_missing_and_matching_entry_like_cpp() {
    let mut summons = SummonListLikeCpp::new();
    let first_match = guid(20);
    let kept = guid(21);
    let missing = guid(22);
    let second_match = guid(23);
    let creatures = HashMap::from([
        (first_match, creature_view(first_match, 777, true)),
        (kept, creature_view(kept, 888, true)),
        (second_match, creature_view(second_match, 777, true)),
    ]);

    for guid in [first_match, kept, missing, second_match] {
        summons.summon_like_cpp(guid);
    }

    let plan = summons.despawn_entry_like_cpp(777, |guid| resolve_from(&creatures, guid));

    assert_eq!(plan, vec![first_match, second_match]);
    assert_eq!(summons.iter_like_cpp().collect::<Vec<_>>(), vec![kept]);
}

#[test]
fn summon_list_remove_not_existing_and_has_entry_use_object_accessor_view_like_cpp() {
    let mut summons = SummonListLikeCpp::new();
    let missing = guid(30);
    let wrong_entry = guid(31);
    let wanted = guid(32);
    let creatures = HashMap::from([
        (wrong_entry, creature_view(wrong_entry, 1, true)),
        (wanted, creature_view(wanted, 2, true)),
    ]);

    for guid in [missing, wrong_entry, wanted] {
        summons.summon_like_cpp(guid);
    }

    assert!(summons.has_entry_like_cpp(2, |guid| resolve_from(&creatures, guid)));
    assert!(!summons.has_entry_like_cpp(3, |guid| resolve_from(&creatures, guid)));

    summons.remove_not_existing_like_cpp(|guid| resolve_from(&creatures, guid));

    assert_eq!(
        summons.iter_like_cpp().collect::<Vec<_>>(),
        vec![wrong_entry, wanted]
    );
}

#[test]
fn summon_list_do_zone_in_combat_filters_ai_and_optional_entry_like_cpp() {
    let mut summons = SummonListLikeCpp::new();
    let ai_match = guid(40);
    let no_ai = guid(41);
    let ai_other_entry = guid(42);
    let missing = guid(43);
    let creatures = HashMap::from([
        (ai_match, creature_view(ai_match, 9, true)),
        (no_ai, creature_view(no_ai, 9, false)),
        (ai_other_entry, creature_view(ai_other_entry, 10, true)),
    ]);

    for guid in [ai_match, no_ai, ai_other_entry, missing] {
        summons.summon_like_cpp(guid);
    }

    assert_eq!(
        summons.do_zone_in_combat_like_cpp(9, |guid| resolve_from(&creatures, guid)),
        vec![ai_match]
    );
    assert_eq!(
        summons.do_zone_in_combat_like_cpp(0, |guid| resolve_from(&creatures, guid)),
        vec![ai_match, ai_other_entry]
    );
    assert_eq!(
        summons.iter_like_cpp().collect::<Vec<_>>(),
        vec![ai_match, no_ai, ai_other_entry, missing],
        "C++ DoZoneInCombat does not prune missing GUIDs"
    );
}

#[test]
fn summon_list_do_action_uncapped_copies_then_resolves_ai_like_cpp() {
    let mut summons = SummonListLikeCpp::new();
    let selected_ai = guid(50);
    let selected_no_ai = guid(51);
    let not_selected = guid(52);
    let creatures = HashMap::from([
        (selected_ai, creature_view(selected_ai, 1, true)),
        (selected_no_ai, creature_view(selected_no_ai, 1, false)),
        (not_selected, creature_view(not_selected, 2, true)),
    ]);

    for guid in [selected_ai, selected_no_ai, not_selected] {
        summons.summon_like_cpp(guid);
    }

    let actions = summons.do_action_like_cpp(
        42,
        |guid| guid != not_selected,
        |guid| resolve_from(&creatures, guid),
    );

    assert_eq!(
        actions,
        vec![SummonListActionLikeCpp {
            guid: selected_ai,
            action: 42,
        }]
    );
}

#[test]
fn summon_list_do_action_capped_uses_random_resize_before_ai_resolution_like_cpp() {
    let mut summons = SummonListLikeCpp::new();
    let selected_ai = guid(60);
    let creatures = HashMap::from([(selected_ai, creature_view(selected_ai, 1, true))]);

    for guid in [selected_ai, selected_ai, selected_ai] {
        summons.summon_like_cpp(guid);
    }

    let actions =
        summons.do_action_with_max_like_cpp(77, |_| true, 2, |guid| resolve_from(&creatures, guid));

    assert_eq!(
        actions,
        vec![
            SummonListActionLikeCpp {
                guid: selected_ai,
                action: 77,
            },
            SummonListActionLikeCpp {
                guid: selected_ai,
                action: 77,
            },
        ]
    );
    assert_eq!(
        summons.iter_like_cpp().collect::<Vec<_>>(),
        vec![selected_ai, selected_ai, selected_ai],
        "C++ DoAction works on a copy and must not mutate the original SummonList"
    );
}
