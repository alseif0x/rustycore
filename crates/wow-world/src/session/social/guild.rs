//! Represented guild membership and its published state.
//!
//! Moved out of the Session root under #615. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn resolved_represented_guild_id_like_cpp(&self) -> Option<u64> {
        wow_world_social::resolved_represented_guild_id_like_cpp(
            &crate::session::hub_ref(self),
            &self.social,
        )
    }
    #[cfg(test)]
    fn mutate_player_guild_state_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerGuildState) -> R,
    ) -> Option<R> {
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::mutate_player_guild_state_for_test_like_cpp(&hub.shared(), social, f)
    }
    #[cfg(test)]
    pub(crate) fn represented_guild_id_invited_like_cpp(&self) -> u64 {
        self.player_guild_state_snapshot_like_cpp()
            .and_then(|state| state.invited_guild_id)
            .unwrap_or(0)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/social/guild/f3_shims.rs"]
mod f3_shims;
