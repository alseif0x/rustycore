//! Player cast publication commits recipient visibility at each Start/Go phase.
//! Delivery uses the existing bounded durable rail and its overflow disconnect.

use super::*;

impl WorldSession {
    pub(in crate::session) fn publish_player_cast_interruption_like_cpp(
        &mut self,
        cast: SpellCastState,
    ) {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.publish_player_cast_interruption_like_cpp(&mut hub, cast)
    }
}
