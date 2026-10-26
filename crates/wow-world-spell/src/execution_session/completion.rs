// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Final cast bookkeeping after the represented effect phases have completed.

use super::SpellExecutionHostLikeCpp;
use super::selected_effects::SelectedCastThreatPlan;
use crate::effect_session::{
    apply_selected_spell_initial_threat, apply_spell_initial_threat_like_cpp,
    send_spell_execute_log_like_cpp,
};
use std::time::Instant;
use wow_core::ObjectGuid;
use wow_entities::SpellCastMetadata;

pub(super) fn complete_spell_execution_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    player_guid: ObjectGuid,
    spell_info: wow_data::SpellInfo,
    metadata: SpellCastMetadata,
    selected_full_server_aura: bool,
    selected_cast_threat_plan: SelectedCastThreatPlan,
) -> Result<(), &'static str> {
    // C++ `Spell::FinishTargetProcessing` (`Spell.cpp:8493-8496`) publishes
    // the execute log once every effect of the cast has resolved.
    send_spell_execute_log_like_cpp(host, spell_id, caster_guid);

    match selected_cast_threat_plan {
        SelectedCastThreatPlan::Legacy => {
            let mut threat_spell_info = spell_info.clone();
            let difficulty = host.core().current_map_difficulty_id_like_cpp();
            if let Some(effects) = host.hub_ref().catalogs.spell_store().and_then(|store| {
                store.effects_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    host.hub_ref()
                        .catalogs
                        .difficulty_store()
                        .map(AsRef::as_ref),
                )
            }) {
                threat_spell_info.effects = effects.to_vec();
            }
            apply_spell_initial_threat_like_cpp(
                host,
                spell_id,
                caster_guid,
                target_guid,
                wow_data::represented_spell_is_positive_like_cpp(&threat_spell_info),
            );
        }
        SelectedCastThreatPlan::ExcludedByAttributes => {}
        SelectedCastThreatPlan::Selected {
            whole_spell_positive,
            attributes,
        } => apply_selected_spell_initial_threat(
            host,
            spell_id,
            caster_guid,
            target_guid,
            whole_spell_positive,
            &attributes,
        ),
    }

    // The selected route uses TRIGGERED_FULL_MASK, including
    // TRIGGERED_IGNORE_SPELL_AND_CATEGORY_CD (SpellDefines.h:243-261).
    // Spell::SendSpellCooldown delegates to SpellHistory::HandleCooldowns,
    // which returns for that flag (Spell.cpp:4195-4203;
    // SpellHistory.cpp:252-255). Skip this route's represented cooldown
    // bookkeeping and event without changing already stored history; this
    // gate is scoped to the selected healer auras, not every FULL-mask cast
    // or every cast carrying generic ignore-GCD metadata.
    if caster_guid == player_guid && !selected_full_server_aura {
        // This represented cooldown state belongs to the session player.
        // A triggered creature cast (for example innkeeper bind spell
        // 3286) must not start or advertise a player cooldown.
        if host
            .mutate_cast_execution_like_cpp(|state| {
                // A prepared client cast already started its global
                // cooldown in `Spell::prepare`; a `TRIGGERED_FULL_MASK`
                // server cast carries `TRIGGERED_IGNORE_GCD` and never
                // starts one.
                if metadata.client_cast_id.is_none()
                    && metadata.trigger_cast_flags_like_cpp
                        & wow_entities::TRIGGERED_IGNORE_GCD_LIKE_CPP
                        == 0
                    && !metadata.server_normal_cast_like_cpp
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
        if !metadata.server_normal_cast_like_cpp {
            let (s, mut h) = host.split_spell_state_mut();
            s.record_cast_character_spell_cooldown_like_cpp(
                &mut h,
                spell_id,
                spell_info.recovery_time_ms.max(spell_info.cooldown_ms),
            );

            // Client casts retain their existing represented cooldown event.
            // Normal Player casts publish their native cooldown before GO.
            use wow_packet::packets::spell::CooldownEvent;
            host.send_packet(&CooldownEvent {
                spell_id,
                is_pet: false,
            });
        }
    }

    Ok(())
}
