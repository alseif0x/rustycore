// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadEquipmentTemplates` world-database store.

use std::collections::BTreeMap;

use anyhow::Result;
use wow_constants::InventoryType;
use wow_database::WorldDatabase;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CreatureEquipmentItemLikeCpp {
    pub item_id: u32,
    pub appearance_mod_id: u16,
    pub item_visual: u16,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CreatureEquipmentInfoLikeCpp {
    pub items: [CreatureEquipmentItemLikeCpp; 3],
}

#[derive(Debug, Clone, Default)]
pub struct CreatureEquipmentStoreLikeCpp {
    entries: BTreeMap<u32, BTreeMap<u8, CreatureEquipmentInfoLikeCpp>>,
}

fn is_hand_equipment_inventory_type_like_cpp(inventory_type: u8) -> bool {
    matches!(
        inventory_type,
        x if x == InventoryType::Weapon as u8
            || x == InventoryType::Shield as u8
            || x == InventoryType::Ranged as u8
            || x == InventoryType::Weapon2Hand as u8
            || x == InventoryType::WeaponMainhand as u8
            || x == InventoryType::WeaponOffhand as u8
            || x == InventoryType::Holdable as u8
            || x == InventoryType::Thrown as u8
            || x == InventoryType::RangedRight as u8
    )
}

impl CreatureEquipmentStoreLikeCpp {
    pub fn from_entries(
        entries: impl IntoIterator<Item = (u32, u8, CreatureEquipmentInfoLikeCpp)>,
    ) -> Self {
        let mut store = Self::default();
        for (entry, id, info) in entries {
            if id == 0 {
                continue;
            }
            store.entries.entry(entry).or_default().insert(id, info);
        }
        store
    }

    /// Mirrors C++ `ObjectMgr::LoadEquipmentTemplates`.
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        mut creature_template_exists: impl FnMut(u32) -> bool,
        mut item_inventory_type: impl FnMut(u32) -> Option<u8>,
        mut item_modified_appearance_exists: impl FnMut(u32, u32) -> bool,
        mut default_item_appearance_mod_id: impl FnMut(u32) -> Option<u16>,
    ) -> Result<Self> {
        let mut result = db
            .direct_query(
                "SELECT CreatureID, ID, ItemID1, AppearanceModID1, ItemVisual1, \
                 ItemID2, AppearanceModID2, ItemVisual2, \
                 ItemID3, AppearanceModID3, ItemVisual3 \
                 FROM creature_equip_template",
            )
            .await?;

        if result.is_empty() {
            return Ok(Self::default());
        }

        let mut entries = Vec::new();
        loop {
            let creature_id = result.try_read::<u32>(0).unwrap_or(0);
            if !creature_template_exists(creature_id) {
                if !result.next_row() {
                    break;
                }
                continue;
            }

            let id = result.try_read::<u8>(1).unwrap_or(0);
            if id == 0 {
                if !result.next_row() {
                    break;
                }
                continue;
            }

            let mut info = CreatureEquipmentInfoLikeCpp::default();
            for slot in 0..3 {
                let base = 2 + slot * 3;
                let item_id = result.try_read::<u32>(base).unwrap_or(0);
                if item_id == 0 {
                    continue;
                }

                let Some(inventory_type) = item_inventory_type(item_id) else {
                    info.items[slot].item_id = 0;
                    continue;
                };

                let mut appearance_mod_id = result.try_read::<u16>(base + 1).unwrap_or(0);
                let item_visual = result.try_read::<u16>(base + 2).unwrap_or(0);

                if !item_modified_appearance_exists(item_id, u32::from(appearance_mod_id)) {
                    appearance_mod_id = default_item_appearance_mod_id(item_id).unwrap_or(0);
                }

                if !is_hand_equipment_inventory_type_like_cpp(inventory_type) {
                    info.items[slot].item_id = 0;
                    continue;
                }

                info.items[slot] = CreatureEquipmentItemLikeCpp {
                    item_id,
                    appearance_mod_id,
                    item_visual,
                };
            }

            entries.push((creature_id, id, info));

            if !result.next_row() {
                break;
            }
        }

        Ok(Self::from_entries(entries))
    }

    pub fn get(&self, entry: u32, id: u8) -> Option<&CreatureEquipmentInfoLikeCpp> {
        self.entries.get(&entry)?.get(&id)
    }

    pub fn len_for_entry(&self, entry: u32) -> usize {
        self.entries.get(&entry).map_or(0, BTreeMap::len)
    }

    pub fn nth_for_entry(
        &self,
        entry: u32,
        index: usize,
    ) -> Option<(u8, &CreatureEquipmentInfoLikeCpp)> {
        self.entries
            .get(&entry)?
            .iter()
            .nth(index)
            .map(|(&id, info)| (id, info))
    }

    pub fn len(&self) -> usize {
        self.entries.values().map(BTreeMap::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_entries_skips_zero_id_like_cpp() {
        let store = CreatureEquipmentStoreLikeCpp::from_entries([
            (10, 0, CreatureEquipmentInfoLikeCpp::default()),
            (
                10,
                2,
                CreatureEquipmentInfoLikeCpp {
                    items: [
                        CreatureEquipmentItemLikeCpp {
                            item_id: 25,
                            appearance_mod_id: 3,
                            item_visual: 4,
                        },
                        CreatureEquipmentItemLikeCpp::default(),
                        CreatureEquipmentItemLikeCpp::default(),
                    ],
                },
            ),
        ]);

        assert!(store.get(10, 0).is_none());
        assert_eq!(store.len_for_entry(10), 1);
        assert_eq!(store.nth_for_entry(10, 0).map(|(id, _)| id), Some(2));
    }
}
