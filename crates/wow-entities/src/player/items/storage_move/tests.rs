use super::*;
use crate::{BANK_SLOT_ITEM_START, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, make_item_pos};
use std::cell::RefCell;
use std::rc::Rc;

fn item(low_guid: i64, entry_id: u32) -> PlayerInventoryItem {
    PlayerInventoryItem {
        guid: ObjectGuid::create_item(1, low_guid),
        entry_id,
        db_guid: low_guid as u64,
        inventory_type: None,
    }
}

fn position(bag: u8, slot: u8, count: u32) -> ItemPosCount {
    ItemPosCount::new(make_item_pos(bag, slot), count)
}

#[test]
fn empty_destination_becomes_moved_stack_without_object_or_template_reads() {
    let source = item(1, 700);
    let plan = plan_inventory_storage_move_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        source.clone(),
        3,
        &[position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, 3)],
        |_bag, _slot| None,
        |_guid| panic!("empty destinations do not read an object count"),
        |_entry| panic!("empty destinations do not read a template"),
    )
    .expect("empty destination is a valid remainder stack");

    assert_eq!(plan.source, source);
    assert!(plan.existing_updates.is_empty());
    assert_eq!(
        plan.moved_destination,
        Some((INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, 3))
    );
}

#[test]
fn merge_then_remainder_preserves_destination_and_read_order() {
    let source = item(1, 701);
    let existing = item(2, 701);
    let read_order = Rc::new(RefCell::new(Vec::new()));
    let item_reads = Rc::clone(&read_order);
    let count_reads = Rc::clone(&read_order);
    let template_reads = Rc::clone(&read_order);
    let existing_for_item = existing.clone();
    let existing_guid = existing.guid;
    let existing_entry = existing.entry_id;

    let plan = plan_inventory_storage_move_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        source,
        5,
        &[
            position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, 2),
            position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START + 1, 3),
        ],
        move |bag, slot| {
            item_reads.borrow_mut().push("destination");
            (bag == INVENTORY_SLOT_BAG_0 && slot == BANK_SLOT_ITEM_START)
                .then(|| existing_for_item.clone())
        },
        move |guid| {
            count_reads.borrow_mut().push("count");
            (guid == existing_guid).then_some(8)
        },
        move |entry| {
            template_reads.borrow_mut().push("template");
            (entry == existing_entry).then_some(10)
        },
    )
    .expect("merge plus remainder is valid");

    assert_eq!(
        read_order.borrow().as_slice(),
        ["destination", "count", "template", "destination"]
    );
    assert_eq!(plan.existing_updates.len(), 1);
    assert_eq!(plan.existing_updates[0].item.guid, existing.guid);
    assert_eq!(plan.existing_updates[0].new_count, 10);
    assert_eq!(
        plan.moved_destination,
        Some((INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START + 1, 3))
    );
}

#[test]
fn remainder_then_merge_preserves_both_allocated_destinations() {
    let source = item(1, 707);
    let existing = item(2, 707);
    let existing_guid = existing.guid;
    let existing_for_item = existing.clone();

    let plan = plan_inventory_storage_move_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        source,
        5,
        &[
            position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, 2),
            position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START + 1, 3),
        ],
        move |_bag, slot| (slot == BANK_SLOT_ITEM_START + 1).then(|| existing_for_item.clone()),
        move |guid| (guid == existing_guid).then_some(4),
        |_entry| Some(10),
    )
    .expect("remainder followed by a merge is valid");

    assert_eq!(plan.existing_updates.len(), 1);
    assert_eq!(plan.existing_updates[0].item.guid, existing.guid);
    assert_eq!(plan.existing_updates[0].bag, INVENTORY_SLOT_BAG_0);
    assert_eq!(plan.existing_updates[0].slot, BANK_SLOT_ITEM_START + 1);
    assert_eq!(plan.existing_updates[0].new_count, 7);
    assert_eq!(
        plan.moved_destination,
        Some((INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, 2))
    );
}

#[test]
fn full_merge_keeps_existing_update_order_without_remainder() {
    let source = item(1, 708);
    let first = item(2, 708);
    let second = item(3, 708);
    let first_for_item = first.clone();
    let second_for_item = second.clone();
    let first_guid = first.guid;
    let second_guid = second.guid;

    let plan = plan_inventory_storage_move_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        source,
        5,
        &[
            position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, 2),
            position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START + 1, 3),
        ],
        move |_bag, slot| match slot {
            BANK_SLOT_ITEM_START => Some(first_for_item.clone()),
            slot if slot == BANK_SLOT_ITEM_START + 1 => Some(second_for_item.clone()),
            _ => None,
        },
        move |guid| {
            if guid == first_guid {
                Some(8)
            } else if guid == second_guid {
                Some(4)
            } else {
                None
            }
        },
        |_entry| Some(10),
    )
    .expect("a source fully consumed by merges is valid");

    assert_eq!(plan.existing_updates.len(), 2);
    assert_eq!(plan.existing_updates[0].item.guid, first.guid);
    assert_eq!(plan.existing_updates[0].new_count, 10);
    assert_eq!(plan.existing_updates[1].item.guid, second.guid);
    assert_eq!(plan.existing_updates[1].new_count, 7);
    assert_eq!(plan.moved_destination, None);
}

#[test]
fn source_guid_or_entry_rejection_precedes_object_and_template_reads() {
    let source = item(1, 702);
    let same_guid = source.clone();
    let result = plan_inventory_storage_move_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        source,
        1,
        &[position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, 1)],
        move |_bag, _slot| Some(same_guid.clone()),
        |_guid| panic!("same-source destinations reject before object lookup"),
        |_entry| panic!("same-source destinations reject before template lookup"),
    );

    assert_eq!(result, Err(InventoryResult::CantStack));

    let source = item(3, 702);
    let wrong_entry = item(4, 799);
    let result = plan_inventory_storage_move_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        source,
        1,
        &[position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, 1)],
        move |_bag, _slot| Some(wrong_entry.clone()),
        |_guid| panic!("wrong-entry destinations reject before object lookup"),
        |_entry| panic!("wrong-entry destinations reject before template lookup"),
    );

    assert_eq!(result, Err(InventoryResult::CantStack));
}

#[test]
fn missing_destination_object_precedes_template_lookup() {
    let source = item(1, 703);
    let existing = item(2, 703);
    let result = plan_inventory_storage_move_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        source,
        1,
        &[position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, 1)],
        move |_bag, _slot| Some(existing.clone()),
        |_guid| None,
        |_entry| panic!("missing object rejects before template lookup"),
    );

    assert_eq!(result, Err(InventoryResult::ItemNotFound));
}

#[test]
fn max_stack_fallback_and_checked_stack_add_match_handler_rules() {
    let source = item(1, 704);
    let existing = item(2, 704);
    let result = plan_inventory_storage_move_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        source,
        1,
        &[position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, 1)],
        move |_bag, _slot| Some(existing.clone()),
        |_guid| Some(1),
        |_entry| None,
    );

    assert_eq!(result, Err(InventoryResult::CantStack));

    let source = item(3, 705);
    let existing = item(4, 705);
    let result = plan_inventory_storage_move_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        source,
        u32::MAX,
        &[position(
            INVENTORY_SLOT_BAG_0,
            BANK_SLOT_ITEM_START,
            u32::MAX,
        )],
        move |_bag, _slot| Some(existing.clone()),
        |_guid| Some(u32::MAX),
        |_entry| Some(u32::MAX),
    );

    assert_eq!(result, Err(InventoryResult::InternalBagError));
}

#[test]
fn self_only_allocation_is_the_cpp_noop_error() {
    let result = plan_inventory_storage_move_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        item(1, 706),
        1,
        &[position(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, 1)],
        |_bag, _slot| panic!("self-only allocation does not look up a destination"),
        |_guid| panic!("self-only allocation does not read an object"),
        |_entry| panic!("self-only allocation does not read a template"),
    );

    assert_eq!(result, Err(InventoryResult::InternalBagError));
}

#[test]
fn a_merge_can_leave_the_remainder_in_the_original_source_position() {
    let existing = item(2, 711);
    let plan = plan_inventory_storage_move_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        item(1, 711),
        5,
        &[
            position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, 2),
            position(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, 3),
        ],
        |bag, slot| {
            assert_eq!((bag, slot), (INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START));
            Some(existing.clone())
        },
        |_guid| Some(8),
        |_entry| Some(10),
    )
    .expect("the source remainder bypasses destination lookups");
    assert_eq!(plan.existing_updates.len(), 1);
    assert_eq!(plan.existing_updates[0].new_count, 10);
    assert_eq!(
        plan.moved_destination,
        Some((INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, 3))
    );
}

#[test]
fn zero_destination_and_total_mismatch_are_internal_errors() {
    let zero_count = plan_inventory_storage_move_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        item(1, 709),
        1,
        &[position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, 0)],
        |_bag, _slot| panic!("zero destination count rejects before destination lookup"),
        |_guid| panic!("zero destination count rejects before object lookup"),
        |_entry| panic!("zero destination count rejects before template lookup"),
    );
    assert_eq!(zero_count, Err(InventoryResult::InternalBagError));

    let total_mismatch = plan_inventory_storage_move_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        item(2, 709),
        2,
        &[position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, 1)],
        |_bag, _slot| None,
        |_guid| panic!("empty destinations do not read an object count"),
        |_entry| panic!("empty destinations do not read a template"),
    );
    assert_eq!(total_mismatch, Err(InventoryResult::InternalBagError));
}

#[test]
fn total_add_overflow_and_multiple_remainders_are_internal_errors() {
    let total_overflow = plan_inventory_storage_move_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        item(1, 710),
        u32::MAX,
        &[
            position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, u32::MAX),
            position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START + 1, 1),
        ],
        |_bag, _slot| None,
        |_guid| panic!("empty destinations do not read an object count"),
        |_entry| panic!("empty destinations do not read a template"),
    );
    assert_eq!(total_overflow, Err(InventoryResult::InternalBagError));

    let multiple_remainders = plan_inventory_storage_move_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        item(2, 710),
        2,
        &[
            position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, 1),
            position(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START + 1, 1),
        ],
        |_bag, _slot| None,
        |_guid| panic!("empty destinations do not read an object count"),
        |_entry| panic!("empty destinations do not read a template"),
    );
    assert_eq!(multiple_remainders, Err(InventoryResult::InternalBagError));
}
