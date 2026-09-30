use super::{PlayerQuestGameplayState, QuestDayCooldownBlock, QuestEligibilityRules};
use std::cell::RefCell;

fn rules(daily: bool, dungeon_finder: bool) -> QuestEligibilityRules<'static> {
    QuestEligibilityRules::new(
        22, false, 0, 0, &[], &[], daily, dungeon_finder, false, false, false, 0,
    )
}

#[test]
fn df_and_daily_without_df_completion_reads_only_df_membership() {
    let reads = RefCell::new(Vec::new());
    let result = PlayerQuestGameplayState::quest_day_cooldown_block_from_membership(
        &rules(true, true),
        || {
            reads.borrow_mut().push("df");
            false
        },
        || panic!("DF priority must not read the completed daily bucket"),
    );

    assert_eq!(result, None);
    assert_eq!(*reads.borrow(), ["df"]);
}

#[test]
fn df_completion_blocks_without_reading_daily_membership() {
    for daily in [false, true] {
        let reads = RefCell::new(Vec::new());
        let result = PlayerQuestGameplayState::quest_day_cooldown_block_from_membership(
            &rules(daily, true),
            || {
                reads.borrow_mut().push("df");
                true
            },
            || panic!("completed DF quest must not read daily membership"),
        );

        assert_eq!(result, Some(QuestDayCooldownBlock::DungeonFinder));
        assert_eq!(*reads.borrow(), ["df"]);
    }
}

#[test]
fn non_df_daily_reads_only_daily_membership_and_preserves_both_results() {
    for completed in [false, true] {
        let reads = RefCell::new(Vec::new());
        let result = PlayerQuestGameplayState::quest_day_cooldown_block_from_membership(
            &rules(true, false),
            || panic!("non-DF daily quest must not read DF membership"),
            || {
                reads.borrow_mut().push("daily");
                completed
            },
        );

        assert_eq!(result, completed.then_some(QuestDayCooldownBlock::Daily));
        assert_eq!(*reads.borrow(), ["daily"]);
    }
}

#[test]
fn non_daily_non_df_quest_reads_neither_membership() {
    let result = PlayerQuestGameplayState::quest_day_cooldown_block_from_membership(
        &rules(false, false),
        || panic!("non-recurring quest must not read DF membership"),
        || panic!("non-recurring quest must not read daily membership"),
    );

    assert_eq!(result, None);
}
