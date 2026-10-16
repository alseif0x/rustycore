// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The player-cast pending request (#1363 slice 12b7), moved from the World
//! `session/player_cast/state.rs`: C++ `Player::RequestSpellCast`,
//! `Player::CanExecutePendingSpellCastRequest` /
//! `Player::ExecutePendingSpellCastRequest`, the queue reads they use, and the
//! `Player::SendLoot` `InterruptNonMeleeSpells(false)` step.
//!
//! The bodies run over [`wow_world_spell::SpellbookHostLikeCpp`] (the tick over
//! [`wow_world_spell::PlayerCastHostLikeCpp`]); the canonical Player stays the
//! owner of the pending request and the handle-less fixture fallback is the
//! same `test`/`test-fixtures` branch it was in the World crate.

use wow_entities::PendingSpellCastRequestLikeCpp as RepresentedPendingSpellCastRequestLikeCpp;
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

/// C++ `Player::SendLoot` calls `InterruptNonMeleeSpells(false)`, i.e.
/// `withDelayed = false`, `spell_id = 0`, `withInstant = true`.
///
/// That single C++ call reaches the caster's current-spell slots, so the
/// represented equivalent must interrupt both the prepared cast execution
/// state and the canonical Unit slots, exactly as the far-teleport path
/// does. Reaching only one of the two would let a canonical generic or
/// channeled spell survive looting.
pub fn interrupt_non_melee_spell_cast_for_loot_like_cpp<
    H: wow_world_spell::SpellbookHostLikeCpp,
>(
    host: &mut H,
) -> bool {
    let session_cast_interrupted = {
        let (state, mut hub) = host.split_spell_state_mut();
        crate::interrupt_player_cast_like_cpp(&mut hub, state, None)
    };
    let canonical_spells_interrupted = host
        .core()
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            if !unit.is_non_melee_spell_cast_like_cpp(false, false, false, true) {
                return false;
            }
            !unit
                .interrupt_non_melee_spells(None, false, true)
                .is_empty()
        })
        .unwrap_or(false);

    session_cast_interrupted || canonical_spells_interrupted
}

pub fn pending_spell_cast_snapshot_like_cpp<H: wow_world_spell::SpellbookHostLikeCpp>(
    host: &H,
) -> Option<Option<RepresentedPendingSpellCastRequestLikeCpp>> {
    let (state, hub) = host.split_spell_state_ref();
    state.pending_spell_cast_snapshot_like_cpp(hub)
}

/// Apply one named canonical pending-cast transition, or the handle-less
/// test fallback that mirrors it.
fn with_pending_spell_cast_owner_like_cpp<H: wow_world_spell::SpellbookHostLikeCpp, R>(
    host: &mut H,
    canonical: impl FnOnce(&mut wow_entities::Player) -> R,
    #[cfg(any(test, feature = "test-fixtures"))] fallback: impl FnOnce(
        &mut Option<RepresentedPendingSpellCastRequestLikeCpp>,
    ) -> R,
) -> Option<R> {
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.core().player_handle_like_cpp.is_none() {
        return Some(
            host.split_spell_state_mut()
                .0
                .mutate_pending_spell_cast_fixture_like_cpp(fallback),
        );
    }
    host.core().with_owned_player_mut_like_cpp(canonical)
}

pub fn remaining_global_cooldown_ms_like_cpp<H: wow_world_spell::SpellbookHostLikeCpp>(
    host: &H,
    spell_info: &wow_data::SpellInfo,
) -> Option<u32> {
    let (state, hub) = host.split_spell_state_ref();
    state.remaining_global_cooldown_ms_like_cpp(hub, spell_info)
}

pub fn remaining_active_spell_cast_ms_like_cpp<H: wow_world_spell::SpellbookHostLikeCpp>(
    host: &H,
) -> Option<u32> {
    let (state, hub) = host.split_spell_state_ref();
    state.remaining_active_spell_cast_ms_like_cpp(hub)
}

pub fn request_represented_spell_cast_like_cpp<H: wow_world_spell::SpellbookHostLikeCpp>(
    host: &mut H,
    request: RepresentedPendingSpellCastRequestLikeCpp,
) {
    {
        let (state, mut hub) = host.split_spell_state_mut();
        crate::cancel_pending_spell_cast_request_like_cpp(&mut hub, state)
    };
    let _ = with_pending_spell_cast_owner_like_cpp(
        host,
        {
            let request = request.clone();
            move |player| {
                player.request_spell_cast_like_cpp(request);
            }
        },
        #[cfg(any(test, feature = "test-fixtures"))]
        move |pending| {
            *pending = Some(request);
        },
    );
}

pub async fn tick_pending_spell_cast_request_with_generator_like_cpp<
    H: wow_world_spell::PlayerCastHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
) {
    let Some(request) = pending_spell_cast_snapshot_like_cpp(host).flatten() else {
        return;
    };
    // C++ `Player::CanExecutePendingSpellCastRequest` cancels the request
    // when the casting unit is missing, is no longer in world, or is no
    // longer `GetUnitBeingMoved()`. A detached or replaced handle must not
    // execute a queued request against the current player.
    let casting_unit_is_current = Some(request.casting_unit_guid) == host.player_guid()
        && host
            .hub_ref()
            .player_moved_unit_guid_like_cpp()
            .is_none_or(|moved| moved == request.casting_unit_guid)
        && host
            .core()
            .player_handle_like_cpp
            .is_none_or(|handle| handle.guid() == request.casting_unit_guid);
    if !casting_unit_is_current {
        {
            let (state, mut hub) = host.split_spell_state_mut();
            crate::cancel_pending_spell_cast_request_like_cpp(&mut hub, state)
        };
        return;
    }
    let Some(spell_info) = host
        .hub_ref()
        .catalogs
        .spell_store()
        .and_then(|store| store.get(request.spell_id))
        .cloned()
    else {
        {
            let (state, mut hub) = host.split_spell_state_mut();
            crate::cancel_pending_spell_cast_request_like_cpp(&mut hub, state)
        };
        return;
    };

    if remaining_global_cooldown_ms_like_cpp(host, &spell_info) != Some(0) {
        return;
    }
    if remaining_active_spell_cast_ms_like_cpp(host) != Some(0) {
        return;
    }

    let (cast_id, spell_id, casting_unit_guid) =
        (request.cast_id, request.spell_id, request.casting_unit_guid);
    let Some(request) = with_pending_spell_cast_owner_like_cpp(
        host,
        move |player| {
            player.take_matching_pending_spell_cast_like_cpp(cast_id, spell_id, casting_unit_guid)
        },
        #[cfg(any(test, feature = "test-fixtures"))]
        move |pending| {
            if pending.as_ref().is_some_and(|current| {
                current.cast_id == cast_id
                    && current.spell_id == spell_id
                    && current.casting_unit_guid == casting_unit_guid
            }) {
                pending.take()
            } else {
                None
            }
        },
    )
    .flatten() else {
        return;
    };
    if wow_world_spell::player_cast::prepare(host, request) {
        wow_world_spell::tick_active_spell_cast_with_generator_like_cpp(
            host,
            item_guid_generator,
            creature_spawn_catalogs,
        )
        .await;
    }
}
