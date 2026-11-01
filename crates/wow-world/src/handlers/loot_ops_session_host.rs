// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the loot bodies that moved to `wow_world_loot_ops`
//! (#1363 slices 5a, 5b and 5c). Each method forwards to the same-named session
//! field or inherent operation (the durable item-loot persistence ones through
//! the inventory context; `Loot::NotifyMoneyRemoved` and four of the 5c money
//! operations through the loot handler owner's default methods); the fixture
//! predicate keeps this crate's `cfg!(test)`. Moved out of `handlers/loot` in
//! #1363 slice 12d (one adapter file per destination crate, outside the
//! hotspot trees), where the retired split forwards' bodies now sit.

use std::sync::atomic::AtomicBool;
use std::{collections::HashMap, future::Future, sync::Arc};

use wow_constants::{InventoryResult, ItemContext};
use wow_core::ObjectGuid;
use wow_entities::ItemPosCount;

use wow_social::group::GroupRegistry;
use wow_world_core::session::directory::PlayerRegistry;
use wow_world_core::session::{HubMut, HubRef};
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_loot_ops::{LootHandlerHostLikeCpp, LootSessionHostLikeCpp};

use crate::session::mailbox::{ApplyLootMoneyLikeCppCommand, SessionCommand};
use crate::session::{
    DurableItemLootCompletionLikeCpp, DurableItemLootPersistenceGuardLikeCpp, InventoryItem,
    LootMoneyDeliveryAddressLikeCpp, LootMoneyViewerFanoutLikeCpp, WorldSession,
};
use wow_loot::LootStores;
use wow_world_core::session_policy::LootDropRatesLikeCpp;
use wow_world_inventory::InventoryState;

use crate::handlers::quest::{
    QuestSourceItemBoundPersistencePlanLikeCpp, QuestSourceItemBoundPreflightLikeCpp,
};
use crate::session::SessionHandlerCatalogsLikeCpp;
use wow_data::ItemStatsStore;
use wow_entities::{Item, ItemObjectUpdateLikeCpp, PlayerValuesUpdate};
use wow_world_quest::SessionQuestState;

impl LootSessionHostLikeCpp for WorldSession {
    fn loot_money_hook_wanted_like_cpp(&self) -> bool {
        self.module_hooks
            .wants_player_hook_like_cpp(wow_module_api::PlayerHook::LootMoney)
    }
    fn loot_money_hook_like_cpp(&mut self, event: &wow_world_loot_ops::LootMoneyHookEventLikeCpp) {
        self.module_hooks
            .loot_money_like_cpp(self.core.packet_publication_access_like_cpp(), event);
    }
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
        crate::handlers::loot::represented_local_loot_fixture_allowed_like_cpp()
    }
    fn current_map_dungeon_state_like_cpp(&self) -> Option<bool> {
        wow_world_spell::current_map_dungeon_state_like_cpp(self)
    }
    fn current_map_is_dungeon_like_cpp(&self) -> bool {
        wow_world_spell::current_map_is_dungeon_like_cpp(self)
    }
    fn do_loot_release_all_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = ()> + Send {
        self.do_loot_release_all_like_cpp(player_guid)
    }
    fn group_registry(&self) -> Option<&Arc<GroupRegistry>> {
        self.group_registry()
    }
    fn has_active_non_item_loot_views_like_cpp(&self) -> bool {
        self.loot.has_active_non_item_loot_views_like_cpp()
    }
    fn item_drop_rate_like_cpp(&self, item_id: u32) -> f32 {
        self.inventory
            .item_drop_rate_like_cpp(crate::session::hub_ref(self), item_id)
    }
    fn item_storage_template(&self, item_id: u32) -> Option<wow_entities::ItemStorageTemplate> {
        self.catalogs.item_storage_template(item_id)
    }
    fn item_template_addon_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>> {
        crate::session::cx_inventory_ref(self)
            .item_template_addon_catalog_persistence_port_like_cpp()
    }
    fn item_template_start_quest_id(&self, item_id: u32) -> Option<i32> {
        self.catalogs.item_template_start_quest_id(item_id)
    }
    fn known_spells_like_cpp(&self) -> Vec<i32> {
        wow_world_spell::known_spells_like_cpp(self)
    }
    fn loot_drop_rates_like_cpp(&self) -> LootDropRatesLikeCpp {
        self.config.loot_drop_rates_like_cpp()
    }
    fn loot_release_cx_like_cpp(&mut self) -> wow_world_loot_ops::LootReleaseCxLikeCpp<'_> {
        WorldSession::loot_release_cx_like_cpp(self)
    }
    fn loot_stores(&self) -> Option<&Arc<LootStores>> {
        self.catalogs.loot_stores()
    }
    fn mutate_canonical_gameobject_by_guid_like_cpp<R>(
        &mut self,
        guid: ObjectGuid,
        f: impl FnOnce(&mut wow_entities::GameObject) -> R,
    ) -> Option<R> {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.mutate_canonical_gameobject_by_guid_like_cpp(&mut hub, guid, f)
    }
    fn plan_store_new_direct_inventory_item(
        &self,
        entry_id: u32,
        count: u32,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        wow_world_application::plan_store_new_direct_inventory_item(self, entry_id, count)
    }
    fn player_quest_gameplay_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerQuestGameplayState> {
        wow_world_quest::QuestSessionHostLikeCpp::player_quest_gameplay_snapshot_like_cpp(self)
    }
    fn player_registry(&self) -> Option<&Arc<PlayerRegistry>> {
        self.player_registry()
    }
    fn represented_inventory_item_counts_like_cpp(&self) -> Option<HashMap<u32, u32>> {
        self.inventory
            .represented_inventory_item_counts_like_cpp(crate::session::hub_ref(self))
    }
    fn represented_money_loot_with_rate_like_cpp(
        &mut self,
        min_amount: u32,
        max_amount: u32,
        rate: f32,
    ) -> u32 {
        let (state, mut hub) = crate::session::split_loot_mut(self);
        state.represented_money_loot_with_rate_like_cpp(&mut hub, min_amount, max_amount, rate)
    }
    fn update_visible_gameobjects_or_spell_clicks_like_cpp(&self) -> usize {
        self.update_visible_gameobjects_or_spell_clicks_like_cpp()
    }
    // #1363 slice 5b.
    fn quest_state(&self) -> &SessionQuestState {
        &self.quest_state
    }
    fn quest_state_mut(&mut self) -> &mut SessionQuestState {
        &mut self.quest_state
    }
    fn split_inventory_ref(&self) -> (&InventoryState, HubRef<'_>) {
        crate::session::split_inventory_ref(self)
    }
    fn apply_inventory_item_object_updates_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        updates: &[ItemObjectUpdateLikeCpp],
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.apply_inventory_item_object_updates_like_cpp(&mut hub, item_guid, updates)
    }
    fn apply_quest_source_item_added_non_bound_objective_progress_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> impl Future<Output = Vec<u32>> + Send {
        wow_world_quest::apply_quest_source_item_added_non_bound_objective_progress_with_generator_like_cpp(
            self,
            item_guid_generator,
            entry_id,
            quest_log_item_id,
            count,
        )
    }
    fn apply_quest_source_item_bound_objective_preflight_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> impl Future<Output = Option<QuestSourceItemBoundPreflightLikeCpp>> + Send {
        wow_world_quest::apply_quest_source_item_bound_objective_preflight_with_generator_like_cpp(
            self,
            item_guid_generator,
            entry_id,
            quest_log_item_id,
            count,
        )
    }
    fn begin_durable_item_loot_persistence_like_cpp(
        &self,
    ) -> DurableItemLootPersistenceGuardLikeCpp {
        crate::session::cx_inventory_ref(self).begin_durable_item_loot_persistence_like_cpp()
    }
    fn do_loot_release_owner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = bool> + Send {
        WorldSession::do_loot_release_owner_like_cpp(self, owner_guid, player_guid)
    }
    fn drain_represented_quest_objective_progress_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send {
        self.drain_represented_quest_objective_progress_with_generator_like_cpp(item_guid_generator)
    }
    fn insert_inventory_item_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.insert_inventory_item_like_cpp(&mut hub, slot, item)
    }
    fn insert_inventory_item_object(&mut self, item: Item) -> Option<Item> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.insert_inventory_item_object(&mut hub, item)
    }
    fn item_stats_store(&self) -> Option<&Arc<ItemStatsStore>> {
        self.catalogs.item_stats_store()
    }
    fn item_template_flags2(&self, item_id: u32) -> Option<u32> {
        self.catalogs.item_template_flags2(item_id)
    }
    fn item_template_inventory_type(&self, item_id: u32) -> Option<u8> {
        crate::session::hub_ref(self).item_template_inventory_type(item_id)
    }
    fn item_template_max_durability(&self, item_id: u32) -> u32 {
        self.catalogs.item_template_max_durability(item_id)
    }
    fn make_inventory_item_object(
        &self,
        item_guid: ObjectGuid,
        entry_id: u32,
        owner_guid: ObjectGuid,
        count: u32,
        durability: u32,
        context: ItemContext,
        slot: u8,
    ) -> Item {
        crate::session::cx_inventory_ref(self).make_inventory_item_object(
            item_guid, entry_id, owner_guid, count, durability, context, slot,
        )
    }
    fn on_item_added_to_collection_like_cpp(&mut self, item: &Item) -> Vec<PlayerValuesUpdate> {
        wow_world_inventory::on_item_added_to_collection_like_cpp(self, item)
    }
    fn plan_quest_source_item_bound_objective_persistence_like_cpp(
        &self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Option<QuestSourceItemBoundPersistencePlanLikeCpp> {
        wow_world_quest::plan_quest_source_item_bound_objective_persistence_like_cpp(
            self,
            entry_id,
            quest_log_item_id,
            count,
        )
    }
    fn player_is_locked_to_dungeon_encounter_like_cpp(
        &self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> Option<bool> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.player_is_locked_to_dungeon_encounter_like_cpp(hub, player_guid, dungeon_encounter_id)
    }
    fn represented_can_receive_creature_message_to_set_by_guid_like_cpp(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        required_3d: bool,
    ) -> Option<bool> {
        wow_world_entities::represented_can_receive_creature_message_to_set_by_guid_like_cpp(
            self,
            guid,
            map_id,
            instance_id,
            required_3d,
        )
    }
    fn represented_notify_money_removed_like_cpp(&mut self, owner_guid: ObjectGuid) {
        <Self as LootHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp>>::represented_notify_money_removed_like_cpp(
            self, owner_guid,
        )
    }
    fn resolved_inventory_item_like_cpp(&self, slot: u8) -> Option<InventoryItem> {
        self.inventory
            .resolved_inventory_item_like_cpp(crate::session::hub_ref(self), slot)
    }
    fn resolved_inventory_item_object_like_cpp(&self, guid: ObjectGuid) -> Option<Item> {
        self.inventory
            .resolved_inventory_item_object_like_cpp(crate::session::hub_ref(self), guid)
    }
    fn resolved_inventory_items_like_cpp(&self) -> Option<HashMap<u8, InventoryItem>> {
        self.inventory
            .resolved_inventory_items_like_cpp(crate::session::hub_ref(self))
    }
    fn resolved_pass_on_group_loot_like_cpp(&self) -> Option<bool> {
        self.loot
            .resolved_pass_on_group_loot_like_cpp(crate::session::hub_ref(self))
    }
    fn resolved_player_money_like_cpp(&self) -> Option<u64> {
        wow_world_inventory::BattlePetPurchaseHostLikeCpp::resolved_player_money_like_cpp(self)
    }
    fn save_changed_represented_quest_statuses_like_cpp(
        &self,
        quest_ids: &mut Vec<u32>,
    ) -> impl Future<Output = ()> + Send {
        wow_world_quest::save_changed_represented_quest_statuses_like_cpp(self, quest_ids)
    }
    fn send_player_values_update_from_entity_bridge(
        &self,
        inv_slot_changes: &[(u8, ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) -> bool {
        self.send_player_values_update_from_entity_bridge(
            inv_slot_changes,
            visible_item_changes,
            virtual_item_changes,
            buyback_changes,
            coinage,
        )
    }
    fn session_command_tx(&self) -> flume::Sender<SessionCommand> {
        self.session_command_tx()
    }
    fn set_player_gold_like_cpp(&mut self, gold: u64) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_player_gold_like_cpp(&mut hub, gold)
    }
    fn sync_player_registry_state_like_cpp(&self) {
        self.sync_player_registry_state_like_cpp()
    }
    fn has_pending_bind_like_cpp(&self) -> bool {
        self.instances.has_pending_bind_like_cpp()
    }
    fn take_durable_item_loot_completions_like_cpp(&self) -> Vec<DurableItemLootCompletionLikeCpp> {
        crate::session::cx_inventory_ref(self).take_durable_item_loot_completions_like_cpp()
    }
    fn wait_for_durable_item_loot_persistence_like_cpp(&self) -> impl Future<Output = ()> + Send {
        async move {
            crate::session::cx_inventory_ref(self)
                .wait_for_durable_item_loot_persistence_like_cpp()
                .await
        }
    } // #1363 slice 5c: the money paths.
    fn handle_apply_loot_money_with_generator_like_cpp_command<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
        command: ApplyLootMoneyLikeCppCommand,
    ) -> impl Future<Output = ()> + Send + 'a {
        <Self as LootHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp>>::handle_apply_loot_money_with_generator_like_cpp_command(
            self, item_guid_generator, command,
        )
    }
    fn represented_loot_money_recipients_like_cpp(&self, loot_guid: ObjectGuid) -> Vec<ObjectGuid> {
        <Self as LootHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp>>::represented_loot_money_recipients_like_cpp(
            self, loot_guid,
        )
    }
    fn spawn_group_loot_money_persistence_like_cpp(
        &self,
        payouts: Vec<(ObjectGuid, u64)>,
        claim: wow_loot::LootClaimLease,
        deliveries: Vec<(LootMoneyDeliveryAddressLikeCpp, SessionCommand)>,
        authority_committed: Arc<AtomicBool>,
        viewer_fanout: LootMoneyViewerFanoutLikeCpp,
    ) -> Result<
        wow_world_lifecycle::loot_money_persistence::LootMoneyPersistenceWorkerHandleLikeCpp,
        crate::session::LootMoneyPersistenceErrorLikeCpp,
    > {
        <Self as LootHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp>>::spawn_group_loot_money_persistence_like_cpp(
            self, payouts, claim, deliveries, authority_committed, viewer_fanout,
        )
    }
    fn persist_and_consume_stored_item_money_like_cpp(
        &self,
        item_guid: ObjectGuid,
        cached_notified_amount: u64,
    ) -> impl Future<Output = Option<(Arc<AtomicBool>, Arc<AtomicBool>, u64, u64)>> + Send {
        <Self as LootHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp>>::persist_and_consume_stored_item_money_like_cpp(
            self, item_guid, cached_notified_amount,
        )
    }
    fn mutate_and_persist_player_gold_exclusive_like_cpp<F>(
        &mut self,
        mutation: F,
    ) -> impl Future<Output = Option<(u64, u64)>> + Send
    where
        F: FnOnce(u64) -> u64 + Send,
    {
        self.mutate_and_persist_player_gold_exclusive_like_cpp(mutation)
    }
}
