// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the application StorageMove owner (#1263 F4 remate).
//!
//! The application crate owns C++ `Player::SwapItem`'s store/bank transitions
//! (`Entities/Player/Player.cpp:12423`) and their inventory persistence
//! (`Player.cpp:19632`); this session only lends what that body cannot borrow:
//! the condition projection the plan composition reads, the borrowed Inventory,
//! persistence, publication and Stats participants, and the shell-only
//! capabilities (the four quest operations, the narrow obtain-spells executor
//! and the two cfg(test) observation records). Every capability delegates to the
//! existing World operation at the exact point the World body invoked it, so no
//! World body is duplicated and no step value is needed.

use std::future::Future;

use wow_constants::InventoryResult;
use wow_core::ObjectGuidGenerator;
use wow_entities::InventoryStorageMovePlanLikeCpp;
use wow_world_application::{
    InventoryStorageMoveCxLikeCpp, InventoryStorageMoveHostLikeCpp,
    InventoryStorageMoveStatsCxLikeCpp, InventoryStorageTargetLikeCpp,
};
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

use crate::session::WorldSession;

impl InventoryStorageMoveHostLikeCpp for WorldSession {
    fn storage_move_plan_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        target: InventoryStorageTargetLikeCpp,
    ) -> Option<Result<InventoryStorageMovePlanLikeCpp, InventoryResult>> {
        let conditions = self.player_condition_projection_cx_like_cpp();
        let planning = wow_world_application::InventoryMovePlanningCxLikeCpp::new(&conditions);
        let access = self.core.owned_inventory_access_like_cpp();
        wow_world_application::plan_inventory_storage_move_like_cpp(
            &planning,
            &self.inventory,
            &access,
            self.catalogs.items.store.as_ref(),
            self.catalogs.items.stats_store.as_ref(),
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            target,
        )
    }

    fn storage_move_cx_like_cpp(&mut self) -> InventoryStorageMoveCxLikeCpp<'_> {
        let (inventory, lifecycle, hub) =
            crate::session::split_inventory_lifecycle_shared_hub_mut(self);
        let item_sets = hub.core.owned_item_set_access_like_cpp(
            hub.catalogs.items.set_store.as_deref(),
            hub.catalogs.spell_catalogs.item_set_spell_store.as_deref(),
            hub.catalogs.spell_catalogs.spell_store.as_deref(),
            hub.catalogs.heirloom_store.as_deref(),
            hub.catalogs.items.stats_store.as_deref(),
            hub.catalogs.curve_store.as_deref(),
            hub.catalogs.curve_point_store.as_deref(),
            hub.catalogs.content_tuning_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            &hub.fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_records_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &hub.fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &hub.fixtures
                .progression
                .represented_primary_specialization_id_like_cpp,
        );
        wow_world_application::InventoryStorageMoveCxLikeCpp::new(
            inventory,
            hub,
            hub.core.owned_inventory_access_like_cpp(),
            hub.core.owned_item_modifiers_access_like_cpp(),
            item_sets,
            hub.core.packet_publication_access_like_cpp(),
            lifecycle,
            hub.catalogs.items.store.as_ref(),
            hub.catalogs.items.stats_store.as_ref(),
            hub.catalogs.items.effect_store.as_ref(),
            hub.catalogs
                .spell_catalogs
                .spell_item_enchantment_store
                .as_deref(),
            wow_world_inventory::ItemModsCatalogsViewLikeCpp::new(
                hub.catalogs.items.store.as_ref(),
                hub.catalogs.items.stats_store.as_ref(),
                hub.catalogs.scaling_stat_distribution_store.as_ref(),
                hub.catalogs.scaling_stat_values_store.as_ref(),
                hub.catalogs.shield_block_regular_game_table.as_ref(),
                hub.catalogs.spell_catalogs.spell_shapeshift_form_store(),
            ),
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_application::InventoryEquipFixtureRefsLikeCpp::new(
                &hub.fixtures.identity.player_level,
                &hub.fixtures.auras.represented_shapeshift_form_like_cpp,
                &hub.fixtures.auras.visible_auras,
            ),
            cfg!(test),
        )
    }

    fn storage_move_stats_cx_like_cpp(&mut self) -> InventoryStorageMoveStatsCxLikeCpp<'_> {
        #[cfg(any(test, feature = "test-fixtures"))]
        let stats = self.core.player_stats_access_with_fixture_refs_like_cpp(
            &self.catalogs,
            &self.config,
            &self.fixtures.identity.player_race,
            &self.fixtures.identity.player_class,
            &self.fixtures.identity.player_level,
            wow_world_core::session::StatsFixtureRefs::new_like_cpp(
                wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
                    &mut self.fixtures.combat.player_health_like_cpp,
                    &mut self.fixtures.combat.player_max_health_like_cpp,
                    &mut self.fixtures.combat.player_alive_like_cpp,
                    &mut self.fixtures.combat.represented_player_powers_like_cpp[0],
                    &mut self.fixtures.combat.represented_player_max_powers_like_cpp[0],
                    &mut self.fixtures.combat.represented_player_base_mana_like_cpp,
                ),
                wow_world_core::session::StatsAuraFixtureRefs::new_like_cpp(
                    &self.fixtures.auras.represented_shapeshift_form_like_cpp,
                    &self.fixtures.auras.player_aura_authority_complete_like_cpp,
                    &self
                        .fixtures
                        .auras
                        .player_spell_hit_aura_authority_tombstoned_like_cpp,
                    &self.fixtures.auras.visible_auras,
                    &self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                ),
            ),
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let stats = self
            .core
            .player_stats_access_like_cpp(&self.catalogs, &self.config);
        wow_world_application::InventoryStorageMoveStatsCxLikeCpp::new(
            &mut self.inventory,
            self.core.owned_inventory_access_like_cpp(),
            self.core.packet_publication_access_like_cpp(),
            stats,
        )
    }

    fn storage_move_quest_log_item_id_like_cpp<'a>(
        &'a mut self,
        entry_id: u32,
    ) -> impl Future<Output = u32> + Send + 'a {
        async move { WorldSession::quest_source_item_quest_log_item_id_like_cpp(self, entry_id).await }
    }

    fn storage_move_plan_quest_statuses_like_cpp(
        &self,
        entry_id: u32,
        quest_log_item_id: u32,
        moving_to_bank: bool,
        post_move_non_bank_count: u32,
        added_count: u32,
    ) -> Vec<wow_entities::PlayerQuestStatusRecord> {
        WorldSession::plan_bank_item_quest_persistence_like_cpp(
            self,
            entry_id,
            quest_log_item_id,
            moving_to_bank,
            post_move_non_bank_count,
            added_count,
        )
    }

    fn storage_move_apply_quest_item_removed_like_cpp(
        &mut self,
        entry_id: u32,
    ) -> Option<Vec<u32>> {
        WorldSession::apply_quest_item_removed_like_cpp(self, entry_id)
    }

    fn storage_move_apply_quest_item_added_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a ObjectGuidGenerator,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> impl Future<Output = Vec<u32>> + Send + 'a {
        async move {
            WorldSession::apply_quest_item_added_objective_progress_with_generator_like_cpp(
                self,
                item_guid_generator,
                entry_id,
                quest_log_item_id,
                count,
            )
            .await
        }
    }

    fn storage_move_apply_obtain_spells_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a ObjectGuidGenerator,
        creature_spawn_catalogs: &'a CreatureSpawnCatalogsLikeCpp,
        entry_id: u32,
    ) -> impl Future<Output = ()> + Send + 'a {
        async move {
            let _ = WorldSession::apply_inventory_item_obtain_spells_with_generator_like_cpp(
                self,
                item_guid_generator,
                creature_spawn_catalogs,
                entry_id,
            )
            .await;
        }
    }

    fn storage_move_record_titan_grip_penalty_action_like_cpp(&mut self) {
        WorldSession::record_represented_titan_grip_penalty_action_like_cpp(self);
    }

    fn storage_move_record_avg_equipped_item_level_update_like_cpp(&mut self) {
        WorldSession::record_represented_avg_equipped_item_level_update_like_cpp(self);
    }
}

#[cfg(test)]
impl WorldSession {
    /// Test entry point for the moved plan composition. The production caller is
    /// the application owner through the host seam above.
    pub(crate) fn plan_inventory_storage_move_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        target: InventoryStorageTargetLikeCpp,
    ) -> Option<Result<InventoryStorageMovePlanLikeCpp, InventoryResult>> {
        <WorldSession as InventoryStorageMoveHostLikeCpp>::storage_move_plan_like_cpp(
            self,
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            target,
        )
    }
}
