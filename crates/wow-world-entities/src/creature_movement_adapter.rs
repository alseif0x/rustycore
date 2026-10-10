// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature movement adapter over the map worker contracts.
//!
//! Session-independent: moved out of `wow-world` under #1263 F5.

use tracing::info;

// ── Creature movement step helper ────────────────────────────────

/// Maps a bridge-built [`CreaturePathQueryLikeCpp`] onto a worker request.
///
/// The map/instance/phase identity belongs to the tick, while every
/// query-sensitive input — filter, owner reads, retained corridor and
/// `forceDest` — is supplied by the generator bridge at query time, so it cannot
/// be sampled before the bridge's own state transitions.
pub fn creature_path_request_like_cpp(
    query: wow_world_core::map_manager::CreaturePathQueryLikeCpp,
    source_map_id: u32,
    source_instance_id: u32,
    phase_shift: &wow_entities::PhaseShift,
) -> wow_world_core::map_manager::WorldMMapPathRequestLikeCpp {
    wow_world_core::map_manager::WorldMMapPathRequestLikeCpp {
        start: query.start,
        destination: query.destination,
        mesh_map_id: source_map_id,
        instance_map_id: source_map_id,
        instance_id: source_instance_id,
        filter_context: query.filter_context,
        owner: query.owner,
        previous_poly_refs: query.previous_poly_refs,
        force_destination: query.force_destination,
        point_path_limit: query.point_path_limit,
        phase_shift: phase_shift.clone(),
    }
}

/// Resolves one creature path request through the off-thread Detour worker with
/// C++ `PathGenerator::CalculatePath` semantics.
///
/// A missing worker, or `Ok(None)` from it, both mean "this map has no usable
/// navmesh for this query" — no `.mmap` map data, no per-instance
/// `dtNavMeshQuery`, or no `.mmtile` covering the endpoints. C++
/// `PathGenerator::CalculatePath` (`PathGenerator.cpp:79-86`) answers that with
/// `BuildShortcut()` and `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH`, i.e. a
/// launchable direct path rather than a failure, so creatures keep moving on
/// unmeshed terrain instead of retrying forever.
///
/// A query error has no C++ counterpart (C++ would already be inside
/// `BuildPolyPath`, which answers failures with `BuildShortcut()` +
/// `PATHFIND_NOPATH`), so it stays a failure and the caller retries like the
/// C++ `!result` / `PATHFIND_NOPATH` branch.
pub fn resolve_creature_detour_path_like_cpp(
    mmap_pathfinder: Option<&wow_world_core::map_manager::WorldMMapPathfinderWorkerLikeCpp>,
    guid: wow_core::ObjectGuid,
    request: wow_world_core::map_manager::WorldMMapPathRequestLikeCpp,
) -> Option<wow_recastdetour::DetourPolyPath> {
    let start = request.start;
    let destination = request.destination;
    let Some(worker) = mmap_pathfinder else {
        return Some(
            wow_world_core::map_manager::detour_path_without_navmesh_like_cpp(start, destination),
        );
    };

    match worker.calculate_path_like_cpp(request) {
        Ok(Some(path)) => Some(path),
        Ok(None) => Some(
            wow_world_core::map_manager::detour_path_without_navmesh_like_cpp(start, destination),
        ),
        Err(error) => {
            tracing::warn!(
                "mmap pathfinding failed for creature {:?}: {:?}",
                guid,
                error
            );
            None
        }
    }
}

pub fn trace_monster_move_packet_like_cpp(
    source: &'static str,
    guid: wow_core::ObjectGuid,
    creature: &wow_entities::Creature,
    move_spline: &wow_movement::MoveSpline,
    packet_spline: &wow_packet::packets::movement::MovementMonsterSpline,
    bytes: &[u8],
) {
    if std::env::var_os("RUSTYCORE_MONSTER_MOVE_TRACE").is_none() {
        return;
    }

    let hex_len = bytes.len().min(160);
    let hex = bytes[..hex_len]
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    let suffix = if bytes.len() > hex_len { " ..." } else { "" };
    let path_points = move_spline.create_object_path_points_like_cpp();
    let (path_z_min, path_z_max) = path_points.iter().fold(
        (f32::INFINITY, f32::NEG_INFINITY),
        |(min_z, max_z), point| (min_z.min(point.z), max_z.max(point.z)),
    );
    let path_z_min = path_z_min.is_finite().then_some(path_z_min);
    let path_z_max = path_z_max.is_finite().then_some(path_z_max);

    info!(
        source,
        ?guid,
        high = ?guid.high_type(),
        realm = guid.realm_id(),
        server = guid.server_id(),
        map = guid.map_id(),
        entry = guid.entry(),
        counter = guid.counter(),
        creature_entry = creature.entry(),
        creature_map = creature.map_id(),
        creature_state = ?creature.state(),
        flags = packet_spline.movement.flags,
        move_time = packet_spline.movement.move_time,
        points = packet_spline.movement.points.len(),
        packed_deltas = packet_spline.movement.packed_deltas.len(),
        face = ?packet_spline.movement.face,
        spline_id = packet_spline.id,
        spline_duration = move_spline.duration_ms(),
        spline_flags = move_spline.flags().bits(),
        spline_final = ?move_spline.final_destination(),
        spline_path_points = path_points.len(),
        spline_path_z_min = ?path_z_min,
        spline_path_z_max = ?path_z_max,
        packet_len = bytes.len(),
        packet_hex = format!("{hex}{suffix}"),
        "RUST_MONSTER_MOVE"
    );
}
