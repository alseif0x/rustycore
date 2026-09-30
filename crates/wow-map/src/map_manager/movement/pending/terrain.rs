//! Lazy scalar height requests, followed by actor capability reads on resume.

use super::*;

pub(super) enum GroundProgress {
    Complete(Position),
    Static(StaticGroundRequest),
    Grid(GridGroundRequest),
}

pub(super) struct StaticGroundRequest {
    pub(super) map_id: u32,
    pub(super) point: Position,
    pub(super) probe_z: f32,
}

pub(super) struct GridGroundRequest {
    pub(super) map_id: u32,
    pub(super) point: Position,
    ground: f32,
}

pub(super) fn prepare_ground(
    actor: &WorldCreature,
    point: Position,
    enabled: bool,
) -> GroundProgress {
    if !enabled {
        return GroundProgress::Complete(point);
    }
    let probe_z = point.z + Z_OFFSET_FIND_HEIGHT;
    GroundProgress::Static(StaticGroundRequest {
        map_id: actor.map_id(),
        point,
        probe_z,
    })
}

impl StaticGroundRequest {
    pub(super) fn resume(self, actor: &WorldCreature, static_ground: f32) -> GroundProgress {
        // C++ GetMapHeight combines terrain and VMap before
        // UpdateAllowedPositionZ clamps the point. Rust does not yet have the
        // VMap half, so lowering a valid elevated Detour point to terrain
        // destroys bridge/platform paths. Preserve elevations; the branch
        // below still raises points that are under known terrain.
        let ground = if static_ground >= self.point.z {
            static_ground
        } else {
            INVALID_HEIGHT
        };
        if ground <= INVALID_HEIGHT {
            GroundProgress::Grid(GridGroundRequest {
                map_id: actor.map_id(),
                point: self.point,
                ground,
            })
        } else {
            GroundProgress::Complete(finish_ground(actor, self.point, ground))
        }
    }
}

impl GridGroundRequest {
    pub(super) fn resume(self, actor: &WorldCreature, grid_ground: f32) -> Position {
        let mut ground = self.ground;
        if grid_ground > INVALID_HEIGHT
            && self.point.z < grid_ground
            && grid_ground - self.point.z <= DEFAULT_HEIGHT_SEARCH
        {
            ground = grid_ground;
        }
        finish_ground(actor, self.point, ground)
    }
}

fn finish_ground(actor: &WorldCreature, point: Position, ground: f32) -> Position {
    // These owner reads deliberately follow the static/grid height responses.
    let z = allowed_position_z_from_ground_like_cpp(
        true,
        ground,
        point.z,
        actor.allowed_position_z_caps_like_cpp(),
    );
    Position::new(point.x, point.y, z, point.orientation)
}

pub(in crate::map_manager::movement) fn normalize_sync(
    actor: &WorldCreature,
    point: Position,
    terrain: Option<&LiveTerrainHeights>,
) -> Position {
    let mut progress = prepare_ground(actor, point, terrain.is_some());
    loop {
        progress = match progress {
            GroundProgress::Complete(point) => return point,
            GroundProgress::Static(request) => {
                let height = terrain
                    .expect("height requests require terrain")
                    .static_height_like_cpp(
                        request.map_id,
                        request.point.x,
                        request.point.y,
                        request.probe_z,
                    );
                request.resume(actor, height)
            }
            GroundProgress::Grid(request) => {
                let height = terrain
                    .expect("height requests require terrain")
                    .grid_height_like_cpp(request.map_id, request.point.x, request.point.y);
                GroundProgress::Complete(request.resume(actor, height))
            }
        };
    }
}

pub(in crate::map_manager::movement) struct StaticHeightRequest {
    pub(super) request: StaticGroundRequest,
    purpose: TerrainPurpose,
}

pub(in crate::map_manager::movement) struct GridHeightRequest {
    pub(super) request: GridGroundRequest,
    purpose: TerrainPurpose,
}

impl StaticHeightRequest {
    pub(in crate::map_manager::movement) fn resume(
        self,
        actor: &mut WorldCreature,
        height: f32,
    ) -> MovementProgress {
        let progress = self.request.resume(actor, height);
        continue_ground(actor, progress, self.purpose)
    }
}

impl GridHeightRequest {
    pub(in crate::map_manager::movement) fn resume(
        self,
        actor: &mut WorldCreature,
        height: f32,
    ) -> MovementProgress {
        let point = self.request.resume(actor, height);
        normalized(actor, point, self.purpose)
    }
}

pub(super) fn normalize(
    actor: &mut WorldCreature,
    point: Position,
    enabled: bool,
    purpose: TerrainPurpose,
) -> MovementProgress {
    let progress = prepare_ground(actor, point, enabled);
    continue_ground(actor, progress, purpose)
}

fn continue_ground(
    actor: &mut WorldCreature,
    progress: GroundProgress,
    purpose: TerrainPurpose,
) -> MovementProgress {
    match progress {
        GroundProgress::Complete(point) => normalized(actor, point, purpose),
        GroundProgress::Static(request) => {
            MovementProgress::Pending(PendingMovement::StaticHeight(StaticHeightRequest {
                request,
                purpose,
            }))
        }
        GroundProgress::Grid(request) => {
            MovementProgress::Pending(PendingMovement::GridHeight(GridHeightRequest {
                request,
                purpose,
            }))
        }
    }
}

fn normalized(
    actor: &mut WorldCreature,
    point: Position,
    purpose: TerrainPurpose,
) -> MovementProgress {
    match purpose {
        TerrainPurpose::Home(destination) => home::destination_ready(actor, destination, point),
        TerrainPurpose::Chase(destination) => chase::destination_ready(actor, destination, point),
        TerrainPurpose::Path(path) => path.normalized(actor, point),
    }
}

pub(super) enum PathFinish {
    Home(home::Launch),
    Random(random::Launch),
    Waypoint(waypoint::Launch),
    Chase(chase::Launch),
}

pub(super) struct PathNormalization {
    detour: DetourPolyPath,
    path: PathGenerator,
    start: Position,
    destination: Position,
    force_destination: bool,
    actual_end: Option<Position>,
    next_point: usize,
    finish: PathFinish,
}

pub(super) fn normalize_path(
    actor: &mut WorldCreature,
    destination: Position,
    detour: DetourPolyPath,
    force_destination: bool,
    enabled: bool,
    finish: PathFinish,
) -> MovementProgress {
    let start = actor.position();
    if !enabled {
        let path = path_generator_from_detour_with_normalizer_like_cpp(
            start,
            destination,
            &detour,
            force_destination,
            |point| point,
        );
        return path_ready(actor, finish, path, detour);
    }
    let path = PathGenerator::new();
    let point = from_detour(detour.point_path.actual_end);
    let pending = PathNormalization {
        detour,
        path,
        start,
        destination,
        force_destination,
        actual_end: None,
        next_point: 0,
        finish,
    };
    normalize(actor, point, true, TerrainPurpose::Path(pending))
}

impl PathNormalization {
    fn normalized(mut self, actor: &mut WorldCreature, point: Position) -> MovementProgress {
        if self.actual_end.is_none() {
            self.actual_end = Some(point);
        } else {
            // Reuse the owned query result's point buffer. No second point
            // collection or actor/generator snapshot is needed across I/O.
            self.detour.point_path.points[self.next_point - 1] = [point.x, point.y, point.z];
        }
        if let Some(raw) = self.detour.point_path.points.get(self.next_point).copied() {
            self.next_point += 1;
            return normalize(actor, from_detour(raw), true, TerrainPurpose::Path(self));
        }
        self.path.apply_detour_path_like_cpp(
            self.start,
            self.destination,
            self.actual_end.expect("actual_end is normalized first"),
            self.detour
                .point_path
                .points
                .iter()
                .copied()
                .map(from_detour),
            &self.detour.poly_refs,
            path_type_from_detour_like_cpp(self.detour.point_path.path_type),
            self.force_destination,
        );
        path_ready(actor, self.finish, self.path, self.detour)
    }
}

fn from_detour(point: [f32; 3]) -> Position {
    Position::new(point[0], point[1], point[2], 0.0)
}

fn path_ready(
    actor: &mut WorldCreature,
    finish: PathFinish,
    path: PathGenerator,
    detour: DetourPolyPath,
) -> MovementProgress {
    // Only Chase retains this owned query result beyond normalization, and its
    // launch reads only poly_refs. The original raw points are not returned.
    match finish {
        PathFinish::Home(launch) => home::launch(actor, launch, Some(path)),
        PathFinish::Random(launch) => random::launch(actor, launch, Some(path)),
        PathFinish::Waypoint(launch) => waypoint::launch(actor, launch, Some(path)),
        PathFinish::Chase(launch) => chase::launch(actor, launch, path, Some(detour)),
    }
}
