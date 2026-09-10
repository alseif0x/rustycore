//! Area-trigger template regressions.
//!
//! Separated from area_trigger_template.rs under #683.

use super::*;
use crate::{WorldSafeLoc, WorldSafeLocStore};
use wow_core::Position;

fn safe_locs(ids: impl IntoIterator<Item = u32>) -> WorldSafeLocStore {
    WorldSafeLocStore::from_locs_for_test(ids.into_iter().map(|id| WorldSafeLoc {
        id,
        map_id: 0,
        position: Position::new(0.0, 0.0, 0.0, 0.0),
    }))
}

fn template(id: u32, is_custom: bool, flags: u32) -> AreaTriggerTemplateRowLikeCpp {
    AreaTriggerTemplateRowLikeCpp {
        id,
        is_custom,
        flags,
    }
}

fn action(
    id: u32,
    is_custom: bool,
    action_type: u32,
    action_param: u32,
    target_type: u32,
) -> AreaTriggerTemplateActionRowLikeCpp {
    AreaTriggerTemplateActionRowLikeCpp {
        area_trigger_id: id,
        is_custom,
        action_type,
        action_param,
        target_type,
    }
}

fn polygon_vertex(
    create_properties_id: u32,
    is_custom: bool,
    idx: u32,
    x: f32,
    y: f32,
    target: Option<(f32, f32)>,
) -> AreaTriggerPolygonVertexRowLikeCpp {
    AreaTriggerPolygonVertexRowLikeCpp {
        create_properties_id,
        is_custom,
        idx,
        vertice_x: x,
        vertice_y: y,
        vertice_target_x: target.map(|(target_x, _)| target_x),
        vertice_target_y: target.map(|(_, target_y)| target_y),
    }
}

fn partial_polygon_vertex(
    create_properties_id: u32,
    is_custom: bool,
    idx: u32,
) -> AreaTriggerPolygonVertexRowLikeCpp {
    AreaTriggerPolygonVertexRowLikeCpp {
        create_properties_id,
        is_custom,
        idx,
        vertice_x: 1.0,
        vertice_y: 2.0,
        vertice_target_x: Some(3.0),
        vertice_target_y: None,
    }
}

fn spline_point(
    create_properties_id: u32,
    is_custom: bool,
    x: f32,
    y: f32,
    z: f32,
) -> AreaTriggerSplinePointRowLikeCpp {
    AreaTriggerSplinePointRowLikeCpp {
        create_properties_id,
        is_custom,
        x,
        y,
        z,
    }
}

fn create_properties(
    id: u32,
    is_custom: bool,
    area_trigger_id: u32,
    is_areatrigger_custom: bool,
    shape: u8,
) -> AreaTriggerCreatePropertiesRowLikeCpp {
    AreaTriggerCreatePropertiesRowLikeCpp {
        id,
        is_custom,
        area_trigger_id,
        is_areatrigger_custom,
        flags: AREATRIGGER_CREATE_PROPERTIES_FLAG_NONE_LIKE_CPP,
        move_curve_id: 0,
        scale_curve_id: 0,
        morph_curve_id: 0,
        facing_curve_id: 0,
        anim_id: 9,
        anim_kit_id: 10,
        decal_properties_id: 11,
        time_to_target: 12,
        time_to_target_scale: 13,
        shape,
        shape_data: [0.0; MAX_AREATRIGGER_ENTITY_DATA_LIKE_CPP],
        script_name: String::new(),
    }
}

fn orbit(create_properties_id: u32, is_custom: bool) -> AreaTriggerCreatePropertiesOrbitRowLikeCpp {
    AreaTriggerCreatePropertiesOrbitRowLikeCpp {
        create_properties_id,
        is_custom,
        start_delay: 7,
        circle_radius: 1.5,
        blend_from_radius: 2.5,
        initial_angle: 3.5,
        z_offset: 4.5,
        counter_clockwise: true,
        can_loop: true,
    }
}

mod scenarios;
