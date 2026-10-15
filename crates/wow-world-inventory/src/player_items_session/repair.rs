// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented item durability repair (#1363 slice 12b1; moved from the World
//! `session/player_items/{durability,bank}.rs`): `Player::DurabilityRepair`
//! for one item and `Player::DurabilityRepairAll` paid with player money or
//! from the guild bank, with their money/durability persistence order (#169).

use std::sync::Arc;

use tracing::warn;
use wow_core::ObjectGuid;
use wow_entities::{
    INVENTORY_SLOT_BAG_0, ITEM_DATA_BITS, ITEM_DATA_DURABILITY_BIT, ItemDataUpdate,
    ItemValuesUpdate, MAX_MONEY_AMOUNT, TYPEID_ITEM, UpdateMask, is_equipment_packed_pos,
    make_item_pos,
};
use wow_world_core::entity_update_bridge::item_values_update_to_update_object;

use super::item_mods::{
    record_represented_item_mods_like_cpp, send_represented_item_bonus_player_stat_update_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use super::world_test_consumer_like_cpp;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedGuildRepairBankWithdrawLikeCpp;
use crate::VendorSessionHostLikeCpp;

/// C++ `Player::DurabilityRepair(pos, takeCost, discountMod)` for one represented item.
pub async fn repair_inventory_item_durability_with_generator_like_cpp<
    H: VendorSessionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    item_guid: ObjectGuid,
    take_cost: bool,
    discount: f32,
    repair_cost_rate: f32,
) -> bool {
    let Some(inventory_items) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_items_like_cpp(hub)
    }) else {
        return false;
    };
    let top_level_inventory_item = inventory_items
        .iter()
        .find(|(_, item)| item.guid == item_guid)
        .map(|(slot, item)| (*slot, item.clone()));
    let Some(item_object) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_object_like_cpp(hub, item_guid)
    }) else {
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
    let cost = host
        .hub_ref()
        .catalogs
        .item_durability_repair_cost_like_cpp(
            item_entry_id,
            current_durability,
            max_durability,
            discount,
            repair_cost_rate,
        );

    if take_cost && cost != 0 {
        let Some(money_persistence) = host
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return false;
        };
        let Some(old_money) = host.resolved_player_money_like_cpp() else {
            host.core_mut()
                .kick("canonical Player money owner is unavailable before durability repair");
            return false;
        };
        if old_money < cost {
            return false;
        }
        let new_money = old_money - cost;

        let money_persistence = if let Some(port) = host
            .lifecycle()
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
        {
            let Some(player_guid) = host.player_guid() else {
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
            let Some(money_persistence) = host
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
        if !host.stage_player_money_change_like_cpp(old_money, new_money) {
            host.core_mut().kick(
                "canonical Player money owner became unavailable after durability-repair COMMIT",
            );
            return false;
        }
        let repaired = apply_inventory_item_durability_repair_runtime_like_cpp(host, item_guid);
        drop(money_persistence);
        host.drain_represented_quest_objective_progress_with_generator_like_cpp(
            item_guid_generator,
        )
        .await;
        return repaired;
    } else if let Some(port) = host
        .lifecycle()
        .player_lifecycle_port_like_cpp()
        .map(Arc::clone)
    {
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

    apply_inventory_item_durability_repair_runtime_like_cpp(host, item_guid)
}

pub fn apply_inventory_item_durability_repair_runtime_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    item_guid: ObjectGuid,
) -> bool {
    let Some(inventory_items) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_items_like_cpp(hub)
    }) else {
        return false;
    };
    let top_level_slot = inventory_items
        .iter()
        .find_map(|(&slot, item)| (item.guid == item_guid).then_some(slot));
    let Some(item_object) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_object_like_cpp(hub, item_guid)
    }) else {
        return false;
    };
    let max_durability = item_object.data().max_durability;
    let was_broken = item_object.is_broken();
    let equipped_slot = top_level_slot
        .filter(|slot| is_equipment_packed_pos(make_item_pos(INVENTORY_SLOT_BAG_0, *slot)));
    let updated = {
        let (state, mut hub) = host.split_inventory_mut();
        state.apply_inventory_item_object_updates_like_cpp(
            &mut hub,
            item_guid,
            &[wow_entities::ItemObjectUpdateLikeCpp::SetDurability(
                max_durability,
            )],
        )
    };
    if updated {
        if let Some(item) = {
            let (state, hub) = host.split_inventory_ref();
            state.resolved_inventory_item_object_like_cpp(hub, item_guid)
        } {
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
                host.core().player_map_id_like_cpp(),
                &durability_update,
            ) {
                host.send_packet(&update);
            }
        }
        if let Some(slot) = equipped_slot
            && was_broken
        {
            record_represented_item_mods_like_cpp(host, item_guid, slot, true);
            send_represented_item_bonus_player_stat_update_like_cpp(host);
        }
    }
    updated
}

/// C++ `Player::DurabilityRepairAll(takeCost=true, guildBank=false)` for represented items.
pub async fn repair_all_inventory_item_durability_with_player_money_and_generator_like_cpp<
    H: VendorSessionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    discount: f32,
    repair_cost_rate: f32,
) -> bool {
    let Some(repair_items) = ({
        let (s, h) = host.split_inventory_ref();
        s.repairable_inventory_item_costs_like_cpp(h, discount, repair_cost_rate)
    }) else {
        return false;
    };
    if repair_items.is_empty() {
        return true;
    }
    let total_cost = repair_items
        .iter()
        .fold(0u64, |total, (_, cost)| total.saturating_add(*cost));
    let Some(item_objects) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_objects_like_cpp(hub)
    }) else {
        return false;
    };
    let Some(inventory_items) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_items_like_cpp(hub)
    }) else {
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

    let Some(money_persistence) = host
        .begin_exclusive_player_money_persistence_like_cpp()
        .await
    else {
        return false;
    };
    let Some(old_money) = host.resolved_player_money_like_cpp() else {
        return false;
    };
    if old_money < total_cost {
        return false;
    }
    let new_money = old_money - total_cost;

    let money_persistence = if let Some(port) = host
        .lifecycle()
        .player_lifecycle_port_like_cpp()
        .map(Arc::clone)
    {
        let Some(player_guid) = host.player_guid() else {
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
        let Some(money_persistence) = host
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
    if !host.stage_player_money_change_like_cpp(old_money, new_money) {
        host.core_mut()
            .kick("canonical Player money owner became unavailable after durability-repair COMMIT");
        return false;
    }
    let mut repaired_any = false;
    for (item_guid, _, _) in planned_repairs {
        repaired_any |= apply_inventory_item_durability_repair_runtime_like_cpp(host, item_guid);
    }
    drop(money_persistence);
    host.drain_represented_quest_objective_progress_with_generator_like_cpp(item_guid_generator)
        .await;
    repaired_any
}

/// C++ `Player::DurabilityRepairAll(takeCost=true, guildBank=true)` for represented items.
pub async fn repair_all_inventory_item_durability_with_guild_bank_and_generator_like_cpp<
    H: VendorSessionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    discount: f32,
    repair_cost_rate: f32,
) -> bool {
    #[cfg(any(test, feature = "test-fixtures"))]
    let world_test = world_test_consumer_like_cpp(host);
    let Some(guild_bank_state) = host
        .split_inventory_ref()
        .0
        .represented_guild_repair_bank_state_like_cpp()
    else {
        return false;
    };
    let available_guild_money = guild_bank_state.available_repair_money;
    if available_guild_money == 0 {
        return false;
    }

    let Some(mut repair_items) = ({
        let (s, h) = host.split_inventory_ref();
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
        repaired_any |= repair_inventory_item_durability_with_generator_like_cpp(
            host,
            item_guid_generator,
            item_guid,
            false,
            0.0,
            repair_cost_rate,
        )
        .await;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    let withdraw_amount = total_cost.min(MAX_MONEY_AMOUNT);
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test {
        host.split_inventory_mut()
            .0
            .record_represented_guild_repair_bank_withdraw_like_cpp(
                RepresentedGuildRepairBankWithdrawLikeCpp {
                    amount: withdraw_amount,
                    repair: true,
                    success: guild_bank_state.withdraw_repair_money_allowed,
                },
            );
    }
    repaired_any || total_cost == 0
}
