use std::collections::HashSet;

use wow_constants::{TypeId, TypeMask};
use wow_core::{ObjectGuid, Position};

use crate::{
    CreateObjectFlags, ObjectDataUpdate, UpdateMask, WorldObject,
    update_fields::{AREA_TRIGGER_DATA_BITS, TYPEID_AREA_TRIGGER},
};

pub const AREA_TRIGGER_DATA_PARENT_BIT: usize = 0;
pub const AREA_TRIGGER_DATA_OVERRIDE_SCALE_CURVE_BIT: usize = 1;
pub const AREA_TRIGGER_DATA_EXTRA_SCALE_CURVE_BIT: usize = 2;
pub const AREA_TRIGGER_DATA_OVERRIDE_MOVE_CURVE_X_BIT: usize = 3;
pub const AREA_TRIGGER_DATA_OVERRIDE_MOVE_CURVE_Y_BIT: usize = 4;
pub const AREA_TRIGGER_DATA_OVERRIDE_MOVE_CURVE_Z_BIT: usize = 5;
pub const AREA_TRIGGER_DATA_CASTER_BIT: usize = 6;
pub const AREA_TRIGGER_DATA_DURATION_BIT: usize = 7;
pub const AREA_TRIGGER_DATA_TIME_TO_TARGET_BIT: usize = 8;
pub const AREA_TRIGGER_DATA_TIME_TO_TARGET_SCALE_BIT: usize = 9;
pub const AREA_TRIGGER_DATA_TIME_TO_TARGET_EXTRA_SCALE_BIT: usize = 10;
pub const AREA_TRIGGER_DATA_TIME_TO_TARGET_POS_BIT: usize = 11;
pub const AREA_TRIGGER_DATA_SPELL_ID_BIT: usize = 12;
pub const AREA_TRIGGER_DATA_SPELL_FOR_VISUALS_BIT: usize = 13;
pub const AREA_TRIGGER_DATA_SPELL_VISUAL_ID_BIT: usize = 14;
pub const AREA_TRIGGER_DATA_BOUNDS_RADIUS_2D_BIT: usize = 15;
pub const AREA_TRIGGER_DATA_DECAL_PROPERTIES_ID_BIT: usize = 16;
pub const AREA_TRIGGER_DATA_CREATING_EFFECT_GUID_BIT: usize = 17;
pub const AREA_TRIGGER_DATA_ORBIT_PATH_TARGET_BIT: usize = 18;
pub const AREA_TRIGGER_DATA_VISUAL_ANIM_BIT: usize = 19;

pub const AREA_TRIGGER_FLAG_IS_SERVER_SIDE: u32 = 0x01;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AreaTriggerShapeType {
    Sphere = 0,
    Box = 1,
    Unknown = 2,
    Polygon = 3,
    Cylinder = 4,
    Disk = 5,
    BoundedPlane = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaTriggerId {
    pub id: u32,
    pub is_custom: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleCurveValues {
    pub override_active: bool,
    pub start_time_offset: u32,
    pub parameter_curve: u32,
}

impl Default for ScaleCurveValues {
    fn default() -> Self {
        Self {
            override_active: false,
            start_time_offset: 0,
            parameter_curve: 1.0f32.to_bits() | 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VisualAnimValues {
    pub field_c: bool,
    pub animation_data_id: u32,
    pub anim_kit_id: u32,
    pub anim_progress: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerDataValues {
    pub override_scale_curve: ScaleCurveValues,
    pub extra_scale_curve: ScaleCurveValues,
    pub override_move_curve_x: ScaleCurveValues,
    pub override_move_curve_y: ScaleCurveValues,
    pub override_move_curve_z: ScaleCurveValues,
    pub caster: ObjectGuid,
    pub duration: u32,
    pub time_to_target: u32,
    pub time_to_target_scale: u32,
    pub time_to_target_extra_scale: u32,
    pub time_to_target_pos: u32,
    pub spell_id: i32,
    pub spell_for_visuals: i32,
    pub spell_visual_id: i32,
    pub bounds_radius_2d: f32,
    pub decal_properties_id: u32,
    pub creating_effect_guid: ObjectGuid,
    pub orbit_path_target: ObjectGuid,
    pub visual_anim: VisualAnimValues,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AreaTriggerPosition2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AreaTriggerPosition3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerShapeInfo {
    pub shape_type: AreaTriggerShapeType,
    pub data: [f32; 8],
    pub polygon_vertices: Vec<AreaTriggerPosition2>,
    pub polygon_vertices_target: Vec<AreaTriggerPosition2>,
}

impl Default for AreaTriggerShapeInfo {
    fn default() -> Self {
        Self {
            shape_type: AreaTriggerShapeType::Sphere,
            data: [0.0; 8],
            polygon_vertices: Vec::new(),
            polygon_vertices_target: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AreaTriggerCreatePropertiesFlags {
    pub flags: u32,
    pub scale_curve_id: u32,
    pub morph_curve_id: u32,
    pub facing_curve_id: u32,
    pub move_curve_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerOrbitInfo {
    pub counter_clockwise: bool,
    pub can_loop: bool,
    pub time_to_target: u32,
    pub elapsed_time_for_movement: i32,
    pub start_delay: u32,
    pub radius: f32,
    pub blend_from_radius: f32,
    pub initial_angle: f32,
    pub z_offset: f32,
}

impl Default for AreaTriggerDataValues {
    fn default() -> Self {
        Self {
            override_scale_curve: ScaleCurveValues::default(),
            extra_scale_curve: ScaleCurveValues::default(),
            override_move_curve_x: ScaleCurveValues::default(),
            override_move_curve_y: ScaleCurveValues::default(),
            override_move_curve_z: ScaleCurveValues::default(),
            caster: ObjectGuid::EMPTY,
            duration: 0,
            time_to_target: 0,
            time_to_target_scale: 0,
            time_to_target_extra_scale: 0,
            time_to_target_pos: 0,
            spell_id: 0,
            spell_for_visuals: 0,
            spell_visual_id: 0,
            bounds_radius_2d: 0.0,
            decal_properties_id: 0,
            creating_effect_guid: ObjectGuid::EMPTY,
            orbit_path_target: ObjectGuid::EMPTY,
            visual_anim: VisualAnimValues::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerDataUpdate {
    pub mask: UpdateMask,
    pub values: AreaTriggerDataValues,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataUpdate>,
    pub area_trigger_data: Option<AreaTriggerDataUpdate>,
}

impl AreaTriggerValuesUpdate {
    pub const fn has_data(&self) -> bool {
        self.changed_object_type_mask != 0
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use wow_core::guid::HighGuid;

    fn caster_guid() -> ObjectGuid {
        ObjectGuid::create_global(HighGuid::Player, 0, 1)
    }

    fn spell_guid() -> ObjectGuid {
        ObjectGuid::create_world_object(HighGuid::DynamicObject, 0, 1, 530, 123, 0, 99)
    }

    #[test]
    fn areatrigger_constructor_matches_cpp_base_state() {
        let area_trigger = AreaTrigger::new();

        assert!(!area_trigger.world().is_world_object());
        assert_eq!(area_trigger.world().object().type_id(), TypeId::AreaTrigger);
        assert_eq!(
            area_trigger.world().object().type_mask(),
            TypeMask::OBJECT | TypeMask::AREA_TRIGGER
        );
        assert!(
            area_trigger
                .world()
                .object()
                .create_flags()
                .contains(CreateObjectFlags::STATIONARY | CreateObjectFlags::AREA_TRIGGER)
        );
        assert_eq!(area_trigger.spawn_id(), 0);
        assert!(!area_trigger.is_static_spawn());
        assert_eq!(area_trigger.duration_ms(), 0);
        assert_eq!(area_trigger.total_duration_ms(), 0);
        assert_eq!(area_trigger.time_since_created_ms(), 0);
        assert!(
            area_trigger
                .vertices_update_previous_orientation()
                .is_infinite()
        );
        assert!(!area_trigger.is_removed());
        assert!(area_trigger.reached_destination());
        assert_eq!(area_trigger.last_spline_index(), 0);
        assert_eq!(area_trigger.movement_time_ms(), 0);
        assert_eq!(area_trigger.create_properties_id(), None);
        assert_eq!(area_trigger.template_id(), None);
        assert!(!area_trigger.is_custom());
        assert!(!area_trigger.is_server_side());
        assert!(!area_trigger.is_aura_effect_bound());
        assert!(!area_trigger.is_ai_initialized());
        assert!(area_trigger.inside_units().is_empty());
    }

    #[test]
    fn areatrigger_data_setters_mark_cpp_bits() {
        let mut area_trigger = AreaTrigger::new();
        area_trigger.set_caster_guid(caster_guid());
        area_trigger.set_duration(1_500);
        area_trigger.set_time_to_target(11);
        area_trigger.set_time_to_target_scale(12);
        area_trigger.set_time_to_target_extra_scale(13);
        area_trigger.set_time_to_target_pos(14);
        area_trigger.set_spell_id(123);
        area_trigger.set_spell_for_visuals(124);
        area_trigger.set_spell_visual_id(125);
        area_trigger.set_bounds_radius_2d(10.5);
        area_trigger.set_decal_properties_id(24);
        area_trigger.set_creating_effect_guid(spell_guid());
        area_trigger.set_orbit_path_target(caster_guid());
        area_trigger.set_visual_anim(VisualAnimValues {
            field_c: true,
            animation_data_id: 1,
            anim_kit_id: 2,
            anim_progress: 3,
        });
        area_trigger.set_override_scale_constant(2.0);
        area_trigger.set_extra_scale_constant(3.0);
        area_trigger.set_override_move_constant(1.0, 2.0, 3.0);

        let mask = area_trigger.area_trigger_data_changes_mask();
        for bit in 0..AREA_TRIGGER_DATA_BITS {
            assert!(mask.is_set(bit), "bit {bit} should be set");
        }
        assert_eq!(area_trigger.caster_guid(), caster_guid());
        assert_eq!(area_trigger.creator_guid(), caster_guid());
        assert_eq!(area_trigger.owner_guid(), caster_guid());
        assert_eq!(area_trigger.spell_id(), 123);
        assert_eq!(area_trigger.data().duration, 1_500);
        assert!(area_trigger.data().override_scale_curve.override_active);
        assert_eq!(
            area_trigger.data().override_scale_curve.parameter_curve,
            2.0f32.to_bits() | 1
        );
    }

    #[test]
    fn areatrigger_duration_and_static_state_follow_cpp_shape() {
        let mut area_trigger = AreaTrigger::new();
        area_trigger.set_duration(-1);
        assert_eq!(area_trigger.duration_ms(), -1);
        assert_eq!(area_trigger.total_duration_ms(), -1);
        assert_eq!(area_trigger.data().duration, 0);
        assert!(!area_trigger.update_time_and_duration(10_000));
        assert_eq!(area_trigger.time_since_created_ms(), 10_000);

        area_trigger.set_duration(100);
        area_trigger.clear_area_trigger_data_changes();
        assert!(!area_trigger.update_time_and_duration(40));
        assert_eq!(area_trigger.duration_ms(), 60);
        assert_eq!(area_trigger.data().duration, 60);
        assert!(!area_trigger.area_trigger_data_changes_mask().is_any_set());
        assert!(area_trigger.update_time_and_duration(60));
        assert!(area_trigger.is_removed());

        area_trigger.set_spawn_id(42);
        assert!(area_trigger.is_static_spawn());
        area_trigger.set_template(
            AreaTriggerId {
                id: 7,
                is_custom: true,
            },
            AREA_TRIGGER_FLAG_IS_SERVER_SIDE,
        );
        assert!(area_trigger.is_custom());
        assert!(area_trigger.is_server_side());
    }

    #[test]
    fn areatrigger_values_update_sets_type_bit() {
        let mut area_trigger = AreaTrigger::new();
        area_trigger.set_spell_id(1);

        let update = area_trigger.values_update();
        assert_eq!(update.changed_object_type_mask, 1 << TYPEID_AREA_TRIGGER);
        assert!(update.object_data.is_none());
        assert!(update.area_trigger_data.is_some());
    }
}
