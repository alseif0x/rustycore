//! Represented guild membership and its published state.
//!
//! Moved out of the Session root under #615. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    fn player_guild_state_snapshot_like_cpp(&self) -> Option<wow_entities::PlayerGuildState> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.gameplay_state().guild.clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(wow_entities::PlayerGuildState {
                guild_id: (self.represented_guild_id_like_cpp != 0)
                    .then_some(self.represented_guild_id_like_cpp),
                invited_guild_id: (self.represented_guild_id_invited_like_cpp != 0)
                    .then_some(self.represented_guild_id_invited_like_cpp),
                rank_id: None,
                authority_complete: self.represented_guild_id_authority_complete_like_cpp,
            });
        }
        canonical
    }
    fn mutate_player_guild_state_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerGuildState) -> R,
    ) -> Option<R> {
        let mut state = self.player_guild_state_snapshot_like_cpp()?;
        let result = f(&mut state);
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.gameplay_state_mut().guild = state.clone()
            })
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_guild_id_like_cpp = state.guild_id.unwrap_or(0);
            self.represented_guild_id_invited_like_cpp = state.invited_guild_id.unwrap_or(0);
            self.represented_guild_id_authority_complete_like_cpp = state.authority_complete;
            return Some(result);
        }
        canonical.then_some(result)
    }
    pub(crate) fn set_represented_guild_id_like_cpp(&mut self, guild_id: u64) -> bool {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.mutate_player_guild_state_like_cpp(|state| {
            state.guild_id = (guild_id != 0).then_some(guild_id);
            state.authority_complete = true;
        })
        .is_some()
    }
    pub(crate) fn resolved_represented_guild_id_like_cpp(&self) -> Option<u64> {
        let state = self.player_guild_state_snapshot_like_cpp()?;
        state
            .authority_complete
            .then_some(state.guild_id.unwrap_or(0))
    }
    #[cfg(test)]
    pub(crate) fn represented_guild_id_like_cpp(&self) -> u64 {
        self.resolved_represented_guild_id_like_cpp()
            .expect("test Player guild owner must resolve")
    }
    #[cfg(test)]
    pub(crate) fn set_represented_guild_id_invited_like_cpp(&mut self, guild_id: u64) -> bool {
        self.mutate_player_guild_state_like_cpp(|state| {
            state.invited_guild_id = (guild_id != 0).then_some(guild_id);
        })
        .is_some()
    }
    #[cfg(test)]
    pub(crate) fn represented_guild_id_invited_like_cpp(&self) -> u64 {
        self.player_guild_state_snapshot_like_cpp()
            .and_then(|state| state.invited_guild_id)
            .unwrap_or(0)
    }
    pub(crate) fn accept_guild_invitation_like_cpp(&mut self) -> bool {
        let Some(state) = self.player_guild_state_snapshot_like_cpp() else {
            return false;
        };
        if !state.authority_complete || state.guild_id.is_some() {
            return false;
        }

        let Some(guild_id) = state.invited_guild_id else {
            return false;
        };
        #[cfg(not(test))]
        let _ = guild_id;

        #[cfg(test)]
        self.represented_guild_accept_invites_like_cpp
            .push(guild_id);
        true
    }
    #[cfg(test)]
    pub(crate) fn represented_guild_accept_invites_like_cpp(&self) -> &[u64] {
        &self.represented_guild_accept_invites_like_cpp
    }
    pub(crate) fn decline_guild_invitation_like_cpp(&mut self) -> bool {
        let Some(state) = self.player_guild_state_snapshot_like_cpp() else {
            return false;
        };
        if !state.authority_complete || state.guild_id.is_some() {
            return false;
        }

        self.mutate_player_guild_state_like_cpp(|state| state.invited_guild_id = None)
            .is_some()
    }
    pub(crate) fn represented_set_auto_decline_guild_invites_like_cpp(
        &mut self,
        allow: bool,
    ) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };

        let changed = self
            .mutate_canonical_player_like_cpp(|player| {
                if allow {
                    player.set_player_flag(PLAYER_FLAGS_AUTO_DECLINE_GUILD_LIKE_CPP);
                } else {
                    player.remove_player_flag(PLAYER_FLAGS_AUTO_DECLINE_GUILD_LIKE_CPP);
                }
            })
            .is_some();

        if changed {
            self.sync_player_registry_state_like_cpp();
        }

        self.canonical_player_has_player_flag_like_cpp(
            guid,
            PLAYER_FLAGS_AUTO_DECLINE_GUILD_LIKE_CPP,
        )
        .unwrap_or(false)
            == allow
    }
    #[cfg(test)]
    pub(crate) fn represented_auto_decline_guild_invites_like_cpp(&self) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };

        self.canonical_player_has_player_flag_like_cpp(
            guid,
            PLAYER_FLAGS_AUTO_DECLINE_GUILD_LIKE_CPP,
        )
        .unwrap_or(false)
    }
}
