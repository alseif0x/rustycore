// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Bank, reagent bank and void storage.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub async fn handle_autobank_item(&mut self, packet: AutoBankItem) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.handle_autobank_item_with_generator_like_cpp(
            generators.item.as_ref(),
            &catalogs,
            packet,
        )
        .await;
    }

    #[cfg(test)]
    pub async fn handle_autostore_bank_item(&mut self, packet: AutoStoreBankItem) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.handle_autostore_bank_item_with_generator_like_cpp(
            generators.item.as_ref(),
            &catalogs,
            packet,
        )
        .await;
    }

    pub(super) fn send_show_bank_like_cpp(&mut self, banker_guid: ObjectGuid) {
        use wow_packet::packets::misc::NpcInteractionOpenResult;

        // C++ `WorldSession::SendShowBank` resets PlayerMenu::InteractionData
        // and stores the banker as the sole active interaction source.
        self.set_player_interaction_source_like_cpp(banker_guid);
        self.send_packet(&NpcInteractionOpenResult::new(banker_guid, 8));
    }

    /// CMSG_BANKER_ACTIVATE — player talks to a banker.
    /// C++ ref: `HandleBankerActivateOpcode`
    /// (`Handlers/BankHandler.cpp:60-65`) opens banker interaction UI.
    pub async fn handle_banker_activate(&mut self, hello: Hello) {
        info!(
            "BankerActivate {:?} account {}",
            hello.unit, self.account_id
        );
        let Some(_banker) = self.represented_npc_can_interact_with_like_cpp(
            hello.unit,
            NPCFlags1::BANKER.bits(),
            0,
        ) else {
            debug!(
                banker_guid = ?hello.unit,
                account = self.account_id,
                "BankerActivate rejected: NPC missing, out of range, dead, or lacks BANKER flag"
            );
            return;
        };

        // C++ removes fake death only after GetNPCIfCanInteractWith accepts the
        // banker and before SendShowBank replaces InteractionData.
        self.remove_represented_feign_death_if_needed_like_cpp();
        self.send_show_bank_like_cpp(hello.unit);
    }

    /// CMSG_AUTOBANK_ITEM — player moves an inventory item into bank storage.
    ///
    /// C++ ref: `WorldSession::HandleAutoBankItemOpcode`.
    pub async fn handle_autobank_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        packet: AutoBankItem,
    ) {
        if !self.represented_can_use_current_bank_like_cpp() {
            debug!(
                bag = packet.bag,
                slot = packet.slot,
                account = self.account_id,
                "AutoBankItem rejected: player cannot use current bank"
            );
            return;
        }

        let represented_move = RepresentedBankItemMoveLikeCpp {
            to_bank: true,
            inv_update_items: packet.inv_update.items,
            bag: packet.bag,
            slot: packet.slot,
        };
        self.execute_inventory_storage_move_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
            packet.bag,
            packet.slot,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Bank,
            InventoryStorageQuestChecksLikeCpp::AutoBankItemRemoved,
            Some(represented_move),
        )
        .await;
    }

    /// CMSG_AUTOSTORE_BANK_ITEM — player moves a bank item back to inventory, or inventory to bank.
    ///
    /// C++ ref: `WorldSession::HandleAutoStoreBankItemOpcode`.
    pub async fn handle_autostore_bank_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        packet: AutoStoreBankItem,
    ) {
        if !self.represented_can_use_current_bank_like_cpp() {
            debug!(
                bag = packet.bag,
                slot = packet.slot,
                account = self.account_id,
                "AutoStoreBankItem rejected: player cannot use current bank"
            );
            return;
        }

        let target = autostore_bank_target_like_cpp(packet.bag, packet.slot);
        let represented_move = RepresentedBankItemMoveLikeCpp {
            to_bank: target == InventoryStorageTargetLikeCpp::Bank,
            inv_update_items: packet.inv_update.items,
            bag: packet.bag,
            slot: packet.slot,
        };
        self.execute_inventory_storage_move_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
            packet.bag,
            packet.slot,
            NULL_BAG,
            NULL_SLOT,
            target,
            autostore_bank_quest_checks_like_cpp(target),
            Some(represented_move),
        )
        .await;
    }

    /// CMSG_BUY_BANK_SLOT — player buys the next personal bank bag slot.
    ///
    /// C++ ref: `WorldSession::HandleBuyBankSlotOpcode`. C++ mutates the bank
    /// slot count and money in one serialized session turn; its later
    /// `Player::SaveToDB`/`SaveInventoryAndGoldToDB` persists both values from
    /// that coherent state. Rust must cross SQL here because detached group
    /// payouts use the character money row as their durable cap authority, so
    /// persist both fields atomically before publishing either runtime field.
    pub(crate) async fn handle_buy_bank_slot_with_prices_and_generator_like_cpp(
        &mut self,
        prices: &wow_data::BankBagSlotPricesStore,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        buy: BuyBankSlot,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(_banker) =
            self.represented_npc_can_interact_with_like_cpp(buy.guid, NPCFlags1::BANKER.bits(), 0)
        else {
            debug!(
                banker_guid = ?buy.guid,
                account = self.account_id,
                "BuyBankSlot rejected: NPC missing, out of range, dead, or lacks BANKER flag"
            );
            return;
        };

        let Some(current_bank_slots) = self.resolved_player_bank_bag_slot_count_like_cpp() else {
            return;
        };
        let next_slot = u32::from(current_bank_slots) + 1;
        let Some(price) = prices.get(next_slot).map(|entry| entry.cost) else {
            debug!(
                next_slot,
                account = self.account_id,
                "BuyBankSlot rejected: missing BankBagSlotPrices.db2 row"
            );
            return;
        };

        #[cfg(test)]
        let test_commit_result = self.loot_money_persistence_test_result_for_worker_like_cpp();
        #[cfg(not(test))]
        let test_commit_result: Option<bool> = None;

        let lifecycle_port = if test_commit_result.is_some() {
            None
        } else {
            let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
                return;
            };
            Some(port)
        };

        // Close payout admission before reading money. A previously admitted
        // payout either completes and is reconciled first, or this purchase
        // closes first and the payout retries its still-available pool.
        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };

        let Some(old_money) = self.resolved_player_money_like_cpp() else {
            return;
        };
        if old_money < u64::from(price) {
            debug!(
                next_slot,
                price,
                old_money,
                account = self.account_id,
                "BuyBankSlot rejected: not enough money"
            );
            return;
        }

        let new_count = u8::try_from(next_slot).unwrap_or(u8::MAX);
        let new_money = old_money - u64::from(price);

        let money_persistence = if let Some(success) = test_commit_result {
            if !success {
                return;
            }
            money_persistence
        } else {
            let request = wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp {
                player_guid: player_guid.counter() as u64,
                money_after: new_money,
                bank_slot_count: new_count,
            };
            let Some(money_persistence) = self
                .await_exclusive_player_money_transaction_outcome_like_cpp(
                    money_persistence,
                    lifecycle_port
                        .as_ref()
                        .expect("production bank-slot purchase retains its lifecycle port")
                        .persist_bank_slot_purchase_like_cpp(request),
                    old_money,
                    new_money,
                    "bank-slot purchase",
                )
                .await
            else {
                return;
            };
            money_persistence
        };

        // Publish both runtime fields only after the combined SQL COMMIT. Set
        // the values synchronously while admission is still closed, then drop
        // the fence before criteria processing can re-enter persistence.
        if !self.set_player_gold_like_cpp(new_money) {
            self.kick("canonical Player money owner became unavailable after bank-slot COMMIT");
            return;
        }
        if !self.set_player_bank_bag_slot_count_like_cpp(new_count) {
            self.kick("canonical Player bank-slot owner became unavailable after COMMIT");
            return;
        }
        self.send_player_bank_bag_slots_update_like_cpp(new_count);
        self.enqueue_represented_quest_objective_progress_like_cpp(
            RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                old_money,
                new_money,
            },
        );
        self.sync_player_registry_state_like_cpp();
        drop(money_persistence);

        self.send_packet(&UpdateObject::player_money_update(
            player_guid,
            self.player_map_id_like_cpp(),
            new_money,
            None,
        ));
        self.drain_represented_quest_objective_progress_with_generator_like_cpp(
            item_guid_generator,
        )
        .await;
    }

    #[cfg(test)]
    pub async fn handle_buy_bank_slot(&mut self, buy: BuyBankSlot) {
        let prices = self
            .bank_bag_slot_prices_store_for_test_like_cpp()
            .cloned()
            .unwrap_or_else(|| Arc::new(wow_data::BankBagSlotPricesStore::from_entries([])));
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_buy_bank_slot_with_prices_and_generator_like_cpp(
            prices.as_ref(),
            generators.item.as_ref(),
            buy,
        )
        .await;
    }

    /// CMSG_CHANGE_BANK_BAG_SLOT_FLAG — player toggles an ActivePlayer bank bag flag.
    ///
    /// C++ ref: `WorldSession::HandleChangeBankBagSlotFlag`.
    pub async fn handle_change_bank_bag_slot_flag(&mut self, packet: ChangeBankBagSlotFlag) {
        if !self.represented_can_use_current_bank_like_cpp() {
            debug!(
                account = self.account_id,
                "ChangeBankBagSlotFlag rejected: player cannot use current bank"
            );
            return;
        }

        let Ok(slot) = usize::try_from(packet.slot) else {
            return;
        };
        if slot >= 7 {
            debug!(
                slot = packet.slot,
                account = self.account_id,
                "ChangeBankBagSlotFlag rejected: invalid bank bag slot"
            );
            return;
        }
        if packet.flag >= u32::BITS {
            debug!(
                flag = packet.flag,
                account = self.account_id,
                "ChangeBankBagSlotFlag rejected: invalid flag bit"
            );
            return;
        }

        let Some(current) = self.represented_bank_bag_slot_flag_like_cpp(slot) else {
            return;
        };
        let mask = 1u32 << packet.flag;
        let updated = if packet.enabled {
            current | mask
        } else {
            current & !mask
        };
        if !self.set_represented_bank_bag_slot_flag_like_cpp(slot, updated) {
            return;
        }
        self.send_player_bank_bag_slot_flag_update_like_cpp(slot, updated);
    }
}
