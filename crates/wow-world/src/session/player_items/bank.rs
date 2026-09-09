//! Represented bank storage slots.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// Set the C++ BankBagSlotPrices.db2 store for this session.
    #[cfg(test)]
    pub fn set_bank_bag_slot_prices_store(&mut self, store: Arc<BankBagSlotPricesStore>) {
        self.bank_bag_slot_prices_store = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn bank_bag_slot_prices_store_for_test_like_cpp(
        &self,
    ) -> Option<&Arc<BankBagSlotPricesStore>> {
        self.bank_bag_slot_prices_store.as_ref()
    }
    /// C++ `Player::DurabilityRepairAll(takeCost=true, guildBank=true)` for represented items.
    pub(crate) async fn repair_all_inventory_item_durability_with_guild_bank_and_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        discount: f32,
        repair_cost_rate: f32,
    ) -> bool {
        let Some(guild_bank_state) = self.represented_guild_repair_bank_state_like_cpp else {
            return false;
        };
        let available_guild_money = guild_bank_state.available_repair_money;
        if available_guild_money == 0 {
            return false;
        }

        let Some(mut repair_items) =
            self.repairable_inventory_item_costs_like_cpp(discount, repair_cost_rate)
        else {
            return false;
        };
        repair_items.sort_by_key(|(_, cost)| *cost);

        let mut total_cost = 0u64;
        let mut repaired_any = false;
        for (item_guid, cost) in repair_items {
            let new_total_cost = total_cost.saturating_add(cost);
            if new_total_cost > available_guild_money || new_total_cost > MAX_MONEY_AMOUNT {
                break;
            }

            total_cost = new_total_cost;
            repaired_any |= self
                .repair_inventory_item_durability_with_generator_like_cpp(
                    item_guid_generator,
                    item_guid,
                    false,
                    0.0,
                    repair_cost_rate,
                )
                .await;
        }

        #[cfg(test)]
        let withdraw_amount = total_cost.min(MAX_MONEY_AMOUNT);
        #[cfg(test)]
        self.represented_guild_repair_bank_withdraws_like_cpp.push(
            RepresentedGuildRepairBankWithdrawLikeCpp {
                amount: withdraw_amount,
                repair: true,
                success: guild_bank_state.withdraw_repair_money_allowed,
            },
        );
        repaired_any || total_cost == 0
    }
    #[cfg(test)]
    pub(crate) async fn repair_all_inventory_item_durability_with_guild_bank_like_cpp(
        &mut self,
        discount: f32,
        repair_cost_rate: f32,
    ) -> bool {
        let generators = self.id_generators_for_test_like_cpp();
        self.repair_all_inventory_item_durability_with_guild_bank_and_generator_like_cpp(
            generators.item.as_ref(),
            discount,
            repair_cost_rate,
        )
        .await
    }
    /// C++ `GetItemCount(entry, false)`: count carried/equipped items while
    /// excluding personal-bank slots and bank-bag contents.
    pub(crate) fn represented_non_bank_item_count_like_cpp(&self, entry_id: u32) -> Option<u32> {
        let inventory_items = self.resolved_inventory_items_like_cpp()?;
        let item_objects = self.resolved_inventory_item_objects_like_cpp()?;
        let top_level_count = inventory_items
            .iter()
            .filter(|(slot, _)| !wow_entities::is_bank_pos(INVENTORY_SLOT_BAG_0, **slot))
            .filter_map(|(_, inventory_item)| item_objects.get(&inventory_item.guid))
            .filter(|item| item.object().entry() == entry_id && !item.is_in_trade())
            .fold(0u32, |count, item| count.saturating_add(item.count()));
        Some(
            item_objects
                .values()
                .filter(|item| {
                    item.is_in_bag()
                        && !wow_entities::is_bank_pos(item.bag_slot(), item.slot())
                        && item.object().entry() == entry_id
                        && !item.is_in_trade()
                })
                .fold(top_level_count, |count, item| {
                    count.saturating_add(item.count())
                }),
        )
    }
    pub(crate) fn send_player_bank_bag_slots_update_like_cpp(&self, count: u8) {
        let Some(guid) = self.player_guid() else {
            return;
        };
        let Some(mut player) = self.player_values_update_snapshot() else {
            return;
        };

        player.set_bank_bag_slot_count(count);
        player.mark_bank_bag_slot_count_changed_like_cpp();
        let update = player.values_update(true);
        if let Some(packet) =
            player_values_update_to_update_object(guid, self.player_map_id_like_cpp(), &update)
        {
            self.send_packet(&packet);
        }
    }
    pub(crate) fn send_player_bank_bag_slot_flag_update_like_cpp(&self, slot: usize, value: u32) {
        let Some(guid) = self.player_guid() else {
            return;
        };
        let Some(mut player) = self.player_values_update_snapshot() else {
            return;
        };

        if !player.set_bank_bag_slot_flag_value_like_cpp(slot, value) {
            return;
        }
        player.mark_bank_bag_slot_flag_changed_like_cpp(slot);
        let update = player.values_update(true);
        if let Some(packet) =
            player_values_update_to_update_object(guid, self.player_map_id_like_cpp(), &update)
        {
            self.send_packet(&packet);
        }
    }
    pub(crate) fn plan_bank_existing_inventory_item_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>)> {
        self.plan_bank_existing_inventory_item_at_like_cpp(
            source_bag,
            source_slot,
            NULL_BAG,
            NULL_SLOT,
            false,
        )
    }
    pub(crate) fn plan_bank_existing_inventory_item_at_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>)> {
        let inventory_item = self.get_inventory_item_by_pos(source_bag, source_slot)?;
        let source_item = self.resolved_inventory_item_object_like_cpp(inventory_item.guid)?;
        let player = self.direct_inventory_player_snapshot()?;
        let proto = self.item_storage_template(inventory_item.entry_id);
        let inventory_items = self.resolved_inventory_items_like_cpp()?;
        let item_objects = self.resolved_inventory_item_objects_like_cpp()?;

        let mut template_cache = HashMap::new();
        for item in item_objects.values() {
            let entry_id = item.object().entry();
            if let std::collections::hash_map::Entry::Vacant(entry) = template_cache.entry(entry_id)
                && let Some(template) = self.item_storage_template(entry_id)
            {
                entry.insert(template);
            }
        }

        let mut represented_bag_slots_by_guid = HashMap::new();
        let mut bag_templates = Vec::new();
        for (&slot, item) in &inventory_items {
            if Self::is_buyback_slot(slot) {
                continue;
            }
            if is_represented_bag_slot(slot) && item_objects.contains_key(&item.guid) {
                represented_bag_slots_by_guid.insert(item.guid, slot);
                if let Some(template) = template_cache.get(&item.entry_id)
                    && template.container_slots > 0
                {
                    bag_templates.push(BagTemplateRef::new(slot, template));
                }
            }
        }

        let mut slot_items = Vec::new();
        let mut stored_items = Vec::new();
        for (&slot, stored) in &inventory_items {
            if Self::is_buyback_slot(slot) {
                continue;
            }
            let Some(item) = item_objects.get(&stored.guid) else {
                continue;
            };
            slot_items.push(ItemSlotRef::new(INVENTORY_SLOT_BAG_0, slot, item));
            stored_items.push(ItemStorageRef::new(
                INVENTORY_SLOT_BAG_0,
                slot,
                item,
                template_cache.get(&stored.entry_id),
            ));
        }
        for item in item_objects.values() {
            let container_guid = item.container_guid();
            if container_guid.is_empty() {
                continue;
            }
            let Some(&bag_slot) = represented_bag_slots_by_guid.get(&container_guid) else {
                continue;
            };
            let entry_id = item.object().entry();
            slot_items.push(ItemSlotRef::new(bag_slot, item.slot(), item));
            stored_items.push(ItemStorageRef::new(
                bag_slot,
                item.slot(),
                item,
                template_cache.get(&entry_id),
            ));
        }

        let limit_category = proto.as_ref().and_then(|proto| {
            self.item_limit_category_template_like_cpp(proto.item_limit_category)
        });
        let can_use_result =
            self.can_use_inventory_item_represented_like_cpp(&inventory_item, Some(&source_item));
        let mut dest = Vec::new();
        let result = player.can_bank_item(
            &mut dest,
            CanBankItemArgs {
                bag: destination_bag,
                slot: destination_slot,
                proto: proto.as_ref(),
                source_item: Some(&source_item),
                source_is_not_empty_bag: self.direct_item_contains_items(inventory_item.guid),
                source_is_bag: proto
                    .as_ref()
                    .is_some_and(|proto| proto.container_slots > 0),
                source_is_currency_token: proto
                    .as_ref()
                    .is_some_and(|proto| proto.bag_family.contains(BagFamilyMask::CURRENCY_TOKENS)),
                source_bop_trade_allowed_for_player: false,
                swap,
                can_use_result,
                limit_category: limit_category.as_ref(),
                slot_items: &slot_items,
                stored_items: &stored_items,
                bag_templates: &bag_templates,
            },
        );

        Some((result, dest))
    }
    pub(crate) fn set_player_bank_bag_slot_count_like_cpp(&mut self, count: u8) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_bank_bag_slot_count(count))
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_bank_bag_slot_count_like_cpp = count;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }
    pub(crate) fn represented_can_use_current_bank_like_cpp(&self) -> bool {
        let Some(banker_guid) = self.player_interaction_source_guid_like_cpp() else {
            return false;
        };

        if Some(banker_guid) == self.player_guid() {
            return true;
        }

        self.represented_npc_can_interact_with_like_cpp(banker_guid, NPCFlags1::BANKER.bits(), 0)
            .is_some()
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_bank_item_move_like_cpp(
        &mut self,
        move_like_cpp: RepresentedBankItemMoveLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_bank_item_moves_like_cpp
            .push(move_like_cpp);
    }
    #[cfg(test)]
    pub(crate) fn represented_bank_item_moves_like_cpp(&self) -> &[RepresentedBankItemMoveLikeCpp] {
        &self.represented_bank_item_moves_like_cpp
    }
    pub(crate) fn represented_guild_bank_can_interact_like_cpp(
        &self,
        banker: ObjectGuid,
    ) -> Option<u64> {
        self.represented_guild_bank_gameobject_can_interact_like_cpp(banker)?;
        let guild_id = self.resolved_represented_guild_id_like_cpp()?;
        (guild_id != 0).then_some(guild_id)
    }
    pub(crate) fn represented_guild_bank_gameobject_can_interact_like_cpp(
        &self,
        banker: ObjectGuid,
    ) -> Option<()> {
        let state = self.represented_gameobject_use_states.get(&banker)?;
        if state.go_type.map(u32::from) != Some(GAMEOBJECT_TYPE_GUILD_BANK) {
            return None;
        }

        self.represented_gameobject_can_interact_with_like_cpp(banker, 10.0)
            .map(|_| ())
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_guild_bank_list_request_like_cpp(
        &mut self,
        banker: ObjectGuid,
        tab: u8,
        full_update: bool,
    ) -> bool {
        let Some(guild_id) = self.represented_guild_bank_can_interact_like_cpp(banker) else {
            return false;
        };
        #[cfg(test)]
        self.represented_guild_bank_list_requests_like_cpp.push(
            RepresentedGuildBankListRequestLikeCpp {
                banker,
                guild_id,
                tab,
                full_update,
            },
        );
        true
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn guild_bank_inventory_move_like_cpp(
        &mut self,
        banker: ObjectGuid,
        to_char: bool,
        bank_tab: u8,
        bank_slot: u8,
        player_bag: u8,
        player_slot: u8,
        stack_count: u32,
    ) -> bool {
        let Some(guild_id) = self.represented_guild_bank_can_interact_like_cpp(banker) else {
            return false;
        };

        let auto_store_to_char_slot =
            player_bag == INVENTORY_SLOT_BAG_0 && player_slot == NULL_SLOT;
        if !auto_store_to_char_slot && !is_inventory_pos(player_bag, player_slot) {
            self.send_equip_error(InventoryResult::InternalBagError, None, None, 0, 0);
            return false;
        }

        #[cfg(test)]
        self.represented_guild_bank_inventory_moves_like_cpp.push(
            RepresentedGuildBankInventoryMoveLikeCpp {
                banker,
                guild_id,
                to_char,
                bank_tab,
                bank_slot,
                player_bag,
                player_slot,
                stack_count,
            },
        );
        true
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn guild_bank_money_move_like_cpp(
        &mut self,
        banker: ObjectGuid,
        deposit: bool,
        money: u64,
    ) -> bool {
        if money == 0 {
            return false;
        }

        let Some(guild_id) = self.represented_guild_bank_can_interact_like_cpp(banker) else {
            return false;
        };

        if deposit
            && !self
                .resolved_player_money_like_cpp()
                .is_some_and(|player_money| player_money >= money)
        {
            return false;
        }

        #[cfg(test)]
        self.represented_guild_bank_money_moves_like_cpp.push(
            RepresentedGuildBankMoneyMoveLikeCpp {
                banker,
                guild_id,
                deposit,
                money,
            },
        );
        true
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn guild_bank_buy_tab_like_cpp(&mut self, banker: ObjectGuid, tab: u8) -> bool {
        if !banker.is_empty()
            && self
                .represented_guild_bank_gameobject_can_interact_like_cpp(banker)
                .is_none()
        {
            return false;
        }

        let Some(guild_id) = self.resolved_represented_guild_id_like_cpp() else {
            return false;
        };
        if guild_id == 0 {
            return false;
        }

        #[cfg(test)]
        self.represented_guild_bank_tab_actions_like_cpp.push(
            RepresentedGuildBankTabActionLikeCpp {
                banker: (!banker.is_empty()).then_some(banker),
                guild_id,
                tab: i32::from(tab),
                action: RepresentedGuildBankTabActionKindLikeCpp::Buy,
            },
        );
        true
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn guild_bank_update_tab_like_cpp(
        &mut self,
        banker: ObjectGuid,
        tab: u8,
        name: String,
        icon: String,
    ) -> bool {
        if name.is_empty() || icon.is_empty() {
            return false;
        }

        let Some(guild_id) = self.represented_guild_bank_can_interact_like_cpp(banker) else {
            return false;
        };

        #[cfg(test)]
        self.represented_guild_bank_tab_actions_like_cpp.push(
            RepresentedGuildBankTabActionLikeCpp {
                banker: Some(banker),
                guild_id,
                tab: i32::from(tab),
                action: RepresentedGuildBankTabActionKindLikeCpp::Update { name, icon },
            },
        );
        true
    }
    pub(crate) fn guild_bank_log_query_like_cpp(&mut self, tab: i32) -> bool {
        self.guild_bank_tab_action_without_banker_like_cpp(
            tab,
            RepresentedGuildBankTabActionKindLikeCpp::LogQuery,
        )
    }
    pub(crate) fn guild_bank_text_query_like_cpp(&mut self, tab: i32) -> bool {
        self.guild_bank_tab_action_without_banker_like_cpp(
            tab,
            RepresentedGuildBankTabActionKindLikeCpp::TextQuery,
        )
    }
    pub(crate) fn guild_bank_set_tab_text_like_cpp(&mut self, tab: i32, text: String) -> bool {
        self.guild_bank_tab_action_without_banker_like_cpp(
            tab,
            RepresentedGuildBankTabActionKindLikeCpp::SetText { text },
        )
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    fn guild_bank_tab_action_without_banker_like_cpp(
        &mut self,
        tab: i32,
        action: RepresentedGuildBankTabActionKindLikeCpp,
    ) -> bool {
        let Some(guild_id) = self.resolved_represented_guild_id_like_cpp() else {
            return false;
        };
        if guild_id == 0 {
            return false;
        }

        #[cfg(test)]
        self.represented_guild_bank_tab_actions_like_cpp.push(
            RepresentedGuildBankTabActionLikeCpp {
                banker: None,
                guild_id,
                tab,
                action,
            },
        );
        true
    }
    #[cfg(test)]
    pub(crate) fn represented_guild_bank_inventory_moves_like_cpp(
        &self,
    ) -> &[RepresentedGuildBankInventoryMoveLikeCpp] {
        &self.represented_guild_bank_inventory_moves_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_guild_bank_list_requests_like_cpp(
        &self,
    ) -> &[RepresentedGuildBankListRequestLikeCpp] {
        &self.represented_guild_bank_list_requests_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_guild_bank_money_moves_like_cpp(
        &self,
    ) -> &[RepresentedGuildBankMoneyMoveLikeCpp] {
        &self.represented_guild_bank_money_moves_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_guild_bank_tab_actions_like_cpp(
        &self,
    ) -> &[RepresentedGuildBankTabActionLikeCpp] {
        &self.represented_guild_bank_tab_actions_like_cpp
    }
    pub(crate) fn represented_bank_bag_slot_flag_like_cpp(&self, slot: usize) -> Option<u32> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.bank_bag_slot_flag_value_like_cpp(slot))
            .flatten();
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self
                .represented_bank_bag_slot_flags_like_cpp
                .get(slot)
                .copied();
        }
        canonical
    }
    pub(crate) fn set_represented_bank_bag_slot_flag_like_cpp(
        &mut self,
        slot: usize,
        value: u32,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_bank_bag_slot_flag_value_like_cpp(slot, value)
            })
            .unwrap_or(false);
        #[cfg(test)]
        if (canonical || self.player_handle_like_cpp.is_none())
            && let Some(flag) = self.represented_bank_bag_slot_flags_like_cpp.get_mut(slot)
        {
            *flag = value;
            return true;
        }
        canonical
    }
    pub(crate) fn resolved_player_bank_bag_slot_count_like_cpp(&self) -> Option<u8> {
        let canonical = self.with_owned_player_like_cpp(Player::bank_bag_slot_count);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_bank_bag_slot_count_like_cpp);
        }
        canonical
    }
    #[cfg(test)]
    pub(crate) fn player_bank_bag_slot_count_like_cpp(&self) -> u8 {
        self.resolved_player_bank_bag_slot_count_like_cpp()
            .expect("test Player bank-bag-slot owner must resolve")
    }
    #[cfg(test)]
    pub(crate) fn set_represented_guild_repair_bank_state_like_cpp(
        &mut self,
        state: Option<RepresentedGuildRepairBankStateLikeCpp>,
    ) {
        self.represented_guild_repair_bank_state_like_cpp = state;
    }
    #[cfg(test)]
    pub(crate) fn represented_guild_repair_bank_withdraws_like_cpp(
        &self,
    ) -> &[RepresentedGuildRepairBankWithdrawLikeCpp] {
        &self.represented_guild_repair_bank_withdraws_like_cpp
    }
}
