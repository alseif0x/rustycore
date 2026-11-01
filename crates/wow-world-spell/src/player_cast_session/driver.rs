// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The ready-cast take and the active-cast tick (#1363 slice 9e), from the
//! World `session/player_cast.rs` and `session/player_cast/state.rs`: the
//! existing driver entry points over the canonical Player/Unit cast state.

use tracing::warn;
use wow_entities::SpellCastState;
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

use crate::SpellExecutionHostLikeCpp;

pub fn take_ready_player_cast_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
) -> Option<SpellCastState> {
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() && host.core().player_handle_like_cpp.is_none() {
        return host
            .mutate_cast_execution_like_cpp(
                wow_entities::CastExecutionStateLikeCpp::take_ready_cast,
            )
            .flatten();
    }
    let handle = host.core().player_handle_like_cpp?;
    if Some(handle.guid()) != host.player_guid() {
        return None;
    }
    let mut manager = host.core().canonical_map_manager.as_ref()?.lock().ok()?;
    let revision = manager
        .player_active_residence_revision_like_cpp(handle)
        .map(|(_, revision)| revision);
    manager
        .with_player_mut_like_cpp(handle, |player| {
            let state = &mut player.unit_mut().subsystems_mut().spells.execution;
            // Any residence-stamped prepared cast is fenced, not only the
            // normal client request: a server-triggered timed cast
            // prepared in a previous residence must not launch after
            // reentry either.
            if state.active.as_ref().is_some_and(|cast| {
                cast.metadata.prepared_residence_revision.is_some()
                    && cast.metadata.prepared_residence_revision != revision
            }) {
                state.active = None;
                return None;
            }
            state.take_ready_cast()
        })
        .flatten()
}

/// Called every ~100ms. Checks if an in-progress spell cast has completed.
///
/// If `active_spell_cast` is set and its cast time has elapsed, this method
/// executes the spell (applies effects, cooldowns, etc.) and clears the cast state.
pub async fn tick_active_spell_cast_with_generator_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
) {
    let Some(cast_state) = take_ready_player_cast_like_cpp(host) else {
        return;
    };

    let spell_id = cast_state.spell_id;
    let target = cast_state.target_guid;
    let target_data = crate::spell_cast_adapter::present_targets(cast_state.target_data);
    let cast_id = cast_state.cast_id;
    let spell_visual = crate::spell_cast_adapter::present_visual(cast_state.spell_visual);
    let metadata = cast_state.metadata;

    // The owner guard is released before effect execution and failure publication.
    if let Err(e) =
        crate::execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp(
            host,
            item_guid_generator,
            creature_spawn_catalogs,
            spell_id,
            target,
            cast_id,
            spell_visual.clone(),
            target_data,
            metadata,
        )
        .await
    {
        warn!(
            account = host.core().account_id,
            "Spell execution failed: {}", e
        );
        if host.core().state == wow_world_core::session::SessionState::Disconnecting {
            return;
        }
        // Send CastFailed so client cancels cast animation
        use wow_packet::packets::spell::CastFailed;
        host.send_packet(&CastFailed {
            cast_id,
            spell_id,
            visual: spell_visual,
            reason: 2, // SpellCastResult::NotKnown
            fail_arg1: 0,
            fail_arg2: 0,
        });
    }
}
