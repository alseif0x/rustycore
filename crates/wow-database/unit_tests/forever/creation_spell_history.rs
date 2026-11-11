use super::*;
use crate::SqlParam;
use wow_persistence::forever::creation::save::{
    NewAction, NewSpell, NewSpellChargeCategory, NewTraitChange, NewTraitState,
};

fn history() -> NewSpellHistory {
    super::super::tests::character().spell_history
}

#[test]
fn empty_spell_history_always_deletes_cooldowns_then_charges_with_guid_scope() {
    for guid in [17, u64::MAX] {
        let history = history();
        let rows = statements(guid, &history);
        assert_eq!(
            rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
            [
                "DELETE FROM character_spell_cooldown WHERE guid = ?",
                "DELETE FROM character_spell_charges WHERE guid = ?",
            ]
        );
        for row in &rows {
            assert_eq!(row.params(), &[SqlParam::U64(guid)]);
        }
        let mut transaction = crate::forever::create::tests::isolated_transaction();
        append(&mut transaction, guid, &history);
        assert_eq!(
            transaction.sqls_for_test(),
            rows.iter().map(|row| row.sql()).collect::<Vec<_>>()
        );
    }
}

#[test]
fn cooldowns_skip_only_on_hold_and_preserve_unsorted_unique_spell_order_and_past_seconds() {
    let mut history = history();
    history.cooldowns = vec![
        NewSpellCooldown {
            spell: 90,
            item: 3,
            end_time: -23,
            category: 5,
            category_end: -24,
            on_hold: false,
        },
        NewSpellCooldown {
            spell: 2,
            item: 4,
            end_time: i64::MAX,
            category: 6,
            category_end: i64::MAX,
            on_hold: true,
        },
        NewSpellCooldown {
            spell: 7,
            item: 8,
            end_time: 0,
            category: 2,
            category_end: -1,
            on_hold: false,
        },
    ];
    let rows = statements(29, &history);
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [
            COOLDOWN_DELETE,
            "INSERT INTO character_spell_cooldown (guid, spell, item, time, categoryId, categoryEnd) VALUES (?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_spell_cooldown (guid, spell, item, time, categoryId, categoryEnd) VALUES (?, ?, ?, ?, ?, ?)",
            CHARGES_DELETE,
        ]
    );
    assert_eq!(
        rows[1].params(),
        &[
            SqlParam::U64(29),
            SqlParam::U32(90),
            SqlParam::U32(3),
            SqlParam::I64(-23),
            SqlParam::U32(5),
            SqlParam::I64(-24),
        ]
    );
    assert_eq!(
        rows[2].params(),
        &[
            SqlParam::U64(29),
            SqlParam::U32(7),
            SqlParam::U32(8),
            SqlParam::I64(0),
            SqlParam::U32(2),
            SqlParam::I64(-1),
        ]
    );
}

#[test]
fn cooldown_bindings_preserve_u32_fields_and_already_projected_i64_epoch_seconds() {
    // These bounds test DTO/SQL binding, not execution of C++ Clock::to_time_t.
    for (end_time, category_end) in [
        (i64::MIN, i64::MAX),
        (i64::MAX, i64::MIN),
        (1_700_000_123, 1_700_000_999),
    ] {
        let cooldown = NewSpellCooldown {
            spell: u32::MAX,
            item: u32::MAX,
            end_time,
            category: u32::MAX,
            category_end,
            on_hold: false,
        };
        let row = cooldown_statement(u64::MAX, &cooldown);
        assert_eq!(row.sql().matches('?').count(), 6);
        assert_eq!(
            row.params(),
            &[
                SqlParam::U64(u64::MAX),
                SqlParam::U32(u32::MAX),
                SqlParam::U32(u32::MAX),
                SqlParam::I64(end_time),
                SqlParam::U32(u32::MAX),
                SqlParam::I64(category_end),
            ]
        );
    }
}

#[test]
fn charge_categories_keep_supplied_order_and_deque_order_without_rows_for_empty_groups() {
    let mut history = history();
    history.charges = vec![
        NewSpellChargeCategory {
            category: u32::MAX,
            charges: vec![
                NewSpellCharge {
                    start_time: -40,
                    end_time: -30,
                },
                NewSpellCharge {
                    start_time: -20,
                    end_time: -10,
                },
            ],
        },
        NewSpellChargeCategory {
            category: 2,
            charges: Vec::new(),
        },
        NewSpellChargeCategory {
            category: 7,
            charges: vec![NewSpellCharge {
                start_time: 0,
                end_time: 10,
            }],
        },
    ];
    let rows = statements(u64::MAX, &history);
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [
            COOLDOWN_DELETE,
            CHARGES_DELETE,
            "INSERT INTO character_spell_charges (guid, categoryId, rechargeStart, rechargeEnd) VALUES (?, ?, ?, ?)",
            "INSERT INTO character_spell_charges (guid, categoryId, rechargeStart, rechargeEnd) VALUES (?, ?, ?, ?)",
            "INSERT INTO character_spell_charges (guid, categoryId, rechargeStart, rechargeEnd) VALUES (?, ?, ?, ?)",
        ]
    );
    for (row, (category, start, end)) in
        rows[2..]
            .iter()
            .zip([(u32::MAX, -40, -30), (u32::MAX, -20, -10), (7, 0, 10)])
    {
        assert_eq!(
            row.params(),
            &[
                SqlParam::U64(u64::MAX),
                SqlParam::U32(category),
                SqlParam::I64(start),
                SqlParam::I64(end),
            ]
        );
    }
    // Binding limits alone do not establish that canonical Clock projection exists.
    for (start_time, end_time) in [(i64::MIN, i64::MAX), (i64::MAX, i64::MIN)] {
        let row = charge_statement(
            31,
            u32::MAX,
            &NewSpellCharge {
                start_time,
                end_time,
            },
        );
        assert_eq!(
            row.params(),
            &[
                SqlParam::U64(31),
                SqlParam::U32(u32::MAX),
                SqlParam::I64(start_time),
                SqlParam::I64(end_time),
            ]
        );
    }
}

#[test]
fn production_builder_places_history_after_traits_and_spells_before_actions() {
    let mut character = super::super::tests::character();
    character.traits = vec![NewTraitChange {
        key: -5,
        state: NewTraitState::Removed,
    }];
    character.spells = vec![NewSpell {
        spell: 42,
        active: true,
        disabled: false,
        dependent: false,
        favorite: true,
    }];
    character.spell_history.cooldowns = vec![NewSpellCooldown {
        spell: 17,
        item: 18,
        end_time: -19,
        category: 20,
        category_end: -21,
        on_hold: false,
    }];
    character.spell_history.charges = vec![NewSpellChargeCategory {
        category: 7,
        charges: vec![NewSpellCharge {
            start_time: -2,
            end_time: -1,
        }],
    }];
    character.actions = vec![NewAction {
        spec: 4,
        trait_config: -5,
        button: 1,
        action: 42,
        kind: 0,
    }];
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    super::super::append_creation_through_actions(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let start = sqls
        .iter()
        .position(|sql| {
            *sql == "DELETE FROM character_trait_entry WHERE guid = ? AND traitConfigId = ?"
        })
        .unwrap();
    assert_eq!(
        &sqls[start..],
        &[
            "DELETE FROM character_trait_entry WHERE guid = ? AND traitConfigId = ?",
            "DELETE FROM character_trait_config WHERE guid = ? AND traitConfigId = ?",
            "DELETE FROM character_action WHERE guid = ? AND traitConfigId = ?",
            "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?)",
            "DELETE FROM character_spell_favorite WHERE guid = ? AND spell = ?",
            "INSERT INTO character_spell_favorite (guid, spell) VALUES (?, ?)",
            "DELETE FROM character_spell_cooldown WHERE guid = ?",
            "INSERT INTO character_spell_cooldown (guid, spell, item, time, categoryId, categoryEnd) VALUES (?, ?, ?, ?, ?, ?)",
            "DELETE FROM character_spell_charges WHERE guid = ?",
            "INSERT INTO character_spell_charges (guid, categoryId, rechargeStart, rechargeEnd) VALUES (?, ?, ?, ?)",
            "INSERT INTO character_action (guid, spec, traitConfigId, button, action, type) VALUES (?,?,?,?,?,?)",
        ]
    );
}
