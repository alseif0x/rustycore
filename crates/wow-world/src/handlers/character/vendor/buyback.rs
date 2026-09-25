// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Vendor buyback transaction and post-commit publication.

use super::*;

impl WorldSession {
    /// Handle CMSG_BUY_BACK_ITEM — player buys back an item from a vendor.
    ///
    /// C++ ref: `WorldSession::HandleBuybackItem`.
    pub async fn handle_buy_back_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        buyback: BuyBackItem,
    ) {
        use wow_packet::packets::update::UpdateObject;

        debug!(
            "BuyBackItem: slot={} from vendor {:?}",
            buyback.slot, buyback.vendor_guid
        );

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };
        let map_id = self.player_map_id_like_cpp();
        if self
            .mutate_world_creature(buyback.vendor_guid, |_| ())
            .is_none()
        {
            self.send_sell_error(SellResult::CantFindVendor, None, ObjectGuid::EMPTY);
            return;
        }

        let Ok(buyback_slot) = u8::try_from(buyback.slot) else {
            self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
            return;
        };
        if !wow_entities::is_buyback_slot(buyback_slot) {
            self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
            return;
        }

        let Some(buyback_items) = self.resolved_buyback_items_like_cpp() else {
            return;
        };
        let buyback_item = match buyback_items.get(&buyback_slot).cloned() {
            Some(item) => item,
            None => {
                self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
                return;
            }
        };
        let Some(runtime_item) = self.resolved_inventory_item_object_like_cpp(buyback_item.guid)
        else {
            self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
            return;
        };

        let buyback_index = (buyback_slot - BUYBACK_SLOT_START) as usize;
        let Some(buyback_price) = self.resolved_buyback_price_like_cpp() else {
            return;
        };
        let price = u64::from(buyback_price[buyback_index]);
        let Some(admission_money) = self.resolved_player_money_like_cpp() else {
            return;
        };
        if admission_money < price {
            self.send_buy_error(
                BuyResult::NotEnoughtMoney,
                Some(buyback.vendor_guid),
                buyback_item.entry_id,
            );
            return;
        }

        let (store_result, store_dest, _) = match self.plan_store_new_direct_inventory_item_at(
            buyback_item.entry_id,
            runtime_item.count(),
            NULL_BAG,
            NULL_SLOT,
        ) {
            Some(plan) => plan,
            None => {
                self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
                return;
            }
        };
        if store_result != InventoryResult::Ok {
            self.send_equip_error(store_result, Some(buyback_item.guid), None, 0, 0);
            return;
        }

        let vendor_trade_port = match self.vendor_trade_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };
        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };
        let Some(old_gold) = self.resolved_player_money_like_cpp() else {
            return;
        };
        let new_gold = old_gold.saturating_sub(price);

        let mut existing_updates = Vec::new();
        let mut persistence_destinations = Vec::new();
        let mut moved_slot = None;
        let mut moved_count = 0u32;
        for dest in &store_dest {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;
            if bag != u8::from(INVENTORY_SLOT_BAG_0) {
                self.send_equip_error(
                    InventoryResult::WrongBagType,
                    Some(buyback_item.guid),
                    None,
                    0,
                    0,
                );
                return;
            }

            if let Some(inv_item) = self.resolved_inventory_item_like_cpp(slot) {
                let Some(existing_item) =
                    self.resolved_inventory_item_object_like_cpp(inv_item.guid)
                else {
                    self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
                    return;
                };
                let new_count = existing_item.count().saturating_add(dest.count);
                existing_updates.push((slot, inv_item.guid, new_count));
                persistence_destinations.push(
                    wow_persistence::VendorBuybackDestinationPersistenceLikeCpp::Merge {
                        item_guid: inv_item.db_guid,
                        new_count,
                    },
                );
            } else {
                if moved_slot.is_some() {
                    self.send_equip_error(
                        InventoryResult::NoSlotAvailable,
                        Some(buyback_item.guid),
                        None,
                        0,
                        0,
                    );
                    return;
                }
                persistence_destinations.push(
                    wow_persistence::VendorBuybackDestinationPersistenceLikeCpp::Move {
                        inventory_slot: slot,
                        item_guid: buyback_item.db_guid,
                        new_count: (runtime_item.count() != dest.count).then_some(dest.count),
                    },
                );
                moved_slot = Some(slot);
                moved_count = dest.count;
            }
        }

        let persistence_request = wow_persistence::VendorTradePersistenceRequestLikeCpp::Buyback(
            wow_persistence::VendorBuybackPersistenceLikeCpp {
                player_guid: player_guid.counter() as u64,
                money_before: old_gold,
                money_after: new_gold,
                destinations: persistence_destinations,
                delete_source_item_guid: moved_slot.is_none().then_some(buyback_item.db_guid),
            },
        );
        let Some(money_persistence) = self
            .await_exclusive_player_money_transaction_outcome_like_cpp(
                money_persistence,
                vendor_trade_port.persist_vendor_trade_like_cpp(persistence_request),
                old_gold,
                new_gold,
                "vendor buyback purchase",
            )
            .await
        else {
            warn!("BuyBackItem: transaction did not commit");
            self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
            return;
        };

        // The same COMMIT moved the buyback item (or merged/deleted it) and
        // charged the player. Mirror that entire durable state before the
        // guard can reopen admission or this future can be cancelled.
        if !self.stage_player_money_change_like_cpp(old_gold, new_gold) {
            self.kick(
                "canonical Player money owner became unavailable after vendor buyback COMMIT",
            );
            return;
        }
        self.remove_buyback_item_like_cpp(buyback_slot);
        self.clear_buyback_slot_metadata_like_cpp(buyback_slot);
        let current_slot_occupied = self
            .resolved_buyback_items_like_cpp()
            .zip(self.resolved_current_buyback_slot_like_cpp())
            .is_some_and(|(items, slot)| items.contains_key(&slot));
        if current_slot_occupied {
            self.set_current_buyback_slot_like_cpp(buyback_slot);
        }

        for &(_, item_guid, new_count) in &existing_updates {
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                item_guid,
                &[wow_entities::ItemObjectUpdateLikeCpp::SetCount(new_count)],
            );
        }

        let mut inv_slot_changes = vec![(buyback_slot, ObjectGuid::EMPTY)];
        if let Some(slot) = moved_slot {
            self.insert_inventory_item_like_cpp(
                slot,
                InventoryItem {
                    guid: buyback_item.guid,
                    entry_id: buyback_item.entry_id,
                    db_guid: buyback_item.db_guid,
                    inventory_type: buyback_item.inventory_type,
                },
            );
            self.set_inventory_item_object_slot(buyback_item.guid, slot);
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                buyback_item.guid,
                &[wow_entities::ItemObjectUpdateLikeCpp::SetCount(moved_count)],
            );
            inv_slot_changes.push((slot, buyback_item.guid));
        } else {
            self.remove_inventory_item_object(buyback_item.guid);
        }
        drop(money_persistence);

        self.drain_represented_quest_objective_progress_with_generator_like_cpp(
            item_guid_generator,
        )
        .await;

        for &(_, item_guid, new_count) in &existing_updates {
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid, map_id, new_count,
            ));
        }
        if moved_slot.is_some() && moved_count != runtime_item.count() {
            self.send_packet(&UpdateObject::item_stack_count_update(
                buyback_item.guid,
                map_id,
                moved_count,
            ));
        }
        self.send_player_values_update_from_entity_bridge(
            &inv_slot_changes,
            &[],
            &[],
            &[(buyback_slot, 0, 0)],
            Some(new_gold),
        );
    }

    #[cfg(test)]
    pub async fn handle_buy_back_item(&mut self, buyback: BuyBackItem) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_buy_back_item_with_generator_like_cpp(generators.item.as_ref(), buyback)
            .await;
    }
}
