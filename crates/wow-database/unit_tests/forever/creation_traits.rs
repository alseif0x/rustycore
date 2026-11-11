use super::*;
use crate::SqlParam;
use wow_persistence::forever::creation::save::NewSpell;

fn config(kind: i32) -> NewTraitConfig {
    NewTraitConfig {
        id: i32::MIN,
        kind,
        specialization: i32::MIN,
        combat_flags: i32::MAX,
        local_identifier: -17,
        skill_line: i32::MIN,
        system: i32::MAX,
        variation: i32::MIN,
        name: "构筑🌙 Élan".into(),
        entries: Vec::new(),
    }
}

#[test]
fn all_five_trait_states_preserve_source_noops_changed_missing_and_removed_delete_order() {
    for state in [
        NewTraitState::Unchanged,
        NewTraitState::New,
        NewTraitState::Temporary,
    ] {
        assert!(change_statements(17, &NewTraitChange { key: -9, state }).is_empty());
    }
    for (state, expected) in [
        (
            NewTraitState::Changed(None),
            vec![
                "DELETE FROM character_trait_entry WHERE guid = ? AND traitConfigId = ?",
                "DELETE FROM character_trait_config WHERE guid = ? AND traitConfigId = ?",
            ],
        ),
        (
            NewTraitState::Removed,
            vec![
                "DELETE FROM character_trait_entry WHERE guid = ? AND traitConfigId = ?",
                "DELETE FROM character_trait_config WHERE guid = ? AND traitConfigId = ?",
                "DELETE FROM character_action WHERE guid = ? AND traitConfigId = ?",
            ],
        ),
    ] {
        let rows = change_statements(
            u64::MAX,
            &NewTraitChange {
                key: i32::MIN,
                state,
            },
        );
        assert_eq!(
            rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
            expected
        );
        for row in rows {
            assert_eq!(
                row.params(),
                &[SqlParam::U64(u64::MAX), SqlParam::I32(i32::MIN)]
            );
        }
    }
}

#[test]
fn known_trait_kinds_preserve_signed_fields_exact_null_patterns_and_unicode_name() {
    for (kind, expected) in [
        (
            1,
            [
                SqlParam::I32(i32::MIN),
                SqlParam::I32(i32::MAX),
                SqlParam::I32(-17),
                SqlParam::Null,
                SqlParam::Null,
                SqlParam::Null,
            ],
        ),
        (
            2,
            [
                SqlParam::Null,
                SqlParam::Null,
                SqlParam::Null,
                SqlParam::I32(i32::MIN),
                SqlParam::Null,
                SqlParam::Null,
            ],
        ),
        (
            3,
            [
                SqlParam::Null,
                SqlParam::Null,
                SqlParam::Null,
                SqlParam::Null,
                SqlParam::I32(i32::MAX),
                SqlParam::I32(i32::MIN),
            ],
        ),
    ] {
        let row = config_statement(u64::MAX, &config(kind));
        assert_eq!(
            row.sql(),
            "INSERT INTO character_trait_config (guid, traitConfigId, type, chrSpecializationId, combatConfigFlags, localIdentifier, skillLineId, traitSystemId, variationId, `name`) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        );
        assert_eq!(row.params().len(), 10);
        assert_eq!(
            &row.params()[..3],
            &[
                SqlParam::U64(u64::MAX),
                SqlParam::I32(i32::MIN),
                SqlParam::I32(kind),
            ]
        );
        assert_eq!(&row.params()[3..9], &expected);
        assert_eq!(row.params()[9], SqlParam::String("构筑🌙 Élan".into()));
    }
}

#[test]
fn invalid_and_unknown_trait_kinds_leave_all_six_slots_bool_false_not_null() {
    for kind in [0, -1, i32::MIN, 4, i32::MAX] {
        let row = config_statement(29, &config(kind));
        assert_eq!(row.params()[2], SqlParam::I32(kind));
        assert_eq!(
            &row.params()[3..9],
            &[
                SqlParam::Bool(false),
                SqlParam::Bool(false),
                SqlParam::Bool(false),
                SqlParam::Bool(false),
                SqlParam::Bool(false),
                SqlParam::Bool(false),
            ]
        );
        assert_eq!(row.params()[9], SqlParam::String("构筑🌙 Élan".into()));
    }
}

#[test]
fn changed_uses_dirty_key_for_deletes_and_own_config_id_for_ordered_signed_entries() {
    let mut config = config(1);
    config.entries = vec![
        NewTraitEntry {
            node: 7,
            entry: i32::MIN,
            rank: i32::MAX,
        },
        NewTraitEntry {
            node: -9,
            entry: i32::MAX,
            rank: i32::MIN,
        },
        NewTraitEntry {
            node: 0,
            entry: -1,
            rank: -2,
        },
    ];
    let rows = change_statements(
        u64::MAX,
        &NewTraitChange {
            key: i32::MAX,
            state: NewTraitState::Changed(Some(config)),
        },
    );
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [
            ENTRIES_DELETE,
            CONFIG_DELETE,
            CONFIG_INSERT,
            "INSERT INTO character_trait_entry (guid, traitConfigId, traitNodeId, traitNodeEntryId, `rank`) VALUES (?, ?, ?, ?, ?)",
            "INSERT INTO character_trait_entry (guid, traitConfigId, traitNodeId, traitNodeEntryId, `rank`) VALUES (?, ?, ?, ?, ?)",
            "INSERT INTO character_trait_entry (guid, traitConfigId, traitNodeId, traitNodeEntryId, `rank`) VALUES (?, ?, ?, ?, ?)",
        ]
    );
    for row in &rows[..2] {
        assert_eq!(
            row.params(),
            &[SqlParam::U64(u64::MAX), SqlParam::I32(i32::MAX)]
        );
    }
    assert_eq!(
        &rows[2].params()[..2],
        &[SqlParam::U64(u64::MAX), SqlParam::I32(i32::MIN)]
    );
    for (row, (node, entry, rank)) in rows[3..].iter().zip([
        (7, i32::MIN, i32::MAX),
        (-9, i32::MAX, i32::MIN),
        (0, -1, -2),
    ]) {
        assert_eq!(
            row.params(),
            &[
                SqlParam::U64(u64::MAX),
                SqlParam::I32(i32::MIN),
                SqlParam::I32(node),
                SqlParam::I32(entry),
                SqlParam::I32(rank),
            ]
        );
    }
}

#[test]
fn trait_change_vector_order_is_retained_without_sorting_dirty_keys_or_config_ids() {
    let mut config = config(2);
    config.id = 100;
    config.entries = vec![NewTraitEntry {
        node: 1,
        entry: 2,
        rank: 3,
    }];
    let changes = vec![
        NewTraitChange {
            key: 90,
            state: NewTraitState::Changed(None),
        },
        NewTraitChange {
            key: -10,
            state: NewTraitState::Removed,
        },
        NewTraitChange {
            key: 30,
            state: NewTraitState::Changed(Some(config)),
        },
    ];
    let rows: Vec<_> = statements(31, &changes).collect();
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [
            ENTRIES_DELETE,
            CONFIG_DELETE,
            ENTRIES_DELETE,
            CONFIG_DELETE,
            ACTIONS_DELETE,
            ENTRIES_DELETE,
            CONFIG_DELETE,
            CONFIG_INSERT,
            ENTRY_INSERT,
        ]
    );
    for (row, id) in rows.iter().zip([90, 90, -10, -10, -10, 30, 30, 100, 100]) {
        assert_eq!(&row.params()[..2], &[SqlParam::U64(31), SqlParam::I32(id)]);
    }
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    append(&mut transaction, 31, &changes);
    assert_eq!(
        transaction.sqls_for_test(),
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>()
    );
}

#[test]
fn production_prefix_places_trait_changes_after_all_pvp_rows_and_before_spells() {
    let mut character = super::super::tests::character();
    let mut config = config(3);
    config.entries = vec![NewTraitEntry {
        node: -1,
        entry: -2,
        rank: -3,
    }];
    character.traits = vec![
        NewTraitChange {
            key: 5,
            state: NewTraitState::Changed(Some(config)),
        },
        NewTraitChange {
            key: -9,
            state: NewTraitState::Removed,
        },
    ];
    character.spells = vec![NewSpell {
        spell: 42,
        active: true,
        disabled: false,
        dependent: false,
        favorite: false,
    }];
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    super::super::append_creation_prefix(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let start = sqls
        .iter()
        .position(|sql| *sql == "DELETE FROM character_pvp_talent WHERE guid = ?")
        .unwrap();
    assert_eq!(
        &sqls[start..],
        &[
            "DELETE FROM character_pvp_talent WHERE guid = ?",
            "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            ENTRIES_DELETE,
            CONFIG_DELETE,
            CONFIG_INSERT,
            ENTRY_INSERT,
            ENTRIES_DELETE,
            CONFIG_DELETE,
            ACTIONS_DELETE,
            "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?)",
            "DELETE FROM character_spell_favorite WHERE guid = ? AND spell = ?",
            "DELETE FROM character_spell_cooldown WHERE guid = ?",
            "DELETE FROM character_spell_charges WHERE guid = ?",
        ]
    );
}
