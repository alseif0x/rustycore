use crate::SessionSpellState;
use wow_constants::SpellCastResult;
use wow_core::ObjectGuid;
use wow_entities::{SpellCastMetadata, SpellCastState};
use wow_packet::packets::spell::CastFailed;
use wow_world_core::session::HubMut;
use wow_world_core::session::mailbox::SendPlayerSpellIfVisibleLikeCppCommand;

impl SessionSpellState {
    pub fn publish_player_cast_interruption_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        cast: SpellCastState,
    ) {
        if cast.metadata.client_cast_id.is_none() {
            return;
        }
        let visual = crate::present_visual(cast.spell_visual);
        self.publish_player_cast_interrupted_frames_like_cpp(
            hub,
            cast.metadata,
            cast.cast_id,
            cast.spell_id,
            &visual,
        );
        hub.core.send_packet(&CastFailed {
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
    pub fn publish_player_cast_interrupted_frames_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        metadata: SpellCastMetadata,
        cast_id: ObjectGuid,
        spell_id: i32,
        visual: &wow_packet::packets::spell::SpellCastVisual,
    ) {
        if metadata.client_cast_id.is_none() {
            return;
        }
        let Some(caster) = hub.core.player_guid() else {
            return;
        };
        use wow_packet::ServerPacket;
        use wow_packet::packets::spell::{SpellFailedOtherPkt, SpellFailurePkt};
        self.publish_player_cast_frame_like_cpp(
            hub,
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
            hub,
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

    pub fn publish_player_cast_frame_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        metadata: SpellCastMetadata,
        bytes: Vec<u8>,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            hub.core.send_raw_packet(&bytes);
            return;
        }
        let Some(handle) = hub.core.player_handle_like_cpp else {
            return;
        };
        let source = (|| {
            let manager = hub.core.canonical_map_manager.as_ref()?.lock().ok()?;
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
        hub.core.send_raw_packet(&bytes);
        let Some(registry) = hub.core.player_registry() else {
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
}
