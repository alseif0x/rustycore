//! Packets and values updates published from the Session boundary.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
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
        let store = self.catalogs.currency_types_store.as_ref()?;
        let currencies = self.player_currencies_like_cpp()?;

        let player_team =
            player_team_for_race_cpp(crate::session::hub_ref(self).player_race_like_cpp());
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
                    .catalogs
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
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.player_values_update_snapshot(hub)
    }
    pub(crate) fn send_player_values_update_from_entity_bridge(
        &self,
        inv_slot_changes: &[(u8, ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_player_values_update_from_entity_bridge(
            hub,
            inv_slot_changes,
            visible_item_changes,
            virtual_item_changes,
            buyback_changes,
            coinage,
        )
    }
    pub(crate) fn send_represented_cinematic_start_like_cpp(&mut self, cinematic_id: u32) {
        if crate::session::hub_ref(self)
            .player_cinematic_state_snapshot_like_cpp()
            .is_none()
        {
            return;
        }
        self.send_packet(&wow_packet::packets::misc::TriggerCinematic {
            cinematic_id,
            conversation_guid: ObjectGuid::EMPTY,
        });
        if let Some(sequence) = self
            .catalogs
            .cinematic_sequences_store
            .as_ref()
            .and_then(|store| store.get(cinematic_id))
        {
            let camera_ids = sequence.camera;
            let _ = crate::session::hub_mut(self).with_player_cinematic_state_like_cpp(|state| {
                state.begin_cinematic_like_cpp(cinematic_id, camera_ids);
            });
        }
    }
    pub(crate) fn send_represented_resting_player_flag_update_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_represented_resting_player_flag_update_like_cpp(hub)
    }
    pub fn send_tx(&self) -> &flume::Sender<Vec<u8>> {
        self.core.send_tx()
    }
    pub fn send_packet<P: wow_packet::ServerPacket>(&self, pkt: &P) -> bool {
        self.core.send_packet(pkt)
    }
    pub fn send_update_world_state_like_cpp(&self, variable_id: u32, value: i32, hidden: bool) {
        self.core
            .send_update_world_state_like_cpp(variable_id, value, hidden)
    }
    pub fn send_raw_packet(&self, data: &[u8]) {
        self.core.send_raw_packet(data)
    }
    pub fn send_buy_error(&self, result: BuyResult, creature_guid: Option<ObjectGuid>, item: u32) {
        self.core.send_buy_error(result, creature_guid, item)
    }
    pub fn send_sell_error(
        &self,
        result: SellResult,
        creature_guid: Option<ObjectGuid>,
        item_guid: ObjectGuid,
    ) {
        self.core.send_sell_error(result, creature_guid, item_guid)
    }
}

impl crate::session::state::InventoryState {
    pub(in crate::session) fn player_values_update_snapshot(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Option<Player> {
        let mut player = self.direct_inventory_player_snapshot(hub)?;
        player.set_money(self.resolved_player_money_like_cpp(hub)?);
        player.set_bank_bag_slot_count(self.resolved_player_bank_bag_slot_count_like_cpp(hub)?);
        for index in 0..7 {
            let value = self.represented_bank_bag_slot_flag_like_cpp(hub, index)?;
            player.set_bank_bag_slot_flag_value_like_cpp(index, value);
        }
        let inventory_items = self.resolved_inventory_items_like_cpp(hub)?;
        let buyback_items = self.resolved_buyback_items_like_cpp(hub)?;
        let buyback_price = self.resolved_buyback_price_like_cpp(hub)?;
        let buyback_timestamp = self.resolved_buyback_timestamp_like_cpp(hub)?;

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
        hub: crate::session::HubRef<'_>,
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
        let Some(guid) = hub.core.player_guid() else {
            return false;
        };
        if !visible_item_changes.is_empty() {
            let _ = hub.core.mutate_canonical_player_like_cpp(|player| {
                for &(slot, item_id, item_appearance_mod_id, item_visual) in visible_item_changes {
                    crate::canonical_player_access::set_player_visible_item_values_like_cpp(
                        player,
                        slot,
                        (item_id, item_appearance_mod_id, item_visual),
                    );
                }
            });
        }
        let Some(mut player) = self.player_values_update_snapshot(hub) else {
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
            player_values_update_to_update_object(guid, hub.core.player_map_id_like_cpp(), &update)
        {
            // Reaching the send is not the same as the send being accepted:
            // the channel can be closed, and a publication recorded on the
            // strength of getting this far would claim a packet the client
            // never received.
            return hub.core.send_packet(&packet);
        }
        false
    }

    pub(crate) fn send_represented_resting_player_flag_update_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> bool {
        let Some(guid) = hub.core.player_guid() else {
            return false;
        };
        let Some(mut player) = self.player_values_update_snapshot(hub) else {
            return false;
        };

        let canonical_flags = hub
            .core
            .canonical_player_snapshot_like_cpp(|player| player.data().player_flags)
            .unwrap_or_default();
        let Some(is_resting) = hub.resolved_is_resting_like_cpp() else {
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
            player_values_update_to_update_object(guid, hub.core.player_map_id_like_cpp(), &update)
        {
            return hub.core.send_packet(&packet);
        }
        false
    }
}

impl crate::session::state::SessionLifecycleState {
    pub(crate) fn tutorial_flags_packet_like_cpp(
        &self,
    ) -> wow_packet::packets::misc::TutorialFlags {
        wow_packet::packets::misc::TutorialFlags {
            tutorial_data: self.tutorials_like_cpp,
        }
    }
}
