// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the loot-source bodies that moved to
//! `wow_world_loot_ops` (#1363 slice 5a). Each method forwards to the
//! same-named session field or inherent operation; the fixture predicate keeps
//! this crate's `cfg!(test)`.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use wow_social::group::GroupRegistry;
use wow_world_core::session::directory::PlayerRegistry;
use wow_world_core::session::{HubMut, HubRef};
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_loot_ops::LootSessionHostLikeCpp;

use super::*;
use wow_loot::LootStores;
use wow_world_core::session_policy::LootDropRatesLikeCpp;
use wow_world_inventory::InventoryState;

impl LootSessionHostLikeCpp for WorldSession {
    fn loot(&self) -> &wow_world_loot::LootState {
        &self.loot
    }
    fn loot_mut(&mut self) -> &mut wow_world_loot::LootState {
        &mut self.loot
    }
    fn split_loot_ref(&self) -> (&wow_world_loot::LootState, HubRef<'_>) {
        crate::session::split_loot_ref(self)
    }
    fn split_loot_mut(&mut self) -> (&mut wow_world_loot::LootState, HubMut<'_>) {
        crate::session::split_loot_mut(self)
    }
    fn lifecycle(&self) -> &SessionLifecycleState {
        &self.lifecycle
    }
    fn inventory(&self) -> &InventoryState {
        &self.inventory
    }
    fn represented_local_loot_fixture_allowed_like_cpp(&self) -> bool {
        represented_local_loot_fixture_allowed_like_cpp()
    }
    fn current_map_dungeon_state_like_cpp(&self) -> Option<bool> {
        self.current_map_dungeon_state_like_cpp()
    }
    fn current_map_is_dungeon_like_cpp(&self) -> bool {
        self.current_map_is_dungeon_like_cpp()
    }
    fn do_loot_release_all_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = ()> + Send {
        self.do_loot_release_all_like_cpp(player_guid)
    }
    fn give_xp(
        &mut self,
        xp: u32,
        victim: ObjectGuid,
        group_rate: f32,
    ) -> impl Future<Output = ()> + Send {
        self.give_xp(xp, victim, group_rate)
    }
    fn group_registry(&self) -> Option<&Arc<GroupRegistry>> {
        self.group_registry()
    }
    fn has_active_non_item_loot_views_like_cpp(&self) -> bool {
        self.has_active_non_item_loot_views_like_cpp()
    }
    fn item_drop_rate_like_cpp(&self, item_id: u32) -> f32 {
        self.item_drop_rate_like_cpp(item_id)
    }
    fn item_storage_template(&self, item_id: u32) -> Option<wow_entities::ItemStorageTemplate> {
        self.item_storage_template(item_id)
    }
    fn item_template_addon_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>> {
        crate::session::cx_inventory_ref(self)
            .item_template_addon_catalog_persistence_port_like_cpp()
    }
    fn item_template_start_quest_id(&self, item_id: u32) -> Option<i32> {
        self.item_template_start_quest_id(item_id)
    }
    fn known_spells_like_cpp(&self) -> Vec<i32> {
        self.known_spells_like_cpp()
    }
    fn loot_drop_rates_like_cpp(&self) -> LootDropRatesLikeCpp {
        self.loot_drop_rates_like_cpp()
    }
    fn loot_release_cx_like_cpp(&mut self) -> wow_world_loot_ops::LootReleaseCxLikeCpp<'_> {
        self.loot_release_cx_like_cpp()
    }
    fn loot_stores(&self) -> Option<&Arc<LootStores>> {
        self.loot_stores()
    }
    fn mutate_canonical_gameobject_by_guid_like_cpp<R>(
        &mut self,
        guid: ObjectGuid,
        f: impl FnOnce(&mut wow_entities::GameObject) -> R,
    ) -> Option<R> {
        self.mutate_canonical_gameobject_by_guid_like_cpp(guid, f)
    }
    fn plan_store_new_direct_inventory_item(
        &self,
        entry_id: u32,
        count: u32,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        self.plan_store_new_direct_inventory_item(entry_id, count)
    }
    fn player_quest_gameplay_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerQuestGameplayState> {
        self.player_quest_gameplay_snapshot_like_cpp()
    }
    fn player_registry(&self) -> Option<&Arc<PlayerRegistry>> {
        self.player_registry()
    }
    fn queue_chest_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize {
        crate::session::cx_loot_ref(self)
            .queue_chest_gameobject_state_refresh_for_same_map_like_cpp(gameobject_guid)
    }
    fn represented_inventory_item_counts_like_cpp(&self) -> Option<HashMap<u32, u32>> {
        self.represented_inventory_item_counts_like_cpp()
    }
    fn represented_money_loot_with_rate_like_cpp(
        &mut self,
        min_amount: u32,
        max_amount: u32,
        rate: f32,
    ) -> u32 {
        self.represented_money_loot_with_rate_like_cpp(min_amount, max_amount, rate)
    }
    fn represented_on_loot_opened_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        response: LootResponse,
    ) {
        self.represented_on_loot_opened_with_catalogs_like_cpp(
            item_valuation,
            owner_guid,
            player_guid,
            response,
        )
    }
    fn represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
        &self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> bool {
        self.represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
            player_guid,
            dungeon_encounter_id,
        )
    }
    fn represented_player_unlocked_for_dungeon_encounter_like_cpp(
        &self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> Option<ObjectGuid> {
        self.represented_player_unlocked_for_dungeon_encounter_like_cpp(
            player_guid,
            dungeon_encounter_id,
        )
    }
    fn store_direct_loot_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
    ) -> impl Future<Output = bool> + Send {
        self.store_direct_loot_item_with_generator_like_cpp(
            item_guid_generator,
            loot_entry,
            dungeon_encounter_id,
        )
    }
    fn store_direct_loot_item_with_source_and_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        stored_item_loot_source: Option<ObjectGuid>,
        claim: Option<&LootClaimLease>,
        claim_commit_context: Option<LootItemClaimCommitContextLikeCpp>,
    ) -> impl Future<Output = bool> + Send {
        self.store_direct_loot_item_with_source_and_generator_like_cpp(
            item_guid_generator,
            loot_entry,
            dungeon_encounter_id,
            stored_item_loot_source,
            claim,
            claim_commit_context,
        )
    }
    fn update_visible_gameobjects_or_spell_clicks_like_cpp(&self) -> usize {
        self.update_visible_gameobjects_or_spell_clicks_like_cpp()
    }
}
