//! World-object visibility and location model operations, part 1 of 1.
//!
//! The inherent `WorldObject` impl is divided by responsibility under
//! #648; every method keeps its original body.

use super::*;

impl WorldObject {
    pub fn new(is_world_object: bool, type_id: TypeId, type_mask: TypeMask) -> Self {
        Self {
            object: EntityObject::new(type_id, type_mask),
            location: WorldLocation::default(),
            instance_id: 0,
            has_current_map: false,
            phase_shift: PhaseShift::default(),
            suppressed_phase_shift: PhaseShift::default(),
            db_phase: 0,
            name: String::new(),
            is_active: false,
            is_far_visible: false,
            visibility_distance_override: None,
            is_world_object,
            static_floor_z: INVALID_HEIGHT,
            zone_id: 0,
            area_id: 0,
            combat_reach: 0.0,
            collision_height_like_cpp: 0.0,
            current_cell: None,
            smooth_phasing: None,
        }
    }
    pub const fn object(&self) -> &EntityObject {
        &self.object
    }
    pub fn object_mut(&mut self) -> &mut EntityObject {
        &mut self.object
    }
    pub const fn guid(&self) -> ObjectGuid {
        self.object.guid()
    }
    pub const fn map_id(&self) -> u32 {
        self.location.map_id()
    }
    pub const fn instance_id(&self) -> u32 {
        self.instance_id
    }
    pub const fn position(&self) -> Position {
        self.location.position()
    }
    pub fn relocate(&mut self, position: Position) {
        self.location.relocate(position);
    }
    pub fn world_relocate(&mut self, map_id: u32, position: Position) {
        self.location.world_relocate(map_id, position);
        self.object.bind_map(map_id, self.instance_id);
    }
    pub fn set_map(&mut self, map_id: u32, instance_id: u32) -> Result<(), MapBindingError> {
        if self.object.is_in_world() {
            return Err(MapBindingError::ObjectInWorld);
        }

        if self.has_current_map {
            if self.map_id() == map_id && self.instance_id == instance_id {
                return Ok(());
            }
            return Err(MapBindingError::AlreadyBound {
                old_map_id: self.map_id(),
                old_instance_id: self.instance_id,
                new_map_id: map_id,
                new_instance_id: instance_id,
            });
        }

        self.has_current_map = true;
        self.location.map_id = map_id;
        self.instance_id = instance_id;
        self.object.bind_map(map_id, instance_id);
        Ok(())
    }
    pub fn reset_map(&mut self) -> Result<(), MapBindingError> {
        if !self.has_current_map {
            return Err(MapBindingError::NoCurrentMap);
        }
        if self.object.is_in_world() {
            return Err(MapBindingError::ObjectInWorld);
        }

        self.has_current_map = false;
        self.current_cell = None;
        self.object.set_grid_presence(false);
        Ok(())
    }
    pub const fn has_current_map(&self) -> bool {
        self.has_current_map
    }
    pub const fn phase_shift(&self) -> &PhaseShift {
        &self.phase_shift
    }
    pub fn phase_shift_mut(&mut self) -> &mut PhaseShift {
        &mut self.phase_shift
    }
    pub const fn suppressed_phase_shift(&self) -> &PhaseShift {
        &self.suppressed_phase_shift
    }
    pub fn suppressed_phase_shift_mut(&mut self) -> &mut PhaseShift {
        &mut self.suppressed_phase_shift
    }
    pub const fn db_phase(&self) -> i32 {
        self.db_phase
    }
    pub fn set_db_phase(&mut self, db_phase: i32) {
        self.db_phase = db_phase;
    }
    pub fn in_same_phase(&self, other: &Self) -> bool {
        self.phase_shift.can_see(&other.phase_shift)
    }
    pub fn set_current_cell(&mut self, cell_x: u32, cell_y: u32) {
        self.current_cell = Some((cell_x, cell_y));
        self.object.set_grid_presence(true);
    }
    pub fn clear_current_cell(&mut self) {
        self.current_cell = None;
        self.object.set_grid_presence(false);
    }
    pub const fn current_cell(&self) -> Option<(u32, u32)> {
        self.current_cell
    }
    pub const fn smooth_phasing_like_cpp(&self) -> Option<&SmoothPhasingLikeCpp> {
        self.smooth_phasing.as_ref()
    }
    pub fn smooth_phasing_mut_like_cpp(&mut self) -> Option<&mut SmoothPhasingLikeCpp> {
        self.smooth_phasing.as_mut()
    }
    pub fn get_or_create_smooth_phasing_like_cpp(&mut self) -> &mut SmoothPhasingLikeCpp {
        self.smooth_phasing
            .get_or_insert_with(SmoothPhasingLikeCpp::default)
    }
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub const fn is_active(&self) -> bool {
        self.is_active
    }
    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
    }
    pub const fn is_far_visible(&self) -> bool {
        self.is_far_visible
    }
    pub fn set_far_visible(&mut self, far_visible: bool) {
        self.is_far_visible = far_visible;
    }
    pub const fn visibility_distance_override_like_cpp(&self) -> Option<f32> {
        self.visibility_distance_override
    }
    pub fn set_visibility_distance_override_like_cpp(
        &mut self,
        visibility_type: VisibilityDistanceTypeLikeCpp,
    ) {
        if self.is_player() {
            return;
        }
        self.visibility_distance_override = Some(visibility_type.distance_like_cpp());
    }
    pub const fn is_world_object(&self) -> bool {
        self.is_world_object
    }
    pub const fn static_floor_z(&self) -> f32 {
        self.static_floor_z
    }
    pub fn set_static_floor_z(&mut self, static_floor_z: f32) {
        self.static_floor_z = static_floor_z;
    }
    pub const fn zone_id(&self) -> u32 {
        self.zone_id
    }
    pub const fn area_id(&self) -> u32 {
        self.area_id
    }
    pub fn set_zone_and_area(&mut self, zone_id: u32, area_id: u32) {
        self.zone_id = zone_id;
        self.area_id = area_id;
    }
    pub const fn combat_reach(&self) -> f32 {
        self.combat_reach
    }
    pub fn set_combat_reach(&mut self, combat_reach: f32) {
        self.combat_reach = combat_reach.max(0.0);
    }
    pub const fn collision_height_like_cpp(&self) -> f32 {
        self.collision_height_like_cpp
    }
    pub fn set_collision_height_like_cpp(&mut self, height: f32) {
        self.collision_height_like_cpp = height.max(0.0);
    }
    pub fn get_hit_sphere_point_for_like_cpp(&self, dest: Position) -> LineOfSightEndpoint {
        let height = self.collision_height_like_cpp();
        let mut contact = self.position();
        contact.z += height;

        let dx = dest.x - contact.x;
        let dy = dest.y - contact.y;
        let dz = dest.z - contact.z;
        let len = (dx * dx + dy * dy + dz * dz).sqrt();
        let move_dist = dest.distance(&self.position()).min(self.combat_reach());
        let hit_sphere_adjusted = len > 0.0 && move_dist > 0.0;
        if hit_sphere_adjusted {
            contact.x += dx / len * move_dist;
            contact.y += dy / len * move_dist;
            contact.z += dz / len * move_dist;
        }
        contact.orientation = self.absolute_angle_to_position(contact);

        LineOfSightEndpoint {
            position: contact,
            collision_height_adjusted: height > 0.0,
            hit_sphere_adjusted,
        }
    }
    pub fn exact_distance(&self, other: &Self) -> f32 {
        self.position().distance(&other.position())
    }
    pub fn exact_distance_2d(&self, other: &Self) -> f32 {
        self.position().distance_2d(&other.position())
    }
    pub fn distance(&self, other: &Self) -> f32 {
        (self.exact_distance(other) - self.combat_reach - other.combat_reach).max(0.0)
    }
    pub fn distance_to_position(&self, position: Position) -> f32 {
        (self.position().distance(&position) - self.combat_reach).max(0.0)
    }
    pub fn distance_2d(&self, other: &Self) -> f32 {
        (self.exact_distance_2d(other) - self.combat_reach - other.combat_reach).max(0.0)
    }
    pub fn distance_z(&self, other: &Self) -> f32 {
        ((self.position().z - other.position().z).abs() - self.combat_reach - other.combat_reach)
            .max(0.0)
    }
    pub fn is_in_map(&self, other: &Self) -> bool {
        self.object.is_in_world()
            && other.object.is_in_world()
            && self.has_current_map
            && other.has_current_map
            && self.map_id() == other.map_id()
            && self.instance_id == other.instance_id
    }
    pub fn is_within_dist(
        &self,
        other: &Self,
        dist: f32,
        is_3d: bool,
        include_own_radius: bool,
        include_target_radius: bool,
    ) -> bool {
        let mut max_dist = dist;
        if include_own_radius {
            max_dist += self.combat_reach;
        }
        if include_target_radius {
            max_dist += other.combat_reach;
        }

        if is_3d {
            self.position().distance_sq(&other.position()) < max_dist * max_dist
        } else {
            self.position().distance_2d_sq(&other.position()) < max_dist * max_dist
        }
    }
    pub fn is_within_dist_in_map(&self, other: &Self, dist: f32, is_3d: bool) -> bool {
        self.is_in_map(other)
            && self.in_same_phase(other)
            && self.is_within_dist(other, dist, is_3d, true, true)
    }
    pub fn absolute_angle_to_position(&self, position: Position) -> f32 {
        normalize_orientation(
            (position.y - self.position().y).atan2(position.x - self.position().x),
        )
    }
    pub fn absolute_angle_to(&self, other: &Self) -> f32 {
        self.absolute_angle_to_position(other.position())
    }
    pub fn to_absolute_angle(&self, relative_angle: f32) -> f32 {
        normalize_orientation(relative_angle + self.position().orientation)
    }
    pub fn to_relative_angle(&self, absolute_angle: f32) -> f32 {
        normalize_orientation(absolute_angle - self.position().orientation)
    }
    pub fn relative_angle_to_position(&self, position: Position) -> f32 {
        self.to_relative_angle(self.absolute_angle_to_position(position))
    }
    pub fn relative_angle_to(&self, other: &Self) -> f32 {
        self.relative_angle_to_position(other.position())
    }
    pub fn has_in_arc(&self, arc: f32, target: &Self, border: f32) -> bool {
        if std::ptr::eq(self, target) {
            return true;
        }

        let arc = normalize_orientation(arc);
        let mut angle = self.relative_angle_to(target);
        if angle > PI {
            angle -= TAU;
        }

        let left_border = -(arc / border);
        let right_border = arc / border;
        left_border <= angle && angle <= right_border
    }
    pub fn is_in_front(&self, target: &Self, arc: f32) -> bool {
        self.has_in_arc(arc, target, 2.0)
    }
    pub fn is_in_back(&self, target: &Self, arc: f32) -> bool {
        !self.has_in_arc(TAU - arc, target, 2.0)
    }
    pub fn has_position_in_line(&self, position: Position, obj_size: f32, width: f32) -> bool {
        if !self.has_position_in_arc(PI, position, 2.0) {
            return false;
        }

        let width = width + obj_size;
        let angle = self.relative_angle_to_position(position);
        angle.sin().abs() * self.position().distance_2d(&position) < width
    }
    pub fn has_in_line(&self, target: &Self, width: f32) -> bool {
        self.has_position_in_line(target.position(), target.combat_reach(), width)
    }
    pub fn has_position_in_arc(&self, arc: f32, position: Position, border: f32) -> bool {
        let arc = normalize_orientation(arc);
        let mut angle = self.relative_angle_to_position(position);
        if angle > PI {
            angle -= TAU;
        }

        let left_border = -(arc / border);
        let right_border = arc / border;
        left_border <= angle && angle <= right_border
    }
    pub fn is_within_box(
        &self,
        center: Position,
        x_radius: f32,
        y_radius: f32,
        z_radius: f32,
    ) -> bool {
        let rotation = TAU - center.orientation;
        let sin = rotation.sin();
        let cos = rotation.cos();
        let position = self.position();

        let box_dist_x = position.x - center.x;
        let box_dist_y = position.y - center.y;
        let rot_x = center.x + box_dist_x * cos - box_dist_y * sin;
        let rot_y = center.y + box_dist_y * cos + box_dist_x * sin;

        let dx = rot_x - center.x;
        let dy = rot_y - center.y;
        let dz = position.z - center.z;
        dx.abs() <= x_radius && dy.abs() <= y_radius && dz.abs() <= z_radius
    }
    pub fn is_within_double_vertical_cylinder(
        &self,
        center: Position,
        radius: f32,
        height: f32,
    ) -> bool {
        self.position().distance_2d_sq(&center) < radius * radius
            && (self.position().z - center.z).abs() <= height
    }
    pub fn get_visibility_range(&self, environment: &impl WorldObjectEnvironment) -> f32 {
        if let Some(override_range) = self.visibility_distance_override_like_cpp() {
            if !self.is_player() {
                return override_range;
            }
        }
        if environment.visibility_override(self).is_some() && !self.is_player() {
            environment
                .visibility_override(self)
                .unwrap_or(environment.visibility_range())
        } else if self.is_far_visible() && !self.is_player() {
            MAX_VISIBILITY_DISTANCE
        } else {
            environment.visibility_range()
        }
    }
    pub fn get_sight_range(
        &self,
        target: Option<&WorldObject>,
        environment: &impl WorldObjectEnvironment,
    ) -> f32 {
        if self.is_unit() {
            if self.is_player() {
                if let Some(target) = target {
                    if let Some(override_range) = target.visibility_distance_override_like_cpp() {
                        if !target.is_player() {
                            return override_range;
                        }
                    }
                    if let Some(override_range) = environment.visibility_override(target) {
                        if !target.is_player() {
                            return override_range;
                        }
                    }
                    if target.is_far_visible() && !target.is_player() {
                        return MAX_VISIBILITY_DISTANCE;
                    }
                }

                if environment.player_on_cinematic(self) {
                    DEFAULT_VISIBILITY_INSTANCE
                } else {
                    environment.visibility_range()
                }
            } else {
                environment
                    .creature_sight_distance(self)
                    .unwrap_or(SIGHT_RANGE_UNIT)
            }
        } else if self.object.type_id() == TypeId::DynamicObject && self.is_active() {
            environment.visibility_range()
        } else {
            0.0
        }
    }
    pub fn is_within_los(
        &self,
        position: Position,
        environment: &impl WorldObjectEnvironment,
        options: LineOfSightOptions,
    ) -> bool {
        if !self.object.is_in_world() {
            return true;
        }

        if !self.is_in_environment(environment) {
            return false;
        }

        environment.line_of_sight(LineOfSightQuery::to_position_like_cpp(
            self, position, options,
        ))
    }
    pub fn is_within_los_in_map(
        &self,
        other: &Self,
        environment: &impl WorldObjectEnvironment,
        options: LineOfSightOptions,
    ) -> bool {
        self.is_in_map(other)
            && self.is_in_environment(environment)
            && environment.line_of_sight(LineOfSightQuery::to_object_like_cpp(self, other, options))
    }
    pub fn get_map_height(
        &self,
        environment: &impl WorldObjectEnvironment,
        x: f32,
        y: f32,
        z: f32,
        query: WorldObjectHeightQuery,
    ) -> f32 {
        if !self.is_in_environment(environment) {
            return INVALID_HEIGHT;
        }

        let query_z = if z == MAX_HEIGHT {
            z
        } else {
            z + Z_OFFSET_FIND_HEIGHT
        };
        environment.map_height(self, x, y, query_z, query)
    }
    pub fn update_ground_position_z(
        &self,
        environment: &impl WorldObjectEnvironment,
        x: f32,
        y: f32,
        z: f32,
        hover_offset: f32,
    ) -> f32 {
        let new_z = self.get_map_height(environment, x, y, z, WorldObjectHeightQuery::default());
        if new_z > INVALID_HEIGHT {
            new_z + if self.is_unit() { hover_offset } else { 0.0 }
        } else {
            z
        }
    }
    /// Port of `WorldObject::UpdateAllowedPositionZ` (`Object.cpp:1411`).
    ///
    /// Returns the corrected Z. Branches:
    /// - **on transport** → unchanged (transport owns the Z).
    /// - **unit, can fly** → `max(z, ground + hover)` (only lift off below-ground).
    /// - **unit, grounded** → clamp into `[ground+hover, max_z+hover]`. With no
    ///   parsed liquid map `GetMapWaterOrGroundLevel` has no water ceiling, so
    ///   `max_z == ground_z` (exactly C++ when liquid is unavailable) and the swim
    ///   case collapses to "sit on ground". When `GetMapHeight` finds no terrain
    ///   (`<= INVALID_HEIGHT`) the Z is left untouched.
    /// - **non-unit** → snap to ground when terrain exists, else unchanged.
    pub fn update_allowed_position_z_like_cpp(
        &self,
        environment: &impl WorldObjectEnvironment,
        caps: AllowedPositionZCaps,
        x: f32,
        y: f32,
        z: f32,
    ) -> f32 {
        if caps.on_transport {
            return z;
        }
        let ground = self.get_map_height(environment, x, y, z, WorldObjectHeightQuery::default());
        allowed_position_z_from_ground_like_cpp(self.is_unit(), ground, z, caps)
    }
    pub fn get_floor_z(&self, environment: &impl WorldObjectEnvironment) -> f32 {
        if !self.object.is_in_world() || !self.is_in_environment(environment) {
            return self.static_floor_z;
        }

        let position = Position::new(
            self.position().x,
            self.position().y,
            self.position().z + Z_OFFSET_FIND_HEIGHT,
            self.position().orientation,
        );
        self.static_floor_z
            .max(environment.floor_z(self, position, DEFAULT_HEIGHT_SEARCH))
    }
    pub fn transport_global_position_from_offset(
        &self,
        transport_position: Position,
        passenger_offset: Position,
    ) -> Position {
        calculate_passenger_position(passenger_offset, transport_position)
    }
    pub fn transport_offset_from_position(&self, transport_position: Position) -> Position {
        calculate_passenger_offset(self.position(), transport_position)
    }
    pub fn relocate_on_transport(
        &mut self,
        transport_position: Position,
        passenger_offset: Position,
    ) -> Position {
        let global =
            self.transport_global_position_from_offset(transport_position, passenger_offset);
        self.relocate(global);
        global
    }
    pub(super) fn own_position_with_collision_height_like_cpp(&self) -> LineOfSightEndpoint {
        self.position_with_collision_height_like_cpp(self.position())
    }
    pub(super) fn position_with_collision_height_like_cpp(
        &self,
        position: Position,
    ) -> LineOfSightEndpoint {
        self.position_with_explicit_collision_height_like_cpp(
            position,
            self.collision_height_like_cpp(),
        )
    }
    pub(super) fn position_with_explicit_collision_height_like_cpp(
        &self,
        mut position: Position,
        height: f32,
    ) -> LineOfSightEndpoint {
        let adjusted_height = height.max(0.0);
        if adjusted_height > 0.0 {
            position.z += adjusted_height;
        }

        LineOfSightEndpoint {
            position,
            collision_height_adjusted: adjusted_height > 0.0,
            hit_sphere_adjusted: false,
        }
    }
    pub(super) fn is_in_environment(&self, environment: &impl WorldObjectEnvironment) -> bool {
        self.has_current_map
            && self.map_id() == environment.map_id()
            && self.instance_id == environment.instance_id()
    }
    pub(super) fn is_player(&self) -> bool {
        matches!(self.object.type_id(), TypeId::Player | TypeId::ActivePlayer)
            || self
                .object
                .type_mask()
                .intersects(TypeMask::PLAYER | TypeMask::ACTIVE_PLAYER)
    }
    pub(super) fn is_unit(&self) -> bool {
        self.object
            .type_mask()
            .intersects(TypeMask::UNIT | TypeMask::PLAYER | TypeMask::ACTIVE_PLAYER)
            || matches!(
                self.object.type_id(),
                TypeId::Unit | TypeId::Player | TypeId::ActivePlayer
            )
    }
}
