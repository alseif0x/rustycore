//! Encounters ID-specific corrections in pinned source order.
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
        &[54258, 54264, 54265, 54266, 54267],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.radius_ids[0] = _catalog
                    .spell_radius(EFFECT_RADIUS_25_YARDS)
                    .map(|row| row.id);
            }
        },
    );
    apply(
        &[62374],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.radius_ids[0] = _catalog
                    .spell_radius(EFFECT_RADIUS_50000_YARDS)
                    .map(|row| row.id);
            }
        },
    );
    apply(
        &[63342],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.max_affected_targets = 1;
        },
    );
    apply(
        &[65584, 64381],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[3] |= SPELL_ATTR3_DOT_STACKING_RULE;
        },
    );
    apply(
        &[63018, 65121, 63024, 64234],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.max_affected_targets = 1;
        },
    );
    apply(
        &[64386, 64389, 64678],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.duration = _catalog.spell_duration(28).map(|row| row.id);
        },
    );
    apply(
        &[64321],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[1] |= SPELL_ATTR1_IMMUNITY_PURGES_EFFECT;
        },
    );
    apply(
        &[62576, 62602],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_DEST_CASTER_LEFT;
            }
        },
    );
    apply(
        &[63414],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.channel_interrupt_flags[0] = 0;
            spell.fields.channel_interrupt_flags[1] = 0;
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[1] = TARGET_UNIT_CASTER;
            }
        },
    );
    apply(
        &[63036],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.speed = 0.0;
        },
    );
    apply(
        &[64668],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.mechanic = MECHANIC_NONE;
        },
    );
    apply(
        &[64468, 64486],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.max_affected_targets = 3;
        },
    );
    apply(
        &[62301],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.max_affected_targets = 1;
        },
    );
    apply(
        &[64598],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.max_affected_targets = 3;
        },
    );
    apply(
        &[62293],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[1] = TARGET_DEST_CASTER;
            }
        },
    );
    apply(
        &[62311, 64596],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(6).map(|row| row.id);
        },
    );
    apply(
        &[
            64014, 64024, 64025, 64028, 64029, 64030, 64031, 64032, 65042,
        ],
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
        &[66258],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.duration = _catalog.spell_duration(85).map(|row| row.id);
        },
    );
    apply(
        &[70781, 70856, 70857, 70858, 70859, 70860, 70861],
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
        &[71169],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[3] |= SPELL_ATTR3_DOT_STACKING_RULE;
        },
    );
    apply(
        &[72723],
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
        &[70460],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.duration = _catalog.spell_duration(1).map(|row| row.id);
        },
    );
    apply(
        &[71412, 71415],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_UNIT_TARGET_ANY;
            }
        },
    );
    apply(
        &[71159],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.duration = _catalog.spell_duration(21).map(|row| row.id);
        },
    );
    apply(
        &[70530],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.effect = SPELL_EFFECT_APPLY_AURA;
            }
        },
    );
    apply(
        &[71604],
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
        &[70911],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[1] = TARGET_UNIT_TARGET_ENEMY;
            }
        },
    );
    apply(
        &[71708],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[3] |= SPELL_ATTR3_IGNORE_CASTER_MODIFIERS;
        },
    );
    apply(
        &[71266],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.required_areas_id = 0;
        },
    );
    apply(
        &[70602],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[3] |= SPELL_ATTR3_DOT_STACKING_RULE;
        },
    );
    apply(
        &[70715],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.duration = _catalog.spell_duration(32).map(|row| row.id);
        },
    );
    apply(
        &[71085],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.duration = _catalog.spell_duration(9).map(|row| row.id);
        },
    );
    apply(
        &[70936],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(157).map(|row| row.id);
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_UNIT_TARGET_ANY;
                effect.implicit_targets[1] = 0;
            }
        },
    );
    apply(
        &[70598],
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
        &[69846],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.speed = 0.0;
        },
    );
    apply(
        &[70106],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[3] |= SPELL_ATTR3_IGNORE_CASTER_MODIFIERS;
            spell.fields.attributes[6] |= SPELL_ATTR6_IGNORE_CASTER_DAMAGE_MODIFIERS;
        },
    );
    apply(
        &[71614],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.mechanic = MECHANIC_STUN;
        },
    );
    apply(
        &[72762],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.duration = _catalog.spell_duration(559).map(|row| row.id);
        },
    );
    apply(
        &[72743],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.duration = _catalog.spell_duration(22).map(|row| row.id);
        },
    );
    apply(
        &[72754],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.radius_ids[0] = _catalog
                    .spell_radius(EFFECT_RADIUS_200_YARDS)
                    .map(|row| row.id);
            }
            if let Some(effect) = effect_slot(spell, 1, _counts) {
                effect.radius_ids[0] = _catalog
                    .spell_radius(EFFECT_RADIUS_200_YARDS)
                    .map(|row| row.id);
            }
        },
    );
    apply(
        &[69030],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[0] |= SPELL_ATTR0_NO_IMMUNITIES;
        },
    );
    apply(
        &[69198],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(13).map(|row| row.id);
        },
    );
    apply(
        &[73655],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[3] |= SPELL_ATTR3_IGNORE_CASTER_MODIFIERS;
        },
    );
    apply(
        &[73540],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.duration = _catalog.spell_duration(3).map(|row| row.id);
        },
    );
    apply(
        &[73530],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.duration = _catalog.spell_duration(27).map(|row| row.id);
        },
    );
    apply(
        &[74302],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.max_affected_targets = 2;
        },
    );
    apply(
        &[73579],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.radius_ids[0] = _catalog
                    .spell_radius(EFFECT_RADIUS_25_YARDS)
                    .map(|row| row.id);
            }
        },
    );
    apply(
        &[72376],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.max_affected_targets = 3;
        },
    );
    apply(
        &[71809],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(5).map(|row| row.id);
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.radius_ids[0] = _catalog
                    .spell_radius(EFFECT_RADIUS_10_YARDS)
                    .map(|row| row.id);
                effect.misc_values[0] = 190;
            }
        },
    );
    apply(
        &[74799],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 1, _counts) {
                effect.radius_ids[0] = _catalog
                    .spell_radius(EFFECT_RADIUS_12_YARDS)
                    .map(|row| row.id);
            }
        },
    );
    apply(
        &[75509],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[6] |= SPELL_ATTR6_IGNORE_PHASE_SHIFT;
            spell.fields.attributes[2] |= SPELL_ATTR2_IGNORE_LINE_OF_SIGHT;
        },
    );
    apply(
        &[75888],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[1] |= SPELL_ATTR1_EXCLUDE_CASTER;
        },
    );
    apply(
        &[57473, 57431, 56091, 56092, 57090, 57143],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[2] |= SPELL_ATTR2_IGNORE_LINE_OF_SIGHT;
        },
    );
    apply(
        &[63934],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[2] |= SPELL_ATTR2_CANT_CRIT;
        },
    );
    apply(
        &[40055, 40165, 40166, 40167],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[0] |= SPELL_ATTR0_AURA_IS_DEBUFF;
        },
    );
    apply(
        &[95284, 95285],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[1] = TARGET_DEST_DB;
            }
        },
    );
    apply(
        &[76606, 76608],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.radius_ids[1] = _catalog
                    .spell_radius(EFFECT_RADIUS_45_YARDS)
                    .map(|row| row.id);
            }
        },
    );
    apply(
        &[24314],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.aura_interrupt_flags[0] |= 0x00000004 | 0x00000008 | 0x00000020;
        },
    );
    apply(
        &[783],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[0] |= SPELL_ATTR0_ONLY_OUTDOORS;
        },
    );
    apply(
        &[5420],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.stances = 1u64 << (FORM_TREE_OF_LIFE - 1);
        },
    );
    apply(
        &[96942],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[1] &= !SPELL_ATTR1_IS_CHANNELLED;
        },
    );
    apply(
        &[75610],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.max_affected_targets = 1;
        },
    );
    apply(
        &[75697],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.implicit_targets[0] = TARGET_UNIT_SRC_AREA_ENTRY;
            }
        },
    );
    apply(
        &[66551],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(13).map(|row| row.id);
        },
    );
    apply(
        &[40453],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.aura = SPELL_AURA_PROC_TRIGGER_SPELL;
                effect.aura_period = 0;
            }
            spell.fields.proc_chance = 10;
        },
    );
    apply(
        &[45853],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.range = _catalog.spell_range(5).map(|row| row.id);
        },
    );
    apply(
        &[17466, 17467],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[2] |= SPELL_ATTR2_NO_INITIAL_THREAT;
        },
    );
    apply(
        &[42525],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[3] |= SPELL_ATTR3_ALLOW_AURA_WHILE_DEAD;
            spell.fields.attributes[2] |= SPELL_ATTR2_ALLOW_DEAD_TARGET;
        },
    );
    apply(
        &[69131],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 1, _counts) {
                effect.aura = SPELL_AURA_MOD_DECREASE_SPEED;
            }
        },
    );
    apply(
        &[99253],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            if let Some(effect) = effect_slot(spell, 0, _counts) {
                effect.radius_ids[1] = _catalog
                    .spell_radius(EFFECT_RADIUS_15_YARDS)
                    .map(|row| row.id);
            }
        },
    );
    apply(
        &[99256],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.attributes[0] |= SPELL_ATTR0_AURA_IS_DEBUFF;
        },
    );
    apply(
        &[99252],
        catalog,
        definitions,
        counts,
        |spell, _catalog, _counts| {
            spell.fields.aura_interrupt_flags[0] |= 0x00080000;
        },
    );
}
