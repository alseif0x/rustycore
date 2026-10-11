// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `ops_2` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    /// CMSG_CLEAR_RAID_MARKER.
    ///
    /// C++ `WorldSession::HandleClearRaidMarker` resolves the player's current
    /// HOME group, gates raid groups to leader/assistant, then calls
    /// `Group::DeleteRaidMarker`. Marker id `8` is the C++ "clear all" sentinel.
    pub(crate) async fn handle_clear_raid_marker(&mut self, mut pkt: wow_packet::WorldPacket) {
        let clear = match ClearRaidMarker::read(&mut pkt) {
            Ok(clear) => clear,
            Err(e) => {
                warn!("Bad ClearRaidMarker: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            None,
        ) else {
            return;
        };

        let outcome = match group_reg.delete_raid_marker_transition_like_cpp(
            group_guid,
            sender_guid,
            clear.marker_id,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        let bytes = raid_markers_changed_like_cpp(&outcome.group);
        let recipients = connected_group_member_txs_like_cpp(&outcome.group, &registry);

        send_group_packet_bytes_like_cpp(&registry, bytes, &recipients);
    }
}
