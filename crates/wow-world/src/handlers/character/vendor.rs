// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Vendor buy/sell/buyback, extended cost, repair and trainer interaction.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use wow_database::{CharStatements, SqlTransaction};

use super::*;

impl WorldSession {
    pub(super) fn vendor_stock_now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    }

    pub(super) fn vendor_item_current_count(
        &mut self,
        vendor_guid: ObjectGuid,
        item_id: u32,
        max_count: u32,
        incr_time: u32,
        buy_count: u32,
    ) -> u32 {
        if max_count == 0 {
            return 0;
        }

        let key = (vendor_guid, item_id);
        let now = Self::vendor_stock_now_secs();
        let Some(count) = self.vendor_item_counts.get(&key).copied() else {
            return max_count;
        };

        let elapsed = now.saturating_sub(count.last_increment_time);
        let (new_count, full) =
            vendor_buy_stock_refill_count(count.count, elapsed, incr_time, buy_count, max_count);
        if full {
            self.vendor_item_counts.remove(&key);
            max_count
        } else {
            if let Some(count) = self.vendor_item_counts.get_mut(&key) {
                count.count = new_count;
                if incr_time > 0 && elapsed >= u64::from(incr_time) {
                    count.last_increment_time = now;
                }
                count.count
            } else {
                new_count
            }
        }
    }

    pub(super) fn update_vendor_item_current_count(
        &mut self,
        vendor_guid: ObjectGuid,
        item_id: u32,
        max_count: u32,
        incr_time: u32,
        buy_count: u32,
        used_count: u32,
    ) -> u32 {
        if max_count == 0 {
            return 0;
        }

        let current =
            self.vendor_item_current_count(vendor_guid, item_id, max_count, incr_time, buy_count);
        let new_count = current.saturating_sub(used_count);
        self.vendor_item_counts.insert(
            (vendor_guid, item_id),
            crate::session::VendorItemCount {
                count: new_count,
                last_increment_time: Self::vendor_stock_now_secs(),
            },
        );
        new_count
    }

    async fn resolve_vendor_buy_item_by_cpp_slot(
        &self,
        port: Option<&dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>,
        root_entry: u32,
        vendor_slot: u32,
        expected_item_id: u32,
    ) -> Option<VendorBuyItem> {
        #[cfg(test)]
        if let Some(item) = self.vendor_buy_item_test_override_like_cpp() {
            if vendor_slot != 0 || item.item_id != expected_item_id {
                return None;
            }
            return Some(VendorBuyItem {
                item_id: item.item_id,
                item_type: item.item_type,
                max_count: item.max_count,
                incr_time: item.incr_time,
                player_condition_id: item.player_condition_id,
                has_vendor_conditions: item.has_vendor_conditions,
                extended_cost: item.extended_cost,
                buy_price: item.buy_price,
                max_durability: item.max_durability,
                buy_count: item.buy_count,
            });
        }

        let port = port?;

        let mut raw_slot = 0u32;
        let mut expanded = std::collections::HashSet::<u32>::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(root_entry);

        while let Some(vendor_entry) = queue.pop_front() {
            if !expanded.insert(vendor_entry) {
                continue;
            }

            let rows = match port
                .load_vendor_rows_like_cpp(root_entry, vendor_entry)
                .await
            {
                wow_persistence::VendorCatalogOutcomeLikeCpp::Loaded(rows) => rows,
                wow_persistence::VendorCatalogOutcomeLikeCpp::Missing => Vec::new(),
                wow_persistence::VendorCatalogOutcomeLikeCpp::Failed { reason } => {
                    warn!("BuyItem: vendor item query failed for entry {vendor_entry}: {reason}");
                    continue;
                }
            };

            for row in rows {
                let item_id = row.item_id;
                if item_id > 0 {
                    let current_slot = raw_slot;
                    raw_slot = raw_slot.saturating_add(1);
                    let item_type = i32::from(row.item_type);
                    let item_known = self
                        .item_store()
                        .map_or(true, |store| store.get(item_id as u32).is_some());
                    let currency_known = item_type == ItemVendorType::Currency as i32
                        && vendor_currency_type_is_known(
                            self.currency_types_store().map(|store| store.as_ref()),
                            item_id as u32,
                        );
                    if (item_known || currency_known) && current_slot == vendor_slot {
                        let row_item_id = item_id as u32;
                        if row_item_id != expected_item_id {
                            return None;
                        }

                        return Some(VendorBuyItem {
                            item_id: row_item_id,
                            item_type,
                            max_count: row.max_count.max(0) as u32,
                            incr_time: row.incr_time,
                            player_condition_id: row.player_condition_id,
                            has_vendor_conditions: row.has_vendor_conditions,
                            extended_cost: row.extended_cost,
                            buy_price: row.buy_price,
                            max_durability: row.max_durability,
                            buy_count: row.buy_count,
                        });
                    }
                } else if item_id < 0 {
                    queue.push_back((-item_id) as u32);
                }
            }
        }

        None
    }

    pub(crate) async fn clear_buyback_on_logout(&mut self) {
        let guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };
        if self.buyback_items_like_cpp().is_empty() {
            self.clear_buyback_runtime_like_cpp();
            return;
        }

        let port = match self.player_lifecycle_port_like_cpp().map(Arc::clone) {
            Some(port) => port,
            None => return,
        };
        let request = wow_persistence::PlayerBuybackClearRequestLikeCpp {
            player_guid: guid.counter() as u64,
            item_db_guids: self
                .buyback_items_like_cpp()
                .values()
                .map(|item| item.db_guid)
                .collect(),
        };
        match port.clear_buyback_like_cpp(request).await {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    "Failed to clear buyback items on logout for guid {}: {reason}",
                    guid.counter()
                );
                return;
            }
        }

        let removed_guids: Vec<_> = self
            .buyback_items_like_cpp()
            .values()
            .map(|item| item.guid)
            .collect();
        for item_guid in removed_guids {
            self.remove_inventory_item_object(item_guid);
        }
        self.clear_buyback_runtime_like_cpp();
        self.sync_object_accessor_player();
    }

    pub(super) fn vendor_item_conditions_meet_like_cpp(
        condition_store: &ConditionEntriesByTypeStore,
        creature_entry: u32,
        item_id: u32,
        player_object: Option<&WorldObject>,
        vendor_object: Option<&WorldObject>,
        player_unit_snapshot: crate::conditions::ConditionUnitSnapshot,
        player_snapshot: crate::conditions::ConditionPlayerSnapshot,
        vendor_unit_snapshot: Option<crate::conditions::ConditionUnitSnapshot>,
        player_condition_store: Option<&PlayerConditionStore>,
        player_condition_context: Option<PlayerConditionContextLikeCpp<'_>>,
    ) -> bool {
        crate::conditions::is_object_meeting_vendor_item_conditions_like_cpp(
            condition_store,
            creature_entry,
            item_id,
            player_object,
            vendor_object,
            |condition, source_info| {
                source_info.set_unit_target_snapshot(0, player_unit_snapshot);
                source_info.set_player_target_snapshot(0, player_snapshot);
                if let Some(vendor_unit_snapshot) = vendor_unit_snapshot {
                    source_info.set_unit_target_snapshot(1, vendor_unit_snapshot);
                }
                if let (Some(store), Some(context)) =
                    (player_condition_store, player_condition_context)
                {
                    source_info.set_player_condition_store(store);
                    source_info.set_player_condition_context(0, context);
                }
                match crate::conditions::condition_meets_basic_like_cpp(
                    condition,
                    source_info,
                    |current_area, required_area| current_area == required_area,
                ) {
                    crate::conditions::ConditionMeetResult::Evaluated(value) => value,
                    crate::conditions::ConditionMeetResult::Unsupported => false,
                }
            },
        )
    }

    pub(crate) fn send_represented_creature_trainer_gossip_menu_like_cpp(
        &mut self,
        npc_guid: ObjectGuid,
        entry: u32,
        npc_flags: u32,
    ) -> bool {
        let mut gossip_options = Vec::new();
        let mut stored_options = Vec::new();
        if !add_represented_trainer_gossip_option_if_missing_like_cpp(
            &mut gossip_options,
            &mut stored_options,
            npc_flags,
        ) {
            return false;
        }

        let gossip_text = if npc_flags & NPCFlags1::QUEST_GIVER.bits() != 0 {
            self.represented_creature_gossip_text_like_cpp(entry)
        } else {
            Vec::new()
        };

        self.gossip_options = stored_options;
        self.set_player_interaction_source_like_cpp(npc_guid);
        self.send_packet(&GossipMessage {
            gossip_guid: npc_guid,
            gossip_id: 0,
            friendship_faction_id: 0,
            text_id: Some(DEFAULT_GOSSIP_MESSAGE_LIKE_CPP),
            broadcast_text_id: None,
            gossip_options,
            gossip_text,
        });
        true
    }

    /// CMSG_TABARD_VENDOR_ACTIVATE — player talks to a tabard designer.
    /// C++ refs: `HandleTabardVendorActivateOpcode` /
    /// `SendTabardVendorActivate` (`Handlers/NPCHandler.cpp:49-91`).
    pub async fn handle_tabard_vendor_activate(&mut self, mut pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::NpcInteractionOpenResult;
        let guid = pkt
            .read_packed_guid()
            .unwrap_or(wow_core::ObjectGuid::EMPTY);
        info!(
            "TabardVendorActivate {:?} account {}",
            guid, self.account_id
        );
        self.send_packet(&NpcInteractionOpenResult::new(guid, 14)); // GuildTabardVendor
    }

    /// CMSG_REPAIR_ITEM — player repairs item at a repair vendor.
    /// C++ ref: WorldSession::HandleRepairItemOpcode.
    pub async fn handle_repair_item(&mut self, repair: RepairItem) {
        let Some(repair_npc) = self.represented_npc_can_interact_with_like_cpp(
            repair.npc_guid,
            NPCFlags1::REPAIR.bits(),
            0,
        ) else {
            debug!(
                npc_guid = ?repair.npc_guid,
                account = self.account_id,
                "RepairItem rejected: NPC missing, out of range, dead, or lacks REPAIR flag"
            );
            return;
        };

        self.remove_represented_feign_death_if_needed_like_cpp();

        // C++ uses GetReputationPriceDiscount(unit) and RATE_REPAIRCOST.
        let discount_mod = self.reputation_price_discount_for_faction_template_like_cpp(
            repair_npc.faction_template_id,
        );
        let repair_cost_rate = self.repair_cost_rate_like_cpp();

        if !repair.item_guid.is_empty() {
            let repaired = self
                .repair_inventory_item_durability_like_cpp(
                    repair.item_guid,
                    true,
                    discount_mod,
                    repair_cost_rate,
                )
                .await;
            debug!(
                npc_guid = ?repair.npc_guid,
                item_guid = ?repair.item_guid,
                repaired,
                account = self.account_id,
                "RepairItem single-item represented runtime"
            );
            return;
        }

        if repair.use_guild_bank {
            let repaired = self
                .repair_all_inventory_item_durability_with_guild_bank_like_cpp(
                    discount_mod,
                    repair_cost_rate,
                )
                .await;
            debug!(
                npc_guid = ?repair.npc_guid,
                repaired,
                account = self.account_id,
                "RepairItem all-items represented guild-bank runtime"
            );
            return;
        }

        let repaired = self
            .repair_all_inventory_item_durability_with_player_money_like_cpp(
                discount_mod,
                repair_cost_rate,
            )
            .await;
        debug!(
            npc_guid = ?repair.npc_guid,
            repaired,
            account = self.account_id,
            "RepairItem all-items represented runtime"
        );
    }

    /// Handle CMSG_BUY_ITEM — player buys an item from a vendor.
    ///
    /// C++ refs: `HandleBuyItemOpcode` (`Handlers/ItemHandler.cpp:530-564`)
    /// delegates to `Player::BuyItemFromVendorSlot` (`Player.cpp:22362+`).
    pub async fn handle_buy_item(&mut self, buy: BuyItem) {
        use wow_packet::packets::update::{ItemCreateData, UpdateObject};

        debug!(
            "BuyItem: item={} qty={} muid={} from {:?}",
            buy.item_id, buy.quantity, buy.muid, buy.vendor_guid
        );

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };
        let map_id = self.player_map_id_like_cpp();
        let vendor_slot = match vendor_buy_muid_to_cpp_slot(buy.muid) {
            Some(slot) => slot,
            None => return,
        };

        // ── Get vendor NPC entry from creature GUID ──
        let vendor_entry = match self.mutate_world_creature(buy.vendor_guid, |c| c.entry()) {
            Some(entry) => entry,
            None => {
                warn!("BuyItem: vendor {:?} not in creatures", buy.vendor_guid);
                self.send_buy_error(
                    BuyResult::DistanceTooFar,
                    Some(buy.vendor_guid),
                    buy.muid as u32,
                );
                return;
            }
        };

        let vendor_catalog = self.vendor_catalog_persistence_port_like_cpp();

        let condition_store = self.condition_store().cloned();
        let player_condition_store = self.player_condition_store().cloned();
        let player_condition_context = self.represented_player_condition_context_like_cpp();
        if let Some(store) = condition_store.as_ref() {
            let player_condition_object = self.build_condition_player_object_like_cpp();
            let vendor_condition_object =
                self.build_condition_creature_object_like_cpp(buy.vendor_guid);
            let (vendor_object, vendor_unit_snapshot) = vendor_condition_object
                .as_ref()
                .map(|(object, snapshot)| (Some(object), Some(*snapshot)))
                .unwrap_or((None, None));
            if !Self::vendor_item_conditions_meet_like_cpp(
                store.as_ref(),
                vendor_entry,
                buy.item_id as u32,
                player_condition_object.as_ref(),
                vendor_object,
                self.condition_player_unit_snapshot_like_cpp(),
                self.condition_player_snapshot_like_cpp(),
                vendor_unit_snapshot,
                player_condition_store.as_deref(),
                Some(player_condition_context.as_context(self)),
            ) {
                warn!(
                    "BuyItem: conditions not met for creature entry {} item {}",
                    vendor_entry, buy.item_id
                );
                self.send_buy_error(
                    BuyResult::CantFindItem,
                    Some(buy.vendor_guid),
                    buy.item_id as u32,
                );
                return;
            }
        }

        if buy.item_type == ItemVendorType::Currency as i32 {
            if !vendor_currency_type_is_known(
                self.currency_types_store().map(|store| store.as_ref()),
                buy.item_id as u32,
            ) {
                self.send_buy_error(BuyResult::CantFindItem, None, buy.item_id as u32);
                return;
            }

            let quantity = vendor_buy_currency_packet_quantity_to_cpp_count(buy.quantity);
            let vendor_item = match self
                .resolve_vendor_buy_item_by_cpp_slot(
                    vendor_catalog.as_deref(),
                    vendor_entry,
                    vendor_slot,
                    buy.item_id as u32,
                )
                .await
            {
                Some(item) if item.item_type == ItemVendorType::Currency as i32 => item,
                _ => {
                    self.send_buy_error(
                        BuyResult::CantFindItem,
                        Some(buy.vendor_guid),
                        buy.item_id as u32,
                    );
                    return;
                }
            };

            if let Some(result) = vendor_buy_player_condition_block_result_like_cpp(
                vendor_item.player_condition_id,
                player_condition_store.as_deref(),
                Some(player_condition_context.as_context(self)),
            ) {
                self.send_equip_error(result, None, None, 0, 0);
                return;
            }

            if let Some(result) =
                vendor_buy_currency_quantity_block_result(vendor_item.max_count, quantity)
            {
                self.send_equip_error(result, None, None, 0, 0);
                return;
            }

            if vendor_item.extended_cost == 0 {
                self.send_buy_error(BuyResult::CantFindItem, None, buy.item_id as u32);
                return;
            }

            if let Some(result) = vendor_buy_extended_cost_block_result(
                self.item_extended_cost_store().map(|store| store.as_ref()),
                self.currency_types_store().map(|store| store.as_ref()),
                |item_id, amount| self.has_item_count_direct_inventory(item_id, amount),
                |currency_id, amount| self.has_currency(currency_id, amount),
                true,
                vendor_item.extended_cost,
                vendor_item.max_count,
                quantity,
            ) {
                match result {
                    VendorExtendedCostBlock::Equip(result) => {
                        self.send_equip_error(result, None, None, 0, 0);
                    }
                    VendorExtendedCostBlock::Buy(result) => {
                        self.send_buy_error(result, Some(buy.vendor_guid), buy.item_id as u32);
                    }
                    VendorExtendedCostBlock::Silent => {}
                }
                // C++ BuyItemFromVendorSlot returns for every failed extended
                // cost preflight before it derives or commits any costs.
                return;
            }

            let extended_cost_item_costs = vendor_buy_extended_cost_item_costs(
                self.item_extended_cost_store().map(|store| store.as_ref()),
                vendor_item.extended_cost,
                vendor_item.max_count,
                quantity,
            );
            let extended_cost_currency_costs = vendor_buy_extended_cost_currency_costs(
                self.item_extended_cost_store().map(|store| store.as_ref()),
                vendor_item.extended_cost,
                vendor_item.max_count,
                quantity,
            );
            let char_db = match self.char_db() {
                Some(db) => Arc::clone(db),
                None => return,
            };
            let mut item_turnin_changes = Vec::new();
            for &(item_id, amount) in &extended_cost_item_costs {
                let Some(mut changes) =
                    self.plan_destroy_item_count_direct_inventory(item_id, amount)
                else {
                    self.send_equip_error(InventoryResult::VendorMissingTurnins, None, None, 0, 0);
                    return;
                };
                item_turnin_changes.append(&mut changes);
            }
            let mut planned_currencies = self.player_currencies_like_cpp().clone();
            let currency_gain = match self.plan_add_currency_vendor_like_cpp(
                &mut planned_currencies,
                buy.item_id as u32,
                quantity,
            ) {
                Ok(delta) => delta,
                Err(()) => {
                    self.send_equip_error(InventoryResult::VendorMissingTurnins, None, None, 0, 0);
                    return;
                }
            };
            for &(currency_id, amount) in &extended_cost_currency_costs {
                if i32::try_from(amount).is_err()
                    || !Self::plan_remove_currency_like_cpp(
                        &mut planned_currencies,
                        currency_id,
                        amount,
                    )
                {
                    self.send_equip_error(InventoryResult::VendorMissingTurnins, None, None, 0, 0);
                    return;
                }
            }

            let mut tx = SqlTransaction::new();
            Self::append_item_turnin_statements(
                char_db.as_ref(),
                &mut tx,
                player_guid,
                &item_turnin_changes,
            );
            let currency_save = self.plan_player_currency_save_like_cpp(
                player_guid.counter() as u64,
                &mut planned_currencies,
            );
            wow_database::player_lifecycle_adapter::append_player_currency_save_request_like_cpp(
                &mut tx,
                &currency_save,
            );

            // C++ mutates currency plus extended-cost turn-ins in one
            // serialized Player turn. Rust crosses SQL here, so retain the
            // same cancellation/unknown-COMMIT quarantine used by purchases
            // that also change money. Equal money sentinels deliberately make
            // an ambiguous result indeterminate: the money row cannot prove
            // whether these currency/item statements committed.
            let Some(money_persistence) = self
                .begin_exclusive_player_money_persistence_like_cpp()
                .await
            else {
                return;
            };
            let money_marker = self.player_gold_like_cpp();
            let Some(money_persistence) = self
                .commit_exclusive_player_money_transaction_like_cpp(
                    money_persistence,
                    char_db.as_ref(),
                    tx,
                    money_marker,
                    money_marker,
                    "vendor currency purchase",
                )
                .await
            else {
                warn!("BuyItem: currency vendor transaction did not commit");
                self.send_buy_error(
                    BuyResult::CantFindItem,
                    Some(buy.vendor_guid),
                    buy.item_id as u32,
                );
                return;
            };

            // Publish the entire committed state before reopening payout/save
            // admission. No await may split durable success from runtime.
            self.set_player_currencies_like_cpp(planned_currencies);
            self.apply_item_turnin_changes(player_guid, map_id, &item_turnin_changes);
            drop(money_persistence);

            if let Some(delta) = currency_gain {
                let (Some(quantity), Some(amount)) = (
                    i32::try_from(delta.quantity).ok(),
                    i32::try_from(delta.amount).ok(),
                ) else {
                    return;
                };
                let mut packet =
                    SetCurrency::vendor_gain(delta.currency_id as i32, quantity, amount);
                packet.weekly_quantity = delta
                    .weekly_quantity
                    .and_then(|value| i32::try_from(value).ok());
                packet.max_quantity = delta
                    .max_quantity
                    .and_then(|value| i32::try_from(value).ok());
                packet.total_earned = delta
                    .total_earned
                    .and_then(|value| i32::try_from(value).ok());
                packet.suppress_chat_log = delta.suppress_chat_log;
                self.send_packet(&packet);
            }
            for &(currency_id, amount) in &extended_cost_currency_costs {
                let Some(quantity) = i32::try_from(self.player_currency_quantity(currency_id)).ok()
                else {
                    continue;
                };
                let Some(amount) = i32::try_from(amount).ok() else {
                    continue;
                };
                self.send_packet(&SetCurrency::vendor_loss(
                    currency_id as i32,
                    quantity,
                    amount,
                ));
            }
            return;
        }

        if buy.item_type != ItemVendorType::Item as i32 {
            warn!("BuyItem: unsupported item type {}", buy.item_type);
            return;
        }

        // ── Validate: player alive ──
        let quantity = vendor_buy_packet_quantity_to_cpp_count(buy.quantity);
        let (store_bag, store_slot) =
            match vendor_buy_direct_inventory_destination(player_guid, &buy) {
                Some(destination) => destination,
                None => {
                    warn!(
                        "BuyItem: rejected slot {} above C++ MAX_BAG_SIZE {}",
                        buy.slot, MAX_BAG_SIZE
                    );
                    return;
                }
            };

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let vendor_item = match self
            .resolve_vendor_buy_item_by_cpp_slot(
                vendor_catalog.as_deref(),
                vendor_entry,
                vendor_slot,
                buy.item_id as u32,
            )
            .await
        {
            Some(item) if item.item_type == ItemVendorType::Item as i32 => item,
            _ => {
                warn!(
                    "BuyItem: vendor slot {} item {} not found for vendor {}",
                    vendor_slot, buy.item_id, vendor_entry
                );
                self.send_buy_error(
                    BuyResult::CantFindItem,
                    Some(buy.vendor_guid),
                    buy.muid as u32,
                );
                return;
            }
        };
        let sparse_template = self
            .item_stats_store()
            .and_then(|store| store.sparse_template(buy.item_id as u32));
        let allowable_class = sparse_template.map(|template| template.allowable_class);
        let bonding = sparse_template.map(|template| template.bonding);
        let flags2 = sparse_template.map(|template| template.flags[1]);
        let required_reputation_faction =
            sparse_template.map(|template| template.required_reputation_faction);
        let required_reputation_rank =
            sparse_template.map(|template| template.required_reputation_rank);
        if let Some(block) = vendor_buy_template_block_result(
            allowable_class,
            bonding,
            flags2,
            self.player_class_like_cpp(),
            self.player_race_like_cpp(),
            self.security > 0,
        ) {
            match block {
                VendorBuyTemplateBlock::BuyError(result) => {
                    self.send_buy_error(result, None, buy.item_id as u32);
                }
                VendorBuyTemplateBlock::Silent => {}
            }
            return;
        }
        if condition_store.is_none()
            && let Some(result) = vendor_conditions_block_result(vendor_item.has_vendor_conditions)
        {
            self.send_buy_error(result, Some(buy.vendor_guid), buy.item_id as u32);
            return;
        }
        if let Some(result) = vendor_buy_player_condition_block_result_like_cpp(
            vendor_item.player_condition_id,
            player_condition_store.as_deref(),
            Some(player_condition_context.as_context(self)),
        ) {
            self.send_equip_error(result, None, None, 0, 0);
            return;
        }
        let vendor_current_count = self.vendor_item_current_count(
            buy.vendor_guid,
            vendor_item.item_id,
            vendor_item.max_count,
            vendor_item.incr_time,
            vendor_item.buy_count,
        );
        if vendor_item.max_count != 0 && vendor_current_count < quantity {
            self.send_buy_error(
                BuyResult::ItemAlreadySold,
                Some(buy.vendor_guid),
                buy.muid as u32,
            );
            return;
        }
        if let Some(result) = vendor_buy_required_reputation_block_result(
            required_reputation_faction,
            required_reputation_rank,
            -1,
        ) {
            self.send_buy_error(result, Some(buy.vendor_guid), buy.item_id as u32);
            return;
        }
        if let Some(result) = vendor_buy_extended_cost_block_result(
            self.item_extended_cost_store().map(|store| store.as_ref()),
            self.currency_types_store().map(|store| store.as_ref()),
            |item_id, amount| self.has_item_count_direct_inventory(item_id, amount),
            |currency_id, amount| self.has_currency(currency_id, amount),
            true,
            vendor_item.extended_cost,
            vendor_item.buy_count,
            quantity,
        ) {
            match result {
                VendorExtendedCostBlock::Equip(result) => {
                    self.send_equip_error(result, None, None, 0, 0);
                }
                VendorExtendedCostBlock::Buy(result) => {
                    self.send_buy_error(result, Some(buy.vendor_guid), buy.item_id as u32);
                }
                VendorExtendedCostBlock::Silent => {}
            }
            return;
        }
        let extended_cost_item_costs = vendor_buy_extended_cost_item_costs(
            self.item_extended_cost_store().map(|store| store.as_ref()),
            vendor_item.extended_cost,
            vendor_item.buy_count,
            quantity,
        );
        let extended_cost_currency_costs = vendor_buy_extended_cost_currency_costs(
            self.item_extended_cost_store().map(|store| store.as_ref()),
            vendor_item.extended_cost,
            vendor_item.buy_count,
            quantity,
        );
        if let Some(result) = vendor_buy_direct_store_block_result(store_bag, store_slot, quantity)
        {
            self.send_equip_error(result, None, None, 0, 0);
            return;
        }

        let (quantity, buy_price): (u32, u64) =
            vendor_buy_quantity_and_price(vendor_item.buy_price, vendor_item.buy_count, quantity);
        let max_durability = vendor_item.max_durability;
        let refund_template = self.item_storage_template(buy.item_id as u32);
        let creates_refund_metadata = vendor_list_item_refundable(
            refund_template.as_ref().map(|template| template.flags),
            refund_template
                .as_ref()
                .map(|template| template.max_stack_size),
            vendor_item.extended_cost as i32,
        );

        // ── Check gold ──
        if self.player_gold_like_cpp() < buy_price {
            self.send_buy_error(
                BuyResult::NotEnoughtMoney,
                Some(buy.vendor_guid),
                buy.muid as u32,
            );
            return;
        }

        let (store_result, store_dest, _) = match self.plan_store_new_direct_inventory_item_at(
            buy.item_id as u32,
            quantity,
            store_bag,
            store_slot,
        ) {
            Some(plan) => plan,
            None => {
                self.send_buy_error(
                    BuyResult::CantFindItem,
                    Some(buy.vendor_guid),
                    buy.muid as u32,
                );
                return;
            }
        };
        if store_result != InventoryResult::Ok {
            self.send_equip_error(store_result, None, None, 0, 0);
            return;
        }
        let quest_log_item_id = self
            .quest_source_item_quest_log_item_id_like_cpp(buy.item_id as u32)
            .await;

        let new_item_count = store_dest
            .iter()
            .filter(|dest| {
                let slot = (dest.pos & 0x00FF) as u8;
                !self.inventory_items_like_cpp().contains_key(&slot)
            })
            .count();
        let Some(allocated_new_item_guids) =
            self.allocate_item_instance_guids_like_cpp(new_item_count)
        else {
            warn!(
                count = new_item_count,
                "BuyItem: process-wide item GUID allocator is unavailable"
            );
            self.send_buy_error(
                BuyResult::CantFindItem,
                Some(buy.vendor_guid),
                buy.muid as u32,
            );
            return;
        };
        let mut allocated_new_item_guids = allocated_new_item_guids.into_iter();

        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };
        let mut tx = SqlTransaction::new();
        let old_gold = self.player_gold_like_cpp();
        let new_gold = old_gold.saturating_sub(buy_price);
        let mut upd_money = char_db.prepare(CharStatements::UPD_CHAR_MONEY);
        upd_money.set_u64(0, new_gold);
        upd_money.set_u64(1, player_guid.counter() as u64);
        tx.append(upd_money);

        let mut existing_updates = Vec::new();
        let mut new_stacks = Vec::new();
        for dest in &store_dest {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;
            if bag != u8::from(INVENTORY_SLOT_BAG_0) {
                warn!(
                    "BuyItem: direct inventory plan produced unsupported bag {}",
                    bag
                );
                self.send_equip_error(InventoryResult::WrongBagType, None, None, 0, 0);
                return;
            }

            if let Some(inv_item) = self.inventory_items_like_cpp().get(&slot) {
                let Some(existing_item) =
                    self.inventory_item_objects_like_cpp().get(&inv_item.guid)
                else {
                    warn!("BuyItem: missing runtime item object for slot {}", slot);
                    self.send_buy_error(
                        BuyResult::CantFindItem,
                        Some(buy.vendor_guid),
                        buy.muid as u32,
                    );
                    return;
                };
                let new_count = existing_item.count().saturating_add(dest.count);
                let mut upd_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
                upd_count.set_u32(0, new_count);
                upd_count.set_u64(1, inv_item.db_guid);
                tx.append(upd_count);
                existing_updates.push((slot, inv_item.guid, new_count));
            } else {
                let Some((db_guid, item_guid)) = allocated_new_item_guids.next() else {
                    warn!("BuyItem: preallocated item GUID count did not match store plan");
                    self.send_buy_error(
                        BuyResult::CantFindItem,
                        Some(buy.vendor_guid),
                        buy.muid as u32,
                    );
                    return;
                };

                let item_flags =
                    vendor_stored_new_item_flags_like_cpp(refund_template.as_ref(), bag, slot);
                let mut ins_item =
                    char_db.prepare(CharStatements::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT);
                ins_item.set_u64(0, db_guid);
                ins_item.set_u32(1, buy.item_id as u32);
                ins_item.set_u64(2, player_guid.counter() as u64);
                ins_item.set_u32(3, dest.count);
                ins_item.set_u32(4, max_durability);
                ins_item.set_u32(5, item_flags);
                ins_item.set_i32(6, 0);
                ins_item.set_i32(7, 0);
                ins_item.set_u8(8, ItemContext::Vendor as u8);
                tx.append(ins_item);

                let mut ins_inv = char_db.prepare(CharStatements::INS_CHAR_INVENTORY);
                ins_inv.set_u64(0, player_guid.counter() as u64);
                ins_inv.set_u8(1, slot);
                ins_inv.set_u64(2, db_guid);
                tx.append(ins_inv);

                new_stacks.push((slot, db_guid, item_guid, dest.count, item_flags));
            }
        }
        let refund_item_db_guid = creates_refund_metadata
            .then(|| {
                new_stacks.last_mut().map(|stack| {
                    stack.4 |= ItemFieldFlags::REFUNDABLE.bits();
                    (stack.1, stack.4)
                })
            })
            .flatten();
        if let Some((refund_item_db_guid, refund_item_flags)) = refund_item_db_guid {
            let mut upd_flags = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_FLAGS);
            upd_flags.set_u32(0, refund_item_flags);
            upd_flags.set_u64(1, refund_item_db_guid);
            tx.append(upd_flags);
            append_item_refund_insert_statements(
                char_db.as_ref(),
                &mut tx,
                refund_item_db_guid,
                player_guid.counter() as u64,
                buy_price,
                vendor_item.extended_cost as u16,
            );
        }

        let mut item_turnin_changes = Vec::new();
        for &(item_id, amount) in &extended_cost_item_costs {
            let Some(mut changes) = self.plan_destroy_item_count_direct_inventory(item_id, amount)
            else {
                self.send_equip_error(InventoryResult::VendorMissingTurnins, None, None, 0, 0);
                return;
            };
            item_turnin_changes.append(&mut changes);
        }
        Self::append_item_turnin_statements(
            char_db.as_ref(),
            &mut tx,
            player_guid,
            &item_turnin_changes,
        );

        let mut planned_currencies = self.player_currencies_like_cpp().clone();
        for &(currency_id, amount) in &extended_cost_currency_costs {
            if i32::try_from(amount).is_err()
                || !Self::plan_remove_currency_like_cpp(
                    &mut planned_currencies,
                    currency_id,
                    amount,
                )
            {
                self.send_equip_error(InventoryResult::VendorMissingTurnins, None, None, 0, 0);
                return;
            }
        }
        let currency_save = self.plan_player_currency_save_like_cpp(
            player_guid.counter() as u64,
            &mut planned_currencies,
        );
        wow_database::player_lifecycle_adapter::append_player_currency_save_request_like_cpp(
            &mut tx,
            &currency_save,
        );

        let Some(money_persistence) = self
            .commit_exclusive_player_money_transaction_like_cpp(
                money_persistence,
                char_db.as_ref(),
                tx,
                old_gold,
                new_gold,
                "vendor item purchase",
            )
            .await
        else {
            warn!("BuyItem: store transaction did not commit");
            self.send_buy_error(
                BuyResult::CantFindItem,
                Some(buy.vendor_guid),
                buy.muid as u32,
            );
            return;
        };

        // The SQL transaction also owns the turn-ins, returned inventory
        // stacks, currencies and refund metadata. Publish all of that runtime
        // state synchronously before reopening payout admission; cancelling
        // the handler after COMMIT must not leave runtime at the pre-buy state.
        self.stage_player_money_change_like_cpp(old_gold, new_gold);
        self.apply_item_turnin_changes(player_guid, map_id, &item_turnin_changes);
        self.set_player_currencies_like_cpp(planned_currencies);
        for &(_, item_guid, new_count) in &existing_updates {
            self.update_inventory_item_object_like_cpp(item_guid, |item| {
                item.set_count(new_count);
            });
        }

        let inv_type = self.item_template_inventory_type(buy.item_id as u32);
        let mut collection_updates = Vec::new();
        for &(slot, db_guid, item_guid, stack_count, item_flags) in &new_stacks {
            self.insert_inventory_item_like_cpp(
                slot,
                crate::session::InventoryItem {
                    guid: item_guid,
                    entry_id: buy.item_id as u32,
                    db_guid,
                    inventory_type: inv_type,
                },
            );
            let mut item_object = self.make_inventory_item_object(
                item_guid,
                buy.item_id as u32,
                player_guid,
                stack_count,
                max_durability,
                ItemContext::Vendor,
                slot,
            );
            item_object.replace_all_item_flags(ItemFieldFlags::from_bits_retain(item_flags));
            if refund_item_db_guid.is_some_and(|(refund_db_guid, _)| refund_db_guid == db_guid) {
                item_object.set_refund_recipient(player_guid);
                item_object.set_paid_money(buy_price);
                item_object.set_paid_extended_cost(vendor_item.extended_cost as u32);
            }
            collection_updates.extend(self.on_item_added_to_collection_like_cpp(&item_object));
            self.insert_inventory_item_object(item_object);
        }
        self.sync_object_accessor_player();

        let changed_slots: Vec<_> = new_stacks
            .iter()
            .map(|&(slot, _, item_guid, _, _)| (slot, item_guid))
            .collect();
        let quantity_in_inventory =
            self.represented_non_bank_item_count_like_cpp(buy.item_id as u32);
        let purchased_item_plan = store_dest.last().and_then(|dest| {
            let slot = (dest.pos & 0x00FF) as u8;
            let item_guid = self.inventory_items_like_cpp().get(&slot)?.guid;
            let item = self.inventory_item_objects_like_cpp().get(&item_guid)?;
            let battle_pet_breed_data = item.get_modifier(ItemModifier::BattlePetBreedData);
            let modifications = item
                .data()
                .modifiers
                .iter()
                .enumerate()
                .filter_map(|(modifier_type, &value)| {
                    (value != 0).then_some(SendNewItemModifier {
                        value: value as i32,
                        modifier_type: modifier_type as u8,
                    })
                })
                .collect();
            Some(SendNewItemPlan {
                player_guid,
                item_guid,
                item_entry: item.object().entry(),
                item_instance: SendNewItemInstancePlan {
                    item_id: item.object().entry(),
                    random_properties_seed: item.data().property_seed,
                    random_properties_id: item.data().random_properties_id,
                    modifications,
                },
                slot: item.bag_slot(),
                slot_in_bag: if item.count() == quantity {
                    i16::from(item.slot())
                } else {
                    -1
                },
                quest_log_item_id,
                quantity,
                quantity_in_inventory,
                battle_pet_species_id: item.get_modifier(ItemModifier::BattlePetSpeciesId),
                battle_pet_breed_id: battle_pet_breed_data & 0x00FF_FFFF,
                battle_pet_breed_quality: ((battle_pet_breed_data >> 24) & 0xFF) as u8,
                battle_pet_level: item.get_modifier(ItemModifier::BattlePetLevel),
                pushed: true,
                created: false,
                display_text: SendNewItemDisplayText::Normal,
                dungeon_encounter_id: 0,
                is_encounter_loot: false,
                delivery: SendNewItemDelivery::Direct,
            })
        });
        let Some(purchased_item_plan) = purchased_item_plan else {
            // The durable purchase is already committed. Fail closed at the
            // packet boundary rather than fabricating an ItemPush GUID or
            // rolling runtime back out of sync with the database.
            warn!(
                item = buy.item_id,
                "BuyItem: committed item is missing from the published runtime inventory"
            );
            return;
        };
        let new_quantity = if vendor_item.max_count == 0 {
            -1
        } else {
            self.update_vendor_item_current_count(
                buy.vendor_guid,
                vendor_item.item_id,
                vendor_item.max_count,
                vendor_item.incr_time,
                vendor_item.buy_count,
                quantity,
            ) as i32
        };
        drop(money_persistence);

        self.drain_represented_quest_objective_progress_like_cpp()
            .await;
        for &(currency_id, amount) in &extended_cost_currency_costs {
            let Some(quantity) = i32::try_from(self.player_currency_quantity(currency_id)).ok()
            else {
                continue;
            };
            let Some(amount) = i32::try_from(amount).ok() else {
                continue;
            };
            self.send_packet(&SetCurrency::vendor_loss(
                currency_id as i32,
                quantity,
                amount,
            ));
        }

        info!(
            "BuyItem: player {:?} bought item {} across {} destination(s) for {} copper (remaining: {})",
            player_guid,
            buy.item_id,
            store_dest.len(),
            buy_price,
            self.player_gold_like_cpp()
        );

        if !new_stacks.is_empty() {
            let item_creates = new_stacks
                .iter()
                .map(
                    |&(_, _, item_guid, stack_count, item_flags)| ItemCreateData {
                        item_guid,
                        entry_id: buy.item_id,
                        owner_guid: player_guid,
                        contained_in: player_guid,
                        stack_count,
                        dynamic_flags: item_flags,
                        durability: max_durability,
                        max_durability,
                        random_properties_seed: 0,
                        random_properties_id: 0,
                        enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
                        gems: Vec::new(),
                        context: ItemContext::Vendor as u8,
                        container_slots: 0,
                        container_item_guids: [ObjectGuid::EMPTY; 36],
                    },
                )
                .collect();
            self.send_packet(&UpdateObject::create_stored_items(item_creates, map_id));
        }

        for &(_, item_guid, new_count) in &existing_updates {
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid, map_id, new_count,
            ));
        }

        // C++ `StoreNewItem` publishes item object changes on the instance
        // socket before `_StoreOrEquipNewItem` emits its two realm-routed
        // result packets. Preserve that physical cross-socket order.
        if !self
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
        {
            self.sync_player_registry_state_like_cpp();
            self.kick("vendor socket ordering fence failed after durable item purchase");
            return;
        }
        self.send_packet_realm(&BuySucceeded {
            vendor_guid: buy.vendor_guid,
            muid: buy.muid,
            new_quantity,
            quantity_bought: quantity as i32,
        });
        self.send_new_item_plan(&purchased_item_plan);
        if !self
            .wait_for_realm_send_before_instance_update_like_cpp()
            .await
        {
            self.sync_player_registry_state_like_cpp();
            self.kick("vendor socket ordering fence failed after durable item purchase");
            return;
        }

        self.send_player_values_update_from_entity_bridge(
            &changed_slots,
            &[],
            &[],
            &[],
            vendor_buy_coinage_update_like_cpp(buy_price, self.player_gold_like_cpp()),
        );
        for update in &collection_updates {
            self.send_player_values_update_like_cpp(update);
        }
    }

    /// Handle CMSG_BUY_BACK_ITEM — player buys back an item from a vendor.
    ///
    /// C++ ref: `WorldSession::HandleBuybackItem`.
    pub async fn handle_buy_back_item(&mut self, buyback: BuyBackItem) {
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
        if !WorldSession::is_buyback_slot(buyback_slot) {
            self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
            return;
        }

        let buyback_item = match self.buyback_items_like_cpp().get(&buyback_slot).cloned() {
            Some(item) => item,
            None => {
                self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
                return;
            }
        };
        let Some(runtime_item) = self
            .inventory_item_objects_like_cpp()
            .get(&buyback_item.guid)
            .cloned()
        else {
            self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
            return;
        };

        let buyback_index = (buyback_slot - BUYBACK_SLOT_START) as usize;
        let price = u64::from(self.buyback_price_like_cpp()[buyback_index]);
        if self.player_gold_like_cpp() < price {
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

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };
        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };
        let mut tx = SqlTransaction::new();
        let old_gold = self.player_gold_like_cpp();
        let new_gold = old_gold.saturating_sub(price);
        let mut upd_money = char_db.prepare(CharStatements::UPD_CHAR_MONEY);
        upd_money.set_u64(0, new_gold);
        upd_money.set_u64(1, player_guid.counter() as u64);
        tx.append(upd_money);

        let mut existing_updates = Vec::new();
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

            if let Some(inv_item) = self.inventory_items_like_cpp().get(&slot) {
                let Some(existing_item) =
                    self.inventory_item_objects_like_cpp().get(&inv_item.guid)
                else {
                    self.send_buy_error(BuyResult::CantFindItem, Some(buyback.vendor_guid), 0);
                    return;
                };
                let new_count = existing_item.count().saturating_add(dest.count);
                let mut upd_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
                upd_count.set_u32(0, new_count);
                upd_count.set_u64(1, inv_item.db_guid);
                tx.append(upd_count);
                existing_updates.push((slot, inv_item.guid, new_count));
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
                let mut upd_slot = char_db.prepare(CharStatements::UPD_CHAR_INVENTORY_SLOT);
                upd_slot.set_u8(0, slot);
                upd_slot.set_u64(1, player_guid.counter() as u64);
                upd_slot.set_u64(2, buyback_item.db_guid);
                tx.append(upd_slot);
                if runtime_item.count() != dest.count {
                    let mut upd_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
                    upd_count.set_u32(0, dest.count);
                    upd_count.set_u64(1, buyback_item.db_guid);
                    tx.append(upd_count);
                }
                moved_slot = Some(slot);
                moved_count = dest.count;
            }
        }

        if moved_slot.is_none() {
            let mut del_inv = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
            del_inv.set_u64(0, player_guid.counter() as u64);
            del_inv.set_u64(1, buyback_item.db_guid);
            tx.append(del_inv);

            let mut del_item = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
            del_item.set_u64(0, buyback_item.db_guid);
            tx.append(del_item);
        }

        let Some(money_persistence) = self
            .commit_exclusive_player_money_transaction_like_cpp(
                money_persistence,
                char_db.as_ref(),
                tx,
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
        self.stage_player_money_change_like_cpp(old_gold, new_gold);
        self.remove_buyback_item_like_cpp(buyback_slot);
        self.clear_buyback_slot_metadata_like_cpp(buyback_slot);
        if self
            .buyback_items_like_cpp()
            .contains_key(&self.current_buyback_slot_like_cpp())
        {
            self.set_current_buyback_slot_like_cpp(buyback_slot);
        }

        for &(_, item_guid, new_count) in &existing_updates {
            self.update_inventory_item_object_like_cpp(item_guid, |item| {
                item.set_count(new_count);
            });
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
            self.update_inventory_item_object_like_cpp(buyback_item.guid, |item_object| {
                item_object.set_count(moved_count);
            });
            inv_slot_changes.push((slot, buyback_item.guid));
        } else {
            self.remove_inventory_item_object(buyback_item.guid);
        }
        self.sync_object_accessor_player();
        drop(money_persistence);

        self.drain_represented_quest_objective_progress_like_cpp()
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
            Some(self.player_gold_like_cpp()),
        );
    }

    /// Handle CMSG_SELL_ITEM — player sells an item to a vendor.
    ///
    /// C++ ref: `HandleSellItemOpcode` (`Handlers/ItemHandler.cpp:365+`).
    pub async fn handle_sell_item(&mut self, sell: SellItem) {
        use wow_packet::packets::update::UpdateObject;

        debug!(
            "SellItem: item={:?} from account {}",
            sell.item_guid, self.account_id
        );

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };
        let map_id = self.player_map_id_like_cpp();

        // ── Find item in inventory by GUID ──
        let (slot, item) = match self
            .inventory_items_like_cpp()
            .iter()
            .find(|(_, item)| item.guid == sell.item_guid)
            .map(|(&s, item)| (s, item.clone()))
        {
            Some(pair) => pair,
            None => {
                warn!("SellItem: item {:?} not in inventory", sell.item_guid);
                self.send_sell_error(
                    SellResult::YouDontOwnThatItem,
                    Some(sell.vendor_guid),
                    sell.item_guid,
                );
                return;
            }
        };

        // Equipped items (slots 0-18) can't be sold without unequipping first
        if slot < 19 {
            self.send_sell_error(
                SellResult::CantSellItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        }

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let Some(runtime_item) = self
            .inventory_item_objects_like_cpp()
            .get(&item.guid)
            .cloned()
        else {
            self.send_sell_error(
                SellResult::CantFindItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        };
        let item_inventory_type = self
            .item_storage_template(item.entry_id)
            .map(|template| template.inventory_type);
        if item_is_not_empty_bag_like_cpp(
            item_inventory_type,
            self.direct_item_contains_items(item.guid),
        ) {
            self.send_sell_error(
                SellResult::CantSellItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        }
        if self.is_active_loot_guid(item.guid) || item_is_currently_looted_like_cpp(&runtime_item) {
            self.send_sell_error(
                SellResult::CantSellItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        }
        if runtime_item.is_refundable() {
            return;
        }
        let sell_amount = match sell_item_amount_action(runtime_item.count(), sell.amount) {
            SellItemAmountAction::Invalid => {
                self.send_sell_error(
                    SellResult::CantSellItem,
                    Some(sell.vendor_guid),
                    sell.item_guid,
                );
                return;
            }
            action => action,
        };
        let sold_count = match sell_amount {
            SellItemAmountAction::FullStack { amount }
            | SellItemAmountAction::PartialStack { amount, .. } => amount,
            SellItemAmountAction::Invalid => unreachable!(),
        };

        // ── Get sell price from item_sparse directly ──
        let sell_price: u64 = {
            let port = match self.vendor_catalog_persistence_port_like_cpp() {
                Some(port) => port,
                None => return,
            };
            match port.load_item_sell_price_like_cpp(item.entry_id).await {
                wow_persistence::VendorCatalogOutcomeLikeCpp::Loaded(price) => price,
                _ => 0,
            }
        };
        if sell_price == 0 {
            self.send_sell_error(
                SellResult::CantSellItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        }

        let money = sell_price.saturating_mul(u64::from(sold_count));
        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };
        let old_gold = self.player_gold_like_cpp();
        let Some(new_gold) = player_money_gain_like_cpp(old_gold, money) else {
            self.send_sell_error(
                SellResult::CantSellItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        };
        let buyback_slot = self.select_buyback_slot_cpp();
        let old_buyback = self.buyback_items_like_cpp().get(&buyback_slot).cloned();
        let buyback_price = sell_price
            .saturating_mul(u64::from(sold_count))
            .min(u64::from(u32::MAX)) as u32;
        let buyback_timestamp = self
            .login_time
            .map(|login_time| login_time.elapsed().as_secs())
            .unwrap_or(0)
            .saturating_add(30 * 3600)
            .min(u64::from(u32::MAX)) as i64;

        let mut tx = SqlTransaction::new();
        if let Some(old_buyback) = &old_buyback {
            let mut del_old_inv = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
            del_old_inv.set_u64(0, player_guid.counter() as u64);
            del_old_inv.set_u64(1, old_buyback.db_guid);
            tx.append(del_old_inv);

            let mut del_old_item = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
            del_old_item.set_u64(0, old_buyback.db_guid);
            tx.append(del_old_item);
        }

        let mut new_buyback_stack = None;
        match sell_amount {
            SellItemAmountAction::FullStack { .. } => {
                let mut upd_slot = char_db.prepare(CharStatements::UPD_CHAR_INVENTORY_SLOT);
                upd_slot.set_u8(0, buyback_slot);
                upd_slot.set_u64(1, player_guid.counter() as u64);
                upd_slot.set_u64(2, item.db_guid);
                tx.append(upd_slot);
            }
            SellItemAmountAction::PartialStack { remaining, amount } => {
                let mut upd_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
                upd_count.set_u32(0, remaining);
                upd_count.set_u64(1, item.db_guid);
                tx.append(upd_count);

                let Some((new_db_guid, new_item_guid)) = self
                    .allocate_item_instance_guids_like_cpp(1)
                    .and_then(|mut allocated| allocated.pop())
                else {
                    warn!("SellItem: process-wide item GUID allocator is unavailable");
                    self.send_sell_error(
                        SellResult::CantSellItem,
                        Some(sell.vendor_guid),
                        sell.item_guid,
                    );
                    return;
                };
                let cloned_item =
                    runtime_item.clone_item_for_store(new_item_guid, Some(player_guid), amount);
                let cloned_data = cloned_item.data();
                let charges = item_spell_charges_db_string(
                    &cloned_data.spell_charges,
                    self.item_effect_count_like_cpp(item.entry_id),
                );
                let Some((enchantments, _)) =
                    self.inventory_remove_enchantment_persistence_like_cpp(item.guid, false)
                else {
                    self.send_sell_error(
                        SellResult::CantSellItem,
                        Some(sell.vendor_guid),
                        sell.item_guid,
                    );
                    return;
                };

                let mut ins_item = char_db.prepare(CharStatements::INS_ITEM_INSTANCE_CLONE);
                ins_item.set_u64(0, new_db_guid);
                ins_item.set_u32(1, item.entry_id);
                ins_item.set_u64(2, player_guid.counter() as u64);
                ins_item.set_u64(3, cloned_data.creator.counter() as u64);
                ins_item.set_u64(4, cloned_data.gift_creator.counter() as u64);
                ins_item.set_u32(5, cloned_item.count());
                ins_item.set_u32(6, cloned_data.expiration);
                ins_item.set_string(7, charges);
                ins_item.set_string(8, enchantments);
                ins_item.set_u32(9, cloned_data.dynamic_flags);
                ins_item.set_u32(10, cloned_data.durability);
                ins_item.set_u32(11, cloned_data.create_played_time);
                ins_item.set_i32(12, cloned_data.random_properties_id);
                ins_item.set_i32(13, cloned_data.property_seed);
                ins_item.set_u8(14, u8::try_from(cloned_data.context).unwrap_or(0));
                tx.append(ins_item);

                let mut ins_inv = char_db.prepare(CharStatements::INS_CHAR_INVENTORY);
                ins_inv.set_u64(0, player_guid.counter() as u64);
                ins_inv.set_u8(1, buyback_slot);
                ins_inv.set_u64(2, new_db_guid);
                tx.append(ins_inv);

                new_buyback_stack = Some((new_db_guid, cloned_item, remaining));
            }
            SellItemAmountAction::Invalid => unreachable!(),
        }

        // ── Add gold + save to DB ──
        let mut upd_money = char_db.prepare(CharStatements::UPD_CHAR_MONEY);
        upd_money.set_u64(0, new_gold);
        upd_money.set_u64(1, player_guid.counter() as u64);
        tx.append(upd_money);

        let Some(money_persistence) = self
            .commit_exclusive_player_money_transaction_like_cpp(
                money_persistence,
                char_db.as_ref(),
                tx,
                old_gold,
                new_gold,
                "vendor item sale",
            )
            .await
        else {
            warn!("SellItem: transaction did not commit");
            self.send_sell_error(
                SellResult::CantSellItem,
                Some(sell.vendor_guid),
                sell.item_guid,
            );
            return;
        };

        // C++ mutates money, inventory and buyback state as one in-memory
        // operation. Our durable-first adaptation must publish the same whole
        // state before reopening admission or reaching a cancellation point.
        self.stage_player_money_change_like_cpp(old_gold, new_gold);
        if let Some(old_buyback) = old_buyback {
            self.remove_buyback_item_like_cpp(buyback_slot);
            self.remove_inventory_item_object(old_buyback.guid);
        }
        self.set_buyback_slot_metadata_like_cpp(buyback_slot, buyback_price, buyback_timestamp);
        self.advance_buyback_slot_cpp();

        let mut created_buyback_item = None;
        let mut stack_update = None;
        if let Some((new_db_guid, cloned_item, remaining)) = new_buyback_stack {
            let new_item_guid = cloned_item.object().guid();
            let stack_count = cloned_item.count();
            let durability = cloned_item.data().durability;
            let max_durability = cloned_item.data().max_durability;
            self.update_inventory_item_object_like_cpp(item.guid, |item_object| {
                item_object.set_count(remaining);
            });
            stack_update = Some((item.guid, remaining));
            self.insert_buyback_item_like_cpp(
                buyback_slot,
                InventoryItem {
                    guid: new_item_guid,
                    entry_id: item.entry_id,
                    db_guid: new_db_guid,
                    inventory_type: item.inventory_type,
                },
            );
            self.insert_inventory_item_object(cloned_item);
            self.set_inventory_item_object_slot(new_item_guid, buyback_slot);
            created_buyback_item = Some((new_item_guid, stack_count, durability, max_durability));
        } else {
            self.remove_inventory_item_like_cpp(slot);
            self.insert_buyback_item_like_cpp(
                buyback_slot,
                InventoryItem {
                    guid: item.guid,
                    entry_id: item.entry_id,
                    db_guid: item.db_guid,
                    inventory_type: item.inventory_type,
                },
            );
            self.set_inventory_item_object_slot(item.guid, buyback_slot);
        }
        self.sync_object_accessor_player();
        drop(money_persistence);

        self.drain_represented_quest_objective_progress_like_cpp()
            .await;

        info!(
            "SellItem: player {:?} sold {}x item {} from slot {} for {} copper (total: {})",
            player_guid,
            sold_count,
            item.entry_id,
            slot,
            money,
            self.player_gold_like_cpp()
        );

        if let Some((item_guid, stack_count, durability, max_durability)) = created_buyback_item {
            self.send_packet(&UpdateObject::create_items(
                vec![ItemCreateData {
                    item_guid,
                    entry_id: item.entry_id as i32,
                    owner_guid: player_guid,
                    contained_in: player_guid,
                    stack_count,
                    dynamic_flags: 0,
                    durability,
                    max_durability,
                    random_properties_seed: 0,
                    random_properties_id: 0,
                    enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
                    gems: Vec::new(),
                    context: 0,
                    container_slots: 0,
                    container_item_guids: [ObjectGuid::EMPTY; 36],
                }],
                map_id,
            ));
        }
        if let Some((item_guid, new_count)) = stack_update {
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid, map_id, new_count,
            ));
        }

        let mut inv_slot_changes = Vec::new();
        if matches!(sell_amount, SellItemAmountAction::FullStack { .. }) {
            inv_slot_changes.push((slot, ObjectGuid::EMPTY));
        }
        let buyback_guid = self
            .buyback_items_like_cpp()
            .get(&buyback_slot)
            .map(|item| item.guid)
            .unwrap_or(ObjectGuid::EMPTY);
        inv_slot_changes.push((buyback_slot, buyback_guid));
        self.send_player_values_update_from_entity_bridge(
            &inv_slot_changes,
            &[],
            &[],
            &[(buyback_slot, buyback_price, buyback_timestamp)],
            Some(self.player_gold_like_cpp()),
        );
    }

    /// Handle CMSG_ITEM_PURCHASE_REFUND.
    ///
    /// C++ ref: `ItemHandler.HandleItemRefund` -> `Player::RefundItem`.
    pub async fn handle_item_purchase_refund(&mut self, refund: ItemPurchaseRefund) {
        const REFUND_RESULT_OK: u8 = 0;
        const REFUND_RESULT_ERR_GENERIC: u8 = 10;

        #[derive(Debug, Clone)]
        struct PlannedNewStack {
            slot: u8,
            entry_id: u32,
            count: u32,
            max_durability: u32,
        }

        let player_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let map_id = self.player_map_id_like_cpp();

        let Some((refund_slot, refund_inv_item)) = self
            .inventory_items_like_cpp()
            .iter()
            .find(|(_, item)| item.guid == refund.item_guid)
            .map(|(&slot, item)| (slot, item.clone()))
        else {
            warn!(
                "ItemPurchaseRefund: item {:?} not in inventory",
                refund.item_guid
            );
            return;
        };

        let Some(refund_item) = self
            .inventory_item_objects_like_cpp()
            .get(&refund.item_guid)
            .cloned()
        else {
            warn!(
                "ItemPurchaseRefund: item {:?} missing runtime object",
                refund.item_guid
            );
            return;
        };

        if self.is_active_loot_guid(refund.item_guid)
            || item_is_currently_looted_like_cpp(&refund_item)
        {
            return;
        }
        if !refund_item.is_refundable() {
            return;
        }

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0);

        if refund_item.is_refund_expired_at(now_secs)
            || refund_item.refund_recipient() != player_guid
        {
            let new_flags = refund_item.item_flags_bits() & !ItemFieldFlags::REFUNDABLE.bits();
            let mut tx = SqlTransaction::new();
            append_item_refund_clear_statements(
                char_db.as_ref(),
                &mut tx,
                refund_inv_item.db_guid,
                new_flags,
            );
            if let Err(e) = char_db.commit_transaction(tx).await {
                warn!("ItemPurchaseRefund: refund cleanup transaction failed: {e}");
                return;
            }

            self.update_inventory_item_object_like_cpp(refund.item_guid, |item| {
                item.set_not_refundable();
            });
            self.sync_object_accessor_player();
            self.send_packet(&ItemExpirePurchaseRefund {
                item_guid: refund.item_guid,
            });

            if refund_item.is_refund_expired_at(now_secs) {
                self.send_packet(&ItemPurchaseRefundResult {
                    item_guid: refund.item_guid,
                    result: REFUND_RESULT_ERR_GENERIC,
                    contents: None,
                });
            }
            return;
        }

        let Some(extended_cost) = self
            .item_extended_cost_store()
            .and_then(|store| store.get(refund_item.paid_extended_cost()))
            .copied()
        else {
            return;
        };

        let contents = crate::handlers::misc::item_purchase_contents_from_extended_cost(
            &extended_cost,
            refund_item.paid_money(),
        );

        let mut item_costs = Vec::new();
        for i in 0..5 {
            let item_id = extended_cost.item_id[i] as u32;
            let count = extended_cost.item_count[i] as u32;
            if item_id != 0 && count != 0 {
                item_costs.push((item_id, count));
            }
        }

        let mut currency_costs = Vec::new();
        for i in 0..5 {
            let season_earned = match i {
                0 => extended_cost
                    .flags
                    .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_1),
                1 => extended_cost
                    .flags
                    .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_2),
                2 => extended_cost
                    .flags
                    .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_3),
                3 => extended_cost
                    .flags
                    .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_4),
                4 => extended_cost
                    .flags
                    .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_5),
                _ => false,
            };
            if season_earned {
                continue;
            }
            let currency_id = extended_cost.currency_id[i] as u32;
            let count = extended_cost.currency_count[i] as u32;
            if currency_id != 0 && count != 0 {
                currency_costs.push((currency_id, count));
            }
        }

        let mut planned_existing_counts =
            std::collections::HashMap::<u8, (ObjectGuid, u64, u32)>::new();
        let mut planned_new_stacks = Vec::<PlannedNewStack>::new();
        for &(entry_id, count) in &item_costs {
            let (store_result, store_dest, _) =
                match self.plan_store_new_direct_inventory_item(entry_id, count) {
                    Some(plan) => plan,
                    None => {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    }
                };
            if store_result != InventoryResult::Ok {
                self.send_packet(&ItemPurchaseRefundResult {
                    item_guid: refund.item_guid,
                    result: REFUND_RESULT_ERR_GENERIC,
                    contents: Some(contents),
                });
                return;
            }

            for dest in store_dest {
                let bag = (dest.pos >> 8) as u8;
                let slot = (dest.pos & 0x00FF) as u8;
                if bag != u8::from(INVENTORY_SLOT_BAG_0) {
                    self.send_packet(&ItemPurchaseRefundResult {
                        item_guid: refund.item_guid,
                        result: REFUND_RESULT_ERR_GENERIC,
                        contents: Some(contents),
                    });
                    return;
                }

                let max_stack = self
                    .item_storage_template(entry_id)
                    .map(|template| template.max_stack_size)
                    .unwrap_or(1)
                    .max(1);

                if let Some(existing) = self.inventory_items_like_cpp().get(&slot) {
                    let Some(existing_object) =
                        self.inventory_item_objects_like_cpp().get(&existing.guid)
                    else {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    };
                    let base_count = planned_existing_counts
                        .get(&slot)
                        .map(|(_, _, count)| *count)
                        .unwrap_or_else(|| existing_object.count());
                    let new_count = base_count.saturating_add(dest.count);
                    if existing.entry_id != entry_id || new_count > max_stack {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    }
                    planned_existing_counts
                        .insert(slot, (existing.guid, existing.db_guid, new_count));
                    continue;
                }

                if let Some(new_stack) = planned_new_stacks
                    .iter_mut()
                    .find(|stack| stack.slot == slot)
                {
                    if new_stack.entry_id == entry_id
                        && new_stack.count.saturating_add(dest.count) <= max_stack
                    {
                        new_stack.count = new_stack.count.saturating_add(dest.count);
                        continue;
                    }

                    let backpack_end =
                        INVENTORY_SLOT_ITEM_START.saturating_add(INVENTORY_DEFAULT_SIZE);
                    let Some(alt_slot) = (INVENTORY_SLOT_ITEM_START..backpack_end).find(|slot| {
                        !self.inventory_items_like_cpp().contains_key(slot)
                            && !planned_new_stacks.iter().any(|stack| stack.slot == *slot)
                    }) else {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    };
                    let Some((InventoryResult::Ok, alt_dest, _)) = self
                        .plan_store_new_direct_inventory_item_at(
                            entry_id,
                            dest.count,
                            u8::from(INVENTORY_SLOT_BAG_0),
                            alt_slot,
                        )
                    else {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    };
                    if alt_dest.len() != 1 || (alt_dest[0].pos & 0x00FF) as u8 != alt_slot {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    }
                    planned_new_stacks.push(PlannedNewStack {
                        slot: alt_slot,
                        entry_id,
                        count: dest.count,
                        max_durability: self.item_template_max_durability(entry_id),
                    });
                    continue;
                }

                planned_new_stacks.push(PlannedNewStack {
                    slot,
                    entry_id,
                    count: dest.count,
                    max_durability: self.item_template_max_durability(entry_id),
                });
            }
        }

        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };
        let mut tx = SqlTransaction::new();
        let mut del_refund = char_db.prepare(CharStatements::DEL_ITEM_REFUND_INSTANCE);
        del_refund.set_u64(0, refund_inv_item.db_guid);
        tx.append(del_refund);

        let mut del_inv = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
        del_inv.set_u64(0, player_guid.counter() as u64);
        del_inv.set_u64(1, refund_inv_item.db_guid);
        tx.append(del_inv);

        let mut del_item = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
        del_item.set_u64(0, refund_inv_item.db_guid);
        tx.append(del_item);

        let old_money = self.player_gold_like_cpp();
        let money_gain = player_money_gain_like_cpp(old_money, refund_item.paid_money());
        let money_overflow = money_gain.is_none();
        let new_gold = money_gain.unwrap_or(old_money);
        let mut upd_money = char_db.prepare(CharStatements::UPD_CHAR_MONEY);
        upd_money.set_u64(0, new_gold);
        upd_money.set_u64(1, player_guid.counter() as u64);
        tx.append(upd_money);

        for &(_, db_guid, new_count) in planned_existing_counts.values() {
            let mut upd_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
            upd_count.set_u32(0, new_count);
            upd_count.set_u64(1, db_guid);
            tx.append(upd_count);
        }

        let mut created_new_stacks = Vec::new();
        if !planned_new_stacks.is_empty() {
            let Some(allocated_guids) =
                self.allocate_item_instance_guids_like_cpp(planned_new_stacks.len())
            else {
                warn!(
                    count = planned_new_stacks.len(),
                    "ItemPurchaseRefund: process-wide item GUID allocator is unavailable"
                );
                self.send_packet(&ItemPurchaseRefundResult {
                    item_guid: refund.item_guid,
                    result: REFUND_RESULT_ERR_GENERIC,
                    contents: Some(contents),
                });
                return;
            };

            for (stack, (db_guid, item_guid)) in planned_new_stacks.iter().zip(allocated_guids) {
                let mut ins_item = char_db.prepare(CharStatements::INS_ITEM_INSTANCE);
                ins_item.set_u64(0, db_guid);
                ins_item.set_u32(1, stack.entry_id);
                ins_item.set_u64(2, player_guid.counter() as u64);
                ins_item.set_u32(3, stack.count);
                ins_item.set_u32(4, stack.max_durability);
                tx.append(ins_item);

                let mut ins_inv = char_db.prepare(CharStatements::INS_CHAR_INVENTORY);
                ins_inv.set_u64(0, player_guid.counter() as u64);
                ins_inv.set_u8(1, stack.slot);
                ins_inv.set_u64(2, db_guid);
                tx.append(ins_inv);

                created_new_stacks.push((stack.clone(), db_guid, item_guid));
            }
        }

        let currency_snapshot = self.player_currencies_like_cpp().clone();
        let mut currency_deltas = Vec::new();
        for &(currency_id, amount) in &currency_costs {
            match self.add_currency_item_refund(currency_id, amount) {
                Ok(Some(delta)) => currency_deltas.push(delta),
                Ok(None) => {}
                Err(()) => {
                    self.set_player_currencies_like_cpp(currency_snapshot);
                    self.send_packet(&ItemPurchaseRefundResult {
                        item_guid: refund.item_guid,
                        result: REFUND_RESULT_ERR_GENERIC,
                        contents: Some(contents),
                    });
                    return;
                }
            }
        }
        let mut persisted_currencies = self.player_currencies_like_cpp().clone();
        let currency_save = self.plan_player_currency_save_like_cpp(
            player_guid.counter() as u64,
            &mut persisted_currencies,
        );
        self.set_player_currencies_like_cpp(persisted_currencies);
        wow_database::player_lifecycle_adapter::append_player_currency_save_request_like_cpp(
            &mut tx,
            &currency_save,
        );

        let Some(money_persistence) = self
            .commit_exclusive_player_money_transaction_like_cpp(
                money_persistence,
                char_db.as_ref(),
                tx,
                old_money,
                new_gold,
                "vendor item purchase refund",
            )
            .await
        else {
            self.set_player_currencies_like_cpp(currency_snapshot);
            warn!("ItemPurchaseRefund: refund transaction did not commit");
            self.send_packet(&ItemPurchaseRefundResult {
                item_guid: refund.item_guid,
                result: REFUND_RESULT_ERR_GENERIC,
                contents: Some(contents),
            });
            return;
        };

        // Refund COMMIT covers money, currencies, destruction of the refunded
        // item, and every restored stack. Publish the corresponding runtime
        // inventory before the guard opens or an await permits cancellation.
        self.stage_player_money_change_like_cpp(old_money, new_gold);
        self.remove_inventory_item_like_cpp(refund_slot);
        self.remove_inventory_item_object(refund.item_guid);

        for &(item_guid, _, new_count) in planned_existing_counts.values() {
            self.update_inventory_item_object_like_cpp(item_guid, |item| {
                item.set_count(new_count);
            });
        }

        for (stack, db_guid, item_guid) in &created_new_stacks {
            self.insert_inventory_item_like_cpp(
                stack.slot,
                InventoryItem {
                    guid: *item_guid,
                    entry_id: stack.entry_id,
                    db_guid: *db_guid,
                    inventory_type: self.item_template_inventory_type(stack.entry_id),
                },
            );
            let item_object = self.make_inventory_item_object(
                *item_guid,
                stack.entry_id,
                player_guid,
                stack.count,
                stack.max_durability,
                ItemContext::None,
                stack.slot,
            );
            self.insert_inventory_item_object(item_object);
        }
        self.sync_object_accessor_player();
        drop(money_persistence);

        self.drain_represented_quest_objective_progress_like_cpp()
            .await;
        if money_overflow {
            self.send_equip_error(InventoryResult::TooMuchGold, None, None, 0, 0);
        }

        self.send_packet(&ItemPurchaseRefundResult {
            item_guid: refund.item_guid,
            result: REFUND_RESULT_OK,
            contents: Some(contents),
        });
        self.send_packet(&ItemExpirePurchaseRefund {
            item_guid: refund.item_guid,
        });

        for delta in currency_deltas {
            let Some(type_id) = i32::try_from(delta.currency_id).ok() else {
                continue;
            };
            let Some(quantity) = i32::try_from(delta.quantity).ok() else {
                continue;
            };
            let Some(amount) = i32::try_from(delta.amount).ok() else {
                continue;
            };
            self.send_packet(&SetCurrency::item_refund_gain(
                type_id,
                quantity,
                amount,
                delta
                    .weekly_quantity
                    .and_then(|value| i32::try_from(value).ok()),
                delta
                    .max_quantity
                    .and_then(|value| i32::try_from(value).ok()),
                delta
                    .total_earned
                    .and_then(|value| i32::try_from(value).ok()),
                delta.suppress_chat_log,
            ));
        }

        if !created_new_stacks.is_empty() {
            let item_creates = created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| ItemCreateData {
                    item_guid: *item_guid,
                    entry_id: stack.entry_id as i32,
                    owner_guid: player_guid,
                    contained_in: player_guid,
                    stack_count: stack.count,
                    dynamic_flags: 0,
                    durability: stack.max_durability,
                    max_durability: stack.max_durability,
                    random_properties_seed: 0,
                    random_properties_id: 0,
                    enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
                    gems: Vec::new(),
                    context: 0,
                    container_slots: 0,
                    container_item_guids: [ObjectGuid::EMPTY; 36],
                })
                .collect();
            self.send_packet(&UpdateObject::create_items(item_creates, map_id));
        }

        for &(item_guid, _, new_count) in planned_existing_counts.values() {
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid, map_id, new_count,
            ));
        }

        let mut changed_slots = Vec::new();
        changed_slots.push((refund_slot, ObjectGuid::EMPTY));
        changed_slots.extend(
            created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| (stack.slot, *item_guid)),
        );
        self.send_player_values_update_from_entity_bridge(
            &changed_slots,
            &[],
            &[],
            &[],
            Some(self.player_gold_like_cpp()),
        );

        if refund_slot < 19 {
            self.send_stat_update();
        }
    }

    async fn persist_repaired_character_homebind_like_cpp(
        &self,
        guid: ObjectGuid,
        homebind: CharacterLoginLocationLikeCpp,
    ) {
        let Some(bind_area_id) = homebind.bind_area_id else {
            return;
        };
        let Ok(map_id) = u16::try_from(homebind.map_id) else {
            return;
        };
        let Ok(bind_area_id) = u16::try_from(bind_area_id) else {
            return;
        };

        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            return;
        };
        match port
            .persist_homebind_like_cpp(
                wow_persistence::PlayerHomebindPersistenceRequestLikeCpp::InsertRepaired {
                    player_guid: guid.counter() as u64,
                    map_id,
                    area_id: bind_area_id,
                    x: homebind.position.x,
                    y: homebind.position.y,
                    z: homebind.position.z,
                    orientation: homebind.position.orientation,
                },
            )
            .await
        {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                player_guid = guid.counter(),
                "failed to persist repaired character homebind like C++ Player::_LoadHomeBind: {reason}"
            ),
        }
    }

    pub(super) async fn repair_character_homebind_like_cpp(
        &self,
        guid: ObjectGuid,
        race: u8,
        player_create_info: PlayerCreateInfoLikeCpp,
        create_mode: u8,
        first_login: bool,
    ) -> Option<CharacterLoginLocationLikeCpp> {
        let mut replacement = if first_login {
            first_login_creation_homebind_like_cpp(player_create_info, create_mode)
        } else {
            None
        };
        if replacement.is_none() {
            replacement = self.load_default_graveyard_homebind_like_cpp(race);
        }
        let mut replacement = replacement?;
        replacement.bind_area_id =
            Some(self.resolved_homebind_area_id_like_cpp(replacement.map_id, replacement.position));

        self.persist_repaired_character_homebind_like_cpp(guid, replacement)
            .await;
        Some(replacement)
    }
}
