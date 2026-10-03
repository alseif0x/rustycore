//! General ID-specific corrections in pinned source order.
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
        &[6727, 7331, 34589, 52562, 57550, 65755],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.aura_period = 1_000;
            }
        },
    );
    apply(
        &[24707, 26263, 29055],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 1, _counts) {
                effect.aura_period = 1_000;
            }
        },
    );
    apply(
        &[37504],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 1, _counts) {
                effect.aura_period = 5 * 1_000;
            }
        },
    );
    apply(
        &[43327],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 1, _counts) {
                effect.aura_period = 1_000;
            }
        },
    );
    apply(
        &[23170],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.trigger_spell = 23171;
            }
        },
    );
    apply(
        &[29917],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.trigger_spell = 29916;
            }
        },
    );
    apply(
        &[38495],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.trigger_spell = 38530;
            }
        },
    );
    apply(
        &[39857],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.trigger_spell = 39856;
            }
        },
    );
    apply(
        &[46736],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.trigger_spell = 46737;
                effect.aura = SPELL_AURA_PERIODIC_TRIGGER_SPELL;
            }
        },
    );
    apply(
        &[63026, 63137],
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
        &[52611, 52612],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.misc_values[1] = 64;
            }
        },
    );
    apply(
        &[40244, 40245, 40246, 40247, 42835],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.effect = SPELL_EFFECT_NONE;
            }
        },
    );
    apply(
        &[63665, 51904, 68933, 29200],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_UNIT_CASTER;
                effect.implicit_targets[1] = 0;
            }
        },
    );
    apply(
        &[56690, 60586, 60776, 60881, 60864],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[4] |= SPELL_ATTR4_IGNORE_DAMAGE_TAKEN_MODIFIERS;
        },
    );
    apply(
        &[42818, 42821, 720, 731],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(6).map(|row| row.id);
        },
    );
    apply(
        &[36350],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.trigger_spell = 36325;
            }
        },
    );
    apply(
        &[
            36327, 39365, 41071, 42442, 42611, 44978, 45001, 45002, 45004, 45006, 45010, 45761,
            45863, 48246, 41635, 44869, 45027, 45976, 52124, 52479, 61588, 55479, 28560, 53096,
            70743, 70614, 4020, 52438, 52449, 53609, 53457, 45907, 52953, 58121, 43109, 58552,
            58533, 21855, 38762, 51122, 71848, 36146, 33711, 38794,
        ],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.max_affected_targets = 1;
        },
    );
    apply(
        &[36384, 47731],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.max_affected_targets = 2;
        },
    );
    apply(
        &[
            28542, 29213, 29576, 37790, 39992, 40816, 41303, 41376, 45248, 46771, 66588,
        ],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.max_affected_targets = 3;
        },
    );
    apply(
        &[38310, 53385],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.max_affected_targets = 4;
        },
    );
    apply(
        &[42005, 38296, 37676, 46008, 45641, 55665, 28796, 37135],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.max_affected_targets = 5;
        },
    );
    apply(
        &[40827, 40859, 40860, 40861, 54098, 54835],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.max_affected_targets = 10;
        },
    );
    apply(
        &[50312],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.max_affected_targets = 15;
        },
    );
    apply(
        &[44544],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.class_mask[0] |= 0x20000;
            }
        },
    );
    apply(
        &[52212, 41485, 41487],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[6] |= SPELL_ATTR6_IGNORE_PHASE_SHIFT;
        },
    );
    apply(
        &[37408],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[3] |= SPELL_ATTR3_DOT_STACKING_RULE;
        },
    );
    apply(
        &[51912],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.aura_period = 3000;
            }
        },
    );
    apply(
        &[36854, 36856],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(5).map(|row| row.id);
        },
    );
    apply(
        &[30421],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 2, _counts) {
                effect.base_points += 30000.0;
            }
        },
    );
    apply(
        &[41913],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.aura = SPELL_AURA_DUMMY;
            }
        },
    );
    apply(
        &[27892, 27928, 27935],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.radius_ids[0] = _catalog
                    .spell_radius(EFFECT_RADIUS_10_YARDS)
                    .map(|row| row.id);
            }
        },
    );
    apply(
        &[29214, 54836],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[1] = TARGET_UNIT_SRC_AREA_ALLY;
            }
            if let Some(effect) = effect_slot(spell, 1, _counts) {
                effect.implicit_targets[1] = TARGET_UNIT_SRC_AREA_ALLY;
            }
        },
    );
    apply(
        &[6474],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[5] |= SPELL_ATTR5_EXTRA_INITIAL_PERIOD;
        },
    );
    apply(
        &[70728, 70840],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_UNIT_CASTER;
                effect.implicit_targets[1] = TARGET_UNIT_PET;
            }
        },
    );
    apply(
        &[45602],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.base_points = 0.0;
            }
        },
    );
    apply(
        &[61719],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.aura_interrupt_flags[0] = 0x00000001 | 0x00000002;
        },
    );
    apply(
        &[71838, 71839],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[2] |= SPELL_ATTR2_CANT_CRIT;
        },
    );
    apply(
        &[51597, 56606, 61791],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.base_points = 1.0;
            }
        },
    );
    apply(
        &[51597],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.scaling_variance = 0.0;
            }
        },
    );
    apply(
        &[59630],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[0] |= SPELL_ATTR0_PASSIVE;
        },
    );
    apply(
        &[48278],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[3] |= SPELL_ATTR3_DOT_STACKING_RULE;
        },
    );
    apply(
        &[51798, 47134],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.effect = SPELL_EFFECT_NONE;
            }
        },
    );
    apply(
        &[85123],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.radius_ids[0] = _catalog
                    .spell_radius(EFFECT_RADIUS_200_YARDS)
                    .map(|row| row.id);
                effect.implicit_targets[0] = TARGET_UNIT_SRC_AREA_ENTRY;
            }
        },
    );
    apply(
        &[187881],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 1, _counts) {
                effect.class_mask[1] |= 0x1000;
            }
        },
    );
    apply(
        &[15538, 42490, 42492, 43115],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[1] |= SPELL_ATTR1_NO_THREAT;
        },
    );
    apply(
        &[29726],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.channel_interrupt_flags[0] &= !0x00000004;
        },
    );
    apply(
        &[42767],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_UNIT_NEARBY_ENTRY;
            }
        },
    );
    apply(
        &[42793],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 2, _counts) {
                effect.misc_values[0] = 24008;
            }
        },
    );
    apply(
        &[59544, 121093],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.spell_family_flags[2] = 0x80000000;
        },
    );
    apply(
        &[50661, 68979, 48714, 7853],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(13).map(|row| row.id);
        },
    );
    apply(
        &[44327, 44408],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.speed = 0.0;
        },
    );
    apply(
        &[28864, 29105],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.radius_ids[0] = _catalog
                    .spell_radius(EFFECT_RADIUS_10_YARDS)
                    .map(|row| row.id);
            }
        },
    );
    apply(
        &[37851, 37918],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.recovery_time = 3000;
        },
    );
    apply(
        &[56513],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.recovery_time = 2000;
        },
    );
    apply(
        &[54997, 56524],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.recovery_time = 6000;
        },
    );
    apply(
        &[47911, 48620, 51752],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.recovery_time = 10000;
        },
    );
    apply(
        &[37727, 54996],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.recovery_time = 12000;
        },
    );
    apply(
        &[51748],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.recovery_time = 15000;
        },
    );
    apply(
        &[51756, 37919, 37917],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.recovery_time = 20000;
        },
    );
    apply(
        &[53525],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.duration = _catalog.spell_duration(4).map(|row| row.id);
        },
    );
    apply(
        &[38469],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(6).map(|row| row.id);
        },
    );
    apply(
        &[188290],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 3, _counts) {
                effect.class_mask = [0x80, 0, 0, 0x8000];
            }
        },
    );
    apply(
        &[236299],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(6).map(|row| row.id);
        },
    );
    apply(
        &[373427],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 3, _counts) {
                effect.effect = SPELL_EFFECT_DUMMY;
            }
        },
    );
    apply(
        &[202112],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_DEST_DB;
            }
        },
    );
}
