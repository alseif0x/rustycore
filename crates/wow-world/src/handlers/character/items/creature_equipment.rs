//! creature equipment for the existing items owner.

use super::*;

impl WorldSession {
    pub(in crate::handlers::character) fn creature_virtual_items_from_row_with_catalogs_like_cpp(
        &mut self,
        catalogs: &CreatureSpawnCatalogsLikeCpp,
        entry: u32,
        persisted_equipment_id: i16,
    ) -> CreatureEquipmentCreateFieldsLikeCpp {
        let mut equipment_id = persisted_equipment_id;
        let original_equipment_id = i8::try_from(equipment_id).unwrap_or(0);
        if equipment_id == 0 {
            return CreatureEquipmentCreateFieldsLikeCpp {
                selected_equipment_id: 0,
                original_equipment_id: 0,
                virtual_items: [(0, 0, 0); 3],
            };
        }

        {
            let store = &catalogs.equipment;
            let equipment = if equipment_id == -1 {
                let count = store.len_for_entry(entry);
                if count == 0 {
                    None
                } else {
                    let index = self.represented_urand_u32_like_cpp(0, (count - 1) as u32) as usize;
                    store.nth_for_entry(entry, index).map(|(id, info)| {
                        equipment_id = i16::from(id);
                        info
                    })
                }
            } else {
                u8::try_from(equipment_id)
                    .ok()
                    .and_then(|id| store.get(entry, id))
            };

            if let Some(equipment) = equipment {
                let selected_equipment_id = u8::try_from(equipment_id).unwrap_or(0);
                return CreatureEquipmentCreateFieldsLikeCpp {
                    selected_equipment_id,
                    original_equipment_id,
                    virtual_items: equipment.items.map(|item| {
                        (
                            i32::try_from(item.item_id).unwrap_or(0),
                            item.appearance_mod_id,
                            item.item_visual,
                        )
                    }),
                };
            }
        }

        CreatureEquipmentCreateFieldsLikeCpp {
            selected_equipment_id: 0,
            original_equipment_id: 0,
            virtual_items: [(0, 0, 0); 3],
        }
    }

    #[cfg(test)]
    pub(in crate::handlers::character) fn creature_virtual_items_from_row_like_cpp(
        &mut self,
        entry: u32,
        persisted_equipment_id: i16,
    ) -> CreatureEquipmentCreateFieldsLikeCpp {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.creature_virtual_items_from_row_with_catalogs_like_cpp(
            &catalogs,
            entry,
            persisted_equipment_id,
        )
    }
}
