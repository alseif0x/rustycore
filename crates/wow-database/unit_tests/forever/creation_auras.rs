use super::*;
use crate::SqlParam;
use wow_persistence::forever::creation::save::{NewAction, NewSkill};

fn aura(spell: u32) -> NewAuraSave {
    NewAuraSave {
        saveable: true,
        caster_guid: std::array::from_fn(|index| index as u8 + 1),
        item_guid: std::array::from_fn(|index| 0xA0 + index as u8),
        spell,
        effect_mask: 1,
        recalculate_mask: 0,
        difficulty: 0,
        stack_count: 1,
        max_duration: 100,
        remaining_time: 50,
        charges: 0,
        cast_item: 17,
        cast_item_level: 1,
        effects: vec![NewAuraEffect {
            index: 0,
            amount: 1.0,
            base_amount: 2.0,
        }],
    }
}

#[test]
fn empty_aura_projection_deletes_effects_before_parents_with_reserved_guid_scope() {
    for guid in [17, u64::MAX] {
        let rows = statements(guid, &[]);
        assert_eq!(
            rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
            [
                "DELETE FROM character_aura_effect WHERE guid = ?",
                "DELETE FROM character_aura WHERE guid = ?",
            ]
        );
        for row in &rows {
            assert_eq!(row.params(), &[SqlParam::U64(guid)]);
        }
        let mut transaction = SqlTransaction::new();
        append(&mut transaction, guid, &[]);
        assert_eq!(
            transaction.sqls_for_test(),
            rows.iter().map(|row| row.sql()).collect::<Vec<_>>()
        );
    }
}

#[test]
fn frozen_saveability_flags_skip_parent_and_effect_rows_without_skipping_deletes() {
    // This checks consumption of supplied flags, not Aura::CanBeSaved policy.
    for flags in 0..8 {
        let mut auras = vec![aura(100), aura(200), aura(300)];
        let mut expected_spells = Vec::new();
        for (index, aura) in auras.iter_mut().enumerate() {
            aura.saveable = flags & (1 << index) != 0;
            if aura.saveable {
                expected_spells.push(aura.spell);
            }
        }
        let rows = statements(29, &auras);
        assert_eq!(rows.len(), 2 + 2 * expected_spells.len());
        assert_eq!(rows[0].sql(), EFFECTS_DELETE);
        assert_eq!(rows[1].sql(), AURAS_DELETE);
        for (pair, spell) in rows[2..].chunks_exact(2).zip(expected_spells) {
            assert_eq!(pair[0].sql(), AURA_INSERT);
            assert_eq!(pair[1].sql(), EFFECT_INSERT);
            assert_eq!(pair[0].params()[3], SqlParam::U32(spell));
            assert_eq!(pair[1].params()[3], SqlParam::U32(spell));
        }
    }
}

#[test]
fn aura_and_effect_binding_bounds_preserve_all_fields_and_both_complete_guid_arrays() {
    // Encoder bounds only; these are not canonical eligibility/key fixtures.
    let mut aura = aura(u32::MAX);
    aura.effect_mask = u32::MAX;
    aura.recalculate_mask = 0x8000_0001;
    aura.difficulty = u8::MAX;
    aura.stack_count = u8::MAX;
    aura.max_duration = i32::MIN;
    aura.remaining_time = i32::MAX;
    aura.charges = u8::MAX;
    aura.cast_item = u32::MAX;
    aura.cast_item_level = i32::MIN;
    let guid = 0x1234_5678_9ABC_DEF0;
    let parent = aura_statement(guid, &aura);
    assert_eq!(
        parent.sql(),
        "INSERT INTO character_aura (guid, casterGuid, itemGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges, castItemId, castItemLevel) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        parent.params(),
        &[
            SqlParam::U64(guid),
            SqlParam::Bytes((1..=16).collect()),
            SqlParam::Bytes((0xA0..=0xAF).collect()),
            SqlParam::U32(u32::MAX),
            SqlParam::U32(u32::MAX),
            SqlParam::U32(0x8000_0001),
            SqlParam::U8(u8::MAX),
            SqlParam::U8(u8::MAX),
            SqlParam::I32(i32::MIN),
            SqlParam::I32(i32::MAX),
            SqlParam::U8(u8::MAX),
            SqlParam::U32(u32::MAX),
            SqlParam::I32(i32::MIN),
        ]
    );
    let effect = effect_statement(
        guid,
        &aura,
        &NewAuraEffect {
            index: u8::MAX,
            amount: f64::MAX,
            base_amount: f64::MIN,
        },
    );
    assert_eq!(
        effect.sql(),
        "INSERT INTO character_aura_effect (guid, casterGuid, itemGuid, spell, effectMask, effectIndex, amount, baseAmount) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        effect.params(),
        &[
            SqlParam::U64(guid),
            SqlParam::Bytes((1..=16).collect()),
            SqlParam::Bytes((0xA0..=0xAF).collect()),
            SqlParam::U32(u32::MAX),
            SqlParam::U32(u32::MAX),
            SqlParam::U8(u8::MAX),
            SqlParam::F64(f64::MAX),
            SqlParam::F64(f64::MIN),
        ]
    );
}

#[test]
fn aura_map_spell_and_equal_key_insertion_order_keep_each_parent_before_its_effects() {
    let mut first = aura(100);
    first.effect_mask = 0b0101;
    first.effects = vec![
        NewAuraEffect {
            index: 0,
            amount: 1.0,
            base_amount: 2.0,
        },
        NewAuraEffect {
            index: 2,
            amount: 3.0,
            base_amount: 4.0,
        },
    ];
    let mut second = aura(100);
    second.item_guid[15] = 0x2F;
    second.effect_mask = 0b0010;
    second.effects = vec![NewAuraEffect {
        index: 1,
        amount: 5.0,
        base_amount: 6.0,
    }];
    let mut third = aura(200);
    third.effect_mask = 0b1000;
    third.effects = vec![NewAuraEffect {
        index: 3,
        amount: 7.0,
        base_amount: 8.0,
    }];
    let rows = statements(31, &[first, second, third]);
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [
            EFFECTS_DELETE,
            AURAS_DELETE,
            AURA_INSERT,
            EFFECT_INSERT,
            EFFECT_INSERT,
            AURA_INSERT,
            EFFECT_INSERT,
            AURA_INSERT,
            EFFECT_INSERT,
        ]
    );
    for (row, (spell, mask, last_item_byte)) in rows[2..].iter().zip([
        (100, 5, 0xAF),
        (100, 5, 0xAF),
        (100, 5, 0xAF),
        (100, 2, 0x2F),
        (100, 2, 0x2F),
        (200, 8, 0xAF),
        (200, 8, 0xAF),
    ]) {
        let mut item: Vec<u8> = (0xA0..=0xAF).collect();
        item[15] = last_item_byte;
        assert_eq!(row.params()[0], SqlParam::U64(31));
        assert_eq!(row.params()[2], SqlParam::Bytes(item));
        assert_eq!(
            &row.params()[3..5],
            &[SqlParam::U32(spell), SqlParam::U32(mask)]
        );
    }
    for (row_index, effect_index) in [(3, 0), (4, 2), (6, 1), (8, 3)] {
        assert_eq!(rows[row_index].params()[5], SqlParam::U8(effect_index));
    }
}

#[test]
fn aura_effect_doubles_preserve_nan_payloads_infinities_and_negative_zero_bits() {
    let aura = aura(100);
    let pairs: [(u64, u64); 3] = [
        (0x7FF8_0000_0000_1234, 0x7FF0_0000_0000_0000),
        (0xFFF0_0000_0000_0000, 0x8000_0000_0000_0000),
        (0x8000_0000_0000_0000, 0x7FF8_0000_0000_5678),
    ];
    for (amount, base) in pairs {
        let row = effect_statement(
            17,
            &aura,
            &NewAuraEffect {
                index: 0,
                amount: f64::from_bits(amount),
                base_amount: f64::from_bits(base),
            },
        );
        for (parameter, expected) in row.params()[6..8].iter().zip([amount, base]) {
            match parameter {
                SqlParam::F64(value) => assert_eq!(value.to_bits(), expected),
                other => panic!("expected raw f64 parameter, got {other:?}"),
            }
        }
    }
}

#[test]
fn production_builder_orders_actions_then_aura_parents_and_effects_then_skills() {
    let mut character = super::super::tests::character();
    character.actions = vec![NewAction {
        spec: 0,
        trait_config: 0,
        button: 1,
        action: 42,
        kind: 0,
    }];
    character.auras = vec![aura(100)];
    character.skills = vec![NewSkill {
        skill: 1,
        value: 1,
        maximum: 2,
        profession_slot: -1,
    }];
    let mut transaction = SqlTransaction::new();
    super::super::append_creation_through_skills(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let start = sqls
        .iter()
        .position(|sql| sql.starts_with("INSERT INTO character_action ("))
        .unwrap();
    assert_eq!(
        &sqls[start..],
        &[
            "INSERT INTO character_action (guid, spec, traitConfigId, button, action, type) VALUES (?,?,?,?,?,?)",
            "DELETE FROM character_aura_effect WHERE guid = ?",
            "DELETE FROM character_aura WHERE guid = ?",
            "INSERT INTO character_aura (guid, casterGuid, itemGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges, castItemId, castItemLevel) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_aura_effect (guid, casterGuid, itemGuid, spell, effectMask, effectIndex, amount, baseAmount) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_skills (guid, skill, value, max, professionSlot) VALUES (?,?,?,?,?)",
        ]
    );
}
