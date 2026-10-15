// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Void-storage session bodies (#1363 slice 4c, moved from the World
//! `handlers/void_storage` tree, the void-storage adapter and the unlock
//! commit).
//!
//! C++ source of truth:
//! `src/server/game/Handlers/VoidStorageHandler.cpp` and
//! `src/server/game/Entities/Player/Player.cpp::{_Load,_Save}VoidStorage`.
//!
//! The four registrations stay in
//! `wow_world_inventory::register_void_storage_handlers_like_cpp`; the World
//! adapter of that registrar calls the handlers below. They live in the
//! application crate, not in inventory, because the transfer composes the
//! inventory moves with the quest item-count bookkeeping
//! (`wow_world_quest`, which sits above inventory): the withdrawal quest plan
//! is a quest type carried across the whole operation.

use std::collections::{HashMap, HashSet};

use num_traits::FromPrimitive;
use wow_constants::unit::NPCFlags1;
use wow_constants::{EnchantmentSlot, ItemContext, ItemFieldFlags, ItemModifier};
use wow_core::{ObjectGuid, VoidStorageItemIdGeneratorLikeCpp};
use wow_entities::{
    INVENTORY_SLOT_BAG_0, ItemPosCount, PlayerInventoryItem as InventoryItem,
    PlayerVoidStorageItemLikeCpp as RepresentedVoidStorageItemLikeCpp,
};

use wow_packet::packets::update::{ItemCreateData, ItemEnchantmentValuesUpdate, UpdateObject};
use wow_packet::packets::void_storage::{
    QueryVoidStorage, SwapVoidItem, UnlockVoidStorage, VoidItemSwapResponse, VoidStorageFailed,
    VoidStorageTransfer, VoidStorageTransferChanges, VoidTransferErrorLikeCpp, VoidTransferResult,
};
use wow_packet::{ClientPacket, WorldPacket};

use wow_world_core::session::{
    HubMut, InventoryPlayerProjectionLikeCpp, PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP,
    SessionIdGeneratorsLikeCpp,
};
use wow_world_inventory::{
    BattlePetPurchaseHostLikeCpp, BattlePetSessionHostLikeCpp,
    DirectInventoryStorageOverlayLikeCpp, EffectiveVoidStorageRandomPropertiesLikeCpp,
    ItemSessionHostLikeCpp, PlannedVoidDestroyedInventoryItemLikeCpp, VendorSessionHostLikeCpp,
};
use wow_world_loot::LootState;
use wow_world_quest::QuestItemStorageHostLikeCpp;

mod items;
mod publication;
mod storage;
mod swap;
mod transfer;
mod unlock_and_query;

pub use items::*;
pub use publication::*;
pub use storage::*;
pub use swap::*;
pub use transfer::*;
pub use unlock_and_query::*;

/// The World operations the moved void-storage bodies call beyond the item
/// session and quest item-storage hosts (#1363 slice 4c). Each method is the
/// same-named `WorldSession` operation (or, for `split_loot_mut`, the session
/// split), called at the point the body called it before the move.
pub trait VoidStorageSessionHostLikeCpp:
    ItemSessionHostLikeCpp + QuestItemStorageHostLikeCpp
{
    fn split_loot_mut(&mut self) -> (&mut LootState, HubMut<'_>);
    fn void_storage_is_unlocked_like_cpp(&self) -> bool;
    fn represented_player_flags_value_like_cpp(&self) -> Option<u32>;
    fn player_values_update_snapshot(&self) -> Option<InventoryPlayerProjectionLikeCpp>;
    fn get_inventory_item_by_guid_like_cpp(
        &self,
        item_guid: ObjectGuid,
    ) -> Option<(u8, u8, InventoryItem)>;
    fn represented_empty_inventory_positions_like_cpp(&self) -> Option<Vec<(u8, u8)>>;
    fn inventory_remove_enchantment_persistence_like_cpp(
        &self,
        item_guid: ObjectGuid,
        clear_mainhand_only: bool,
    ) -> Option<(String, Vec<EnchantmentSlot>)>;
    fn plan_store_new_direct_inventory_item_with_overlays_like_cpp(
        &self,
        entry_id: u32,
        count: u32,
        overlays: &[DirectInventoryStorageOverlayLikeCpp],
        vacated_positions: &[(u8, u8)],
    ) -> Option<(
        wow_constants::InventoryResult,
        Vec<ItemPosCount>,
        Option<u32>,
    )>;
    fn apply_committed_new_inventory_item_at_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        inventory_item: InventoryItem,
        item_object: wow_entities::Item,
    ) -> bool;
    fn refresh_inventory_item_enchantment_duration_refs_like_cpp(&mut self, item_guid: ObjectGuid);
    fn send_bag_slot_values_update_like_cpp(&self, bag_slot: u8, changed_slot: u8);
}

const VOID_STORAGE_UNLOCK_COST_LIKE_CPP: u64 = 100 * 10_000;
const VOID_STORAGE_STORE_ITEM_COST_LIKE_CPP: u64 = 10 * 10_000;

#[derive(Debug, Clone)]
struct PlannedVoidDepositLikeCpp {
    destroyed_items: Vec<PlannedVoidDestroyedInventoryItemLikeCpp>,
    void_item: RepresentedVoidStorageItemLikeCpp,
    void_slot: u8,
}

#[derive(Debug, Clone)]
struct PlannedVoidWithdrawalLikeCpp {
    old_void_slot: u8,
    void_item: RepresentedVoidStorageItemLikeCpp,
    quest_log_item_id: u32,
    destination: PlannedVoidWithdrawalDestinationLikeCpp,
}

#[derive(Debug, Clone)]
enum PlannedVoidWithdrawalDestinationLikeCpp {
    QuestBoundNoItem,
    New {
        bag: u8,
        slot: u8,
        db_guid: u64,
        item_guid: wow_core::ObjectGuid,
        item_state: RepresentedVoidStorageItemLikeCpp,
        item_object: wow_entities::Item,
        create_item_object: wow_entities::Item,
        post_store_item_object: wow_entities::Item,
        enchantments: String,
        create_dynamic_flags: u32,
    },
    MergeExisting {
        inventory_item: InventoryItem,
        item_object: wow_entities::Item,
        store_stack_count: u32,
        store_dynamic_flags: Option<u32>,
        post_store_item_object: wow_entities::Item,
        enchantments: String,
    },
    MergedIntoPlanned {
        item_guid: wow_core::ObjectGuid,
        store_stack_count: u32,
        store_dynamic_flags: Option<u32>,
        post_store_item_object: wow_entities::Item,
    },
}

pub enum VoidWithdrawalItemPublicationLikeCpp {
    New {
        create: ItemCreateData,
        post_store: Option<UpdateObject>,
    },
    Values(UpdateObject),
}

#[derive(Debug, Clone)]
struct PlannedVoidDestinationStateLikeCpp {
    target: PlannedVoidDestinationTargetLikeCpp,
    item_object: wow_entities::Item,
    enchantments: String,
}

#[derive(Debug, Clone)]
enum PlannedVoidDestinationTargetLikeCpp {
    Existing(InventoryItem),
    Planned(usize),
}

pub fn void_withdrawal_item_create_data_like_cpp(
    item: &wow_entities::Item,
    create_dynamic_flags: u32,
    container_slots: u32,
) -> ItemCreateData {
    let data = item.data();

    ItemCreateData {
        item_guid: item.object().guid(),
        entry_id: item.object().entry() as i32,
        owner_guid: data.owner,
        contained_in: data.contained_in,
        stack_count: data.stack_count,
        dynamic_flags: create_dynamic_flags,
        durability: data.durability,
        max_durability: data.max_durability,
        // C++ sends `_StoreItem`'s create before `StoreNewItem` applies
        // random properties and before the void handler restores creator and
        // unconditional binding. Those fields follow in one VALUES update.
        random_properties_seed: 0,
        random_properties_id: 0,
        enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
        // Void storage persists neither gems nor container contents.
        gems: Vec::new(),
        context: u8::try_from(data.context).unwrap_or(ItemContext::None as u8),
        container_slots,
        container_item_guids: [wow_core::ObjectGuid::EMPTY; 36],
    }
}
