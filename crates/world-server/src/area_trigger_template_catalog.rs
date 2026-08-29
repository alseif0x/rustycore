//! Composition boundary between SQLx-free AreaTrigger rows and `wow-data`.

use anyhow::{Result, bail};
use wow_persistence::{
    AreaTriggerCreatePropertiesOrbitPersistenceRowLikeCpp,
    AreaTriggerCreatePropertiesPersistenceRowLikeCpp,
    AreaTriggerPolygonVertexPersistenceRowLikeCpp, AreaTriggerSplinePointPersistenceRowLikeCpp,
    AreaTriggerTemplateActionPersistenceRowLikeCpp, AreaTriggerTemplateCatalogLoadOutcomeLikeCpp,
    AreaTriggerTemplateCatalogPersistencePortLikeCpp, AreaTriggerTemplatePersistenceRowLikeCpp,
};

fn template_row_like_cpp(
    row: AreaTriggerTemplatePersistenceRowLikeCpp,
) -> wow_data::AreaTriggerTemplateRowLikeCpp {
    wow_data::AreaTriggerTemplateRowLikeCpp {
        id: row.id,
        is_custom: row.is_custom,
        flags: row.flags,
    }
}

fn action_row_like_cpp(
    row: AreaTriggerTemplateActionPersistenceRowLikeCpp,
) -> wow_data::AreaTriggerTemplateActionRowLikeCpp {
    wow_data::AreaTriggerTemplateActionRowLikeCpp {
        area_trigger_id: row.area_trigger_id,
        is_custom: row.is_custom,
        action_type: row.action_type,
        action_param: row.action_param,
        target_type: row.target_type,
    }
}

fn polygon_vertex_row_like_cpp(
    row: AreaTriggerPolygonVertexPersistenceRowLikeCpp,
) -> wow_data::AreaTriggerPolygonVertexRowLikeCpp {
    wow_data::AreaTriggerPolygonVertexRowLikeCpp {
        create_properties_id: row.create_properties_id,
        is_custom: row.is_custom,
        idx: row.idx,
        vertice_x: row.vertice_x,
        vertice_y: row.vertice_y,
        vertice_target_x: row.vertice_target_x,
        vertice_target_y: row.vertice_target_y,
    }
}

fn spline_point_row_like_cpp(
    row: AreaTriggerSplinePointPersistenceRowLikeCpp,
) -> wow_data::AreaTriggerSplinePointRowLikeCpp {
    wow_data::AreaTriggerSplinePointRowLikeCpp {
        create_properties_id: row.create_properties_id,
        is_custom: row.is_custom,
        x: row.x,
        y: row.y,
        z: row.z,
    }
}

fn create_properties_row_like_cpp(
    row: AreaTriggerCreatePropertiesPersistenceRowLikeCpp,
) -> wow_data::AreaTriggerCreatePropertiesRowLikeCpp {
    wow_data::AreaTriggerCreatePropertiesRowLikeCpp {
        id: row.id,
        is_custom: row.is_custom,
        area_trigger_id: row.area_trigger_id,
        is_areatrigger_custom: row.is_areatrigger_custom,
        flags: row.flags,
        move_curve_id: row.move_curve_id,
        scale_curve_id: row.scale_curve_id,
        morph_curve_id: row.morph_curve_id,
        facing_curve_id: row.facing_curve_id,
        anim_id: row.anim_id,
        anim_kit_id: row.anim_kit_id,
        decal_properties_id: row.decal_properties_id,
        time_to_target: row.time_to_target,
        time_to_target_scale: row.time_to_target_scale,
        shape: row.shape,
        shape_data: row.shape_data,
        script_name: row.script_name,
    }
}

fn orbit_row_like_cpp(
    row: AreaTriggerCreatePropertiesOrbitPersistenceRowLikeCpp,
) -> wow_data::AreaTriggerCreatePropertiesOrbitRowLikeCpp {
    wow_data::AreaTriggerCreatePropertiesOrbitRowLikeCpp {
        create_properties_id: row.create_properties_id,
        is_custom: row.is_custom,
        start_delay: row.start_delay,
        circle_radius: row.circle_radius,
        blend_from_radius: row.blend_from_radius,
        initial_angle: row.initial_angle,
        z_offset: row.z_offset,
        counter_clockwise: row.counter_clockwise,
        can_loop: row.can_loop,
    }
}

pub(super) async fn load_area_trigger_template_store_like_cpp(
    persistence: &dyn AreaTriggerTemplateCatalogPersistencePortLikeCpp,
    world_safe_locs: &wow_data::WorldSafeLocStore,
    curve_exists: impl FnMut(u32) -> bool,
    script_id_for_name: impl FnMut(&str) -> wow_data::ScriptIdLikeCpp,
) -> Result<wow_data::AreaTriggerTemplateLoadOutcomeLikeCpp> {
    let rows = match persistence.load_template_rows_like_cpp().await {
        AreaTriggerTemplateCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        AreaTriggerTemplateCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    };

    Ok(wow_data::AreaTriggerTemplateStore::from_rows_like_cpp(
        rows.template_rows.into_iter().map(template_row_like_cpp),
        rows.action_rows.into_iter().map(action_row_like_cpp),
        rows.polygon_vertex_rows
            .into_iter()
            .map(polygon_vertex_row_like_cpp),
        rows.spline_point_rows
            .into_iter()
            .map(spline_point_row_like_cpp),
        rows.create_properties_rows
            .into_iter()
            .map(create_properties_row_like_cpp),
        rows.orbit_rows.into_iter().map(orbit_row_like_cpp),
        world_safe_locs,
        curve_exists,
        script_id_for_name,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::{AreaTriggerTemplateCatalogRowsLikeCpp, PersistenceFutureLikeCpp};

    #[derive(Clone)]
    struct FakeAreaTriggerPersistenceLikeCpp {
        outcome: AreaTriggerTemplateCatalogLoadOutcomeLikeCpp,
    }

    impl AreaTriggerTemplateCatalogPersistencePortLikeCpp for FakeAreaTriggerPersistenceLikeCpp {
        fn load_template_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, AreaTriggerTemplateCatalogLoadOutcomeLikeCpp> {
            Box::pin(async { self.outcome.clone() })
        }
    }

    fn empty_safe_locs() -> wow_data::WorldSafeLocStore {
        wow_data::WorldSafeLocStore::default()
    }

    #[tokio::test]
    async fn typed_rows_cross_the_port_then_keep_area_trigger_rules_like_cpp() {
        let rows = AreaTriggerTemplateCatalogRowsLikeCpp {
            template_rows: vec![AreaTriggerTemplatePersistenceRowLikeCpp {
                id: 10,
                is_custom: false,
                flags: 1,
            }],
            action_rows: vec![AreaTriggerTemplateActionPersistenceRowLikeCpp {
                area_trigger_id: 10,
                is_custom: false,
                action_type: 0,
                action_param: 55,
                target_type: 0,
            }],
            polygon_vertex_rows: vec![AreaTriggerPolygonVertexPersistenceRowLikeCpp {
                create_properties_id: 20,
                is_custom: false,
                idx: 0,
                vertice_x: 1.0,
                vertice_y: 2.0,
                vertice_target_x: Some(3.0),
                vertice_target_y: None,
            }],
            spline_point_rows: vec![AreaTriggerSplinePointPersistenceRowLikeCpp {
                create_properties_id: 20,
                is_custom: false,
                x: 4.0,
                y: 5.0,
                z: 6.0,
            }],
            create_properties_rows: vec![AreaTriggerCreatePropertiesPersistenceRowLikeCpp {
                id: 20,
                is_custom: false,
                area_trigger_id: 10,
                is_areatrigger_custom: false,
                flags: 2,
                move_curve_id: 77,
                scale_curve_id: 0,
                morph_curve_id: 0,
                facing_curve_id: 0,
                anim_id: 8,
                anim_kit_id: 9,
                decal_properties_id: 11,
                time_to_target: 12,
                time_to_target_scale: 13,
                shape: 0,
                shape_data: [0.5; 8],
                script_name: "typed_at".to_string(),
            }],
            orbit_rows: vec![AreaTriggerCreatePropertiesOrbitPersistenceRowLikeCpp {
                create_properties_id: 20,
                is_custom: false,
                start_delay: 14,
                circle_radius: 1.5,
                blend_from_radius: 2.5,
                initial_angle: 3.5,
                z_offset: 4.5,
                counter_clockwise: true,
                can_loop: false,
            }],
        };
        let persistence = FakeAreaTriggerPersistenceLikeCpp {
            outcome: AreaTriggerTemplateCatalogLoadOutcomeLikeCpp::Loaded(rows),
        };

        let outcome = load_area_trigger_template_store_like_cpp(
            &persistence,
            &empty_safe_locs(),
            |curve_id| curve_id != 77,
            |_| wow_data::ScriptIdLikeCpp(42),
        )
        .await
        .unwrap();

        assert_eq!(outcome.report.loaded_templates, 1);
        assert_eq!(outcome.report.loaded_actions, 1);
        assert_eq!(outcome.report.loaded_create_properties, 1);
        assert_eq!(outcome.report.loaded_orbit_infos, 1);
        assert_eq!(outcome.report.invalid_partial_target_vertices.len(), 1);
        assert_eq!(
            outcome
                .report
                .corrected_create_properties_invalid_curves
                .len(),
            1
        );
        let loaded = outcome
            .store
            .get_create_properties_like_cpp(wow_data::AreaTriggerIdLikeCpp {
                id: 20,
                is_custom: false,
            })
            .unwrap();
        assert_eq!(loaded.move_curve_id, 0);
        assert_eq!(loaded.script_name, "typed_at");
        assert_eq!(loaded.spline_points[0].z, 6.0);
        assert_eq!(loaded.orbit_info.as_ref().unwrap().start_delay, 14);
    }

    #[tokio::test]
    async fn empty_and_failed_loads_remain_distinct_before_publication() {
        let empty = FakeAreaTriggerPersistenceLikeCpp {
            outcome: AreaTriggerTemplateCatalogLoadOutcomeLikeCpp::Loaded(Default::default()),
        };
        let outcome = load_area_trigger_template_store_like_cpp(
            &empty,
            &empty_safe_locs(),
            |_| true,
            |_| wow_data::ScriptIdLikeCpp::NONE,
        )
        .await
        .unwrap();
        assert!(outcome.store.is_empty());
        assert_eq!(outcome.report.template_rows_seen, 0);

        let failed = FakeAreaTriggerPersistenceLikeCpp {
            outcome: AreaTriggerTemplateCatalogLoadOutcomeLikeCpp::Failed {
                reason: "template query failed".to_string(),
            },
        };
        let result = load_area_trigger_template_store_like_cpp(
            &failed,
            &empty_safe_locs(),
            |_| true,
            |_| wow_data::ScriptIdLikeCpp::NONE,
        )
        .await;
        let error = match result {
            Ok(_) => panic!("failed load must not publish an AreaTrigger store"),
            Err(error) => error,
        };
        assert_eq!(error.to_string(), "template query failed");
    }

    #[test]
    fn every_boundary_row_keeps_field_and_nullable_mapping() {
        let polygon = polygon_vertex_row_like_cpp(AreaTriggerPolygonVertexPersistenceRowLikeCpp {
            create_properties_id: 1,
            is_custom: true,
            idx: 2,
            vertice_x: 3.0,
            vertice_y: 4.0,
            vertice_target_x: Some(5.0),
            vertice_target_y: None,
        });
        assert_eq!(polygon.create_properties_id, 1);
        assert!(polygon.is_custom);
        assert_eq!(polygon.idx, 2);
        assert_eq!(polygon.vertice_target_x, Some(5.0));
        assert_eq!(polygon.vertice_target_y, None);

        let mut source = AreaTriggerCreatePropertiesPersistenceRowLikeCpp {
            id: 10,
            is_custom: false,
            area_trigger_id: 11,
            is_areatrigger_custom: true,
            flags: 12,
            move_curve_id: 13,
            scale_curve_id: 14,
            morph_curve_id: 15,
            facing_curve_id: 16,
            anim_id: -17,
            anim_kit_id: -18,
            decal_properties_id: 19,
            time_to_target: 20,
            time_to_target_scale: 21,
            shape: 6,
            shape_data: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
            script_name: "mapped".to_string(),
        };
        source.shape_data[7] = 70.0;
        let mapped = create_properties_row_like_cpp(source);
        assert_eq!(mapped.area_trigger_id, 11);
        assert!(mapped.is_areatrigger_custom);
        assert_eq!(mapped.facing_curve_id, 16);
        assert_eq!(mapped.anim_kit_id, -18);
        assert_eq!(mapped.shape_data[7], 70.0);
        assert_eq!(mapped.script_name, "mapped");
    }
}
