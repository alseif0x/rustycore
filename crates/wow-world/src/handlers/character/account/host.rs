// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the character/account registration family (#1263 F5).
//!
//! The application crate owns the 23 registrations that used to be submitted
//! through the legacy `inventory::submit!` path in
//! `crates/wow-world/src/handlers/character/account/registrations/`. This module
//! lends that registrar the session operations it needs and nothing else: every
//! method delegates to the existing `WorldSession` operation the legacy
//! registration closure invoked, so no handler body is duplicated and the
//! catalog view is destructured exactly where the closure destructured it.
//!
//! C++ anchors are recorded once, with the registrar, in
//! `wow_world_application::character_account_handlers`.

use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
use wow_packet::packets::{gossip, item, misc};
use wow_world_application::CharacterAccountHandlerHostLikeCpp;

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl CharacterAccountHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn list_inventory_like_cpp<'a>(&'a mut self, hello: gossip::Hello) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_list_inventory(self, hello).await })
    }

    fn buy_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        buy: misc::BuyItem,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_buy_item_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                buy,
            )
            .await
        })
    }

    fn buy_back_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        buyback: misc::BuyBackItem,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_buy_back_item_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                buyback,
            )
            .await
        })
    }

    fn sell_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        sell: misc::SellItem,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_sell_item_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                sell,
            )
            .await
        })
    }

    fn item_purchase_refund_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        refund: item::ItemPurchaseRefund,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_item_purchase_refund_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                refund,
            )
            .await
        })
    }

    fn banker_activate_like_cpp<'a>(&'a mut self, hello: gossip::Hello) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_banker_activate(self, hello).await })
    }

    fn autobank_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        packet: misc::AutoBankItem,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_autobank_item_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                catalogs.creature_spawns.as_ref(),
                packet,
            )
            .await
        })
    }

    fn autostore_bank_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        packet: misc::AutoStoreBankItem,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_autostore_bank_item_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                catalogs.creature_spawns.as_ref(),
                packet,
            )
            .await
        })
    }

    fn buy_bank_slot_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        buy: misc::BuyBankSlot,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_buy_bank_slot_with_prices_and_generator_like_cpp(
                self,
                catalogs.bank_bag_slot_prices.as_ref(),
                catalogs.id_generators.item.as_ref(),
                buy,
            )
            .await
        })
    }

    fn binder_activate_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        hello: gossip::Hello,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_binder_activate_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                catalogs.creature_spawns.as_ref(),
                hello,
            )
            .await
        })
    }

    fn hearth_and_resurrect_like_cpp<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_hearth_and_resurrect(self, pkt).await })
    }

    fn repair_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        repair: misc::RepairItem,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_repair_item_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                repair,
            )
            .await
        })
    }

    fn quest_giver_status_multiple_query_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_quest_giver_status_multiple_query_with_catalog_like_cpp(
                self,
                catalogs.quest_info.as_ref(),
            )
            .await
        })
    }

    fn quest_giver_status_tracked_query_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_quest_giver_status_tracked_query_with_catalog_like_cpp(
                self,
                catalogs.quest_info.as_ref(),
                pkt,
            )
            .await
        })
    }

    fn swap_inv_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        swap: item::SwapInvItem,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_swap_inv_item_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                catalogs.creature_spawns.as_ref(),
                swap,
            )
            .await
        })
    }

    fn auto_equip_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        equip: item::AutoEquipItem,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_auto_equip_item_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                catalogs.creature_spawns.as_ref(),
                equip,
            )
            .await
        })
    }

    fn auto_equip_item_slot_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        equip: item::AutoEquipItemSlot,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_auto_equip_item_slot_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                catalogs.creature_spawns.as_ref(),
                equip,
            )
            .await
        })
    }

    fn swap_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        swap: item::SwapItem,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_swap_item_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                catalogs.creature_spawns.as_ref(),
                swap,
            )
            .await
        })
    }

    fn auto_store_bag_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        store: item::AutoStoreBagItem,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_auto_store_bag_item_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                catalogs.creature_spawns.as_ref(),
                store,
            )
            .await
        })
    }

    fn destroy_item_like_cpp<'a>(
        &'a mut self,
        destroy: item::DestroyItemPkt,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_destroy_item(self, destroy).await })
    }

    fn gossip_hello_like_cpp<'a>(&'a mut self, hello: gossip::Hello) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_gossip_hello(self, hello).await })
    }

    fn gossip_select_option_like_cpp<'a>(
        &'a mut self,
        select: gossip::GossipSelectOption,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_gossip_select_option(self, select).await })
    }

    fn logout_request_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        req: misc::LogoutRequest,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_logout_request_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                req,
            )
            .await
        })
    }
}
