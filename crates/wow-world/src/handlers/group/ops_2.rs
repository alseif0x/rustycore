//! Group handlers operations, part 2 of 3.
//!
//! The inherent `WorldSession` impl is divided by responsibility under
//! #662; every method keeps its original body.

use super::*;

impl WorldSession {
    /// CMSG_CLEAR_RAID_MARKER.
    ///
    /// C++ `WorldSession::HandleClearRaidMarker` resolves the player's current
    /// HOME group, gates raid groups to leader/assistant, then calls
    /// `Group::DeleteRaidMarker`. Marker id `8` is the C++ "clear all" sentinel.
    pub async fn handle_clear_raid_marker(&mut self, mut pkt: wow_packet::WorldPacket) {
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
    /// CMSG_OPT_OUT_OF_LOOT — toggle automatic pass on group-loot rolls.
    pub async fn handle_opt_out_of_loot(&mut self, mut pkt: wow_packet::WorldPacket) {
        let opt_out = match OptOutOfLoot::read(&mut pkt) {
            Ok(opt_out) => opt_out,
            Err(e) => {
                warn!("Bad OptOutOfLoot: {e}");
                return;
            }
        };

        if self.player_guid().is_none() {
            if opt_out.pass_on_loot {
                warn!("CMSG_OPT_OUT_OF_LOOT value<>0 for not-loaded character");
            }
            return;
        }

        let _ = self.set_pass_on_group_loot_like_cpp(opt_out.pass_on_loot);
    }

    pub async fn handle_random_roll(&mut self, mut pkt: wow_packet::WorldPacket) {
        let roll = match RandomRollClient::read(&mut pkt) {
            Ok(roll) => roll,
            Err(e) => {
                warn!("Bad RandomRoll: {e}");
                return;
            }
        };

        if roll.min > roll.max || roll.max > 1_000_000 {
            return;
        }

        let Some(sender_guid) = self.player_guid() else {
            return;
        };

        let result = rand::thread_rng().gen_range(roll.min..=roll.max);
        let response = RandomRoll {
            roller: sender_guid,
            roller_wow_account: ObjectGuid::new(
                (HighGuid::WowAccount as i64) << 58,
                i64::from(self.core.account_id),
            ),
            min: roll.min,
            max: roll.max,
            result,
        };
        let bytes = response.to_bytes();

        let Some(group_reg) = self.group_registry().map(std::sync::Arc::clone) else {
            self.send_packet(&response);
            return;
        };

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            None,
        ) else {
            self.send_packet(&response);
            return;
        };

        let Some(group) = group_reg.get(&group_guid) else {
            self.send_packet(&response);
            return;
        };

        let Some(registry) = self.player_registry().map(std::sync::Arc::clone) else {
            self.send_packet(&response);
            return;
        };

        let mut sent_to_sender = false;
        // C++ `group->BroadcastPacket(randomRoll.Write(), false)` includes the roller.
        for member_guid in &group.members {
            if let Some(member) = registry.group_presence(*member_guid) {
                let _ = registry.send_current_packet(member.registration, bytes.clone());
                if *member_guid == sender_guid {
                    sent_to_sender = true;
                }
            }
        }

        if !sent_to_sender {
            self.send_packet(&response);
        }
    }
}
