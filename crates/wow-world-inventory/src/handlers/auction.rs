// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Auction house command handlers.
//!
//! C++ source of truth: `AuctionHouseHandler.cpp` plus the represented
//! `AuctionMgr` boundary these handlers currently observe. The family owns the
//! packet bodies, the auctioneer interaction gate and the represented request
//! records; the World session only builds the borrowed inventory + hub context
//! (#1263 F5).

use tracing::{debug, info, warn};
use wow_constants::ClientOpcodes;
use wow_constants::unit::NPCFlags1;
use wow_entities::MAX_MONEY_AMOUNT;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;
use wow_packet::packets::misc::{
    AuctionListItems, AuctionPlaceBid, AuctionRemoveItem, AuctionReplicateItems, AuctionSellItem,
    AuctionableTokenSell, AuctionableTokenSellAtMarketPrice, CommerceTokenGetLog,
    CommerceTokenGetLogResponse,
};
use wow_world_core::session::{HubMut, PacketPublicationAccessLikeCpp};

use crate::state::InventoryState;
use crate::{
    RepresentedAuctionPlaceBidLikeCpp, RepresentedAuctionRemoveItemLikeCpp,
    RepresentedAuctionReplicateRequestLikeCpp, RepresentedAuctionSellItemLikeCpp,
};

const SILVER_LIKE_CPP: u64 = 100;
const MIN_AUCTION_TIME_MINUTES_LIKE_CPP: u32 = 12 * 60;
const SHORT_AUCTION_TIME_MINUTES_LIKE_CPP: u32 = MIN_AUCTION_TIME_MINUTES_LIKE_CPP;
const MEDIUM_AUCTION_TIME_MINUTES_LIKE_CPP: u32 = 2 * MIN_AUCTION_TIME_MINUTES_LIKE_CPP;
const LONG_AUCTION_TIME_MINUTES_LIKE_CPP: u32 = 4 * MIN_AUCTION_TIME_MINUTES_LIKE_CPP;

/// Borrowed inputs of one auction handler invocation.
pub struct AuctionHandlerCxLikeCpp<'a> {
    inventory: &'a mut InventoryState,
    hub: HubMut<'a>,
}

impl<'a> AuctionHandlerCxLikeCpp<'a> {
    pub fn new(inventory: &'a mut InventoryState, hub: HubMut<'a>) -> Self {
        Self { inventory, hub }
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    /// CMSG_AUCTION_HELLO_REQUEST — open the auctioneer window.
    pub async fn handle_auction_hello_request(&mut self, mut pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::AuctionHelloResponse;
        let guid = pkt
            .read_packed_guid()
            .unwrap_or(wow_core::ObjectGuid::EMPTY);
        info!(
            "AuctionHelloRequest from {:?} account {}",
            guid,
            self.hub.shared().core.account_id
        );
        self.publication_like_cpp()
            .send_packet(&AuctionHelloResponse::open(guid));
    }

    pub async fn handle_auction_list_bidder_items(&mut self, _pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::AuctionListBidderItemsResult;
        self.publication_like_cpp()
            .send_packet(&AuctionListBidderItemsResult);
    }

    pub async fn handle_auction_list_items(
        &mut self,
        _packet: wow_packet::packets::misc::AuctionListItems,
    ) {
    }

    pub async fn handle_auction_place_bid(&mut self, packet: AuctionPlaceBid) {
        let Some(_auctioneer) = self
            .hub
            .shared()
            .represented_npc_can_interact_with_like_cpp(
                packet.auctioneer,
                NPCFlags1::AUCTIONEER.bits(),
                0,
            )
        else {
            debug!(
                account = self.hub.shared().core.account_id,
                auctioneer = ?packet.auctioneer,
                auction_id = packet.auction_id,
                "AuctionPlaceBid rejected: auctioneer missing, invalid, hostile/dead, out of range, or lacks AUCTIONEER flag"
            );
            return;
        };

        #[cfg(any(test, feature = "test-fixtures"))]
        self.inventory
            .record_represented_auction_place_bid_like_cpp(RepresentedAuctionPlaceBidLikeCpp {
                auctioneer: packet.auctioneer,
                auction_id: packet.auction_id,
                bid_amount: packet.bid_amount,
                tainted_by_present: packet.tainted_by.is_some(),
                copper_rejected: packet.bid_amount % SILVER_LIKE_CPP != 0,
            });
    }

    pub async fn handle_auction_remove_item(&mut self, packet: AuctionRemoveItem) {
        let Some(_auctioneer) = self
            .hub
            .shared()
            .represented_npc_can_interact_with_like_cpp(
                packet.auctioneer,
                NPCFlags1::AUCTIONEER.bits(),
                0,
            )
        else {
            debug!(
                account = self.hub.shared().core.account_id,
                auctioneer = ?packet.auctioneer,
                auction_id = packet.auction_id,
                item_id = packet.item_id,
                "AuctionRemoveItem rejected: auctioneer missing, invalid, hostile/dead, out of range, or lacks AUCTIONEER flag"
            );
            return;
        };

        self.inventory
            .record_represented_auction_remove_item_like_cpp(RepresentedAuctionRemoveItemLikeCpp {
                auctioneer: packet.auctioneer,
                auction_id: packet.auction_id,
                item_id: packet.item_id,
                tainted_by_present: packet.tainted_by.is_some(),
            });
    }

    pub async fn handle_auction_sell_item(&mut self, packet: AuctionSellItem) {
        let first_item = packet.items.first().copied();
        let item_list_rejected = packet.items.len() != 1;
        let use_count_rejected = packet.items.len() == 1
            && first_item
                .map(|item| item.use_count != 1)
                .unwrap_or_default();
        let no_price_rejected = packet.min_bid == 0 && packet.buyout_price == 0;
        let max_money_rejected =
            packet.min_bid > MAX_MONEY_AMOUNT || packet.buyout_price > MAX_MONEY_AMOUNT;
        let copper_rejected =
            packet.min_bid % SILVER_LIKE_CPP != 0 || packet.buyout_price % SILVER_LIKE_CPP != 0;

        let mut represented = RepresentedAuctionSellItemLikeCpp {
            auctioneer: packet.auctioneer,
            item_guid: first_item.map(|item| item.guid),
            item_use_count: first_item.map(|item| item.use_count),
            min_bid: packet.min_bid,
            buyout_price: packet.buyout_price,
            runtime_minutes: packet.runtime,
            tainted_by_present: packet.tainted_by.is_some(),
            item_list_rejected,
            use_count_rejected,
            no_price_rejected,
            max_money_rejected,
            copper_rejected,
            auctioneer_accepted: false,
            runtime_rejected: false,
        };

        if item_list_rejected
            || use_count_rejected
            || no_price_rejected
            || max_money_rejected
            || copper_rejected
        {
            self.inventory
                .record_represented_auction_sell_item_like_cpp(represented);
            return;
        }

        let Some(_auctioneer) = self
            .hub
            .shared()
            .represented_npc_can_interact_with_like_cpp(
                packet.auctioneer,
                NPCFlags1::AUCTIONEER.bits(),
                0,
            )
        else {
            debug!(
                account = self.hub.shared().core.account_id,
                auctioneer = ?packet.auctioneer,
                runtime = packet.runtime,
                "AuctionSellItem rejected: auctioneer missing, invalid, hostile/dead, out of range, or lacks AUCTIONEER flag"
            );
            return;
        };
        represented.auctioneer_accepted = true;

        represented.runtime_rejected = !matches!(
            packet.runtime,
            SHORT_AUCTION_TIME_MINUTES_LIKE_CPP
                | MEDIUM_AUCTION_TIME_MINUTES_LIKE_CPP
                | LONG_AUCTION_TIME_MINUTES_LIKE_CPP
        );
        self.inventory
            .record_represented_auction_sell_item_like_cpp(represented);
    }

    pub async fn handle_auction_replicate_items(&mut self, packet: AuctionReplicateItems) {
        let Some(_auctioneer) = self
            .hub
            .shared()
            .represented_npc_can_interact_with_like_cpp(
                packet.auctioneer,
                NPCFlags1::AUCTIONEER.bits(),
                0,
            )
        else {
            debug!(
                account = self.hub.shared().core.account_id,
                auctioneer = ?packet.auctioneer,
                "AuctionReplicateItems rejected: auctioneer missing, invalid, hostile/dead, out of range, or lacks AUCTIONEER flag"
            );
            return;
        };

        #[cfg(any(test, feature = "test-fixtures"))]
        self.inventory
            .record_represented_auction_replicate_request_like_cpp(
                RepresentedAuctionReplicateRequestLikeCpp {
                    auctioneer: packet.auctioneer,
                    change_number_global: packet.change_number_global,
                    change_number_cursor: packet.change_number_cursor,
                    change_number_tombstone: packet.change_number_tombstone,
                    count: packet.count,
                    tainted_by_present: packet.tainted_by.is_some(),
                },
            );
    }

    pub async fn handle_auction_list_owner_items(&mut self, _pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::AuctionListOwnerItemsResult;
        self.publication_like_cpp()
            .send_packet(&AuctionListOwnerItemsResult);
    }

    pub async fn handle_auction_list_pending_sales(&mut self, _pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::AuctionListPendingSalesResult;
        self.publication_like_cpp()
            .send_packet(&AuctionListPendingSalesResult);
    }

    pub async fn handle_auctionable_token_sell(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = AuctionableTokenSell::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "AuctionableTokenSell parse failed: {error}"
            );
        }
    }

    pub async fn handle_auctionable_token_sell_at_market_price(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        if let Err(error) = AuctionableTokenSellAtMarketPrice::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "AuctionableTokenSellAtMarketPrice parse failed: {error}"
            );
        }
    }

    pub async fn handle_commerce_token_get_log(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match CommerceTokenGetLog::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "CommerceTokenGetLog parse failed: {error}"
                );
                return;
            }
        };

        // C++ has a TODO here and returns TOKEN_RESULT_SUCCESS with an empty
        // auctionable-token list while echoing the request integer.
        self.publication_like_cpp()
            .send_packet(&CommerceTokenGetLogResponse::success_empty(request.unk_int));
    }
}

pub(crate) fn handle_auction_hello_request_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: crate::handlers::InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .auction_handler_cx_like_cpp(catalogs)
            .handle_auction_hello_request(pkt)
            .await;
    })
}

pub(crate) fn handle_auction_list_bidder_items_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: crate::handlers::InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .auction_handler_cx_like_cpp(catalogs)
            .handle_auction_list_bidder_items(pkt)
            .await;
    })
}

pub(crate) fn handle_auction_list_items_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: crate::handlers::InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match AuctionListItems::read(&mut pkt) {
            Ok(value) => {
                session
                    .auction_handler_cx_like_cpp(catalogs)
                    .handle_auction_list_items(value)
                    .await;
            }
            Err(e) => warn!("Failed to read AuctionListItems: {e}"),
        }
    })
}

pub(crate) fn handle_auction_place_bid_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: crate::handlers::InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match AuctionPlaceBid::read(&mut pkt) {
            Ok(value) => {
                session
                    .auction_handler_cx_like_cpp(catalogs)
                    .handle_auction_place_bid(value)
                    .await;
            }
            Err(e) => warn!("Failed to read AuctionPlaceBid: {e}"),
        }
    })
}

pub(crate) fn handle_auction_remove_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: crate::handlers::InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match AuctionRemoveItem::read(&mut pkt) {
            Ok(value) => {
                session
                    .auction_handler_cx_like_cpp(catalogs)
                    .handle_auction_remove_item(value)
                    .await;
            }
            Err(e) => warn!("Failed to read AuctionRemoveItem: {e}"),
        }
    })
}

pub(crate) fn handle_auction_sell_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: crate::handlers::InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match AuctionSellItem::read(&mut pkt) {
            Ok(value) => {
                session
                    .auction_handler_cx_like_cpp(catalogs)
                    .handle_auction_sell_item(value)
                    .await;
            }
            Err(e) => warn!("Failed to read AuctionSellItem: {e}"),
        }
    })
}

pub(crate) fn handle_auction_replicate_items_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: crate::handlers::InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match AuctionReplicateItems::read(&mut pkt) {
            Ok(value) => {
                session
                    .auction_handler_cx_like_cpp(catalogs)
                    .handle_auction_replicate_items(value)
                    .await;
            }
            Err(e) => warn!("Failed to read AuctionReplicateItems: {e}"),
        }
    })
}

pub(crate) fn handle_auction_list_owner_items_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: crate::handlers::InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .auction_handler_cx_like_cpp(catalogs)
            .handle_auction_list_owner_items(pkt)
            .await;
    })
}

pub(crate) fn handle_auction_list_pending_sales_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: crate::handlers::InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .auction_handler_cx_like_cpp(catalogs)
            .handle_auction_list_pending_sales(pkt)
            .await;
    })
}

pub(crate) fn handle_auctionable_token_sell_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: crate::handlers::InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .auction_handler_cx_like_cpp(catalogs)
            .handle_auctionable_token_sell(pkt)
            .await;
    })
}

pub(crate) fn handle_auctionable_token_sell_at_market_price_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: crate::handlers::InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .auction_handler_cx_like_cpp(catalogs)
            .handle_auctionable_token_sell_at_market_price(pkt)
            .await;
    })
}

pub(crate) fn handle_commerce_token_get_log_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: crate::handlers::InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .auction_handler_cx_like_cpp(catalogs)
            .handle_commerce_token_get_log(pkt)
            .await;
    })
}
