//! Unit test for the pure destroy-count helper moved with the item bodies to
//! `item_session` (#1363 slice 8c). The test moved verbatim from the World
//! character scenarios (`character_tests/item_4.rs`).

use super::{DestroyItemCountAction, destroy_item_count_action};

#[test]
fn destroy_item_count_action_matches_cpp_direct_item_branch() {
    assert_eq!(
        destroy_item_count_action(5, 0),
        DestroyItemCountAction::FullStack
    );
    assert_eq!(
        destroy_item_count_action(5, 5),
        DestroyItemCountAction::FullStack
    );
    assert_eq!(
        destroy_item_count_action(5, 7),
        DestroyItemCountAction::FullStack
    );
    assert_eq!(
        destroy_item_count_action(5, 2),
        DestroyItemCountAction::PartialStack { new_count: 3 }
    );
}
