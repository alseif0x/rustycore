// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `Player::SendNewItem` (#1363 slice 12b7), moved from the World
//! `session/player_items/{persistence,publication}.rs`: the item push result
//! goes to the group (`Group::BroadcastPacket`) when the plan asks for it and
//! otherwise to the player on the realm connection. The group guid is read
//! through [`WorldEntitiesHostLikeCpp`].

use wow_entities::{SendNewItemDelivery, SendNewItemPlan};
use wow_world_entities::WorldEntitiesHostLikeCpp;

pub fn send_new_item_plan<H: WorldEntitiesHostLikeCpp>(host: &H, plan: &SendNewItemPlan) {
    let packet = crate::item_push_result_from_send_new_item_plan(plan);
    if plan.delivery == SendNewItemDelivery::GroupBroadcast {
        use wow_packet::ServerPacket;

        if broadcast_item_push_result_to_group(host, packet.to_bytes()) {
            return;
        }
    }

    // C++ `Player::SendNewItem` uses `SendDirectMessage`; opcode routing
    // places `SMSG_ITEM_PUSH_RESULT` on CONNECTION_TYPE_REALM even while
    // the inventory object updates remain on the instance connection.
    host.core().send_packet_realm(&packet);
}

fn broadcast_item_push_result_to_group<H: WorldEntitiesHostLikeCpp>(
    host: &H,
    bytes: Vec<u8>,
) -> bool {
    let (Some(group_guid), Some(group_registry), Some(player_registry)) = (
        host.resolved_group_guid_like_cpp(),
        &host.core().directory.group_registry,
        &host.core().player_registry,
    ) else {
        return false;
    };

    let Some(group) = group_registry.get(&group_guid) else {
        return false;
    };

    let mut delivered = false;
    for member_guid in &group.members {
        if let Some(member) = player_registry.loot_presence(*member_guid) {
            delivered |= player_registry
                .send_current_realm_packet(member.registration, bytes.clone())
                .is_ok();
        }
    }

    delivered
}
