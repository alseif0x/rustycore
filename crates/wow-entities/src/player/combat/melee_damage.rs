//! Ordered melee damage application to the canonical Player victim.

use super::Player;

impl Player {
    /// Apply the caller's resolved melee damages in their original order.
    ///
    /// Returns `(damage, over_damage)` for every input and the victim's initial
    /// level clamped to a byte. A non-alive victim returns `None` before the
    /// iterator is consumed; an alive victim accepts an empty batch.
    /// Zero damage leaves health untouched. Positive damage uses the existing
    /// Unit health setter, with exact lethal damage reporting zero over-damage.
    ///
    /// Source boundaries: `a5f8da2e`, `Unit.cpp:1468-1510`
    /// (`DealMeleeDamage`) and `5463-5472` (`SendAttackStateUpdate`).
    /// The represented operation retains one admission check before the batch,
    /// continues after zero health and does not change death state. It does not
    /// add the remaining C++ damage stages, including parry haste, scripts,
    /// duel/death prevention, Kill, aura interrupts or procs. The caller retains
    /// its existing publication after mutation, whereas C++ sends the attack
    /// log before DealMeleeDamage (`Unit.cpp:2223-2229`). Victim resolution and
    /// locks also remain with the caller.
    pub fn apply_melee_damage_batch(
        &mut self,
        damages: impl Iterator<Item = u32>,
    ) -> Option<(Vec<(u32, i32)>, u8)> {
        if !self.unit().is_alive() {
            return None;
        }
        let target_level = self.unit().data().level.clamp(0, i32::from(u8::MAX)) as u8;
        let mut sent_swings = Vec::new();
        for damage in damages {
            // A resolved zero-damage swing produces a log entry without a health write.
            if damage == 0 {
                sent_swings.push((0, -1));
                continue;
            }
            let health_before = self.unit().data().health;
            let health_after = health_before.saturating_sub(u64::from(damage));
            self.unit_mut().set_health(health_after);
            let over_damage = if health_after == 0 {
                u64::from(damage).saturating_sub(health_before) as i32
            } else {
                -1
            };
            sent_swings.push((damage, over_damage));
        }
        Some((sent_swings, target_level))
    }
}

#[cfg(test)]
mod tests;
