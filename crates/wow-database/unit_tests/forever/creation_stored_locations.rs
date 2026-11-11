use super::*;
use crate::SqlParam;
use wow_persistence::forever::creation::save::NewSkill;

fn position() -> NewStoredAuraPosition {
    NewStoredAuraPosition {
        map: 42,
        position: [1.0, 2.0, 3.0, 4.0],
    }
}

#[test]
fn three_states_preserve_unrepaired_source_spell_zero_delete_default() {
    assert_eq!(statements(17, &[]).count(), 0);
    for guid in [17, u64::MAX] {
        for spell in [0, u32::MAX] {
            let unchanged = NewStoredAuraLocation {
                spell,
                state: NewStoredAuraLocationState::Unchanged,
            };
            assert_eq!(statements(guid, &[unchanged]).count(), 0);

            let deleted = NewStoredAuraLocation {
                spell,
                state: NewStoredAuraLocationState::Deleted,
            };
            let rows: Vec<_> = statements(guid, &[deleted]).collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].sql(),
                "DELETE FROM character_aura_stored_location WHERE Guid = ? AND Spell = ?"
            );
            assert_eq!(
                rows[0].params(),
                &[SqlParam::U64(guid), SqlParam::Bool(false)]
            );

            let changed = NewStoredAuraLocation {
                spell,
                state: NewStoredAuraLocationState::Changed(position()),
            };
            let rows: Vec<_> = statements(guid, &[changed]).collect();
            assert_eq!(
                rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
                [DELETE, INSERT]
            );
            assert_eq!(
                rows[0].params(),
                &[SqlParam::U64(guid), SqlParam::Bool(false)]
            );
            assert_eq!(rows[1].params()[0], SqlParam::U64(guid));
            assert_eq!(rows[1].params()[1], SqlParam::U32(spell));
        }
    }
}

#[test]
fn stored_position_binds_actual_spell_u32_map_and_four_raw_f32_values_without_timestamps() {
    let data = NewStoredAuraPosition {
        map: u32::MAX,
        position: [1.25, -2.5, 3.75, -4.0],
    };
    let row = insert_statement(u64::MAX, u32::MAX, &data);
    assert_eq!(
        row.sql(),
        "INSERT INTO character_aura_stored_location (Guid, Spell, MapId, PositionX, PositionY, PositionZ, Orientation) VALUES (?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(row.sql().matches('?').count(), 7);
    assert_eq!(
        row.params(),
        &[
            SqlParam::U64(u64::MAX),
            SqlParam::U32(u32::MAX),
            SqlParam::U32(u32::MAX),
            SqlParam::F32(1.25),
            SqlParam::F32(-2.5),
            SqlParam::F32(3.75),
            SqlParam::F32(-4.0),
        ]
    );
}

#[test]
fn stored_position_retains_nan_payload_infinities_and_negative_zero_bits() {
    let bits = [0x7FC0_1234, 0x7F80_0000, 0xFF80_0000, 0x8000_0000];
    let data = NewStoredAuraPosition {
        map: 42,
        position: bits.map(f32::from_bits),
    };
    let row = insert_statement(17, 90, &data);
    for (parameter, expected) in row.params()[3..7].iter().zip(bits) {
        match parameter {
            SqlParam::F32(value) => assert_eq!(value.to_bits(), expected),
            other => panic!("expected raw f32 parameter, got {other:?}"),
        }
    }
}

#[test]
fn unordered_projection_order_and_guid_scope_preserve_frozen_snapshot_states() {
    let locations = vec![
        NewStoredAuraLocation {
            spell: 90,
            state: NewStoredAuraLocationState::Changed(position()),
        },
        NewStoredAuraLocation {
            spell: 2,
            state: NewStoredAuraLocationState::Unchanged,
        },
        NewStoredAuraLocation {
            spell: 7,
            state: NewStoredAuraLocationState::Deleted,
        },
        NewStoredAuraLocation {
            spell: 1,
            state: NewStoredAuraLocationState::Changed(position()),
        },
    ];
    let rows: Vec<_> = statements(31, &locations).collect();
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [DELETE, INSERT, DELETE, DELETE, INSERT]
    );
    assert_eq!(rows[1].params()[1], SqlParam::U32(90));
    assert_eq!(rows[4].params()[1], SqlParam::U32(1));
    for index in [0, 2, 3] {
        assert_eq!(
            rows[index].params(),
            &[SqlParam::U64(31), SqlParam::Bool(false)]
        );
    }
    for row in &rows {
        assert_eq!(row.params()[0], SqlParam::U64(31));
    }
    let mut transaction = SqlTransaction::new();
    append(&mut transaction, 31, &locations);
    assert_eq!(
        transaction.sqls_for_test(),
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>()
    );
    // Snapshot-only assertions: canonical Deleted-entry erasure is not applied.
    assert_eq!(locations.len(), 4);
    assert!(matches!(
        &locations[0].state,
        NewStoredAuraLocationState::Changed(_)
    ));
    assert!(matches!(
        &locations[1].state,
        NewStoredAuraLocationState::Unchanged
    ));
    assert!(matches!(
        &locations[2].state,
        NewStoredAuraLocationState::Deleted
    ));
    assert!(matches!(
        &locations[3].state,
        NewStoredAuraLocationState::Changed(_)
    ));
}

#[test]
fn production_builder_orders_skills_then_stored_locations_then_equipment_cache() {
    let mut character = super::super::tests::character();
    character.skills = vec![NewSkill {
        skill: 1,
        value: 1,
        maximum: 2,
        profession_slot: -1,
    }];
    character.stored_aura_locations = vec![
        NewStoredAuraLocation {
            spell: 9,
            state: NewStoredAuraLocationState::Changed(position()),
        },
        NewStoredAuraLocation {
            spell: 7,
            state: NewStoredAuraLocationState::Deleted,
        },
    ];
    let mut transaction = SqlTransaction::new();
    super::super::append_character_save(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let cache = super::super::visual_statement(character.guid, &character.visual_items);
    let start = sqls
        .iter()
        .position(|sql| sql.starts_with("INSERT INTO character_skills ("))
        .unwrap();
    assert_eq!(
        &sqls[start..],
        &[
            "INSERT INTO character_skills (guid, skill, value, max, professionSlot) VALUES (?,?,?,?,?)",
            "DELETE FROM character_aura_stored_location WHERE Guid = ? AND Spell = ?",
            "INSERT INTO character_aura_stored_location (Guid, Spell, MapId, PositionX, PositionY, PositionZ, Orientation) VALUES (?, ?, ?, ?, ?, ?, ?)",
            "DELETE FROM character_aura_stored_location WHERE Guid = ? AND Spell = ?",
            "DELETE FROM character_queststatus_objectives_criteria WHERE guid = ?",
            "DELETE FROM character_select_screen_equipment_cache WHERE guid = ?",
            cache.sql(),
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_bank_tab_settings WHERE characterGuid = ?",
        ]
    );
}
