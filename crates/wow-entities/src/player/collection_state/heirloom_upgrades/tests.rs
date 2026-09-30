use super::*;
use std::cell::RefCell;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
enum Read {
    Lookup(u32),
    Item(i32),
    Static(i32),
    Contains(u32),
}

fn select(start: i32, rows: &BTreeMap<u32, (i32, i32)>, owned: &[u32]) -> Option<u32> {
    PlayerCollectionStateLikeCpp::static_heirloom_upgrade(
        start,
        |id| rows.get(&id),
        |row| row.0,
        |row| row.1,
        |id| owned.contains(&id),
    )
}

#[test]
fn heirloom_bonus_uses_highest_represented_flag_and_ignores_other_bits() {
    let kits = [1, 2, 3, 4, 5, 6];
    let bonuses = [10, 20, 30, 40, 50, 60];
    assert_eq!(
        PlayerCollectionStateLikeCpp::heirloom_bonus_for_flags(&kits, &bonuses, 0),
        0,
    );
    assert_eq!(
        PlayerCollectionStateLikeCpp::heirloom_bonus_for_flags(&kits, &bonuses, 1),
        10,
    );
    assert_eq!(
        PlayerCollectionStateLikeCpp::heirloom_bonus_for_flags(&kits, &bonuses, 3),
        20,
    );
    assert_eq!(
        PlayerCollectionStateLikeCpp::heirloom_bonus_for_flags(&kits, &bonuses, 0x21),
        60,
    );
    assert_eq!(
        PlayerCollectionStateLikeCpp::heirloom_bonus_for_flags(&kits, &bonuses, 1 << 31),
        0,
    );
}

#[test]
fn heirloom_upgrade_keeps_all_matching_bits_and_last_kit_bonus() {
    let kits = [7, 8, 7, 9, 10, 11];
    let bonuses = [10, 20, 30, 40, 50, 60];
    assert_eq!(
        PlayerCollectionStateLikeCpp::heirloom_upgrade(&kits, &bonuses, 0x80, 7),
        (0x85, 30),
    );
}

#[test]
fn heirloom_upgrade_without_matching_kit_keeps_flags_and_zeroes_bonus() {
    assert_eq!(
        PlayerCollectionStateLikeCpp::heirloom_upgrade(
            &[1, 2, 3, 4, 5, 6],
            &[10, 20, 30, 40, 50, 60],
            0x23,
            99,
        ),
        (0x23, 0),
    );
}

#[test]
fn heirloom_upgrade_retains_raw_zero_kit_matching() {
    assert_eq!(
        PlayerCollectionStateLikeCpp::heirloom_upgrade(&[0; 6], &[1, 2, 3, 4, 5, 6], 0, 0),
        (0x3f, 6),
    );
}

#[test]
fn static_heirloom_chain_preserves_lazy_query_and_inventory_order() {
    let rows = BTreeMap::from([(10, (10, 20)), (20, (20, 30)), (30, (30, 0))]);
    let reads = RefCell::new(Vec::new());
    let selected = PlayerCollectionStateLikeCpp::static_heirloom_upgrade(
        10,
        |id| {
            reads.borrow_mut().push(Read::Lookup(id));
            rows.get(&id)
        },
        |row| {
            reads.borrow_mut().push(Read::Item(row.0));
            row.0
        },
        |row| {
            reads.borrow_mut().push(Read::Static(row.1));
            row.1
        },
        |id| {
            reads.borrow_mut().push(Read::Contains(id));
            id != 20
        },
    );
    assert_eq!(selected, Some(30));
    assert_eq!(
        reads.into_inner(),
        vec![
            Read::Lookup(10),
            Read::Item(10),
            Read::Contains(10),
            Read::Static(20),
            Read::Lookup(20),
            Read::Item(20),
            Read::Lookup(20),
            Read::Item(20),
            Read::Contains(20),
            Read::Static(30),
            Read::Lookup(30),
            Read::Item(30),
            Read::Lookup(30),
            Read::Item(30),
            Read::Contains(30),
            Read::Static(0),
            Read::Lookup(0),
        ],
    );
}

#[test]
fn static_heirloom_negative_start_skips_every_callback() {
    assert_eq!(
        PlayerCollectionStateLikeCpp::static_heirloom_upgrade::<(i32, i32)>(
            -1,
            |_| panic!("negative start must not query"),
            |_| panic!("negative start must not project"),
            |_| panic!("negative start must not project"),
            |_| panic!("negative start must not read inventory"),
        ),
        None,
    );
}

#[test]
fn static_heirloom_negative_current_item_returns_before_inventory_and_next_row() {
    let row = (-1, 20);
    assert_eq!(
        PlayerCollectionStateLikeCpp::static_heirloom_upgrade(
            10,
            |_| Some(&row),
            |row| row.0,
            |_| panic!("negative item must not read successor"),
            |_| panic!("negative item must not read inventory"),
        ),
        None,
    );
}

#[test]
fn static_heirloom_missing_or_negative_successor_keeps_last_owned_item() {
    let missing = BTreeMap::from([(10, (10, 20))]);
    assert_eq!(select(10, &missing, &[10]), Some(10));
    let negative = BTreeMap::from([(10, (10, -1))]);
    assert_eq!(select(10, &negative, &[10]), Some(10));
    let negative_row = BTreeMap::from([(10, (10, 20)), (20, (-1, 0))]);
    assert_eq!(select(10, &negative_row, &[10]), Some(10));
}

#[test]
fn static_heirloom_later_negative_current_row_discards_previous_selection() {
    let rows = BTreeMap::from([(10, (10, 20)), (20, (30, 0)), (30, (-1, 0))]);
    assert_eq!(select(10, &rows, &[10]), None);
}

#[test]
fn static_heirloom_missing_initial_row_and_no_owned_items_return_none() {
    assert_eq!(select(10, &BTreeMap::new(), &[]), None);
    let rows = BTreeMap::from([(10, (10, 0))]);
    assert_eq!(select(10, &rows, &[]), None);
    let zero = BTreeMap::from([(0, (0, -1))]);
    assert_eq!(select(0, &zero, &[0]), None);
}
