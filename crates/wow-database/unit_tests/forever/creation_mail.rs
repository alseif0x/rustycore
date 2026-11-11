use super::*;
use crate::SqlParam;

fn changed(items: Vec<u64>, removed_items: Vec<u64>) -> NewMailSaveEntry {
    NewMailSaveEntry::Changed {
        message_id: 90,
        items,
        removed_items,
        expire_time: 123,
        deliver_time: 100,
        money: 71,
        cod: 29,
        checked: 3,
    }
}

fn attachment_sqls() -> [&'static str; 11] {
    [
        "DELETE FROM item_instance WHERE guid = ?",
        "DELETE FROM item_instance_gems WHERE itemGuid = ?",
        "DELETE FROM item_instance_transmog WHERE itemGuid = ?",
        "DELETE FROM item_instance_artifact WHERE itemGuid = ?",
        "DELETE FROM item_instance_artifact_powers WHERE itemGuid = ?",
        "DELETE FROM item_instance_modifiers WHERE itemGuid = ?",
        "DELETE FROM character_gifts WHERE item_guid = ?",
        "DELETE FROM item_instance_azerite WHERE itemGuid = ?",
        "DELETE FROM item_instance_azerite_milestone_power WHERE itemGuid = ?",
        "DELETE FROM item_instance_azerite_unlocked_essence WHERE itemGuid = ?",
        "DELETE FROM item_instance_azerite_empowered WHERE itemGuid = ?",
    ]
}

#[test]
fn frozen_updated_gate_skips_every_state_and_unchanged_never_emits_sql() {
    // Frozen gate only, not canonical mail ownership or post-save acknowledgement proof.
    let data = NewMailSave {
        updated: false,
        messages: vec![
            changed(vec![17], vec![2]),
            NewMailSaveEntry::Deleted {
                message_id: 7,
                items: vec![3],
            },
            NewMailSaveEntry::Unchanged,
        ],
    };
    let mut transaction = SqlTransaction::new();
    append(&mut transaction, &data);
    assert!(transaction.sqls_for_test().is_empty());
    let unchanged = NewMailSave {
        updated: true,
        messages: vec![NewMailSaveEntry::Unchanged],
    };
    append(&mut transaction, &unchanged);
    assert!(transaction.sqls_for_test().is_empty());
    assert!(!data.updated);
    assert!(unchanged.updated);
}

#[test]
fn changed_mail_binds_exact_seven_fields_and_derives_has_items_from_vector() {
    for items in [vec![], vec![17], vec![17, 2]] {
        let expected_has_items = u8::from(!items.is_empty());
        let row = changed_statement(&changed(items, Vec::new())).unwrap();
        assert_eq!(
            row.sql(),
            "UPDATE mail SET has_items = ?, expire_time = ?, deliver_time = ?, money = ?, cod = ?, checked = ? WHERE id = ?"
        );
        assert_eq!(row.sql().matches('?').count(), 7);
        assert_eq!(
            row.params(),
            &[
                SqlParam::U8(expected_has_items),
                SqlParam::I64(123),
                SqlParam::I64(100),
                SqlParam::U64(71),
                SqlParam::U64(29),
                SqlParam::U32(3),
                SqlParam::U64(90),
            ]
        );
    }
}

#[test]
fn encoder_bounds_keep_signed_times_unsigned_money_ids_and_checked_u8_then_u32() {
    // SQL setter ranges only, not canonical identity or database admission proof.
    for (checked, narrowed) in [(0, 0), (0x1234_56ab, 0xab), (u32::MAX, 255)] {
        let data = NewMailSaveEntry::Changed {
            message_id: u64::MAX,
            items: vec![],
            removed_items: vec![],
            expire_time: i64::MIN,
            deliver_time: i64::MAX,
            money: u64::MAX,
            cod: u64::MAX - 1,
            checked,
        };
        let row = changed_statement(&data).unwrap();
        assert_eq!(
            row.params(),
            &[
                SqlParam::U8(0),
                SqlParam::I64(i64::MIN),
                SqlParam::I64(i64::MAX),
                SqlParam::U64(u64::MAX),
                SqlParam::U64(u64::MAX - 1),
                SqlParam::U32(narrowed),
                SqlParam::U64(u64::MAX),
            ]
        );
    }
    for sql in [LINK_DELETE, MAIL_DELETE, LINKS_DELETE] {
        let row = delete_statement(sql, u64::MAX);
        assert_eq!(row.params(), &[SqlParam::U64(u64::MAX)]);
        assert_eq!(row.sql().matches('?').count(), 1);
    }
}

#[test]
fn changed_removals_delete_only_links_in_supplied_order_without_item_deletion() {
    let data = NewMailSave {
        updated: true,
        messages: vec![changed(vec![17], vec![90, 2, 7])],
    };
    let mut transaction = SqlTransaction::new();
    append(&mut transaction, &data);
    assert_eq!(
        transaction.sqls_for_test(),
        [
            UPDATE,
            "DELETE FROM mail_items WHERE item_guid = ?",
            "DELETE FROM mail_items WHERE item_guid = ?",
            "DELETE FROM mail_items WHERE item_guid = ?",
        ]
    );
    if let NewMailSaveEntry::Changed { removed_items, .. } = &data.messages[0] {
        let params = transaction.params_for_test();
        assert_eq!(params.len(), 4);
        assert_eq!(
            params[0],
            &[
                SqlParam::U8(1),
                SqlParam::I64(123),
                SqlParam::I64(100),
                SqlParam::U64(71),
                SqlParam::U64(29),
                SqlParam::U32(3),
                SqlParam::U64(90),
            ]
        );
        for (actual, expected) in params[1..].iter().zip([90, 2, 7]) {
            // Exactly one item counter: no owner or message scope is added.
            assert_eq!(*actual, &[SqlParam::U64(expected)]);
        }
        // Frozen removed list remains intact; canonical mutation is outside this adapter.
        assert_eq!(removed_items.len(), 3);
    } else {
        panic!("expected changed mail");
    }
}

#[test]
fn deleted_mail_empty_has_two_rows_and_each_attachment_has_all_eleven_before_message_deletes() {
    // The final case exercises encoder widths only, not canonical ID admission.
    for (attachments, message_id) in [
        (vec![], 7),
        (vec![90, 2], 7),
        (vec![u64::MAX], u64::MAX - 1),
    ] {
        let count = attachments.len();
        let expected_ids = attachments.clone();
        let data = NewMailSave {
            updated: true,
            messages: vec![NewMailSaveEntry::Deleted {
                message_id,
                items: attachments,
            }],
        };
        let mut transaction = SqlTransaction::new();
        append(&mut transaction, &data);
        let mut expected = Vec::new();
        for _ in 0..count {
            expected.extend(attachment_sqls());
        }
        expected.extend([
            "DELETE FROM mail WHERE id = ?",
            "DELETE FROM mail_items WHERE mail_id = ?",
        ]);
        assert_eq!(transaction.sqls_for_test(), expected);
        assert_eq!(expected.len(), count * 11 + 2);
        assert!(expected.iter().all(|sql| sql.matches('?').count() == 1));
        let params = transaction.params_for_test();
        assert_eq!(params.len(), count * 11 + 2);
        for (chunk, item) in params[..count * 11].chunks_exact(11).zip(expected_ids) {
            for actual in chunk {
                // Every actual attachment statement binds only its item GUID.
                assert_eq!(*actual, &[SqlParam::U64(item)]);
            }
        }
        assert_eq!(params[count * 11], &[SqlParam::U64(message_id)]);
        assert_eq!(params[count * 11 + 1], &[SqlParam::U64(message_id)]);
    }
}

#[test]
fn mixed_deque_order_keeps_changed_links_deleted_attachments_and_later_updates_together() {
    let mut final_message = changed(vec![], vec![]);
    if let NewMailSaveEntry::Changed { message_id, .. } = &mut final_message {
        *message_id = 91;
    }
    let data = NewMailSave {
        updated: true,
        messages: vec![
            changed(vec![17], vec![90]),
            NewMailSaveEntry::Unchanged,
            NewMailSaveEntry::Deleted {
                message_id: 2,
                items: vec![7],
            },
            final_message,
        ],
    };
    let mut transaction = SqlTransaction::new();
    append(&mut transaction, &data);
    let mut expected = vec![UPDATE, LINK_DELETE];
    expected.extend(attachment_sqls());
    expected.extend([MAIL_DELETE, LINKS_DELETE, UPDATE]);
    assert_eq!(transaction.sqls_for_test(), expected);
    let params = transaction.params_for_test();
    assert_eq!(params[0][6], SqlParam::U64(90));
    assert_eq!(params.last().unwrap()[6], SqlParam::U64(91));
}

#[test]
fn production_prefix_places_fishing_then_mail_then_customizations_battleground_and_inventory() {
    let mut character = super::super::tests::character();
    character.guid = 0x12_0000_0009;
    character.fishing_steps = 7;
    character.mail = NewMailSave {
        updated: true,
        messages: vec![changed(vec![], vec![90])],
    };
    character.customizations = vec![wow_persistence::forever::creation::save::NewCustomization {
        option: 17,
        choice: 2,
    }];
    character.items = vec![super::super::items::tests::item()];
    let mut transaction = SqlTransaction::new();
    super::super::append_character_and_inventory(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let fishing = sqls
        .iter()
        .position(|sql| sql.starts_with("INSERT INTO character_fishingsteps "))
        .unwrap();
    assert_eq!(
        &sqls[fishing + 1..fishing + 7],
        &[
            UPDATE,
            LINK_DELETE,
            "DELETE FROM character_customizations WHERE guid = ?",
            "INSERT INTO character_customizations (guid, chrCustomizationOptionID, chrCustomizationChoiceID) VALUES (?, ?, ?)",
            "DELETE FROM character_battleground_data WHERE guid = ?",
            "INSERT INTO character_battleground_data (guid, instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, taxiStart, taxiEnd, mountSpell, queueId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        ]
    );
    assert_eq!(
        sqls[fishing + 7],
        "REPLACE INTO character_inventory (guid, bag, slot, item) VALUES (?,?,?,?)"
    );
}
