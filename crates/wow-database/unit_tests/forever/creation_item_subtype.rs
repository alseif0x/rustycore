use super::*;
use crate::{SqlParam, SqlTransaction};
use wow_persistence::forever::creation::save::{
    NewItem, NewItemAzerite, NewItemAzeriteSelectedEssences, NewItemAzeriteUnlockedEssence,
    NewItemSubtype,
};

fn item() -> NewItem {
    super::super::tests::item()
}

fn azerite() -> NewItemAzerite {
    NewItemAzerite {
        xp: 0,
        level: 0,
        knowledge_level: 0,
        selected_essences: Vec::new(),
        unlocked_milestones: Vec::new(),
        unlocked_essences: Vec::new(),
    }
}

#[test]
fn creation_item_base_subtype_adds_no_subtype_statements() {
    let mut item = item();
    item.subtype = NewItemSubtype::Base;
    assert!(statements(&item).is_empty());
}

#[test]
fn creation_item_azerite_zero_xp_and_empty_vectors_still_write_the_parent_row() {
    let mut item = item();
    item.subtype = NewItemSubtype::Azerite(azerite());
    let statements = statements(&item);
    assert_eq!(
        statements
            .iter()
            .map(|statement| statement.sql())
            .collect::<Vec<_>>(),
        [
            DELETE_AZERITE,
            DELETE_AZERITE_MILESTONES,
            DELETE_AZERITE_ESSENCES,
            INSERT_AZERITE,
        ]
    );
    let insert = statements.last().unwrap();
    assert_eq!(insert.sql().matches('?').count(), 24);
    let mut expected = vec![
        SqlParam::U64(item.guid),
        SqlParam::U64(0),
        SqlParam::U32(0),
        SqlParam::U32(0),
    ];
    expected.extend((0..20).map(|_| SqlParam::U32(0)));
    assert_eq!(insert.params(), expected);
}

#[test]
fn creation_item_azerite_selected_group_zero_pads_missing_groups() {
    let mut item = item();
    let mut data = azerite();
    data.selected_essences = vec![NewItemAzeriteSelectedEssences {
        specialization_id: u32::MAX,
        essence_ids: [1, 2, 3, 4],
    }];
    item.subtype = NewItemSubtype::Azerite(data);
    let insert = statements(&item)
        .into_iter()
        .find(|statement| statement.sql() == INSERT_AZERITE)
        .unwrap();
    assert_eq!(
        &insert.params()[4..],
        &[
            SqlParam::U32(u32::MAX),
            SqlParam::U32(1),
            SqlParam::U32(2),
            SqlParam::U32(3),
            SqlParam::U32(4),
            SqlParam::U32(0),
            SqlParam::U32(0),
            SqlParam::U32(0),
            SqlParam::U32(0),
            SqlParam::U32(0),
            SqlParam::U32(0),
            SqlParam::U32(0),
            SqlParam::U32(0),
            SqlParam::U32(0),
            SqlParam::U32(0),
            SqlParam::U32(0),
            SqlParam::U32(0),
            SqlParam::U32(0),
            SqlParam::U32(0),
            SqlParam::U32(0),
        ]
    );
}

#[test]
fn creation_item_azerite_preserves_u64_xp_and_all_four_u32_selected_groups() {
    let mut item = item();
    let groups = [
        NewItemAzeriteSelectedEssences {
            specialization_id: u32::MAX,
            essence_ids: [u32::MAX, 2, 3, 4],
        },
        NewItemAzeriteSelectedEssences {
            specialization_id: 5,
            essence_ids: [6, 7, 8, 9],
        },
        NewItemAzeriteSelectedEssences {
            specialization_id: 10,
            essence_ids: [11, 12, 13, 14],
        },
        NewItemAzeriteSelectedEssences {
            specialization_id: 15,
            essence_ids: [16, 17, 18, 19],
        },
    ];
    let mut data = azerite();
    data.xp = u64::MAX;
    data.level = u32::MAX;
    data.knowledge_level = u32::MAX - 1;
    data.selected_essences = groups.to_vec();
    item.subtype = NewItemSubtype::Azerite(data);

    let insert = statements(&item)
        .into_iter()
        .find(|statement| statement.sql() == INSERT_AZERITE)
        .unwrap();
    let mut expected = vec![
        SqlParam::U64(item.guid),
        SqlParam::U64(u64::MAX),
        SqlParam::U32(u32::MAX),
        SqlParam::U32(u32::MAX - 1),
    ];
    for group in groups {
        expected.push(SqlParam::U32(group.specialization_id));
        expected.extend(group.essence_ids.into_iter().map(SqlParam::U32));
    }
    assert_eq!(insert.params(), expected);
}

#[test]
fn creation_item_azerite_milestones_and_unlocked_essences_keep_source_order_and_u32_rank() {
    let mut item = item();
    let mut data = azerite();
    data.unlocked_milestones = vec![u32::MAX, 7, 2];
    data.unlocked_essences = vec![
        NewItemAzeriteUnlockedEssence {
            id: 11,
            rank: u32::MAX,
        },
        NewItemAzeriteUnlockedEssence { id: 3, rank: 4 },
    ];
    item.subtype = NewItemSubtype::Azerite(data);
    let statements = statements(&item);
    assert_eq!(
        statements
            .iter()
            .map(|statement| statement.sql())
            .collect::<Vec<_>>(),
        [
            DELETE_AZERITE,
            DELETE_AZERITE_MILESTONES,
            DELETE_AZERITE_ESSENCES,
            INSERT_AZERITE,
            INSERT_AZERITE_MILESTONE,
            INSERT_AZERITE_MILESTONE,
            INSERT_AZERITE_MILESTONE,
            INSERT_AZERITE_ESSENCE,
            INSERT_AZERITE_ESSENCE,
        ]
    );
    assert_eq!(
        statements[4].params(),
        &[SqlParam::U64(item.guid), SqlParam::U32(u32::MAX)]
    );
    assert_eq!(
        statements[5].params(),
        &[SqlParam::U64(item.guid), SqlParam::U32(7)]
    );
    assert_eq!(
        statements[6].params(),
        &[SqlParam::U64(item.guid), SqlParam::U32(2)]
    );
    assert_eq!(
        statements[7].params(),
        &[
            SqlParam::U64(item.guid),
            SqlParam::U32(11),
            SqlParam::U32(u32::MAX),
        ]
    );
    assert_eq!(
        statements[8].params(),
        &[SqlParam::U64(item.guid), SqlParam::U32(3), SqlParam::U32(4),]
    );
}

#[test]
fn creation_item_azerite_rejects_more_than_four_selected_groups_before_statements() {
    let mut item = item();
    let group = NewItemAzeriteSelectedEssences {
        specialization_id: 1,
        essence_ids: [2, 3, 4, 5],
    };
    let mut data = azerite();
    data.selected_essences = vec![group; 5];
    item.subtype = NewItemSubtype::Azerite(data);
    assert_eq!(
        super::super::validate(&[item]),
        Err(CreateError::InvalidRow)
    );
}

#[test]
fn creation_item_azerite_empowered_deletes_then_inserts_five_signed_power_ids() {
    let mut item = item();
    item.subtype = NewItemSubtype::AzeriteEmpowered([i32::MIN, -1, 0, i32::MAX, -73]);
    let statements = statements(&item);
    assert_eq!(
        statements
            .iter()
            .map(|statement| statement.sql())
            .collect::<Vec<_>>(),
        [DELETE_AZERITE_EMPOWERED, INSERT_AZERITE_EMPOWERED]
    );
    let insert = statements.last().unwrap();
    assert_eq!(insert.sql().matches('?').count(), 6);
    assert_eq!(
        insert.params(),
        &[
            SqlParam::U64(item.guid),
            SqlParam::I32(i32::MIN),
            SqlParam::I32(-1),
            SqlParam::I32(0),
            SqlParam::I32(i32::MAX),
            SqlParam::I32(-73),
        ]
    );
}

#[test]
fn creation_item_mixed_subtypes_save_after_inventory_and_before_each_base_row() {
    let mut azerite_item = item();
    azerite_item.guid = 42;
    azerite_item.subtype = NewItemSubtype::Azerite(azerite());
    let mut base_item = item();
    base_item.guid = 43;
    base_item.slot = 20;
    base_item.subtype = NewItemSubtype::Base;
    let mut empowered_item = item();
    empowered_item.guid = 44;
    empowered_item.slot = 21;
    empowered_item.subtype = NewItemSubtype::AzeriteEmpowered([-1, -2, -3, -4, -5]);
    let items = [azerite_item, base_item, empowered_item];
    assert_eq!(super::super::validate(&items), Ok(()));

    let mut transaction = SqlTransaction::new();
    super::super::append(&mut transaction, 41, &items);
    assert_eq!(
        transaction.sqls_for_test(),
        [
            super::super::INVENTORY_INSERT,
            DELETE_AZERITE,
            DELETE_AZERITE_MILESTONES,
            DELETE_AZERITE_ESSENCES,
            INSERT_AZERITE,
            super::super::ITEM_INSERT,
            super::super::auxiliary::DELETE_GEMS,
            super::super::auxiliary::DELETE_TRANSMOG,
            super::super::auxiliary::DELETE_ARTIFACT,
            super::super::auxiliary::DELETE_ARTIFACT_POWERS,
            super::super::auxiliary::DELETE_MODIFIERS,
            super::super::INVENTORY_INSERT,
            super::super::ITEM_INSERT,
            super::super::auxiliary::DELETE_GEMS,
            super::super::auxiliary::DELETE_TRANSMOG,
            super::super::auxiliary::DELETE_ARTIFACT,
            super::super::auxiliary::DELETE_ARTIFACT_POWERS,
            super::super::auxiliary::DELETE_MODIFIERS,
            super::super::INVENTORY_INSERT,
            DELETE_AZERITE_EMPOWERED,
            INSERT_AZERITE_EMPOWERED,
            super::super::ITEM_INSERT,
            super::super::auxiliary::DELETE_GEMS,
            super::super::auxiliary::DELETE_TRANSMOG,
            super::super::auxiliary::DELETE_ARTIFACT,
            super::super::auxiliary::DELETE_ARTIFACT_POWERS,
            super::super::auxiliary::DELETE_MODIFIERS,
        ]
    );
    for (item, expected_guid) in items.iter().zip([42, 43, 44]) {
        assert!(
            statements(item)
                .iter()
                .all(|statement| statement.params()[0] == SqlParam::U64(expected_guid))
        );
    }
}
