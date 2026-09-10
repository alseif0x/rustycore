// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Void-storage handlers.
//!
//! C++ source of truth:
//! `src/server/game/Handlers/VoidStorageHandler.cpp` and
//! `src/server/game/Entities/Player/Player.cpp::{_Load,_Save}VoidStorage`.

use std::collections::{HashMap, HashSet};

use num_traits::FromPrimitive;
use wow_constants::unit::NPCFlags1;
use wow_constants::{ClientOpcodes, EnchantmentSlot, ItemContext, ItemFieldFlags, ItemModifier};
use wow_entities::INVENTORY_SLOT_BAG_0;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::packets::update::{ItemCreateData, ItemEnchantmentValuesUpdate, UpdateObject};
use wow_packet::packets::void_storage::{
    QueryVoidStorage, SwapVoidItem, UnlockVoidStorage, VoidItemSwapResponse, VoidStorageFailed,
    VoidStorageTransfer, VoidStorageTransferChanges, VoidTransferErrorLikeCpp, VoidTransferResult,
};
use wow_packet::{ClientPacket, WorldPacket};

use crate::session::{
    DirectInventoryStorageOverlayLikeCpp, InventoryItem, RepresentedVoidStorageItemLikeCpp,
    SessionIdGeneratorsLikeCpp, WorldSession,
};

const VOID_STORAGE_UNLOCK_COST_LIKE_CPP: u64 = 100 * 10_000;
const VOID_STORAGE_STORE_ITEM_COST_LIKE_CPP: u64 = 10 * 10_000;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UnlockVoidStorage,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_void_storage_unlock",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_void_storage_unlock_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryVoidStorage,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_void_storage_query",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_void_storage_query(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::VoidStorageTransfer,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_void_storage_transfer",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_void_storage_transfer_with_generators_like_cpp(
                        catalogs.id_generators.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SwapVoidItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_void_storage_swap_item",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_void_storage_swap_item(pkt).await })
        },
    }
}

#[derive(Debug, Clone)]
struct PlannedVoidDepositLikeCpp {
    destroyed_items: Vec<PlannedVoidDestroyedInventoryItemLikeCpp>,
    void_item: RepresentedVoidStorageItemLikeCpp,
    void_slot: u8,
}

#[derive(Debug, Clone)]
struct PlannedVoidDestroyedInventoryItemLikeCpp {
    bag: u8,
    slot: u8,
    inventory_item: InventoryItem,
    cleared_mainhand_enchantments: Vec<wow_constants::EnchantmentSlot>,
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

enum VoidWithdrawalItemPublicationLikeCpp {
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

fn void_withdrawal_initial_item_flags_like_cpp(
    template: Option<&wow_entities::ItemStorageTemplate>,
    bag: u8,
    slot: u8,
) -> u32 {
    let mut item = wow_entities::Item::new(0);
    if let Some(template) = template {
        item.set_bonding(template.bonding);
    }
    item.set_item_flag(ItemFieldFlags::NEW_ITEM);
    item.bind_if_stored(wow_entities::is_bag_pos(wow_entities::make_item_pos(
        bag, slot,
    )));
    item.item_flags_bits()
}

fn void_withdrawal_item_create_data_like_cpp(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EffectiveVoidStorageRandomPropertiesLikeCpp {
    id: i32,
    seed: i32,
    enchantment_ids: [i32; wow_entities::MAX_ENCHANTMENT_SLOT],
}

impl Default for EffectiveVoidStorageRandomPropertiesLikeCpp {
    fn default() -> Self {
        Self {
            id: 0,
            seed: 0,
            enchantment_ids: [0; wow_entities::MAX_ENCHANTMENT_SLOT],
        }
    }
}

#[cfg(test)]
#[path = "void_storage_tests/mod.rs"]
mod tests;

mod items;
mod publication;
mod swap;
mod transfer;
mod unlock_and_query;
