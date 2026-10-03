// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `Session::inventory` sub-state (#1241 F2): moved fields, no logic.

use crate::RepresentedGuildRepairBankStateLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::{
    PlayerItemTestFixtureLikeCpp, RepresentedAuctionPlaceBidLikeCpp,
    RepresentedAuctionRemoveItemLikeCpp, RepresentedAuctionReplicateRequestLikeCpp,
    RepresentedAuctionSellItemLikeCpp, RepresentedAutoUnequipOffhandLikeCpp,
    RepresentedBankItemMoveLikeCpp, RepresentedGuildBankInventoryMoveLikeCpp,
    RepresentedGuildBankListRequestLikeCpp, RepresentedGuildBankMoneyMoveLikeCpp,
    RepresentedGuildBankTabActionLikeCpp, RepresentedGuildRepairBankWithdrawLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedTransmogCriteriaEvent;
#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::HashMap;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_core::ObjectGuid;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_entities::{Item, PlayerCurrency};

/// Player items, bank and equipment sets, money and currencies, and the represented bank, guild-
/// bank and auction request sinks.
pub struct InventoryState {
    /// Handle-less compatibility for older tests. Production C++
    /// `Player::_usePvpItemLevels` lives on the canonical Player.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_using_pvp_item_levels_like_cpp: bool,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    /// Production money lives exclusively in `Player::ActivePlayerData::Coinage`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) player_gold: u64,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) player_item_test_fixture_like_cpp: PlayerItemTestFixtureLikeCpp,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_bank_bag_slot_flags_like_cpp: [u32; 7],
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_bank_item_moves_like_cpp: Vec<RepresentedBankItemMoveLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_guild_bank_inventory_moves_like_cpp:
        Vec<RepresentedGuildBankInventoryMoveLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_guild_bank_list_requests_like_cpp:
        Vec<RepresentedGuildBankListRequestLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_guild_bank_money_moves_like_cpp:
        Vec<RepresentedGuildBankMoneyMoveLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_guild_bank_tab_actions_like_cpp:
        Vec<RepresentedGuildBankTabActionLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_auction_replicate_requests_like_cpp:
        Vec<RepresentedAuctionReplicateRequestLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_auction_place_bids_like_cpp: Vec<RepresentedAuctionPlaceBidLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_auction_remove_items_like_cpp: Vec<RepresentedAuctionRemoveItemLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_auction_sell_items_like_cpp: Vec<RepresentedAuctionSellItemLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_auto_unequip_offhand_requests_like_cpp:
        Vec<RepresentedAutoUnequipOffhandLikeCpp>,
    pub(crate) represented_guild_repair_bank_state_like_cpp:
        Option<RepresentedGuildRepairBankStateLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_guild_repair_bank_withdraws_like_cpp:
        Vec<RepresentedGuildRepairBankWithdrawLikeCpp>,

    /// Legacy handle-less test fixture for C++ `Player::_currencyStorage`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) player_currencies: HashMap<u32, PlayerCurrency>,

    /// In-memory item objects keyed by item GUID, mirroring C++ `Player::m_items` ownership.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) inventory_item_objects: HashMap<ObjectGuid, Item>,
    /// Handle-less test fallback; production C++ `Player::_equipmentSets` lives on canonical Player.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_equipment_sets_like_cpp: wow_entities::PlayerEquipmentSetsLikeCpp,
    /// Handle-less test fallback; production C++ `Player::_voidStorageItems` lives on canonical Player.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_void_storage_items_like_cpp:
        [Option<wow_entities::PlayerVoidStorageItemLikeCpp>;
            wow_packet::packets::void_storage::VOID_STORAGE_MAX_SLOT_LIKE_CPP],
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_void_storage_loaded_like_cpp: bool,
    /// True only when `SEL_CHAR_EQUIPMENT` succeeded and proved that the active
    /// character has no top-level persisted item rows. Empty runtime inventory
    /// alone is not source proof because malformed rows may be rejected.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) player_equipment_inventory_authority_complete_like_cpp: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_transmog_criteria_events: Vec<RepresentedTransmogCriteriaEvent>,
}

impl InventoryState {
    pub fn new_like_cpp() -> Self {
        Self {
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_using_pvp_item_levels_like_cpp: false,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_gold: 0,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_item_test_fixture_like_cpp: PlayerItemTestFixtureLikeCpp::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_bank_bag_slot_flags_like_cpp: [0; 7],
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_bank_item_moves_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_guild_bank_inventory_moves_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_guild_bank_list_requests_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_guild_bank_money_moves_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_guild_bank_tab_actions_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_auction_replicate_requests_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_auction_place_bids_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_auction_remove_items_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_auction_sell_items_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_auto_unequip_offhand_requests_like_cpp: Vec::new(),
            represented_guild_repair_bank_state_like_cpp: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_guild_repair_bank_withdraws_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            player_currencies: HashMap::new(),

            #[cfg(any(test, feature = "test-fixtures"))]
            inventory_item_objects: HashMap::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_equipment_sets_like_cpp:
                wow_entities::PlayerEquipmentSetsLikeCpp::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_void_storage_items_like_cpp: std::array::from_fn(|_| None),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_void_storage_loaded_like_cpp: false,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_equipment_inventory_authority_complete_like_cpp: false,
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_transmog_criteria_events: Vec::new(),
        }
    }
}
