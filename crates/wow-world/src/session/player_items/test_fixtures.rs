//! Handle-less Player item state and effect evidence used by Session tests.

use super::*;

pub(crate) struct PlayerItemTestFixtureLikeCpp {
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    pub(in crate::session) player_bank_bag_slot_count_like_cpp: u8,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    pub(in crate::session) player_inventory_slot_count_like_cpp: u8,
    /// In-memory inventory: slot → (item ObjectGuid, entry_id, db_guid).
    pub(in crate::session) inventory_items: HashMap<u8, InventoryItem>,
    /// In-memory buyback slots, kept separate from normal inventory like C++ `GetItemByGuid`.
    pub(in crate::session) buyback_items: HashMap<u8, InventoryItem>,
    pub(in crate::session) buyback_price: [u32; BUYBACK_SLOT_COUNT],
    pub(in crate::session) buyback_timestamp: [i64; BUYBACK_SLOT_COUNT],
    pub(in crate::session) current_buyback_slot: u8,
    pub(in crate::session) represented_item_mod_reapply_events_like_cpp:
        Vec<RepresentedItemModsReapplyEventLikeCpp>,
    pub(in crate::session) represented_item_bonus_actions_like_cpp:
        Vec<RepresentedItemBonusActionLikeCpp>,
    pub(in crate::session) represented_item_modifier_runtime_like_cpp:
        wow_entities::PlayerItemModifierRuntimeStateLikeCpp,
    pub(in crate::session) represented_item_set_spell_events_like_cpp:
        Vec<RepresentedItemSetSpellEventLikeCpp>,
    pub(in crate::session) represented_item_set_aura_refresh_events_like_cpp:
        Vec<RepresentedItemSetAuraRefreshEventLikeCpp>,
    pub(in crate::session) represented_combat_stat_recalculations_like_cpp:
        Vec<RepresentedCombatStatRecalculationLikeCpp>,
    pub(in crate::session) represented_titan_grip_penalty_actions_like_cpp:
        Vec<TitanGripPenaltyAction>,
    pub(in crate::session) represented_avg_equipped_item_level_updates_like_cpp: Vec<f32>,
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
