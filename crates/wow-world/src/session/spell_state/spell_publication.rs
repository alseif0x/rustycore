//! Represented spell state published to other systems: trade spells, the
//! money-coupled acquisition commit and visible spell-click refreshes.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged. The spell-pull attack start moved to
//! `wow_world_spell` (#1363 slice 12b3).

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn set_represented_trade_spell_like_cpp_for_test(
        &mut self,
        spell_id: u32,
        cast_item_guid: Option<ObjectGuid>,
    ) {
        let _ = self.mutate_player_trade_state_like_cpp(|state| {
            if let Some(state) = state {
                state.spell_id = spell_id;
                state.spell_cast_item_guid = cast_item_guid;
            }
        });
    }
    #[cfg(test)]
    pub(crate) fn represented_trade_spell_like_cpp(&self) -> u32 {
        self.player_trade_state_snapshot_like_cpp()
            .flatten()
            .map_or(0, |state| state.spell_id)
    }
}
