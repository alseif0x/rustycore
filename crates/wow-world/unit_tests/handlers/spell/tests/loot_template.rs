//! Spell handler loot template scenarios.
//!
//! Split out of the inline test module under #624; assertions unchanged.

use super::*;

#[test]
fn plain_item_loot_template_validation_matches_cpp_basic_guards() {
    assert!(loot_template_plain_row_can_roll_like_cpp(
        25, 100.0, false, 1, 1, 3, true, true
    ));
    assert!(!loot_template_plain_row_can_roll_like_cpp(
        0, 100.0, false, 1, 1, 3, true, true
    ));
    assert!(!loot_template_plain_row_can_roll_like_cpp(
        25, 0.0, false, 1, 1, 3, true, true
    ));
    assert!(loot_template_plain_row_can_roll_like_cpp(
        25, 100.0, true, 1, 1, 3, true, true
    ));
    assert!(!loot_template_plain_row_can_roll_like_cpp(
        25, 100.0, true, 1, 1, 3, true, false
    ));
    assert!(!loot_template_plain_row_can_roll_like_cpp(
        25, 100.0, false, 0, 1, 3, true, true
    ));
    assert!(!loot_template_plain_row_can_roll_like_cpp(
        25, 100.0, false, 1, 0, 3, true, true
    ));
    assert!(!loot_template_plain_row_can_roll_like_cpp(
        25, 100.0, false, 1, 4, 3, true, true
    ));
    assert!(!loot_template_plain_row_can_roll_like_cpp(
        25, 100.0, false, 1, 1, 3, false, true
    ));
}
#[test]
fn loot_conditions_else_group_and_negative_match_cpp() {
    let conditions = vec![
        condition(0, 25, 100, false),
        condition(0, 25, 200, false),
        condition(1, 25, 300, true),
    ];

    assert!(loot_conditions_allow_player_like_cpp_representable(
        &conditions,
        |condition| Some(condition.value1 == 100 || condition.value1 == 200),
    ));
    assert!(loot_conditions_allow_player_like_cpp_representable(
        &conditions,
        |condition| Some(condition.value1 == 100),
    ));
    assert!(!loot_conditions_allow_player_like_cpp_representable(
        &conditions,
        |condition| Some(condition.value1 == 300),
    ));

    let mut scripted = condition(0, 25, 100, false);
    scripted.script_name = "npc_custom".to_string();
    assert!(!loot_conditions_allow_player_like_cpp_representable(
        &[scripted],
        |_| Some(true),
    ));
    assert!(loot_conditions_allow_player_like_cpp_representable(
        &[condition(0, -1, 100, false)],
        |_| Some(true),
    ));
}
#[test]
fn loot_condition_compare_and_quest_state_match_cpp_values() {
    assert_eq!(condition_compare_values_like_cpp(0, 10, 10), Some(true));
    assert_eq!(condition_compare_values_like_cpp(1, 11, 10), Some(true));
    assert_eq!(condition_compare_values_like_cpp(2, 9, 10), Some(true));
    assert_eq!(condition_compare_values_like_cpp(3, 10, 10), Some(true));
    assert_eq!(condition_compare_values_like_cpp(4, 10, 10), Some(true));
    assert_eq!(condition_compare_values_like_cpp(5, 10, 10), None);

    assert_eq!(player_quest_status_mask_like_cpp(None, false), 1);
    assert_eq!(player_quest_status_mask_like_cpp(Some(2), false), 2);
    assert_eq!(player_quest_status_mask_like_cpp(Some(1), false), 8);
    assert_eq!(player_quest_status_mask_like_cpp(Some(3), false), 32);
    assert_eq!(player_quest_status_mask_like_cpp(Some(1), true), 64);

    assert_eq!(player_class_mask_like_cpp(13), Some(1 << 12));
    assert_eq!(player_class_mask_like_cpp(14), None);
    assert_eq!(player_race_mask_like_cpp(34), Some(1 << 11));
    assert_eq!(player_race_mask_like_cpp(35), Some(1 << 12));
    assert_eq!(player_race_mask_like_cpp(52), Some(1 << 16));
    assert_eq!(player_race_mask_like_cpp(70), Some(1 << 15));
    assert_eq!(player_race_mask_like_cpp(33), None);
}
#[test]
fn add_loot_template_row_item_uses_caller_rng_like_cpp_urand_count() {
    let row = LootTemplateRow {
        item_id: 25,
        reference: 0,
        chance: 100.0,
        needs_quest: false,
        loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
        group_id: 0,
        min_count: 2,
        max_count: 7,
        conditions: Vec::new(),
    };
    let mut expected_rng = StdRng::seed_from_u64(0x5151);
    let expected_count = expected_rng.gen_range(2..=7);

    let mut rng = StdRng::seed_from_u64(0x5151);
    let mut loot_items = Vec::new();
    add_loot_template_row_item_like_cpp(
        &mut loot_items,
        &row,
        Default::default(),
        |_| 20,
        &mut rng,
    );

    assert_eq!(loot_items.len(), 1);
    assert_eq!(loot_items[0].quantity, expected_count);
}
#[test]
fn reference_loot_template_validation_matches_cpp_basic_guards() {
    assert!(loot_template_reference_row_can_roll_like_cpp(
        10, 100.0, 1, 1
    ));
    assert!(loot_template_reference_row_can_roll_like_cpp(
        10, 0.0000001, 1, 1
    ));
    assert!(!loot_template_reference_row_can_roll_like_cpp(
        0, 100.0, 1, 1
    ));
    assert!(!loot_template_reference_row_can_roll_like_cpp(
        10, 0.0, 1, 1
    ));
    assert!(!loot_template_reference_row_can_roll_like_cpp(
        10, 100.0, 0, 1
    ));
    assert!(!loot_template_reference_row_can_roll_like_cpp(
        10, 100.0, 1, 0
    ));
}
#[test]
fn grouped_loot_template_roll_matches_cpp_explicit_then_equal_order() {
    assert!(loot_template_group_row_can_roll_like_cpp(
        25, 0.0, false, 1, 1, 1, true, true
    ));
    assert!(loot_template_group_row_can_roll_like_cpp(
        25, 0.0, true, 1, 1, 1, true, true
    ));
    assert!(!loot_template_group_row_can_roll_like_cpp(
        25, 0.0, true, 1, 1, 1, true, false
    ));
    assert!(!loot_template_group_row_can_roll_like_cpp(
        25, 0.0000001, false, 1, 1, 1, true, true
    ));

    let rows = vec![
        LootTemplateRow {
            item_id: 25,
            reference: 0,
            chance: 100.0,
            needs_quest: false,
            loot_mode: 1,
            group_id: 1,
            min_count: 1,
            max_count: 1,
            conditions: Vec::new(),
        },
        LootTemplateRow {
            item_id: 26,
            reference: 0,
            chance: 0.0,
            needs_quest: false,
            loot_mode: 1,
            group_id: 1,
            min_count: 1,
            max_count: 1,
            conditions: Vec::new(),
        },
    ];
    let mut rng = StdRng::seed_from_u64(0xBEEF);
    let selected =
        roll_group_loot_row_like_cpp(&rows, 1, |_| true, |_| true, |_| 1.0, &mut rng).unwrap();
    assert_eq!(selected.item_id, 25);

    let equal_rows = vec![
        LootTemplateRow {
            chance: 0.0,
            item_id: 25,
            ..rows[0].clone()
        },
        LootTemplateRow {
            chance: 0.0,
            item_id: 26,
            ..rows[1].clone()
        },
    ];
    let mut rng = StdRng::seed_from_u64(0xBEEF);
    let selected =
        roll_group_loot_row_like_cpp(&equal_rows, 1, |_| true, |_| true, |_| 1.0, &mut rng)
            .unwrap();
    assert!([25, 26].contains(&selected.item_id));
}
