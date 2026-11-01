// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_guild_accept_invites_like_cpp(&self) -> &[u64] {
        self.social.represented_guild_accept_invites_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_auto_decline_guild_invites_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_social_ref(self);
        state.represented_auto_decline_guild_invites_like_cpp(hub)
    }
}

impl WorldSession {
    pub(crate) fn player_guild_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerGuildState> {
        wow_world_social::player_guild_state_snapshot_like_cpp(
            &crate::session::hub_ref(self),
            &self.social,
        )
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
}
