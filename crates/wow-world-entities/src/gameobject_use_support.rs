// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Single-caller helpers of the moved gameobject use (#1363 slice 4a): the
//! battleground-object gates, the item forge, the fishing skill base and the
//! stealth/invisibility removal, each moved from its World session module.

use std::time::{Duration, Instant};

use wow_constants::unit::UnitFlags;
use wow_core::ObjectGuid;
use wow_entities::RepresentedAuraEffectLikeCpp;

use crate::gameobject_use_host::GameObjectUseHostLikeCpp;
use crate::{RepresentedBattlegroundObjectUseRejection, RepresentedGameObjectUseEffect};
use crate::{RepresentedCapturePointStateLikeCpp, RepresentedNewFlagStateRequest};

pub fn represented_player_can_use_battleground_object_like_cpp<H: GameObjectUseHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> bool {
    let gameobject_faction = host
        .world_entities()
        .represented_gameobject_use_state_like_cpp(gameobject_guid)
        .and_then(|state| state.faction_template);
    if let (Some(player_faction), Some(gameobject_faction), Some(store)) = (
        host.hub_ref().player_faction_template_id_like_cpp(),
        gameobject_faction,
        host.hub_ref().catalogs.factions.template_store.as_ref(),
    ) && let (Some(player_entry), Some(gameobject_entry)) =
        (store.get(player_faction), store.get(gameobject_faction))
        && !player_entry.is_friendly_to_like_cpp(gameobject_entry)
    {
        host.world_entities_mut()
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::UnfriendlyFaction,
                },
            );
        return false;
    }

    let Some((player_unit_flags, _, _)) =
        host.hub_ref().player_unit_presentation_snapshot_like_cpp()
    else {
        return false;
    };
    if player_unit_flags.contains(UnitFlags::IMMUNE) {
        host.world_entities_mut()
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::DamageImmune,
                },
            );
        return false;
    }

    let Some(has_recently_dropped_flag_debuff) =
        host.hub_ref().has_recently_dropped_flag_debuff_like_cpp()
    else {
        return false;
    };
    if has_recently_dropped_flag_debuff {
        host.world_entities_mut()
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::RecentlyDroppedFlag,
                },
            );
        return false;
    }

    let Some(player_is_alive) = host.hub_ref().resolved_player_is_alive_like_cpp() else {
        return false;
    };
    if !player_is_alive {
        host.world_entities_mut()
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::Dead,
                },
            );
        return false;
    }

    true
}

pub fn represented_player_battleground_type_id_or_reject_like_cpp<H: GameObjectUseHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> Option<u32> {
    let state = host
        .hub_ref()
        .player_battleground_state_snapshot_like_cpp()?;
    if let Some(bg_type_id) = state.battleground_type_id_like_cpp() {
        return Some(bg_type_id);
    }

    host.world_entities_mut()
        .record_represented_gameobject_use_effect_like_cpp(
            RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::NotInBattleground,
            },
        );
    None
}

pub fn represented_player_reject_battleground_object_vehicle_like_cpp<
    H: GameObjectUseHostLikeCpp,
>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> bool {
    if host
        .hub_ref()
        .player_vehicle_seat_state_like_cpp()
        .and_then(|(flags, _)| flags)
        .is_none()
    {
        return false;
    }

    host.world_entities_mut()
        .record_represented_gameobject_use_effect_like_cpp(
            RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::Vehicle,
            },
        );
    true
}

pub fn use_represented_gameobject_item_forge_like_cpp<H: GameObjectUseHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
    source: wow_entities::ItemForgeUseSource,
) -> bool {
    host.world_entities_mut()
        .record_represented_gameobject_use_effect_like_cpp(
            RepresentedGameObjectUseEffect::ItemForgeUsed {
                gameobject_guid,
                player_guid,
                condition_id: source.condition_id,
                forge_type: source.forge_type,
            },
        );

    true
}

pub fn represented_fishing_base_skill_level_like_cpp<H: GameObjectUseHostLikeCpp>(
    host: &H,
    gameobject_guid: ObjectGuid,
) -> Option<i32> {
    let area_id = host
        .world_entities()
        .represented_gameobject_use_state_like_cpp(gameobject_guid)
        .and_then(|state| state.area_id)?;
    let area_store = host.hub_ref().catalogs.area_table_store()?;
    let fishing_store = host.hub_ref().catalogs.fishing_base_skill_store()?;
    Some(fishing_store.base_skill_level_like_cpp(area_store, area_id))
}

pub fn remove_represented_stealth_or_invisibility_auras_by_type_like_cpp<
    H: GameObjectUseHostLikeCpp,
>(
    host: &mut H,
) -> Option<usize> {
    let visible_auras = host.hub_ref().resolved_player_visible_auras_like_cpp()?;
    let slots: Vec<u8> = visible_auras
        .iter()
        .filter_map(|(slot, aura)| {
            matches!(
                aura.represented_effect,
                Some(RepresentedAuraEffectLikeCpp::Stealth)
                    | Some(RepresentedAuraEffectLikeCpp::Invisibility)
            )
            .then_some(*slot)
        })
        .collect();
    let removed = slots.len();
    for slot in slots {
        let _ = host.remove_aura(slot);
    }
    Some(removed)
}

pub fn record_represented_capture_point_update_like_cpp<H: GameObjectUseHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    source: wow_entities::CapturePointUseSource,
    state: RepresentedCapturePointStateLikeCpp,
    broadcast_text_id: u32,
    event_id: u32,
    assault_timer_ms: u32,
) {
    let (custom_anim, spell_visual_id) = state.custom_anim_and_spell_visual_like_cpp(source);
    if let Some(position) = host
        .world_entities()
        .represented_gameobject_use_state_like_cpp(gameobject_guid)
        .and_then(|state| state.position)
    {
        host.send_packet(&wow_packet::packets::misc::UpdateCapturePoint {
            guid: gameobject_guid,
            position,
            state: state.packet_state_like_cpp(),
            capture_time_ms: assault_timer_ms,
            capture_total_duration_ms: source.capture_time_ms,
        });
    }
    host.world_entities_mut()
        .record_represented_gameobject_use_effect_like_cpp(
            RepresentedGameObjectUseEffect::CapturePointUpdated {
                gameobject_guid,
                state,
                broadcast_text_id,
                event_id,
                world_state_id: source.world_state_id,
                spell_visual_id,
                custom_anim,
                assault_timer_ms,
            },
        );
}

pub fn apply_represented_new_flag_state_command_like_cpp<H: GameObjectUseHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: Option<ObjectGuid>,
    new_state: RepresentedNewFlagStateRequest,
    respawn_time_ms: u32,
) -> bool {
    let state = host
        .world_entities_mut()
        .ensure_represented_gameobject_use_state_like_cpp(gameobject_guid);
    let old_state = state
        .new_flag_state
        .unwrap_or(RepresentedNewFlagStateRequest::InBase);
    if old_state == new_state {
        return false;
    }

    state.new_flag_state = Some(new_state);
    state.new_flag_carrier_guid = if new_state == RepresentedNewFlagStateRequest::Taken {
        player_guid
    } else {
        None
    };

    if new_state == RepresentedNewFlagStateRequest::Taken
        && old_state == RepresentedNewFlagStateRequest::InBase
    {
        state.new_flag_taken_from_base_game_time_ms =
            Some(wow_world_core::session::game_time_ms_like_cpp());
    } else if matches!(
        new_state,
        RepresentedNewFlagStateRequest::InBase | RepresentedNewFlagStateRequest::Respawning
    ) {
        state.new_flag_taken_from_base_game_time_ms = None;
    }

    state.new_flag_respawn_until = (new_state == RepresentedNewFlagStateRequest::Respawning)
        .then(|| Instant::now() + Duration::from_millis(u64::from(respawn_time_ms)));

    host.world_entities_mut()
        .record_represented_gameobject_use_effect_like_cpp(
            RepresentedGameObjectUseEffect::NewFlagOwnerStateRequested {
                gameobject_guid,
                player_guid: player_guid.unwrap_or(ObjectGuid::EMPTY),
                state: new_state,
            },
        );
    true
}

#[allow(dead_code)]
pub fn record_represented_gameobject_owner_guid_like_cpp<H: GameObjectUseHostLikeCpp>(
    host: &mut H,
    guid: ObjectGuid,
    owner_guid: ObjectGuid,
) {
    let (state, mut hub) = host.split_world_entities_mut();
    state.record_represented_gameobject_owner_guid_like_cpp(&mut hub, guid, owner_guid)
}

pub fn represented_gameobject_is_friendly_to_player_like_cpp<H: GameObjectUseHostLikeCpp>(
    host: &H,
    gameobject_guid: ObjectGuid,
) -> Option<bool> {
    let (state, hub) = host.split_world_entities_ref();
    state.represented_gameobject_is_friendly_to_player_like_cpp(hub, gameobject_guid)
}

pub fn send_represented_gameobject_despawn_to_visible_set_like_cpp<H: GameObjectUseHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
) {
    use wow_packet::ServerPacket;

    let packet = wow_packet::packets::misc::GameObjectDespawn {
        object_guid: gameobject_guid,
    };
    host.send_packet(&packet);
    let _ = host
        .queue_visible_gameobject_packet_for_same_map_like_cpp(gameobject_guid, packet.to_bytes());
}

pub fn send_represented_gameobject_delete_packets_like_cpp<H: GameObjectUseHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
) {
    send_represented_gameobject_despawn_to_visible_set_like_cpp(host, gameobject_guid);
    host.world_entities_mut()
        .restore_represented_gameobject_override_flags_like_cpp(gameobject_guid);
    let map_id = host
        .world_entities()
        .represented_gameobject_use_state_like_cpp(gameobject_guid)
        .and_then(|state| state.map_id)
        .unwrap_or_else(|| host.core().player_map_id_like_cpp());
    host.send_packet(&wow_packet::packets::update::UpdateObject::destroy_objects(
        vec![gameobject_guid],
        map_id,
    ));
}
