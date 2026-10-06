//! Represented guild membership and its published state.
//!
//! Moved out of the Session root under #615. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    fn player_guild_state_snapshot_like_cpp(&self) -> Option<wow_entities::PlayerGuildState> {
        wow_world_social::player_guild_state_snapshot_like_cpp(
            &crate::session::hub_ref(self),
            &self.social,
        )
    }
    pub(crate) fn set_represented_guild_id_like_cpp(&mut self, guild_id: u64) -> bool {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_guild_id_like_cpp(guild_id))
            .is_some();
        #[cfg(test)]
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_guild_state_like_cpp(|state| {
                    state.guild_id = (guild_id != 0).then_some(guild_id);
                    state.authority_complete = true;
                })
                .is_some();
        }
        canonical
    }
    pub(crate) fn resolved_represented_guild_id_like_cpp(&self) -> Option<u64> {
        wow_world_social::resolved_represented_guild_id_like_cpp(
            &crate::session::hub_ref(self),
            &self.social,
        )
    }
    #[cfg(test)]
    pub(crate) fn represented_guild_id_like_cpp(&self) -> u64 {
        self.resolved_represented_guild_id_like_cpp()
            .expect("test Player guild owner must resolve")
    }
    #[cfg(test)]
    fn mutate_player_guild_state_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerGuildState) -> R,
    ) -> Option<R> {
        let (social, hub) = crate::session::split_social_mut(self);
        wow_world_social::mutate_player_guild_state_for_test_like_cpp(&hub.shared(), social, f)
    }
    pub(crate) fn set_represented_guild_id_invited_like_cpp(&mut self, guild_id: u64) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_guild_id_invited_like_cpp(guild_id))
            .is_some();
        #[cfg(test)]
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_guild_state_like_cpp(|state| {
                    state.invited_guild_id = (guild_id != 0).then_some(guild_id);
                })
                .is_some();
        }
        canonical
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
