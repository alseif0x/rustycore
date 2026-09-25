//! Final cast bookkeeping after the represented effect phases have completed.

use super::*;

impl WorldSession {
    pub(super) fn complete_spell_execution_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        player_guid: ObjectGuid,
        spell_info: wow_data::SpellInfo,
        metadata: SpellCastMetadata,
    ) -> Result<(), &'static str> {
        // C++ `Spell::FinishTargetProcessing` (`Spell.cpp:8493-8496`) publishes
        // the execute log once every effect of the cast has resolved.
        self.send_spell_execute_log_like_cpp(spell_id, caster_guid);

        let mut threat_spell_info = spell_info.clone();
        let difficulty = self.current_map_difficulty_id_like_cpp();
        if let Some(effects) = self.spell_store().and_then(|store| {
            store.effects_for_difficulty_like_cpp(
                spell_id,
                difficulty,
                self.difficulty_store().map(AsRef::as_ref),
            )
        }) {
            threat_spell_info.effects = effects.to_vec();
        }
        self.apply_spell_initial_threat_like_cpp(
            spell_id,
            caster_guid,
            target_guid,
            wow_data::represented_spell_is_positive_like_cpp(&threat_spell_info),
        );

        if caster_guid == player_guid {
            // This represented cooldown state belongs to the session player.
            // A triggered creature cast (for example innkeeper bind spell
            // 3286) must not start or advertise a player cooldown.
            if self
                .mutate_cast_execution_like_cpp(|state| {
                    // A prepared client cast already started its global
                    // cooldown in `Spell::prepare`; a `TRIGGERED_FULL_MASK`
                    // server cast carries `TRIGGERED_IGNORE_GCD` and never
                    // starts one.
                    if metadata.client_cast_id.is_none()
                        && !metadata.triggered_ignores_global_cooldown_like_cpp
                    {
                        state.last_cast_time = Some(Instant::now());
                    }
                    state
                        .last_cast_time_per_spell
                        .insert(spell_id, Instant::now());
                })
                .is_none()
            {
                return Ok(());
            }
            self.record_cast_character_spell_cooldown_like_cpp(
                spell_id,
                spell_info.recovery_time_ms.max(spell_info.cooldown_ms),
            );

            // Notify the owning player so the action bar shows the cooldown.
            use wow_packet::packets::spell::CooldownEvent;
            self.send_packet(&CooldownEvent {
                spell_id,
                is_pet: false,
            });
        }

        Ok(())
    }
}
