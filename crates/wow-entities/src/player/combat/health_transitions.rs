use super::Player;
use wow_constants::DeathState;

impl Player {
    /// Apply the represented snapshot in death-state, maximum, health order.
    ///
    /// Uses the existing Unit setters (`Unit.cpp:9214-9267`); this retains the
    /// current Rust revival policy and setter gaps, including positive health
    /// leaving `Dead` unchanged. It does not implement the full death lifecycle.
    pub fn apply_represented_health_snapshot(
        &mut self,
        health: u32,
        max_health: u32,
    ) -> (u32, u32) {
        let max_health = max_health.max(1);
        let health = health.min(max_health);
        if health == 0 {
            self.unit_mut().set_death_state(DeathState::Corpse);
        } else if matches!(
            self.unit().death_state(),
            DeathState::JustDied | DeathState::Corpse
        ) {
            self.unit_mut().set_death_state(DeathState::Alive);
        }
        self.unit_mut().set_max_health(u64::from(max_health));
        self.unit_mut().set_health(u64::from(health));
        (
            self.unit().data().health.min(u64::from(u32::MAX)) as u32,
            self.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
        )
    }

    /// Apply represented damage and return `(before, after, max, applied, killed)`.
    ///
    /// Retains the current admission, clamping and requested death-state write
    /// before `SetHealth`. This is the represented health transition, not the
    /// complete `DealDamage`/`Kill` gameplay operation (`Unit.cpp:1016-1022`;
    /// `ModifyHealth:8115-8145`, `SetHealth:9214-9241`).
    pub fn apply_represented_damage(
        &mut self,
        requested_damage: u32,
        lethal_death_state: DeathState,
    ) -> (u32, u32, u32, u32, bool) {
        let max_health = self.unit().data().max_health.clamp(1, u64::from(u32::MAX)) as u32;
        let before = self.unit().data().health.min(u64::from(max_health)) as u32;
        if !self.unit().is_alive() || before == 0 {
            return (before, before, max_health, 0, false);
        }
        let applied = requested_damage.min(before);
        let after = before.saturating_sub(applied);
        let killed = applied > 0 && after == 0;
        if killed {
            self.unit_mut().set_death_state(lethal_death_state);
        }
        self.unit_mut().set_health(u64::from(after));
        (before, after, max_health, applied, killed)
    }
}

#[cfg(test)]
mod tests;
