//! Represented bank storage slots.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// C++ `Player::DurabilityRepairAll(takeCost=true, guildBank=true)` for represented items.
    pub(crate) async fn repair_all_inventory_item_durability_with_guild_bank_and_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        discount: f32,
        repair_cost_rate: f32,
    ) -> bool {
        let Some(guild_bank_state) = self.inventory.represented_guild_repair_bank_state_like_cpp()
        else {
            return false;
        };
        let available_guild_money = guild_bank_state.available_repair_money;
        if available_guild_money == 0 {
            return false;
        }

        let Some(mut repair_items) = ({
            let (s, h) = crate::session::split_inventory_ref(self);
            s.repairable_inventory_item_costs_like_cpp(h, discount, repair_cost_rate)
        }) else {
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
        self.inventory
            .record_represented_guild_repair_bank_withdraw_like_cpp(
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
    pub(crate) fn represented_non_bank_item_count_like_cpp(&self, entry_id: u32) -> Option<u32> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_non_bank_item_count_like_cpp(hub, entry_id)
    }
    pub(crate) fn plan_bank_existing_inventory_item_at_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>)> {
        let conditions = self.player_condition_projection_cx_like_cpp();
        wow_world_application::InventoryMovePlanningCxLikeCpp::new(&conditions)
            .plan_bank_existing_inventory_item_at_like_cpp(source_bag, source_slot, destination_bag, destination_slot, swap)
    }
    pub(crate) fn set_player_bank_bag_slot_count_like_cpp(&mut self, count: u8) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_player_bank_bag_slot_count_like_cpp(&mut hub, count)
    }
    pub(crate) fn represented_can_use_current_bank_like_cpp(&self) -> bool {
        wow_world_application::can_use_current_bank_with_access_like_cpp(
            &self.interaction,
            &self.bank_interaction_access_like_cpp(),
            &self.core.owned_inventory_access_like_cpp(),
        )
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_bank_item_move_like_cpp(
        &mut self,
        move_like_cpp: RepresentedBankItemMoveLikeCpp,
    ) {
        self.inventory
            .record_represented_bank_item_move_like_cpp(move_like_cpp)
    }
    pub(crate) fn represented_guild_bank_can_interact_like_cpp(
        &self,
        banker: ObjectGuid,
    ) -> Option<u64> {
        crate::session::cx_inventory_ref(self)
            .represented_guild_bank_gameobject_can_interact_like_cpp(banker)?;
        let guild_id = self.resolved_represented_guild_id_like_cpp()?;
        (guild_id != 0).then_some(guild_id)
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
        self.inventory
            .record_represented_guild_bank_list_request_like_cpp(
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
        self.inventory
            .record_represented_guild_bank_inventory_move_like_cpp(
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
        self.inventory
            .record_represented_guild_bank_money_move_like_cpp(
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
            && crate::session::cx_inventory_ref(self)
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
        self.inventory
            .record_represented_guild_bank_tab_action_like_cpp(
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
        self.inventory
            .record_represented_guild_bank_tab_action_like_cpp(
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
        self.inventory
            .record_represented_guild_bank_tab_action_like_cpp(
                RepresentedGuildBankTabActionLikeCpp {
                    banker: None,
                    guild_id,
                    tab,
                    action,
                },
            );
        true
    }
    pub(crate) fn represented_bank_bag_slot_flag_like_cpp(&self, slot: usize) -> Option<u32> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_bank_bag_slot_flag_like_cpp(hub, slot)
    }
    pub(crate) fn set_represented_bank_bag_slot_flag_like_cpp(
        &mut self,
        slot: usize,
        value: u32,
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_represented_bank_bag_slot_flag_like_cpp(&mut hub, slot, value)
    }
    pub(crate) fn resolved_player_bank_bag_slot_count_like_cpp(&self) -> Option<u8> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_player_bank_bag_slot_count_like_cpp(hub)
    }
}

impl crate::session::InventoryCxRef<'_> {
    pub(crate) fn represented_guild_bank_gameobject_can_interact_like_cpp(
        &self,
        banker: ObjectGuid,
    ) -> Option<()> {
        let state = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(banker)?;
        if state.go_type.map(u32::from) != Some(GAMEOBJECT_TYPE_GUILD_BANK) {
            return None;
        }

        self.world_entities
            .represented_gameobject_can_interact_with_like_cpp(self.hub, banker, 10.0)
            .map(|_| ())
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/bank/f3_shims.rs"]
mod f3_shims;
