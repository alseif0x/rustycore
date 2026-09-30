//! Spell-specific energize valuation.
//!
//! Spell::EffectEnergize, SpellEffects.cpp:1507-1527, target a5f8da2e.
//! The caller samples a player caster's level before invoking this valuation,
//! including ordinary flat spells; the engineering lookup remains lazy.

pub fn energize_spell_amount(
    spell_id: i32,
    damage: i32,
    caster_is_player: bool,
    caster_level: u8,
    engineering: impl FnOnce() -> Option<u16>,
) -> i32 {
    let mut damage = match spell_id {
        // Blood Fury.
        24_571 => damage - 10 * i32::from(caster_level.saturating_sub(60).min(30)),
        // Burst of Energy.
        24_532 => damage - 4 * i32::from(caster_level.saturating_sub(60).min(15)),
        _ => damage,
    };
    if spell_id == 67_490
        && caster_is_player
        && engineering().is_some_and(|value| value != 0)
    {
        damage += (damage as f32 * 25.0 / 100.0) as i32;
    }
    damage
}

#[cfg(test)]
mod tests;
