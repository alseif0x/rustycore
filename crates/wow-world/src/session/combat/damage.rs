//! Represented damage application, absorption and resistance.
//!
//! Moved out of the Session root under #617. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_weapon_damage_bounds_like_cpp(
        &self,
        item_entry: u32,
        weapon: &wow_data::ItemWeaponTemplateEntry,
    ) -> (f32, f32) {
        let mut min_damage = f32::from(weapon.min_damage[0]);
        let mut max_damage = f32::from(weapon.max_damage[0]);
        let Some(context) = self.represented_scaling_stat_context_like_cpp(item_entry) else {
            return (min_damage, max_damage);
        };

        if context.dps_mod != 0 {
            let average = context.dps_mod as f32 * f32::from(weapon.item_delay) / 1000.0;
            let modifier = if context.is_two_hand { 0.2 } else { 0.3 };
            min_damage = (1.0 - modifier) * average;
            max_damage = (1.0 + modifier) * average;
        }

        (min_damage, max_damage)
    }
    pub(in crate::session) fn represented_resistances_with_scaling_armor_like_cpp(
        &self,
        resistances: &[i16; 7],
        scaling_context: Option<RepresentedScalingStatContextLikeCpp>,
    ) -> [i16; 7] {
        let mut adjusted = *resistances;
        if let Some(context) = scaling_context {
            if context.armor_mod > 0 {
                adjusted[0] = i16::try_from(context.armor_mod).unwrap_or(i16::MAX);
            } else if context.armor_mod < 0 {
                adjusted[0] = i16::MIN;
            }
        }
        adjusted
    }
    pub(in crate::session) fn send_environmental_damage_log_like_cpp(
        &self,
        victim: ObjectGuid,
        damage_type: u8,
        amount: u32,
        resisted: u32,
        absorbed: u32,
    ) {
        self.send_packet(&wow_packet::packets::combat::EnvironmentalDamageLog {
            victim,
            damage_type: if damage_type == DAMAGE_FALL_TO_VOID_LIKE_CPP {
                DAMAGE_FALL_LIKE_CPP
            } else {
                damage_type
            },
            amount: amount.min(i32::MAX as u32) as i32,
            resisted: resisted.min(i32::MAX as u32) as i32,
            absorbed: absorbed.min(i32::MAX as u32) as i32,
        });
    }
    /// Apply damage to the canonical Player owner and return
    /// `(before, after, max, applied, killed)`.
    pub(in crate::session) fn apply_owned_player_damage_like_cpp(
        &mut self,
        requested_damage: u32,
        lethal_death_state: wow_constants::DeathState,
    ) -> Option<(u32, u32, u32, u32, bool)> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            let max_health = player
                .unit()
                .data()
                .max_health
                .clamp(1, u64::from(u32::MAX)) as u32;
            let before = player.unit().data().health.min(u64::from(max_health)) as u32;
            if !player.unit().is_alive() || before == 0 {
                return (before, before, max_health, 0, false);
            }
            let applied = requested_damage.min(before);
            let after = before.saturating_sub(applied);
            let killed = applied > 0 && after == 0;
            if killed {
                player.unit_mut().set_death_state(lethal_death_state);
            }
            player.unit_mut().set_health(u64::from(after));
            (before, after, max_health, applied, killed)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            let max_health = self.player_max_health_like_cpp.max(1);
            let before = self.player_health_like_cpp.min(max_health);
            if !self.player_alive_like_cpp || before == 0 {
                return Some((before, before, max_health, 0, false));
            }
            let applied = requested_damage.min(before);
            let after = before.saturating_sub(applied);
            let killed = applied > 0 && after == 0;
            self.player_health_like_cpp = after;
            self.player_alive_like_cpp = !killed;
            return Some((before, after, max_health, applied, killed));
        }
        canonical
    }
    #[cfg(test)]
    pub(crate) fn set_player_normal_damage_immune_like_cpp(&mut self, immune: bool) {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_normal_damage_immune_like_cpp(immune)
            })
            .is_some();
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_normal_damage_immune_like_cpp = immune;
        }
    }
    #[cfg(test)]
    pub(crate) fn set_player_environmental_damage_immune_like_cpp(&mut self, immune: bool) {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_environmental_damage_immune_like_cpp(immune)
            })
            .is_some();
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_environmental_damage_immune_like_cpp = immune;
        }
    }
    pub(in crate::session) fn resolved_player_damage_control_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerDamageControlStateLikeCpp> {
        let canonical = self.with_owned_player_like_cpp(|player| player.damage_control_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(wow_entities::PlayerDamageControlStateLikeCpp {
                cheat_god: self.player_cheat_god_like_cpp,
                normal_damage_immune: self.player_normal_damage_immune_like_cpp,
                environmental_damage_immune: self.player_environmental_damage_immune_like_cpp,
            });
        }
        canonical
    }
    pub(crate) fn set_player_health_after_runtime_damage_like_cpp(&mut self, health_after: u64) {
        let Some((_, max_health, _)) = self.resolved_player_vitals_like_cpp() else {
            return;
        };
        let _ = self.sync_canonical_player_health_like_cpp(
            health_after.min(u64::from(max_health)) as u32,
            max_health,
        );
        self.sync_player_registry_state_like_cpp();
    }
}
