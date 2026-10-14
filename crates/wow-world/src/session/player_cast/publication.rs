//! Player cast publication commits recipient visibility at each Start/Go phase.
//! Delivery uses the existing bounded durable rail and its overflow disconnect.

use super::*;
use crate::session::mailbox::SendPlayerSpellIfVisibleLikeCppCommand;

impl WorldSession {
    pub(in crate::session) fn publish_player_cast_interruption_like_cpp(
        &mut self,
        cast: SpellCastState,
    ) {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.publish_player_cast_interruption_like_cpp(&mut hub, cast)
    }

    pub(crate) fn handle_player_cast_publication_like_cpp(
        &mut self,
        command: SendPlayerSpellIfVisibleLikeCppCommand,
    ) {
        let opcode = command
            .packet_bytes
            .get(..2)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u16::from_le_bytes);
        if !matches!(opcode, Some(value) if value == ServerOpcodes::SpellStart as u16
            || value == ServerOpcodes::SpellGo as u16 || value == ServerOpcodes::SpellFailure as u16
            || value == ServerOpcodes::SpellFailedOther as u16)
        {
            return;
        }
        if self.state() != SessionState::LoggedIn
            || !self
                .core
                .client_visible_guids_like_cpp
                .shares_storage_like_cpp(&command.committed_visibility_like_cpp)
            || self.core.current_canonical_player_map_key_like_cpp()
                != Some(wow_map::MapKey::new(
                    u32::from(command.map_id),
                    command.instance_id,
                ))
        {
            return;
        }
        // Honor publication-time membership; reconnect/map changes are rejected.
        self.send_raw_packet(&command.packet_bytes);
    }
}
