//! Primary item save projection only; no inventory/equipment admission claim.
use super::*;
use crate::SqlParam;
use wow_persistence::forever::creation::save::{
    NewItemArtifact, NewItemArtifactPower, NewItemEnchantment, NewItemGem, NewItemModifiers,
    NewItemSubtype,
};

pub(in crate::forever::create) fn item() -> NewItem {
    let mut enchantments = [None; 13];
    enchantments[12] = Some(NewItemEnchantment {
        id: 7,
        duration: 8,
        charges: 9,
    });
    NewItem {
        guid: 42,
        entry: 17,
        count: 2,
        bag: None,
        slot: 19,
        creator: NewItemCreator::None,
        gift_creator_guid: 23,
        expiration: 24,
        charges: vec![-1, i32::MIN, 0, 12],
        dynamic_flags: 25,
        enchantments,
        random_bonus_list: 26,
        durability: u16::MAX,
        played_time: 27,
        create_time: -17,
        text: "Synthetic item text".into(),
        battle_pet_modifiers: [28, 29, 30, 31],
        context: 32,
        bonus_lists: vec![-2, 33],
        gems: Vec::new(),
        transmog_modifiers: [0; 18],
        artifact: None,
        modifiers: NewItemModifiers {
            timewalker_level: 0,
            artifact_knowledge_level: 0,
        },
        subtype: NewItemSubtype::Base,
    }
}

#[test]
fn primary_item_row_preserves_all_scalar_and_signed_sequence_values() {
    let statement = item_statement(41, &item(), &[]);
    let mut enchantments = "0 0 0 ".repeat(12);
    enchantments.push_str("7 8 9 ");
    assert_eq!(statement.sql().matches('?').count(), 21);
    assert_eq!(
        statement.params(),
        &[
            SqlParam::U32(17),
            SqlParam::U64(41),
            SqlParam::U64(0),
            SqlParam::U64(23),
            SqlParam::U32(2),
            SqlParam::U32(24),
            SqlParam::String("-1 -2147483648 0 12 ".into()),
            SqlParam::U32(25),
            SqlParam::String(enchantments),
            SqlParam::U32(26),
            SqlParam::U16(u16::MAX),
            SqlParam::U32(27),
            SqlParam::I64(-17),
            SqlParam::String("Synthetic item text".into()),
            SqlParam::U32(28),
            SqlParam::U32(29),
            SqlParam::U32(30),
            SqlParam::U32(31),
            SqlParam::U8(32),
            SqlParam::String("-2 33 ".into()),
            SqlParam::U64(42),
        ]
    );
}

#[test]
fn root_owner_is_not_implicitly_the_item_creator() {
    let mut item = item();
    for (creator, expected) in [
        (NewItemCreator::None, 0),
        (NewItemCreator::Character, 41),
        (NewItemCreator::Item(1), 102),
    ] {
        item.creator = creator;
        let statement = item_statement(41, &item, &[101, 102]);
        assert_eq!(statement.params()[1], SqlParam::U64(41));
        assert_eq!(statement.params()[2], SqlParam::U64(expected));
    }
}

#[test]
fn forward_container_and_child_creator_references_use_the_same_allocated_parent() {
    let mut child = item();
    child.guid = 101;
    child.bag = Some(1);
    child.slot = 0;
    child.creator = NewItemCreator::Item(1);
    let mut parent = item();
    parent.guid = 102;
    let items = [child, parent];
    assert_eq!(validate(&items), Ok(()));
    let inventory = inventory_statement(41, &items[0], &[101, 102]);
    assert_eq!(
        inventory.params(),
        &[
            SqlParam::U64(41),
            SqlParam::U64(102),
            SqlParam::U8(0),
            SqlParam::U64(101),
        ]
    );
    assert_eq!(
        item_statement(41, &items[0], &[101, 102]).params()[2],
        SqlParam::U64(102)
    );
}

#[test]
fn position_keys_distinguish_root_and_container_slots_but_reject_replacement() {
    let mut root = item();
    root.guid = 41;
    root.slot = 20;
    let mut contents = item();
    contents.bag = Some(0);
    let mut items = vec![root, contents, item()];
    items[2].guid = 43;
    assert_eq!(validate(&items), Ok(()));
    items[2].bag = Some(0);
    assert_eq!(validate(&items), Err(CreateError::InvalidRow));
    items[2].bag = None;
    items[2].slot = 20;
    assert_eq!(validate(&items), Err(CreateError::InvalidRow));
}

#[test]
fn zero_items_dangling_references_self_references_and_cycles_fail_before_binding() {
    assert_eq!(validate(&[]), Ok(()));
    for zero_entry in [false, true] {
        let mut item = item();
        if zero_entry {
            item.entry = 0;
        } else {
            item.count = 0;
        }
        assert_eq!(validate(&[item]), Err(CreateError::InvalidRow));
    }
    for reference in [0, 1, usize::MAX] {
        for creator_reference in [false, true] {
            let mut item = item();
            if creator_reference {
                item.creator = NewItemCreator::Item(reference);
            } else {
                item.bag = Some(reference);
            }
            assert_eq!(validate(&[item]), Err(CreateError::InvalidRow));
        }
    }
    for creator_reference in [false, true] {
        let mut items = [item(), item()];
        items[1].guid = 43;
        items[1].slot = 20;
        if creator_reference {
            items[0].creator = NewItemCreator::Item(1);
            items[1].creator = NewItemCreator::Item(0);
        } else {
            items[0].bag = Some(1);
            items[1].bag = Some(0);
        }
        assert_eq!(validate(&items), Err(CreateError::InvalidRow));
    }
}

#[test]
fn inventory_primary_and_all_base_group_deletes_keep_the_source_order() {
    let items = [item()];
    assert_eq!(validate(&items), Ok(()));
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    append(&mut transaction, 41, &items);
    assert_eq!(
        transaction.sqls_for_test(),
        [
            INVENTORY_INSERT,
            ITEM_INSERT,
            auxiliary::DELETE_GEMS,
            auxiliary::DELETE_TRANSMOG,
            auxiliary::DELETE_ARTIFACT,
            auxiliary::DELETE_ARTIFACT_POWERS,
            auxiliary::DELETE_MODIFIERS,
        ]
    );
}

#[test]
fn multiple_items_keep_each_inventory_primary_and_auxiliary_group_before_the_next_item() {
    let mut first = item();
    first.guid = 42;
    let mut second = item();
    second.guid = 43;
    second.slot = 20;
    second.gems = vec![NewItemGem {
        item_id: 900,
        bonus_list_ids: [0; 16],
        context: 7,
        scaling_level: 8,
    }];
    second.transmog_modifiers[17] = u32::MAX;
    second.artifact = Some(NewItemArtifact {
        xp: u64::MAX,
        appearance_id: 11,
        tier_id: 12,
        powers: vec![NewItemArtifactPower {
            id: 13,
            purchased_rank: u8::MAX,
        }],
    });
    second.modifiers = NewItemModifiers {
        timewalker_level: 14,
        artifact_knowledge_level: 15,
    };
    let items = [first, second];
    assert_eq!(validate(&items), Ok(()));

    let mut transaction = crate::forever::create::tests::isolated_transaction();
    append(&mut transaction, 41, &items);
    assert_eq!(
        transaction.sqls_for_test(),
        [
            INVENTORY_INSERT,
            ITEM_INSERT,
            auxiliary::DELETE_GEMS,
            auxiliary::DELETE_TRANSMOG,
            auxiliary::DELETE_ARTIFACT,
            auxiliary::DELETE_ARTIFACT_POWERS,
            auxiliary::DELETE_MODIFIERS,
            INVENTORY_INSERT,
            ITEM_INSERT,
            auxiliary::DELETE_GEMS,
            auxiliary::INSERT_GEMS,
            auxiliary::DELETE_TRANSMOG,
            auxiliary::INSERT_TRANSMOG,
            auxiliary::DELETE_ARTIFACT,
            auxiliary::DELETE_ARTIFACT_POWERS,
            auxiliary::INSERT_ARTIFACT,
            auxiliary::INSERT_ARTIFACT_POWER,
            auxiliary::DELETE_MODIFIERS,
            auxiliary::INSERT_MODIFIERS,
        ]
    );
    assert_eq!(
        inventory_statement(41, &items[0], &[42, 43]).params()[3],
        SqlParam::U64(42)
    );
    assert_eq!(
        inventory_statement(41, &items[1], &[42, 43]).params()[3],
        SqlParam::U64(43)
    );
    for (item, expected_guid) in items.iter().zip([42, 43]) {
        assert!(
            auxiliary::statements(item)
                .iter()
                .all(|statement| statement.params()[0] == SqlParam::U64(expected_guid))
        );
    }
}

#[test]
fn absent_enchantments_write_all_thirteen_slots_and_empty_sequences_stay_empty() {
    let mut item = item();
    item.enchantments = [None; 13];
    item.charges.clear();
    item.bonus_lists.clear();
    let statement = item_statement(41, &item, &[]);
    assert_eq!(statement.params()[6], SqlParam::String(String::new()));
    assert_eq!(statement.params()[8], SqlParam::String("0 0 0 ".repeat(13)));
    assert_eq!(statement.params()[19], SqlParam::String(String::new()));
}

#[test]
fn item_identities_must_be_nonzero_and_distinct_even_in_different_slots() {
    let mut items = [item(), item()];
    items[1].slot = 20;
    assert_eq!(validate(&items), Err(CreateError::InvalidRow));
    items[1].guid = 43;
    assert_eq!(validate(&items), Ok(()));
    items[0].guid = 0;
    assert_eq!(validate(&items), Err(CreateError::InvalidRow));
}
