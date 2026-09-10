// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadEquipmentTemplates` world-database store.

use std::collections::BTreeMap;

use wow_constants::InventoryType;

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CreatureEquipmentRowLikeCpp {
    pub creature_id: u32,
    pub id: u8,
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

    /// Pure C++ `ObjectMgr::LoadEquipmentTemplates` validation/composition.
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = CreatureEquipmentRowLikeCpp>,
        mut creature_template_exists: impl FnMut(u32) -> bool,
        mut item_inventory_type: impl FnMut(u32) -> Option<u8>,
        mut item_modified_appearance_exists: impl FnMut(u32, u32) -> bool,
        mut default_item_appearance_mod_id: impl FnMut(u32) -> Option<u16>,
    ) -> Self {
        let mut entries = Vec::new();
        for row in rows {
            if !creature_template_exists(row.creature_id) || row.id == 0 {
                continue;
            }
            let mut info = CreatureEquipmentInfoLikeCpp::default();
            for slot in 0..3 {
                let item = row.items[slot];
                let item_id = item.item_id;
                if item_id == 0 {
                    continue;
                }

                let Some(inventory_type) = item_inventory_type(item_id) else {
                    info.items[slot].item_id = 0;
                    continue;
                };

                let mut appearance_mod_id = item.appearance_mod_id;
                let item_visual = item.item_visual;

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

            entries.push((row.creature_id, row.id, info));
        }
        Self::from_entries(entries)
    }

    pub fn get(&self, entry: u32, id: u8) -> Option<&CreatureEquipmentInfoLikeCpp> {
        self.entries.get(&entry)?.get(&id)
    }

    /// Mirrors C++ `ObjectMgr::GetEquipmentInfo(entry, int8& id)`.
    ///
    /// `id == -1` selects a random equipment row and mutates `id` to the selected
    /// one-based equipment template id. Other signed ids are looked up through the
    /// C++ `uint8` key domain; the caller keeps the original signed id unless the
    /// lookup fails and its own load path normalizes it.
    pub fn get_equipment_info_like_cpp(
        &self,
        entry: u32,
        id: &mut i8,
        mut urand_inclusive: impl FnMut(u32, u32) -> u32,
    ) -> Option<&CreatureEquipmentInfoLikeCpp> {
        let equipment = self.entries.get(&entry)?;
        if equipment.is_empty() {
            return None;
        }

        if *id == -1 {
            let max = u32::try_from(equipment.len().saturating_sub(1)).ok()?;
            let index = usize::try_from(urand_inclusive(0, max).min(max)).ok()?;
            let (selected_id, info) = equipment.iter().nth(index)?;
            *id = i8::try_from(*selected_id).ok()?;
            return Some(info);
        }

        equipment.get(&(*id as u8))
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

    #[test]
    fn get_equipment_info_random_mutates_id_like_cpp() {
        let store = CreatureEquipmentStoreLikeCpp::from_entries([
            (
                10,
                1,
                CreatureEquipmentInfoLikeCpp {
                    items: [
                        CreatureEquipmentItemLikeCpp {
                            item_id: 100,
                            appearance_mod_id: 1,
                            item_visual: 2,
                        },
                        CreatureEquipmentItemLikeCpp::default(),
                        CreatureEquipmentItemLikeCpp::default(),
                    ],
                },
            ),
            (
                10,
                3,
                CreatureEquipmentInfoLikeCpp {
                    items: [
                        CreatureEquipmentItemLikeCpp {
                            item_id: 300,
                            appearance_mod_id: 3,
                            item_visual: 4,
                        },
                        CreatureEquipmentItemLikeCpp::default(),
                        CreatureEquipmentItemLikeCpp::default(),
                    ],
                },
            ),
        ]);
        let mut id = -1;

        let info = store
            .get_equipment_info_like_cpp(10, &mut id, |_min, max| max)
            .expect("random equipment should select existing row");

        assert_eq!(id, 3);
        assert_eq!(info.items[0].item_id, 300);
    }

    #[test]
    fn get_equipment_info_missing_does_not_mutate_non_random_id_like_cpp() {
        let store = CreatureEquipmentStoreLikeCpp::from_entries([(
            10,
            1,
            CreatureEquipmentInfoLikeCpp::default(),
        )]);
        let mut id = 2;

        assert!(
            store
                .get_equipment_info_like_cpp(10, &mut id, |_, _| unreachable!())
                .is_none()
        );
        assert_eq!(id, 2);
    }
}
