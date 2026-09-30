//! required items operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    pub(super) fn represented_direct_inventory_count_like_cpp(&self, item_entry: u32) -> Option<u32> {
        Some(
            self.resolved_inventory_items_like_cpp()?
                .values()
                .filter(|item| item.entry_id == item_entry)
                .filter_map(|inventory_item| {
                    self.resolved_inventory_item_object_like_cpp(inventory_item.guid)
                        .filter(|item| !item.is_in_trade())
                        .map(|item| item.count())
                })
                .fold(0u32, u32::saturating_add),
        )
    }

    pub(super) fn plan_quest_destroy_item_count_direct_like_cpp(
        &self,
        item_entry: u32,
        count: u32,
    ) -> Option<Vec<ExtendedCostItemTurninChange>> {
        let effective_count = if count == u32::MAX {
            self.represented_direct_inventory_count_like_cpp(item_entry)?
        } else {
            count
        };

        if effective_count == 0 {
            return Some(Vec::new());
        }

        self.plan_destroy_item_count_direct_inventory(item_entry, effective_count)
    }

    pub(super) async fn remove_quest_required_items_and_currencies_like_cpp(
        &mut self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let map_id = self.player_map_id_like_cpp();
        let mut item_changes = Vec::new();
        let Some(currency_snapshot) = self.player_currencies_like_cpp() else {
            return false;
        };
        let mut currency_losses = Vec::new();

        for objective in &quest.objectives {
            match objective.obj_type {
                QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL => {
                    let Ok(item_entry) = u32::try_from(objective.object_id) else {
                        return false;
                    };
                    let count = if (quest.flags & QUEST_FLAGS_REMOVE_SURPLUS_ITEMS_LIKE_CPP) != 0 {
                        u32::MAX
                    } else {
                        u32::try_from(objective.amount).unwrap_or(u32::MAX)
                    };
                    let Some(mut changes) =
                        self.plan_quest_destroy_item_count_direct_like_cpp(item_entry, count)
                    else {
                        return false;
                    };
                    item_changes.append(&mut changes);
                }
                QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL => {
                    let (Ok(currency_id), Ok(amount)) = (
                        u32::try_from(objective.object_id),
                        u32::try_from(objective.amount),
                    ) else {
                        return false;
                    };
                    let Some(before) = self.player_currency_quantity(currency_id) else {
                        self.set_player_currencies_like_cpp(currency_snapshot);
                        return false;
                    };
                    if !self.remove_currency(currency_id, amount) {
                        self.set_player_currencies_like_cpp(currency_snapshot);
                        return false;
                    }
                    let Some(after) = self.player_currency_quantity(currency_id) else {
                        self.set_player_currencies_like_cpp(currency_snapshot);
                        return false;
                    };
                    let removed = before.saturating_sub(after);
                    if removed > 0 {
                        currency_losses.push((currency_id, after, removed));
                    }
                }
                _ => {}
            }
        }

        if (quest.flags_ex & QUEST_FLAGS_EX_NO_ITEM_REMOVAL_LIKE_CPP) == 0 {
            for (item_entry, count) in quest.item_drop.iter().zip(quest.item_drop_quantity.iter()) {
                if *item_entry == 0 {
                    continue;
                }
                let count = if *count == 0 { u32::MAX } else { *count };
                let Some(mut changes) =
                    self.plan_quest_destroy_item_count_direct_like_cpp(*item_entry, count)
                else {
                    self.set_player_currencies_like_cpp(currency_snapshot);
                    return false;
                };
                item_changes.append(&mut changes);
            }
        }

        {
            let items = item_changes
                .iter()
                .map(|change| match *change {
                    ExtendedCostItemTurninChange::Update {
                        db_guid, new_count, ..
                    } => wow_persistence::QuestTurnInItemPersistenceLikeCpp::Update {
                        item_guid: db_guid,
                        new_count,
                    },
                    ExtendedCostItemTurninChange::Delete { db_guid, .. } => {
                        wow_persistence::QuestTurnInItemPersistenceLikeCpp::Delete {
                            item_guid: db_guid,
                        }
                    }
                })
                .collect();
            // The removals join the operation's single character transaction.
            // Their currency half is empty here because `_SaveCurrency` writes
            // the complete state once when the operation closes.
            plan.push_inventory_mutation(
                wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::QuestTurnIn(
                    wow_persistence::QuestTurnInPersistenceLikeCpp {
                        owner_guid: player_guid.counter() as u64,
                        items,
                        currency_save: wow_persistence::PlayerCurrencySaveRequestLikeCpp {
                            player_guid: player_guid.counter() as u64,
                            rows: Vec::new(),
                        },
                    },
                ),
            );
        }

        self.apply_item_turnin_changes(player_guid, map_id, &item_changes);
        for (currency_id, quantity, removed) in currency_losses {
            let (Some(quantity), Some(removed)) =
                (i32::try_from(quantity).ok(), i32::try_from(removed).ok())
            else {
                continue;
            };
            self.send_packet(&SetCurrency {
                type_id: currency_id as i32,
                quantity,
                flags: 0,
                weekly_quantity: None,
                tracked_quantity: None,
                max_quantity: None,
                total_earned: None,
                suppress_chat_log: false,
                quantity_change: Some(-removed),
                quantity_gain_source: None,
                quantity_lost_source: Some(CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP),
                first_craft_operation_id: None,
                next_recharge_time: None,
                recharge_cycle_start_time: None,
                overflown_currency_id: None,
            });
        }

        true
    }
}
