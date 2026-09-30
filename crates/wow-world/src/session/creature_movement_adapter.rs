// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature movement adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::info;

/// Converts the authoritative movement spline into the `MonsterMove` wire
/// representation. Packet serialization stays in `wow-packet`; this adapter
/// owns the boundary from the movement runtime model to that packet model.
pub(in crate::session) fn movement_monster_spline_from_move_spline_like_cpp(
    move_spline: &wow_movement::MoveSpline,
) -> wow_packet::packets::movement::MovementMonsterSpline {
    use wow_movement::MoveSplineFlag;
    use wow_packet::packets::movement::{
        MonsterMoveFace, MonsterSplineAnimTierTransition, MonsterSplineJumpExtraData,
        MonsterSplineSpellEffectExtraData, MovementMonsterSpline, MovementSpline,
    };

    let mut flags = move_spline.flags();
    if move_spline.is_cyclic() {
        flags.insert(MoveSplineFlag::ENTER_CYCLE);
    }
    flags.remove(MoveSplineFlag::MASK_NO_MONSTER_MOVE);

    let path_data = move_spline.monster_move_path_data();
    let facing = move_spline.facing();
    MovementMonsterSpline {
        id: move_spline.id(),
        // C++ `MonsterMove::InitializeSplineData` leaves
        // `MovementMonsterSpline::Destination` at its default value for
        // SMSG_ON_MONSTER_MOVE; only the nested MovementSpline path carries
        // the destination.
        destination: wow_core::Position::ZERO,
        movement: MovementSpline {
            flags: flags.bits(),
            face: match facing.kind {
                wow_movement::MonsterMoveType::Normal => MonsterMoveFace::Normal,
                wow_movement::MonsterMoveType::FacingSpot => {
                    MonsterMoveFace::FacingSpot(facing.spot)
                }
                wow_movement::MonsterMoveType::FacingTarget => MonsterMoveFace::FacingTarget {
                    direction: facing.angle,
                    target_guid: facing.target,
                },
                wow_movement::MonsterMoveType::FacingAngle => {
                    MonsterMoveFace::FacingAngle(facing.angle)
                }
            },
            move_time: move_spline.duration_ms().max(0) as u32,
            fade_object_time: if flags.contains(MoveSplineFlag::FADE_OBJECT) {
                move_spline.effect_start_time_ms().max(0) as u32
            } else {
                0
            },
            points: path_data.points,
            packed_deltas: path_data.packed_deltas,
            spell_effect_extra: move_spline.spell_effect_extra().map(|data| {
                MonsterSplineSpellEffectExtraData {
                    target_guid: data.target,
                    spell_visual_id: data.spell_visual_id,
                    progress_curve_id: data.progress_curve_id,
                    parabolic_curve_id: data.parabolic_curve_id,
                    jump_gravity: move_spline.vertical_acceleration(),
                }
            }),
            jump_extra: (flags.contains(MoveSplineFlag::PARABOLIC)
                && (move_spline.spell_effect_extra().is_none()
                    || move_spline.effect_start_time_ms() != 0))
                .then(|| MonsterSplineJumpExtraData {
                    jump_gravity: move_spline.vertical_acceleration(),
                    start_time: move_spline.effect_start_time_ms().max(0) as u32,
                    duration: 0,
                }),
            anim_tier_transition: (flags.contains(MoveSplineFlag::ANIMATION))
                .then_some(move_spline.anim_tier())
                .flatten()
                .map(|anim_tier| MonsterSplineAnimTierTransition {
                    tier_transition_id: anim_tier.tier_transition_id as i32,
                    start_time: move_spline.effect_start_time_ms().max(0) as u32,
                    end_time: 0,
                    anim_tier: anim_tier.anim_tier,
                }),
            ..MovementSpline::default()
        },
        ..MovementMonsterSpline::default()
    }
}

// ── Creature movement step helper ────────────────────────────────

/// Maps a bridge-built [`CreaturePathQueryLikeCpp`] onto a worker request.
///
/// The map/instance/phase identity belongs to the tick, while every
/// query-sensitive input — filter, owner reads, retained corridor and
/// `forceDest` — is supplied by the generator bridge at query time, so it cannot
/// be sampled before the bridge's own state transitions.
pub(in crate::session) fn creature_path_request_like_cpp(
    query: crate::map_manager::CreaturePathQueryLikeCpp,
    source_map_id: u32,
    source_instance_id: u32,
    phase_shift: &wow_entities::PhaseShift,
) -> crate::map_manager::WorldMMapPathRequestLikeCpp {
    crate::map_manager::WorldMMapPathRequestLikeCpp {
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
pub(in crate::session) fn resolve_creature_detour_path_like_cpp(
    mmap_pathfinder: Option<&crate::map_manager::WorldMMapPathfinderWorkerLikeCpp>,
    guid: wow_core::ObjectGuid,
    request: crate::map_manager::WorldMMapPathRequestLikeCpp,
) -> Option<wow_recastdetour::DetourPolyPath> {
    let start = request.start;
    let destination = request.destination;
    let Some(worker) = mmap_pathfinder else {
        return Some(crate::map_manager::detour_path_without_navmesh_like_cpp(
            start,
            destination,
        ));
    };

    match worker.calculate_path_like_cpp(request) {
        Ok(Some(path)) => Some(path),
        Ok(None) => Some(crate::map_manager::detour_path_without_navmesh_like_cpp(
            start,
            destination,
        )),
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

pub(in crate::session) fn trace_monster_move_packet_like_cpp(
    source: &'static str,
    guid: wow_core::ObjectGuid,
    creature: &crate::map_manager::WorldCreature,
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

#[cfg(test)]
mod tests {
    use super::movement_monster_spline_from_move_spline_like_cpp;
    use wow_core::{ObjectGuid, Position};
    use wow_movement::{
        AnimTierTransition, FacingInfo, MonsterMoveType, MoveSpline, MoveSplineFlag,
        MoveSplineInitArgs, SpellEffectExtraData,
    };
    use wow_packet::packets::movement::{
        MonsterMoveFace, MonsterSplineAnimTierTransition, MonsterSplineJumpExtraData,
        MonsterSplineSpellEffectExtraData,
    };

    #[test]
    fn movement_monster_spline_from_move_spline_matches_cpp_mapping() {
        let target = ObjectGuid::create_player(1, 55);
        let args = MoveSplineInitArgs {
            path: vec![
                Position::xyz(0.0, 0.0, 0.0),
                Position::xyz(10.0, 0.0, 0.0),
                Position::xyz(20.0, 0.0, 0.0),
            ],
            facing: FacingInfo {
                kind: MonsterMoveType::FacingTarget,
                target,
                angle: 1.75,
                ..FacingInfo::default()
            },
            flags: MoveSplineFlag::UNCOMPRESSED_PATH | MoveSplineFlag::PARABOLIC,
            velocity: 10.0,
            vertical_acceleration: 12.5,
            effect_start_time_ms: 250,
            spline_id: 123,
            spell_effect_extra: Some(SpellEffectExtraData {
                target,
                spell_visual_id: 777,
                progress_curve_id: 888,
                parabolic_curve_id: 999,
            }),
            ..MoveSplineInitArgs::default()
        };
        let mut move_spline = MoveSpline::new();
        move_spline.initialize(&args).unwrap();
        move_spline.finalize();

        let packet_spline = movement_monster_spline_from_move_spline_like_cpp(&move_spline);

        assert_eq!(packet_spline.id, 123);
        assert_eq!(packet_spline.destination, Position::ZERO);
        assert_eq!(
            packet_spline.movement.flags,
            (MoveSplineFlag::UNCOMPRESSED_PATH | MoveSplineFlag::PARABOLIC).bits()
        );
        assert_eq!(
            packet_spline.movement.face,
            MonsterMoveFace::FacingTarget {
                direction: 1.75,
                target_guid: target,
            }
        );
        assert_eq!(
            packet_spline.movement.points,
            vec![Position::xyz(10.0, 0.0, 0.0), Position::xyz(20.0, 0.0, 0.0)]
        );
        assert!(packet_spline.movement.packed_deltas.is_empty());
        assert_eq!(
            packet_spline.movement.spell_effect_extra,
            Some(MonsterSplineSpellEffectExtraData {
                target_guid: target,
                spell_visual_id: 777,
                progress_curve_id: 888,
                parabolic_curve_id: 999,
                jump_gravity: 12.5,
            })
        );
        assert_eq!(
            packet_spline.movement.jump_extra,
            Some(MonsterSplineJumpExtraData {
                jump_gravity: 12.5,
                start_time: 250,
                duration: 0,
            })
        );
    }

    #[test]
    fn movement_monster_spline_from_move_spline_maps_animation_tier_like_cpp() {
        let mut flags = MoveSplineFlag::empty();
        flags.enable_animation();
        let args = MoveSplineInitArgs {
            path: vec![Position::xyz(0.0, 0.0, 0.0), Position::xyz(10.0, 0.0, 0.0)],
            flags,
            velocity: 10.0,
            effect_start_time_ms: 125,
            anim_tier: Some(AnimTierTransition {
                tier_transition_id: 44,
                anim_tier: 2,
            }),
            ..MoveSplineInitArgs::default()
        };
        let mut move_spline = MoveSpline::new();
        move_spline.initialize(&args).unwrap();

        let packet_spline = movement_monster_spline_from_move_spline_like_cpp(&move_spline);

        assert_eq!(
            packet_spline.movement.anim_tier_transition,
            Some(MonsterSplineAnimTierTransition {
                tier_transition_id: 44,
                start_time: 125,
                end_time: 0,
                anim_tier: 2,
            })
        );
        assert!(packet_spline.movement.jump_extra.is_none());
    }
}
