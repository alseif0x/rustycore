// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

impl Player {
    /// C++ `Player::SetInGuild` (`Player.cpp:7216`) updates the Player's
    /// guild identity. The session/database layer owns cache and GuildMgr
    /// effects; this operation owns only the represented Player state.
    pub fn set_guild_id_like_cpp(&mut self, guild_id: u64) {
        let state = &mut self.gameplay_state_mut().guild;
        state.guild_id = (guild_id != 0).then_some(guild_id);
        state.authority_complete = true;
    }

    /// C++ `Player::SetGuildIdInvited` (`Player.h:1943`) writes the pending
    /// invitation kept beside the Player's guild membership.
    pub fn set_guild_id_invited_like_cpp(&mut self, guild_id: u64) {
        self.gameplay_state_mut().guild.invited_guild_id = (guild_id != 0).then_some(guild_id);
    }

    /// C++ `Player::SetGuildRank` (`Player.h:1939`) keeps the rank with the
    /// Player. Guild membership and rank publication remain application work.
    pub fn set_guild_rank_like_cpp(&mut self, rank_id: Option<u32>) {
        self.gameplay_state_mut().guild.rank_id = rank_id;
    }

    /// C++ `Player::SetGuildIdInvited(0)` clears a declined or consumed invite.
    pub fn clear_guild_invitation_like_cpp(&mut self) {
        self.gameplay_state_mut().guild.invited_guild_id = None;
    }

    /// Return the Player-owned guild snapshot without exposing a mutable
    /// composite state to the Session adapter.
    pub fn guild_state_like_cpp(&self) -> PlayerGuildState {
        self.gameplay_state().guild.clone()
    }

    pub fn set_party_type_like_cpp(&mut self, category: u8, party_type: u8) -> bool {
        let index = usize::from(category);
        if index >= self.data.party_type.len() {
            return false;
        }

        if self.data.party_type[index] != party_type {
            self.data.party_type[index] = party_type;
            self.mark_player_data_array(
                PLAYER_DATA_PARTY_TYPE_PARENT_BIT,
                PLAYER_DATA_PARTY_TYPE_FIRST_BIT,
                index,
            );
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guild_membership_and_invitation_transitions_remain_player_owned_like_cpp() {
        let mut player = Player::new(Some(7), false);

        player.set_guild_id_like_cpp(42);
        player.set_guild_rank_like_cpp(Some(3));
        player.set_guild_id_invited_like_cpp(84);
        assert_eq!(
            player.guild_state_like_cpp(),
            PlayerGuildState {
                guild_id: Some(42),
                invited_guild_id: Some(84),
                rank_id: Some(3),
                authority_complete: true,
            }
        );

        player.clear_guild_invitation_like_cpp();
        assert_eq!(player.guild_state_like_cpp().invited_guild_id, None);
    }

    #[test]
    fn zero_guild_id_clears_membership_but_keeps_resolved_authority_like_cpp() {
        let mut player = Player::new(Some(8), false);
        player.set_guild_id_like_cpp(42);
        player.set_guild_id_like_cpp(0);

        let state = player.guild_state_like_cpp();
        assert_eq!(state.guild_id, None);
        assert!(state.authority_complete);
    }
}
