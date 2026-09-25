//! Prepare represented casts before target snapshots and SpellGo publication.

use super::*;

impl WorldSession {
    pub(super) fn prepare_represented_spell_execution_like_cpp(
        &mut self,
        spell_info: &wow_data::SpellInfo,
        target_data: &SpellTargetData,
        spell_id: i32,
        cast_id: ObjectGuid,
        spell_visual: &wow_packet::packets::spell::SpellCastVisual,
        metadata: SpellCastMetadata,
    ) -> Result<Option<Option<RepresentedSpellFocusObjectLikeCpp>>, &'static str> {
        let Some(represented_focus_object) = self.check_represented_cast_preparation_like_cpp(
            spell_info,
            cast_id,
            spell_visual,
            metadata,
        )?
        else {
            self.publish_player_cast_interrupted_frames_like_cpp(
                metadata,
                cast_id,
                spell_id,
                spell_visual,
            );
            return Ok(None);
        };

        if self.represented_gameobject_summon_missing_nearby_entry_destination_like_cpp(
            spell_id,
            spell_info,
            target_data,
            represented_focus_object,
        ) {
            // C++ `Spell::SelectImplicitNearbyTargets` rejects this cast before
            // `SMSG_SPELL_GO` when a required nearby-entry destination is absent.
            self.send_packet(&wow_packet::packets::spell::CastFailed {
                cast_id,
                spell_id,
                visual: spell_visual.clone(),
                reason: SpellCastResult::BadImplicitTargets as i32,
                fail_arg1: 0,
                fail_arg2: 0,
            });
            debug!(
                account = self.account_id,
                spell_id = spell_id,
                "Failing represented GameObject summon because C++ nearby-entry destination search found no target"
            );
            self.publish_player_cast_interrupted_frames_like_cpp(
                metadata,
                cast_id,
                spell_id,
                spell_visual,
            );
            return Ok(None);
        }

        if metadata.from_client
            && !self.take_spell_power_like_cpp(spell_info, cast_id, spell_id, spell_visual)
        {
            if metadata.restore_last_spell_cast_time_on_power_failure {
                let _ = self.mutate_cast_execution_like_cpp(|state| {
                    state.last_cast_time = metadata.previous_last_spell_cast_time_on_power_failure;
                });
            }
            self.publish_player_cast_interrupted_frames_like_cpp(
                metadata,
                cast_id,
                spell_id,
                spell_visual,
            );
            return Ok(None);
        }

        Ok(Some(represented_focus_object))
    }
}
