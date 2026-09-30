use super::quest_template;

#[test]
fn nonpositive_minimum_level_is_disabled_for_all_player_levels() {
    let mut quest = quest_template(7002);
    for minimum in [i32::MIN, -1, 0] {
        quest.min_level = minimum;
        for level in [0, 1, 254, u8::MAX] {
            assert!(
                quest.meets_min_level(level),
                "minimum={minimum}, level={level}"
            );
        }
    }
}

#[test]
fn minimum_level_is_inclusive_and_does_not_clamp_signed_row_values() {
    let mut quest = quest_template(7002);
    for (minimum, level, allowed) in [
        (1, 0, false),
        (1, 1, true),
        (20, 19, false),
        (20, 20, true),
        (20, 21, true),
        (255, 254, false),
        (255, 255, true),
        (256, 255, false),
        (i32::MAX, 255, false),
    ] {
        quest.min_level = minimum;
        assert_eq!(
            quest.meets_min_level(level),
            allowed,
            "minimum={minimum}, level={level}",
        );
    }
}

#[test]
fn zero_maximum_level_is_disabled_for_all_player_levels() {
    let mut quest = quest_template(7002);
    quest.max_level = 0;
    for level in [0, 1, 254, u8::MAX] {
        assert!(quest.meets_max_level(level), "level={level}");
    }
}

#[test]
fn maximum_level_is_inclusive_without_clamping_player_level() {
    let mut quest = quest_template(7002);
    for (maximum, level, allowed) in [
        (1, 0, true),
        (1, 1, true),
        (1, 2, false),
        (20, 19, true),
        (20, 20, true),
        (20, 21, false),
        (254, 255, false),
        (255, 255, true),
    ] {
        quest.max_level = maximum;
        assert_eq!(
            quest.meets_max_level(level),
            allowed,
            "maximum={maximum}, level={level}",
        );
    }
}

#[test]
fn availability_retains_minimum_and_maximum_level_decisions() {
    let mut quest = quest_template(7002);
    for (minimum, maximum, level, allowed) in [
        (i32::MIN, 0, 0, true),
        (0, 0, 255, true),
        (20, 40, 19, false),
        (20, 40, 20, true),
        (20, 40, 40, true),
        (20, 40, 41, false),
        (40, 20, 30, false),
        (256, 0, 255, false),
        (i32::MAX, 255, 255, false),
    ] {
        quest.min_level = minimum;
        quest.max_level = maximum;
        assert_eq!(
            quest.is_available_for(1, 1, level),
            allowed,
            "minimum={minimum}, maximum={maximum}, level={level}",
        );
    }
}

#[test]
fn availability_keeps_race_and_class_masks_alongside_level_bounds() {
    let mut quest = quest_template(7002);
    quest.min_level = 20;
    quest.max_level = 40;
    quest.allowable_races = 1;
    quest.allowable_classes = 1;

    assert!(!quest.is_available_for(2, 1, 30));
    assert!(!quest.is_available_for(1, 2, 30));
    assert!(quest.is_available_for(1, 1, 30));
    assert!(quest.is_available_for(0, 0, 30));
    assert!(!quest.is_available_for(1, 1, 19));
    assert!(!quest.is_available_for(1, 1, 41));
}
