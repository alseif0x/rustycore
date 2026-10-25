//! Selected aura metadata for causal Player stat application.

use wow_world_core::session::PlayerStatApplicationCause;

pub(super) struct AuraStatApplication {
    pub cause: Option<PlayerStatApplicationCause>,
    pub is_ability: bool,
    pub total_stat_percentage_effects: Vec<(u32, i32, i32, i32)>,
}

pub(super) fn select_aura_stat_application(
    spell_store: Option<&wow_data::SpellStore>,
    spell_id: i32,
    effect_mask: u32,
    required: bool,
) -> Result<Option<AuraStatApplication>, &'static str> {
    let Some(spell_store) = spell_store else {
        return if required {
            Err("Missing SpellStore for the race-selected sickness aura")
        } else {
            Ok(None)
        };
    };
    let Some(spell) = spell_store.get(spell_id) else {
        return if required {
            Err("Missing SpellInfo for the race-selected sickness aura")
        } else {
            Ok(None)
        };
    };

    let is_ability = spell_store.has_attribute0_like_cpp(
        spell_id,
        wow_data::spell::attributes::SPELL_ATTR0_IS_ABILITY,
    );
    let mut has_stat_aura = false;
    let mut includes_all_stats = false;
    let mut stat_mask = 0_u8;
    let mut modifies_damage = false;
    let mut preserves_health_pct = false;
    let mut total_stat_percentage_effects = Vec::new();

    for effect in spell.effects() {
        let Some(bit) = 1_u32.checked_shl(effect.effect_index) else {
            continue;
        };
        if effect_mask & bit == 0 {
            continue;
        }

        let selected_mask = match effect.effect_aura {
            wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT => {
                match effect.effect_misc_value_1 {
                    -1 => {
                        has_stat_aura = true;
                        Some(0)
                    }
                    0..=4 => {
                        has_stat_aura = true;
                        Some(1_u8 << effect.effect_misc_value_1)
                    }
                    _ if required => {
                        return Err("Selected percent-stat aura has invalid stat index");
                    }
                    _ => None,
                }
            }
            wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE => {
                // C++ loops only over the five Stat values and tests the raw
                // signed miscB against each bit (`SpellAuraEffects.cpp:3673`).
                // Zero is the special all-stats value; nonzero values with no
                // low-five bits simply select no stat effects.
                let raw_mask = effect.effect_misc_value_2 as u32;
                let mask = (raw_mask & 0b1_1111) as u8;
                if raw_mask == 0 || mask != 0 {
                    has_stat_aura = true;
                    total_stat_percentage_effects.push((
                        effect.effect_index,
                        effect.calc_value_no_caster_like_cpp(),
                        effect.effect_misc_value_1,
                        effect.effect_misc_value_2,
                    ));
                    if is_ability && (raw_mask == 0 || mask & (1 << 2) != 0) {
                        preserves_health_pct = true;
                    }
                    Some(if raw_mask == 0 { 0 } else { mask })
                } else {
                    None
                }
            }
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE => {
                modifies_damage = true;
                None
            }
            _ => None,
        };

        if let Some(mask) = selected_mask {
            if mask == 0 {
                includes_all_stats = true;
            } else {
                stat_mask |= mask;
            }
        }
    }

    let cause = if has_stat_aura {
        Some(PlayerStatApplicationCause::StatAura {
            stat_mask: if includes_all_stats { 0 } else { stat_mask },
            preserve_health_pct: preserves_health_pct,
        })
    } else if modifies_damage {
        Some(PlayerStatApplicationCause::DamageModifiers)
    } else {
        None
    };

    if required && cause.is_none() {
        return Err("Race-selected sickness aura has no supported stat effects");
    }

    Ok(Some(AuraStatApplication {
        cause,
        is_ability,
        total_stat_percentage_effects,
    }))
}
