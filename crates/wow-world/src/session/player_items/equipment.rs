//! Represented equipment: equip and unequip over the canonical Player.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(in crate::session) fn apply_initial_item_equip_auras_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> usize {
        self.player_aura_application_cx_like_cpp()
            .apply_initial_item_equip_auras_like_cpp(item_guid)
    }
    pub(crate) fn record_represented_avg_equipped_item_level_update_like_cpp(&mut self) {
        #[cfg(test)]
        {
            let Some(avg_equipped_item_level) = self.represented_avg_equipped_item_level_like_cpp()
            else {
                return;
            };
            self.inventory
                .record_represented_avg_equipped_item_level_update_for_test_like_cpp(
                    avg_equipped_item_level,
                );
        }
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/equipment/f3_shims.rs"]
mod f3_shims;
