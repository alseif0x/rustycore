// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `CMSG_CAST_SPELL`, `CMSG_SPELL_CLICK` and `CMSG_SELF_RES` (#1363 slice
//! 9e), from the World `handlers/spell/ops_{1,2}.rs`: C++
//! `WorldSession::HandleCastSpellOpcode`, `HandleSpellClick` and
//! `HandleSelfResOpcode` (`Handlers/SpellHandler.cpp`). Normal requests
//! decode/adapt into `player_cast`; immediate and queued requests share
//! preparation, while the existing driver consumes timed casts.

use tracing::{debug, warn};
use wow_constants::ClientOpcodes;
use wow_entities::PendingSpellCastRequestLikeCpp as RepresentedPendingSpellCastRequestLikeCpp;
use wow_entities::SpellCastMetadata;
use wow_packet::ClientPacket;
use wow_packet::packets::spell::{CastSpellRequest, SelfRes, SpellClick};
use wow_world_core::session::ProgressionCatalogsLikeCpp;
use wow_world_core::session::map_admission::PlayerGridLoadResolverLikeCpp;
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

use super::PlayerCastHostLikeCpp;
use crate::spell_click_session::{
    execute_represented_spell_click_plan_with_generator_like_cpp,
    represented_handle_spell_click_plan_like_cpp,
};

/// Handle `CMSG_CAST_SPELL` (0x329C).
///
/// Decode movement and the original client request, then enter the shared
/// application admission/preparation path for both instant and timed casts.
pub async fn handle_cast_spell_with_catalogs_like_cpp<H: PlayerCastHostLikeCpp>(
    host: &mut H,
    area_trigger_catalogs: &H::AreaTriggerCatalogsLikeCpp,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    progression: &ProgressionCatalogsLikeCpp,
    player_grid_loader: &PlayerGridLoadResolverLikeCpp,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    mut pkt: wow_packet::WorldPacket,
) {
    let player_guid = match host.player_guid() {
        Some(g) => g,
        None => {
            warn!("handle_cast_spell: no player_guid");
            return;
        }
    };

    let req = match CastSpellRequest::read(&mut pkt) {
        Ok(r) => r,
        Err(e) => {
            warn!(
                account = host.core().account_id,
                "Failed to parse CMSG_CAST_SPELL: {e}"
            );
            return;
        }
    };

    let original_spell_id = req.spell_id;
    let cast_id = req.cast_id;

    debug!(
        account = host.core().account_id,
        spell_id = original_spell_id,
        cast_id = ?cast_id,
        target = ?req.target.unit,
        "CMSG_CAST_SPELL"
    );

    // C++ ignores a nonexistent spell before applying embedded movement.
    if host
        .hub_ref()
        .catalogs
        .spell_store()
        .and_then(|store| store.get(original_spell_id))
        .is_none()
    {
        warn!(
            account = host.core().account_id,
            spell_id = original_spell_id,
            "Ignoring cast request without an effective spell"
        );
        return;
    }

    // C++ `WorldSession::HandleCastSpellOpcode` applies an embedded
    // `MoveUpdate` through `HandleMovementOpcode(CMSG_MOVE_STOP, ...)`
    // after validating the `SpellInfo` and before the spell cast request
    // continues.
    if let Some(move_update) = req.move_update.clone() {
        wow_world_entities::handle_movement_info_with_catalogs_like_cpp(
            host,
            area_trigger_catalogs,
            creature_spawn_catalogs,
            progression,
            player_grid_loader,
            Some(ClientOpcodes::MoveStop),
            move_update,
        )
        .await;
    }

    let target_guid = if req.target.unit.is_empty() {
        player_guid
    } else {
        req.target.unit
    };
    let request = RepresentedPendingSpellCastRequestLikeCpp {
        cast_id,
        spell_id: original_spell_id,
        casting_unit_guid: player_guid,
        target_guid,
        target_data: crate::spell_cast_adapter::retain_targets(req.target),
        spell_visual: wow_entities::SpellCastVisualLikeCpp {
            spell_visual_id: req.visual.spell_visual_id,
            script_visual_id: 0,
        },
        metadata: SpellCastMetadata {
            from_client: true,
            misc: req.misc,
            // C++ `HandleCastSpellOpcode` copies the request trajectory
            // into `m_targets`; `SpellCastTargets::HasTraj()` then gates
            // `CAST_FLAG_ADJUST_MISSILE` in `Spell::SendSpellGo`.
            request_has_trajectory_like_cpp: req.has_trajectory_like_cpp,
            request_trajectory_pitch_like_cpp: req.trajectory_pitch_like_cpp,
            ..Default::default()
        },
    };
    if crate::player_cast::request(host, request) {
        host.tick_pending_spell_cast_request_with_generator_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
        )
        .await;
    }
}

/// Handle `CMSG_SPELL_CLICK`.
///
/// C++ `WorldSession::HandleSpellClick` resolves an in-world creature, pet,
/// or vehicle and then delegates to `Unit::HandleSpellClick`. Rust already
/// represents the packet shape plus spellclick stores/conditions/visibility;
/// executing spellclick casts, vehicle seat handling, and AI callbacks stays
/// in the next bounded runtime slice.
pub async fn handle_spell_click_with_generator_like_cpp<H: PlayerCastHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    mut pkt: wow_packet::WorldPacket,
) {
    let spell_click = match SpellClick::read(&mut pkt) {
        Ok(spell_click) => spell_click,
        Err(e) => {
            warn!(
                account = host.core().account_id,
                "Failed to parse CMSG_SPELL_CLICK: {e}"
            );
            return;
        }
    };

    debug!(
        account = host.core().account_id,
        target = ?spell_click.unit_guid,
        try_auto_dismount = spell_click.try_auto_dismount,
        "CMSG_SPELL_CLICK"
    );

    let plan = represented_handle_spell_click_plan_like_cpp(host, spell_click.unit_guid);
    debug!(
        account = host.core().account_id,
        target = ?spell_click.unit_guid,
        casts = plan.casts.len(),
        exact_context_unrepresented = plan.exact_context_unrepresented,
        ai_on_spell_click_unrepresented = plan.ai_on_spell_click_unrepresented,
        "CMSG_SPELL_CLICK represented execution plan"
    );
    let outcome = execute_represented_spell_click_plan_with_generator_like_cpp(
        host,
        item_guid_generator,
        creature_spawn_catalogs,
        spell_click.unit_guid,
        &plan,
    )
    .await;
    debug!(
        account = host.core().account_id,
        target = ?spell_click.unit_guid,
        planned_casts = outcome.planned_casts,
        executed_casts = outcome.executed_casts,
        skipped_unrepresented_caster = outcome.skipped_unrepresented_caster,
        skipped_unrepresented_target = outcome.skipped_unrepresented_target,
        skipped_unrepresented_original_caster = outcome.skipped_unrepresented_original_caster,
        failed_casts = outcome.failed_casts,
        "CMSG_SPELL_CLICK represented execution outcome"
    );
}

/// Handle `CMSG_SELF_RES`.
pub async fn handle_self_res_with_generator_like_cpp<H: PlayerCastHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    mut pkt: wow_packet::WorldPacket,
) {
    let request = match SelfRes::read(&mut pkt) {
        Ok(request) => request,
        Err(error) => {
            warn!(
                account = host.core().account_id,
                "SelfRes parse failed: {error}"
            );
            return;
        }
    };

    debug!(
        account = host.core().account_id,
        spell_id = request.spell_id,
        "CMSG_SELF_RES parsed"
    );
    if !host.has_represented_self_res_spell_like_cpp(request.spell_id) {
        return;
    }
    let Some(player_guid) = host.player_guid() else {
        return;
    };
    // C++ `HandleSelfResOpcode` uses
    // `CastSpell(_player, SpellID, GetMap()->GetDifficultyID())`, whose
    // trigger flags are TRIGGERED_NONE: not a triggered cast, and the
    // global cooldown applies.
    if crate::execute_server_triggered_spell_like_cpp(
        host,
        item_guid_generator,
        creature_spawn_catalogs,
        request.spell_id,
        player_guid,
        SpellCastMetadata::default(),
    )
    .await
    .is_ok()
    {
        {
            let (s, mut h) = host.split_spell_state_mut();
            s.remove_represented_self_res_spell_like_cpp(&mut h, request.spell_id)
        };
    }
}
