//! Immutable spell metadata projections; no actor snapshot or catalog cache.
use super::*;

pub(in crate::session) fn spell_info(spell: &wow_data::SpellInfo) -> SpellInfoFacts {
    SpellInfoFacts {
        spell_id: spell.spell_id, cast_time_ms: spell.cast_time_ms,
        requires_spell_focus: spell.requires_spell_focus, effect_type: spell.effect_type,
        aura_type: spell.aura_type, effect_base_points: spell.effect_base_points,
        effects: spell.effects().iter().map(|row| SpellEffectFacts {
            effect: row.effect, effect_index: row.effect_index, effect_aura: row.effect_aura,
            effect_base_points: row.effect_base_points, implicit_target_1: row.implicit_target_1,
            implicit_target_2: row.implicit_target_2, chain_targets: row.chain_targets,
            effect_radius_index_1: row.effect_radius_index_1, effect_trigger_spell: row.effect_trigger_spell,
        }).collect(),
        power_costs: spell.power_costs.iter().map(|row| SpellPowerFacts {
            power_type: row.power_type, mana_cost: row.mana_cost, mana_cost_per_level: row.mana_cost_per_level,
            mana_per_second: row.mana_per_second, power_cost_pct: row.power_cost_pct,
            power_cost_max_pct: row.power_cost_max_pct, power_pct_per_second: row.power_pct_per_second,
            required_aura_spell_id: row.required_aura_spell_id, optional_cost: row.optional_cost,
        }).collect(),
    }
}

pub(in crate::session) fn hit_metadata(row: wow_data::spell::SpellHitMetadataLikeCpp) -> SpellHitFacts {
    SpellHitFacts { defense_type: row.defense_type, school_mask: row.school_mask,
        spell_mechanic: row.spell_mechanic, effect_mechanics: row.effect_mechanics }
}

pub(in crate::session) fn log(log: SpellLog) -> wow_packet::packets::spell::SpellCastLogData {
    wow_packet::packets::spell::SpellCastLogData {
        health: log.health, attack_power: log.attack_power, spell_power: log.spell_power, armor: log.armor,
        power_data: log.power_data.into_iter().map(|row| wow_packet::packets::spell::SpellLogPowerData {
            power_type: row.power_type, amount: row.amount, cost: row.cost,
        }).collect(),
    }
}

pub(in crate::session) fn hit(hit: SpellHit) -> CreatureSpellTargetHitResultLikeCpp {
    match hit { SpellHit::Hit => CreatureSpellTargetHitResultLikeCpp::Hit,
        SpellHit::Miss => CreatureSpellTargetHitResultLikeCpp::Miss }
}
