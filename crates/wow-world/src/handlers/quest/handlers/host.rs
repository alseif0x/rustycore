// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the quest-giver interaction registration family
//! (#1263 F5 remaining families).
//!
//! The application crate owns the eleven registrations that used to be
//! submitted through the legacy `inventory::submit!` path at the end of this
//! module's parent. This adapter lends that registrar the session operations it
//! needs and nothing else: every method delegates to the existing
//! `WorldSession` operation the legacy registration closure invoked, so no
//! handler body is duplicated and the catalog view is destructured exactly
//! where the closure destructured it.
//!
//! C++ anchors are recorded once, with the registrar, in
//! `wow_world_application::quest_handlers`.

use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
use wow_packet::packets::query::QuestPoiQuery;
use wow_world_application::QuestHandlerHostLikeCpp;

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl QuestHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn handle_adventure_map_start_quest_with_catalog_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_adventure_map_start_quest_with_catalog_like_cpp(
                self,
                catalogs.adventure_map_pois.as_ref(),
                pkt,
            )
            .await
        })
    }

    fn handle_quest_giver_status_query_with_catalog_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_quest_giver_status_query_with_catalog_like_cpp(
                self,
                catalogs.quest_info.as_ref(),
                pkt,
            )
            .await
        })
    }

    fn handle_quest_giver_hello<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_quest_giver_hello(self, pkt).await })
    }

    fn handle_quest_giver_query_quest<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_quest_giver_query_quest(self, pkt).await })
    }

    fn handle_quest_giver_accept_quest_with_generator_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_quest_giver_accept_quest_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                pkt,
            )
            .await
        })
    }

    fn handle_quest_poi_query<'a>(&'a mut self, query: QuestPoiQuery) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_quest_poi_query(self, query).await })
    }

    fn handle_quest_giver_request_reward_with_generator_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_quest_giver_request_reward_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                pkt,
            )
            .await
        })
    }

    fn handle_quest_giver_choose_reward_with_generator_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_quest_giver_choose_reward_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                pkt,
            )
            .await
        })
    }

    fn handle_quest_confirm_accept_with_generator_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_quest_confirm_accept_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                pkt,
            )
            .await
        })
    }

    fn handle_push_quest_to_party<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_push_quest_to_party(self, pkt).await })
    }
}
