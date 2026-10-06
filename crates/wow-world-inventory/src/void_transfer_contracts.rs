// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_entities::PlayerInventoryItem as InventoryItem;

#[derive(Debug, Clone)]
pub struct PlannedVoidDestroyedInventoryItemLikeCpp {
    pub bag: u8,
    pub slot: u8,
    pub inventory_item: InventoryItem,
    pub cleared_mainhand_enchantments: Vec<wow_constants::EnchantmentSlot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveVoidStorageRandomPropertiesLikeCpp {
    pub id: i32,
    pub seed: i32,
    pub enchantment_ids: [i32; wow_entities::MAX_ENCHANTMENT_SLOT],
}

impl Default for EffectiveVoidStorageRandomPropertiesLikeCpp {
    fn default() -> Self {
        Self {
            id: 0,
            seed: 0,
            enchantment_ids: [0; wow_entities::MAX_ENCHANTMENT_SLOT],
        }
    }
}
