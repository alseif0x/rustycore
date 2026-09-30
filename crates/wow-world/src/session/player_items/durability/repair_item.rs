//! repair item for the existing durability owner.

use super::*;

impl WorldSession {
    /// C++ `Player::DurabilityRepair(pos, takeCost, discountMod)` for one represented item.
    pub(crate) async fn repair_inventory_item_durability_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_guid: ObjectGuid,
        take_cost: bool,
        discount: f32,
        repair_cost_rate: f32,
    ) -> bool {
        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
            return false;
        };
        let top_level_inventory_item = inventory_items
            .iter()
            .find(|(_, item)| item.guid == item_guid)
            .map(|(slot, item)| (*slot, item.clone()));
        let Some(item_object) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return false;
        };

        let top_level_inventory_item = top_level_inventory_item.map(|(_, item)| item);
        let item_entry_id = top_level_inventory_item
            .as_ref()
            .map(|item| item.entry_id)
            .unwrap_or_else(|| item_object.object().entry());
        let item_db_guid = top_level_inventory_item
            .as_ref()
            .map(|item| item.db_guid)
            .unwrap_or_else(|| item_guid.counter() as u64);
        let current_durability = item_object.data().durability;
        let max_durability = item_object.data().max_durability;
        let cost = self.item_durability_repair_cost_like_cpp(
            item_entry_id,
            current_durability,
            max_durability,
            discount,
            repair_cost_rate,
        );

        if take_cost && cost != 0 {
            let Some(money_persistence) = self
                .begin_exclusive_player_money_persistence_like_cpp()
                .await
            else {
                return false;
            };
            let Some(old_money) = self.resolved_player_money_like_cpp() else {
                self.kick("canonical Player money owner is unavailable before durability repair");
                return false;
            };
            if old_money < cost {
                return false;
            }
            let new_money = old_money - cost;

            let money_persistence = if let Some(port) =
                self.player_lifecycle_port_like_cpp().map(Arc::clone)
            {
                let Some(player_guid) = self.player_guid() else {
                    return false;
                };
                let request = wow_persistence::PlayerMoneyTransactionRequestLikeCpp {
                    player_guid: player_guid.counter() as u64,
                    money_after: new_money,
                    durability_repairs: vec![wow_persistence::PlayerDurabilityRepairSaveLikeCpp {
                        item_db_guid,
                        durability: max_durability,
                    }],
                };
                let Some(money_persistence) = self
                    .await_exclusive_player_money_transaction_outcome_like_cpp(
                        money_persistence,
                        port.persist_money_transaction_like_cpp(request),
                        old_money,
                        new_money,
                        "single item durability repair",
                    )
                    .await
                else {
                    return false;
                };
                money_persistence
            } else {
                // Unit fixtures do not install a CharacterDatabase. A live
                // session always has one; without it no detached SQL payout
                // can originate from this session either.
                money_persistence
            };

            // The committed transaction repaired the item and charged money.
            // Publish both runtime fields before the guard opens; otherwise a
            // cancelled criteria drain can leave durable durability repaired
            // while the live item remains broken.
            if !self.stage_player_money_change_like_cpp(old_money, new_money) {
                self.kick("canonical Player money owner became unavailable after durability-repair COMMIT");
                return false;
            }
            let repaired = self.apply_inventory_item_durability_repair_runtime_like_cpp(item_guid);
            drop(money_persistence);
            self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                item_guid_generator,
            )
            .await;
            return repaired;
        } else if let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) {
            match port
                .persist_durability_repair_like_cpp(
                    wow_persistence::PlayerDurabilityRepairSaveLikeCpp {
                        item_db_guid,
                        durability: max_durability,
                    },
                )
                .await
            {
                wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
                wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
                | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                    warn!(
                        item_guid = item_guid.counter(),
                        error = %reason,
                        "failed to persist represented item durability repair"
                    );
                    return false;
                }
            }
        }

        self.apply_inventory_item_durability_repair_runtime_like_cpp(item_guid)
    }
    #[cfg(test)]
    pub(crate) async fn repair_inventory_item_durability_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        take_cost: bool,
        discount: f32,
        repair_cost_rate: f32,
    ) -> bool {
        let generators = self.id_generators_for_test_like_cpp();
        self.repair_inventory_item_durability_with_generator_like_cpp(
            generators.item.as_ref(),
            item_guid,
            take_cost,
            discount,
            repair_cost_rate,
        )
        .await
    }
    pub(super) fn apply_inventory_item_durability_repair_runtime_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> bool {
        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
            return false;
        };
        let top_level_slot = inventory_items
            .iter()
            .find_map(|(&slot, item)| (item.guid == item_guid).then_some(slot));
        let Some(item_object) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return false;
        };
        let max_durability = item_object.data().max_durability;
        let was_broken = item_object.is_broken();
        let equipped_slot = top_level_slot
            .filter(|slot| is_equipment_packed_pos(make_item_pos(INVENTORY_SLOT_BAG_0, *slot)));
        let updated = self.apply_inventory_item_object_updates_like_cpp(
            item_guid,
            &[wow_entities::ItemObjectUpdateLikeCpp::SetDurability(
                max_durability,
            )],
        );
        if updated {
            if let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) {
                let mut item_data_mask = UpdateMask::new(ITEM_DATA_BITS);
                item_data_mask.set(ITEM_DATA_DURABILITY_BIT);
                let durability_update = ItemValuesUpdate {
                    changed_object_type_mask: 1 << TYPEID_ITEM,
                    object_data: None,
                    item_data: Some(ItemDataUpdate {
                        mask: item_data_mask,
                        values: item.data().clone(),
                    }),
                };
                if let Some(update) = item_values_update_to_update_object(
                    item_guid,
                    self.player_map_id_like_cpp(),
                    &durability_update,
                ) {
                    self.send_packet(&update);
                }
            }
            if let Some(slot) = equipped_slot
                && was_broken
            {
                self.record_represented_item_mods_like_cpp(item_guid, slot, true);
                self.send_represented_item_bonus_player_stat_update_like_cpp();
            }
        }
        updated
    }
}
