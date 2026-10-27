// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Prepare represented casts before target snapshots and SpellGo publication.

use super::{
    SpellExecutionHostLikeCpp, check_represented_cast_preparation_like_cpp,
    check_represented_caster_death_like_cpp,
    publish_player_cast_interrupted_frames_like_cpp,
    represented_gameobject_summon_missing_nearby_entry_destination_like_cpp,
    take_spell_power_like_cpp,
};
use crate::RepresentedSpellFocusObjectLikeCpp;
use tracing::debug;
use wow_constants::SpellCastResult;
use wow_core::ObjectGuid;
use wow_entities::SpellCastMetadata;
use wow_packet::packets::spell::SpellTargetData;

pub(super) fn prepare_represented_spell_execution_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    spell_info: &wow_data::SpellInfo,
    target_data: &SpellTargetData,
    spell_id: i32,
    cast_id: ObjectGuid,
    spell_visual: &wow_packet::packets::spell::SpellCastVisual,
    mut metadata: SpellCastMetadata,
) -> Result<Option<Option<RepresentedSpellFocusObjectLikeCpp>>, &'static str> {
    if !check_represented_caster_death_like_cpp(
        host,
        spell_info,
        cast_id,
        spell_visual,
        &mut metadata,
    )? {
        publish_player_cast_interrupted_frames_like_cpp(
            host,
            metadata,
            cast_id,
            spell_id,
            spell_visual,
        );
        return Ok(None);
    }

    let Some(represented_focus_object) = check_represented_cast_preparation_like_cpp(
        host,
        spell_info,
        cast_id,
        spell_visual,
        metadata,
    )?
    else {
        publish_player_cast_interrupted_frames_like_cpp(
            host,
            metadata,
            cast_id,
            spell_id,
            spell_visual,
        );
        return Ok(None);
    };

    if represented_gameobject_summon_missing_nearby_entry_destination_like_cpp(
        host,
        spell_id,
        spell_info,
        target_data,
        represented_focus_object,
    ) {
        // C++ `Spell::SelectImplicitNearbyTargets` rejects this cast before
        // `SMSG_SPELL_GO` when a required nearby-entry destination is absent.
        host.send_packet(&wow_packet::packets::spell::CastFailed {
            cast_id,
            spell_id,
            visual: spell_visual.clone(),
            reason: SpellCastResult::BadImplicitTargets as i32,
            fail_arg1: 0,
            fail_arg2: 0,
        });
        debug!(
            account = host.core().account_id,
            spell_id = spell_id,
            "Failing represented GameObject summon because C++ nearby-entry destination search found no target"
        );
        publish_player_cast_interrupted_frames_like_cpp(
            host,
            metadata,
            cast_id,
            spell_id,
            spell_visual,
        );
        return Ok(None);
    }

    if metadata.from_client
        && !take_spell_power_like_cpp(host, spell_info, cast_id, spell_id, spell_visual)
    {
        if metadata.restore_last_spell_cast_time_on_power_failure {
            let _ = host.mutate_cast_execution_like_cpp(|state| {
                state.last_cast_time = metadata.previous_last_spell_cast_time_on_power_failure;
            });
        }
        publish_player_cast_interrupted_frames_like_cpp(
            host,
            metadata,
            cast_id,
            spell_id,
            spell_visual,
        );
        return Ok(None);
    }

    Ok(Some(represented_focus_object))
}
