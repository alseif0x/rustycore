// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Void-storage handlers.
//!
//! C++ source of truth:
//! `src/server/game/Handlers/VoidStorageHandler.cpp` and
//! `src/server/game/Entities/Player/Player.cpp::{_Load,_Save}VoidStorage`.

use std::collections::HashSet;
use std::sync::Arc;

use num_traits::FromPrimitive;
use wow_constants::unit::NPCFlags1;
use wow_constants::{ClientOpcodes, EnchantmentSlot, ItemContext, ItemModifier};
use wow_database::{CharStatements, SqlTransaction};
use wow_entities::INVENTORY_SLOT_BAG_0;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::void_storage::{
    QueryVoidStorage, SwapVoidItem, UnlockVoidStorage, VoidItemSwapResponse, VoidStorageFailed,
    VoidStorageTransfer, VoidStorageTransferChanges, VoidTransferErrorLikeCpp, VoidTransferResult,
};
use wow_packet::{ClientPacket, WorldPacket};

use crate::session::{InventoryItem, RepresentedVoidStorageItemLikeCpp, WorldSession};

const VOID_STORAGE_UNLOCK_COST_LIKE_CPP: u64 = 100 * 10_000;
const VOID_STORAGE_STORE_ITEM_COST_LIKE_CPP: u64 = 10 * 10_000;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UnlockVoidStorage,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_void_storage_unlock",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryVoidStorage,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_void_storage_query",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::VoidStorageTransfer,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_void_storage_transfer",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SwapVoidItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_void_storage_swap_item",
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
    random_properties: EffectiveVoidStorageRandomPropertiesLikeCpp,
    bag: u8,
    slot: u8,
    db_guid: u64,
    item_guid: wow_core::ObjectGuid,
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

impl WorldSession {
    /// Resolve the state installed by C++ `Item::SetItemRandomProperties`.
    fn effective_void_storage_random_properties_like_cpp(
        &self,
        random_properties_id: i32,
        random_properties_seed: i32,
    ) -> EffectiveVoidStorageRandomPropertiesLikeCpp {
        let mut result = EffectiveVoidStorageRandomPropertiesLikeCpp::default();
        if random_properties_id > 0 {
            let Some(entry) = self
                .item_random_properties_store()
                .and_then(|store| store.get(random_properties_id as u32))
            else {
                return result;
            };
            result.id = random_properties_id;
            // C++ only installs PropertySeed for a suffix; positive random
            // properties keep the newly created item's zero seed.
            for (offset, enchantment_id) in entry.enchantments.iter().take(3).enumerate() {
                result.enchantment_ids[EnchantmentSlot::Property2 as usize + offset] =
                    i32::from(*enchantment_id);
            }
        } else if random_properties_id < 0 {
            let Some(entry) = self
                .item_random_suffix_store()
                .and_then(|store| store.get(random_properties_id.unsigned_abs()))
            else {
                return result;
            };
            result.id = random_properties_id;
            result.seed = random_properties_seed;
            for (offset, enchantment_id) in entry.enchantments.iter().take(3).enumerate() {
                result.enchantment_ids[EnchantmentSlot::Property0 as usize + offset] =
                    i32::from(*enchantment_id);
            }
        }
        result
    }

    fn void_storage_enchantments_db_string_like_cpp(
        enchantment_ids: &[i32; wow_entities::MAX_ENCHANTMENT_SLOT],
    ) -> String {
        enchantment_ids
            .iter()
            .flat_map(|id| [id.to_string(), "0".to_string(), "0".to_string()])
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn apply_effective_void_storage_random_properties_like_cpp(
        item: &mut wow_entities::Item,
        random_properties: &EffectiveVoidStorageRandomPropertiesLikeCpp,
    ) {
        item.set_random_properties_id(random_properties.id);
        item.set_property_seed(random_properties.seed);
        for (slot_index, enchantment_id) in random_properties
            .enchantment_ids
            .iter()
            .copied()
            .enumerate()
        {
            let Some(slot) = EnchantmentSlot::from_usize(slot_index) else {
                continue;
            };
            item.set_enchantment(slot, enchantment_id, 0, 0);
        }
    }

    fn plan_void_storage_destroyed_items_like_cpp(
        &self,
        bag: u8,
        slot: u8,
        inventory_item: InventoryItem,
        cleared_mainhand_enchantments: Vec<wow_constants::EnchantmentSlot>,
    ) -> Vec<PlannedVoidDestroyedInventoryItemLikeCpp> {
        let mut destroyed_items = self
            .represented_inventory_descendants_postorder_like_cpp(inventory_item.guid)
            .into_iter()
            .map(
                |(bag, slot, inventory_item)| PlannedVoidDestroyedInventoryItemLikeCpp {
                    bag,
                    slot,
                    inventory_item,
                    cleared_mainhand_enchantments: Vec::new(),
                },
            )
            .collect::<Vec<_>>();
        destroyed_items.push(PlannedVoidDestroyedInventoryItemLikeCpp {
            bag,
            slot,
            inventory_item,
            cleared_mainhand_enchantments,
        });
        destroyed_items
    }

    fn void_storage_destroy_item_statements_like_cpp(
        char_db: &wow_database::CharacterDatabase,
        player_guid_counter: u64,
        item_db_guid: u64,
    ) -> Vec<wow_database::PreparedStatement> {
        let mut statements = Vec::with_capacity(9);
        let mut delete_inventory = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
        delete_inventory.set_u64(0, player_guid_counter);
        delete_inventory.set_u64(1, item_db_guid);
        statements.push(delete_inventory);
        for cleanup_kind in [
            CharStatements::DEL_ITEM_REFUND_INSTANCE,
            CharStatements::DEL_ITEM_BOP_TRADE,
            CharStatements::DEL_ITEM_INSTANCE_GEMS,
            CharStatements::DEL_ITEM_INSTANCE_TRANSMOG,
            CharStatements::DEL_GIFT,
            CharStatements::DEL_ITEMCONTAINER_ITEMS,
            CharStatements::DEL_ITEMCONTAINER_MONEY,
        ] {
            let mut cleanup = char_db.prepare(cleanup_kind);
            cleanup.set_u64(0, item_db_guid);
            statements.push(cleanup);
        }
        let mut delete_item = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
        delete_item.set_u64(0, item_db_guid);
        statements.push(delete_item);
        statements
    }

    fn apply_committed_void_storage_destroyed_items_like_cpp(
        &mut self,
        destroyed_items: &[PlannedVoidDestroyedInventoryItemLikeCpp],
    ) -> Vec<wow_core::ObjectGuid> {
        let mut destroyed_guids = Vec::with_capacity(destroyed_items.len());
        for destroyed in destroyed_items {
            let _ = self.apply_inventory_item_remove_side_effects_like_cpp(
                destroyed.bag,
                destroyed.slot,
                destroyed.inventory_item.guid,
                &destroyed.cleared_mainhand_enchantments,
            );
            let removed = self.apply_committed_inventory_item_removal_like_cpp(
                destroyed.bag,
                destroyed.slot,
                destroyed.inventory_item.guid,
            );
            debug_assert!(removed);
            destroyed_guids.push(destroyed.inventory_item.guid);
        }
        destroyed_guids
    }

    fn send_void_storage_transfer_result_like_cpp(&self, result: VoidTransferErrorLikeCpp) {
        self.send_packet(&VoidTransferResult { result });
    }

    pub async fn handle_void_storage_unlock(&mut self, mut pkt: WorldPacket) {
        let Ok(unlock) = UnlockVoidStorage::read(&mut pkt) else {
            return;
        };
        if self
            .represented_npc_can_interact_with_like_cpp(
                unlock.npc,
                NPCFlags1::VAULT_KEEPER.bits(),
                0,
            )
            .is_none()
            || self.void_storage_is_unlocked_like_cpp()
        {
            return;
        }

        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(char_db) = self.char_db().map(Arc::clone) else {
            return;
        };
        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };

        // C++ ModifyMoney clamps at zero; unlocking does not have a separate
        // HasEnoughMoney gate in this audited branch.
        let old_money = self.player_gold_like_cpp();
        let new_money = old_money.saturating_sub(VOID_STORAGE_UNLOCK_COST_LIKE_CPP);
        let new_flags = self.represented_player_flags_value_like_cpp()
            | crate::session::PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP;
        let mut tx = SqlTransaction::new();
        let mut update_money = char_db.prepare(CharStatements::UPD_CHAR_MONEY);
        update_money.set_u64(0, new_money);
        update_money.set_u64(1, player_guid.counter() as u64);
        tx.append(update_money);
        let mut update_flags = char_db.prepare(CharStatements::UPD_CHAR_PLAYER_FLAGS);
        update_flags.set_u32(0, new_flags);
        update_flags.set_u64(1, player_guid.counter() as u64);
        tx.append(update_flags);

        let Some(money_persistence) = self
            .commit_exclusive_player_money_transaction_like_cpp(
                money_persistence,
                char_db.as_ref(),
                tx,
                old_money,
                new_money,
                "void-storage unlock",
            )
            .await
        else {
            return;
        };

        self.stage_player_money_change_like_cpp(old_money, new_money);
        self.apply_committed_void_storage_unlock_like_cpp();
        self.sync_object_accessor_player();
        self.sync_player_registry_state_like_cpp();
        drop(money_persistence);

        self.drain_represented_quest_objective_progress_like_cpp()
            .await;
        if old_money != new_money {
            self.send_player_values_update_from_entity_bridge(&[], &[], &[], &[], Some(new_money));
        }
    }

    pub async fn handle_void_storage_query(&mut self, mut pkt: WorldPacket) {
        let Ok(query) = QueryVoidStorage::read(&mut pkt) else {
            return;
        };
        if self
            .represented_npc_can_interact_with_like_cpp(
                query.npc,
                (NPCFlags1::TRANSMOGRIFIER | NPCFlags1::VAULT_KEEPER).bits(),
                0,
            )
            .is_none()
            || !self.void_storage_is_unlocked_like_cpp()
            || !self.represented_void_storage_loaded_like_cpp()
        {
            self.send_packet(&VoidStorageFailed::default());
            return;
        }

        self.send_packet(&self.represented_void_storage_contents_like_cpp());
    }

    pub async fn handle_void_storage_transfer(&mut self, mut pkt: WorldPacket) {
        let Ok(transfer) = VoidStorageTransfer::read(&mut pkt) else {
            return;
        };
        if self
            .represented_npc_can_interact_with_like_cpp(
                transfer.npc,
                NPCFlags1::VAULT_KEEPER.bits(),
                0,
            )
            .is_none()
            || !self.void_storage_is_unlocked_like_cpp()
            || !self.represented_void_storage_loaded_like_cpp()
        {
            return;
        }

        // These three admission checks intentionally use the request lengths,
        // before invalid GUIDs are skipped, exactly like C++.
        if transfer.deposits.len() > self.represented_void_storage_free_slots_like_cpp() {
            self.send_void_storage_transfer_result_like_cpp(VoidTransferErrorLikeCpp::Full);
            return;
        }
        let empty_positions = self.represented_empty_inventory_positions_like_cpp();
        if transfer.withdrawals.len() > empty_positions.len() {
            self.send_void_storage_transfer_result_like_cpp(
                VoidTransferErrorLikeCpp::InventoryFull,
            );
            return;
        }
        let requested_cost =
            (transfer.deposits.len() as u64).saturating_mul(VOID_STORAGE_STORE_ITEM_COST_LIKE_CPP);
        if self.player_gold_like_cpp() < requested_cost {
            self.send_void_storage_transfer_result_like_cpp(
                VoidTransferErrorLikeCpp::NotEnoughMoney,
            );
            return;
        }

        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(char_db) = self.char_db().map(Arc::clone) else {
            return;
        };

        let mut planned_deposits = Vec::new();
        let mut used_deposit_guids = HashSet::new();
        let mut reserved_destroyed_guids = HashSet::new();
        let mut reserved_void_slots = HashSet::new();
        for deposit_guid in transfer.deposits {
            if !used_deposit_guids.insert(deposit_guid)
                || reserved_destroyed_guids.contains(&deposit_guid)
            {
                continue;
            }
            let Some((bag, slot, inventory_item)) =
                self.get_inventory_item_by_guid_like_cpp(deposit_guid)
            else {
                continue;
            };
            let Some(runtime_item) = self
                .inventory_item_objects_like_cpp()
                .get(&inventory_item.guid)
                .cloned()
            else {
                continue;
            };
            let Some(void_item_id) = self.next_represented_void_storage_item_id_like_cpp() else {
                self.send_void_storage_transfer_result_like_cpp(
                    VoidTransferErrorLikeCpp::InternalError1,
                );
                return;
            };
            let Some(void_slot) = (0
                ..wow_packet::packets::void_storage::VOID_STORAGE_MAX_SLOT_LIKE_CPP)
                .find(|candidate| {
                    self.represented_void_storage_item_at_like_cpp(*candidate as u8)
                        .is_none()
                        && !reserved_void_slots.contains(candidate)
                })
                .and_then(|slot| u8::try_from(slot).ok())
            else {
                self.send_void_storage_transfer_result_like_cpp(VoidTransferErrorLikeCpp::Full);
                return;
            };
            reserved_void_slots.insert(usize::from(void_slot));
            let Some((_, cleared_mainhand_enchantments)) = self
                .inventory_remove_enchantment_persistence_like_cpp(
                    inventory_item.guid,
                    bag == INVENTORY_SLOT_BAG_0 && slot == wow_entities::EQUIPMENT_SLOT_MAINHAND,
                )
            else {
                continue;
            };
            let data = runtime_item.data();
            let mut destroyed_items = self.plan_void_storage_destroyed_items_like_cpp(
                bag,
                slot,
                inventory_item,
                cleared_mainhand_enchantments,
            );
            // Planning is detached from runtime publication, so emulate C++'s
            // request-order destruction when a bag and one of its children are
            // both listed: a child already claimed by an earlier deposit is no
            // longer part of the later bag destruction, while a child claimed
            // by an earlier bag makes a later explicit deposit invalid.
            destroyed_items.retain(|destroyed| {
                !reserved_destroyed_guids.contains(&destroyed.inventory_item.guid)
            });
            reserved_destroyed_guids.extend(
                destroyed_items
                    .iter()
                    .map(|destroyed| destroyed.inventory_item.guid),
            );
            planned_deposits.push(PlannedVoidDepositLikeCpp {
                destroyed_items,
                void_item: RepresentedVoidStorageItemLikeCpp {
                    item_id: void_item_id,
                    item_entry: runtime_item.object().entry(),
                    creator_guid: data.creator,
                    fixed_scaling_level: runtime_item.get_modifier(ItemModifier::TimewalkerLevel),
                    random_properties_id: data.random_properties_id,
                    random_properties_seed: data.property_seed,
                    context: u8::try_from(data.context).unwrap_or(0),
                },
                void_slot,
            });
        }

        // Unlike C++'s in-memory-only mutation order, this issue's durability
        // contract requires the mixed request to be one atomic CharacterDB
        // operation. Validate and plan every withdrawal before committing any
        // deposit so a later item-specific storage failure cannot expose a
        // charged/destroyed deposit without the rest of the request.
        let mut planned_withdrawals = Vec::new();
        let mut used_withdrawal_ids = HashSet::new();
        let mut reserved_positions = HashSet::new();
        for withdrawal_guid in transfer.withdrawals {
            let void_item_id = withdrawal_guid.counter() as u64;
            if !used_withdrawal_ids.insert(void_item_id) {
                continue;
            }
            let Some((old_void_slot, void_item)) =
                self.represented_void_storage_item_by_id_like_cpp(void_item_id)
            else {
                continue;
            };
            let destination = empty_positions.iter().copied().find(|&(bag, slot)| {
                !reserved_positions.contains(&(bag, slot))
                    && self
                        .plan_store_new_direct_inventory_item_at(void_item.item_entry, 1, bag, slot)
                        .is_some_and(|(result, destinations, _)| {
                            result == wow_constants::InventoryResult::Ok
                                && destinations.len() == 1
                                && destinations[0].pos == (u16::from(bag) << 8) | u16::from(slot)
                        })
            });
            let Some((bag, slot)) = destination else {
                self.send_void_storage_transfer_result_like_cpp(
                    VoidTransferErrorLikeCpp::InventoryFull,
                );
                return;
            };
            reserved_positions.insert((bag, slot));
            let Some((db_guid, item_guid)) = self
                .allocate_item_instance_guids_like_cpp(1)
                .and_then(|mut ids| ids.pop())
            else {
                self.send_void_storage_transfer_result_like_cpp(
                    VoidTransferErrorLikeCpp::InternalError1,
                );
                return;
            };
            let random_properties = self.effective_void_storage_random_properties_like_cpp(
                void_item.random_properties_id,
                void_item.random_properties_seed,
            );
            planned_withdrawals.push(PlannedVoidWithdrawalLikeCpp {
                old_void_slot,
                void_item,
                random_properties,
                bag,
                slot,
                db_guid,
                item_guid,
            });
        }

        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };
        let old_money = self.player_gold_like_cpp();
        let actual_cost =
            (planned_deposits.len() as u64).saturating_mul(VOID_STORAGE_STORE_ITEM_COST_LIKE_CPP);
        let new_money = old_money.saturating_sub(actual_cost);
        let mut tx = SqlTransaction::new();
        let mut update_money = char_db.prepare(CharStatements::UPD_CHAR_MONEY);
        update_money.set_u64(0, new_money);
        update_money.set_u64(1, player_guid.counter() as u64);
        tx.append(update_money);

        for deposit in &planned_deposits {
            for destroyed in &deposit.destroyed_items {
                for statement in Self::void_storage_destroy_item_statements_like_cpp(
                    char_db.as_ref(),
                    player_guid.counter() as u64,
                    destroyed.inventory_item.db_guid,
                ) {
                    tx.append(statement);
                }
            }
            tx.append(Self::build_void_storage_replace_statement_like_cpp(
                player_guid.counter() as u64,
                deposit.void_slot,
                &deposit.void_item,
            ));
        }

        let (total_played_time, _) = self.current_played_time_values_like_cpp();
        for withdrawal in &planned_withdrawals {
            let item = &withdrawal.void_item;
            let max_durability = self.item_template_max_durability(item.item_entry);
            tx.append(
                Self::build_void_storage_withdrawal_item_insert_statement_like_cpp(
                    withdrawal.db_guid,
                    player_guid.counter() as u64,
                    item,
                    max_durability,
                    total_played_time,
                    withdrawal.random_properties.id,
                    withdrawal.random_properties.seed,
                    &Self::void_storage_enchantments_db_string_like_cpp(
                        &withdrawal.random_properties.enchantment_ids,
                    ),
                ),
            );

            let Some(container_db_guid) = self.inventory_container_db_guid_like_cpp(withdrawal.bag)
            else {
                self.send_void_storage_transfer_result_like_cpp(
                    VoidTransferErrorLikeCpp::InventoryFull,
                );
                return;
            };
            let mut insert_inventory = char_db.prepare(CharStatements::REP_CHAR_INVENTORY_ITEM);
            insert_inventory.set_u64(0, player_guid.counter() as u64);
            insert_inventory.set_u64(1, container_db_guid);
            insert_inventory.set_u8(2, withdrawal.slot);
            insert_inventory.set_u64(3, withdrawal.db_guid);
            tx.append(insert_inventory);
            tx.append(Self::build_void_storage_delete_slot_statement_like_cpp(
                player_guid.counter() as u64,
                withdrawal.old_void_slot,
            ));
        }

        let Some(money_persistence) = self
            .commit_exclusive_player_money_transaction_like_cpp(
                money_persistence,
                char_db.as_ref(),
                tx,
                old_money,
                new_money,
                "void-storage transfer",
            )
            .await
        else {
            self.send_void_storage_transfer_result_like_cpp(
                VoidTransferErrorLikeCpp::TransferUnknown,
            );
            return;
        };

        // Publish the complete durable state before reopening payout
        // admission or reaching any cancellation point.
        self.stage_player_money_change_like_cpp(old_money, new_money);
        let mut added_items = Vec::new();
        let mut removed_items = Vec::new();
        let mut destroyed_deposit_items = Vec::new();
        let map_id = self.player_map_id_like_cpp();
        for deposit in &planned_deposits {
            let parent = deposit
                .destroyed_items
                .last()
                .expect("every void deposit includes its source item");
            let parent_position = (parent.bag, parent.slot);
            let destroyed_guids = self
                .apply_committed_void_storage_destroyed_items_like_cpp(&deposit.destroyed_items);
            destroyed_deposit_items.push((parent_position, destroyed_guids));
            let inserted_slot =
                self.add_represented_void_storage_item_like_cpp(deposit.void_item.clone());
            debug_assert_eq!(inserted_slot, Some(deposit.void_slot));
            added_items.push(self.represented_void_storage_item_packet_like_cpp(
                deposit.void_slot,
                &deposit.void_item,
            ));
        }
        for withdrawal in &planned_withdrawals {
            let removed =
                self.delete_represented_void_storage_item_like_cpp(withdrawal.old_void_slot);
            debug_assert_eq!(removed.as_ref(), Some(&withdrawal.void_item));
            let context =
                ItemContext::from_u8(withdrawal.void_item.context).unwrap_or(ItemContext::None);
            let mut item_object = self.make_inventory_item_object(
                withdrawal.item_guid,
                withdrawal.void_item.item_entry,
                player_guid,
                1,
                self.item_template_max_durability(withdrawal.void_item.item_entry),
                context,
                withdrawal.slot,
            );
            item_object.set_creator(withdrawal.void_item.creator_guid);
            Self::apply_effective_void_storage_random_properties_like_cpp(
                &mut item_object,
                &withdrawal.random_properties,
            );
            item_object.set_context_value(i32::from(withdrawal.void_item.context));
            item_object.set_binding(true);
            let collection_item = item_object.clone();
            let inserted = self.apply_committed_new_inventory_item_at_like_cpp(
                withdrawal.bag,
                withdrawal.slot,
                InventoryItem {
                    guid: withdrawal.item_guid,
                    entry_id: withdrawal.void_item.item_entry,
                    db_guid: withdrawal.db_guid,
                    inventory_type: self
                        .item_template_inventory_type(withdrawal.void_item.item_entry),
                },
                item_object,
            );
            debug_assert!(inserted);
            self.apply_loaded_inventory_item_collection_hooks_like_cpp(&collection_item);
            removed_items.push(wow_core::ObjectGuid::create_item(
                self.realm_id(),
                withdrawal.void_item.item_id as i64,
            ));
        }
        self.sync_object_accessor_player();
        self.sync_player_registry_state_like_cpp();
        drop(money_persistence);

        self.drain_represented_quest_objective_progress_like_cpp()
            .await;
        if old_money != new_money {
            self.send_player_values_update_from_entity_bridge(&[], &[], &[], &[], Some(new_money));
        }
        for ((bag, slot), destroyed_guids) in destroyed_deposit_items {
            self.send_packet(&wow_packet::packets::update::UpdateObject::destroy_objects(
                destroyed_guids,
                map_id,
            ));
            if bag == INVENTORY_SLOT_BAG_0 {
                let visible = (slot < 19)
                    .then_some((slot, 0, 0, 0))
                    .into_iter()
                    .collect::<Vec<_>>();
                let virtual_item = ((15..=17).contains(&slot))
                    .then_some((slot - 15, 0, 0, 0))
                    .into_iter()
                    .collect::<Vec<_>>();
                self.send_player_values_update_from_entity_bridge(
                    &[(slot, wow_core::ObjectGuid::EMPTY)],
                    &visible,
                    &virtual_item,
                    &[],
                    None,
                );
            } else {
                self.send_bag_slot_values_update_like_cpp(bag, slot);
            }
        }
        for withdrawal in &planned_withdrawals {
            if withdrawal.bag == INVENTORY_SLOT_BAG_0 {
                self.send_player_values_update_from_entity_bridge(
                    &[(withdrawal.slot, withdrawal.item_guid)],
                    &[],
                    &[],
                    &[],
                    None,
                );
            } else {
                self.send_bag_slot_values_update_like_cpp(withdrawal.bag, withdrawal.slot);
            }
        }
        self.send_packet(&VoidStorageTransferChanges {
            removed_items,
            added_items,
        });
        self.send_void_storage_transfer_result_like_cpp(VoidTransferErrorLikeCpp::NoError);
    }

    pub async fn handle_void_storage_swap_item(&mut self, mut pkt: WorldPacket) {
        let Ok(swap) = SwapVoidItem::read(&mut pkt) else {
            return;
        };
        if self
            .represented_npc_can_interact_with_like_cpp(swap.npc, NPCFlags1::VAULT_KEEPER.bits(), 0)
            .is_none()
            || !self.void_storage_is_unlocked_like_cpp()
            || !self.represented_void_storage_loaded_like_cpp()
        {
            return;
        }

        let Some((old_slot, source_item)) =
            self.represented_void_storage_item_by_id_like_cpp(swap.void_item_guid.counter() as u64)
        else {
            return;
        };
        let Ok(new_slot) = u8::try_from(swap.dst_slot) else {
            self.send_void_storage_transfer_result_like_cpp(
                VoidTransferErrorLikeCpp::InternalError1,
            );
            return;
        };
        let destination_item = self.represented_void_storage_item_at_like_cpp(new_slot);
        if old_slot == new_slot
            || usize::from(new_slot)
                >= wow_packet::packets::void_storage::VOID_STORAGE_MAX_SLOT_LIKE_CPP
        {
            self.send_void_storage_transfer_result_like_cpp(
                VoidTransferErrorLikeCpp::InternalError1,
            );
            return;
        }

        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(char_db) = self.char_db().map(Arc::clone) else {
            return;
        };
        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };
        let money = self.player_gold_like_cpp();
        let mut tx = SqlTransaction::new();
        tx.append(Self::build_void_storage_replace_statement_like_cpp(
            player_guid.counter() as u64,
            new_slot,
            &source_item,
        ));
        match &destination_item {
            Some(item) => tx.append(Self::build_void_storage_replace_statement_like_cpp(
                player_guid.counter() as u64,
                old_slot,
                item,
            )),
            None => tx.append(Self::build_void_storage_delete_slot_statement_like_cpp(
                player_guid.counter() as u64,
                old_slot,
            )),
        }
        let Some(money_persistence) = self
            .commit_exclusive_player_money_transaction_like_cpp(
                money_persistence,
                char_db.as_ref(),
                tx,
                money,
                money,
                "void-storage slot swap",
            )
            .await
        else {
            self.send_void_storage_transfer_result_like_cpp(
                VoidTransferErrorLikeCpp::InternalError1,
            );
            return;
        };
        let swapped = self.swap_represented_void_storage_item_like_cpp(old_slot, new_slot);
        debug_assert!(swapped);
        drop(money_persistence);

        self.send_packet(&VoidItemSwapResponse {
            void_item_a: swap.void_item_guid,
            void_item_b: destination_item
                .as_ref()
                .map_or(wow_core::ObjectGuid::EMPTY, |item| {
                    wow_core::ObjectGuid::create_item(self.realm_id(), item.item_id as i64)
                }),
            void_item_slot_a: u32::from(new_slot),
            void_item_slot_b: destination_item.as_ref().map_or(0, |_| u32::from(old_slot)),
        });
    }
}

#[cfg(test)]
#[path = "void_storage_tests.rs"]
mod tests;
