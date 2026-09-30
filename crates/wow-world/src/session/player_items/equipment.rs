//! Represented equipment: equip and unequip over the canonical Player.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

mod equip_spells;
mod admission;
mod valuation_and_publication;

impl WorldSession {
    /// C++ `DB2Manager::GetItemChildEquipment(parentItemId)`.
    pub(crate) fn item_child_equipment_for_parent_like_cpp(
        &self,
        parent_item_id: u32,
    ) -> Option<&ItemChildEquipmentEntry> {
        self.items
            .child_equipment_store
            .as_ref()?
            .values()
            .find(|entry| entry.parent_item_id == parent_item_id)
    }
    #[cfg(test)]
    pub(crate) fn creature_equipment_store_like_cpp(
        &self,
    ) -> Option<&Arc<CreatureEquipmentStoreLikeCpp>> {
        self.creature_equipment_store_like_cpp.as_ref()
    }
}
