// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The World operations the moved loot-source bodies still call (#1363 slice 5a).
//!
//! C++ runs these bodies on the Player and its Map (`Loot::FillLoot`,
//! `Unit::Kill`, `GameObject::Use`, `LootHandler.cpp`). Their Rust bodies now
//! live in this crate; each method below is the same-named `WorldSession`
//! operation or field, called at the exact point the body called it before the
//! move. The loot-opened publication, release, item storage, dungeon checks,
//! inventory and quest reads stay with the host.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use wow_constants::InventoryResult;
use wow_core::ObjectGuid;
use wow_entities::{ItemPosCount, ItemStorageTemplate, PlayerQuestGameplayState};
use wow_loot::{LootClaimLease, LootEntry, LootStores};
use wow_packet::packets::loot::LootResponse;
use wow_social::group::GroupRegistry;
use wow_world_core::session::directory::PlayerRegistry;
use wow_world_core::session::{HubMut, HubRef, ItemValuationCatalogsLikeCpp};
use wow_world_core::session_policy::LootDropRatesLikeCpp;
use wow_world_entities::WorldEntitiesHostLikeCpp;
use wow_world_inventory::InventoryState;
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_loot::LootState;
use wow_world_loot::storage_plans::LootItemClaimCommitContextLikeCpp;

pub trait LootSessionHostLikeCpp: WorldEntitiesHostLikeCpp {
    fn loot(&self) -> &LootState;
    fn loot_mut(&mut self) -> &mut LootState;
    fn split_loot_ref(&self) -> (&LootState, HubRef<'_>);
    fn split_loot_mut(&mut self) -> (&mut LootState, HubMut<'_>);
    fn lifecycle(&self) -> &SessionLifecycleState;
    fn inventory(&self) -> &InventoryState;
    /// The World crate's `cfg!(test)`: legacy packet fixtures may fall back to
    /// the session cache only in the World test build.
    fn represented_local_loot_fixture_allowed_like_cpp(&self) -> bool;
    fn current_map_dungeon_state_like_cpp(&self) -> Option<bool>;
    fn current_map_is_dungeon_like_cpp(&self) -> bool;
    fn do_loot_release_all_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = ()> + Send;
    fn give_xp(
        &mut self,
        xp: u32,
        victim: ObjectGuid,
        group_rate: f32,
    ) -> impl Future<Output = ()> + Send;
    fn group_registry(&self) -> Option<&Arc<GroupRegistry>>;
    fn has_active_non_item_loot_views_like_cpp(&self) -> bool;
    fn item_drop_rate_like_cpp(&self, item_id: u32) -> f32;
    fn item_storage_template(&self, item_id: u32) -> Option<ItemStorageTemplate>;
    fn item_template_addon_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>>;
    fn item_template_start_quest_id(&self, item_id: u32) -> Option<i32>;
    fn known_spells_like_cpp(&self) -> Vec<i32>;
    fn loot_drop_rates_like_cpp(&self) -> LootDropRatesLikeCpp;
    fn loot_release_cx_like_cpp(&mut self) -> crate::LootReleaseCxLikeCpp<'_>;
    fn loot_stores(&self) -> Option<&Arc<LootStores>>;
    fn mutate_canonical_gameobject_by_guid_like_cpp<R>(
        &mut self,
        guid: ObjectGuid,
        f: impl FnOnce(&mut wow_entities::GameObject) -> R,
    ) -> Option<R>;
    fn plan_store_new_direct_inventory_item(
        &self,
        entry_id: u32,
        count: u32,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)>;
    fn player_quest_gameplay_snapshot_like_cpp(&self) -> Option<PlayerQuestGameplayState>;
    fn player_registry(&self) -> Option<&Arc<PlayerRegistry>>;
    fn queue_chest_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize;
    fn represented_inventory_item_counts_like_cpp(&self) -> Option<HashMap<u32, u32>>;
    fn represented_money_loot_with_rate_like_cpp(
        &mut self,
        min_amount: u32,
        max_amount: u32,
        rate: f32,
    ) -> u32;
    fn represented_on_loot_opened_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        response: LootResponse,
    );
    fn represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
        &self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> bool;
    fn represented_player_unlocked_for_dungeon_encounter_like_cpp(
        &self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> Option<ObjectGuid>;
    fn store_direct_loot_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
    ) -> impl Future<Output = bool> + Send;
    fn store_direct_loot_item_with_source_and_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        stored_item_loot_source: Option<ObjectGuid>,
        claim: Option<&LootClaimLease>,
        claim_commit_context: Option<LootItemClaimCommitContextLikeCpp>,
    ) -> impl Future<Output = bool> + Send;
    fn update_visible_gameobjects_or_spell_clicks_like_cpp(&self) -> usize;
}
