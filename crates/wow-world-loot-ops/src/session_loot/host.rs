// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The World operations the moved loot bodies still call (#1363 slices 5a-5c).
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
use std::sync::atomic::AtomicBool;

use wow_constants::{InventoryResult, ItemContext};
use wow_core::ObjectGuid;
use wow_data::ItemStatsStore;
use wow_entities::{
    Item, ItemObjectUpdateLikeCpp, ItemPosCount, ItemStorageTemplate,
    PlayerInventoryItem as InventoryItem, PlayerQuestGameplayState, PlayerValuesUpdate,
};
use wow_loot::{LootClaimLease, LootStores};
use wow_social::group::GroupRegistry;
use wow_world_core::session::directory::PlayerRegistry;
use wow_world_core::session::mailbox::{ApplyLootMoneyLikeCppCommand, SessionCommand};
use wow_world_core::session::{HubMut, HubRef};
use wow_world_core::session_policy::LootDropRatesLikeCpp;
use wow_world_entities::WorldEntitiesHostLikeCpp;
use wow_world_inventory::InventoryState;
use wow_world_lifecycle::loot_delivery_contracts::{
    LootMoneyDeliveryAddressLikeCpp, LootMoneyViewerFanoutLikeCpp,
};
use wow_world_lifecycle::loot_money_persistence::LootMoneyPersistenceWorkerHandleLikeCpp;
use wow_world_lifecycle::{
    DurableItemLootCompletionLikeCpp, DurableItemLootPersistenceGuardLikeCpp,
    LootMoneyPersistenceErrorLikeCpp, SessionLifecycleState,
};
use wow_world_loot::LootState;
use wow_world_quest::{
    QuestSourceItemBoundPersistencePlanLikeCpp, QuestSourceItemBoundPreflightLikeCpp,
    SessionQuestState,
};

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
    fn represented_inventory_item_counts_like_cpp(&self) -> Option<HashMap<u32, u32>>;
    fn represented_money_loot_with_rate_like_cpp(
        &mut self,
        min_amount: u32,
        max_amount: u32,
        rate: f32,
    ) -> u32;
    fn update_visible_gameobjects_or_spell_clicks_like_cpp(&self) -> usize;
    // #1363 slice 5b: what the moved rolls, requests, item storage,
    // persistence and fanout bodies call.
    fn quest_state(&self) -> &SessionQuestState;
    fn quest_state_mut(&mut self) -> &mut SessionQuestState;
    fn split_inventory_ref(&self) -> (&InventoryState, HubRef<'_>);
    fn apply_inventory_item_object_updates_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        updates: &[ItemObjectUpdateLikeCpp],
    ) -> bool;
    fn apply_quest_source_item_added_non_bound_objective_progress_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> impl Future<Output = Vec<u32>> + Send;
    fn apply_quest_source_item_bound_objective_preflight_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> impl Future<Output = Option<QuestSourceItemBoundPreflightLikeCpp>> + Send;
    fn begin_durable_item_loot_persistence_like_cpp(
        &self,
    ) -> DurableItemLootPersistenceGuardLikeCpp;
    fn do_loot_release_owner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = bool> + Send;
    fn drain_represented_quest_objective_progress_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send;
    fn insert_inventory_item_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem>;
    fn insert_inventory_item_object(&mut self, item: Item) -> Option<Item>;
    fn item_stats_store(&self) -> Option<&Arc<ItemStatsStore>>;
    fn item_template_flags2(&self, item_id: u32) -> Option<u32>;
    fn item_template_inventory_type(&self, item_id: u32) -> Option<u8>;
    fn item_template_max_durability(&self, item_id: u32) -> u32;
    #[allow(clippy::too_many_arguments)]
    fn make_inventory_item_object(
        &self,
        item_guid: ObjectGuid,
        entry_id: u32,
        owner_guid: ObjectGuid,
        count: u32,
        durability: u32,
        context: ItemContext,
        slot: u8,
    ) -> Item;
    fn on_item_added_to_collection_like_cpp(&mut self, item: &Item) -> Vec<PlayerValuesUpdate>;
    fn plan_quest_source_item_bound_objective_persistence_like_cpp(
        &self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Option<QuestSourceItemBoundPersistencePlanLikeCpp>;
    fn player_is_locked_to_dungeon_encounter_like_cpp(
        &self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> Option<bool>;
    fn represented_can_receive_creature_message_to_set_by_guid_like_cpp(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        required_3d: bool,
    ) -> Option<bool>;
    /// C++ `Loot::NotifyMoneyRemoved`, the loot handler owner's default method.
    fn represented_notify_money_removed_like_cpp(&mut self, owner_guid: ObjectGuid);
    fn resolved_inventory_item_like_cpp(&self, slot: u8) -> Option<InventoryItem>;
    fn resolved_inventory_item_object_like_cpp(&self, guid: ObjectGuid) -> Option<Item>;
    fn resolved_inventory_items_like_cpp(&self) -> Option<HashMap<u8, InventoryItem>>;
    fn resolved_pass_on_group_loot_like_cpp(&self) -> Option<bool>;
    fn resolved_player_money_like_cpp(&self) -> Option<u64>;
    fn save_changed_represented_quest_statuses_like_cpp(
        &self,
        quest_ids: &mut Vec<u32>,
    ) -> impl Future<Output = ()> + Send;
    fn send_player_values_update_from_entity_bridge(
        &self,
        inv_slot_changes: &[(u8, ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) -> bool;
    fn session_command_tx(&self) -> flume::Sender<SessionCommand>;
    fn set_player_gold_like_cpp(&mut self, gold: u64) -> bool;
    fn sync_player_registry_state_like_cpp(&self);
    fn take_durable_item_loot_completions_like_cpp(&self) -> Vec<DurableItemLootCompletionLikeCpp>;
    fn wait_for_durable_item_loot_persistence_like_cpp(&self) -> impl Future<Output = ()> + Send;
    // #1363 slice 5c: what the moved money paths call. The first four are the
    // loot handler owner's default methods (`LootHandlerHostLikeCpp`).
    fn handle_apply_loot_money_with_generator_like_cpp_command<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
        command: ApplyLootMoneyLikeCppCommand,
    ) -> impl Future<Output = ()> + Send + 'a;
    fn represented_loot_money_recipients_like_cpp(&self, loot_guid: ObjectGuid) -> Vec<ObjectGuid>;
    fn spawn_group_loot_money_persistence_like_cpp(
        &self,
        payouts: Vec<(ObjectGuid, u64)>,
        claim: LootClaimLease,
        deliveries: Vec<(LootMoneyDeliveryAddressLikeCpp, SessionCommand)>,
        authority_committed: Arc<AtomicBool>,
        viewer_fanout: LootMoneyViewerFanoutLikeCpp,
    ) -> Result<LootMoneyPersistenceWorkerHandleLikeCpp, LootMoneyPersistenceErrorLikeCpp>;
    #[allow(clippy::type_complexity)]
    fn persist_and_consume_stored_item_money_like_cpp(
        &self,
        item_guid: ObjectGuid,
        cached_notified_amount: u64,
    ) -> impl Future<Output = Option<(Arc<AtomicBool>, Arc<AtomicBool>, u64, u64)>> + Send;
    fn mutate_and_persist_player_gold_exclusive_like_cpp<F>(
        &mut self,
        mutation: F,
    ) -> impl Future<Output = Option<(u64, u64)>> + Send
    where
        F: FnOnce(u64) -> u64 + Send;
}
