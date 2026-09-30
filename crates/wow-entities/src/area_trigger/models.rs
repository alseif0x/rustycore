use wow_core::ObjectGuid;

use crate::{ObjectDataUpdate, UpdateMask};

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
