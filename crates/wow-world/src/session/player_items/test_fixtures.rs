//! Handle-less Player item state and effect evidence used by Session tests.

use super::*;

pub(crate) struct PlayerItemTestFixtureLikeCpp {
    /// Explicit fixture metadata; never a Player authority or a normal feature fallback.
    ownerless_inventory_snapshots_enabled: bool,
    #[cfg(not(test))]
    appearance_criteria_diagnostics_enabled: bool,
    #[cfg(not(test))]
    appearance_criteria_events: Vec<RepresentedTransmogCriteriaEvent>,
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
    #[cfg(test)]
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
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) represented_combat_stat_recalculations_like_cpp:
        Vec<RepresentedCombatStatRecalculationLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_titan_grip_penalty_actions_like_cpp:
        Vec<TitanGripPenaltyAction>,
    pub(in crate::session) represented_avg_equipped_item_level_updates_like_cpp: Vec<f32>,
}

impl Default for PlayerItemTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            ownerless_inventory_snapshots_enabled: false,
            #[cfg(not(test))]
            appearance_criteria_diagnostics_enabled: false,
            #[cfg(not(test))]
            appearance_criteria_events: Vec::new(),
            player_bank_bag_slot_count_like_cpp: 0,
            player_inventory_slot_count_like_cpp: INVENTORY_DEFAULT_SIZE,
            inventory_items: HashMap::new(),
            buyback_items: HashMap::new(),
            buyback_price: [0; BUYBACK_SLOT_COUNT],
            buyback_timestamp: [0; BUYBACK_SLOT_COUNT],
            current_buyback_slot: BUYBACK_SLOT_START,
            #[cfg(test)]
            represented_item_mod_reapply_events_like_cpp: Vec::new(),
            represented_item_bonus_actions_like_cpp: Vec::new(),
            represented_item_modifier_runtime_like_cpp:
                wow_entities::PlayerItemModifierRuntimeStateLikeCpp::default(),
            represented_item_set_spell_events_like_cpp: Vec::new(),
            represented_item_set_aura_refresh_events_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_combat_stat_recalculations_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_titan_grip_penalty_actions_like_cpp: Vec::new(),
            represented_avg_equipped_item_level_updates_like_cpp: Vec::new(),
        }
    }
}

impl PlayerItemTestFixtureLikeCpp {
    pub(super) fn enable_ownerless_inventory_snapshots_for_test(&mut self) {
        self.ownerless_inventory_snapshots_enabled = true;
    }

    pub(super) fn diagnostics_enabled_for_test(&self) -> bool {
        cfg!(test) || self.ownerless_inventory_snapshots_enabled
    }
}

impl WorldSession {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn enable_ownerless_inventory_snapshots_for_test(&mut self) {
        self.player_item_test_fixture_like_cpp
            .enable_ownerless_inventory_snapshots_for_test();
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn inventory_player_snapshot_for_test(&self) -> Option<Player> {
        self.direct_inventory_player_snapshot()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn inventory_player_handle_present_for_test(&self) -> bool {
        self.player_handle_like_cpp.is_some()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) fn inventory_fixture_diagnostics_enabled_for_test(&self) -> bool {
        self.player_item_test_fixture_like_cpp.diagnostics_enabled_for_test()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) fn ownerless_inventory_fallback_enabled_for_test(&self) -> bool {
        self.player_handle_like_cpp.is_none()
            && self.inventory_fixture_diagnostics_enabled_for_test()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn inventory_combat_recalculations_for_test(
        &self,
    ) -> &[RepresentedCombatStatRecalculationLikeCpp] {
        &self.player_item_test_fixture_like_cpp.represented_combat_stat_recalculations_like_cpp
    }
}

impl PlayerItemTestFixtureLikeCpp {
    pub(super) fn enable_appearance_criteria_diagnostics_for_test(&mut self) {
        #[cfg(not(test))]
        {
            self.appearance_criteria_diagnostics_enabled = true;
        }
    }

    #[cfg(not(test))]
    pub(super) fn record_appearance_criteria_for_test(&mut self, event: RepresentedTransmogCriteriaEvent) {
        if self.appearance_criteria_diagnostics_enabled {
            self.appearance_criteria_events.push(event);
        }
    }

    #[cfg(not(test))]
    pub(super) fn appearance_criteria_events_for_test(&self) -> &[RepresentedTransmogCriteriaEvent] {
        &self.appearance_criteria_events
    }

    #[cfg(not(test))]
    pub(super) fn clear_appearance_criteria_events_for_test(&mut self) {
        self.appearance_criteria_events.clear();
    }
}
