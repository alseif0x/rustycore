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

        let map_id = self.core.player_map_id_like_cpp();
        let instance_id = self
            .core
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
        let Some((source_position, source_combat_reach, visibility_range)) =
            self.core.mutate_world_creature(attacker_guid, |creature| {
                (
                    creature.position(),
                    creature.creature.unit().world().combat_reach(),
                    creature.visibility_range_like_cpp(),
                )
            })
        else {
            return;
        };
        let Some(registry) = self.core.player_registry.as_ref() else {
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
}
