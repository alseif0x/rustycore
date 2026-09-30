//! Ordered SQL AreaTrigger template startup and diagnostics.

use anyhow::Context;
use std::sync::Arc;
use tracing::info;

pub(super) async fn load_area_trigger_templates(
    area_trigger_template_persistence: &dyn wow_persistence::AreaTriggerTemplateCatalogPersistencePortLikeCpp,
    world_safe_loc_store: &wow_data::WorldSafeLocStore,
    curve_store: &wow_data::progression_rewards::CurveStore,
    script_name_interner: &mut wow_data::ScriptNameInternerLikeCpp,
) -> anyhow::Result<(
    Arc<wow_data::AreaTriggerTemplateStore>,
    wow_data::AreaTriggerTemplateLoadReportLikeCpp,
)> {
    let area_trigger_template_outcome =
        crate::area::trigger_template_catalog::load_area_trigger_template_store_like_cpp(
            area_trigger_template_persistence,
            world_safe_loc_store,
            |id| curve_store.get(id).is_some(),
            |name| script_name_interner.get_script_id_like_cpp(name, true),
        )
        .await
        .context("Failed to load C++ AreaTriggerDataStore template/create-properties rows")?;
    for (area_trigger_id, action_type, param) in &area_trigger_template_outcome
        .report
        .skipped_actions_invalid_action_type
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_template_actions` has invalid ActionType {} for AreaTriggerId ({},{}) and Param {}",
            action_type,
            area_trigger_id.id,
            u32::from(area_trigger_id.is_custom),
            param
        );
    }
    for (area_trigger_id, target_type, param) in &area_trigger_template_outcome
        .report
        .skipped_actions_invalid_target_type
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_template_actions` has invalid TargetType {} for AreaTriggerId ({},{}) and Param {}",
            target_type,
            area_trigger_id.id,
            u32::from(area_trigger_id.is_custom),
            param
        );
    }
    for (area_trigger_id, param) in &area_trigger_template_outcome
        .report
        .skipped_actions_invalid_teleport_world_safe_loc
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_template_actions` has invalid entry for AreaTriggerId ({},{}) with TargetType=Teleport and Param ({}) not a valid world safe loc entry",
            area_trigger_id.id,
            u32::from(area_trigger_id.is_custom),
            param
        );
    }
    for (create_properties_id, idx) in &area_trigger_template_outcome
        .report
        .invalid_partial_target_vertices
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_create_properties_polygon_vertex` has listed invalid target vertices (AreaTriggerCreatePropertiesId: ({},{}), Index: {}).",
            create_properties_id.id,
            u32::from(create_properties_id.is_custom),
            idx
        );
    }
    for (create_properties_id, area_trigger_id) in &area_trigger_template_outcome
        .report
        .skipped_create_properties_invalid_template
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_create_properties` references invalid AreaTrigger (Id: {}, IsCustom: {}) for AreaTriggerCreatePropertiesId (Id: {}, IsCustom: {})",
            area_trigger_id.id,
            u32::from(area_trigger_id.is_custom),
            create_properties_id.id,
            u32::from(create_properties_id.is_custom)
        );
    }
    for (create_properties_id, shape) in &area_trigger_template_outcome
        .report
        .skipped_create_properties_invalid_shape
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_create_properties` has listed AreaTriggerCreatePropertiesId (Id: {}, IsCustom: {}) with invalid shape {}.",
            create_properties_id.id,
            u32::from(create_properties_id.is_custom),
            shape
        );
    }
    for (area_trigger_id, create_properties_id, curve_field, curve_id) in
        &area_trigger_template_outcome
            .report
            .corrected_create_properties_invalid_curves
    {
        let curve_name = match curve_field {
            wow_data::AreaTriggerCurveFieldLikeCpp::Move => "MoveCurveId",
            wow_data::AreaTriggerCurveFieldLikeCpp::Scale => "ScaleCurveId",
            wow_data::AreaTriggerCurveFieldLikeCpp::Morph => "MorphCurveId",
            wow_data::AreaTriggerCurveFieldLikeCpp::Facing => "FacingCurveId",
        };
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_create_properties` has listed AreaTrigger (Id: {}, IsCustom: {}) for AreaTriggerCreatePropertiesId (Id: {}, IsCustom: {}) with invalid {} ({}), set to 0!",
            area_trigger_id.id,
            u32::from(area_trigger_id.is_custom),
            create_properties_id.id,
            u32::from(create_properties_id.is_custom),
            curve_name,
            curve_id
        );
    }
    for create_properties_id in &area_trigger_template_outcome
        .report
        .invalid_polygon_target_vertex_counts
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_create_properties_polygon_vertex` has invalid target vertices, either all or none vertices must have a corresponding target vertex (AreaTriggerCreatePropertiesId: (Id: {}, IsCustom: {})).",
            create_properties_id.id,
            u32::from(create_properties_id.is_custom)
        );
    }
    for create_properties_id in &area_trigger_template_outcome
        .report
        .skipped_orbit_invalid_create_properties
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_create_properties_orbit` reference invalid AreaTriggerCreatePropertiesId: (Id: {}, IsCustom: {})",
            create_properties_id.id,
            u32::from(create_properties_id.is_custom)
        );
    }
    for (create_properties_id, float_field, value) in &area_trigger_template_outcome
        .report
        .corrected_orbit_invalid_floats
    {
        let float_name = match float_field {
            wow_data::AreaTriggerOrbitFloatFieldLikeCpp::Radius => "Radius",
            wow_data::AreaTriggerOrbitFloatFieldLikeCpp::BlendFromRadius => "BlendFromRadius",
            wow_data::AreaTriggerOrbitFloatFieldLikeCpp::InitialAngle => "InitialAngle",
            wow_data::AreaTriggerOrbitFloatFieldLikeCpp::ZOffset => "ZOffset",
        };
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_create_properties_orbit` has listed areatrigger (AreaTriggerCreatePropertiesId: {}, IsCustom: {}) with invalid {} ({}), set to 0!",
            create_properties_id.id,
            u32::from(create_properties_id.is_custom),
            float_name,
            value
        );
    }
    let area_trigger_template_report = area_trigger_template_outcome.report;
    let area_trigger_template_store = Arc::new(area_trigger_template_outcome.store);
    info!(
        "Loaded {} C++ area-trigger templates, {} create properties, {} orbit infos, {} actions, {} polygon vertices ({} targets), and {} spline points from {} template rows / {} create-property rows / {} orbit rows / {} action rows / {} polygon rows / {} spline rows ({} invalid rows skipped; spawns pending)",
        area_trigger_template_report.loaded_templates,
        area_trigger_template_report.loaded_create_properties,
        area_trigger_template_report.loaded_orbit_infos,
        area_trigger_template_report.loaded_actions,
        area_trigger_template_report.loaded_polygon_vertices,
        area_trigger_template_report.loaded_polygon_target_vertices,
        area_trigger_template_report.loaded_spline_points,
        area_trigger_template_report.template_rows_seen,
        area_trigger_template_report.create_properties_rows_seen,
        area_trigger_template_report.orbit_rows_seen,
        area_trigger_template_report.action_rows_seen,
        area_trigger_template_report.polygon_vertex_rows_seen,
        area_trigger_template_report.spline_point_rows_seen,
        area_trigger_template_report
            .skipped_actions_invalid_action_type
            .len()
            + area_trigger_template_report
                .skipped_actions_invalid_target_type
                .len()
            + area_trigger_template_report
                .skipped_actions_invalid_teleport_world_safe_loc
                .len()
            + area_trigger_template_report
                .invalid_partial_target_vertices
                .len()
            + area_trigger_template_report
                .skipped_create_properties_invalid_template
                .len()
            + area_trigger_template_report
                .skipped_create_properties_invalid_shape
                .len()
            + area_trigger_template_report
                .corrected_create_properties_invalid_curves
                .len()
            + area_trigger_template_report
                .invalid_polygon_target_vertex_counts
                .len()
            + area_trigger_template_report
                .skipped_orbit_invalid_create_properties
                .len()
            + area_trigger_template_report
                .corrected_orbit_invalid_floats
                .len()
    );
    Ok((area_trigger_template_store, area_trigger_template_report))
}

pub(super) struct AreaTriggerWorldCatalogs {
    pub(super) tavern_area_trigger_report: wow_data::TavernAreaTriggerLoadReportLikeCpp,
    pub(super) area_trigger_script_report: wow_data::AreaTriggerScriptLoadReportLikeCpp,
    pub(super) tavern_area_trigger_store: Arc<wow_data::TavernAreaTriggerStoreLikeCpp>,
    pub(super) area_trigger_script_store: Arc<wow_data::AreaTriggerScriptStoreLikeCpp>,
    pub(super) area_trigger_store: Arc<wow_data::AreaTriggerStore>,
    pub(super) area_trigger_world_persistence: wow_database::MariaDbAreaTriggerWorldCatalogPersistenceAdapterLikeCpp,
}

pub(super) async fn load_world_catalogs(
    world_db: &Arc<wow_database::WorldDatabase>,
    area_trigger_db2_store: &wow_data::AreaTriggerDb2Store,
    script_name_interner: &mut Arc<wow_data::ScriptNameInternerLikeCpp>,
) -> anyhow::Result<AreaTriggerWorldCatalogs> {
    // Load area trigger store (collision detection + teleportation)
    let area_trigger_world_persistence =
        wow_database::MariaDbAreaTriggerWorldCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        ));
    let area_trigger_world_catalogs =
        crate::area::trigger_world_catalog::load_area_trigger_world_catalogs_like_cpp(
            &area_trigger_world_persistence,
            area_trigger_db2_store,
            Arc::make_mut(script_name_interner),
        )
        .await?;
    let area_trigger_store = area_trigger_world_catalogs.area_trigger_store;
    let area_trigger_script_outcome = area_trigger_world_catalogs.script_outcome;
    let area_trigger_script_store = Arc::new(area_trigger_script_outcome.store);
    info!(
        "Loaded {} C++ area trigger script bindings ({} skipped missing area trigger)",
        area_trigger_script_store.len(),
        area_trigger_script_outcome
            .report
            .skipped_missing_area_trigger
            .len()
    );
    let tavern_area_trigger_outcome = area_trigger_world_catalogs.tavern_outcome;
    let tavern_area_trigger_store = Arc::new(tavern_area_trigger_outcome.store);
    info!(
        "Loaded {} C++ tavern area triggers ({} rows seen; {} skipped missing AreaTrigger.db2)",
        tavern_area_trigger_store.len(),
        tavern_area_trigger_outcome.report.rows_seen,
        tavern_area_trigger_outcome
            .report
            .skipped_missing_area_trigger
            .len()
    );
    Ok(AreaTriggerWorldCatalogs {
        area_trigger_world_persistence,
        area_trigger_store,
        area_trigger_script_store,
        tavern_area_trigger_store,
        area_trigger_script_report: area_trigger_script_outcome.report,
        tavern_area_trigger_report: tavern_area_trigger_outcome.report,
    })
}
