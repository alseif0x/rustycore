// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handle-less Player item state and effect evidence used by Session tests.

use crate::{
    RepresentedCombatStatRecalculationLikeCpp, RepresentedItemBonusActionLikeCpp,
    RepresentedItemModsReapplyEventLikeCpp, RepresentedItemSetAuraRefreshEventLikeCpp,
    RepresentedItemSetSpellEventLikeCpp,
};
use std::collections::HashMap;
use wow_entities::{
    BUYBACK_SLOT_COUNT, BUYBACK_SLOT_START, INVENTORY_DEFAULT_SIZE,
    PlayerInventoryItem as InventoryItem, PlayerItemModifierRuntimeStateLikeCpp,
    TitanGripPenaltyAction,
};

pub struct PlayerItemTestFixtureLikeCpp {
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    pub(crate) player_bank_bag_slot_count_like_cpp: u8,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    pub(crate) player_inventory_slot_count_like_cpp: u8,
    /// In-memory inventory: slot → (item ObjectGuid, entry_id, db_guid).
    pub(crate) inventory_items: HashMap<u8, InventoryItem>,
    /// In-memory buyback slots, kept separate from normal inventory like C++ `GetItemByGuid`.
    pub(crate) buyback_items: HashMap<u8, InventoryItem>,
    pub(crate) buyback_price: [u32; BUYBACK_SLOT_COUNT],
    pub(crate) buyback_timestamp: [i64; BUYBACK_SLOT_COUNT],
    pub(crate) current_buyback_slot: u8,
    pub(crate) represented_item_mod_reapply_events_like_cpp:
        Vec<RepresentedItemModsReapplyEventLikeCpp>,
    pub(crate) represented_item_bonus_actions_like_cpp: Vec<RepresentedItemBonusActionLikeCpp>,
    pub(crate) represented_item_modifier_runtime_like_cpp: PlayerItemModifierRuntimeStateLikeCpp,
    pub(crate) represented_item_set_spell_events_like_cpp:
        Vec<RepresentedItemSetSpellEventLikeCpp>,
    pub(crate) represented_item_set_aura_refresh_events_like_cpp:
        Vec<RepresentedItemSetAuraRefreshEventLikeCpp>,
    pub(crate) represented_combat_stat_recalculations_like_cpp:
        Vec<RepresentedCombatStatRecalculationLikeCpp>,
    pub(crate) represented_titan_grip_penalty_actions_like_cpp: Vec<TitanGripPenaltyAction>,
    pub(crate) represented_avg_equipped_item_level_updates_like_cpp: Vec<f32>,
}

impl Default for PlayerItemTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            player_bank_bag_slot_count_like_cpp: 0,
            player_inventory_slot_count_like_cpp: INVENTORY_DEFAULT_SIZE,
            inventory_items: HashMap::new(),
            buyback_items: HashMap::new(),
            buyback_price: [0; BUYBACK_SLOT_COUNT],
            buyback_timestamp: [0; BUYBACK_SLOT_COUNT],
            current_buyback_slot: BUYBACK_SLOT_START,
            represented_item_mod_reapply_events_like_cpp: Vec::new(),
            represented_item_bonus_actions_like_cpp: Vec::new(),
            represented_item_modifier_runtime_like_cpp:
                wow_entities::PlayerItemModifierRuntimeStateLikeCpp::default(),
            represented_item_set_spell_events_like_cpp: Vec::new(),
            represented_item_set_aura_refresh_events_like_cpp: Vec::new(),
            represented_combat_stat_recalculations_like_cpp: Vec::new(),
            represented_titan_grip_penalty_actions_like_cpp: Vec::new(),
            represented_avg_equipped_item_level_updates_like_cpp: Vec::new(),
        }
    }
}
