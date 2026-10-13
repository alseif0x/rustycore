// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented homebind and the bind effect (#1363 slice 9c), from the
//! World `session/spell_effects/{effects,effect_apply}.rs`.

use super::{RepresentedHomebindLikeCpp, SpellEffectHostLikeCpp};
use wow_core::ObjectGuid;
use wow_packet::packets::spell::SpellTargetData;

pub fn set_homebind_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    binder_id: ObjectGuid,
    homebind: RepresentedHomebindLikeCpp,
) {
    if !host.set_represented_homebind_like_cpp(homebind) {
        return;
    }
    host.persist_player_homebind_like_cpp(homebind);

    host.send_packet(&wow_packet::packets::misc::BindPointUpdate {
        x: homebind.position.x,
        y: homebind.position.y,
        z: homebind.position.z,
        map_id: homebind.map_id,
        area_id: homebind.area_id,
    });
    host.core()
        .send_packet_realm(&wow_packet::packets::misc::PlayerBound {
            binder_id,
            area_id: homebind.area_id,
        });
}

pub fn represented_homebind_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &H,
) -> Option<RepresentedHomebindLikeCpp> {
    let canonical = host
        .core()
        .with_owned_player_like_cpp(|player| player.gameplay_state().homebind)
        .flatten();
    if canonical.is_some() {
        return canonical;
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.core().player_handle_like_cpp.is_none()
        && let Some(fixtures) = host.test_fixtures_ref_like_cpp()
    {
        return fixtures.teleport.represented_homebind_like_cpp;
    }
    None
}

pub async fn apply_effect_bind_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    effect: &wow_data::SpellEffectInfo,
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    target_data: &SpellTargetData,
) {
    if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND {
        return;
    }

    let Some(player_guid) = host.player_guid() else {
        return;
    };
    if target_guid != player_guid {
        return;
    }

    let Some((_, current_area_id)) = host.hub_ref().player_zone_area_like_cpp() else {
        return;
    };
    let area_id = wow_entities::bind_area_id_like_cpp(effect.effect_misc_value_1, current_area_id);

    let Some(current_position) = host.hub_ref().player_position_like_cpp() else {
        return;
    };
    let position = target_data
        .dst_location
        .as_ref()
        .map(|dst| dst.position)
        .unwrap_or(current_position);
    let map_id = target_data
        .map_id
        .and_then(|map_id| u32::try_from(map_id).ok())
        .unwrap_or_else(|| u32::from(host.core().player_map_id_like_cpp()));

    set_homebind_like_cpp(
        host,
        caster_guid,
        RepresentedHomebindLikeCpp {
            map_id,
            area_id,
            position,
        },
    );
}
