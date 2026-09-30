use super::remaining_source_item_count as direct_item_count_after_loot_release_like_cpp;

#[test]
fn prospecting_and_milling_release_consume_at_most_five_source_items_like_cpp() {
    assert_eq!(
        direct_item_count_after_loot_release_like_cpp(20, Some(5)),
        15
    );
    assert_eq!(direct_item_count_after_loot_release_like_cpp(5, Some(5)), 0);
    assert_eq!(direct_item_count_after_loot_release_like_cpp(3, Some(5)), 0);
    assert_eq!(direct_item_count_after_loot_release_like_cpp(20, None), 0);
}
