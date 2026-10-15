// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The farsight seer consumer (#1363 slice 12b2), moved from the World
//! `session/spell_effects/effects.rs`: the represented `Player::SetViewpoint`
//! after `Spell::EffectAddFarsight`. It reads and writes the session
//! visibility state, so it runs over the entities visibility host; the
//! World test session's seer fixture is a spell-execution host hook.

use wow_core::ObjectGuid;
use wow_world_entities::VisibilitySessionHostLikeCpp;

use super::SpellExecutionHostLikeCpp;

/// Consume represented C++ `DynamicObject::SetCasterViewpoint()` ->
/// `Player::SetViewpoint(..., true)` evidence into this live session's
/// represented `Player::m_seer` seam after the canonical map mutation has
/// returned and no map lock is held by this caller.
///
/// C++ anchors: `SpellEffects.cpp:2237-2261`, `DynamicObject.cpp:209-225`,
/// `Player.cpp:25344-25387`, `Player.h:2432,2438`, and
/// `Player.cpp:23343-23349`. Ownership remains canonical `Map::map_objects`;
/// the session consumes the outcome for packet publication and test
/// fixtures; production visibility derives the seer from Player state.
pub fn send_set_viewpoint_target_visibility_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &mut H,
    dynamic_object_guid: ObjectGuid,
) -> bool {
    if host
        .core()
        .client_visible_guids_like_cpp
        .contains(&dynamic_object_guid)
    {
        return false;
    }

    let Some(player_map_key) = host.core().current_canonical_player_map_key_like_cpp() else {
        return false;
    };
    let Ok(packet_map_id) = u16::try_from(player_map_key.map_id) else {
        return false;
    };
    let Some(manager) = host.core().canonical_map_manager.as_ref() else {
        return false;
    };
    let Ok(manager) = manager.lock() else {
        return false;
    };
    let Some(map) = manager.find_map(player_map_key.map_id, player_map_key.instance_id) else {
        return false;
    };
    let Some(dynamic_object) = map.map().get_typed_dynamic_object(dynamic_object_guid) else {
        return false;
    };
    if dynamic_object.world().map_id() != player_map_key.map_id {
        return false;
    }

    let create_data = wow_world_entities::dynamic_object_create_data_from_canonical_like_cpp(
        dynamic_object_guid,
        dynamic_object,
    );
    drop(manager);

    let block = wow_packet::packets::update::UpdateObject::create_dynamic_object_block(create_data);
    host.send_packet(
        &wow_packet::packets::update::UpdateObject::create_world_objects(
            vec![block],
            packet_map_id,
        ),
    );
    host.core_mut()
        .client_visible_guids_like_cpp
        .insert(dynamic_object_guid);
    true
}

pub fn consume_add_farsight_set_seer_outcome_like_cpp<
    H: SpellExecutionHostLikeCpp + VisibilitySessionHostLikeCpp,
>(
    host: &mut H,
    outcome: &wow_map::map::FarsightDynamicObjectCreateOutcomeLikeCpp,
) -> bool {
    if outcome.status != wow_map::map::FarsightDynamicObjectCreateStatusLikeCpp::Created {
        return false;
    }
    let Some(dynamic_object_guid) = outcome.dynamic_object_guid else {
        return false;
    };
    let Some(caster_viewpoint) = outcome.caster_viewpoint.as_ref() else {
        return false;
    };
    if !caster_viewpoint.apply || caster_viewpoint.dynamic_object_guid != dynamic_object_guid {
        return false;
    }
    let player_set_viewpoint = &caster_viewpoint.player_set_viewpoint;
    if !player_set_viewpoint.apply
        || player_set_viewpoint.target_guid != dynamic_object_guid
        || !player_set_viewpoint.set_seer_requested
        || player_set_viewpoint.status != wow_map::map::PlayerSetViewpointStatusLikeCpp::Applied
    {
        return false;
    }

    let Some(player_guid) = host.player_guid() else {
        return false;
    };

    {
        let (s, h) = (host.visibility(), host.hub_ref());
        s.send_active_player_farsight_object_values_update_like_cpp(
            h,
            player_guid,
            dynamic_object_guid,
        )
    };
    // C++ `Player::SetViewpoint(target, true)` orders the direct
    // `UpdateVisibilityOf(target)` after writing `FarsightObject` and before
    // `SetSeer(target)`, so this represented target-only create is consumed
    // before the next visibility read derives the seer from canonical
    // Player state.
    send_set_viewpoint_target_visibility_like_cpp(host, dynamic_object_guid);
    host.visibility_mut()
        .observe_farsight_object_like_cpp(dynamic_object_guid);
    #[cfg(any(test, feature = "test-fixtures"))]
    {
        host.set_represented_seer_guid_fixture_like_cpp(Some(dynamic_object_guid));
    }
    player_set_viewpoint.update_visibility_requested
}
