use std::collections::HashSet;

use wow_constants::{TypeId, TypeMask};
use wow_core::{ObjectGuid, Position};

use crate::{
    CreateObjectFlags, UpdateMask, WorldObject,
    update_fields::{AREA_TRIGGER_DATA_BITS, TYPEID_AREA_TRIGGER},
};

mod models;
#[cfg(test)]
mod tests;

pub use models::{
    AREA_TRIGGER_DATA_BOUNDS_RADIUS_2D_BIT, AREA_TRIGGER_DATA_CASTER_BIT,
    AREA_TRIGGER_DATA_CREATING_EFFECT_GUID_BIT, AREA_TRIGGER_DATA_DECAL_PROPERTIES_ID_BIT,
    AREA_TRIGGER_DATA_DURATION_BIT, AREA_TRIGGER_DATA_EXTRA_SCALE_CURVE_BIT,
    AREA_TRIGGER_DATA_ORBIT_PATH_TARGET_BIT, AREA_TRIGGER_DATA_OVERRIDE_MOVE_CURVE_X_BIT,
    AREA_TRIGGER_DATA_OVERRIDE_MOVE_CURVE_Y_BIT, AREA_TRIGGER_DATA_OVERRIDE_MOVE_CURVE_Z_BIT,
    AREA_TRIGGER_DATA_OVERRIDE_SCALE_CURVE_BIT, AREA_TRIGGER_DATA_PARENT_BIT,
    AREA_TRIGGER_DATA_SPELL_FOR_VISUALS_BIT, AREA_TRIGGER_DATA_SPELL_ID_BIT,
    AREA_TRIGGER_DATA_SPELL_VISUAL_ID_BIT, AREA_TRIGGER_DATA_TIME_TO_TARGET_BIT,
    AREA_TRIGGER_DATA_TIME_TO_TARGET_EXTRA_SCALE_BIT, AREA_TRIGGER_DATA_TIME_TO_TARGET_POS_BIT,
    AREA_TRIGGER_DATA_TIME_TO_TARGET_SCALE_BIT, AREA_TRIGGER_DATA_VISUAL_ANIM_BIT,
    AREA_TRIGGER_FLAG_IS_SERVER_SIDE, AreaTriggerCreatePropertiesFlags, AreaTriggerDataUpdate,
    AreaTriggerDataValues, AreaTriggerId, AreaTriggerOrbitInfo, AreaTriggerPosition2,
    AreaTriggerPosition3, AreaTriggerShapeInfo, AreaTriggerShapeType, AreaTriggerValuesUpdate,
    ScaleCurveValues, VisualAnimValues,
};

/// C++ box-shape containment used by area-trigger admission.
pub fn position_is_within_area_trigger_box_like_cpp(
    pos: &Position,
    center: &Position,
    half_length: f32,
    half_width: f32,
    half_height: f32,
) -> bool {
    let dx = pos.x - center.x;
    let dy = pos.y - center.y;
    let cos_yaw = center.orientation.cos();
    let sin_yaw = center.orientation.sin();
    let rel_x = dx * cos_yaw + dy * sin_yaw;
    let rel_y = -dx * sin_yaw + dy * cos_yaw;

    rel_x.abs() <= half_length
        && rel_y.abs() <= half_width
        && (pos.z - center.z).abs() <= half_height
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTrigger {
    world: WorldObject,
    data: AreaTriggerDataValues,
    area_trigger_data_changes: UpdateMask,
    spawn_id: u64,
    target_guid: ObjectGuid,
    aura_effect_bound: bool,
    stationary_position: Position,
    shape_type: AreaTriggerShapeType,
    shape: AreaTriggerShapeInfo,
    create_properties_flags: AreaTriggerCreatePropertiesFlags,
    spline_points: Vec<AreaTriggerPosition3>,
    orbit_info: Option<AreaTriggerOrbitInfo>,
    duration_ms: i32,
    total_duration_ms: i32,
    time_since_created_ms: u32,
    vertices_update_previous_orientation: f32,
    is_removed: bool,
    roll_pitch_yaw: Position,
    target_roll_pitch_yaw: Position,
    reached_destination: bool,
    last_spline_index: i32,
    movement_time_ms: u32,
    create_properties_id: Option<AreaTriggerId>,
    template_id: Option<AreaTriggerId>,
    template_flags: u32,
    inside_units: HashSet<ObjectGuid>,
    ai_initialized: bool,
    grid_unload_cleanup_before_delete_count: u32,
    grid_unload_delete_requested: bool,
}

impl AreaTrigger {
    pub fn new() -> Self {
        let mut world = WorldObject::new(
            false,
            TypeId::AreaTrigger,
            TypeMask::OBJECT | TypeMask::AREA_TRIGGER,
        );
        world
            .object_mut()
            .create_flags_mut()
            .insert(CreateObjectFlags::STATIONARY | CreateObjectFlags::AREA_TRIGGER);

        Self {
            world,
            data: AreaTriggerDataValues::default(),
            area_trigger_data_changes: UpdateMask::new(AREA_TRIGGER_DATA_BITS),
            spawn_id: 0,
            target_guid: ObjectGuid::EMPTY,
            aura_effect_bound: false,
            stationary_position: Position::new(0.0, 0.0, 0.0, 0.0),
            shape_type: AreaTriggerShapeType::Sphere,
            shape: AreaTriggerShapeInfo::default(),
            create_properties_flags: AreaTriggerCreatePropertiesFlags::default(),
            spline_points: Vec::new(),
            orbit_info: None,
            duration_ms: 0,
            total_duration_ms: 0,
            time_since_created_ms: 0,
            vertices_update_previous_orientation: f32::INFINITY,
            is_removed: false,
            roll_pitch_yaw: Position::new(0.0, 0.0, 0.0, 0.0),
            target_roll_pitch_yaw: Position::new(0.0, 0.0, 0.0, 0.0),
            reached_destination: true,
            last_spline_index: 0,
            movement_time_ms: 0,
            create_properties_id: None,
            template_id: None,
            template_flags: 0,
            inside_units: HashSet::new(),
            ai_initialized: false,
            grid_unload_cleanup_before_delete_count: 0,
            grid_unload_delete_requested: false,
        }
    }

    pub const fn world(&self) -> &WorldObject {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut WorldObject {
        &mut self.world
    }

    pub const fn data(&self) -> &AreaTriggerDataValues {
        &self.data
    }

    pub fn area_trigger_data_changes_mask(&self) -> &UpdateMask {
        &self.area_trigger_data_changes
    }

    pub fn clear_area_trigger_data_changes(&mut self) {
        self.area_trigger_data_changes.reset_all();
    }

    pub const fn cleanup_before_delete_count(&self) -> u32 {
        self.grid_unload_cleanup_before_delete_count
    }

    pub const fn grid_unload_delete_requested(&self) -> bool {
        self.grid_unload_delete_requested
    }

    pub fn set_destroyed_object(&mut self, destroyed: bool) {
        self.world.object_mut().set_destroyed_object(destroyed);
    }

    pub fn cleanup_before_delete(&mut self) {
        self.grid_unload_cleanup_before_delete_count = self
            .grid_unload_cleanup_before_delete_count
            .saturating_add(1);
    }

    pub fn request_delete_from_grid_unload(&mut self) {
        self.grid_unload_delete_requested = true;
        self.world.clear_current_cell();
    }

    pub const fn spawn_id(&self) -> u64 {
        self.spawn_id
    }

    pub const fn is_static_spawn(&self) -> bool {
        self.spawn_id != 0
    }

    pub const fn is_removed(&self) -> bool {
        self.is_removed
    }

    pub const fn duration_ms(&self) -> i32 {
        self.duration_ms
    }

    pub const fn total_duration_ms(&self) -> i32 {
        self.total_duration_ms
    }

    pub const fn time_since_created_ms(&self) -> u32 {
        self.time_since_created_ms
    }

    pub const fn target_guid(&self) -> ObjectGuid {
        self.target_guid
    }

    pub const fn caster_guid(&self) -> ObjectGuid {
        self.data.caster
    }

    pub const fn creator_guid(&self) -> ObjectGuid {
        self.caster_guid()
    }

    pub const fn owner_guid(&self) -> ObjectGuid {
        self.caster_guid()
    }

    pub const fn spell_id(&self) -> i32 {
        self.data.spell_id
    }

    pub const fn stationary_position(&self) -> Position {
        self.stationary_position
    }

    pub const fn shape_type(&self) -> AreaTriggerShapeType {
        self.shape_type
    }

    pub const fn shape(&self) -> &AreaTriggerShapeInfo {
        &self.shape
    }

    pub const fn create_properties_flags(&self) -> AreaTriggerCreatePropertiesFlags {
        self.create_properties_flags
    }

    pub fn spline_points(&self) -> &[AreaTriggerPosition3] {
        &self.spline_points
    }

    pub const fn orbit_info(&self) -> Option<AreaTriggerOrbitInfo> {
        self.orbit_info
    }

    pub const fn vertices_update_previous_orientation(&self) -> f32 {
        self.vertices_update_previous_orientation
    }

    pub const fn reached_destination(&self) -> bool {
        self.reached_destination
    }

    pub const fn last_spline_index(&self) -> i32 {
        self.last_spline_index
    }

    pub const fn movement_time_ms(&self) -> u32 {
        self.movement_time_ms
    }

    pub const fn create_properties_id(&self) -> Option<AreaTriggerId> {
        self.create_properties_id
    }

    pub const fn template_id(&self) -> Option<AreaTriggerId> {
        self.template_id
    }

    pub const fn template_flags(&self) -> u32 {
        self.template_flags
    }

    pub const fn is_custom(&self) -> bool {
        match self.template_id {
            Some(id) => id.is_custom,
            None => false,
        }
    }

    pub const fn is_server_side(&self) -> bool {
        (self.template_flags & AREA_TRIGGER_FLAG_IS_SERVER_SIDE) != 0
    }

    pub const fn is_aura_effect_bound(&self) -> bool {
        self.aura_effect_bound
    }

    pub const fn roll_pitch_yaw(&self) -> Position {
        self.roll_pitch_yaw
    }

    pub const fn target_roll_pitch_yaw(&self) -> Position {
        self.target_roll_pitch_yaw
    }

    pub fn inside_units(&self) -> &HashSet<ObjectGuid> {
        &self.inside_units
    }

    pub const fn is_ai_initialized(&self) -> bool {
        self.ai_initialized
    }

    pub fn set_spawn_id(&mut self, spawn_id: u64) {
        self.spawn_id = spawn_id;
    }

    pub fn set_target_guid(&mut self, target_guid: ObjectGuid) {
        self.target_guid = target_guid;
    }

    pub fn set_aura_effect_bound(&mut self, bound: bool) {
        self.aura_effect_bound = bound;
    }

    pub fn relocate_stationary_position(&mut self, position: Position) {
        self.stationary_position = position;
    }

    pub fn set_shape_type(&mut self, shape_type: AreaTriggerShapeType) {
        self.shape_type = shape_type;
        self.shape.shape_type = shape_type;
    }

    pub fn set_shape_info(&mut self, shape: AreaTriggerShapeInfo) {
        self.shape_type = shape.shape_type;
        self.shape = shape;
    }

    pub fn set_create_properties_flags(&mut self, flags: AreaTriggerCreatePropertiesFlags) {
        self.create_properties_flags = flags;
    }

    pub fn set_spline_points(&mut self, spline_points: Vec<AreaTriggerPosition3>) {
        self.spline_points = spline_points;
    }

    pub fn set_orbit_info(&mut self, orbit_info: Option<AreaTriggerOrbitInfo>) {
        self.orbit_info = orbit_info;
    }

    pub fn set_create_properties_id(&mut self, id: AreaTriggerId) {
        self.create_properties_id = Some(id);
    }

    pub fn set_template(&mut self, id: AreaTriggerId, flags: u32) {
        self.template_id = Some(id);
        self.template_flags = flags;
    }

    pub fn ai_initialize(&mut self) {
        self.ai_initialized = true;
    }

    pub fn ai_destroy(&mut self) {
        self.ai_initialized = false;
    }

    pub fn remove(&mut self) {
        self.is_removed = true;
    }

    pub fn set_duration(&mut self, new_duration_ms: i32) {
        self.duration_ms = new_duration_ms;
        self.total_duration_ms = new_duration_ms;
        self.set_u32_field(
            AREA_TRIGGER_DATA_DURATION_BIT,
            new_duration_ms.max(0) as u32,
            |data| &mut data.duration,
        );
    }

    pub fn delay(&mut self, delay_ms: i32) {
        self.set_duration(self.duration_ms - delay_ms);
    }

    pub fn update_duration_without_field_change(&mut self, new_duration_ms: i32) {
        self.duration_ms = new_duration_ms;
        self.data.duration = new_duration_ms.max(0) as u32;
    }

    pub fn update_time_and_duration(&mut self, diff_ms: u32) -> bool {
        self.time_since_created_ms = self.time_since_created_ms.wrapping_add(diff_ms);

        if self.duration_ms == -1 {
            return false;
        }

        if self.duration_ms > diff_ms as i32 {
            self.update_duration_without_field_change(self.duration_ms - diff_ms as i32);
            false
        } else {
            self.remove();
            true
        }
    }

    pub fn set_caster_guid(&mut self, caster: ObjectGuid) {
        self.set_guid_field(AREA_TRIGGER_DATA_CASTER_BIT, caster, |data| {
            &mut data.caster
        });
    }

    pub fn set_time_to_target(&mut self, time_to_target: u32) {
        self.set_u32_field(
            AREA_TRIGGER_DATA_TIME_TO_TARGET_BIT,
            time_to_target,
            |data| &mut data.time_to_target,
        );
    }

    pub fn set_time_to_target_scale(&mut self, time_to_target_scale: u32) {
        self.set_u32_field(
            AREA_TRIGGER_DATA_TIME_TO_TARGET_SCALE_BIT,
            time_to_target_scale,
            |data| &mut data.time_to_target_scale,
        );
    }

    pub fn set_time_to_target_extra_scale(&mut self, time_to_target_extra_scale: u32) {
        self.set_u32_field(
            AREA_TRIGGER_DATA_TIME_TO_TARGET_EXTRA_SCALE_BIT,
            time_to_target_extra_scale,
            |data| &mut data.time_to_target_extra_scale,
        );
    }

    pub fn set_time_to_target_pos(&mut self, time_to_target_pos: u32) {
        self.set_u32_field(
            AREA_TRIGGER_DATA_TIME_TO_TARGET_POS_BIT,
            time_to_target_pos,
            |data| &mut data.time_to_target_pos,
        );
    }

    pub fn set_spell_id(&mut self, spell_id: i32) {
        self.set_i32_field(AREA_TRIGGER_DATA_SPELL_ID_BIT, spell_id, |data| {
            &mut data.spell_id
        });
    }

    pub fn set_spell_for_visuals(&mut self, spell_for_visuals: i32) {
        self.set_i32_field(
            AREA_TRIGGER_DATA_SPELL_FOR_VISUALS_BIT,
            spell_for_visuals,
            |data| &mut data.spell_for_visuals,
        );
    }

    pub fn set_spell_visual_id(&mut self, spell_visual_id: i32) {
        self.set_i32_field(
            AREA_TRIGGER_DATA_SPELL_VISUAL_ID_BIT,
            spell_visual_id,
            |data| &mut data.spell_visual_id,
        );
    }

    pub fn set_bounds_radius_2d(&mut self, bounds_radius_2d: f32) {
        self.set_f32_field(
            AREA_TRIGGER_DATA_BOUNDS_RADIUS_2D_BIT,
            bounds_radius_2d,
            |data| &mut data.bounds_radius_2d,
        );
    }

    pub fn set_decal_properties_id(&mut self, decal_properties_id: u32) {
        self.set_u32_field(
            AREA_TRIGGER_DATA_DECAL_PROPERTIES_ID_BIT,
            decal_properties_id,
            |data| &mut data.decal_properties_id,
        );
    }

    pub fn set_creating_effect_guid(&mut self, creating_effect_guid: ObjectGuid) {
        self.set_guid_field(
            AREA_TRIGGER_DATA_CREATING_EFFECT_GUID_BIT,
            creating_effect_guid,
            |data| &mut data.creating_effect_guid,
        );
    }

    pub fn set_orbit_path_target(&mut self, orbit_path_target: ObjectGuid) {
        self.set_guid_field(
            AREA_TRIGGER_DATA_ORBIT_PATH_TARGET_BIT,
            orbit_path_target,
            |data| &mut data.orbit_path_target,
        );
    }

    pub fn set_visual_anim(&mut self, visual_anim: VisualAnimValues) {
        if self.data.visual_anim != visual_anim {
            self.data.visual_anim = visual_anim;
            self.mark_area_trigger_data(AREA_TRIGGER_DATA_VISUAL_ANIM_BIT);
        }
    }

    pub fn set_override_scale_constant(&mut self, scale: f32) {
        Self::set_scale_curve_constant(
            &mut self.data.override_scale_curve,
            scale,
            &mut self.area_trigger_data_changes,
            AREA_TRIGGER_DATA_OVERRIDE_SCALE_CURVE_BIT,
        );
    }

    pub fn clear_override_scale_curve(&mut self) {
        Self::clear_scale_curve(
            &mut self.data.override_scale_curve,
            &mut self.area_trigger_data_changes,
            AREA_TRIGGER_DATA_OVERRIDE_SCALE_CURVE_BIT,
        );
    }

    pub fn set_extra_scale_constant(&mut self, scale: f32) {
        Self::set_scale_curve_constant(
            &mut self.data.extra_scale_curve,
            scale,
            &mut self.area_trigger_data_changes,
            AREA_TRIGGER_DATA_EXTRA_SCALE_CURVE_BIT,
        );
    }

    pub fn clear_extra_scale_curve(&mut self) {
        Self::clear_scale_curve(
            &mut self.data.extra_scale_curve,
            &mut self.area_trigger_data_changes,
            AREA_TRIGGER_DATA_EXTRA_SCALE_CURVE_BIT,
        );
    }

    pub fn set_override_move_constant(&mut self, x: f32, y: f32, z: f32) {
        Self::set_scale_curve_constant(
            &mut self.data.override_move_curve_x,
            x,
            &mut self.area_trigger_data_changes,
            AREA_TRIGGER_DATA_OVERRIDE_MOVE_CURVE_X_BIT,
        );
        Self::set_scale_curve_constant(
            &mut self.data.override_move_curve_y,
            y,
            &mut self.area_trigger_data_changes,
            AREA_TRIGGER_DATA_OVERRIDE_MOVE_CURVE_Y_BIT,
        );
        Self::set_scale_curve_constant(
            &mut self.data.override_move_curve_z,
            z,
            &mut self.area_trigger_data_changes,
            AREA_TRIGGER_DATA_OVERRIDE_MOVE_CURVE_Z_BIT,
        );
    }

    pub fn clear_override_move_curve(&mut self) {
        Self::clear_scale_curve(
            &mut self.data.override_move_curve_x,
            &mut self.area_trigger_data_changes,
            AREA_TRIGGER_DATA_OVERRIDE_MOVE_CURVE_X_BIT,
        );
        Self::clear_scale_curve(
            &mut self.data.override_move_curve_y,
            &mut self.area_trigger_data_changes,
            AREA_TRIGGER_DATA_OVERRIDE_MOVE_CURVE_Y_BIT,
        );
        Self::clear_scale_curve(
            &mut self.data.override_move_curve_z,
            &mut self.area_trigger_data_changes,
            AREA_TRIGGER_DATA_OVERRIDE_MOVE_CURVE_Z_BIT,
        );
    }

    pub fn set_inside_units(&mut self, units: impl IntoIterator<Item = ObjectGuid>) {
        self.inside_units = units.into_iter().collect();
    }

    pub fn changed_object_type_mask(&self) -> u32 {
        self.world.object().changed_object_type_mask()
            | if self.area_trigger_data_changes.is_any_set() {
                1 << TYPEID_AREA_TRIGGER
            } else {
                0
            }
    }

    pub fn values_update(&self) -> AreaTriggerValuesUpdate {
        let object_update = self.world.object().values_update();
        AreaTriggerValuesUpdate {
            changed_object_type_mask: self.changed_object_type_mask(),
            object_data: object_update.object_data,
            area_trigger_data: self.area_trigger_data_changes.is_any_set().then(|| {
                AreaTriggerDataUpdate {
                    mask: self.area_trigger_data_changes.clone(),
                    values: self.data,
                }
            }),
        }
    }

    fn set_scale_curve_constant(
        target: &mut ScaleCurveValues,
        scale: f32,
        mask: &mut UpdateMask,
        bit: usize,
    ) {
        let value = ScaleCurveValues {
            override_active: true,
            start_time_offset: 0,
            parameter_curve: scale.to_bits() | 1,
        };
        if *target != value {
            *target = value;
            mask.set(AREA_TRIGGER_DATA_PARENT_BIT);
            mask.set(bit);
        }
    }

    fn clear_scale_curve(target: &mut ScaleCurveValues, mask: &mut UpdateMask, bit: usize) {
        let value = ScaleCurveValues {
            override_active: false,
            ..*target
        };
        if *target != value {
            *target = value;
            mask.set(AREA_TRIGGER_DATA_PARENT_BIT);
            mask.set(bit);
        }
    }

    fn set_u32_field(
        &mut self,
        bit: usize,
        value: u32,
        field: impl FnOnce(&mut AreaTriggerDataValues) -> &mut u32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_area_trigger_data(bit);
        }
    }

    fn set_i32_field(
        &mut self,
        bit: usize,
        value: i32,
        field: impl FnOnce(&mut AreaTriggerDataValues) -> &mut i32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_area_trigger_data(bit);
        }
    }

    fn set_f32_field(
        &mut self,
        bit: usize,
        value: f32,
        field: impl FnOnce(&mut AreaTriggerDataValues) -> &mut f32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_area_trigger_data(bit);
        }
    }

    fn set_guid_field(
        &mut self,
        bit: usize,
        value: ObjectGuid,
        field: impl FnOnce(&mut AreaTriggerDataValues) -> &mut ObjectGuid,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_area_trigger_data(bit);
        }
    }

    fn mark_area_trigger_data(&mut self, bit: usize) {
        self.area_trigger_data_changes
            .set(AREA_TRIGGER_DATA_PARENT_BIT);
        self.area_trigger_data_changes.set(bit);
    }
}

impl Default for AreaTrigger {
    fn default() -> Self {
        Self::new()
    }
}
