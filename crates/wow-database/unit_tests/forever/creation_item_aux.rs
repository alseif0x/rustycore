use super::*;
use crate::SqlParam;
use wow_persistence::forever::creation::save::{
    NewItem, NewItemArtifact, NewItemArtifactPower, NewItemGem, NewItemModifiers,
};

fn item() -> NewItem {
    super::super::tests::item()
}

#[test]
fn creation_item_absent_groups_still_delete_in_cpp_order() {
    let statements = statements(&item());
    assert_eq!(
        statements
            .iter()
            .map(|statement| statement.sql())
            .collect::<Vec<_>>(),
        [
            DELETE_GEMS,
            DELETE_TRANSMOG,
            DELETE_ARTIFACT,
            DELETE_ARTIFACT_POWERS,
            DELETE_MODIFIERS,
        ]
    );
}

#[test]
fn creation_item_empty_gems_delete_only_but_a_zero_entry_still_inserts_a_padded_row() {
    let mut item = item();
    let empty = statements(&item);
    assert!(!empty.iter().any(|statement| statement.sql() == INSERT_GEMS));

    item.gems.push(NewItemGem {
        item_id: 0,
        bonus_list_ids: [u16::MAX; 16],
        context: u8::MAX,
        scaling_level: u32::MAX,
    });
    let populated = statements(&item);
    let insert = populated
        .iter()
        .find(|statement| statement.sql() == INSERT_GEMS)
        .unwrap();
    let mut expected = vec![SqlParam::U64(item.guid)];
    for _ in 0..3 {
        expected.extend([
            SqlParam::U32(0),
            SqlParam::String(String::new()),
            SqlParam::U8(0),
            SqlParam::U32(0),
        ]);
    }
    assert_eq!(insert.params(), expected);
}

#[test]
fn creation_item_gems_filter_zero_bonuses_and_pad_all_three_sockets() {
    let mut item = item();
    item.gems = vec![
        NewItemGem {
            item_id: u32::MAX,
            bonus_list_ids: [0, u16::MAX, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            context: u8::MAX,
            scaling_level: u32::MAX,
        },
        NewItemGem {
            item_id: 2,
            bonus_list_ids: [1, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            context: 3,
            scaling_level: 4,
        },
    ];
    let insert = statements(&item)
        .into_iter()
        .find(|statement| statement.sql() == INSERT_GEMS)
        .unwrap();
    assert_eq!(
        insert.params(),
        &[
            SqlParam::U64(item.guid),
            SqlParam::U32(u32::MAX),
            SqlParam::String("65535 7 ".into()),
            SqlParam::U8(u8::MAX),
            SqlParam::U32(u32::MAX),
            SqlParam::U32(2),
            SqlParam::String("1 2 ".into()),
            SqlParam::U8(3),
            SqlParam::U32(4),
            SqlParam::U32(0),
            SqlParam::String(String::new()),
            SqlParam::U8(0),
            SqlParam::U32(0),
        ]
    );
}

#[test]
fn creation_item_transmog_binds_all_eighteen_values_as_u32_in_source_order() {
    let mut item = item();
    item.transmog_modifiers = std::array::from_fn(|index| index as u32 + 1);
    item.transmog_modifiers[17] = u32::MAX;
    let insert = statements(&item)
        .into_iter()
        .find(|statement| statement.sql() == INSERT_TRANSMOG)
        .unwrap();
    assert_eq!(insert.sql().matches('?').count(), 19);
    assert_eq!(insert.params()[0], SqlParam::U64(item.guid));
    let expected: Vec<_> = item
        .transmog_modifiers
        .iter()
        .copied()
        .map(SqlParam::U32)
        .collect();
    assert_eq!(&insert.params()[1..], expected);
}

#[test]
fn creation_item_empty_artifact_row_is_present_when_template_artifact_is_present() {
    let mut item = item();
    item.artifact = Some(NewItemArtifact {
        xp: 0,
        appearance_id: 0,
        tier_id: 0,
        powers: Vec::new(),
    });
    let statements = statements(&item);
    let sqls: Vec<_> = statements.iter().map(|statement| statement.sql()).collect();
    assert_eq!(
        sqls,
        [
            DELETE_GEMS,
            DELETE_TRANSMOG,
            DELETE_ARTIFACT,
            DELETE_ARTIFACT_POWERS,
            INSERT_ARTIFACT,
            DELETE_MODIFIERS,
        ]
    );
    let artifact = statements
        .iter()
        .find(|statement| statement.sql() == INSERT_ARTIFACT)
        .unwrap();
    assert_eq!(
        artifact.params(),
        &[
            SqlParam::U64(item.guid),
            SqlParam::U64(0),
            SqlParam::U32(0),
            SqlParam::U32(0),
        ]
    );
}

#[test]
fn creation_item_artifact_xp_rank_widths_and_power_order_are_preserved() {
    let mut item = item();
    item.artifact = Some(NewItemArtifact {
        xp: u64::MAX,
        appearance_id: u32::MAX,
        tier_id: 0x8000_0001,
        powers: vec![
            NewItemArtifactPower {
                id: 7,
                purchased_rank: u8::MAX,
            },
            NewItemArtifactPower {
                id: 3,
                purchased_rank: 4,
            },
        ],
    });
    let statements = statements(&item);
    let artifact = statements
        .iter()
        .find(|statement| statement.sql() == INSERT_ARTIFACT)
        .unwrap();
    assert_eq!(
        artifact.params(),
        &[
            SqlParam::U64(item.guid),
            SqlParam::U64(u64::MAX),
            SqlParam::U32(u32::MAX),
            SqlParam::U32(0x8000_0001),
        ]
    );
    let powers: Vec<_> = statements
        .iter()
        .filter(|statement| statement.sql() == INSERT_ARTIFACT_POWER)
        .map(|statement| statement.params())
        .collect();
    assert_eq!(
        powers,
        [
            &[
                SqlParam::U64(item.guid),
                SqlParam::U32(7),
                SqlParam::U8(u8::MAX),
            ][..],
            &[SqlParam::U64(item.guid), SqlParam::U32(3), SqlParam::U8(4),][..],
        ]
    );
}

#[test]
fn creation_item_modifiers_insert_when_only_artifact_knowledge_is_nonzero() {
    let mut item = item();
    item.modifiers = NewItemModifiers {
        timewalker_level: 0,
        artifact_knowledge_level: u32::MAX,
    };
    let statements = statements(&item);
    let insert = statements
        .iter()
        .find(|statement| statement.sql() == INSERT_MODIFIERS)
        .unwrap();
    assert_eq!(
        insert.params(),
        &[
            SqlParam::U64(item.guid),
            SqlParam::U32(0),
            SqlParam::U32(u32::MAX),
        ]
    );
    assert!(
        !statements
            .iter()
            .any(|statement| statement.sql() == INSERT_TRANSMOG
                || statement.sql() == INSERT_ARTIFACT)
    );
}

#[test]
fn creation_item_more_than_three_gems_fails_validation_before_statement_assembly() {
    let mut item = item();
    item.gems = vec![
        NewItemGem {
            item_id: 1,
            bonus_list_ids: [0; 16],
            context: 0,
            scaling_level: 0,
        };
        4
    ];
    assert_eq!(
        super::super::validate(&[item]),
        Err(CreateError::InvalidRow)
    );
}
