// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `AreaTriggerDataStore::LoadAreaTriggerTemplates` template/action/create-properties subset.

use std::collections::HashMap;

use crate::{ScriptIdLikeCpp, WorldSafeLocStore};

pub const AREATRIGGER_ACTION_CAST_LIKE_CPP: u32 = 0;
pub const AREATRIGGER_ACTION_ADDAURA_LIKE_CPP: u32 = 1;
pub const AREATRIGGER_ACTION_TELEPORT_LIKE_CPP: u32 = 2;
pub const AREATRIGGER_ACTION_MAX_LIKE_CPP: u32 = 3;

pub const AREATRIGGER_ACTION_USER_ANY_LIKE_CPP: u32 = 0;
pub const AREATRIGGER_ACTION_USER_FRIEND_LIKE_CPP: u32 = 1;
pub const AREATRIGGER_ACTION_USER_ENEMY_LIKE_CPP: u32 = 2;
pub const AREATRIGGER_ACTION_USER_RAID_LIKE_CPP: u32 = 3;
pub const AREATRIGGER_ACTION_USER_PARTY_LIKE_CPP: u32 = 4;
pub const AREATRIGGER_ACTION_USER_CASTER_LIKE_CPP: u32 = 5;
pub const AREATRIGGER_ACTION_USER_MAX_LIKE_CPP: u32 = 6;

pub const AREATRIGGER_FLAG_NONE_LIKE_CPP: u32 = 0;
pub const AREATRIGGER_FLAG_IS_SERVER_SIDE_LIKE_CPP: u32 = 0x01;

pub const AREATRIGGER_CREATE_PROPERTIES_FLAG_NONE_LIKE_CPP: u32 = 0x00000;
pub const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ABSOLUTE_ORIENTATION_LIKE_CPP: u32 = 0x00001;
pub const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_DYNAMIC_SHAPE_LIKE_CPP: u32 = 0x00002;
pub const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ATTACHED_LIKE_CPP: u32 = 0x00004;
pub const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_FACE_MOVEMENT_DIR_LIKE_CPP: u32 = 0x00008;
pub const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_FOLLOWS_TERRAIN_LIKE_CPP: u32 = 0x00010;
pub const AREATRIGGER_CREATE_PROPERTIES_FLAG_UNK1_LIKE_CPP: u32 = 0x00020;
pub const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_TARGET_ROLL_PITCH_YAW_LIKE_CPP: u32 = 0x00040;
pub const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ANIM_ID_LIKE_CPP: u32 = 0x00080;
pub const AREATRIGGER_CREATE_PROPERTIES_FLAG_UNK3_LIKE_CPP: u32 = 0x00100;
pub const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ANIM_KIT_ID_LIKE_CPP: u32 = 0x00200;
pub const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_CIRCULAR_MOVEMENT_LIKE_CPP: u32 = 0x00400;
pub const AREATRIGGER_CREATE_PROPERTIES_FLAG_UNK5_LIKE_CPP: u32 = 0x00800;

pub const AREATRIGGER_SHAPE_SPHERE_LIKE_CPP: u8 = 0;
pub const AREATRIGGER_SHAPE_BOX_LIKE_CPP: u8 = 1;
pub const AREATRIGGER_SHAPE_UNK_LIKE_CPP: u8 = 2;
pub const AREATRIGGER_SHAPE_POLYGON_LIKE_CPP: u8 = 3;
pub const AREATRIGGER_SHAPE_CYLINDER_LIKE_CPP: u8 = 4;
pub const AREATRIGGER_SHAPE_DISK_LIKE_CPP: u8 = 5;
pub const AREATRIGGER_SHAPE_BOUNDED_PLANE_LIKE_CPP: u8 = 6;
pub const AREATRIGGER_SHAPE_MAX_LIKE_CPP: u8 = 7;
pub const MAX_AREATRIGGER_ENTITY_DATA_LIKE_CPP: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AreaTriggerIdLikeCpp {
    pub id: u32,
    pub is_custom: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaTriggerActionLikeCpp {
    pub param: u32,
    pub action_type: u32,
    pub target_type: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerPosition2LikeCpp {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerPosition3LikeCpp {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaTriggerTemplateRowLikeCpp {
    pub id: u32,
    pub is_custom: bool,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaTriggerTemplateActionRowLikeCpp {
    pub area_trigger_id: u32,
    pub is_custom: bool,
    pub action_type: u32,
    pub action_param: u32,
    pub target_type: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerPolygonVertexRowLikeCpp {
    pub create_properties_id: u32,
    pub is_custom: bool,
    pub idx: u32,
    pub vertice_x: f32,
    pub vertice_y: f32,
    pub vertice_target_x: Option<f32>,
    pub vertice_target_y: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerSplinePointRowLikeCpp {
    pub create_properties_id: u32,
    pub is_custom: bool,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerCreatePropertiesRowLikeCpp {
    pub id: u32,
    pub is_custom: bool,
    pub area_trigger_id: u32,
    pub is_areatrigger_custom: bool,
    pub flags: u32,
    pub move_curve_id: u32,
    pub scale_curve_id: u32,
    pub morph_curve_id: u32,
    pub facing_curve_id: u32,
    pub anim_id: i32,
    pub anim_kit_id: i32,
    pub decal_properties_id: u32,
    pub time_to_target: u32,
    pub time_to_target_scale: u32,
    pub shape: u8,
    pub shape_data: [f32; MAX_AREATRIGGER_ENTITY_DATA_LIKE_CPP],
    pub script_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerCreatePropertiesOrbitRowLikeCpp {
    pub create_properties_id: u32,
    pub is_custom: bool,
    pub start_delay: u32,
    pub circle_radius: f32,
    pub blend_from_radius: f32,
    pub initial_angle: f32,
    pub z_offset: f32,
    pub counter_clockwise: bool,
    pub can_loop: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaTriggerTemplateLikeCpp {
    pub id: AreaTriggerIdLikeCpp,
    pub flags: u32,
    pub actions: Vec<AreaTriggerActionLikeCpp>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerShapeInfoLikeCpp {
    pub shape_type: u8,
    pub data: [f32; MAX_AREATRIGGER_ENTITY_DATA_LIKE_CPP],
    pub polygon_vertices: Vec<AreaTriggerPosition2LikeCpp>,
    pub polygon_vertices_target: Vec<AreaTriggerPosition2LikeCpp>,
}

impl AreaTriggerShapeInfoLikeCpp {
    fn new_like_cpp(shape_type: u8, data: [f32; MAX_AREATRIGGER_ENTITY_DATA_LIKE_CPP]) -> Self {
        Self {
            shape_type,
            data,
            polygon_vertices: Vec::new(),
            polygon_vertices_target: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerCreatePropertiesLikeCpp {
    pub id: AreaTriggerIdLikeCpp,
    pub template_id: Option<AreaTriggerIdLikeCpp>,
    pub flags: u32,
    pub move_curve_id: u32,
    pub scale_curve_id: u32,
    pub morph_curve_id: u32,
    pub facing_curve_id: u32,
    pub anim_id: i32,
    pub anim_kit_id: i32,
    pub decal_properties_id: u32,
    pub time_to_target: u32,
    pub time_to_target_scale: u32,
    pub shape: AreaTriggerShapeInfoLikeCpp,
    pub spline_points: Vec<AreaTriggerPosition3LikeCpp>,
    pub orbit_info: Option<AreaTriggerOrbitInfoLikeCpp>,
    pub script_id: ScriptIdLikeCpp,
    pub script_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerOrbitInfoLikeCpp {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaTriggerOrbitFloatFieldLikeCpp {
    Radius,
    BlendFromRadius,
    InitialAngle,
    ZOffset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaTriggerCurveFieldLikeCpp {
    Move,
    Scale,
    Morph,
    Facing,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AreaTriggerTemplateLoadReportLikeCpp {
    pub template_rows_seen: usize,
    pub action_rows_seen: usize,
    pub loaded_templates: usize,
    pub loaded_actions: usize,
    pub polygon_vertex_rows_seen: usize,
    pub spline_point_rows_seen: usize,
    pub loaded_polygon_vertices: usize,
    pub loaded_polygon_target_vertices: usize,
    pub loaded_spline_points: usize,
    pub create_properties_rows_seen: usize,
    pub orbit_rows_seen: usize,
    pub loaded_create_properties: usize,
    pub loaded_orbit_infos: usize,
    pub skipped_actions_invalid_action_type: Vec<(AreaTriggerIdLikeCpp, u32, u32)>,
    pub skipped_actions_invalid_target_type: Vec<(AreaTriggerIdLikeCpp, u32, u32)>,
    pub skipped_actions_invalid_teleport_world_safe_loc: Vec<(AreaTriggerIdLikeCpp, u32)>,
    pub invalid_partial_target_vertices: Vec<(AreaTriggerIdLikeCpp, u32)>,
    pub skipped_create_properties_invalid_template:
        Vec<(AreaTriggerIdLikeCpp, AreaTriggerIdLikeCpp)>,
    pub skipped_create_properties_invalid_shape: Vec<(AreaTriggerIdLikeCpp, u8)>,
    pub corrected_create_properties_invalid_curves: Vec<(
        AreaTriggerIdLikeCpp,
        AreaTriggerIdLikeCpp,
        AreaTriggerCurveFieldLikeCpp,
        u32,
    )>,
    pub corrected_polygon_heights: Vec<AreaTriggerIdLikeCpp>,
    pub invalid_polygon_target_vertex_counts: Vec<AreaTriggerIdLikeCpp>,
    pub skipped_orbit_invalid_create_properties: Vec<AreaTriggerIdLikeCpp>,
    pub corrected_orbit_invalid_floats:
        Vec<(AreaTriggerIdLikeCpp, AreaTriggerOrbitFloatFieldLikeCpp, f32)>,
}

#[derive(Debug, Clone, Default)]
pub struct AreaTriggerTemplateStore {
    templates: HashMap<AreaTriggerIdLikeCpp, AreaTriggerTemplateLikeCpp>,
    polygon_vertices_by_create_properties:
        HashMap<AreaTriggerIdLikeCpp, Vec<AreaTriggerPosition2LikeCpp>>,
    polygon_target_vertices_by_create_properties:
        HashMap<AreaTriggerIdLikeCpp, Vec<AreaTriggerPosition2LikeCpp>>,
    spline_points_by_create_properties:
        HashMap<AreaTriggerIdLikeCpp, Vec<AreaTriggerPosition3LikeCpp>>,
    create_properties: HashMap<AreaTriggerIdLikeCpp, AreaTriggerCreatePropertiesLikeCpp>,
}

pub struct AreaTriggerTemplateLoadOutcomeLikeCpp {
    pub store: AreaTriggerTemplateStore,
    pub report: AreaTriggerTemplateLoadReportLikeCpp,
}

impl AreaTriggerTemplateStore {
    pub fn from_keys(keys: impl IntoIterator<Item = (u32, bool)>) -> Self {
        Self {
            templates: keys
                .into_iter()
                .map(|(id, is_custom)| {
                    let id = AreaTriggerIdLikeCpp { id, is_custom };
                    (
                        id,
                        AreaTriggerTemplateLikeCpp {
                            id,
                            flags: AREATRIGGER_FLAG_NONE_LIKE_CPP,
                            actions: Vec::new(),
                        },
                    )
                })
                .collect(),
            polygon_vertices_by_create_properties: HashMap::new(),
            polygon_target_vertices_by_create_properties: HashMap::new(),
            spline_points_by_create_properties: HashMap::new(),
            create_properties: HashMap::new(),
        }
    }

    pub fn from_rows_like_cpp(
        template_rows: impl IntoIterator<Item = AreaTriggerTemplateRowLikeCpp>,
        action_rows: impl IntoIterator<Item = AreaTriggerTemplateActionRowLikeCpp>,
        polygon_vertex_rows: impl IntoIterator<Item = AreaTriggerPolygonVertexRowLikeCpp>,
        spline_point_rows: impl IntoIterator<Item = AreaTriggerSplinePointRowLikeCpp>,
        create_properties_rows: impl IntoIterator<Item = AreaTriggerCreatePropertiesRowLikeCpp>,
        orbit_rows: impl IntoIterator<Item = AreaTriggerCreatePropertiesOrbitRowLikeCpp>,
        world_safe_locs: &WorldSafeLocStore,
        mut curve_exists: impl FnMut(u32) -> bool,
        mut script_id_for_name: impl FnMut(&str) -> ScriptIdLikeCpp,
    ) -> AreaTriggerTemplateLoadOutcomeLikeCpp {
        let mut report = AreaTriggerTemplateLoadReportLikeCpp::default();
        let mut actions_by_area_trigger: HashMap<
            AreaTriggerIdLikeCpp,
            Vec<AreaTriggerActionLikeCpp>,
        > = HashMap::new();

        for row in action_rows {
            report.action_rows_seen += 1;
            let area_trigger_id = AreaTriggerIdLikeCpp {
                id: row.area_trigger_id,
                is_custom: row.is_custom,
            };

            if row.action_type >= AREATRIGGER_ACTION_MAX_LIKE_CPP {
                report.skipped_actions_invalid_action_type.push((
                    area_trigger_id,
                    row.action_type,
                    row.action_param,
                ));
                continue;
            }

            if row.target_type >= AREATRIGGER_ACTION_USER_MAX_LIKE_CPP {
                report.skipped_actions_invalid_target_type.push((
                    area_trigger_id,
                    row.target_type,
                    row.action_param,
                ));
                continue;
            }

            if row.action_type == AREATRIGGER_ACTION_TELEPORT_LIKE_CPP
                && !world_safe_locs.contains(row.action_param)
            {
                report
                    .skipped_actions_invalid_teleport_world_safe_loc
                    .push((area_trigger_id, row.action_param));
                continue;
            }

            actions_by_area_trigger
                .entry(area_trigger_id)
                .or_default()
                .push(AreaTriggerActionLikeCpp {
                    param: row.action_param,
                    action_type: row.action_type,
                    target_type: row.target_type,
                });
            report.loaded_actions += 1;
        }

        let mut polygon_vertices_by_create_properties: HashMap<
            AreaTriggerIdLikeCpp,
            Vec<AreaTriggerPosition2LikeCpp>,
        > = HashMap::new();
        let mut polygon_target_vertices_by_create_properties: HashMap<
            AreaTriggerIdLikeCpp,
            Vec<AreaTriggerPosition2LikeCpp>,
        > = HashMap::new();
        for row in polygon_vertex_rows {
            report.polygon_vertex_rows_seen += 1;
            let create_properties_id = AreaTriggerIdLikeCpp {
                id: row.create_properties_id,
                is_custom: row.is_custom,
            };

            polygon_vertices_by_create_properties
                .entry(create_properties_id)
                .or_default()
                .push(AreaTriggerPosition2LikeCpp {
                    x: row.vertice_x,
                    y: row.vertice_y,
                });
            report.loaded_polygon_vertices += 1;

            match (row.vertice_target_x, row.vertice_target_y) {
                (Some(x), Some(y)) => {
                    polygon_target_vertices_by_create_properties
                        .entry(create_properties_id)
                        .or_default()
                        .push(AreaTriggerPosition2LikeCpp { x, y });
                    report.loaded_polygon_target_vertices += 1;
                }
                (None, None) => {}
                _ => report
                    .invalid_partial_target_vertices
                    .push((create_properties_id, row.idx)),
            }
        }

        let mut spline_points_by_create_properties: HashMap<
            AreaTriggerIdLikeCpp,
            Vec<AreaTriggerPosition3LikeCpp>,
        > = HashMap::new();
        for row in spline_point_rows {
            report.spline_point_rows_seen += 1;
            let create_properties_id = AreaTriggerIdLikeCpp {
                id: row.create_properties_id,
                is_custom: row.is_custom,
            };
            spline_points_by_create_properties
                .entry(create_properties_id)
                .or_default()
                .push(AreaTriggerPosition3LikeCpp {
                    x: row.x,
                    y: row.y,
                    z: row.z,
                });
            report.loaded_spline_points += 1;
        }

        let mut templates = HashMap::new();
        for row in template_rows {
            report.template_rows_seen += 1;
            let id = AreaTriggerIdLikeCpp {
                id: row.id,
                is_custom: row.is_custom,
            };
            templates.insert(
                id,
                AreaTriggerTemplateLikeCpp {
                    id,
                    flags: row.flags,
                    actions: actions_by_area_trigger.remove(&id).unwrap_or_default(),
                },
            );
        }

        let mut create_properties = HashMap::new();
        for row in create_properties_rows {
            report.create_properties_rows_seen += 1;
            let id = AreaTriggerIdLikeCpp {
                id: row.id,
                is_custom: row.is_custom,
            };
            let template_id = AreaTriggerIdLikeCpp {
                id: row.area_trigger_id,
                is_custom: row.is_areatrigger_custom,
            };
            if template_id.id != 0 && !templates.contains_key(&template_id) {
                report
                    .skipped_create_properties_invalid_template
                    .push((id, template_id));
                continue;
            }

            if row.shape >= AREATRIGGER_SHAPE_MAX_LIKE_CPP {
                report
                    .skipped_create_properties_invalid_shape
                    .push((id, row.shape));
                continue;
            }

            let area_trigger_id_for_log = template_id;
            let mut move_curve_id = row.move_curve_id;
            let mut scale_curve_id = row.scale_curve_id;
            let mut morph_curve_id = row.morph_curve_id;
            let mut facing_curve_id = row.facing_curve_id;

            for (curve_id, field) in [
                (move_curve_id, AreaTriggerCurveFieldLikeCpp::Move),
                (scale_curve_id, AreaTriggerCurveFieldLikeCpp::Scale),
                (morph_curve_id, AreaTriggerCurveFieldLikeCpp::Morph),
                (facing_curve_id, AreaTriggerCurveFieldLikeCpp::Facing),
            ] {
                if curve_id != 0 && !curve_exists(curve_id) {
                    report.corrected_create_properties_invalid_curves.push((
                        area_trigger_id_for_log,
                        id,
                        field,
                        curve_id,
                    ));
                    match field {
                        AreaTriggerCurveFieldLikeCpp::Move => move_curve_id = 0,
                        AreaTriggerCurveFieldLikeCpp::Scale => scale_curve_id = 0,
                        AreaTriggerCurveFieldLikeCpp::Morph => morph_curve_id = 0,
                        AreaTriggerCurveFieldLikeCpp::Facing => facing_curve_id = 0,
                    }
                }
            }

            let mut shape = AreaTriggerShapeInfoLikeCpp::new_like_cpp(row.shape, row.shape_data);
            if shape.shape_type == AREATRIGGER_SHAPE_POLYGON_LIKE_CPP && shape.data[0] <= 0.0 {
                shape.data[0] = 1.0;
                if shape.data[1] <= 0.0 {
                    shape.data[1] = 1.0;
                }
                report.corrected_polygon_heights.push(id);
            }

            shape.polygon_vertices = polygon_vertices_by_create_properties
                .get(&id)
                .cloned()
                .unwrap_or_default();
            shape.polygon_vertices_target = polygon_target_vertices_by_create_properties
                .get(&id)
                .cloned()
                .unwrap_or_default();
            if !shape.polygon_vertices_target.is_empty()
                && shape.polygon_vertices.len() != shape.polygon_vertices_target.len()
            {
                report.invalid_polygon_target_vertex_counts.push(id);
                shape.polygon_vertices_target.clear();
            }

            let spline_points = spline_points_by_create_properties
                .get(&id)
                .cloned()
                .unwrap_or_default();
            let script_id = script_id_for_name(&row.script_name);

            create_properties.insert(
                id,
                AreaTriggerCreatePropertiesLikeCpp {
                    id,
                    template_id: (template_id.id != 0).then_some(template_id),
                    flags: row.flags,
                    move_curve_id,
                    scale_curve_id,
                    morph_curve_id,
                    facing_curve_id,
                    anim_id: row.anim_id,
                    anim_kit_id: row.anim_kit_id,
                    decal_properties_id: row.decal_properties_id,
                    time_to_target: row.time_to_target,
                    time_to_target_scale: row.time_to_target_scale,
                    shape,
                    spline_points,
                    orbit_info: None,
                    script_id,
                    script_name: row.script_name,
                },
            );
        }

        for row in orbit_rows {
            report.orbit_rows_seen += 1;
            let id = AreaTriggerIdLikeCpp {
                id: row.create_properties_id,
                is_custom: row.is_custom,
            };
            let Some(create_properties) = create_properties.get_mut(&id) else {
                report.skipped_orbit_invalid_create_properties.push(id);
                continue;
            };

            let mut radius = row.circle_radius;
            let mut blend_from_radius = row.blend_from_radius;
            let mut initial_angle = row.initial_angle;
            let mut z_offset = row.z_offset;
            for (value, field) in [
                (radius, AreaTriggerOrbitFloatFieldLikeCpp::Radius),
                (
                    blend_from_radius,
                    AreaTriggerOrbitFloatFieldLikeCpp::BlendFromRadius,
                ),
                (
                    initial_angle,
                    AreaTriggerOrbitFloatFieldLikeCpp::InitialAngle,
                ),
                (z_offset, AreaTriggerOrbitFloatFieldLikeCpp::ZOffset),
            ] {
                if !value.is_finite() {
                    report
                        .corrected_orbit_invalid_floats
                        .push((id, field, value));
                    match field {
                        AreaTriggerOrbitFloatFieldLikeCpp::Radius => radius = 0.0,
                        AreaTriggerOrbitFloatFieldLikeCpp::BlendFromRadius => {
                            blend_from_radius = 0.0
                        }
                        AreaTriggerOrbitFloatFieldLikeCpp::InitialAngle => initial_angle = 0.0,
                        AreaTriggerOrbitFloatFieldLikeCpp::ZOffset => z_offset = 0.0,
                    }
                }
            }

            create_properties.orbit_info = Some(AreaTriggerOrbitInfoLikeCpp {
                counter_clockwise: row.counter_clockwise,
                can_loop: row.can_loop,
                time_to_target: 0,
                elapsed_time_for_movement: 0,
                start_delay: row.start_delay,
                radius,
                blend_from_radius,
                initial_angle,
                z_offset,
            });
            report.loaded_orbit_infos += 1;
        }

        report.loaded_templates = templates.len();
        report.loaded_create_properties = create_properties.len();
        AreaTriggerTemplateLoadOutcomeLikeCpp {
            store: Self {
                templates,
                polygon_vertices_by_create_properties,
                polygon_target_vertices_by_create_properties,
                spline_points_by_create_properties,
                create_properties,
            },
            report,
        }
    }

    pub fn contains(&self, id: u32, is_custom: bool) -> bool {
        self.templates
            .contains_key(&AreaTriggerIdLikeCpp { id, is_custom })
    }

    pub fn get_template_like_cpp(
        &self,
        id: AreaTriggerIdLikeCpp,
    ) -> Option<&AreaTriggerTemplateLikeCpp> {
        self.templates.get(&id)
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn action_len(&self) -> usize {
        self.templates
            .values()
            .map(|template| template.actions.len())
            .sum()
    }

    pub fn polygon_vertices_like_cpp(
        &self,
        id: AreaTriggerIdLikeCpp,
    ) -> Option<&[AreaTriggerPosition2LikeCpp]> {
        self.polygon_vertices_by_create_properties
            .get(&id)
            .map(Vec::as_slice)
    }

    pub fn polygon_target_vertices_like_cpp(
        &self,
        id: AreaTriggerIdLikeCpp,
    ) -> Option<&[AreaTriggerPosition2LikeCpp]> {
        self.polygon_target_vertices_by_create_properties
            .get(&id)
            .map(Vec::as_slice)
    }

    pub fn spline_points_like_cpp(
        &self,
        id: AreaTriggerIdLikeCpp,
    ) -> Option<&[AreaTriggerPosition3LikeCpp]> {
        self.spline_points_by_create_properties
            .get(&id)
            .map(Vec::as_slice)
    }

    pub fn polygon_vertex_len(&self) -> usize {
        self.polygon_vertices_by_create_properties
            .values()
            .map(Vec::len)
            .sum()
    }

    pub fn spline_point_len(&self) -> usize {
        self.spline_points_by_create_properties
            .values()
            .map(Vec::len)
            .sum()
    }

    pub fn get_create_properties_like_cpp(
        &self,
        id: AreaTriggerIdLikeCpp,
    ) -> Option<&AreaTriggerCreatePropertiesLikeCpp> {
        self.create_properties.get(&id)
    }

    pub fn create_properties_len(&self) -> usize {
        self.create_properties.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

#[cfg(test)]
#[path = "area_trigger_template/tests/mod.rs"]
mod tests;
