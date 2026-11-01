// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! CMSG_ADD_TOY — learn a Toy.db2 item and consume the inventory item.
//!
//! C++ ref: `WorldSession::HandleAddToy` (`ToyHandler.cpp:28`) validates the
//! item guid, checks `sDB2Manager.IsToyItem(item->GetEntry())`, calls
//! `CollectionMgr::AddToy(item->GetEntry(), false, false)`, which inserts
//! the account row and calls `Player::AddToy`, then destroys the item only
//! when the account toy was newly inserted.
//!
//! Moved verbatim from the World `handlers/collections/mod.rs` (#1363 slice
//! 4d).

use tracing::{info, warn};
use wow_constants::InventoryResult;
use wow_entities::{Item, PlayerInventoryItem as InventoryItem};
use wow_packet::ClientPacket;
use wow_packet::packets::item::InventoryChangeFailure;
use wow_packet::packets::misc::AddToy;
use wow_world_core::entity_update_bridge::player_values_update_to_update_object;

use crate::ItemSessionHostLikeCpp;

/// The World operation the add-toy body still calls.
pub trait AddToyHostLikeCpp: ItemSessionHostLikeCpp {
    /// `WorldSession::can_use_inventory_item_represented_like_cpp`.
    fn can_use_inventory_item_represented_like_cpp(
        &self,
        item: &InventoryItem,
        runtime_item: Option<&Item>,
    ) -> InventoryResult;
}

pub async fn handle_add_toy<H: AddToyHostLikeCpp>(host: &mut H, mut pkt: wow_packet::WorldPacket) {
    let request = match AddToy::read(&mut pkt) {
        Ok(request) => request,
        Err(error) => {
            warn!(
                account = host.core().account_id,
                "AddToy parse failed: {error}"
            );
            return;
        }
    };

    if request.item_guid == wow_core::ObjectGuid::EMPTY {
        return;
    }

    let Some((bag, slot, item)) = ({
        let (state, hub) = host.split_inventory_ref();
        state.get_inventory_item_by_guid_like_cpp(hub, request.item_guid)
    }) else {
        host.core()
            .send_packet_realm(&InventoryChangeFailure::error(
                InventoryResult::ItemNotFound,
            ));
        return;
    };

    if !host.hub_ref().catalogs.is_toy_item_like_cpp(item.entry_id) {
        return;
    }

    let runtime_item = {
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_objects_like_cpp(hub)
    }
    .and_then(|items| items.get(&item.guid).cloned());
    let can_use_result =
        host.can_use_inventory_item_represented_like_cpp(&item, runtime_item.as_ref());
    if can_use_result != InventoryResult::Ok {
        host.core()
            .send_equip_error(can_use_result, Some(item.guid), None, 0, 0);
        return;
    }

    if !{
        let (state, mut hub) = host.split_lifecycle_mut();
        state.add_account_toy_like_cpp(&mut hub, item.entry_id, false, false)
    } {
        return;
    }

    let destroyed_entry_id = item.entry_id;
    if crate::destroy_inventory_full_stack_by_pos_like_cpp(
        host,
        bag,
        slot,
        item,
        runtime_item,
        "AddToy",
    )
    .await
    {
        if let Some(update) = host
            .hub_mut()
            .add_player_toy_dynamic_field_like_cpp(destroyed_entry_id)
        {
            if let Some(guid) = host.player_guid() {
                if let Some(packet) = player_values_update_to_update_object(
                    guid,
                    host.core().player_map_id_like_cpp(),
                    &update,
                ) {
                    host.send_packet(&packet);
                }
            }
        }
        info!(
            "Added toy item={} from bag {} slot {} for account {}",
            destroyed_entry_id,
            bag,
            slot,
            host.core().account_id
        );
    } else {
        host.hub_mut()
            .remove_account_toy_like_cpp(destroyed_entry_id);
    }
}
