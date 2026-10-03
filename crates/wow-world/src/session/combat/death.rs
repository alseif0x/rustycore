//! Represented death, corpse and resurrection handling.
//!
//! Moved out of the Session root under #617. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    fn apply_represented_player_environmental_death_like_cpp(&mut self) {
        // C++ `Player::EnvironmentalDamage` routes lethal damage through
        // `Unit::Kill` -> `Player::setDeathState(JUST_DIED)` before the client
        // proceeds into release/cemetery flows.
        let _ = self.core.with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::JustDied);
            player.unit_mut().set_health(0);
        });
        #[cfg(test)]
        {
            self.fixtures.combat.player_health_like_cpp = 0;
            self.fixtures.combat.player_alive_like_cpp = false;
        }
        self.sync_player_registry_state_like_cpp();
    }
    pub(in crate::session) fn player_resurrection_state_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerResurrectionStateLikeCpp> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.resurrection_state_like_cpp().clone());
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(PlayerResurrectionStateLikeCpp {
                request: self
                    .fixtures
                    .combat
                    .represented_resurrection_request_like_cpp,
                delayed_after_teleport: self
                    .fixtures
                    .combat
                    .represented_delayed_resurrection_after_teleport_like_cpp,
                self_res_spells: self
                    .spell_state
                    .represented_self_res_spells_for_test_like_cpp()
                    .clone(),
                death_timer_active: self.fixtures.combat.represented_death_timer_active_like_cpp,
                area_spirit_healer_guid: self.fixtures.combat.area_spirit_healer_guid_like_cpp,
            });
        }
        canonical
    }
}
