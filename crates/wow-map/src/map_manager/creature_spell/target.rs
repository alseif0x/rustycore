//! Stock polarity classification, evaluated only after the max-range gate.
use super::*;
use wow_constants::spell::aura_types;
pub fn spell_is_positive(spell_info: &SpellInfoFacts) -> bool {
    let effects: Vec<(u32, i32, i32, u32, u32)> = if spell_info.effects.is_empty() {
        vec![(
            spell_info.effect_type,
            spell_info.aura_type.unwrap_or(0),
            spell_info.effect_base_points,
            0,
            0,
        )]
    } else {
        spell_info
            .effects
            .iter()
            .map(|effect| {
                (
                    effect.effect,
                    effect.effect_aura,
                    effect.effect_base_points,
                    effect.implicit_target_1,
                    effect.implicit_target_2,
                )
            })
            .collect()
    };

    const fn target_checks_enemy(target: u32) -> bool {
        matches!(
            target,
            2 | 6 | 15 | 16 | 24 | 28 | 53 | 54 | 93 | 104 | 108 | 115 | 116 | 129 | 134 | 151
        )
    }

    !effects
        .into_iter()
        .any(|(effect, aura, amount, target_a, target_b)| {
            let targets_enemy = target_checks_enemy(target_a) || target_checks_enemy(target_b);
            effect == 1 /* SPELL_EFFECT_INSTAKILL */
                || effect == 2 /* SPELL_EFFECT_SCHOOL_DAMAGE */
                || effect == 7 /* SPELL_EFFECT_ENVIRONMENTAL_DAMAGE */
                || effect == 8 /* SPELL_EFFECT_POWER_DRAIN */
                || effect == 62 /* SPELL_EFFECT_POWER_BURN */
                || effect == 9 /* SPELL_EFFECT_HEALTH_LEECH */
                || effect == 63 /* SPELL_EFFECT_THREAT */
                || effect == 125 /* SPELL_EFFECT_MODIFY_THREAT_PERCENT */
                || effect == 114 /* SPELL_EFFECT_ATTACK_ME */
                || effect == 69 /* SPELL_EFFECT_DISTRACT */
                || (effect == 6 /* SPELL_EFFECT_APPLY_AURA */
                    && (targets_enemy
                        || matches!(
                            aura,
                            aura_types::SPELL_AURA_PERIODIC_DAMAGE
                                | aura_types::SPELL_AURA_PERIODIC_DAMAGE_PERCENT
                                | aura_types::SPELL_AURA_MOD_CONFUSE
                                | aura_types::SPELL_AURA_MOD_FEAR
                                | aura_types::SPELL_AURA_MOD_TAUNT
                                | aura_types::SPELL_AURA_MOD_STUN
                                | aura_types::SPELL_AURA_MOD_ROOT
                                | aura_types::SPELL_AURA_MOD_SILENCE
                                | aura_types::SPELL_AURA_MOD_DECREASE_SPEED
                                | aura_types::SPELL_AURA_SCHOOL_HEAL_ABSORB
                        )
                        || (matches!(
                            aura,
                            aura_types::SPELL_AURA_MOD_STAT
                                | aura_types::SPELL_AURA_MOD_INCREASE_HEALTH
                                | aura_types::SPELL_AURA_MOD_INCREASE_HEALTH_PERCENT
                                | aura_types::SPELL_AURA_MOD_SCALE
                        ) && amount < 0)))
        })
}

pub fn classify_target(spell: &SpellInfoFacts) -> SpellTarget {
    let positive = spell_is_positive(spell);
    spell
        .effects
        .iter()
        .fold(SpellTarget::SelfTarget, |selected, effect| {
            let target_a = effect.implicit_target_1;
            let mut candidate = match target_a {
                6 | 53 => SpellTarget::Victim,
                16 => SpellTarget::Enemy,
                _ => SpellTarget::SelfTarget,
            };
            if effect.effect == 6 {
                if target_a == 6 {
                    candidate = SpellTarget::Debuff;
                } else if positive {
                    candidate = SpellTarget::Buff;
                }
            }
            selected.max(candidate)
        })
}
