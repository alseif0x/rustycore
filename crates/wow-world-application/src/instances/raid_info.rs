// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_packet::packets::instance::{InstanceInfo, InstanceLockInfo};

use crate::instances::InstanceRaidInfoHandlerCxLikeCpp;

pub async fn handle_request_raid_info_like_cpp(
    cx: &mut InstanceRaidInfoHandlerCxLikeCpp<'_>,
    _pkt: wow_packet::WorldPacket,
) {
    let locks = match cx.player.player_guid_like_cpp() {
        Some(player_guid) => cx
            .locks
            .raid_info_locks_like_cpp(
                player_guid,
                cx.map_store,
                cx.map_difficulty_store,
            )
            .unwrap_or_default(),
        None => Vec::new(),
    };

    cx.packets.send_packet_realm(&InstanceInfo {
        locks: locks
            .into_iter()
            .map(|lock| InstanceLockInfo {
                instance_id: lock.instance_id,
                map_id: lock.map_id,
                difficulty_id: lock.difficulty_id,
                time_remaining: lock.time_remaining,
                completed_mask: lock.completed_mask,
                locked: lock.locked,
                extended: lock.extended,
            })
            .collect(),
    });
}
