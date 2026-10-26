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
use wow_core::ObjectGuid;
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

fn player_has_aura_spell_like_cpp<H: PlayerCastHostLikeCpp>(
    host: &H,
    spell_id: i32,
) -> Option<bool> {
    let hub = host.hub_ref();
    if hub.player_aura_authority_ready_like_cpp() != Some(true) {
        return None;
    }
    hub.resolved_player_visible_auras_like_cpp()
        .map(|auras| auras.values().any(|aura| aura.spell_id == spell_id))
}

fn player_has_aura_type_like_cpp<H: PlayerCastHostLikeCpp>(
    host: &H,
    aura_type: i32,
) -> Option<bool> {
    use wow_data::ServersideSpellInfoLikeCpp;

    enum Source<'a> {
        Regular(u8),
        Server(&'a ServersideSpellInfoLikeCpp),
    }

    let hub = host.hub_ref();
    if hub.player_aura_authority_ready_like_cpp() != Some(true) {
        return None;
    }
    let auras = hub.resolved_player_visible_auras_like_cpp()?;
    let regular = hub.catalogs.spell_store().map(AsRef::as_ref)?;
    let serverside = hub.catalogs.spell_catalogs.serverside_spell_store.as_ref();
    let difficulties = hub.catalogs.difficulty_store().map(AsRef::as_ref);

    for aura in auras.values() {
        if aura.effect_mask == 0 {
            continue;
        }
        let spell_id = aura.spell_id as u32;
        let has_any_key = regular.contains_spell_info_any_difficulty_like_cpp(spell_id)
            || serverside.is_some_and(|store| {
                (u8::MIN..=u8::MAX).any(|difficulty| {
                    store
                        .get_serverside_spell_like_cpp(spell_id, u32::from(difficulty))
                        .is_some()
                })
            });
        let source = wow_data::spell::resolve_spell_catalog_value(
            aura.difficulty_id,
            difficulties,
            has_any_key,
            |difficulty_id| {
                let regular_key =
                    regular.contains_spell_info_exact_like_cpp(spell_id, difficulty_id);
                let server_info = serverside.and_then(|store| {
                    store.get_serverside_spell_like_cpp(spell_id, u32::from(difficulty_id))
                });
                match (regular_key, server_info) {
                    (true, Some(_)) => Some(Err(())),
                    (true, None) => Some(Ok(Source::Regular(difficulty_id))),
                    (false, Some(info)) => Some(Ok(Source::Server(info))),
                    (false, None) => None,
                }
            },
        )
        .ok()??
        .ok()?;
        let active_effects = match source {
            Source::Regular(difficulty_id) => wow_data::spell::resolve_spell_effect_rows(
                regular,
                aura.spell_id,
                difficulty_id,
                difficulties,
            )
            .ok()?
            .into_iter()
            .map(|effect| {
                Some((
                    i32::try_from(effect.effect_index).ok()?,
                    i32::from(effect.effect_aura),
                ))
            })
            .collect::<Option<Vec<_>>>()?,
            Source::Server(info) => info
                .effects
                .iter()
                .map(|effect| (effect.effect_index, effect.effect_aura))
                .collect(),
        };
        for effect_index in 0..32u32 {
            if aura.effect_mask & (1 << effect_index) == 0 {
                continue;
            }
            let Some((_, effect_aura)) = active_effects
                .iter()
                .find(|(index, _)| *index == effect_index as i32)
            else {
                return None;
            };
            if *effect_aura == aura_type {
                return Some(true);
            }
        }
    }
    Some(false)
}

/// Execute a native normal Player `CastSpell` call using the shared prepare,
/// active-cast, START/GCD and timer path. Instant effects finish before return;
/// an accepted timed cast remains installed for the regular cast driver.
pub async fn cast_server_normal_player_spell_with_generator_like_cpp<H: PlayerCastHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    spell_id: i32,
    target_guid: ObjectGuid,
    metadata: SpellCastMetadata,
) -> bool {
    let difficulty_id = host.core().current_map_difficulty_id_like_cpp();
    match crate::player_cast::prepare_server_normal(
        host,
        spell_id,
        difficulty_id,
        target_guid,
        metadata,
    ) {
        crate::player_cast::ServerNormalPrepareOutcomeLikeCpp::Rejected => false,
        crate::player_cast::ServerNormalPrepareOutcomeLikeCpp::Started => true,
        crate::player_cast::ServerNormalPrepareOutcomeLikeCpp::Instant => {
            crate::tick_active_spell_cast_with_generator_like_cpp(
                host,
                item_guid_generator,
                creature_spawn_catalogs,
            )
            .await;
            true
        }
        crate::player_cast::ServerNormalPrepareOutcomeLikeCpp::Direct(cast) => {
            crate::execution_session::execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp(
                host,
                item_guid_generator,
                creature_spawn_catalogs,
                cast.spell_id,
                cast.target_guid,
                cast.cast_id,
                crate::spell_cast_adapter::present_visual(cast.spell_visual),
                crate::spell_cast_adapter::present_targets(cast.target_data),
                cast.metadata,
            )
            .await
            .is_ok()
        }
    }
}

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
    let requested_difficulty = host.core().current_map_difficulty_id_like_cpp();
    let Some((spell_info, actual_difficulty)) =
        host.spell_for_difficulty(request.spell_id, requested_difficulty)
    else {
        return;
    };
    match player_has_aura_type_like_cpp(
        host,
        wow_data::spell::aura_types::SPELL_AURA_PREVENT_RESURRECTION,
    ) {
        Some(false) => {}
        Some(true) => {
            let bypass = host.hub_ref().catalogs.spell_store().is_some_and(|store| {
                store.has_attribute_for_difficulty_like_cpp(
                    spell_info.spell_id,
                    actual_difficulty,
                    host.hub_ref()
                        .catalogs
                        .difficulty_store()
                        .map(AsRef::as_ref),
                    7,
                    wow_data::spell::attributes::SPELL_ATTR7_BYPASS_NO_RESURRECT_AURA,
                )
            });
            if !bypass {
                return;
            }
        }
        None => return,
    }
    // C++ `HandleSelfResOpcode` uses normal `CastSpell` with the current Map
    // difficulty. The self-res list is consumed after the prepare attempt,
    // including native busy/cost/cooldown rejection.
    let _ = cast_server_normal_player_spell_with_generator_like_cpp(
        host,
        item_guid_generator,
        creature_spawn_catalogs,
        request.spell_id,
        player_guid,
        SpellCastMetadata::default(),
    )
    .await;
    let (s, mut h) = host.split_spell_state_mut();
    s.remove_represented_self_res_spell_like_cpp(&mut h, request.spell_id);
}
