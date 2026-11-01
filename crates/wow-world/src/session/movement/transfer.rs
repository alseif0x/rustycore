//! The public teleport entry point. The teleport and far-transfer bodies moved
//! to `wow_world_entities::teleport_session` (#1363 slice 11a).

use super::*;

impl WorldSession {
    /// Teleport the player to a new map and position.
    ///
    /// Sends SMSG_TRANSFER_PENDING (0x25cd) to initiate the transfer.
    /// The client will respond with CMSG_WORLD_PORT_ACK when ready.
    ///
    /// C++ `Player::TeleportTo` → `SendTransferPending`.
    pub async fn teleport_to(&mut self, new_map: u32, new_pos: wow_core::Position) {
        wow_world_entities::teleport_to_with_options(self, new_map, new_pos, TELE_TO_NONE_LIKE_CPP)
            .await;
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/movement/transfer/f3_shims.rs"]
mod f3_shims;
