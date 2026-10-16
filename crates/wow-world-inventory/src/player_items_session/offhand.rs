// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented automatic offhand unequip (#1363 slice 12b1; moved from the
//! World `session/player_items/{offhand,storage,storage_slots,items,persistence}.rs`):
//! C++ `Player::AutoUnequipOffhandIfNeed` with its direct move to a free
//! position, its item-set and item-mod removal and its mail fallback.

use wow_constants::InventoryResult;
use wow_core::ObjectGuid;
use wow_entities::{
    EQUIPMENT_SLOT_OFFHAND, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, ItemPosCount, NULL_BAG,
    NULL_SLOT,
};
use wow_world_core::session::HubMut;

use super::item_mods::{
    record_represented_item_mods_like_cpp, record_represented_items_set_item_like_cpp,
    send_represented_item_bonus_player_stat_update_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use super::world_test_consumer_like_cpp;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedAutoUnequipOffhandLikeCpp;
use crate::{ItemSessionHostLikeCpp, is_represented_bag_slot};

pub fn represented_auto_unequip_offhand_if_need_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    force: bool,
) -> bool {
    #[cfg(any(test, feature = "test-fixtures"))]
    let world_test = world_test_consumer_like_cpp(host);
    let Some(offhand_item) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_like_cpp(hub, EQUIPMENT_SLOT_OFFHAND)
    }) else {
        return false;
    };

    let Some(reason) = ({
        let (s, h) = host.split_inventory_ref();
        s.represented_auto_unequip_offhand_reason_like_cpp(h, force)
    }) else {
        return false;
    };
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = reason;

    ({
        let (state, mut hub) = host.split_inventory_mut();
        state.remove_inventory_item_duration_refs_like_cpp(&mut hub, offhand_item.guid)
    });
    {
        let (s, mut h) = host.split_inventory_mut();
        s.clear_represented_offhand_equipped_flag_like_cpp(&mut h, offhand_item.guid)
    };
    ({
        let (state, mut hub) = host.split_inventory_mut();
        state.remove_inventory_tradeable_item_like_cpp(&mut hub, offhand_item.guid)
    });
    let _item_set_changed = record_direct_inventory_item_set_remove_like_cpp(
        host,
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_OFFHAND,
        offhand_item.guid,
    );
    let item_mods_changed =
        record_represented_offhand_item_mod_remove_like_cpp(host, offhand_item.guid);
    host.split_inventory_mut()
        .0
        .record_inventory_item_combat_stat_recalculations_like_cpp(EQUIPMENT_SLOT_OFFHAND);

    #[cfg(any(test, feature = "test-fixtures"))]
    let mut stored_destination = None;
    let mut needs_mail_fallback = true;
    if let Some((InventoryResult::Ok, destinations, _)) =
        plan_store_existing_direct_inventory_item_like_cpp(host, EQUIPMENT_SLOT_OFFHAND)
        && let Some(destination) = destinations.first()
    {
        let [bag, slot] = destination.pos.to_be_bytes();
        if move_represented_direct_inventory_item_to_pos_like_cpp(
            host,
            EQUIPMENT_SLOT_OFFHAND,
            bag,
            slot,
        ) {
            {
                let (s, mut h) = host.split_inventory_mut();
                s.sync_canonical_direct_inventory_move_like_cpp(
                    &mut h,
                    EQUIPMENT_SLOT_OFFHAND,
                    bag,
                    slot,
                    offhand_item.guid,
                )
            };
            {
                let (s, h) = host.split_inventory_ref();
                s.send_auto_unequip_offhand_values_update_like_cpp(
                    h,
                    Some((bag, slot)),
                    offhand_item.guid,
                )
            };
            {
                let (s, h) = host.split_inventory_ref();
                s.send_item_contained_in_values_update_like_cpp(h, offhand_item.guid)
            };
            if bag != INVENTORY_SLOT_BAG_0 {
                ({
                    let (state, hub) = host.split_inventory_ref();
                    state.send_bag_slot_values_update_like_cpp(hub, bag, slot)
                });
            }
            if item_mods_changed {
                send_represented_item_bonus_player_stat_update_like_cpp(host);
            }
            #[cfg(any(test, feature = "test-fixtures"))]
            {
                stored_destination = Some((bag, slot));
            }
            needs_mail_fallback = false;
        }
    }
    if needs_mail_fallback {
        ({
            let (state, mut hub) = host.split_inventory_mut();
            state.remove_inventory_item_like_cpp(&mut hub, EQUIPMENT_SLOT_OFFHAND)
        });
        {
            let (s, mut h) = host.split_inventory_mut();
            s.sync_canonical_direct_inventory_remove_like_cpp(&mut h, EQUIPMENT_SLOT_OFFHAND)
        };
        {
            let (s, h) = host.split_inventory_ref();
            s.send_auto_unequip_offhand_values_update_like_cpp(h, None, offhand_item.guid)
        };
        let _ = {
            let (state, mut hub) = host.split_inventory_mut();
            state.apply_inventory_item_object_updates_like_cpp(
                &mut hub,
                offhand_item.guid,
                &[
                    wow_entities::ItemObjectUpdateLikeCpp::SetContainedIn(ObjectGuid::EMPTY),
                    wow_entities::ItemObjectUpdateLikeCpp::SetContainerGuid(ObjectGuid::EMPTY),
                    wow_entities::ItemObjectUpdateLikeCpp::SetSlot(NULL_SLOT),
                ],
            )
        };
        {
            let (s, h) = host.split_inventory_ref();
            s.send_item_contained_in_values_update_like_cpp(h, offhand_item.guid)
        };
        if item_mods_changed {
            send_represented_item_bonus_player_stat_update_like_cpp(host);
        }
    }

    host.record_represented_titan_grip_penalty_action_like_cpp();
    crate::item_session::record_represented_avg_equipped_item_level_update_like_cpp(host);

    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test {
        host.split_inventory_mut()
            .0
            .record_represented_auto_unequip_offhand_request_like_cpp(
                RepresentedAutoUnequipOffhandLikeCpp {
                    item_guid: offhand_item.guid,
                    item_entry: offhand_item.entry_id,
                    reason,
                    stored_destination,
                    needs_mail_fallback,
                },
            );
    }
    true
}

fn move_represented_direct_inventory_item_to_pos_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    src: u8,
    dst_bag: u8,
    dst_slot: u8,
) -> bool {
    if dst_bag == INVENTORY_SLOT_BAG_0 {
        return move_represented_direct_inventory_item_like_cpp(host, src, dst_slot);
    }
    if !is_represented_bag_slot(dst_bag)
        || ({
            let (state, hub) = host.split_inventory_ref();
            state.get_inventory_item_by_pos(hub, dst_bag, dst_slot)
        })
        .is_some()
    {
        return false;
    }

    let Some(src_item) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_like_cpp(hub, src)
    }) else {
        return false;
    };
    let Some(bag_item) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_like_cpp(hub, dst_bag)
    }) else {
        return false;
    };
    let Some(item_objects) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_objects_like_cpp(hub)
    }) else {
        return false;
    };
    if !item_objects.contains_key(&src_item.guid) || !item_objects.contains_key(&bag_item.guid) {
        return false;
    }

    ({
        let (state, mut hub) = host.split_inventory_mut();
        state.remove_inventory_item_like_cpp(&mut hub, src)
    });
    {
        let (state, mut hub) = host.split_inventory_mut();
        state.apply_inventory_item_object_updates_like_cpp(
            &mut hub,
            src_item.guid,
            &[
                wow_entities::ItemObjectUpdateLikeCpp::SetContainedIn(bag_item.guid),
                wow_entities::ItemObjectUpdateLikeCpp::SetSlot(dst_slot),
                wow_entities::ItemObjectUpdateLikeCpp::SetContainerGuidAndSlot(
                    bag_item.guid,
                    dst_bag,
                ),
            ],
        )
    }
}

fn move_represented_direct_inventory_item_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    src: u8,
    dst: u8,
) -> bool {
    let (inventory, hub) = host.split_inventory_mut();
    let HubMut { core, .. } = hub;
    let access = core.owned_inventory_access_like_cpp();
    inventory.move_direct_inventory_item_with_access_like_cpp(&access, src, dst)
}

pub fn record_direct_inventory_item_set_remove_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    bag: u8,
    slot: u8,
    item_guid: ObjectGuid,
) -> bool {
    if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END {
        return false;
    }

    record_represented_items_set_item_like_cpp(host, item_guid, false)
}

fn record_represented_offhand_item_mod_remove_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    item_guid: ObjectGuid,
) -> bool {
    if ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_object_like_cpp(hub, item_guid)
    })
    .is_some_and(|item| item.is_broken())
    {
        return false;
    }

    record_represented_item_mods_like_cpp(host, item_guid, EQUIPMENT_SLOT_OFFHAND, false) != 0
}

fn plan_store_existing_direct_inventory_item_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &H,
    source_slot: u8,
) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
    plan_store_existing_inventory_item_like_cpp(host, INVENTORY_SLOT_BAG_0, source_slot)
}

fn plan_store_existing_inventory_item_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &H,
    source_bag: u8,
    source_slot: u8,
) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
    host.plan_store_existing_inventory_item_at_like_cpp(
        source_bag,
        source_slot,
        NULL_BAG,
        NULL_SLOT,
        false,
    )
}
