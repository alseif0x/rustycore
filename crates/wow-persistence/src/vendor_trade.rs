//! SQLx-free persistence contract for Player/vendor commerce.

use crate::{
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerCurrencySaveRequestLikeCpp,
    PlayerMoneyTransactionOutcomeLikeCpp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorItemTurninPersistenceLikeCpp {
    Update { item_guid: u64, new_count: u32 },
    Delete { owner_guid: u64, item_guid: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorCurrencyPurchasePersistenceLikeCpp {
    pub player_guid: u64,
    pub money_before: u64,
    pub money_after: u64,
    pub item_turnins: Vec<VendorItemTurninPersistenceLikeCpp>,
    pub currency_save: PlayerCurrencySaveRequestLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendorExistingStackPersistenceLikeCpp {
    pub item_guid: u64,
    pub new_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorPurchasedStackPersistenceLikeCpp {
    pub item_guid: u64,
    pub item_entry: u32,
    pub owner_guid: u64,
    pub count: u32,
    pub durability: u32,
    pub flags: u32,
    pub random_properties_id: i32,
    pub property_seed: i32,
    pub context: u8,
    pub inventory_slot: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendorRefundMetadataPersistenceLikeCpp {
    pub item_guid: u64,
    pub player_guid: u64,
    pub paid_money: u64,
    pub paid_extended_cost: u16,
    pub flags_after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorItemPurchasePersistenceLikeCpp {
    pub player_guid: u64,
    pub money_before: u64,
    pub money_after: u64,
    pub existing_stacks: Vec<VendorExistingStackPersistenceLikeCpp>,
    pub new_stacks: Vec<VendorPurchasedStackPersistenceLikeCpp>,
    pub refund_metadata: Option<VendorRefundMetadataPersistenceLikeCpp>,
    pub item_turnins: Vec<VendorItemTurninPersistenceLikeCpp>,
    pub currency_save: PlayerCurrencySaveRequestLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorBuybackDestinationPersistenceLikeCpp {
    Merge {
        item_guid: u64,
        new_count: u32,
    },
    Move {
        inventory_slot: u8,
        item_guid: u64,
        new_count: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorBuybackPersistenceLikeCpp {
    pub player_guid: u64,
    pub money_before: u64,
    pub money_after: u64,
    pub destinations: Vec<VendorBuybackDestinationPersistenceLikeCpp>,
    pub delete_source_item_guid: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorSoldClonePersistenceLikeCpp {
    pub item_guid: u64,
    pub item_entry: u32,
    pub owner_guid: u64,
    pub creator_guid: u64,
    pub gift_creator_guid: u64,
    pub count: u32,
    pub expiration: u32,
    pub charges: String,
    pub enchantments: String,
    pub flags: u32,
    pub durability: u32,
    pub create_played_time: u32,
    pub random_properties_id: i32,
    pub property_seed: i32,
    pub context: u8,
    pub buyback_slot: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VendorSaleItemPersistenceLikeCpp {
    FullStack {
        item_guid: u64,
        buyback_slot: u8,
    },
    PartialStack {
        source_item_guid: u64,
        source_count_after: u32,
        sold_clone: VendorSoldClonePersistenceLikeCpp,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorSalePersistenceLikeCpp {
    pub player_guid: u64,
    pub money_before: u64,
    pub money_after: u64,
    pub evicted_buyback_item_guid: Option<u64>,
    pub sold_item: VendorSaleItemPersistenceLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendorRefundReturnedStackPersistenceLikeCpp {
    pub item_guid: u64,
    pub item_entry: u32,
    pub owner_guid: u64,
    pub count: u32,
    pub durability: u32,
    pub inventory_slot: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorRefundPersistenceLikeCpp {
    pub player_guid: u64,
    pub refunded_item_guid: u64,
    pub money_before: u64,
    pub money_after: u64,
    pub existing_stacks: Vec<VendorExistingStackPersistenceLikeCpp>,
    pub new_stacks: Vec<VendorRefundReturnedStackPersistenceLikeCpp>,
    pub currency_save: PlayerCurrencySaveRequestLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VendorTradePersistenceRequestLikeCpp {
    CurrencyPurchase(VendorCurrencyPurchasePersistenceLikeCpp),
    ItemPurchase(VendorItemPurchasePersistenceLikeCpp),
    Buyback(VendorBuybackPersistenceLikeCpp),
    Sale(VendorSalePersistenceLikeCpp),
    Refund(VendorRefundPersistenceLikeCpp),
}

impl VendorTradePersistenceRequestLikeCpp {
    pub fn player_guid(&self) -> u64 {
        match self {
            Self::CurrencyPurchase(request) => request.player_guid,
            Self::ItemPurchase(request) => request.player_guid,
            Self::Buyback(request) => request.player_guid,
            Self::Sale(request) => request.player_guid,
            Self::Refund(request) => request.player_guid,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendorRefundCleanupPersistenceLikeCpp {
    pub item_guid: u64,
    pub flags_after: u32,
}

pub trait VendorTradePersistencePortLikeCpp: Send + Sync {
    fn persist_vendor_trade_like_cpp(
        &self,
        request: VendorTradePersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PlayerMoneyTransactionOutcomeLikeCpp>;

    fn clear_refund_metadata_like_cpp(
        &self,
        request: VendorRefundCleanupPersistenceLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp>;
}
