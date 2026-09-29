use super::*;

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
