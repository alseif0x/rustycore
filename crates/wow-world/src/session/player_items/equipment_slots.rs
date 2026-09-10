//! Equipment slot resolution and the stats an equipped item contributes.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
        &self,
        slot: u8,
        equipped: &SpellEquippedItemsEntry,
    ) -> bool {
        self.resolved_inventory_item_objects_like_cpp()
            .is_some_and(|items| {
                items
                    .values()
                    .find(|item| item.container_guid().is_empty() && item.slot() == slot)
                    .is_some_and(|item| {
                        self.represented_item_fits_spell_requirements_like_cpp(
                            item.object().entry(),
                            equipped,
                        )
                    })
            })
    }
}
