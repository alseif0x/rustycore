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

