//! spell requirements for the existing items owner.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_has_item_fit_to_spell_requirements_like_cpp(
        &self,
        equipped: &SpellEquippedItemsEntry,
    ) -> bool {
        const SPELL_ATTR8_REQUIRES_EQUIPPED_INV_TYPES_LIKE_CPP: u32 = 0x0010_0000;

        if equipped.equipped_item_class < 0 {
            return true;
        }

        match equipped.equipped_item_class {
            class if class == ItemClass::Weapon as i8 => {
                self.represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
                    EQUIPMENT_SLOT_MAINHAND,
                    equipped,
                ) || self.represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
                    EQUIPMENT_SLOT_OFFHAND,
                    equipped,
                )
            }
            class if class == ItemClass::Armor as i8 => {
                if self
                    .spell_catalogs
                    .spell_store
                    .as_ref()
                    .is_some_and(|store| {
                        store.has_attribute8_like_cpp(
                            equipped.spell_id,
                            SPELL_ATTR8_REQUIRES_EQUIPPED_INV_TYPES_LIKE_CPP,
                        )
                    })
                {
                    [
                        EQUIPMENT_SLOT_HEAD,
                        EQUIPMENT_SLOT_SHOULDERS,
                        EQUIPMENT_SLOT_CHEST,
                        EQUIPMENT_SLOT_WAIST,
                        EQUIPMENT_SLOT_LEGS,
                        EQUIPMENT_SLOT_FEET,
                        EQUIPMENT_SLOT_WRISTS,
                        EQUIPMENT_SLOT_HANDS,
                    ]
                    .into_iter()
                    .all(|slot| {
                        self.represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
                            slot, equipped,
                        )
                    })
                } else {
                    self.represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
                        EQUIPMENT_SLOT_OFFHAND,
                        equipped,
                    ) || (EQUIPMENT_SLOT_HEAD..EQUIPMENT_SLOT_MAINHAND).any(|slot| {
                        self.represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
                            slot, equipped,
                        )
                    })
                }
            }
            _ => false,
        }
    }
    pub(in crate::session) fn represented_item_fits_spell_requirements_like_cpp(
        &self,
        item_id: u32,
        equipped: &SpellEquippedItemsEntry,
    ) -> bool {
        let Some(item) = self
            .items
            .store
            .as_ref()
            .and_then(|store| store.get(item_id))
        else {
            return false;
        };

        if equipped.equipped_item_class >= 0 {
            if equipped.equipped_item_class != item.class_id as i8 {
                return false;
            }

            if equipped.equipped_item_subclass != 0 {
                let subclass = u32::from(item.subclass_id);
                if subclass >= i32::BITS
                    || (equipped.equipped_item_subclass & (1_i32 << subclass)) == 0
                {
                    return false;
                }
            }
        }

        true
    }
}
