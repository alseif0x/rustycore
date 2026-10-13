// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the loot bodies moved to `wow_world_loot_ops` (#1363 5a-5c);
//! each forwards with this session as host, outside the loot hotspot's test lines.

#![cfg(test)]

use std::collections::HashMap;

use wow_core::ObjectGuid;
pub(crate) use wow_entities::GameObjectLootSource;
use wow_entities::Item;
use wow_loot::LootConditionRowLikeCpp;
pub(crate) use wow_packet::packets::loot::CreatureLoot;
use wow_world_core::session::OwnedLootAuthorityLookupOutcomeLikeCpp;
use wow_world_core::session::mailbox::ApplyLootMoneyLikeCppCommand;
use wow_world_loot::RepresentedCreatureLootStateLikeCpp;
pub(crate) use wow_world_loot::storage_plans::LootItemClaimCommitContextLikeCpp;
use wow_world_loot_ops::LootHandlerHostLikeCpp;

use crate::session::WorldSession;

// The loot unit tests name these through `super::*` of the World loot module.
// #1363 slice 5b: the names the moved roll, request, storage, persistence and
// fanout bodies no longer import in the World loot module.
pub(crate) use rand::Rng;
pub(crate) use wow_constants::ClientOpcodes;
pub(crate) use wow_entities::GatheringNodeUseSource;
pub(crate) use wow_entities::{INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START};
pub(crate) use wow_loot::{
    LOOT_METHOD_ROUND_ROBIN_LIKE_CPP, LootClaimLease, OwnedLootAuthority, ROLL_VOTE_GREED_LIKE_CPP,
    ROLL_VOTE_NEED_LIKE_CPP, ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP, ROLL_VOTE_NOT_VALID_LIKE_CPP,
    ROLL_VOTE_PASS_LIKE_CPP, creature_loot_is_allowed_to_player_like_cpp,
    mark_loot_item_looted_for_player_like_cpp,
};
pub(crate) use wow_loot::{
    loot_is_looted_like_cpp, mark_loot_allowed_for_player_like_cpp,
    prepare_represented_shared_loot_generation_like_cpp,
    rebuild_represented_personal_loot_counts_like_cpp,
    rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp,
};
pub(crate) use wow_packet::packets::loot::{LootEntry, LootResponse};
pub(crate) use wow_persistence::StoredItemMoneyReconciliationLikeCpp;
pub(crate) use wow_world_core::session::mailbox::{
    SyncChestGameobjectStateAndRefreshLikeCppCommand,
    SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    SyncGooberGameobjectStateAndRefreshLikeCppCommand,
};
pub(crate) use wow_world_core::session::state::hub_support::RepresentedLootPlayerContext;
pub(crate) use wow_world_lifecycle::loot_delivery_contracts::loot_money_durable_outcome_like_cpp;
pub(crate) use wow_world_lifecycle::loot_template_rules::ItemTemplateAddonLootMetadataLikeCpp;
pub(crate) use wow_world_loot::{
    LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP, LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP,
    LootStoreRandomProperties, loot_type_for_client_like_cpp,
    represented_loot_response_items_like_cpp,
};
pub(crate) use wow_world_loot::{loot_item_context, start_loot_roll_packet_like_cpp};
pub(crate) use wow_world_loot_ops::{
    LOOT_MODE_DEFAULT_LIKE_CPP, LOOT_MODE_JUNK_FISH_LIKE_CPP, LootClaimPersistenceWorkerError,
    RepresentedGameObjectLootInstallObservationLikeCpp, disenchant_loot_template_frame_like_cpp,
    generated_creature_loot_item_to_entry_like_cpp,
    generated_shared_gameobject_loot_item_to_entry_like_cpp,
    master_loot_error_for_inventory_result_like_cpp,
    record_represented_gameobject_chest_release_metadata_like_cpp,
    represented_disenchant_loot_plain_row_can_roll_like_cpp,
    represented_disenchant_loot_reference_row_can_roll_like_cpp,
    represented_gameobject_loot_install_observation_like_cpp,
    represented_loot_object_guid_like_cpp, spawn_loot_claim_persistence_worker_like_cpp,
    spawn_loot_item_persistence_worker_like_cpp,
};

impl WorldSession {
    pub(crate) async fn ensure_represented_creature_kill_loot_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
    ) {
        wow_world_loot_ops::ensure_represented_creature_kill_loot_like_cpp(self, creature_guid)
            .await
    }

    pub(crate) fn evaluate_creature_loot_condition_for_player_like_cpp_representable(
        &self,
        condition: &LootConditionRowLikeCpp,
        player_context: &RepresentedLootPlayerContext,
    ) -> Option<bool> {
        wow_world_loot_ops::evaluate_creature_loot_condition_for_player_like_cpp_representable(
            self,
            condition,
            player_context,
        )
    }

    pub(crate) async fn generate_represented_creature_loot_items_for_player_like_cpp(
        &mut self,
        loot_id: u32,
        player_guid: ObjectGuid,
    ) -> Option<Vec<LootEntry>> {
        wow_world_loot_ops::generate_represented_creature_loot_items_for_player_like_cpp(
            self,
            loot_id,
            player_guid,
        )
        .await
    }

    pub(crate) async fn generate_represented_creature_loot_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        loot_owner_guid: ObjectGuid,
        _level: u8,
        entry: u32,
        loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        dungeon_encounter_id: u32,
    ) -> Option<CreatureLoot> {
        wow_world_loot_ops::generate_represented_creature_loot_like_cpp(
            self,
            creature_guid,
            loot_owner_guid,
            _level,
            entry,
            loot_id,
            gold_min,
            gold_max,
            dungeon_encounter_id,
        )
        .await
    }

    pub(crate) fn has_incomplete_quest_objective_for_item_like_cpp(&self, item_id: u32) -> bool {
        wow_world_loot_ops::has_incomplete_quest_objective_for_item_like_cpp(self, item_id)
    }

    pub(crate) fn item_loot_quest_status_allows_for_player_like_cpp(
        &self,
        item_id: u32,
        needs_quest: bool,
        addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
        player_context: &RepresentedLootPlayerContext,
    ) -> bool {
        wow_world_loot_ops::item_loot_quest_status_allows_for_player_like_cpp(
            self,
            item_id,
            needs_quest,
            addon_metadata,
            player_context,
        )
    }

    pub(crate) fn item_loot_quest_status_allows_like_cpp(
        &self,
        item_id: u32,
        needs_quest: bool,
        addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
    ) -> bool {
        wow_world_loot_ops::item_loot_quest_status_allows_like_cpp(
            self,
            item_id,
            needs_quest,
            addon_metadata,
        )
    }

    pub(crate) fn next_canonical_loot_object_guid_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        wow_world_loot_ops::next_canonical_loot_object_guid_like_cpp(self, owner_guid)
    }

    pub(crate) fn next_represented_loot_object_guid_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        wow_world_loot_ops::next_represented_loot_object_guid_like_cpp(self, owner_guid)
    }

    pub(crate) fn prepare_owned_loot_authority_for_active_request_outcome_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        scope_player: ObjectGuid,
    ) -> OwnedLootAuthorityLookupOutcomeLikeCpp {
        wow_world_loot_ops::prepare_owned_loot_authority_for_active_request_outcome_like_cpp(
            self,
            owner_guid,
            scope_player,
        )
    }

    pub(crate) fn reconcile_represented_loot_cache_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        wow_world_loot_ops::reconcile_represented_loot_cache_like_cpp(self, owner_guid, player_guid)
    }

    pub(crate) fn refresh_represented_loot_owner_canonical_summary_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        wow_world_loot_ops::refresh_represented_loot_owner_canonical_summary_like_cpp(
            self,
            owner_guid,
            player_guid,
        )
    }

    pub(crate) fn represented_creature_loot_state_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<RepresentedCreatureLootStateLikeCpp> {
        wow_world_loot_ops::represented_creature_loot_state_like_cpp(self, guid)
    }

    pub(crate) fn represented_gameobject_chest_group_state_like_cpp(
        &self,
        use_group_loot_rules: bool,
        _player_guid: ObjectGuid,
    ) -> (u8, ObjectGuid, ObjectGuid) {
        wow_world_loot_ops::represented_gameobject_chest_group_state_like_cpp(
            self,
            use_group_loot_rules,
            _player_guid,
        )
    }

    pub(crate) fn represented_group_looters_at_reward_distance_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        wow_world_loot_ops::represented_group_looters_at_reward_distance_like_cpp(self, player_guid)
    }

    pub(crate) fn represented_loot_authority_pools_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot: CreatureLoot,
        personal: bool,
    ) -> Option<(Option<CreatureLoot>, HashMap<ObjectGuid, CreatureLoot>)> {
        wow_world_loot_ops::represented_loot_authority_pools_like_cpp(
            self,
            owner_guid,
            player_guid,
            loot,
            personal,
        )
    }

    pub(crate) fn represented_owned_loot_authority_outcome_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> OwnedLootAuthorityLookupOutcomeLikeCpp {
        wow_world_loot_ops::represented_owned_loot_authority_outcome_like_cpp(self, owner_guid)
    }

    pub(crate) fn sync_represented_creature_loot_to_canonical_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> Option<()> {
        wow_world_loot_ops::sync_represented_creature_loot_to_canonical_like_cpp(
            self,
            creature_guid,
            player_guid,
        )
    }

    pub(crate) fn generate_loot_store_random_properties_with_rng_like_cpp<R: rand::Rng + ?Sized>(
        &self,
        item_id: u32,
        rng: &mut R,
    ) -> wow_world_loot::LootStoreRandomProperties {
        wow_world_loot_ops::generate_loot_store_random_properties_with_rng_like_cpp(
            self, item_id, rng,
        )
    }
}

// #1363 slice 5b: the rolls, requests, item storage, persistence and fanout
// bodies moved to `wow_world_loot_ops`.
impl WorldSession {
    pub(crate) async fn tick_represented_loot_rolls_like_cpp(&mut self) {
        let generators = self.id_generators_for_test_like_cpp();
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        wow_world_loot_ops::tick_represented_loot_rolls_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            &item_valuation,
        )
        .await;
    }

    pub(crate) async fn apply_pending_durable_item_loot_completions_like_cpp(&mut self) {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_loot_ops::apply_pending_durable_item_loot_completions_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
        )
        .await;
    }

    pub(crate) async fn wait_for_active_loot_persistence_like_cpp(&mut self) {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_loot_ops::wait_for_active_loot_persistence_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
        )
        .await;
    }

    pub(crate) fn close_stale_active_loot_view_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        wow_world_loot_ops::close_stale_active_loot_view_like_cpp(self, owner_guid, player_guid)
    }

    pub(crate) async fn represented_loot_response_for_owner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        ae_looting: bool,
    ) -> Option<LootResponse> {
        wow_world_loot_ops::represented_loot_response_for_owner_like_cpp(
            self,
            owner_guid,
            player_guid,
            ae_looting,
        )
        .await
    }

    pub(crate) fn send_loot_item_push_result(
        &self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
        loot_entry: &LootEntry,
        random_properties_id: i32,
        random_properties_seed: i32,
        slot: u8,
        quantity: u32,
        quantity_in_inventory: u32,
        created: bool,
        dungeon_encounter_id: u32,
    ) {
        wow_world_loot_ops::send_loot_item_push_result(
            self,
            player_guid,
            item_guid,
            loot_entry,
            random_properties_id,
            random_properties_seed,
            slot,
            quantity,
            quantity_in_inventory,
            created,
            dungeon_encounter_id,
        )
    }

    pub(crate) fn stored_existing_item_dynamic_flags_like_cpp(
        &self,
        item_id: u32,
        slot: u8,
        existing: &Item,
    ) -> u32 {
        wow_world_loot_ops::stored_existing_item_dynamic_flags_like_cpp(
            self, item_id, slot, existing,
        )
    }

    pub(crate) fn stored_new_item_dynamic_flags_like_cpp(&self, item_id: u32, slot: u8) -> u32 {
        wow_world_loot_ops::stored_new_item_dynamic_flags_like_cpp(self, item_id, slot)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn request_represented_remote_loot_roll_winner_store_like_cpp(
        &self,
        target: ObjectGuid,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        dungeon_encounter_id: u32,
        entries: Vec<LootEntry>,
        is_disenchant: bool,
        claim: Option<wow_loot::LootClaimLease>,
    ) -> wow_world_core::session::mailbox::MasterLootGiveResult {
        wow_world_loot_ops::request_represented_remote_loot_roll_winner_store_like_cpp(
            self,
            target,
            owner_guid,
            loot_obj,
            loot_list_id,
            dungeon_encounter_id,
            entries,
            is_disenchant,
            claim,
        )
        .await
    }
}

// #1363 slice 5c: the two `CMSG_LOOT_MONEY`/apply-loot-money test entry points
// that lived beside the moved `handlers/loot/handlers/money.rs` body.
impl WorldSession {
    /// Test entry point for `CMSG_LOOT_MONEY`. It dispatches through the
    /// **production registration** (`PacketHandlerEntry` for
    /// `ClientOpcodes::LootMoney`) instead of re-implementing the body, so every
    /// caller exercises the registered consumer and the application-owned
    /// operation dependencies it now reaches (#1263 F4).
    pub(crate) async fn handle_loot_money(&mut self, pkt: wow_packet::WorldPacket) {
        let mut catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
        catalogs.id_generators = std::sync::Arc::new(self.id_generators_for_test_like_cpp());
        let entry = crate::session::registry::registered_handler_entries_like_cpp()
            .find(|entry| entry.opcode == ClientOpcodes::LootMoney)
            .expect("registered loot money handler");
        (entry.handler)(self, &catalogs, pkt).await;
    }

    /// Test-only receiver entry point. The receiver itself now lives in the
    /// application loot owner; this shim only supplies the session generator
    /// bundle the pre-move entry point built, and has three test callers.
    pub(crate) async fn handle_apply_loot_money_like_cpp_command(
        &mut self,
        command: ApplyLootMoneyLikeCppCommand,
    ) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_apply_loot_money_with_generator_like_cpp_command(
            generators.item.as_ref(),
            command,
        )
        .await;
    }
}
