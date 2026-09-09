//! Group handlers operations, part 3 of 3.
//!
//! The inherent `WorldSession` impl is divided by responsibility under
//! #662; every method keeps its original body.

use super::*;

impl WorldSession {
    /// CMSG_MINIMAP_PING — broadcasts minimap ping to group members excluding sender.
    ///
    /// C++ anchor: `WorldSession::HandleMinimapPingOpcode`
    /// (`GroupHandler.cpp:401-412`)
    ///
    /// Handler reads `MinimapPingClient`, resolves group via `GroupRegistry`
    /// (finds group containing sender_guid), builds `MinimapPing` server packet
    /// with `Sender`, `PositionX`, `PositionY`, and sends to all connected
    /// group members except sender via `PlayerRegistry` send_tx.
    ///
    /// Boundary: `PartyIndex` is parsed as `Option<u8>` but only used as a
    /// represented semantic boundary; the implementation finds the group
    /// containing the sender_guid in `GroupRegistry` (same pattern as other
    /// group handlers). Multi-group/raid-subgroup PartyIndex selection is not
    /// fully modelled.
    pub async fn handle_minimap_ping(&mut self, mut pkt: wow_packet::WorldPacket) {
        let ping = match MinimapPingClient::read(&mut pkt) {
            Ok(ping) => ping,
            Err(e) => {
                warn!("Bad MinimapPing: {e}");
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
            ping.party_index,
        ) else {
            return;
        };

        let Some(group) = group_reg.get(&group_guid) else {
            return;
        };

        let bytes = MinimapPing {
            sender: sender_guid,
            position_x: ping.position_x,
            position_y: ping.position_y,
        }
        .to_bytes();

        // C++ BroadcastPacket(packet, true, -1, GetPlayer()->GetGUID()) excludes sender.
        for member_guid in &group.members {
            if *member_guid == sender_guid {
                continue;
            }
            if let Some(member) = registry.group_presence(*member_guid) {
                let _ = registry.send_current_packet(member.registration, bytes.clone());
            }
        }
    }
    /// CMSG_RANDOM_ROLL — validates roll bounds and broadcasts the result.
    ///
    /// C++ anchors:
    /// - `WorldSession::HandleRandomRollOpcode` (`GroupHandler.cpp:414-421`)
    /// - `Player::DoRandomRoll` (`Player.cpp:28718-28734`)
    ///
    /// The client packet contains an optional `PartyIndex`, but C++ ignores it
    /// here: the handler only validates `Min`/`Max` and calls
    /// `GetPlayer()->DoRandomRoll(packet.Min, packet.Max)`, whose group lookup
    /// is `GetGroup()` without a party index. Rust preserves that HOME-group
    /// represented behavior by resolving with `party_index=None`.
    ///
    /// Boundary: C++ reads `int32` but passes the values to a `uint32`
    /// `DoRandomRoll`. Negative ranges therefore hit a signed/unsigned edge in
    /// legacy. Rust keeps the explicit C++ gates and produces a signed result
    /// for the received signed range instead of silently wrapping negatives.
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
                i64::from(self.account_id),
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
