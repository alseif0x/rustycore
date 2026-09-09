//! Navmesh FFI wrapper state definitions, part 2 of 4.
//!
//! Separated from the lib.rs root under #658. Behaviour is preserved.

use super::*;

/// C++ `PathGenerator::GetPathPolyByPosition` (`PathGenerator.cpp:94-123`).
///
/// Scans the *current* corridor for the polygon closest to `point`, breaking
/// early once its 3D squared distance is below `1.0`, and accepts the result
/// only while that squared distance is below the literal `3.0` (an effective
/// linear radius of `sqrt(3)`, not 3 yards). The returned distance is the real
/// distance to the closest polygon even when the reference is rejected,
/// matching C++ writing `*distance` before its `minDist < 3.0f` test.
///
/// The audited legacy source and TrinityCore's `3.3.5`, `cata_classic` and
/// `master` branches all retain this squared comparison. Changing it to `9.0`
/// would be a plausible optimization, but would also change corridor selection
/// around close stacked surfaces without evidence that C++ intended a
/// three-yard linear radius, so this port preserves the observable heuristic.
pub fn get_path_poly_by_position_like_cpp(
    query: &DetourNavMeshQuery<'_>,
    poly_path: &[DetourPolyRef],
    point: [f32; 3],
) -> (DetourPolyRef, f32) {
    if poly_path.is_empty() {
        return (0, f32::MAX);
    }

    let mut nearest_poly = 0;
    let mut min_dist_sq = f32::MAX;
    for poly_ref in poly_path.iter().copied() {
        let Ok((closest, _)) = query.closest_point_on_poly(poly_ref, point) else {
            continue;
        };
        let dist_sq = detour_distance_sq(point, closest);
        if dist_sq < min_dist_sq {
            min_dist_sq = dist_sq;
            nearest_poly = poly_ref;
        }
        if min_dist_sq < 1.0 {
            break;
        }
    }

    let distance = min_dist_sq.sqrt();
    if min_dist_sq < PATH_POLY_ACCEPT_DISTANCE_SQ_LIKE_CPP {
        (nearest_poly, distance)
    } else {
        (0, distance)
    }
}

/// C++ `PathGenerator::GetPolyByLocation` (`PathGenerator.cpp:125-158`).
///
/// It checks the current corridor first and only falls back to the expensive
/// `findNearestPoly`, first with a low search box and then with a taller one.
pub fn get_poly_by_location_with_previous_path_like_cpp(
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    previous_poly_refs: &[DetourPolyRef],
    point: [f32; 3],
) -> Result<(DetourPolyRef, f32), DetourNavMeshQueryError> {
    let (from_path, path_distance) =
        get_path_poly_by_position_like_cpp(query, previous_poly_refs, point);
    if from_path != 0 {
        return Ok((from_path, path_distance));
    }

    let low = query.find_nearest_poly(point, [3.0, 5.0, 3.0], filter)?;
    if low.poly_ref != 0 {
        return Ok((low.poly_ref, detour_distance(low.nearest_point, point)));
    }

    let high = query.find_nearest_poly(point, [3.0, 50.0, 3.0], filter)?;
    if high.poly_ref != 0 {
        return Ok((high.poly_ref, detour_distance(high.nearest_point, point)));
    }

    Ok((0, f32::MAX))
}

/// [`get_poly_by_location_with_previous_path_like_cpp`] with no prior corridor,
/// i.e. the C++ behaviour on a freshly constructed `PathGenerator`.
pub fn get_poly_by_location_like_cpp(
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    point: [f32; 3],
) -> Result<(DetourPolyRef, f32), DetourNavMeshQueryError> {
    get_poly_by_location_with_previous_path_like_cpp(query, filter, &[], point)
}

/// Internal result of C++ `PathGenerator::BuildPointPath`
/// (`PathGenerator.cpp:530-622`).
///
/// `BuildShortcut()` calls `Clear()` before installing the two direct points
/// (`PathGenerator.h:117-121`, `PathGenerator.cpp:630-645`), so point data
/// cannot be returned without its corridor-lifecycle decision.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BuildPointPathOutcomeLikeCpp {
    pub(crate) point_path: DetourPointPath,
    pub(crate) cleared_poly_path: bool,
}

/// `end_point` is the (possibly clamped) `endPoint` C++ passes as an argument
/// and queries against, while `requested_end_point` is `GetEndPosition()` — the
/// destination `CalculatePath` was originally asked for. The `_forceDestination`
/// block compares against the latter (`PathGenerator.cpp:603-619`), so the two
/// must stay distinct whenever the far-from-poly branch clamped the endpoint.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_point_path_outcome_like_cpp(
    nav_mesh: &DetourNavMesh,
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    start_point: [f32; 3],
    end_point: [f32; 3],
    requested_end_point: [f32; 3],
    poly_refs: &[DetourPolyRef],
    point_path_limit: usize,
    mut path_type: DetourPathType,
    force_destination: bool,
    use_straight_path: bool,
    use_raycast: bool,
) -> Result<BuildPointPathOutcomeLikeCpp, DetourNavMeshQueryError> {
    if use_raycast {
        return Ok(BuildPointPathOutcomeLikeCpp {
            point_path: DetourPointPath {
                points: vec![start_point, end_point],
                actual_end: end_point,
                path_type: DetourPathType::NOPATH,
            },
            cleared_poly_path: true,
        });
    }

    let point_result = if use_straight_path {
        query
            .find_straight_path(start_point, end_point, poly_refs, point_path_limit, 0)
            .map(|points| points.into_iter().map(|point| point.position).collect())
    } else {
        find_smooth_path_like_cpp(
            nav_mesh,
            query,
            filter,
            start_point,
            end_point,
            poly_refs,
            point_path_limit,
        )
    };

    // C++ `BuildShortcut()` *assigns* `_type = PATHFIND_SHORTCUT`
    // (`PathGenerator.cpp:645`) and the failure branches then OR onto that
    // fresh value, so the incoming `NORMAL`/`INCOMPLETE`/`FARFROMPOLY_*` bits
    // are discarded rather than merged (`PathGenerator.cpp:575-591`).
    let mut points = match point_result {
        Ok(points) => points,
        Err(_) => {
            return Ok(BuildPointPathOutcomeLikeCpp {
                point_path: DetourPointPath {
                    points: vec![start_point, end_point],
                    actual_end: end_point,
                    path_type: DetourPathType::SHORTCUT | DetourPathType::NOPATH,
                },
                cleared_poly_path: true,
            });
        }
    };

    if poly_refs.len() == 1 && points.len() == 1 {
        points.push(end_point);
    } else if points.len() < 2 {
        return Ok(BuildPointPathOutcomeLikeCpp {
            point_path: DetourPointPath {
                points: vec![start_point, end_point],
                actual_end: end_point,
                path_type: DetourPathType::SHORTCUT | DetourPathType::NOPATH,
            },
            cleared_poly_path: true,
        });
    } else if points.len() >= point_path_limit {
        return Ok(BuildPointPathOutcomeLikeCpp {
            point_path: DetourPointPath {
                points: vec![start_point, end_point],
                actual_end: end_point,
                path_type: DetourPathType::SHORTCUT | DetourPathType::SHORT,
            },
            cleared_poly_path: true,
        });
    }

    // C++ `SetActualEndPosition(_pathPoints[pointCount-1])`.
    let mut actual_end = points.last().copied().unwrap_or(end_point);
    let mut cleared_poly_path = false;
    if force_destination
        && (!path_type.contains(DetourPathType::NORMAL)
            || !detour_in_range(requested_end_point, actual_end, 1.0, 1.0))
    {
        actual_end = requested_end_point;
        if detour_distance_sq(
            points.last().copied().unwrap_or(start_point),
            requested_end_point,
        ) < 0.3 * detour_distance_sq(start_point, requested_end_point)
        {
            if let Some(last) = points.last_mut() {
                *last = requested_end_point;
            }
        } else {
            // C++ `BuildShortcut()`: current position -> actual end position,
            // which the branch above has just set to the requested destination.
            points = vec![start_point, requested_end_point];
            cleared_poly_path = true;
        }
        path_type = DetourPathType::NORMAL | DetourPathType::NOT_USING_PATH;
    }

    Ok(BuildPointPathOutcomeLikeCpp {
        point_path: DetourPointPath {
            points,
            actual_end,
            path_type,
        },
        cleared_poly_path,
    })
}

pub fn reuse_previous_poly_path_like_cpp(
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    previous_poly_refs: &[DetourPolyRef],
    start_poly: DetourPolyRef,
    end_poly: DetourPolyRef,
    end_point: [f32; 3],
    use_raycast: bool,
) -> Result<PreviousPolyPathLikeCpp, DetourNavMeshQueryError> {
    if previous_poly_refs.is_empty() {
        return Ok(PreviousPolyPathLikeCpp::Recalculate);
    }

    let Some(path_start_index) = previous_poly_refs
        .iter()
        .position(|poly| *poly == start_poly)
    else {
        return Ok(PreviousPolyPathLikeCpp::Recalculate);
    };

    let path_end_index = previous_poly_refs
        .iter()
        .enumerate()
        .skip(path_start_index + 1)
        .rev()
        .find_map(|(index, poly)| (*poly == end_poly).then_some(index));

    if let Some(path_end_index) = path_end_index {
        return Ok(PreviousPolyPathLikeCpp::PolyRefs(
            previous_poly_refs[path_start_index..=path_end_index].to_vec(),
        ));
    }

    let remaining = &previous_poly_refs[path_start_index..];
    let mut prefix_poly_length = ((remaining.len() as f32) * 0.8 + 0.5) as usize;
    prefix_poly_length = prefix_poly_length.clamp(1, remaining.len());
    let mut prefix = remaining[..prefix_poly_length].to_vec();

    let mut suffix_start_poly = *prefix.last().unwrap();
    let suffix_end_point = match query.closest_point_on_poly(suffix_start_poly, end_point) {
        Ok((closest, _)) => closest,
        Err(_) if prefix.len() > 1 => {
            prefix.pop();
            suffix_start_poly = *prefix.last().unwrap();
            match query.closest_point_on_poly(suffix_start_poly, end_point) {
                Ok((closest, _)) => closest,
                Err(_) => return Ok(PreviousPolyPathLikeCpp::ShortcutNoPath),
            }
        }
        Err(_) => return Ok(PreviousPolyPathLikeCpp::ShortcutNoPath),
    };

    if use_raycast {
        return Ok(PreviousPolyPathLikeCpp::ShortcutNoPath);
    }

    let max_suffix_path = MAX_PATH_LENGTH_LIKE_CPP.saturating_sub(prefix.len());
    let suffix = query
        .find_path(
            suffix_start_poly,
            end_poly,
            suffix_end_point,
            end_point,
            filter,
            max_suffix_path,
        )
        .unwrap_or_default();

    if suffix.is_empty() {
        // C++ deliberately keeps the valid prefix after an empty/failed suffix
        // so the creature can advance and recover on the next update
        // (`PathGenerator.cpp:401-412`). Its final `prefix + suffix - overlap`
        // arithmetic assumes `findPath` returned at least `suffixStartPoly`;
        // with an empty suffix there is no overlap to remove. Reproducing the
        // unconditional `- 1` discards a valid last prefix polygon and can make
        // a remote destination look like a same-poly direct path.
        if prefix.len() == 1 {
            // A singleton cannot provide a useful retained segment. Recalculate
            // rather than reproduce C++'s zero-length underflow.
            return Ok(PreviousPolyPathLikeCpp::Recalculate);
        }
        return Ok(PreviousPolyPathLikeCpp::PolyRefs(prefix));
    }

    prefix.pop();
    prefix.extend(suffix);
    Ok(PreviousPolyPathLikeCpp::PolyRefs(prefix))
}

/// C++ `PathGenerator::BuildPolyPath` (`PathGenerator.cpp:160-528`).
///
/// This produces the *polygon corridor* and `_type`. Following C++, it does
/// **not** build the point path itself: the branches that answer with
/// `BuildShortcut()` return terminal results whose `point_path.points` are
/// already populated, while the branches that fall through to
/// `BuildPointPath(startPoint, endPoint)` (`PathGenerator.cpp:287` and
/// `:527`) return an **empty** `point_path.points` so the caller runs
/// `build_point_path_outcome_like_cpp` exactly once, in the mode
/// `_useStraightPath`/`_useRaycast` selects.
///
/// `point_path.actual_end` carries the possibly clamped `endPoint`, mirroring
/// the `SetActualEndPosition(closestPointOnPoly(endPoly, endPoint))` the
/// far-from-poly branch performs (`PathGenerator.cpp:253-259`); the caller must
/// feed it back into `BuildPointPath` the way C++ passes its mutated local
/// `endPoint`.
pub fn build_straight_poly_path_like_cpp(
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    start_point: [f32; 3],
    mut end_point: [f32; 3],
    owner: DetourOwnerCapabilitiesLikeCpp,
    previous_poly_refs: &[DetourPolyRef],
) -> Result<DetourPolyPath, DetourNavMeshQueryError> {
    // C++ resolves both endpoints through `GetPolyByLocation`, which consults the
    // corridor it already holds before paying for `findNearestPoly`
    // (`PathGenerator.cpp:125-158`); the distance it reports there feeds the
    // `> 7.0f` far-from-poly decision below.
    let (start_poly, dist_to_start_poly) = get_poly_by_location_with_previous_path_like_cpp(
        query,
        filter,
        previous_poly_refs,
        start_point,
    )?;
    let (end_poly, dist_to_end_poly) = get_poly_by_location_with_previous_path_like_cpp(
        query,
        filter,
        previous_poly_refs,
        end_point,
    )?;

    if start_poly == 0 || end_poly == 0 {
        // C++ runs `BuildShortcut()` first, then grants
        // `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH` when the owner `CanFly()`,
        // or `CanSwim()` with every shortcut point in liquid
        // (`PathGenerator.cpp:178-202`). Otherwise, and while `!_useRaycast`, it
        // *assigns* `_type = PATHFIND_NOPATH` (`:207`), discarding the
        // `PATHFIND_SHORTCUT` that `BuildShortcut()` had set.
        //
        // Boundary: `waterPath` needs `Map::GetLiquidStatus` for each shortcut
        // point, which this layer has no access to. A swimming owner therefore
        // still falls through to `PATHFIND_NOPATH` here, exactly as it did
        // before — the C++ `waterPath` loop can only ever *reduce* `waterPath`
        // to false, so treating "no liquid data" as "not fully submerged" never
        // grants a shortcut C++ would have withheld.
        let path_type = if owner.can_fly {
            DetourPathType::NORMAL | DetourPathType::NOT_USING_PATH
        } else {
            DetourPathType::NOPATH
        };
        return Ok(DetourPolyPath {
            poly_refs: Vec::new(),
            point_path: DetourPointPath {
                points: vec![start_point, end_point],
                actual_end: end_point,
                path_type,
            },
            start_far_from_poly: false,
            end_far_from_poly: false,
        });
    }

    let start_far_from_poly = dist_to_start_poly > 7.0;
    let end_far_from_poly = dist_to_end_poly > 7.0;
    let mut path_type = DetourPathType::NORMAL;
    if start_far_from_poly || end_far_from_poly {
        // C++ picks the offending endpoint, and when it is *not* under water
        // allows a direct shortcut for a flying owner, or for one that is
        // falling towards a lower destination — the charge case
        // (`PathGenerator.cpp:221-240`).
        //
        // Boundary: C++ selects the offending endpoint
        // (`(distToStartPoly > 7.0f) ? startPos : endPos`) only to ask
        // `Map::IsUnderWater` about it and, if so, consult `CanSwim()` instead.
        // That liquid lookup is unavailable at this layer, so the "flying case"
        // arm is always taken — the same arm Rust already took unconditionally.
        // This only adds the shortcuts C++ grants inside it.
        //
        // Detour space is Y-up and `wow_position_to_detour_like_cpp` maps WoW z
        // onto index 1, so index 1 is the height C++ compares as `endPos.z <
        // startPos.z`.
        let build_shortcut = owner.can_fly || (owner.is_falling && end_point[1] < start_point[1]);
        if build_shortcut {
            let mut path_type = DetourPathType::NORMAL | DetourPathType::NOT_USING_PATH;
            add_far_from_poly_flags_like_cpp(
                &mut path_type,
                start_far_from_poly,
                end_far_from_poly,
            );
            return Ok(DetourPolyPath {
                poly_refs: Vec::new(),
                point_path: DetourPointPath {
                    points: vec![start_point, end_point],
                    actual_end: end_point,
                    path_type,
                },
                start_far_from_poly,
                end_far_from_poly,
            });
        }

        if let Ok((closest, _)) = query.closest_point_on_poly(end_poly, end_point) {
            end_point = closest;
        }
        path_type = DetourPathType::INCOMPLETE;
        add_far_from_poly_flags_like_cpp(&mut path_type, start_far_from_poly, end_far_from_poly);
    }

    // C++ `BuildShortcut(); _type = PATHFIND_NOPATH;` with no
    // `AddFarFromPolyFlags` afterwards (`PathGenerator.cpp:371-374`, `:508-515`).
    let shortcut_no_path = || DetourPolyPath {
        poly_refs: Vec::new(),
        point_path: DetourPointPath {
            points: vec![start_point, end_point],
            actual_end: end_point,
            path_type: DetourPathType::NOPATH,
        },
        start_far_from_poly,
        end_far_from_poly,
    };

    let poly_refs = if start_poly == end_poly {
        // C++ `PathGenerator.cpp:271-289` treats this as a one-polygon corridor
        // and hands it straight to `BuildPointPath`.
        vec![start_poly]
    } else {
        // C++ `PathGenerator.cpp:291-413` first tries to reuse the corridor the
        // generator already holds: an exact subpath when both polygons are still
        // on it, or an ~80% prefix plus a freshly generated suffix when only the
        // start polygon is.
        match reuse_previous_poly_path_like_cpp(
            query,
            filter,
            previous_poly_refs,
            start_poly,
            end_poly,
            end_point,
            false,
        )? {
            PreviousPolyPathLikeCpp::PolyRefs(reused) if !reused.is_empty() => reused,
            PreviousPolyPathLikeCpp::ShortcutNoPath => return Ok(shortcut_no_path()),
            // C++ `Clear()`s and generates the whole corridor when the retained
            // path does not contain the current start polygon (`:414-516`).
            // `reuse_previous_poly_path_like_cpp` also selects this branch as a
            // deliberate safety repair for C++'s one-prefix/empty-suffix
            // zero-length underflow.
            PreviousPolyPathLikeCpp::Recalculate | PreviousPolyPathLikeCpp::PolyRefs(_) => {
                match query.find_path(
                    start_poly,
                    end_poly,
                    start_point,
                    end_point,
                    filter,
                    MAX_PATH_LENGTH_LIKE_CPP,
                ) {
                    Ok(path) if !path.is_empty() => path,
                    // C++ has already `Clear()`ed the previous corridor in
                    // this branch and turns either a failed status or zero
                    // length into `BuildShortcut(); PATHFIND_NOPATH`.
                    Ok(_) | Err(_) => return Ok(shortcut_no_path()),
                }
            }
        }
    };

    if poly_refs.last().copied() == Some(end_poly)
        && !path_type.contains(DetourPathType::INCOMPLETE)
    {
        path_type = DetourPathType::NORMAL;
    } else {
        path_type = DetourPathType::INCOMPLETE;
    }
    add_far_from_poly_flags_like_cpp(&mut path_type, start_far_from_poly, end_far_from_poly);

    if start_poly != end_poly && poly_refs.len() == 1 {
        // A successful Detour query may still return only `startPoly` when the
        // destination is on a disconnected island or every neighbour is
        // excluded by the filter. C++ passes the remote endpoint into
        // `FindSmoothPath`, whose singleton shortcut treats it as same-poly and
        // appends a final straight segment across the gap. Keep the valid
        // partial result, but clamp its effective endpoint to the reachable
        // boundary. Genuine same-poly paths retain their requested endpoint.
        match query.closest_point_on_poly_boundary(poly_refs[0], end_point) {
            Ok(boundary) => end_point = boundary,
            // C++ `FindSmoothPath` cannot build a usable singleton partial
            // path when the reachable boundary itself is invalid. Fall back
            // to the same cleared shortcut/NOPATH state as its failed point
            // query branches instead of retaining the remote endpoint.
            Err(_) => return Ok(shortcut_no_path()),
        }
    }

    Ok(DetourPolyPath {
        poly_refs,
        point_path: DetourPointPath {
            points: Vec::new(),
            actual_end: end_point,
            path_type,
        },
        start_far_from_poly,
        end_far_from_poly,
    })
}

pub fn build_raycast_poly_path_like_cpp(
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    start_point: [f32; 3],
    mut end_point: [f32; 3],
) -> Result<DetourPolyPath, DetourNavMeshQueryError> {
    let (start_poly, dist_to_start_poly) =
        get_poly_by_location_like_cpp(query, filter, start_point)?;
    let (end_poly, dist_to_end_poly) = get_poly_by_location_like_cpp(query, filter, end_point)?;
    let start_far_from_poly = dist_to_start_poly > 7.0;
    let end_far_from_poly = dist_to_end_poly > 7.0;

    if start_far_from_poly || end_far_from_poly {
        if let Ok((closest, _)) = query.closest_point_on_poly(end_poly, end_point) {
            end_point = closest;
        }
    }

    let raycast = match query.raycast(
        start_poly,
        start_point,
        end_point,
        filter,
        MAX_PATH_LENGTH_LIKE_CPP,
    ) {
        Ok(raycast) if !raycast.path.is_empty() => raycast,
        Ok(raycast) => {
            let mut path_type = DetourPathType::NOPATH | DetourPathType::SHORTCUT;
            add_far_from_poly_flags_like_cpp(
                &mut path_type,
                start_far_from_poly,
                end_far_from_poly,
            );
            return Ok(DetourPolyPath {
                poly_refs: raycast.path,
                point_path: DetourPointPath {
                    points: vec![start_point, end_point],
                    actual_end: end_point,
                    path_type,
                },
                start_far_from_poly,
                end_far_from_poly,
            });
        }
        Err(error) => {
            let mut path_type = DetourPathType::NOPATH | DetourPathType::SHORTCUT;
            add_far_from_poly_flags_like_cpp(
                &mut path_type,
                start_far_from_poly,
                end_far_from_poly,
            );
            if start_poly == 0 {
                return Ok(DetourPolyPath {
                    poly_refs: Vec::new(),
                    point_path: DetourPointPath {
                        points: vec![start_point, end_point],
                        actual_end: end_point,
                        path_type,
                    },
                    start_far_from_poly,
                    end_far_from_poly,
                });
            }
            return Err(error);
        }
    };

    let last_poly = raycast.path.last().copied().unwrap_or(start_poly);
    if raycast.hit_t != f32::MAX {
        let mut hit_t = raycast.hit_t * 0.99;
        if !hit_t.is_finite() {
            hit_t = 0.0;
        }
        let mut hit_pos = detour_lerp(start_point, end_point, hit_t);
        match query.get_poly_height(last_poly, hit_pos) {
            Ok(height) => hit_pos[1] = height,
            Err(_) => {
                if let Ok(boundary) = query.closest_point_on_poly_boundary(last_poly, hit_pos) {
                    hit_pos = boundary;
                }
            }
        }

        let mut path_type = DetourPathType::INCOMPLETE;
        add_far_from_poly_flags_like_cpp(&mut path_type, start_far_from_poly, false);
        return Ok(DetourPolyPath {
            poly_refs: raycast.path,
            point_path: DetourPointPath {
                points: vec![start_point, hit_pos],
                actual_end: hit_pos,
                path_type,
            },
            start_far_from_poly,
            end_far_from_poly,
        });
    }

    match query.get_poly_height(last_poly, end_point) {
        Ok(height) => end_point[1] = height,
        Err(_) => {
            if let Ok(boundary) = query.closest_point_on_poly_boundary(last_poly, end_point) {
                end_point = boundary;
            }
        }
    }

    let mut path_type = if start_far_from_poly || end_far_from_poly {
        DetourPathType::INCOMPLETE
    } else {
        DetourPathType::NORMAL
    };
    add_far_from_poly_flags_like_cpp(&mut path_type, start_far_from_poly, end_far_from_poly);

    Ok(DetourPolyPath {
        poly_refs: raycast.path,
        point_path: DetourPointPath {
            points: vec![start_point, end_point],
            actual_end: end_point,
            path_type,
        },
        start_far_from_poly,
        end_far_from_poly,
    })
}

/// [`calculate_detour_path_with_previous_path_like_cpp`] on a freshly
/// constructed `PathGenerator`, i.e. with no corridor to reuse.
pub fn calculate_detour_path_like_cpp(
    nav_mesh: &DetourNavMesh,
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    start_wow: [f32; 3],
    end_wow: [f32; 3],
    options: DetourPathOptions,
) -> Result<DetourPolyPath, DetourNavMeshQueryError> {
    calculate_detour_path_with_previous_path_like_cpp(
        nav_mesh,
        query,
        filter,
        start_wow,
        end_wow,
        options,
        &[],
    )
}

/// C++ `PathGenerator::CalculatePath` from `BuildPolyPath` onwards, for a
/// generator that kept its `PathGenerator` alive across updates and therefore
/// still holds `_pathPolyRefs` (`PathGenerator.cpp:291-413`).
#[allow(clippy::too_many_arguments)]
pub fn calculate_detour_path_with_previous_path_like_cpp(
    nav_mesh: &DetourNavMesh,
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    start_wow: [f32; 3],
    end_wow: [f32; 3],
    options: DetourPathOptions,
    previous_poly_refs: &[DetourPolyRef],
) -> Result<DetourPolyPath, DetourNavMeshQueryError> {
    let start_point = wow_position_to_detour_like_cpp(start_wow);
    let end_point = wow_position_to_detour_like_cpp(end_wow);
    let mut poly_path = if options.use_raycast {
        build_raycast_poly_path_like_cpp(query, filter, start_point, end_point)?
    } else {
        build_straight_poly_path_like_cpp(
            query,
            filter,
            start_point,
            end_point,
            options.owner,
            previous_poly_refs,
        )?
    };

    // C++ `BuildPolyPath` runs `BuildPointPath` exactly once, and only on the
    // branches that did not already answer with `BuildShortcut()`
    // (`PathGenerator.cpp:287`, `:527`). Those shortcut branches arrive here
    // with their two points already set, so re-running the point build would
    // both waste a Detour query and let the discarded pass leak
    // `PATHFIND_SHORTCUT`/`PATHFIND_SHORT` into an otherwise usable path.
    if !options.use_raycast && poly_path.point_path.points.is_empty() {
        let point_path_outcome = build_point_path_outcome_like_cpp(
            nav_mesh,
            query,
            filter,
            start_point,
            // C++ passes its own possibly clamped local `endPoint`, while
            // `GetEndPosition()` keeps the originally requested destination.
            poly_path.point_path.actual_end,
            end_point,
            &poly_path.poly_refs,
            options.point_path_limit,
            poly_path.point_path.path_type,
            options.force_destination,
            options.use_straight_path,
            false,
        )?;
        poly_path.point_path = point_path_outcome.point_path;
        if point_path_outcome.cleared_poly_path {
            poly_path.poly_refs.clear();
        }
    }

    for point in &mut poly_path.point_path.points {
        *point = detour_position_to_wow_like_cpp(*point);
    }
    poly_path.point_path.actual_end =
        detour_position_to_wow_like_cpp(poly_path.point_path.actual_end);

    Ok(poly_path)
}

pub(crate) fn calculate_detour_path_with_raw_query_like_cpp(
    nav_mesh: &DetourNavMesh,
    raw_query: *mut RawDetourNavMeshQuery,
    filter: &DetourQueryFilter,
    start_wow: [f32; 3],
    end_wow: [f32; 3],
    options: DetourPathOptions,
    previous_poly_refs: &[DetourPolyRef],
) -> Result<DetourPolyPath, DetourNavMeshQueryError> {
    let Some(raw) = NonNull::new(raw_query) else {
        return Err(DetourNavMeshQueryError::AllocationFailed);
    };
    let query = ManuallyDrop::new(DetourNavMeshQuery {
        raw,
        _mesh_lifetime_and_thread_model: PhantomData,
    });

    calculate_detour_path_with_previous_path_like_cpp(
        nav_mesh,
        &query,
        filter,
        start_wow,
        end_wow,
        options,
        previous_poly_refs,
    )
}

#[must_use]
pub fn fixup_corridor_like_cpp(
    path: &[DetourPolyRef],
    max_path: usize,
    visited: &[DetourPolyRef],
) -> Vec<DetourPolyRef> {
    let mut furthest_path = None;
    let mut furthest_visited = None;

    for i in (0..path.len()).rev() {
        let mut found = false;
        for j in (0..visited.len()).rev() {
            if path[i] == visited[j] {
                furthest_path = Some(i);
                furthest_visited = Some(j);
                found = true;
            }
        }
        if found {
            break;
        }
    }

    let (Some(furthest_path), Some(furthest_visited)) = (furthest_path, furthest_visited) else {
        return path.to_vec();
    };

    let req = visited.len() - furthest_visited;
    let orig = (furthest_path + 1).min(path.len());
    let mut size = path.len().saturating_sub(orig);
    if req + size > max_path {
        size = max_path.saturating_sub(req);
    }

    let mut fixed = Vec::with_capacity((req + size).min(max_path));
    fixed.extend((0..req).map(|i| visited[(visited.len() - 1) - i]));
    fixed.extend_from_slice(&path[orig..orig + size]);
    fixed
}

pub fn get_steer_target_like_cpp(
    query: &DetourNavMeshQuery<'_>,
    start_pos: [f32; 3],
    end_pos: [f32; 3],
    min_target_dist: f32,
    path: &[DetourPolyRef],
) -> Result<Option<DetourSteerTarget>, DetourNavMeshQueryError> {
    const MAX_STEER_POINTS: usize = 3;

    let steer_path = query.find_straight_path(start_pos, end_pos, path, MAX_STEER_POINTS, 0)?;
    if steer_path.is_empty() {
        return Ok(None);
    }

    let steer = steer_path.into_iter().find(|point| {
        point.flags & DT_STRAIGHTPATH_OFFMESH_CONNECTION_LIKE_CPP != 0
            || !detour_in_range(point.position, start_pos, min_target_dist, 1000.0)
    });

    Ok(steer.map(|point| {
        let mut position = point.position;
        position[1] = start_pos[1];
        DetourSteerTarget {
            position,
            flags: point.flags,
            poly_ref: point.poly_ref,
        }
    }))
}
