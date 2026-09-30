//! Power-pool transitions used by spell effects.
//!
//! Target-version anchors: Unit::EnergizeBySpell (Unit.cpp:6579-6590),
//! Spell::EffectPowerDrain/EffectPowerBurn (SpellEffects.cpp:1069-1165).
//! Admission, regen interruption, threat and publication remain with callers.
//! These transitions preserve the represented pool arithmetic, not the wider
//! C++ ModifyPower aura/min-power and SetPower publication behavior.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellPowerAmount {
    Flat(i32),
    Percent(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellPowerGain {
    pub requested: i32,
    pub applied: i32,
}

impl Unit {
    pub fn energize_spell_power(
        &mut self,
        power: PowerType,
        amount: SpellPowerAmount,
    ) -> Option<SpellPowerGain> {
        let max_power = self.get_max_power(power);
        if max_power <= 0 {
            return None;
        }
        let requested = match amount {
            SpellPowerAmount::Percent(damage) => {
                ((i64::from(max_power) * i64::from(damage)) / 100)
                    .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
            }
            SpellPowerAmount::Flat(damage) => damage,
        };
        let current = self.get_power(power);
        let after = current.saturating_add(requested).clamp(0, max_power);
        self.set_power(power, after);
        Some(SpellPowerGain {
            requested,
            applied: after - current,
        })
    }

    /// The caller converts the active-power fact from this same Unit while
    /// retaining its canonical guard. Return the raw drain, including when
    /// SetPower's existing missing-index or upper-cap behavior changes no pool.
    pub fn drain_spell_power(
        &mut self,
        power: PowerType,
        active_power: PowerType,
        amount: i32,
    ) -> Option<i32> {
        if active_power != power {
            return None;
        }
        let current = self.get_power(power).max(0);
        let drain = current.min(amount);
        self.set_power(power, current - drain);
        Some(drain)
    }
}

#[cfg(test)]
mod tests;
