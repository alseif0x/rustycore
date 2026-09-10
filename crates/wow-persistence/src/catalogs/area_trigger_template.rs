//! SQLx-free startup source contract for C++ AreaTrigger templates.

use crate::PersistenceFutureLikeCpp;

pub const AREA_TRIGGER_SHAPE_DATA_COUNT_LIKE_CPP: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaTriggerTemplatePersistenceRowLikeCpp {
    pub id: u32,
    pub is_custom: bool,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaTriggerTemplateActionPersistenceRowLikeCpp {
    pub area_trigger_id: u32,
    pub is_custom: bool,
    pub action_type: u32,
    pub action_param: u32,
    pub target_type: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerPolygonVertexPersistenceRowLikeCpp {
    pub create_properties_id: u32,
    pub is_custom: bool,
    pub idx: u32,
    pub vertice_x: f32,
    pub vertice_y: f32,
    pub vertice_target_x: Option<f32>,
    pub vertice_target_y: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerSplinePointPersistenceRowLikeCpp {
    pub create_properties_id: u32,
    pub is_custom: bool,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerCreatePropertiesPersistenceRowLikeCpp {
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
    pub shape_data: [f32; AREA_TRIGGER_SHAPE_DATA_COUNT_LIKE_CPP],
    pub script_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerCreatePropertiesOrbitPersistenceRowLikeCpp {
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AreaTriggerTemplateCatalogRowsLikeCpp {
    pub action_rows: Vec<AreaTriggerTemplateActionPersistenceRowLikeCpp>,
    pub polygon_vertex_rows: Vec<AreaTriggerPolygonVertexPersistenceRowLikeCpp>,
    pub spline_point_rows: Vec<AreaTriggerSplinePointPersistenceRowLikeCpp>,
    pub create_properties_rows: Vec<AreaTriggerCreatePropertiesPersistenceRowLikeCpp>,
    pub orbit_rows: Vec<AreaTriggerCreatePropertiesOrbitPersistenceRowLikeCpp>,
    pub template_rows: Vec<AreaTriggerTemplatePersistenceRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AreaTriggerTemplateCatalogLoadOutcomeLikeCpp {
    Loaded(AreaTriggerTemplateCatalogRowsLikeCpp),
    Failed { reason: String },
}

/// C++ `AreaTriggerDataStore` World-table source. The concrete adapter owns
/// statement identity and row decoding; the data owner retains validation,
/// correction, attachment and immutable publication semantics.
pub trait AreaTriggerTemplateCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, AreaTriggerTemplateCatalogLoadOutcomeLikeCpp>;
}
