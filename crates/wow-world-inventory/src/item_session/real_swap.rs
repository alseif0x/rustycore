// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Real two-item swap planning and application.

use tracing::warn;
use wow_constants::{InventoryResult, ItemFieldFlags};
use wow_core::ObjectGuid;
use wow_entities::{
    INVENTORY_SLOT_BAG_0, PlayerInventoryItem as InventoryItem, is_equipment_pos,
    item_can_go_into_bag,
};
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

use super::item_mutations::execute_inventory_auto_unequip_offhand_if_need_like_cpp;
use super::moves::{execute_inventory_equip_child_like_cpp, plan_inventory_equip_child_like_cpp};
use super::{InventoryEquipChildPlanLikeCpp, ItemSessionHostLikeCpp};
use crate::{
    InventorySwapTargetLikeCpp, bind_inventory_item_for_destination_like_cpp,
    item_dynamic_flags_changed_like_cpp, item_storage_mutable_persistence_like_cpp,
};

#[allow(clippy::too_many_arguments)]
pub fn plan_inventory_real_swap_children_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &H,
    source_bag: u8,
    source_slot: u8,
    source_guid: ObjectGuid,
    destination_bag: u8,
    destination_slot: u8,
    destination_guid: ObjectGuid,
) -> Result<Vec<InventoryEquipChildPlanLikeCpp>, InventoryResult> {
    let mut plans = Vec::new();
    if is_equipment_pos(destination_bag, destination_slot) {
        if let Some(plan) =
            plan_inventory_equip_child_like_cpp(host, source_bag, source_slot, source_guid)?
        {
            plans.push(plan);
        }
    }
    if is_equipment_pos(source_bag, source_slot) {
        if let Some(plan) = plan_inventory_equip_child_like_cpp(
            host,
            destination_bag,
            destination_slot,
            destination_guid,
        )? {
            plans.push(plan);
        }
    }
    Ok(plans)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_inventory_real_swap_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    source_bag: u8,
    source_slot: u8,
    destination_bag: u8,
    destination_slot: u8,
    source: InventoryItem,
    destination: InventoryItem,
    _source_target: InventorySwapTargetLikeCpp,
    _destination_target: InventorySwapTargetLikeCpp,
) {
    let Some(player_guid) = host.player_guid() else {
        return;
    };
    let Some(source_object) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_object_like_cpp(hub, source.guid)
    }) else {
        return;
    };
    let Some(destination_object) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_object_like_cpp(hub, destination.guid)
    }) else {
        return;
    };
    let child_plans = match plan_inventory_real_swap_children_like_cpp(
        host,
        source_bag,
        source_slot,
        source.guid,
        destination_bag,
        destination_slot,
        destination.guid,
    ) {
        Ok(plans) => plans,
        Err(result) => {
            host.core()
                .send_equip_error(result, Some(source.guid), Some(destination.guid), 0, 0);
            return;
        }
    };
    let Some(source_container_db_guid) = ({
        let (state, hub) = host.split_inventory_ref();
        state.inventory_container_db_guid_like_cpp(hub, source_bag)
    }) else {
        return;
    };
    let Some(destination_container_db_guid) = ({
        let (state, hub) = host.split_inventory_ref();
        state.inventory_container_db_guid_like_cpp(hub, destination_bag)
    }) else {
        return;
    };
    let Some((source_enchantments, source_cleared)) = ({
        let (state, hub) = host.split_inventory_ref();
        state.inventory_remove_enchantment_persistence_like_cpp(
            hub,
            source.guid,
            source_bag == INVENTORY_SLOT_BAG_0
                && source_slot == wow_entities::EQUIPMENT_SLOT_MAINHAND,
        )
    }) else {
        return;
    };
    let Some((destination_enchantments, destination_cleared)) = ({
        let (state, hub) = host.split_inventory_ref();
        state.inventory_remove_enchantment_persistence_like_cpp(
            hub,
            destination.guid,
            destination_bag == INVENTORY_SLOT_BAG_0
                && destination_slot == wow_entities::EQUIPMENT_SLOT_MAINHAND,
        )
    }) else {
        return;
    };

    let source_destination_pos = wow_entities::make_item_pos(destination_bag, destination_slot);
    let destination_source_pos = wow_entities::make_item_pos(source_bag, source_slot);
    let mut planned_source = source_object.clone();
    bind_inventory_item_for_destination_like_cpp(&mut planned_source, source_destination_pos);
    let source_dynamic_flags_changed =
        item_dynamic_flags_changed_like_cpp(&source_object, &planned_source);
    for slot in &source_cleared {
        planned_source.clear_enchantment(*slot);
    }
    let mut planned_destination = destination_object.clone();
    bind_inventory_item_for_destination_like_cpp(&mut planned_destination, destination_source_pos);
    let destination_dynamic_flags_changed =
        item_dynamic_flags_changed_like_cpp(&destination_object, &planned_destination);
    for slot in &destination_cleared {
        planned_destination.clear_enchantment(*slot);
    }

    let source_mutable = item_storage_mutable_persistence_like_cpp(
        source.db_guid,
        &planned_source,
        planned_source.count(),
        planned_source.item_flags_bits(),
        source_enchantments,
        host.hub_ref()
            .catalogs
            .item_effect_count_like_cpp(source.entry_id),
    );
    let destination_mutable = item_storage_mutable_persistence_like_cpp(
        destination.db_guid,
        &planned_destination,
        planned_destination.count(),
        planned_destination.item_flags_bits(),
        destination_enchantments,
        host.hub_ref()
            .catalogs
            .item_effect_count_like_cpp(destination.entry_id),
    );

    // C++ exchanges the contents when an empty bag outside a bag slot is
    // swapped with a non-empty equipped/bank bag. Persist those child
    // container changes in the same transaction as the two bag positions.
    let source_template = host
        .hub_ref()
        .catalogs
        .item_storage_template(source.entry_id);
    let destination_template = host
        .hub_ref()
        .catalogs
        .item_storage_template(destination.entry_id);
    let Some(item_objects) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_objects_like_cpp(hub)
    }) else {
        return;
    };
    let source_children = item_objects
        .values()
        .filter(|item| item.container_guid() == source.guid)
        .map(|item| (item.slot(), item.object().guid(), item.object().entry()))
        .collect::<Vec<_>>();
    let destination_children = item_objects
        .values()
        .filter(|item| item.container_guid() == destination.guid)
        .map(|item| (item.slot(), item.object().guid(), item.object().entry()))
        .collect::<Vec<_>>();
    let bag_exchange = match (source_template.as_ref(), destination_template.as_ref()) {
        (Some(source_proto), Some(destination_proto))
            if source_proto.container_slots > 0 && destination_proto.container_slots > 0 =>
        {
            if source_children.is_empty()
                && !wow_entities::is_bag_pos(destination_source_pos)
                && !destination_children.is_empty()
            {
                Some((
                    source.guid,
                    source.db_guid,
                    source_proto,
                    destination.guid,
                    destination_children.clone(),
                ))
            } else if destination_children.is_empty()
                && !wow_entities::is_bag_pos(source_destination_pos)
                && !source_children.is_empty()
            {
                Some((
                    destination.guid,
                    destination.db_guid,
                    destination_proto,
                    source.guid,
                    source_children.clone(),
                ))
            } else {
                None
            }
        }
        _ => None,
    };
    let mut child_moves = Vec::new();
    if let Some((empty_guid, empty_db_guid, empty_proto, full_guid, mut children)) = bag_exchange {
        children.sort_by_key(|(slot, guid, _)| (*slot, guid.counter()));
        if children.len() > usize::from(empty_proto.container_slots) {
            host.core().send_equip_error(
                InventoryResult::CantSwap,
                Some(source.guid),
                Some(destination.guid),
                0,
                0,
            );
            return;
        }
        for (to_slot, (from_slot, child_guid, child_entry)) in children.into_iter().enumerate() {
            let Some(child_proto) = host.hub_ref().catalogs.item_storage_template(child_entry)
            else {
                host.core().send_equip_error(
                    InventoryResult::BagInBag,
                    Some(source.guid),
                    Some(destination.guid),
                    0,
                    0,
                );
                return;
            };
            if !item_can_go_into_bag(&child_proto, empty_proto) {
                host.core().send_equip_error(
                    InventoryResult::BagInBag,
                    Some(source.guid),
                    Some(destination.guid),
                    0,
                    0,
                );
                return;
            }
            child_moves.push((
                child_guid,
                child_guid.counter() as u64,
                full_guid,
                empty_guid,
                empty_db_guid,
                from_slot,
                to_slot as u8,
            ));
        }
    }

    let Some(inventory_port) = host
        .lifecycle()
        .player_inventory_persistence_port_like_cpp()
    else {
        host.core().send_equip_error(
            InventoryResult::InternalBagError,
            Some(source.guid),
            Some(destination.guid),
            0,
            0,
        );
        return;
    };
    let child_links = child_moves
        .iter()
        .map(|(_, child_db_guid, _, _, empty_db_guid, _, to_slot)| {
            wow_persistence::InventoryLinkPersistenceLikeCpp {
                owner_guid: player_guid.counter() as u64,
                bag_guid: *empty_db_guid,
                slot: *to_slot,
                item_guid: *child_db_guid,
            }
        })
        .collect();
    let request = wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::Swap(
        wow_persistence::InventorySwapPersistenceLikeCpp {
            source_item: source_mutable,
            destination_item: destination_mutable,
            child_links,
            source_link: wow_persistence::InventoryLinkPersistenceLikeCpp {
                owner_guid: player_guid.counter() as u64,
                bag_guid: destination_container_db_guid,
                slot: destination_slot,
                item_guid: source.db_guid,
            },
            destination_link: wow_persistence::InventoryLinkPersistenceLikeCpp {
                owner_guid: player_guid.counter() as u64,
                bag_guid: source_container_db_guid,
                slot: source_slot,
                item_guid: destination.db_guid,
            },
        },
    );
    let outcome = inventory_port
        .persist_inventory_mutation_like_cpp(request)
        .await;
    if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
    | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } = outcome
    {
        warn!(
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            error = %reason,
            "inventory real swap transaction failed; runtime left unchanged"
        );
        host.core().send_equip_error(
            InventoryResult::InternalBagError,
            Some(source.guid),
            Some(destination.guid),
            0,
            0,
        );
        return;
    }

    let removed_source_mods = host.apply_inventory_item_remove_side_effects_like_cpp(
        source_bag,
        source_slot,
        source.guid,
        &source_cleared,
    );
    let removed_destination_mods = host.apply_inventory_item_remove_side_effects_like_cpp(
        destination_bag,
        destination_slot,
        destination.guid,
        &destination_cleared,
    );
    for (child_guid, _, _, empty_guid, _, _, to_slot) in &child_moves {
        let _ = {
            let (state, mut hub) = host.split_inventory_mut();
            state.apply_inventory_item_object_updates_like_cpp(
                &mut hub,
                *child_guid,
                &[
                    wow_entities::ItemObjectUpdateLikeCpp::RelocateBagExchangeChild {
                        destination_bag_guid: *empty_guid,
                        destination_slot: *to_slot,
                    },
                ],
            )
        };
    }
    let swapped = host.apply_committed_inventory_item_swap_like_cpp(
        source_bag,
        source_slot,
        destination_bag,
        destination_slot,
    );
    debug_assert!(swapped);
    let _ = {
        let (state, mut hub) = host.split_inventory_mut();
        state.apply_inventory_item_object_updates_like_cpp(
            &mut hub,
            source.guid,
            &[wow_entities::ItemObjectUpdateLikeCpp::ReplaceAllItemFlags(
                ItemFieldFlags::from_bits_retain(planned_source.item_flags_bits()),
            )],
        )
    };
    let _ = {
        let (state, mut hub) = host.split_inventory_mut();
        state.apply_inventory_item_object_updates_like_cpp(
            &mut hub,
            destination.guid,
            &[wow_entities::ItemObjectUpdateLikeCpp::ReplaceAllItemFlags(
                ItemFieldFlags::from_bits_retain(planned_destination.item_flags_bits()),
            )],
        )
    };
    let added_source_mods = host.apply_inventory_item_store_side_effects_like_cpp(
        destination_bag,
        destination_slot,
        source.guid,
    );
    let added_destination_mods = host.apply_inventory_item_store_side_effects_like_cpp(
        source_bag,
        source_slot,
        destination.guid,
    );

    host.publish_inventory_position_changes_like_cpp(&[
        (source_bag, source_slot),
        (destination_bag, destination_slot),
    ]);
    {
        let (state, hub) = host.split_inventory_ref();
        state.send_item_relocation_values_update_with_access_like_cpp(
            &hub.core.owned_inventory_access_like_cpp(),
            &hub.core.packet_publication_access_like_cpp(),
            source.guid,
            true,
            &source_cleared,
        )
    };
    if source_dynamic_flags_changed {
        {
            let (state, hub) = host.split_inventory_ref();
            state.send_item_dynamic_flags_values_update_with_access_like_cpp(
                &hub.core.owned_inventory_access_like_cpp(),
                &hub.core.packet_publication_access_like_cpp(),
                source.guid,
            )
        };
    }
    {
        let (state, hub) = host.split_inventory_ref();
        state.send_item_relocation_values_update_with_access_like_cpp(
            &hub.core.owned_inventory_access_like_cpp(),
            &hub.core.packet_publication_access_like_cpp(),
            destination.guid,
            true,
            &destination_cleared,
        )
    };
    if destination_dynamic_flags_changed {
        {
            let (state, hub) = host.split_inventory_ref();
            state.send_item_dynamic_flags_values_update_with_access_like_cpp(
                &hub.core.owned_inventory_access_like_cpp(),
                &hub.core.packet_publication_access_like_cpp(),
                destination.guid,
            )
        };
    }
    for (child_guid, _, full_guid, empty_guid, _, from_slot, to_slot) in &child_moves {
        {
            let (state, hub) = host.split_inventory_ref();
            state.send_item_relocation_values_update_with_access_like_cpp(
                &hub.core.owned_inventory_access_like_cpp(),
                &hub.core.packet_publication_access_like_cpp(),
                *child_guid,
                false,
                &[],
            )
        };
        {
            let (state, hub) = host.split_inventory_ref();
            state.send_bag_object_slot_values_update_with_access_like_cpp(
                &hub.core.owned_inventory_access_like_cpp(),
                &hub.core.packet_publication_access_like_cpp(),
                hub.catalogs.items.store.as_ref(),
                hub.catalogs.items.stats_store.as_ref(),
                *full_guid,
                *from_slot,
            )
        };
        {
            let (state, hub) = host.split_inventory_ref();
            state.send_bag_object_slot_values_update_with_access_like_cpp(
                &hub.core.owned_inventory_access_like_cpp(),
                &hub.core.packet_publication_access_like_cpp(),
                hub.catalogs.items.store.as_ref(),
                hub.catalogs.items.stats_store.as_ref(),
                *empty_guid,
                *to_slot,
            )
        };
    }
    if removed_source_mods
        || removed_destination_mods
        || added_source_mods
        || added_destination_mods
    {
        crate::send_represented_item_bonus_player_stat_update_like_cpp(host);
    }
    // Preserve the local 3.4.3 C++ ordering in Player::SwapItem: exchange
    // bag contents first, then inspect the items still contained by bags
    // that occupied src/dst bag slots. Snapshotting the pre-exchange
    // contents here would release loot in cases where C++ does not.
    let source_moved_bag_has_active_loot = wow_entities::is_bag_pos(destination_source_pos)
        && host.represented_bag_contains_active_item_loot_like_cpp(source.guid);
    let destination_moved_bag_has_active_loot = wow_entities::is_bag_pos(source_destination_pos)
        && host.represented_bag_contains_active_item_loot_like_cpp(destination.guid);
    if source_moved_bag_has_active_loot || destination_moved_bag_has_active_loot {
        host.do_loot_release_all_like_cpp(player_guid).await;
    }
    host.record_represented_titan_grip_penalty_action_like_cpp();
    super::record_represented_avg_equipped_item_level_update_like_cpp(host);
    host.sync_player_registry_state_like_cpp();
    for plan in child_plans {
        let _ = execute_inventory_equip_child_like_cpp(
            host,
            item_guid_generator,
            creature_spawn_catalogs,
            plan,
        )
        .await;
    }
    execute_inventory_auto_unequip_offhand_if_need_like_cpp(
        host,
        item_guid_generator,
        creature_spawn_catalogs,
    )
    .await;
}
