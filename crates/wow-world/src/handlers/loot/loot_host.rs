// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the loot handler contexts (#1263 F5).
//!
//! The application crate owns the handlers and their contexts; the host lives
//! next to the loot handlers so the session aggregate only exposes what the
//! release context already needs.
//!
//! `CMSG_LOOT_UNIT` additionally lends the shell-only capabilities of the moved
//! `WorldSession::HandleLootOpcode` body (`Handlers/LootHandler.cpp:216`): the
//! session hub, the loot state, the represented corpse/loot-request/loot-open
//! transitions, the spell-cast and aura interruption and the item valuation
//! catalogs of the handler bundle. Every method delegates to the existing World
//! operation at the exact point the World body invoked it, so no World body is
//! duplicated and no step value is needed.

use std::future::Future;

use wow_core::ObjectGuid;
use wow_packet::packets::loot::LootResponse;
use wow_world_application::{LootHandlerCxLikeCpp, LootHandlerHostLikeCpp, LootReleaseCxLikeCpp};
use wow_world_core::session::{HubRef, ItemValuationCatalogsLikeCpp};
use wow_world_loot::{LootState, RepresentedCreatureLootStateLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl LootHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn loot_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> LootHandlerCxLikeCpp<'a> {
        let (loot, hub) = crate::session::split_loot_mut(self);
        LootHandlerCxLikeCpp::new(hub, loot)
    }

    fn loot_release_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> LootReleaseCxLikeCpp<'a> {
        self.loot_release_cx_like_cpp()
    }

    fn loot_unit_item_valuation_like_cpp<'c>(
        &self,
        catalogs: &'c SessionHandlerCatalogsLikeCpp,
    ) -> &'c ItemValuationCatalogsLikeCpp {
        catalogs.item_valuation.as_ref()
    }

    fn loot_unit_hub_ref_like_cpp(&self) -> HubRef<'_> {
        crate::session::hub_ref(self)
    }

    fn loot_unit_loot_ref_like_cpp(&self) -> &LootState {
        &self.loot
    }

    fn loot_unit_loot_mut_like_cpp(&mut self) -> &mut LootState {
        &mut self.loot
    }

    fn loot_unit_represented_creature_loot_state_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<RepresentedCreatureLootStateLikeCpp> {
        WorldSession::represented_creature_loot_state_like_cpp(self, guid)
    }

    fn loot_unit_interrupt_non_melee_spell_cast_like_cpp(&mut self) {
        let _ = WorldSession::interrupt_non_melee_spell_cast_for_loot_like_cpp(self);
    }

    fn loot_unit_remove_auras_with_looting_interrupt_flags_like_cpp(&mut self) {
        let _ = WorldSession::remove_auras_with_looting_interrupt_flags_like_cpp(self);
    }

    fn loot_unit_ae_loot_creature_targets_like_cpp<'a>(
        &'a mut self,
        main_loot_target: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = Vec<ObjectGuid>> + Send + 'a {
        WorldSession::represented_ae_loot_creature_targets_like_cpp(
            self,
            main_loot_target,
            player_guid,
        )
    }

    fn loot_unit_loot_response_for_owner_like_cpp<'a>(
        &'a mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        ae_looting: bool,
    ) -> impl Future<Output = Option<LootResponse>> + Send + 'a {
        WorldSession::represented_loot_response_for_owner_like_cpp(
            self,
            owner_guid,
            player_guid,
            ae_looting,
        )
    }

    fn loot_unit_release_all_like_cpp<'a>(
        &'a mut self,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = ()> + Send + 'a {
        WorldSession::do_loot_release_all_like_cpp(self, player_guid)
    }

    fn loot_unit_on_loot_opened_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        response: LootResponse,
    ) {
        WorldSession::represented_on_loot_opened_with_catalogs_like_cpp(
            self,
            item_valuation,
            owner_guid,
            player_guid,
            response,
        )
    }
}
