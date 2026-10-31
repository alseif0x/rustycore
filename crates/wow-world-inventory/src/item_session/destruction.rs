// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory item destruction and recursive persistence.

use std::collections::{BTreeSet, HashMap};

use tracing::{debug, info, warn};
use wow_constants::{InventoryResult, ItemFlags};
use wow_core::ObjectGuid;
use wow_entities::{INVENTORY_SLOT_BAG_0, PlayerInventoryItem as InventoryItem, is_bank_pos};
use wow_packet::packets::item::{InventoryChangeFailure, ItemExpirePurchaseRefund};
use wow_packet::packets::update::UpdateObject;

use super::{
    DestroyItemCountAction, DestroyQuestItemLikeCpp, ItemSessionHostLikeCpp,
    destroy_item_count_action,
};

#[derive(Debug)]
pub(super) enum PartialDestroyFailureLikeCpp {
    PersistencePortUnavailable,
    QuestPersistencePlanUnavailable,
    Persistence { reason: String },
    PlayerIncarnationChanged,
    ItemCountApplicationUnavailable,
    QuestRemovalCallbackUnavailable,
}

/// Handle CMSG_DESTROY_ITEM: delete an item from inventory.
pub async fn handle_destroy_item<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    destroy: wow_packet::packets::item::DestroyItemPkt,
) {
    let player_guid = match host.player_guid() {
        Some(g) => g,
        None => return,
    };

    debug!(
        "DestroyItem: container={} slot={} count={} for {:?}",
        destroy.container_id, destroy.slot_num, destroy.count, player_guid
    );

    let bag = destroy.container_id;
    let slot = destroy.slot_num;
    let item = match {
        let (state, hub) = host.split_inventory_ref();
        state.get_inventory_item_by_pos(hub, bag, slot)
    } {
        Some(item) => item,
        None => {
            host.core()
                .send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
            return;
        }
    };

    let runtime_item = {
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_object_like_cpp(hub, item.guid)
    };
    let item_proto = host.hub_ref().catalogs.item_storage_template(item.entry_id);
    let unequip_result = {
        let (state, hub) = host.split_inventory_ref();
        state.can_unequip_inventory_item_at_like_cpp(
            hub,
            bag,
            slot,
            false,
            runtime_item.as_ref(),
            item_proto.as_ref(),
            {
                let (state, hub) = host.split_inventory_ref();
                state.direct_item_contains_items(hub, item.guid)
            },
        )
    };
    if unequip_result != InventoryResult::Ok {
        host.core()
            .send_packet_realm(&InventoryChangeFailure::error(unequip_result));
        return;
    }

    if host
        .hub_ref()
        .catalogs
        .item_template_flags(item.entry_id)
        .is_some_and(|flags| flags.contains(ItemFlags::NO_USER_DESTROY))
    {
        host.core()
            .send_packet_realm(&InventoryChangeFailure::error(
                InventoryResult::DropBoundItem,
            ));
        return;
    }

    let count_action = runtime_item
        .as_ref()
        .map(|item_object| {
            destroy_item_count_action(
                item_object.count(),
                u32::try_from(destroy.count).unwrap_or(u32::MAX),
            )
        })
        .unwrap_or(DestroyItemCountAction::FullStack);

    if let DestroyItemCountAction::PartialStack { new_count } = count_action {
        let item_entry_id = item.entry_id;
        let removed_count = runtime_item
            .as_ref()
            .map(|item_object| item_object.count().saturating_sub(new_count))
            .unwrap_or(0);
        match apply_partial_inventory_destroy_like_cpp(
            host,
            player_guid,
            bag,
            slot,
            item,
            new_count,
            removed_count,
            None,
            "DestroyItem",
        )
        .await
        {
            Ok(()) => info!(
                "Destroyed partial item entry={} at ({},{}) count={} for {:?}",
                item_entry_id, bag, slot, destroy.count, player_guid
            ),
            Err(PartialDestroyFailureLikeCpp::Persistence { reason }) => {
                warn!(error = %reason, "DestroyItem: update partial stack count failed");
                host.core()
                    .send_packet_realm(&InventoryChangeFailure::error(
                        InventoryResult::InternalBagError,
                    ));
            }
            Err(_) => {}
        }
        return;
    }

    let destroyed_entry_id = item.entry_id;
    if destroy_inventory_full_stack_by_pos_like_cpp(
        host,
        bag,
        slot,
        item,
        runtime_item,
        "DestroyItem",
    )
    .await
    {
        info!(
            "Destroyed item entry={} at ({},{}) for {:?}",
            destroyed_entry_id, bag, slot, player_guid
        );
    }
}

/// Persist and apply one partial stack removal shared by the client handler
/// and native `Player::DestroyItemCount`. The optional owner identity is the
/// generation-checked session Player captured by the native operation.
#[allow(clippy::too_many_arguments)]
pub(super) async fn apply_partial_inventory_destroy_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    player_guid: ObjectGuid,
    bag: u8,
    slot: u8,
    item: InventoryItem,
    new_count: u32,
    removed_count: u32,
    expected_owner: Option<(ObjectGuid, u64)>,
    context: &str,
) -> Result<(), PartialDestroyFailureLikeCpp> {
    let inventory_port = host
        .lifecycle()
        .player_inventory_persistence_port_like_cpp()
        .ok_or(PartialDestroyFailureLikeCpp::PersistencePortUnavailable)?;
    let Some(planned_quest_statuses) = plan_destroyed_inventory_quest_persistence_like_cpp(
        host,
        &[DestroyQuestItemLikeCpp {
            bag,
            slot,
            entry_id: item.entry_id,
            count: removed_count,
        }],
    ) else {
        return Err(PartialDestroyFailureLikeCpp::QuestPersistencePlanUnavailable);
    };
    let request = wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::PartialDestroy(
        wow_persistence::InventoryPartialDestroyPersistenceLikeCpp {
            owner_guid: player_guid.counter() as u64,
            item_guid: item.db_guid,
            new_count,
            quest_statuses: host
                .hub_ref()
                .represented_quest_status_persistence_rows_like_cpp(&planned_quest_statuses),
        },
    );
    let outcome = inventory_port
        .persist_inventory_mutation_like_cpp(request)
        .await;

    if let Some((expected_guid, expected_generation)) = expected_owner
        && !item_destruction_owner_matches_like_cpp(host, expected_guid, expected_generation)
    {
        return Err(PartialDestroyFailureLikeCpp::PlayerIncarnationChanged);
    }

    if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
    | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } = outcome
    {
        return Err(PartialDestroyFailureLikeCpp::Persistence { reason });
    }

    let count_applied = {
        let (state, mut hub) = host.split_inventory_mut();
        state.apply_inventory_item_object_updates_like_cpp(
            &mut hub,
            item.guid,
            &[wow_entities::ItemObjectUpdateLikeCpp::SetCount(new_count)],
        )
    };
    if !count_applied && expected_owner.is_some() {
        return Err(PartialDestroyFailureLikeCpp::ItemCountApplicationUnavailable);
    }
    let Some(changed_quest_ids) = host.apply_quest_item_removed_like_cpp(item.entry_id) else {
        return Err(PartialDestroyFailureLikeCpp::QuestRemovalCallbackUnavailable);
    };
    debug_assert_eq!(
        changed_quest_ids.len(),
        planned_quest_statuses.len(),
        "partial destroy quest persistence must match committed runtime removal"
    );

    let publish_update = if let Some((expected_guid, expected_generation)) = expected_owner {
        if !item_destruction_owner_matches_like_cpp(host, expected_guid, expected_generation) {
            return Err(PartialDestroyFailureLikeCpp::PlayerIncarnationChanged);
        }
        host.core()
            .with_owned_player_like_cpp(|player| player.unit().world().object().is_in_world())
            .ok_or(PartialDestroyFailureLikeCpp::PlayerIncarnationChanged)?
    } else {
        // Preserve the existing client handler's publication behavior.
        true
    };
    if publish_update {
        host.send_packet(&UpdateObject::item_stack_count_update(
            item.guid,
            host.core().player_map_id_like_cpp(),
            new_count,
        ));
    }
    debug!(
        "{context}: applied partial item count entry={} bag={} slot={} count={new_count}",
        item.entry_id, bag, slot
    );
    Ok(())
}

fn item_destruction_owner_matches_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &H,
    expected_guid: ObjectGuid,
    expected_generation: u64,
) -> bool {
    let actual_handle = host
        .core()
        .player_handle_like_cpp
        .map(|handle| (handle.guid(), handle.generation()));
    let current_player_guid = host
        .core()
        .with_owned_player_like_cpp(|player| player.guid());
    item_destruction_identity_matches_like_cpp(
        (expected_guid, expected_generation),
        actual_handle,
        current_player_guid,
    )
}

fn item_destruction_identity_matches_like_cpp(
    expected: (ObjectGuid, u64),
    actual_handle: Option<(ObjectGuid, u64)>,
    current_player_guid: Option<ObjectGuid>,
) -> bool {
    actual_handle == Some(expected) && current_player_guid == Some(expected.0)
}

pub fn plan_destroyed_inventory_quest_persistence_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &H,
    destroyed_items: &[DestroyQuestItemLikeCpp],
) -> Option<Vec<wow_entities::PlayerQuestStatusRecord>> {
    let removed_entries_in_order = destroyed_items
        .iter()
        .map(|item| item.entry_id)
        .collect::<Vec<_>>();
    let mut removed_non_bank_counts = HashMap::<u32, u32>::new();
    for item in destroyed_items {
        if !is_bank_pos(item.bag, item.slot) {
            removed_non_bank_counts
                .entry(item.entry_id)
                .and_modify(|count| *count = count.saturating_add(item.count))
                .or_insert(item.count);
        }
    }
    let post_removal_non_bank_counts = removed_entries_in_order
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|entry_id| {
            let current = ({
                let (state, hub) = host.split_inventory_ref();
                state.represented_non_bank_item_count_like_cpp(hub, entry_id)
            })?;
            let removed = removed_non_bank_counts.get(&entry_id).copied().unwrap_or(0);
            Some((entry_id, current.saturating_sub(removed)))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(host.plan_item_transfer_quest_persistence_like_cpp(
        &removed_entries_in_order,
        &post_removal_non_bank_counts,
        &[],
    ))
}

/// C++ `Player::DestroyItem(bag, slot, update=true)` for a full-stack item.
pub async fn destroy_inventory_full_stack_by_pos_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    bag: u8,
    slot: u8,
    item: InventoryItem,
    runtime_item: Option<wow_entities::Item>,
    context: &str,
) -> bool {
    destroy_inventory_full_stack_by_pos_common_like_cpp(
        host,
        bag,
        slot,
        item,
        runtime_item,
        None,
        None,
        context,
    )
    .await
}

pub async fn destroy_inventory_full_stack_by_pos_with_expected_owner_like_cpp<
    H: ItemSessionHostLikeCpp,
>(
    host: &mut H,
    bag: u8,
    slot: u8,
    item: InventoryItem,
    runtime_item: Option<wow_entities::Item>,
    expected_owner_db_guid: Option<u64>,
    context: &str,
) -> bool {
    destroy_inventory_full_stack_by_pos_common_like_cpp(
        host,
        bag,
        slot,
        item,
        runtime_item,
        expected_owner_db_guid,
        None,
        context,
    )
    .await
}

pub(super) async fn destroy_inventory_full_stack_by_pos_with_expected_runtime_owner_like_cpp<
    H: ItemSessionHostLikeCpp,
>(
    host: &mut H,
    bag: u8,
    slot: u8,
    item: InventoryItem,
    runtime_item: Option<wow_entities::Item>,
    expected_runtime_owner: (ObjectGuid, u64),
    context: &str,
) -> bool {
    destroy_inventory_full_stack_by_pos_common_like_cpp(
        host,
        bag,
        slot,
        item,
        runtime_item,
        None,
        Some(expected_runtime_owner),
        context,
    )
    .await
}

// Database item ownership and live Session Player incarnation are separate guards.
async fn destroy_inventory_full_stack_by_pos_common_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    bag: u8,
    slot: u8,
    item: InventoryItem,
    runtime_item: Option<wow_entities::Item>,
    expected_owner_db_guid: Option<u64>,
    expected_runtime_owner: Option<(ObjectGuid, u64)>,
    context: &str,
) -> bool {
    let player_guid = match host.player_guid() {
        Some(guid) => guid,
        None => return false,
    };
    let inventory_port = match host
        .lifecycle()
        .player_inventory_persistence_port_like_cpp()
    {
        Some(port) => port,
        None => return false,
    };

    // C++ Player::DestroyItem recursively destroys a bag's contents before
    // deleting the bag itself. Keep all corresponding character rows in a
    // single transaction so a persistence failure cannot orphan children
    // or expose a partially destroyed runtime graph.
    let Some(descendants) = ({
        let (state, hub) = host.split_inventory_ref();
        state.represented_inventory_descendants_postorder_like_cpp(hub, item.guid)
    }) else {
        return false;
    };
    let descendant_runtime = descendants
        .iter()
        .map(|(child_bag, child_slot, child)| {
            (*child_bag, *child_slot, child.clone(), {
                let (state, hub) = host.split_inventory_ref();
                state.resolved_inventory_item_object_like_cpp(hub, child.guid)
            })
        })
        .collect::<Vec<_>>();

    let mut destroyed_quest_items = descendant_runtime
        .iter()
        .map(
            |(child_bag, child_slot, child, child_runtime)| DestroyQuestItemLikeCpp {
                bag: *child_bag,
                slot: *child_slot,
                entry_id: child.entry_id,
                count: child_runtime
                    .as_ref()
                    .map(wow_entities::Item::count)
                    .unwrap_or(1),
            },
        )
        .collect::<Vec<_>>();
    destroyed_quest_items.push(DestroyQuestItemLikeCpp {
        bag,
        slot,
        entry_id: item.entry_id,
        count: runtime_item
            .as_ref()
            .map(wow_entities::Item::count)
            .unwrap_or(1),
    });
    let Some(planned_quest_statuses) =
        plan_destroyed_inventory_quest_persistence_like_cpp(host, &destroyed_quest_items)
    else {
        return false;
    };

    let should_expire_refund = runtime_item
        .as_ref()
        .is_some_and(|item_object| item_object.is_refundable());

    let nodes = descendant_runtime
        .iter()
        .map(|(_, _, child, _)| child.db_guid)
        .chain(std::iter::once(item.db_guid))
        .map(
            |db_guid| wow_persistence::InventoryDestroyNodePersistenceLikeCpp {
                item_guid: db_guid,
                expected_owner_db_guid: (db_guid == item.db_guid)
                    .then_some(expected_owner_db_guid)
                    .flatten(),
            },
        )
        .collect();
    let request = wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::GraphDestroy(
        wow_persistence::InventoryGraphDestroyPersistenceLikeCpp {
            owner_guid: player_guid.counter() as u64,
            nodes,
            quest_statuses: host
                .hub_ref()
                .represented_quest_status_persistence_rows_like_cpp(&planned_quest_statuses),
        },
    );
    let outcome = inventory_port
        .persist_inventory_mutation_like_cpp(request)
        .await;
    if let Some((expected_guid, expected_generation)) = expected_runtime_owner
        && !item_destruction_owner_matches_like_cpp(host, expected_guid, expected_generation)
    {
        return false;
    }
    if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
    | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } = outcome
    {
        warn!(error = %reason, "{context}: delete transaction failed");
        host.core()
            .send_packet_realm(&InventoryChangeFailure::error(
                InventoryResult::InternalBagError,
            ));
        return false;
    }

    if let Some(expected_owner_db_guid) = expected_owner_db_guid {
        let outcome = host
            .uncage_item_state_like_cpp(expected_owner_db_guid, item.db_guid)
            .await;
        if let Some((expected_guid, expected_generation)) = expected_runtime_owner
            && !item_destruction_owner_matches_like_cpp(host, expected_guid, expected_generation)
        {
            return false;
        }
        match outcome {
            wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Loaded(state)
                if state.owner_guid.is_none() && !state.inventory_linked => {}
            wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Loaded(state) => {
                warn!(
                    item_guid = item.db_guid,
                    owner_guid = ?state.owner_guid,
                    inventory_linked = state.inventory_linked,
                    "{context}: guarded item deletion did not reach its durable postcondition"
                );
                return false;
            }
            wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    item_guid = item.db_guid,
                    %reason,
                    "{context}: failed to verify guarded item deletion"
                );
                return false;
            }
        }
    }

    let mut destroyed_guids = Vec::with_capacity(descendant_runtime.len() + 1);
    let mut changed_quest_ids = Vec::new();
    for (child_bag, child_slot, child, child_runtime) in descendant_runtime {
        let should_expire_child_refund = child_runtime
            .as_ref()
            .is_some_and(|item_object| item_object.is_refundable());
        let _ = host.apply_inventory_item_remove_side_effects_like_cpp(
            child_bag,
            child_slot,
            child.guid,
            &[],
        );
        let removed =
            host.apply_committed_inventory_item_removal_like_cpp(child_bag, child_slot, child.guid);
        debug_assert!(removed);
        if should_expire_child_refund {
            host.send_packet(&ItemExpirePurchaseRefund {
                item_guid: child.guid,
            });
        }
        destroyed_guids.push(child.guid);
        let Some(changed) = host.apply_quest_item_removed_like_cpp(child.entry_id) else {
            return false;
        };
        changed_quest_ids.extend(changed);
    }

    let represented_item_mods_changed =
        host.apply_inventory_item_remove_side_effects_like_cpp(bag, slot, item.guid, &[]);

    let removed = host.apply_committed_inventory_item_removal_like_cpp(bag, slot, item.guid);
    debug_assert!(removed);
    destroyed_guids.push(item.guid);
    let Some(changed) = host.apply_quest_item_removed_like_cpp(item.entry_id) else {
        return false;
    };
    changed_quest_ids.extend(changed);
    changed_quest_ids.sort_unstable();
    changed_quest_ids.dedup();
    debug_assert_eq!(
        changed_quest_ids.len(),
        planned_quest_statuses.len(),
        "recursive destroy quest persistence must match child/parent runtime removals"
    );
    host.sync_player_registry_state_like_cpp();

    host.send_packet(&UpdateObject::destroy_objects(
        destroyed_guids,
        host.core().player_map_id_like_cpp(),
    ));

    if should_expire_refund {
        host.send_packet(&ItemExpirePurchaseRefund {
            item_guid: item.guid,
        });
    }

    if bag == INVENTORY_SLOT_BAG_0 {
        let inv_slot_changes = vec![(slot, ObjectGuid::EMPTY)];
        let mut visible_item_changes = Vec::new();
        let mut virtual_item_changes = Vec::new();

        if (slot as usize) < 19 {
            visible_item_changes.push((slot, 0i32, 0u16, 0u16));
        }
        if (15..=17).contains(&slot) {
            virtual_item_changes.push((slot - 15, 0i32, 0u16, 0u16));
        }

        host.send_player_values_update_from_entity_bridge(
            &inv_slot_changes,
            &visible_item_changes,
            &virtual_item_changes,
            &[],
            None,
        );

        if slot < 19 || represented_item_mods_changed {
            host.send_stat_update();
        }
    } else {
        {
            let (state, hub) = host.split_inventory_ref();
            state.send_bag_slot_values_update_like_cpp(hub, bag, slot)
        };
    }

    true
}

#[cfg(test)]
mod identity_tests {
    use super::*;

    #[test]
    fn item_destruction_owner_match_rejects_stale_player_generation() {
        let guid = ObjectGuid::create_player(1, 42);
        let expected = (guid, 7);

        assert!(item_destruction_identity_matches_like_cpp(
            expected,
            Some(expected),
            Some(guid),
        ));
        assert!(!item_destruction_identity_matches_like_cpp(
            expected,
            Some((guid, 8)),
            Some(guid),
        ));
    }
}
