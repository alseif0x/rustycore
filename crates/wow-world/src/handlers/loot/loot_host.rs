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
//!
//! `CMSG_LOOT_ROLL` (`Handlers/LootHandler.cpp:489`) and
//! `CMSG_MASTER_LOOT_ITEM` (`WorldSession::HandleLootMasterGiveOpcode`) reuse
//! the same session hub, loot state and item valuation projections and add the
//! shell-only capabilities their bodies need: the represented roll vote, the
//! group resolution, the master-loot inventory probe, the fixture gate, the
//! authority preparation/reconciliation, the item storage transitions and the
//! `LootRemoved` publication. Each one still delegates to the existing World
//! operation at its original call point.
//!
//! `CMSG_LOOT_ITEM` (`WorldSession::HandleAutostoreLootItemOpcode`,
//! `Handlers/LootHandler.cpp:77`) reuses those same projections (the hub, the
//! loot state and its mut accessor, the authority preparation/reconciliation,
//! the fixture gate, the item storage transitions and the item generator) and
//! adds only the GameObject autostore gate, the corpse position, the canonical
//! summary refresh, the durable-completion drain and the shared-owner
//! `LootRemoved` publication, again as one-line delegations to the existing
//! World operation at its original call point.
//!
//! The `LootMoney` **command receivers** (`Handlers/LootHandler.cpp` money
//! path) join the application loot owner with the mutable release-owner access
//! they refresh the loot summary through and the durable payout that mutates
//! canonical money, the quest objectives and `SMSG_LOOT_MONEY_NOTIFY`. The
//! `CMSG_LOOT_MONEY` consumer itself stays in the shell for a later slice.

use std::future::Future;

use wow_core::ObjectGuid;
use wow_loot::{LootClaimLease, LootEntry, OwnedLootAuthority};
use wow_packet::packets::loot::{LootResponse, LootRoll};
use wow_world_application::{LootHandlerCxLikeCpp, LootHandlerHostLikeCpp, LootReleaseCxLikeCpp};
use wow_world_core::session::mailbox::ApplyLootMoneyResultLikeCpp;
use wow_world_core::session::{
    HubRef, ItemValuationCatalogsLikeCpp, OwnedLootAuthorityLookupOutcomeLikeCpp,
};
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
    fn loot_roll_item_guid_generator_like_cpp<'c>(
        &self,
        catalogs: &'c SessionHandlerCatalogsLikeCpp,
    ) -> &'c wow_core::ObjectGuidGenerator {
        catalogs.id_generators.item.as_ref()
    }

    fn loot_roll_player_vote_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
        item_valuation: &'a ItemValuationCatalogsLikeCpp,
        roll: &'a LootRoll,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = bool> + Send + 'a {
        WorldSession::represented_player_vote_on_loot_roll_with_generator_like_cpp(
            self,
            item_guid_generator,
            item_valuation,
            roll,
            player_guid,
        )
    }

    fn master_loot_resolved_group_guid_like_cpp(&self) -> Option<u64> {
        WorldSession::resolved_group_guid_like_cpp(self)
    }

    fn master_loot_local_fixture_allowed_like_cpp(&self) -> bool {
        super::represented_local_loot_fixture_allowed_like_cpp()
    }

    fn master_loot_prepare_owned_authority_outcome_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        scope_player: ObjectGuid,
    ) -> OwnedLootAuthorityLookupOutcomeLikeCpp {
        WorldSession::prepare_owned_loot_authority_for_active_request_outcome_like_cpp(
            self,
            owner_guid,
            scope_player,
        )
    }

    fn master_loot_reconcile_loot_cache_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        WorldSession::reconcile_represented_loot_cache_like_cpp(self, owner_guid, player_guid)
    }

    fn master_loot_can_store_error_like_cpp(
        &self,
        target: ObjectGuid,
        item_id: u32,
        count: u32,
    ) -> Option<u8> {
        WorldSession::represented_master_loot_can_store_error_like_cpp(self, target, item_id, count)
    }

    fn master_loot_store_claimed_direct_item_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
        loot_entry: &'a LootEntry,
        dungeon_encounter_id: u32,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        claim: &'a LootClaimLease,
    ) -> impl Future<Output = bool> + Send + 'a {
        WorldSession::store_claimed_direct_loot_item_from_owner_with_generator_like_cpp(
            self,
            item_guid_generator,
            loot_entry,
            dungeon_encounter_id,
            owner_guid,
            loot_obj,
            claim,
        )
    }

    fn master_loot_store_direct_item_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
        loot_entry: &'a LootEntry,
        dungeon_encounter_id: u32,
        owner_guid: ObjectGuid,
    ) -> impl Future<Output = bool> + Send + 'a {
        WorldSession::store_direct_loot_item_from_owner_with_generator_like_cpp(
            self,
            item_guid_generator,
            loot_entry,
            dungeon_encounter_id,
            owner_guid,
        )
    }

    fn master_loot_mark_item_removed_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        target: ObjectGuid,
    ) {
        WorldSession::mark_represented_master_loot_item_removed_like_cpp(
            self,
            owner_guid,
            loot_obj,
            loot_list_id,
            target,
        )
    }

    fn loot_item_gameobject_can_autostore_like_cpp(
        &self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        WorldSession::represented_gameobject_can_autostore_loot_item_like_cpp(
            self,
            owner_guid,
            player_guid,
        )
    }

    fn loot_item_represented_creature_position_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<wow_core::Position> {
        WorldSession::represented_creature_position_for_loot_like_cpp(self, owner_guid)
    }

    fn loot_item_apply_pending_durable_completions_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send + 'a {
        WorldSession::apply_pending_durable_item_loot_completions_with_generator_like_cpp(
            self,
            item_guid_generator,
        )
    }

    fn loot_item_refresh_owner_canonical_summary_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        WorldSession::refresh_represented_loot_owner_canonical_summary_like_cpp(
            self,
            owner_guid,
            player_guid,
        )
    }

    fn loot_item_notify_item_removed_like_cpp(&mut self, owner_guid: ObjectGuid, loot_list_id: u8) {
        WorldSession::represented_notify_loot_item_removed_like_cpp(self, owner_guid, loot_list_id)
    }

    fn loot_money_release_owner_access_like_cpp(
        &mut self,
    ) -> wow_world_core::session::LootReleaseOwnerAccessLikeCpp<'_> {
        self.core.loot_release_owner_access_like_cpp()
    }

    fn loot_money_apply_durable_payout_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
        notified_amount: u64,
        durable_applied_amount: u64,
        sole_looter: bool,
        apply_money: bool,
        publish: bool,
    ) -> impl Future<Output = ApplyLootMoneyResultLikeCpp> + Send + 'a {
        WorldSession::apply_durable_represented_loot_money_payout_like_cpp(
            self,
            item_guid_generator,
            notified_amount,
            durable_applied_amount,
            sole_looter,
            apply_money,
            publish,
        )
    }
}
