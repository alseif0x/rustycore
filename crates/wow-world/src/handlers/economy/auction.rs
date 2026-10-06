// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Auction handlers that remain in the World shell.
//!
//! The auction command handlers and their registration moved to
//! `wow-world-inventory` under #1263 F5. The auctioneer hello request stays
//! here because the character/account world-service family registers it, and
//! the scenario that drives the commerce-token log keeps a cfg(test) entry
//! point so it can exercise one handler without composing a dispatch table.

use tracing::info;
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;

impl crate::session::WorldSession {
    pub async fn handle_auction_hello_request(&mut self, mut pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::AuctionHelloResponse;
        let guid = pkt
            .read_packed_guid()
            .unwrap_or(wow_core::ObjectGuid::EMPTY);
        info!(
            "AuctionHelloRequest from {:?} account {}",
            guid, self.core.account_id
        );
        self.send_packet(&AuctionHelloResponse::open(guid));
    }
}

#[cfg(test)]
mod session_shims {
    use wow_packet::WorldPacket;
    use wow_packet::packets::misc::{
        AuctionListItems, AuctionPlaceBid, AuctionRemoveItem, AuctionReplicateItems,
        AuctionSellItem,
    };

    use crate::session::WorldSession;

    impl WorldSession {
        pub(crate) async fn handle_auction_list_items(
            &mut self,
            _packet: wow_packet::packets::misc::AuctionListItems,
        ) {
            self.build_auction_handler_cx_like_cpp()
                .handle_auction_list_items(_packet)
                .await;
        }

        pub(crate) async fn handle_auction_place_bid(&mut self, packet: AuctionPlaceBid) {
            self.build_auction_handler_cx_like_cpp()
                .handle_auction_place_bid(packet)
                .await;
        }

        pub(crate) async fn handle_auction_remove_item(&mut self, packet: AuctionRemoveItem) {
            self.build_auction_handler_cx_like_cpp()
                .handle_auction_remove_item(packet)
                .await;
        }

        pub(crate) async fn handle_auction_sell_item(&mut self, packet: AuctionSellItem) {
            self.build_auction_handler_cx_like_cpp()
                .handle_auction_sell_item(packet)
                .await;
        }

        pub(crate) async fn handle_auction_replicate_items(
            &mut self,
            packet: AuctionReplicateItems,
        ) {
            self.build_auction_handler_cx_like_cpp()
                .handle_auction_replicate_items(packet)
                .await;
        }

        pub(crate) async fn handle_auctionable_token_sell(
            &mut self,
            mut pkt: wow_packet::WorldPacket,
        ) {
            self.build_auction_handler_cx_like_cpp()
                .handle_auctionable_token_sell(pkt)
                .await;
        }

        pub(crate) async fn handle_auctionable_token_sell_at_market_price(
            &mut self,
            mut pkt: wow_packet::WorldPacket,
        ) {
            self.build_auction_handler_cx_like_cpp()
                .handle_auctionable_token_sell_at_market_price(pkt)
                .await;
        }

        pub(crate) async fn handle_commerce_token_get_log(
            &mut self,
            mut pkt: wow_packet::WorldPacket,
        ) {
            self.build_auction_handler_cx_like_cpp()
                .handle_commerce_token_get_log(pkt)
                .await;
        }
    }
}
