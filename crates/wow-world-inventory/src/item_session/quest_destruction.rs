// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Native `Player::DestroyItemCount` for quest-failure item removal.

use std::ops::Range;

use wow_constants::{InventoryResult, TypeMask};
use wow_core::ObjectGuid;
use wow_entities::{
    BANK_SLOT_BAG_END, BANK_SLOT_BAG_START, BANK_SLOT_ITEM_END, BANK_SLOT_ITEM_START,
    CHILD_EQUIPMENT_SLOT_END, CHILD_EQUIPMENT_SLOT_START, EQUIPMENT_SLOT_HEAD,
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START,
    INVENTORY_SLOT_ITEM_END, INVENTORY_SLOT_ITEM_START, KEYRING_SLOT_END, KEYRING_SLOT_START,
};

use super::ItemSessionHostLikeCpp;
use super::destruction::{
    PartialDestroyFailureLikeCpp, apply_partial_inventory_destroy_like_cpp,
    destroy_inventory_full_stack_by_pos_with_expected_runtime_owner_like_cpp,
};

const NATIVE_DESTROY_SCAN_ORDER_LIKE_CPP: [NativeDestroySectionLikeCpp; 8] = [
    NativeDestroySectionLikeCpp::Backpack,
    NativeDestroySectionLikeCpp::Keyring,
    NativeDestroySectionLikeCpp::CarriedBagContents,
    NativeDestroySectionLikeCpp::EquipmentAndBagList,
    NativeDestroySectionLikeCpp::BankItems,
    NativeDestroySectionLikeCpp::BankBagContents,
    NativeDestroySectionLikeCpp::BankBagList,
    NativeDestroySectionLikeCpp::ChildEquipment,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeDestroySectionLikeCpp {
    Backpack,
    Keyring,
    CarriedBagContents,
    EquipmentAndBagList,
    BankItems,
    BankBagContents,
    BankBagList,
    ChildEquipment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeDestroyActionLikeCpp {
    FullStack,
    PartialStack { new_count: u32, removed_count: u32 },
}

/// C++ `GetCount() + remcount <= count`, including its unsigned arithmetic.
fn native_destroy_action_like_cpp(
    current_count: u32,
    already_removed: u32,
    requested_count: u32,
) -> NativeDestroyActionLikeCpp {
    if current_count.wrapping_add(already_removed) <= requested_count {
        NativeDestroyActionLikeCpp::FullStack
    } else {
        NativeDestroyActionLikeCpp::PartialStack {
            new_count: current_count
                .wrapping_sub(requested_count)
                .wrapping_add(already_removed),
            removed_count: requested_count.wrapping_sub(already_removed),
        }
    }
}

fn direct_slots_for_section_like_cpp(
    section: NativeDestroySectionLikeCpp,
    backpack_end: u8,
) -> Option<Range<u8>> {
    match section {
        NativeDestroySectionLikeCpp::Backpack => Some(INVENTORY_SLOT_ITEM_START..backpack_end),
        NativeDestroySectionLikeCpp::Keyring => Some(KEYRING_SLOT_START..KEYRING_SLOT_END),
        NativeDestroySectionLikeCpp::EquipmentAndBagList => {
            Some(EQUIPMENT_SLOT_HEAD..INVENTORY_SLOT_BAG_END)
        }
        NativeDestroySectionLikeCpp::BankItems => Some(BANK_SLOT_ITEM_START..BANK_SLOT_ITEM_END),
        NativeDestroySectionLikeCpp::BankBagList => Some(BANK_SLOT_BAG_START..BANK_SLOT_BAG_END),
        NativeDestroySectionLikeCpp::ChildEquipment => {
            Some(CHILD_EQUIPMENT_SLOT_START..CHILD_EQUIPMENT_SLOT_END)
        }
        NativeDestroySectionLikeCpp::CarriedBagContents
        | NativeDestroySectionLikeCpp::BankBagContents => None,
    }
}

fn bag_slots_for_section_like_cpp(section: NativeDestroySectionLikeCpp) -> Option<Range<u8>> {
    match section {
        NativeDestroySectionLikeCpp::CarriedBagContents => {
            Some(INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END)
        }
        NativeDestroySectionLikeCpp::BankBagContents => {
            Some(BANK_SLOT_BAG_START..BANK_SLOT_BAG_END)
        }
        _ => None,
    }
}

/// C++ `Player::DestroyItemCount` over the canonical, generation-checked
/// Player. The order, full-stack equipment checks, callbacks and per-position
/// persistence follow `Player.cpp:11784-12035`; failure and indeterminate
/// persistence outcomes remain errors for the caller's native-operation fence.
pub async fn destroy_quest_item_count_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    item_entry: u32,
    count: u32,
) -> Result<u32, &'static str> {
    let identity = capture_item_destruction_owner_like_cpp(host)?;
    let player_guid = identity.0;
    let inventory_authority_complete = host
        .core()
        .with_owned_player_like_cpp(|player| {
            player
                .inventory_runtime_like_cpp()
                .equipment_inventory_authority_complete_like_cpp()
        })
        .ok_or("canonical Player inventory authority is unavailable")?;
    if !inventory_authority_complete {
        return Err("canonical Player inventory authority is incomplete");
    }
    let inventory_slot_count = {
        let (state, hub) = host.split_inventory_ref();
        state.resolved_player_inventory_slot_count_like_cpp(hub)
    }
    .ok_or("canonical Player inventory slot authority is unavailable")?;
    let backpack_end = INVENTORY_SLOT_ITEM_START
        .saturating_add(inventory_slot_count)
        .min(INVENTORY_SLOT_ITEM_END);
    let mut already_removed = 0_u32;

    for section in NATIVE_DESTROY_SCAN_ORDER_LIKE_CPP {
        if let Some(slots) = direct_slots_for_section_like_cpp(section, backpack_end) {
            let bag = INVENTORY_SLOT_BAG_0;
            let full_stack_unequip_check = section
                == NativeDestroySectionLikeCpp::EquipmentAndBagList
                || section == NativeDestroySectionLikeCpp::BankBagList;
            for slot in slots {
                if let Some(return_count) = destroy_matching_item_at_like_cpp(
                    host,
                    identity,
                    player_guid,
                    item_entry,
                    count,
                    &mut already_removed,
                    bag,
                    slot,
                    full_stack_unequip_check,
                )
                .await?
                {
                    return Ok(return_count);
                }
            }
            continue;
        }

        let bag_slots = bag_slots_for_section_like_cpp(section)
            .expect("non-direct inventory sections are bag contents");
        for bag in bag_slots {
            require_item_destruction_owner_like_cpp(host, identity)?;
            let Some(bag_size) = native_bag_size_like_cpp(host, bag)? else {
                continue;
            };
            for slot in 0..bag_size {
                if let Some(return_count) = destroy_matching_item_at_like_cpp(
                    host,
                    identity,
                    player_guid,
                    item_entry,
                    count,
                    &mut already_removed,
                    bag,
                    slot,
                    false,
                )
                .await?
                {
                    return Ok(return_count);
                }
            }
        }
    }

    Ok(already_removed)
}

#[allow(clippy::too_many_arguments)]
async fn destroy_matching_item_at_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    identity: (ObjectGuid, u64),
    player_guid: ObjectGuid,
    item_entry: u32,
    requested_count: u32,
    already_removed: &mut u32,
    bag: u8,
    slot: u8,
    full_stack_unequip_check: bool,
) -> Result<Option<u32>, &'static str> {
    require_item_destruction_owner_like_cpp(host, identity)?;
    let item = {
        let (state, hub) = host.split_inventory_ref();
        state.get_inventory_item_by_pos(hub, bag, slot)
    };
    let Some(item) = item.filter(|item| item.entry_id == item_entry) else {
        return Ok(None);
    };
    let runtime_item = {
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_object_like_cpp(hub, item.guid)
    }
    .ok_or("canonical quest item object is unavailable")?;
    if runtime_item.object().entry() != item_entry {
        return Err("canonical quest item entry authority disagrees");
    }
    if runtime_item.is_in_trade() {
        return Ok(None);
    }
    let current_count = runtime_item.count();

    match native_destroy_action_like_cpp(current_count, *already_removed, requested_count) {
        NativeDestroyActionLikeCpp::FullStack => {
            if full_stack_unequip_check {
                let proto = host
                    .hub_ref()
                    .catalogs
                    .item_storage_template(item.entry_id)
                    .ok_or("quest item template is unavailable for CanUnequipItem")?;
                let source_is_not_empty_bag = {
                    let (state, hub) = host.split_inventory_ref();
                    state.direct_item_contains_items(hub, item.guid)
                };
                let unequip_result = {
                    let (state, hub) = host.split_inventory_ref();
                    state.can_unequip_inventory_item_at_like_cpp(
                        hub,
                        bag,
                        slot,
                        false,
                        Some(&runtime_item),
                        Some(&proto),
                        source_is_not_empty_bag,
                    )
                };
                if unequip_result != InventoryResult::Ok {
                    return Ok(None);
                }
            }

            preflight_full_destroy_graph_like_cpp(host, item.guid)?;

            if !destroy_inventory_full_stack_by_pos_with_expected_runtime_owner_like_cpp(
                host,
                bag,
                slot,
                item,
                Some(runtime_item),
                identity,
                "QuestFailure::DestroyItemCount",
            )
            .await
            {
                return Err("quest item full-stack destruction failed or is indeterminate");
            }
            require_item_destruction_owner_like_cpp(host, identity)?;
            *already_removed = (*already_removed).wrapping_add(current_count);
            Ok((*already_removed >= requested_count).then_some(*already_removed))
        }
        NativeDestroyActionLikeCpp::PartialStack {
            new_count,
            removed_count,
        } => {
            apply_partial_inventory_destroy_like_cpp(
                host,
                player_guid,
                bag,
                slot,
                item,
                new_count,
                removed_count,
                Some(identity),
                "QuestFailure::DestroyItemCount",
            )
            .await
            .map_err(|failure| partial_destroy_error_like_cpp(failure))?;
            *already_removed = requested_count;
            Ok(Some(requested_count))
        }
    }
}

fn preflight_full_destroy_graph_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &H,
    item_guid: ObjectGuid,
) -> Result<(), &'static str> {
    let descendants = {
        let (state, hub) = host.split_inventory_ref();
        state.represented_inventory_descendants_postorder_like_cpp(hub, item_guid)
    }
    .ok_or("canonical quest item child-slot authority is unavailable")?;

    for (_, _, child) in descendants {
        let runtime_item = {
            let (state, hub) = host.split_inventory_ref();
            state.resolved_inventory_item_object_like_cpp(hub, child.guid)
        }
        .ok_or("canonical quest item child object is unavailable")?;
        if runtime_item.object().entry() != child.entry_id {
            return Err("canonical quest item child entry authority disagrees");
        }
    }
    Ok(())
}

fn native_bag_size_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &H,
    bag_slot: u8,
) -> Result<Option<u8>, &'static str> {
    let (bag_item, bag_storage) = {
        let (state, hub) = host.split_inventory_ref();
        let bag_item = state.get_inventory_item_by_pos(hub, INVENTORY_SLOT_BAG_0, bag_slot);
        let bag_storage = hub
            .core
            .owned_inventory_access_like_cpp()
            .canonical_inventory_bag_slots_snapshot_like_cpp(bag_slot)
            .map(|(guid, size, _)| (guid, size));
        (bag_item, bag_storage)
    };
    let Some(bag_item) = bag_item else {
        if bag_storage.is_some() {
            return Err("canonical PlayerBagStorage has no matching bag item");
        }
        return Ok(None);
    };
    let runtime_item = {
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_object_like_cpp(hub, bag_item.guid)
    }
    .ok_or("canonical bag item object is unavailable")?;
    if runtime_item.object().entry() != bag_item.entry_id {
        return Err("canonical bag item entry authority disagrees");
    }
    // PlayerBagStorage is the capacity authority. The item template is only
    // checked against that canonical runtime capacity for admitted bag states.
    let template_slots = host
        .hub_ref()
        .catalogs
        .item_storage_template(bag_item.entry_id)
        .map(|template| template.container_slots);
    match (
        runtime_item.object().is_type(TypeMask::CONTAINER),
        bag_storage,
    ) {
        (true, Some((storage_guid, bag_size))) if storage_guid == bag_item.guid => {
            if template_slots != Some(bag_size) {
                return Err("canonical bag capacity disagrees with its item template");
            }
            Ok(Some(bag_size))
        }
        (true, Some(_)) => Err("canonical PlayerBagStorage belongs to a different bag"),
        (true, None) => Err("canonical bag capacity is unavailable"),
        (false, Some(_)) => Err("canonical PlayerBagStorage has a non-bag item"),
        (false, None) if template_slots.is_none() || template_slots == Some(0) => Ok(None),
        (false, None) => Err("bag item template disagrees with the canonical runtime item"),
    }
}

fn capture_item_destruction_owner_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &H,
) -> Result<(ObjectGuid, u64), &'static str> {
    let handle = host
        .core()
        .player_handle_like_cpp
        .ok_or("canonical Player owner is unavailable")?;
    let player_guid = host
        .core()
        .with_owned_player_like_cpp(|player| player.guid())
        .ok_or("canonical Player owner is unavailable")?;
    if player_guid != handle.guid() || host.player_guid() != Some(player_guid) {
        return Err("canonical Player identity is inconsistent");
    }
    Ok((player_guid, handle.generation()))
}

fn require_item_destruction_owner_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &H,
    expected: (ObjectGuid, u64),
) -> Result<(), &'static str> {
    let same_handle = host
        .core()
        .player_handle_like_cpp
        .is_some_and(|handle| handle.guid() == expected.0 && handle.generation() == expected.1);
    if !same_handle
        || host
            .core()
            .with_owned_player_like_cpp(|player| player.guid())
            != Some(expected.0)
    {
        return Err("quest item destruction Player incarnation changed");
    }
    Ok(())
}

fn partial_destroy_error_like_cpp(failure: PartialDestroyFailureLikeCpp) -> &'static str {
    match failure {
        PartialDestroyFailureLikeCpp::PersistencePortUnavailable => {
            "quest item persistence owner is unavailable"
        }
        PartialDestroyFailureLikeCpp::QuestPersistencePlanUnavailable => {
            "quest item quest-persistence authority is unavailable"
        }
        PartialDestroyFailureLikeCpp::Persistence { .. } => {
            "quest item partial persistence failed or is indeterminate"
        }
        PartialDestroyFailureLikeCpp::PlayerIncarnationChanged => {
            "quest item destruction Player incarnation changed"
        }
        PartialDestroyFailureLikeCpp::ItemCountApplicationUnavailable => {
            "quest item count could not be applied to the canonical Player"
        }
        PartialDestroyFailureLikeCpp::QuestRemovalCallbackUnavailable => {
            "quest item removal callback authority is unavailable"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_destroy_action_matches_cpp_zero_and_cumulative_cases() {
        assert_eq!(
            native_destroy_action_like_cpp(5, 0, 0),
            NativeDestroyActionLikeCpp::PartialStack {
                new_count: 5,
                removed_count: 0,
            }
        );
        assert_eq!(
            native_destroy_action_like_cpp(0, 0, 0),
            NativeDestroyActionLikeCpp::FullStack
        );
        assert_eq!(
            native_destroy_action_like_cpp(3, 2, 5),
            NativeDestroyActionLikeCpp::FullStack
        );
        assert_eq!(
            native_destroy_action_like_cpp(6, 2, 5),
            NativeDestroyActionLikeCpp::PartialStack {
                new_count: 3,
                removed_count: 3,
            }
        );
        assert_eq!(
            native_destroy_action_like_cpp(u32::MAX, 1, 0),
            NativeDestroyActionLikeCpp::FullStack
        );
    }

    #[test]
    fn direct_inventory_scan_ranges_match_cpp_order_and_bounds() {
        let backpack_end = INVENTORY_SLOT_ITEM_START + 16;
        assert_eq!(
            NATIVE_DESTROY_SCAN_ORDER_LIKE_CPP,
            [
                NativeDestroySectionLikeCpp::Backpack,
                NativeDestroySectionLikeCpp::Keyring,
                NativeDestroySectionLikeCpp::CarriedBagContents,
                NativeDestroySectionLikeCpp::EquipmentAndBagList,
                NativeDestroySectionLikeCpp::BankItems,
                NativeDestroySectionLikeCpp::BankBagContents,
                NativeDestroySectionLikeCpp::BankBagList,
                NativeDestroySectionLikeCpp::ChildEquipment,
            ]
        );
        assert_eq!(
            direct_slots_for_section_like_cpp(NativeDestroySectionLikeCpp::Backpack, backpack_end,)
                .unwrap()
                .collect::<Vec<_>>(),
            (35..51).collect::<Vec<_>>()
        );
        assert_eq!(
            direct_slots_for_section_like_cpp(NativeDestroySectionLikeCpp::Keyring, backpack_end,)
                .unwrap()
                .collect::<Vec<_>>(),
            (KEYRING_SLOT_START..KEYRING_SLOT_END).collect::<Vec<_>>()
        );
        assert_eq!(
            direct_slots_for_section_like_cpp(
                NativeDestroySectionLikeCpp::EquipmentAndBagList,
                backpack_end,
            )
            .unwrap()
            .collect::<Vec<_>>(),
            (0..34).collect::<Vec<_>>()
        );
        assert_eq!(
            direct_slots_for_section_like_cpp(
                NativeDestroySectionLikeCpp::BankItems,
                backpack_end,
            )
            .unwrap()
            .collect::<Vec<_>>(),
            (59..87).collect::<Vec<_>>()
        );
        assert_eq!(
            direct_slots_for_section_like_cpp(
                NativeDestroySectionLikeCpp::BankBagList,
                backpack_end,
            )
            .unwrap()
            .collect::<Vec<_>>(),
            (87..94).collect::<Vec<_>>()
        );
        assert_eq!(
            direct_slots_for_section_like_cpp(
                NativeDestroySectionLikeCpp::ChildEquipment,
                backpack_end,
            )
            .unwrap()
            .collect::<Vec<_>>(),
            (138..141).collect::<Vec<_>>()
        );
        assert_eq!(
            bag_slots_for_section_like_cpp(NativeDestroySectionLikeCpp::CarriedBagContents)
                .unwrap()
                .collect::<Vec<_>>(),
            (30..34).collect::<Vec<_>>()
        );
        assert_eq!(
            bag_slots_for_section_like_cpp(NativeDestroySectionLikeCpp::BankBagContents)
                .unwrap()
                .collect::<Vec<_>>(),
            (87..94).collect::<Vec<_>>()
        );
    }
}
