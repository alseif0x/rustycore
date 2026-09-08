//! Player cast publication commits recipient visibility at each Start/Go phase.
//! Delivery uses the existing bounded durable rail and its overflow disconnect.

use super::*;
use crate::session::mailbox::SendPlayerSpellIfVisibleLikeCppCommand;

impl WorldSession {
    pub(in crate::session) fn publish_player_cast_interruption_like_cpp(
        &mut self,
        cast: SpellCastState,
    ) {
        if cast.metadata.client_cast_id.is_none() {
            return;
        }
        let visual = crate::spell_cast_adapter::present_visual(cast.spell_visual);
        self.publish_player_cast_interrupted_frames_like_cpp(
            cast.metadata,
            cast.cast_id,
            cast.spell_id,
            &visual,
        );
        self.send_packet(&CastFailed {
            cast_id: cast.cast_id,
            spell_id: cast.spell_id,
            visual,
            reason: SpellCastResult::Interrupted as i32,
            fail_arg1: 0,
            fail_arg2: 0,
        });
    }

    /// Spell::_cast cleanup sends its specific CastFailed first, followed by
    /// SendInterrupted(0). Cancellation uses these same frames before its result.
    pub(in crate::session) fn publish_player_cast_interrupted_frames_like_cpp(
        &mut self,
        metadata: SpellCastMetadata,
        cast_id: ObjectGuid,
        spell_id: i32,
        visual: &wow_packet::packets::spell::SpellCastVisual,
    ) {
        if metadata.client_cast_id.is_none() {
            return;
        }
        let Some(caster) = self.player_guid() else {
            return;
        };
        use wow_packet::ServerPacket;
        use wow_packet::packets::spell::{SpellFailedOtherPkt, SpellFailurePkt};
        self.publish_player_cast_frame_like_cpp(
            metadata,
            SpellFailurePkt {
                caster,
                cast_id,
                spell_id,
                visual: visual.clone(),
                reason: 0,
            }
            .to_bytes(),
        );
        self.publish_player_cast_frame_like_cpp(
            metadata,
            SpellFailedOtherPkt {
                caster,
                cast_id,
                spell_id: spell_id as u32,
                visual: visual.clone(),
                reason: 0,
            }
            .to_bytes(),
        );
    }

    pub(crate) fn player_cast_wire_data_like_cpp(
        &self,
        spell: &wow_data::SpellInfo,
    ) -> wow_packet::packets::spell::SpellCastData {
        let remaining_power = self
            .with_owned_player_like_cpp(|player| {
                let costs =
                    spell.calc_power_costs_like_cpp(player.unit().get_create_mana_like_cpp());
                if !costs
                    .iter()
                    .any(|cost| cost.power_type != PowerType::Health as i8)
                {
                    return Vec::new();
                }
                costs
                    .iter()
                    .filter_map(|cost| {
                        let power =
                            <PowerType as num_traits::FromPrimitive>::from_i8(cost.power_type)?;
                        Some(wow_packet::packets::spell::SpellPowerData {
                            amount: player.get_power(power),
                            power_type: cost.power_type,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        wow_packet::packets::spell::SpellCastData {
            remaining_power,
            ..Default::default()
        }
    }

    pub(crate) fn publish_player_cast_frame_like_cpp(
        &mut self,
        metadata: SpellCastMetadata,
        bytes: Vec<u8>,
    ) {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.send_raw_packet(&bytes);
            return;
        }
        let Some(handle) = self.player_handle_like_cpp else {
            return;
        };
        let source = (|| {
            let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
            let (key, revision) = manager.player_active_residence_revision_like_cpp(handle)?;
            if Some(revision) != metadata.prepared_residence_revision {
                return None;
            }
            let map = manager.find_map(key.map_id, key.instance_id)?.map();
            let player = map.get_typed_player(handle.guid())?;
            Some((
                key,
                player.unit().world().position(),
                player.unit().world().get_visibility_range(map),
            ))
        })();
        let Some((key, position, range)) = source else {
            return;
        };
        // Canonical map guard ended above. No packet delivery under it.
        self.send_raw_packet(&bytes);
        let Some(registry) = self.player_registry() else {
            return;
        };
        for recipient in registry.runtime_recipients() {
            if recipient.guid == handle.guid()
                || !recipient.is_in_world
                || u32::from(recipient.map_id) != key.map_id
                || recipient.instance_id != key.instance_id
                || !recipient.committed_visibility.contains(&handle.guid())
            {
                continue;
            }
            let dx = recipient.position.x - position.x;
            let dy = recipient.position.y - position.y;
            if dx * dx + dy * dy > range * range {
                continue;
            }
            let _ = registry.publish_current_player_spell_if_visible(
                recipient.registration,
                SendPlayerSpellIfVisibleLikeCppCommand {
                    map_id: recipient.map_id,
                    instance_id: key.instance_id,
                    packet_bytes: bytes.clone(),
                    committed_visibility_like_cpp: recipient.committed_visibility,
                },
            );
        }
    }

    pub(crate) fn handle_player_cast_publication_like_cpp(
        &mut self,
        command: SendPlayerSpellIfVisibleLikeCppCommand,
    ) {
        let opcode = command
            .packet_bytes
            .get(..2)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u16::from_le_bytes);
        if !matches!(opcode, Some(value) if value == ServerOpcodes::SpellStart as u16
            || value == ServerOpcodes::SpellGo as u16 || value == ServerOpcodes::SpellFailure as u16
            || value == ServerOpcodes::SpellFailedOther as u16)
        {
            return;
        }
        if self.state() != SessionState::LoggedIn
            || !self
                .client_visible_guids_like_cpp
                .shares_storage_like_cpp(&command.committed_visibility_like_cpp)
            || self.current_canonical_player_map_key_like_cpp()
                != Some(wow_map::MapKey::new(
                    u32::from(command.map_id),
                    command.instance_id,
                ))
        {
            return;
        }
        // Honor publication-time membership; reconnect/map changes are rejected.
        self.send_raw_packet(&command.packet_bytes);
    }
}
