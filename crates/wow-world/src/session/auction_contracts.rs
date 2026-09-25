// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Auction contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::ObjectGuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedAuctionReplicateRequestLikeCpp {
    pub auctioneer: ObjectGuid,
    pub change_number_global: u32,
    pub change_number_cursor: u32,
    pub change_number_tombstone: u32,
    pub count: u32,
    pub tainted_by_present: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedAuctionPlaceBidLikeCpp {
    pub auctioneer: ObjectGuid,
    pub auction_id: i32,
    pub bid_amount: u64,
    pub tainted_by_present: bool,
    pub copper_rejected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedAuctionRemoveItemLikeCpp {
    pub auctioneer: ObjectGuid,
    pub auction_id: i32,
    pub item_id: i32,
    pub tainted_by_present: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedAuctionSellItemLikeCpp {
    pub auctioneer: ObjectGuid,
    pub item_guid: Option<ObjectGuid>,
    pub item_use_count: Option<u32>,
    pub min_bid: u64,
    pub buyout_price: u64,
    pub runtime_minutes: u32,
    pub tainted_by_present: bool,
    pub item_list_rejected: bool,
    pub use_count_rejected: bool,
    pub no_price_rejected: bool,
    pub max_money_rejected: bool,
    pub copper_rejected: bool,
    pub auctioneer_accepted: bool,
    pub runtime_rejected: bool,
}
