//! Canonical resource checks and consumption for represented cast execution.

use super::*;
use wow_packet::packets::spell::SpellCastVisual;

impl WorldSession {
    pub(crate) fn take_spell_power_like_cpp(
        &mut self,
        spell_info: &wow_data::SpellInfo,
        cast_id: ObjectGuid,
        spell_id: i32,
        visual: &SpellCastVisual,
    ) -> bool {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.take_spell_power_like_cpp(&mut hub, spell_info, cast_id, spell_id, visual)
    }
}
