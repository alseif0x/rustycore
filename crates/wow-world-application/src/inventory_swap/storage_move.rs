// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! StorageMove planning and execution in their application owner (#1263 F4).
//!
//! C++ anchors, verified in the reference checkout at
//! `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player::SwapItem`'s store/bank
//! transitions (`src/server/game/Entities/Player/Player.cpp:12423`) and the
//! inventory persistence at `Player.cpp:19632`, reached from
//! `WorldSession::HandleAutoBankItemOpcode` / `HandleAutoStoreBankItemOpcode`
//! (`Handlers/BankHandler.cpp`) and `HandleAutoStoreBagItemOpcode`
//! (`Handlers/ItemHandler.cpp`).
//!
//! The module owns the operation and borrows its participants from Inventory
//! ([`InventoryStorageMoveCxLikeCpp`] over `InventoryState` plus its canonical
//! access and shared hub), Stats
//! ([`InventoryStorageMoveStatsCxLikeCpp`] over `PlayerStatsAccessLikeCpp`) and
//! persistence (`SessionLifecycleState`'s inventory port). The shell-only
//! capabilities the body still needs — the quest item/objective operations, the
//! narrow obtain-spells executor and the two cfg(test) observation records —
//! stay behind [`InventoryStorageMoveHostLikeCpp`], implemented by the session
//! at the exact points the World body invoked them.
//!
//! Preserved exactly: the silent absence (`None` from the planner), the
//! planning error publication, the `Failed`/`Unknown` commit classification
//! with its unchanged runtime and `InternalBagError` reply, and every
//! publication step's order. `Unknown` does NOT prove the database is
//! unchanged: the runtime stays untouched and nothing is published, which is
//! not a claim about the SQL state.

use std::sync::Arc;

use wow_constants::InventoryResult;
use wow_core::ObjectGuid;
use wow_data::{ItemEffectStore, ItemStatsStore, ItemStore, SpellItemEnchantmentStore};
use wow_entities::{
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START,
    InventoryStorageMovePlanLikeCpp, ItemObjectUpdateLikeCpp, ItemStorageTemplate,
    PlayerInventoryItem, is_bank_pos,
};
use wow_packet::ServerPacket;
use wow_persistence::{PlayerInventoryPersistencePortLikeCpp, QuestStatusPersistenceLikeCpp};
use wow_world_core::catalogs::item::item_storage_template_like_cpp;
use wow_world_core::session::{
    HubRef, OwnedInventoryAccessLikeCpp, OwnedItemModifiersAccessLikeCpp,
    OwnedItemSetAccessLikeCpp, PacketPublicationAccessLikeCpp, PlayerStatsAccessLikeCpp,
};
use wow_world_inventory::{
    InventoryState, ItemModsCatalogsViewLikeCpp, RepresentedBankItemMoveLikeCpp,
};
use wow_world_lifecycle::SessionLifecycleState;

use crate::inventory_move_planning::InventoryMovePlanningCxLikeCpp;
use crate::stats::CharacterStatsApplicationCxLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
use super::InventoryEquipFixtureRefsLikeCpp;

mod execution;
pub use execution::execute_inventory_storage_move_like_cpp;

/// Which storage the move targets. Moved out of the character handler module
/// with the operation it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryStorageTargetLikeCpp {
    Inventory,
    Bank,
}

/// Which C++ quest checks `_StoreItem`/`_BankItem` run for this caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryStorageQuestChecksLikeCpp {
    None,
    AutoBankItemRemoved,
    AutoStoreBankItemAdded,
}

/// C++ `HandleAutoStoreBankItemOpcode` targets the opposite storage of the
/// source position.
pub fn autostore_bank_target_like_cpp(
    source_bag: u8,
    source_slot: u8,
) -> InventoryStorageTargetLikeCpp {
    if is_bank_pos(source_bag, source_slot) {
        InventoryStorageTargetLikeCpp::Inventory
    } else {
        InventoryStorageTargetLikeCpp::Bank
    }
}

/// The `(moving_to_bank, moving_from_bank)` pair the quest checks read.
pub fn inventory_storage_move_quest_directions_like_cpp(
    source_bag: u8,
    source_slot: u8,
    target: InventoryStorageTargetLikeCpp,
) -> (bool, bool) {
    let moving_to_bank = target == InventoryStorageTargetLikeCpp::Bank;
    let moving_from_bank = !moving_to_bank && is_bank_pos(source_bag, source_slot);
    (moving_to_bank, moving_from_bank)
}

/// C++ `HandleAutoStoreBankItemOpcode` passes `storedItem->GetCount()` after
/// `StoreItem`. `_StoreItem` returns the last destination item, so a full merge
/// reports that destination stack's total and a merge+remainder reports the
/// final remainder stack count. This is deliberately not `source_count`.
pub fn bank_store_item_added_quest_count_like_cpp(plan: &InventoryStorageMovePlanLikeCpp) -> u32 {
    plan.moved_destination
        .map(|(_, _, count)| count)
        .or_else(|| plan.existing_updates.last().map(|update| update.new_count))
        .unwrap_or(0)
}

/// C++ `Player::_StoreItem` checks only the bag value. `INVENTORY_SLOT_BAG_0`
/// therefore includes top-level personal-bank slots as well as carried
/// top-level slots; bank-bag containers remain excluded.
pub fn bank_store_destination_applies_obtain_spells_like_cpp(bag: u8) -> bool {
    bag == INVENTORY_SLOT_BAG_0 || (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&bag)
}

/// Compose the complete StorageMove plan: source lookup, the owned
/// `CanStoreItem`/`CanBankItem` allocation and the deterministic validation of
/// the ordered destination list.
///
/// The allocator stays with its existing owner
/// ([`InventoryMovePlanningCxLikeCpp`], built by the shell seam from its
/// condition projection). `None` is the silent absence the handler preserves;
/// `Some(Err(result))` is the planning rejection the caller publishes.
#[allow(clippy::too_many_arguments)]
pub fn plan_inventory_storage_move_like_cpp(
    planning: &InventoryMovePlanningCxLikeCpp<'_, '_>,
    inventory: &InventoryState,
    access: &OwnedInventoryAccessLikeCpp<'_>,
    item_store: Option<&Arc<ItemStore>>,
    item_stats_store: Option<&Arc<ItemStatsStore>>,
    source_bag: u8,
    source_slot: u8,
    destination_bag: u8,
    destination_slot: u8,
    target: InventoryStorageTargetLikeCpp,
) -> Option<Result<InventoryStorageMovePlanLikeCpp, InventoryResult>> {
    let source = inventory.get_inventory_item_by_pos_with_access_like_cpp(
        access,
        item_store,
        item_stats_store,
        source_bag,
        source_slot,
    )?;
    let source_object = inventory
        .resolved_player_inventory_item_object_with_access_like_cpp(access, source.guid)?;
    let source_count = source_object.count();
    let moving_to_bank = target == InventoryStorageTargetLikeCpp::Bank;
    let (result, destinations) = if moving_to_bank {
        planning.plan_bank_existing_inventory_item_at_like_cpp(
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            false,
        )?
    } else {
        let (result, destinations, _) = planning.plan_store_existing_inventory_item_at_like_cpp(
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            false,
        )?;
        (result, destinations)
    };
    if result != InventoryResult::Ok {
        return Some(Err(result));
    }

    Some(wow_entities::plan_inventory_storage_move_like_cpp(
        source_bag,
        source_slot,
        source,
        source_count,
        &destinations,
        |bag, slot| {
            inventory.get_inventory_item_by_pos_with_access_like_cpp(
                access,
                item_store,
                item_stats_store,
                bag,
                slot,
            )
        },
        |guid| {
            inventory
                .resolved_player_inventory_item_object_with_access_like_cpp(access, guid)
                .map(|item| item.count())
        },
        |entry_id| {
            item_storage_template_like_cpp(item_store, item_stats_store, entry_id)
                .map(|template| template.max_stack_size)
        },
    ))
}

/// Capabilities of the StorageMove operation that the World shell still owns.
///
/// Each method is invoked at the exact point the World body invoked its
/// counterpart, so no step value or resumption is needed. The planning seam is
/// `&self` on purpose: it builds the condition projection from the immutable
/// participants, while every mutable participant is lent separately by
/// [`InventoryStorageMoveHostLikeCpp::storage_move_cx_like_cpp`].
pub trait InventoryStorageMoveHostLikeCpp {
    /// The complete plan, composed by the application owner from the shell's
    /// condition projection. `None` preserves the C++ silent absence.
    #[allow(clippy::too_many_arguments)]
    fn storage_move_plan_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        target: InventoryStorageTargetLikeCpp,
    ) -> Option<Result<InventoryStorageMovePlanLikeCpp, InventoryResult>>;

    /// Borrowed Inventory, persistence and publication participants.
    fn storage_move_cx_like_cpp(&mut self) -> InventoryStorageMoveCxLikeCpp<'_>;

    /// Borrowed Stats participant, kept separate from the operation
    /// participants because the cfg(test) stat fixtures are lent mutably.
    fn storage_move_stats_cx_like_cpp(&mut self) -> InventoryStorageMoveStatsCxLikeCpp<'_>;

    /// C++ `Player::GetQuestLogItemIdForItem` source for the store-bank check.
    fn storage_move_quest_log_item_id_like_cpp<'a>(
        &'a mut self,
        entry_id: u32,
    ) -> impl std::future::Future<Output = u32> + Send + 'a;

    /// C++ `Player::UpdateQuestStatusForItem` persistence plan.
    #[allow(clippy::too_many_arguments)]
    fn storage_move_plan_quest_statuses_like_cpp(
        &self,
        entry_id: u32,
        quest_log_item_id: u32,
        moving_to_bank: bool,
        post_move_non_bank_count: u32,
        added_count: u32,
    ) -> Vec<wow_entities::PlayerQuestStatusRecord>;

    /// C++ `Player::ItemRemovedQuestCheck` for the autobank direction.
    fn storage_move_apply_quest_item_removed_like_cpp(&mut self, entry_id: u32)
    -> Option<Vec<u32>>;

    /// C++ `Player::ItemAddedQuestCheck` for the autostore-bank direction.
    fn storage_move_apply_quest_item_added_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> impl std::future::Future<Output = Vec<u32>> + Send + 'a;

    /// The narrow spell executor: C++
    /// `Player::ApplyItemObtainSpells(item, true)` after `_StoreItem`.
    fn storage_move_apply_obtain_spells_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &'a wow_world_entities::CreatureSpawnCatalogsLikeCpp,
        entry_id: u32,
    ) -> impl std::future::Future<Output = ()> + Send + 'a;

    /// cfg(test) observation record for the C++ TitanGrip penalty action.
    fn storage_move_record_titan_grip_penalty_action_like_cpp(&mut self);

    /// cfg(test) observation record for C++
    /// `Player::UpdateAverageItemLevelEquipped`.
    fn storage_move_record_avg_equipped_item_level_update_like_cpp(&mut self);
}

/// Borrowed participants of a StorageMove execution.
///
/// Inventory lends its canonical state, the shared hub (both the
/// `Player::GetItemByGuid` lookups and the represented bank/quest counts) and
/// its access capabilities; persistence lends the lifecycle state that owns the
/// inventory port. Every inventory publication below goes through the same
/// owned access the shell used.
pub struct InventoryStorageMoveCxLikeCpp<'a> {
    inventory: &'a mut InventoryState,
    hub: HubRef<'a>,
    access: OwnedInventoryAccessLikeCpp<'a>,
    modifiers: OwnedItemModifiersAccessLikeCpp<'a>,
    item_sets: OwnedItemSetAccessLikeCpp<'a>,
    publication: PacketPublicationAccessLikeCpp<'a>,
    lifecycle: &'a SessionLifecycleState,
    item_store: Option<&'a Arc<ItemStore>>,
    item_stats_store: Option<&'a Arc<ItemStatsStore>>,
    item_effect_store: Option<&'a Arc<ItemEffectStore>>,
    enchantment_store: Option<&'a SpellItemEnchantmentStore>,
    mods_catalogs: ItemModsCatalogsViewLikeCpp<'a>,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixtures: InventoryEquipFixtureRefsLikeCpp<'a>,
    consumer_test: bool,
}

impl<'a> InventoryStorageMoveCxLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        inventory: &'a mut InventoryState,
        hub: HubRef<'a>,
        access: OwnedInventoryAccessLikeCpp<'a>,
        modifiers: OwnedItemModifiersAccessLikeCpp<'a>,
        item_sets: OwnedItemSetAccessLikeCpp<'a>,
        publication: PacketPublicationAccessLikeCpp<'a>,
        lifecycle: &'a SessionLifecycleState,
        item_store: Option<&'a Arc<ItemStore>>,
        item_stats_store: Option<&'a Arc<ItemStatsStore>>,
        item_effect_store: Option<&'a Arc<ItemEffectStore>>,
        enchantment_store: Option<&'a SpellItemEnchantmentStore>,
        mods_catalogs: ItemModsCatalogsViewLikeCpp<'a>,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: InventoryEquipFixtureRefsLikeCpp<'a>,
        consumer_test: bool,
    ) -> Self {
        Self {
            inventory,
            hub,
            access,
            modifiers,
            item_sets,
            publication,
            lifecycle,
            item_store,
            item_stats_store,
            item_effect_store,
            enchantment_store,
            mods_catalogs,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures,
            consumer_test,
        }
    }

    pub fn player_guid(&self) -> Option<ObjectGuid> {
        self.hub.core.player_guid()
    }

    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.hub.core.player_map_id_like_cpp()
    }

    pub fn inventory_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn PlayerInventoryPersistencePortLikeCpp>> {
        self.lifecycle.player_inventory_persistence_port_like_cpp()
    }

    pub fn get_inventory_item_by_pos(&self, bag: u8, slot: u8) -> Option<PlayerInventoryItem> {
        self.inventory
            .get_inventory_item_by_pos_with_access_like_cpp(
                &self.access,
                self.item_store,
                self.item_stats_store,
                bag,
                slot,
            )
    }

    pub fn resolved_inventory_item_object_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<wow_entities::Item> {
        self.inventory
            .resolved_player_inventory_item_object_with_access_like_cpp(&self.access, guid)
    }

    pub fn get_inventory_item_by_guid_like_cpp(
        &self,
        item_guid: ObjectGuid,
    ) -> Option<(u8, u8, wow_entities::PlayerInventoryItem)> {
        self.inventory
            .get_inventory_item_by_guid_like_cpp(self.hub, item_guid)
    }

    pub fn represented_non_bank_item_count_like_cpp(&self, entry_id: u32) -> Option<u32> {
        self.inventory
            .represented_non_bank_item_count_like_cpp(self.hub, entry_id)
    }

    pub fn represented_quest_status_persistence_rows_like_cpp(
        &self,
        statuses: &[wow_entities::PlayerQuestStatusRecord],
    ) -> Vec<QuestStatusPersistenceLikeCpp> {
        self.hub
            .represented_quest_status_persistence_rows_like_cpp(statuses)
    }

    pub fn item_storage_template(&self, entry: u32) -> Option<ItemStorageTemplate> {
        item_storage_template_like_cpp(self.item_store, self.item_stats_store, entry)
    }

    pub fn item_effect_count_like_cpp(&self, entry: u32) -> usize {
        self.item_effect_store
            .map(|store| {
                store
                    .item_effects_for_item_id_like_cpp(entry)
                    .len()
                    .min(wow_entities::MAX_ITEM_SPELLS)
            })
            .unwrap_or(0)
    }

    pub fn inventory_container_db_guid_like_cpp(&self, bag: u8) -> Option<u64> {
        self.inventory
            .inventory_container_db_guid_with_access_like_cpp(&self.access, bag)
    }

    pub fn inventory_remove_enchantment_persistence_like_cpp(
        &self,
        guid: ObjectGuid,
        clear: bool,
    ) -> Option<(String, Vec<wow_constants::item::EnchantmentSlot>)> {
        self.inventory
            .inventory_remove_enchantment_persistence_with_access_like_cpp(
                &self.access,
                self.enchantment_store,
                guid,
                clear,
            )
    }

    pub fn refresh_inventory_item_enchantment_duration_refs_like_cpp(&mut self, guid: ObjectGuid) {
        self.inventory
            .refresh_inventory_item_enchantment_duration_refs_with_access_like_cpp(
                &self.access,
                self.hub,
                guid,
            )
    }

    pub fn apply_inventory_item_object_updates_like_cpp(
        &mut self,
        guid: ObjectGuid,
        updates: &[ItemObjectUpdateLikeCpp],
    ) -> bool {
        self.inventory
            .apply_inventory_item_object_updates_with_access_like_cpp(&self.access, guid, updates)
    }

    pub fn add_inventory_item_duration_refs_like_cpp(&mut self, guid: ObjectGuid) {
        self.inventory
            .add_inventory_item_duration_refs_with_access_like_cpp(
                &self.access,
                &self.publication,
                guid,
            )
    }

    pub fn remove_inventory_item_duration_refs_like_cpp(&mut self, guid: ObjectGuid) {
        self.inventory
            .remove_inventory_item_duration_refs_with_access_like_cpp(&self.access, guid)
    }

    pub fn remove_inventory_tradeable_item_like_cpp(&mut self, guid: ObjectGuid) {
        self.inventory
            .remove_inventory_tradeable_item_with_access_like_cpp(&self.access, guid)
    }

    pub fn record_represented_bank_item_move_like_cpp(
        &mut self,
        move_like_cpp: RepresentedBankItemMoveLikeCpp,
    ) {
        self.inventory
            .record_represented_bank_item_move_like_cpp(move_like_cpp)
    }

    pub fn apply_inventory_item_remove_side_effects_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        guid: ObjectGuid,
        cleared: &[wow_constants::item::EnchantmentSlot],
    ) -> bool {
        super::effects::InventorySwapEffectsCxLikeCpp::new(
            self.inventory,
            self.access.reborrow_like_cpp(),
            self.modifiers.reborrow_like_cpp(),
            self.item_sets.reborrow_like_cpp(),
            self.publication.reborrow_like_cpp(),
            self.mods_catalogs,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.level_like_cpp(),
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.form_like_cpp(),
            self.consumer_test,
        )
        .remove_item_effects_like_cpp(bag, slot, guid, cleared)
    }

    pub fn apply_committed_inventory_item_relocation_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        moved_count: u32,
    ) -> bool {
        super::relocation::InventoryCommittedRelocationCxLikeCpp::new(
            self.inventory,
            self.access.reborrow_like_cpp(),
            self.item_store,
            self.item_stats_store,
        )
        .apply_committed_inventory_item_relocation_like_cpp(
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            moved_count,
        )
    }

    pub fn apply_committed_inventory_item_removal_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        super::relocation::InventoryCommittedRelocationCxLikeCpp::new(
            self.inventory,
            self.access.reborrow_like_cpp(),
            self.item_store,
            self.item_stats_store,
        )
        .apply_committed_inventory_item_removal_like_cpp(
            source_bag,
            source_slot,
            item_guid,
        )
    }

    pub fn send_bag_slot_values_update_like_cpp(&self, bag: u8, slot: u8) {
        self.inventory
            .send_bag_slot_values_update_with_access_like_cpp(
                &self.access,
                &self.publication,
                bag,
                slot,
            );
    }

    pub fn send_item_relocation_values_update_like_cpp(
        &self,
        guid: ObjectGuid,
        dynamic_flags2_changed: bool,
        cleared: &[wow_constants::item::EnchantmentSlot],
    ) {
        self.inventory
            .send_item_relocation_values_update_with_access_like_cpp(
                &self.access,
                &self.publication,
                guid,
                dynamic_flags2_changed,
                cleared,
            );
    }

    pub fn send_item_dynamic_flags_values_update_like_cpp(&self, guid: ObjectGuid) {
        self.inventory
            .send_item_dynamic_flags_values_update_with_access_like_cpp(
                &self.access,
                &self.publication,
                guid,
            );
    }

    pub fn send_player_values_update_from_entity_bridge(
        &self,
        inv_slot_changes: &[(u8, ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) -> bool {
        self.inventory
            .send_player_values_update_from_entity_bridge_with_access_like_cpp(
                &self.access,
                &self.publication,
                self.item_store,
                self.item_stats_store,
                inv_slot_changes,
                visible_item_changes,
                virtual_item_changes,
                buyback_changes,
                coinage,
            )
    }

    pub fn send_packet<P: ServerPacket>(&self, packet: &P) -> bool {
        self.publication.send_packet(packet)
    }

    pub fn send_equip_error(
        &self,
        result: InventoryResult,
        item1: Option<ObjectGuid>,
        item2: Option<ObjectGuid>,
        level: u32,
        category: u32,
    ) {
        self.access
            .send_equip_error_like_cpp(result, item1, item2, level, category);
    }
}

/// Borrowed Stats participant for the committed StorageMove publication.
pub struct InventoryStorageMoveStatsCxLikeCpp<'a> {
    inventory: &'a mut InventoryState,
    access: OwnedInventoryAccessLikeCpp<'a>,
    publication: PacketPublicationAccessLikeCpp<'a>,
    stats: PlayerStatsAccessLikeCpp<'a>,
}

impl<'a> InventoryStorageMoveStatsCxLikeCpp<'a> {
    pub fn new(
        inventory: &'a mut InventoryState,
        access: OwnedInventoryAccessLikeCpp<'a>,
        publication: PacketPublicationAccessLikeCpp<'a>,
        stats: PlayerStatsAccessLikeCpp<'a>,
    ) -> Self {
        Self {
            inventory,
            access,
            publication,
            stats,
        }
    }

    pub fn send_stat_update_like_cpp(&mut self) -> bool {
        let player = self.stats.reborrow_like_cpp();
        CharacterStatsApplicationCxLikeCpp::new(
            player,
            self.inventory,
            self.publication.reborrow_like_cpp(),
        )
        .send_stat_update_like_cpp()
    }

    pub fn access_like_cpp(&self) -> &OwnedInventoryAccessLikeCpp<'a> {
        &self.access
    }
}
