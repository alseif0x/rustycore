//! Packets and values updates published from the Session boundary.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn represented_void_storage_item_packet_like_cpp(
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
    pub(crate) fn represented_unit_values_update_to_update_object_like_cpp(
        &self,
        unit_guid: ObjectGuid,
        map_id: u16,
        values_update: &wow_entities::UnitValuesUpdate,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        let packet_update = unit_values_update_to_packet(values_update)?;
        Some(
            self.represented_unit_packet_update_to_update_object_like_cpp(
                unit_guid,
                map_id,
                packet_update,
            ),
        )
    }
    pub(crate) fn represented_unit_packet_update_to_update_object_like_cpp(
        &self,
        unit_guid: ObjectGuid,
        map_id: u16,
        mut packet_update: wow_packet::packets::update::UnitDataValuesDeltaUpdate,
    ) -> wow_packet::packets::update::UpdateObject {
        if unit_guid.is_any_type_creature()
            && (packet_update.npc_flags[0] & UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32) != 0
        {
            let npc_flags = u64::from(packet_update.npc_flags[0])
                | (u64::from(packet_update.npc_flags[1]) << 32);
            let filtered =
                self.represented_viewer_dependent_creature_npc_flags_like_cpp(unit_guid, npc_flags);
            packet_update.npc_flags[0] = filtered as u32;
            packet_update.npc_flags[1] = (filtered >> 32) as u32;
        }

        wow_packet::packets::update::UpdateObject::unit_values_update(
            unit_guid,
            map_id,
            packet_update,
        )
    }
    /// C++ `Player::SendCurrencies`.
    pub(crate) fn setup_currencies_packet_like_cpp(&self) -> Option<SetupCurrency> {
        let store = self.currency_types_store.as_ref()?;
        let currencies = self.player_currencies_like_cpp()?;

        let player_team = player_team_for_race_cpp(self.player_race_like_cpp());
        let mut records = Vec::with_capacity(currencies.len());
        for (&currency_id, currency) in &currencies {
            let Some(entry) = store.get(currency_id).copied() else {
                continue;
            };

            if (entry.is_alliance() && player_team != Team::Alliance)
                || (entry.is_horde() && player_team != Team::Horde)
            {
                continue;
            }

            if entry.award_condition_id != 0 {
                if let Some(condition) = self
                    .player_condition_store
                    .as_ref()
                    .and_then(|store| store.get(entry.award_condition_id as u32))
                {
                    let Some(context) = self.represented_player_condition_context_like_cpp() else {
                        continue;
                    };
                    if !context.as_context(self).is_some_and(|context| {
                        is_player_meeting_condition_like_cpp(condition, &context)
                    }) {
                        continue;
                    }
                }
            }

            let scaler = entry.scaler().max(1) as u32;
            let max_quantity = currency_max_quantity_cpp(&entry, currency);
            records.push(SetupCurrencyRecord {
                type_id: entry.id as i32,
                quantity: currency.quantity as i32,
                weekly_quantity: ((currency.weekly_quantity / scaler) > 0)
                    .then_some(currency.weekly_quantity),
                max_weekly_quantity: entry
                    .has_max_earnable_per_week()
                    .then_some(entry.max_earnable_per_week),
                tracked_quantity: entry
                    .is_tracking_quantity()
                    .then_some(currency.tracked_quantity),
                max_quantity: (max_quantity != 0).then_some(max_quantity as i32),
                total_earned: entry
                    .has_total_earned()
                    .then_some(currency.earned_quantity as i32),
                next_recharge_time: None,
                recharge_cycle_start_time: None,
                flags: currency.flags & !CURRENCY_DB_UNUSED_FLAGS_LIKE_CPP,
            });
        }

        Some(SetupCurrency::from_records(records))
    }
    pub(in crate::session) fn player_values_update_snapshot(&self) -> Option<Player> {
        let mut player = self.direct_inventory_player_snapshot()?;
        player.set_money(self.resolved_player_money_like_cpp()?);
        player.set_bank_bag_slot_count(self.resolved_player_bank_bag_slot_count_like_cpp()?);
        for index in 0..7 {
            let value = self.represented_bank_bag_slot_flag_like_cpp(index)?;
            player.set_bank_bag_slot_flag_value_like_cpp(index, value);
        }
        let inventory_items = self.resolved_inventory_items_like_cpp()?;
        let buyback_items = self.resolved_buyback_items_like_cpp()?;
        let buyback_price = self.resolved_buyback_price_like_cpp()?;
        let buyback_timestamp = self.resolved_buyback_timestamp_like_cpp()?;

        for slot in 0..19u8 {
            let visible = inventory_items.get(&slot).map(|item| VisibleItemValues {
                item_id: item.entry_id as i32,
                item_appearance_mod_id: 0,
                item_visual: 0,
            });
            player.set_visible_item_slot(slot, visible);
        }

        for slot in 15..=17u8 {
            let visible = inventory_items.get(&slot).map(|item| VisibleItemValues {
                item_id: item.entry_id as i32,
                item_appearance_mod_id: 0,
                item_visual: 0,
            });
            player
                .unit_mut()
                .set_virtual_item((slot - 15) as usize, visible);
        }

        for (&slot, item) in &buyback_items {
            if (slot as usize) < PLAYER_SLOT_END {
                player.set_inv_slot(slot as usize, item.guid);
            }
        }
        for index in 0..BUYBACK_SLOT_COUNT {
            player.set_buyback_price(index, buyback_price[index]);
            player.set_buyback_timestamp(index, buyback_timestamp[index]);
        }

        player.clear_data_changes();
        Some(player)
    }
    pub(crate) fn send_player_values_update_from_entity_bridge(
        &self,
        inv_slot_changes: &[(u8, ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) -> bool {
        // Reports whether a packet was enqueued. Callers that record a
        // publication need to know: this returns early when there is no player
        // GUID or snapshot, and the trace must not claim the client saw
        // something that was never sent.
        let Some(guid) = self.player_guid() else {
            return false;
        };
        if !visible_item_changes.is_empty() {
            let _ = self.mutate_canonical_player_like_cpp(|player| {
                for &(slot, item_id, item_appearance_mod_id, item_visual) in visible_item_changes {
                    crate::canonical_player_access::set_player_visible_item_values_like_cpp(
                        player,
                        slot,
                        (item_id, item_appearance_mod_id, item_visual),
                    );
                }
            });
        }
        let Some(mut player) = self.player_values_update_snapshot() else {
            return false;
        };

        if let Some(coinage) = coinage {
            player.set_money(coinage);
            player.mark_money_changed();
        }

        for &(slot, item_guid) in inv_slot_changes {
            player.set_inv_slot(slot as usize, item_guid);
            player.mark_inv_slot_changed(slot as usize);
        }

        for &(slot, item_id, appearance_mod_id, item_visual) in visible_item_changes {
            crate::canonical_player_access::set_player_visible_item_values_like_cpp(
                &mut player,
                slot,
                (item_id, appearance_mod_id, item_visual),
            );
            player.mark_visible_item_slot_changed(slot);
        }

        for &(index, item_id, appearance_mod_id, item_visual) in virtual_item_changes {
            let visible = (item_id != 0 || appearance_mod_id != 0 || item_visual != 0).then_some(
                VisibleItemValues {
                    item_id,
                    item_appearance_mod_id: appearance_mod_id,
                    item_visual,
                },
            );
            player.unit_mut().set_virtual_item(index as usize, visible);
            player.unit_mut().mark_virtual_item_changed(index as usize);
        }

        for &(slot, price, timestamp) in buyback_changes {
            if !(BUYBACK_SLOT_START..BUYBACK_SLOT_END).contains(&slot) {
                continue;
            }
            let index = (slot - BUYBACK_SLOT_START) as usize;
            player.set_buyback_price(index, price);
            player.mark_buyback_price_changed(index);
            player.set_buyback_timestamp(index, timestamp);
            player.mark_buyback_timestamp_changed(index);
        }

        let update = player.values_update(true);
        if let Some(packet) =
            player_values_update_to_update_object(guid, self.player_map_id_like_cpp(), &update)
        {
            // Reaching the send is not the same as the send being accepted:
            // the channel can be closed, and a publication recorded on the
            // strength of getting this far would claim a packet the client
            // never received.
            return self.send_packet(&packet);
        }
        false
    }
    pub(crate) fn send_player_values_update_like_cpp(
        &self,
        update: &wow_entities::PlayerValuesUpdate,
    ) {
        let Some(guid) = self.player_guid() else {
            return;
        };
        if let Some(packet) =
            player_values_update_to_update_object(guid, self.player_map_id_like_cpp(), update)
        {
            self.send_packet(&packet);
        }
    }
    pub(crate) fn send_represented_cinematic_start_like_cpp(&mut self, cinematic_id: u32) {
        if self.player_cinematic_state_snapshot_like_cpp().is_none() {
            return;
        }
        self.send_packet(&wow_packet::packets::misc::TriggerCinematic {
            cinematic_id,
            conversation_guid: ObjectGuid::EMPTY,
        });
        if let Some(sequence) = self
            .cinematic_sequences_store
            .as_ref()
            .and_then(|store| store.get(cinematic_id))
        {
            let camera_ids = sequence.camera;
            let _ = self.mutate_player_cinematic_state_like_cpp(|state| {
                state.cinematic_id = Some(cinematic_id);
                state.camera_ids = Some(camera_ids);
                state.camera_index = -1;
            });
        }
    }
    pub(crate) fn send_represented_resting_player_flag_update_like_cpp(&self) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };
        let Some(mut player) = self.player_values_update_snapshot() else {
            return false;
        };

        let canonical_flags = self
            .canonical_player_snapshot_like_cpp(|player| player.data().player_flags)
            .unwrap_or_default();
        let Some(is_resting) = self.resolved_is_resting_like_cpp() else {
            return false;
        };
        if is_resting {
            player.replace_all_player_flags(canonical_flags & !PLAYER_FLAGS_RESTING_LIKE_CPP);
            player.set_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
        } else {
            player.replace_all_player_flags(canonical_flags | PLAYER_FLAGS_RESTING_LIKE_CPP);
            player.remove_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
        }
        let update = player.values_update(true);
        if let Some(packet) =
            player_values_update_to_update_object(guid, self.player_map_id_like_cpp(), &update)
        {
            return self.send_packet(&packet);
        }
        false
    }
    pub(crate) fn send_represented_rest_info_update_like_cpp(&self, nested_mask: u8) {
        let Some(guid) = self.player_guid() else {
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
            player_values_update_to_update_object(guid, self.player_map_id_like_cpp(), &update)
        {
            self.send_packet(&packet);
        }
    }
    /// Get a clone of the send channel.
    pub fn send_tx(&self) -> &flume::Sender<Vec<u8>> {
        self.connection.send_tx()
    }
    pub(in crate::session) fn send_represented_mount_unit_update_like_cpp(
        &mut self,
        display_id: i32,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some((unit_flags, _, _)) = self.player_unit_presentation_snapshot_like_cpp() else {
            return;
        };

        use wow_packet::packets::update::{UnitDataValuesDeltaUpdate, UpdateObject};
        let mut data = UnitDataValuesDeltaUpdate::default();
        data.unit_data_mask[1] |= 1 << (41 - 32);
        data.unit_data_mask[1] |= 1 << (51 - 32);
        data.flags = unit_flags.bits();
        data.mount_display_id = display_id;

        self.send_packet(&UpdateObject::unit_values_update(
            player_guid,
            self.player_map_id_like_cpp(),
            data,
        ));
    }
    /// Send a TimeSyncRequest and schedule the next one.
    pub(crate) fn send_time_sync(&mut self) {
        use wow_packet::packets::misc::TimeSyncRequest;
        let sequence_index = self.time_sync_next_counter;
        self.send_packet(&TimeSyncRequest { sequence_index });
        trace!(
            "Sent TimeSyncRequest(seq={}) for account {}",
            sequence_index, self.account_id
        );
        self.time_sync_pending_requests
            .insert(sequence_index, Self::game_time_ms_like_cpp());
        // C++ uses 5s for the first request, then 10s.
        self.time_sync_timer_ms = if self.time_sync_next_counter == 0 {
            5000
        } else {
            10000
        };
        self.time_sync_next_counter += 1;
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
    pub(crate) fn try_send_packet<P: wow_packet::ServerPacket>(&self, pkt: &P) -> bool {
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
    pub(in crate::session) fn send_system_message_like_cpp(&self, text: &str) {
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
    pub(in crate::session) fn send_notification_like_cpp(&self, text: String) {
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
    pub(crate) fn tutorial_flags_packet_like_cpp(
        &self,
    ) -> wow_packet::packets::misc::TutorialFlags {
        wow_packet::packets::misc::TutorialFlags {
            tutorial_data: self.tutorials_like_cpp,
        }
    }
    pub(in crate::session) fn try_send_connected_player_command_like_cpp(
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
    pub(in crate::session) fn send_active_player_multi_action_bars_update_like_cpp(
        &self,
        guid: ObjectGuid,
    ) {
        let Some((_, _, multi_action_bars)) = self.active_player_update_state_like_cpp() else {
            return;
        };
        use wow_packet::packets::update::{ActivePlayerDataValuesUpdate, UpdateObject};

        let mut data = ActivePlayerDataValuesUpdate::default();
        set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 70);
        set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 72);
        data.multi_action_bars = multi_action_bars;
        self.send_packet(&UpdateObject::full_active_player_values_update(
            guid,
            self.player_map_id_like_cpp(),
            data,
        ));
    }
    pub(in crate::session) fn represented_dynamic_object_values_update_delivery_fingerprint_like_cpp(
        guid: ObjectGuid,
        bytes: &[u8],
    ) -> u64 {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        guid.hash(&mut hasher);
        bytes.hash(&mut hasher);
        hasher.finish()
    }
}
