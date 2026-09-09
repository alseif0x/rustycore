//! Represented spell state published to other systems: trade spells, the
//! money-coupled acquisition commit and visible spell-click refreshes.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn publish_spell_pull_attack_start_like_cpp(
        &mut self,
        attacker_guid: ObjectGuid,
        victim_guid: ObjectGuid,
    ) {
        use wow_packet::ServerPacket;

        let map_id = self.player_map_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let command = CreatureAttackStartLikeCppCommand {
            attacker_guid,
            victim_guid,
            previous_victim_guid: None,
            map_id,
            instance_id,
            packet_already_broadcast: false,
        };
        self.handle_creature_attack_start_like_cpp_command_like_cpp(command);

        let packet_bytes = wow_packet::packets::combat::AttackStart {
            attacker: attacker_guid,
            victim: victim_guid,
        }
        .to_bytes();
        let Some((source_position, source_combat_reach, visibility_range)) = self
            .mutate_world_creature(attacker_guid, |creature| {
                (
                    creature.position(),
                    creature.creature.unit().world().combat_reach(),
                    creature.visibility_range_like_cpp(),
                )
            })
        else {
            return;
        };
        let Some(registry) = self.player_registry.as_ref() else {
            return;
        };
        for registration in registry.spell_pull_recipients(
            victim_guid,
            map_id,
            instance_id,
            source_position,
            source_combat_reach,
            visibility_range,
        ) {
            let _ = registry.publish_current_send_if_visible(
                registration,
                SendIfVisibleLikeCppCommand {
                    queued_at: Instant::now(),
                    source_guid: attacker_guid,
                    map_id,
                    instance_id,
                    packet_bytes: packet_bytes.clone(),
                },
            );
        }
    }
    #[cfg(test)]
    pub(crate) fn set_represented_trade_spell_like_cpp_for_test(
        &mut self,
        spell_id: u32,
        cast_item_guid: Option<ObjectGuid>,
    ) {
        let _ = self.mutate_player_trade_state_like_cpp(|state| {
            if let Some(state) = state {
                state.spell_id = spell_id;
                state.spell_cast_item_guid = cast_item_guid;
            }
        });
    }
    #[cfg(test)]
    pub(crate) fn represented_trade_spell_like_cpp(&self) -> u32 {
        self.player_trade_state_snapshot_like_cpp()
            .flatten()
            .map_or(0, |state| state.spell_id)
    }
    pub(crate) fn set_represented_trade_spell_like_cpp(
        &mut self,
        spell_id: u32,
        pack_slot: u8,
        item_slot_in_pack: u8,
    ) {
        let Some(Some(trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        if spell_id == 0 {
            self.set_represented_trade_spell_state_like_cpp(partner_guid, 0, None);
            return;
        }

        let cast_item_guid = if pack_slot != NULL_BAG || item_slot_in_pack != NULL_SLOT {
            self.get_inventory_item_by_pos(pack_slot, item_slot_in_pack)
                .map(|item| item.guid)
        } else {
            None
        };

        let Ok(spell_id_i32) = i32::try_from(spell_id) else {
            self.set_represented_trade_spell_state_like_cpp(partner_guid, 0, None);
            return;
        };

        let Some(spell_store) = self.spell_store() else {
            self.set_represented_trade_spell_state_like_cpp(partner_guid, 0, None);
            return;
        };

        if spell_store.get(spell_id_i32).is_none() {
            self.set_represented_trade_spell_state_like_cpp(partner_guid, 0, None);
            return;
        }

        if !self.known_spells_like_cpp().contains(&spell_id_i32) {
            self.set_represented_trade_spell_state_like_cpp(partner_guid, 0, None);
            return;
        }

        self.set_represented_trade_spell_state_like_cpp(partner_guid, spell_id, cast_item_guid);
    }
    fn set_represented_trade_spell_state_like_cpp(
        &mut self,
        partner_guid: ObjectGuid,
        spell_id: u32,
        cast_item_guid: Option<ObjectGuid>,
    ) {
        use wow_packet::ServerPacket;

        let Some(Some(mut trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        if trade.partner_guid != partner_guid {
            return;
        }
        if trade.spell_id == spell_id && trade.spell_cast_item_guid == cast_item_guid {
            return;
        }

        trade.spell_id = spell_id;
        trade.spell_cast_item_guid = cast_item_guid;
        trade.accepted = false;
        trade.server_state_index = trade.server_state_index.wrapping_add(1);
        if self
            .mutate_player_trade_state_like_cpp(|state| *state = Some(trade))
            .is_none()
        {
            return;
        }

        let packet_bytes =
            TradeStatus::status_only_like_cpp(TRADE_STATUS_UNACCEPTED_LIKE_CPP).to_bytes();
        self.send_raw_packet(&packet_bytes);

        self.try_send_connected_player_command_like_cpp(
            partner_guid,
            SessionCommand::UnacceptRepresentedTradeLikeCpp(
                crate::session::mailbox::UnacceptRepresentedTradeLikeCppCommand { packet_bytes },
            ),
        );
    }
}
