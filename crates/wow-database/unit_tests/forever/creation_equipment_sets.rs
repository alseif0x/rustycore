use super::*;
use crate::SqlParam;
use wow_persistence::forever::creation::save::NewCompletedAchievement;

fn saved_set(transmog: bool, state: NewSavedSetState) -> NewSavedEquipmentSet {
    NewSavedEquipmentSet {
        guid: 777,
        set_index: 3,
        name: "Élan 装備".into(),
        icon: "图标🌙".into(),
        ignore_mask: 5,
        kind: if transmog {
            NewSavedSetKind::Transmog {
                appearances: std::array::from_fn(|index| 1000 + index as i32),
                enchants: [7, 8],
            }
        } else {
            NewSavedSetKind::Equipment {
                assigned_spec: Some(2),
                items: std::array::from_fn(|index| 100 + index as u64),
            }
        },
        state,
    }
}

fn expected_sql(transmog: bool, new: bool) -> String {
    let table = if transmog {
        "character_transmog_outfits"
    } else {
        "character_equipmentsets"
    };
    let mut payload = vec![
        "name".to_owned(),
        "iconname".to_owned(),
        "ignore_mask".to_owned(),
    ];
    if transmog {
        payload.extend((0..19).map(|index| format!("appearance{index}")));
        payload.extend(["mainHandEnchant".to_owned(), "offHandEnchant".to_owned()]);
    } else {
        payload.push("AssignedSpecIndex".to_owned());
        payload.extend((0..19).map(|index| format!("item{index}")));
    }
    if new {
        let mut columns = vec![
            "guid".to_owned(),
            "setguid".to_owned(),
            "setindex".to_owned(),
        ];
        columns.extend(payload);
        let values = vec!["?"; columns.len()].join(", ");
        format!(
            "INSERT INTO {table} ({}) VALUES ({values})",
            columns.join(", ")
        )
    } else {
        let fields = payload
            .iter()
            .map(|column| format!("{column}=?"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("UPDATE {table} SET {fields} WHERE guid=? AND setguid=? AND setindex=?")
    }
}

#[test]
fn all_four_states_and_both_kinds_use_exact_source_sql_and_parameter_counts() {
    for transmog in [false, true] {
        for state in [
            NewSavedSetState::Unchanged,
            NewSavedSetState::Changed,
            NewSavedSetState::New,
            NewSavedSetState::Deleted,
        ] {
            let set = saved_set(transmog, state);
            let row = set_statement(29, &set);
            match state {
                NewSavedSetState::Unchanged => assert!(row.is_none()),
                NewSavedSetState::Deleted => {
                    let row = row.unwrap();
                    assert_eq!(
                        row.sql(),
                        if transmog {
                            "DELETE FROM character_transmog_outfits WHERE setguid=?"
                        } else {
                            "DELETE FROM character_equipmentsets WHERE setguid=?"
                        }
                    );
                    assert_eq!(row.params(), &[SqlParam::U64(777)]);
                }
                NewSavedSetState::Changed | NewSavedSetState::New => {
                    let row = row.unwrap();
                    assert_eq!(
                        row.sql(),
                        expected_sql(transmog, matches!(state, NewSavedSetState::New))
                    );
                    let count = if transmog { 27 } else { 26 };
                    assert_eq!(row.params().len(), count);
                    assert_eq!(row.sql().matches('?').count(), count);
                }
            }
        }
    }
}

#[test]
fn equipment_new_and_changed_bind_all_nineteen_counters_and_private_strings_in_source_order() {
    for state in [NewSavedSetState::New, NewSavedSetState::Changed] {
        let set = saved_set(false, state);
        let new = matches!(state, NewSavedSetState::New);
        let row = set_statement(29, &set).unwrap();
        let mut expected = if new {
            vec![
                SqlParam::U64(29),
                SqlParam::U64(777),
                SqlParam::U32(3),
                SqlParam::String("Élan 装備".into()),
                SqlParam::String("图标🌙".into()),
                SqlParam::U32(5),
                SqlParam::I32(2),
            ]
        } else {
            vec![
                SqlParam::String("Élan 装備".into()),
                SqlParam::String("图标🌙".into()),
                SqlParam::U32(5),
                SqlParam::I32(2),
            ]
        };
        expected.extend((100..119).map(SqlParam::U64));
        if !new {
            expected.extend([SqlParam::U64(29), SqlParam::U64(777), SqlParam::U32(3)]);
        }
        assert_eq!(row.params(), expected);
    }
}

#[test]
fn legacy_transmog_new_and_changed_bind_all_nineteen_appearances_then_both_enchants() {
    for state in [NewSavedSetState::New, NewSavedSetState::Changed] {
        let set = saved_set(true, state);
        let new = matches!(state, NewSavedSetState::New);
        let row = set_statement(31, &set).unwrap();
        let mut expected = if new {
            vec![
                SqlParam::U64(31),
                SqlParam::U64(777),
                SqlParam::U32(3),
                SqlParam::String("Élan 装備".into()),
                SqlParam::String("图标🌙".into()),
                SqlParam::U32(5),
            ]
        } else {
            vec![
                SqlParam::String("Élan 装備".into()),
                SqlParam::String("图标🌙".into()),
                SqlParam::U32(5),
            ]
        };
        expected.extend((1000..1019).map(SqlParam::I32));
        expected.extend([SqlParam::I32(7), SqlParam::I32(8)]);
        if !new {
            expected.extend([SqlParam::U64(31), SqlParam::U64(777), SqlParam::U32(3)]);
        }
        assert_eq!(row.params(), expected);
    }
}

#[test]
fn encoding_bounds_keep_u32_index_u64_counters_signed_spec_and_transmog_values() {
    // Binding ranges only: u32::MAX setindex is not schema/canonical admission proof.
    for state in [NewSavedSetState::New, NewSavedSetState::Changed] {
        let new = matches!(state, NewSavedSetState::New);
        for assigned_spec in [None, Some(i32::MIN), Some(i32::MAX)] {
            let mut set = saved_set(false, state);
            set.guid = u64::MAX - 1;
            set.set_index = u32::MAX;
            set.ignore_mask = u32::MAX;
            set.kind = NewSavedSetKind::Equipment {
                assigned_spec,
                items: [u64::MAX; 19],
            };
            let row = set_statement(u64::MAX, &set).unwrap();
            let owner = if new { 0 } else { 23 };
            assert_eq!(
                &row.params()[owner..owner + 3],
                &[
                    SqlParam::U64(u64::MAX),
                    SqlParam::U64(u64::MAX - 1),
                    SqlParam::U32(u32::MAX),
                ]
            );
            assert_eq!(
                row.params()[if new { 5 } else { 2 }],
                SqlParam::U32(u32::MAX)
            );
            assert_eq!(
                row.params()[if new { 6 } else { 3 }],
                SqlParam::I32(assigned_spec.unwrap_or(-1))
            );
            let item_start = if new { 7 } else { 4 };
            assert!(
                row.params()[item_start..item_start + 19]
                    .iter()
                    .all(|value| *value == SqlParam::U64(u64::MAX))
            );
        }
        let mut set = saved_set(true, state);
        set.guid = u64::MAX - 1;
        set.set_index = u32::MAX;
        set.kind = NewSavedSetKind::Transmog {
            appearances: std::array::from_fn(
                |index| if index % 2 == 0 { i32::MIN } else { i32::MAX },
            ),
            enchants: [i32::MIN, i32::MAX],
        };
        let row = set_statement(u64::MAX, &set).unwrap();
        let owner = if new { 0 } else { 24 };
        assert_eq!(
            &row.params()[owner..owner + 3],
            &[
                SqlParam::U64(u64::MAX),
                SqlParam::U64(u64::MAX - 1),
                SqlParam::U32(u32::MAX),
            ]
        );
        let start = if new { 6 } else { 3 };
        for index in 0..19 {
            assert_eq!(
                row.params()[start + index],
                SqlParam::I32(if index % 2 == 0 { i32::MIN } else { i32::MAX })
            );
        }
        assert_eq!(
            &row.params()[start + 19..start + 21],
            &[SqlParam::I32(i32::MIN), SqlParam::I32(i32::MAX)]
        );
    }
}

#[test]
fn ordered_map_keys_control_iteration_but_value_guids_control_all_sql_identity() {
    let mut sets = BTreeMap::new();
    for (key, guid, transmog, state) in [
        (90, 11, false, NewSavedSetState::Deleted),
        (2, 900, true, NewSavedSetState::New),
        (7, 700, false, NewSavedSetState::Changed),
        (1, 111, true, NewSavedSetState::Unchanged),
    ] {
        let mut set = saved_set(transmog, state);
        set.guid = guid;
        sets.insert(key, set);
    }
    let rows: Vec<_> = statements(29, &sets).collect();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].sql(), expected_sql(true, true));
    assert_eq!(
        &rows[0].params()[..3],
        &[SqlParam::U64(29), SqlParam::U64(900), SqlParam::U32(3)]
    );
    assert_eq!(rows[1].sql(), expected_sql(false, false));
    assert_eq!(
        &rows[1].params()[23..],
        &[SqlParam::U64(29), SqlParam::U64(700), SqlParam::U32(3)]
    );
    assert_eq!(
        rows[2].sql(),
        "DELETE FROM character_equipmentsets WHERE setguid=?"
    );
    assert_eq!(rows[2].params(), &[SqlParam::U64(11)]);
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    append(&mut transaction, 29, &sets);
    assert_eq!(
        transaction.sqls_for_test(),
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>()
    );
    // Frozen snapshot only; canonical transition/erasure remains unimplemented.
    assert!(matches!(sets[&2].state, NewSavedSetState::New));
    assert!(matches!(sets[&7].state, NewSavedSetState::Changed));
    assert!(matches!(sets[&90].state, NewSavedSetState::Deleted));
}

#[test]
fn existing_through_cache_orders_progress_then_saved_sets_then_visual_cache() {
    let mut character = super::super::tests::character();
    character.achievements.completed = vec![NewCompletedAchievement {
        achievement: 17,
        changed: true,
        date: 1,
    }];
    character
        .equipment_sets
        .insert(2, saved_set(false, NewSavedSetState::New));
    let mut deleted = saved_set(true, NewSavedSetState::Deleted);
    deleted.guid = 888;
    character.equipment_sets.insert(7, deleted);
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    super::super::append_character_save(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let start = sqls
        .iter()
        .position(|sql| {
            *sql == "DELETE FROM character_achievement WHERE achievement = ? AND guid = ?"
        })
        .unwrap();
    let equipment = expected_sql(false, true);
    let cache = super::super::visual_statement(character.guid, &character.visual_items);
    assert_eq!(
        &sqls[start..],
        &[
            "DELETE FROM character_achievement WHERE achievement = ? AND guid = ?",
            "INSERT INTO character_achievement (guid, achievement, date) VALUES (?, ?, ?)",
            "DELETE FROM character_queststatus_objectives_criteria WHERE guid = ?",
            equipment.as_str(),
            "DELETE FROM character_transmog_outfits WHERE setguid=?",
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
