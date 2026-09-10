//! Area-trigger template regressions.
//!
//! Moved out of area_trigger_template.rs under #683; every test is unchanged.

use super::*;

#[test]
fn area_trigger_template_store_keys_by_id_and_custom_flag_like_cpp() {
    let store = AreaTriggerTemplateStore::from_keys([(7, false), (7, true)]);

    assert!(store.contains(7, false));
    assert!(store.contains(7, true));
    assert!(!store.contains(8, false));
    assert_eq!(store.len(), 2);
}

#[test]
fn load_templates_moves_valid_actions_into_matching_template_like_cpp() {
    let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
        [template(
            10,
            false,
            AREATRIGGER_FLAG_IS_SERVER_SIDE_LIKE_CPP,
        )],
        [
            action(
                10,
                false,
                AREATRIGGER_ACTION_CAST_LIKE_CPP,
                123,
                AREATRIGGER_ACTION_USER_ANY_LIKE_CPP,
            ),
            action(
                10,
                false,
                AREATRIGGER_ACTION_TELEPORT_LIKE_CPP,
                7,
                AREATRIGGER_ACTION_USER_CASTER_LIKE_CPP,
            ),
        ],
        [],
        [],
        [],
        [],
        &safe_locs([7]),
        |_| true,
        |_| ScriptIdLikeCpp::NONE,
    );

    let loaded = outcome
        .store
        .get_template_like_cpp(AreaTriggerIdLikeCpp {
            id: 10,
            is_custom: false,
        })
        .unwrap();

    assert_eq!(outcome.report.template_rows_seen, 1);
    assert_eq!(outcome.report.action_rows_seen, 2);
    assert_eq!(outcome.report.loaded_templates, 1);
    assert_eq!(outcome.report.loaded_actions, 2);
    assert_eq!(loaded.flags, AREATRIGGER_FLAG_IS_SERVER_SIDE_LIKE_CPP);
    assert_eq!(loaded.actions.len(), 2);
    assert_eq!(loaded.actions[0].param, 123);
    assert_eq!(loaded.actions[1].param, 7);
}

#[test]
fn load_templates_skips_invalid_actions_like_cpp() {
    let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
        [template(10, false, 0)],
        [
            action(10, false, AREATRIGGER_ACTION_MAX_LIKE_CPP, 1, 0),
            action(10, false, 0, 2, AREATRIGGER_ACTION_USER_MAX_LIKE_CPP),
            action(10, false, AREATRIGGER_ACTION_TELEPORT_LIKE_CPP, 999, 0),
        ],
        [],
        [],
        [],
        [],
        &safe_locs([7]),
        |_| true,
        |_| ScriptIdLikeCpp::NONE,
    );

    let loaded = outcome
        .store
        .get_template_like_cpp(AreaTriggerIdLikeCpp {
            id: 10,
            is_custom: false,
        })
        .unwrap();

    assert!(loaded.actions.is_empty());
    assert_eq!(
        outcome.report.skipped_actions_invalid_action_type,
        [(
            AreaTriggerIdLikeCpp {
                id: 10,
                is_custom: false
            },
            AREATRIGGER_ACTION_MAX_LIKE_CPP,
            1
        )]
    );
    assert_eq!(
        outcome.report.skipped_actions_invalid_target_type,
        [(
            AreaTriggerIdLikeCpp {
                id: 10,
                is_custom: false
            },
            AREATRIGGER_ACTION_USER_MAX_LIKE_CPP,
            2
        )]
    );
    assert_eq!(
        outcome
            .report
            .skipped_actions_invalid_teleport_world_safe_loc,
        [(
            AreaTriggerIdLikeCpp {
                id: 10,
                is_custom: false
            },
            999
        )]
    );
}

#[test]
fn actions_without_template_are_kept_only_in_staging_like_cpp() {
    let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
        [template(10, false, 0)],
        [action(99, false, AREATRIGGER_ACTION_CAST_LIKE_CPP, 1, 0)],
        [],
        [],
        [],
        [],
        &safe_locs([]),
        |_| true,
        |_| ScriptIdLikeCpp::NONE,
    );

    assert_eq!(outcome.report.loaded_actions, 1);
    assert_eq!(outcome.store.action_len(), 0);
}

#[test]
fn load_templates_stages_polygon_vertices_and_spline_points_like_cpp() {
    let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
        [],
        [],
        [
            polygon_vertex(90, false, 0, 1.0, 2.0, Some((10.0, 20.0))),
            polygon_vertex(90, false, 1, 3.0, 4.0, None),
        ],
        [
            spline_point(90, false, 5.0, 6.0, 7.0),
            spline_point(90, false, 8.0, 9.0, 10.0),
        ],
        [],
        [],
        &safe_locs([]),
        |_| true,
        |_| ScriptIdLikeCpp::NONE,
    );
    let id = AreaTriggerIdLikeCpp {
        id: 90,
        is_custom: false,
    };

    assert_eq!(outcome.report.polygon_vertex_rows_seen, 2);
    assert_eq!(outcome.report.spline_point_rows_seen, 2);
    assert_eq!(outcome.report.loaded_polygon_vertices, 2);
    assert_eq!(outcome.report.loaded_polygon_target_vertices, 1);
    assert_eq!(outcome.report.loaded_spline_points, 2);
    assert_eq!(
        outcome.store.polygon_vertices_like_cpp(id).unwrap(),
        [
            AreaTriggerPosition2LikeCpp { x: 1.0, y: 2.0 },
            AreaTriggerPosition2LikeCpp { x: 3.0, y: 4.0 },
        ]
    );
    assert_eq!(
        outcome.store.polygon_target_vertices_like_cpp(id).unwrap(),
        [AreaTriggerPosition2LikeCpp { x: 10.0, y: 20.0 }]
    );
    assert_eq!(
        outcome.store.spline_points_like_cpp(id).unwrap(),
        [
            AreaTriggerPosition3LikeCpp {
                x: 5.0,
                y: 6.0,
                z: 7.0,
            },
            AreaTriggerPosition3LikeCpp {
                x: 8.0,
                y: 9.0,
                z: 10.0,
            },
        ]
    );
}

#[test]
fn load_templates_keeps_base_vertex_but_skips_partial_target_like_cpp() {
    let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
        [],
        [],
        [partial_polygon_vertex(77, true, 4)],
        [],
        [],
        [],
        &safe_locs([]),
        |_| true,
        |_| ScriptIdLikeCpp::NONE,
    );
    let id = AreaTriggerIdLikeCpp {
        id: 77,
        is_custom: true,
    };

    assert_eq!(outcome.report.loaded_polygon_vertices, 1);
    assert_eq!(outcome.report.loaded_polygon_target_vertices, 0);
    assert_eq!(outcome.report.invalid_partial_target_vertices, [(id, 4)]);
    assert_eq!(
        outcome.store.polygon_vertices_like_cpp(id).unwrap(),
        [AreaTriggerPosition2LikeCpp { x: 1.0, y: 2.0 }]
    );
    assert!(outcome.store.polygon_target_vertices_like_cpp(id).is_none());
}

#[test]
fn load_templates_builds_create_properties_and_attaches_shape_data_like_cpp() {
    let mut row = create_properties(200, false, 10, false, AREATRIGGER_SHAPE_POLYGON_LIKE_CPP);
    row.flags = AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ATTACHED_LIKE_CPP;
    row.move_curve_id = 44;
    row.scale_curve_id = 55;
    row.shape_data[0] = -1.0;
    row.shape_data[1] = 0.0;
    row.script_name = "at_script".to_string();

    let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
        [template(10, false, 0)],
        [],
        [
            polygon_vertex(200, false, 0, 1.0, 2.0, Some((11.0, 12.0))),
            polygon_vertex(200, false, 1, 3.0, 4.0, Some((13.0, 14.0))),
        ],
        [spline_point(200, false, 5.0, 6.0, 7.0)],
        [row],
        [],
        &safe_locs([]),
        |curve_id| curve_id == 44 || curve_id == 55,
        |name| {
            assert_eq!(name, "at_script");
            ScriptIdLikeCpp(88)
        },
    );
    let id = AreaTriggerIdLikeCpp {
        id: 200,
        is_custom: false,
    };
    let props = outcome.store.get_create_properties_like_cpp(id).unwrap();

    assert_eq!(outcome.report.create_properties_rows_seen, 1);
    assert_eq!(outcome.report.loaded_create_properties, 1);
    assert_eq!(
        outcome.report.corrected_polygon_heights,
        [AreaTriggerIdLikeCpp {
            id: 200,
            is_custom: false
        }]
    );
    assert_eq!(
        props.template_id,
        Some(AreaTriggerIdLikeCpp {
            id: 10,
            is_custom: false
        })
    );
    assert_eq!(
        props.flags,
        AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ATTACHED_LIKE_CPP
    );
    assert_eq!(props.move_curve_id, 44);
    assert_eq!(props.scale_curve_id, 55);
    assert_eq!(props.shape.shape_type, AREATRIGGER_SHAPE_POLYGON_LIKE_CPP);
    assert_eq!(props.shape.data[0], 1.0);
    assert_eq!(props.shape.data[1], 1.0);
    assert_eq!(props.shape.polygon_vertices.len(), 2);
    assert_eq!(props.shape.polygon_vertices_target.len(), 2);
    assert_eq!(
        props.spline_points,
        [AreaTriggerPosition3LikeCpp {
            x: 5.0,
            y: 6.0,
            z: 7.0
        }]
    );
    assert_eq!(props.script_id, ScriptIdLikeCpp(88));
    assert_eq!(props.script_name, "at_script");
}

#[test]
fn load_templates_skips_invalid_create_properties_like_cpp() {
    let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
        [template(10, false, 0)],
        [],
        [],
        [],
        [
            create_properties(201, false, 99, false, AREATRIGGER_SHAPE_SPHERE_LIKE_CPP),
            create_properties(202, true, 0, false, AREATRIGGER_SHAPE_MAX_LIKE_CPP),
        ],
        [],
        &safe_locs([]),
        |_| true,
        |_| ScriptIdLikeCpp::NONE,
    );

    assert_eq!(outcome.report.loaded_create_properties, 0);
    assert_eq!(
        outcome.report.skipped_create_properties_invalid_template,
        [(
            AreaTriggerIdLikeCpp {
                id: 201,
                is_custom: false,
            },
            AreaTriggerIdLikeCpp {
                id: 99,
                is_custom: false,
            }
        )]
    );
    assert_eq!(
        outcome.report.skipped_create_properties_invalid_shape,
        [(
            AreaTriggerIdLikeCpp {
                id: 202,
                is_custom: true,
            },
            AREATRIGGER_SHAPE_MAX_LIKE_CPP
        )]
    );
}

#[test]
fn load_templates_zeroes_invalid_create_property_curves_like_cpp() {
    let mut row = create_properties(203, false, 10, false, AREATRIGGER_SHAPE_SPHERE_LIKE_CPP);
    row.move_curve_id = 100;
    row.scale_curve_id = 101;
    row.morph_curve_id = 102;
    row.facing_curve_id = 103;

    let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
        [template(10, false, 0)],
        [],
        [],
        [],
        [row],
        [],
        &safe_locs([]),
        |curve_id| curve_id == 101,
        |_| ScriptIdLikeCpp::NONE,
    );
    let id = AreaTriggerIdLikeCpp {
        id: 203,
        is_custom: false,
    };
    let props = outcome.store.get_create_properties_like_cpp(id).unwrap();

    assert_eq!(props.move_curve_id, 0);
    assert_eq!(props.scale_curve_id, 101);
    assert_eq!(props.morph_curve_id, 0);
    assert_eq!(props.facing_curve_id, 0);
    assert_eq!(
        outcome.report.corrected_create_properties_invalid_curves,
        [
            (
                AreaTriggerIdLikeCpp {
                    id: 10,
                    is_custom: false,
                },
                id,
                AreaTriggerCurveFieldLikeCpp::Move,
                100,
            ),
            (
                AreaTriggerIdLikeCpp {
                    id: 10,
                    is_custom: false,
                },
                id,
                AreaTriggerCurveFieldLikeCpp::Morph,
                102,
            ),
            (
                AreaTriggerIdLikeCpp {
                    id: 10,
                    is_custom: false,
                },
                id,
                AreaTriggerCurveFieldLikeCpp::Facing,
                103,
            ),
        ]
    );
}

#[test]
fn load_templates_clears_mismatched_polygon_targets_like_cpp() {
    let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
        [],
        [],
        [
            polygon_vertex(204, false, 0, 1.0, 2.0, Some((11.0, 12.0))),
            polygon_vertex(204, false, 1, 3.0, 4.0, None),
        ],
        [],
        [create_properties(
            204,
            false,
            0,
            false,
            AREATRIGGER_SHAPE_POLYGON_LIKE_CPP,
        )],
        [],
        &safe_locs([]),
        |_| true,
        |_| ScriptIdLikeCpp::NONE,
    );
    let id = AreaTriggerIdLikeCpp {
        id: 204,
        is_custom: false,
    };
    let props = outcome.store.get_create_properties_like_cpp(id).unwrap();

    assert_eq!(props.shape.polygon_vertices.len(), 2);
    assert!(props.shape.polygon_vertices_target.is_empty());
    assert_eq!(outcome.report.invalid_polygon_target_vertex_counts, [id]);
}

#[test]
fn load_templates_attaches_orbit_info_like_cpp() {
    let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
        [],
        [],
        [],
        [],
        [create_properties(
            205,
            false,
            0,
            false,
            AREATRIGGER_SHAPE_SPHERE_LIKE_CPP,
        )],
        [orbit(205, false)],
        &safe_locs([]),
        |_| true,
        |_| ScriptIdLikeCpp::NONE,
    );
    let id = AreaTriggerIdLikeCpp {
        id: 205,
        is_custom: false,
    };
    let orbit = outcome
        .store
        .get_create_properties_like_cpp(id)
        .unwrap()
        .orbit_info
        .unwrap();

    assert_eq!(outcome.report.orbit_rows_seen, 1);
    assert_eq!(outcome.report.loaded_orbit_infos, 1);
    assert_eq!(orbit.start_delay, 7);
    assert_eq!(orbit.radius, 1.5);
    assert_eq!(orbit.blend_from_radius, 2.5);
    assert_eq!(orbit.initial_angle, 3.5);
    assert_eq!(orbit.z_offset, 4.5);
    assert!(orbit.counter_clockwise);
    assert!(orbit.can_loop);
    assert_eq!(orbit.time_to_target, 0);
    assert_eq!(orbit.elapsed_time_for_movement, 0);
}

#[test]
fn load_templates_skips_invalid_orbit_reference_and_zeroes_nonfinite_floats_like_cpp() {
    let mut invalid_float_orbit = orbit(206, false);
    invalid_float_orbit.circle_radius = f32::NAN;
    invalid_float_orbit.blend_from_radius = f32::INFINITY;
    invalid_float_orbit.initial_angle = f32::NEG_INFINITY;

    let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
        [],
        [],
        [],
        [],
        [create_properties(
            206,
            false,
            0,
            false,
            AREATRIGGER_SHAPE_SPHERE_LIKE_CPP,
        )],
        [orbit(999, true), invalid_float_orbit],
        &safe_locs([]),
        |_| true,
        |_| ScriptIdLikeCpp::NONE,
    );
    let id = AreaTriggerIdLikeCpp {
        id: 206,
        is_custom: false,
    };
    let orbit = outcome
        .store
        .get_create_properties_like_cpp(id)
        .unwrap()
        .orbit_info
        .unwrap();

    assert_eq!(
        outcome.report.skipped_orbit_invalid_create_properties,
        [AreaTriggerIdLikeCpp {
            id: 999,
            is_custom: true,
        }]
    );
    assert_eq!(orbit.radius, 0.0);
    assert_eq!(orbit.blend_from_radius, 0.0);
    assert_eq!(orbit.initial_angle, 0.0);
    assert_eq!(orbit.z_offset, 4.5);
    assert_eq!(
        outcome
            .report
            .corrected_orbit_invalid_floats
            .iter()
            .map(|(_, field, _)| *field)
            .collect::<Vec<_>>(),
        [
            AreaTriggerOrbitFloatFieldLikeCpp::Radius,
            AreaTriggerOrbitFloatFieldLikeCpp::BlendFromRadius,
            AreaTriggerOrbitFloatFieldLikeCpp::InitialAngle,
        ]
    );
}
