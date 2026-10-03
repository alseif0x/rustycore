//! Represented guild fixtures and flag queries owned by Social.

use crate::SessionSocialLimits;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::PLAYER_FLAGS_AUTO_DECLINE_GUILD_LIKE_CPP;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::HubRef;

impl SessionSocialLimits {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_guild_state_for_test_like_cpp(&self) -> wow_entities::PlayerGuildState {
        wow_entities::PlayerGuildState {
            guild_id: (self
                .guild_test_fixture_like_cpp
                .represented_guild_id_like_cpp
                != 0)
                .then_some(
                    self.guild_test_fixture_like_cpp
                        .represented_guild_id_like_cpp,
                ),
            invited_guild_id: (self
                .guild_test_fixture_like_cpp
                .represented_guild_id_invited_like_cpp
                != 0)
                .then_some(
                    self.guild_test_fixture_like_cpp
                        .represented_guild_id_invited_like_cpp,
                ),
            rank_id: None,
            authority_complete: self
                .guild_test_fixture_like_cpp
                .represented_guild_id_authority_complete_like_cpp,
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn mutate_represented_guild_state_for_test_like_cpp<R>(
        &mut self,
        mut state: wow_entities::PlayerGuildState,
        mutate: impl FnOnce(&mut wow_entities::PlayerGuildState) -> R,
    ) -> R {
        let result = mutate(&mut state);
        self.guild_test_fixture_like_cpp.represented_guild_id_like_cpp =
            state.guild_id.unwrap_or(0);
        self.guild_test_fixture_like_cpp.represented_guild_id_invited_like_cpp = state
            .invited_guild_id
            .unwrap_or(0);
        self.guild_test_fixture_like_cpp
            .represented_guild_id_authority_complete_like_cpp = state.authority_complete;
        result
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_guild_accept_invite_for_test_like_cpp(&mut self, guild_id: u64) {
        self.guild_test_fixture_like_cpp
            .represented_guild_accept_invites_like_cpp
            .push(guild_id);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_guild_accept_invites_like_cpp(&self) -> &[u64] {
        &self
            .guild_test_fixture_like_cpp
            .represented_guild_accept_invites_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_auto_decline_guild_invites_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> bool {
        let Some(guid) = hub.core.player_guid() else {
            return false;
        };

        hub.core
            .canonical_player_has_player_flag_like_cpp(
                guid,
                PLAYER_FLAGS_AUTO_DECLINE_GUILD_LIKE_CPP,
            )
            .unwrap_or(false)
    }
}
