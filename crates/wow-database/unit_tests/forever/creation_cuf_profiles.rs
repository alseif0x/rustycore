use super::*;
use crate::SqlParam;
use wow_persistence::forever::creation::save::{NewCurrencySave, NewCurrencySaveState};

fn profile(name: &str) -> NewCufProfile {
    NewCufProfile {
        name: name.into(),
        frame_height: 40,
        frame_width: 80,
        sort_by: 1,
        health_text: 2,
        bool_options: [false; 27],
        top_point: 1,
        bottom_point: 2,
        left_point: 3,
        top_offset: 10,
        bottom_offset: 20,
        left_offset: 30,
    }
}

#[test]
fn absent_profiles_always_delete_all_five_slots_in_owner_order() {
    for guid in [17, 0x12_0000_0009] {
        let profiles = std::array::from_fn(|_| None);
        let rows: Vec<_> = statements(guid, &profiles).collect();
        assert_eq!(rows.len(), 5);
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(
                row.sql(),
                "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?"
            );
            assert_eq!(
                row.params(),
                &[SqlParam::U64(guid), SqlParam::U8(index as u8)]
            );
        }
        let mut transaction = crate::forever::create::tests::isolated_transaction();
        append(&mut transaction, guid, &profiles);
        assert_eq!(transaction.sqls_for_test(), vec![DELETE; 5]);
    }
}

#[test]
fn mixed_profiles_emit_one_statement_per_array_slot_without_record_ids() {
    let mut profiles = std::array::from_fn(|_| None);
    profiles[1] = Some(profile("Équipe🌙"));
    profiles[4] = Some(profile("Raid"));
    let rows: Vec<_> = statements(29, &profiles).collect();
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [DELETE, REPLACE, DELETE, DELETE, REPLACE]
    );
    for (index, row) in rows.iter().enumerate() {
        assert_eq!(
            &row.params()[..2],
            &[SqlParam::U64(29), SqlParam::U8(index as u8)]
        );
    }
    assert_eq!(rows[1].params()[2], SqlParam::String("Équipe🌙".into()));
    assert_eq!(rows[4].params()[2], SqlParam::String("Raid".into()));
}

#[test]
fn encoder_limits_preserve_all_fourteen_source_parameter_widths() {
    // Binding ranges only, not canonical profile or database admission proof.
    let mut data = profile("Équipe🌙");
    data.frame_height = u16::MAX;
    data.frame_width = u16::MAX - 1;
    data.sort_by = u8::MAX;
    data.health_text = u8::MAX - 1;
    for bit in [0, 13, 26] {
        data.bool_options[bit] = true;
    }
    data.top_point = u8::MAX;
    data.bottom_point = u8::MAX - 1;
    data.left_point = u8::MAX - 2;
    data.top_offset = u16::MAX;
    data.bottom_offset = u16::MAX - 1;
    data.left_offset = u16::MAX - 2;
    let row = profile_statement(u64::MAX, 4, Some(&data));
    assert_eq!(
        row.sql(),
        "REPLACE INTO character_cuf_profiles (guid, id, name, frameHeight, frameWidth, sortBy, healthText, boolOptions, topPoint, bottomPoint, leftPoint, topOffset, bottomOffset, leftOffset) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(row.sql().matches('?').count(), 14);
    assert_eq!(
        row.params(),
        &[
            SqlParam::U64(u64::MAX),
            SqlParam::U8(4),
            SqlParam::String("Équipe🌙".into()),
            SqlParam::U16(u16::MAX),
            SqlParam::U16(u16::MAX - 1),
            SqlParam::U8(u8::MAX),
            SqlParam::U8(u8::MAX - 1),
            SqlParam::U32(0x0400_2001),
            SqlParam::U8(u8::MAX),
            SqlParam::U8(u8::MAX - 1),
            SqlParam::U8(u8::MAX - 2),
            SqlParam::U16(u16::MAX),
            SqlParam::U16(u16::MAX - 1),
            SqlParam::U16(u16::MAX - 2),
        ]
    );
}

#[test]
fn twenty_seven_boolean_options_pack_each_bit_and_all_bits_exactly() {
    let mut data = profile("Raid");
    assert_eq!(
        profile_statement(17, 0, Some(&data)).params()[7],
        SqlParam::U32(0)
    );
    for bit in 0..27 {
        data.bool_options = [false; 27];
        data.bool_options[bit] = true;
        assert_eq!(
            profile_statement(17, 0, Some(&data)).params()[7],
            SqlParam::U32(1u32 << bit)
        );
    }
    data.bool_options = [true; 27];
    assert_eq!(
        profile_statement(17, 0, Some(&data)).params()[7],
        SqlParam::U32(0x07FF_FFFF)
    );
}

#[test]
fn allocated_empty_zero_profile_still_replaces_without_dirty_predicate() {
    // DTO projection only; this does not establish canonical profile admission.
    let data = NewCufProfile {
        name: String::new(),
        frame_height: 0,
        frame_width: 0,
        sort_by: 0,
        health_text: 0,
        bool_options: [false; 27],
        top_point: 0,
        bottom_point: 0,
        left_point: 0,
        top_offset: 0,
        bottom_offset: 0,
        left_offset: 0,
    };
    let mut profiles = std::array::from_fn(|_| None);
    profiles[0] = Some(data);
    let rows: Vec<_> = statements(17, &profiles).collect();
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [REPLACE, DELETE, DELETE, DELETE, DELETE]
    );
    assert_eq!(rows[0].params()[2], SqlParam::String(String::new()));
    assert_eq!(rows[0].params()[7], SqlParam::U32(0));
}

#[test]
fn character_save_places_all_five_cuf_slots_immediately_after_currency() {
    let mut character = super::super::tests::character();
    character.guid = 0x12_0000_0009;
    character.currencies = vec![NewCurrencySave {
        currency_id: 17,
        has_currency_entry: true,
        state: NewCurrencySaveState::New,
        quantity: 1,
        weekly_quantity: 2,
        tracked_quantity: 3,
        increased_cap_quantity: 4,
        earned_quantity: 5,
        flags: 0,
    }];
    character.cuf_profiles[2] = Some(profile("Raid"));
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    super::super::append_character_save(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let currency = sqls
        .iter()
        .position(|sql| sql.starts_with("REPLACE INTO character_currency "))
        .unwrap();
    assert_eq!(
        &sqls[currency + 1..],
        &[
            DELETE,
            DELETE,
            REPLACE,
            DELETE,
            DELETE,
            "DELETE FROM character_bank_tab_settings WHERE characterGuid = ?"
        ]
    );
    // Borrowed frozen snapshot preservation, not canonical flag acknowledgement.
    assert_eq!(character.cuf_profiles[2].as_ref().unwrap().name, "Raid");
}
