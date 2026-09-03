// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private auction capability handlers extracted from the legacy misc owner.

use tracing::{debug, warn};
use wow_constants::ClientOpcodes;
use wow_constants::unit::NPCFlags1;
use wow_entities::MAX_MONEY_AMOUNT;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    AuctionPlaceBid, AuctionRemoveItem, AuctionReplicateItems, AuctionSellItem,
    AuctionableTokenSell, AuctionableTokenSellAtMarketPrice, CommerceTokenGetLog,
    CommerceTokenGetLogResponse,
};

use super::{
    LONG_AUCTION_TIME_MINUTES_LIKE_CPP, MEDIUM_AUCTION_TIME_MINUTES_LIKE_CPP,
    SHORT_AUCTION_TIME_MINUTES_LIKE_CPP, SILVER_LIKE_CPP,
};
use crate::session::{
    RepresentedAuctionPlaceBidLikeCpp, RepresentedAuctionRemoveItemLikeCpp,
    RepresentedAuctionReplicateRequestLikeCpp, RepresentedAuctionSellItemLikeCpp,
};

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionListBidderItems,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_list_bidder_items",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_auction_list_bidder_items(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionListItems,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_list_items",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::AuctionListItems::read(&mut pkt) {
                    Ok(packet) => session.handle_auction_list_items(packet).await,
                    Err(e) => tracing::warn!("Failed to read AuctionListItems: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionPlaceBid,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_place_bid",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::AuctionPlaceBid::read(&mut pkt) {
                    Ok(packet) => session.handle_auction_place_bid(packet).await,
                    Err(e) => tracing::warn!("Failed to read AuctionPlaceBid: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionRemoveItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_remove_item",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::AuctionRemoveItem::read(&mut pkt) {
                    Ok(packet) => session.handle_auction_remove_item(packet).await,
                    Err(e) => tracing::warn!("Failed to read AuctionRemoveItem: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionSellItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_sell_item",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::AuctionSellItem::read(&mut pkt) {
                    Ok(packet) => session.handle_auction_sell_item(packet).await,
                    Err(e) => tracing::warn!("Failed to read AuctionSellItem: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionReplicateItems,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_replicate_items",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::AuctionReplicateItems::read(&mut pkt) {
                    Ok(packet) => session.handle_auction_replicate_items(packet).await,
                    Err(e) => tracing::warn!("Failed to read AuctionReplicateItems: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionListOwnerItems,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_list_owner_items",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_auction_list_owner_items(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionListPendingSales,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_list_pending_sales",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_auction_list_pending_sales(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionableTokenSell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auctionable_token_sell",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_auctionable_token_sell(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionableTokenSellAtMarketPrice,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auctionable_token_sell_at_market_price",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_auctionable_token_sell_at_market_price(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CommerceTokenGetLog,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_commerce_token_get_log",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_commerce_token_get_log(pkt).await })
        },
    }
}

impl crate::session::WorldSession {
    // ── Auction house list stubs ──────────────────────────────────────────────

    /// CMSG_AUCTION_LIST_BIDDER_ITEMS — list items bid on.
    /// Returns empty list until AH system is implemented.
    pub async fn handle_auction_list_bidder_items(&mut self, _pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::AuctionListBidderItemsResult;
        self.send_packet(&AuctionListBidderItemsResult);
    }

    /// CMSG_AUCTION_LIST_ITEMS — legacy list opcode.
    ///
    /// The current C++ legacy branch reads no fields and only logs that this
    /// opcode is superseded by CMSG_AUCTION_BROWSE_QUERY.

    pub async fn handle_auction_list_items(
        &mut self,
        _packet: wow_packet::packets::misc::AuctionListItems,
    ) {
    }

    /// CMSG_AUCTION_PLACE_BID — bid or buyout an auction.
    ///
    /// C++ gates on throttle, auctioneer interaction, and silver granularity
    /// before reaching AuctionMgr state. Rust has no live AH state yet, so this
    /// records the represented request after the interaction gate.

    pub async fn handle_auction_place_bid(&mut self, packet: AuctionPlaceBid) {
        let Some(_auctioneer) = self.represented_npc_can_interact_with_like_cpp(
            packet.auctioneer,
            NPCFlags1::AUCTIONEER.bits(),
            0,
        ) else {
            debug!(
                account = self.account_id,
                auctioneer = ?packet.auctioneer,
                auction_id = packet.auction_id,
                "AuctionPlaceBid rejected: auctioneer missing, invalid, hostile/dead, out of range, or lacks AUCTIONEER flag"
            );
            return;
        };

        self.record_represented_auction_place_bid_like_cpp(RepresentedAuctionPlaceBidLikeCpp {
            auctioneer: packet.auctioneer,
            auction_id: packet.auction_id,
            bid_amount: packet.bid_amount,
            tainted_by_present: packet.tainted_by.is_some(),
            copper_rejected: packet.bid_amount % SILVER_LIKE_CPP != 0,
        });
    }

    /// CMSG_AUCTION_REMOVE_ITEM — cancel one of the player's auctions.
    ///
    /// C++ gates on throttle and auctioneer interaction before checking
    /// AuctionMgr ownership/bidder state and DB. Rust has no live AH map yet,
    /// so this records the represented cancel request after the interaction
    /// gate without pretending the auction mutation exists.

    pub async fn handle_auction_remove_item(&mut self, packet: AuctionRemoveItem) {
        let Some(_auctioneer) = self.represented_npc_can_interact_with_like_cpp(
            packet.auctioneer,
            NPCFlags1::AUCTIONEER.bits(),
            0,
        ) else {
            debug!(
                account = self.account_id,
                auctioneer = ?packet.auctioneer,
                auction_id = packet.auction_id,
                item_id = packet.item_id,
                "AuctionRemoveItem rejected: auctioneer missing, invalid, hostile/dead, out of range, or lacks AUCTIONEER flag"
            );
            return;
        };

        self.record_represented_auction_remove_item_like_cpp(RepresentedAuctionRemoveItemLikeCpp {
            auctioneer: packet.auctioneer,
            auction_id: packet.auction_id,
            item_id: packet.item_id,
            tainted_by_present: packet.tainted_by.is_some(),
        });
    }

    /// CMSG_AUCTION_SELL_ITEM — post a single non-commodity item for auction.
    ///
    /// C++ validates packet-level sell-item constraints before auctioneer
    /// lookup, then validates auctioneer, runtime, live item state, deposit,
    /// and DB. Rust captures the packet/auctioneer/runtime gates currently
    /// representable and leaves live AuctionMgr/item mutation open.

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
            self.record_represented_auction_sell_item_like_cpp(represented);
            return;
        }

        let Some(_auctioneer) = self.represented_npc_can_interact_with_like_cpp(
            packet.auctioneer,
            NPCFlags1::AUCTIONEER.bits(),
            0,
        ) else {
            debug!(
                account = self.account_id,
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
        self.record_represented_auction_sell_item_like_cpp(represented);
    }

    /// CMSG_AUCTION_REPLICATE_ITEMS — replicate auction-house changes.
    ///
    /// C++ gates on an alive, usable auctioneer before building the replicate
    /// response from AuctionMgr. The live AH object map/response builder are
    /// not ported yet, so this slice records the accepted represented request.

    pub async fn handle_auction_replicate_items(&mut self, packet: AuctionReplicateItems) {
        let Some(_auctioneer) = self.represented_npc_can_interact_with_like_cpp(
            packet.auctioneer,
            NPCFlags1::AUCTIONEER.bits(),
            0,
        ) else {
            debug!(
                account = self.account_id,
                auctioneer = ?packet.auctioneer,
                "AuctionReplicateItems rejected: auctioneer missing, invalid, hostile/dead, out of range, or lacks AUCTIONEER flag"
            );
            return;
        };

        self.record_represented_auction_replicate_request_like_cpp(
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

    /// CMSG_AUCTION_LIST_OWNER_ITEMS — list items the player put up for auction.
    /// Returns empty list until AH system is implemented.

    pub async fn handle_auction_list_owner_items(&mut self, _pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::AuctionListOwnerItemsResult;
        self.send_packet(&AuctionListOwnerItemsResult);
    }

    /// CMSG_AUCTION_LIST_PENDING_SALES — list pending sales / completed auctions.
    /// Returns empty list until AH system is implemented.

    pub async fn handle_auction_list_pending_sales(&mut self, _pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::AuctionListPendingSalesResult;
        self.send_packet(&AuctionListPendingSalesResult);
    }

    /// CMSG_AUCTIONABLE_TOKEN_SELL — WoW Token sell request.
    ///
    /// The legacy C++ WotLK branch keeps this as an explicit empty stub because
    /// WoW Token is not available in WotLK.

    pub async fn handle_auctionable_token_sell(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = AuctionableTokenSell::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "AuctionableTokenSell parse failed: {error}"
            );
        }
    }

    /// CMSG_AUCTIONABLE_TOKEN_SELL_AT_MARKET_PRICE — WoW Token sell confirmation.
    ///
    /// The legacy C++ WotLK branch keeps this as an explicit empty stub because
    /// WoW Token is not available in WotLK.

    pub async fn handle_auctionable_token_sell_at_market_price(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        if let Err(error) = AuctionableTokenSellAtMarketPrice::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "AuctionableTokenSellAtMarketPrice parse failed: {error}"
            );
        }
    }

    /// CMSG_COMMERCE_TOKEN_GET_LOG — WoW Token transaction log.

    pub async fn handle_commerce_token_get_log(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match CommerceTokenGetLog::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "CommerceTokenGetLog parse failed: {error}"
                );
                return;
            }
        };

        // C++ has a TODO here and returns TOKEN_RESULT_SUCCESS with an empty
        // auctionable-token list while echoing the request integer.
        self.send_packet(&CommerceTokenGetLogResponse::success_empty(request.unk_int));
    }
}
