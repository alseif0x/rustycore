use super::Player;
use wow_constants::{DeathState, PowerType};

impl Player {
    /// Apply the represented self-resurrection health and power transition.
    ///
    /// C++ `Spell::EffectSelfResurrect` (`SpellEffects.cpp:3757-3793`). Retains
    /// the current integer saturation and clamping. This does not add the full
    /// `ResurrectPlayer` lifecycle, sickness, ghost cleanup, timer reset or bones.
    pub fn apply_self_resurrection(&mut self, damage: i32, misc_value: i32) -> Option<u32> {
        if self.unit().is_alive() {
            return None;
        }
        let max_health = self
            .unit()
            .data()
            .max_health
            .clamp(1, u64::from(u32::MAX)) as u32;
        let (health, mana) = if damage < 0 {
            (damage.saturating_abs() as u32, misc_value.max(0))
        } else {
            let pct = damage.max(0);
            (
                max_health
                    .saturating_mul(u32::try_from(pct).unwrap_or(u32::MAX))
                    .saturating_div(100),
                self.get_max_power(PowerType::Mana)
                    .max(0)
                    .saturating_mul(pct)
                    / 100,
            )
        };
        let health = health.min(max_health);
        self.unit_mut().set_death_state(DeathState::Alive);
        self.unit_mut().set_health(u64::from(health));
        self.unit_mut().set_power(PowerType::Mana, mana);
        self.unit_mut().set_power(PowerType::Rage, 0);
        let max_energy = self.get_max_power(PowerType::Energy);
        self.unit_mut().set_power(PowerType::Energy, max_energy);
        self.unit_mut().set_power(PowerType::Focus, 0);
        Some(health)
    }

    /// Apply the represented percentage revival used by teleport.
    ///
    /// C++ `Player::ResurrectPlayer` (`Player.cpp:4320-4404`). Retains the
    /// existing f32 calculation, admission of Alive and nonpositive/NaN
    /// percentages, and omission of the surrounding resurrection lifecycle.
    /// All three power benefits are read before any power write.
    pub fn apply_percentage_resurrection(&mut self, restore_percent: f32) -> u32 {
        let max_health = self
            .unit()
            .data()
            .max_health
            .clamp(1, u64::from(u32::MAX)) as u32;
        let health = ((max_health as f32) * restore_percent)
            .max(0.0)
            .min(max_health as f32) as u32;
        self.unit_mut().set_death_state(DeathState::Alive);
        self.unit_mut().set_health(u64::from(health));
        let mana = ((self.get_max_power(PowerType::Mana).max(0) as f32) * restore_percent)
            .max(0.0) as i32;
        let energy = ((self.get_max_power(PowerType::Energy).max(0) as f32) * restore_percent)
            .max(0.0) as i32;
        let focus = ((self.get_max_power(PowerType::Focus).max(0) as f32) * restore_percent)
            .max(0.0) as i32;
        self.unit_mut().set_power(PowerType::Mana, mana);
        self.unit_mut().set_power(PowerType::Rage, 0);
        self.unit_mut().set_power(PowerType::Energy, energy);
        self.unit_mut().set_power(PowerType::Focus, focus);
        health
    }
}

#[cfg(test)]
mod tests;
