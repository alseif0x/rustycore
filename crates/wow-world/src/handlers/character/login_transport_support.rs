// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character-login transport and create-block support.

use super::{
    GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_LIKE_CPP, GameObjectCreateData, HighGuid, ItemCreateData,
    ObjectGuid, PI, PlayerCombatStats, PlayerLoginTransportLoadRowLikeCpp, Position, PowerType,
    TAXI_PATH_NODE_FLAG_STOP_LIKE_CPP, TAXI_PATH_NODE_FLAG_TELEPORT_LIKE_CPP, TaxiPathNodeEntry,
    UpdateBlock, UpdateObject, UpdateType,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MapTransportCreateLikeCpp {
    pub(super) guid_low: u32,
    pub(super) entry: u32,
    pub(super) display_id: u32,
    pub(super) scale: f32,
    pub(super) taxi_path_id: u16,
    pub(super) move_speed: u32,
    pub(super) accel_rate: u32,
    pub(super) allow_stopping: bool,
    pub(super) phase_use_flags: u8,
    pub(super) phase_id: u16,
    pub(super) phase_group_id: u32,
    pub(super) gameobject_flags: u32,
    pub(super) faction_template: i32,
}

pub(super) fn map_transport_create_from_load_row_like_cpp(
    row: PlayerLoginTransportLoadRowLikeCpp,
) -> MapTransportCreateLikeCpp {
    MapTransportCreateLikeCpp {
        guid_low: row.guid_low,
        entry: row.entry,
        phase_use_flags: row.phase_use_flags,
        phase_id: row.phase_id,
        phase_group_id: row.phase_group_id,
        display_id: row.display_id,
        scale: row.scale,
        taxi_path_id: row.taxi_path_id,
        move_speed: row.move_speed,
        accel_rate: row.accel_rate,
        allow_stopping: row.allow_stopping,
        gameobject_flags: row.gameobject_flags,
        faction_template: row.faction_template,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TransportCreatePositionLikeCpp {
    pub(super) map_id: u16,
    pub(super) position: Position,
    pub(super) timer_ms: u32,
    pub(super) total_time_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PersistedTransportLoginLikeCpp {
    pub(super) guid: ObjectGuid,
    pub(super) map_id: u16,
    pub(super) offset: Position,
    pub(super) world_position: Position,
    pub(super) transport_position: TransportCreatePositionLikeCpp,
    pub(super) transport_create: MapTransportCreateLikeCpp,
}

pub(super) fn validate_persisted_transport_login_like_cpp(
    guid: ObjectGuid,
    offset: Position,
    transport_position: TransportCreatePositionLikeCpp,
    transport_create: MapTransportCreateLikeCpp,
) -> Option<PersistedTransportLoginLikeCpp> {
    // C++ Player::LoadFromDB first converts the saved passenger offset to
    // world coordinates, then rejects invalid world coordinates and transport
    // offsets outside the hard ±250-yard transport-size limit.
    if !offset.x.is_finite()
        || !offset.y.is_finite()
        || !offset.z.is_finite()
        || !offset.orientation.is_finite()
        || offset.x.abs() > 250.0
        || offset.y.abs() > 250.0
        || offset.z.abs() > 250.0
    {
        return None;
    }

    let world_position =
        wow_entities::calculate_passenger_position(offset, transport_position.position);
    world_position
        .is_valid_map_coord_like_cpp()
        .then_some(PersistedTransportLoginLikeCpp {
            guid,
            map_id: transport_position.map_id,
            offset,
            world_position,
            transport_position,
            transport_create,
        })
}

pub(super) fn transport_route_contains_saved_map_like_cpp(
    route_map_ids: impl IntoIterator<Item = u16>,
    saved_map_id: u16,
) -> bool {
    route_map_ids
        .into_iter()
        .any(|map_id| map_id == saved_map_id)
}

pub(super) fn map_transport_create_block_like_cpp(
    transport: MapTransportCreateLikeCpp,
    path_position: TransportCreatePositionLikeCpp,
    now_ms: u32,
) -> UpdateBlock {
    let transport_guid =
        ObjectGuid::create_transport(HighGuid::Transport, transport.guid_low as i64);
    let path_progress =
        ((path_position.timer_ms as f32 / path_position.total_time_ms as f32) * 65535.0) as u32;
    let create_data = GameObjectCreateData {
        guid: transport_guid,
        entry: transport.entry,
        dynamic_flags: path_progress << 16,
        display_id: transport.display_id,
        go_type: GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_LIKE_CPP,
        position: path_position.position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        anim_progress: 255,
        state: if transport.allow_stopping {
            wow_entities::GoState::Active as i8
        } else {
            wow_entities::GoState::Ready as i8
        },
        art_kit: 0,
        created_by: ObjectGuid::EMPTY,
        faction_template: transport.faction_template,
        // Transport.cpp + GameObject flags:
        // GO_FLAG_TRANSPORT | GO_FLAG_NODESPAWN | GO_FLAG_MAP_OBJECT.
        gameobject_flags: transport.gameobject_flags | 0x0010_0028,
        world_effect_id: 0,
        scale: transport.scale,
        level: path_position.total_time_ms,
        parent_rotation: [0.0, 0.0, 0.0, 1.0],
    };
    UpdateObject::create_transport_block(create_data, now_ms)
}

#[derive(Default)]
pub(super) struct InitTransportsPlanLikeCpp {
    pub(super) own_transport: Option<(ObjectGuid, UpdateBlock)>,
    pub(super) other_blocks: Vec<UpdateBlock>,
    pub(super) other_visible_guids: Vec<ObjectGuid>,
    pub(super) considered: usize,
    pub(super) skipped_other_map: usize,
    pub(super) skipped_missing_path: usize,
    pub(super) skipped_phase: usize,
}

pub(crate) fn player_visibility_create_update_from_snapshot_like_cpp(
    player: &crate::session::directory::PlayerVisibilityCreateSnapshot,
    map_id: u16,
) -> UpdateObject {
    let max_mana = if player.power_type == PowerType::Mana as u8 {
        i64::from(player.max_power)
    } else {
        0
    };
    let combat = PlayerCombatStats {
        health: i64::from(player.current_health),
        max_health: i64::from(player.max_health),
        base_mana: player.base_mana,
        max_mana,
        ..PlayerCombatStats::default()
    };
    let mut update = UpdateObject::create_player_with_party_type(
        player.guid,
        player.race,
        player.class,
        player.sex,
        player.level,
        player.display_id,
        &player.position,
        map_id,
        player.zone_id,
        false,
        *player.visible_items,
        [ObjectGuid::EMPTY; 141],
        combat,
        Vec::new(),
        0,
        Vec::new(),
        player.party_member_party_type,
    );
    update.set_player_current_power0_like_cpp(i32::from(player.current_power));
    update.set_player_customizations_like_cpp(player.customizations.as_ref().clone());
    if let Some(transport) = player.transport.clone() {
        update.set_player_movement_transport_like_cpp(transport);
    }
    update
}

pub(super) fn compose_init_self_create_blocks_like_cpp(
    player_update: &mut UpdateObject,
    item_creates: Vec<ItemCreateData>,
    own_transport: Option<(ObjectGuid, UpdateBlock)>,
    fellow_passenger_blocks: Vec<UpdateBlock>,
) -> Option<ObjectGuid> {
    if item_creates.is_empty() && own_transport.is_none() && fellow_passenger_blocks.is_empty() {
        return None;
    }

    let mut blocks = Vec::with_capacity(
        usize::from(own_transport.is_some())
            + item_creates.len()
            + player_update.blocks.len()
            + fellow_passenger_blocks.len(),
    );
    let own_transport_guid = own_transport.map(|(guid, block)| {
        blocks.push(block);
        guid
    });
    blocks.extend(
        item_creates
            .into_iter()
            .map(|create_data| UpdateBlock::CreateItem {
                update_type: UpdateType::CreateObject,
                guid: create_data.item_guid,
                create_data,
            }),
    );
    blocks.append(&mut player_update.blocks);
    blocks.extend(fellow_passenger_blocks);
    player_update.blocks = blocks;
    player_update.num_updates = player_update.blocks.len() as u32;
    own_transport_guid
}

pub(super) fn object_guid_from_db_binary_like_cpp(raw: Vec<u8>) -> ObjectGuid {
    let Ok(bytes) = <[u8; 16]>::try_from(raw.as_slice()) else {
        return ObjectGuid::EMPTY;
    };
    ObjectGuid::from_raw_bytes(&bytes)
}

fn distance_3d_like_cpp(a: &TaxiPathNodeEntry, b: &TaxiPathNodeEntry) -> f32 {
    let dx = b.loc.x - a.loc.x;
    let dy = b.loc.y - a.loc.y;
    let dz = b.loc.z - a.loc.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn movement_time_ms_for_transport_segment_like_cpp(
    distance: f32,
    speed: u32,
    accel_rate: u32,
    accel_from_pause: bool,
) -> u32 {
    if distance <= 0.0 {
        return 0;
    }

    let speed = speed.max(1) as f32;
    let accel = accel_rate.max(1) as f32;
    if accel_from_pause {
        let accel_dist = 0.5 * speed * speed / accel;
        if accel_dist >= distance {
            ((distance * 2.0 / accel).sqrt() * 1000.0) as u32
        } else {
            (((distance - accel_dist) / speed + speed / accel) * 1000.0) as u32
        }
    } else {
        (distance / speed * 1000.0) as u32
    }
}

pub(super) fn transport_position_for_login_like_cpp(
    nodes: &[TaxiPathNodeEntry],
    move_speed: u32,
    accel_rate: u32,
    now_ms: u32,
) -> Option<TransportCreatePositionLikeCpp> {
    let mut sorted_nodes = nodes.to_vec();
    sorted_nodes.sort_by_key(|node| node.node_index);
    if sorted_nodes.len() < 2 {
        return None;
    }

    let mut legs: Vec<Vec<TaxiPathNodeEntry>> = Vec::new();
    let mut current_leg: Vec<TaxiPathNodeEntry> = Vec::new();
    let mut current_map = sorted_nodes[0].continent_id;
    let mut prev_node_was_teleport = false;

    for node in sorted_nodes {
        if !current_leg.is_empty() && (node.continent_id != current_map || prev_node_was_teleport) {
            legs.push(std::mem::take(&mut current_leg));
            current_map = node.continent_id;
        }
        prev_node_was_teleport = (node.flags & TAXI_PATH_NODE_FLAG_TELEPORT_LIKE_CPP) != 0;
        current_leg.push(node);
    }
    if !current_leg.is_empty() {
        legs.push(current_leg);
    }

    let mut leg_durations: Vec<u32> = Vec::with_capacity(legs.len());
    let mut total_time_ms = 0u32;
    for leg in &legs {
        let mut duration = 0u32;
        let mut accel_from_pause = false;
        for segment in leg.windows(2) {
            let distance = distance_3d_like_cpp(&segment[0], &segment[1]);
            duration = duration.saturating_add(movement_time_ms_for_transport_segment_like_cpp(
                distance,
                move_speed,
                accel_rate,
                accel_from_pause,
            ));
            let stop_delay = if (segment[1].flags & TAXI_PATH_NODE_FLAG_STOP_LIKE_CPP) != 0 {
                segment[1].delay.saturating_mul(1000)
            } else {
                0
            };
            if stop_delay > 0 {
                duration = duration.saturating_add(stop_delay);
                accel_from_pause = true;
            } else {
                accel_from_pause = false;
            }
        }
        leg_durations.push(duration);
        total_time_ms = total_time_ms.saturating_add(duration);
    }

    if total_time_ms == 0 {
        return None;
    }

    let timer_ms = now_ms % total_time_ms;
    let mut leg_start_ms = 0u32;
    for (leg, leg_duration) in legs.iter().zip(leg_durations.iter().copied()) {
        let leg_end_ms = leg_start_ms.saturating_add(leg_duration);
        if timer_ms >= leg_end_ms {
            leg_start_ms = leg_end_ms;
            continue;
        }

        let mut leg_elapsed_ms = timer_ms.saturating_sub(leg_start_ms);
        let mut accel_from_pause = false;
        for segment in leg.windows(2) {
            let from = &segment[0];
            let to = &segment[1];
            let distance = distance_3d_like_cpp(from, to);
            let move_time = movement_time_ms_for_transport_segment_like_cpp(
                distance,
                move_speed,
                accel_rate,
                accel_from_pause,
            )
            .max(1);

            if leg_elapsed_ms <= move_time {
                let pct = (leg_elapsed_ms as f32 / move_time as f32).clamp(0.0, 1.0);
                let x = from.loc.x + (to.loc.x - from.loc.x) * pct;
                let y = from.loc.y + (to.loc.y - from.loc.y) * pct;
                let z = from.loc.z + (to.loc.z - from.loc.z) * pct;
                let orientation = (to.loc.y - from.loc.y).atan2(to.loc.x - from.loc.x) + PI;
                return Some(TransportCreatePositionLikeCpp {
                    map_id: from.continent_id,
                    position: Position::new(x, y, z, orientation),
                    timer_ms,
                    total_time_ms,
                });
            }
            leg_elapsed_ms = leg_elapsed_ms.saturating_sub(move_time);

            let stop_delay = if (to.flags & TAXI_PATH_NODE_FLAG_STOP_LIKE_CPP) != 0 {
                to.delay.saturating_mul(1000)
            } else {
                0
            };
            if stop_delay > 0 {
                if leg_elapsed_ms <= stop_delay {
                    let orientation = (to.loc.y - from.loc.y).atan2(to.loc.x - from.loc.x) + PI;
                    return Some(TransportCreatePositionLikeCpp {
                        map_id: to.continent_id,
                        position: Position::new(to.loc.x, to.loc.y, to.loc.z, orientation),
                        timer_ms,
                        total_time_ms,
                    });
                }
                leg_elapsed_ms = leg_elapsed_ms.saturating_sub(stop_delay);
                accel_from_pause = true;
            } else {
                accel_from_pause = false;
            }
        }

        if let Some(last) = leg.last() {
            return Some(TransportCreatePositionLikeCpp {
                map_id: last.continent_id,
                position: Position::new(last.loc.x, last.loc.y, last.loc.z, 0.0),
                timer_ms,
                total_time_ms,
            });
        }
    }

    None
}
