//! Serialize completed movement only after releasing the canonical manager.

use wow_core::{ObjectGuid, Position};
use wow_entities::{CreatureAiState, UnitValuesUpdate};
use wow_map::{CreatureMovementSource, CreatureMovementStep};
use wow_packet::ServerPacket;
use crate::map_manager::{RecipientRule, RuntimeEvent, RuntimePlan};
use crate::session::creature_movement_adapter::movement_monster_spline_from_move_spline_like_cpp;
use crate::session::unit_values_update_to_update_object;

/// The three Actor facts used by the existing movement trace.
pub(super) struct MovementTrace {
    pub(super) entry: u32,
    pub(super) map_id: u32,
    pub(super) state: CreatureAiState,
}

pub(super) fn append_completed_movement(
    plan: &mut RuntimePlan,
    guid: ObjectGuid,
    map_id: u16,
    instance_id: u32,
    position: Position,
    visibility_range: f32,
    movement: Option<CreatureMovementStep>,
    home_health_update: Option<UnitValuesUpdate>,
    trace: MovementTrace,
) -> bool {
    // The legacy adapter serializes/traces movement before projecting the
    // pending home-health update, but inserts the durable health event first.
    let packet_bytes = movement.map(|movement| serialize_movement(guid, movement, &trace));
    if let Some(health) = home_health_update
        && let Some(update) = unit_values_update_to_update_object(guid, map_id, &health)
    {
        plan.events.push(RuntimeEvent {
            source_guid: guid,
            recipients: RecipientRule::NearbyVisibleDurable {
                source_guid: guid, map_id, instance_id, source_position: position,
                range: visibility_range, required_3d: false,
            },
            packet_bytes: update.to_bytes(),
        });
    }
    if let Some(packet_bytes) = packet_bytes {
        plan.events.push(RuntimeEvent {
            source_guid: guid,
            recipients: RecipientRule::NearbyVisible {
                source_guid: guid, map_id, instance_id, source_position: position,
                range: visibility_range, required_3d: false,
            },
            packet_bytes,
        });
        true
    } else {
        false
    }
}

fn serialize_movement(
    guid: ObjectGuid,
    movement: CreatureMovementStep,
    trace: &MovementTrace,
) -> Vec<u8> {
    use wow_packet::packets::movement::{MonsterMove, MonsterMoveStop};
    match movement {
        CreatureMovementStep::Stop(stop) => MonsterMoveStop {
            mover_guid: guid, current_pos: stop.position, spline_id: stop.spline_id,
        }.to_bytes(),
        CreatureMovementStep::Launch { source, from, spline } => {
            let source = match source {
                CreatureMovementSource::Home => "home",
                CreatureMovementSource::Random => "random",
                CreatureMovementSource::Waypoint => "waypoint",
                CreatureMovementSource::Chase => "chase",
            };
            let packet_spline = movement_monster_spline_from_move_spline_like_cpp(&spline);
            let packet = MonsterMove { mover_guid: guid, current_pos: from, spline: packet_spline.clone() };
            let bytes = packet.to_bytes();
            trace_launch(source, guid, trace, &spline, &packet_spline, &bytes);
            bytes
        }
    }
}

fn trace_launch(
    source: &'static str,
    guid: ObjectGuid,
    creature: &MovementTrace,
    move_spline: &wow_movement::MoveSpline,
    packet_spline: &wow_packet::packets::movement::MovementMonsterSpline,
    bytes: &[u8],
) {
    if std::env::var_os("RUSTYCORE_MONSTER_MOVE_TRACE").is_none() {
        return;
    }
    let hex_len = bytes.len().min(160);
    let hex = bytes[..hex_len].iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ");
    let suffix = if bytes.len() > hex_len { " ..." } else { "" };
    let path_points = move_spline.create_object_path_points_like_cpp();
    let (path_z_min, path_z_max) = path_points.iter().fold(
        (f32::INFINITY, f32::NEG_INFINITY),
        |(min_z, max_z), point| (min_z.min(point.z), max_z.max(point.z)),
    );
    let path_z_min = path_z_min.is_finite().then_some(path_z_min);
    let path_z_max = path_z_max.is_finite().then_some(path_z_max);
    tracing::info!(
        source, ?guid, high = ?guid.high_type(), realm = guid.realm_id(),
        server = guid.server_id(), map = guid.map_id(), entry = guid.entry(), counter = guid.counter(),
        creature_entry = creature.entry, creature_map = creature.map_id, creature_state = ?creature.state,
        flags = packet_spline.movement.flags, move_time = packet_spline.movement.move_time,
        points = packet_spline.movement.points.len(), packed_deltas = packet_spline.movement.packed_deltas.len(),
        face = ?packet_spline.movement.face, spline_id = packet_spline.id,
        spline_duration = move_spline.duration_ms(), spline_flags = move_spline.flags().bits(),
        spline_final = ?move_spline.final_destination(), spline_path_points = path_points.len(),
        spline_path_z_min = ?path_z_min, spline_path_z_max = ?path_z_max,
        packet_len = bytes.len(), packet_hex = format!("{hex}{suffix}"), "RUST_MONSTER_MOVE"
    );
}
