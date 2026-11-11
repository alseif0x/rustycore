//! Authoritative coordinator + canonical membership; synthetic aura/data inputs.
//! Ordered closures here are NOT GNU unordered_set order proof.
use super::*;
const IGNORE_POWER: u32 = 0x4;
const IGNORE_CAST_TIME: u32 = 0x40;
const IGNORE_SHAPESHIFT: u32 = 0x400;
use crate::forever::player::{CastOverrideAura, CastSpellAuras, CastSpellError};
use crate::forever::spells::{
    SpellConstructorFields, SpellDefinitionError, SpellDefinitionSeeds, SpellEffectValues,
    cast_test_definitions,
};
use wow_data::forever_spells::{DifficultyRecord, SpellText};

type Row = ((u32, i16), SpellConstructorFields, Vec<SpellEffectValues>);
fn row(id: u32) -> Row {
    ((id, 0), Default::default(), vec![])
}
fn aura_row(id: u32, kind: u32, misc: i32, family: u32, mask: [u32; 4], flags: u32) -> Row {
    let mut fields = SpellConstructorFields {
        spell_family_name: family,
        ..Default::default()
    };
    if flags & IGNORE_POWER != 0 {
        fields.attributes[8] = 0x40000;
    }
    if flags & IGNORE_SHAPESHIFT != 0 {
        fields.attributes[11] = 0x200;
    }
    (
        (id, 0),
        fields,
        vec![SpellEffectValues {
            effect: 6,
            aura: kind,
            misc_values: [misc, 0],
            class_mask: mask,
            // These base points must never replace the live amount.
            base_points: 999.0,
            ..Default::default()
        }],
    )
}
fn run(
    book: &PlayerSpellBook,
    s: &SpellDefinitionSeeds,
    id: u32,
    difficulty: i16,
    auras: &CastSpellAuras<'_>,
    order: impl FnMut(&[SpellBookMutation], usize) -> Result<Vec<u32>, &'static str>,
) -> Result<(u32, i16, u32), CastSpellError<&'static str>> {
    book.cast_spell_info(s.get_exact(id, 0).unwrap(), s, difficulty, auras, order)
        .map(|result| {
            (
                result.definition().spell_id(),
                result.definition().difficulty(),
                result.trigger_flags(),
            )
        })
}
fn none() -> CastSpellAuras<'static> {
    CastSpellAuras::new(&[], &[])
}
fn panic_order(_: &[SpellBookMutation], _: usize) -> Result<Vec<u32>, &'static str> {
    panic!("absent group must not call provider")
}
fn difficulty(id: u32, fallback: i16) -> DifficultyRecord {
    DifficultyRecord {
        id,
        name: SpellText::default(),
        instance_type: 0,
        order_index: 0,
        old_enum_value: 0,
        fallback_difficulty_id: fallback,
        min_players: 0,
        max_players: 0,
        flags: 0,
        item_context: 0,
        toggle_difficulty_id: 0,
        group_size_health_curve_id: 0,
        group_size_dmg_curve_id: 0,
        group_size_spell_points_curve_id: 0,
        unknown1105: 0,
    }
}

mod boundary;
mod operations;
