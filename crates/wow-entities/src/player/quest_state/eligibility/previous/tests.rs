use super::PlayerQuestGameplayState;
use std::cell::RefCell;
use wow_constants::quest::{
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    QUEST_STATUS_NONE_LIKE_CPP, QUEST_STATUS_REWARDED_LIKE_CPP,
};

#[derive(Debug, PartialEq, Eq)]
enum Read {
    Rewarded(u32),
    Status(u32),
}

#[test]
fn zero_previous_id_requires_no_membership_reads() {
    assert!(
        PlayerQuestGameplayState::previous_quest_requirement_satisfied_from_membership(
            0,
            |_| panic!("zero previous ID must not read rewarded membership"),
            |_| panic!("zero previous ID must not read active status"),
        )
    );
}

#[test]
fn positive_previous_id_reads_only_rewarded_membership_for_both_results() {
    for rewarded in [false, true] {
        let reads = RefCell::new(Vec::new());
        let result = PlayerQuestGameplayState::previous_quest_requirement_satisfied_from_membership(
            11,
            |id| {
                reads.borrow_mut().push(Read::Rewarded(id));
                rewarded
            },
            |_| panic!("positive previous ID must not read active status"),
        );

        assert_eq!(result, rewarded);
        assert_eq!(*reads.borrow(), [Read::Rewarded(11)]);
    }
}

#[test]
fn negative_previous_id_reads_only_exact_incomplete_status() {
    for (status, expected) in [
        (None, false),
        (Some(QUEST_STATUS_NONE_LIKE_CPP), false),
        (Some(QUEST_STATUS_COMPLETE_LIKE_CPP), false),
        (Some(QUEST_STATUS_FAILED_LIKE_CPP), false),
        (Some(QUEST_STATUS_REWARDED_LIKE_CPP), false),
        (Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP), true),
    ] {
        let reads = RefCell::new(Vec::new());
        let result = PlayerQuestGameplayState::previous_quest_requirement_satisfied_from_membership(
            -11,
            |_| panic!("negative previous ID must not read rewarded membership"),
            |id| {
                reads.borrow_mut().push(Read::Status(id));
                status
            },
        );

        assert_eq!(result, expected);
        assert_eq!(*reads.borrow(), [Read::Status(11)]);
    }
}

#[test]
fn minimum_signed_previous_id_reads_unsigned_status_id_without_rewarded_reads() {
    for (status, expected) in [
        (None, false),
        (Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP), true),
    ] {
        let reads = RefCell::new(Vec::new());
        let result = PlayerQuestGameplayState::previous_quest_requirement_satisfied_from_membership(
            i32::MIN,
            |_| panic!("minimum negative previous ID must not read rewarded membership"),
            |id| {
                reads.borrow_mut().push(Read::Status(id));
                status
            },
        );

        assert_eq!(result, expected);
        assert_eq!(*reads.borrow(), [Read::Status(2_147_483_648)]);
    }
}
