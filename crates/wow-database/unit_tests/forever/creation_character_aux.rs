use super::*;
use crate::SqlParam;
use wow_persistence::forever::creation::save::NewCustomization;

#[test]
fn fishing_primary_customizations_battleground_and_inventory_keep_source_order() {
    for steps in [0, u8::MAX] {
        let mut character = super::super::tests::character();
        character.fishing_steps = steps;
        character.customizations = vec![NewCustomization {
            option: 21,
            choice: 22,
        }];
        character.items = vec![super::super::items::tests::item()];
        let mut transaction = SqlTransaction::new();
        super::super::append_character_and_inventory(&mut transaction, character.guid, &character);
        let mut expected = vec![
            "DELETE FROM character_fishingsteps WHERE guid = ?",
            super::super::CHARACTER_INSERT,
        ];
        if steps != 0 {
            expected.push("INSERT INTO character_fishingsteps (guid, fishingSteps) VALUES (?, ?)");
        }
        expected.extend([
            "DELETE FROM character_customizations WHERE guid = ?",
            "INSERT INTO character_customizations (guid, chrCustomizationOptionID, chrCustomizationChoiceID) VALUES (?, ?, ?)",
            "DELETE FROM character_battleground_data WHERE guid = ?",
            "INSERT INTO character_battleground_data (guid, instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, taxiStart, taxiEnd, mountSpell, queueId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            "REPLACE INTO character_inventory (guid, bag, slot, item) VALUES (?,?,?,?)",
        ]);
        let sqls = transaction.sqls_for_test();
        assert_eq!(&sqls[..expected.len()], expected.as_slice());
        assert!(sqls[expected.len()].starts_with("REPLACE INTO item_instance ("));
    }
}

#[test]
fn fishing_zero_skips_insert_and_u8_max_widens_to_sql_u32_with_guid_scope() {
    let mut transaction = SqlTransaction::new();
    append_fishing(&mut transaction, 17, 0);
    assert!(transaction.is_empty());
    append_fishing(&mut transaction, u64::MAX, u8::MAX);
    assert_eq!(
        transaction.sqls_for_test(),
        ["INSERT INTO character_fishingsteps (guid, fishingSteps) VALUES (?, ?)"]
    );
    for guid in [17, u64::MAX] {
        assert_eq!(
            fishing_delete_statement(guid).params(),
            &[SqlParam::U64(guid)]
        );
        assert_eq!(
            fishing_insert_statement(guid, u8::MAX).params(),
            &[SqlParam::U64(guid), SqlParam::U32(255)]
        );
    }
}

#[test]
fn battleground_initial_snapshot_always_deletes_then_inserts_with_its_guid() {
    let character = super::super::tests::character();
    // BGData uses TEAM_OTHER and a default WorldLocation, whose invalid map
    // is narrowed to uint16 by _SaveBGData rather than replaced with zero.
    for guid in [17, u64::MAX] {
        let mut transaction = SqlTransaction::new();
        append_battleground(&mut transaction, guid, &character.battleground);
        assert_eq!(
            transaction.sqls_for_test(),
            [
                "DELETE FROM character_battleground_data WHERE guid = ?",
                "INSERT INTO character_battleground_data (guid, instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, taxiStart, taxiEnd, mountSpell, queueId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            ]
        );
        assert_eq!(
            battleground_delete_statement(guid).params(),
            &[SqlParam::U64(guid)]
        );
        assert_eq!(
            battleground_insert_statement(guid, &character.battleground).params(),
            &[
                SqlParam::U64(guid),
                SqlParam::U32(0),
                SqlParam::U16(0),
                SqlParam::F32(0.0),
                SqlParam::F32(0.0),
                SqlParam::F32(0.0),
                SqlParam::F32(0.0),
                SqlParam::U16(u16::MAX),
                SqlParam::U32(0),
                SqlParam::U32(0),
                SqlParam::U32(0),
                SqlParam::U64(0),
            ]
        );
    }
}

#[test]
fn battleground_all_twelve_bindings_preserve_source_widths_and_field_order() {
    let data = NewBattlegroundData {
        instance: u32::MAX,
        team: u16::MAX,
        position: [1.25, -2.5, 3.75, -4.0],
        map: u16::MAX - 1,
        taxi: [0x8000_0001, u32::MAX],
        mount_spell: u32::MAX - 1,
        queue_id: u64::MAX,
    };
    let statement = battleground_insert_statement(0x1234_5678_9ABC_DEF0, &data);
    assert_eq!(statement.sql().matches('?').count(), 12);
    assert_eq!(
        statement.params(),
        &[
            SqlParam::U64(0x1234_5678_9ABC_DEF0),
            SqlParam::U32(u32::MAX),
            SqlParam::U16(u16::MAX),
            SqlParam::F32(1.25),
            SqlParam::F32(-2.5),
            SqlParam::F32(3.75),
            SqlParam::F32(-4.0),
            SqlParam::U16(u16::MAX - 1),
            SqlParam::U32(0x8000_0001),
            SqlParam::U32(u32::MAX),
            SqlParam::U32(u32::MAX - 1),
            SqlParam::U64(u64::MAX),
        ]
    );
}

#[test]
fn battleground_position_preserves_nonfinite_payloads_and_negative_zero_bits() {
    let bits = [0x7FC0_1234, 0x7F80_0000, 0xFF80_0000, 0x8000_0000];
    let data = NewBattlegroundData {
        instance: 0,
        team: 0,
        position: bits.map(f32::from_bits),
        map: u16::MAX,
        taxi: [0; 2],
        mount_spell: 0,
        queue_id: 0,
    };
    let statement = battleground_insert_statement(17, &data);
    for (parameter, expected_bits) in statement.params()[3..7].iter().zip(bits) {
        match parameter {
            SqlParam::F32(value) => assert_eq!(value.to_bits(), expected_bits),
            other => panic!("expected raw f32 parameter, got {other:?}"),
        }
    }
}
