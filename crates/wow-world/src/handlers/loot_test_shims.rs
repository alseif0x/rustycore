// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the loot-source bodies moved to
//! `wow_world_loot_ops` (#1363 slice 5a). Each forwards to the moved body with
//! this session as its host, so the loot unit tests keep their method syntax
//! and the loot hotspot's test lines do not grow.

use std::collections::HashMap;

use wow_core::ObjectGuid;
pub(crate) use wow_entities::GameObjectLootSource;
use wow_loot::{LootConditionRowLikeCpp, LootEntry, OwnedLootAuthority};
pub(crate) use wow_packet::packets::loot::CreatureLoot;
use wow_world_core::session::OwnedLootAuthorityLookupOutcomeLikeCpp;
use wow_world_loot::RepresentedCreatureLootStateLikeCpp;

use crate::session::WorldSession;

// The loot unit tests name these through `super::*` of the World loot module.
pub(crate) use rand::Rng;
pub(crate) use wow_entities::GatheringNodeUseSource;
pub(crate) use wow_loot::{
    mark_loot_allowed_for_player_like_cpp, prepare_represented_shared_loot_generation_like_cpp,
    rebuild_represented_personal_loot_counts_like_cpp,
    rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp,
};
pub(crate) use wow_world_core::session::mailbox::{
    SyncChestGameobjectStateAndRefreshLikeCppCommand,
    SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    SyncGooberGameobjectStateAndRefreshLikeCppCommand,
};
pub(crate) use wow_world_core::session::state::hub_support::RepresentedLootPlayerContext;
pub(crate) use wow_world_lifecycle::loot_template_rules::ItemTemplateAddonLootMetadataLikeCpp;
pub(crate) use wow_world_loot_ops::{
    LOOT_MODE_DEFAULT_LIKE_CPP, LOOT_MODE_JUNK_FISH_LIKE_CPP,
    RepresentedGameObjectLootInstallObservationLikeCpp, disenchant_loot_template_frame_like_cpp,
    generated_creature_loot_item_to_entry_like_cpp,
    generated_shared_gameobject_loot_item_to_entry_like_cpp,
    master_loot_error_for_inventory_result_like_cpp,
    record_represented_gameobject_chest_release_metadata_like_cpp,
    represented_disenchant_loot_plain_row_can_roll_like_cpp,
    represented_disenchant_loot_reference_row_can_roll_like_cpp,
    represented_gameobject_loot_install_observation_like_cpp,
    represented_loot_object_guid_like_cpp,
};

impl WorldSession {
    pub(crate) fn creature_loot_release_values_for_viewer_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        viewer_guid: ObjectGuid,
        viewer_has_pending_bind: bool,
        authority: Option<&OwnedLootAuthority>,
        update: wow_packet::packets::update::UnitDataValuesDeltaUpdate,
    ) -> wow_packet::packets::update::UnitDataValuesDeltaUpdate {
        wow_world_loot_ops::creature_loot_release_values_for_viewer_like_cpp(
            self,
            creature_guid,
            viewer_guid,
            viewer_has_pending_bind,
            authority,
            update,
        )
    }

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

    pub(crate) async fn generate_represented_gameobject_chest_loot_with_template_money_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: GameObjectLootSource,
        allowed_looters: &[ObjectGuid],
        template_money: (u32, u32),
    ) -> Option<CreatureLoot> {
        wow_world_loot_ops::generate_represented_gameobject_chest_loot_with_template_money_like_cpp(
            self,
            gameobject_guid,
            player_guid,
            source,
            allowed_looters,
            template_money,
        )
        .await
    }

    pub(crate) fn has_incomplete_quest_objective_for_item_like_cpp(&self, item_id: u32) -> bool {
        wow_world_loot_ops::has_incomplete_quest_objective_for_item_like_cpp(self, item_id)
    }

    pub(crate) fn install_represented_creature_kill_loot_if_current_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        expected_authority: &OwnedLootAuthority,
        expected_object_generation: u64,
        expected_loot_lifecycle_revision: u64,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> bool {
        wow_world_loot_ops::install_represented_creature_kill_loot_if_current_like_cpp(
            self,
            creature_guid,
            expected_authority,
            expected_object_generation,
            expected_loot_lifecycle_revision,
            shared,
            personal,
        )
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

    pub(crate) fn mark_represented_master_loot_item_removed_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        target: ObjectGuid,
    ) {
        wow_world_loot_ops::mark_represented_master_loot_item_removed_like_cpp(
            self,
            owner_guid,
            loot_obj,
            loot_list_id,
            target,
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

    pub(crate) fn upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
        observation: &RepresentedGameObjectLootInstallObservationLikeCpp,
    ) -> Option<()> {
        wow_world_loot_ops::upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(self, gameobject_guid, player_guid, loot, replace, observation)
    }

    pub(crate) fn upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
        discard_empty_pool: bool,
        observation: &RepresentedGameObjectLootInstallObservationLikeCpp,
    ) -> Option<()> {
        wow_world_loot_ops::upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp(self, gameobject_guid, player_guid, loot, replace, discard_empty_pool, observation)
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
