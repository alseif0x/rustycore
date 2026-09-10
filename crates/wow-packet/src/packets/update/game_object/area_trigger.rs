//! Area trigger packets.
//!
//! Separated from game_object.rs under #689.

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub area_trigger_data_mask: u32,
    pub override_scale_curve: ScaleCurveValuesUpdate,
    pub extra_scale_curve: ScaleCurveValuesUpdate,
    pub override_move_curve_x: ScaleCurveValuesUpdate,
    pub override_move_curve_y: ScaleCurveValuesUpdate,
    pub override_move_curve_z: ScaleCurveValuesUpdate,
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
    pub visual_anim: VisualAnimValuesUpdate,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AreaTriggerPosition2CreateData {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AreaTriggerPosition3CreateData {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerShapeCreateData {
    pub shape_type: u8,
    pub data: [f32; 8],
    pub polygon_vertices: Vec<AreaTriggerPosition2CreateData>,
    pub polygon_vertices_target: Vec<AreaTriggerPosition2CreateData>,
}

impl Default for AreaTriggerShapeCreateData {
    fn default() -> Self {
        Self {
            shape_type: 0,
            data: [0.0; 8],
            polygon_vertices: Vec::new(),
            polygon_vertices_target: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerOrbitCreateData {
    pub counter_clockwise: bool,
    pub can_loop: bool,
    pub time_to_target: u32,
    pub elapsed_time_for_movement: i32,
    pub start_delay: u32,
    pub radius: f32,
    pub blend_from_radius: f32,
    pub initial_angle: f32,
    pub z_offset: f32,
    pub center: AreaTriggerPosition3CreateData,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerCreateData {
    pub guid: ObjectGuid,
    pub entry_id: u32,
    pub dynamic_flags: u32,
    pub scale: f32,
    pub position: Position,
    pub time_since_created_ms: u32,
    pub roll_pitch_yaw: Position,
    pub target_roll_pitch_yaw: Position,
    pub create_properties_flags: u32,
    pub scale_curve_id: u32,
    pub morph_curve_id: u32,
    pub facing_curve_id: u32,
    pub move_curve_id: u32,
    pub shape: AreaTriggerShapeCreateData,
    pub spline_points: Vec<AreaTriggerPosition3CreateData>,
    pub orbit: Option<AreaTriggerOrbitCreateData>,
    pub override_scale_curve: ScaleCurveValuesUpdate,
    pub extra_scale_curve: ScaleCurveValuesUpdate,
    pub override_move_curve_x: ScaleCurveValuesUpdate,
    pub override_move_curve_y: ScaleCurveValuesUpdate,
    pub override_move_curve_z: ScaleCurveValuesUpdate,
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
    pub visual_anim: VisualAnimValuesUpdate,
}

pub(in crate::packets::update) fn debug_area_trigger_create_block_len_like_cpp(
    guid: &ObjectGuid,
    data: &AreaTriggerCreateData,
) -> usize {
    let mut block = WorldPacket::new_empty();
    write_area_trigger_create_block(&mut block, guid, data);
    block.into_data().len()
}

const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ABSOLUTE_ORIENTATION_LIKE_CPP: u32 = 0x00001;

const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_DYNAMIC_SHAPE_LIKE_CPP: u32 = 0x00002;

const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ATTACHED_LIKE_CPP: u32 = 0x00004;

const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_FACE_MOVEMENT_DIR_LIKE_CPP: u32 = 0x00008;

const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_FOLLOWS_TERRAIN_LIKE_CPP: u32 = 0x00010;

const AREATRIGGER_CREATE_PROPERTIES_FLAG_UNK1_LIKE_CPP: u32 = 0x00020;

const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_TARGET_ROLL_PITCH_YAW_LIKE_CPP: u32 = 0x00040;

pub(in crate::packets::update) fn write_area_trigger_create_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    create_data: &AreaTriggerCreateData,
) {
    buf.write_uint8(UpdateType::CreateObject as u8);
    buf.write_packed_guid(guid);
    buf.write_uint8(TypeId::AreaTrigger as u8);

    buf.write_bit(false); // NoBirthAnim
    buf.write_bit(false); // EnablePortals
    buf.write_bit(false); // PlayHoverAnim
    buf.write_bit(false); // MovementUpdate
    buf.write_bit(false); // MovementTransport
    buf.write_bit(true); // Stationary
    buf.write_bit(false); // CombatVictim
    buf.write_bit(false); // ServerTime
    buf.write_bit(false); // Vehicle
    buf.write_bit(false); // AnimKit
    buf.write_bit(false); // Rotation
    buf.write_bit(true); // AreaTrigger
    buf.write_bit(false); // GameObject
    buf.write_bit(false); // SmoothPhasing
    buf.write_bit(false); // ThisIsYou
    buf.write_bit(false); // SceneObject
    buf.write_bit(false); // ActivePlayer
    buf.write_bit(false); // Conversation
    buf.flush_bits();

    buf.write_int32(0); // PauseTimes count

    buf.write_float(create_data.position.x);
    buf.write_float(create_data.position.y);
    buf.write_float(create_data.position.z);
    buf.write_float(create_data.position.orientation);

    buf.write_uint32(create_data.time_since_created_ms);
    write_position_xyz_like_cpp(buf, create_data.roll_pitch_yaw);

    let flags = create_data.create_properties_flags;
    let has_absolute_orientation =
        flags & AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ABSOLUTE_ORIENTATION_LIKE_CPP != 0;
    let has_dynamic_shape =
        flags & AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_DYNAMIC_SHAPE_LIKE_CPP != 0;
    let has_attached = flags & AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ATTACHED_LIKE_CPP != 0;
    let has_face_movement_dir =
        flags & AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_FACE_MOVEMENT_DIR_LIKE_CPP != 0;
    let has_follows_terrain =
        flags & AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_FOLLOWS_TERRAIN_LIKE_CPP != 0;
    let has_unk1 = flags & AREATRIGGER_CREATE_PROPERTIES_FLAG_UNK1_LIKE_CPP != 0;
    let has_target_roll_pitch_yaw =
        flags & AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_TARGET_ROLL_PITCH_YAW_LIKE_CPP != 0;
    let has_scale_curve_id = create_data.scale_curve_id != 0;
    let has_morph_curve_id = create_data.morph_curve_id != 0;
    let has_facing_curve_id = create_data.facing_curve_id != 0;
    let has_move_curve_id = create_data.move_curve_id != 0;
    let has_area_trigger_sphere = create_data.shape.shape_type == 0;
    let has_area_trigger_box = create_data.shape.shape_type == 1;
    let has_area_trigger_polygon = create_data.shape.shape_type == 3;
    let has_area_trigger_cylinder = create_data.shape.shape_type == 4;
    let has_disk = create_data.shape.shape_type == 5;
    let has_bounded_plane = create_data.shape.shape_type == 6;
    let has_area_trigger_spline = !create_data.spline_points.is_empty();
    let has_orbit = create_data.orbit.is_some();
    let has_movement_script = false;

    buf.write_bit(has_absolute_orientation);
    buf.write_bit(has_dynamic_shape);
    buf.write_bit(has_attached);
    buf.write_bit(has_face_movement_dir);
    buf.write_bit(has_follows_terrain);
    buf.write_bit(has_unk1);
    buf.write_bit(has_target_roll_pitch_yaw);
    buf.write_bit(has_scale_curve_id);
    buf.write_bit(has_morph_curve_id);
    buf.write_bit(has_facing_curve_id);
    buf.write_bit(has_move_curve_id);
    buf.write_bit(has_area_trigger_sphere);
    buf.write_bit(has_area_trigger_box);
    buf.write_bit(has_area_trigger_polygon);
    buf.write_bit(has_area_trigger_cylinder);
    buf.write_bit(has_disk);
    buf.write_bit(has_bounded_plane);
    buf.write_bit(has_area_trigger_spline);
    buf.write_bit(has_orbit);
    buf.write_bit(has_movement_script);
    buf.flush_bits();

    if has_area_trigger_spline {
        buf.write_uint32(create_data.time_to_target);
        buf.write_int32(0); // elapsed time for movement
        buf.write_bits(create_data.spline_points.len() as u32, 16);
        for point in &create_data.spline_points {
            buf.write_float(point.x);
            buf.write_float(point.y);
            buf.write_float(point.z);
        }
    }

    if has_target_roll_pitch_yaw {
        write_position_xyz_like_cpp(buf, create_data.target_roll_pitch_yaw);
    }
    if has_scale_curve_id {
        buf.write_uint32(create_data.scale_curve_id);
    }
    if has_morph_curve_id {
        buf.write_uint32(create_data.morph_curve_id);
    }
    if has_facing_curve_id {
        buf.write_uint32(create_data.facing_curve_id);
    }
    if has_move_curve_id {
        buf.write_uint32(create_data.move_curve_id);
    }

    let shape = &create_data.shape;
    if has_area_trigger_sphere {
        buf.write_float(shape.data[0]);
        buf.write_float(shape.data[1]);
    }
    if has_area_trigger_box {
        for index in 0..6 {
            buf.write_float(shape.data[index]);
        }
    }
    if has_area_trigger_polygon {
        buf.write_int32(shape.polygon_vertices.len() as i32);
        buf.write_int32(shape.polygon_vertices_target.len() as i32);
        buf.write_float(shape.data[0]);
        buf.write_float(shape.data[1]);
        for vertex in &shape.polygon_vertices {
            buf.write_float(vertex.x);
            buf.write_float(vertex.y);
        }
        for vertex in &shape.polygon_vertices_target {
            buf.write_float(vertex.x);
            buf.write_float(vertex.y);
        }
    }
    if has_area_trigger_cylinder {
        for index in 0..6 {
            buf.write_float(shape.data[index]);
        }
    }
    if has_disk {
        for index in 0..8 {
            buf.write_float(shape.data[index]);
        }
    }
    if has_bounded_plane {
        buf.write_float(shape.data[0]);
        buf.write_float(shape.data[1]);
        buf.write_float(shape.data[3]);
        buf.write_float(shape.data[4]);
    }

    if let Some(orbit) = create_data.orbit {
        buf.write_bit(false); // PathTarget
        buf.write_bit(true); // Center
        buf.write_bit(orbit.counter_clockwise);
        buf.write_bit(orbit.can_loop);
        buf.write_uint32(orbit.time_to_target);
        buf.write_int32(orbit.elapsed_time_for_movement);
        buf.write_uint32(orbit.start_delay);
        buf.write_float(orbit.radius);
        buf.write_float(orbit.blend_from_radius);
        buf.write_float(orbit.initial_angle);
        buf.write_float(orbit.z_offset);
        buf.write_float(orbit.center.x);
        buf.write_float(orbit.center.y);
        buf.write_float(orbit.center.z);
    }

    write_area_trigger_values_create(buf, create_data);
}

fn write_area_trigger_values_create(buf: &mut WorldPacket, data: &AreaTriggerCreateData) {
    let mut values = WorldPacket::new_empty();
    values.write_uint8(0x00); // UpdateFieldFlag

    values.write_int32(data.entry_id as i32);
    values.write_uint32(data.dynamic_flags);
    values.write_float(data.scale);

    write_scale_curve_values_create(&mut values, &data.override_scale_curve);
    values.write_packed_guid(&data.caster);
    values.write_uint32(data.duration);
    values.write_uint32(data.time_to_target);
    values.write_uint32(data.time_to_target_scale);
    values.write_uint32(data.time_to_target_extra_scale);
    values.write_uint32(data.time_to_target_pos);
    values.write_int32(data.spell_id);
    values.write_int32(data.spell_for_visuals);
    values.write_int32(data.spell_visual_id);
    values.write_float(data.bounds_radius_2d);
    values.write_uint32(data.decal_properties_id);
    values.write_packed_guid(&data.creating_effect_guid);
    values.write_packed_guid(&data.orbit_path_target);
    write_scale_curve_values_create(&mut values, &data.extra_scale_curve);
    write_scale_curve_values_create(&mut values, &data.override_move_curve_x);
    write_scale_curve_values_create(&mut values, &data.override_move_curve_y);
    write_scale_curve_values_create(&mut values, &data.override_move_curve_z);
    write_visual_anim_values_create(&mut values, &data.visual_anim);

    let data = values.into_data();
    buf.write_uint32(data.len() as u32);
    buf.write_bytes(&data);
}

pub(in crate::packets::update) const VALUES_TYPE_AREA_TRIGGER: u32 = 1 << 11;

pub(in crate::packets::update) fn write_area_trigger_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &AreaTriggerDataValuesUpdate,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(data.changed_object_type_mask);

    if data.changed_object_type_mask & VALUES_TYPE_OBJECT != 0 {
        if let Some(object_data) = data.object_data {
            write_object_data_values_update_section(&mut val_buf, object_data);
        } else {
            write_object_data_values_update_section(
                &mut val_buf,
                ObjectDataValuesUpdate {
                    changed_object_type_mask: VALUES_TYPE_OBJECT,
                    object_data_mask: 0,
                    entry_id: 0,
                    dynamic_flags: 0,
                    scale: 0.0,
                },
            );
        }
    }

    if data.changed_object_type_mask & VALUES_TYPE_AREA_TRIGGER != 0 {
        let mask = data.area_trigger_data_mask & 0x000F_FFFF;
        val_buf.write_bits(mask, 20);
        val_buf.flush_bits();

        if mask & 0x0000_0001 != 0 {
            if mask & 0x0000_0002 != 0 {
                write_scale_curve_values_update(&mut val_buf, &data.override_scale_curve);
            }
            if mask & 0x0000_0040 != 0 {
                val_buf.write_packed_guid(&data.caster);
            }
            if mask & 0x0000_0080 != 0 {
                val_buf.write_uint32(data.duration);
            }
            if mask & 0x0000_0100 != 0 {
                val_buf.write_uint32(data.time_to_target);
            }
            if mask & 0x0000_0200 != 0 {
                val_buf.write_uint32(data.time_to_target_scale);
            }
            if mask & 0x0000_0400 != 0 {
                val_buf.write_uint32(data.time_to_target_extra_scale);
            }
            if mask & 0x0000_0800 != 0 {
                val_buf.write_uint32(data.time_to_target_pos);
            }
            if mask & 0x0000_1000 != 0 {
                val_buf.write_int32(data.spell_id);
            }
            if mask & 0x0000_2000 != 0 {
                val_buf.write_int32(data.spell_for_visuals);
            }
            if mask & 0x0000_4000 != 0 {
                val_buf.write_int32(data.spell_visual_id);
            }
            if mask & 0x0000_8000 != 0 {
                val_buf.write_float(data.bounds_radius_2d);
            }
            if mask & 0x0001_0000 != 0 {
                val_buf.write_uint32(data.decal_properties_id);
            }
            if mask & 0x0002_0000 != 0 {
                val_buf.write_packed_guid(&data.creating_effect_guid);
            }
            if mask & 0x0004_0000 != 0 {
                val_buf.write_packed_guid(&data.orbit_path_target);
            }
            if mask & 0x0000_0004 != 0 {
                write_scale_curve_values_update(&mut val_buf, &data.extra_scale_curve);
            }
            if mask & 0x0000_0008 != 0 {
                write_scale_curve_values_update(&mut val_buf, &data.override_move_curve_x);
            }
            if mask & 0x0000_0010 != 0 {
                write_scale_curve_values_update(&mut val_buf, &data.override_move_curve_y);
            }
            if mask & 0x0000_0020 != 0 {
                write_scale_curve_values_update(&mut val_buf, &data.override_move_curve_z);
            }
            if mask & 0x0008_0000 != 0 {
                write_visual_anim_values_update(&mut val_buf, &data.visual_anim);
            }
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}
