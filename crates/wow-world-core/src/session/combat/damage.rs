use crate::session::RepresentedScalingStatContextLikeCpp;
use crate::session::state::SessionCore;
use wow_core::ObjectGuid;

pub const DAMAGE_FALL_LIKE_CPP: u8 = 2;
pub const DAMAGE_FALL_TO_VOID_LIKE_CPP: u8 = 6;

impl SessionCore {
    pub fn send_environmental_damage_log_like_cpp(
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
}

impl crate::session::HubMut<'_> {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_normal_damage_immune_like_cpp(&mut self, immune: bool) {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_normal_damage_immune_like_cpp(immune)
            })
            .is_some();
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.combat.player_normal_damage_immune_like_cpp = immune;
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_environmental_damage_immune_like_cpp(&mut self, immune: bool) {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_environmental_damage_immune_like_cpp(immune)
            })
            .is_some();
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .combat
                .player_environmental_damage_immune_like_cpp = immune;
        }
    }
}

impl crate::session::HubRef<'_> {
    /// Preserve `AttackerStateUpdate -> self share -> primary health` on the
    /// victim session's FIFO for C++ `SPELL_AURA_SHARE_DAMAGE_PCT`.
    pub fn publish_self_share_health_like_cpp(
        &self,
        command: &crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand,
    ) {
        for health in &command.self_share_health_updates {
            self.core
                .send_packet(&wow_packet::packets::combat::HealthUpdate {
                    guid: command.victim_guid,
                    health: (*health).min(i64::MAX as u64) as i64,
                });
        }
    }

    pub fn represented_resistances_with_scaling_armor_like_cpp(
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
}

impl crate::session::HubRef<'_> {
    pub fn resolved_player_damage_control_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerDamageControlStateLikeCpp> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.damage_control_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(wow_entities::PlayerDamageControlStateLikeCpp {
                cheat_god: self.fixtures.combat.player_cheat_god_like_cpp,
                normal_damage_immune: self.fixtures.combat.player_normal_damage_immune_like_cpp,
                environmental_damage_immune: self
                    .fixtures
                    .combat
                    .player_environmental_damage_immune_like_cpp,
            });
        }
        canonical
    }
}

impl crate::session::HubMut<'_> {
    /// Apply damage to the canonical Player owner and return
    /// `(before, after, max, applied, killed)`.
    pub fn apply_owned_player_damage_like_cpp(
        &mut self,
        requested_damage: u32,
        lethal_death_state: wow_constants::DeathState,
    ) -> Option<(u32, u32, u32, u32, bool)> {
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
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
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            let max_health = self.fixtures.combat.player_max_health_like_cpp.max(1);
            let before = self.fixtures.combat.player_health_like_cpp.min(max_health);
            if !self.fixtures.combat.player_alive_like_cpp || before == 0 {
                return Some((before, before, max_health, 0, false));
            }
            let applied = requested_damage.min(before);
            let after = before.saturating_sub(applied);
            let killed = applied > 0 && after == 0;
            self.fixtures.combat.player_health_like_cpp = after;
            self.fixtures.combat.player_alive_like_cpp = !killed;
            return Some((before, after, max_health, applied, killed));
        }
        canonical
    }
}
