//! repair all for the existing durability owner.

use super::*;

impl WorldSession {
    /// C++ `Player::DurabilityRepairAll(takeCost=true, guildBank=false)` for represented items.
    pub(crate) async fn repair_all_inventory_item_durability_with_player_money_and_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        discount: f32,
        repair_cost_rate: f32,
    ) -> bool {
        let Some(repair_items) =
            self.repairable_inventory_item_costs_like_cpp(discount, repair_cost_rate)
        else {
            return false;
        };
        if repair_items.is_empty() {
            return true;
        }
        let total_cost = repair_items
            .iter()
            .fold(0u64, |total, (_, cost)| total.saturating_add(*cost));
        let Some(item_objects) = self.resolved_inventory_item_objects_like_cpp() else {
            return false;
        };
        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
            return false;
        };
        let planned_repairs = repair_items
            .iter()
            .filter_map(|(item_guid, _)| {
                let item = item_objects.get(item_guid)?;
                let db_guid = inventory_items
                    .values()
                    .find(|inventory_item| inventory_item.guid == *item_guid)
                    .map(|inventory_item| inventory_item.db_guid)
                    .unwrap_or_else(|| item_guid.counter() as u64);
                Some((*item_guid, db_guid, item.data().max_durability))
            })
            .collect::<Vec<_>>();
        if planned_repairs.len() != repair_items.len() {
            return false;
        }

        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return false;
        };
        let Some(old_money) = self.resolved_player_money_like_cpp() else {
            return false;
        };
        if old_money < total_cost {
            return false;
        }
        let new_money = old_money - total_cost;

        let money_persistence =
            if let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) {
                let Some(player_guid) = self.player_guid() else {
                    return false;
                };
                let request = wow_persistence::PlayerMoneyTransactionRequestLikeCpp {
                    player_guid: player_guid.counter() as u64,
                    money_after: new_money,
                    durability_repairs: planned_repairs
                        .iter()
                        .map(|&(_, item_db_guid, durability)| {
                            wow_persistence::PlayerDurabilityRepairSaveLikeCpp {
                                item_db_guid,
                                durability,
                            }
                        })
                        .collect(),
                };
                let Some(money_persistence) = self
                    .await_exclusive_player_money_transaction_outcome_like_cpp(
                        money_persistence,
                        port.persist_money_transaction_like_cpp(request),
                        old_money,
                        new_money,
                        "all-items durability repair",
                    )
                    .await
                else {
                    return false;
                };
                money_persistence
            } else {
                money_persistence
            };

        // Money and every durability row share one COMMIT. Mirror all of those
        // rows in runtime while admission is still fenced and before the first
        // cancellation point.
        if !self.stage_player_money_change_like_cpp(old_money, new_money) {
            self.kick(
                "canonical Player money owner became unavailable after durability-repair COMMIT",
            );
            return false;
        }
        let mut repaired_any = false;
        for (item_guid, _, _) in planned_repairs {
            repaired_any |= self.apply_inventory_item_durability_repair_runtime_like_cpp(item_guid);
        }
        drop(money_persistence);
        self.drain_represented_quest_objective_progress_with_generator_like_cpp(
            item_guid_generator,
        )
        .await;
        repaired_any
    }
    #[cfg(test)]
    pub(crate) async fn repair_all_inventory_item_durability_with_player_money_like_cpp(
        &mut self,
        discount: f32,
        repair_cost_rate: f32,
    ) -> bool {
        let generators = self.id_generators_for_test_like_cpp();
        self.repair_all_inventory_item_durability_with_player_money_and_generator_like_cpp(
            generators.item.as_ref(),
            discount,
            repair_cost_rate,
        )
        .await
    }
}
