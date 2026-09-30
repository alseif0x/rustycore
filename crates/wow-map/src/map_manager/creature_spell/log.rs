//! Capture canonical log state at the original admitted hit point.
use super::*;
use wow_constants::PowerType;

pub fn cast_log(
    caster: &wow_entities::Creature,
    spell: &SpellInfoFacts,
    difficulty_id: u8,
) -> Option<SpellLog> {

    // The enclosing M2.6 cast path admits only base-difficulty spells and zero
    // effective costs. Keep the cost/aura portions independently fail-closed
    // so later callers cannot fabricate a complete log snapshot.
    if difficulty_id != 0
        || has_nonzero_power_cost(spell)
        || !caster
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_cast_log_aura_authority_like_cpp()
    {
        return None;
    }

    // C++ `SpellInfo::CalcPowerCost` retains one row per power type even when
    // its effective amount is zero. That includes signed enum sentinels such
    // as `POWER_ALL=127`: the unknown-power rejection is reached only by
    // cost-bearing branches (for example percentage or use-all-power), all of
    // which M2.6 rejects before this helper. Preserve the remaining zero rows
    // in DB2 order and deduplicate them by their raw signed type.
    let mut power_data = Vec::new();
    for power in &spell.power_costs {
        let power_type = i32::from(power.power_type);
        if power_data
            .iter()
            .any(|known: &SpellLogPower| known.power_type == power_type)
        {
            continue;
        }
        // C++ retains the signed raw enum value in the wire row. Values that
        // cannot address its Unit power array keep an amount of zero without
        // changing or dropping the row (including HEALTH=-2 and POWER_ALL).
        let amount = <PowerType as num_traits::FromPrimitive>::from_i8(power.power_type)
            .map(|represented_power| caster.unit().get_power(represented_power))
            .unwrap_or(0);
        power_data.push(SpellLogPower {
            power_type,
            amount,
            cost: 0,
        });
    }

    let primary_power = caster.power_type();
    let primary_power_type = primary_power as i32;
    if !power_data
        .iter()
        .any(|power| power.power_type == primary_power_type)
    {
        power_data.insert(
            0,
            SpellLogPower {
                power_type: primary_power_type,
                amount: caster.unit().get_power(primary_power),
                cost: 0,
            },
        );
    }

    let stats = caster.combat_log_stats_like_cpp();
    Some(SpellLog {
        // Mirrors the C++ `uint64 GetHealth()` assignment to the signed wire
        // field. DB-backed creature health originates in a u32 and always fits.
        health: caster.unit().data().health as i64,
        attack_power: caster.combat_log_attack_power_like_cpp(),
        spell_power: stats.spell_power,
        armor: stats.armor,
        power_data,
    })
}
