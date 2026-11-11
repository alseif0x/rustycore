use super::*;
use crate::SqlParam;
use std::collections::BTreeMap;
use wow_persistence::forever::creation::save::{NewPlayerDataSave, NewPlayerDataValue};

fn tab(name: &str, icon: &str, description: &str, deposit_flags: i32) -> NewBankTabSettings {
    NewBankTabSettings {
        name: name.into(),
        icon: icon.into(),
        description: description.into(),
        deposit_flags,
    }
}

#[test]
fn empty_tabs_always_delete_only_the_character_owner() {
    for guid in [17, 0x12_0000_0009] {
        let rows: Vec<_> = statements(guid, &[]).collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].sql(),
            "DELETE FROM character_bank_tab_settings WHERE characterGuid = ?"
        );
        assert_eq!(rows[0].params(), &[SqlParam::U64(guid)]);
        let mut transaction = crate::forever::create::tests::isolated_transaction();
        append(&mut transaction, guid, &[]);
        assert_eq!(transaction.sqls_for_test(), [DELETE]);
    }
}

#[test]
fn supplied_vector_order_binds_all_six_fields_with_unique_generated_indices() {
    let guid = 0x12_0000_0009;
    let tabs = [
        tab("Équipe🌙", "INV_Sword_01", "Réserve pour raid", 3),
        tab("Matériaux", "INV_Ore_01", "Forge", 1),
    ];
    let rows: Vec<_> = statements(guid, &tabs).collect();
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [DELETE, INSERT, INSERT]
    );
    assert_eq!(
        rows[1].sql(),
        "INSERT INTO character_bank_tab_settings (characterGuid, tabId, name, icon, description, depositFlags) VALUES (?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(rows[1].sql().matches('?').count(), 6);
    for (index, data) in tabs.iter().enumerate() {
        assert_eq!(
            rows[index + 1].params(),
            &[
                SqlParam::U64(guid),
                SqlParam::U8(index as u8),
                SqlParam::String(data.name.clone()),
                SqlParam::String(data.icon.clone()),
                SqlParam::String(data.description.clone()),
                SqlParam::I32(data.deposit_flags),
            ]
        );
    }
}

#[test]
fn signed_flag_encoding_bounds_and_empty_strings_still_insert_every_allocated_tab() {
    // Signed binding ranges only, not canonical deposit-policy admission proof.
    let tabs = [
        tab("", "", "", i32::MIN),
        tab("", "", "", 0),
        tab("", "", "", i32::MAX),
    ];
    let rows: Vec<_> = statements(17, &tabs).collect();
    assert_eq!(rows.len(), 4);
    for (index, flags) in [i32::MIN, 0, i32::MAX].into_iter().enumerate() {
        assert_eq!(rows[index + 1].sql(), INSERT);
        assert_eq!(
            rows[index + 1].params(),
            &[
                SqlParam::U64(17),
                SqlParam::U8(index as u8),
                SqlParam::String(String::new()),
                SqlParam::String(String::new()),
                SqlParam::String(String::new()),
                SqlParam::I32(flags),
            ]
        );
    }
}

#[test]
fn encoder_only_vector_indices_narrow_to_u8_without_clamping_or_rejecting() {
    // 257 tabs deliberately exceed unique SQL tab keys: encoder behavior only,
    // not canonical cardinality or successful database admission proof.
    let tabs: Vec<_> = (0..257).map(|_| tab("", "", "", 0)).collect();
    let rows: Vec<_> = statements(17, &tabs).collect();
    assert_eq!(rows.len(), 258);
    for (index, row) in rows[1..].iter().enumerate() {
        assert_eq!(row.params()[1], SqlParam::U8(index as u8));
    }
    assert_eq!(rows[256].params()[1], SqlParam::U8(255));
    assert_eq!(rows[257].params()[1], SqlParam::U8(0));
}

#[test]
fn character_save_places_bank_delete_and_tabs_after_cuf_elements_and_flag_words() {
    let mut character = super::super::tests::character();
    character.guid = 0x12_0000_0009;
    character.player_data = NewPlayerDataSave {
        elements: BTreeMap::from([(90, Some(NewPlayerDataValue::Int64(-7)))]),
        flags: BTreeMap::from([(2, 5)]),
    };
    character.bank_tabs = vec![tab("Raid", "INV_Sword_01", "Réserve", 3)];
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    super::super::append_character_save(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let cuf = sqls
        .iter()
        .position(|sql| *sql == "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?")
        .unwrap();
    assert_eq!(
        &sqls[cuf..],
        &[
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_player_data_element WHERE characterGuid = ? AND playerDataElementCharacterId = ?",
            "INSERT INTO character_player_data_element (characterGuid, playerDataElementCharacterId, floatValue, int64Value) VALUES (?, ?, ?, ?)",
            "DELETE FROM character_player_data_flag WHERE characterGuid = ? AND storageIndex = ?",
            "INSERT INTO character_player_data_flag (characterGuid, storageIndex, mask) VALUES (?, ?, ?)",
            DELETE,
            INSERT,
        ]
    );
    // Borrowed frozen DTO preservation, not canonical state acknowledgement.
    assert_eq!(character.bank_tabs[0].name, "Raid");
}
