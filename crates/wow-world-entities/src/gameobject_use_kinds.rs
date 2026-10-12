// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Per-type represented gameobject use handlers (#1363 slice 4a; moved from the
//! World `session/world_entities/gameobject_use_types.rs`).

use wow_core::{ObjectGuid, Position};

use crate::gameobject_use_preamble::apply_represented_gameobject_post_use_spell_like_cpp;
use crate::world_entities_host::WorldEntitiesHostLikeCpp;
use crate::{RepresentedGameObjectSpellCaster, RepresentedGameObjectUseEffect};

pub fn use_represented_gameobject_chair_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
    player_position: Position,
    gameobject_position: Position,
    gameobject_size: f32,
    source: wow_entities::ChairUseSource,
) -> bool {
    let slot_count = source.chair_slots.max(1).min(5);
    let state = host
        .world_entities_mut()
        .ensure_represented_gameobject_use_state_like_cpp(gameobject_guid);
    if state.chair_slots.is_empty() {
        state.chair_slots = vec![None; slot_count as usize];
    }

    let orthogonal_orientation = gameobject_position.orientation + std::f32::consts::PI * 0.5;
    let mut nearest_slot = None;
    let mut lowest_dist = f32::MAX;
    let mut nearest_position = gameobject_position;
    for slot in 0..state.chair_slots.len() {
        if state.chair_slots[slot].is_some() {
            continue;
        }

        let relative_distance =
            (gameobject_size * slot as f32) - (gameobject_size * (slot_count - 1) as f32 / 2.0);
        let candidate = Position::new(
            gameobject_position.x + relative_distance * orthogonal_orientation.cos(),
            gameobject_position.y + relative_distance * orthogonal_orientation.sin(),
            gameobject_position.z,
            gameobject_position.orientation,
        );
        let dist = player_position.distance_2d(&candidate);
        if dist <= lowest_dist {
            nearest_slot = Some(slot);
            lowest_dist = dist;
            nearest_position = candidate;
        }
    }

    let Some(slot) = nearest_slot else {
        host.world_entities_mut()
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::ChairNoFreeSlot {
                    gameobject_guid,
                    player_guid,
                },
            );
        return false;
    };

    state.chair_slots[slot] = Some(player_guid);
    let stand_state = 4_u32.saturating_add(source.chair_height);
    host.hub_mut()
        .set_player_position_like_cpp(nearest_position);
    host.hub_mut()
        .set_player_stand_state_like_cpp(wow_entities::chair_stand_state_like_cpp(
            source.chair_height,
        ));
    host.world_entities_mut()
        .record_represented_gameobject_use_effect_like_cpp(
            RepresentedGameObjectUseEffect::ChairUsed {
                gameobject_guid,
                player_guid,
                slot: slot as u32,
                teleport_position: nearest_position,
                stand_state,
            },
        );
    if source.triggered_event_id != 0 {
        host.world_entities_mut()
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::TriggerGameEvent {
                    gameobject_guid,
                    player_guid,
                    event_id: source.triggered_event_id,
                },
            );
    }

    true
}

pub fn use_represented_gameobject_barber_chair_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
    gameobject_position: Position,
    source: wow_entities::BarberChairUseSource,
) -> bool {
    host.send_packet(&wow_packet::packets::misc::EnableBarberShop {
        customization_scope: source.customization_scope.min(u32::from(u8::MAX)) as u8,
    });
    host.world_entities_mut()
        .record_represented_gameobject_use_effect_like_cpp(
            RepresentedGameObjectUseEffect::BarberChairUsed {
                gameobject_guid,
                player_guid,
                customization_scope: source.customization_scope,
                teleport_position: gameobject_position,
                stand_state: 4_u32.saturating_add(source.chair_height),
                sit_anim_kit: source.sit_anim_kit,
            },
        );
    host.hub_mut()
        .set_player_position_like_cpp(gameobject_position);
    host.hub_mut()
        .set_player_stand_state_like_cpp(wow_entities::chair_stand_state_like_cpp(
            source.chair_height,
        ));

    true
}

pub fn use_represented_gameobject_ui_link_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
    source: wow_entities::UiLinkUseSource,
) -> bool {
    let (state, mut hub) = host.split_world_entities_mut();
    state.use_represented_gameobject_ui_link_like_cpp(
        &mut hub,
        gameobject_guid,
        player_guid,
        source,
    )
}

pub fn use_represented_gameobject_spellcaster_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
    gameobject_entry: u32,
    source: wow_entities::SpellcasterUseSource,
) -> bool {
    if source.party_only {
        let owner_guid = {
            let (s, h) = host.split_world_entities_ref();
            s.represented_or_canonical_gameobject_owner_guid_like_cpp(h, gameobject_guid)
        };
        if !owner_guid.is_some_and(|owner_guid| {
            host.represented_player_is_same_raid_with_like_cpp(player_guid, owner_guid)
        }) {
            host.world_entities_mut()
                .record_represented_gameobject_use_effect_like_cpp(
                    RepresentedGameObjectUseEffect::SpellcasterPartyOnlyRejected {
                        gameobject_guid,
                        player_guid,
                    },
                );
            return false;
        }
    }

    if !host.remove_represented_mounted_auras_by_type_like_cpp() {
        return false;
    }
    host.world_entities_mut()
        .record_represented_gameobject_use_effect_like_cpp(
            RepresentedGameObjectUseEffect::RemoveMountedAuras {
                gameobject_guid,
                player_guid,
            },
        );

    let use_count = {
        let state = host
            .world_entities_mut()
            .ensure_represented_gameobject_use_state_like_cpp(gameobject_guid);
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_SPELLCASTER as u8);
        if source.charges != 0 {
            state.max_charges = Some(source.charges);
        }
        state.use_count = state.use_count.saturating_add(1);
        state.use_count
    };
    host.world_entities_mut()
        .record_represented_gameobject_use_effect_like_cpp(
            RepresentedGameObjectUseEffect::GameObjectUseCountIncremented {
                gameobject_guid,
                use_count,
            },
        );

    apply_represented_gameobject_post_use_spell_like_cpp(
        host,
        gameobject_guid,
        player_guid,
        gameobject_entry,
        wow_entities::GAMEOBJECT_TYPE_SPELLCASTER,
        source.spell_id,
        false,
        RepresentedGameObjectSpellCaster::User,
        player_guid,
    );

    true
}

pub fn use_represented_gameobject_camera_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
    source: wow_entities::CameraUseSource,
) -> bool {
    if source.cinematic_id != 0 {
        host.send_represented_cinematic_start_like_cpp(source.cinematic_id);
        host.world_entities_mut()
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::TriggerCinematic {
                    gameobject_guid,
                    player_guid,
                    cinematic_id: source.cinematic_id,
                },
            );
    }

    if source.event_id != 0 {
        host.world_entities_mut()
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::TriggerGameEvent {
                    gameobject_guid,
                    player_guid,
                    event_id: source.event_id,
                },
            );
    }

    true
}
