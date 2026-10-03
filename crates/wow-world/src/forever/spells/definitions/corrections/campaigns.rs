//! Campaigns ID-specific corrections in pinned source order.
//! SpellMgr::LoadSpellInfoCorrections, 02245dcd. Not a legacy fallback.
use super::constants::*;
use super::{Definition, IdCorrectionCounts, Key, SpellCatalog, apply, effect_slot};
use std::collections::BTreeMap;

pub(super) fn run(
    catalog: &SpellCatalog,
    definitions: &mut BTreeMap<Key, Definition>,
    counts: &mut IdCorrectionCounts,
) {
    apply(
        &[111755],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[2] |= SPELL_ATTR2_IGNORE_LINE_OF_SIGHT;
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_DEST_DB;
            }
        },
    );
    apply(
        &[111756],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[2] |= SPELL_ATTR2_IGNORE_LINE_OF_SIGHT;
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_DEST_DB;
            }
        },
    );
    apply(
        &[187382],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(13).map(|row| row.id);
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_DEST_DEST;
            }
        },
    );
    apply(
        &[195061],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_DEST_DEST;
            }
        },
    );
    apply(
        &[193465],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.misc_values[0] = 5838;
            }
        },
    );
    apply(
        &[194981],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(6).map(|row| row.id);
        },
    );
    apply(
        &[196930],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(7).map(|row| row.id);
        },
    );
    apply(
        &[193018],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[0] |= SPELL_ATTR0_AURA_IS_DEBUFF;
        },
    );
    apply(
        &[244449],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 2, _counts) {
                effect.effect = SPELL_EFFECT_NONE;
            }
        },
    );
    apply(
        &[255416],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[2] |= SPELL_ATTR2_IGNORE_LINE_OF_SIGHT;
        },
    );
    apply(
        &[273467],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.radius_ids[0] = _catalog
                    .spell_radius(EFFECT_RADIUS_0_5_YARDS)
                    .map(|row| row.id);
            }
        },
    );
    apply(
        &[269936],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[0] |= SPELL_ATTR0_AURA_IS_DEBUFF;
        },
    );
    apply(
        &[268308],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[2] |= SPELL_ATTR2_IGNORE_LINE_OF_SIGHT;
        },
    );
    apply(
        &[267595, 267597, 267609],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.effect = SPELL_EFFECT_CREATE_CONVERSATION;
            }
        },
    );
    apply(
        &[260566, 260570],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[2] |= SPELL_ATTR2_IGNORE_LINE_OF_SIGHT;
            spell.fields.attributes[9] |= SPELL_ATTR9_FORCE_DEST_LOCATION;
        },
    );
    apply(
        &[365021],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[0] |= SPELL_ATTR0_AURA_IS_DEBUFF;
        },
    );
    apply(
        &[367632],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[0] |= SPELL_ATTR0_AURA_IS_DEBUFF;
        },
    );
    apply(
        &[365017],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_DEST_DEST;
            }
        },
    );
    apply(
        &[365228],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[6] |= SPELL_ATTR6_IGNORE_PHASE_SHIFT;
        },
    );
    apply(
        &[365217],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[6] |= SPELL_ATTR6_IGNORE_PHASE_SHIFT;
        },
    );
    apply(
        &[368913],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[0] |= SPELL_ATTR0_AURA_IS_DEBUFF;
        },
    );
    apply(
        &[365816],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_DEST_DEST;
            }
            if let Some(effect) = effect_slot(spell, 1, _counts) {
                effect.implicit_targets[0] = TARGET_DEST_DEST;
            }
        },
    );
    apply(
        &[363976],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[1] = TARGET_DEST_DEST;
            }
        },
    );
    apply(
        &[374523],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[8] |= SPELL_ATTR8_CAN_ATTACK_IMMUNE_PC;
        },
    );
    apply(
        &[387981],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_DEST_DEST;
            }
        },
    );
    apply(
        &[388082],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_DEST_DEST;
            }
        },
    );
    apply(
        &[274365, 274367, 264911, 264912, 264913],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_DEST_DEST;
            }
        },
    );
    apply(
        &[274668, 274669, 274622, 274640, 274641, 274674, 274675],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.effect = SPELL_EFFECT_CREATE_CONVERSATION;
            }
        },
    );
    apply(
        &[259845],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_DEST_DEST;
            }
        },
    );
    apply(
        &[258344],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[8] &= !SPELL_ATTR8_ONLY_TARGET_IF_SAME_CREATOR;
        },
    );
    apply(
        &[258388, 259205, 259209],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_DEST_DEST;
            }
        },
    );
    apply(
        &[255592],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[2] |= SPELL_ATTR2_IGNORE_LINE_OF_SIGHT;
        },
    );
    apply(
        &[102445],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_DEST_DB;
            }
        },
    );
    apply(
        &[114685],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[1] |= SPELL_ATTR1_NO_THREAT;
            spell.fields.attributes[8] |= SPELL_ATTR8_CAN_ATTACK_IMMUNE_PC;
        },
    );
    apply(
        &[130162],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(245).map(|row| row.id);
        },
    );
    apply(
        &[130237],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 1, _counts) {
                effect.effect = SPELL_EFFECT_NONE;
            }
        },
    );
    apply(
        &[130996, 130997, 130998],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(12).map(|row| row.id);
            spell.fields.attributes[4] &= !SPELL_ATTR4_USE_FACING_FROM_SPELL;
        },
    );
    apply(
        &[130960],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.aura_interrupt_flags[0] |= 0x00080000;
        },
    );
    apply(
        &[421277],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[2] |= SPELL_ATTR2_IGNORE_LINE_OF_SIGHT;
        },
    );
    apply(
        &[420696],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[0] |= SPELL_ATTR0_AURA_IS_DEBUFF;
        },
    );
    apply(
        &[421250],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.duration = _catalog.spell_duration(165).map(|row| row.id);
        },
    );
    apply(
        &[61882],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.negative_effects[2] = true;
        },
    );
    apply(
        &[197214],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 2, _counts) {
                effect.implicit_targets[1] = 0;
            }
        },
    );
    apply(
        &[42401, 43105, 42428],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[0] |= SPELL_ATTR0_NO_IMMUNITIES;
        },
    );
    apply(
        &[61874, 71068, 71071, 71073, 71074],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 1, _counts) {
                effect.effect = SPELL_EFFECT_APPLY_AURA;
                effect.implicit_targets[0] = TARGET_UNIT_CASTER;
                effect.aura = SPELL_AURA_PERIODIC_TRIGGER_SPELL;
                effect.aura_period = 10 * 1_000;
                effect.trigger_spell = 24870;
            }
        },
    );
    apply(
        &[195838, 195843],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.effect = SPELL_EFFECT_APPLY_AURA;
            }
            if let Some(effect) = effect_slot(spell, 1, _counts) {
                effect.effect = SPELL_EFFECT_APPLY_AURA;
            }
            if let Some(effect) = effect_slot(spell, 2, _counts) {
                effect.effect = SPELL_EFFECT_APPLY_AURA;
            }
        },
    );
    apply(
        &[181593],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.trigger_spell = 0;
            }
        },
    );
    apply(
        &[265057],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.trigger_spell = 16403;
            }
        },
    );
    apply(
        &[269748],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[1] &= !SPELL_ATTR1_IS_CHANNELLED;
        },
    );
    apply(
        &[111400],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[4] |= SPELL_ATTR4_AURA_IS_BUFF;
        },
    );
    apply(
        &[404468],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.custom_attributes |= SPELL_ATTR0_CU_AURA_CANNOT_BE_SAVED;
        },
    );
    apply(
        &[204598],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 2, _counts) {
                effect.effect = SPELL_EFFECT_NONE;
            }
        },
    );
    apply(
        &[1225826, 1226019, 1245453],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.negative_effects[0] = true;
        },
    );
    apply(
        &[391057, 393831],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[1] &= !SPELL_ATTR1_IS_CHANNELLED;
        },
    );
}
