// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::inventory` sub-state (#1241 F2): moved fields, no logic.

use super::*;

/// Player items, bank and equipment sets, money and currencies, and the represented bank, guild-
/// bank and auction request sinks.
pub(crate) struct InventoryState {
    /// Handle-less compatibility for older tests. Production C++
    /// `Player::_usePvpItemLevels` lives on the canonical Player.
    #[cfg(test)]
    pub(in crate::session) represented_using_pvp_item_levels_like_cpp: bool,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    /// Production money lives exclusively in `Player::ActivePlayerData::Coinage`.
    #[cfg(test)]
    pub(in crate::session) player_gold: u64,
    #[cfg(test)]
    pub(in crate::session) player_item_test_fixture_like_cpp: PlayerItemTestFixtureLikeCpp,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) represented_bank_bag_slot_flags_like_cpp: [u32; 7],
    #[cfg(test)]
    pub(in crate::session) represented_bank_item_moves_like_cpp:
        Vec<RepresentedBankItemMoveLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_guild_bank_inventory_moves_like_cpp:
        Vec<RepresentedGuildBankInventoryMoveLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_guild_bank_list_requests_like_cpp:
        Vec<RepresentedGuildBankListRequestLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_guild_bank_money_moves_like_cpp:
        Vec<RepresentedGuildBankMoneyMoveLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_guild_bank_tab_actions_like_cpp:
        Vec<RepresentedGuildBankTabActionLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_auction_replicate_requests_like_cpp:
        Vec<RepresentedAuctionReplicateRequestLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_auction_place_bids_like_cpp:
        Vec<RepresentedAuctionPlaceBidLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_auction_remove_items_like_cpp:
        Vec<RepresentedAuctionRemoveItemLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_auction_sell_items_like_cpp:
        Vec<RepresentedAuctionSellItemLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_auto_unequip_offhand_requests_like_cpp:
        Vec<RepresentedAutoUnequipOffhandLikeCpp>,
    pub(in crate::session) represented_guild_repair_bank_state_like_cpp:
        Option<RepresentedGuildRepairBankStateLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_guild_repair_bank_withdraws_like_cpp:
        Vec<RepresentedGuildRepairBankWithdrawLikeCpp>,

    /// Legacy handle-less test fixture for C++ `Player::_currencyStorage`.
    #[cfg(test)]
    pub(in crate::session) player_currencies: HashMap<u32, PlayerCurrency>,

    /// In-memory item objects keyed by item GUID, mirroring C++ `Player::m_items` ownership.
    #[cfg(test)]
    pub(in crate::session) inventory_item_objects: HashMap<ObjectGuid, Item>,
    /// Handle-less test fallback; production C++ `Player::_equipmentSets` lives on canonical Player.
    #[cfg(test)]
    pub(in crate::session) represented_equipment_sets_like_cpp:
        wow_entities::PlayerEquipmentSetsLikeCpp,
    /// Handle-less test fallback; production C++ `Player::_voidStorageItems` lives on canonical Player.
    #[cfg(test)]
    pub(in crate::session) represented_void_storage_items_like_cpp:
        [Option<RepresentedVoidStorageItemLikeCpp>;
            wow_packet::packets::void_storage::VOID_STORAGE_MAX_SLOT_LIKE_CPP],
    #[cfg(test)]
    pub(in crate::session) represented_void_storage_loaded_like_cpp: bool,
    /// True only when `SEL_CHAR_EQUIPMENT` succeeded and proved that the active
    /// character has no top-level persisted item rows. Empty runtime inventory
    /// alone is not source proof because malformed rows may be rejected.
    #[cfg(test)]
    pub(in crate::session) player_equipment_inventory_authority_complete_like_cpp: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_transmog_criteria_events: Vec<RepresentedTransmogCriteriaEvent>,
}
