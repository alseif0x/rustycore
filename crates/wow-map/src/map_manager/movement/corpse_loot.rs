//! Corpse loot operations of movement.
//!
//! Divided out of the single inherent impl under #705; every method keeps
//! its name, signature and body.

use super::*;

impl WorldCreature {
    pub fn all_loot_removed_from_corpse_like_cpp(
        &mut self,
        decay_rate: f32,
        is_fully_skinned: bool,
    ) -> bool {
        self.all_loot_removed_from_corpse_at_game_time_like_cpp(
            Instant::now(),
            game_time_secs_like_cpp(),
            decay_rate,
            is_fully_skinned,
        )
    }

    pub fn all_loot_removed_from_corpse_at_game_time_like_cpp(
        &mut self,
        now: Instant,
        game_time_secs: i64,
        decay_rate: f32,
        is_fully_skinned: bool,
    ) -> bool {
        let plan = self.creature.all_loot_removed_from_corpse(
            game_time_secs,
            decay_rate,
            is_fully_skinned,
        );
        if plan.is_empty() {
            return false;
        }

        // C++ stores `m_corpseRemoveTime` in the absolute GameTime domain.
        // Translate only the remaining duration into the creature's logical
        // Map-tick clock; wall time is not an ongoing runtime clock source.
        let corpse_remove_at = instant_from_respawn_time_like_cpp(
            self.creature.corpse_remove_time(),
            now,
            game_time_secs,
        );
        let remaining_ms = corpse_remove_at
            .checked_duration_since(now)
            .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);
        let corpse_remove_time_ms = self
            .runtime_elapsed_ms_like_cpp()
            .saturating_add(remaining_ms);
        self.creature
            .set_ai_corpse_despawn_at(Some(corpse_remove_time_ms));
        true
    }

    pub fn remove_lootable_dynamic_flag_like_cpp(&mut self) {
        let object = self.creature.unit_mut().world_mut().object_mut();
        object.remove_dynamic_flag(UnitDynFlags::Lootable as u32);
        object.force_dynamic_flags_update_like_cpp();
    }
}
