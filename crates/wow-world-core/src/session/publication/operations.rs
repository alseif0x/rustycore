use crate::entity_update_bridge::player_values_update_to_update_object;
use crate::session::mailbox::SessionCommand;
use crate::session::set_active_player_update_bit_like_cpp;
use crate::session::state::SessionCore;
use tracing::{info, trace, warn};
use wow_constants::{BuyResult, ItemModifier, SellResult};
use wow_core::ObjectGuid;
use wow_entities::Player;
use wow_entities::PlayerVoidStorageItemLikeCpp as RepresentedVoidStorageItemLikeCpp;
use wow_packet::packets::chat::{ChatMsg, ChatPkt, PrintNotification};
use wow_packet::packets::misc::{BuyFailed, SellResponse};

impl SessionCore {
    pub fn represented_void_storage_item_packet_like_cpp(
        &self,
        slot: u8,
        item: &RepresentedVoidStorageItemLikeCpp,
    ) -> wow_packet::packets::void_storage::VoidItem {
        let modifications = (item.fixed_scaling_level != 0)
            .then(|| {
                wow_packet::packets::item::ItemMod::new(
                    item.fixed_scaling_level as i32,
                    ItemModifier::TimewalkerLevel as u8,
                )
            })
            .into_iter()
            .collect();
        wow_packet::packets::void_storage::VoidItem {
            guid: ObjectGuid::create_item(self.realm_id, item.item_id as i64),
            creator: item.creator_guid,
            slot: u32::from(slot),
            item: wow_packet::packets::item::ItemInstance {
                item_id: item.item_entry as i32,
                // C++ `ItemInstance::Initialize(VoidStorageItem const*)`
                // intentionally initializes only ItemID and the optional
                // TimewalkerLevel modifier. Random properties and ItemBonus
                // remain their protocol defaults on void-storage packets.
                modifications: wow_packet::packets::item::ItemModList {
                    values: modifications,
                },
                ..Default::default()
            },
        }
    }

    pub fn send_player_values_update_like_cpp(&self, update: &wow_entities::PlayerValuesUpdate) {
        let Some(guid) = self.player_guid() else {
            return;
        };
        if let Some(packet) =
            player_values_update_to_update_object(guid, self.player_map_id_like_cpp(), update)
        {
            self.send_packet(&packet);
        }
    }

    /// Get a clone of the send channel.
    pub fn send_tx(&self) -> &flume::Sender<Vec<u8>> {
        self.transport.connection.send_tx()
    }

    /// Send a TimeSyncRequest and schedule the next one.
    pub fn send_time_sync(&mut self) {
        use wow_packet::packets::misc::TimeSyncRequest;
        let sequence_index = self.driver.time_synchronization.next_counter;
        self.send_packet(&TimeSyncRequest { sequence_index });
        trace!(
            "Sent TimeSyncRequest(seq={}) for account {}",
            sequence_index, self.account_id
        );
        self.driver
            .time_synchronization
            .pending_requests
            .insert(sequence_index, crate::session::game_time_ms_like_cpp());
        // C++ uses 5s for the first request, then 10s.
        self.driver.time_synchronization.timer_ms =
            if self.driver.time_synchronization.next_counter == 0 {
                5000
            } else {
                10000
            };
        self.driver.time_synchronization.next_counter += 1;
    }

    /// Send a server packet back to the client via the instance (default) channel.
    /// Enqueue one packet, reporting whether it was accepted.
    ///
    /// The channel can be closed, and swallowing that made every caller unable
    /// to tell a delivered packet from a discarded one. Callers that record a
    /// publication need the difference; the rest ignore the value as before.
    pub fn send_packet<P: wow_packet::ServerPacket>(&self, pkt: &P) -> bool {
        let data = pkt.to_bytes();
        if std::env::var_os("RUSTYCORE_LOGIN_TRACE").is_some() {
            info!(
                account = self.account_id,
                opcode = ?P::OPCODE,
                bytes = data.len(),
                "RUST_LOGIN_TRACE send_packet"
            );
        }
        if self.send_tx().send(data).is_err() {
            warn!("Send channel closed for account {}", self.account_id);
            return false;
        }
        true
    }

    /// Attempts to enqueue one instance-channel packet without waiting for
    /// socket-writer capacity.
    ///
    /// This is deliberately narrow rather than a replacement for the normal
    /// packet API. Object-owned loot uses it while holding its short authority
    /// mutex so a stalled client cannot block every concurrent claim/release.
    pub fn try_send_packet<P: wow_packet::ServerPacket>(&self, pkt: &P) -> bool {
        let data = pkt.to_bytes();
        if std::env::var_os("RUSTYCORE_LOGIN_TRACE").is_some() {
            info!(
                account = self.account_id,
                opcode = ?P::OPCODE,
                bytes = data.len(),
                "RUST_LOGIN_TRACE try_send_packet"
            );
        }
        match self.send_tx().try_send(data) {
            Ok(()) => true,
            Err(flume::TrySendError::Full(_)) => {
                warn!(
                    "Send channel full for account {}; packet rejected without blocking",
                    self.account_id
                );
                false
            }
            Err(flume::TrySendError::Disconnected(_)) => {
                warn!("Send channel closed for account {}", self.account_id);
                false
            }
        }
    }

    pub fn send_system_message_like_cpp(&self, text: &str) {
        for line in text.split('\n') {
            self.send_packet(&ChatPkt {
                msg_type: ChatMsg::System,
                language: 0,
                sender_guid: ObjectGuid::EMPTY,
                sender_name: String::new(),
                target_guid: ObjectGuid::EMPTY,
                target_name: String::new(),
                prefix: String::new(),
                channel: String::new(),
                text: line.to_string(),
                virtual_realm: self.virtual_realm_address(),
            });
        }
    }

    pub fn send_notification_like_cpp(&self, text: String) {
        self.send_packet(&PrintNotification { notify_text: text });
    }

    /// C++ `Player::SendUpdateWorldState(variable, value, hidden)` direct-session send.
    ///
    /// Mirrors `SendDirectMessage(worldstate.Write())`: constructs one
    /// `SMSG_UPDATE_WORLD_STATE` packet and sends it only through this session's
    /// outbound channel. GameEvent fanout, login initialization, and global
    /// `WorldStateMgr` ownership are intentionally out of scope for this seam.
    pub fn send_update_world_state_like_cpp(&self, variable_id: u32, value: i32, hidden: bool) {
        let packet = wow_packet::packets::misc::UpdateWorldState {
            variable_id,
            value,
            hidden,
        };
        self.send_packet(&packet);
    }

    /// Send pre-serialized packet bytes to the client.
    ///
    /// Used for packets with dynamic opcodes (e.g. `SetSpellModifier`
    /// which uses the same struct for Flat and Pct variants).
    pub fn send_raw_packet(&self, data: &[u8]) {
        if std::env::var_os("RUSTYCORE_LOGIN_TRACE").is_some() {
            let opcode_text = data
                .get(0..2)
                .map(|bytes| format!("0x{:04X}", u16::from_le_bytes([bytes[0], bytes[1]])))
                .unwrap_or_else(|| "<short>".to_string());
            info!(
                account = self.account_id,
                opcode = opcode_text.as_str(),
                bytes = data.len(),
                "RUST_LOGIN_TRACE send_raw_packet"
            );
        }
        if self.send_tx().send(data.to_vec()).is_err() {
            warn!("Send channel closed for account {}", self.account_id);
        }
    }

    pub fn send_buy_error(&self, result: BuyResult, creature_guid: Option<ObjectGuid>, item: u32) {
        self.send_packet(&BuyFailed {
            vendor_guid: creature_guid.unwrap_or(ObjectGuid::EMPTY),
            muid: item as i32,
            reason: result,
        });
    }

    pub fn send_sell_error(
        &self,
        result: SellResult,
        creature_guid: Option<ObjectGuid>,
        item_guid: ObjectGuid,
    ) {
        self.send_packet(&SellResponse::error(
            creature_guid.unwrap_or(ObjectGuid::EMPTY),
            item_guid,
            result,
        ));
    }

    pub fn try_send_connected_player_command_like_cpp(
        &self,
        target_guid: ObjectGuid,
        command: SessionCommand,
    ) {
        if let Some(address) = self
            .player_registry()
            .and_then(|registry| registry.control_address(target_guid))
        {
            let _ = address.try_send(command);
        }
    }
}

impl crate::session::HubMut<'_> {
    pub fn send_represented_mount_unit_update_like_cpp(&mut self, display_id: i32) {
        let (presentation, control) = self.aura_removal_mount_accesses_like_cpp();
        control.send_represented_mount_unit_update_like_cpp(&presentation, display_id);
    }
}

impl crate::session::HubRef<'_> {
    pub fn send_represented_rest_info_update_like_cpp(&self, nested_mask: u8) {
        let Some(guid) = self.core.player_guid() else {
            return;
        };
        let mut player = Player::new(None, false);
        let Some(rest_threshold) = self.resolved_xp_rest_threshold_like_cpp() else {
            return;
        };
        let Some(rest_state) = self.resolved_xp_rest_state_like_cpp() else {
            return;
        };
        player.prepare_rest_info_values_update_like_cpp(0, rest_threshold, rest_state, nested_mask);
        let update = player.values_update(true);
        if let Some(packet) =
            player_values_update_to_update_object(guid, self.core.player_map_id_like_cpp(), &update)
        {
            self.core.send_packet(&packet);
        }
    }

    pub fn send_active_player_multi_action_bars_update_like_cpp(&self, guid: ObjectGuid) {
        let Some((_, _, multi_action_bars)) = self.active_player_update_state_like_cpp() else {
            return;
        };
        use wow_packet::packets::update::{ActivePlayerDataValuesUpdate, UpdateObject};

        let mut data = ActivePlayerDataValuesUpdate::default();
        set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 70);
        set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 72);
        data.multi_action_bars = multi_action_bars;
        self.core
            .send_packet(&UpdateObject::full_active_player_values_update(
                guid,
                self.core.player_map_id_like_cpp(),
                data,
            ));
    }
}
