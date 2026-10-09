// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory storage, equip/swap, destroy, durability and item modification.

use super::*;
mod destruction;
mod equipment_sets;
mod handlers;
mod inventory_moves;
pub(super) mod login_load;

pub(crate) fn item_turnin_persistence_rows_like_cpp(
    player_guid: ObjectGuid,
    changes: &[ExtendedCostItemTurninChange],
) -> Vec<wow_persistence::VendorItemTurninPersistenceLikeCpp> {
    changes
        .iter()
        .map(|change| match *change {
            ExtendedCostItemTurninChange::Update {
                db_guid, new_count, ..
            } => wow_persistence::VendorItemTurninPersistenceLikeCpp::Update {
                item_guid: db_guid,
                new_count,
            },
            ExtendedCostItemTurninChange::Delete { db_guid, .. } => {
                wow_persistence::VendorItemTurninPersistenceLikeCpp::Delete {
                    owner_guid: player_guid.counter() as u64,
                    item_guid: db_guid,
                }
            }
        })
        .collect()
}

impl WorldSession {
    pub(super) fn creature_virtual_items_from_row_with_catalogs_like_cpp(
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
                    let index = self
                        .core
                        .represented_urand_u32_like_cpp(0, (count - 1) as u32)
                        as usize;
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
    pub(super) fn creature_virtual_items_from_row_like_cpp(
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

    pub(crate) fn inventory_container_db_guid_like_cpp(&self, bag: u8) -> Option<u64> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.inventory_container_db_guid_like_cpp(hub, bag)
    }

    pub(super) fn has_item_count_direct_inventory(&self, item_entry: u32, count: u32) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.has_item_count_direct_inventory(hub, item_entry, count)
    }

    pub(crate) fn plan_destroy_item_count_direct_inventory(
        &self,
        item_entry: u32,
        count: u32,
    ) -> Option<Vec<ExtendedCostItemTurninChange>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.plan_destroy_item_count_direct_inventory(hub, item_entry, count)
    }

    pub(crate) fn apply_item_turnin_changes(
        &mut self,
        _player_guid: ObjectGuid,
        map_id: u16,
        changes: &[ExtendedCostItemTurninChange],
    ) {
        let access = self.core.owned_inventory_access_like_cpp();
        let publication = self.core.packet_publication_access_like_cpp();
        let send_stat_update = self
            .inventory
            .apply_item_turnin_changes_with_access_like_cpp(
                &access,
                &publication,
                self.catalogs.items.store.as_ref(),
                self.catalogs.items.stats_store.as_ref(),
                _player_guid,
                map_id,
                changes,
            );
        if send_stat_update {
            self.send_stat_update();
        }
    }
}
