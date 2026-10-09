// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The C++ StorageMove execution body, moved out of the World shell (#1263 F4).
//!
//! C++ `Player::SwapItem`'s store/bank transitions
//! (`Entities/Player/Player.cpp:12423`) and the inventory persistence at
//! `Player.cpp:19632`. The body keeps its original order, log strings and
//! packets; only the access path changed. The participants are lent by the host
//! ([`InventoryStorageMoveHostLikeCpp::storage_move_cx_like_cpp`] borrows
//! Inventory, persistence and publication, and
//! [`InventoryStorageMoveHostLikeCpp::storage_move_stats_cx_like_cpp`] borrows
//! Stats), while the shell-only capabilities stay behind the same trait.
//!
//! Preserved exactly: the silent absence of a plan, the planning rejection
//! publication, and the `Failed`/`Unknown` commit classification that leaves the
//! runtime unchanged and replies `InternalBagError`. `Unknown` does not prove
//! the database is unchanged: this body makes no claim about the SQL state, it
//! only refuses to mutate or publish after an unproven commit.

use tracing::warn;
use wow_packet::packets::update::UpdateObject;

use super::*;

#[allow(clippy::too_many_arguments)]
pub async fn execute_inventory_storage_move_like_cpp<H>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &wow_world_entities::CreatureSpawnCatalogsLikeCpp,
    source_bag: u8,
    source_slot: u8,
    destination_bag: u8,
    destination_slot: u8,
    target: InventoryStorageTargetLikeCpp,
    quest_checks: InventoryStorageQuestChecksLikeCpp,
    represented_move: Option<RepresentedBankItemMoveLikeCpp>,
) where
    H: InventoryStorageMoveHostLikeCpp + Send,
{
    let (player_guid, source_guid, source_limit_category) = {
        let cx = host.storage_move_cx_like_cpp();
        let Some(player_guid) = cx.player_guid() else {
            return;
        };
        let source_item = cx.get_inventory_item_by_pos(source_bag, source_slot);
        let source_guid = source_item.as_ref().map(|item| item.guid);
        let source_limit_category = source_item
            .as_ref()
            .and_then(|item| cx.item_storage_template(item.entry_id))
            .map(|template| template.item_limit_category)
            .unwrap_or(0);
        (player_guid, source_guid, source_limit_category)
    };
    let plan = match host.storage_move_plan_like_cpp(
        source_bag,
        source_slot,
        destination_bag,
        destination_slot,
        target,
    ) {
        Some(Ok(plan)) => plan,
        Some(Err(result)) => {
            host.storage_move_cx_like_cpp().send_equip_error(
                result,
                source_guid,
                None,
                0,
                source_limit_category,
            );
            return;
        }
        None => return,
    };
    let inventory_port = {
        let cx = host.storage_move_cx_like_cpp();
        let Some(inventory_port) = cx.inventory_persistence_port_like_cpp() else {
            return;
        };
        inventory_port
    };

    let source_stays_in_place = plan
        .moved_destination
        .is_some_and(|(bag, slot, _)| bag == plan.source_bag && slot == plan.source_slot);
    let (moving_to_bank, moving_from_bank) =
        inventory_storage_move_quest_directions_like_cpp(plan.source_bag, plan.source_slot, target);
    let runs_added_quest_check = quest_checks
        == InventoryStorageQuestChecksLikeCpp::AutoStoreBankItemAdded
        && moving_from_bank;
    let quest_log_item_id = if runs_added_quest_check {
        host.storage_move_quest_log_item_id_like_cpp(plan.source.entry_id)
            .await
    } else {
        0
    };
    let added_quest_count = if runs_added_quest_check {
        bank_store_item_added_quest_count_like_cpp(&plan)
    } else {
        0
    };
    let (apply_obtain_spells, current_non_bank_count) = {
        let cx = host.storage_move_cx_like_cpp();
        let apply_obtain_spells = plan
            .moved_destination
            .is_some_and(|(bag, _, _)| bank_store_destination_applies_obtain_spells_like_cpp(bag))
            || plan.existing_updates.iter().any(|update| {
                cx.get_inventory_item_by_guid_like_cpp(update.item.guid)
                    .is_some_and(|(bag, _, _)| {
                        bank_store_destination_applies_obtain_spells_like_cpp(bag)
                    })
            });
        let Some(current_non_bank_count) =
            cx.represented_non_bank_item_count_like_cpp(plan.source.entry_id)
        else {
            return;
        };
        (apply_obtain_spells, current_non_bank_count)
    };
    let post_move_non_bank_count = if quest_checks
        == InventoryStorageQuestChecksLikeCpp::AutoBankItemRemoved
        && moving_to_bank
        && !is_bank_pos(plan.source_bag, plan.source_slot)
    {
        current_non_bank_count.saturating_sub(plan.source_count)
    } else {
        current_non_bank_count
    };
    let planned_quest_statuses = match quest_checks {
        InventoryStorageQuestChecksLikeCpp::AutoBankItemRemoved => host
            .storage_move_plan_quest_statuses_like_cpp(
                plan.source.entry_id,
                0,
                true,
                post_move_non_bank_count,
                0,
            ),
        InventoryStorageQuestChecksLikeCpp::AutoStoreBankItemAdded if moving_from_bank => host
            .storage_move_plan_quest_statuses_like_cpp(
                plan.source.entry_id,
                quest_log_item_id,
                false,
                post_move_non_bank_count,
                added_quest_count,
            ),
        InventoryStorageQuestChecksLikeCpp::None
        | InventoryStorageQuestChecksLikeCpp::AutoStoreBankItemAdded => Vec::new(),
    };

    let (enchantment_persistence, binding_updates, request) = {
        let cx = host.storage_move_cx_like_cpp();
        let enchantment_persistence = plan.moved_destination.and_then(|_| {
            cx.inventory_remove_enchantment_persistence_like_cpp(
                plan.source.guid,
                !source_stays_in_place
                    && plan.source_bag == INVENTORY_SLOT_BAG_0
                    && plan.source_slot == wow_entities::EQUIPMENT_SLOT_MAINHAND,
            )
        });
        let mut binding_updates = Vec::new();
        for update in &plan.existing_updates {
            if let Some(mut item) = cx.resolved_inventory_item_object_like_cpp(update.item.guid) {
                let old_flags = item.item_flags_bits();
                item.bind_if_stored(wow_entities::is_bag_pos(wow_entities::make_item_pos(
                    update.bag,
                    update.slot,
                )));
                if item.item_flags_bits() != old_flags {
                    binding_updates.push((
                        update.item.guid,
                        update.item.db_guid,
                        item.item_flags_bits(),
                    ));
                }
            }
        }
        if let Some((destination_bag, destination_slot, _)) = plan.moved_destination
            && let Some(mut item) = cx.resolved_inventory_item_object_like_cpp(plan.source.guid)
        {
            let old_flags = item.item_flags_bits();
            item.bind_if_stored(wow_entities::is_bag_pos(wow_entities::make_item_pos(
                destination_bag,
                destination_slot,
            )));
            if item.item_flags_bits() != old_flags {
                binding_updates.push((
                    plan.source.guid,
                    plan.source.db_guid,
                    item.item_flags_bits(),
                ));
            }
        }

        let planned_flags = |item_guid: ObjectGuid, fallback: u32| {
            binding_updates
                .iter()
                .find(|(guid, _, _)| *guid == item_guid)
                .map_or(fallback, |(_, _, flags)| *flags)
        };
        let mut mutable_persistence = Vec::new();
        for update in &plan.existing_updates {
            let Some(item) = cx.resolved_inventory_item_object_like_cpp(update.item.guid) else {
                cx.send_equip_error(
                    InventoryResult::ItemNotFound,
                    Some(update.item.guid),
                    None,
                    0,
                    source_limit_category,
                );
                return;
            };
            let Some((enchantments, _)) =
                cx.inventory_remove_enchantment_persistence_like_cpp(update.item.guid, false)
            else {
                cx.send_equip_error(
                    InventoryResult::ItemNotFound,
                    Some(update.item.guid),
                    None,
                    0,
                    source_limit_category,
                );
                return;
            };
            mutable_persistence.push(
                crate::inventory_swap::item_storage_mutable_persistence_like_cpp(
                    update.item.db_guid,
                    &item,
                    update.new_count,
                    planned_flags(update.item.guid, item.item_flags_bits()),
                    enchantments,
                    cx.item_effect_count_like_cpp(update.item.entry_id),
                ),
            );
        }
        if let Some((_, _, moved_count)) = plan.moved_destination {
            let Some(item) = cx.resolved_inventory_item_object_like_cpp(plan.source.guid) else {
                cx.send_equip_error(
                    InventoryResult::ItemNotFound,
                    Some(plan.source.guid),
                    None,
                    0,
                    source_limit_category,
                );
                return;
            };
            let Some((enchantments, _)) = enchantment_persistence.as_ref() else {
                cx.send_equip_error(
                    InventoryResult::ItemNotFound,
                    Some(plan.source.guid),
                    None,
                    0,
                    source_limit_category,
                );
                return;
            };
            mutable_persistence.push(
                crate::inventory_swap::item_storage_mutable_persistence_like_cpp(
                    plan.source.db_guid,
                    &item,
                    moved_count,
                    planned_flags(plan.source.guid, item.item_flags_bits()),
                    enchantments.clone(),
                    cx.item_effect_count_like_cpp(plan.source.entry_id),
                ),
            );
        }

        let destination_link = match plan.moved_destination {
            Some((destination_bag, destination_slot, _)) if !source_stays_in_place => {
                let Some(container_db_guid) =
                    cx.inventory_container_db_guid_like_cpp(destination_bag)
                else {
                    cx.send_equip_error(
                        InventoryResult::WrongBagType,
                        Some(plan.source.guid),
                        None,
                        0,
                        source_limit_category,
                    );
                    return;
                };
                Some(wow_persistence::InventoryLinkPersistenceLikeCpp {
                    owner_guid: player_guid.counter() as u64,
                    bag_guid: container_db_guid,
                    slot: destination_slot,
                    item_guid: plan.source.db_guid,
                })
            }
            _ => None,
        };
        let request = wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::StorageMove(
            wow_persistence::InventoryStorageMovePersistenceLikeCpp {
                owner_guid: player_guid.counter() as u64,
                mutable_items: mutable_persistence,
                delete_source_link_item_guid: (!source_stays_in_place)
                    .then_some(plan.source.db_guid),
                destination_link,
                fully_merged_source_item_guid: plan
                    .moved_destination
                    .is_none()
                    .then_some(plan.source.db_guid),
                quest_statuses: cx
                    .represented_quest_status_persistence_rows_like_cpp(&planned_quest_statuses),
            },
        );
        (enchantment_persistence, binding_updates, request)
    };
    let outcome = inventory_port
        .persist_inventory_mutation_like_cpp(request)
        .await;
    if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
    | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } = outcome
    {
        warn!(
            source_bag,
            source_slot,
            item_guid = plan.source.db_guid,
            error = %reason,
            "bank storage transaction failed; runtime left unchanged"
        );
        host.storage_move_cx_like_cpp().send_equip_error(
            InventoryResult::InternalBagError,
            Some(plan.source.guid),
            None,
            0,
            source_limit_category,
        );
        return;
    }

    let (source_leaves_position, represented_item_mods_changed) = {
        let mut cx = host.storage_move_cx_like_cpp();
        let map_id = cx.player_map_id_like_cpp();
        for (item_guid, _, _) in &binding_updates {
            let _ = cx.apply_inventory_item_object_updates_like_cpp(
                *item_guid,
                &[wow_entities::ItemObjectUpdateLikeCpp::SetBinding(true)],
            );
            cx.send_item_dynamic_flags_values_update_like_cpp(*item_guid);
        }
        for update in &plan.existing_updates {
            let _ = cx.apply_inventory_item_object_updates_like_cpp(
                update.item.guid,
                &[wow_entities::ItemObjectUpdateLikeCpp::SetCount(
                    update.new_count,
                )],
            );
            cx.send_packet(&UpdateObject::item_stack_count_update(
                update.item.guid,
                map_id,
                update.new_count,
            ));
            cx.refresh_inventory_item_enchantment_duration_refs_like_cpp(update.item.guid);
        }

        let source_leaves_position = !source_stays_in_place;
        let source_dynamic_flags2_changed = source_leaves_position
            && plan.source_bag == INVENTORY_SLOT_BAG_0
            && plan.source_slot < INVENTORY_SLOT_BAG_END
            && cx
                .resolved_inventory_item_object_like_cpp(plan.source.guid)
                .is_some_and(|item| item.has_item_flag2(wow_constants::ItemFieldFlags2::EQUIPPED));
        if source_stays_in_place {
            cx.remove_inventory_item_duration_refs_like_cpp(plan.source.guid);
            cx.remove_inventory_tradeable_item_like_cpp(plan.source.guid);
        }
        let represented_item_mods_changed = if source_leaves_position {
            cx.apply_inventory_item_remove_side_effects_like_cpp(
                plan.source_bag,
                plan.source_slot,
                plan.source.guid,
                enchantment_persistence
                    .as_ref()
                    .map(|(_, slots)| slots.as_slice())
                    .unwrap_or_default(),
            )
        } else {
            false
        };

        let mut top_level_changes = Vec::new();
        let mut visible_item_changes = Vec::new();
        let mut virtual_item_changes = Vec::new();
        if source_leaves_position && plan.source_bag == INVENTORY_SLOT_BAG_0 {
            top_level_changes.push((plan.source_slot, ObjectGuid::EMPTY));
            if plan.source_slot < 19 {
                visible_item_changes.push((plan.source_slot, 0, 0, 0));
            }
            if (15..=17).contains(&plan.source_slot) {
                virtual_item_changes.push((plan.source_slot - 15, 0, 0, 0));
            }
        }
        if let Some((destination_bag, destination_slot, moved_count)) = plan.moved_destination {
            if source_stays_in_place {
                let _ = cx.apply_inventory_item_object_updates_like_cpp(
                    plan.source.guid,
                    &[wow_entities::ItemObjectUpdateLikeCpp::SetCount(moved_count)],
                );
                cx.add_inventory_item_duration_refs_like_cpp(plan.source.guid);
                cx.send_packet(&UpdateObject::item_stack_count_update(
                    plan.source.guid,
                    map_id,
                    moved_count,
                ));
            } else {
                // All possible failure conditions were checked before the commit.
                let relocated = cx.apply_committed_inventory_item_relocation_like_cpp(
                    plan.source_bag,
                    plan.source_slot,
                    destination_bag,
                    destination_slot,
                    moved_count,
                );
                debug_assert!(relocated);
                cx.add_inventory_item_duration_refs_like_cpp(plan.source.guid);
                if destination_bag == INVENTORY_SLOT_BAG_0 {
                    top_level_changes.push((destination_slot, plan.source.guid));
                }
                cx.send_item_relocation_values_update_like_cpp(
                    plan.source.guid,
                    source_dynamic_flags2_changed,
                    enchantment_persistence
                        .as_ref()
                        .map(|(_, slots)| slots.as_slice())
                        .unwrap_or_default(),
                );
                if moved_count != plan.source_count {
                    cx.send_packet(&UpdateObject::item_stack_count_update(
                        plan.source.guid,
                        map_id,
                        moved_count,
                    ));
                }
                if plan.source_bag != INVENTORY_SLOT_BAG_0 {
                    cx.send_bag_slot_values_update_like_cpp(plan.source_bag, plan.source_slot);
                }
                if destination_bag != INVENTORY_SLOT_BAG_0 {
                    cx.send_bag_slot_values_update_like_cpp(destination_bag, destination_slot);
                }
            }
        } else {
            let removed = cx.apply_committed_inventory_item_removal_like_cpp(
                plan.source_bag,
                plan.source_slot,
                plan.source.guid,
            );
            debug_assert!(removed);
            cx.send_packet(&UpdateObject::destroy_objects(
                vec![plan.source.guid],
                map_id,
            ));
            if plan.source_bag != INVENTORY_SLOT_BAG_0 {
                cx.send_bag_slot_values_update_like_cpp(plan.source_bag, plan.source_slot);
            }
        }
        if !top_level_changes.is_empty() {
            let _ = cx.send_player_values_update_from_entity_bridge(
                &top_level_changes,
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );
        }
        (source_leaves_position, represented_item_mods_changed)
    };
    if (source_leaves_position && plan.source_bag == INVENTORY_SLOT_BAG_0 && plan.source_slot < 19)
        || represented_item_mods_changed
    {
        host.storage_move_stats_cx_like_cpp()
            .send_stat_update_like_cpp();
    }
    if source_leaves_position && plan.source_bag == INVENTORY_SLOT_BAG_0 {
        if plan.source_slot < wow_entities::EQUIPMENT_SLOT_END {
            host.storage_move_record_titan_grip_penalty_action_like_cpp();
        }
        host.storage_move_record_avg_equipped_item_level_update_like_cpp();
    }
    if apply_obtain_spells {
        host.storage_move_apply_obtain_spells_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
            plan.source.entry_id,
        )
        .await;
    }

    let mut changed_quest_ids =
        if quest_checks == InventoryStorageQuestChecksLikeCpp::AutoBankItemRemoved {
            let Some(changed) =
                host.storage_move_apply_quest_item_removed_like_cpp(plan.source.entry_id)
            else {
                return;
            };
            changed
        } else {
            Vec::new()
        };
    if runs_added_quest_check {
        changed_quest_ids.extend(
            host.storage_move_apply_quest_item_added_like_cpp(
                item_guid_generator,
                plan.source.entry_id,
                quest_log_item_id,
                added_quest_count,
            )
            .await,
        );
    }
    changed_quest_ids.sort_unstable();
    changed_quest_ids.dedup();
    debug_assert_eq!(
        changed_quest_ids.len(),
        planned_quest_statuses.len(),
        "bank quest persistence plan must match committed runtime removal"
    );
    if let Some(represented_move) = represented_move {
        host.storage_move_cx_like_cpp()
            .record_represented_bank_item_move_like_cpp(represented_move);
    }
}
