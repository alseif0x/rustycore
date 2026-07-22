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
use std::sync::Arc;

use num_traits::FromPrimitive;
use wow_constants::unit::NPCFlags1;
use wow_constants::{ClientOpcodes, EnchantmentSlot, ItemContext, ItemFieldFlags, ItemModifier};
use wow_database::{CharStatements, SqlTransaction};
use wow_entities::INVENTORY_SLOT_BAG_0;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::update::{ItemCreateData, ItemEnchantmentValuesUpdate, UpdateObject};
use wow_packet::packets::void_storage::{
    QueryVoidStorage, SwapVoidItem, UnlockVoidStorage, VoidItemSwapResponse, VoidStorageFailed,
    VoidStorageTransfer, VoidStorageTransferChanges, VoidTransferErrorLikeCpp, VoidTransferResult,
};
use wow_packet::{ClientPacket, WorldPacket};

use crate::session::{
    DirectInventoryStorageOverlayLikeCpp, InventoryItem, RepresentedVoidStorageItemLikeCpp,
    WorldSession,
};

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
        enchantments: String,
        create_dynamic_flags: u32,
    },
    MergeExisting {
        inventory_item: InventoryItem,
        item_object: wow_entities::Item,
        enchantments: String,
    },
    MergedIntoPlanned,
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
        container_slots: 0,
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

impl WorldSession {
    /// C++ passes packet `uint32 DstSlot` to helpers taking `uint8`, so the
    /// language conversion truncates before the 160-slot range check.
    fn void_storage_swap_destination_slot_like_cpp(dst_slot: u32) -> u8 {
        dst_slot as u8
    }

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

    fn overwrite_void_storage_random_property_enchantments_like_cpp(
        enchantments: &str,
        random_properties: &EffectiveVoidStorageRandomPropertiesLikeCpp,
    ) -> String {
        let mut fields = enchantments
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if fields.len() != wow_entities::MAX_ENCHANTMENT_SLOT * 3 {
            fields = vec!["0".to_string(); wow_entities::MAX_ENCHANTMENT_SLOT * 3];
        }
        let slots = if random_properties.id > 0 {
            EnchantmentSlot::Property2 as usize..=EnchantmentSlot::Property4 as usize
        } else if random_properties.id < 0 {
            EnchantmentSlot::Property0 as usize..=EnchantmentSlot::Property2 as usize
        } else {
            return fields.join(" ");
        };
        for slot in slots {
            let base = slot * 3;
            fields[base] = random_properties.enchantment_ids[slot].to_string();
            fields[base + 1] = "0".to_string();
            fields[base + 2] = "0".to_string();
        }
        fields.join(" ")
    }

    fn build_void_storage_merged_item_update_statement_like_cpp(
        &self,
        char_db: &wow_database::CharacterDatabase,
        inventory_item: &InventoryItem,
        item: &wow_entities::Item,
        enchantments: &str,
    ) -> wow_database::PreparedStatement {
        let data = item.data();
        let mut charges = String::new();
        for charge in data
            .spell_charges
            .iter()
            .take(self.item_effect_count_like_cpp(item.object().entry()))
        {
            charges.push_str(&charge.to_string());
            charges.push(' ');
        }

        let mut statement = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE);
        statement.set_u32(0, item.object().entry());
        statement.set_u64(1, data.owner.counter() as u64);
        statement.set_u64(2, data.creator.counter() as u64);
        statement.set_u64(3, data.gift_creator.counter() as u64);
        statement.set_u32(4, item.count());
        statement.set_u32(5, data.expiration);
        statement.set_string(6, charges);
        statement.set_u32(7, data.dynamic_flags);
        statement.set_string(8, enchantments);
        statement.set_u32(9, data.durability);
        statement.set_u32(10, data.create_played_time);
        statement.set_string(11, item.text());
        statement.set_u32(12, item.get_modifier(ItemModifier::BattlePetSpeciesId));
        statement.set_u32(13, item.get_modifier(ItemModifier::BattlePetBreedData));
        statement.set_u32(14, item.get_modifier(ItemModifier::BattlePetLevel));
        statement.set_u32(15, item.get_modifier(ItemModifier::BattlePetDisplayId));
        statement.set_i32(16, data.random_properties_id);
        statement.set_i32(17, data.property_seed);
        statement.set_i32(18, data.context);
        statement.set_u64(19, inventory_item.db_guid);
        statement
    }

    fn apply_effective_void_storage_random_properties_like_cpp(
        item: &mut wow_entities::Item,
        random_properties: &EffectiveVoidStorageRandomPropertiesLikeCpp,
    ) {
        if random_properties.id == 0 {
            return;
        }
        item.set_random_properties_id(random_properties.id);
        item.set_property_seed(random_properties.seed);
        let slots = if random_properties.id > 0 {
            EnchantmentSlot::Property2 as usize..=EnchantmentSlot::Property4 as usize
        } else {
            EnchantmentSlot::Property0 as usize..=EnchantmentSlot::Property2 as usize
        };
        for slot_index in slots {
            if let Some(slot) = EnchantmentSlot::from_usize(slot_index) {
                item.set_enchantment(slot, random_properties.enchantment_ids[slot_index], 0, 0);
            }
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
    ) -> (Vec<wow_core::ObjectGuid>, Vec<u32>) {
        let mut destroyed_guids = Vec::with_capacity(destroyed_items.len());
        let mut changed_quest_ids = Vec::new();
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
            // C++ recursive DestroyItem runs ItemRemovedQuestCheck after each
            // child/parent removal, preserving intermediate objective updates.
            changed_quest_ids
                .extend(self.apply_quest_item_removed_like_cpp(destroyed.inventory_item.entry_id));
        }
        changed_quest_ids.sort_unstable();
        changed_quest_ids.dedup();
        (destroyed_guids, changed_quest_ids)
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
        // A locked login deliberately skipped any residual rows like C++.
        // Persist that coherent empty authority together with the flag so a
        // restart cannot expose stale contents before the next full save.
        tx.append(Self::build_void_storage_delete_all_statement_like_cpp(
            player_guid.counter() as u64,
        ));

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

        // Accepted failure-only divergence required by issue #114's explicit
        // Done contract: money, inventory and every affected void slot commit
        // in one CharacterDB transaction, and a definite failure leaves
        // runtime unchanged. Unlike C++'s intermediate in-memory mutations,
        // validate and plan every withdrawal before committing any deposit so
        // an item-specific storage failure cannot expose a charged/destroyed
        // deposit without the rest of the request. Successful wire/state order
        // remains C++-compatible and is covered separately by capture/runtime QA.
        let mut removed_entry_order = Vec::new();
        let mut removed_non_bank_counts = HashMap::<u32, u32>::new();
        for destroyed in planned_deposits
            .iter()
            .flat_map(|deposit| deposit.destroyed_items.iter())
        {
            removed_entry_order.push(destroyed.inventory_item.entry_id);
            if wow_entities::is_bank_pos(destroyed.bag, destroyed.slot) {
                continue;
            }
            let count = self
                .inventory_item_objects_like_cpp()
                .get(&destroyed.inventory_item.guid)
                .map_or(0, wow_entities::Item::count);
            removed_non_bank_counts
                .entry(destroyed.inventory_item.entry_id)
                .and_modify(|removed| *removed = removed.saturating_add(count))
                .or_insert(count);
        }
        let mut post_removal_non_bank_counts = removed_entry_order
            .iter()
            .copied()
            .collect::<HashSet<_>>()
            .into_iter()
            .map(|entry_id| {
                let removed = removed_non_bank_counts.get(&entry_id).copied().unwrap_or(0);
                (
                    entry_id,
                    self.represented_non_bank_item_count_like_cpp(entry_id)
                        .saturating_sub(removed),
                )
            })
            .collect::<Vec<_>>();
        post_removal_non_bank_counts.sort_unstable_by_key(|(entry_id, _)| *entry_id);
        let mut quest_persistence_plan = self.begin_item_transfer_quest_persistence_like_cpp(
            &removed_entry_order,
            &post_removal_non_bank_counts,
        );
        let mut planned_withdrawals: Vec<PlannedVoidWithdrawalLikeCpp> = Vec::new();
        let mut used_withdrawal_ids = HashSet::new();
        let mut storage_overlays = Vec::new();
        let mut destination_states = HashMap::<(u8, u8), PlannedVoidDestinationStateLikeCpp>::new();
        let vacated_inventory_positions = planned_deposits
            .iter()
            .flat_map(|deposit| deposit.destroyed_items.iter())
            .map(|destroyed| (destroyed.bag, destroyed.slot))
            .collect::<Vec<_>>();
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
            let Some((result, destinations, _)) = self
                .plan_store_new_direct_inventory_item_with_overlays_like_cpp(
                    void_item.item_entry,
                    1,
                    &storage_overlays,
                    &vacated_inventory_positions,
                )
            else {
                self.send_void_storage_transfer_result_like_cpp(
                    VoidTransferErrorLikeCpp::InventoryFull,
                );
                return;
            };
            if result != wow_constants::InventoryResult::Ok
                || destinations.len() != 1
                || destinations[0].count != 1
            {
                self.send_void_storage_transfer_result_like_cpp(
                    VoidTransferErrorLikeCpp::InventoryFull,
                );
                return;
            }
            let [bag, slot] = destinations[0].pos.to_be_bytes();
            let quest_log_item_id = self
                .quest_source_item_quest_log_item_id_like_cpp(void_item.item_entry)
                .await;
            if self.plan_item_transfer_withdrawal_quest_persistence_like_cpp(
                &mut quest_persistence_plan,
                void_item.item_entry,
                quest_log_item_id,
                1,
            ) {
                planned_withdrawals.push(PlannedVoidWithdrawalLikeCpp {
                    old_void_slot,
                    void_item,
                    quest_log_item_id,
                    destination: PlannedVoidWithdrawalDestinationLikeCpp::QuestBoundNoItem,
                });
                continue;
            }
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
            let existing_inventory_item = (!vacated_inventory_positions.contains(&(bag, slot)))
                .then(|| self.get_inventory_item_by_pos(bag, slot))
                .flatten();
            let previous_state = destination_states.get(&(bag, slot)).cloned();
            let (target, mut item_object, base_enchantments) = if let Some(state) = previous_state {
                (state.target, state.item_object, state.enchantments)
            } else if let Some(inventory_item) = existing_inventory_item {
                let Some(item_object) = self
                    .inventory_item_objects_like_cpp()
                    .get(&inventory_item.guid)
                    .cloned()
                else {
                    self.send_void_storage_transfer_result_like_cpp(
                        VoidTransferErrorLikeCpp::InventoryFull,
                    );
                    return;
                };
                let Some((enchantments, _)) = self
                    .inventory_remove_enchantment_persistence_like_cpp(inventory_item.guid, false)
                else {
                    self.send_void_storage_transfer_result_like_cpp(
                        VoidTransferErrorLikeCpp::InventoryFull,
                    );
                    return;
                };
                (
                    PlannedVoidDestinationTargetLikeCpp::Existing(inventory_item),
                    item_object,
                    enchantments,
                )
            } else {
                let context = ItemContext::from_u8(void_item.context).unwrap_or(ItemContext::None);
                (
                    PlannedVoidDestinationTargetLikeCpp::Planned(planned_withdrawals.len()),
                    self.make_inventory_item_object(
                        item_guid,
                        void_item.item_entry,
                        player_guid,
                        1,
                        self.item_template_max_durability(void_item.item_entry),
                        context,
                        slot,
                    ),
                    Self::void_storage_enchantments_db_string_like_cpp(
                        &[0; wow_entities::MAX_ENCHANTMENT_SLOT],
                    ),
                )
            };
            let is_new_destination = matches!(target, PlannedVoidDestinationTargetLikeCpp::Planned(index) if index == planned_withdrawals.len());
            let create_dynamic_flags = if is_new_destination {
                let template = self.item_storage_template(void_item.item_entry);
                if let Some(template) = template.as_ref() {
                    item_object.set_bonding(template.bonding);
                }
                let flags =
                    void_withdrawal_initial_item_flags_like_cpp(template.as_ref(), bag, slot);
                item_object.replace_all_item_flags(ItemFieldFlags::from_bits_retain(flags));
                flags
            } else {
                0
            };
            item_object.set_count(item_object.count().saturating_add(1).max(1));
            // A newly constructed destination already has count one; an
            // existing/planned merge gains the withdrawn unit.
            if matches!(target, PlannedVoidDestinationTargetLikeCpp::Planned(index) if index == planned_withdrawals.len())
            {
                item_object.set_count(1);
            }
            item_object.set_creator(void_item.creator_guid);
            Self::apply_effective_void_storage_random_properties_like_cpp(
                &mut item_object,
                &random_properties,
            );
            // C++ creates the temporary source with the void item's context,
            // but `_StoreItem` keeps the destination stack's context when it
            // merges. A brand-new destination already received this context
            // from `make_inventory_item_object` above.
            item_object.set_binding(true);
            let enchantments = Self::overwrite_void_storage_random_property_enchantments_like_cpp(
                &base_enchantments,
                &random_properties,
            );

            let destination = match target.clone() {
                PlannedVoidDestinationTargetLikeCpp::Existing(inventory_item) => {
                    PlannedVoidWithdrawalDestinationLikeCpp::MergeExisting {
                        inventory_item,
                        item_object: item_object.clone(),
                        enchantments: enchantments.clone(),
                    }
                }
                PlannedVoidDestinationTargetLikeCpp::Planned(index)
                    if index < planned_withdrawals.len() =>
                {
                    let Some(target_withdrawal) = planned_withdrawals.get_mut(index) else {
                        self.send_void_storage_transfer_result_like_cpp(
                            VoidTransferErrorLikeCpp::InternalError1,
                        );
                        return;
                    };
                    let PlannedVoidWithdrawalDestinationLikeCpp::New {
                        item_state,
                        item_object: target_item,
                        enchantments: target_enchantments,
                        ..
                    } = &mut target_withdrawal.destination
                    else {
                        self.send_void_storage_transfer_result_like_cpp(
                            VoidTransferErrorLikeCpp::InternalError1,
                        );
                        return;
                    };
                    // `_StoreItem` keeps the first destination's context when
                    // later temporary items merge into it. The handler still
                    // overwrites the returned stack's creator after each
                    // withdrawal, so only that persisted field follows the
                    // latest item here.
                    item_state.creator_guid = void_item.creator_guid;
                    *target_item = item_object.clone();
                    *target_enchantments = enchantments.clone();
                    PlannedVoidWithdrawalDestinationLikeCpp::MergedIntoPlanned
                }
                PlannedVoidDestinationTargetLikeCpp::Planned(_) => {
                    PlannedVoidWithdrawalDestinationLikeCpp::New {
                        bag,
                        slot,
                        db_guid,
                        item_guid,
                        item_state: void_item.clone(),
                        item_object: item_object.clone(),
                        enchantments: enchantments.clone(),
                        create_dynamic_flags,
                    }
                }
            };

            if let Some(overlay) = storage_overlays
                .iter_mut()
                .find(|overlay| overlay.bag == bag && overlay.slot == slot)
            {
                overlay.entry_id = item_object.object().entry();
                overlay.count = item_object.count();
            } else {
                storage_overlays.push(DirectInventoryStorageOverlayLikeCpp {
                    bag,
                    slot,
                    entry_id: item_object.object().entry(),
                    count: item_object.count(),
                });
            }
            destination_states.insert(
                (bag, slot),
                PlannedVoidDestinationStateLikeCpp {
                    target,
                    item_object,
                    enchantments,
                },
            );
            planned_withdrawals.push(PlannedVoidWithdrawalLikeCpp {
                old_void_slot,
                void_item,
                quest_log_item_id,
                destination,
            });
        }

        let planned_quest_statuses =
            self.finish_item_transfer_quest_persistence_like_cpp(quest_persistence_plan);

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
            match &withdrawal.destination {
                PlannedVoidWithdrawalDestinationLikeCpp::QuestBoundNoItem => {}
                PlannedVoidWithdrawalDestinationLikeCpp::New {
                    bag,
                    slot,
                    db_guid,
                    item_state,
                    item_object,
                    enchantments,
                    ..
                } => {
                    tx.append(
                        Self::build_void_storage_withdrawal_item_insert_statement_like_cpp(
                            *db_guid,
                            player_guid.counter() as u64,
                            &item_state,
                            item_object.count(),
                            item_object.data().max_durability,
                            total_played_time,
                            item_object.data().random_properties_id,
                            item_object.data().property_seed,
                            item_object.item_flags_bits(),
                            &enchantments,
                        ),
                    );
                    let Some(container_db_guid) = self.inventory_container_db_guid_like_cpp(*bag)
                    else {
                        self.send_void_storage_transfer_result_like_cpp(
                            VoidTransferErrorLikeCpp::InventoryFull,
                        );
                        return;
                    };
                    let mut insert_inventory =
                        char_db.prepare(CharStatements::REP_CHAR_INVENTORY_ITEM);
                    insert_inventory.set_u64(0, player_guid.counter() as u64);
                    insert_inventory.set_u64(1, container_db_guid);
                    insert_inventory.set_u8(2, *slot);
                    insert_inventory.set_u64(3, *db_guid);
                    tx.append(insert_inventory);
                }
                PlannedVoidWithdrawalDestinationLikeCpp::MergeExisting {
                    inventory_item,
                    item_object,
                    enchantments,
                } => tx.append(
                    self.build_void_storage_merged_item_update_statement_like_cpp(
                        char_db.as_ref(),
                        inventory_item,
                        item_object,
                        enchantments,
                    ),
                ),
                PlannedVoidWithdrawalDestinationLikeCpp::MergedIntoPlanned => {}
            }
            tx.append(Self::build_void_storage_delete_slot_statement_like_cpp(
                player_guid.counter() as u64,
                withdrawal.old_void_slot,
            ));
        }
        self.append_planned_quest_statuses_to_transaction_like_cpp(
            &mut tx,
            char_db.as_ref(),
            player_guid.counter() as u64,
            &planned_quest_statuses,
        );

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
        let mut new_withdrawal_item_creates = Vec::new();
        let mut collection_updates = Vec::new();
        let mut changed_quest_ids = Vec::new();
        let mut added_changed_quest_ids = Vec::new();
        let map_id = self.player_map_id_like_cpp();
        for deposit in &planned_deposits {
            let parent = deposit
                .destroyed_items
                .last()
                .expect("every void deposit includes its source item");
            let parent_position = (parent.bag, parent.slot);
            let (destroyed_guids, deposit_changed_quest_ids) = self
                .apply_committed_void_storage_destroyed_items_like_cpp(&deposit.destroyed_items);
            changed_quest_ids.extend(deposit_changed_quest_ids);
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
            match &withdrawal.destination {
                PlannedVoidWithdrawalDestinationLikeCpp::QuestBoundNoItem => {
                    added_changed_quest_ids.extend(
                        self.apply_quest_item_added_bound_state_like_cpp(
                            withdrawal.void_item.item_entry,
                            withdrawal.quest_log_item_id,
                            1,
                        ),
                    );
                }
                PlannedVoidWithdrawalDestinationLikeCpp::New {
                    bag,
                    slot,
                    db_guid,
                    item_guid,
                    item_object,
                    create_dynamic_flags,
                    ..
                } => {
                    let inserted = self.apply_committed_new_inventory_item_at_like_cpp(
                        *bag,
                        *slot,
                        InventoryItem {
                            guid: *item_guid,
                            entry_id: item_object.object().entry(),
                            db_guid: *db_guid,
                            inventory_type: self
                                .item_template_inventory_type(item_object.object().entry()),
                        },
                        item_object.clone(),
                    );
                    debug_assert!(inserted);
                    collection_updates
                        .extend(self.on_item_added_to_collection_like_cpp(item_object));
                    if let Some(committed_item) =
                        self.inventory_item_objects_like_cpp().get(item_guid)
                    {
                        new_withdrawal_item_creates.push((
                            void_withdrawal_item_create_data_like_cpp(
                                committed_item,
                                *create_dynamic_flags,
                            ),
                            *create_dynamic_flags,
                        ));
                    } else {
                        debug_assert!(false, "committed void withdrawal item is missing");
                    }
                }
                PlannedVoidWithdrawalDestinationLikeCpp::MergeExisting {
                    inventory_item,
                    item_object,
                    ..
                } => {
                    let updated = self
                        .update_inventory_item_object_like_cpp(inventory_item.guid, |target| {
                            *target = item_object.clone()
                        });
                    debug_assert!(updated);
                    collection_updates
                        .extend(self.on_item_added_to_collection_like_cpp(item_object));
                    self.send_inventory_item_pending_values_update_like_cpp(inventory_item.guid);
                    self.refresh_inventory_item_enchantment_duration_refs_like_cpp(
                        inventory_item.guid,
                    );
                }
                PlannedVoidWithdrawalDestinationLikeCpp::MergedIntoPlanned => {}
            }
            if !matches!(
                &withdrawal.destination,
                PlannedVoidWithdrawalDestinationLikeCpp::QuestBoundNoItem
            ) {
                added_changed_quest_ids.extend(
                    self.apply_quest_item_added_non_bound_state_like_cpp(
                        withdrawal.void_item.item_entry,
                        withdrawal.quest_log_item_id,
                        1,
                    ),
                );
            }
            removed_items.push(wow_core::ObjectGuid::create_item(
                self.realm_id(),
                withdrawal.void_item.item_id as i64,
            ));
        }
        changed_quest_ids.extend(added_changed_quest_ids.iter().copied());
        changed_quest_ids.sort_unstable();
        changed_quest_ids.dedup();
        added_changed_quest_ids.sort_unstable();
        added_changed_quest_ids.dedup();
        let planned_changed_quest_ids = planned_quest_statuses
            .iter()
            .map(|status| status.quest_id)
            .collect::<Vec<_>>();
        debug_assert_eq!(changed_quest_ids, planned_changed_quest_ids);
        self.sync_object_accessor_player();
        self.sync_player_registry_state_like_cpp();
        drop(money_persistence);

        self.drain_represented_quest_objective_progress_like_cpp()
            .await;
        if old_money != new_money {
            self.send_player_values_update_from_entity_bridge(&[], &[], &[], &[], Some(new_money));
        }
        if !new_withdrawal_item_creates.is_empty() {
            // C++ `StoreNewItem(..., true)` publishes each newly withdrawn
            // object before post-store random properties plus the void
            // handler's creator/binding changes produce a VALUES update, and
            // before the player/bag slot starts referencing its GUID.
            let post_store_updates = new_withdrawal_item_creates
                .iter()
                .map(|(create, flags)| (create.item_guid, *flags))
                .collect::<Vec<_>>();
            self.send_packet(&UpdateObject::create_stored_items(
                new_withdrawal_item_creates
                    .into_iter()
                    .map(|(create, _)| create)
                    .collect(),
                map_id,
            ));
            for (item_guid, create_dynamic_flags) in post_store_updates {
                self.send_void_withdrawal_post_store_item_values_update_like_cpp(
                    item_guid,
                    create_dynamic_flags,
                );
            }
        }
        self.publish_quest_item_added_status_changes_like_cpp(&added_changed_quest_ids);
        for update in &collection_updates {
            self.send_player_values_update_like_cpp(update);
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
            if let PlannedVoidWithdrawalDestinationLikeCpp::New {
                bag,
                slot,
                item_guid,
                ..
            } = &withdrawal.destination
            {
                if *bag == INVENTORY_SLOT_BAG_0 {
                    self.send_player_values_update_from_entity_bridge(
                        &[(*slot, *item_guid)],
                        &[],
                        &[],
                        &[],
                        None,
                    );
                } else {
                    self.send_bag_slot_values_update_like_cpp(*bag, *slot);
                }
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
        let new_slot = Self::void_storage_swap_destination_slot_like_cpp(swap.dst_slot);
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
